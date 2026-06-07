use std::ffi::CString;
use std::fs::File;
use std::io::Write;
use std::os::raw::c_char;

#[tauri::command]
fn select_file() -> Option<String> {
    rfd::FileDialog::new()
        .set_title("Chọn File để Upload")
        .pick_file()
        .map(|p| p.to_string_lossy().into_owned())
}

#[tauri::command]
fn get_file_size(file_path: String) -> Result<u64, String> {
    std::fs::metadata(&file_path)
        .map(|m| m.len())
        .map_err(|e| e.to_string())
}

#[tauri::command]
fn create_demo_file(content: String, file_name: String) -> Result<String, String> {
    let temp_dir = std::env::temp_dir();
    let file_path = temp_dir.join(&file_name);
    
    let mut file = File::create(&file_path).map_err(|e| e.to_string())?;
    file.write_all(content.as_bytes()).map_err(|e| e.to_string())?;
    
    Ok(file_path.to_string_lossy().into_owned())
}

#[tauri::command]
async fn calculate_hash(file_path: String) -> Result<String, String> {
    tokio::task::spawn_blocking(move || {
        let c_file_path = CString::new(file_path).map_err(|e| e.to_string())?;
        let mut out_buf = vec![0u8; 256];
        let res = client_lib::rtk_calculate_hash_id(
            c_file_path.as_ptr(),
            out_buf.as_mut_ptr() as *mut c_char,
            out_buf.len(),
        );
        if res == 0 {
            let len = out_buf.iter().position(|&x| x == 0).unwrap_or(out_buf.len());
            let hash_str = std::str::from_utf8(&out_buf[..len])
                .map_err(|e| e.to_string())?
                .to_string();
            Ok(hash_str)
        } else {
            Err(format!("Error code {}", res))
        }
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
async fn perform_upload(
    file_path: String,
    server_ip: String,
    udp_port: u16,
    http_port: u16,
    block_size: usize,
    password: Option<String>,
) -> Result<i32, String> {
    tokio::task::spawn_blocking(move || {
        let c_file_path = CString::new(file_path).map_err(|e| e.to_string())?;
        let c_server_ip = CString::new(server_ip).map_err(|e| e.to_string())?;
        
        let c_password = match password {
            Some(ref p) if !p.is_empty() => Some(CString::new(p.clone()).map_err(|e| e.to_string())?),
            _ => None,
        };
        
        let pwd_ptr = match c_password {
            Some(ref cp) => cp.as_ptr(),
            None => std::ptr::null(),
        };

        let res = client_lib::rtk_upload_file_with_password(
            c_file_path.as_ptr(),
            c_server_ip.as_ptr(),
            udp_port,
            http_port,
            block_size,
            pwd_ptr,
        );
        
        Ok(res)
    })
    .await
    .map_err(|e| e.to_string())?
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![
            select_file,
            get_file_size,
            create_demo_file,
            calculate_hash,
            perform_upload
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
