# Hướng Dẫn Tích Hợp Android App (Kotlin/Java)

Thư viện động C-FFI `client_lib` được thiết kế để tích hợp vào ứng dụng Android. File đầu ra sau khi biên dịch chéo sẽ là một thư viện động dạng `.so` (ví dụ: `libclient_lib.so`).

---

## 1. Hướng Dẫn Biên Dịch Chéo sang Android `.so`

### 1.1. Cài đặt các Target của Android NDK trong Rust
Để biên dịch ra thư viện phù hợp với các cấu trúc CPU của thiết bị Android, hãy chạy các lệnh sau trên máy tính của bạn:

```bash
rustup target add aarch64-linux-android      # Kiến trúc ARM64 (hầu hết thiết bị hiện nay)
rustup target add armv7-linux-androideabi    # Kiến trúc ARM 32-bit (thiết bị cũ)
rustup target add x86_64-linux-android        # Giả lập Android Studio x64
rustup target add i686-linux-android          # Giả lập Android Studio x86
```

### 1.2. Cài đặt công cụ hỗ trợ `cargo-ndk`
`cargo-ndk` tự động cấu hình các biến môi trường cần thiết từ Android NDK để biên dịch dự án Rust:
```bash
cargo install cargo-ndk
```

### 1.3. Thực Hiện Biên Dịch
Trước khi build, hãy chắc chắn rằng bạn đã tải Android NDK trong Android Studio và cấu hình đường dẫn `ANDROID_NDK_HOME` hoặc `ANDROID_HOME` trong biến môi trường hệ thống.

Biên dịch cho thiết bị thật chạy ARM64:
```bash
cargo ndk -t aarch64-linux-android build --release -p client_lib
```
Đường dẫn tệp đầu ra:
`target/aarch64-linux-android/release/libclient_lib.so`

---

## 2. Tích Hợp Vào Dự Án Android (Android Studio)

### 2.1. Đặt tệp `.so` vào dự án
Tạo cấu trúc thư mục và đặt tệp `.so` tương ứng vào dự án Android của bạn:
```
[Tên dự án Android]
 └── app/
      └── src/
           └── main/
                └── jniLibs/
                     ├── arm64-v8a/
                     │    └── libclient_lib.so
                     └── armeabi-v7a/
                          └── libclient_lib.so
```

---

## 3. Cách Gọi Thư Viện Rust từ Kotlin

### Phương Pháp 1: Sử Dụng JNA (Java Native Access - Khuyên Dùng)
JNA cho phép gọi trực tiếp hàm FFI từ Rust mà không cần viết mã trung gian C/JNI phức tạp trên cả 2 phía.

1.  Thêm dependency JNA vào file `app/build.gradle`:
    ```groovy
    dependencies {
        implementation 'net.java.dev.jna:jna:5.13.0@aar'
    }
    ```

2.  Định nghĩa Interface trong Kotlin:
    ```kotlin
    package com.example.myapp

    import com.sun.jna.Library
    import com.sun.jna.Native

    interface RustUploaderLib : Library {
        /**
         * Hàm FFI xuất bản từ Rust.
         * Trả về: 0 nếu thành công, số âm đại diện cho mã lỗi.
         */
        fun rtk_upload_file(
            filePath: String,
            serverIp: String,
            udpPort: Short,
            httpPort: Short,
            blockSize: Long
        ): Int

        companion object {
            val INSTANCE: RustUploaderLib = Native.load("client_lib", RustUploaderLib::class.java) as RustUploaderLib
        }
    }
    ```

3.  Gọi thực thi trên Background Thread:
    ```kotlin
    import kotlinx.coroutines.Dispatchers
    import kotlinx.coroutines.withContext

    suspend fun performUpload(
        localPath: String, 
        serverIp: String, 
        udpPort: Int, 
        httpPort: Int
    ): Int = withContext(Dispatchers.IO) {
        
        // Gọi hàm Rust
        val resultCode = RustUploaderLib.INSTANCE.rtk_upload_file(
            filePath = localPath,
            serverIp = serverIp,
            udpPort = udpPort.toShort(),
            httpPort = httpPort.toShort(),
            blockSize = 16384L // 16KB
        )
        
        return@withContext resultCode
    }
    ```

---

### Phương Pháp 2: Sử Dụng JNI (Truyền Thống)
Nếu bạn không muốn sử dụng JNA, bạn phải viết interface JNI như sau:

1.  Trong `client_lib/src/lib.rs`, thêm hàm JNI (sử dụng crate `jni`):
    ```rust
    // Cần thêm jni = "0.21" vào client_lib/Cargo.toml
    #[no_mangle]
    pub unsafe extern "system" fn Java_com_example_myapp_RustUploader_rtkUploadFile(
        mut env: jni::JNIEnv,
        _class: jni::objects::JClass,
        file_path: jni::objects::JString,
        server_ip: jni::objects::JString,
        udp_port: jni::sys::jint,
        http_port: jni::sys::jint,
        block_size: jni::sys::jint,
    ) -> jni::sys::jint {
        let file_path_str: String = env.get_string(&file_path).unwrap().into();
        let server_ip_str: String = env.get_string(&server_ip).unwrap().into();

        // Gọi logic truyền tải nội bộ...
        0
    }
    ```

2.  Trong Kotlin, khai báo Class Native:
    ```kotlin
    package com.example.myapp

    class RustUploader {
        companion object {
            init {
                System.loadLibrary("client_lib")
            }
        }

        // Tên hàm khớp chính xác với định nghĩa JNI phía Rust
        external fun rtkUploadFile(
            filePath: String,
            serverIp: String,
            udpPort: Int,
            httpPort: Int,
            blockSize: Int
        ): Int
    }
    ```
