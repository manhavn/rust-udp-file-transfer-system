# 📡 RTK UDP File Transfer System

Hệ thống truyền tải file tốc độ cao và đáng tin cậy qua giao thức **UDP tùy chỉnh (Custom UDP)** kết hợp **REST API điều phối (HTTP)** và **Web Dashboard giám sát thời gian thực**, phát triển hoàn toàn bằng ngôn ngữ **Rust**.

---

## 1. Tính Năng Nổi Bật

*   **Giao thức UDP tùy chỉnh độc lập:** Sử dụng cơ chế phân tách các trường dữ liệu bằng byte phân tách `255u8`.
*   **Thuật toán mã hóa số thập phân tham lam (Greedy Digit Grouping):** Chuyển đổi các số lớn (`seek begin`, `mã gói tin`) sang mảng byte giá trị nhỏ hơn `255` để tránh xung đột với byte phân tách.
*   **Cơ chế truyền tin cậy Stop-and-Wait ARQ:**
    *   Tự động gửi lại (Retry) khối dữ liệu khi quá thời gian chờ (Timeout).
    *   Cơ chế lùi bước lũy thừa (Exponential Backoff) tránh gây nghẽn mạng.
    *   Xử lý trùng lặp gói tin và chống ghi đè lỗi trên Server (Idempotency).
*   **Cơ chế tải lên tiếp tục (Resumable Upload):** Cho phép tự động phát hiện và tiếp tục tải từ điểm ngắt kết nối trước đó (Checkpoint) được làm tròn về biên block để loại bỏ hoàn toàn khả năng sai lệch file.
*   **Lưu vết bền vững bằng SQLite:** Lịch sử truyền tệp được lưu giữ vào cơ sở dữ liệu `db/data.sqlite` và tự động nạp lại khi khởi động lại Server.
*   **Ghi dữ liệu dạng Giao dịch (Transactional Writes):** Trạng thái tiến trình chỉ được cập nhật khi hệ thống đã ghi thành công dữ liệu xuống ổ đĩa cứng vật lý.
*   **Tiến trình ngầm dọn dẹp (Cleanup Worker):** Tự động dọn dẹp tệp tin vật lý và logs trong cơ sở dữ liệu khi vượt quá thời gian lưu trữ cho phép. Ngoài ra, nó cũng tự động phát hiện và xóa các tệp tin không rõ nguồn gốc (không có trong cơ sở dữ liệu) trong thư mục uploads nếu chúng không được sửa đổi trong khoảng thời gian `--incomplete-timeout`.
*   **Đa nền tảng và Biên dịch chéo:** Biên dịch trực tiếp cho Windows CLI (`.exe`), Linux CLI, macOS CLI và thư viện liên kết động FFI `.so` cho Android (Kotlin/Java) qua JNA/JNI.

---

## 2. Cấu Trúc Thư Mục Dự Án

*   **`common/`**: Thư viện lõi chứa thuật toán mã hóa chữ số, cấu trúc và phân tích cú pháp gói tin `UdpPacket` & `AckPacket`.
*   **`server/`**: Server lắng nghe cổng UDP, REST API đăng ký và Dashboard quản lý thời gian thực.
*   **`client_cli/`**: Ứng dụng dòng lệnh (CLI) giúp tính mã băm SHA-256 của file và đẩy lên Server.
*   **`client_lib/`**: Thư viện động C-FFI dùng để tích hợp vào ứng dụng di động (Android Kotlin/Java).
*   **`scripts/`**: Các kịch bản chạy thử nghiệm, khởi động Server và Client cho từng OS cụ thể.
*   **`docs/`**: Tài liệu kỹ thuật chuyên sâu về cấu trúc gói tin, hướng dẫn tích hợp Android và tài liệu triển khai VPS.

---

## 3. Hướng Dẫn Biên Dịch Toàn Bộ Workspace

### 3.1. Biên dịch mặc định (Native)
Trước tiên, hãy chắc chắn bạn đã cài đặt Rust và Cargo trên hệ thống. Sau đó, chạy lệnh sau ở thư mục gốc của dự án:
```bash
cargo build --release
```
Các tệp nhị phân đầu ra sẽ nằm tại thư mục `target/release/`.

### 3.2. Biên dịch đa nền tảng và kiến trúc (Interactive Cross-Platform Build Scripts)
Để thuận tiện cho việc biên dịch và đóng gói lên các hệ điều hành và kiến trúc phần cứng khác nhau (Linux x86_64/aarch64, Windows x86_64, macOS x86_64/aarch64), bạn có thể sử dụng 2 kịch bản build tương tác ở thư mục gốc:

*   **Biên dịch Server:**
    ```bash
    ./build_server.sh
    ```
*   **Biên dịch Client CLI:**
    ```bash
    ./build_client.sh
    ```

Khi chạy, kịch bản sẽ hiển thị một menu để bạn lựa chọn môi trường đích mong muốn, tự động thêm target tương ứng thông qua `rustup` và tiến hành biên dịch ra file thực thi ở chế độ tối ưu (`--release`).

---

## 4. Hướng Dẫn Sử Dụng & Các Lệnh Ví Dụ (Examples)

### 4.1. Khởi chạy Server
Bạn có thể khởi chạy server sử dụng các cổng và cấu hình mặc định (UDP port 5000, HTTP port 8080, dọn dẹp tệp hoàn thành sau 15 phút, chưa hoàn thành sau 1 giờ):

*   **Linux/macOS:**
    ```bash
    ./scripts/run_server_linux.sh
    ```
*   **Windows:**
    ```batch
    scripts\run_server_windows.bat
    ```

#### Cấu hình tham số khởi chạy tùy chỉnh (Linh hoạt điều chỉnh):
Bạn có thể tùy ý điều chỉnh chu kỳ quét dọn dẹp, thời gian lưu giữ tệp hoàn thành hoặc chưa hoàn thành thông qua các tham số dòng lệnh:
*   `--cleanup-interval`: Chu kỳ quét dọn dẹp tệp tin (đơn vị: phút).
*   `--incomplete-timeout`: Thời gian tối đa lưu giữ tệp tải lên dang dở (đơn vị: phút).
*   `--completed-timeout`: Thời gian tối đa lưu giữ tệp đã hoàn thành (đơn vị: phút).
*   `--udp-port`: Cổng UDP để nhận dữ liệu.
*   `--http-port`: Cổng HTTP chạy dashboard REST API.
*   `--upload-dir`: Đường dẫn thư mục lưu trữ các tệp tải lên (mặc định: `./uploads`).
*   `--db-path`: Đường dẫn tệp cơ sở dữ liệu SQLite (mặc định: `./db/data.sqlite`).
*   `--disable-request-log`: Tắt toàn bộ output log request của HTTP server và các log tiến trình/lỗi của UDP server (mặc định server sẽ bật log).

**Ví dụ chạy Server (Quét dọn dẹp mỗi 3 phút, lưu tệp dở dang tối đa 30 phút, lưu tệp hoàn thành tối đa 20 phút):**
*   **Linux:**
    ```bash
    ./scripts/run_server_linux.sh --cleanup-interval 3 --incomplete-timeout 30 --completed-timeout 20
    ```
*   **macOS:**
    ```bash
    ./scripts/run_server_macos.sh --cleanup-interval 3 --incomplete-timeout 30 --completed-timeout 20
    ```
*   **Windows:**
    ```batch
    scripts\run_server_windows.bat --cleanup-interval 3 --incomplete-timeout 30 --completed-timeout 20
    ```

---

### 4.2. Gửi tệp tin lên Server (Client CLI)
Để gửi một tệp tin lên Server, hãy truyền đường dẫn tệp làm tham số đầu tiên, theo sau là cấu hình Server IP, cổng UDP và HTTP tùy chọn (mặc định gửi tới `127.0.0.1:5000` / `8080`):

*   **Linux:**
    ```bash
    ./scripts/send_file_linux.sh <đường_dẫn_file> [ip_server] [cổng_udp] [cổng_http]
    # Ví dụ:
    ./scripts/send_file_linux.sh video.mp4 127.0.0.1 5000 8080
    ```
*   **macOS:**
    ```bash
    ./scripts/send_file_macos.sh <đường_dẫn_file> [ip_server] [cổng_udp] [cổng_http]
    # Ví dụ:
    ./scripts/send_file_macos.sh video.mp4 127.0.0.1 5000 8080
    ```
*   **Windows:**
    ```batch
    scripts\send_file_windows.bat <đường_dẫn_file> [ip_server] [cổng_udp] [cổng_http]
    # Ví dụ:
    scripts\send_file_windows.bat video.mp4 127.0.0.1 5000 8080
    ```

---

### 4.3. Chạy ở môi trường Production (Production Run Scripts)
Để chạy các file thực thi đã biên dịch Release trực tiếp ở thư mục gốc (hoặc khi triển khai độc lập lên máy chủ), bạn có thể sử dụng các kịch bản chạy sau:

*   **Khởi chạy Server:**
    *   **Linux/macOS:**
        ```bash
        ./run_server.sh [các tham số cấu hình...]
        # Ví dụ:
        ./run_server.sh --cleanup-interval 5 --completed-timeout 25
        ```
    *   **Windows:**
        ```batch
        run_server.bat [các tham số cấu hình...]
        ```
*   **Khởi chạy Client CLI (Gửi file):**
    *   **Linux/macOS:**
        ```bash
        ./run_client.sh <đường_dẫn_file> [ip_server] [cổng_udp] [cổng_http]
        # Ví dụ:
        ./run_client.sh video.mp4 127.0.0.1 5000 8080
        ```
    *   **Windows:**
        ```batch
        run_client.bat <đường_dẫn_file> [ip_server] [cổng_udp] [cổng_http]
        ```

Các kịch bản này tự động phát hiện đường dẫn tệp thực thi release (trong thư mục `target/release/` hoặc cùng thư mục hiện tại nếu tệp chạy được sao chép độc lập), tự động tạo các thư mục cần thiết (`uploads/` và `db/`), rồi khởi chạy chương trình với đầy đủ tham số.

---

## 5. Chạy Demo Nhanh (Quick Start)
Để kiểm tra nhanh toàn bộ hệ thống (biên dịch, tạo tệp kiểm thử 1MB, chạy server ngầm và upload dữ liệu tự động), chạy lệnh:
```bash
./run_demo.sh
```
Sau đó truy cập **`http://localhost:8080`** trên trình duyệt để thưởng thức giao diện Dashboard trực quan.

---

## 6. Tài Liệu Kỹ Thuật Chuyên Sâu

Vui lòng tham khảo các tài liệu chuyên biệt nằm trong thư mục [`docs/`](docs/) để biết thêm chi tiết:
1.  **[Đặc tả Giao thức & Mã hóa (protocol_spec.md)](docs/protocol_spec.md)**
2.  **[Hướng dẫn Tích hợp Android Kotlin/Java (android_integration.md)](docs/android_integration.md)**
3.  **[Hướng dẫn Triển khai Production systemd & Nginx Proxy (deployment_guide.md)](docs/deployment_guide.md)**
