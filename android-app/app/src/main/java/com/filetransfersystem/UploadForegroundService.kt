package com.filetransfersystem

import android.app.Notification
import android.app.NotificationChannel
import android.app.NotificationManager
import android.app.PendingIntent
import android.app.Service
import android.content.Context
import android.content.Intent
import android.content.pm.ServiceInfo
import android.os.Build
import android.os.IBinder
import android.util.Log
import androidx.core.app.NotificationCompat
import androidx.core.app.ServiceCompat
import androidx.core.content.ContextCompat
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.Job
import kotlinx.coroutines.SupervisorJob
import kotlinx.coroutines.cancel
import kotlinx.coroutines.launch
import java.io.File
import java.util.UUID

/**
 * User-visible foreground uploader. The Rust upload runs independently from MainActivity,
 * so switching apps or destroying/recreating the Activity does not interrupt the transfer.
 */
class UploadForegroundService : Service() {

    companion object {
        private const val TAG = "UploadForegroundService"
        private const val CHANNEL_ID = "udp_file_uploads"
        private const val CHANNEL_NAME = "Đang gửi file"
        private const val NOTIFICATION_ID = 2107

        private const val ACTION_START = "com.filetransfersystem.action.START_UPLOAD"

        private const val EXTRA_TRANSFER_ID = "transfer_id"
        private const val EXTRA_FILE_PATH = "file_path"
        private const val EXTRA_SERVER_IP = "server_ip"
        private const val EXTRA_UDP_PORT = "udp_port"
        private const val EXTRA_HTTP_PORT = "http_port"
        private const val EXTRA_BLOCK_SIZE = "block_size"
        private const val EXTRA_PASSWORD = "password"

        fun start(context: Context, config: UploadConfig): String {
            val transferId = UUID.randomUUID().toString()
            val intent = Intent(context, UploadForegroundService::class.java).apply {
                action = ACTION_START
                putExtra(EXTRA_TRANSFER_ID, transferId)
                putExtra(EXTRA_FILE_PATH, config.filePath)
                putExtra(EXTRA_SERVER_IP, config.serverIp)
                putExtra(EXTRA_UDP_PORT, config.udpPort)
                putExtra(EXTRA_HTTP_PORT, config.httpPort)
                putExtra(EXTRA_BLOCK_SIZE, config.blockSize)
                putExtra(EXTRA_PASSWORD, config.password)
            }
            ContextCompat.startForegroundService(context, intent)
            return transferId
        }
    }

    private val serviceScope = CoroutineScope(SupervisorJob() + Dispatchers.IO)
    private var uploadJob: Job? = null
    private var currentTransferId: String = ""

    override fun onCreate() {
        super.onCreate()
        createNotificationChannel()
        UploadServiceState.restore(this)
    }

    override fun onStartCommand(intent: Intent?, flags: Int, startId: Int): Int {
        if (intent?.action != ACTION_START) {
            return START_NOT_STICKY
        }

        if (uploadJob?.isActive == true) {
            val replacementId = intent.getStringExtra(EXTRA_TRANSFER_ID).orEmpty()
            if (replacementId.isNotBlank()) {
                currentTransferId = replacementId
                UploadServiceState.publish(
                    this,
                    UploadServiceState.state.value.copy(
                        transferId = replacementId,
                        updatedAt = System.currentTimeMillis()
                    )
                )
            }
            Log.w(TAG, "An upload is already running; attached caller to the active transfer")
            return START_REDELIVER_INTENT
        }

        val filePath = intent.getStringExtra(EXTRA_FILE_PATH).orEmpty()
        val serverIp = intent.getStringExtra(EXTRA_SERVER_IP).orEmpty()
        val udpPort = intent.getIntExtra(EXTRA_UDP_PORT, 5000)
        val httpPort = intent.getIntExtra(EXTRA_HTTP_PORT, 8080)
        val blockSize = intent.getIntExtra(EXTRA_BLOCK_SIZE, 16384)
        val password = intent.getStringExtra(EXTRA_PASSWORD)
        currentTransferId = intent.getStringExtra(EXTRA_TRANSFER_ID).orEmpty().ifBlank {
            UUID.randomUUID().toString()
        }

        val file = File(filePath)
        val initialState = BackgroundUploadState(
            transferId = currentTransferId,
            running = true,
            filePath = filePath,
            fileName = file.name.ifBlank { "file" },
            serverIp = serverIp,
            bytesTransferred = 0L,
            totalBytes = file.takeIf { it.exists() }?.length() ?: 0L,
            bytesPerSecond = 0L,
            statusText = "Đang chuẩn bị gửi file…",
            resultCode = null,
            updatedAt = System.currentTimeMillis()
        )
        UploadServiceState.publish(this, initialState)
        startAsDataSyncForeground(buildNotification(initialState))

        uploadJob = serviceScope.launch {
            executeUpload(
                startId = startId,
                filePath = filePath,
                serverIp = serverIp,
                udpPort = udpPort,
                httpPort = httpPort,
                blockSize = blockSize,
                password = password
            )
        }

        // If Android has to kill the process for memory pressure, ask it to redeliver the
        // original start Intent. The server-side resume support makes the restarted transfer safe.
        return START_REDELIVER_INTENT
    }

    private suspend fun executeUpload(
        startId: Int,
        filePath: String,
        serverIp: String,
        udpPort: Int,
        httpPort: Int,
        blockSize: Int,
        password: String?
    ) {
        val file = File(filePath)
        if (!file.exists() || !file.canRead()) {
            finishUpload(startId, filePath, serverIp, "", -2)
            return
        }

        val fileHash = try {
            RustUploader.calculateHashId(filePath)
        } catch (t: Throwable) {
            Log.e(TAG, "Failed to calculate file hash", t)
            ""
        }

        var lastPersistAt = 0L
        var lastNotificationAt = 0L
        val sendingState = UploadServiceState.state.value.copy(
            running = true,
            statusText = "Đang gửi ${file.name}…",
            fileHash = fileHash,
            totalBytes = file.length(),
            updatedAt = System.currentTimeMillis()
        )
        UploadServiceState.publish(this, sendingState)
        notifyProgress(sendingState)

        val resultCode = try {
            RustUploader.performUpload(
                filePath = filePath,
                serverIp = serverIp,
                udpPort = udpPort,
                httpPort = httpPort,
                blockSize = blockSize,
                password = password,
                onProgress = { progress ->
                    val now = System.currentTimeMillis()
                    val state = BackgroundUploadState(
                        transferId = currentTransferId,
                        running = true,
                        filePath = filePath,
                        fileName = file.name,
                        serverIp = serverIp,
                        bytesTransferred = progress.bytesTransferred,
                        totalBytes = progress.totalBytes,
                        bytesPerSecond = progress.bytesPerSecond,
                        statusText = "Đang gửi ${file.name}…",
                        resultCode = null,
                        fileHash = fileHash,
                        updatedAt = now
                    )
                    val shouldPersist = now - lastPersistAt >= 1_000L
                    if (shouldPersist) lastPersistAt = now
                    UploadServiceState.publish(this, state, persist = shouldPersist)
                    if (now - lastNotificationAt >= 500L) {
                        lastNotificationAt = now
                        notifyProgress(state)
                    }
                }
            )
        } catch (t: Throwable) {
            Log.e(TAG, "Foreground upload crashed", t)
            -97
        }

        finishUpload(startId, filePath, serverIp, fileHash, resultCode)
    }

    private fun finishUpload(
        startId: Int,
        filePath: String,
        serverIp: String,
        fileHash: String,
        resultCode: Int
    ) {
        val file = File(filePath)
        val success = resultCode == 0
        val current = UploadServiceState.state.value
        val finishedState = current.copy(
            running = false,
            filePath = filePath,
            fileName = file.name.ifBlank { current.fileName },
            serverIp = serverIp,
            bytesTransferred = if (success) maxOf(current.totalBytes, file.length()) else current.bytesTransferred,
            totalBytes = maxOf(current.totalBytes, file.length()),
            bytesPerSecond = 0L,
            statusText = if (success) "Gửi file thành công" else serviceErrorMessage(resultCode),
            resultCode = resultCode,
            fileHash = fileHash,
            updatedAt = System.currentTimeMillis()
        )
        appendHistory(finishedState)
        UploadServiceState.publish(this, finishedState)
        if (success) {
            cleanupPendingUpload(file)

            val manager = getSystemService(NotificationManager::class.java)
            manager.notify(NOTIFICATION_ID, buildCompletedNotification(finishedState))
        } else {
            val manager = getSystemService(NotificationManager::class.java)
            manager.notify(NOTIFICATION_ID, buildFailedNotification(finishedState))
        }

        ServiceCompat.stopForeground(this, ServiceCompat.STOP_FOREGROUND_DETACH)
        stopSelf(startId)
    }

    private fun appendHistory(state: BackgroundUploadState) {
        val item = UploadHistoryItem(
            fileName = state.fileName,
            filePath = state.filePath,
            fileSize = state.totalBytes,
            timestamp = System.currentTimeMillis(),
            sha256 = state.fileHash,
            isSuccess = state.resultCode == 0,
            statusMsg = state.statusText
        )
        val history = listOf(item) + loadUploadHistory(this)
        saveUploadHistory(this, history)
    }

    private fun cleanupPendingUpload(file: File) {
        try {
            val pendingRoot = File(filesDir, "pending_uploads").canonicalFile
            val candidate = file.canonicalFile
            if (candidate.path.startsWith(pendingRoot.path + File.separator)) {
                candidate.parentFile?.deleteRecursively()
            }
        } catch (t: Throwable) {
            Log.w(TAG, "Could not remove temporary upload copy: ${t.message}")
        }
    }

    private fun startAsDataSyncForeground(notification: Notification) {
        val type = if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.Q) {
            ServiceInfo.FOREGROUND_SERVICE_TYPE_DATA_SYNC
        } else {
            0
        }
        ServiceCompat.startForeground(this, NOTIFICATION_ID, notification, type)
    }

    private fun notifyProgress(state: BackgroundUploadState) {
        getSystemService(NotificationManager::class.java)
            .notify(NOTIFICATION_ID, buildNotification(state))
    }

    private fun buildNotification(state: BackgroundUploadState): Notification {
        val progress = state.progress
        val hasTotal = progress.totalBytes > 0L
        val title = if (state.fileName.isBlank()) "Đang gửi file" else "Đang gửi ${state.fileName}"
        val detail = if (hasTotal) {
            "${progress.percent}% · ${formatTransferRate(progress.bytesPerSecond)} · ${formatEta(progress.etaSeconds)}"
        } else {
            "Đang kết nối tới ${state.serverIp}"
        }

        return NotificationCompat.Builder(this, CHANNEL_ID)
            .setSmallIcon(R.drawable.ic_stat_upload)
            .setContentTitle(title)
            .setContentText(detail)
            .setOnlyAlertOnce(true)
            .setOngoing(true)
            .setCategory(NotificationCompat.CATEGORY_PROGRESS)
            .setPriority(NotificationCompat.PRIORITY_LOW)
            .setContentIntent(openAppPendingIntent())
            .setProgress(
                100,
                if (hasTotal) progress.percent else 0,
                !hasTotal
            )
            .build()
    }

    private fun buildCompletedNotification(state: BackgroundUploadState): Notification {
        return NotificationCompat.Builder(this, CHANNEL_ID)
            .setSmallIcon(R.drawable.ic_stat_upload)
            .setContentTitle("Gửi file thành công")
            .setContentText(state.fileName)
            .setAutoCancel(true)
            .setOngoing(false)
            .setPriority(NotificationCompat.PRIORITY_DEFAULT)
            .setContentIntent(openAppPendingIntent())
            .build()
    }

    private fun buildFailedNotification(state: BackgroundUploadState): Notification {
        return NotificationCompat.Builder(this, CHANNEL_ID)
            .setSmallIcon(R.drawable.ic_stat_upload)
            .setContentTitle("Gửi file thất bại")
            .setContentText(state.statusText)
            .setAutoCancel(true)
            .setOngoing(false)
            .setPriority(NotificationCompat.PRIORITY_DEFAULT)
            .setContentIntent(openAppPendingIntent())
            .build()
    }

    private fun openAppPendingIntent(): PendingIntent {
        val intent = Intent(this, MainActivity::class.java).apply {
            flags = Intent.FLAG_ACTIVITY_SINGLE_TOP or Intent.FLAG_ACTIVITY_CLEAR_TOP
        }
        return PendingIntent.getActivity(
            this,
            0,
            intent,
            PendingIntent.FLAG_UPDATE_CURRENT or PendingIntent.FLAG_IMMUTABLE
        )
    }

    private fun createNotificationChannel() {
        if (Build.VERSION.SDK_INT < Build.VERSION_CODES.O) return
        val manager = getSystemService(NotificationManager::class.java)
        val channel = NotificationChannel(
            CHANNEL_ID,
            CHANNEL_NAME,
            NotificationManager.IMPORTANCE_LOW
        ).apply {
            description = "Hiển thị tiến độ gửi file khi ứng dụng chạy nền"
            setShowBadge(false)
        }
        manager.createNotificationChannel(channel)
    }

    private fun serviceErrorMessage(code: Int): String = when (code) {
        -1 -> "Cấu hình truyền không hợp lệ"
        -2 -> "Không tìm thấy hoặc không đọc được file"
        -3 -> "Không thể tính mã băm của file"
        -4 -> "Không thể đăng ký phiên upload với server"
        -5 -> "Không phân giải được địa chỉ server"
        -6 -> "Không thể mở socket UDP cục bộ"
        -7 -> "Kết nối UDP hết thời gian chờ"
        -97 -> "Lỗi ngoài dự kiến khi gọi thư viện Rust"
        -98 -> "Không tải được thư viện native client_lib"
        -99 -> "Thư viện native/JNA gặp lỗi"
        else -> "Lỗi không xác định (mã $code)"
    }

    override fun onDestroy() {
        uploadJob?.cancel()
        serviceScope.cancel()
        super.onDestroy()
    }

    override fun onBind(intent: Intent?): IBinder? = null
}
