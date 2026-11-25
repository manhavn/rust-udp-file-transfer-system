use clap::{Args, Subcommand};
use std::env;
use std::ffi::OsString;
use std::fs;
use std::path::{Component, Path, PathBuf};

pub const DEFAULT_SERVICE_NAME: &str = "rtk-udp-server";
const DEFAULT_UDP_PORT: u16 = 5000;
const DEFAULT_HTTP_PORT: u16 = 8080;
const DEFAULT_CLEANUP_INTERVAL: u64 = 5;
const DEFAULT_INCOMPLETE_TIMEOUT: i64 = 60;
const DEFAULT_COMPLETED_TIMEOUT: i64 = 15;

#[derive(Subcommand, Debug, Clone)]
pub enum ServiceCommand {
    /// Khởi động service systemd
    Start(ScopeArgs),
    /// Dừng service systemd
    Stop(ScopeArgs),
    /// Khởi động lại service systemd
    Restart(ScopeArgs),
    /// Kiểm tra trạng thái hoạt động và cấu hình tự khởi động của service
    Status(StatusArgs),
    /// Bật tự khởi động service cùng hệ thống (tự tạo unit nếu chưa có)
    Enable(EnableArgs),
    /// Tắt tự khởi động service cùng hệ thống
    Disable(DisableArgs),
    /// Tạo hoặc cập nhật tệp systemd unit mà không bật/chạy ngay
    Install(InstallArgs),
    /// Gỡ bỏ tệp systemd unit (dừng và tắt service trước, giữ nguyên dữ liệu)
    #[command(visible_alias = "remove")]
    Uninstall(ScopeArgs),
    /// Xem nhật ký hoạt động của service qua journalctl
    Logs(LogsArgs),
}

#[derive(Args, Debug, Clone)]
pub struct ScopeArgs {
    /// Quản lý service trong phạm vi user (~/.config/systemd/user)
    #[arg(long, conflicts_with = "system")]
    pub user: bool,

    /// Quản lý service trong phạm vi toàn hệ thống (/etc/systemd/system, cần quyền root/sudo)
    #[arg(long, conflicts_with = "user")]
    pub system: bool,

    /// Tên systemd service
    #[arg(long, default_value = DEFAULT_SERVICE_NAME)]
    pub name: String,
}

#[derive(Args, Debug, Clone, Default)]
pub struct ServiceConfigOverrides {
    /// Cổng UDP lắng nghe
    #[arg(short = 'u', long, env = "UDP_PORT")]
    pub udp_port: Option<u16>,

    /// Cổng HTTP REST API & Dashboard
    #[arg(short = 't', long, env = "HTTP_PORT")]
    pub http_port: Option<u16>,

    /// Thư mục chứa các tệp tải lên
    #[arg(long, env = "UPLOAD_DIR")]
    pub upload_dir: Option<String>,

    /// Đường dẫn cơ sở dữ liệu SQLite
    #[arg(long, env = "DB_PATH")]
    pub db_path: Option<String>,

    /// Chu kỳ quét dọn dẹp tệp tin (phút)
    #[arg(long, env = "CLEANUP_INTERVAL")]
    pub cleanup_interval: Option<u64>,

    /// Thời gian tối đa lưu trữ tệp chưa hoàn thành (phút)
    #[arg(long, env = "INCOMPLETE_TIMEOUT")]
    pub incomplete_timeout: Option<i64>,

    /// Thời gian tối đa lưu trữ tệp đã hoàn thành (phút)
    #[arg(long, env = "COMPLETED_TIMEOUT")]
    pub completed_timeout: Option<i64>,

    /// Tắt toàn bộ output log request của HTTP server
    #[arg(long, env = "DISABLE_REQUEST_LOG", default_missing_value = "true", num_args = 0..=1)]
    pub disable_request_log: Option<bool>,

    /// Thư mục làm việc (WorkingDirectory) của service
    #[arg(long)]
    pub working_dir: Option<PathBuf>,

    /// Chỉ định đường dẫn file thực thi cho ExecStart (ưu tiên dùng shim khi chạy qua mise)
    #[arg(long)]
    pub bin: Option<PathBuf>,

    /// Tên user Linux chạy service (chỉ áp dụng khi dùng --system)
    #[arg(long)]
    pub service_user: Option<String>,
}

impl ServiceConfigOverrides {
    pub fn has_any(&self) -> bool {
        self.udp_port.is_some()
            || self.http_port.is_some()
            || self.upload_dir.is_some()
            || self.db_path.is_some()
            || self.cleanup_interval.is_some()
            || self.incomplete_timeout.is_some()
            || self.completed_timeout.is_some()
            || self.disable_request_log.is_some()
            || self.working_dir.is_some()
            || self.bin.is_some()
            || self.service_user.is_some()
    }
}

#[derive(Args, Debug, Clone)]
pub struct InstallArgs {
    #[command(flatten)]
    pub scope: ScopeArgs,

    #[command(flatten)]
    pub config: ServiceConfigOverrides,
}

#[derive(Args, Debug, Clone)]
pub struct EnableArgs {
    #[command(flatten)]
    pub scope: ScopeArgs,

    #[command(flatten)]
    pub config: ServiceConfigOverrides,

    /// Khởi động service ngay sau khi bật tự khởi động (systemctl enable --now)
    #[arg(long)]
    pub now: bool,

    /// Ghi đè lại tệp systemd unit ngay cả khi đã tồn tại
    #[arg(long)]
    pub force: bool,

    /// Không tự động chạy loginctl enable-linger khi bật user service
    #[arg(long)]
    pub no_linger: bool,
}

#[derive(Args, Debug, Clone)]
pub struct DisableArgs {
    #[command(flatten)]
    pub scope: ScopeArgs,

    /// Dừng service ngay khi tắt tự khởi động (systemctl disable --now)
    #[arg(long)]
    pub now: bool,
}

#[derive(Args, Debug, Clone)]
pub struct StatusArgs {
    #[command(flatten)]
    pub scope: ScopeArgs,

    /// Hiển thị thêm output đầy đủ từ `systemctl status`
    #[arg(short, long)]
    pub verbose: bool,

    /// Chạy trực tiếp `systemctl status --no-pager`
    #[arg(long)]
    pub raw: bool,
}

#[derive(Args, Debug, Clone)]
pub struct LogsArgs {
    #[command(flatten)]
    pub scope: ScopeArgs,

    /// Theo dõi log liên tục (journalctl -f)
    #[arg(short, long)]
    pub follow: bool,

    /// Số dòng log gần nhất cần hiển thị
    #[arg(short = 'n', long, default_value_t = 50)]
    pub lines: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ServiceScope {
    User,
    System,
}

impl ServiceScope {
    pub fn label(self) -> &'static str {
        match self {
            ServiceScope::User => "user",
            ServiceScope::System => "system",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolvedExecutable {
    pub path: PathBuf,
    pub is_mise_shim: bool,
    pub mise_data_dir: Option<PathBuf>,
    pub mise_owner_home: Option<PathBuf>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UnitConfig {
    pub service_name: String,
    pub scope: ServiceScope,
    pub exec_path: PathBuf,
    pub is_mise_shim: bool,
    pub working_dir: PathBuf,
    pub upload_dir: PathBuf,
    pub db_path: PathBuf,
    pub udp_port: u16,
    pub http_port: u16,
    pub cleanup_interval: u64,
    pub incomplete_timeout: i64,
    pub completed_timeout: i64,
    pub disable_request_log: bool,
    pub service_user: Option<String>,
    pub env_vars: Vec<(String, String)>,
}

#[derive(Debug, Clone, Default)]
struct EnvSnapshot {
    mise_data_dir: Option<PathBuf>,
    mise_shims_dir: Option<PathBuf>,
    mise_config_dir: Option<PathBuf>,
    xdg_data_home: Option<PathBuf>,
    xdg_config_home: Option<PathBuf>,
    home: Option<PathBuf>,
}

impl EnvSnapshot {
    fn from_process() -> Self {
        Self {
            mise_data_dir: non_empty_path_env("MISE_DATA_DIR"),
            mise_shims_dir: non_empty_path_env("MISE_SHIMS_DIR"),
            mise_config_dir: non_empty_path_env("MISE_CONFIG_DIR"),
            xdg_data_home: non_empty_path_env("XDG_DATA_HOME"),
            xdg_config_home: non_empty_path_env("XDG_CONFIG_HOME"),
            home: non_empty_path_env("HOME"),
        }
    }
}

fn non_empty_path_env(key: &str) -> Option<PathBuf> {
    env::var_os(key)
        .filter(|v| !v.is_empty())
        .map(PathBuf::from)
}

pub fn normalize_service_name(raw: &str) -> Result<String, String> {
    let trimmed = raw.trim();
    let base = trimmed.strip_suffix(".service").unwrap_or(trimmed);
    if base.is_empty() {
        return Err("Tên service không được để trống.".to_string());
    }
    if base.contains("..") {
        return Err(format!("Tên service không hợp lệ: '{}'", raw));
    }
    let valid = base
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || matches!(c, '@' | '.' | '_' | '-'));
    if !valid {
        return Err(format!(
            "Tên service '{}' chứa ký tự không hợp lệ (chỉ cho phép a-z, A-Z, 0-9, '-', '_', '.', '@').",
            raw
        ));
    }
    Ok(base.to_string())
}

pub fn make_lexical_absolute(path: &Path, base_dir: &Path) -> PathBuf {
    let joined = if path.is_absolute() {
        path.to_path_buf()
    } else {
        base_dir.join(path)
    };
    let mut normalized = PathBuf::new();
    for comp in joined.components() {
        match comp {
            Component::Prefix(p) => normalized.push(p.as_os_str()),
            Component::RootDir => normalized.push(comp.as_os_str()),
            Component::CurDir => {}
            Component::ParentDir => {
                if !normalized.pop() {
                    // At root; ignore extra `..`
                }
            }
            Component::Normal(c) => normalized.push(c),
        }
    }
    if normalized.as_os_str().is_empty() {
        PathBuf::from("/")
    } else {
        normalized
    }
}

/// Kiểm tra xem một path có tồn tại trên filesystem hay không mà KHÔNG dereference symlink.
fn path_entry_exists(path: &Path) -> bool {
    fs::symlink_metadata(path).is_ok()
}

fn infer_mise_owner_home(mise_root: &Path) -> Option<PathBuf> {
    let comps: Vec<&std::ffi::OsStr> = mise_root
        .components()
        .filter_map(|c| match c {
            Component::Normal(s) => Some(s),
            _ => None,
        })
        .collect();
    if comps.len() >= 3
        && comps[comps.len() - 1] == "mise"
        && comps[comps.len() - 2] == "share"
        && comps[comps.len() - 3] == ".local"
    {
        return mise_root
            .parent()
            .and_then(|p| p.parent())
            .and_then(|p| p.parent())
            .map(Path::to_path_buf);
    }
    None
}

fn looks_like_mise_root(dir: &Path, env_snap: &EnvSnapshot) -> bool {
    if let Some(ref mise_data) = env_snap.mise_data_dir {
        if dir == mise_data {
            return true;
        }
    }
    if dir.file_name().and_then(|n| n.to_str()) == Some("mise") {
        return true;
    }
    path_entry_exists(&dir.join("shims"))
}

fn detect_mise_root_from_install_path(exe_path: &Path, env_snap: &EnvSnapshot) -> Option<PathBuf> {
    let components: Vec<Component<'_>> = exe_path.components().collect();
    for (idx, comp) in components.iter().enumerate() {
        if let Component::Normal(name) = comp {
            if *name == "installs" {
                // Cần có ít nhất <mise_root>/installs/<tool>/<exe_name> (hoặc <tool>/<version>/<exe_name>)
                if idx == 0 || idx + 2 >= components.len() {
                    continue;
                }
                let mut mise_root = PathBuf::new();
                for c in &components[..idx] {
                    mise_root.push(c.as_os_str());
                }
                if looks_like_mise_root(&mise_root, env_snap) {
                    return Some(mise_root);
                }
            }
        }
    }
    None
}

fn detect_mise_root_from_shim_path(shim_path: &Path, env_snap: &EnvSnapshot) -> Option<PathBuf> {
    let parent = shim_path.parent()?;
    if parent.file_name().and_then(|n| n.to_str()) != Some("shims") {
        if let Some(ref shims_dir) = env_snap.mise_shims_dir {
            if parent == shims_dir {
                return env_snap
                    .mise_data_dir
                    .clone()
                    .or_else(|| parent.parent().map(Path::to_path_buf));
            }
        }
        return None;
    }
    let grandparent = parent.parent()?;
    if looks_like_mise_root(grandparent, env_snap)
        || path_entry_exists(&grandparent.join("installs"))
    {
        return Some(grandparent.to_path_buf());
    }
    None
}

fn push_unique_dir(dirs: &mut Vec<PathBuf>, dir: PathBuf) {
    if !dirs.iter().any(|d| d == &dir) {
        dirs.push(dir);
    }
}

fn resolve_executable_inner(
    explicit_bin: Option<&Path>,
    current_exe: &Path,
    argv0: Option<&OsString>,
    cwd: &Path,
    env_snap: &EnvSnapshot,
) -> Result<ResolvedExecutable, String> {
    if let Some(bin_path) = explicit_bin {
        let abs_bin = make_lexical_absolute(bin_path, cwd);
        if !path_entry_exists(&abs_bin) {
            return Err(format!(
                "Đường dẫn thực thi được chỉ định qua --bin không tồn tại: {}",
                abs_bin.display()
            ));
        }
        if let Some(mise_root) = detect_mise_root_from_shim_path(&abs_bin, env_snap) {
            let mise_owner_home = infer_mise_owner_home(&mise_root);
            return Ok(ResolvedExecutable {
                path: abs_bin,
                is_mise_shim: true,
                mise_data_dir: Some(mise_root),
                mise_owner_home,
            });
        }
        return Ok(ResolvedExecutable {
            path: abs_bin,
            is_mise_shim: false,
            mise_data_dir: None,
            mise_owner_home: None,
        });
    }

    // Kiểm tra xem binary đang chạy có nằm trong thư mục cài đặt version của mise hay không
    let mise_root_from_install = detect_mise_root_from_install_path(current_exe, env_snap);

    // Kiểm tra xem argv0 có trỏ trực tiếp tới một shim trong mise/shims không
    let argv0_shim_candidate = argv0.and_then(|raw| {
        let p = Path::new(raw);
        if p.components().count() > 1 {
            let abs = make_lexical_absolute(p, cwd);
            if path_entry_exists(&abs) && detect_mise_root_from_shim_path(&abs, env_snap).is_some()
            {
                return Some(abs);
            }
        }
        None
    });

    if mise_root_from_install.is_some() || argv0_shim_candidate.is_some() {
        let exe_name = current_exe
            .file_name()
            .ok_or_else(|| "Không xác định được tên file thực thi.".to_string())?;

        if let Some(shim_from_argv0) = argv0_shim_candidate {
            // Đảm bảo tên file khớp với binary hiện tại hoặc ưu tiên shim từ argv0
            let mise_root = detect_mise_root_from_shim_path(&shim_from_argv0, env_snap)
                .or(mise_root_from_install);
            let mise_owner_home = mise_root.as_deref().and_then(infer_mise_owner_home);
            return Ok(ResolvedExecutable {
                path: shim_from_argv0,
                is_mise_shim: true,
                mise_data_dir: mise_root,
                mise_owner_home,
            });
        }

        let mise_root = mise_root_from_install.unwrap();
        let mut shim_dirs = Vec::new();
        push_unique_dir(&mut shim_dirs, mise_root.join("shims"));
        if let Some(ref d) = env_snap.mise_shims_dir {
            push_unique_dir(&mut shim_dirs, d.clone());
        }
        if let Some(ref d) = env_snap.mise_data_dir {
            push_unique_dir(&mut shim_dirs, d.join("shims"));
        }
        if let Some(ref xdg) = env_snap.xdg_data_home {
            push_unique_dir(&mut shim_dirs, xdg.join("mise").join("shims"));
        }
        if let Some(ref home) = env_snap.home {
            push_unique_dir(&mut shim_dirs, home.join(".local/share/mise/shims"));
        }

        for dir in &shim_dirs {
            let candidate = dir.join(exe_name);
            // TUYỆT ĐỐI KHÔNG gọi canonicalize() trên shim của mise!
            if path_entry_exists(&candidate) {
                let effective_mise_root =
                    detect_mise_root_from_shim_path(&candidate, env_snap).unwrap_or(mise_root);
                let mise_owner_home = infer_mise_owner_home(&effective_mise_root);
                return Ok(ResolvedExecutable {
                    path: candidate,
                    is_mise_shim: true,
                    mise_data_dir: Some(effective_mise_root),
                    mise_owner_home,
                });
            }
        }

        return Err(format!(
            "Phát hiện binary đang chạy từ thư mục version của mise ({}), nhưng không tìm thấy shim '{}' trong shims.\n\
             Không thể dùng đường dẫn theo version vì sẽ hỏng khi nâng cấp.\n\
             Hãy chạy `mise reshim` rồi thử lại, hoặc truyền `--bin <đường_dẫn_shim>`.",
            current_exe.display(),
            exe_name.to_string_lossy()
        ));
    }

    Ok(ResolvedExecutable {
        path: make_lexical_absolute(current_exe, cwd),
        is_mise_shim: false,
        mise_data_dir: None,
        mise_owner_home: None,
    })
}

pub fn resolve_executable_path(explicit_bin: Option<&Path>) -> Result<ResolvedExecutable, String> {
    let current_exe = env::current_exe()
        .map_err(|e| format!("Không thể xác định đường dẫn file thực thi hiện tại: {}", e))?;
    let argv0 = env::args_os().next();
    let cwd = env::current_dir().unwrap_or_else(|_| PathBuf::from("/"));
    let env_snap = EnvSnapshot::from_process();
    resolve_executable_inner(explicit_bin, &current_exe, argv0.as_ref(), &cwd, &env_snap)
}

fn current_euid() -> u32 {
    if let Ok(status) = fs::read_to_string("/proc/self/status") {
        for line in status.lines() {
            if let Some(rest) = line.strip_prefix("Uid:") {
                let mut cols = rest.split_whitespace();
                let _real = cols.next();
                if let Some(effective) = cols.next() {
                    if let Ok(uid) = effective.parse::<u32>() {
                        return uid;
                    }
                }
            }
        }
    }
    if let Ok(out) = std::process::Command::new("id").arg("-u").output() {
        if out.status.success() {
            if let Ok(uid) = String::from_utf8_lossy(&out.stdout).trim().parse::<u32>() {
                return uid;
            }
        }
    }
    1000
}

fn unit_file_path_with_env(
    scope: ServiceScope,
    service_name: &str,
    env_snap: &EnvSnapshot,
) -> Result<PathBuf, String> {
    match scope {
        ServiceScope::System => Ok(PathBuf::from(format!(
            "/etc/systemd/system/{}.service",
            service_name
        ))),
        ServiceScope::User => {
            let config_dir = if let Some(ref xdg) = env_snap.xdg_config_home {
                xdg.clone()
            } else if let Some(ref home) = env_snap.home {
                home.join(".config")
            } else {
                return Err(
                    "Không xác định được thư mục HOME/XDG_CONFIG_HOME để lưu user unit."
                        .to_string(),
                );
            };
            Ok(config_dir
                .join("systemd")
                .join("user")
                .join(format!("{}.service", service_name)))
        }
    }
}

pub fn unit_file_path(scope: ServiceScope, service_name: &str) -> Result<PathBuf, String> {
    let env_snap = EnvSnapshot::from_process();
    unit_file_path_with_env(scope, service_name, &env_snap)
}

fn default_working_dir_with_env(
    scope: ServiceScope,
    env_snap: &EnvSnapshot,
) -> Result<PathBuf, String> {
    match scope {
        ServiceScope::System => Ok(PathBuf::from("/var/lib/rtk-udp-server")),
        ServiceScope::User => {
            let data_home = if let Some(ref xdg) = env_snap.xdg_data_home {
                xdg.clone()
            } else if let Some(ref home) = env_snap.home {
                home.join(".local/share")
            } else {
                return Err(
                    "Không xác định được thư mục HOME/XDG_DATA_HOME cho thư mục dữ liệu."
                        .to_string(),
                );
            };
            Ok(data_home.join("rtk-udp-server"))
        }
    }
}

fn resolve_scope(
    scope_args: &ScopeArgs,
    service_name: &str,
    prefer_existing: bool,
) -> ServiceScope {
    if scope_args.user {
        return ServiceScope::User;
    }
    if scope_args.system {
        return ServiceScope::System;
    }
    if current_euid() == 0 {
        return ServiceScope::System;
    }
    if prefer_existing {
        let env_snap = EnvSnapshot::from_process();
        if let Ok(user_unit) = unit_file_path_with_env(ServiceScope::User, service_name, &env_snap)
        {
            if path_entry_exists(&user_unit) {
                return ServiceScope::User;
            }
        }
        if let Ok(sys_unit) = unit_file_path_with_env(ServiceScope::System, service_name, &env_snap)
        {
            if path_entry_exists(&sys_unit) {
                return ServiceScope::System;
            }
        }
    }
    ServiceScope::User
}

pub fn systemd_escape_arg(arg: &str) -> String {
    let needs_quotes = arg.is_empty()
        || arg
            .chars()
            .any(|c| c.is_whitespace() || matches!(c, '"' | '\'' | '\\' | '$' | '`'));
    let escaped_percent = arg.replace('%', "%%");
    if !needs_quotes {
        return escaped_percent;
    }
    let mut out = String::with_capacity(escaped_percent.len() + 2);
    out.push('"');
    for ch in escaped_percent.chars() {
        match ch {
            '\\' => out.push_str("\\\\"),
            '"' => out.push_str("\\\""),
            '$' => out.push_str("\\$"),
            _ => out.push(ch),
        }
    }
    out.push('"');
    out
}

fn systemd_escape_env(key: &str, value: &str) -> String {
    let pair = format!("{}={}", key, value);
    let escaped_percent = pair.replace('%', "%%");
    let mut out = String::with_capacity(escaped_percent.len() + 2);
    out.push('"');
    for ch in escaped_percent.chars() {
        match ch {
            '\\' => out.push_str("\\\\"),
            '"' => out.push_str("\\\""),
            _ => out.push(ch),
        }
    }
    out.push('"');
    out
}

fn collect_mise_env_vars(
    scope: ServiceScope,
    resolved: &ResolvedExecutable,
    service_user: Option<&str>,
    env_snap: &EnvSnapshot,
) -> Vec<(String, String)> {
    if !resolved.is_mise_shim {
        return Vec::new();
    }
    let mut vars = Vec::new();
    match scope {
        ServiceScope::System => {
            if let Some(ref data_dir) = resolved.mise_data_dir {
                vars.push((
                    "MISE_DATA_DIR".to_string(),
                    data_dir.to_string_lossy().into_owned(),
                ));
            } else if let Some(ref data_dir) = env_snap.mise_data_dir {
                vars.push((
                    "MISE_DATA_DIR".to_string(),
                    data_dir.to_string_lossy().into_owned(),
                ));
            }

            if service_user.is_none() {
                if let Some(ref owner_home) = resolved.mise_owner_home {
                    vars.push((
                        "HOME".to_string(),
                        owner_home.to_string_lossy().into_owned(),
                    ));
                }
            }

            if let Some(ref cfg_dir) = env_snap.mise_config_dir {
                vars.push((
                    "MISE_CONFIG_DIR".to_string(),
                    cfg_dir.to_string_lossy().into_owned(),
                ));
            } else if let Some(ref owner_home) = resolved.mise_owner_home {
                let candidate = owner_home.join(".config/mise");
                if path_entry_exists(&candidate) {
                    vars.push((
                        "MISE_CONFIG_DIR".to_string(),
                        candidate.to_string_lossy().into_owned(),
                    ));
                }
            }
        }
        ServiceScope::User => {
            if let Some(ref v) = env_snap.mise_data_dir {
                vars.push((
                    "MISE_DATA_DIR".to_string(),
                    v.to_string_lossy().into_owned(),
                ));
            }
            if let Some(ref v) = env_snap.mise_config_dir {
                vars.push((
                    "MISE_CONFIG_DIR".to_string(),
                    v.to_string_lossy().into_owned(),
                ));
            }
            if let Some(ref v) = env_snap.xdg_data_home {
                vars.push((
                    "XDG_DATA_HOME".to_string(),
                    v.to_string_lossy().into_owned(),
                ));
            }
            if let Some(ref v) = env_snap.xdg_config_home {
                vars.push((
                    "XDG_CONFIG_HOME".to_string(),
                    v.to_string_lossy().into_owned(),
                ));
            }
        }
    }
    vars
}

pub fn render_unit_file(cfg: &UnitConfig) -> String {
    let mut exec_parts = vec![
        systemd_escape_arg(&cfg.exec_path.to_string_lossy()),
        "--udp-port".to_string(),
        cfg.udp_port.to_string(),
        "--http-port".to_string(),
        cfg.http_port.to_string(),
        "--upload-dir".to_string(),
        systemd_escape_arg(&cfg.upload_dir.to_string_lossy()),
        "--db-path".to_string(),
        systemd_escape_arg(&cfg.db_path.to_string_lossy()),
        "--cleanup-interval".to_string(),
        cfg.cleanup_interval.to_string(),
        "--incomplete-timeout".to_string(),
        cfg.incomplete_timeout.to_string(),
        "--completed-timeout".to_string(),
        cfg.completed_timeout.to_string(),
    ];
    if cfg.disable_request_log {
        exec_parts.push("--disable-request-log".to_string());
    }

    let wanted_by = match cfg.scope {
        ServiceScope::User => "default.target",
        ServiceScope::System => "multi-user.target",
    };

    let mut lines = vec![
        "[Unit]".to_string(),
        "Description=RTK UDP File Transfer Server".to_string(),
        "After=network-online.target".to_string(),
        "Wants=network-online.target".to_string(),
        "".to_string(),
        "[Service]".to_string(),
        "Type=simple".to_string(),
    ];

    if cfg.scope == ServiceScope::System {
        if let Some(ref user) = cfg.service_user {
            lines.push(format!("User={}", user));
        }
    }

    lines.push(format!(
        "WorkingDirectory={}",
        systemd_escape_arg(&cfg.working_dir.to_string_lossy())
    ));

    for (k, v) in &cfg.env_vars {
        lines.push(format!("Environment={}", systemd_escape_env(k, v)));
    }

    lines.push(format!("ExecStart={}", exec_parts.join(" ")));
    lines.push("Restart=on-failure".to_string());
    lines.push("RestartSec=5".to_string());
    lines.push(format!(
        "SyslogIdentifier={}",
        systemd_escape_arg(&cfg.service_name)
    ));
    lines.push("".to_string());
    lines.push("[Install]".to_string());
    lines.push(format!("WantedBy={}", wanted_by));
    lines.push("".to_string());

    lines.join("\n")
}

fn build_unit_config(
    service_name: &str,
    scope: ServiceScope,
    overrides: &ServiceConfigOverrides,
) -> Result<UnitConfig, String> {
    if scope == ServiceScope::User && overrides.service_user.is_some() {
        return Err("Tùy chọn --service-user chỉ áp dụng cho phạm vi --system.".to_string());
    }
    let env_snap = EnvSnapshot::from_process();
    let cwd = env::current_dir().unwrap_or_else(|_| PathBuf::from("/"));
    let resolved = resolve_executable_path(overrides.bin.as_deref())?;

    let working_dir = match &overrides.working_dir {
        Some(dir) => make_lexical_absolute(dir, &cwd),
        None => default_working_dir_with_env(scope, &env_snap)?,
    };

    let upload_dir = match &overrides.upload_dir {
        Some(dir) => make_lexical_absolute(Path::new(dir), &working_dir),
        None => working_dir.join("uploads"),
    };

    let db_path = match &overrides.db_path {
        Some(path) => make_lexical_absolute(Path::new(path), &working_dir),
        None => working_dir.join("db").join("data.sqlite"),
    };

    let env_vars = collect_mise_env_vars(
        scope,
        &resolved,
        overrides.service_user.as_deref(),
        &env_snap,
    );

    Ok(UnitConfig {
        service_name: service_name.to_string(),
        scope,
        exec_path: resolved.path,
        is_mise_shim: resolved.is_mise_shim,
        working_dir,
        upload_dir,
        db_path,
        udp_port: overrides.udp_port.unwrap_or(DEFAULT_UDP_PORT),
        http_port: overrides.http_port.unwrap_or(DEFAULT_HTTP_PORT),
        cleanup_interval: overrides
            .cleanup_interval
            .unwrap_or(DEFAULT_CLEANUP_INTERVAL),
        incomplete_timeout: overrides
            .incomplete_timeout
            .unwrap_or(DEFAULT_INCOMPLETE_TIMEOUT),
        completed_timeout: overrides
            .completed_timeout
            .unwrap_or(DEFAULT_COMPLETED_TIMEOUT),
        disable_request_log: overrides.disable_request_log.unwrap_or(false),
        service_user: overrides.service_user.clone(),
        env_vars,
    })
}

#[cfg(target_os = "linux")]
mod linux {
    use super::*;
    use std::collections::HashMap;
    use std::io::Write;
    use std::process::{Command, Stdio};

    fn unit_arg(service_name: &str) -> String {
        format!("{}.service", service_name)
    }

    fn systemctl_cmd(scope: ServiceScope) -> Command {
        let mut cmd = Command::new("systemctl");
        if scope == ServiceScope::User {
            cmd.arg("--user");
        }
        cmd
    }

    fn explain_systemctl_failure(scope: ServiceScope, stderr: &str) -> String {
        let trimmed = stderr.trim();
        let mut msg = if trimmed.is_empty() {
            "Lệnh systemctl thất bại.".to_string()
        } else {
            trimmed.to_string()
        };

        if scope == ServiceScope::System
            && (trimmed.contains("Access denied")
                || trimmed.contains("Permission denied")
                || trimmed.contains("Interactive authentication required"))
        {
            msg.push_str(
                "\nGợi ý: Phạm vi --system yêu cầu quyền quản trị. Hãy chạy lại với `sudo` (hoặc dùng `--user`).",
            );
        } else if scope == ServiceScope::User
            && (trimmed.contains("Failed to connect to bus")
                || trimmed.contains("No medium found")
                || trimmed.contains("XDG_RUNTIME_DIR"))
        {
            msg.push_str(
                "\nGợi ý: Không thể kết nối tới systemd user bus. Hãy đảm bảo phiên user có `XDG_RUNTIME_DIR` (thường là `/run/user/$(id -u)`) hoặc dùng `--system` với `sudo`.",
            );
        }
        msg
    }

    fn run_systemctl(scope: ServiceScope, args: &[&str]) -> Result<String, String> {
        let mut cmd = systemctl_cmd(scope);
        cmd.args(args);
        let output = cmd.output().map_err(|e| {
            format!(
                "Không thể thực thi `systemctl`: {}. Hãy kiểm tra máy đã cài đặt systemd chưa.",
                e
            )
        })?;
        if output.status.success() {
            Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
        } else {
            let stderr = String::from_utf8_lossy(&output.stderr);
            let stdout = String::from_utf8_lossy(&output.stdout);
            let combined = if stderr.trim().is_empty() {
                stdout.as_ref()
            } else {
                stderr.as_ref()
            };
            Err(explain_systemctl_failure(scope, combined))
        }
    }

    fn run_systemctl_inherit(scope: ServiceScope, args: &[&str]) -> i32 {
        let mut cmd = systemctl_cmd(scope);
        cmd.args(args)
            .stdin(Stdio::inherit())
            .stdout(Stdio::inherit())
            .stderr(Stdio::inherit());
        match cmd.status() {
            Ok(status) => status.code().unwrap_or(1),
            Err(e) => {
                eprintln!("Lỗi khi chạy systemctl: {}", e);
                1
            }
        }
    }

    fn write_unit_file(cfg: &UnitConfig) -> Result<PathBuf, String> {
        let unit_path = unit_file_path(cfg.scope, &cfg.service_name)?;
        if let Some(parent) = unit_path.parent() {
            fs::create_dir_all(parent).map_err(|e| {
                if cfg.scope == ServiceScope::System && current_euid() != 0 {
                    format!(
                        "Không thể tạo thư mục '{}': {}. Hãy chạy lại với `sudo` hoặc dùng `--user`.",
                        parent.display(),
                        e
                    )
                } else {
                    format!("Không thể tạo thư mục '{}': {}", parent.display(), e)
                }
            })?;
        }

        fs::create_dir_all(&cfg.working_dir).map_err(|e| {
            format!(
                "Không thể tạo thư mục làm việc '{}': {}",
                cfg.working_dir.display(),
                e
            )
        })?;
        fs::create_dir_all(&cfg.upload_dir).map_err(|e| {
            format!(
                "Không thể tạo thư mục upload '{}': {}",
                cfg.upload_dir.display(),
                e
            )
        })?;
        if let Some(db_parent) = cfg.db_path.parent() {
            fs::create_dir_all(db_parent).map_err(|e| {
                format!(
                    "Không thể tạo thư mục database '{}': {}",
                    db_parent.display(),
                    e
                )
            })?;
        }

        let content = render_unit_file(cfg);
        let mut file = fs::File::create(&unit_path).map_err(|e| {
            if cfg.scope == ServiceScope::System && current_euid() != 0 {
                format!(
                    "Không có quyền ghi tệp '{}': {}. Hãy chạy lại với `sudo` hoặc dùng `--user`.",
                    unit_path.display(),
                    e
                )
            } else {
                format!("Không thể ghi tệp unit '{}': {}", unit_path.display(), e)
            }
        })?;
        file.write_all(content.as_bytes())
            .map_err(|e| format!("Lỗi ghi nội dung tệp unit '{}': {}", unit_path.display(), e))?;

        run_systemctl(cfg.scope, &["daemon-reload"])?;
        Ok(unit_path)
    }

    fn current_username() -> Option<String> {
        if let Ok(user) = env::var("USER") {
            if !user.trim().is_empty() {
                return Some(user);
            }
        }
        if let Ok(out) = Command::new("id").arg("-un").output() {
            if out.status.success() {
                let u = String::from_utf8_lossy(&out.stdout).trim().to_string();
                if !u.is_empty() {
                    return Some(u);
                }
            }
        }
        None
    }

    fn check_linger_enabled(username: &str) -> Option<bool> {
        let out = Command::new("loginctl")
            .args(["show-user", username, "-p", "Linger", "--value"])
            .output()
            .ok()?;
        if !out.status.success() {
            return None;
        }
        let val = String::from_utf8_lossy(&out.stdout).trim().to_string();
        Some(val == "yes")
    }

    fn ensure_linger_for_user(no_linger: bool) {
        let Some(username) = current_username() else {
            return;
        };
        match check_linger_enabled(&username) {
            Some(true) => {}
            Some(false) | None => {
                if no_linger {
                    println!(
                        "Lưu ý: Bỏ qua bật linger (--no-linger). User service chỉ chạy khi user đang đăng nhập; bật thủ công bằng: sudo loginctl enable-linger {}",
                        username
                    );
                    return;
                }
                match Command::new("loginctl")
                    .args(["enable-linger", &username])
                    .output()
                {
                    Ok(out) if out.status.success() => {
                        println!(
                            "✔ Đã bật linger cho user '{}' để service tiếp tục chạy sau khi khởi động lại hệ thống mà không cần đăng nhập.",
                            username
                        );
                    }
                    _ => {
                        println!(
                            "Lưu ý: Chưa thể tự động bật linger cho user '{}'. Để service chạy ngay khi máy boot trước khi đăng nhập, hãy chạy:\n  sudo loginctl enable-linger {}",
                            username, username
                        );
                    }
                }
            }
        }
    }

    fn ensure_unit_exists_or_hint(
        scope: ServiceScope,
        service_name: &str,
    ) -> Result<PathBuf, String> {
        let unit_path = unit_file_path(scope, service_name)?;
        if path_entry_exists(&unit_path) {
            return Ok(unit_path);
        }
        let scope_flag = match scope {
            ServiceScope::User => "--user",
            ServiceScope::System => "--system",
        };
        Err(format!(
            "Chưa tìm thấy service '{}.service' ở phạm vi {} ({}).\nHãy chạy `rtk-udp-server install {}` hoặc `rtk-udp-server enable {} --now` trước.",
            service_name,
            scope.label(),
            unit_path.display(),
            scope_flag,
            scope_flag
        ))
    }

    fn format_memory(raw: Option<&String>) -> Option<String> {
        let val = raw?;
        if val == "[not set]" || val == "infinity" {
            return None;
        }
        let bytes: u64 = val.parse().ok()?;
        if bytes >= 1024 * 1024 * 1024 {
            Some(format!(
                "{:.2} GiB",
                bytes as f64 / (1024.0 * 1024.0 * 1024.0)
            ))
        } else if bytes >= 1024 * 1024 {
            Some(format!("{:.1} MiB", bytes as f64 / (1024.0 * 1024.0)))
        } else if bytes >= 1024 {
            Some(format!("{:.1} KiB", bytes as f64 / 1024.0))
        } else {
            Some(format!("{} B", bytes))
        }
    }

    fn parse_systemctl_show(output: &str) -> HashMap<String, String> {
        let mut map = HashMap::new();
        for line in output.lines() {
            if let Some((k, v)) = line.split_once('=') {
                map.insert(k.to_string(), v.to_string());
            }
        }
        map
    }

    fn cmd_install(args: InstallArgs) -> i32 {
        let service_name = match normalize_service_name(&args.scope.name) {
            Ok(n) => n,
            Err(e) => {
                eprintln!("Lỗi: {}", e);
                return 2;
            }
        };
        let scope = resolve_scope(&args.scope, &service_name, false);
        let cfg = match build_unit_config(&service_name, scope, &args.config) {
            Ok(c) => c,
            Err(e) => {
                eprintln!("Lỗi: {}", e);
                return 1;
            }
        };
        match write_unit_file(&cfg) {
            Ok(unit_path) => {
                println!(
                    "✔ Đã cài đặt systemd unit '{}' (phạm vi: {})",
                    unit_path.display(),
                    scope.label()
                );
                if cfg.is_mise_shim {
                    println!("  ExecStart (mise shim): {}", cfg.exec_path.display());
                } else {
                    println!("  ExecStart: {}", cfg.exec_path.display());
                }
                println!("  WorkingDir: {}", cfg.working_dir.display());
                println!(
                    "  UDP Port: {} | HTTP Port: {}",
                    cfg.udp_port, cfg.http_port
                );
                println!("Tiếp theo:");
                let scope_hint = if scope == ServiceScope::User {
                    ""
                } else {
                    " --system"
                };
                println!(
                    "  rtk-udp-server enable --now{} --name {}",
                    scope_hint, service_name
                );
                0
            }
            Err(e) => {
                eprintln!("Lỗi: {}", e);
                1
            }
        }
    }

    fn cmd_enable(args: EnableArgs) -> i32 {
        let service_name = match normalize_service_name(&args.scope.name) {
            Ok(n) => n,
            Err(e) => {
                eprintln!("Lỗi: {}", e);
                return 2;
            }
        };
        let scope = resolve_scope(&args.scope, &service_name, false);
        let unit_path = match unit_file_path(scope, &service_name) {
            Ok(p) => p,
            Err(e) => {
                eprintln!("Lỗi: {}", e);
                return 1;
            }
        };

        let unit_exists = path_entry_exists(&unit_path);
        if !unit_exists || args.force || args.config.has_any() {
            let cfg = match build_unit_config(&service_name, scope, &args.config) {
                Ok(c) => c,
                Err(e) => {
                    eprintln!("Lỗi: {}", e);
                    return 1;
                }
            };
            match write_unit_file(&cfg) {
                Ok(written) => {
                    if unit_exists {
                        println!("✔ Đã cập nhật systemd unit: {}", written.display());
                    } else {
                        println!("✔ Đã tạo mới systemd unit: {}", written.display());
                    }
                    if cfg.is_mise_shim {
                        println!("  ExecStart (mise shim): {}", cfg.exec_path.display());
                    }
                }
                Err(e) => {
                    eprintln!("Lỗi: {}", e);
                    return 1;
                }
            }
        }

        let unit = unit_arg(&service_name);
        let enable_res = if args.now {
            run_systemctl(scope, &["enable", "--now", &unit])
        } else {
            run_systemctl(scope, &["enable", &unit])
        };

        if let Err(e) = enable_res {
            eprintln!("Lỗi khi bật tự khởi động service: {}", e);
            return 1;
        }

        if args.now {
            println!(
                "✔ Đã bật tự khởi động cùng hệ thống và khởi chạy '{}' (phạm vi: {}).",
                unit,
                scope.label()
            );
        } else {
            println!(
                "✔ Đã bật tự khởi động cùng hệ thống cho '{}' (phạm vi: {}).",
                unit,
                scope.label()
            );
        }

        if scope == ServiceScope::User {
            ensure_linger_for_user(args.no_linger);
        }
        0
    }

    fn cmd_disable(args: DisableArgs) -> i32 {
        let service_name = match normalize_service_name(&args.scope.name) {
            Ok(n) => n,
            Err(e) => {
                eprintln!("Lỗi: {}", e);
                return 2;
            }
        };
        let scope = resolve_scope(&args.scope, &service_name, true);
        if let Err(e) = ensure_unit_exists_or_hint(scope, &service_name) {
            eprintln!("Lỗi: {}", e);
            return 4;
        }

        let unit = unit_arg(&service_name);
        let res = if args.now {
            run_systemctl(scope, &["disable", "--now", &unit])
        } else {
            run_systemctl(scope, &["disable", &unit])
        };

        match res {
            Ok(_) => {
                if args.now {
                    println!(
                        "✔ Đã tắt tự khởi động cùng hệ thống và dừng '{}' (phạm vi: {}).",
                        unit,
                        scope.label()
                    );
                } else {
                    println!(
                        "✔ Đã tắt tự khởi động cùng hệ thống cho '{}' (phạm vi: {}).",
                        unit,
                        scope.label()
                    );
                }
                0
            }
            Err(e) => {
                eprintln!("Lỗi khi tắt service: {}", e);
                1
            }
        }
    }

    fn cmd_lifecycle(scope_args: ScopeArgs, action: &str, action_label: &str) -> i32 {
        let service_name = match normalize_service_name(&scope_args.name) {
            Ok(n) => n,
            Err(e) => {
                eprintln!("Lỗi: {}", e);
                return 2;
            }
        };
        let scope = resolve_scope(&scope_args, &service_name, true);
        if let Err(e) = ensure_unit_exists_or_hint(scope, &service_name) {
            eprintln!("Lỗi: {}", e);
            return 4;
        }

        let unit = unit_arg(&service_name);
        match run_systemctl(scope, &[action, &unit]) {
            Ok(_) => {
                println!(
                    "✔ Đã {} '{}' (phạm vi: {}).",
                    action_label,
                    unit,
                    scope.label()
                );
                0
            }
            Err(e) => {
                eprintln!("Lỗi khi {} '{}': {}", action_label, unit, e);
                1
            }
        }
    }

    fn cmd_uninstall(scope_args: ScopeArgs) -> i32 {
        let service_name = match normalize_service_name(&scope_args.name) {
            Ok(n) => n,
            Err(e) => {
                eprintln!("Lỗi: {}", e);
                return 2;
            }
        };
        let scope = resolve_scope(&scope_args, &service_name, true);
        let unit_path = match unit_file_path(scope, &service_name) {
            Ok(p) => p,
            Err(e) => {
                eprintln!("Lỗi: {}", e);
                return 1;
            }
        };

        if !path_entry_exists(&unit_path) {
            eprintln!(
                "Không tìm thấy tệp unit '{}' ở phạm vi {}.",
                unit_path.display(),
                scope.label()
            );
            return 4;
        }

        let unit = unit_arg(&service_name);
        let _ = run_systemctl(scope, &["stop", &unit]);
        let _ = run_systemctl(scope, &["disable", &unit]);

        if let Err(e) = fs::remove_file(&unit_path) {
            if scope == ServiceScope::System && current_euid() != 0 {
                eprintln!(
                    "Lỗi: Không có quyền xóa '{}': {}. Hãy chạy lại với `sudo`.",
                    unit_path.display(),
                    e
                );
            } else {
                eprintln!("Lỗi khi xóa '{}': {}", unit_path.display(), e);
            }
            return 1;
        }

        if let Err(e) = run_systemctl(scope, &["daemon-reload"]) {
            eprintln!("Cảnh báo: daemon-reload gặp lỗi: {}", e);
        }
        let _ = run_systemctl(scope, &["reset-failed", &unit]);

        println!(
            "✔ Đã gỡ bỏ systemd unit '{}' (dữ liệu uploads/db vẫn được giữ nguyên).",
            unit_path.display()
        );
        0
    }

    fn cmd_status(args: StatusArgs) -> i32 {
        let service_name = match normalize_service_name(&args.scope.name) {
            Ok(n) => n,
            Err(e) => {
                eprintln!("Lỗi: {}", e);
                return 2;
            }
        };
        let scope = resolve_scope(&args.scope, &service_name, true);
        let unit = unit_arg(&service_name);

        if args.raw {
            return run_systemctl_inherit(scope, &["status", "--no-pager", &unit]);
        }

        let props_out = match run_systemctl(
            scope,
            &[
                "show",
                &unit,
                "--no-pager",
                "--property=LoadState,ActiveState,SubState,UnitFileState,MainPID,MemoryCurrent,ExecMainStartTimestamp,FragmentPath,ExecStart",
            ],
        ) {
            Ok(out) => out,
            Err(e) => {
                eprintln!("Lỗi khi kiểm tra trạng thái '{}': {}", unit, e);
                return 1;
            }
        };

        let props = parse_systemctl_show(&props_out);
        let load_state = props
            .get("LoadState")
            .map(String::as_str)
            .unwrap_or("unknown");
        let active_state = props
            .get("ActiveState")
            .map(String::as_str)
            .unwrap_or("unknown");
        let sub_state = props
            .get("SubState")
            .map(String::as_str)
            .unwrap_or("unknown");
        let unit_file_state = props
            .get("UnitFileState")
            .map(String::as_str)
            .unwrap_or("unknown");
        let fragment_path = props.get("FragmentPath").map(String::as_str).unwrap_or("");

        if load_state == "not-found" && fragment_path.is_empty() {
            println!("Service: {} (phạm vi: {})", unit, scope.label());
            println!("Trạng thái: Chưa được cài đặt (not-found)");
            if let Ok(expected) = unit_file_path(scope, &service_name) {
                println!("Đường dẫn unit dự kiến: {}", expected.display());
            }
            let scope_flag = if scope == ServiceScope::User {
                "--user"
            } else {
                "--system"
            };
            println!(
                "Gợi ý: Chạy `rtk-udp-server enable {} --now --name {}` để cài đặt và bật cùng hệ thống.",
                scope_flag, service_name
            );
            return 4;
        }

        let active_icon = match active_state {
            "active" => "●",
            "failed" => "✘",
            _ => "○",
        };
        let boot_label = match unit_file_state {
            "enabled" => "enabled (tự khởi động cùng hệ thống)",
            "disabled" => "disabled (không tự khởi động)",
            other => other,
        };

        println!("{} {} (phạm vi: {})", active_icon, unit, scope.label());
        println!("  Trạng thái    : {} ({})", active_state, sub_state);
        println!("  Khởi động     : {}", boot_label);
        if !fragment_path.is_empty() {
            println!("  Unit file     : {}", fragment_path);
        }
        if scope == ServiceScope::User {
            if let Some(username) = current_username() {
                if let Some(linger) = check_linger_enabled(&username) {
                    println!(
                        "  User linger   : {} ({})",
                        if linger { "yes" } else { "no" },
                        username
                    );
                }
            }
        }
        if let Some(pid) = props.get("MainPID") {
            if pid != "0" && !pid.is_empty() {
                println!("  Main PID      : {}", pid);
            }
        }
        if let Some(mem) = format_memory(props.get("MemoryCurrent")) {
            println!("  Bộ nhớ        : {}", mem);
        }
        if let Some(started) = props.get("ExecMainStartTimestamp") {
            if !started.is_empty() {
                println!("  Bắt đầu lúc   : {}", started);
            }
        }
        if let Some(exec_start) = props.get("ExecStart") {
            if !exec_start.is_empty() {
                println!("  ExecStart     : {}", exec_start);
            }
        }

        if args.verbose || active_state == "failed" {
            println!();
            let _ = run_systemctl_inherit(scope, &["status", "--no-pager", &unit]);
        }

        match active_state {
            "active" => 0,
            "failed" => 1,
            _ => 3,
        }
    }

    fn cmd_logs(args: LogsArgs) -> i32 {
        let service_name = match normalize_service_name(&args.scope.name) {
            Ok(n) => n,
            Err(e) => {
                eprintln!("Lỗi: {}", e);
                return 2;
            }
        };
        let scope = resolve_scope(&args.scope, &service_name, true);
        let unit = unit_arg(&service_name);

        let mut cmd = Command::new("journalctl");
        if scope == ServiceScope::User {
            cmd.arg("--user");
        }
        cmd.args(["-u", &unit, "-n", &args.lines.to_string(), "--no-pager"]);
        if args.follow {
            cmd.arg("-f");
        }
        cmd.stdin(Stdio::inherit())
            .stdout(Stdio::inherit())
            .stderr(Stdio::inherit());

        match cmd.status() {
            Ok(s) => s.code().unwrap_or(1),
            Err(e) => {
                eprintln!("Không thể thực thi `journalctl`: {}", e);
                1
            }
        }
    }

    pub fn dispatch(cmd: ServiceCommand) -> i32 {
        match cmd {
            ServiceCommand::Start(scope) => cmd_lifecycle(scope, "start", "khởi động"),
            ServiceCommand::Stop(scope) => cmd_lifecycle(scope, "stop", "dừng"),
            ServiceCommand::Restart(scope) => cmd_lifecycle(scope, "restart", "khởi động lại"),
            ServiceCommand::Status(args) => cmd_status(args),
            ServiceCommand::Enable(args) => cmd_enable(args),
            ServiceCommand::Disable(args) => cmd_disable(args),
            ServiceCommand::Install(args) => cmd_install(args),
            ServiceCommand::Uninstall(scope) => cmd_uninstall(scope),
            ServiceCommand::Logs(args) => cmd_logs(args),
        }
    }
}

pub fn handle_command(cmd: ServiceCommand) -> i32 {
    #[cfg(target_os = "linux")]
    {
        linux::dispatch(cmd)
    }
    #[cfg(not(target_os = "linux"))]
    {
        let _ = cmd;
        eprintln!("Quản lý dịch vụ systemd/systemctl chỉ được hỗ trợ trên hệ điều hành Linux.");
        1
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::Parser;
    use std::time::{SystemTime, UNIX_EPOCH};

    #[derive(Parser, Debug)]
    struct TestCli {
        #[arg(short = 'u', long, env = "UDP_PORT", default_value_t = 5000)]
        udp_port: u16,
        #[arg(short = 't', long, env = "HTTP_PORT", default_value_t = 8080)]
        http_port: u16,
        #[arg(long, env = "DISABLE_REQUEST_LOG", default_value_t = false)]
        disable_request_log: bool,
        #[command(subcommand)]
        command: Option<ServiceCommand>,
    }

    fn make_temp_dir(label: &str) -> PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let dir = env::temp_dir().join(format!(
            "rtk-udp-service-test-{}-{}-{}",
            label,
            std::process::id(),
            nanos
        ));
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn test_normalize_service_name() {
        assert_eq!(
            normalize_service_name("rtk-udp-server").unwrap(),
            "rtk-udp-server"
        );
        assert_eq!(
            normalize_service_name("rtk-udp-server.service").unwrap(),
            "rtk-udp-server"
        );
        assert_eq!(normalize_service_name("my_app@1").unwrap(), "my_app@1");
        assert!(normalize_service_name("").is_err());
        assert!(normalize_service_name("   ").is_err());
        assert!(normalize_service_name("../evil").is_err());
        assert!(normalize_service_name("a/b").is_err());
        assert!(normalize_service_name("bad name").is_err());
    }

    #[cfg(unix)]
    #[test]
    fn test_mise_install_resolves_to_shim_without_canonicalizing() {
        use std::os::unix::fs::symlink;

        let temp = make_temp_dir("mise-shim");
        let fake_home = temp.join("home");
        let mise_bin = fake_home.join(".local/bin/mise");
        let mise_root = fake_home.join(".local/share/mise");
        let shims_dir = mise_root.join("shims");
        let install_bin = mise_root
            .join("installs/github-manhavn-rust-udp-file-transfer-system/0.1.1/rtk-udp-server");

        fs::create_dir_all(mise_bin.parent().unwrap()).unwrap();
        fs::create_dir_all(&shims_dir).unwrap();
        fs::create_dir_all(install_bin.parent().unwrap()).unwrap();
        fs::write(&mise_bin, b"#!/bin/sh\n").unwrap();
        fs::write(&install_bin, b"binary").unwrap();

        let shim_path = shims_dir.join("rtk-udp-server");
        symlink(&mise_bin, &shim_path).unwrap();

        let env_snap = EnvSnapshot {
            home: Some(fake_home.clone()),
            ..Default::default()
        };

        let resolved =
            resolve_executable_inner(None, &install_bin, None, &temp, &env_snap).unwrap();

        assert!(resolved.is_mise_shim);
        assert_eq!(resolved.path, shim_path);
        assert_ne!(resolved.path, install_bin);
        assert_ne!(resolved.path, mise_bin);
        assert_eq!(resolved.mise_data_dir.as_deref(), Some(mise_root.as_path()));
        assert_eq!(
            resolved.mise_owner_home.as_deref(),
            Some(fake_home.as_path())
        );

        let _ = fs::remove_dir_all(&temp);
    }

    #[cfg(unix)]
    #[test]
    fn test_mise_custom_data_dir_resolves_shim() {
        use std::os::unix::fs::symlink;

        let temp = make_temp_dir("mise-custom-dir");
        let mise_bin = temp.join("mise");
        let custom_mise = temp.join("custom_mise_data");
        let shims_dir = custom_mise.join("shims");
        let install_bin = custom_mise.join("installs/mytool/0.2.0/rtk-udp-server");

        fs::create_dir_all(&shims_dir).unwrap();
        fs::create_dir_all(install_bin.parent().unwrap()).unwrap();
        fs::write(&mise_bin, b"#!/bin/sh\n").unwrap();
        fs::write(&install_bin, b"binary").unwrap();

        let shim_path = shims_dir.join("rtk-udp-server");
        symlink(&mise_bin, &shim_path).unwrap();

        let env_snap = EnvSnapshot {
            mise_data_dir: Some(custom_mise.clone()),
            home: Some(temp.join("other_home")),
            ..Default::default()
        };

        let resolved =
            resolve_executable_inner(None, &install_bin, None, &temp, &env_snap).unwrap();
        assert!(resolved.is_mise_shim);
        assert_eq!(resolved.path, shim_path);
        assert_eq!(
            resolved.mise_data_dir.as_deref(),
            Some(custom_mise.as_path())
        );

        let _ = fs::remove_dir_all(&temp);
    }

    #[test]
    fn test_mise_install_without_shim_errors_instead_of_version_path() {
        let temp = make_temp_dir("mise-no-shim");
        let mise_root = temp.join(".local/share/mise");
        let install_bin = mise_root
            .join("installs/github-manhavn-rust-udp-file-transfer-system/0.1.1/rtk-udp-server");
        fs::create_dir_all(install_bin.parent().unwrap()).unwrap();
        fs::create_dir_all(mise_root.join("shims")).unwrap();
        fs::write(&install_bin, b"binary").unwrap();

        let env_snap = EnvSnapshot {
            home: Some(temp.clone()),
            ..Default::default()
        };

        let err = resolve_executable_inner(None, &install_bin, None, &temp, &env_snap).unwrap_err();
        assert!(err.contains("mise reshim"), "unexpected error: {}", err);

        let _ = fs::remove_dir_all(&temp);
    }

    #[test]
    fn test_non_mise_binary_keeps_direct_path() {
        let temp = make_temp_dir("non-mise");
        let direct_bin = temp.join("opt/rtk-udp-server/rtk-udp-server");
        fs::create_dir_all(direct_bin.parent().unwrap()).unwrap();
        fs::write(&direct_bin, b"binary").unwrap();

        let env_snap = EnvSnapshot::default();
        let resolved = resolve_executable_inner(None, &direct_bin, None, &temp, &env_snap).unwrap();
        assert!(!resolved.is_mise_shim);
        assert_eq!(resolved.path, direct_bin);

        let _ = fs::remove_dir_all(&temp);
    }

    #[test]
    fn test_render_unit_file_user_and_system_with_escaping() {
        let user_cfg = UnitConfig {
            service_name: "rtk-udp-server".to_string(),
            scope: ServiceScope::User,
            exec_path: PathBuf::from("/home/dev/.local/share/mise/shims/rtk-udp-server"),
            is_mise_shim: true,
            working_dir: PathBuf::from("/home/dev/.local/share/rtk-udp-server"),
            upload_dir: PathBuf::from("/home/dev/my uploads/100%data"),
            db_path: PathBuf::from("/home/dev/.local/share/rtk-udp-server/db/data.sqlite"),
            udp_port: 5000,
            http_port: 8080,
            cleanup_interval: 5,
            incomplete_timeout: 60,
            completed_timeout: 15,
            disable_request_log: true,
            service_user: None,
            env_vars: vec![(
                "MISE_DATA_DIR".to_string(),
                "/home/dev/.local/share/mise".to_string(),
            )],
        };
        let rendered_user = render_unit_file(&user_cfg);
        assert!(
            rendered_user.contains("ExecStart=/home/dev/.local/share/mise/shims/rtk-udp-server")
        );
        assert!(rendered_user.contains("\"/home/dev/my uploads/100%%data\""));
        assert!(rendered_user.contains("--disable-request-log"));
        assert!(rendered_user.contains("WantedBy=default.target"));
        assert!(rendered_user.contains("Environment=\"MISE_DATA_DIR=/home/dev/.local/share/mise\""));
        assert!(!rendered_user.contains("StandardOutput=syslog"));

        let sys_cfg = UnitConfig {
            scope: ServiceScope::System,
            service_user: Some("rtk".to_string()),
            disable_request_log: false,
            ..user_cfg
        };
        let rendered_sys = render_unit_file(&sys_cfg);
        assert!(rendered_sys.contains("User=rtk"));
        assert!(rendered_sys.contains("WantedBy=multi-user.target"));
        assert!(!rendered_sys.contains("--disable-request-log"));
    }

    #[test]
    fn test_clap_parsing_foreground_and_all_subcommands() {
        let fg = TestCli::try_parse_from(["rtk-udp-server", "-u", "5500", "-t", "8800"]).unwrap();
        assert!(fg.command.is_none());
        assert_eq!(fg.udp_port, 5500);
        assert_eq!(fg.http_port, 8800);

        let help_err = TestCli::try_parse_from(["rtk-udp-server", "-h"]).unwrap_err();
        assert_eq!(help_err.kind(), clap::error::ErrorKind::DisplayHelp);

        for sub in [
            "start",
            "stop",
            "restart",
            "status",
            "enable",
            "disable",
            "install",
            "uninstall",
            "logs",
        ] {
            let parsed = TestCli::try_parse_from(["rtk-udp-server", sub, "--user"]).unwrap();
            assert!(
                parsed.command.is_some(),
                "subcommand '{}' should parse",
                sub
            );
        }

        let en = TestCli::try_parse_from([
            "rtk-udp-server",
            "enable",
            "--system",
            "--now",
            "--force",
            "-u",
            "5001",
            "-t",
            "8081",
            "--disable-request-log",
        ])
        .unwrap();
        match en.command {
            Some(ServiceCommand::Enable(e)) => {
                assert!(e.scope.system);
                assert!(e.now);
                assert!(e.force);
                assert_eq!(e.config.udp_port, Some(5001));
                assert_eq!(e.config.http_port, Some(8081));
                assert_eq!(e.config.disable_request_log, Some(true));
            }
            other => panic!("unexpected parsed command: {:?}", other),
        }

        let dis = TestCli::try_parse_from(["rtk-udp-server", "disable", "--now"]).unwrap();
        match dis.command {
            Some(ServiceCommand::Disable(d)) => assert!(d.now),
            other => panic!("unexpected parsed command: {:?}", other),
        }
    }
}
