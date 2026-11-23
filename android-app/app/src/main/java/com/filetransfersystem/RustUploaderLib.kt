package com.filetransfersystem

import android.util.Log
import com.sun.jna.Callback
import com.sun.jna.Library
import com.sun.jna.Native
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.withContext
import java.io.File
import java.io.FileInputStream
import java.security.MessageDigest
import kotlin.math.ceil

/**
 * Progress reported by the Rust uploader from server-confirmed ACKs.
 */
data class UploadProgress(
    val bytesTransferred: Long,
    val totalBytes: Long,
    val bytesPerSecond: Long
) {
    val fraction: Float
        get() = if (totalBytes <= 0L) 0f else
            (bytesTransferred.toDouble() / totalBytes.toDouble()).coerceIn(0.0, 1.0).toFloat()

    val percent: Int
        get() = (fraction * 100f).toInt().coerceIn(0, 100)

    val etaSeconds: Long?
        get() {
            if (bytesPerSecond <= 0L || totalBytes <= bytesTransferred) {
                return if (totalBytes > 0L && bytesTransferred >= totalBytes) 0L else null
            }
            return ceil((totalBytes - bytesTransferred).toDouble() / bytesPerSecond.toDouble()).toLong()
        }
}

/**
 * JNA callback implemented by Kotlin and invoked by Rust.
 * Values are based on bytes ACKed by the server, not raw socket writes.
 */
interface UploadProgressCallback : Callback {
    fun invoke(bytesTransferred: Long, totalBytes: Long, bytesPerSecond: Long)
}

/**
 * JNA Interface mapping the Rust client_lib C-FFI functions.
 */
interface RustUploaderLib : Library {
    fun rtk_upload_file(
        filePath: String,
        serverIp: String,
        udpPort: Short,
        httpPort: Short,
        blockSize: Long
    ): Int

    fun rtk_upload_file_with_password(
        filePath: String,
        serverIp: String,
        udpPort: Short,
        httpPort: Short,
        blockSize: Long,
        password: String?
    ): Int

    fun rtk_upload_file_with_password_and_progress(
        filePath: String,
        serverIp: String,
        udpPort: Short,
        httpPort: Short,
        blockSize: Long,
        password: String?,
        progressCallback: UploadProgressCallback?
    ): Int

    fun rtk_calculate_hash_id(
        filePath: String,
        outBuf: ByteArray,
        maxLen: Long
    ): Int

    companion object {
        private const val TAG = "RustUploaderLib"

        init {
            try {
                Log.i(TAG, "Attempting to load client_lib native library via System.loadLibrary")
                System.loadLibrary("client_lib")
                Log.i(TAG, "Native library client_lib loaded successfully")
            } catch (e: UnsatisfiedLinkError) {
                Log.e(TAG, "Failed to load native library client_lib via System.loadLibrary: ${e.message}")
            }
        }

        val INSTANCE: RustUploaderLib by lazy {
            try {
                Native.load("client_lib", RustUploaderLib::class.java) as RustUploaderLib
            } catch (e: Throwable) {
                Log.e(TAG, "JNA failed to load client_lib: ${e.message}")
                throw e
            }
        }
    }
}

/**
 * Helper class to run the upload process asynchronously using Kotlin Coroutines.
 */
object RustUploader {
    private const val TAG = "RustUploader"

    /**
     * Calculates the SHA-256 hash of a file at [filePath] and returns its hexadecimal string representation.
     */
    fun calculateSHA256(filePath: String): String {
        val file = File(filePath)
        if (!file.exists() || !file.canRead()) {
            return "file_not_found_or_unreadable"
        }
        return try {
            val digest = MessageDigest.getInstance("SHA-256")
            val buffer = ByteArray(8192)
            FileInputStream(file).use { fis ->
                var bytesRead: Int
                while (fis.read(buffer).also { bytesRead = it } != -1) {
                    digest.update(buffer, 0, bytesRead)
                }
            }
            digest.digest().joinToString("") { "%02x".format(it) }
        } catch (e: Exception) {
            Log.e(TAG, "Error calculating SHA-256 for file $filePath", e)
            "hash_calculation_error"
        }
    }

    /**
     * Computes the exact Hash ID (packet code string) for the file using the Rust FFI.
     */
    fun calculateHashId(filePath: String): String {
        return try {
            val buf = ByteArray(256)
            val code = RustUploaderLib.INSTANCE.rtk_calculate_hash_id(filePath, buf, buf.size.toLong())
            if (code == 0) {
                val len = buf.indexOf(0)
                if (len >= 0) {
                    String(buf, 0, len, Charsets.UTF_8)
                } else {
                    String(buf, Charsets.UTF_8).trim { it <= ' ' }
                }
            } else {
                Log.e(TAG, "rtk_calculate_hash_id returned error: $code")
                calculateSHA256(filePath).take(10)
            }
        } catch (e: Throwable) {
            Log.e(TAG, "Failed to call rtk_calculate_hash_id: ${e.message}", e)
            calculateSHA256(filePath).take(10)
        }
    }

    suspend fun performUpload(
        filePath: String,
        serverIp: String,
        udpPort: Int,
        httpPort: Int,
        blockSize: Int = 16384,
        password: String? = null,
        onProgress: ((UploadProgress) -> Unit)? = null
    ): Int = withContext(Dispatchers.IO) {
        Log.i(
            TAG,
            "Starting upload. File: $filePath, Server: $serverIp, UDP: $udpPort, HTTP: $httpPort"
        )

        val nativeCallback = onProgress?.let { listener ->
            object : UploadProgressCallback {
                override fun invoke(
                    bytesTransferred: Long,
                    totalBytes: Long,
                    bytesPerSecond: Long
                ) {
                    try {
                        listener(
                            UploadProgress(
                                bytesTransferred = bytesTransferred.coerceAtLeast(0L),
                                totalBytes = totalBytes.coerceAtLeast(0L),
                                bytesPerSecond = bytesPerSecond.coerceAtLeast(0L)
                            )
                        )
                    } catch (t: Throwable) {
                        Log.w(TAG, "Progress callback failed: ${t.message}", t)
                    }
                }
            }
        }

        try {
            val resultCode = if (nativeCallback != null) {
                RustUploaderLib.INSTANCE.rtk_upload_file_with_password_and_progress(
                    filePath = filePath,
                    serverIp = serverIp,
                    udpPort = udpPort.toShort(),
                    httpPort = httpPort.toShort(),
                    blockSize = blockSize.toLong(),
                    password = password,
                    progressCallback = nativeCallback
                )
            } else if (password.isNullOrEmpty()) {
                RustUploaderLib.INSTANCE.rtk_upload_file(
                    filePath = filePath,
                    serverIp = serverIp,
                    udpPort = udpPort.toShort(),
                    httpPort = httpPort.toShort(),
                    blockSize = blockSize.toLong()
                )
            } else {
                RustUploaderLib.INSTANCE.rtk_upload_file_with_password(
                    filePath = filePath,
                    serverIp = serverIp,
                    udpPort = udpPort.toShort(),
                    httpPort = httpPort.toShort(),
                    blockSize = blockSize.toLong(),
                    password = password
                )
            }

            if (resultCode == 0) {
                Log.i(TAG, "Upload completed successfully! File: $filePath")
            } else {
                Log.e(TAG, "Upload failed! Code: $resultCode, File: $filePath")
            }
            resultCode
        } catch (e: UnsatisfiedLinkError) {
            // Helpful when an old .so is accidentally packaged with the new APK.
            Log.e(TAG, "Progress-enabled native symbol is missing: ${e.message}", e)
            -98
        } catch (e: Throwable) {
            Log.e(TAG, "FFI call crashed: ${e.message}, File: $filePath", e)
            -99
        }
    }
}
