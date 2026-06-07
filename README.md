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

**Ví dụ chạy Server với cấu hình đầy đủ tất cả tham số:**
*   **Linux:**
    ```bash
    ./scripts/run_server_linux.sh --udp-port 5000 --http-port 8080 --upload-dir ./uploads --db-path ./db/data.sqlite --cleanup-interval 3 --incomplete-timeout 30 --completed-timeout 20 --disable-request-log
    ```
*   **macOS:**
    ```bash
    ./scripts/run_server_macos.sh --udp-port 5000 --http-port 8080 --upload-dir ./uploads --db-path ./db/data.sqlite --cleanup-interval 3 --incomplete-timeout 30 --completed-timeout 20 --disable-request-log
    ```
*   **Windows:**
    ```batch
    scripts\run_server_windows.bat --udp-port 5000 --http-port 8080 --upload-dir .\uploads --db-path .\db\data.sqlite --cleanup-interval 3 --incomplete-timeout 30 --completed-timeout 20 --disable-request-log
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

#### Các tham số cấu hình của Client CLI:
*   `--server-ip` / `-s`: Địa chỉ IP của Server (mặc định: `127.0.0.1`).
*   `--udp-port` / `-u`: Cổng UDP của Server (mặc định: `5000`).
*   `--http-port` / `-t`: Cổng HTTP của Server (mặc định: `8080`).
*   `--block-size` / `-b`: Kích thước mỗi khối dữ liệu UDP gửi đi tính bằng bytes (mặc định: `16384` bytes).
*   `--log-progress`: Hiển thị tiến trình upload dưới dạng log dòng mới (mặc định: tắt, hiển thị bằng `\r`).
*   `--password` / `-p`: Mật khẩu bảo mật tải xuống dành cho file này (mặc định: không có).

#### Ví dụ gửi tệp tin với cấu hình đầy đủ tất cả tham số (Khởi chạy trực tiếp file thực thi):
*   **Linux/macOS:**
    *   **Không sử dụng mật khẩu:**
        ```bash
        ./target/release/client_cli video.mp4 --server-ip 127.0.0.1 --udp-port 5000 --http-port 8080 --block-size 16384 --log-progress
        ```
    *   **Có sử dụng mật khẩu:**
        ```bash
        ./target/release/client_cli video.mp4 --server-ip 127.0.0.1 --udp-port 5000 --http-port 8080 --block-size 16384 --log-progress --password mysecret123
        ```
*   **Windows (Command Prompt):**
    *   **Không sử dụng mật khẩu:**
        ```cmd
        target\release\client_cli.exe video.mp4 --server-ip 127.0.0.1 --udp-port 5000 --http-port 8080 --block-size 16384 --log-progress
        ```
    *   **Có sử dụng mật khẩu:**
        ```cmd
        target\release\client_cli.exe video.mp4 --server-ip 127.0.0.1 --udp-port 5000 --http-port 8080 --block-size 16384 --log-progress --password mysecret123
        ```

---

### 4.3. Chạy ở môi trường Production (Production Run Scripts)
Để chạy các file thực thi đã biên dịch Release trực tiếp ở thư mục gốc (hoặc khi triển khai độc lập lên máy chủ), bạn có thể sử dụng các kịch bản chạy sau:

*   **Khởi chạy Server:**
    *   **Linux/macOS:**
        ```bash
        ./run_server.sh [các tham số cấu hình...]
        # Ví dụ với đầy đủ tham số:
        ./run_server.sh --udp-port 5000 --http-port 8080 --upload-dir ./uploads --db-path ./db/data.sqlite --cleanup-interval 5 --incomplete-timeout 60 --completed-timeout 15 --disable-request-log
        ```
    *   **Windows:**
        ```batch
        run_server.bat [các tham số cấu hình...]
        # Ví dụ với đầy đủ tham số:
        run_server.bat --udp-port 5000 --http-port 8080 --upload-dir .\uploads --db-path .\db\data.sqlite --cleanup-interval 5 --incomplete-timeout 60 --completed-timeout 15 --disable-request-log
        ```
*   **Khởi chạy Client CLI (Gửi file):**
    *   **Linux/macOS:**
        ```bash
        # Ví dụ chạy nhanh sử dụng script (các tham số cấu hình vị trí):
        ./run_client.sh video.mp4 127.0.0.1 5000 8080
        # Ví dụ chạy trực tiếp file thực thi với đầy đủ tất cả tham số:
        ./target/release/client_cli video.mp4 --server-ip 127.0.0.1 --udp-port 5000 --http-port 8080 --block-size 16384 --log-progress
        ```
    *   **Windows:**
        ```batch
        # Ví dụ chạy nhanh sử dụng script (các tham số cấu hình vị trí):
        run_client.bat video.mp4 127.0.0.1 5000 8080
        # Ví dụ chạy trực tiếp file thực thi với đầy đủ tất cả tham số:
        target\release\client_cli.exe video.mp4 --server-ip 127.0.0.1 --udp-port 5000 --http-port 8080 --block-size 16384 --log-progress
        ```

Các kịch bản này tự động phát hiện đường dẫn tệp thực thi release (trong thư mục `target/release/` hoặc cùng thư mục hiện tại nếu tệp chạy được sao chép độc lập), tự động tạo các thư mục cần thiết (`uploads/` và `db/`), rồi khởi chạy chương trình với đầy đủ tham số.

---

### 4.4. Cấu hình qua Biến Môi Trường (Environment Variables)
Cả Server và Client đều hỗ trợ đọc các giá trị cấu hình từ biến môi trường của hệ thống. Điều này vô cùng tiện lợi khi chạy ứng dụng thông qua Docker, Podman hoặc Docker Compose.

#### Các biến môi trường hỗ trợ cho Server:
| Biến môi trường | Tham số CLI tương ứng | Giá trị mặc định | Mô tả |
| :--- | :--- | :--- | :--- |
| `UDP_PORT` | `--udp-port`, `-u` | `5000` | Cổng UDP nhận dữ liệu |
| `HTTP_PORT` | `--http-port`, `-h` | `8080` | Cổng HTTP REST API & Dashboard |
| `UPLOAD_DIR` | `--upload-dir` | `./uploads` | Thư mục lưu trữ tệp tin trên disk |
| `DB_PATH` | `--db-path` | `./db/data.sqlite` | Đường dẫn file cơ sở dữ liệu SQLite |
| `CLEANUP_INTERVAL` | `--cleanup-interval` | `5` | Chu kỳ quét dọn dẹp (phút) |
| `INCOMPLETE_TIMEOUT` | `--incomplete-timeout` | `60` | Hạn lưu tệp chưa xong (phút) |
| `COMPLETED_TIMEOUT` | `--completed-timeout` | `15` | Hạn lưu tệp đã xong (phút) |
| `DISABLE_REQUEST_LOG` | `--disable-request-log` | `false` | Tắt toàn bộ logs HTTP/UDP |

#### Các biến môi trường hỗ trợ cho Client CLI:
| Biến môi trường | Tham số CLI tương ứng | Giá trị mặc định | Mô tả |
| :--- | :--- | :--- | :--- |
| `FILE_PATH` | `<đường_dẫn_file>` (vị trí số 1) | (Bắt buộc) | Đường dẫn tới tệp tin cần upload |
| `SERVER_IP` | `--server-ip`, `-s` | `127.0.0.1` | Địa chỉ IP của Server |
| `UDP_PORT` | `--udp-port`, `-u` | `5000` | Cổng UDP của Server |
| `HTTP_PORT` | `--http-port`, `-t` | `8080` | Cổng HTTP của Server |
| `BLOCK_SIZE` | `--block-size`, `-b` | `16384` | Kích thước khối UDP gửi đi (bytes) |
| `LOG_PROGRESS` | `--log-progress` | `false` | Bật/tắt in log dòng mới |
| `DOWNLOAD_PASSWORD` | `--password`, `-p` | (Không có) | Mật khẩu bảo mật khi tải file xuống |

> [!NOTE]
> Các tham số được truyền trực tiếp qua dòng lệnh (CLI Parameters) sẽ luôn có **độ ưu tiên cao nhất** và ghi đè lên các giá trị cấu hình được thiết lập trong biến môi trường.

---

### 4.5. Khởi chạy với Docker hoặc Podman (Container Deployment)
Hệ thống hỗ trợ đóng gói và chạy Server cũng như Client trên bất kỳ môi trường container nào.

Để tối ưu hóa thời gian biên dịch, hệ thống áp dụng cơ chế **Cache Base Images** tương tự như quy trình triển khai của các hệ thống Production lớn. Gói cài đặt các package hệ thống (`musl-dev`, `build-base` cho builder và `ca-certificates` cho runtime) sẽ được đóng gói một lần thành các tệp `.tar` cục bộ. Trong những lần build tiếp theo, Docker/Podman sẽ tự động nạp lại từ tệp tar này mà không cần truy vấn tải lại từ internet.

#### 1. Xây dựng Docker Image (Build):
Bạn có thể build tự động qua script (khuyên dùng) hoặc build thủ công:

*   **Cách 1: Khởi chạy biên dịch tự động qua Script Cache (Khuyên dùng):**
    Script sẽ tự động phát hiện Docker/Podman, tạo và lưu cache base images (`builder.Dockerfile` và `runtime.Dockerfile` dưới dạng `.tar` ẩn), rồi hiển thị menu để build Server/Client tương ứng:
    ```bash
    ./build_container.sh
    ```

*   **Cách 2: Biên dịch thủ công từng bước (Nếu không dùng script tự động):**
    Bạn cần phải tự xây dựng các base image và cache image theo đúng thứ tự (thay thế `docker` bằng `podman` nếu cần):
    ```bash
    # Bước 1: Build các base image
    docker build -f builder.Dockerfile -t rtk.builder/base:latest .
    docker build -f runtime.Dockerfile -t rtk.runtime/base:latest .

    # Bước 2: Build cache dependencies trung gian (Chạy cargo build dependencies)
    docker build -f dep-cache.Dockerfile -t rtk.app/dep-cache:latest .
    # (Tùy chọn) Lưu cache tar để tái sử dụng lần sau:
    docker save rtk.app/dep-cache:latest -o .rtk-dep-cache.tar

    # Bước 3: Build Server Image thành phẩm và xuất ra file tar
    docker build -f server.Dockerfile -t rtk.udp/server .
    docker save rtk.udp/server -o rtk-udp-server.tar
    
    # Bước 4: Build Client Image thành phẩm và xuất ra file tar
    docker build -f client.Dockerfile -t rtk.udp/client .
    docker save rtk.udp/client -o rtk-udp-client.tar

    # Bước 5: Dọn dẹp các image để giải phóng dung lượng đĩa
    docker rmi rtk.udp/server rtk.udp/client rtk.app/dep-cache:latest rtk.builder/base:latest rtk.runtime/base:latest
    docker image prune -f
    ```

> [!IMPORTANT]
> **Lưu ý về dung lượng đĩa:** Sau khi chạy biên dịch tự động qua `./build_container.sh`, toàn bộ các image build trung gian và image thành phẩm đều sẽ được tự động xóa khỏi Docker/Podman Engine để tiết kiệm dung lượng đĩa tối đa (chỉ giữ lại cục bộ dưới dạng các tệp `.tar`).
>
> Trước khi chạy các lệnh khởi chạy dưới đây, bạn cần nạp lại image mong muốn từ tệp `.tar` tương ứng:
> *   **Nạp Server Image:**
>     *   Docker: `docker load -i rtk-udp-server.tar`
>     *   Podman: `podman load -i rtk-udp-server.tar`
> *   **Nạp Client Image:**
>     *   Docker: `docker load -i rtk-udp-client.tar`
>     *   Podman: `podman load -i rtk-udp-client.tar`

#### 2. Khởi chạy Container (Run):
Để đơn giản hóa việc khởi chạy (như tự động nạp ảnh từ tệp `.tar`, ánh xạ các thư mục volume, cấu hình cổng và chuyển đổi đường dẫn tuyệt đối cho tệp gửi), hệ thống cung cấp sẵn các kịch bản chạy trong các thư mục `docker/` và `podman/`.

##### Cách 1: Sử dụng các kịch bản chạy tự động (Khuyên dùng):
Các kịch bản này sẽ tự động kiểm tra và nạp ảnh từ tệp `.tar` nếu chưa có trong registry, tự động ánh xạ ổ đĩa volume và chuyển đổi đường dẫn file gửi tương ứng.

*   **Sử dụng Docker:**
    *   **Khởi chạy Server:**
        ```bash
        ./docker/run_server.sh [các tham số cấu hình bổ sung...]
        # Ví dụ với đầy đủ tham số:
        ./docker/run_server.sh --udp-port 5000 --http-port 8080 --disable-request-log
        ```
    *   **Khởi chạy Client (Gửi file):**
        ```bash
        ./docker/run_client.sh <đường_dẫn_file> [các tham số CLI bổ sung...]
        # Ví dụ gửi tệp tin với cấu hình đầy đủ:
        ./docker/run_client.sh video.mp4 --server-ip 192.168.1.100 --udp-port 5000 --http-port 8080 --block-size 16384 --log-progress
        ```
*   **Sử dụng Podman:**
    *   **Khởi chạy Server:**
        ```bash
        ./podman/run_server.sh [các tham số cấu hình bổ sung...]
        # Ví dụ với đầy đủ tham số:
        ./podman/run_server.sh --udp-port 5000 --http-port 8080 --disable-request-log
        ```
    *   **Khởi chạy Client (Gửi file):**
        ```bash
        ./podman/run_client.sh <đường_dẫn_file> [các tham số CLI bổ sung...]
        # Ví dụ gửi tệp tin với cấu hình đầy đủ:
        ./podman/run_client.sh video.mp4 --server-ip 192.168.1.100 --udp-port 5000 --http-port 8080 --block-size 16384 --log-progress
        ```

##### Cách 2: Khởi chạy bằng lệnh Docker/Podman thô (Thủ công):
*(Lưu ý: Bạn phải nạp ảnh từ tệp `.tar` trước bằng lệnh `docker load -i <file.tar>` hoặc `podman load -i <file.tar>` trước khi chạy các lệnh này).*

*   **Khởi chạy Server mặc định:**
    *   **Docker:**
        ```bash
        docker run -d \
          --name rtk-server \
          -p 5000:5000/udp \
          -p 8080:8080/tcp \
          -v $(pwd)/uploads:/app/uploads \
          -v $(pwd)/db:/app/db \
          rtk.udp/server
        ```
    *   **Podman:**
        ```bash
        podman run -d \
          --name rtk-server \
          -p 5000:5000/udp \
          -p 8080:8080/tcp \
          -v $(pwd)/uploads:/app/uploads:Z \
          -v $(pwd)/db:/app/db:Z \
          rtk.udp/server
        ```
        *(Lưu ý đối với Podman trên các hệ thống Linux bật SELinux, hậu tố `:Z` là bắt buộc để phân quyền volume).*

*   **Khởi chạy Server với cấu hình đầy đủ tất cả các biến môi trường (Full ENV):**
    *   **Docker:**
        ```bash
        docker run -d \
          --name rtk-server \
          -p 5005:5005/udp \
          -p 8085:8085/tcp \
          -e UDP_PORT=5005 \
          -e HTTP_PORT=8085 \
          -e UPLOAD_DIR=/app/uploads \
          -e DB_PATH=/app/db/data.sqlite \
          -e CLEANUP_INTERVAL=5 \
          -e INCOMPLETE_TIMEOUT=60 \
          -e COMPLETED_TIMEOUT=15 \
          -e DISABLE_REQUEST_LOG=false \
          -v $(pwd)/uploads:/app/uploads \
          -v $(pwd)/db:/app/db \
          rtk.udp/server
        ```
    *   **Podman:**
        ```bash
        podman run -d \
          --name rtk-server \
          -p 5005:5005/udp \
          -p 8085:8085/tcp \
          -e UDP_PORT=5005 \
          -e HTTP_PORT=8085 \
          -e UPLOAD_DIR=/app/uploads \
          -e DB_PATH=/app/db/data.sqlite \
          -e CLEANUP_INTERVAL=5 \
          -e INCOMPLETE_TIMEOUT=60 \
          -e COMPLETED_TIMEOUT=15 \
          -e DISABLE_REQUEST_LOG=false \
          -v $(pwd)/uploads:/app/uploads:Z \
          -v $(pwd)/db:/app/db:Z \
          rtk.udp/server
        ```

#### 3. Khởi chạy Client bằng lệnh Container thô (Gửi file):
Vì tệp tin cần gửi nằm trên máy Host, bạn cần gắn kết (Volume Mount) tệp tin hoặc thư mục chứa tệp tin vào trong Container. Nên sử dụng chế độ chỉ đọc (`:ro`) để bảo vệ dữ liệu gốc trên máy host.

*   **Sử dụng Docker:**
    *   **Truyền tham số qua dòng lệnh (CLI):**
        *   *Không mật khẩu:*
            ```bash
            docker run --rm -it -v $(pwd):/data:ro rtk.udp/client /data/video.mp4 --server-ip 192.168.1.100 --udp-port 5000 --http-port 8080 --block-size 16384 --log-progress
            ```
        *   *Có mật khẩu:*
            ```bash
            docker run --rm -it -v $(pwd):/data:ro rtk.udp/client /data/video.mp4 --server-ip 192.168.1.100 --udp-port 5000 --http-port 8080 --block-size 16384 --log-progress --password mysecret123
            ```
    *   **Truyền cấu hình qua biến môi trường (ENV):**
        *   *Không mật khẩu:*
            ```bash
            docker run --rm -it \
              -v $(pwd):/data:ro \
              -e FILE_PATH=/data/video.mp4 \
              -e SERVER_IP=192.168.1.100 \
              -e UDP_PORT=5000 \
              -e HTTP_PORT=8080 \
              -e BLOCK_SIZE=16384 \
              -e LOG_PROGRESS=true \
              rtk.udp/client
            ```
        *   *Có mật khẩu:*
            ```bash
            docker run --rm -it \
              -v $(pwd):/data:ro \
              -e FILE_PATH=/data/video.mp4 \
              -e SERVER_IP=192.168.1.100 \
              -e UDP_PORT=5000 \
              -e HTTP_PORT=8080 \
              -e BLOCK_SIZE=16384 \
              -e LOG_PROGRESS=true \
              -e DOWNLOAD_PASSWORD=mysecret123 \
              rtk.udp/client
            ```

*   **Sử dụng Podman (sử dụng thêm nhãn `:Z` để gán nhãn SELinux phù hợp):**
    *   **Truyền tham số qua dòng lệnh (CLI):**
        *   *Không mật khẩu:*
            ```bash
            podman run --rm -it -v $(pwd):/data:ro,Z rtk.udp/client /data/video.mp4 --server-ip 192.168.1.100 --udp-port 5000 --http-port 8080 --block-size 16384 --log-progress
            ```
        *   *Có mật khẩu:*
            ```bash
            podman run --rm -it -v $(pwd):/data:ro,Z rtk.udp/client /data/video.mp4 --server-ip 192.168.1.100 --udp-port 5000 --http-port 8080 --block-size 16384 --log-progress --password mysecret123
            ```
    *   **Truyền cấu hình qua biến môi trường (ENV):**
        *   *Không mật khẩu:*
            ```bash
            podman run --rm -it \
              -v $(pwd):/data:ro,Z \
              -e FILE_PATH=/data/video.mp4 \
              -e SERVER_IP=192.168.1.100 \
              -e UDP_PORT=5000 \
              -e HTTP_PORT=8080 \
              -e BLOCK_SIZE=16384 \
              -e LOG_PROGRESS=true \
              rtk.udp/client
            ```
        *   *Có mật khẩu:*
            ```bash
            podman run --rm -it \
              -v $(pwd):/data:ro,Z \
              -e FILE_PATH=/data/video.mp4 \
              -e SERVER_IP=192.168.1.100 \
              -e UDP_PORT=5000 \
              -e HTTP_PORT=8080 \
              -e BLOCK_SIZE=16384 \
              -e LOG_PROGRESS=true \
              -e DOWNLOAD_PASSWORD=mysecret123 \
              rtk.udp/client
            ```

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
