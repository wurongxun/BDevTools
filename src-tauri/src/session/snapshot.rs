/*! # 会话快照读写
会话快照以 JSON 文件形式存储在 {app_dir}/sessions/ 目录下。
主键为工作区 ID：{workspace_id}.session.json；
无工作区时回退为窗口 label：{window_label}.session.json。
工作区锁保证同一工作区同时只有一个窗口在读写，无多窗口竞争。
*/
use std::fs;
use std::path::PathBuf;

use crate::entity::window::{SessionSnapshot, SessionSummary};

/// 获取会话存储目录路径
fn get_sessions_dir() -> PathBuf {
    let app_dir = PathBuf::from(&crate::app::app_store::APP.lock().app_dir);
    app_dir.join("sessions")
}

/// 确保会话目录存在
pub fn ensure_sessions_dir() {
    let dir = get_sessions_dir();
    if !dir.exists() {
        let _ = fs::create_dir_all(&dir);
    }
}

/// 获取会话快照文件路径（优先以工作区 ID 为键）
fn get_session_file_by_key(key: &str) -> PathBuf {
    get_sessions_dir().join(format!("{}.session.json", key))
}

/// 解析快照存储键：有工作区用工作区 ID，否则用窗口 label
fn snapshot_key(snapshot: &SessionSnapshot) -> String {
    snapshot.workspace_id.clone().unwrap_or_else(|| snapshot.window_label.clone())
}

/// 保存会话快照
pub fn save_snapshot(snapshot: &SessionSnapshot) -> Result<(), String> {
    ensure_sessions_dir();
    let path = get_session_file_by_key(&snapshot_key(snapshot));
    let content = serde_json::to_string_pretty(snapshot)
        .map_err(|e| format!("序列化会话快照失败: {}", e))?;
    fs::write(&path, content)
        .map_err(|e| format!("写入会话快照失败: {}", e))
}

/// 加载指定窗口的会话快照（兼容旧键：窗口 label）
pub fn load_snapshot(window_label: &str) -> Option<SessionSnapshot> {
    let path = get_session_file_by_key(window_label);
    if !path.exists() {
        return None;
    }
    let content = fs::read_to_string(&path).ok()?;
    serde_json::from_str(&content).ok()
}

/// 按工作区 ID 加载会话快照（多窗口体系下的主入口）
pub fn load_snapshot_by_workspace(workspace_id: &str) -> Option<SessionSnapshot> {
    let path = get_session_file_by_key(workspace_id);
    if !path.exists() {
        return None;
    }
    let content = fs::read_to_string(&path).ok()?;
    serde_json::from_str(&content).ok()
}

/// 按工作区 ID 删除会话快照（删除工作区时调用）
pub fn delete_snapshot_by_workspace(workspace_id: &str) -> bool {
    let path = get_session_file_by_key(workspace_id);
    if path.exists() {
        fs::remove_file(&path).is_ok()
    } else {
        true
    }
}

/// 加载所有会话快照
pub fn load_all_snapshots() -> Vec<SessionSnapshot> {
    let dir = get_sessions_dir();
    if !dir.exists() {
        return Vec::new();
    }

    let mut snapshots = Vec::new();
    if let Ok(entries) = fs::read_dir(&dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().and_then(|e| e.to_str()) == Some("json")
                && path.file_name().and_then(|n| n.to_str()).map_or(false, |n| n.ends_with(".session.json"))
            {
                if let Ok(content) = fs::read_to_string(&path) {
                    if let Ok(snapshot) = serde_json::from_str::<SessionSnapshot>(&content) {
                        snapshots.push(snapshot);
                    }
                }
            }
        }
    }

    // 按时间戳降序排列
    snapshots.sort_by(|a, b| b.timestamp.cmp(&a.timestamp));
    snapshots
}

/// 列出所有会话摘要（不含完整数据）
pub fn list_sessions() -> Vec<SessionSummary> {
    load_all_snapshots()
        .into_iter()
        .map(|s| SessionSummary {
            window_label: s.window_label,
            window_role: s.window_role,
            workspace_id: s.workspace_id,
            open_file_count: s.open_files.len(),
            timestamp: s.timestamp,
        })
        .collect()
}

/// 删除指定窗口的会话快照（兼容旧键：窗口 label）
pub fn delete_snapshot(window_label: &str) -> bool {
    let path = get_session_file_by_key(window_label);
    if path.exists() {
        fs::remove_file(&path).is_ok()
    } else {
        true // 不存在也视为成功
    }
}

/// 清除所有会话快照
pub fn clear_all_snapshots() -> bool {
    let dir = get_sessions_dir();
    if !dir.exists() {
        return true;
    }
    if let Ok(entries) = fs::read_dir(&dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_file() {
                let _ = fs::remove_file(&path);
            }
        }
    }
    true
}
