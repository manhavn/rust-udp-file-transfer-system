use std::collections::BTreeMap;
use std::ffi::CStr;
use std::fs::File;
use std::io::{Read, Seek, SeekFrom};
use std::net::SocketAddr;
use std::os::raw::c_char;
use std::path::Path;
use tokio::net::UdpSocket;
use tokio::time::{timeout, Duration, Instant};

const MAX_RETRIES: usize = 30;
const ACK_TIMEOUT: Duration = Duration::from_millis(150);
const DEFAULT_WINDOW_SIZE: usize = 32;
const PROGRESS_INTERVAL: Duration = Duration::from_millis(250);

pub type RtkProgressCallback = Option<extern "C" fn(u64, u64, u64)>;

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
    progress_callback: RtkProgressCallback,
) -> Result<(), String> {
    let mut next_offset = start_offset;
    let mut confirmed_bytes = 0u64;
    let transfer_started = Instant::now();
    let mut last_progress_report: Option<Instant> = None;

    if let Some(callback) = progress_callback {
        callback(start_offset.min(file_size), file_size, 0);
    }
    let mut in_flight: BTreeMap<u64, InFlightChunk> = BTreeMap::new();
    let mut read_buf = vec![0u8; block_size];
    let mut ack_buf = [0u8; 1024];
    file.seek(SeekFrom::Start(start_offset))
        .map_err(|e| e.to_string())?;

    while next_offset < file_size || !in_flight.is_empty() {
        while next_offset < file_size && in_flight.len() < window_size {
            let to_read = ((file_size - next_offset) as usize).min(block_size);
            let n = file
                .read(&mut read_buf[..to_read])
                .map_err(|e| e.to_string())?;
            if n == 0 {
                break;
            }
            let packet = common::UdpPacket::serialize(1, packet_code, next_offset, &read_buf[..n]);
            socket
                .send_to(&packet, server_addr)
                .await
                .map_err(|e| e.to_string())?;
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
                        let valid = in_flight
                            .get(&ack.seek_begin)
                            .map(|c| c.expected_len == ack.bytes_received)
                            .unwrap_or(false);
                        if valid {
                            if let Some(chunk) = in_flight.remove(&ack.seek_begin) {
                                confirmed_bytes =
                                    confirmed_bytes.saturating_add(chunk.expected_len);
                                if let Some(callback) = progress_callback {
                                    let now = Instant::now();
                                    let should_report = last_progress_report
                                        .map(|last| now.duration_since(last) >= PROGRESS_INTERVAL)
                                        .unwrap_or(true)
                                        || start_offset.saturating_add(confirmed_bytes)
                                            >= file_size;

                                    if should_report {
                                        let elapsed =
                                            now.duration_since(transfer_started).as_secs_f64();
                                        let bytes_per_second = if elapsed > 0.0 {
                                            (confirmed_bytes as f64 / elapsed) as u64
                                        } else {
                                            0
                                        };
                                        callback(
                                            start_offset
                                                .saturating_add(confirmed_bytes)
                                                .min(file_size),
                                            file_size,
                                            bytes_per_second,
                                        );
                                        last_progress_report = Some(now);
                                    }
                                }
                            }
                        }
                    }
                }
            }
            Ok(Err(e)) => return Err(e.to_string()),
            _ => {}
        }

        let now = Instant::now();
        for (&offset, chunk) in in_flight.iter_mut() {
            if now.duration_since(chunk.last_sent) >= ACK_TIMEOUT {
                if chunk.retries >= MAX_RETRIES {
                    return Err(format!(
                        "Timeout at offset {} after {} retries",
                        offset, MAX_RETRIES
                    ));
                }
                socket
                    .send_to(&chunk.packet, server_addr)
                    .await
                    .map_err(|e| e.to_string())?;
                chunk.retries += 1;
                chunk.last_sent = Instant::now();
            }
        }
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
    Err("End packet ACK timeout".to_string())
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
    upload_file_impl(
        c_file_path,
        c_server_ip,
        udp_port,
        http_port,
        block_size,
        std::ptr::null(),
        None,
    )
}

#[no_mangle]
pub extern "C" fn rtk_upload_file_with_password(
    c_file_path: *const c_char,
    c_server_ip: *const c_char,
    udp_port: u16,
    http_port: u16,
    block_size: usize,
    c_password: *const c_char,
) -> i32 {
    upload_file_impl(
        c_file_path,
        c_server_ip,
        udp_port,
        http_port,
        block_size,
        c_password,
        None,
    )
}

/// Uploads a file and reports confirmed transfer progress through a C callback.
///
/// The callback arguments are:
/// - confirmed bytes (ACKed by the server, including resume offset)
/// - total file bytes
/// - effective confirmed bytes per second for this upload session
#[no_mangle]
pub extern "C" fn rtk_upload_file_with_password_and_progress(
    c_file_path: *const c_char,
    c_server_ip: *const c_char,
    udp_port: u16,
    http_port: u16,
    block_size: usize,
    c_password: *const c_char,
    progress_callback: RtkProgressCallback,
) -> i32 {
    upload_file_impl(
        c_file_path,
        c_server_ip,
        udp_port,
        http_port,
        block_size,
        c_password,
        progress_callback,
    )
}

fn upload_file_impl(
    c_file_path: *const c_char,
    c_server_ip: *const c_char,
    udp_port: u16,
    http_port: u16,
    block_size: usize,
    c_password: *const c_char,
    progress_callback: RtkProgressCallback,
) -> i32 {
    if c_file_path.is_null() || c_server_ip.is_null() {
        return -1;
    }

    let password_str = if c_password.is_null() {
        None
    } else {
        unsafe {
            match CStr::from_ptr(c_password).to_str() {
                Ok(s) => {
                    if s.is_empty() {
                        None
                    } else {
                        Some(s.to_string())
                    }
                }
                Err(_) => return -1,
            }
        }
    };

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

    if block_size == 0 {
        return -7;
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

        let mut hasher = xxhash_rust::xxh3::Xxh3::new();
        let mut hash_buf = vec![0u8; 65536];
        loop {
            let n = match file.read(&mut hash_buf) {
                Ok(0) => break,
                Ok(n) => n,
                Err(_) => return -3,
            };
            hasher.update(&hash_buf[..n]);
        }
        let hash_result = hasher.digest();
        let hash_bytes = hash_result.to_be_bytes();

        let packet_code_bytes = common::generate_packet_code_from_hash(&hash_bytes);
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

        let mut seek_begin = 0u64;

        // HTTP Registration
        let client = reqwest::Client::new();
        let register_url = format!("http://{}:{}/api/register", server_ip_str, http_port);
        let reg_res = client
            .post(&register_url)
            .json(&serde_json::json!({
                "packet_code": packet_code_str,
                "file_name": file_name,
                "file_size": file_size,
                "password": password_str,
            }))
            .send()
            .await;

        // Note: we warn but do not strictly abort if registration fails, to support direct UDP transmission fallback.
        if let Ok(resp) = reg_res {
            if resp.status().is_success() {
                if let Ok(json) = resp.json::<serde_json::Value>().await {
                    if let Some(offset) = json.get("bytes_received").and_then(|v| v.as_u64()) {
                        let b_size = block_size as u64;
                        seek_begin = (offset / b_size) * b_size;
                        // Rewind one window because the server high-water mark may include out-of-order chunks.
                        seek_begin = seek_begin.saturating_sub(DEFAULT_WINDOW_SIZE as u64 * b_size);
                    }
                }
            }
        }

        // UDP Address DNS Resolution
        let server_udp_addr: SocketAddr =
            match tokio::net::lookup_host(format!("{}:{}", server_ip_str, udp_port)).await {
                Ok(mut addrs) => match addrs.next() {
                    Some(addr) => addr,
                    None => return -5,
                },
                Err(_) => return -5,
            };

        // Bind UDP socket
        let udp_socket = match UdpSocket::bind("0.0.0.0:0").await {
            Ok(s) => s,
            Err(_) => return -6,
        };

        // Windowed transmission: keep multiple chunks in flight to hide network RTT.
        if send_windowed_file(
            &udp_socket,
            server_udp_addr,
            &mut file,
            &packet_code_bytes,
            seek_begin,
            file_size,
            block_size,
            DEFAULT_WINDOW_SIZE,
            progress_callback,
        )
        .await
        .is_err()
        {
            return -7;
        }

        if send_end_with_retry(&udp_socket, server_udp_addr, &packet_code_bytes, file_size)
            .await
            .is_err()
        {
            return -7;
        }

        0 // Success
    })
}

#[no_mangle]
pub extern "C" fn rtk_calculate_hash_id(
    c_file_path: *const c_char,
    out_buf: *mut c_char,
    max_len: usize,
) -> i32 {
    if c_file_path.is_null() || out_buf.is_null() || max_len == 0 {
        return -1;
    }

    let file_path_str = unsafe {
        match CStr::from_ptr(c_file_path).to_str() {
            Ok(s) => s,
            Err(_) => return -1,
        }
    };

    let path = Path::new(file_path_str);
    let mut file = match File::open(path) {
        Ok(f) => f,
        Err(_) => return -2,
    };

    let mut hasher = xxhash_rust::xxh3::Xxh3::new();
    let mut hash_buf = vec![0u8; 65536];
    loop {
        let n = match file.read(&mut hash_buf) {
            Ok(0) => break,
            Ok(n) => n,
            Err(_) => return -3,
        };
        hasher.update(&hash_buf[..n]);
    }
    let hash_result = hasher.digest();
    let hash_bytes = hash_result.to_be_bytes();

    let packet_code_bytes = common::generate_packet_code_from_hash(&hash_bytes);
    let packet_code_str = common::bytes_to_unique_id(&packet_code_bytes);

    let bytes = packet_code_str.as_bytes();
    if bytes.len() >= max_len {
        return -8; // Buffer too small
    }

    unsafe {
        std::ptr::copy_nonoverlapping(bytes.as_ptr(), out_buf as *mut u8, bytes.len());
        // Null-terminate the string
        std::ptr::write(out_buf.add(bytes.len()), 0);
    }

    0 // Success
}
