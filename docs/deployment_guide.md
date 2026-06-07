# Hướng Dẫn Triển Khai Server & Biên Dịch Chéo CLI

Hướng dẫn này chỉ ra các bước để triển khai Server lên môi trường production trên Linux (sử dụng `systemd`) và cách biên dịch chéo Client CLI cho các hệ điều hành khác nhau.

---

## 1. Triển Khai Server Trên Production (Linux VPS)

Để server hoạt động liên tục ở chế độ chạy ngầm và tự động khởi động cùng hệ thống, bạn nên cấu hình nó dưới dạng một dịch vụ `systemd`.

### 1.1. Cấu Hình Dịch Vụ Systemd
Tạo một tệp cấu hình service tại `/etc/systemd/system/rtk-udp-server.service`:

```ini
[Unit]
Description=RTK UDP File Transfer Server
After=network.target

[Service]
Type=simple
WorkingDirectory=/opt/rtk-udp-server
ExecStart=/opt/rtk-udp-server/server
Restart=always
RestartSec=5
StandardOutput=syslog
StandardError=syslog
SyslogIdentifier=rtk-udp-server

[Install]
WantedBy=multi-user.target
```

### 1.2. Kích Hoạt & Khởi Chạy
Di chuyển file chạy của server (`target/release/server`) vào thư mục `/opt/rtk-udp-server/server` và phân quyền chạy:
```bash
sudo mkdir -p /opt/rtk-udp-server
sudo cp target/release/server /opt/rtk-udp-server/
sudo chmod +x /opt/rtk-udp-server/server

# Nạp lại systemd daemon
sudo systemctl daemon-reload

# Kích hoạt tự khởi động và chạy dịch vụ
sudo systemctl enable rtk-udp-server
sudo systemctl start rtk-udp-server

# Kiểm tra trạng thái hoạt động
sudo systemctl status rtk-udp-server
```

---

## 2. Cấu Hình Tường Lửa (Firewall Configuration)

Server chạy đồng thời 2 cổng dịch vụ:
*   **Cổng `5000` (UDP):** Dành cho việc truyền dữ liệu tệp tin.
*   **Cổng `8080` (TCP):** Giao diện Dashboard HTTP và REST API điều phối.

Bạn cần mở hai cổng này trên tường lửa của máy chủ:

#### Trên Ubuntu (sử dụng UFW):
```bash
sudo ufw allow 5000/udp
sudo ufw allow 8080/tcp
sudo ufw reload
```

#### Trên CentOS / RHEL (sử dụng firewalld):
```bash
sudo firewall-cmd --zone=public --add-port=5000/udp --permanent
sudo firewall-cmd --zone=public --add-port=8080/tcp --permanent
sudo firewall-cmd --reload
```

---

## 3. Cấu Hình Reverse Proxy (HTTPS cho Dashboard)

Để bảo vệ đường dẫn Dashboard và mã băm truyền tải, bạn nên chạy Proxy bảo mật (SSL/TLS) bằng **Nginx** hoặc **Caddy**.

### Ví dụ Cấu Hình Nginx (`/etc/nginx/sites-available/rtk-transfer`):
```nginx
server {
    listen 80;
    server_name transfer.domain.com;

    location / {
        proxy_pass http://127.0.0.1:8080;
        proxy_set_header Host $host;
        proxy_set_header X-Real-IP $remote_addr;
        proxy_set_header X-Forwarded-For $proxy_add_x_forwarded_for;
        proxy_set_header X-Forwarded-Proto $scheme;
    }
}
```
*Sau đó sử dụng Certbot để tự động lấy chứng chỉ Let's Encrypt SSL.*

---

## 4. Biên Dịch Chéo CLI Cho Các Hệ Điều Hành

Ứng dụng Client CLI có thể biên dịch trực tiếp trên Linux thành các tệp thực thi cho Windows, macOS hoặc các kiến trúc Linux khác.

### 4.1. Tạo File Chạy Cho Windows (.exe)
Để tạo tệp thực thi Windows từ Linux, cài đặt toolchain Windows:
```bash
# Cài đặt target Windows GNU
rustup target add x86_64-pc-windows-gnu

# Cài đặt bộ liên kết (linker) Mingw-w64
sudo apt-get install mingw-w64 -y

# Biên dịch release
cargo build --release --target x86_64-pc-windows-gnu --bin client_cli
```
*Tệp tin xuất ra nằm tại:* `target/x86_64-pc-windows-gnu/release/client_cli.exe`

### 4.2. Tạo File Chạy Cho macOS
Đối với macOS, tốt nhất bạn nên biên dịch trực tiếp trên máy Mac (hoặc cấu hình toolchain Apple SDK nếu build trên Linux):
```bash
# Thêm target cho chip Apple Silicon (M1/M2/M3)
rustup target add aarch64-apple-darwin

# Thêm target cho chip Intel Mac
rustup target add x86_64-apple-darwin

# Biên dịch
cargo build --release --target aarch64-apple-darwin --bin client_cli
```
*Tệp tin xuất ra nằm tại:* `target/aarch64-apple-darwin/release/client_cli`
