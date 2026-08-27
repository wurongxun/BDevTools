/*! # 工作区锁管理
防止多个窗口同时打开同一个工作区。
使用内存中的 HashMap 维护 workspace_id → window_label 的映射。
窗口关闭时自动释放该窗口持有的所有锁。
*/
use lazy_static::lazy_static;
use parking_lot::Mutex;
use std::collections::HashMap;

use crate::response::response::Response;

lazy_static! {
    /// 全局工作区锁表：workspace_id → window_label
    pub static ref WORKSPACE_LOCKS: Mutex<HashMap<String, String>> =
        Mutex::new(HashMap::new());
}

/// 尝试锁定工作区
/// - 返回 true：锁定成功（或已是该窗口持有）
/// - 返回 false：已被其他窗口锁定
pub fn try_lock(workspace_id: &str, window_label: &str) -> Result<bool, String> {
    let mut locks = WORKSPACE_LOCKS.lock();
    match locks.get(workspace_id) {
        Some(holder) if holder == window_label => {
            // 同一窗口重复锁定，允许
            Ok(true)
        }
        Some(holder) => {
            // 已被其他窗口锁定
            println!(
                "[workspace_lock] 工作区 {} 已被窗口 {} 锁定，拒绝窗口 {} 的锁定请求",
                workspace_id, holder, window_label
            );
            Ok(false)
        }
        None => {
            locks.insert(workspace_id.to_string(), window_label.to_string());
            println!(
                "[workspace_lock] 窗口 {} 已锁定工作区 {}",
                window_label, workspace_id
            );
            Ok(true)
        }
    }
}

/// 释放工作区锁（仅持有者可释放）
pub fn unlock(workspace_id: &str, window_label: &str) -> bool {
    let mut locks = WORKSPACE_LOCKS.lock();
    let removed = match locks.get(workspace_id) {
        Some(holder) if holder == window_label => {
            locks.remove(workspace_id);
            println!(
                "[workspace_lock] 窗口 {} 已释放工作区 {}",
                window_label, workspace_id
            );
            true
        }
        Some(holder) => {
            println!(
                "[workspace_lock] 窗口 {} 无法释放工作区 {}（当前由 {} 持有）",
                window_label, workspace_id, holder
            );
            false
        }
        None => false,
    };
    removed
}

/// 释放某个窗口持有的所有锁（窗口关闭时调用）
pub fn unlock_all(window_label: &str) {
    let mut locks = WORKSPACE_LOCKS.lock();
    let before = locks.len();
    locks.retain(|_, v| v != window_label);
    let after = locks.len();
    if before != after {
        println!(
            "[workspace_lock] 窗口 {} 关闭，释放了 {} 个工作区锁",
            window_label,
            before - after
        );
    }
}

/// 查询工作区是否被锁定，返回持有者 window_label
pub fn get_lock_holder(workspace_id: &str) -> Option<String> {
    let locks = WORKSPACE_LOCKS.lock();
    locks.get(workspace_id).cloned()
}

// ─── Tauri 命令 ───

/// 尝试锁定工作区
pub async fn workspace_lock(workspace_id: String, window_label: String) -> Response<bool> {
    match try_lock(&workspace_id, &window_label) {
        Ok(true) => Response::new(200, true, "锁定成功"),
        Ok(false) => {
            let holder = get_lock_holder(&workspace_id).unwrap_or_default();
            Response::new(
                300,
                false,
                &format!("该工作区已被窗口 [{}] 占用", holder),
            )
        }
        Err(e) => Response::new(500, false, &format!("锁定失败: {}", e)),
    }
}

/// 释放工作区锁
pub async fn workspace_unlock(workspace_id: String, window_label: String) -> Response<bool> {
    let ok = unlock(&workspace_id, &window_label);
    if ok {
        Response::new(200, true, "解锁成功")
    } else {
        Response::new(300, false, "解锁失败（非持有者或工作区未锁定）")
    }
}

/// 查询工作区锁定状态
pub async fn workspace_lock_status(workspace_id: String) -> Response<Option<String>> {
    let holder = get_lock_holder(&workspace_id);
    Response::new(200, holder.clone(), if holder.is_some() { "工作区已锁定" } else { "工作区未锁定" })
}
