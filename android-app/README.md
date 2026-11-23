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
2. **[UploadForegroundService.kt](app/src/main/java/com/filetransfersystem/UploadForegroundService.kt)**: Foreground Service loại `dataSync` sở hữu tiến trình upload, vì vậy việc chuyển sang ứng dụng khác không làm upload phụ thuộc vào `Activity`.
3. **[UploadServiceState.kt](app/src/main/java/com/filetransfersystem/UploadServiceState.kt)**: Lưu và phát lại trạng thái upload để UI kết nối lại sau khi Activity được tạo lại.
4. **[MainActivity.kt](app/src/main/java/com/filetransfersystem/MainActivity.kt)**: Giao diện Jetpack Compose Material 3: chọn file, cấu hình server, bắt đầu upload nền, theo dõi tiến độ và lịch sử.
5. **[UploadWorker.kt](app/src/main/java/com/filetransfersystem/UploadWorker.kt)**: Giữ lại như implementation WorkManager/fallback cũ; luồng upload do người dùng bấm gửi hiện dùng Foreground Service trực tiếp.

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
File được chọn thông qua Android Storage Access Framework (`OpenDocument`), vì vậy không cần quyền đọc toàn bộ bộ nhớ. Để upload tiếp tục khi app bị thu nhỏ, app dùng Foreground Service `dataSync` và notification:
```xml
<manifest xmlns:android="http://schemas.android.com/apk/res/android">
    <uses-permission android:name="android.permission.INTERNET" />
    <uses-permission android:name="android.permission.POST_NOTIFICATIONS" />
    <uses-permission android:name="android.permission.FOREGROUND_SERVICE" />
    <uses-permission android:name="android.permission.FOREGROUND_SERVICE_DATA_SYNC" />
</manifest>
```
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
        password = "my_secure_password", // Đổi thành null nếu không dùng mật khẩu
        onProgress = { progress ->
            println("${progress.percent}% - ${progress.bytesPerSecond} B/s - ETA ${progress.etaSeconds}s")
        }
    )
    
    if (result == 0) {
        // Tải lên thành công!
    } else {
        // Thất bại, kiểm tra mã lỗi (ví dụ: -4 là lỗi API HTTP, -7 là lỗi kết nối UDP...)
    }
}
```

### Cách B: Upload nền giống ứng dụng cloud drive (mặc định của app)

`MainActivity` gọi `UploadForegroundService.start(...)` ngay khi người dùng bấm **GỬI FILE**. Service hiển thị notification tiến độ và tiếp tục chạy khi Activity bị đưa xuống background. Xem [BACKGROUND_UPLOAD.md](BACKGROUND_UPLOAD.md) để biết luồng lifecycle, resume và cách kiểm thử.


---

## 5. Tiến độ truyền thực tế

Bản này nhận progress từ ACK thực tế của server và hiển thị phần trăm, dung lượng đã truyền, tốc độ và ETA. Xem `PROGRESS.md` để biết chi tiết luồng callback Rust → JNA → Compose/WorkManager.
