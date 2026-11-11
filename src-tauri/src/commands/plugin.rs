//! 第三方插件安装、排序与调用 命令（原 commands.rs 对应分区的机械拆分）。
// 分片内命令可能被 feature 整体裁掉，故 glob import 允许未使用。
#[allow(unused_imports)]
use super::*;

// ==================== 第三方插件（安装 / 卸载 / 排序 / 调用预留） ====================

/// 列出已安装第三方插件（内置插件由前端提供，不在此列）
#[tauri::command]
pub fn plugin_list() -> Response<Vec<plugin_host::InstalledPlugin>> {
    plugin_host::plugin_list()
}

/// 安装插件：`source` 为插件目录或 .zip 包；同名插件覆盖安装
#[tauri::command]
pub async fn plugin_install(source: String) -> Response<plugin_host::InstalledPlugin> {
    match tokio::task::spawn_blocking(move || plugin_host::plugin_install(source)).await {
        Ok(r) => r,
        Err(e) => Response::new(
            500,
            plugin_host::InstalledPlugin::empty(),
            &format!("安装线程异常: {e}"),
        ),
    }
}

/// 卸载插件（removeData=true 时一并清理插件私有数据）
#[tauri::command]
pub async fn plugin_uninstall(id: String, remove_data: bool) -> Response<bool> {
    match tokio::task::spawn_blocking(move || plugin_host::plugin_uninstall(id, remove_data)).await {
        Ok(r) => r,
        Err(e) => Response::new(500, false, &format!("卸载线程异常: {e}")),
    }
}

/// 启用 / 禁用插件（内置与第三方同一套：禁用集合持久化在 settings.json）
#[tauri::command]
pub fn plugin_set_enabled(id: String, enabled: bool) -> Response<bool> {
    plugin_host::plugin_set_enabled(id, enabled)
}

/// 被禁用的插件 ID 列表（内置 + 第三方统一集合；前端据此决定注册哪些插件）
#[tauri::command]
pub fn plugin_get_disabled() -> Response<Vec<String>> {
    plugin_host::plugin_get_disabled()
}

/// 读取统一排序（内置 + 第三方插件 ID 的全量顺序）
#[tauri::command]
pub fn plugin_get_order() -> Response<Vec<String>> {
    plugin_host::plugin_get_order()
}

/// 保存统一排序（内置 + 第三方插件 ID 的全量顺序）
#[tauri::command]
pub fn plugin_set_order(order: Vec<String>) -> Response<bool> {
    plugin_host::plugin_set_order(order)
}

/// 收紧某插件的页面能力（**只能减，不能加**；传空数组 = 恢复插件自己的声明）
///
/// 生效口径 = 清单 `pageCapabilities` − `denied`，下一次加载插件页时生效
/// （CSP 是按响应算的，已打开的页面要重新打开才拿得到新策略）。
#[tauri::command]
pub fn plugin_set_page_caps(id: String, denied: Vec<String>) -> Response<bool> {
    match plugin_host::set_denied_page_caps(&id, &denied) {
        Ok(()) => Response::new(200, true, "ok"),
        Err(e) => Response::new(500, false, &e),
    }
}

/// 调用插件后端逻辑（进 Extism WASM 沙箱）。
/// Extism 的调用是同步阻塞的，必须放 blocking 线程池，否则卡死 Tauri 的 async 运行时。
/// 插件未声明 wasm / 运行时未启用时返回 501 结构化错误。
#[tauri::command]
pub async fn plugin_call(id: String, func: String, input: String) -> Response<String> {
    match tokio::task::spawn_blocking(move || plugin_host::plugin_call(id, func, input)).await {
        Ok(r) => r,
        Err(e) => Response::new(500, String::new(), &format!("插件调用线程异常: {e}")),
    }
}

/// 第三方插件 WASM 运行时信息（是否启用 / 资源限制 / 支持的权限声明形态 / 调试开关）
///
/// 会读 settings.json（拿出错快照开关），所以走 blocking 线程池而不是主线程。
#[tauri::command]
pub async fn plugin_runtime_info() -> Response<plugin_wasm::RuntimeInfo> {
    match tokio::task::spawn_blocking(plugin_host::plugin_runtime_info).await {
        Ok(r) => r,
        Err(e) => Response::new(
            500,
            plugin_wasm::runtime_info_placeholder(),
            &format!("读取运行时信息线程异常: {e}"),
        ),
    }
}

/// 某插件上一次调用的资源占用（进程内存增量 / fuel / 出错快照）
#[tauri::command]
pub fn plugin_last_stats(id: String) -> Response<Option<plugin_wasm::CallStats>> {
    plugin_host::plugin_last_stats(id)
}

/// 插件设置的当前值（已存的优先，没有则回落到清单 default）
#[tauri::command]
pub fn plugin_get_settings(id: String) -> Response<serde_json::Value> {
    Response::new(200, plugin_host::plugin_settings_of(&id), "ok")
}

/// 写入插件设置值（按清单定义校验类型与范围；只覆盖提交上来的键）
#[tauri::command]
pub fn plugin_set_settings(id: String, values: String) -> Response<bool> {
    match serde_json::from_str::<serde_json::Value>(&values) {
        Ok(v) => match plugin_host::plugin_set_settings(&id, &v) {
            Ok(()) => Response::new(200, true, "ok"),
            Err(e) => Response::new(500, false, &e),
        },
        Err(e) => Response::new(500, false, &format!("设置值不是合法 JSON: {e}")),
    }
}

/// 插件声明的 `fs` 作用域根目录（宿主文件对话框用它把用户选中的路径限制在声明范围内）
///
/// 只读查询，不进 blocking 线程池（解析清单 + 建一次能力上下文，量级很小）。
#[tauri::command]
pub fn plugin_fs_roots(id: String) -> Response<plugin_wasm::FsRoots> {
    match plugin_wasm::fs_roots(&id) {
        Ok(r) => Response::new(200, r, "ok"),
        Err(e) => Response::new(500, plugin_wasm::FsRoots::default(), &e),
    }
}

/// 设置「插件调用出错时导出线性内存快照」的调试开关（写 settings.json，需 blocking）
#[tauri::command]
pub async fn plugin_set_memdump(enabled: bool) -> Response<bool> {
    match tokio::task::spawn_blocking(move || plugin_host::plugin_set_memdump(enabled)).await {
        Ok(r) => r,
        Err(e) => Response::new(500, false, &format!("更新出错快照开关线程异常: {e}")),
    }
}
