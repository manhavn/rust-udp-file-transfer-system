# Tài Liệu Hệ Thống Truyền Tải File UDP (RTK UDP Transfer)

Chào mừng bạn đến với thư mục tài liệu triển khai và đánh giá của hệ thống. Dưới đây là các hướng dẫn chi tiết được chia theo từng chủ đề để phục vụ quá trình phát triển, tích hợp và triển khai thực tế.

## Mục Lục Tài Liệu

1. **[Đặc Tả Giao Thức Tùy Chỉnh (Custom UDP Protocol)](protocol_spec.md)**
   * Mô tả cấu trúc các gói tin dữ liệu (`UdpPacket`) và phản hồi (`AckPacket`).
   * Thuật toán mã hóa số thập phân tham lam (Greedy Digit Grouping) để loại bỏ byte phân tách `255u8`.
   * Nguyên lý hoạt động của cơ chế Stop-and-Wait ARQ để đảm bảo truyền tin cậy qua UDP.

2. **[Tích Hợp Ứng Dụng Android (Android App Integration)](android_integration.md)**
   * Hướng dẫn biên dịch chéo thư viện động `.so` (`client_lib`) bằng `cargo-ndk`.
   * Cách liên kết Rust FFI sang Kotlin/Java bằng hai phương pháp JNA (khuyên dùng) và JNI (truyền thống).
   * Các mẫu code Kotlin hoàn chỉnh cho background threads.

3. **[Hướng Dẫn Triển Khai Server & Biên Dịch Chéo (Deployment & Cross-Compilation Guide)](deployment_guide.md)**
   * Cách thiết lập Server chạy dưới dạng dịch vụ hệ thống (`systemd`) trên Linux.
   * Cấu hình tường lửa cho cổng UDP và HTTP.
   * Hướng dẫn biên dịch chéo Client CLI cho Windows (tạo file `.exe`), macOS và Linux.

4. **[Tổng Quan Hệ Thống (udp_transfer_system.md)](udp_transfer_system.md)**
   * Bản báo cáo thiết kế tổng thể kiến trúc, giao thức và các cách tích hợp nhanh.
