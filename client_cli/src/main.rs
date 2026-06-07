use std::fs::File;
use std::io::{Read, Seek, SeekFrom};
use std::net::SocketAddr;
use std::path::Path;
use clap::Parser;
use tokio::net::UdpSocket;
use tokio::time::{timeout, Duration};

#[derive(Parser, Debug)]
#[command(name = "rtk-client")]
#[command(about = "Client truyền tải file qua UDP đáng tin cậy", long_about = None)]
struct Args {
    /// Đường dẫn tới file cần upload
    file: String,

    /// Địa chỉ IP của Server
    #[arg(short, long, default_value = "127.0.0.1")]
    server_ip: String,

    /// Cổng UDP của Server
    #[arg(short, long, default_value_t = 5000)]
    udp_port: u16,

    /// Cổng HTTP của Server
    #[arg(short = 't', long, default_value_t = 8080)]
    http_port: u16,

    /// Kích thước mỗi khối dữ liệu UDP gửi đi (bytes)
    #[arg(short, long, default_value_t = 16384)]
    block_size: usize,

    /// Hiển thị tiến trình upload dạng log dòng mới
    #[arg(long, default_value_t = false)]
    log_progress: bool,
}

async fn send_chunk_with_retry(
    socket: &UdpSocket,
    server_addr: SocketAddr,
    status: u8,
    packet_code: &[u8],
    seek_begin: u64,
    data: &[u8],
) -> Result<(), String> {
    let pkt_bytes = common::UdpPacket::serialize(status, packet_code, seek_begin, data);
    let mut retry_count = 0;
    const MAX_RETRIES: usize = 30;
    const TIMEOUT_DUR: Duration = Duration::from_millis(150);

    let expected_bytes_received = if status == 0 { 0 } else { data.len() as u64 };

    loop {
        // Send packet
        if let Err(e) = socket.send_to(&pkt_bytes, server_addr).await {
            eprintln!("[Cảnh báo] Gửi gói tin thất bại: {}", e);
        }

        // Wait for ACK
        let mut ack_buf = vec![0u8; 1024];
        match timeout(TIMEOUT_DUR, socket.recv_from(&mut ack_buf)).await {
            Ok(Ok((len, _src))) => {
                match common::AckPacket::parse(&ack_buf[..len]) {
                    Ok(ack) => {
                        if ack.packet_code == packet_code
                            && ack.seek_begin == seek_begin
                            && ack.bytes_received == expected_bytes_received
                        {
                            // Success
                            return Ok(());
                        } else {
                            eprintln!(
                                "[Cảnh báo] Nhận sai ACK (Khớp mã: {}, Seek: {} vs {}, Nhận: {} vs {}). Đang thử lại...",
                                ack.packet_code == packet_code,
                                ack.seek_begin, seek_begin,
                                ack.bytes_received, expected_bytes_received
                            );
                        }
                    }
                    Err(e) => {
                        eprintln!("[Cảnh báo] Lỗi phân tích ACK: {}. Đang thử lại...", e);
                    }
                }
            }
            Ok(Err(e)) => {
                eprintln!("[Cảnh báo] Lỗi socket nhận: {}. Đang thử lại...", e);
            }
            Err(_) => {
                // Timeout, silently retry
            }
        }

        retry_count += 1;
        if retry_count > MAX_RETRIES {
            return Err(format!(
                "Thất bại khi truyền gói tin tại offset {} sau {} lần thử lại.",
                seek_begin, MAX_RETRIES
            ));
        }

        // Backoff delay
        tokio::time::sleep(Duration::from_millis(5 * (retry_count as u64).min(10))).await;
    }
}

#[tokio::main]
async fn main() -> Result<(), String> {
    let args = Args::parse();

    let file_path = Path::new(&args.file);
    if !file_path.exists() {
        return Err(format!("File không tồn tại: {}", args.file));
    }

    println!("1. Đang tính toán mã băm XXH3 của file...");
    let mut file = File::open(&args.file).map_err(|e| format!("Không thể mở file: {}", e))?;
    let mut hasher = xxhash_rust::xxh3::Xxh3::new();
    let mut hash_buf = vec![0u8; 65536];
    loop {
        let n = file.read(&mut hash_buf).map_err(|e| format!("Lỗi đọc file khi băm: {}", e))?;
        if n == 0 {
            break;
        }
        hasher.update(&hash_buf[..n]);
    }
    let hash_result = hasher.digest();
    let hash_bytes = hash_result.to_be_bytes();

    // Generate unique packet code bytes
    let packet_code_bytes = common::generate_packet_code_from_hash(&hash_bytes);
    let packet_code_str = common::bytes_to_unique_id(&packet_code_bytes);
    println!("   -> Mã gói tin (Hash ID): {}", packet_code_str);

    let file_size = file.metadata().map_err(|e| format!("Lỗi đọc metadata: {}", e))?.len();
    let file_name = file_path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("unknown")
        .to_string();

    let mut seek_begin = 0u64;

    // 2. HTTP Register
    let client = reqwest::Client::new();
    let register_url = format!("http://{}:{}/api/register", args.server_ip, args.http_port);
    println!("2. Đang đăng ký file qua HTTP API...");
    let reg_res = client
        .post(&register_url)
        .json(&serde_json::json!({
            "packet_code": packet_code_str,
            "file_name": file_name,
            "file_size": file_size,
        }))
        .send()
        .await;

    match reg_res {
        Ok(resp) => {
            if resp.status().is_success() {
                println!("   -> Đăng ký thành công trên Server.");
                if let Ok(json) = resp.json::<serde_json::Value>().await {
                    if let Some(offset) = json.get("bytes_received").and_then(|v| v.as_u64()) {
                        if offset > 0 {
                            // Align down to block size to ensure any partially written block is completely overwritten
                            let block_size = args.block_size as u64;
                            let aligned_offset = (offset / block_size) * block_size;
                            if aligned_offset > 0 {
                                println!("   -> Phát hiện tệp tải lên dang dở. Sẽ tiếp tục truyền từ byte thứ {} (làm tròn từ {}, {:.2}%)", aligned_offset, offset, (aligned_offset as f64 / file_size as f64) * 100.0);
                                seek_begin = aligned_offset;
                            } else {
                                println!("   -> Phát hiện tệp tải lên dang dở nhưng nhỏ hơn kích thước block ({} bytes). Sẽ tải lên lại từ đầu.", offset);
                            }
                        }
                    }
                }
            } else {
                eprintln!("   -> [Cảnh báo] Đăng ký HTTP không thành công: Code {}", resp.status());
            }
        }
        Err(e) => {
            eprintln!("   -> [Cảnh báo] Lỗi kết nối HTTP đăng ký: {}. Sẽ truyền trực tiếp qua UDP từ byte 0.", e);
        }
    }

    // 3. Bind local UDP socket and start upload
    let server_udp_addr: SocketAddr = format!("{}:{}", args.server_ip, args.udp_port)
        .parse()
        .map_err(|e| format!("Địa chỉ Server không hợp lệ: {}", e))?;

    let udp_socket = UdpSocket::bind("0.0.0.0:0")
        .await
        .map_err(|e| format!("Không thể bind cổng UDP local: {}", e))?;

    println!("3. Bắt đầu truyền dữ liệu qua UDP...");
    let mut buffer = vec![0u8; args.block_size];

    while seek_begin < file_size {
        file.seek(SeekFrom::Start(seek_begin)).map_err(|e| format!("Seek thất bại: {}", e))?;
        let bytes_read = file.read(&mut buffer).map_err(|e| format!("Đọc file thất bại: {}", e))?;
        if bytes_read == 0 {
            break;
        }

        if args.log_progress {
            println!(
                "   -> Đang tải lên: {} / {}",
                seek_begin,
                file_size
            );
        } else {
            let percent = (seek_begin + bytes_read as u64) as f64 / file_size as f64 * 100.0;
            print!(
                "\r   -> Đang tải lên: {}/{} bytes ({:.2}%)",
                seek_begin + bytes_read as u64,
                file_size,
                percent
            );
            std::io::Write::flush(&mut std::io::stdout()).unwrap();
        }

        send_chunk_with_retry(
            &udp_socket,
            server_udp_addr,
            1, // Trạng thái 1: Đang gửi
            &packet_code_bytes,
            seek_begin,
            &buffer[..bytes_read],
        )
        .await?;

        seek_begin += bytes_read as u64;
    }
    if args.log_progress {
        println!(
            "   -> Đang tải lên: {} / {}",
            file_size,
            file_size
        );
    } else {
        println!();
    }

    // 4. Send end packet
    println!("4. Gửi tín hiệu hoàn thành (Trạng thái kết thúc)...");
    send_chunk_with_retry(
        &udp_socket,
        server_udp_addr,
        0, // Trạng thái kết thúc: 0
        &packet_code_bytes,
        file_size, // seek begin khớp với độ dài file
        &[], // không có dữ liệu
    )
    .await?;

    println!("🎉 Tải file lên thành công!");
    Ok(())
}
