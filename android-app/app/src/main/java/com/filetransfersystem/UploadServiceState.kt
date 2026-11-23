package com.filetransfersystem

import android.content.Context
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow

/**
 * Process-wide upload state backed by SharedPreferences so the UI can recover the latest
 * foreground-transfer status after Activity recreation or process restart.
 */
data class BackgroundUploadState(
    val transferId: String = "",
    val running: Boolean = false,
    val filePath: String = "",
    val fileName: String = "",
    val serverIp: String = "",
    val bytesTransferred: Long = 0L,
    val totalBytes: Long = 0L,
    val bytesPerSecond: Long = 0L,
    val statusText: String = "Sẵn sàng",
    val resultCode: Int? = null,
    val fileHash: String = "",
    val updatedAt: Long = 0L
) {
    val progress: UploadProgress
        get() = UploadProgress(bytesTransferred, totalBytes, bytesPerSecond)
}

object UploadServiceState {
    private const val PREFS = "foreground_upload_state"

    private val mutableState = MutableStateFlow(BackgroundUploadState())
    val state: StateFlow<BackgroundUploadState> = mutableState.asStateFlow()

    fun restore(context: Context): BackgroundUploadState {
        val prefs = context.getSharedPreferences(PREFS, Context.MODE_PRIVATE)
        val code = if (prefs.contains("result_code")) prefs.getInt("result_code", 0) else null
        val restored = BackgroundUploadState(
            transferId = prefs.getString("transfer_id", "").orEmpty(),
            running = prefs.getBoolean("running", false),
            filePath = prefs.getString("file_path", "").orEmpty(),
            fileName = prefs.getString("file_name", "").orEmpty(),
            serverIp = prefs.getString("server_ip", "").orEmpty(),
            bytesTransferred = prefs.getLong("bytes_transferred", 0L),
            totalBytes = prefs.getLong("total_bytes", 0L),
            bytesPerSecond = prefs.getLong("bytes_per_second", 0L),
            statusText = prefs.getString("status_text", "Sẵn sàng").orEmpty(),
            resultCode = code,
            fileHash = prefs.getString("file_hash", "").orEmpty(),
            updatedAt = prefs.getLong("updated_at", 0L)
        )
        mutableState.value = restored
        return restored
    }

    fun publish(context: Context, state: BackgroundUploadState, persist: Boolean = true) {
        mutableState.value = state
        if (!persist) return

        val editor = context.getSharedPreferences(PREFS, Context.MODE_PRIVATE).edit()
            .putString("transfer_id", state.transferId)
            .putBoolean("running", state.running)
            .putString("file_path", state.filePath)
            .putString("file_name", state.fileName)
            .putString("server_ip", state.serverIp)
            .putLong("bytes_transferred", state.bytesTransferred)
            .putLong("total_bytes", state.totalBytes)
            .putLong("bytes_per_second", state.bytesPerSecond)
            .putString("status_text", state.statusText)
            .putString("file_hash", state.fileHash)
            .putLong("updated_at", state.updatedAt)

        if (state.resultCode == null) {
            editor.remove("result_code")
        } else {
            editor.putInt("result_code", state.resultCode)
        }
        editor.apply()
    }
}
