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
import androidx.compose.ui.text.input.VisualTransformation
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import androidx.work.OneTimeWorkRequestBuilder
import androidx.work.WorkManager
import androidx.work.workDataOf
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.launch
import kotlinx.coroutines.withContext
import android.util.Log
import android.content.ClipboardManager
import android.content.ClipData
import org.json.JSONArray
import org.json.JSONObject
import java.io.File
import java.io.FileOutputStream
import java.text.SimpleDateFormat
import java.util.Date
import java.util.Locale

data class UploadHistoryItem(
    val fileName: String,
    val filePath: String,
    val fileSize: Long,
    val timestamp: Long,
    val sha256: String,
    val isSuccess: Boolean,
    val statusMsg: String
)

fun loadUploadHistory(context: Context): List<UploadHistoryItem> {
    val prefs = context.getSharedPreferences("udp_transfer_prefs", Context.MODE_PRIVATE)
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
    val prefs = context.getSharedPreferences("udp_transfer_prefs", Context.MODE_PRIVATE)
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
    return String.format(Locale.US, "%.2f %s", size / Math.pow(1024.0, digitGroups.toDouble()), units[digitGroups])
}

fun formatTimestamp(timestamp: Long): String {
    val sdf = SimpleDateFormat("dd/MM/yyyy HH:mm:ss", Locale.getDefault())
    return sdf.format(Date(timestamp))
}

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
                    val context = LocalContext.current
                    var uploadHistory by remember { mutableStateOf(loadUploadHistory(context)) }

                    UdpUploadScreen(
                        uploadHistory = uploadHistory,
                        onClearHistory = {
                            uploadHistory = emptyList()
                            saveUploadHistory(context, emptyList())
                        },
                        onStartUpload = { filePath, ip, udpPort, httpPort, blockSize, password, onStatusChange ->
                            CoroutineScope(Dispatchers.Main).launch {
                                onStatusChange("Đang tính toán hash và bắt đầu tải lên...")
                                val fileHash = withContext(Dispatchers.IO) {
                                    RustUploader.calculateHashId(filePath)
                                }
                                Log.i("MainActivity", "Starting upload: $filePath, Hash ID: $fileHash")
                                val code = RustUploader.performUpload(
                                    filePath = filePath,
                                    serverIp = ip,
                                    udpPort = udpPort,
                                    httpPort = httpPort,
                                    blockSize = blockSize,
                                    password = password
                                )
                                val isSuccess = (code == 0)
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
                                val statusMsg = if (isSuccess) "Thành công (Mã 0)" else "Lỗi ($code): $errorMsg"
                                
                                val newItem = UploadHistoryItem(
                                    fileName = File(filePath).name,
                                    filePath = filePath,
                                    fileSize = File(filePath).length(),
                                    timestamp = System.currentTimeMillis(),
                                    sha256 = fileHash,
                                    isSuccess = isSuccess,
                                    statusMsg = statusMsg
                                )
                                val updatedHistory = listOf(newItem) + uploadHistory
                                uploadHistory = updatedHistory
                                saveUploadHistory(context, updatedHistory)

                                if (code == 0) {
                                    Log.i("MainActivity", "Upload completed successfully! File: $filePath, Hash ID: $fileHash")
                                    onStatusChange("Thành công: Tải lên hoàn tất (Mã 0)!")
                                } else {
                                    Log.e("MainActivity", "Upload failed! Code: $code, File: $filePath, Hash ID: $fileHash")
                                    onStatusChange("Lỗi ($code): $errorMsg")
                                }
                            }
                        },
                        onStartWorkManager = { filePath, ip, udpPort, httpPort, blockSize, password, onStatusChange ->
                            CoroutineScope(Dispatchers.Main).launch {
                                val fileHash = withContext(Dispatchers.IO) {
                                    RustUploader.calculateHashId(filePath)
                                }
                                Log.i("MainActivity", "Enqueuing WorkManager upload: $filePath, Hash ID: $fileHash")

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
                                Toast.makeText(this@MainActivity, "Đã đẩy tác vụ tải lên vào WorkManager", Toast.LENGTH_SHORT).show()
                                
                                // Observe live status
                                WorkManager.getInstance(applicationContext)
                                    .getWorkInfoByIdLiveData(workRequest.id)
                                    .observe(this@MainActivity) { workInfo ->
                                        if (workInfo != null) {
                                            val state = workInfo.state.name
                                            val code = workInfo.outputData.getInt(UploadWorker.KEY_RESULT_CODE, -99)
                                            val errMsg = workInfo.outputData.getString(UploadWorker.KEY_ERROR_MESSAGE) ?: ""
                                            val returnedHash = workInfo.outputData.getString(UploadWorker.KEY_FILE_HASH) ?: fileHash
                                            
                                            if (workInfo.state.isFinished) {
                                                val isSuccess = (code == 0)
                                                val statusMsg = if (isSuccess) "WorkManager HOÀN THÀNH: Thành công (Mã 0)!" else "WorkManager THẤT BẠI: Mã $code - $errMsg"
                                                
                                                val newItem = UploadHistoryItem(
                                                    fileName = File(filePath).name,
                                                    filePath = filePath,
                                                    fileSize = File(filePath).length(),
                                                    timestamp = System.currentTimeMillis(),
                                                    sha256 = returnedHash,
                                                    isSuccess = isSuccess,
                                                    statusMsg = statusMsg
                                                )
                                                
                                                val alreadyExists = uploadHistory.any { 
                                                    it.sha256 == newItem.sha256 && Math.abs(it.timestamp - newItem.timestamp) < 5000 
                                                }
                                                if (!alreadyExists) {
                                                    val updatedHistory = listOf(newItem) + uploadHistory
                                                    uploadHistory = updatedHistory
                                                    saveUploadHistory(context, updatedHistory)
                                                }

                                                if (code == 0) {
                                                    Log.i("MainActivity", "WorkManager upload completed successfully! File: $filePath, SHA-256: $returnedHash")
                                                    onStatusChange("WorkManager HOÀN THÀNH: Thành công (Mã 0)!")
                                                } else {
                                                    Log.e("MainActivity", "WorkManager upload failed! Code: $code, Error: $errMsg, File: $filePath, SHA-256: $returnedHash")
                                                    onStatusChange("WorkManager THẤT BẠI: Mã $code - $errMsg")
                                                }
                                            } else {
                                                onStatusChange("WorkManager Trạng thái: $state...")
                                            }
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
    uploadHistory: List<UploadHistoryItem>,
    onClearHistory: () -> Unit,
    onStartUpload: (String, String, Int, Int, Int, String?, (String) -> Unit) -> Unit,
    onStartWorkManager: (String, String, Int, Int, Int, String?, (String) -> Unit) -> Unit
) {
    val context = LocalContext.current
    val sharedPreferences = remember {
        context.getSharedPreferences("udp_transfer_prefs", Context.MODE_PRIVATE)
    }
    
    // States
    var filePath by remember { mutableStateOf("") }
    var fileName by remember { mutableStateOf("demo_data.bin") }
    var demoContent by remember { mutableStateOf("Nội dung thử nghiệm của file demo truyền tải UDP.") }
    var serverIp by remember { mutableStateOf(sharedPreferences.getString("server_ip", "") ?: "") }
    var udpPortStr by remember { mutableStateOf(sharedPreferences.getString("udp_port", "5000") ?: "5000") }
    var httpPortStr by remember { mutableStateOf(sharedPreferences.getString("http_port", "8080") ?: "8080") }
    var blockSizeStr by remember { mutableStateOf(sharedPreferences.getString("block_size", "16384") ?: "16384") }
    var password by remember { mutableStateOf(sharedPreferences.getString("password", "") ?: "") }
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

        var isPasswordVisible by remember { mutableStateOf(false) }

        OutlinedTextField(
            value = serverIp,
            onValueChange = { 
                serverIp = it 
                sharedPreferences.edit().putString("server_ip", it).apply()
            },
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
                onValueChange = { 
                    udpPortStr = it 
                    sharedPreferences.edit().putString("udp_port", it).apply()
                },
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
                onValueChange = { 
                    httpPortStr = it 
                    sharedPreferences.edit().putString("http_port", it).apply()
                },
                label = { Text("Cổng HTTP") },
                keyboardOptions = KeyboardOptions(keyboardType = KeyboardType.Number),
                modifier = Modifier.weight(1f),
                colors = OutlinedTextFieldDefaults.colors(
                    focusedBorderColor = MaterialTheme.colorScheme.secondary,
                    unfocusedBorderColor = Color.Gray
                )
            )
        }

        OutlinedTextField(
            value = blockSizeStr,
            onValueChange = { 
                blockSizeStr = it 
                sharedPreferences.edit().putString("block_size", it).apply()
            },
            label = { Text("Kích thước Block (bytes)") },
            keyboardOptions = KeyboardOptions(keyboardType = KeyboardType.Number),
            modifier = Modifier.fillMaxWidth(),
            colors = OutlinedTextFieldDefaults.colors(
                focusedBorderColor = MaterialTheme.colorScheme.secondary,
                unfocusedBorderColor = Color.Gray
            )
        )

        OutlinedTextField(
            value = password,
            onValueChange = { 
                password = it 
                sharedPreferences.edit().putString("password", it).apply()
            },
            label = { Text("Mật khẩu (Tùy chọn)") },
            visualTransformation = if (isPasswordVisible) VisualTransformation.None else PasswordVisualTransformation(),
            trailingIcon = {
                TextButton(onClick = { isPasswordVisible = !isPasswordVisible }) {
                    Text(
                        text = if (isPasswordVisible) "Ẩn" else "Hiện",
                        color = MaterialTheme.colorScheme.secondary,
                        fontWeight = FontWeight.Bold,
                        fontSize = 12.sp
                    )
                }
            },
            modifier = Modifier.fillMaxWidth(),
            colors = OutlinedTextFieldDefaults.colors(
                focusedBorderColor = MaterialTheme.colorScheme.secondary,
                unfocusedBorderColor = Color.Gray
            )
        )

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

        Spacer(modifier = Modifier.height(16.dp))

        Row(
            modifier = Modifier.fillMaxWidth(),
            horizontalArrangement = Arrangement.SpaceBetween,
            verticalAlignment = Alignment.CenterVertically
        ) {
            Text(
                text = "3. LỊCH SỬ TẢI LÊN (${uploadHistory.size})",
                fontSize = 14.sp,
                fontWeight = FontWeight.Bold,
                color = MaterialTheme.colorScheme.secondary
            )
            if (uploadHistory.isNotEmpty()) {
                TextButton(onClick = onClearHistory) {
                    Text("Xóa lịch sử", color = Color.Red, fontSize = 12.sp)
                }
            }
        }

        if (uploadHistory.isEmpty()) {
            Card(
                modifier = Modifier.fillMaxWidth(),
                colors = CardDefaults.cardColors(containerColor = MaterialTheme.colorScheme.surface.copy(alpha = 0.5f))
            ) {
                Box(
                    modifier = Modifier
                        .fillMaxWidth()
                        .padding(24.dp),
                    contentAlignment = Alignment.Center
                ) {
                    Text("Chưa có file nào được tải lên", color = Color.Gray, fontSize = 14.sp)
                }
            }
        } else {
            Column(
                verticalArrangement = Arrangement.spacedBy(8.dp),
                modifier = Modifier.fillMaxWidth()
            ) {
                uploadHistory.forEach { item ->
                    Card(
                        modifier = Modifier.fillMaxWidth(),
                        colors = CardDefaults.cardColors(
                            containerColor = MaterialTheme.colorScheme.surface
                        ),
                        shape = RoundedCornerShape(8.dp)
                    ) {
                        Column(
                            modifier = Modifier.padding(12.dp),
                            verticalArrangement = Arrangement.spacedBy(4.dp)
                        ) {
                            Row(
                                modifier = Modifier.fillMaxWidth(),
                                horizontalArrangement = Arrangement.SpaceBetween,
                                verticalAlignment = Alignment.CenterVertically
                            ) {
                                Text(
                                    text = item.fileName,
                                    fontWeight = FontWeight.Bold,
                                    fontSize = 14.sp,
                                    color = if (item.isSuccess) Color(0xFF81C784) else Color(0xFFE57373),
                                    modifier = Modifier.weight(1f)
                                )
                                Text(
                                    text = formatFileSize(item.fileSize),
                                    fontSize = 12.sp,
                                    color = Color.Gray
                                )
                            }

                            Text(
                                text = "Thời gian: ${formatTimestamp(item.timestamp)}",
                                fontSize = 12.sp,
                                color = Color.LightGray
                            )

                            Text(
                                text = "Trạng thái: ${item.statusMsg}",
                                fontSize = 12.sp,
                                color = if (item.isSuccess) Color(0xFF81C784) else Color(0xFFE57373)
                            )

                            Row(
                                modifier = Modifier
                                    .fillMaxWidth()
                                    .background(Color(0xFF2E2E2E), RoundedCornerShape(4.dp))
                                    .padding(horizontal = 8.dp, vertical = 6.dp),
                                horizontalArrangement = Arrangement.SpaceBetween,
                                verticalAlignment = Alignment.CenterVertically
                            ) {
                                Column(modifier = Modifier.weight(1f)) {
                                    Text(
                                        text = "Hash ID (Khớp Backend):",
                                        fontSize = 10.sp,
                                        color = Color.Gray
                                    )
                                    Text(
                                        text = item.sha256,
                                        fontSize = 11.sp,
                                        color = Color.White,
                                        fontFamily = FontFamily.Monospace,
                                        maxLines = 1
                                    )
                                }
                                TextButton(
                                    onClick = {
                                        val clipboard = context.getSystemService(Context.CLIPBOARD_SERVICE) as ClipboardManager
                                        val clip = ClipData.newPlainText("Hash ID", item.sha256)
                                        clipboard.setPrimaryClip(clip)
                                        Toast.makeText(context, "Đã sao chép Hash ID!", Toast.LENGTH_SHORT).show()
                                    },
                                    contentPadding = PaddingValues(horizontal = 8.dp, vertical = 2.dp),
                                    modifier = Modifier.height(28.dp)
                                ) {
                                    Text("SAO CHÉP", fontSize = 10.sp, color = MaterialTheme.colorScheme.secondary)
                                }
                            }
                        }
                    }
                }
            }
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
