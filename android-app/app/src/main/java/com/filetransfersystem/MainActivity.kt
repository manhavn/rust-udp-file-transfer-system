package com.filetransfersystem

import android.content.Context
import android.net.Uri
import android.os.Bundle
import android.provider.OpenableColumns
import android.widget.Toast
import androidx.activity.ComponentActivity
import androidx.activity.compose.rememberLauncherForActivityResult
import androidx.activity.compose.setContent
import androidx.activity.result.contract.ActivityResultContracts
import androidx.compose.foundation.background
import androidx.compose.foundation.layout.*
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.foundation.text.KeyboardOptions
import androidx.compose.foundation.verticalScroll
import androidx.compose.material3.*
import androidx.compose.runtime.*
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.text.font.FontFamily
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.input.KeyboardType
import androidx.compose.ui.text.input.PasswordVisualTransformation
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import androidx.work.OneTimeWorkRequestBuilder
import androidx.work.WorkManager
import androidx.work.workDataOf
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.launch
import java.io.File
import java.io.FileOutputStream

class MainActivity : ComponentActivity() {
    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        
        setContent {
            // Dark Color Scheme
            val darkColorScheme = darkColorScheme(
                primary = Color(0xFF6200EE),
                secondary = Color(0xFF03DAC6),
                background = Color(0xFF121212),
                surface = Color(0xFF1E1E1E),
                onPrimary = Color.White,
                onSecondary = Color.Black,
                onBackground = Color(0xFFE0E0E0),
                onSurface = Color(0xFFE0E0E0),
            )

            MaterialTheme(colorScheme = darkColorScheme) {
                Surface(
                    modifier = Modifier.fillMaxSize(),
                    color = MaterialTheme.colorScheme.background
                ) {
                    UdpUploadScreen(
                        onStartUpload = { filePath, ip, udpPort, httpPort, blockSize, password, onStatusChange ->
                            CoroutineScope(Dispatchers.Main).launch {
                                onStatusChange("Đang tính toán hash và bắt đầu tải lên...")
                                val code = RustUploader.performUpload(
                                    filePath = filePath,
                                    serverIp = ip,
                                    udpPort = udpPort,
                                    httpPort = httpPort,
                                    blockSize = blockSize,
                                    password = password
                                )
                                if (code == 0) {
                                    onStatusChange("Thành công: Tải lên hoàn tất (Mã 0)!")
                                } else {
                                    val errorMsg = when (code) {
                                        -1 -> "Tham số không hợp lệ"
                                        -2 -> "Không tìm thấy/không đọc được file"
                                        -3 -> "Tính mã băm (hash) thất bại"
                                        -4 -> "Đăng ký tải lên qua HTTP thất bại"
                                        -5 -> "Không phân giải được địa chỉ IP/UDP"
                                        -6 -> "Lỗi bind cổng UDP cục bộ"
                                        -7 -> "Truyền tải UDP thất bại hoặc quá hạn (Timeout)"
                                        -99 -> "Native FFI crash hoặc JNA lỗi"
                                        else -> "Mã lỗi không xác định: $code"
                                    }
                                    onStatusChange("Lỗi ($code): $errorMsg")
                                }
                            }
                        },
                        onStartWorkManager = { filePath, ip, udpPort, httpPort, blockSize, password, onStatusChange ->
                            val workRequest = OneTimeWorkRequestBuilder<UploadWorker>()
                                .setInputData(
                                    workDataOf(
                                        UploadWorker.KEY_FILE_PATH to filePath,
                                        UploadWorker.KEY_SERVER_IP to ip,
                                        UploadWorker.KEY_UDP_PORT to udpPort,
                                        UploadWorker.KEY_HTTP_PORT to httpPort,
                                        UploadWorker.KEY_BLOCK_SIZE to blockSize,
                                        UploadWorker.KEY_PASSWORD to password
                                    )
                                )
                                .build()
                            
                            WorkManager.getInstance(applicationContext).enqueue(workRequest)
                            Toast.makeText(this, "Đã đẩy tác vụ tải lên vào WorkManager", Toast.LENGTH_SHORT).show()
                            
                            // Observe live status
                            WorkManager.getInstance(applicationContext)
                                .getWorkInfoByIdLiveData(workRequest.id)
                                .observe(this) { workInfo ->
                                    if (workInfo != null) {
                                        val state = workInfo.state.name
                                        val code = workInfo.outputData.getInt(UploadWorker.KEY_RESULT_CODE, -99)
                                        val errMsg = workInfo.outputData.getString(UploadWorker.KEY_ERROR_MESSAGE) ?: ""
                                        
                                        if (workInfo.state.isFinished) {
                                            if (code == 0) {
                                                onStatusChange("WorkManager HOÀN THÀNH: Thành công (Mã 0)!")
                                            } else {
                                                onStatusChange("WorkManager THẤT BẠI: Mã $code - $errMsg")
                                            }
                                        } else {
                                            onStatusChange("WorkManager Trạng thái: $state...")
                                        }
                                    }
                                }
                        }
                    )
                }
            }
        }
    }
}

@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun UdpUploadScreen(
    onStartUpload: (String, String, Int, Int, Int, String?, (String) -> Unit) -> Unit,
    onStartWorkManager: (String, String, Int, Int, Int, String?, (String) -> Unit) -> Unit
) {
    val context = LocalContext.current
    
    // States
    var filePath by remember { mutableStateOf("") }
    var fileName by remember { mutableStateOf("demo_data.bin") }
    var demoContent by remember { mutableStateOf("Nội dung thử nghiệm của file demo truyền tải UDP.") }
    var serverIp by remember { mutableStateOf("") }
    var udpPortStr by remember { mutableStateOf("5000") }
    var httpPortStr by remember { mutableStateOf("8080") }
    var blockSizeStr by remember { mutableStateOf("16384") }
    var password by remember { mutableStateOf("") }
    var statusText by remember { mutableStateOf("Chọn một file hoặc tạo file demo để bắt đầu.") }

    val scrollState = rememberScrollState()

    // File Selector Contract
    val filePickerLauncher = rememberLauncherForActivityResult(
        contract = ActivityResultContracts.GetContent(),
        onResult = { uri ->
            if (uri != null) {
                val resolvedName = getFileNameFromUri(context, uri)
                fileName = resolvedName
                
                // Copy selected file to cache to bypass Scoped Storage file path limitations
                statusText = "Đang xử lý sao chép file..."
                val cachedFile = copyUriToCache(context, uri, resolvedName)
                if (cachedFile != null) {
                    filePath = cachedFile.absolutePath
                    statusText = "Đã chọn file: ${cachedFile.name} (${cachedFile.length()} bytes)"
                } else {
                    statusText = "Sao chép file chọn thất bại."
                }
            }
        }
    )

    Column(
        modifier = Modifier
            .fillMaxSize()
            .padding(16.dp)
            .verticalScroll(scrollState),
        horizontalAlignment = Alignment.CenterHorizontally,
        verticalArrangement = Arrangement.spacedBy(12.dp)
    ) {
        // Status Card
        Card(
            modifier = Modifier.fillMaxWidth(),
            colors = CardDefaults.cardColors(containerColor = MaterialTheme.colorScheme.surface),
            shape = RoundedCornerShape(8.dp)
        ) {
            Column(modifier = Modifier.padding(12.dp)) {
                Text(
                    text = "Trạng thái:",
                    fontSize = 12.sp,
                    fontWeight = FontWeight.Bold,
                    color = Color.Gray
                )
                Text(
                    text = statusText,
                    fontSize = 14.sp,
                    color = MaterialTheme.colorScheme.onSurface,
                    fontFamily = FontFamily.Monospace,
                    modifier = Modifier.padding(top = 4.dp)
                )
            }
        }

        // Section: File Source Selection
        Text(
            text = "1. NGUỒN FILE TRUYỀN TẢI",
            fontSize = 14.sp,
            fontWeight = FontWeight.Bold,
            modifier = Modifier.align(Alignment.Start),
            color = MaterialTheme.colorScheme.secondary
        )

        // Text Field for File Name (Editable)
        OutlinedTextField(
            value = fileName,
            onValueChange = { fileName = it },
            label = { Text("Tên File trước khi lưu & upload") },
            modifier = Modifier.fillMaxWidth(),
            colors = OutlinedTextFieldDefaults.colors(
                focusedBorderColor = MaterialTheme.colorScheme.secondary,
                unfocusedBorderColor = Color.Gray
            )
        )

        Row(
            modifier = Modifier.fillMaxWidth(),
            horizontalArrangement = Arrangement.spacedBy(8.dp)
        ) {
            // Button to choose file from SD Card / Storage
            Button(
                onClick = { filePickerLauncher.launch("*/*") },
                modifier = Modifier.weight(1f),
                colors = ButtonDefaults.buttonColors(containerColor = MaterialTheme.colorScheme.primary)
            ) {
                Text("Chọn File SDCard")
            }
        }

        // Text Field for Custom Demo Content
        OutlinedTextField(
            value = demoContent,
            onValueChange = { demoContent = it },
            label = { Text("Nội dung file demo tự tạo") },
            modifier = Modifier.fillMaxWidth(),
            colors = OutlinedTextFieldDefaults.colors(
                focusedBorderColor = MaterialTheme.colorScheme.secondary,
                unfocusedBorderColor = Color.Gray
            )
        )

        // Button to generate file from demo content
        Button(
            onClick = {
                if (fileName.isEmpty()) {
                    Toast.makeText(context, "Vui lòng nhập tên file", Toast.LENGTH_SHORT).show()
                } else {
                    val demoFile = File(context.cacheDir, fileName)
                    try {
                        FileOutputStream(demoFile).use { fos ->
                            fos.write(demoContent.toByteArray(Charsets.UTF_8))
                        }
                        filePath = demoFile.absolutePath
                        statusText = "Đã tạo file demo thành công: ${demoFile.name} (${demoFile.length()} bytes)"
                    } catch (e: Exception) {
                        statusText = "Tạo file demo lỗi: ${e.message}"
                    }
                }
            },
            modifier = Modifier.fillMaxWidth(),
            colors = ButtonDefaults.buttonColors(containerColor = Color(0xFF2C2C2C))
        ) {
            Text("Tạo File Demo từ Nội dung trên", color = MaterialTheme.colorScheme.secondary)
        }

        // File Path Info (read-only)
        OutlinedTextField(
            value = filePath,
            onValueChange = {},
            readOnly = true,
            label = { Text("Đường dẫn File được chọn để Upload") },
            modifier = Modifier.fillMaxWidth(),
            colors = OutlinedTextFieldDefaults.colors(
                focusedBorderColor = Color.DarkGray,
                unfocusedBorderColor = Color.DarkGray
            )
        )

        // Section: Network Configurations
        Text(
            text = "2. CẤU HÌNH KẾT NỐI SERVER",
            fontSize = 14.sp,
            fontWeight = FontWeight.Bold,
            modifier = Modifier.align(Alignment.Start),
            color = MaterialTheme.colorScheme.secondary
        )

        OutlinedTextField(
            value = serverIp,
            onValueChange = { serverIp = it },
            label = { Text("Địa chỉ IP Server") },
            placeholder = { Text("Ví dụ: 10.0.2.2 hoặc 192.168.1.100", color = Color.Gray) },
            modifier = Modifier.fillMaxWidth(),
            colors = OutlinedTextFieldDefaults.colors(
                focusedBorderColor = MaterialTheme.colorScheme.secondary,
                unfocusedBorderColor = Color.Gray
            )
        )

        Row(
            modifier = Modifier.fillMaxWidth(),
            horizontalArrangement = Arrangement.spacedBy(8.dp)
        ) {
            OutlinedTextField(
                value = udpPortStr,
                onValueChange = { udpPortStr = it },
                label = { Text("Cổng UDP") },
                keyboardOptions = KeyboardOptions(keyboardType = KeyboardType.Number),
                modifier = Modifier.weight(1f),
                colors = OutlinedTextFieldDefaults.colors(
                    focusedBorderColor = MaterialTheme.colorScheme.secondary,
                    unfocusedBorderColor = Color.Gray
                )
            )

            OutlinedTextField(
                value = httpPortStr,
                onValueChange = { httpPortStr = it },
                label = { Text("Cổng HTTP") },
                keyboardOptions = KeyboardOptions(keyboardType = KeyboardType.Number),
                modifier = Modifier.weight(1f),
                colors = OutlinedTextFieldDefaults.colors(
                    focusedBorderColor = MaterialTheme.colorScheme.secondary,
                    unfocusedBorderColor = Color.Gray
                )
            )
        }

        Row(
            modifier = Modifier.fillMaxWidth(),
            horizontalArrangement = Arrangement.spacedBy(8.dp)
        ) {
            OutlinedTextField(
                value = blockSizeStr,
                onValueChange = { blockSizeStr = it },
                label = { Text("Kích thước Block (bytes)") },
                keyboardOptions = KeyboardOptions(keyboardType = KeyboardType.Number),
                modifier = Modifier.weight(1.5f),
                colors = OutlinedTextFieldDefaults.colors(
                    focusedBorderColor = MaterialTheme.colorScheme.secondary,
                    unfocusedBorderColor = Color.Gray
                )
            )

            OutlinedTextField(
                value = password,
                onValueChange = { password = it },
                label = { Text("Mật khẩu (Tùy chọn)") },
                visualTransformation = PasswordVisualTransformation(),
                modifier = Modifier.weight(1.5f),
                colors = OutlinedTextFieldDefaults.colors(
                    focusedBorderColor = MaterialTheme.colorScheme.secondary,
                    unfocusedBorderColor = Color.Gray
                )
            )
        }

        Spacer(modifier = Modifier.height(8.dp))

        // Section: Action Buttons
        Button(
            onClick = {
                if (filePath.isEmpty()) {
                    Toast.makeText(context, "Vui lòng chọn hoặc tạo file trước", Toast.LENGTH_SHORT).show()
                } else {
                    val udp = udpPortStr.toIntOrNull() ?: 5000
                    val http = httpPortStr.toIntOrNull() ?: 8080
                    val size = blockSizeStr.toIntOrNull() ?: 16384
                    val pwd = if (password.isEmpty()) null else password
                    onStartUpload(filePath, serverIp, udp, http, size, pwd) { statusText = it }
                }
            },
            modifier = Modifier
                .fillMaxWidth()
                .height(50.dp),
            colors = ButtonDefaults.buttonColors(containerColor = MaterialTheme.colorScheme.primary)
        ) {
            Text("BẮT ĐẦU TẢI LÊN (DIRECT UDP)", fontSize = 16.sp, fontWeight = FontWeight.Bold)
        }

        Button(
            onClick = {
                if (filePath.isEmpty()) {
                    Toast.makeText(context, "Vui lòng chọn hoặc tạo file trước", Toast.LENGTH_SHORT).show()
                } else {
                    val udp = udpPortStr.toIntOrNull() ?: 5000
                    val http = httpPortStr.toIntOrNull() ?: 8080
                    val size = blockSizeStr.toIntOrNull() ?: 16384
                    val pwd = if (password.isEmpty()) null else password
                    onStartWorkManager(filePath, serverIp, udp, http, size, pwd) { statusText = it }
                }
            },
            modifier = Modifier
                .fillMaxWidth()
                .height(50.dp),
            colors = ButtonDefaults.buttonColors(containerColor = MaterialTheme.colorScheme.secondary)
        ) {
            Text("TẢI LÊN QUA WORKMANAGER", fontSize = 16.sp, fontWeight = FontWeight.Bold, color = Color.Black)
        }
    }
}

// Helper: Get original filename from URI
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

// Helper: Copy selected URI stream to a cache file
fun copyUriToCache(context: Context, uri: Uri, targetFileName: String): File? {
    return try {
        val cacheFile = File(context.cacheDir, targetFileName)
        context.contentResolver.openInputStream(uri)?.use { input ->
            FileOutputStream(cacheFile).use { output ->
                input.copyTo(output)
            }
        }
        cacheFile
    } catch (e: Exception) {
        e.printStackTrace()
        null
    }
}
