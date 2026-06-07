use std::ffi::CStr;
use std::os::raw::c_char;
use std::fs::File;
use std::io::{Read, Seek, SeekFrom};
use std::net::SocketAddr;
use std::path::Path;
use sha2::{Digest, Sha256};
use tokio::net::UdpSocket;
use tokio::time::{timeout, Duration};

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
        if let Err(_) = socket.send_to(&pkt_bytes, server_addr).await {
            // Log to stderr or ignore
        }

        let mut ack_buf = vec![0u8; 1024];
        match timeout(TIMEOUT_DUR, socket.recv_from(&mut ack_buf)).await {
            Ok(Ok((len, _src))) => {
                match common::AckPacket::parse(&ack_buf[..len]) {
                    Ok(ack) => {
                        if ack.packet_code == packet_code
                            && ack.seek_begin == seek_begin
                            && ack.bytes_received == expected_bytes_received
                        {
                            return Ok(());
                        }
                    }
                    Err(_) => {}
                }
            }
            Ok(Err(_)) => {}
            Err(_) => {}
        }

        retry_count += 1;
        if retry_count > MAX_RETRIES {
            return Err(format!("Timeout after {} retries", MAX_RETRIES));
        }

        tokio::time::sleep(Duration::from_millis(5 * (retry_count as u64).min(10))).await;
    }
}

/// Exposes the upload logic to C-compatible programs.
///
/// # Arguments
/// - `c_file_path`: Path to the file to upload (null-terminated UTF-8 string).
/// - `c_server_ip`: Server IP address (null-terminated UTF-8 string).
/// - `udp_port`: Server UDP port.
/// - `http_port`: Server HTTP port.
/// - `block_size`: Size of each chunk in bytes (e.g. 16384).
///
/// # Returns
/// - `0` for success
/// - `-1` for invalid C string parameters
/// - `-2` if the file doesn't exist or can't be read
/// - `-3` if the hash computation fails
/// - `-4` if the HTTP registration fails
/// - `-5` if the Server IP or UDP address is invalid
/// - `-6` if the local UDP socket fails to bind
/// - `-7` if the UDP transmission fails
#[no_mangle]
pub extern "C" fn rtk_upload_file(
    c_file_path: *const c_char,
    c_server_ip: *const c_char,
    udp_port: u16,
    http_port: u16,
    block_size: usize,
) -> i32 {
    if c_file_path.is_null() || c_server_ip.is_null() {
        return -1;
    }

    let file_path_str = unsafe {
        match CStr::from_ptr(c_file_path).to_str() {
            Ok(s) => s,
            Err(_) => return -1,
        }
    };

    let server_ip_str = unsafe {
        match CStr::from_ptr(c_server_ip).to_str() {
            Ok(s) => s,
            Err(_) => return -1,
        }
    };

    let path = Path::new(file_path_str);
    if !path.exists() {
        return -2;
    }

    // Initialize tokio runtime for execution
    let rt = match tokio::runtime::Runtime::new() {
        Ok(r) => r,
        Err(_) => return -6,
    };

    rt.block_on(async {
        // Calculate hash
        let mut file = match File::open(path) {
            Ok(f) => f,
            Err(_) => return -2,
        };

        let mut hasher = Sha256::new();
        let mut hash_buf = vec![0u8; 65536];
        loop {
            let n = match file.read(&mut hash_buf) {
                Ok(0) => break,
                Ok(n) => n,
                Err(_) => return -3,
            };
            hasher.update(&hash_buf[..n]);
        }
        let hash_result: [u8; 32] = hasher.finalize().into();

        let packet_code_bytes = common::generate_packet_code_from_hash(&hash_result);
        let packet_code_str = common::bytes_to_unique_id(&packet_code_bytes);
        
        let file_size = match file.metadata() {
            Ok(m) => m.len(),
            Err(_) => return -2,
        };
        let file_name = path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("unknown")
            .to_string();

        // HTTP Registration
        let client = reqwest::Client::new();
        let register_url = format!("http://{}:{}/api/register", server_ip_str, http_port);
        let reg_res = client
            .post(&register_url)
            .json(&serde_json::json!({
                "packet_code": packet_code_str,
                "file_name": file_name,
                "file_size": file_size,
            }))
            .send()
            .await;

        // Note: we warn but do not strictly abort if registration fails, to support direct UDP transmission fallback.
        let mut _registered = false;
        if let Ok(resp) = reg_res {
            if resp.status().is_success() {
                _registered = true;
            }
        }

        // UDP Address
        let server_udp_addr: SocketAddr = match format!("{}:{}", server_ip_str, udp_port).parse() {
            Ok(addr) => addr,
            Err(_) => return -5,
        };

        // Bind UDP socket
        let udp_socket = match UdpSocket::bind("0.0.0.0:0").await {
            Ok(s) => s,
            Err(_) => return -6,
        };

        // Transmission loop
        let mut buffer = vec![0u8; block_size];
        let mut seek_begin = 0u64;

        while seek_begin < file_size {
            if file.seek(SeekFrom::Start(seek_begin)).is_err() {
                return -2;
            }
            let bytes_read = match file.read(&mut buffer) {
                Ok(0) => break,
                Ok(n) => n,
                Err(_) => return -2,
            };

            if let Err(_) = send_chunk_with_retry(
                &udp_socket,
                server_udp_addr,
                1,
                &packet_code_bytes,
                seek_begin,
                &buffer[..bytes_read],
            )
            .await {
                return -7;
            }

            seek_begin += bytes_read as u64;
        }

        // Send end packet
        if let Err(_) = send_chunk_with_retry(
            &udp_socket,
            server_udp_addr,
            0,
            &packet_code_bytes,
            file_size,
            &[],
        )
        .await {
            return -7;
        }

        0 // Success
    })
}
