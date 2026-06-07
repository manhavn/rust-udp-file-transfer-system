package com.filetransfersystem

import android.content.Context
import android.util.Log
import androidx.work.CoroutineWorker
import androidx.work.WorkerParameters
import androidx.work.workDataOf
import java.io.File

/**
 * Android WorkManager CoroutineWorker for reliable background file uploads.
 */
class UploadWorker(
    context: Context,
    params: WorkerParameters
) : CoroutineWorker(context, params) {

    companion object {
        const val TAG = "UploadWorker"
        
        // Input Keys
        const val KEY_FILE_PATH = "file_path"
        const val KEY_SERVER_IP = "server_ip"
        const val KEY_UDP_PORT = "udp_port"
        const val KEY_HTTP_PORT = "http_port"
        const val KEY_BLOCK_SIZE = "block_size"
        const val KEY_PASSWORD = "password"

        // Output/Progress Keys
        const val KEY_RESULT_CODE = "result_code"
        const val KEY_ERROR_MESSAGE = "error_message"
        const val KEY_FILE_HASH = "file_hash"
    }

    override suspend fun doWork(): Result {
        val filePath = inputData.getString(KEY_FILE_PATH)
        val serverIp = inputData.getString(KEY_SERVER_IP)
        val udpPort = inputData.getInt(KEY_UDP_PORT, 5000)
        val httpPort = inputData.getInt(KEY_HTTP_PORT, 8080)
        val blockSize = inputData.getInt(KEY_BLOCK_SIZE, 16384)
        val password = inputData.getString(KEY_PASSWORD)

        if (filePath.isNullOrEmpty() || serverIp.isNullOrEmpty()) {
            Log.e(TAG, "Missing required parameters: file_path or server_ip")
            return Result.failure(
                workDataOf(
                    KEY_RESULT_CODE to -1,
                    KEY_ERROR_MESSAGE to "Missing required parameters"
                )
            )
        }

        // Try loading the native library inside the worker process
        try {
            System.loadLibrary("client_lib")
        } catch (e: UnsatisfiedLinkError) {
            Log.e(TAG, "UnsatisfiedLinkError loading client_lib inside Worker: ${e.message}")
            return Result.failure(
                workDataOf(
                    KEY_RESULT_CODE to -98,
                    KEY_ERROR_MESSAGE to "UnsatisfiedLinkError: ${e.message}"
                )
            )
        }

        val file = File(filePath)
        if (!file.exists() || !file.canRead()) {
            Log.e(TAG, "File does not exist or cannot be read: $filePath")
            return Result.failure(
                workDataOf(
                    KEY_RESULT_CODE to -2,
                    KEY_ERROR_MESSAGE to "File does not exist or is not readable"
                )
            )
        }

        val fileHash = RustUploader.calculateSHA256(filePath)
        val hashId = RustUploader.calculateHashId(filePath)
        Log.i(TAG, "Starting UDP file upload: ${file.name} (${file.length()} bytes) to $serverIp:$udpPort. Hash ID: $hashId, SHA-256: $fileHash")
        
        try {
            // Call the Rust library wrapper
            val resultCode = RustUploader.performUpload(
                filePath = filePath,
                serverIp = serverIp,
                udpPort = udpPort,
                httpPort = httpPort,
                blockSize = blockSize,
                password = password
            )

            return if (resultCode == 0) {
                Log.i(TAG, "Upload completed successfully! File: ${file.name}, Hash ID: $hashId")
                Result.success(
                    workDataOf(
                        KEY_RESULT_CODE to resultCode,
                        KEY_FILE_HASH to hashId
                    )
                )
            } else {
                val errorMsg = when (resultCode) {
                    -1 -> "Invalid parameters passed to Rust library"
                    -2 -> "Rust library: File access error"
                    -3 -> "Rust library: Hash calculation failed"
                    -4 -> "Rust library: HTTP registration failed"
                    -5 -> "Rust library: Server address resolution failed"
                    -6 -> "Rust library: Local UDP socket binding failed"
                    -7 -> "Rust library: UDP transfer connection/transmission timed out"
                    -99 -> "Rust library: Native library call crashed"
                    else -> "Unknown error code: $resultCode"
                }
                Log.e(TAG, "Upload failed: $errorMsg (code: $resultCode). File: ${file.name}, Hash ID: $hashId")
                Result.failure(
                    workDataOf(
                        KEY_RESULT_CODE to resultCode,
                        KEY_ERROR_MESSAGE to errorMsg,
                        KEY_FILE_HASH to hashId
                    )
                )
            }
        } catch (e: Throwable) {
            Log.e(TAG, "Unexpected error executing Rust FFI upload in Worker for file: ${file.name}, Hash ID: $hashId", e)
            return Result.failure(
                workDataOf(
                    KEY_RESULT_CODE to -97,
                    KEY_ERROR_MESSAGE to "FFI exception: ${e.message}",
                    KEY_FILE_HASH to hashId
                )
            )
        }
    }
}
