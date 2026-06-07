package com.filetransfersystem

import android.util.Log
import com.sun.jna.Library
import com.sun.jna.Native
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.withContext

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
    suspend fun performUpload(
        filePath: String,
        serverIp: String,
        udpPort: Int,
        httpPort: Int,
        blockSize: Int = 16384,
        password: String? = null
    ): Int = withContext(Dispatchers.IO) {
        try {
            if (password.isNullOrEmpty()) {
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
        } catch (e: Throwable) {
            Log.e("RustUploader", "FFI call crashed: ${e.message}")
            -99 // Custom crash code
        }
    }
}
