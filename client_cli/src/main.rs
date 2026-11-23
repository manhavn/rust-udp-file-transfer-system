use clap::Parser;
use std::collections::BTreeMap;
use std::fs::File;
use std::io::{Read, Seek, SeekFrom};
use std::net::SocketAddr;
use std::path::Path;
use tokio::net::UdpSocket;
use tokio::time::{timeout, Duration, Instant};

#[derive(Parser, Debug)]
#[command(name = "rtk-client")]
#[command(about = "Client truyền tải file qua UDP đáng tin cậy", long_about = None)]
struct Args {
    /// Đường dẫn tới file cần upload
    #[arg(env = "FILE_PATH")]
    file: String,

    /// Địa chỉ IP của Server
    #[arg(short, long, env = "SERVER_IP", default_value = "127.0.0.1")]
    server_ip: String,

    /// Cổng UDP của Server
    #[arg(short, long, env = "UDP_PORT", default_value_t = 5000)]
    udp_port: u16,

    /// Cổng HTTP của Server
    #[arg(short = 't', long, env = "HTTP_PORT", default_value_t = 8080)]
    http_port: u16,

    /// Kích thước mỗi khối dữ liệu UDP gửi đi (bytes)
    #[arg(short, long, env = "BLOCK_SIZE", default_value_t = 16384)]
    block_size: usize,

    /// Số gói UDP được phép đang chờ ACK đồng thời
    #[arg(long, env = "WINDOW_SIZE", default_value_t = 32)]
    window_size: usize,

    /// Hiển thị tiến trình upload dạng log dòng mới
    #[arg(long, env = "LOG_PROGRESS", default_value_t = false)]
    log_progress: bool,

    /// Mật khẩu bảo mật để tải xuống file (mặc định không có)
    #[arg(short = 'p', long, env = "DOWNLOAD_PASSWORD")]
    password: Option<String>,
}

const MAX_RETRIES: usize = 30;
const ACK_TIMEOUT: Duration = Duration::from_millis(150);

#[derive(Debug)]
struct InFlightChunk {
    packet: Vec<u8>,
    expected_len: u64,
    last_sent: Instant,
    retries: usize,
}

async fn send_windowed_file(
    socket: &UdpSocket,
    server_addr: SocketAddr,
    file: &mut File,
    packet_code: &[u8],
    start_offset: u64,
    file_size: u64,
    block_size: usize,
    window_size: usize,
    log_progress: bool,
) -> Result<(), String> {
    let mut next_offset = start_offset;
    let mut acked_bytes = start_offset;
    let mut in_flight: BTreeMap<u64, InFlightChunk> = BTreeMap::new();
    let mut read_buf = vec![0u8; block_size];
    let mut ack_buf = [0u8; 1024];

    file.seek(SeekFrom::Start(start_offset))
        .map_err(|e| format!("Seek thất bại: {}", e))?;

    while next_offset < file_size || !in_flight.is_empty() {
        while next_offset < file_size && in_flight.len() < window_size {
            let to_read = ((file_size - next_offset) as usize).min(block_size);
            let n = file
                .read(&mut read_buf[..to_read])
                .map_err(|e| format!("Đọc file thất bại: {}", e))?;
            if n == 0 {
                break;
            }
            let packet = common::UdpPacket::serialize(1, packet_code, next_offset, &read_buf[..n]);
            socket
                .send_to(&packet, server_addr)
                .await
                .map_err(|e| format!("Gửi UDP thất bại: {}", e))?;
            in_flight.insert(
                next_offset,
                InFlightChunk {
                    packet,
                    expected_len: n as u64,
                    last_sent: Instant::now(),
                    retries: 0,
                },
            );
            next_offset += n as u64;
        }

        match timeout(Duration::from_millis(10), socket.recv_from(&mut ack_buf)).await {
            Ok(Ok((len, src))) if src == server_addr => {
                if let Ok(ack) = common::AckPacket::parse(&ack_buf[..len]) {
                    if ack.packet_code == packet_code {
                        if let Some(chunk) = in_flight.get(&ack.seek_begin) {
                            if ack.bytes_received == chunk.expected_len {
                                in_flight.remove(&ack.seek_begin);
                                acked_bytes = next_offset.saturating_sub(
                                    in_flight.values().map(|c| c.expected_len).sum::<u64>(),
                                );
                                if log_progress {
                                    println!(
                                        "   -> Đã ACK: {} / {}",
                                        acked_bytes.min(file_size),
                                        file_size
                                    );
                                } else if file_size > 0 {
                                    let percent = acked_bytes.min(file_size) as f64
                                        / file_size as f64
                                        * 100.0;
                                    print!(
                                        "\r   -> Đang tải lên: {}/{} bytes ({:.2}%)",
                                        acked_bytes.min(file_size),
                                        file_size,
                                        percent
                                    );
                                    let _ = std::io::Write::flush(&mut std::io::stdout());
                                }
                            }
                        }
                    }
                }
            }
            Ok(Err(e)) => return Err(format!("Lỗi socket nhận ACK: {}", e)),
            _ => {}
        }

        let now = Instant::now();
        for (&offset, chunk) in in_flight.iter_mut() {
            if now.duration_since(chunk.last_sent) >= ACK_TIMEOUT {
                if chunk.retries >= MAX_RETRIES {
                    return Err(format!(
                        "Thất bại tại offset {} sau {} lần gửi lại",
                        offset, MAX_RETRIES
                    ));
                }
                socket
                    .send_to(&chunk.packet, server_addr)
                    .await
                    .map_err(|e| format!("Gửi lại UDP thất bại: {}", e))?;
                chunk.retries += 1;
                chunk.last_sent = Instant::now();
            }
        }
    }
    if !log_progress {
        println!();
    }
    Ok(())
}

async fn send_end_with_retry(
    socket: &UdpSocket,
    server_addr: SocketAddr,
    packet_code: &[u8],
    file_size: u64,
) -> Result<(), String> {
    let packet = common::UdpPacket::serialize(0, packet_code, file_size, &[]);
    let mut ack_buf = [0u8; 1024];
    for _ in 0..=MAX_RETRIES {
        socket
            .send_to(&packet, server_addr)
            .await
            .map_err(|e| e.to_string())?;
        if let Ok(Ok((len, src))) = timeout(ACK_TIMEOUT, socket.recv_from(&mut ack_buf)).await {
            if src == server_addr {
                if let Ok(ack) = common::AckPacket::parse(&ack_buf[..len]) {
                    if ack.packet_code == packet_code
                        && ack.seek_begin == file_size
                        && ack.bytes_received == 0
                    {
                        return Ok(());
                    }
                }
            }
        }
    }
    Err("Không nhận được ACK kết thúc".to_string())
}

#[tokio::main]
async fn main() -> Result<(), String> {
    let args = Args::parse();

    let file_path = Path::new(&args.file);
    if !file_path.exists() {
        return Err(format!("File không tồn tại: {}", args.file));
    }

    if args.block_size == 0 {
        return Err("Kích thước block phải lớn hơn 0".to_string());
    }

    println!("1. Đang tính toán mã băm XXH3 của file...");
    let mut file = File::open(&args.file).map_err(|e| format!("Không thể mở file: {}", e))?;
    let mut hasher = xxhash_rust::xxh3::Xxh3::new();
    let mut hash_buf = vec![0u8; 65536];
    loop {
        let n = file
            .read(&mut hash_buf)
            .map_err(|e| format!("Lỗi đọc file khi băm: {}", e))?;
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

    let file_size = file
        .metadata()
        .map_err(|e| format!("Lỗi đọc metadata: {}", e))?
        .len();
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
            "password": args.password,
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
                            // bytes_received is a high-water mark; with windowed/out-of-order UDP, rewind one full window to fill possible holes after an interrupted upload.
                            let aligned_offset = aligned_offset
                                .saturating_sub((args.window_size.max(1) as u64) * block_size);
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
                eprintln!(
                    "   -> [Cảnh báo] Đăng ký HTTP không thành công: Code {}",
                    resp.status()
                );
            }
        }
        Err(e) => {
            eprintln!("   -> [Cảnh báo] Lỗi kết nối HTTP đăng ký: {}. Sẽ truyền trực tiếp qua UDP từ byte 0.", e);
        }
    }

    // 3. Bind local UDP socket and start upload
    let server_udp_addr = tokio::net::lookup_host(format!("{}:{}", args.server_ip, args.udp_port))
        .await
        .map_err(|e| format!("Không thể phân giải địa chỉ Server: {}", e))?
        .next()
        .ok_or_else(|| "Không tìm thấy địa chỉ IP cho Server".to_string())?;

    let udp_socket = UdpSocket::bind("0.0.0.0:0")
        .await
        .map_err(|e| format!("Không thể bind cổng UDP local: {}", e))?;

    println!(
        "3. Bắt đầu truyền dữ liệu qua UDP (window = {})...",
        args.window_size.max(1)
    );
    send_windowed_file(
        &udp_socket,
        server_udp_addr,
        &mut file,
        &packet_code_bytes,
        seek_begin,
        file_size,
        args.block_size,
        args.window_size.max(1),
        args.log_progress,
    )
    .await?;

    println!("4. Gửi tín hiệu hoàn thành (Trạng thái kết thúc)...");
    send_end_with_retry(&udp_socket, server_udp_addr, &packet_code_bytes, file_size).await?;

    println!(
        "🎉 Tải file lên thành công! Mã Hash ID: {}",
        packet_code_str
    );
    Ok(())
}
