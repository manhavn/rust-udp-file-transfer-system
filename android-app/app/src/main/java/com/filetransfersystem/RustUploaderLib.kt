package com.filetransfersystem

import android.util.Log
import com.sun.jna.Library
import com.sun.jna.Native
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.withContext
import java.io.File
import java.io.FileInputStream
import java.security.MessageDigest

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
        password: String? = null
    ): Int = withContext(Dispatchers.IO) {
        val hashId = calculateHashId(filePath)
        val sha256 = calculateSHA256(filePath)
        Log.i(TAG, "Starting upload. File: $filePath, Hash ID: $hashId, SHA-256: $sha256, Server: $serverIp, UDP: $udpPort, HTTP: $httpPort")

        try {
            val resultCode = if (password.isNullOrEmpty()) {
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
                Log.i(TAG, "Upload completed successfully! File: $filePath, Hash ID: $hashId")
            } else {
                Log.e(TAG, "Upload failed! Code: $resultCode, File: $filePath, Hash ID: $hashId")
            }
            resultCode
        } catch (e: Throwable) {
            Log.e(TAG, "FFI call crashed: ${e.message}, File: $filePath, Hash ID: $hashId")
            -99 // Custom crash code
        }
    }
}
