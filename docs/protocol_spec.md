# Đặc Tả Giao Thức UDP Truyền Tải File Tùy Chỉnh

Giao thức này sử dụng byte phân tách `255u8` để phân đoạn các trường dữ liệu trong gói tin UDP. Các trường số như `mã gói tin` và `seek begin` được gửi dưới dạng mảng byte chứa các giá trị từ `0` đến `254`.

---

## 1. Cấu Trúc Gói Tin UDP

### 1.1. Gói Dữ Liệu Gửi (Client $\rightarrow$ Server)
Gói dữ liệu gửi có cấu trúc tuần tự như sau:

| Tên Trường | Kích Thước | Kiểu Dữ Liệu | Giá Trị / Mô Tả |
| :--- | :--- | :--- | :--- |
| **Trạng Thái** | 1 byte | `u8` | `1` (Đang gửi block dữ liệu) hoặc `0` (Kết thúc truyền tải) |
| **Phân Tách 1** | 1 byte | `u8` | Cố định `255` |
| **Mã Gói Tin** | Biến đổi (vL) | `&[u8]` | Mảng byte đại diện cho mã định danh file (không chứa byte `255`) |
| **Phân Tách 2** | 1 byte | `u8` | Cố định `255` |
| **Seek Begin** | Biến đổi (vL) | `&[u8]` | Offset ghi dữ liệu trong file, mã hóa theo quy tắc Greedy (không chứa byte `255`) |
| **Phân Tách 3** | 1 byte | `u8` | Cố định `255` |
| **Dữ Liệu** | Biến đổi (vL) | `&[u8]` | Khối dữ liệu thô (khoảng 16KB). Để trống đối với gói kết thúc (`status = 0`) |

---

### 1.2. Gói Phản Hồi ACK (Server $\rightarrow$ Client)
Gói phản hồi ACK được gửi ngược lại để xác nhận ghi thành công:

| Tên Trường | Kích Thước | Kiểu Dữ Liệu | Giá Trị / Mô Tả |
| :--- | :--- | :--- | :--- |
| **Mã Gói Tin** | Biến đổi (vL) | `&[u8]` | Trả lại mã gói tin nhận được từ client |
| **Phân Tách 1** | 1 byte | `u8` | Cố định `255` |
| **Seek Begin** | Biến đổi (vL) | `&[u8]` | Trả lại offset đã nhận và ghi thành công |
| **Phân Tách 2** | 1 byte | `u8` | Cố định `255` |
| **Số Byte Đã Nhận**| Biến đổi (vL) | `&[u8]` | Số byte dữ liệu thô ghi nhận được trong block vừa rồi (báo `0` nếu là gói kết thúc) |
| **Phân Tách 3** | 1 byte | `u8` | Cố định `255` |

---

## 2. Thuật Toán Mã Hóa Số Thập Phân (Greedy Grouping)

Để truyền tải các số nguyên lớn (`u64` hoặc `u128`) qua các trường byte có giá trị giới hạn từ `0..=254` (tránh đụng độ với byte phân tách `255`), hệ thống sử dụng thuật toán phân tách chuỗi thập phân từ trái qua phải theo cơ chế tham lam.

### Mã Nguồn Triển Khai (Rust)
Hàm mã hóa phân tích chuỗi số thập phân thành các cụm chữ số có giá trị nhỏ hơn `255`:
```rust
pub fn encode_number_to_bytes(val: u64) -> Vec<u8> {
    let s = val.to_string();
    let mut bytes = Vec::new();
    let mut chars = s.as_bytes();

    while !chars.is_empty() {
        if chars[0] == b'0' {
            bytes.push(0);
            chars = &chars[1..];
        } else {
            let mut taken = false;
            // Thử lấy cụm 3 chữ số
            if chars.len() >= 3 {
                if let Ok(chunk_str) = std::str::from_utf8(&chars[..3]) {
                    if let Ok(num) = chunk_str.parse::<u16>() {
                        if num <= 254 {
                            bytes.push(num as u8);
                            chars = &chars[3..];
                            taken = true;
                        }
                    }
                }
            }
            // Thử lấy cụm 2 chữ số
            if !taken && chars.len() >= 2 {
                if let Ok(chunk_str) = std::str::from_utf8(&chars[..2]) {
                    if let Ok(num) = chunk_str.parse::<u8>() {
                        if num <= 254 {
                            bytes.push(num);
                            chars = &chars[2..];
                            taken = true;
                        }
                    }
                }
            }
            // Lấy 1 chữ số
            if !taken {
                let num = (chars[0] - b'0') as u8;
                bytes.push(num);
                chars = &chars[1..];
            }
        }
    }
    bytes
}
```

Hàm giải mã (chuyển đổi ngược lại):
```rust
pub fn decode_bytes_to_number(bytes: &[u8]) -> Result<u64, String> {
    let s: String = bytes.iter().map(|b| b.to_string()).collect();
    s.parse::<u64>().map_err(|e| format!("Lỗi đổi sang u64: {}", e))
}
```

---

## 3. Cơ Chế Truyền Tải Tin Cậy (Reliable Stop-and-Wait ARQ)

Hệ thống bảo đảm tính toàn vẹn của file thông qua cơ chế tự động gửi lại (ARQ):
1.  **Gửi & Chờ ACK:** Client gửi khối dữ liệu (ví dụ 16KB) tại `seek_begin` nhất định và chờ gói ACK từ Server trong thời gian quy định (mặc định `150ms`).
2.  **Khớp dữ liệu ACK:** Khi nhận được ACK, Client kiểm tra xem `mã gói tin`, `seek_begin` và `số byte đã nhận` có khớp chính xác với gói tin đã gửi hay không.
3.  **Gửi lại khi hết hạn (Timeout & Retry):** Nếu quá `150ms` mà chưa có ACK hợp lệ, Client sẽ tự động gửi lại khối dữ liệu này. Quá trình gửi lại hỗ trợ lùi bước lũy thừa (backoff delay) để tránh làm nghẽn mạng cục bộ.
4.  **Tính lũy thừa (Idempotency) trên Server:** Do Server ghi dữ liệu vào file bằng cách tìm đến vị trí tuyệt đối (`SeekFrom::Start(seek_begin)`) trước khi ghi, nếu một gói tin bị trùng lặp (ví dụ do ACK bị mất trên đường truyền về Client), Server chỉ ghi đè lại đúng vùng dữ liệu đó. Điều này giúp loại bỏ rủi ro dữ liệu bị nhân đôi hoặc sai lệch.
