//! 本地交互式终端 命令（原 commands.rs 对应分区的机械拆分）。
// 分片内命令可能被 feature 整体裁掉，故 glob import 允许未使用。
#[allow(unused_imports)]
use super::*;

// ==================== 本地交互式终端（terminal 插件） ====================
//
// 输出流经 Tauri v2 `Channel` 推送（IPC 层自带消息序号，前端按序投递，不会乱序/丢块）。
// 这里把 `Channel` 适配成业务模块的中性 `TermSink` 闭包，业务模块不感知任何 tauri 类型。

/// 打开一个本地 PTY 会话（跨平台 shell），输出经 `onEvent` 持续推送
#[cfg(feature = "plugin-terminal")]
#[tauri::command]
pub async fn terminal_open(
    options: terminal::TermOpenOptions,
    on_event: tauri::ipc::Channel<terminal::TermEvent>,
) -> Response<terminal::TermOpenResult> {
    let sink: terminal::TermSink = std::sync::Arc::new(move |ev| {
        if let Err(e) = on_event.send(ev) {
            log::debug!("[terminal] 推送事件失败（会话可能已关闭）: {e}");
        }
    });
    match tokio::task::spawn_blocking(move || terminal::terminal_open(options, sink)).await {
        Ok(r) => r,
        Err(e) => Response::new(500, terminal::empty_open_result_default(), &format!("打开终端线程异常: {e}")),
    }
}

/// 向终端写入按键 / 粘贴内容
///
/// 注意 `async` 不是装饰：**Tauri 的同步命令跑在主线程**（就是 webview 的事件循环线程）上，
/// 而「写 PTY」在 shell 不读 stdin 时（正在跑长命令 / 分页器）会阻塞住，
/// 一旦阻塞主线程整个界面就无响应了（粘贴大段文本时最容易撞上）。
/// 凡是有可能阻塞的命令一律 `async` + `spawn_blocking`（与 mat_* 一致）。
#[cfg(feature = "plugin-terminal")]
#[tauri::command]
pub async fn terminal_write(session_id: String, data: String) -> Response<bool> {
    match tokio::task::spawn_blocking(move || terminal::terminal_write(session_id, data)).await {
        Ok(r) => r,
        Err(e) => Response::new(500, false, &format!("写入终端线程异常: {e}")),
    }
}

/// 调整终端尺寸（内核据此向 shell 发 SIGWINCH / ConPTY 同步窗口大小）
#[cfg(feature = "plugin-terminal")]
#[tauri::command]
pub async fn terminal_resize(session_id: String, cols: u16, rows: u16) -> Response<bool> {
    match tokio::task::spawn_blocking(move || terminal::terminal_resize(session_id, cols, rows)).await {
        Ok(r) => r,
        Err(e) => Response::new(500, false, &format!("调整终端尺寸线程异常: {e}")),
    }
}

/// 关闭会话（kill shell + 释放会话表 + 清理集成脚本临时目录）
///
/// 释放 PTY 时 Windows 上会走 `ClosePseudoConsole`，它会等 conhost 退出；
/// 该调用可能阻塞（见 `terminal::terminal_close` 的说明），绝不能落在主线程上。
#[cfg(feature = "plugin-terminal")]
#[tauri::command]
pub async fn terminal_close(session_id: String) -> Response<bool> {
    match tokio::task::spawn_blocking(move || terminal::terminal_close(session_id)).await {
        Ok(r) => r,
        Err(e) => Response::new(500, false, &format!("关闭终端线程异常: {e}")),
    }
}

/// 会话是否存活（前端自检 / 崩溃恢复用）
#[cfg(feature = "plugin-terminal")]
#[tauri::command]
pub async fn terminal_alive(session_id: String) -> Response<bool> {
    match tokio::task::spawn_blocking(move || terminal::terminal_alive(session_id)).await {
        Ok(r) => r,
        Err(e) => Response::new(500, false, &format!("查询终端状态线程异常: {e}")),
    }
}

/// 本机可用 shell 列表（「新建会话」下拉）
#[cfg(feature = "plugin-terminal")]
#[tauri::command]
pub async fn terminal_list_shells() -> Response<Vec<terminal::ShellInfo>> {
    match tokio::task::spawn_blocking(terminal::terminal_list_shells).await {
        Ok(r) => r,
        Err(e) => Response::new(500, Vec::new(), &format!("探测 shell 线程异常: {e}")),
    }
}

/// 终端默认启动信息（默认 shell / 默认 cwd / 平台）
#[cfg(feature = "plugin-terminal")]
#[tauri::command]
pub async fn terminal_defaults() -> Response<terminal::TermDefaults> {
    match tokio::task::spawn_blocking(terminal::terminal_defaults).await {
        Ok(r) => r,
        Err(e) => Response::new(500, terminal::empty_defaults(), &format!("读取终端默认值线程异常: {e}")),
    }
}
