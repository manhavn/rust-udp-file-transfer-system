# Hướng Dẫn Chạy & Biên Dịch Ứng Dụng Desktop GUI (Tauri v2)

Thư mục này chứa ứng dụng Desktop UI cho **Linux, macOS và Windows** được viết bằng **Tauri v2** kết hợp giao diện HTML/CSS/JS (Dark Mode). Thiết kế giao diện và chức năng được tối ưu hóa tương tự như phiên bản ứng dụng Android.

---

## 1. Yêu Cầu Hệ Thống (Prerequisites)

Tauri biên dịch thành ứng dụng native nên cần các thư viện hệ thống tương ứng trên từng hệ điều hành:

### A. Linux (Ubuntu / Debian)
Chạy lệnh sau trong Terminal để cài đặt các thư viện đồ họa và Webview:
```bash
sudo apt-get update
sudo apt-get install -y \
  libwebkit2gtk-4.1-dev \
  build-essential \
  curl \
  wget \
  file \
  libssl-dev \
  libgtk-3-dev \
  libayatana-appindicator3-dev \
  librsvg2-dev
```

### B. macOS
Cài đặt **Xcode Command Line Tools** bằng cách chạy lệnh:
```bash
xcode-select --install
```

### C. Windows
1. Cài đặt **Microsoft C++ Build Tools** thông qua Visual Studio Installer (chọn mục *Desktop development with C++*).
2. Đảm bảo máy tính đã cài đặt **WebView2 Runtime** (thường đã có sẵn trên Windows 10/11).

---

## 2. Hướng Dẫn Cài Đặt & Khởi Chạy

Chạy các lệnh sau tại thư mục `client_gui`:

### Bước 1: Cài đặt các thư viện Node.js
```bash
npm install
```

### Bước 2: Khởi chạy chế độ phát triển (Development Mode)
```bash
npm run tauri dev
```
Lệnh này sẽ mở ứng dụng dạng cửa sổ Desktop độc lập, tự động reload khi bạn chỉnh sửa file frontend (`src/`) hoặc backend (`src-tauri/`).

### Bước 3: Đóng gói ứng dụng (Production Build)
```bash
npm run tauri build
```
Tauri sẽ biên dịch code Rust tối ưu hóa và xuất ra file cài đặt native tương ứng với hệ điều hành của bạn:
* **Linux**: File `.deb` hoặc `AppImage` tại `src-tauri/target/release/bundle/`
* **Windows**: File `.msi` hoặc `.exe`
* **macOS**: File `.app` hoặc `.dmg`

---

## 3. Cấu Trúc Mã Nguồn Chính

* **[index.html](src/index.html)**: Khung HTML xây dựng bố cục ứng dụng bao gồm 3 phần chính giống Android:
  1. *Trạng thái (Status Card)*: Hiển thị Live Log trạng thái tải lên hoặc lỗi.
  2. *Nguồn File (File Source)*: Cho phép chọn file từ máy tính, viết nội dung file thử nghiệm, hoặc tự động sinh file demo.
  3. *Cấu Hình Server & Action Buttons*: Thiết lập địa chỉ IP, các cổng UDP/HTTP, kích thước block dữ liệu, mật khẩu bảo mật, và 2 nút tải lên (Trực tiếp hoặc Chạy ngầm).
* **[styles.css](src/styles.css)**: Định nghĩa giao diện Dark Mode cao cấp với hiệu ứng gradient, bo góc, và animation khi di chuột.
* **[main.js](src/main.js)**: Logic frontend xử lý sự kiện, lưu trữ cấu hình mạng & lịch sử tải lên qua `LocalStorage`, và gọi các API Rust của Tauri.
* **[lib.rs](src-tauri/src/lib.rs)**: Mã nguồn backend Rust tích hợp trực tiếp thư viện lõi `client_lib` trong Workspace để xử lý các tác vụ băm file (`calculate_hash`) và truyền tải dữ liệu (`perform_upload`) một cách bất tuần tự (asynchronous).
