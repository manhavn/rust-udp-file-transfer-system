package com.filetransfersystem

import android.content.ClipData
import android.Manifest
import android.content.ClipboardManager
import android.content.Context
import android.content.pm.PackageManager
import android.net.Uri
import android.os.Build
import android.os.Bundle
import android.provider.OpenableColumns
import android.util.Log
import android.widget.Toast
import androidx.activity.ComponentActivity
import androidx.activity.compose.rememberLauncherForActivityResult
import androidx.activity.compose.setContent
import androidx.activity.result.contract.ActivityResultContracts
import androidx.compose.animation.AnimatedVisibility
import androidx.compose.foundation.BorderStroke
import androidx.compose.foundation.isSystemInDarkTheme
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.PaddingValues
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.items
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.foundation.text.KeyboardOptions
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.filled.CheckCircle
import androidx.compose.material.icons.filled.CloudUpload
import androidx.compose.material.icons.filled.ContentCopy
import androidx.compose.material.icons.filled.DeleteOutline
import androidx.compose.material.icons.filled.Error
import androidx.compose.material.icons.filled.ExpandLess
import androidx.compose.material.icons.filled.ExpandMore
import androidx.compose.material.icons.filled.FolderOpen
import androidx.compose.material.icons.filled.History
import androidx.compose.material.icons.filled.InsertDriveFile
import androidx.compose.material.icons.filled.Lock
import androidx.compose.material.icons.filled.Send
import androidx.compose.material.icons.filled.Settings
import androidx.compose.material.icons.filled.Visibility
import androidx.compose.material.icons.filled.VisibilityOff
import androidx.compose.material3.Button
import androidx.compose.material3.CardDefaults
import androidx.compose.material3.CenterAlignedTopAppBar
import androidx.compose.material3.ElevatedCard
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.FilledTonalButton
import androidx.compose.material3.Divider
import androidx.compose.material3.Icon
import androidx.compose.material3.IconButton
import androidx.compose.material3.LinearProgressIndicator
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.NavigationBar
import androidx.compose.material3.NavigationBarItem
import androidx.compose.material3.OutlinedButton
import androidx.compose.material3.OutlinedTextField
import androidx.compose.material3.Scaffold
import androidx.compose.material3.Surface
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.material3.darkColorScheme
import androidx.compose.material3.lightColorScheme
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.collectAsState
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.runtime.saveable.rememberSaveable
import androidx.compose.runtime.saveable.rememberSaveableStateHolder
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.text.font.FontFamily
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.input.ImeAction
import androidx.compose.ui.text.input.KeyboardType
import androidx.compose.ui.text.input.PasswordVisualTransformation
import androidx.compose.ui.text.input.VisualTransformation
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import androidx.core.content.ContextCompat
import androidx.lifecycle.lifecycleScope
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.flow.filter
import kotlinx.coroutines.flow.first
import kotlinx.coroutines.flow.onEach
import kotlinx.coroutines.launch
import kotlinx.coroutines.withContext
import org.json.JSONArray
import org.json.JSONObject
import java.io.File
import java.io.FileOutputStream
import java.text.SimpleDateFormat
import java.util.Date
import java.util.Locale

private const val PREFS_NAME = "udp_transfer_prefs"
private const val DEFAULT_UDP_PORT = 5000
private const val DEFAULT_HTTP_PORT = 8080
private const val DEFAULT_BLOCK_SIZE = 16384

data class UploadHistoryItem(
    val fileName: String,
    val filePath: String,
    val fileSize: Long,
    val timestamp: Long,
    val sha256: String,
    val isSuccess: Boolean,
    val statusMsg: String
)

data class UploadConfig(
    val filePath: String,
    val serverIp: String,
    val udpPort: Int,
    val httpPort: Int,
    val blockSize: Int,
    val password: String?
)

private enum class AppSection {
    TRANSFER,
    HISTORY
}

private enum class UploadStatusTone {
    IDLE,
    RUNNING,
    SUCCESS,
    ERROR
}

fun loadUploadHistory(context: Context): List<UploadHistoryItem> {
    val prefs = context.getSharedPreferences(PREFS_NAME, Context.MODE_PRIVATE)
    val historyJson = prefs.getString("upload_history", "[]") ?: "[]"
    val list = mutableListOf<UploadHistoryItem>()
    try {
        val jsonArray = JSONArray(historyJson)
        for (i in 0 until jsonArray.length()) {
            val obj = jsonArray.getJSONObject(i)
            list.add(
                UploadHistoryItem(
                    fileName = obj.optString("fileName", ""),
                    filePath = obj.optString("filePath", ""),
                    fileSize = obj.optLong("fileSize", 0),
                    timestamp = obj.optLong("timestamp", 0),
                    sha256 = obj.optString("sha256", ""),
                    isSuccess = obj.optBoolean("isSuccess", false),
                    statusMsg = obj.optString("statusMsg", "")
                )
            )
        }
    } catch (e: Exception) {
        Log.e("UploadHistory", "Error parsing upload history", e)
    }
    return list.sortedByDescending { it.timestamp }
}

fun saveUploadHistory(context: Context, history: List<UploadHistoryItem>) {
    val prefs = context.getSharedPreferences(PREFS_NAME, Context.MODE_PRIVATE)
    try {
        val jsonArray = JSONArray()
        history.take(50).forEach { item ->
            val obj = JSONObject().apply {
                put("fileName", item.fileName)
                put("filePath", item.filePath)
                put("fileSize", item.fileSize)
                put("timestamp", item.timestamp)
                put("sha256", item.sha256)
                put("isSuccess", item.isSuccess)
                put("statusMsg", item.statusMsg)
            }
            jsonArray.put(obj)
        }
        prefs.edit().putString("upload_history", jsonArray.toString()).apply()
    } catch (e: Exception) {
        Log.e("UploadHistory", "Error saving upload history", e)
    }
}

fun formatFileSize(size: Long): String {
    if (size <= 0) return "0 B"
    val units = arrayOf("B", "KB", "MB", "GB", "TB")
    val digitGroups = (Math.log10(size.toDouble()) / Math.log10(1024.0)).toInt()
        .coerceIn(0, units.lastIndex)
    return String.format(
        Locale.US,
        "%.2f %s",
        size / Math.pow(1024.0, digitGroups.toDouble()),
        units[digitGroups]
    )
}

fun formatTimestamp(timestamp: Long): String {
    val sdf = SimpleDateFormat("dd/MM/yyyy · HH:mm", Locale.getDefault())
    return sdf.format(Date(timestamp))
}

fun formatTransferRate(bytesPerSecond: Long): String {
    return if (bytesPerSecond <= 0L) "Đang đo tốc độ…" else "${formatFileSize(bytesPerSecond)}/s"
}

fun formatEta(seconds: Long?): String = when {
    seconds == null -> "ETA đang tính…"
    seconds <= 0L -> "Sắp hoàn tất"
    seconds < 60L -> "Còn ~${seconds}s"
    seconds < 3600L -> "Còn ~${seconds / 60} phút ${seconds % 60}s"
    else -> "Còn ~${seconds / 3600} giờ ${(seconds % 3600) / 60} phút"
}


class MainActivity : ComponentActivity() {
    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        UploadServiceState.restore(this)

        setContent {
            UdpTransferTheme {
                val context = this@MainActivity
                var uploadHistory by remember { mutableStateOf(loadUploadHistory(context)) }
                val foregroundState by UploadServiceState.state.collectAsState()

                LaunchedEffect(foregroundState.updatedAt, foregroundState.running) {
                    if (!foregroundState.running && foregroundState.resultCode != null) {
                        uploadHistory = loadUploadHistory(context)
                    }
                }

                UdpUploadApp(
                    uploadHistory = uploadHistory,
                    onClearHistory = {
                        uploadHistory = emptyList()
                        saveUploadHistory(context, emptyList())
                    },
                    onStartBackgroundUpload = { config, onStatusChange, onProgress, onFinished ->
                        lifecycleScope.launch {
                            try {
                                ensureUploadNotificationPermission()
                                onStatusChange("Đang khởi chạy tiến trình gửi nền…")
                                val transferId = UploadForegroundService.start(applicationContext, config)

                                val finalState = UploadServiceState.state
                                    .filter { it.transferId == transferId }
                                    .onEach { state ->
                                        onStatusChange(state.statusText)
                                        if (state.totalBytes > 0L) {
                                            onProgress(state.progress)
                                        }
                                    }
                                    .first { state -> !state.running && state.resultCode != null }

                                onFinished(finalState.resultCode == 0)
                            } catch (t: Throwable) {
                                Log.e("MainActivity", "Foreground upload start failed", t)
                                onStatusChange(t.message ?: "Không thể khởi chạy tiến trình gửi nền")
                                onFinished(false)
                            }
                        }
                    }
                )
            }
        }
    }

    private fun ensureUploadNotificationPermission() {
        if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.TIRAMISU &&
            ContextCompat.checkSelfPermission(this, Manifest.permission.POST_NOTIFICATIONS) != PackageManager.PERMISSION_GRANTED
        ) {
            requestPermissions(arrayOf(Manifest.permission.POST_NOTIFICATIONS), 2107)
        }
    }
}

@Composable
private fun UdpTransferTheme(content: @Composable () -> Unit) {
    val dark = isSystemInDarkTheme()
    val scheme = if (dark) {
        darkColorScheme(
            primary = Color(0xFFA6C8FF),
            secondary = Color(0xFFB7CCBE),
            background = Color(0xFF101418),
            surface = Color(0xFF171C20),
            surfaceVariant = Color(0xFF41484D),
            error = Color(0xFFFFB4AB)
        )
    } else {
        lightColorScheme(
            primary = Color(0xFF245E91),
            secondary = Color(0xFF466554),
            background = Color(0xFFF7F9FC),
            surface = Color(0xFFFFFFFF),
            surfaceVariant = Color(0xFFDDE3EA),
            error = Color(0xFFBA1A1A)
        )
    }
    MaterialTheme(colorScheme = scheme, content = content)
}

@OptIn(ExperimentalMaterial3Api::class)
@Composable
private fun UdpUploadApp(
    uploadHistory: List<UploadHistoryItem>,
    onClearHistory: () -> Unit,
    onStartBackgroundUpload: (UploadConfig, (String) -> Unit, (UploadProgress) -> Unit, (Boolean) -> Unit) -> Unit
) {
    var section by rememberSaveable { mutableStateOf(AppSection.TRANSFER) }
    val stateHolder = rememberSaveableStateHolder()

    Scaffold(
        topBar = {
            CenterAlignedTopAppBar(
                title = {
                    Column(horizontalAlignment = Alignment.CenterHorizontally) {
                        Text("UDP File Transfer", fontWeight = FontWeight.SemiBold)
                        Text(
                            "Gửi file nhanh tới server của bạn",
                            style = MaterialTheme.typography.labelSmall,
                            color = MaterialTheme.colorScheme.onSurfaceVariant
                        )
                    }
                }
            )
        },
        bottomBar = {
            NavigationBar {
                NavigationBarItem(
                    selected = section == AppSection.TRANSFER,
                    onClick = { section = AppSection.TRANSFER },
                    icon = { Icon(Icons.Default.Send, contentDescription = null) },
                    label = { Text("Gửi file") }
                )
                NavigationBarItem(
                    selected = section == AppSection.HISTORY,
                    onClick = { section = AppSection.HISTORY },
                    icon = { Icon(Icons.Default.History, contentDescription = null) },
                    label = { Text("Lịch sử") }
                )
            }
        }
    ) { innerPadding ->
        stateHolder.SaveableStateProvider(section.name) {
            when (section) {
                AppSection.TRANSFER -> TransferScreen(
                    modifier = Modifier.padding(innerPadding),
                    onStartBackgroundUpload = onStartBackgroundUpload
                )

                AppSection.HISTORY -> HistoryScreen(
                    modifier = Modifier.padding(innerPadding),
                    uploadHistory = uploadHistory,
                    onClearHistory = onClearHistory
                )
            }
        }
    }
}

@OptIn(ExperimentalMaterial3Api::class)
@Composable
private fun TransferScreen(
    modifier: Modifier = Modifier,
    onStartBackgroundUpload: (UploadConfig, (String) -> Unit, (UploadProgress) -> Unit, (Boolean) -> Unit) -> Unit
) {
    val context = androidx.compose.ui.platform.LocalContext.current
    val scope = rememberCoroutineScope()
    val prefs = remember { context.getSharedPreferences(PREFS_NAME, Context.MODE_PRIVATE) }

    var filePath by rememberSaveable { mutableStateOf("") }
    var fileName by rememberSaveable { mutableStateOf("") }
    var fileSize by rememberSaveable { mutableStateOf(0L) }
    var serverIp by rememberSaveable { mutableStateOf(prefs.getString("server_ip", "") ?: "") }
    var udpPortStr by rememberSaveable { mutableStateOf(prefs.getString("udp_port", DEFAULT_UDP_PORT.toString()) ?: DEFAULT_UDP_PORT.toString()) }
    var httpPortStr by rememberSaveable { mutableStateOf(prefs.getString("http_port", DEFAULT_HTTP_PORT.toString()) ?: DEFAULT_HTTP_PORT.toString()) }
    var blockSizeStr by rememberSaveable { mutableStateOf(prefs.getString("block_size", DEFAULT_BLOCK_SIZE.toString()) ?: DEFAULT_BLOCK_SIZE.toString()) }
    var password by rememberSaveable { mutableStateOf(prefs.getString("password", "") ?: "") }
    var showAdvanced by rememberSaveable { mutableStateOf(false) }
    var showDemoCreator by rememberSaveable { mutableStateOf(false) }
    var demoFileName by rememberSaveable { mutableStateOf("demo_data.txt") }
    var demoContent by rememberSaveable { mutableStateOf("Nội dung file thử nghiệm UDP.") }
    var isPasswordVisible by rememberSaveable { mutableStateOf(false) }
    var isPreparingFile by rememberSaveable { mutableStateOf(false) }
    var isUploading by rememberSaveable { mutableStateOf(false) }
    var statusText by rememberSaveable { mutableStateOf("Sẵn sàng. Chọn file để bắt đầu.") }
    var statusTone by rememberSaveable { mutableStateOf(UploadStatusTone.IDLE) }
    var progressBytes by rememberSaveable { mutableStateOf(0L) }
    var progressTotal by rememberSaveable { mutableStateOf(0L) }
    var progressBps by rememberSaveable { mutableStateOf(0L) }
    var hasTransferProgress by rememberSaveable { mutableStateOf(false) }
    var validationMessage by rememberSaveable { mutableStateOf<String?>(null) }
    val foregroundState by UploadServiceState.state.collectAsState()

    LaunchedEffect(foregroundState.updatedAt) {
        if (foregroundState.running) {
            if (foregroundState.filePath.isNotBlank()) {
                val activeFile = File(foregroundState.filePath)
                filePath = foregroundState.filePath
                fileName = foregroundState.fileName.ifBlank { activeFile.name }
                fileSize = foregroundState.totalBytes.takeIf { it > 0L } ?: activeFile.length()
            }
            isUploading = true
            statusText = foregroundState.statusText
            statusTone = UploadStatusTone.RUNNING
            if (foregroundState.totalBytes > 0L) {
                progressBytes = foregroundState.bytesTransferred
                progressTotal = foregroundState.totalBytes
                progressBps = foregroundState.bytesPerSecond
                hasTransferProgress = true
            }
        } else if (isUploading && foregroundState.filePath == filePath && foregroundState.resultCode != null) {
            isUploading = false
            statusText = foregroundState.statusText
            statusTone = if (foregroundState.resultCode == 0) UploadStatusTone.SUCCESS else UploadStatusTone.ERROR
            if (foregroundState.totalBytes > 0L) {
                progressBytes = foregroundState.bytesTransferred
                progressTotal = foregroundState.totalBytes
                progressBps = 0L
                hasTransferProgress = true
            }
        }
    }

    fun updateSelectedFile(file: File) {
        filePath = file.absolutePath
        fileName = file.name
        fileSize = file.length()
        statusText = "Đã chọn ${file.name}"
        statusTone = UploadStatusTone.IDLE
        validationMessage = null
    }

    val filePickerLauncher = rememberLauncherForActivityResult(
        contract = ActivityResultContracts.OpenDocument(),
        onResult = { uri ->
            if (uri != null) {
                scope.launch {
                    isPreparingFile = true
                    statusText = "Đang chuẩn bị file…"
                    statusTone = UploadStatusTone.RUNNING
                    val resolvedName = getFileNameFromUri(context, uri)
                    val cachedFile = withContext(Dispatchers.IO) {
                        copyUriToPendingUpload(context, uri, resolvedName)
                    }
                    isPreparingFile = false
                    if (cachedFile != null) {
                        updateSelectedFile(cachedFile)
                    } else {
                        statusText = "Không thể đọc file đã chọn"
                        statusTone = UploadStatusTone.ERROR
                    }
                }
            }
        }
    )

    fun validateAndBuildConfig(): UploadConfig? {
        val selectedFile = File(filePath)
        val udpPort = udpPortStr.toIntOrNull()
        val httpPort = httpPortStr.toIntOrNull()
        val blockSize = blockSizeStr.toIntOrNull()

        val error = when {
            filePath.isBlank() || !selectedFile.exists() -> "Hãy chọn file cần gửi"
            serverIp.trim().isBlank() -> "Hãy nhập địa chỉ IP hoặc hostname của server"
            udpPort == null || udpPort !in 1..65535 -> "Cổng UDP phải nằm trong khoảng 1–65535"
            httpPort == null || httpPort !in 1..65535 -> "Cổng HTTP phải nằm trong khoảng 1–65535"
            blockSize == null || blockSize !in 512..1_048_576 -> "Block size nên nằm trong khoảng 512 B–1 MB"
            else -> null
        }

        validationMessage = error
        if (error != null) return null

        return UploadConfig(
            filePath = filePath,
            serverIp = serverIp.trim(),
            udpPort = udpPort!!,
            httpPort = httpPort!!,
            blockSize = blockSize!!,
            password = password.takeIf { it.isNotBlank() }
        )
    }

    fun startUpload() {
        val config = validateAndBuildConfig() ?: return
        prefs.edit()
            .putString("server_ip", config.serverIp)
            .putString("udp_port", config.udpPort.toString())
            .putString("http_port", config.httpPort.toString())
            .putString("block_size", config.blockSize.toString())
            .putString("password", password)
            .apply()

        isUploading = true
        statusTone = UploadStatusTone.RUNNING
        validationMessage = null
        progressBytes = 0L
        progressTotal = fileSize
        progressBps = 0L
        hasTransferProgress = false

        val statusCallback: (String) -> Unit = { message ->
            statusText = message
        }
        val progressCallback: (UploadProgress) -> Unit = { progress ->
            progressBytes = progress.bytesTransferred
            progressTotal = progress.totalBytes
            progressBps = progress.bytesPerSecond
            hasTransferProgress = progress.totalBytes > 0L
        }
        val finishedCallback: (Boolean) -> Unit = { success ->
            isUploading = false
            statusTone = if (success) UploadStatusTone.SUCCESS else UploadStatusTone.ERROR
        }

        onStartBackgroundUpload(config, statusCallback, progressCallback, finishedCallback)
    }

    LazyColumn(
        modifier = modifier.fillMaxSize(),
        contentPadding = PaddingValues(horizontal = 16.dp, vertical = 16.dp),
        verticalArrangement = Arrangement.spacedBy(14.dp)
    ) {
        item {
            StatusCard(
                statusText = statusText,
                tone = statusTone,
                showProgress = isUploading || isPreparingFile || (hasTransferProgress && statusTone == UploadStatusTone.SUCCESS),
                progress = if (hasTransferProgress) {
                    UploadProgress(progressBytes, progressTotal, progressBps)
                } else {
                    null
                }
            )
        }

        item {
            SectionTitle(
                title = "File cần gửi",
                description = "Chọn file từ thiết bị. Ứng dụng sẽ chuẩn bị một bản tạm an toàn để truyền."
            )
        }

        item {
            FilePickerCard(
                fileName = fileName,
                filePath = filePath,
                fileSize = fileSize,
                enabled = !isUploading && !isPreparingFile,
                onPickFile = { filePickerLauncher.launch(arrayOf("*/*")) }
            )
        }

        item {
            TextButton(
                onClick = { showDemoCreator = !showDemoCreator },
                enabled = !isUploading,
                contentPadding = PaddingValues(horizontal = 4.dp)
            ) {
                Icon(
                    imageVector = if (showDemoCreator) Icons.Default.ExpandLess else Icons.Default.ExpandMore,
                    contentDescription = null,
                    modifier = Modifier.size(18.dp)
                )
                Spacer(modifier = Modifier.size(6.dp))
                Text(if (showDemoCreator) "Ẩn công cụ tạo file thử" else "Tạo nhanh một file thử")
            }
        }

        item {
            AnimatedVisibility(visible = showDemoCreator) {
                ElevatedCard(modifier = Modifier.fillMaxWidth()) {
                    Column(
                        modifier = Modifier.padding(16.dp),
                        verticalArrangement = Arrangement.spacedBy(10.dp)
                    ) {
                        OutlinedTextField(
                            value = demoFileName,
                            onValueChange = { demoFileName = it },
                            modifier = Modifier.fillMaxWidth(),
                            label = { Text("Tên file thử") },
                            singleLine = true,
                            enabled = !isUploading
                        )
                        OutlinedTextField(
                            value = demoContent,
                            onValueChange = { demoContent = it },
                            modifier = Modifier.fillMaxWidth(),
                            label = { Text("Nội dung") },
                            minLines = 3,
                            enabled = !isUploading
                        )
                        FilledTonalButton(
                            onClick = {
                                val safeName = demoFileName.trim().ifBlank { "demo_data.txt" }.replace('/', '_').replace('\\', '_')
                                try {
                                    val demoFile = File(context.cacheDir, safeName)
                                    FileOutputStream(demoFile).use { output ->
                                        output.write(demoContent.toByteArray(Charsets.UTF_8))
                                    }
                                    updateSelectedFile(demoFile)
                                } catch (e: Exception) {
                                    statusText = "Không thể tạo file thử: ${e.message ?: "lỗi không xác định"}"
                                    statusTone = UploadStatusTone.ERROR
                                }
                            },
                            enabled = !isUploading
                        ) {
                            Icon(Icons.Default.InsertDriveFile, contentDescription = null)
                            Spacer(modifier = Modifier.size(8.dp))
                            Text("Tạo và chọn file này")
                        }
                    }
                }
            }
        }

        item {
            SectionTitle(
                title = "Server",
                description = "Nhập địa chỉ máy đang chạy UDP File Transfer Server."
            )
        }

        item {
            ElevatedCard(modifier = Modifier.fillMaxWidth()) {
                Column(
                    modifier = Modifier.padding(16.dp),
                    verticalArrangement = Arrangement.spacedBy(12.dp)
                ) {
                    OutlinedTextField(
                        value = serverIp,
                        onValueChange = {
                            serverIp = it
                            validationMessage = null
                        },
                        modifier = Modifier.fillMaxWidth(),
                        label = { Text("IP / hostname server") },
                        placeholder = { Text("Ví dụ: 192.168.1.100") },
                        singleLine = true,
                        enabled = !isUploading,
                        keyboardOptions = KeyboardOptions(imeAction = ImeAction.Next)
                    )

                    OutlinedTextField(
                        value = password,
                        onValueChange = { password = it },
                        modifier = Modifier.fillMaxWidth(),
                        label = { Text("Mật khẩu server (nếu có)") },
                        singleLine = true,
                        enabled = !isUploading,
                        visualTransformation = if (isPasswordVisible) {
                            VisualTransformation.None
                        } else {
                            PasswordVisualTransformation()
                        },
                        leadingIcon = { Icon(Icons.Default.Lock, contentDescription = null) },
                        trailingIcon = {
                            IconButton(onClick = { isPasswordVisible = !isPasswordVisible }) {
                                Icon(
                                    imageVector = if (isPasswordVisible) Icons.Default.VisibilityOff else Icons.Default.Visibility,
                                    contentDescription = if (isPasswordVisible) "Ẩn mật khẩu" else "Hiện mật khẩu"
                                )
                            }
                        }
                    )

                    Divider()

                    Row(
                        modifier = Modifier.fillMaxWidth(),
                        verticalAlignment = Alignment.CenterVertically,
                        horizontalArrangement = Arrangement.spacedBy(12.dp)
                    ) {
                        Icon(
                            Icons.Default.CloudUpload,
                            contentDescription = null,
                            tint = MaterialTheme.colorScheme.primary
                        )
                        Column(modifier = Modifier.weight(1f)) {
                            Text("Tiếp tục gửi khi chuyển ứng dụng", fontWeight = FontWeight.Medium)
                            Text(
                                "Luôn bật. Tiến trình tiếp tục trong Foreground Service và hiển thị trên thông báo hệ thống.",
                                style = MaterialTheme.typography.bodySmall,
                                color = MaterialTheme.colorScheme.onSurfaceVariant
                            )
                        }
                    }
                }
            }
        }

        item {
            TextButton(
                onClick = { showAdvanced = !showAdvanced },
                enabled = !isUploading,
                contentPadding = PaddingValues(horizontal = 4.dp)
            ) {
                Icon(Icons.Default.Settings, contentDescription = null, modifier = Modifier.size(18.dp))
                Spacer(modifier = Modifier.size(6.dp))
                Text(if (showAdvanced) "Ẩn cài đặt nâng cao" else "Cài đặt nâng cao")
                Spacer(modifier = Modifier.size(4.dp))
                Icon(
                    imageVector = if (showAdvanced) Icons.Default.ExpandLess else Icons.Default.ExpandMore,
                    contentDescription = null,
                    modifier = Modifier.size(18.dp)
                )
            }
        }

        item {
            AnimatedVisibility(visible = showAdvanced) {
                ElevatedCard(modifier = Modifier.fillMaxWidth()) {
                    Column(
                        modifier = Modifier.padding(16.dp),
                        verticalArrangement = Arrangement.spacedBy(12.dp)
                    ) {
                        Row(
                            modifier = Modifier.fillMaxWidth(),
                            horizontalArrangement = Arrangement.spacedBy(10.dp)
                        ) {
                            OutlinedTextField(
                                value = udpPortStr,
                                onValueChange = { udpPortStr = it.filter(Char::isDigit) },
                                label = { Text("UDP port") },
                                modifier = Modifier.weight(1f),
                                singleLine = true,
                                enabled = !isUploading,
                                keyboardOptions = KeyboardOptions(keyboardType = KeyboardType.Number)
                            )
                            OutlinedTextField(
                                value = httpPortStr,
                                onValueChange = { httpPortStr = it.filter(Char::isDigit) },
                                label = { Text("HTTP port") },
                                modifier = Modifier.weight(1f),
                                singleLine = true,
                                enabled = !isUploading,
                                keyboardOptions = KeyboardOptions(keyboardType = KeyboardType.Number)
                            )
                        }
                        OutlinedTextField(
                            value = blockSizeStr,
                            onValueChange = { blockSizeStr = it.filter(Char::isDigit) },
                            label = { Text("Block size (bytes)") },
                            supportingText = { Text("Mặc định: $DEFAULT_BLOCK_SIZE bytes") },
                            modifier = Modifier.fillMaxWidth(),
                            singleLine = true,
                            enabled = !isUploading,
                            keyboardOptions = KeyboardOptions(keyboardType = KeyboardType.Number)
                        )
                        OutlinedButton(
                            onClick = {
                                udpPortStr = DEFAULT_UDP_PORT.toString()
                                httpPortStr = DEFAULT_HTTP_PORT.toString()
                                blockSizeStr = DEFAULT_BLOCK_SIZE.toString()
                            },
                            enabled = !isUploading
                        ) {
                            Text("Khôi phục mặc định")
                        }
                    }
                }
            }
        }

        if (validationMessage != null) {
            item {
                Surface(
                    modifier = Modifier.fillMaxWidth(),
                    color = MaterialTheme.colorScheme.errorContainer,
                    contentColor = MaterialTheme.colorScheme.onErrorContainer,
                    shape = RoundedCornerShape(12.dp)
                ) {
                    Row(
                        modifier = Modifier.padding(12.dp),
                        verticalAlignment = Alignment.CenterVertically
                    ) {
                        Icon(Icons.Default.Error, contentDescription = null)
                        Spacer(modifier = Modifier.size(8.dp))
                        Text(validationMessage.orEmpty(), style = MaterialTheme.typography.bodyMedium)
                    }
                }
            }
        }

        item {
            Button(
                onClick = { startUpload() },
                enabled = !isUploading && !isPreparingFile,
                modifier = Modifier
                    .fillMaxWidth()
                    .height(54.dp)
            ) {
                Icon(Icons.Default.CloudUpload, contentDescription = null)
                Spacer(modifier = Modifier.size(8.dp))
                Text(
                    text = if (isUploading) "ĐANG GỬI…" else "GỬI FILE",
                    fontSize = 16.sp,
                    fontWeight = FontWeight.Bold
                )
            }
        }

        item {
            Text(
                text = "Chạy nền luôn bật · UDP ${udpPortStr.ifBlank { DEFAULT_UDP_PORT.toString() }} · HTTP ${httpPortStr.ifBlank { DEFAULT_HTTP_PORT.toString() }}",
                modifier = Modifier.fillMaxWidth(),
                style = MaterialTheme.typography.bodySmall,
                color = MaterialTheme.colorScheme.onSurfaceVariant
            )
        }
    }
}

@Composable
private fun SectionTitle(title: String, description: String) {
    Column(verticalArrangement = Arrangement.spacedBy(2.dp)) {
        Text(
            text = title,
            style = MaterialTheme.typography.titleMedium,
            fontWeight = FontWeight.SemiBold
        )
        Text(
            text = description,
            style = MaterialTheme.typography.bodySmall,
            color = MaterialTheme.colorScheme.onSurfaceVariant
        )
    }
}

@Composable
private fun StatusCard(
    statusText: String,
    tone: UploadStatusTone,
    showProgress: Boolean,
    progress: UploadProgress?
) {
    val (container, content, icon) = when (tone) {
        UploadStatusTone.SUCCESS -> Triple(
            MaterialTheme.colorScheme.primaryContainer,
            MaterialTheme.colorScheme.onPrimaryContainer,
            Icons.Default.CheckCircle
        )

        UploadStatusTone.ERROR -> Triple(
            MaterialTheme.colorScheme.errorContainer,
            MaterialTheme.colorScheme.onErrorContainer,
            Icons.Default.Error
        )

        UploadStatusTone.RUNNING -> Triple(
            MaterialTheme.colorScheme.secondaryContainer,
            MaterialTheme.colorScheme.onSecondaryContainer,
            Icons.Default.CloudUpload
        )

        UploadStatusTone.IDLE -> Triple(
            MaterialTheme.colorScheme.surfaceVariant,
            MaterialTheme.colorScheme.onSurfaceVariant,
            Icons.Default.CloudUpload
        )
    }

    Surface(
        modifier = Modifier.fillMaxWidth(),
        color = container,
        contentColor = content,
        shape = RoundedCornerShape(16.dp)
    ) {
        Column {
            Row(
                modifier = Modifier.padding(horizontal = 16.dp, vertical = 14.dp),
                verticalAlignment = Alignment.CenterVertically
            ) {
                Icon(icon, contentDescription = null)
                Spacer(modifier = Modifier.size(10.dp))
                Column(modifier = Modifier.weight(1f)) {
                    Text("Trạng thái", style = MaterialTheme.typography.labelMedium, fontWeight = FontWeight.SemiBold)
                    Text(statusText, style = MaterialTheme.typography.bodyMedium)
                }
            }
            if (showProgress) {
                if (progress != null) {
                    LinearProgressIndicator(
                        progress = progress.fraction,
                        modifier = Modifier.fillMaxWidth()
                    )
                    Column(
                        modifier = Modifier.padding(horizontal = 16.dp, vertical = 12.dp),
                        verticalArrangement = Arrangement.spacedBy(6.dp)
                    ) {
                        Row(
                            modifier = Modifier.fillMaxWidth(),
                            horizontalArrangement = Arrangement.SpaceBetween
                        ) {
                            Text(
                                text = "${progress.percent}%",
                                style = MaterialTheme.typography.titleMedium,
                                fontWeight = FontWeight.Bold
                            )
                            Text(
                                text = "${formatFileSize(progress.bytesTransferred)} / ${formatFileSize(progress.totalBytes)}",
                                style = MaterialTheme.typography.bodySmall
                            )
                        }
                        Row(
                            modifier = Modifier.fillMaxWidth(),
                            horizontalArrangement = Arrangement.SpaceBetween
                        ) {
                            Text(
                                text = formatTransferRate(progress.bytesPerSecond),
                                style = MaterialTheme.typography.bodySmall
                            )
                            Text(
                                text = formatEta(progress.etaSeconds),
                                style = MaterialTheme.typography.bodySmall
                            )
                        }
                    }
                } else {
                    LinearProgressIndicator(modifier = Modifier.fillMaxWidth())
                }
            }
        }
    }
}

@Composable
private fun FilePickerCard(
    fileName: String,
    filePath: String,
    fileSize: Long,
    enabled: Boolean,
    onPickFile: () -> Unit
) {
    val hasFile = filePath.isNotBlank()
    ElevatedCard(
        modifier = Modifier.fillMaxWidth(),
        colors = CardDefaults.elevatedCardColors(containerColor = MaterialTheme.colorScheme.surface)
    ) {
        Column(
            modifier = Modifier.padding(16.dp),
            verticalArrangement = Arrangement.spacedBy(12.dp)
        ) {
            Row(verticalAlignment = Alignment.CenterVertically) {
                Surface(
                    color = MaterialTheme.colorScheme.primaryContainer,
                    contentColor = MaterialTheme.colorScheme.onPrimaryContainer,
                    shape = RoundedCornerShape(12.dp)
                ) {
                    Box(modifier = Modifier.padding(10.dp)) {
                        Icon(Icons.Default.InsertDriveFile, contentDescription = null)
                    }
                }
                Spacer(modifier = Modifier.size(12.dp))
                Column(modifier = Modifier.weight(1f)) {
                    Text(
                        text = if (hasFile) fileName else "Chưa chọn file",
                        style = MaterialTheme.typography.titleSmall,
                        fontWeight = FontWeight.SemiBold,
                        maxLines = 1,
                        overflow = TextOverflow.Ellipsis
                    )
                    Text(
                        text = if (hasFile) formatFileSize(fileSize) else "Ảnh, video, tài liệu hoặc file bất kỳ",
                        style = MaterialTheme.typography.bodySmall,
                        color = MaterialTheme.colorScheme.onSurfaceVariant
                    )
                }
            }

            if (hasFile) {
                Text(
                    text = filePath,
                    style = MaterialTheme.typography.bodySmall,
                    color = MaterialTheme.colorScheme.onSurfaceVariant,
                    maxLines = 2,
                    overflow = TextOverflow.Ellipsis
                )
            }

            OutlinedButton(
                onClick = onPickFile,
                enabled = enabled,
                modifier = Modifier.fillMaxWidth(),
                border = BorderStroke(1.dp, MaterialTheme.colorScheme.outline)
            ) {
                Icon(Icons.Default.FolderOpen, contentDescription = null)
                Spacer(modifier = Modifier.size(8.dp))
                Text(if (hasFile) "Đổi file" else "Chọn file")
            }
        }
    }
}

@Composable
private fun HistoryScreen(
    modifier: Modifier = Modifier,
    uploadHistory: List<UploadHistoryItem>,
    onClearHistory: () -> Unit
) {
    val context = androidx.compose.ui.platform.LocalContext.current

    LazyColumn(
        modifier = modifier.fillMaxSize(),
        contentPadding = PaddingValues(horizontal = 16.dp, vertical = 16.dp),
        verticalArrangement = Arrangement.spacedBy(10.dp)
    ) {
        item {
            Row(
                modifier = Modifier.fillMaxWidth(),
                horizontalArrangement = Arrangement.SpaceBetween,
                verticalAlignment = Alignment.CenterVertically
            ) {
                Column {
                    Text(
                        "Lịch sử truyền file",
                        style = MaterialTheme.typography.titleLarge,
                        fontWeight = FontWeight.SemiBold
                    )
                    Text(
                        "${uploadHistory.size} lần gần nhất",
                        style = MaterialTheme.typography.bodySmall,
                        color = MaterialTheme.colorScheme.onSurfaceVariant
                    )
                }
                if (uploadHistory.isNotEmpty()) {
                    TextButton(onClick = onClearHistory) {
                        Icon(Icons.Default.DeleteOutline, contentDescription = null, modifier = Modifier.size(18.dp))
                        Spacer(modifier = Modifier.size(4.dp))
                        Text("Xóa")
                    }
                }
            }
        }

        if (uploadHistory.isEmpty()) {
            item {
                ElevatedCard(modifier = Modifier.fillMaxWidth()) {
                    Column(
                        modifier = Modifier
                            .fillMaxWidth()
                            .padding(vertical = 36.dp, horizontal = 20.dp),
                        horizontalAlignment = Alignment.CenterHorizontally,
                        verticalArrangement = Arrangement.spacedBy(8.dp)
                    ) {
                        Icon(
                            Icons.Default.History,
                            contentDescription = null,
                            modifier = Modifier.size(36.dp),
                            tint = MaterialTheme.colorScheme.onSurfaceVariant
                        )
                        Text("Chưa có lịch sử truyền file", fontWeight = FontWeight.Medium)
                        Text(
                            "Các lần gửi thành công hoặc thất bại sẽ xuất hiện ở đây.",
                            style = MaterialTheme.typography.bodySmall,
                            color = MaterialTheme.colorScheme.onSurfaceVariant
                        )
                    }
                }
            }
        } else {
            items(uploadHistory, key = { "${it.timestamp}-${it.sha256}-${it.fileName}" }) { item ->
                HistoryItemCard(
                    item = item,
                    onCopyHash = {
                        val clipboard = context.getSystemService(Context.CLIPBOARD_SERVICE) as ClipboardManager
                        clipboard.setPrimaryClip(ClipData.newPlainText("Hash ID", item.sha256))
                        Toast.makeText(context, "Đã sao chép Hash ID", Toast.LENGTH_SHORT).show()
                    }
                )
            }
        }
    }
}

@Composable
private fun HistoryItemCard(
    item: UploadHistoryItem,
    onCopyHash: () -> Unit
) {
    val statusColor = if (item.isSuccess) {
        MaterialTheme.colorScheme.primary
    } else {
        MaterialTheme.colorScheme.error
    }

    ElevatedCard(modifier = Modifier.fillMaxWidth()) {
        Column(
            modifier = Modifier.padding(14.dp),
            verticalArrangement = Arrangement.spacedBy(8.dp)
        ) {
            Row(
                modifier = Modifier.fillMaxWidth(),
                verticalAlignment = Alignment.CenterVertically
            ) {
                Icon(
                    imageVector = if (item.isSuccess) Icons.Default.CheckCircle else Icons.Default.Error,
                    contentDescription = null,
                    tint = statusColor
                )
                Spacer(modifier = Modifier.size(10.dp))
                Column(modifier = Modifier.weight(1f)) {
                    Text(
                        item.fileName,
                        fontWeight = FontWeight.SemiBold,
                        maxLines = 1,
                        overflow = TextOverflow.Ellipsis
                    )
                    Text(
                        "${formatFileSize(item.fileSize)} · ${formatTimestamp(item.timestamp)}",
                        style = MaterialTheme.typography.bodySmall,
                        color = MaterialTheme.colorScheme.onSurfaceVariant
                    )
                }
            }

            Text(
                item.statusMsg,
                style = MaterialTheme.typography.bodySmall,
                color = statusColor
            )

            if (item.sha256.isNotBlank()) {
                Surface(
                    modifier = Modifier.fillMaxWidth(),
                    color = MaterialTheme.colorScheme.surfaceVariant,
                    shape = RoundedCornerShape(10.dp)
                ) {
                    Row(
                        modifier = Modifier.padding(start = 10.dp, end = 4.dp, top = 4.dp, bottom = 4.dp),
                        verticalAlignment = Alignment.CenterVertically
                    ) {
                        Column(modifier = Modifier.weight(1f)) {
                            Text(
                                "Hash ID",
                                style = MaterialTheme.typography.labelSmall,
                                color = MaterialTheme.colorScheme.onSurfaceVariant
                            )
                            Text(
                                item.sha256,
                                style = MaterialTheme.typography.bodySmall,
                                fontFamily = FontFamily.Monospace,
                                maxLines = 1,
                                overflow = TextOverflow.Ellipsis
                            )
                        }
                        IconButton(onClick = onCopyHash) {
                            Icon(Icons.Default.ContentCopy, contentDescription = "Sao chép Hash ID")
                        }
                    }
                }
            }
        }
    }
}

fun getFileNameFromUri(context: Context, uri: Uri): String {
    var name = ""
    if (uri.scheme == "content") {
        val cursor = context.contentResolver.query(uri, null, null, null, null)
        cursor?.use {
            if (it.moveToFirst()) {
                val index = it.getColumnIndex(OpenableColumns.DISPLAY_NAME)
                if (index != -1) {
                    name = it.getString(index)
                }
            }
        }
    }
    if (name.isEmpty()) {
        name = uri.path?.substringAfterLast('/') ?: "file.bin"
    }
    return name
}

fun copyUriToPendingUpload(context: Context, uri: Uri, targetFileName: String): File? {
    return try {
        val safeName = targetFileName.ifBlank { "file.bin" }.replace('/', '_').replace('\\', '_')
        // Keep a private durable copy while the foreground service owns the transfer.
        // Unlike cacheDir, filesDir is not automatically reclaimed under storage pressure.
        val transferDir = File(
            context.filesDir,
            "pending_uploads/${System.currentTimeMillis()}_${java.util.UUID.randomUUID()}"
        )
        if (!transferDir.mkdirs() && !transferDir.isDirectory) return null
        val pendingFile = File(transferDir, safeName)
        val input = context.contentResolver.openInputStream(uri) ?: return null
        input.use { source ->
            FileOutputStream(pendingFile).use { output ->
                source.copyTo(output, bufferSize = 256 * 1024)
            }
        }
        pendingFile
    } catch (e: Exception) {
        Log.e("FilePicker", "Failed to prepare selected URI for background upload", e)
        null
    }
}
