use clap::Parser;
use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::sync::RwLock;
use chrono::{DateTime, Utc};
use serde::{Serialize, Deserialize};
use axum::{
    routing::{get, post},
    extract::State,
    response::Html,
    Json, Router,
};
use tower_http::cors::CorsLayer;
use tower_http::services::ServeDir;
use serde_json::json;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UploadInfo {
    pub packet_code: String,
    pub file_name: String,
    pub file_size: u64,
    pub bytes_received: u64,
    pub status: String, // "Đang nhận" or "Hoàn thành"
    pub started_at: DateTime<Utc>,
    pub completed_at: Option<DateTime<Utc>>,
    pub delete_at: Option<DateTime<Utc>>,
}

#[derive(Parser, Debug, Clone)]
#[command(name = "rtk-server")]
#[command(about = "Server truyền tải file qua UDP & Web Dashboard", long_about = None)]
pub struct Args {
    /// Cổng UDP lắng nghe
    #[arg(short, long, default_value_t = 5000)]
    pub udp_port: u16,

    /// Cổng HTTP REST API & Dashboard
    #[arg(short, long, default_value_t = 8080)]
    pub http_port: u16,

    /// Thư mục chứa các tệp tải lên
    #[arg(long, default_value = "./uploads")]
    pub upload_dir: String,

    /// Đường dẫn cơ sở dữ liệu SQLite
    #[arg(long, default_value = "./db/data.sqlite")]
    pub db_path: String,

    /// Chu kỳ quét dọn dẹp tệp tin (phút)
    #[arg(long, default_value_t = 5)]
    pub cleanup_interval: u64,

    /// Thời gian tối đa lưu trữ tệp chưa hoàn thành (phút)
    #[arg(long, default_value_t = 60)]
    pub incomplete_timeout: i64,

    /// Thời gian tối đa lưu trữ tệp đã hoàn thành (phút)
    #[arg(long, default_value_t = 15)]
    pub completed_timeout: i64,
}

pub struct ServerState {
    pub uploads: HashMap<String, UploadInfo>,
    pub upload_dir: String,
    pub db_path: String,
    pub cleanup_interval: u64,
    pub incomplete_timeout_mins: i64,
    pub completed_timeout_mins: i64,
}

fn init_db(db_path: &str) -> Result<(), rusqlite::Error> {
    let conn = rusqlite::Connection::open(db_path)?;
    conn.execute(
        "CREATE TABLE IF NOT EXISTS uploads (
            packet_code TEXT PRIMARY KEY,
            file_name TEXT NOT NULL,
            file_size INTEGER NOT NULL,
            bytes_received INTEGER NOT NULL,
            status TEXT NOT NULL,
            started_at TEXT NOT NULL,
            completed_at TEXT,
            delete_at TEXT
        )",
        [],
    )?;
    // Attempt migration for existing databases missing the delete_at column
    let _ = conn.execute("ALTER TABLE uploads ADD COLUMN delete_at TEXT", []);
    Ok(())
}

fn load_uploads_from_db(db_path: &str) -> Result<HashMap<String, UploadInfo>, rusqlite::Error> {
    let conn = rusqlite::Connection::open(db_path)?;
    let mut stmt = conn.prepare("SELECT packet_code, file_name, file_size, bytes_received, status, started_at, completed_at, delete_at FROM uploads")?;
    let upload_iter = stmt.query_map([], |row| {
        let started_at_str: String = row.get(5)?;
        let completed_at_str: Option<String> = row.get(6)?;
        let delete_at_str: Option<String> = row.get(7)?;

        let started_at = DateTime::parse_from_rfc3339(&started_at_str)
            .map(|dt| dt.with_timezone(&Utc))
            .unwrap_or_else(|_| Utc::now());

        let completed_at = completed_at_str.and_then(|s| {
            DateTime::parse_from_rfc3339(&s)
                .map(|dt| dt.with_timezone(&Utc))
                .ok()
        });

        let delete_at = delete_at_str.and_then(|s| {
            DateTime::parse_from_rfc3339(&s)
                .map(|dt| dt.with_timezone(&Utc))
                .ok()
        });

        Ok(UploadInfo {
            packet_code: row.get(0)?,
            file_name: row.get(1)?,
            file_size: row.get(2)?,
            bytes_received: row.get(3)?,
            status: row.get(4)?,
            started_at,
            completed_at,
            delete_at,
        })
    })?;

    let mut uploads = HashMap::new();
    for upload in upload_iter {
        let u = upload?;
        uploads.insert(u.packet_code.clone(), u);
    }
    Ok(uploads)
}

fn save_upload_to_db(db_path: &str, info: &UploadInfo) -> Result<(), rusqlite::Error> {
    let conn = rusqlite::Connection::open(db_path)?;
    conn.execute(
        "INSERT INTO uploads (packet_code, file_name, file_size, bytes_received, status, started_at, completed_at, delete_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)
         ON CONFLICT(packet_code) DO UPDATE SET
            file_name = excluded.file_name,
            file_size = excluded.file_size,
            bytes_received = excluded.bytes_received,
            status = excluded.status,
            started_at = excluded.started_at,
            completed_at = excluded.completed_at,
            delete_at = excluded.delete_at",
        rusqlite::params![
            info.packet_code,
            info.file_name,
            info.file_size,
            info.bytes_received,
            info.status,
            info.started_at.to_rfc3339(),
            info.completed_at.map(|dt| dt.to_rfc3339()),
            info.delete_at.map(|dt| dt.to_rfc3339()),
        ],
    )?;
    Ok(())
}

#[derive(Debug, Deserialize)]
pub struct RegisterRequest {
    pub packet_code: String,
    pub file_name: String,
    pub file_size: u64,
}

const INDEX_HTML: &str = r#"
<!DOCTYPE html>
<html lang="vi">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>RTK UDP File Transfer Control Center</title>
    <link href="https://fonts.googleapis.com/css2?family=Outfit:wght@300;400;500;600;700&display=swap" rel="stylesheet">
    <style>
        :root {
            --bg-color: #0b0f19;
            --card-bg: rgba(17, 24, 39, 0.7);
            --primary: #6366f1;
            --primary-glow: rgba(99, 102, 241, 0.15);
            --secondary: #8b5cf6;
            --success: #10b981;
            --success-glow: rgba(16, 185, 129, 0.15);
            --warning: #f59e0b;
            --warning-glow: rgba(245, 158, 11, 0.15);
            --text-main: #f3f4f6;
            --text-muted: #9ca3af;
            --border: rgba(255, 255, 255, 0.08);
            --border-hover: rgba(99, 102, 241, 0.4);
        }

        * {
            margin: 0;
            padding: 0;
            box-sizing: border-box;
            font-family: 'Outfit', sans-serif;
        }

        body {
            background-color: var(--bg-color);
            background-image: 
                radial-gradient(circle at 10% 20%, rgba(99, 102, 241, 0.08) 0%, transparent 40%),
                radial-gradient(circle at 90% 80%, rgba(139, 92, 246, 0.08) 0%, transparent 40%);
            color: var(--text-main);
            min-height: 100vh;
            padding: 2rem;
            display: flex;
            flex-direction: column;
            align-items: center;
        }

        .container {
            width: 100%;
            max-width: 1200px;
            display: flex;
            flex-direction: column;
            gap: 2rem;
        }

        header {
            display: flex;
            justify-content: space-between;
            align-items: center;
            border-bottom: 1px solid var(--border);
            padding-bottom: 1.5rem;
        }

        .logo-section h1 {
            font-size: 2rem;
            font-weight: 700;
            background: linear-gradient(135deg, var(--primary) 0%, var(--secondary) 100%);
            -webkit-background-clip: text;
            -webkit-text-fill-color: transparent;
            display: flex;
            align-items: center;
            gap: 0.5rem;
        }

        .logo-section p {
            color: var(--text-muted);
            margin-top: 0.25rem;
            font-size: 0.95rem;
        }

        .server-status {
            display: flex;
            gap: 1rem;
        }

        .status-pill {
            background: var(--card-bg);
            border: 1px solid var(--border);
            padding: 0.5rem 1rem;
            border-radius: 9999px;
            font-size: 0.85rem;
            display: flex;
            align-items: center;
            gap: 0.5rem;
            backdrop-filter: blur(10px);
        }

        .status-dot {
            width: 8px;
            height: 8px;
            background-color: var(--success);
            border-radius: 50%;
            box-shadow: 0 0 8px var(--success);
            animation: pulse 2s infinite;
        }

        @keyframes pulse {
            0% { transform: scale(0.95); box-shadow: 0 0 0 0 var(--success-glow); }
            70% { transform: scale(1); box-shadow: 0 0 0 6px transparent; }
            100% { transform: scale(0.95); box-shadow: 0 0 0 0 transparent; }
        }

        /* Stats Grid */
        .stats-grid {
            display: grid;
            grid-template-columns: repeat(auto-fit, minmax(280px, 1fr));
            gap: 1.5rem;
        }

        .stat-card {
            background: var(--card-bg);
            border: 1px solid var(--border);
            padding: 1.5rem;
            border-radius: 16px;
            backdrop-filter: blur(12px);
            transition: all 0.3s ease;
            display: flex;
            flex-direction: column;
            gap: 0.5rem;
        }

        .stat-card:hover {
            transform: translateY(-2px);
            border-color: var(--border-hover);
            box-shadow: 0 10px 20px rgba(0, 0, 0, 0.2);
        }

        .stat-card .label {
            color: var(--text-muted);
            font-size: 0.9rem;
            font-weight: 500;
        }

        .stat-card .value {
            font-size: 2rem;
            font-weight: 700;
            color: var(--text-main);
        }

        /* Transfer Queue */
        .queue-section {
            background: var(--card-bg);
            border: 1px solid var(--border);
            border-radius: 20px;
            padding: 1.5rem;
            backdrop-filter: blur(12px);
            display: flex;
            flex-direction: column;
            gap: 1.5rem;
        }

        .queue-header {
            display: flex;
            justify-content: space-between;
            align-items: center;
        }

        .queue-title {
            font-size: 1.25rem;
            font-weight: 600;
        }

        .refresh-btn {
            background: rgba(255, 255, 255, 0.05);
            border: 1px solid var(--border);
            color: var(--text-main);
            padding: 0.5rem 1rem;
            border-radius: 8px;
            font-size: 0.85rem;
            cursor: pointer;
            transition: all 0.2s;
            display: flex;
            align-items: center;
            gap: 0.5rem;
        }

        .refresh-btn:hover {
            background: var(--primary);
            border-color: var(--primary);
        }

        .table-container {
            overflow-x: auto;
        }

        table {
            width: 100%;
            border-collapse: collapse;
            text-align: left;
        }

        th {
            padding: 1rem;
            color: var(--text-muted);
            font-weight: 500;
            font-size: 0.9rem;
            border-bottom: 1px solid var(--border);
        }

        td {
            padding: 1.25rem 1rem;
            border-bottom: 1px solid var(--border);
            font-size: 0.95rem;
        }

        tr {
            transition: background-color 0.2s;
        }

        tr:hover {
            background-color: rgba(255, 255, 255, 0.02);
        }

        .file-info {
            display: flex;
            flex-direction: column;
            gap: 0.25rem;
        }

        .file-name {
            font-weight: 600;
            color: var(--text-main);
        }

        .packet-code {
            font-family: monospace;
            font-size: 0.8rem;
            color: var(--text-muted);
        }

        .badge {
            display: inline-flex;
            align-items: center;
            padding: 0.25rem 0.75rem;
            border-radius: 9999px;
            font-size: 0.8rem;
            font-weight: 600;
            text-shadow: 0 0 10px rgba(0,0,0,0.5);
        }

        .badge-receiving {
            background: var(--warning-glow);
            color: var(--warning);
            border: 1px solid rgba(245, 158, 11, 0.3);
            animation: pulse-warn 1.5s infinite;
        }

        @keyframes pulse-warn {
            0% { opacity: 0.8; }
            50% { opacity: 1; }
            100% { opacity: 0.8; }
        }

        .badge-completed {
            background: var(--success-glow);
            color: var(--success);
            border: 1px solid rgba(16, 185, 129, 0.3);
        }

        .progress-container {
            display: flex;
            align-items: center;
            gap: 1rem;
            width: 100%;
            min-width: 180px;
        }

        .progress-bar-bg {
            flex-grow: 1;
            height: 6px;
            background: rgba(255, 255, 255, 0.05);
            border-radius: 9999px;
            overflow: hidden;
        }

        .progress-bar-fill {
            height: 100%;
            background: linear-gradient(90deg, var(--primary), var(--secondary));
            border-radius: 9999px;
            width: 0%;
            transition: width 0.3s ease;
            box-shadow: 0 0 8px var(--primary-glow);
        }

        .progress-text {
            font-size: 0.85rem;
            font-weight: 600;
            width: 45px;
            text-align: right;
        }

        .download-btn {
            background: linear-gradient(135deg, var(--primary) 0%, var(--secondary) 100%);
            border: none;
            color: white;
            padding: 0.4rem 0.8rem;
            border-radius: 6px;
            font-size: 0.85rem;
            cursor: pointer;
            text-decoration: none;
            display: inline-flex;
            align-items: center;
            gap: 0.25rem;
            font-weight: 500;
            transition: all 0.2s;
        }

        .download-btn:hover {
            opacity: 0.9;
            transform: scale(1.03);
            box-shadow: 0 4px 12px var(--primary-glow);
        }

        .download-btn.disabled {
            background: rgba(255,255,255,0.05);
            color: var(--text-muted);
            cursor: not-allowed;
            pointer-events: none;
            border: 1px solid var(--border);
        }

        .empty-state {
            text-align: center;
            padding: 3rem 1rem;
            color: var(--text-muted);
            display: flex;
            flex-direction: column;
            align-items: center;
            gap: 0.75rem;
        }

        .empty-icon {
            font-size: 3rem;
            opacity: 0.4;
        }
    </style>
</head>
<body>
    <div class="container">
        <header>
            <div class="logo-section">
                <h1>📡 RTK UDP Transmission Center</h1>
                <p>Hệ thống truyền tải file tốc độ cao qua UDP & Dashboard giám sát thời gian thực</p>
            </div>
            <div class="server-status">
                <div class="status-pill">
                    <span class="status-dot"></span>
                    <span>UDP: <b>5000</b></span>
                </div>
                <div class="status-pill">
                    <span class="status-dot"></span>
                    <span>HTTP API: <b>8080</b></span>
                </div>
            </div>
        </header>

        <!-- Stats Grid -->
        <div class="stats-grid">
            <div class="stat-card">
                <span class="label">Tổng số file</span>
                <span class="value" id="stat-total">0</span>
            </div>
            <div class="stat-card">
                <span class="label">Đang truyền tải</span>
                <span class="value" id="stat-active" style="color: var(--warning);">0</span>
            </div>
            <div class="stat-card">
                <span class="label">Tổng dung lượng đã nhận</span>
                <span class="value" id="stat-size" style="color: var(--success);">0 B</span>
            </div>
        </div>

        <!-- Queue Section -->
        <div class="queue-section">
            <div class="queue-header">
                <h2 class="queue-title">Danh sách file đã & đang truyền tải</h2>
                <button class="refresh-btn" onclick="fetchUploads()">
                    🔄 Làm mới
                </button>
            </div>

            <div class="table-container">
                <table>
                    <thead>
                        <tr>
                            <th>Tên File / Mã Hash</th>
                            <th>Kích thước</th>
                            <th>Trạng thái</th>
                            <th>Tiến trình</th>
                            <th>Thời gian truyền</th>
                            <th>Hành động</th>
                        </tr>
                    </thead>
                    <tbody id="uploads-table-body">
                        <!-- Dynamic content -->
                    </tbody>
                </table>
                <div id="empty-view" class="empty-state" style="display: none;">
                    <div class="empty-icon">📁</div>
                    <h3>Không có file nào đang hoặc đã truyền tải</h3>
                    <p>Khởi chạy client app để bắt đầu truyền tải file lên server.</p>
                </div>
            </div>
        </div>
    </div>

    <script>
        function formatBytes(bytes, decimals = 2) {
            if (bytes === 0) return '0 Bytes';
            const k = 1024;
            const dm = decimals < 0 ? 0 : decimals;
            const sizes = ['Bytes', 'KB', 'MB', 'GB', 'TB'];
            const i = Math.floor(Math.log(bytes) / Math.log(k));
            return parseFloat((bytes / Math.pow(k, i)).toFixed(dm)) + ' ' + sizes[i];
        }

        function formatDateTime(dateStr) {
            if (!dateStr) return '-';
            const date = new Date(dateStr);
            return date.toLocaleString('vi-VN', { hour: '2-digit', minute: '2-digit', day: '2-digit', month: '2-digit', year: 'numeric' });
        }

        function calculateDuration(startStr, endStr) {
            const start = new Date(startStr);
            const end = endStr ? new Date(endStr) : new Date();
            const diffMs = end - start;
            const diffMins = Math.max(0, Math.floor(diffMs / 60000));
            if (diffMins === 0) return `< 1 phút`;
            return `${diffMins} phút`;
        }

        function formatDeleteAt(deleteAtStr) {
            if (!deleteAtStr) return '';
            const deleteAt = new Date(deleteAtStr);
            const now = new Date();
            const diffMs = deleteAt - now;
            const diffMins = Math.ceil(diffMs / (1000 * 60));
            
            if (diffMins <= 0) {
                return `<span style="font-size: 0.82rem; color: var(--warning)">Đang xóa...</span>`;
            }
            
            const timeStr = deleteAt.toLocaleTimeString('vi-VN', { hour: '2-digit', minute: '2-digit' });
            return `<span style="font-size: 0.82rem; color: #f43f5e; font-weight: 500;">Tự hủy: ${timeStr} (còn ${diffMins} phút)</span>`;
        }

        async function fetchUploads() {
            try {
                const response = await fetch('/api/list');
                const data = await response.json();
                
                // Update stats
                document.getElementById('stat-total').innerText = data.length;
                const activeCount = data.filter(u => u.status === 'Đang nhận').length;
                document.getElementById('stat-active').innerText = activeCount;
                
                const totalBytes = data.reduce((acc, curr) => acc + curr.bytes_received, 0);
                document.getElementById('stat-size').innerText = formatBytes(totalBytes);

                const tbody = document.getElementById('uploads-table-body');
                const emptyView = document.getElementById('empty-view');

                if (data.length === 0) {
                    tbody.innerHTML = '';
                    emptyView.style.display = 'flex';
                    return;
                }
                emptyView.style.display = 'none';

                let html = '';
                data.forEach(upload => {
                    const progress = upload.file_size > 0 
                        ? Math.min(100, Math.round((upload.bytes_received / upload.file_size) * 100)) 
                        : 0;

                    const isCompleted = upload.status === 'Hoàn thành';
                    const statusClass = isCompleted ? 'badge-completed' : 'badge-receiving';
                    const downloadAttr = isCompleted ? `href="/uploads/${encodeURIComponent(upload.file_name)}"` : '';
                    const downloadClass = isCompleted ? 'download-btn' : 'download-btn disabled';

                    html += `
                        <tr>
                            <td>
                                <div class="file-info">
                                    <span class="file-name">${upload.file_name}</span>
                                    <span class="packet-code">Hash ID: ${upload.packet_code}</span>
                                </div>
                            </td>
                            <td>
                                <b>${formatBytes(upload.bytes_received)}</b> / ${formatBytes(upload.file_size)}
                            </td>
                            <td>
                                <span class="badge ${statusClass}">${upload.status}</span>
                            </td>
                            <td>
                                <div class="progress-container">
                                    <div class="progress-bar-bg">
                                        <div class="progress-bar-fill" style="width: ${progress}%"></div>
                                    </div>
                                    <span class="progress-text">${progress}%</span>
                                </div>
                            </td>
                            <td>
                                 <div class="file-info">
                                     <span style="font-size: 0.85rem; color: var(--text-muted)">Bắt đầu: ${formatDateTime(upload.started_at)}</span>
                                     ${upload.completed_at ? `<span style="font-size: 0.85rem; color: var(--success); font-weight: 500;">Kết thúc: ${formatDateTime(upload.completed_at)}</span>` : ''}
                                     <span style="font-size: 0.85rem; color: var(--text-muted)">Thời gian truyền: ${calculateDuration(upload.started_at, upload.completed_at)}</span>
                                     ${formatDeleteAt(upload.delete_at)}
                                 </div>
                             </td>
                            <td>
                                <a ${downloadAttr} class="${downloadClass}" download>
                                    📥 Tải về
                                </a>
                            </td>
                        </tr>
                    `;
                });
                tbody.innerHTML = html;
            } catch (error) {
                console.error('Error fetching uploads:', error);
            }
        }

        // Auto fetch every 1.5 seconds
        fetchUploads();
        setInterval(fetchUploads, 5000);
    </script>
</body>
</html>
"#;

async fn index_handler() -> Html<&'static str> {
    Html(INDEX_HTML)
}

async fn list_uploads(
    State(state): State<Arc<RwLock<ServerState>>>,
) -> Json<Vec<UploadInfo>> {
    let lock = state.read().await;
    let mut list: Vec<UploadInfo> = lock.uploads.values().cloned().collect();
    // Sort by started_at descending
    list.sort_by(|a, b| b.started_at.cmp(&a.started_at));
    Json(list)
}

async fn register_upload(
    State(state): State<Arc<RwLock<ServerState>>>,
    Json(payload): Json<RegisterRequest>,
) -> Json<serde_json::Value> {
    let mut lock = state.write().await;
    let file_path = std::path::Path::new(&lock.upload_dir).join(&payload.file_name);
    
    // Check if the upload already exists and determine the safe resume offset
    let (bytes_received, status, delete_at) = if let Some(existing) = lock.uploads.get(&payload.packet_code) {
        let disk_size = if file_path.exists() {
            file_path.metadata().map(|m| m.len()).unwrap_or(0)
        } else {
            0
        };
        // The resume point is the minimum of what we recorded and what is actually on disk
        let resume_offset = existing.bytes_received.min(disk_size);
        let new_delete_at = if existing.status == "Hoàn thành" {
            Some(Utc::now() + chrono::Duration::minutes(lock.completed_timeout_mins))
        } else {
            Some(Utc::now() + chrono::Duration::minutes(lock.incomplete_timeout_mins))
        };
        (resume_offset, existing.status.clone(), new_delete_at)
    } else {
        let new_delete_at = Some(Utc::now() + chrono::Duration::minutes(lock.incomplete_timeout_mins));
        (0, "Đang nhận".to_string(), new_delete_at)
    };

    let started_at = Utc::now();
    let info = UploadInfo {
        packet_code: payload.packet_code.clone(),
        file_name: payload.file_name.clone(),
        file_size: payload.file_size,
        bytes_received,
        status: status.clone(),
        started_at,
        completed_at: if status == "Hoàn thành" { Some(started_at) } else { None },
        delete_at,
    };
    lock.uploads.insert(payload.packet_code.clone(), info.clone());

    // Save to SQLite
    let db_path = lock.db_path.clone();
    let save_info = info.clone();
    tokio::task::spawn_blocking(move || {
        if let Err(e) = save_upload_to_db(&db_path, &save_info) {
            eprintln!("[DB] Failed to save register to DB: {}", e);
        }
    });

    // Only create/truncate the file if it's a fresh upload
    if bytes_received == 0 {
        if let Some(parent) = file_path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        if let Ok(file) = std::fs::File::create(&file_path) {
            let _ = file.set_len(payload.file_size);
        }
    } else {
        // If resuming, just ensure the file exists
        if !file_path.exists() {
            if let Some(parent) = file_path.parent() {
                let _ = std::fs::create_dir_all(parent);
            }
            if let Ok(file) = std::fs::File::create(&file_path) {
                let _ = file.set_len(payload.file_size);
            }
        }
    }
    
    println!("[HTTP] Registered upload. File: {}, Size: {}, Hash ID: {}, Resume Offset: {}", 
             payload.file_name, payload.file_size, payload.packet_code, bytes_received);

    Json(json!({
        "status": "registered",
        "packet_code": payload.packet_code,
        "bytes_received": bytes_received
    }))
}

async fn run_cleanup_worker(state: Arc<RwLock<ServerState>>) {
    let interval_mins = {
        let lock = state.read().await;
        lock.cleanup_interval
    };
    println!("File retention cleanup worker started. Scan interval: {} minutes.", interval_mins);
    loop {
        // Run checks based on the configured cleanup interval
        tokio::time::sleep(tokio::time::Duration::from_secs(interval_mins * 60)).await;
        let now = Utc::now();
        let mut to_delete = Vec::new();

        {
            let lock = state.read().await;
            for (code, upload) in &lock.uploads {
                if let Some(delete_at) = upload.delete_at {
                    if now >= delete_at {
                        to_delete.push((code.clone(), upload.file_name.clone(), upload.status == "Hoàn thành"));
                    }
                }
            }
        }

        for (code, file_name, is_completed) in to_delete {
            let mut lock = state.write().await;
            lock.uploads.remove(&code);

            let file_path = std::path::Path::new(&lock.upload_dir).join(&file_name);
            let _ = std::fs::remove_file(&file_path);

            let db_path = lock.db_path.clone();
            let code_clone = code.clone();
            let _ = tokio::task::spawn_blocking(move || {
                if let Ok(conn) = rusqlite::Connection::open(&db_path) {
                    let _ = conn.execute(
                        "DELETE FROM uploads WHERE packet_code = ?1",
                        rusqlite::params![code_clone],
                    );
                }
            }).await;

            let type_str = if is_completed { "completed" } else { "incomplete" };
            println!(
                "[Cleanup] Automatically deleted {} file and logs for Hash ID: {}, file name: {}",
                type_str, code, file_name
            );
        }
    }
}

async fn run_udp_server(state: Arc<RwLock<ServerState>>, port: u16) {
    let addr = SocketAddr::from(([0, 0, 0, 0], port));
    let socket = match std::net::UdpSocket::bind(addr) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("Failed to bind UDP socket to port {}: {}", port, e);
            return;
        }
    };
    println!("UDP Server running on {}", addr);

    socket.set_nonblocking(true).unwrap();
    let socket = tokio::net::UdpSocket::from_std(socket).unwrap();

    let mut buf = vec![0u8; 65535];

    loop {
        match socket.recv_from(&mut buf).await {
            Ok((len, src)) => {
                let pkt_buf = &buf[..len];
                match common::UdpPacket::parse(pkt_buf) {
                    Ok(packet) => {
                        let unique_id = common::bytes_to_unique_id(packet.packet_code);
                        let is_end = packet.status == 0;

                         let file_name = {
                            let mut lock = state.write().await;
                            let incomplete_timeout = lock.incomplete_timeout_mins;
                            let completed_timeout = lock.completed_timeout_mins;
                            let upload_dir = lock.upload_dir.clone();
                            let entry = lock.uploads.entry(unique_id.clone()).or_insert_with(|| {
                                let name = format!("upload_{}.bin", unique_id);
                                let file_path = std::path::Path::new(&upload_dir).join(&name);
                                if let Some(parent) = file_path.parent() {
                                    let _ = std::fs::create_dir_all(parent);
                                }
                                let _ = std::fs::File::create(&file_path);
                                let delete_at = if is_end {
                                    Some(Utc::now() + chrono::Duration::minutes(completed_timeout))
                                } else {
                                    Some(Utc::now() + chrono::Duration::minutes(incomplete_timeout))
                                };
                                UploadInfo {
                                    packet_code: unique_id.clone(),
                                    file_name: name.clone(),
                                    file_size: if is_end { packet.seek_begin } else { 0 },
                                    bytes_received: 0,
                                    status: "Đang nhận".to_string(),
                                    started_at: Utc::now(),
                                    completed_at: None,
                                    delete_at,
                                }
                            });
                            entry.file_name.clone()
                        };

                        let mut write_success = is_end; // End packet has no data, treated as success
                        if !is_end && !packet.data.is_empty() {
                            let file_path = std::path::Path::new(&state.read().await.upload_dir).join(&file_name);
                            let seek_begin = packet.seek_begin;
                            let data = packet.data.to_vec();

                            let write_res = tokio::task::spawn_blocking(move || {
                                use std::fs::OpenOptions;
                                use std::io::{Seek, SeekFrom, Write};
                                let mut file = OpenOptions::new().write(true).open(&file_path)?;
                                file.seek(SeekFrom::Start(seek_begin))?;
                                file.write_all(&data)?;
                                file.flush()?;
                                Ok::<(), std::io::Error>(())
                            }).await;

                            if let Ok(Ok(())) = write_res {
                                write_success = true;
                            }
                        }

                        if write_success {
                            let mut lock = state.write().await;
                            let db_path = lock.db_path.clone();
                            let completed_timeout = lock.completed_timeout_mins;
                            let incomplete_timeout = lock.incomplete_timeout_mins;
                            if let Some(entry) = lock.uploads.get_mut(&unique_id) {
                                if is_end {
                                    entry.status = "Hoàn thành".to_string();
                                    entry.completed_at = Some(Utc::now());
                                    entry.file_size = packet.seek_begin;
                                    entry.bytes_received = packet.seek_begin;
                                    entry.delete_at = Some(Utc::now() + chrono::Duration::minutes(completed_timeout));
                                    println!("[UDP] Completed upload of file: {}", entry.file_name);
                                } else {
                                    let end_pos = packet.seek_begin + packet.data.len() as u64;
                                    if end_pos > entry.bytes_received {
                                        entry.bytes_received = end_pos;
                                    }
                                    if entry.file_size < entry.bytes_received {
                                        entry.file_size = entry.bytes_received;
                                    }
                                    entry.delete_at = Some(Utc::now() + chrono::Duration::minutes(incomplete_timeout));
                                }

                                // Save to SQLite
                                let save_info = entry.clone();
                                tokio::task::spawn_blocking(move || {
                                    if let Err(e) = save_upload_to_db(&db_path, &save_info) {
                                        eprintln!("[DB] Failed to save update to DB: {}", e);
                                    }
                                });
                            }

                            // Send back ACK only on success
                            let ack_bytes = if is_end {
                                common::AckPacket::serialize(packet.packet_code, packet.seek_begin, 0)
                            } else {
                                common::AckPacket::serialize(packet.packet_code, packet.seek_begin, packet.data.len() as u64)
                            };
                            let _ = socket.send_to(&ack_bytes, src).await;
                        }
                    }
                    Err(e) => {
                        eprintln!("[UDP] Error parsing packet: {}", e);
                    }
                }
            }
            Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                tokio::time::sleep(std::time::Duration::from_millis(2)).await;
            }
            Err(e) => {
                eprintln!("[UDP] Recv error: {}", e);
            }
        }
    }
}

#[tokio::main]
async fn main() {
    // Parse command line arguments
    let args = Args::parse();

    let upload_dir = args.upload_dir.clone();
    let _ = std::fs::create_dir_all(&upload_dir);

    // Initialize SQLite database
    let db_path = args.db_path.clone();
    let db_dir = std::path::Path::new(&db_path).parent().unwrap_or(std::path::Path::new("."));
    let _ = std::fs::create_dir_all(db_dir);
    if let Err(e) = init_db(&db_path) {
        eprintln!("[DB] Failed to initialize SQLite database: {}", e);
    }

    // Load existing uploads from SQLite
    let uploads = match load_uploads_from_db(&db_path) {
        Ok(map) => {
            println!("[DB] Loaded {} uploads from SQLite database.", map.len());
            map
        }
        Err(e) => {
            eprintln!("[DB] Failed to load uploads from DB: {}. Starting with empty cache.", e);
            HashMap::new()
        }
    };

    let state = Arc::new(RwLock::new(ServerState {
        uploads,
        upload_dir: upload_dir.clone(),
        db_path,
        cleanup_interval: args.cleanup_interval,
        incomplete_timeout_mins: args.incomplete_timeout,
        completed_timeout_mins: args.completed_timeout,
    }));

    // Start UDP Server Task
    let udp_state = state.clone();
    let udp_port = args.udp_port;
    tokio::spawn(async move {
        run_udp_server(udp_state, udp_port).await;
    });

    // Start Cleanup Worker Task
    let cleanup_state = state.clone();
    tokio::spawn(async move {
        run_cleanup_worker(cleanup_state).await;
    });

    // Start HTTP Server
    let app = Router::new()
        .route("/", get(index_handler))
        .route("/api/register", post(register_upload))
        .route("/api/list", get(list_uploads))
        .nest_service("/uploads", ServeDir::new(&upload_dir))
        .layer(CorsLayer::permissive())
        .with_state(state);

    let http_addr = SocketAddr::from(([0, 0, 0, 0], args.http_port));
    println!("HTTP Server running on http://{}", http_addr);
    let listener = tokio::net::TcpListener::bind(http_addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}
