# Hướng Dẫn Tích Hợp Android (Kotlin / Java)

Thư mục này chứa toàn bộ dự án Android mẫu (`android-app`) tích hợp lõi truyền tải file UDP (`client_lib`) của Rust.

---

## 1. Biên Dịch Thư Viện `.so` & Build APK
Bạn chỉ cần đứng ở thư mục gốc của dự án Rust và chạy một trong các script sau:
* **Build ở chế độ Debug**: `./build_debug.sh`
* **Build ở chế độ Release (Đã ký)**: `./build_release.sh`

Các script này tự động:
1. Biên dịch thư viện Rust cho cả 4 kiến trúc CPU của Android.
2. Sao chép và tổ chức các tệp `.so` trực tiếp vào thư mục `app/src/main/jniLibs/` dưới dạng:
   * `arm64-v8a/libclient_lib.so` (Điện thoại ARM 64-bit hiện đại)
   * `armeabi-v7a/libclient_lib.so` (Các máy ARM 32-bit cũ)
   * `x86_64/libclient_lib.so` (Giả lập Android Studio x64)
   * `x86/libclient_lib.so` (Giả lập Android Studio x86)
3. Chạy Gradle để biên dịch ứng dụng tạo thành file APK cài đặt tại `app/build/outputs/apk/`.

---

## 2. Cấu Trúc File Tích Hợp Chính Trong App

Mã nguồn tích hợp FFI đã được viết sẵn tại:
1. **[RustUploaderLib.kt](app/src/main/java/com/rustcore/RustUploaderLib.kt)**: Định nghĩa interface JNA để tải thư viện động và map các hàm Rust `rtk_upload_file`, đồng thời cung cấp helper `RustUploader` chạy bất tuần tự qua Kotlin Coroutines.
2. **[UploadWorker.kt](app/src/main/java/com/rustcore/UploadWorker.kt)**: Triển khai Jetpack `WorkManager` giúp chạy tác vụ tải lên an toàn dưới dạng background worker mà không lo bị hệ điều hành tắt tiến trình.
3. **[MainActivity.kt](app/src/main/java/com/rustcore/MainActivity.kt)**: Giao diện người dùng Dark Mode Compose cho phép chọn file từ bộ nhớ thiết bị, nhập cấu hình, viết nội dung file test demo và xem tiến trình upload live.

---

## 3. Các Cấu Hình Gradle & Manifest Cần Thiết

### 3.1 Dependencies (`app/build.gradle`)
Đảm bảo đã bao gồm JNA và WorkManager:
```groovy
dependencies {
    // REQUIRED FOR JNA FFI LOOKUP
    implementation "net.java.dev.jna:jna:5.13.0@aar"
    
    // WorkManager for background upload tasks
    implementation "androidx.work:work-runtime-ktx:2.9.0"
}
```

### 3.2 Quyền Hạn (`app/src/main/AndroidManifest.xml`)
Đảm bảo đã yêu cầu quyền kết nối mạng và đọc bộ nhớ thiết bị:
```xml
<manifest xmlns:android="http://schemas.android.com/apk/res/android">
    <uses-permission android:name="android.permission.INTERNET" />
    <uses-permission android:name="android.permission.READ_EXTERNAL_STORAGE" />
</manifest>
```

---

## 4. Ví Dụ Sử Dụng (Usage Examples)

### Cách A: Gọi trực tiếp từ Coroutine (Chạy trong App)
```kotlin
import androidx.lifecycle.lifecycleScope
import kotlinx.coroutines.launch
import com.filetransfersystem.RustUploader

// Gọi từ Fragment hoặc Activity
lifecycleScope.launch {
    val result = RustUploader.performUpload(
        filePath = "/storage/emulated/0/Download/test_video.mp4",
        serverIp = "192.168.1.100",
        udpPort = 5000,
        httpPort = 8080,
        blockSize = 16384, // 16KB
        password = "my_secure_password" // Đổi thành null nếu không dùng mật khẩu
    )
    
    if (result == 0) {
        // Tải lên thành công!
    } else {
        // Thất bại, kiểm tra mã lỗi (ví dụ: -4 là lỗi API HTTP, -7 là lỗi kết nối UDP...)
    }
}
```

### Cách B: Gọi bằng WorkManager (Khuyên Dùng cho File Lớn)
Sử dụng `UploadWorker` đã tạo sẵn để đảm bảo tiến trình truyền tải không bị hệ điều hành Android kill:

```kotlin
import androidx.work.OneTimeWorkRequestBuilder
import androidx.work.WorkManager
import androidx.work.workDataOf
import com.filetransfersystem.UploadWorker

fun startBackgroundUpload(context: Context, filePath: String, serverIp: String) {
    val uploadWorkRequest = OneTimeWorkRequestBuilder<UploadWorker>()
        .setInputData(
            workDataOf(
                UploadWorker.KEY_FILE_PATH to filePath,
                UploadWorker.KEY_SERVER_IP to serverIp,
                UploadWorker.KEY_UDP_PORT to 5000,
                UploadWorker.KEY_HTTP_PORT to 8080,
                UploadWorker.KEY_BLOCK_SIZE to 16384,
                UploadWorker.KEY_PASSWORD to "optional_password"
            )
        )
        .build()

    WorkManager.getInstance(context).enqueue(uploadWorkRequest)
}
```
