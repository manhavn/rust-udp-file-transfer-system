# Hệ Thống Truyền Tải File UDP Tin Cậy (RTK UDP Transfer System)

Hệ thống được phát triển hoàn toàn bằng **Rust**, bao gồm một **Server** (UDP Receiver + HTTP Dashboard), một **Client CLI** (Windows, macOS, Linux), và một thư viện FFI **Client Lib** (dành cho Android app và tích hợp hệ thống khác).

---

## 1. Kiến Trúc Giao Thức (Custom UDP Protocol)

Để đáp ứng việc truyền tải qua UDP không tin cậy (mất gói, trùng lặp hoặc đảo lộn thứ tự), hệ thống triển khai giao thức tự cấu trúc với thuật toán **Stop-and-Wait ARQ** cùng cơ chế băm và định danh số duy nhất.

### 1.1. Cấu Trúc Gói Tin UDP (Client gửi lên Server)
Mỗi gói dữ liệu gửi lên được ngăn cách bởi byte phân tách `255u8`. Các trường số (`seek begin`, `mã gói tin`) được biến đổi sang dạng chuỗi chữ số thập phân rồi chuyển thành mảng byte có giá trị nhỏ hơn `255` (để tránh đụng độ với byte phân tách).

Gói tin gồm các thành phần liên tiếp:
```
┌─────────────────┬─────┬─────────────────┬─────┬─────────────────┬─────┬─────────────┐
│ Trạng thái (1B) │ 255 │ Mã gói tin (vL) │ 255 │ Seek Begin (vL) │ 255 │ Dữ liệu (vL)│
└─────────────────┴─────┴─────────────────┴─────┴─────────────────┴─────┴─────────────┘
```

*   **Trạng thái gửi (1 byte):**
    *   `1`: Đang gửi dữ liệu (Data block).
    *   `0`: Báo hiệu kết thúc truyền tải thành công (End/Finished).
*   **Mã gói tin (độ dài biến đổi):**
    *   Mã băm SHA-256 của file được đọc lúc bắt đầu, lấy 10 byte đầu tiên, chuyển đổi mỗi byte qua phép chia lấy dư `byte % 255` (đảm bảo không chứa byte `255`).
    *   *Số định danh duy nhất:* Là chuỗi nối liền các giá trị thập phân của mảng byte này (Ví dụ: `[100, 25, 5, 31, 40, 254, 0, 62, 55, 56]` -> `"10025531402540625556"`).
*   **Seek Begin (độ dài biến đổi):**
    *   Vị trí (offset) bắt đầu ghi dữ liệu trong file (đơn vị: byte).
    *   Được phân rã bằng thuật toán tham lam (greedy) từ trái qua phải để tạo mảng byte dưới `255` (Ví dụ: `16384` -> `[163, 84]`).
*   **Dữ liệu (độ dài biến đổi):**
    *   Dữ liệu thô của file (khoảng 16KB ở mỗi block). Đối với gói kết thúc (`status = 0`), phần dữ liệu này để trống.

---

### 1.2. Thuật Toán Mã Hóa Số Thập Phân (Greedy Digit Grouping)
Do yêu cầu mã gói tin và seek begin phải được gửi dưới dạng byte nhưng không được chứa byte `255`, hệ thống sử dụng thuật toán gom nhóm chữ số từ trái qua phải:

1.  Nếu chữ số hiện tại là `'0'`, ghi nhận byte `0` và tiến tới chữ số tiếp theo.
2.  Thử lấy cụm 3 chữ số tiếp theo. Nếu giá trị cụm này $\le 254$ và không có số 0 ở đầu (trừ khi là chính nó), đẩy giá trị này vào mảng byte và tiến lên 3 chữ số.
3.  Nếu không thỏa mãn cụm 3 chữ số, thử lấy cụm 2 chữ số. Nếu $\le 254$, đẩy vào mảng byte và tiến lên 2 chữ số.
4.  Nếu không được, lấy duy nhất 1 chữ số hiện tại.

#### Ví dụ Thực Tế từ Khách Hàng:
*   `16384` $\rightarrow$ Tách thành `163` và `84` (mảng byte: `[163, 84]`).
    *   *Kiểm tra ngược:* `"163" + "84" = "16384"`.
*   `1020085001163` $\rightarrow$ Tách thành `102`, `0`, `0`, `85`, `0`, `0`, `116`, `3` (mảng byte: `[102, 0, 0, 85, 0, 0, 116, 3]`).
    *   *Kiểm tra ngược:* `"102" + "0" + "0" + "85" + "0" + "0" + "116" + "3" = "1020085001163"`.

---

### 1.3. Gói Phản Hồi ACK (Server trả về Client)
Để xác thực gói tin đã đến đích an toàn, Server gửi phản hồi ACK có cấu trúc tương tự:
```
┌─────────────────┬─────┬─────────────────┬─────┬───────────────────────┬─────┐
│ Mã gói tin (vL) │ 255 │ Seek Begin (vL) │ 255 │ Số byte đã nhận (vL)   │ 255 │
└─────────────────┴─────┴─────────────────┴─────┴───────────────────────┴─────┘
```

---

## 2. Cấu Trúc Thư Mục Dự Án

Dự án được tổ chức theo mô hình Rust Workspace để dễ quản lý:
*   [common/](file:///home/dev/Desktop/rust-projects/udp/common): Chứa mã nguồn chia sẻ (thuật toán mã hóa/giải mã số, định nghĩa và phân tích gói tin `UdpPacket` / `AckPacket`).
*   [server/](file:///home/dev/Desktop/rust-projects/udp/server): Server lắng nghe UDP (cổng 5000), HTTP API đăng ký truyền tải (cổng 8080) và giao diện Dashboard giám sát thời gian thực.
*   [client_cli/](file:///home/dev/Desktop/rust-projects/udp/client_cli): Khởi chạy truyền file từ dòng lệnh dành cho Windows, Linux, macOS.
*   [client_lib/](file:///home/dev/Desktop/rust-projects/udp/client_lib): Thư viện động (`.so` / `.dll` / `.dylib`) chứa lõi truyền tải, cung cấp hàm FFI cho ứng dụng Android (Kotlin/Java) hoặc các nền tảng khác.

---

## 3. Hướng Dẫn Biên Dịch & Chạy Demo (Linux/macOS/Windows)

### 3.1. Chạy Demo Nhanh trên Máy Hiện Tại (Linux/macOS)
Chúng tôi đã chuẩn bị sẵn script chạy thử nghiệm toàn bộ quy trình:
```bash
./run_demo.sh
```
Script này tự động:
1.  Biên dịch Server và Client ở chế độ tối ưu (`--release`).
2.  Tạo file dữ liệu thử nghiệm `demo_data.bin` dung lượng 1MB.
3.  Khởi chạy Server chạy ngầm.
4.  Chạy Client CLI để upload dữ liệu lên Server.
5.  Giữ Server hoạt động để bạn kiểm tra giao diện Dashboard.

> [!TIP]
> Sau khi chạy demo, hãy mở trình duyệt và truy cập **`http://localhost:8080`** để xem Dashboard thời gian thực cực kỳ mượt mà với hiệu ứng Glassmorphism và chế độ Dark Mode! Bạn có thể trực tiếp nhấn nút **Tải về** trên giao diện để nhận lại file đã hoàn thành.

---

### 3.2. Biên Dịch Đa Nền Tảng (Cross-Compilation)
Nhờ tính chất độc lập của Rust, bạn có thể dễ dàng biên dịch ra các file thực thi cho các hệ điều hành khác nhau:

#### Biên dịch cho Linux (Linux CLI):
```bash
cargo build --release --bin client_cli
# File thực thi nằm tại: target/release/client_cli
```

#### Biên dịch cho Windows CLI (chạy trên Windows):
Nếu đang ở Linux, bạn cần cài đặt target `x86_64-pc-windows-gnu`:
```bash
rustup target add x86_64-pc-windows-gnu
cargo build --release --target x86_64-pc-windows-gnu --bin client_cli
# File thực thi client_cli.exe nằm tại: target/x86_64-pc-windows-gnu/release/client_cli.exe
```

#### Biên dịch cho macOS CLI (chạy trên máy Mac):
```bash
rustup target add aarch64-apple-darwin # Cho Apple Silicon (M1/M2/M3)
# Hoặc x86_64-apple-darwin cho Intel Macs
cargo build --release --target aarch64-apple-darwin --bin client_cli
```

---

## 4. Tích Hợp Android App (Kotlin/Java)

Thư viện [client_lib](file:///home/dev/Desktop/rust-projects/udp/client_lib) xuất bản hàm liên kết C-FFI sau:
```rust
pub extern "C" fn rtk_upload_file(
    c_file_path: *const c_char,
    c_server_ip: *const c_char,
    udp_port: u16,
    http_port: u16,
    block_size: usize,
) -> i32
```

### 4.1. Biên Dịch Thư Viện `.so` Cho Android
Bạn cần cài đặt Android NDK trên máy phát triển và công cụ `cargo-ndk` để biên dịch chéo sang các kiến trúc CPU của Android:

```bash
# 1. Cài đặt các target Android của Rust
rustup target add aarch64-linux-android      # Thiết bị Android 64-bit hiện đại (phổ biến nhất)
rustup target add armv7-linux-androideabi    # Thiết bị Android 32-bit cũ
rustup target add i686-linux-android          # Giả lập Android x86
rustup target add x86_64-linux-android        # Giả lập Android x64

# 2. Cài đặt cargo-ndk
cargo install cargo-ndk

# 3. Biên dịch thư viện cho ARM64 (aarch64)
cargo ndk -t aarch64-linux-android build --release -p client_lib
```
Sau khi build xong, file thư viện Android của bạn sẽ nằm tại:
`target/aarch64-linux-android/release/libclient_lib.so`

---

### 4.2. Tích Hợp Vào Android App (Android Studio)
Sao chép file `.so` đã biên dịch vào thư mục dự án Android của bạn:
`app/src/main/jniLibs/arm64-v8a/libclient_lib.so`

Bạn có hai cách để gọi thư viện này từ Kotlin/Java:

#### Cách 1: Sử dụng JNA (Java Native Access - Khuyên Dùng vì cực kỳ đơn giản)
Thêm dependency JNA vào `build.gradle` của Android:
```groovy
implementation 'net.java.dev.jna:jna:5.13.0@aar'
```

Định nghĩa interface liên kết trong Kotlin:
```kotlin
import com.sun.jna.Library
import com.sun.jna.Native

interface RustUploaderLib : Library {
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

Gọi hàm upload trong ứng dụng của bạn:
```kotlin
// Chạy trên Background Thread (ví dụ IO Dispatcher hoặc AsyncTask) để không bị block UI
val result = RustUploaderLib.INSTANCE.rtk_upload_file(
    "/sdcard/Download/video.mp4",
    "192.168.1.50",
    5000.toShort(),
    8080.toShort(),
    16384L // 16KB block size
)

if (result == 0) {
    Log.d("UDP_UPLOAD", "Tải lên thành công!")
} else {
    Log.e("UDP_UPLOAD", "Lỗi tải lên, mã lỗi: $result")
}
```

---

#### Cách 2: Sử dụng JNI (Java Native Interface truyền thống)
Nếu bạn không muốn sử dụng thư viện bên thứ ba như JNA, bạn có thể triển khai hàm JNI trực tiếp.

1.  Thêm hàm JNI trực tiếp trong `client_lib/src/lib.rs` (cần thêm `jni = "0.21"` vào dependencies):
    ```rust
    #[no_mangle]
    pub unsafe extern "system" fn Java_com_example_myapp_Uploader_rtkUploadFile(
        mut env: jni::JNIEnv,
        _class: jni::objects::JClass,
        file_path: jni::objects::JString,
        server_ip: jni::objects::JString,
        udp_port: jni::sys::jint,
        http_port: jni::sys::jint,
        block_size: jni::sys::jint,
    ) -> jni::sys::jint {
        let file_path_raw: String = env.get_string(&file_path).unwrap().into();
        let server_ip_raw: String = env.get_string(&server_ip).unwrap().into();

        // Gọi logic upload nội bộ tương tự như trên
        // ...
        0 // Trả về mã thành công
    }
    ```

2.  Khai báo lớp Kotlin tương ứng trong package `com.example.myapp`:
    ```kotlin
    package com.example.myapp

    class Uploader {
        companion object {
            init {
                System.loadLibrary("client_lib")
            }
        }

        external fun rtkUploadFile(
            filePath: String,
            serverIp: String,
            udpPort: Int,
            httpPort: Int,
            blockSize: Int
        ): Int
    }
    ```
