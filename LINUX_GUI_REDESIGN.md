# Linux Desktop GUI Redesign

Bản cập nhật này chỉ thay đổi giao diện desktop trong `client_gui/`. Thư mục `android-app/` không bị chỉnh sửa.

## Thay đổi chính

- Làm lại bố cục theo luồng sử dụng: chọn file -> cấu hình server -> gửi.
- Hiển thị file đã chọn (tên, dung lượng, đường dẫn) rõ ràng hơn.
- Đưa UDP port, HTTP port và block size vào mục "Cài đặt nâng cao" có thể thu gọn.
- Đưa công cụ tạo file demo vào mục thu gọn để giao diện chính gọn hơn.
- Thêm trạng thái theo ngữ nghĩa: sẵn sàng, đang xử lý, thành công và lỗi.
- Thêm thanh hoạt động indeterminate khi đang hash/upload (không giả lập % tiến trình).
- Chỉ bật nút gửi khi đã có file, server và cấu hình cổng hợp lệ.
- Thêm tóm tắt endpoint hiện tại.
- Cải thiện lịch sử: trạng thái, dung lượng, thời gian, hash, copy hash và chọn lại file cũ.
- Thêm phím tắt `Ctrl+O` để chọn file và `Ctrl+Enter` để gửi trực tiếp.
- Loại bỏ phụ thuộc Google Fonts qua mạng, dùng system font để hoạt động tốt khi offline.
- Cải thiện responsive cho cửa sổ hẹp.

## File đã thay đổi

- `client_gui/src/index.html`
- `client_gui/src/styles.css`
- `client_gui/src/main.js`
- `client_gui/src-tauri/tauri.conf.json`
- `client_gui/src-tauri/Cargo.toml`

## Chạy trên Linux

```bash
cd client_gui
npm install
npm run tauri dev
```

Build release:

```bash
npm run tauri build
```
