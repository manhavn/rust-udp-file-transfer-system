use std::ffi::CString;
use std::fs::File;
use std::io::Write;
use std::os::raw::c_char;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;
use tauri::{
    menu::{Menu, MenuItem, PredefinedMenuItem},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    Manager, RunEvent, WindowEvent,
};

struct AppQuitState(AtomicBool);

#[cfg_attr(not(target_os = "linux"), allow(dead_code))]
struct SavedWindowGeometry(Mutex<Option<(tauri::PhysicalSize<u32>, bool)>>);

#[cfg_attr(not(target_os = "linux"), allow(dead_code))]
fn capture_window_geometry(app: &tauri::AppHandle) {
    if let Some(window) = app.get_webview_window("main") {
        if !window.is_visible().unwrap_or(true) {
            return;
        }
        let is_maximized = window.is_maximized().unwrap_or(false);
        let size = window
            .inner_size()
            .ok()
            .filter(|s| s.width >= 680 && s.height >= 560);
        if let Ok(mut guard) = app.state::<SavedWindowGeometry>().0.lock() {
            let prev_size = guard.map(|(prev, _)| prev);
            if let Some(s) = size.or(prev_size) {
                *guard = Some((s, is_maximized));
            }
        }
    }
}

fn show_main_window(app: &tauri::AppHandle) {
    if let Some(window) = app.get_webview_window("main") {
        let was_hidden = !window.is_visible().unwrap_or(true);
        let _ = window.unminimize();
        let _ = window.show();

        #[cfg(not(target_os = "linux"))]
        {
            let _ = was_hidden;
            let _ = window.set_focus();
        }

        #[cfg(target_os = "linux")]
        {
            let win = window.clone();
            let app_handle = app.clone();
            tauri::async_runtime::spawn(async move {
                // Allow GTK/tao to process WindowRequest::Visible(true) and close the tray menu
                // before issuing set_focus(), otherwise tao's get_visible() check drops Focus.
                tokio::time::sleep(std::time::Duration::from_millis(60)).await;
                let _ = win.set_focus();

                if was_hidden {
                    // Workaround for Linux Wayland/GTK3 (Tauri #11856): toggling resizable
                    // forces GTK/Wayland to rebuild CSD headerbar input regions so
                    // minimize/maximize/close buttons respond after hide() -> show().
                    let _ = win.set_resizable(false);
                    tokio::time::sleep(std::time::Duration::from_millis(30)).await;
                    let _ = win.set_resizable(true);
                    let _ = win.set_minimizable(true);
                    let _ = win.set_maximizable(true);
                    let _ = win.set_closable(true);

                    tokio::time::sleep(std::time::Duration::from_millis(20)).await;
                    let saved = app_handle
                        .state::<SavedWindowGeometry>()
                        .0
                        .lock()
                        .ok()
                        .and_then(|g| *g);
                    if let Some((saved_size, was_maximized)) = saved {
                        if was_maximized {
                            let _ = win.maximize();
                        } else {
                            let _ = win.set_size(tauri::Size::Physical(saved_size));
                        }
                    }
                    let _ = win.set_focus();
                }
            });
        }
    }
}

fn hide_main_window(app: &tauri::AppHandle) {
    capture_window_geometry(app);
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.hide();
    }
}

#[cfg(desktop)]
fn setup_tray(app: &tauri::App) -> tauri::Result<()> {
    let show_item = MenuItem::with_id(app, "show", "Mở RTK UDP Transfer", true, None::<&str>)?;
    let hide_item = MenuItem::with_id(app, "hide", "Ẩn xuống khay hệ thống", true, None::<&str>)?;
    let separator = PredefinedMenuItem::separator(app)?;
    let quit_item = MenuItem::with_id(app, "quit", "Thoát hoàn toàn", true, None::<&str>)?;
    let menu = Menu::with_items(app, &[&show_item, &hide_item, &separator, &quit_item])?;

    TrayIconBuilder::with_id("main-tray")
        .icon(tauri::include_image!("icons/128x128.png"))
        .tooltip("RTK UDP Transfer")
        .menu(&menu)
        .show_menu_on_left_click(false)
        .on_menu_event(|app, event| match event.id.as_ref() {
            "show" => show_main_window(app),
            "hide" => hide_main_window(app),
            "quit" => {
                app.state::<AppQuitState>().0.store(true, Ordering::SeqCst);
                app.exit(0);
            }
            _ => {}
        })
        .on_tray_icon_event(|tray, event| {
            if let TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                ..
            } = event
            {
                show_main_window(tray.app_handle());
            }
        })
        .build(app)?;

    Ok(())
}

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
    file.write_all(content.as_bytes())
        .map_err(|e| e.to_string())?;

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
            let len = out_buf
                .iter()
                .position(|&x| x == 0)
                .unwrap_or(out_buf.len());
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
            Some(ref p) if !p.is_empty() => {
                Some(CString::new(p.clone()).map_err(|e| e.to_string())?)
            }
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
    let app = tauri::Builder::default()
        .manage(AppQuitState(AtomicBool::new(false)))
        .manage(SavedWindowGeometry(Mutex::new(None)))
        .plugin(tauri_plugin_opener::init())
        .setup(|app| {
            #[cfg(desktop)]
            setup_tray(app)?;
            Ok(())
        })
        .on_window_event(|window, event| {
            if let WindowEvent::CloseRequested { api, .. } = event {
                let is_quitting = window
                    .app_handle()
                    .state::<AppQuitState>()
                    .0
                    .load(Ordering::SeqCst);
                if !is_quitting {
                    api.prevent_close();
                    capture_window_geometry(window.app_handle());
                    let _ = window.hide();
                }
            }
        })
        .invoke_handler(tauri::generate_handler![
            select_file,
            get_file_size,
            create_demo_file,
            calculate_hash,
            perform_upload
        ])
        .build(tauri::generate_context!())
        .expect("error while building tauri application");

    app.run(|app_handle, event| {
        if let RunEvent::ExitRequested { api, .. } = event {
            let is_quitting = app_handle.state::<AppQuitState>().0.load(Ordering::SeqCst);
            if !is_quitting {
                api.prevent_exit();
            }
        }
    });
}
