//! 「文本工具箱」示例第三方插件的 **WASM 后端**（Extism PDK）
//!
//! 这份代码演示第三方插件的完整能力模型：**逻辑跑在 WASM 沙箱里，宿主能力全靠 host
//! function 注入**，每个能力都要求清单里声明对应权限（见包内 `plugin.json`）。
//!
//! 重新构建（需要 `rustup target add wasm32-unknown-unknown`）：
//! ```bash
//! cargo build --release --target wasm32-unknown-unknown
//! cp target/wasm32-unknown-unknown/release/text_toolkit_guest.wasm ../plugin.wasm
//! ```
//!
//! 关键点：
//! - 没有权限的 host 调用会**直接失败**（宿主 host function 返回错误 → 整次调用中止），
//!   所以别把「文件不存在」当错误路径去 catch——用 `host_fs_stat` 判断存在性。
//! - WASM 实例**每次调用都是新的**：全局变量不跨调用保留，持久状态请写 KV / FS。
//! - 大文件走**流式通道**（`host_fs_open` + `host_fs_read_at`/`host_fs_write_at`/`host_fs_append`）：
//!   文件留在宿主侧，这里每次只拿一块。因为单次调用有 15s / 1e9 fuel 的上限，长任务必须**可续跑**
//!   —— 处理到预算就把游标返回给页面，由页面接着调下一次（见 `scan` / `copy`）。

use extism_pdk::*;
use serde::{Deserialize, Serialize};

/// 宿主提供的函数。模块名默认 `extism:host/user`，参数/返回值的「字符串 ↔ 内存指针」
/// 转换由宏负责，插件侧不要手写裸指针。
#[host_fn]
extern "ExtismHost" {
    fn host_log(msg: String);
    fn host_info() -> String;
    fn host_kv_get(key: String) -> String;
    fn host_kv_set(key: String, value: String);
    fn host_kv_remove(key: String);
    fn host_kv_keys() -> String;
    fn host_fs_read(path: String) -> String;
    fn host_fs_write(path: String, content: String);
    fn host_fs_mkdir(path: String);
    fn host_fs_remove(path: String);
    fn host_fs_list(path: String) -> String;
    fn host_fs_stat(path: String) -> String;

    // 流式通道：句柄 + 按偏移读写（字节进出，不走 base64）。同样只需要 fs:read / fs:write。
    fn host_fs_open(path: String, mode: String) -> String;
    fn host_fs_size(handle: i64) -> String;
    fn host_fs_read_at(handle: i64, offset: i64, len: i64) -> Vec<u8>;
    fn host_fs_write_at(handle: i64, offset: i64, data: Vec<u8>);
    fn host_fs_append(handle: i64, data: Vec<u8>);
    fn host_fs_truncate(handle: i64, size: i64);
    fn host_fs_close(handle: i64);
}

/// 页面发来的请求（`plugin_call` 的 input，JSON 字符串）
#[derive(Debug, Default, Deserialize)]
struct Req {
    action: String,
    #[serde(default)]
    text: String,
    #[serde(default)]
    key: String,
    #[serde(default)]
    path: String,
    #[serde(default)]
    url: String,

    // ---- 流式动作用的字段 ----
    /// 分块扫描要找的子串（`scan`）
    #[serde(default)]
    needle: String,
    /// 复制目标（`copy`）
    #[serde(default)]
    dest: String,
    /// 续跑游标：上一次响应里回传的 `cursor`（0 = 从头开始）
    #[serde(default)]
    cursor: u64,
    /// 本次调用最多处理多少字节（0 = 用默认预算）
    #[serde(default)]
    budget: u64,
    /// 就地写入的偏移（`patch`）
    #[serde(default)]
    offset: i64,
    /// 写完后把文件截断到这个长度（`patch`；0 = 不截断）
    #[serde(default, rename = "truncateTo")]
    truncate_to: u64,
}

/// 回给页面的响应（JSON 字符串）
#[derive(Debug, Serialize)]
struct Resp {
    ok: bool,
    message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    data: Option<serde_json::Value>,
}

fn ok(message: impl Into<String>, data: Option<serde_json::Value>) -> String {
    serde_json::to_string(&Resp { ok: true, message: message.into(), data })
        .unwrap_or_else(|_| r#"{"ok":false,"message":"响应序列化失败"}"#.to_string())
}

fn fail(message: impl Into<String>) -> String {
    serde_json::to_string(&Resp { ok: false, message: message.into(), data: None })
        .unwrap_or_else(|_| r#"{"ok":false,"message":"响应序列化失败"}"#.to_string())
}

/// 清单里声明了 `"init": "init"`：宿主在每次新建实例后会先调用它。
/// 注意「每次调用都是新实例」——init 的副作用不该被当成跨调用的一次性初始化。
#[plugin_fn]
pub fn init(_input: String) -> FnResult<String> {
    let _ = unsafe { host_log("text-toolkit: 实例已创建".to_string()) };
    Ok(r#"{"ok":true}"#.to_string())
}

/// 唯一入口：页面所有请求都打到这个函数上（`plugin_call(id, "handle", input)`）
#[plugin_fn]
pub fn handle(input: String) -> FnResult<String> {
    let req: Req = match serde_json::from_str(&input) {
        Ok(r) => r,
        Err(e) => return Ok(fail(format!("入参不是合法 JSON: {}", e))),
    };

    let out = match req.action.as_str() {
        // ---- 纯 WASM 计算：不需要任何宿主权限 ----
        "stats" => stats(&req.text),
        "upper" => Ok(ok(
            "已转为大写",
            Some(serde_json::json!({ "text": req.text.to_uppercase() })),
        )),
        "reverse" => Ok(ok(
            "已反转",
            Some(serde_json::json!({ "text": req.text.chars().rev().collect::<String>() })),
        )),

        // ---- 宿主能力：KV（需要 kv 权限）----
        "kv_save" => kv_save(&req),
        "kv_load" => kv_load(&req),
        "kv_delete" => kv_delete(&req),
        "kv_keys" => kv_keys(),

        // ---- 宿主能力：插件私有数据目录（需要 fs:read / fs:write 权限）----
        "fs_save" => fs_save(&req),
        "fs_load" => fs_load(&req),
        "fs_list" => fs_list(&req),
        "fs_stat" => fs_stat(&req),
        "fs_delete" => fs_delete(&req),

        // ---- 宿主能力：流式通道（大文件；仍然只需要 fs:read / fs:write）----
        "scan" => scan(&req),
        "copy" => copy(&req),
        "patch" => patch(&req),

        // ---- 宿主能力：HTTP（需要 net:<host> 权限，白名单外的域名会被 Extism 拒绝）----
        "fetch" => fetch(&req),

        // ---- 自查：看看宿主到底给了哪些作用域（不需要权限）----
        "probe" => probe(),
        // ---- 自报：本插件的线性内存用量（不依赖宿主采样，精度到字节）----
        "mem" => mem(),

        other => Ok(fail(format!("未知动作: {}", other))),
    };

    match out {
        Ok(body) => Ok(body),
        Err(e) => Ok(fail(e)),
    }
}

// ==================== 纯计算 ====================

fn stats(text: &str) -> Result<String, String> {
    let lines: Vec<&str> = text.lines().collect();
    let non_empty = lines.iter().filter(|l| !l.trim().is_empty()).count();
    let longest = lines.iter().map(|l| l.chars().count()).max().unwrap_or(0);
    Ok(ok(
        "统计完成（在 WASM 内计算）",
        Some(serde_json::json!({
            "chars": text.chars().count(),
            "bytes": text.len(),
            "lines": lines.len(),
            "nonEmptyLines": non_empty,
            "longestLine": longest,
        })),
    ))
}

// ==================== KV ====================

fn kv_save(req: &Req) -> Result<String, String> {
    if req.key.trim().is_empty() {
        return Ok(fail("key 不能为空"));
    }
    unsafe { host_kv_set(req.key.clone(), req.text.clone()) }
        .map_err(|e| format!("写入 KV 失败（是否声明了 kv 权限？）: {}", e))?;
    Ok(ok(format!("已保存到 KV：{}", req.key), None))
}

fn kv_load(req: &Req) -> Result<String, String> {
    let value = unsafe { host_kv_get(req.key.clone()) }
        .map_err(|e| format!("读取 KV 失败（是否声明了 kv 权限？）: {}", e))?;
    let empty = value.is_empty();
    Ok(ok(
        if empty { "键不存在（或值为空）" } else { "读取成功" },
        Some(serde_json::json!({ "key": req.key, "value": value })),
    ))
}

fn kv_delete(req: &Req) -> Result<String, String> {
    unsafe { host_kv_remove(req.key.clone()) }.map_err(|e| format!("删除 KV 失败: {}", e))?;
    Ok(ok(format!("已删除：{}", req.key), None))
}

fn kv_keys() -> Result<String, String> {
    let raw = unsafe { host_kv_keys() }.map_err(|e| format!("枚举 KV 失败: {}", e))?;
    let keys: serde_json::Value = serde_json::from_str(&raw).unwrap_or(serde_json::json!([]));
    Ok(ok("KV 键列表", Some(keys)))
}

// ==================== 插件私有数据目录 ====================

fn fs_save(req: &Req) -> Result<String, String> {
    if req.path.trim().is_empty() {
        return Ok(fail("path 不能为空（相对路径即插件数据目录下的文件）"));
    }
    unsafe { host_fs_write(req.path.clone(), req.text.clone()) }
        .map_err(|e| format!("写文件失败（是否声明了 fs:write 权限？）: {}", e))?;
    Ok(ok(format!("已写入 {}", req.path), None))
}

fn fs_load(req: &Req) -> Result<String, String> {
    let stat = unsafe { host_fs_stat(req.path.clone()) }
        .map_err(|e| format!("查询文件失败（是否声明了 fs 权限？）: {}", e))?;
    let exists = serde_json::from_str::<serde_json::Value>(&stat)
        .ok()
        .and_then(|v| v.get("exists").and_then(|x| x.as_bool()))
        .unwrap_or(false);
    if !exists {
        return Ok(fail(format!("文件不存在：{}", req.path)));
    }
    let content =
        unsafe { host_fs_read(req.path.clone()) }.map_err(|e| format!("读文件失败: {}", e))?;
    Ok(ok(
        format!("已读取 {}", req.path),
        Some(serde_json::json!({ "content": content })),
    ))
}

fn fs_list(req: &Req) -> Result<String, String> {
    let dir = if req.path.trim().is_empty() {
        ".".to_string()
    } else {
        req.path.clone()
    };
    let raw = unsafe { host_fs_list(dir.clone()) }
        .map_err(|e| format!("列目录失败（是否声明了 fs:read 权限？）: {}", e))?;
    let entries: serde_json::Value = serde_json::from_str(&raw).unwrap_or(serde_json::json!([]));
    Ok(ok(format!("目录 {} 的内容", dir), Some(entries)))
}

fn fs_stat(req: &Req) -> Result<String, String> {
    let raw = unsafe { host_fs_stat(req.path.clone()) }.map_err(|e| format!("查询失败: {}", e))?;
    let info: serde_json::Value = serde_json::from_str(&raw).unwrap_or(serde_json::json!({}));
    Ok(ok(format!("{} 的元信息", req.path), Some(info)))
}

fn fs_delete(req: &Req) -> Result<String, String> {
    unsafe { host_fs_remove(req.path.clone()) }.map_err(|e| format!("删除失败: {}", e))?;
    Ok(ok(format!("已删除 {}", req.path), None))
}

// ==================== 流式通道（大文件） ====================

/// 单块大小：必须 ≤ 宿主 `MAX_FS_CHUNK`（1MB），否则宿主直接拒绝
const CHUNK: u64 = 1024 * 1024;
/// 一次调用最多处理多少字节：受 15s / 1e9 fuel 约束，**故意小于宿主的调用预算**，
/// 处理不完就靠游标续跑（一次调用硬扫完一个大文件必然撞超时，进度还会全丢）
const CALL_BUDGET: u64 = 2 * 1024 * 1024;

/// 打开一个流式句柄并取到编号（模式：r / w / a / rw）
fn open_handle(path: &str, mode: &str) -> Result<i64, String> {
    let raw = unsafe { host_fs_open(path.to_string(), mode.to_string()) }
        .map_err(|e| format!("打开文件失败（是否声明了 fs 权限？）: {}", e))?;
    serde_json::from_str::<serde_json::Value>(&raw)
        .ok()
        .and_then(|v| v.get("handle").and_then(|h| h.as_i64()))
        .ok_or_else(|| format!("宿主没有返回句柄: {}", raw))
}

/// 问宿主要文件当前长度（每次现查，不是打开时的快照）
fn handle_size(handle: i64) -> Result<u64, String> {
    let raw = unsafe { host_fs_size(handle) }.map_err(|e| format!("取文件长度失败: {}", e))?;
    raw.trim()
        .parse::<u64>()
        .map_err(|_| format!("文件长度不是数字: {}", raw))
}

fn close_handle(handle: i64) {
    // 幂等，所以「出错也要关」可以放心这么写
    let _ = unsafe { host_fs_close(handle) };
}

/// 统计不重叠出现的次数（找子串是字节级比较，不做 Unicode 归一化）
fn count_occurrences(hay: &[u8], needle: &[u8]) -> usize {
    let mut count = 0usize;
    let mut i = 0usize;
    while i + needle.len() <= hay.len() {
        if &hay[i..i + needle.len()] == needle {
            count += 1;
            i += needle.len();
        } else {
            i += 1;
        }
    }
    count
}

/// 分块扫描一个文件，数 `needle` 出现多少次。**每次调用只处理一段**，靠游标续跑。
///
/// 关键点：下一轮的游标会**回退 `needle.len() - 1` 字节**，这样「正好跨块边界」的匹配
/// 不会漏；又因为一个完整匹配塞不进这 `len-1` 字节，所以也不会重复计数。
fn scan(req: &Req) -> Result<String, String> {
    if req.path.trim().is_empty() {
        return Ok(fail("path 不能为空"));
    }
    let needle = if req.needle.is_empty() { "NEEDLE".to_string() } else { req.needle.clone() };
    let needle = needle.as_bytes().to_vec();
    if needle.is_empty() {
        return Ok(fail("needle 不能为空"));
    }

    let handle = match open_handle(&req.path, "r") {
        Ok(h) => h,
        Err(e) => return Ok(fail(e)),
    };
    let out = (|| -> Result<String, String> {
        let size = handle_size(handle)?;
        let budget = if req.budget == 0 { CALL_BUDGET } else { req.budget };
        let overlap = (needle.len() - 1) as u64;
        let mut cursor = req.cursor.min(size);
        let mut matched = 0u64;
        let mut processed = 0u64;

        while cursor < size && processed < budget {
            let want = CHUNK.min(size - cursor);
            // 返回的是原始字节；到末尾会短读，空块也正常（不是错误）
            let buf = unsafe { host_fs_read_at(handle, cursor as i64, want as i64) }
                .map_err(|e| format!("读分块失败: {}", e))?;
            if buf.is_empty() {
                break;
            }
            matched += count_occurrences(&buf, &needle) as u64;
            processed += buf.len() as u64;
            let end = cursor + buf.len() as u64;
            if end >= size {
                cursor = size;
                break;
            }
            cursor = end.saturating_sub(overlap).max(cursor + 1);
        }

        Ok(ok(
            format!("本轮扫描 {} 字节", processed),
            Some(serde_json::json!({
                "done": cursor >= size,
                "cursor": cursor,
                "matched": matched,
                "processed": processed,
                "size": size,
            })),
        ))
    })();
    close_handle(handle);
    out
}

/// 分块复制一个文件（源 → 目标）。第 0 轮用 `w`（截断重建）+ 按偏移写，
/// 之后的轮次用 `a`（追加）+ `host_fs_append`：顺序复制下两者等价，但各自都是「绝不复写
/// 已写好的部分」，所以中途失败也不会把副本弄成半新半旧。
fn copy(req: &Req) -> Result<String, String> {
    if req.path.trim().is_empty() || req.dest.trim().is_empty() {
        return Ok(fail("path / dest 不能为空"));
    }
    let src = match open_handle(&req.path, "r") {
        Ok(h) => h,
        Err(e) => return Ok(fail(e)),
    };
    let (mode, use_append) = if req.cursor == 0 { ("w", false) } else { ("a", true) };
    let dst = match open_handle(&req.dest, mode) {
        Ok(h) => h,
        Err(e) => {
            close_handle(src);
            return Ok(fail(e));
        }
    };

    let out = (|| -> Result<String, String> {
        let size = handle_size(src)?;
        let budget = if req.budget == 0 { CALL_BUDGET } else { req.budget };
        let mut cursor = req.cursor.min(size);
        let mut processed = 0u64;

        while cursor < size && processed < budget {
            let want = CHUNK.min(size - cursor);
            let buf = unsafe { host_fs_read_at(src, cursor as i64, want as i64) }
                .map_err(|e| format!("读分块失败: {}", e))?;
            if buf.is_empty() {
                break;
            }
            let n = buf.len() as u64;
            if use_append {
                unsafe { host_fs_append(dst, buf) }.map_err(|e| format!("追加分块失败: {}", e))?;
            } else {
                unsafe { host_fs_write_at(dst, cursor as i64, buf) }
                    .map_err(|e| format!("写分块失败: {}", e))?;
            }
            cursor += n;
            processed += n;
        }

        Ok(ok(
            format!("本轮复制 {} 字节", processed),
            Some(serde_json::json!({
                "done": cursor >= size,
                "cursor": cursor,
                "processed": processed,
                "size": size,
            })),
        ))
    })();

    close_handle(src);
    close_handle(dst);
    out
}

/// 就地修改：`rw` 模式打开 → 在 `offset` 处覆盖写 `text` → 需要时截断到 `truncateTo`。
fn patch(req: &Req) -> Result<String, String> {
    if req.path.trim().is_empty() {
        return Ok(fail("path 不能为空"));
    }
    let handle = match open_handle(&req.path, "rw") {
        Ok(h) => h,
        Err(e) => return Ok(fail(e)),
    };
    let out = (|| -> Result<String, String> {
        if !req.text.is_empty() {
            let data = req.text.as_bytes().to_vec();
            let written = data.len();
            unsafe { host_fs_write_at(handle, req.offset, data) }
                .map_err(|e| format!("按偏移写入失败: {}", e))?;
            if req.truncate_to > 0 {
                unsafe { host_fs_truncate(handle, req.truncate_to as i64) }
                    .map_err(|e| format!("截断失败: {}", e))?;
            }
            let size = handle_size(handle)?;
            return Ok(ok(
                format!("已在偏移 {} 写入 {} 字节", req.offset, written),
                Some(serde_json::json!({ "size": size, "path": req.path })),
            ));
        }
        let size = handle_size(handle)?;
        Ok(ok(format!("{} 当前 {} 字节", req.path, size), Some(serde_json::json!({ "size": size }))))
    })();
    close_handle(handle);
    out
}

// ==================== HTTP ====================

fn fetch(req: &Req) -> Result<String, String> {
    let url = if req.url.trim().is_empty() {
        "https://api.github.com/zen".to_string()
    } else {
        req.url.clone()
    };
    // 只有 plugin.json 里 net:<host> 声明过的域名才放行（支持 *.example.com 通配）
    let resp = extism_pdk::http::request(&HttpRequest::new(url.clone()), None::<String>)
        .map_err(|e| format!("HTTP 请求失败（域名是否在 net: 白名单内？）: {}", e))?;
    let body = String::from_utf8_lossy(&resp.body()).to_string();
    Ok(ok(
        format!("HTTP {} · {}", resp.status_code(), url),
        Some(serde_json::json!({ "status": resp.status_code(), "body": body })),
    ))
}

// ==================== 线性内存自报 ====================

/// 当前已提交的线性内存字节数（页数 × 64KiB）。
///
/// 为什么插件要自报：宿主侧也能给一个内存口径，但那只能用**宿主进程**的提交量增量近似
/// ——里面混着 wasmtime 实例元数据、其他插件、宿主自身的波动。这里的数字是插件线性内存
/// 本身，配合 `host_info` 给的 `memoryLimitMb` 算「占用比」最准。
///
/// `core::arch::wasm32::memory_size(0)` 是 wasm32 的稳定 intrinsic，返回**页数**（1 页 = 64KiB）。
/// 非 wasm32 目标（例如在宿主机三元组上 `cargo check`）没有这个 intrinsic，给 0 兜底，
/// 所以这个文件在宿主目标上也能编译。
#[cfg(target_arch = "wasm32")]
fn linear_memory_bytes() -> u64 {
    (core::arch::wasm32::memory_size(0) as u64) * 64 * 1024
}

#[cfg(not(target_arch = "wasm32"))]
fn linear_memory_bytes() -> u64 {
    0
}

/// 自报线性内存占用 + 宿主给的上限（页 / MB / 百分比）。
///
/// 注意「每页 64KiB」的口径：wasm 内存是**按页提交**的，所以这个数字永远是 64KiB 的倍数，
/// 比「实际用到的字节数」略大——这才是真正占到宿主内存的量，也正是该看的量。
fn mem() -> Result<String, String> {
    let used = linear_memory_bytes();
    // host_info 拿不到（极少数情况）不该让自报失败：上限按未知处理，用量照样给
    let info = unsafe { host_info() }
        .ok()
        .and_then(|raw| serde_json::from_str::<serde_json::Value>(&raw).ok())
        .unwrap_or_else(|| serde_json::json!({}));
    let limit_bytes = info
        .get("memoryLimitPages")
        .and_then(|v| v.as_u64())
        .map(|pages| pages * 64 * 1024);
    let percent = match limit_bytes {
        Some(l) if l > 0 => Some((used as f64) * 1000.0 / (l as f64) / 10.0),
        _ => None,
    };
    Ok(ok(
        format!("线性内存 {:.2} MB", used as f64 / 1024.0 / 1024.0),
        Some(serde_json::json!({
            "usedBytes": used,
            "usedPages": used / (64 * 1024),
            "limitBytes": limit_bytes,
            "limitMb": info.get("memoryLimitMb").cloned(),
            "percent": percent,
        })),
    ))
}

// ==================== 自查 ====================

fn probe() -> Result<String, String> {
    let raw = unsafe { host_info() }.map_err(|e| format!("读取宿主信息失败: {}", e))?;
    let info: serde_json::Value = serde_json::from_str(&raw).unwrap_or(serde_json::json!({}));
    Ok(ok("宿主授予的能力", Some(info)))
}
