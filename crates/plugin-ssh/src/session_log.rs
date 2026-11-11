/*! # P3 会话日志录制（终端输出落盘通道）
 *
 * 前端把带时间戳的行攒够一批后调用 `ssh_log_append`，后端只做「安全文件名 + 追加写」，
 * 不解析内容、不持有文件句柄：多个 tab 同时录制各自写各自的文件，进程重启无状态。
 *
 * 为什么放在后端而不是前端：WebView 没有任意路径写权限；`save_note` 带
 * `#[cfg(feature = "plugin-files")]`，单开 `plugin-ssh` 时会被裁剪守卫判为不可达。
 * 日志与 known_hosts 一起放在 `{app_dir}/ssh/`（用户资产目录，卸载不删）。
 */
use bdevtools_core::response::response::Response;
use lazy_static::lazy_static;
use serde::Serialize;
use std::{
    fs::{self, OpenOptions},
    io::{Read, Seek, SeekFrom, Write},
    path::PathBuf,
    sync::Mutex,
    time::SystemTime,
};

lazy_static! {
    /// 同一进程内串行化追加写，避免两个 tab 写同一文件时字节交错。
    static ref WRITE_LOCK: Mutex<()> = Mutex::new(());
}

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct LogFileInfo {
    pub name: String,
    pub size: u64,
    /// 本地时间 `YYYY-MM-DD HH:mm:ss`
    pub modified: String,
}

pub fn log_dir() -> PathBuf {
    bdevtools_core::app::app_store::APP
        .lock()
        .app_dir
        .join("ssh")
        .join("logs")
}

/// 文件名白名单：只允许 ASCII 字母数字 / `.` `_` `-`，且必须以 `.log` 结尾。
/// 前端传进来的会话名可能含中文或路径分隔符，这里一律拒绝，杜绝路径穿越。
fn safe_path(name: &str) -> Result<PathBuf, String> {
    if name.is_empty()
        || name.len() > 120
        || name.starts_with('.')
        || name.contains("..")
        || !name
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '.' || c == '_' || c == '-')
        || !name.ends_with(".log")
    {
        return Err("日志文件名非法（仅允许字母数字 . _ -，且以 .log 结尾）".to_string());
    }
    Ok(log_dir().join(name))
}

fn now_text(system_time: SystemTime) -> String {
    let secs = system_time
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as i64;
    chrono::DateTime::from_timestamp(secs, 0)
        .map(|dt| dt.with_timezone(&chrono::Local).format("%Y-%m-%d %H:%M:%S").to_string())
        .unwrap_or_default()
}

/// 追加一批文本。前端已按行加好时间戳，这里不再加工。
pub async fn ssh_log_append(name: String, text: String) -> Response<bool> {
    let path = match safe_path(&name) {
        Ok(p) => p,
        Err(msg) => return Response::new(300, false, &msg),
    };
    if text.is_empty() {
        return Response::new(200, true, "无内容");
    }
    let result = (|| -> std::io::Result<()> {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        let _guard = WRITE_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        let mut file = OpenOptions::new().create(true).append(true).open(&path)?;
        file.write_all(text.as_bytes())?;
        file.flush()
    })();
    match result {
        Ok(()) => Response::new(200, true, "已写入"),
        Err(e) => Response::new(300, false, &format!("写入日志失败：{}", e)),
    }
}

pub async fn ssh_log_list() -> Response<Vec<LogFileInfo>> {
    let dir = log_dir();
    let Ok(entries) = fs::read_dir(&dir) else {
        return Response::new(200, Vec::new(), "暂无日志");
    };
    let mut files: Vec<LogFileInfo> = entries
        .flatten()
        .filter_map(|entry| {
            let meta = entry.metadata().ok()?;
            if !meta.is_file() {
                return None;
            }
            let name = entry.file_name().to_string_lossy().to_string();
            // 只暴露本模块能写出的文件，避免把同目录下的其它文件枚举给前端
            if safe_path(&name).is_err() {
                return None;
            }
            Some(LogFileInfo {
                name,
                size: meta.len(),
                modified: meta.modified().map(now_text).unwrap_or_default(),
            })
        })
        .collect();
    files.sort_by(|a, b| b.name.cmp(&a.name));
    Response::new(200, files, "操作成功！")
}

/// 读取日志尾部（默认 256KB），供设置中心的「查看」直接展示，不会把整份大日志塞进 IPC。
pub async fn ssh_log_read(name: String, max_bytes: Option<u64>) -> Response<String> {
    let path = match safe_path(&name) {
        Ok(p) => p,
        Err(msg) => return Response::new(300, String::new(), &msg),
    };
    let limit = max_bytes.unwrap_or(256 * 1024).min(4 * 1024 * 1024) as usize;
    let result = (|| -> std::io::Result<String> {
        let mut file = fs::File::open(&path)?;
        let len = file.metadata()?.len();
        let start = len.saturating_sub(limit as u64);
        file.seek(SeekFrom::Start(start))?;
        let mut buf = Vec::new();
        file.read_to_end(&mut buf)?;
        Ok(String::from_utf8_lossy(&buf).to_string())
    })();
    match result {
        Ok(text) => Response::new(200, text, "操作成功！"),
        Err(e) => Response::new(300, String::new(), &format!("读取日志失败：{}", e)),
    }
}

pub async fn ssh_log_delete(name: String) -> Response<bool> {
    let path = match safe_path(&name) {
        Ok(p) => p,
        Err(msg) => return Response::new(300, false, &msg),
    };
    match fs::remove_file(&path) {
        Ok(()) => Response::new(200, true, "已删除"),
        Err(e) => Response::new(300, false, &format!("删除日志失败：{}", e)),
    }
}

pub async fn ssh_log_dir() -> Response<String> {
    Response::new(200, log_dir().to_string_lossy().to_string(), "操作成功！")
}
