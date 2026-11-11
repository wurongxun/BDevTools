/*! # 第三方插件 WASM 运行时（Extism）

第三方插件的「逻辑层」：插件作者把代码编译成 `plugin.wasm`，由本模块在 Extism（wasmtime）
沙箱里执行；插件页面（沙箱 iframe）只能经 `plugin_call` 间接用到它。

三条边界里的第三条就在这儿：**WASM ↔ 宿主能力**。插件拿不到任何原始句柄——所有能力都由
host function 注入，每个函数**先校验权限、再干活**。

## 能力模型（清单 `permissions` 声明 → host 函数）

| 声明 | 宿主函数 | 语义 |
|---|---|---|
| `log` | `host_log` | 写宿主日志，带插件 id 前缀 |
| `kv` | `host_kv_get` / `host_kv_set` / `host_kv_remove` / `host_kv_keys` | 插件私有 KV，落 `{config_dir}/plugins_ext_kv/<id>.json`，跨会话保留 |
| `fs:read` | `host_fs_read` / `host_fs_list` / `host_fs_stat` | 读插件私有数据目录 `{app_dir}/plugin_data_ext/<id>/` |
| `fs:read:<路径>` | 同上 | 额外可读该路径前缀（`$HOME` / `~/` 会展开；必须是绝对路径） |
| `fs:write[:<路径>]` | `host_fs_write` / `host_fs_mkdir` / `host_fs_remove` | 同上，写 |
| `fs:read:*` / `fs:write:*` | 同上 | **任意路径**（与 `net:*` 同构的字面通配），但其它插件的数据目录与插件安装目录仍是禁区 —— 见下节 |
| `net:<host>[:<port>]` | Extism 内建 `http_request`；`host_tcp_connect` / `host_tcp_send` / `host_tcp_recv` / `host_tcp_close` | **网络访问授权（协议无关）**：一条声明同时放行两个出口 —— HTTP（按 host glob 匹配）与原始 socket（按 `host:port` 匹配）。宿主只判定「这个目标能不能连」，**不管上面跑什么协议**；字节收发与协议实现（RESP / TLS / 自定义）全在插件侧。`host` / `port` 支持 `*` 通配 |
| — | `host_info` | 不需要权限：返回插件 id / 数据目录 / 已授权作用域，便于自查 |
| 同 `fs:*`（**无新增权限名**） | `host_fs_open` / `host_fs_size` / `host_fs_read_at` / `host_fs_write_at` / `host_fs_append` / `host_fs_truncate` / `host_fs_close` | **流式通道**：按偏移读写，给超过一次性上限的大文件用（见下节） |

**默认无权限**：没声明就走不通对应 host 函数（返回结构化错误）。路径参数由宿主解析成绝对
路径后与已授权作用域做**组件级前缀比较**（`canonicalize` 之后），`../`、软链逃逸一律拒绝。

## `fs:*:` 的边界：任意路径，但不是别的插件的地盘

`fs:read:*` / `fs:write:*` 是字面意义上的**任意路径**（能读 `%APPDATA%`、SSH 私钥 —— 要防恶意
插件唯一的手段是**不安装它**）。唯一收窄的是**跨插件隔离**，以下三处即使声明了 `*` 也一律拒绝：

1. 其它第三方插件的数据目录 `{app_dir}/plugin_data_ext/<其它 id>/`
2. 内置插件数据根 `{app_dir}/plugin_data/`
3. 插件安装根 `{app_dir}/plugins_ext/`（否则 `fs:write:*` 能改写**别人的** `plugin.wasm`，
   把一次越权变成持久感染）

自己的数据目录不受影响。这不是「多一道安全边界」（那要靠不安装），而是**一个插件的越权不该
连带污染另一个插件的持久状态** —— 故障半径要按插件切分，而不是按声明切分。

## 信任模型：安装即信任

**装进来的插件被视为可信代码。** `permissions` 是**诚实性校验 + 用户知情**，不是安全沙箱边界：

1. **用户知情** —— 用户在 设置 → 插件 里能看到每个插件声明了什么。
2. **诚实性校验** —— 插件因 bug 或配置漂移碰到声明之外的目标 / 路径时**立刻报错**，而不是悄悄干完。
3. **事故半径** —— 即使行为异常，能碰到的大致不超出声明过的范围。

两条推论，别搞反：

- **不拦「合法但危险」的目标**：没有地址黑名单，`net:*` 就是字面意义上的任意目标，回环 /
  私有网段 / 云元数据地址（`169.254.169.254`）一视同仁。要防恶意插件，唯一的手段是**不安装它**。
- **也别指望声明能挡住攻击者**：真要挡是换信任模型（OS 沙箱 / 独立进程），不是在这份清单里
  再加几条限制。**不要**出于「补一个 SSRF 防护」的好意在 host 函数里加黑名单 —— 那会伪造出
  一道并不存在的边界，比没有更危险。

## 资源限制（防死循环 / 吃满内存）

fuel（指令数）、wall-clock 超时、线性内存页上限、HTTP 响应上限、单次 FS 读写与 KV 值上限。

## 大文件怎么走（流式通道）

一次性通道（`host_fs_read` / `host_fs_write`）要**整份内容进内存**，而且各限 4MB，所以大文件
走流式：`host_fs_open` 拿句柄 → `host_fs_read_at` / `host_fs_write_at` / `host_fs_append` 按偏移
读写，每块 ≤ `MAX_FS_CHUNK`。文件本身留在宿主侧，WASM 里只有当前这一块，峰值内存与文件大小无关。

四条约束必须记住（踩中任何一条都会「看起来能跑但跑不动」）：

1. **单块上限同时受「入参 1MB」约束**：块要能塞进一次 `plugin_call`，所以 `MAX_FS_CHUNK` 不能
   超过 `plugin_host::MAX_CALL_INPUT`。
2. **单次调用只有 15s / 1e9 fuel**：一次调用能处理的字节数有限（解析类工作每字节要几十条指令，
   量级是**几十 MB 而不是几百 MB**），所以长任务必须**可续跑**——把游标随结果返回给页面，由页面
   按游标再调一次，进度由页面自己聚合。**别指望一次调用扫完一个大文件。**
3. **句柄跨调用有效，但别把它当持久状态**：句柄表挂在编译产物缓存上（插件升级 / 禁用 / 卸载会
   整个丢掉）。**唯一真相是游标**（放请求 / 响应里），句柄只是「同一次调用内少解析几次路径」的
   优化——每次按游标打开、用完关闭最稳。
4. **EOF 是短读，不是异常**：`host_fs_read_at` 到末尾会返回不足一块甚至空块，用 `host_fs_size`
   判断总长度，别拿错误当循环出口（宿主错误会让整次调用中止）。

写侧另有一个易踩的点：`host_fs_append` 只接受 `a` 模式打开的句柄，`host_fs_write_at` 只接受
`w` / `rw` 模式——因为 `O_APPEND` 句柄上的「按偏移写」在各平台上行为不一致，混用会静默写错位置。

## 状态归属

**每次调用都新建一个 WASM 实例**（模块编译产物按包目录指纹缓存，升级后自动重编）：
实例内存与 fuel 都在调用结束后丢弃，插件不能靠 WASM 全局变量跨调用保存状态——持久状态
请写 KV / FS。这样既天然重置了 fuel，也避免了「上一次调用的脏状态影响下一次」。

## 构建开关

整段实现挂在 cargo feature `wasm-runtime` 下（默认开启，见 `Cargo.toml`）。
`--no-default-features` 时下面导出的是桩函数，`plugin_call` 回落为 501，
插件退化成「纯前端沙箱页面」，前端桥与插件页代码无需改动。
*/
use serde::Serialize;
use std::path::{Path, PathBuf};

/// 单次调用 wall-clock 超时
const CALL_TIMEOUT_MS: u64 = 15_000;
/// 单次调用指令数上限（防死循环）
const CALL_FUEL: u64 = 1_000_000_000;
/// WASM 线性内存上限（页；1 页 = 64KiB）→ 512 页 = 32MB
const MAX_MEMORY_PAGES: u32 = 512;
/// HTTP 响应体上限（Extism 内建 http_request 的读取上限）
const MAX_HTTP_RESPONSE: u64 = 4 * 1024 * 1024;
/// 单次 FS 读上限
const MAX_FS_READ: u64 = 4 * 1024 * 1024;
/// 单次 FS 写上限
const MAX_FS_WRITE: usize = 4 * 1024 * 1024;
/// 流式通道的单块上限（`host_fs_read_at` / `host_fs_write_at` / `host_fs_append`）
///
/// 必须 ≤ 页面调用的入参上限（`plugin_host::MAX_CALL_INPUT`）：块要能塞进一次 `plugin_call`。
const MAX_FS_CHUNK: u64 = 1024 * 1024;
/// 同时打开的 FS 句柄数上限（句柄随编译产物缓存存活，靠上限 + 显式 close 防泄漏）
const MAX_FS_HANDLES: usize = 8;
/// 同时打开的原始 TCP 连接数上限（连接随编译产物缓存存活，靠上限 + 显式 close 防泄漏）
const MAX_TCP_CONNS: usize = 8;
/// 单次 TCP 收发的字节上限（与流式 FS 同口径，块要能塞进一次宿主函数调用）
const MAX_TCP_CHUNK: u64 = 1024 * 1024;
/// 单次 TCP 建连超时（毫秒）
const TCP_CONNECT_TIMEOUT_MS: u64 = 5_000;
/// 单次 TCP 读/写超时（毫秒）。**必须小于 Extism 单次调用 15s 上限**，否则超时会被
/// 吞成笼统的 `timeout`，而不是这里给出的可读错误。
const TCP_RW_TIMEOUT_MS: u64 = 10_000;
/// `net:` 声明条数上限（防清单塞爆）
const MAX_HTTP_HOSTS: usize = 16;
/// 出错内存快照的文件名（落在插件私有数据目录内，见 `memdump_enabled`）
///
/// `pub(crate)`：`plugin_host` 的 `plugin://` 协议层要用它把快照从 URL 可访问集合里排除
/// （内存快照不能重复两处定义，否则一侧改名另一侧会静默放行）。
pub(crate) const MEMDUMP_FILE: &str = "last-error.mem";
/// 编译缓存指纹里区分「开了出错快照」的盐值（快照是编译期选项，切换开关要让缓存失配）
const MEMDUMP_FP_SALT: u64 = 0x4D45_4D44_554D_5001;
/// 同时执行的 WASM 调用数上限
///
/// 每次调用都要实例化一块最多 32MB 的线性内存 + 走一次
/// [`tokio::task::spawn_blocking`] 的线程；不设上限的话，一个插件里几十个面板同时发请求
/// 就能把阻塞线程池与内存一起吃光。
const MAX_CONCURRENT_CALLS: usize = 4;
/// 拿不到并发额度时最多等多久（毫秒）
///
/// 队列本身也是资源，所以是「排队 + 超时报错」而不是「无限排队」：宁可让页面立刻拿到一句
/// 「太忙了」，也不要让点击变成「转圈十几秒然后超时」。
const CALL_QUEUE_WAIT_MS: u64 = 3_000;
/// 连续多少次**宿主级故障**后跳闸（见 `imp::is_host_fault` 的取舍）
const BREAKER_THRESHOLD: u32 = 3;
/// 跳闸后的冷却时长（毫秒）
///
/// 冷却结束不直接「恢复」，而是**放一次探针通过**（`Circuit::check` 顺手清零计数）：
/// 探针成功就彻底恢复，失败则重新计数。省掉一个 half-open 状态，语义一样。
const BREAKER_COOLDOWN_MS: u64 = 30_000;

/// 运行时信息（`plugin_runtime_info` 命令返回；设置页展示 + 排查用）
#[derive(Debug, Clone, Serialize)]
pub struct RuntimeInfo {
    /// 是否启用 WASM 运行时（构建期决定）
    pub available: bool,
    /// 运行时名称（未启用时为空串）
    pub runtime: String,
    /// 单次调用指令数上限
    pub fuel: u64,
    /// 单次调用超时（毫秒）
    pub timeout_ms: u64,
    /// 线性内存上限（MB）
    pub max_memory_mb: u64,
    /// HTTP 响应上限（MB）
    pub max_http_response_mb: u64,
    /// 单次 FS 读上限（MB）
    pub max_fs_read_mb: u64,
    /// 流式通道的单块上限（MB）
    pub max_fs_chunk_mb: u64,
    /// 可同时打开的 FS 句柄数
    pub max_fs_handles: usize,
    /// 支持的权限声明形态（给插件作者看的白名单）
    pub permissions: Vec<String>,
    /// 调试开关：插件调用出错时是否把线性内存快照导出到插件数据目录
    pub memdump_on_error: bool,
}

/// 构造运行时信息（两种构建共用，available 由 feature 决定）
fn runtime_info_of(available: bool, memdump_on_error: bool) -> RuntimeInfo {
    RuntimeInfo {
        available,
        runtime: if available { "Extism (wasmtime)".to_string() } else { String::new() },
        fuel: CALL_FUEL,
        timeout_ms: CALL_TIMEOUT_MS,
        max_memory_mb: (MAX_MEMORY_PAGES as u64 * 64) / 1024,
        max_http_response_mb: MAX_HTTP_RESPONSE / 1024 / 1024,
        max_fs_read_mb: MAX_FS_READ / 1024 / 1024,
        max_fs_chunk_mb: MAX_FS_CHUNK / 1024 / 1024,
        max_fs_handles: MAX_FS_HANDLES,
        permissions: vec![
            "log".to_string(),
            "kv".to_string(),
            "fs:read[:<绝对路径|$HOME[/子路径]>]".to_string(),
            "fs:write[:<绝对路径|$HOME[/子路径]>]".to_string(),
            "net:<host>[:<port>]（网络访问授权；协议不限，host / port 支持 * 通配）".to_string(),
        ],
        memdump_on_error,
    }
}

/// 取运行时信息失败时的占位值（`plugin_runtime_info` 的兜底）
///
/// `available=false` 会让 UI 显示「未启用」，真正的原因在 `Response.msg` 里——
/// 总比让整个命令 panic 掉、前端拿不到任何东西要好。
pub fn runtime_info_placeholder() -> RuntimeInfo {
    runtime_info_of(false, false)
}

/// 插件声明的 `fs` 作用域根目录（已展开 `$HOME`、已规范化）
///
/// 用途：宿主的**文件对话框**要把用户选中的路径限制在声明过的范围内 —— 对话框返回的是字符串，
/// 本身不授予任何访问权，但让它能指向任意位置会误导用户（「选了却读不了」），也可能被用来
/// 套出目录结构。判定用的就是 WASM 侧 `host_fs_open` 的同一份 `read_roots` / `write_roots`，
/// **不另写一套前缀比较**，避免两处口径漂移成「声明了却拦不住」或反过来。
#[derive(Debug, Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FsRoots {
    /// `fs:read[:作用域]` 解析出的根目录（不带作用域 = 插件私有数据目录）
    pub read: Vec<String>,
    /// `fs:write[:作用域]` 解析出的根目录
    pub write: Vec<String>,
    /// 声明了 `fs:read:*`：任意路径可读（`read` 可能为空，判定以本字段为准）
    pub read_any: bool,
    /// 声明了 `fs:write:*`：任意路径可写
    pub write_any: bool,
    /// 即使在 `*_any` 下也**禁止**选择的路径（跨插件隔离：其它插件数据目录 / 内置插件数据 / 插件安装根）
    ///
    /// 前端对话框用它做排除：不给这份列表，用户会在对话框里选到「选完才被拒」的路径。
    pub deny: Vec<String>,
}

/// 一条路径是否被 fs 权限放行（**纯函数**，判定只有这一处，便于单测）
///
/// - `deny` **优先级最高**：跨插件隔离不因「显式声明了该作用域」而放开 —— 故障半径要按插件切分。
/// - `any` 为真（声明了 `fs:read:*` / `fs:write:*`）时，除 `deny` 之外一律放行。
/// - 其余情况逐个 `roots` 做组件级前缀比较（入参都已 `canonicalize`，`../` 与软链逃逸已在解析阶段解决）。
pub(crate) fn fs_permitted(any: bool, roots: &[PathBuf], deny: &[PathBuf], abs: &Path) -> bool {
    if deny.iter().any(|r| abs.starts_with(r)) {
        return false;
    }
    if any {
        return true;
    }
    roots.iter().any(|r| abs.starts_with(r))
}

// ==================== 进程内存口径（宿主侧） ====================

/// 宿主进程内存快照
///
/// 为什么用进程口径：WASM 线性内存是 wasmtime 在**宿主进程内** mmap 出来的私有内存，
/// 插件的线性内存必然计入宿主进程的提交量，所以「调用前后进程提交量的增量」是
/// 不依赖插件配合就能拿到的近似口径（见 `CallStats`）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize)]
pub struct ProcMem {
    /// 私有提交字节（Windows `PrivateUsage`）——最贴近「线性内存 + 实例元数据」
    pub commit: u64,
    /// 物理驻留字节（Windows `WorkingSetSize`）——是否真的被换入物理内存
    pub working_set: u64,
}

/// 采样当前进程内存；拿不到的平台返回 `None`（前端按「无数据」展示，不做假值）
#[cfg(target_os = "windows")]
pub fn proc_mem() -> Option<ProcMem> {
    use std::mem::size_of;
    use windows::Win32::System::ProcessStatus::{
        K32GetProcessMemoryInfo, PROCESS_MEMORY_COUNTERS, PROCESS_MEMORY_COUNTERS_EX,
    };
    use windows::Win32::System::Threading::GetCurrentProcess;

    let mut counters = PROCESS_MEMORY_COUNTERS_EX::default();
    counters.cb = size_of::<PROCESS_MEMORY_COUNTERS_EX>() as u32;
    // 传 _EX 结构 + 对应 cb 是 PSAPI 的约定用法（GetProcessMemoryInfo 的宏别名在 windows-rs 里叫 K32*）
    // SAFETY: 句柄是本进程伪句柄；指针指向已按 API 要求填好 cb 的栈上结构体
    let ok = unsafe {
        K32GetProcessMemoryInfo(
            GetCurrentProcess(),
            &mut counters as *mut PROCESS_MEMORY_COUNTERS_EX as *mut PROCESS_MEMORY_COUNTERS,
            counters.cb,
        )
    };
    if !ok.as_bool() {
        return None;
    }
    Some(ProcMem {
        commit: counters.PrivateUsage as u64,
        working_set: counters.WorkingSetSize as u64,
    })
}

/// 非 Windows：暂不采集（macOS 可用 `task_info(TASK_VM_INFO)`，尚未实现，见 docs）
#[cfg(not(target_os = "windows"))]
pub fn proc_mem() -> Option<ProcMem> {
    None
}

/// 单次调用的资源占用（`plugin_last_stats` 返回）
///
/// 口径说明（务必按这个理解，否则会误判）：
/// * `mem_delta_bytes` / `rss_delta_bytes` 是**调用前后宿主进程的增量**，包含插件的线性内存、
///   WASM 实例元数据、以及 wasmtime 为这次调用临时分配的东西；同时也会被宿主其它活动的噪声影响。
///   因为实例在 `call` 返回后才析构，采样点选在**实例仍存活时**，所以它偏「本次实例占用」而不是峰值。
/// * 想要插件的**精确**线性内存用量，让插件自报（`core::arch::wasm32::memory_size`），
///   Extism 没把 Store/Memory 暴露给宿主（详见 docs/plugin-dev/capabilities.md）。
/// * 拿不到（非 Windows / 采样失败）时为 `None`，不要当成 0。
#[derive(Debug, Clone, Serialize)]
pub struct CallStats {
    pub plugin_id: String,
    pub func: String,
    /// 本次调用是否成功
    pub ok: bool,
    /// 本次调用消耗的指令数（Extism fuel；拿不到时为 None）
    pub fuel_used: Option<u64>,
    /// 进程提交内存增量（字节，负增量按 0 计）
    pub mem_delta_bytes: Option<u64>,
    /// 进程驻留内存增量（字节，负增量按 0 计）
    pub rss_delta_bytes: Option<u64>,
    /// 采样时进程提交内存总量（字节）
    pub proc_commit_bytes: Option<u64>,
    /// 出错快照文件路径（仅在开启 memdump 且文件存在时给出）
    pub memdump_path: Option<String>,
    /// 出错快照文件大小（字节）——**等于出错那一刻插件的线性内存字节数**
    pub memdump_bytes: Option<u64>,
    /// 连续「宿主级故障」次数（累计到 `BREAKER_THRESHOLD` 就跳闸）
    pub consec_failures: u32,
    /// 熔断剩余冷却毫秒（0 = 没在冷却）
    ///
    /// 出现在这个结构体里是为了让设置页能回答「为什么这个插件点不动」：
    /// 熔断期间的调用会被直接拒，**不会产生新的交付**，不写在这里就查不到。
    pub circuit_cooldown_ms: u64,
}

/// 调试开关：插件调用出错时是否导出线性内存快照
///
/// 存在核心 KV `settings.json` 的 `plugin_memdump_on_error`（默认关）。
/// 注意它是**编译期**选项（Extism 的 `with_memdump` 作用于 PluginBuilder），
/// 所以开关值会折进编译缓存指纹，切换后下一次调用自动重编。
/// 开启后**每次出错**都会把整块线性内存（最多 `MAX_MEMORY_PAGES` 页）写到插件的
/// 私有数据目录，反复出错的插件会反复写盘 —— 只作为排障开关，别长期开着。
pub fn memdump_enabled() -> bool {
    crate::app::app_store::read_settings()
        .get("plugin_memdump_on_error")
        .and_then(|v| v.as_bool())
        .unwrap_or(false)
}

#[cfg(feature = "wasm-runtime")]
pub use imp::{call, drop_instance, fs_roots, last_stats, probe, runtime_info};

#[cfg(feature = "wasm-runtime")]
mod imp {
    use std::collections::HashMap;
    use std::ffi::OsString;
    use std::fs;
    use std::hash::{Hash, Hasher};
    use std::path::{Path, PathBuf};
    use std::sync::{Arc, Condvar, LazyLock, Mutex, MutexGuard};
    use std::time::{Duration, Instant, UNIX_EPOCH};

    use extism::{
        CompiledPlugin, CurrentPlugin, Error, Function, Manifest, Plugin, PluginBuilder, UserData,
        Val, Wasm, EXTISM_USER_MODULE, PTR,
    };

    use super::{
        fs_permitted, memdump_enabled, proc_mem, runtime_info_of, CallStats, FsRoots, ProcMem,
        RuntimeInfo,
        BREAKER_COOLDOWN_MS, BREAKER_THRESHOLD, CALL_FUEL,
        CALL_QUEUE_WAIT_MS, CALL_TIMEOUT_MS, MAX_CONCURRENT_CALLS, MAX_FS_CHUNK, MAX_FS_HANDLES,
        MAX_FS_READ, MAX_FS_WRITE, MAX_HTTP_HOSTS,
        MAX_HTTP_RESPONSE, MAX_MEMORY_PAGES, MAX_TCP_CHUNK, MAX_TCP_CONNS, MEMDUMP_FILE,
        MEMDUMP_FP_SALT, TCP_CONNECT_TIMEOUT_MS, TCP_RW_TIMEOUT_MS,
    };
    use crate::app::kv_store;
    use crate::plugin_host::host::{install_root, parse_manifest, plugin_data_ext_dir,
        plugin_data_root, plugin_dir, plugin_settings_of, PluginManifest};
    use crate::response::response::Response;

    /// 路径参数长度上限
    const MAX_PATH_LEN: usize = 1024;
    /// KV 键名长度上限
    const MAX_KV_KEY_LEN: usize = 256;
    /// KV 单值上限
    const MAX_KV_VALUE_LEN: usize = 256 * 1024;
    /// KV 键数量上限
    const MAX_KV_KEYS: usize = 512;

    // ==================== 编译产物缓存 ====================

    /// `plugin_id -> (包目录指纹, 已编译模块)`
    ///
    /// 同一份 wasm 只编译一次；包目录内容变化（覆盖安装、手工改动）后指纹失配自动重编。
    /// 只缓存**编译产物**，每次调用仍旧新建 `Plugin` 实例（见模块头「状态归属」）。
    static REGISTRY: LazyLock<Mutex<HashMap<String, (u64, CompiledPlugin)>>> =
        LazyLock::new(|| Mutex::new(HashMap::new()));

    fn registry() -> Result<MutexGuard<'static, HashMap<String, (u64, CompiledPlugin)>>, String> {
        REGISTRY
            .lock()
            .map_err(|_| "插件运行时注册表已损坏".to_string())
    }

    /// 各插件「上一次调用」的资源占用（见 `last_stats`）
    ///
    /// 与 `REGISTRY`（编译产物，按目录指纹缓存）不同，这里只是给设置页自查用的观测数据：
    /// 每个插件只留最近一次，避免无界增长；禁用 / 卸载时随 `drop_instance` 一起清掉。
    static LAST_STATS: LazyLock<Mutex<HashMap<String, CallStats>>> =
        LazyLock::new(|| Mutex::new(HashMap::new()));

    /// 读取某插件上一次调用的资源占用；从没调用过（或刚被丢弃）时为 `None`
    pub fn last_stats(id: &str) -> Response<Option<CallStats>> {
        match LAST_STATS.lock() {
            Ok(m) => Response::new(200, m.get(id).cloned(), "ok"),
            Err(_) => Response::new(500, None, "插件资源统计表已损坏"),
        }
    }

    /// 丢弃某插件的编译产物与调用统计（安装 / 卸载 / 禁用后调用）
    ///
    /// 顺带清熔断计数：重装 / 重新启用本来就是用户表达「我已经修好了」，
    /// 让它继续吃上一次的冷却期只会让人以为「重装也没用」。
    pub fn drop_instance(id: &str) {
        if let Ok(mut reg) = registry() {
            reg.remove(id);
        }
        if let Ok(mut stats) = LAST_STATS.lock() {
            stats.remove(id);
        }
        circuits().remove(id);
    }

    // ==================== 路径工具 ====================

    /// 用户主目录（用于展开 `$HOME` / `~/`）
    fn home_dir() -> PathBuf {
        #[cfg(windows)]
        let key = "USERPROFILE";
        #[cfg(not(windows))]
        let key = "HOME";
        std::env::var_os(key).map(PathBuf::from).unwrap_or_default()
    }

    /// 展开 `$HOME` / `~/` 前缀（其余原样返回，含 `$HOME` 后无分隔符的写法）
    fn expand_home(raw: &str) -> String {
        let home = home_dir();
        let home = home.to_string_lossy().to_string();
        if home.is_empty() {
            return raw.to_string();
        }
        if raw == "$HOME" || raw == "~" {
            return home;
        }
        for prefix in ["$HOME/", "~/"] {
            if let Some(rest) = raw.strip_prefix(prefix) {
                return format!("{}/{}", home.trim_end_matches(['/', '\\']), rest);
            }
        }
        raw.to_string()
    }

    /// 规范化路径：尽量 `canonicalize`，不存在的部分原样拼回。
    ///
    /// 用于「写一个还不存在的文件」这类场景——把最深的已存在祖先规范化后拼上剩余组件，
    /// 这样软链逃逸同样会被解析出来，后续前缀比较才有意义。
    fn canonicalize_loose(p: &Path) -> PathBuf {
        if let Ok(c) = fs::canonicalize(p) {
            return c;
        }
        let mut probe = p.to_path_buf();
        let mut rest: Vec<OsString> = Vec::new();
        while let Some(name) = probe.file_name().map(|n| n.to_os_string()) {
            rest.push(name);
            let Some(parent) = probe.parent().map(|x| x.to_path_buf()) else {
                break;
            };
            probe = parent;
            if let Ok(c) = fs::canonicalize(&probe) {
                let mut out = c;
                for n in rest.iter().rev() {
                    out.push(n);
                }
                return out;
            }
        }
        p.to_path_buf()
    }

    /// 解析 `fs:read[:作用域]` / `fs:write[:作用域]` 的作用域根目录。
    /// 无作用域 = 插件私有数据目录；有作用域时必须是绝对路径或 `$HOME` 前缀。
    fn scope_root(scope: &str, data_root: &Path) -> Result<PathBuf, String> {
        let raw = scope.strip_prefix(':').unwrap_or("");
        if raw.is_empty() {
            return Ok(data_root.to_path_buf());
        }
        let expanded = expand_home(raw);
        let p = PathBuf::from(&expanded);
        if !p.is_absolute() {
            return Err(format!("fs 作用域必须是绝对路径或 $HOME 前缀: {}", raw));
        }
        Ok(canonicalize_loose(&p))
    }

    // ==================== FS 句柄表（流式通道） ====================

    /// IO 的访问要求（`CapCtx::fs_take` 用它校验「模式是否匹配」）
    #[derive(Clone, Copy, PartialEq, Eq)]
    enum Access {
        Read,
        Write,
        Any,
    }

    /// 一个打开的流式句柄（宿主侧资源；WASM 拿到的只是一个编号，没有真实句柄）
    struct FsHandle {
        file: fs::File,
        readable: bool,
        writable: bool,
        /// 是否以追加模式打开（决定能用 `host_fs_append` 还是 `host_fs_write_at`）
        append: bool,
        /// 打开时的绝对路径：只用于错误信息（权限在打开那一刻就校验过了）
        path: PathBuf,
    }

    /// 做一次 IO 用的句柄快照（克隆出的 `File` 在锁外使用）
    struct HandleIo {
        file: fs::File,
        append: bool,
        path: PathBuf,
    }

    /// 句柄表：跟着 `CapCtx`（编译产物缓存）存活，所以**句柄跨调用有效**；
    /// 插件升级 / 卸载 / 禁用会丢掉整个 ctx，句柄随之析构关闭。
    struct HandleTable {
        next: u32,
        open: HashMap<u32, FsHandle>,
    }

    impl Default for HandleTable {
        fn default() -> Self {
            // 编号从 1 开始：留 0 给「空 / 无效」的直觉
            Self { next: 1, open: HashMap::new() }
        }
    }

    // ==================== 网络访问授权 + 原始连接表 ====================

    /// 一条网络访问权限声明（`net:<host>[:<port>]`）。
    ///
    /// **宿主只判定「这个目标能不能连」，不认协议。** 同一个声明同时作用于两个网络出口：
    /// Extism 内建 HTTP（按 host glob 匹配）与原始 socket（按 `host:port` 匹配）。连上之后发什么
    /// 字节、上层是什么协议（RESP / TLS / 自定义二进制…）全由插件自己实现，宿主不看内容。
    struct NetAllow {
        /// 目标主机；`*` = 任意（子域通配写 `*.example.com`）
        host: String,
        /// 目标端口；`None` = 任意端口
        port: Option<u16>,
    }

    /// 原始 TCP 连接表：键由插件自己给（通常是它自己的连接 id），值是一条**阻塞** `TcpStream`。
    ///
    /// 与 FS 句柄一样挂在 `CapCtx`（编译产物缓存）上 ⇒ **连接跨调用保留**（这样才能一条连接
    /// 跑多条命令），插件升级 / 禁用 / 卸载时整表析构关闭。插件的 WASM 实例每次调用都重建，
    /// 所以「连接」这个持久资源只能放宿主侧，插件的状态只有它自己给的那个键。
    struct TcpTable {
        conns: HashMap<String, std::net::TcpStream>,
    }

    impl Default for TcpTable {
        fn default() -> Self {
            Self { conns: HashMap::new() }
        }
    }

    /// 解析一条 `net:` 声明：`net` / `net:*`（任意目标）、`net:<host>`、`net:<host>:<port>`。
    /// `host` / `port` 任一可为 `*`；`host` 允许 `*.example.com` 子域通配（HTTP 侧交给 glob）。
    ///
    /// 非 `net:` 前缀返回 `None`（交给别的权限分支），格式非法返回 `Some(Err)` —— **响亮失败**：
    /// 静默降级成「忽略这条」等于悄悄收窄权限，降级成「当成通配」则是提权，两者都不能接受。
    fn parse_net_allow(perm: &str) -> Option<Result<NetAllow, String>> {
        if perm == "net" {
            return Some(Err("`net` 声明缺冒号，应为 `net:<host>[:<port>]` 或 `net:*`".to_string()));
        }
        let rest = perm.strip_prefix("net:")?.trim();
        if rest.is_empty() || rest == "*" || rest == "*:*" {
            return Some(Ok(NetAllow { host: "*".to_string(), port: None }));
        }
        // 旧写法 `net:tcp:…` 已废弃：必须显式报错，否则会被当成主机名 `tcp:127.0.0.1` 静默误解析
        if rest == "tcp" || rest.starts_with("tcp:") {
            return Some(Err(format!(
                "`net:tcp:` 写法已废弃，请改成 `net:<host>[:<port>]`（网络权限不区分协议）: {}",
                perm
            )));
        }
        // 主机名本身可能带冒号（IPv6 字面量 `[::1]`）：只有 `]` 之后的冒号才是端口分隔符
        let (h, p) = if rest.starts_with('[') {
            match rest.find(']') {
                Some(i) => match rest[i + 1..].strip_prefix(':') {
                    Some(port) => (&rest[..=i], port),
                    None => (rest, ""),
                },
                None => (rest, ""),
            }
        } else {
            rest.rsplit_once(':').unwrap_or((rest, ""))
        };
        let host = if h.is_empty() || h == "*" { "*".to_string() } else { h.to_string() };
        let port = if p.is_empty() || p == "*" {
            None
        } else {
            match p.parse::<u16>() {
                Ok(n) => Some(n),
                Err(_) => return Some(Err(format!("net 端口非法（应为 0-65535 或 *）: {}", p))),
            }
        };
        Some(Ok(NetAllow { host, port }))
    }

    /// `host:port` 是否被任一条 `net:` 声明覆盖（host 大小写不敏感；`*` 匹配任意主机 / 端口）
    fn net_allows_target(allows: &[NetAllow], host: &str, port: u16) -> bool {
        allows.iter().any(|a| {
            (a.host == "*" || a.host.eq_ignore_ascii_case(host))
                && (a.port.is_none() || a.port == Some(port))
        })
    }

    /// 供 Extism `allowed_hosts`（HTTP）消费的域名白名单，**由同一套 `net:` 声明派生**。
    ///
    /// 权限是对「目标」的授权、不是对协议的授权，所以一条声明要同时覆盖两个出口。这里只取 host
    /// 部分：Extism 是拿 `url.host_str()`（**不含端口**）做 glob 匹配的（见 `extism/src/pdk.rs`），
    /// 写成 `host:port` 会永远匹配不上；`*` 交给 glob 表示任意主机。
    fn http_allowed_hosts(perms: &[String]) -> Vec<String> {
        let mut out: Vec<String> = Vec::new();
        for p in perms {
            let Some(Ok(a)) = parse_net_allow(p) else { continue };
            if !out.iter().any(|h| h.eq_ignore_ascii_case(&a.host)) {
                out.push(a.host);
            }
            if out.len() >= MAX_HTTP_HOSTS {
                break;
            }
        }
        out
    }

    // ==================== 能力上下文 ====================

    /// 一次调用期间的宿主能力上下文（由清单 permissions 解析而来，不可变）
    struct CapCtx {
        id: String,
        /// 插件私有数据目录（相对路径的基准，也是无作用域 fs 权限的范围）
        data_dir: PathBuf,
        /// 允许读取的根目录（已规范化）
        read_roots: Vec<PathBuf>,
        /// 允许写入的根目录（已规范化）
        write_roots: Vec<PathBuf>,
        /// `fs:read:*`：任意路径可读（**排除区除外**，见 `deny_roots`）
        read_any: bool,
        /// `fs:write:*`：任意路径可写（同上）
        write_any: bool,
        /// 跨插件隔离的排除区：即使 `*_any` 为真也一律拒绝（其它插件数据 / 内置插件数据 / 插件安装根）
        deny_roots: Vec<PathBuf>,
        can_log: bool,
        can_kv: bool,
        /// 流式 FS 句柄（跨调用保留，见 `HandleTable`）
        handles: Mutex<HandleTable>,
        /// 允许访问的目标（`net:<host>[:<port>]` 声明解析而来；HTTP 与原始 socket 共用）
        net_allowed: Vec<NetAllow>,
        /// 已建立的原始 TCP 连接（跨调用保留，见 `TcpTable`）
        tcp: Mutex<TcpTable>,
    }

    impl CapCtx {
        fn require_log(&self) -> Result<(), Error> {
            if self.can_log {
                Ok(())
            } else {
                Err(Error::msg(format!("插件 {} 未声明 log 权限", self.id)))
            }
        }

        fn require_kv(&self) -> Result<(), Error> {
            if self.can_kv {
                Ok(())
            } else {
                Err(Error::msg(format!("插件 {} 未声明 kv 权限", self.id)))
            }
        }

        /// 校验目标 `host:port` 是否落在已声明的 `net:` 作用域内
        fn net_permitted(&self, host: &str, port: u16) -> bool {
            net_allows_target(&self.net_allowed, host, port)
        }

        /// 取一条 TCP 连接的可克隆句柄（克隆是为了**在锁外做阻塞 IO**，
        /// 避免把连接表的锁攥在一次网络往返上）
        fn tcp_take(&self, key: &str) -> Result<std::net::TcpStream, Error> {
            let table = self
                .tcp
                .lock()
                .map_err(|_| Error::msg("插件 TCP 连接表已损坏"))?;
            let stream = table
                .conns
                .get(key)
                .ok_or_else(|| Error::msg(format!("TCP 连接不存在或已关闭: {}", key)))?;
            stream
                .try_clone()
                .map_err(|e| Error::msg(format!("复制 TCP 连接失败: {}", e)))
        }

        /// 把请求路径解析成规范化绝对路径：相对路径一律相对插件数据目录
        fn resolve(&self, req: &str) -> Result<PathBuf, Error> {
            if req.is_empty() || req.len() > MAX_PATH_LEN || req.contains('\0') {
                return Err(Error::msg("路径参数非法（空 / 过长 / 含空字节）"));
            }
            let expanded = expand_home(req);
            let p = PathBuf::from(&expanded);
            let abs = if p.is_absolute() { p } else { self.data_dir.join(p) };
            Ok(canonicalize_loose(&abs))
        }

        /// 是否落在跨插件隔离的排除区（自己的数据目录**不在**其中）
        fn is_denied(&self, abs: &Path) -> bool {
            self.deny_roots.iter().any(|r| abs.starts_with(r))
        }

        /// 读权限判定：`fs:read:*` 时只剩「排除区」这一道闸
        fn read_permitted(&self, abs: &Path) -> bool {
            fs_permitted(self.read_any, &self.read_roots, &self.deny_roots, abs)
        }

        fn write_permitted(&self, abs: &Path) -> bool {
            fs_permitted(self.write_any, &self.write_roots, &self.deny_roots, abs)
        }

        /// 读或写任一作用域覆盖即可（用于 `host_fs_stat` 这类中性的存在性查询）
        fn any_permitted(&self, abs: &Path) -> bool {
            if self.read_any || self.write_any {
                fs_permitted(true, &[], &self.deny_roots, abs)
            } else {
                fs_permitted(false, &self.read_roots, &self.deny_roots, abs)
                    || fs_permitted(false, &self.write_roots, &self.deny_roots, abs)
            }
        }

        fn read_path(&self, req: &str) -> Result<PathBuf, Error> {
            let abs = self.resolve(req)?;
            if self.read_permitted(&abs) {
                Ok(abs)
            } else if self.is_denied(&abs) {
                Err(Error::msg(format!(
                    "无读取权限（其它插件的数据目录与插件安装目录即使声明 fs:read:* 也不可访问）: {}",
                    req
                )))
            } else {
                Err(Error::msg(format!(
                    "无读取权限（插件未声明覆盖该路径的 fs:read 作用域）: {}",
                    req
                )))
            }
        }

        fn write_path(&self, req: &str) -> Result<PathBuf, Error> {
            let abs = self.resolve(req)?;
            if self.write_permitted(&abs) {
                Ok(abs)
            } else if self.is_denied(&abs) {
                Err(Error::msg(format!(
                    "无写入权限（其它插件的数据目录与插件安装目录即使声明 fs:write:* 也不可访问）: {}",
                    req
                )))
            } else {
                Err(Error::msg(format!(
                    "无写入权限（插件未声明覆盖该路径的 fs:write 作用域）: {}",
                    req
                )))
            }
        }

        fn any_path(&self, req: &str) -> Result<PathBuf, Error> {
            let abs = self.resolve(req)?;
            if self.any_permitted(&abs) {
                Ok(abs)
            } else if self.is_denied(&abs) {
                Err(Error::msg(format!(
                    "无访问权限（其它插件的数据目录与插件安装目录即使声明 fs:*:* 也不可访问）: {}",
                    req
                )))
            } else {
                Err(Error::msg(format!("无访问权限（未声明覆盖该路径的 fs 作用域）: {}", req)))
            }
        }

        // ---- 流式通道（大文件） ----

        fn handles(&self) -> Result<MutexGuard<'_, HandleTable>, Error> {
            self.handles
                .lock()
                .map_err(|_| Error::msg("插件 FS 句柄表已损坏"))
        }

        /// 打开一个流式句柄，返回编号。
        ///
        /// 模式：`r` 只读（必须存在）/ `w` 写（截断或新建）/ `a` 追加（新建）/
        /// `rw` 读写（不截断，新建）。**权限只在打开这一次校验**，之后按句柄操作不再重复解析
        /// 路径——句柄本身就是「已授权」的凭证，所以它只在沙箱内可见，出不去。
        fn fs_open(&self, req: &str, mode: &str) -> Result<u32, Error> {
            let (path, readable, writable, truncate, append) = match mode {
                "r" => (self.read_path(req)?, true, false, false, false),
                "w" => (self.write_path(req)?, false, true, true, false),
                "a" => (self.write_path(req)?, false, true, false, true),
                // 读写同一个文件（就地改几个字节）：两个方向的作用域都要求
                "rw" => (self.read_path(req)?, true, true, false, false),
                other => {
                    return Err(Error::msg(format!(
                        "打开模式非法（只支持 r / w / a / rw）: {}",
                        other
                    )))
                }
            };
            if mode == "rw" {
                // 读得动不代表写得动：再校验一次写作用域
                self.write_path(req)?;
            }
            if readable && !writable {
                let meta = fs::metadata(&path)
                    .map_err(|e| Error::msg(format!("读取文件信息失败（{}）: {}", req, e)))?;
                if meta.is_dir() {
                    return Err(Error::msg(format!("目标是目录，读文件请用 host_fs_read: {}", req)));
                }
            }
            let file = fs::OpenOptions::new()
                .read(readable)
                .write(writable)
                .append(append)
                .create(writable)
                .truncate(truncate)
                .open(&path)
                .map_err(|e| Error::msg(format!("打开文件失败（{}）: {}", req, e)))?;

            let mut table = self.handles()?;
            if table.open.len() >= MAX_FS_HANDLES {
                return Err(Error::msg(format!(
                    "同时打开的文件数超过 {} 上限（用完记得 host_fs_close）",
                    MAX_FS_HANDLES
                )));
            }
            let id = table.next;
            table.next = table.next.wrapping_add(1).max(1);
            table.open.insert(id, FsHandle { file, readable, writable, append, path });
            Ok(id)
        }

        /// 取句柄做一次 IO：返回 `(File 克隆, 是否追加模式, 路径)`。
        ///
        /// 克隆是为了**在锁外做 IO**：读写都走「按偏移」语义、不依赖游标，所以克隆之间互不
        /// 干扰，也就不会把句柄表的锁攥在磁盘 IO 上。
        fn fs_take(&self, id: i64, want: Access) -> Result<HandleIo, Error> {
            let key = u32::try_from(id).map_err(|_| Error::msg(format!("FS 句柄非法: {}", id)))?;
            let table = self.handles()?;
            let h = table
                .open
                .get(&key)
                .ok_or_else(|| Error::msg(format!("FS 句柄无效或已关闭: {}", id)))?;
            if want == Access::Read && !h.readable {
                return Err(Error::msg(format!(
                    "句柄 {} 不是读模式（用 host_fs_open 的 r / rw 重新打开）",
                    id
                )));
            }
            if want == Access::Write && !h.writable {
                return Err(Error::msg(format!(
                    "句柄 {} 不是写模式（用 host_fs_open 的 w / a / rw 重新打开）",
                    id
                )));
            }
            let file = h
                .file
                .try_clone()
                .map_err(|e| Error::msg(format!("复制文件句柄失败: {}", e)))?;
            Ok(HandleIo { file, append: h.append, path: h.path.clone() })
        }

        /// 关闭句柄（幂等：重复关闭返回成功，方便写「出错也要关」的清理路径）
        fn fs_close(&self, id: i64) -> Result<(), Error> {
            let Ok(key) = u32::try_from(id) else {
                return Ok(());
            };
            self.handles()?.open.remove(&key);
            Ok(())
        }
    }

    /// 跨插件隔离的排除区（`CapCtx::deny_roots`）
    ///
    /// 三条：其它第三方插件的数据目录、内置插件数据根、插件安装根。自己的数据目录**不在**其中 ——
    /// 声明了 `fs:read:*` 的插件当然还得能读自己的地盘。
    ///
    /// 这里**枚举**兄弟目录而不是「整根排除 + 单独放行自己」：排除根会同时被前端对话框拿去做
    /// 前缀比较，整根排除就得再传一份「例外」，两处口径很容易漂。
    fn foreign_roots(id: &str, data_dir: &Path) -> Vec<PathBuf> {
        let mut out: Vec<PathBuf> = Vec::new();
        // 第三方插件数据根就是自己数据目录的父目录
        if let Some(ext_root) = data_dir.parent() {
            if let Ok(entries) = fs::read_dir(ext_root) {
                for entry in entries.flatten() {
                    let name = entry.file_name().to_string_lossy().to_string();
                    if name.eq_ignore_ascii_case(id) {
                        continue;
                    }
                    let p = entry.path();
                    if !p.is_dir() {
                        continue;
                    }
                    out.push(canonicalize_loose(&p));
                }
            }
        }
        out.push(canonicalize_loose(&plugin_data_root()));
        out.push(canonicalize_loose(&install_root()));
        out
    }

    fn build_ctx(id: &str, manifest: &PluginManifest) -> Result<Arc<CapCtx>, String> {
        let data_dir = plugin_data_ext_dir(id)?;
        fs::create_dir_all(&data_dir).map_err(|e| format!("创建插件数据目录失败: {}", e))?;
        let data_root = canonicalize_loose(&data_dir);

        let mut ctx = CapCtx {
            id: id.to_string(),
            data_dir: data_root.clone(),
            read_roots: Vec::new(),
            write_roots: Vec::new(),
            read_any: false,
            write_any: false,
            deny_roots: foreign_roots(id, &data_root),
            can_log: false,
            can_kv: false,
            handles: Mutex::new(HandleTable::default()),
            net_allowed: Vec::new(),
            tcp: Mutex::new(TcpTable::default()),
        };
        for perm in &manifest.permissions {
            if perm == "log" {
                ctx.can_log = true;
            } else if perm == "kv" {
                ctx.can_kv = true;
            } else if perm == "fs:read:*" {
                // 通配作用域必须**先**判：`strip_prefix("fs:read")` 也会命中它，
                // 落到 `scope_root` 会被当成路径解析而报「必须是绝对路径」。
                ctx.read_any = true;
            } else if perm == "fs:write:*" {
                ctx.write_any = true;
            } else if let Some(scope) = perm.strip_prefix("fs:read") {
                ctx.read_roots.push(scope_root(scope, &data_root)?);
            } else if let Some(scope) = perm.strip_prefix("fs:write") {
                ctx.write_roots.push(scope_root(scope, &data_root)?);
            } else if let Some(parsed) = parse_net_allow(perm) {
                // 网络访问授权：一次声明同时供两个出口（HTTP 白名单 + 原始 socket），宿主不认协议
                ctx.net_allowed.push(parsed?);
            }
        }
        Ok(Arc::new(ctx))
    }

    /// 插件声明的 fs 作用域根目录（对话框用它收敛用户选中的路径）
    ///
    /// 走 `build_ctx` 而不是自己解析一遍：作用域的展开（`$HOME`）、规范化（软链）与
    /// 「无作用域 = 私有数据目录」这条默认规则都必须与 `host_fs_open` 完全一致。
    pub fn fs_roots(id: &str) -> Result<FsRoots, String> {
        let dir = plugin_dir(id)?;
        let manifest = parse_manifest(&dir)?;
        let ctx = build_ctx(id, &manifest)?;
        Ok(FsRoots {
            read: ctx
                .read_roots
                .iter()
                .map(|p| p.to_string_lossy().to_string())
                .collect(),
            write: ctx
                .write_roots
                .iter()
                .map(|p| p.to_string_lossy().to_string())
                .collect(),
            read_any: ctx.read_any,
            write_any: ctx.write_any,
            deny: ctx
                .deny_roots
                .iter()
                .map(|p| p.to_string_lossy().to_string())
                .collect(),
        })
    }

    // ==================== 宿主函数 ====================

    /// 取出能力上下文（`UserData` 内部的 Arc/Mutex 只用于跨线程共享，业务上只读）
    fn ctx_of(ud: &UserData<Arc<CapCtx>>) -> Result<Arc<CapCtx>, Error> {
        let guard = ud.get()?;
        let ctx = guard
            .lock()
            .map_err(|_| Error::msg("插件能力上下文已损坏"))?
            .clone();
        Ok(ctx)
    }

    /// 读一个字符串入参
    fn arg_str(plugin: &mut CurrentPlugin, inputs: &[Val], i: usize) -> Result<String, Error> {
        let v = inputs
            .get(i)
            .ok_or_else(|| Error::msg("宿主函数参数缺失"))?;
        plugin.memory_get_val::<String>(v)
    }

    /// 把字符串写回 WASM 内存作为返回值（空串 → offset 0，guest 侧读到空串）
    fn ret_str(plugin: &mut CurrentPlugin, outputs: &mut [Val], s: String) -> Result<(), Error> {
        if outputs.is_empty() {
            return Ok(());
        }
        plugin.memory_set_val(&mut outputs[0], s)
    }

    /// 读一个整数入参：guest 侧的 `i64` 会先编码成 8 字节，再把这些字节的指针传进来
    /// （PDK 的 host_fn 宏对所有参数都走「转成内存 → 传 offset」）
    fn arg_i64(plugin: &mut CurrentPlugin, inputs: &[Val], i: usize) -> Result<i64, Error> {
        let v = inputs
            .get(i)
            .ok_or_else(|| Error::msg("宿主函数参数缺失"))?;
        plugin.memory_get_val::<i64>(v)
    }

    /// 读一个字节块入参。空数据在 guest 侧表现为 offset 0，这里统一当空块
    /// （与字符串通道的「空串 ↔ offset 0」同一约定）
    fn arg_bytes(plugin: &mut CurrentPlugin, inputs: &[Val], i: usize) -> Result<Vec<u8>, Error> {
        let v = inputs
            .get(i)
            .ok_or_else(|| Error::msg("宿主函数参数缺失"))?;
        if v.i64().unwrap_or(0) == 0 {
            return Ok(Vec::new());
        }
        plugin.memory_get_val::<Vec<u8>>(v)
    }

    /// 把原始字节写回 WASM 内存作为返回值（空块 → offset 0，guest 侧读到空块）。
    ///
    /// 这是流式通道的**字节出口**：不经 base64，所以没有 4/3 的体积放大，也没有多余的一次拷贝。
    fn ret_bytes(plugin: &mut CurrentPlugin, outputs: &mut [Val], bytes: &[u8]) -> Result<(), Error> {
        if outputs.is_empty() {
            return Ok(());
        }
        plugin.memory_set_val(&mut outputs[0], bytes)
    }

    // ---- 按偏移读写（平台差异只出现在这几个函数里） ----

    /// 按偏移读，**不动文件游标**：Unix 用 `pread`，Windows 用 `seek_read`
    #[cfg(unix)]
    pub(super) fn read_at(file: &fs::File, buf: &mut [u8], off: u64) -> std::io::Result<usize> {
        use std::os::unix::fs::FileExt;
        file.read_at(buf, off)
    }

    #[cfg(windows)]
    pub(super) fn read_at(file: &fs::File, buf: &mut [u8], off: u64) -> std::io::Result<usize> {
        use std::os::windows::fs::FileExt;
        file.seek_read(buf, off)
    }

    /// 其它平台兜底：克隆句柄后 seek + read（克隆各自持有游标，不影响原句柄）
    #[cfg(not(any(unix, windows)))]
    pub(super) fn read_at(file: &fs::File, buf: &mut [u8], off: u64) -> std::io::Result<usize> {
        use std::io::{Read, Seek, SeekFrom};
        let mut g = file.try_clone()?;
        g.seek(SeekFrom::Start(off))?;
        g.read(buf)
    }

    /// 按偏移写，不动文件游标：Unix 用 `pwrite`，Windows 用 `seek_write`
    #[cfg(unix)]
    pub(super) fn write_at(file: &fs::File, buf: &[u8], off: u64) -> std::io::Result<usize> {
        use std::os::unix::fs::FileExt;
        file.write_at(buf, off)
    }

    #[cfg(windows)]
    pub(super) fn write_at(file: &fs::File, buf: &[u8], off: u64) -> std::io::Result<usize> {
        use std::os::windows::fs::FileExt;
        file.seek_write(buf, off)
    }

    #[cfg(not(any(unix, windows)))]
    pub(super) fn write_at(file: &fs::File, buf: &[u8], off: u64) -> std::io::Result<usize> {
        use std::io::{Seek, SeekFrom, Write};
        let mut g = file.try_clone()?;
        g.seek(SeekFrom::Start(off))?;
        g.write(buf)
    }

    /// 尽量读满一块；到文件末尾就返回**已读到的字节数**（短读是正常路径，不是错误）
    pub(super) fn read_full_at(file: &fs::File, buf: &mut [u8], off: u64) -> std::io::Result<usize> {
        let mut filled = 0usize;
        while filled < buf.len() {
            match read_at(file, &mut buf[filled..], off + filled as u64) {
                Ok(0) => break,
                Ok(k) => filled += k,
                Err(e) if e.kind() == std::io::ErrorKind::Interrupted => continue,
                Err(e) => return Err(e),
            }
        }
        Ok(filled)
    }

    /// 按偏移写满整块（短写要补齐，否则文件里会留空洞）
    pub(super) fn write_full_at(file: &fs::File, buf: &[u8], off: u64) -> std::io::Result<()> {
        let mut done = 0usize;
        while done < buf.len() {
            match write_at(file, &buf[done..], off + done as u64) {
                Ok(0) => {
                    return Err(std::io::Error::new(
                        std::io::ErrorKind::WriteZero,
                        "写入返回 0 字节",
                    ))
                }
                Ok(k) => done += k,
                Err(e) if e.kind() == std::io::ErrorKind::Interrupted => continue,
                Err(e) => return Err(e),
            }
        }
        Ok(())
    }

    fn host_log(
        plugin: &mut CurrentPlugin,
        inputs: &[Val],
        _outputs: &mut [Val],
        ud: UserData<Arc<CapCtx>>,
    ) -> Result<(), Error> {
        let ctx = ctx_of(&ud)?;
        ctx.require_log()?;
        let msg = arg_str(plugin, inputs, 0)?;
        println!("[Plugin:{}] {}", ctx.id, msg);
        Ok(())
    }

    fn host_info(
        plugin: &mut CurrentPlugin,
        _inputs: &[Val],
        outputs: &mut [Val],
        ud: UserData<Arc<CapCtx>>,
    ) -> Result<(), Error> {
        let ctx = ctx_of(&ud)?;
        let text = serde_json::json!({
            "id": ctx.id,
            "dataDir": ctx.data_dir.to_string_lossy(),
            "canLog": ctx.can_log,
            "canKv": ctx.can_kv,
            "readRoots": ctx.read_roots.iter().map(|p| p.to_string_lossy().to_string()).collect::<Vec<_>>(),
            "writeRoots": ctx.write_roots.iter().map(|p| p.to_string_lossy().to_string()).collect::<Vec<_>>(),
            // 通配作用域：readRoots 为空不代表「哪儿都读不了」，要看这两个标志
            "readAny": ctx.read_any,
            "writeAny": ctx.write_any,
            "denyRoots": ctx.deny_roots.iter().map(|p| p.to_string_lossy().to_string()).collect::<Vec<_>>(),
            // 线性内存上限：让插件能自己算「用了多少 / 上限多少」
            // （插件侧用 core::arch::wasm32::memory_size(0) * 64KiB 读自己的用量）
            "memoryLimitMb": (MAX_MEMORY_PAGES as u64 * 64) / 1024,
            "memoryLimitPages": MAX_MEMORY_PAGES,
        })
        .to_string();
        ret_str(plugin, outputs, text)
    }

    /// 读取**本插件**的设置值（`contributes.configuration` 声明的那些键）。
    ///
    /// **不需要任何权限**：它只能读自己清单里声明过的设置，不是能力授予 ——
    /// 给权限反而会制造「这个也需要授权」的假边界。
    ///
    /// 返回 JSON 对象：没存过的键回落到清单 default；类型/范围不符的旧值同样回落。
    fn host_plugin_settings(
        plugin: &mut CurrentPlugin,
        _inputs: &[Val],
        outputs: &mut [Val],
        ud: UserData<Arc<CapCtx>>,
    ) -> Result<(), Error> {
        let ctx = ctx_of(&ud)?;
        ret_str(plugin, outputs, plugin_settings_of(&ctx.id).to_string())
    }

    // ---- KV（命名空间 plugins_ext_kv/<id>.json，值为字符串） ----

    fn kv_file(id: &str) -> String {
        format!("plugins_ext_kv/{}.json", id)
    }

    fn host_kv_get(
        plugin: &mut CurrentPlugin,
        inputs: &[Val],
        outputs: &mut [Val],
        ud: UserData<Arc<CapCtx>>,
    ) -> Result<(), Error> {
        let ctx = ctx_of(&ud)?;
        ctx.require_kv()?;
        let key = arg_str(plugin, inputs, 0)?;
        check_kv_key(&key)?;
        let val = kv_store::kv_read_value(&kv_file(&ctx.id), &key)
            .and_then(|v| v.as_str().map(str::to_string))
            .unwrap_or_default();
        ret_str(plugin, outputs, val)
    }

    fn host_kv_set(
        plugin: &mut CurrentPlugin,
        inputs: &[Val],
        _outputs: &mut [Val],
        ud: UserData<Arc<CapCtx>>,
    ) -> Result<(), Error> {
        let ctx = ctx_of(&ud)?;
        ctx.require_kv()?;
        let key = arg_str(plugin, inputs, 0)?;
        let value = arg_str(plugin, inputs, 1)?;
        check_kv_key(&key)?;
        if value.len() > MAX_KV_VALUE_LEN {
            return Err(Error::msg(format!(
                "KV 值超过 {}KB 上限",
                MAX_KV_VALUE_LEN / 1024
            )));
        }
        let file = kv_file(&ctx.id);
        // 配额：只在「新增键」时检查总数（覆盖已有键不受限）
        if kv_store::kv_read_value(&file, &key).is_none() && kv_key_count(&file) >= MAX_KV_KEYS {
            return Err(Error::msg(format!("KV 键数量超过 {} 上限", MAX_KV_KEYS)));
        }
        let resp = kv_store::kv_set(file, key, serde_json::json!(value));
        if resp.code != 200 {
            return Err(Error::msg(format!("写入 KV 失败: {}", resp.msg)));
        }
        Ok(())
    }

    fn host_kv_remove(
        plugin: &mut CurrentPlugin,
        inputs: &[Val],
        _outputs: &mut [Val],
        ud: UserData<Arc<CapCtx>>,
    ) -> Result<(), Error> {
        let ctx = ctx_of(&ud)?;
        ctx.require_kv()?;
        let key = arg_str(plugin, inputs, 0)?;
        check_kv_key(&key)?;
        let file = kv_file(&ctx.id);
        let resp = kv_store::kv_delete(file, key);
        if resp.code != 200 {
            return Err(Error::msg(format!("删除 KV 失败: {}", resp.msg)));
        }
        Ok(())
    }

    /// 返回全部键名（JSON 数组字符串）
    fn host_kv_keys(
        plugin: &mut CurrentPlugin,
        _inputs: &[Val],
        outputs: &mut [Val],
        ud: UserData<Arc<CapCtx>>,
    ) -> Result<(), Error> {
        let ctx = ctx_of(&ud)?;
        ctx.require_kv()?;
        let mut keys: Vec<String> = kv_store::kv_entries(kv_file(&ctx.id))
            .data
            .into_iter()
            .map(|(k, _)| k)
            .collect();
        keys.sort();
        ret_str(plugin, outputs, serde_json::json!(keys).to_string())
    }

    /// 当前键数量（读一次 KV 文件；写操作前用于配额判断）
    fn kv_key_count(file: &str) -> usize {
        kv_store::kv_entries(file.to_string()).data.len()
    }

    fn check_kv_key(key: &str) -> Result<(), Error> {
        if key.is_empty() || key.len() > MAX_KV_KEY_LEN || key.contains('\0') {
            return Err(Error::msg("KV 键名非法（空 / 过长）"));
        }
        Ok(())
    }

    // ---- 文件系统（限定在已授权作用域内） ----

    fn host_fs_read(
        plugin: &mut CurrentPlugin,
        inputs: &[Val],
        outputs: &mut [Val],
        ud: UserData<Arc<CapCtx>>,
    ) -> Result<(), Error> {
        let ctx = ctx_of(&ud)?;
        let req = arg_str(plugin, inputs, 0)?;
        let path = ctx.read_path(&req)?;
        let Ok(meta) = fs::metadata(&path) else {
            // 文件不存在 → 返回空串（用 host_fs_stat 区分「空文件」与「不存在」）
            return ret_str(plugin, outputs, String::new());
        };
        if meta.is_dir() {
            return Err(Error::msg("目标是目录，请用 host_fs_list"));
        }
        if meta.len() > MAX_FS_READ {
            return Err(Error::msg(format!(
                "文件超过 {}MB 上限",
                MAX_FS_READ / 1024 / 1024
            )));
        }
        let bytes = fs::read(&path).map_err(|e| Error::msg(format!("读取失败: {}", e)))?;
        let text = String::from_utf8(bytes).map_err(|_| {
            Error::msg("文件不是 UTF-8 文本（v1 的 fs 通道只传文本，二进制请自行 base64）")
        })?;
        ret_str(plugin, outputs, text)
    }

    fn host_fs_write(
        plugin: &mut CurrentPlugin,
        inputs: &[Val],
        _outputs: &mut [Val],
        ud: UserData<Arc<CapCtx>>,
    ) -> Result<(), Error> {
        let ctx = ctx_of(&ud)?;
        let req = arg_str(plugin, inputs, 0)?;
        let content = arg_str(plugin, inputs, 1)?;
        if content.len() > MAX_FS_WRITE {
            return Err(Error::msg(format!(
                "写入内容超过 {}MB 上限",
                MAX_FS_WRITE / 1024 / 1024
            )));
        }
        let path = ctx.write_path(&req)?;
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).map_err(|e| Error::msg(format!("创建目录失败: {}", e)))?;
        }
        fs::write(&path, content).map_err(|e| Error::msg(format!("写入失败: {}", e)))?;
        Ok(())
    }

    fn host_fs_mkdir(
        plugin: &mut CurrentPlugin,
        inputs: &[Val],
        _outputs: &mut [Val],
        ud: UserData<Arc<CapCtx>>,
    ) -> Result<(), Error> {
        let ctx = ctx_of(&ud)?;
        let req = arg_str(plugin, inputs, 0)?;
        let path = ctx.write_path(&req)?;
        fs::create_dir_all(&path).map_err(|e| Error::msg(format!("创建目录失败: {}", e)))?;
        Ok(())
    }

    fn host_fs_remove(
        plugin: &mut CurrentPlugin,
        inputs: &[Val],
        _outputs: &mut [Val],
        ud: UserData<Arc<CapCtx>>,
    ) -> Result<(), Error> {
        let ctx = ctx_of(&ud)?;
        let req = arg_str(plugin, inputs, 0)?;
        let path = ctx.write_path(&req)?;
        let Ok(meta) = fs::metadata(&path) else {
            // 不存在视为已删除（幂等）
            return Ok(());
        };
        if meta.is_dir() {
            fs::remove_dir(&path).map_err(|e| Error::msg(format!("删除目录失败（仅限空目录）: {}", e)))
        } else {
            fs::remove_file(&path).map_err(|e| Error::msg(format!("删除文件失败: {}", e)))
        }
    }

    /// 查询路径元信息（JSON：`{exists, dir, size}`）
    fn host_fs_stat(
        plugin: &mut CurrentPlugin,
        inputs: &[Val],
        outputs: &mut [Val],
        ud: UserData<Arc<CapCtx>>,
    ) -> Result<(), Error> {
        let ctx = ctx_of(&ud)?;
        let req = arg_str(plugin, inputs, 0)?;
        let path = ctx.any_path(&req)?;
        let text = match fs::metadata(&path) {
            Ok(m) => serde_json::json!({
                "exists": true,
                "dir": m.is_dir(),
                "size": m.len(),
            }),
            Err(_) => serde_json::json!({ "exists": false, "dir": false, "size": 0 }),
        };
        ret_str(plugin, outputs, text.to_string())
    }

    /// 列目录（JSON 数组：`[{name, dir, size}]`）
    fn host_fs_list(
        plugin: &mut CurrentPlugin,
        inputs: &[Val],
        outputs: &mut [Val],
        ud: UserData<Arc<CapCtx>>,
    ) -> Result<(), Error> {
        let ctx = ctx_of(&ud)?;
        let req = arg_str(plugin, inputs, 0)?;
        let path = ctx.read_path(&req)?;
        let entries = match fs::read_dir(&path) {
            Ok(e) => e,
            // 目录不存在 → 空列表（用 host_fs_stat 区分）
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                return ret_str(plugin, outputs, "[]".to_string());
            }
            Err(e) => return Err(Error::msg(format!("列目录失败: {}", e))),
        };
        let mut items: Vec<serde_json::Value> = Vec::new();
        for entry in entries.flatten() {
            let meta = entry.metadata().ok();
            items.push(serde_json::json!({
                "name": entry.file_name().to_string_lossy(),
                "dir": meta.as_ref().map(|m| m.is_dir()).unwrap_or(false),
                "size": meta.as_ref().map(|m| m.len()).unwrap_or(0),
            }));
        }
        items.sort_by(|a, b| {
            let na = a.get("name").and_then(|v| v.as_str()).unwrap_or_default();
            let nb = b.get("name").and_then(|v| v.as_str()).unwrap_or_default();
            na.cmp(nb)
        });
        ret_str(plugin, outputs, serde_json::json!(items).to_string())
    }

    // ---- FS 流式通道（大文件：文件留在宿主，WASM 每次只拿一块） ----

    /// 打开流式句柄：`(path, mode)` → `{"handle":N}`
    ///
    /// mode：`r` 只读（必须存在）/ `w` 写（截断或新建）/ `a` 追加（新建）/ `rw` 读写（不截断，新建）
    fn host_fs_open(
        plugin: &mut CurrentPlugin,
        inputs: &[Val],
        outputs: &mut [Val],
        ud: UserData<Arc<CapCtx>>,
    ) -> Result<(), Error> {
        let ctx = ctx_of(&ud)?;
        let req = arg_str(plugin, inputs, 0)?;
        let mode = arg_str(plugin, inputs, 1)?;
        let handle = ctx.fs_open(&req, mode.trim())?;
        ret_str(plugin, outputs, serde_json::json!({ "handle": handle }).to_string())
    }

    /// 句柄当前长度（字节，十进制字符串）。**每次现查**，所以能反映别的写入者造成的长度变化
    fn host_fs_size(
        plugin: &mut CurrentPlugin,
        inputs: &[Val],
        outputs: &mut [Val],
        ud: UserData<Arc<CapCtx>>,
    ) -> Result<(), Error> {
        let ctx = ctx_of(&ud)?;
        let io = ctx.fs_take(arg_i64(plugin, inputs, 0)?, Access::Any)?;
        let len = io
            .file
            .metadata()
            .map_err(|e| Error::msg(format!("查询文件长度失败（{}）: {}", io.path.display(), e)))?
            .len();
        ret_str(plugin, outputs, len.to_string())
    }

    /// 按偏移读一块：`(handle, offset, len)` → **原始字节**（不走 base64）
    ///
    /// 到末尾（或越过末尾）时返回不足一块、甚至空块，**不报错**：循环条件请用
    /// `host_fs_size` 判断，别拿异常当 EOF。
    fn host_fs_read_at(
        plugin: &mut CurrentPlugin,
        inputs: &[Val],
        outputs: &mut [Val],
        ud: UserData<Arc<CapCtx>>,
    ) -> Result<(), Error> {
        let ctx = ctx_of(&ud)?;
        let handle = arg_i64(plugin, inputs, 0)?;
        let offset = arg_i64(plugin, inputs, 1)?;
        let len = arg_i64(plugin, inputs, 2)?;
        if offset < 0 || len < 0 {
            return Err(Error::msg("offset / len 不能为负"));
        }
        let len = len as u64;
        if len > MAX_FS_CHUNK {
            return Err(Error::msg(format!(
                "单块读取超过 {}KB 上限（请分块读）",
                MAX_FS_CHUNK / 1024
            )));
        }
        let io = ctx.fs_take(handle, Access::Read)?;
        if len == 0 {
            return ret_bytes(plugin, outputs, &[]);
        }
        let mut buf = vec![0u8; len as usize];
        let filled = read_full_at(&io.file, &mut buf, offset as u64)
            .map_err(|e| Error::msg(format!("读取 {} 失败: {}", io.path.display(), e)))?;
        buf.truncate(filled);
        ret_bytes(plugin, outputs, &buf)
    }

    /// 按偏移写一块：`(handle, offset, data)`（超出末尾会扩展文件，不动文件游标）
    ///
    /// 只接受 `w` / `rw` 模式：`a`（O_APPEND）句柄上的「按偏移写」在各平台上行为不一致，
    /// 混用会静默写到文件末尾，所以这里直接拒绝。
    fn host_fs_write_at(
        plugin: &mut CurrentPlugin,
        inputs: &[Val],
        _outputs: &mut [Val],
        ud: UserData<Arc<CapCtx>>,
    ) -> Result<(), Error> {
        let ctx = ctx_of(&ud)?;
        let handle = arg_i64(plugin, inputs, 0)?;
        let offset = arg_i64(plugin, inputs, 1)?;
        let data = arg_bytes(plugin, inputs, 2)?;
        if offset < 0 {
            return Err(Error::msg("offset 不能为负"));
        }
        if data.len() as u64 > MAX_FS_CHUNK {
            return Err(Error::msg(format!(
                "单块写入超过 {}KB 上限（请分块写）",
                MAX_FS_CHUNK / 1024
            )));
        }
        let io = ctx.fs_take(handle, Access::Write)?;
        if io.append {
            return Err(Error::msg(
                "该句柄是 a（追加）模式：请用 host_fs_append，或改用 w / rw 模式后按偏移写",
            ));
        }
        write_full_at(&io.file, &data, offset as u64)
            .map_err(|e| Error::msg(format!("写入 {} 失败: {}", io.path.display(), e)))
    }

    /// 追加一块到文件末尾：`(handle, data)`（只接受 `a` 模式句柄）
    fn host_fs_append(
        plugin: &mut CurrentPlugin,
        inputs: &[Val],
        _outputs: &mut [Val],
        ud: UserData<Arc<CapCtx>>,
    ) -> Result<(), Error> {
        use std::io::Write;

        let ctx = ctx_of(&ud)?;
        let handle = arg_i64(plugin, inputs, 0)?;
        let data = arg_bytes(plugin, inputs, 1)?;
        if data.len() as u64 > MAX_FS_CHUNK {
            return Err(Error::msg(format!(
                "单块写入超过 {}KB 上限（请分块写）",
                MAX_FS_CHUNK / 1024
            )));
        }
        let io = ctx.fs_take(handle, Access::Write)?;
        if !io.append {
            return Err(Error::msg(
                "该句柄不是 a（追加）模式：追加请用 host_fs_open 的 a 模式，按偏移写请用 host_fs_write_at",
            ));
        }
        // O_APPEND 句柄上的 write 一定落在文件末尾，所以这里不传偏移（也就没有竞态）
        let mut w = &io.file;
        w.write_all(&data)
            .map_err(|e| Error::msg(format!("追加 {} 失败: {}", io.path.display(), e)))
    }

    /// 把文件截断到指定长度：`(handle, size)`
    ///
    /// 用「按偏移覆盖 + 收尾截断」重写一个更短的文件时收尾用；只写新内容（不覆盖旧文件）时
    /// 直接用 `w` 模式打开即可，不需要它。
    fn host_fs_truncate(
        plugin: &mut CurrentPlugin,
        inputs: &[Val],
        _outputs: &mut [Val],
        ud: UserData<Arc<CapCtx>>,
    ) -> Result<(), Error> {
        let ctx = ctx_of(&ud)?;
        let handle = arg_i64(plugin, inputs, 0)?;
        let size = arg_i64(plugin, inputs, 1)?;
        if size < 0 {
            return Err(Error::msg("size 不能为负"));
        }
        let io = ctx.fs_take(handle, Access::Write)?;
        io.file
            .set_len(size as u64)
            .map_err(|e| Error::msg(format!("截断 {} 失败: {}", io.path.display(), e)))
    }

    /// 关闭句柄（幂等）
    fn host_fs_close(
        plugin: &mut CurrentPlugin,
        inputs: &[Val],
        _outputs: &mut [Val],
        ud: UserData<Arc<CapCtx>>,
    ) -> Result<(), Error> {
        let ctx = ctx_of(&ud)?;
        ctx.fs_close(arg_i64(plugin, inputs, 0)?)
    }

    // ---- 原始 socket（net: 权限；连接按插件给的键保存在宿主侧，跨调用有效） ----

    /// 建立一条原始 TCP 连接：`(key, host, port)` → `{"ok":true,"peer":"host:port"}`
    ///
    /// `key` 由插件自定义（通常是它自己的连接 id），之后 `host_tcp_send` / `host_tcp_recv` /
    /// `host_tcp_close` 都用它寻址。**权限只在建立时校验一次**，之后按 key 收发不再重复校验——
    /// 与 FS 句柄「打开时授权、句柄即凭证」同一套思路。同键重连会静默关掉旧连接。
    fn host_tcp_connect(
        plugin: &mut CurrentPlugin,
        inputs: &[Val],
        outputs: &mut [Val],
        ud: UserData<Arc<CapCtx>>,
    ) -> Result<(), Error> {
        use std::net::{TcpStream, ToSocketAddrs};
        use std::time::Duration;

        let ctx = ctx_of(&ud)?;
        let key = arg_str(plugin, inputs, 0)?;
        let host = arg_str(plugin, inputs, 1)?;
        let port_i = arg_i64(plugin, inputs, 2)?;

        if key.is_empty() || key.len() > MAX_PATH_LEN {
            return Err(Error::msg("TCP 连接键非法（空 / 过长）"));
        }
        if host.is_empty() || host.len() > MAX_PATH_LEN {
            return Err(Error::msg("TCP host 非法（空 / 过长）"));
        }
        let port = u16::try_from(port_i)
            .map_err(|_| Error::msg(format!("TCP 端口非法（应为 0-65535）: {}", port_i)))?;
        // 只做「清单声明一致性」校验：插件连到它自己没声明的目标时快速失败。
        // 这里**刻意不做地址黑名单**（不拦回环 / 私有网段 / 云元数据）—— 信任模型是「安装即信任」，
        // 权限不是安全边界。别在这儿加黑名单来假装有一道边界，见模块文档「信任模型」一节。
        if !ctx.net_permitted(&host, port) {
            return Err(Error::msg(format!(
                "无网络访问权限（{}:{} 未被任何 net: 声明覆盖）",
                host, port
            )));
        }

        let addrs: Vec<_> = (host.as_str(), port)
            .to_socket_addrs()
            .map_err(|e| Error::msg(format!("解析 {}:{} 失败: {}", host, port, e)))?
            .collect();
        if addrs.is_empty() {
            return Err(Error::msg(format!("解析 {}:{} 得不到任何地址", host, port)));
        }
        let mut last: Option<std::io::Error> = None;
        let mut connected = None;
        for addr in addrs {
            match TcpStream::connect_timeout(&addr, Duration::from_millis(TCP_CONNECT_TIMEOUT_MS)) {
                Ok(s) => {
                    connected = Some(s);
                    break;
                }
                Err(e) => last = Some(e),
            }
        }
        let stream = connected.ok_or_else(|| {
            Error::msg(format!(
                "连接 {}:{} 失败: {}",
                host,
                port,
                last.map(|e| e.to_string()).unwrap_or_default()
            ))
        })?;
        stream.set_read_timeout(Some(Duration::from_millis(TCP_RW_TIMEOUT_MS))).ok();
        stream.set_write_timeout(Some(Duration::from_millis(TCP_RW_TIMEOUT_MS))).ok();
        stream.set_nodelay(true).ok();

        let mut table = ctx.tcp.lock().map_err(|_| Error::msg("插件 TCP 连接表已损坏"))?;
        if !table.conns.contains_key(&key) && table.conns.len() >= MAX_TCP_CONNS {
            return Err(Error::msg(format!(
                "同时打开的 TCP 连接数超过 {} 上限（请先 host_tcp_close）",
                MAX_TCP_CONNS
            )));
        }
        // 同键重连：旧连接随 insert 被析构关闭
        table.conns.insert(key, stream);
        ret_str(
            plugin,
            outputs,
            serde_json::json!({ "ok": true, "peer": format!("{}:{}", host, port) }).to_string(),
        )
    }

    /// 往连接写字节：`(key, data)`（一次调用可发多块；协议由插件自己拼）
    fn host_tcp_send(
        plugin: &mut CurrentPlugin,
        inputs: &[Val],
        _outputs: &mut [Val],
        ud: UserData<Arc<CapCtx>>,
    ) -> Result<(), Error> {
        use std::io::Write;

        let ctx = ctx_of(&ud)?;
        let key = arg_str(plugin, inputs, 0)?;
        let data = arg_bytes(plugin, inputs, 1)?;
        if data.len() as u64 > MAX_TCP_CHUNK {
            return Err(Error::msg(format!(
                "单次 TCP 写入超过 {}KB 上限（请分块写）",
                MAX_TCP_CHUNK / 1024
            )));
        }
        let mut stream = ctx.tcp_take(&key)?;
        stream
            .write_all(&data)
            .map_err(|e| Error::msg(format!("TCP 写入失败: {}", e)))
    }

    /// 从连接读一块：`(key, maxLen)` → **原始字节**（不走 base64）
    ///
    /// 返回「本次内核缓冲区里能拿到的、最多 maxLen 字节」，**可能不足 maxLen**，空块 = 对端关闭。
    /// 调用方要按协议自行累积 —— 这正是「插件自己实现协议」的含义。
    fn host_tcp_recv(
        plugin: &mut CurrentPlugin,
        inputs: &[Val],
        outputs: &mut [Val],
        ud: UserData<Arc<CapCtx>>,
    ) -> Result<(), Error> {
        use std::io::Read;

        let ctx = ctx_of(&ud)?;
        let key = arg_str(plugin, inputs, 0)?;
        let max = arg_i64(plugin, inputs, 1)?;
        if max <= 0 || max as u64 > MAX_TCP_CHUNK {
            return Err(Error::msg(format!(
                "单次 TCP 读取长度非法（1..={}）: {}",
                MAX_TCP_CHUNK, max
            )));
        }
        let mut stream = ctx.tcp_take(&key)?;
        let mut buf = vec![0u8; max as usize];
        match stream.read(&mut buf) {
            Ok(n) => {
                buf.truncate(n); // n == 0 = 对端关闭（短读是正常路径）
                ret_bytes(plugin, outputs, &buf)
            }
            Err(e)
                if e.kind() == std::io::ErrorKind::WouldBlock
                    || e.kind() == std::io::ErrorKind::TimedOut =>
            {
                Err(Error::msg(format!("TCP 读取超时（{}ms）", TCP_RW_TIMEOUT_MS)))
            }
            Err(e) => Err(Error::msg(format!("TCP 读取失败: {}", e))),
        }
    }

    /// 关闭并移除连接（**幂等**：不存在的键视为已关闭）
    fn host_tcp_close(
        plugin: &mut CurrentPlugin,
        inputs: &[Val],
        _outputs: &mut [Val],
        ud: UserData<Arc<CapCtx>>,
    ) -> Result<(), Error> {
        let ctx = ctx_of(&ud)?;
        let key = arg_str(plugin, inputs, 0)?;
        ctx.tcp
            .lock()
            .map_err(|_| Error::msg("插件 TCP 连接表已损坏"))?
            .conns
            .remove(&key);
        Ok(())
    }

    // ==================== 编译 / 调用 ====================

    /// 包目录指纹：plugin.json 与 wasm 的 (长度, mtime)，外加目录 mtime。
    /// 任一变化 → 重编译。
    fn fingerprint(dir: &Path, manifest_path: &Path, wasm_path: &Path) -> u64 {
        fn stamp(p: &Path) -> (u64, u64) {
            let Ok(meta) = fs::metadata(p) else {
                return (0, 0);
            };
            let mtime = meta
                .modified()
                .ok()
                .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
                .map(|d| d.as_nanos() as u64)
                .unwrap_or(0);
            (meta.len(), mtime)
        }
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        stamp(manifest_path).hash(&mut hasher);
        stamp(wasm_path).hash(&mut hasher);
        stamp(dir).hash(&mut hasher);
        hasher.finish()
    }

    /// 构造本次运行时的宿主函数表（每个函数自带能力上下文）
    fn build_functions(ctx: &Arc<CapCtx>) -> Vec<Function> {
        fn make(
            name: &str,
            params: usize,
            results: usize,
            ctx: &Arc<CapCtx>,
            f: fn(
                &mut CurrentPlugin,
                &[Val],
                &mut [Val],
                UserData<Arc<CapCtx>>,
            ) -> Result<(), Error>,
        ) -> Function {
            Function::new(
                name,
                vec![PTR; params],
                vec![PTR; results],
                UserData::new(ctx.clone()),
                f,
            )
            .with_namespace(EXTISM_USER_MODULE)
        }

        vec![
            make("host_log", 1, 0, ctx, host_log),
            make("host_info", 0, 1, ctx, host_info),
            make("host_plugin_settings", 0, 1, ctx, host_plugin_settings),
            make("host_kv_get", 1, 1, ctx, host_kv_get),
            make("host_kv_set", 2, 0, ctx, host_kv_set),
            make("host_kv_remove", 1, 0, ctx, host_kv_remove),
            make("host_kv_keys", 0, 1, ctx, host_kv_keys),
            make("host_fs_read", 1, 1, ctx, host_fs_read),
            make("host_fs_write", 2, 0, ctx, host_fs_write),
            make("host_fs_mkdir", 1, 0, ctx, host_fs_mkdir),
            make("host_fs_remove", 1, 0, ctx, host_fs_remove),
            make("host_fs_list", 1, 1, ctx, host_fs_list),
            make("host_fs_stat", 1, 1, ctx, host_fs_stat),
            // 流式通道（大文件）：文件留在宿主，WASM 每次只拿一块
            make("host_fs_open", 2, 1, ctx, host_fs_open),
            make("host_fs_size", 1, 1, ctx, host_fs_size),
            make("host_fs_read_at", 3, 1, ctx, host_fs_read_at),
            make("host_fs_write_at", 3, 0, ctx, host_fs_write_at),
            make("host_fs_append", 2, 0, ctx, host_fs_append),
            make("host_fs_truncate", 2, 0, ctx, host_fs_truncate),
            make("host_fs_close", 1, 0, ctx, host_fs_close),
            // 原始 socket（net: 权限）：连接持久在宿主侧，插件自己实现线协议
            make("host_tcp_connect", 3, 1, ctx, host_tcp_connect),
            make("host_tcp_send", 2, 0, ctx, host_tcp_send),
            make("host_tcp_recv", 2, 1, ctx, host_tcp_recv),
            make("host_tcp_close", 1, 0, ctx, host_tcp_close),
        ]
    }

    /// 取编译产物（命中缓存直接返回；未命中或指纹失配则编译）
    fn compiled_for(
        id: &str,
        manifest: &PluginManifest,
        wasm_path: &Path,
        fp: u64,
    ) -> Result<CompiledPlugin, String> {
        // 出错快照是**编译期**选项（`PluginBuilder::with_memdump`），所以把开关折进指纹：
        // 切换开关后指纹失配 → 下一次调用自动重编，不需要手工清缓存
        let memdump = memdump_enabled();
        let fp = fp ^ if memdump { MEMDUMP_FP_SALT } else { 0 };

        {
            let reg = registry()?;
            if let Some((cached_fp, cached)) = reg.get(id) {
                if *cached_fp == fp {
                    return Ok(cached.clone());
                }
            }
        }

        let ctx = build_ctx(id, manifest)?;
        let functions = build_functions(&ctx);

        let mut mf = Manifest::new([Wasm::file(wasm_path)]);
        mf.memory.max_pages = Some(MAX_MEMORY_PAGES);
        mf.memory.max_http_response_bytes = Some(MAX_HTTP_RESPONSE);
        // 关闭 Extism 自带的 var 存储（0 = 禁用）；持久状态统一走我们的 KV
        mf.memory.max_var_bytes = Some(0);
        mf.timeout_ms = Some(CALL_TIMEOUT_MS);
        let hosts = http_allowed_hosts(&manifest.permissions);
        mf.allowed_hosts = if hosts.is_empty() { None } else { Some(hosts) };

        let mut builder = PluginBuilder::new(mf)
            // 打开 WASI 以兼容 `wasm32-wasip1` 构建（不预开放任何目录，allowed_paths 为空）；
            // `wasm32-unknown-unknown` + extism-pdk 的插件同样可用
            .with_wasi(true)
            .with_fuel_limit(CALL_FUEL)
            .with_functions(functions)
            .with_http_response_headers(false);

        if memdump {
            // 出错时 Extism 会把**整块线性内存**写进这个文件 ⇒ 文件大小就是出错那刻的线性内存字节数，
            // 是拿不到 Store 的情况下唯一能「事后看到插件涨到多大」的手段（见 CallStats）
            let dump_dir = plugin_data_ext_dir(id)?;
            fs::create_dir_all(&dump_dir).map_err(|e| format!("创建插件数据目录失败: {}", e))?;
            builder = builder.with_memdump(dump_dir.join(MEMDUMP_FILE));
        }

        let compiled = builder
            .compile()
            .map_err(|e| format!("编译插件 WASM 失败: {}", e))?;

        let mut reg = registry()?;
        reg.insert(id.to_string(), (fp, compiled.clone()));
        Ok(compiled)
    }

    /// 解析插件包里的 wasm 路径并做存在性检查
    fn wasm_path_of(dir: &Path, manifest: &PluginManifest) -> Result<PathBuf, String> {
        let rel = manifest
            .wasm
            .as_deref()
            .ok_or_else(|| "插件清单未声明 wasm".to_string())?;
        let path = dir.join(rel);
        if !path.is_file() {
            return Err(format!("插件 WASM 文件不存在: {}", rel));
        }
        Ok(path)
    }

    /// 安装期预检：能编译、能实例化（缺导入 / 非 Extism PDK 模块都会在这里暴露）
    pub fn probe(id: &str, dir: &Path, manifest: &PluginManifest) -> Result<(), String> {
        let wasm_path = wasm_path_of(dir, manifest)?;
        let fp = fingerprint(dir, &dir.join("plugin.json"), &wasm_path);
        let compiled = compiled_for(id, manifest, &wasm_path, fp)?;
        Plugin::new_from_compiled(&compiled)
            .map(|_| ())
            .map_err(|e| format!("插件 WASM 无法实例化（缺少 Extism PDK 导入？）: {}", e))
    }

    /// 把 Extism 的裸错误翻成可操作的中文
    ///
    /// `oom` / `timeout` 不是普通报错，而是 Extism 撞到我们设的限额时给出的**根因字符串**：
    /// 直接透传会让插件页只看到「插件执行失败: oom」，完全看不出下一步该干什么。
    /// （`pub(super)` 只为了单元测试能直接验证文案）
    pub(super) fn explain(e: &Error, what: &str) -> String {
        match e.root_cause().to_string().as_str() {
            "oom" => format!(
                "{}失败：超出线性内存上限 {}MB。大文件请改用流式通道（host_fs_open + host_fs_read_at / host_fs_write_at / host_fs_append），或把任务拆小",
                what,
                (MAX_MEMORY_PAGES as u64 * 64) / 1024
            ),
            "timeout" => format!(
                "{}失败：超过单次调用上限 {}s。长任务请做成可续跑——把游标回传页面，页面接着再调一次",
                what,
                CALL_TIMEOUT_MS / 1000
            ),
            _ => format!("{}失败: {}", what, e),
        }
    }

    /// 插件数据目录里的出错快照（仅在开启过快照、且真的出错过时存在）
    ///
    /// 文件大小 == 出错那一刻的插件线性内存字节数（Extism 写的是整块线性内存）。
    fn memdump_info(id: &str) -> (Option<String>, Option<u64>) {
        let Ok(dir) = plugin_data_ext_dir(id) else {
            return (None, None);
        };
        let path = dir.join(MEMDUMP_FILE);
        match fs::metadata(&path) {
            Ok(m) if m.is_file() => (Some(path.to_string_lossy().to_string()), Some(m.len())),
            _ => (None, None),
        }
    }

    /// 记一次调用的资源占用（成功与失败都记——失败时的数字同样是排查素材）
    ///
    /// `before` / `after` 是调用前后的进程内存；差值按「负增量记 0」处理（宿主的释放会把它压成负）。
    fn record_stats(
        id: &str,
        func: &str,
        ok: bool,
        before: Option<ProcMem>,
        after: Option<ProcMem>,
        fuel_used: Option<u64>,
    ) {
        let delta = before.zip(after).map(|(b, a)| ProcMem {
            commit: a.commit.saturating_sub(b.commit),
            working_set: a.working_set.saturating_sub(b.working_set),
        });
        let (memdump_path, memdump_bytes) = memdump_info(id);
        // 必须在 `circuit_record` **之后**读：这里想展示的是「算上本次」的状态 ——
        // 第 3 次故障时应该立刻看到「已跳闸」，而不是下一次调用才看到
        let (consec_failures, circuit_cooldown_ms) = circuit_snapshot(id);
        let stats = CallStats {
            plugin_id: id.to_string(),
            func: func.to_string(),
            ok,
            fuel_used,
            mem_delta_bytes: delta.map(|d| d.commit),
            rss_delta_bytes: delta.map(|d| d.working_set),
            proc_commit_bytes: after.map(|a| a.commit),
            memdump_path,
            memdump_bytes,
            consec_failures,
            circuit_cooldown_ms,
        };
        if let Ok(mut m) = LAST_STATS.lock() {
            m.insert(id.to_string(), stats);
        }
    }

    // ==================== 并发额度与熔断（故障隔离） ====================
    //
    // 两道闸门各管一类「一个插件生病，整个宿主跟着遭殃」：
    //   * 并发额度 —— 限制同时在跑的实例数：每次调用要实例化一块最多 32MB 的线性内存并占住一个
    //     阻塞线程，不封顶的话几个面板并发请求就能把线程池和内存一起吃光（外部反馈症状是「整
    //     个应用变卡」，而不是「这个插件失败」）；
    //   * 熔断 —— 反复失败的插件（wasm 损坏、死循环撞超时）在冷却期内**直接被拒**，不重编译、
    //     不实例化、不占线程。这是唯一一条能让「一个插件坏了」变成「所有插件都变慢」的路径。
    //
    // 都用 std 同步原语：`call()` 是同步函数（跑在 spawn_blocking 线程里），不能用 tokio 的
    // async 信号量（`acquire().await` 在这里没有可以 await 的地方）。

    /// 并发额度闸：`MAX_CONCURRENT_CALLS` 个额度，超出后有上限地排队
    struct Slots {
        active: Mutex<usize>,
        free: Condvar,
    }

    impl Slots {
        const fn new() -> Self {
            Self { active: Mutex::new(0), free: Condvar::new() }
        }

        /// 取一个额度，最多等 `timeout`；拿不到返回 `None`
        fn acquire(&self, timeout: Duration) -> Option<SlotGuard<'_>> {
            let deadline = Instant::now() + timeout;
            let mut active = self.lock();
            loop {
                if *active < MAX_CONCURRENT_CALLS {
                    *active += 1;
                    return Some(SlotGuard(self));
                }
                let left = deadline.saturating_duration_since(Instant::now());
                if left.is_zero() {
                    return None;
                }
                // 被唤醒或超时都回到循环顶部重新判定：醒来那一刻额度可能又被别人抢走了
                let (guard, _) = self
                    .free
                    .wait_timeout(active, left)
                    .unwrap_or_else(|e| e.into_inner());
                active = guard;
            }
        }

        /// 中毒恢复：持锁期间 panic 只会让计数偏差，**不该让所有插件从此都调不动**
        /// （互斥语义由 `Mutex` 本身保证，这里承受得起状态略有偏差）
        fn lock(&self) -> MutexGuard<'_, usize> {
            self.active.lock().unwrap_or_else(|e| e.into_inner())
        }

        fn release(&self) {
            let mut active = self.lock();
            *active = active.saturating_sub(1);
            self.free.notify_one();
        }
    }

    /// 额度句柄：离开作用域即归还（panic 时 `Drop` 同样会跑，不会漏还）
    struct SlotGuard<'a>(&'a Slots);

    impl Drop for SlotGuard<'_> {
        fn drop(&mut self) {
            self.0.release();
        }
    }

    static CALL_SLOTS: Slots = Slots::new();

    /// 单个插件的熔断状态
    #[derive(Debug, Clone, Default)]
    struct Circuit {
        /// 连续宿主级故障次数
        consec: u32,
        /// 跳闸截止时刻（`None` = 没跳闸）
        open_until: Option<Instant>,
    }

    impl Circuit {
        /// 放行判定；`Err(剩余冷却毫秒)` = 仍在冷却期内
        fn check(&mut self, now: Instant) -> Result<(), u64> {
            match self.open_until {
                Some(until) if until > now => Err((until - now).as_millis() as u64),
                // 冷却自然结束 → 清零并放行：这一次调用就是探针，再失败会重新计数
                Some(_) => {
                    self.open_until = None;
                    self.consec = 0;
                    Ok(())
                }
                None => Ok(()),
            }
        }

        /// 记一次调用结果；成功（非故障）立刻复位
        fn record(&mut self, now: Instant, fault: bool) {
            if !fault {
                self.consec = 0;
                self.open_until = None;
                return;
            }
            self.consec = self.consec.saturating_add(1);
            if self.consec >= BREAKER_THRESHOLD {
                self.open_until = Some(now + Duration::from_millis(BREAKER_COOLDOWN_MS));
            }
        }
    }

    static CIRCUITS: LazyLock<Mutex<HashMap<String, Circuit>>> =
        LazyLock::new(|| Mutex::new(HashMap::new()));

    fn circuits() -> MutexGuard<'static, HashMap<String, Circuit>> {
        CIRCUITS.lock().unwrap_or_else(|e| e.into_inner())
    }

    /// 剩余冷却毫秒；`None` = 可以调用（顺手清掉已过期的冷却）
    fn circuit_remaining(id: &str) -> Option<u64> {
        circuits().get_mut(id).and_then(|c| c.check(Instant::now()).err())
    }

    /// 记一次调用是否宿主级故障
    fn circuit_record(id: &str, fault: bool) {
        circuits().entry(id.to_string()).or_default().record(Instant::now(), fault);
    }

    /// 当前故障计数与剩余冷却，给 `CallStats` 用（设置页据此回答「为什么点不动」）
    fn circuit_snapshot(id: &str) -> (u32, u64) {
        match circuits().get(id) {
            Some(c) => (
                c.consec,
                c.open_until
                    .and_then(|u| u.checked_duration_since(Instant::now()))
                    .map(|d| d.as_millis() as u64)
                    .unwrap_or(0),
            ),
            None => (0, 0),
        }
    }

    /// 哪些错误算「宿主级故障」
    ///
    /// 只认**重试也没用**的两类：`oom` / `timeout`（撞到资源限额，是插件的结构性问题），
    /// 以及编译 / 实例化失败（不经过这里，调用点直接传 `true`）。
    ///
    /// 插件自己 `return Err` 不算 —— 那可能只是这次的输入不对，重试是有价值的；
    /// 把它算成故障会让用户看到「点两次就失灵」，比没有熔断更难解释。
    fn is_host_fault(error: &Error) -> bool {
        is_host_fault_cause(&error.root_cause().to_string())
    }

    fn is_host_fault_cause(cause: &str) -> bool {
        matches!(cause, "oom" | "timeout")
    }

    /// 调用插件导出的函数（每次新建实例，见模块头「状态归属」）
    ///
    /// 顺带记一次资源占用（进程内存增量 + fuel），见 `last_stats`。
    /// 入口处依次过熔断闸门与并发闸门，见「并发额度与熔断」。
    pub fn call(
        id: &str,
        dir: &Path,
        manifest: &PluginManifest,
        func: &str,
        input: &str,
    ) -> Result<String, String> {
        // 闸门 ①：熔断。正在冷却的插件连编译都不做 —— 坏掉的 wasm 每次调用都会重编译一次，
        // 再加上一次 15s 超时，这是「一个插件坏了」唯一能拖慢**别的插件**的路径。
        if let Some(left) = circuit_remaining(id) {
            record_stats(id, func, false, None, None, None);
            return Err(format!(
                "插件 {} 连续失败过多，已熔断 {}s（还剩约 {}s）。修好后会自动恢复，急用可在设置里重装该插件",
                id,
                BREAKER_COOLDOWN_MS / 1000,
                (left + 999) / 1000,
            ));
        }

        // 闸门 ②：并发额度。`_slot` 活到函数返回为止（含整个实例化的存活期）；
        // panic 时 Drop 也会跑 —— 额度不会漏还，几次异常扩张不会让所有插件从此永久排队。
        let _slot = match CALL_SLOTS.acquire(Duration::from_millis(CALL_QUEUE_WAIT_MS)) {
            Some(s) => s,
            None => {
                record_stats(id, func, false, None, None, None);
                return Err(format!(
                    "插件调用过多（同时最多 {} 个，已等 {}s），请稍后再试",
                    MAX_CONCURRENT_CALLS,
                    CALL_QUEUE_WAIT_MS / 1000,
                ));
            }
        };

        let wasm_path = wasm_path_of(dir, manifest)?;
        let fp = fingerprint(dir, &dir.join("plugin.json"), &wasm_path);
        // 编译失败是结构性的：再试一次也是同样的错误，所以直接算故障
        let compiled = match compiled_for(id, manifest, &wasm_path, fp) {
            Ok(c) => c,
            Err(e) => {
                circuit_record(id, true);
                return Err(e);
            }
        };

        // 采样点 1：实例化之前，取进程内存基线
        let before = proc_mem();
        let mut plugin = match Plugin::new_from_compiled(&compiled) {
            Ok(p) => p,
            Err(e) => {
                let after = proc_mem();
                circuit_record(id, true);
                record_stats(id, func, false, before, after, None);
                return Err(format!("实例化插件失败（缺少 Extism PDK 导入？）: {}", e));
            }
        };

        // 清单声明了 init 时，先在同一实例上跑一次初始化（不与目标函数同名时）
        if let Some(init) = manifest.init.as_deref() {
            if !init.is_empty() && init != func {
                if let Err(e) = plugin.call::<&str, String>(init, "") {
                    let fault = is_host_fault(&e);
                    let after = proc_mem();
                    circuit_record(id, fault);
                    record_stats(id, func, false, before, after, plugin.fuel_consumed());
                    return Err(explain(&e, "插件 init 执行"));
                }
            }
        }

        let out_res: Result<String, Error> = plugin.call(func, input);
        // 采样点 2：必须赶在 `plugin` 析构之前采，否则线性内存已经还给系统，增量恒为 0
        let after = proc_mem();
        // fuel_consumed 是「实例创建至今」的差额 ⇒ 本次调用（含 init）的指令数
        let fuel_used = plugin.fuel_consumed();
        let fault = out_res.as_ref().err().is_some_and(is_host_fault);
        circuit_record(id, fault);
        record_stats(id, func, out_res.is_ok(), before, after, fuel_used);

        out_res.map_err(|e| explain(&e, "插件执行"))
    }

    /// 运行时信息
    pub fn runtime_info() -> Response<RuntimeInfo> {
        Response::new(200, runtime_info_of(true, memdump_enabled()), "ok")
    }

    #[cfg(test)]
    mod isolation_tests {
        use std::time::{Duration, Instant};

        use super::{
            is_host_fault_cause, Circuit, Slots, BREAKER_COOLDOWN_MS, BREAKER_THRESHOLD,
            MAX_CONCURRENT_CALLS,
        };

        #[test]
        fn circuit_trips_after_threshold_then_probes() {
            let mut c = Circuit::default();
            let now = Instant::now();
            for i in 1..BREAKER_THRESHOLD {
                c.record(now, true);
                assert!(c.check(now).is_ok(), "第 {i} 次故障还不该跳闸");
            }
            c.record(now, true);
            let left = c.check(now).unwrap_err();
            assert!(
                left > 0 && left <= BREAKER_COOLDOWN_MS,
                "跳闸后要给出剩余冷却毫秒，实测 {left}"
            );

            // 冷却期内一律不放行（也不重编译/不沾线程池的那条捷径靠它）
            assert!(c.check(now + Duration::from_millis(BREAKER_COOLDOWN_MS - 1)).is_err());
            // 冷却结束不是「恢复」而是「放一次探针」：清零后才算真正自愈
            assert!(c.check(now + Duration::from_millis(BREAKER_COOLDOWN_MS + 1)).is_ok());
            assert_eq!(c.consec, 0, "探针放行必须连带清零");
        }

        #[test]
        fn plugin_logic_errors_never_trip() {
            let now = Instant::now();
            let mut c = Circuit::default();
            for _ in 0..BREAKER_THRESHOLD * 3 {
                c.record(now, false);
                assert!(c.check(now).is_ok(), "插件自己的业务报错不该跳闸");
            }
            // 中间夹一次非故障（成功）同样要复位累计
            let mut c = Circuit::default();
            c.record(now, true);
            c.record(now, false);
            assert_eq!(c.consec, 0);
        }

        #[test]
        fn only_resource_faults_count() {
            assert!(is_host_fault_cause("oom"));
            assert!(is_host_fault_cause("timeout"));
            assert!(!is_host_fault_cause("bad input"), "普通报错不该算宿主故障");
        }

        #[test]
        fn slots_bound_concurrency_and_recover() {
            let slots = Slots::new();
            let held: Vec<_> = (0..MAX_CONCURRENT_CALLS)
                .map(|_| slots.acquire(Duration::from_millis(50)).expect("额度未满应可取到"))
                .collect();
            // 第 N+1 个：排队到超时也不该放行（宁可报错，也不无限排队）
            assert!(slots.acquire(Duration::from_millis(30)).is_none());
            drop(held);
            assert!(slots.acquire(Duration::from_millis(30)).is_some(), "归还后要能再取到");
        }
    }

    #[cfg(test)]
    mod net_perm_tests {
        use super::{http_allowed_hosts, net_allows_target, parse_net_allow, NetAllow};

        fn allow(p: &str) -> NetAllow {
            parse_net_allow(p).unwrap_or_else(|| panic!("应识别为 net: 声明: {}", p)).unwrap()
        }

        #[test]
        fn parses_net_scopes() {
            // `net:` / `net:*` / `net:*:*` 都表示任意主机任意端口
            for p in ["net:", "net:*", "net:*:*"] {
                let a = allow(p);
                assert_eq!(a.host, "*");
                assert_eq!(a.port, None, "{} 应是任意端口", p);
            }
            // 具体 host:port
            let a = allow("net:127.0.0.1:6379");
            assert_eq!(a.host, "127.0.0.1");
            assert_eq!(a.port, Some(6379));
            // 只写 host：该主机任意端口
            let a = allow("net:api.github.com");
            assert_eq!(a.host, "api.github.com");
            assert_eq!(a.port, None);
            // 子域通配 + 固定端口
            let a = allow("net:*.redis.example.com:6380");
            assert_eq!(a.host, "*.redis.example.com");
            assert_eq!(a.port, Some(6380));
            // IPv6 字面量：方括号内的冒号不是端口分隔符
            let a = allow("net:[::1]:6379");
            assert_eq!(a.host, "[::1]");
            assert_eq!(a.port, Some(6379));
            // 端口非法 / 旧写法 / 漏冒号 → 报错，而不是静默放行或静默忽略
            assert!(parse_net_allow("net:host:notaport").unwrap().is_err());
            assert!(
                parse_net_allow("net:tcp:127.0.0.1:6379").unwrap().is_err(),
                "旧 net:tcp 写法必须响亮报错，否则会被当成主机名静默误解析"
            );
            assert!(parse_net_allow("net").unwrap().is_err(), "裸 net 应提示缺冒号");
            // 不是 net 声明 → None（交给其它权限分支）
            assert!(parse_net_allow("fs:read").is_none());
            assert!(parse_net_allow("log").is_none());
        }

        #[test]
        fn matches_only_declared_targets() {
            let exact = vec![allow("net:127.0.0.1:6379")];
            assert!(net_allows_target(&exact, "127.0.0.1", 6379));
            assert!(!net_allows_target(&exact, "127.0.0.1", 6380), "端口不同不该放行");
            assert!(!net_allows_target(&exact, "10.0.0.1", 6379), "主机不同不该放行");

            // host 大小写不敏感
            let host_case = vec![allow("net:Redis.Example.com:6379")];
            assert!(net_allows_target(&host_case, "redis.example.com", 6379));

            // 不限端口的声明：该主机任意端口都算授权范围内
            let any_port = vec![allow("net:api.github.com")];
            assert!(net_allows_target(&any_port, "api.github.com", 443));
            assert!(net_allows_target(&any_port, "api.github.com", 6379));

            // 通配：任意主机 + 任意端口
            let any = vec![allow("net:*")];
            assert!(net_allows_target(&any, "10.0.0.1", 6379));
            assert!(net_allows_target(&any, "example.com", 1234));

            // 空声明一律不放行（默认全拒）
            assert!(!net_allows_target(&[], "127.0.0.1", 6379));
        }

        #[test]
        fn one_declaration_feeds_both_network_exits() {
            let perms: Vec<String> = ["log", "net:api.example.com", "net:127.0.0.1:6379", "fs:read"]
                .iter()
                .map(|s| s.to_string())
                .collect();

            // HTTP 白名单由同一声明派生，且只取 host：Extism 拿 url.host_str() 做 glob 匹配，
            // 带端口的条目会永远匹配不上
            assert_eq!(
                http_allowed_hosts(&perms),
                vec!["api.example.com".to_string(), "127.0.0.1".to_string()]
            );
            // 同一条声明也放行原始 socket（宿主不区分协议）
            let allows: Vec<NetAllow> = perms
                .iter()
                .filter_map(|p| parse_net_allow(p))
                .filter_map(Result::ok)
                .collect();
            assert!(net_allows_target(&allows, "api.example.com", 443));

            // 同一主机的多条声明只占一个 HTTP 条目
            let dup =
                vec!["net:api.example.com".to_string(), "net:api.example.com:443".to_string()];
            assert_eq!(http_allowed_hosts(&dup), vec!["api.example.com".to_string()]);

            // 通配声明 → HTTP 侧得到 `*`（glob 匹配任意主机）
            let any = vec!["net:*".to_string()];
            assert_eq!(http_allowed_hosts(&any), vec!["*".to_string()]);

            // 没有任何 net 声明 → 空（给 Extism 的是 None，不是空 Some）
            let none = vec!["log".to_string(), "fs:read".to_string()];
            assert!(http_allowed_hosts(&none).is_empty());
        }
    }
}

// ==================== 未启用运行时（桩） ====================

#[cfg(not(feature = "wasm-runtime"))]
mod stub {
    use std::path::Path;

    use super::{runtime_info_of, CallStats, FsRoots, RuntimeInfo};
    use crate::plugin_host::host::PluginManifest;
    use crate::response::response::Response;

    /// 未启用运行时：不阻塞安装（插件可能仍只有沙箱页面）
    pub fn probe(_id: &str, _dir: &Path, _manifest: &PluginManifest) -> Result<(), String> {
        Ok(())
    }

    /// 未启用运行时：保留 501 语义，前端桥无需改动
    pub fn call(
        _id: &str,
        _dir: &Path,
        _manifest: &PluginManifest,
        _func: &str,
        _input: &str,
    ) -> Result<String, String> {
        Err("WASM 运行时未启用：本次构建关闭了 wasm-runtime feature".to_string())
    }

    pub fn drop_instance(_id: &str) {}

    /// 未启用运行时：没有实例可采，恒为 None（前端按「无数据」展示）
    pub fn last_stats(_id: &str) -> Response<Option<CallStats>> {
        Response::new(200, None, "WASM 运行时未启用")
    }

    pub fn runtime_info() -> Response<RuntimeInfo> {
        Response::new(200, runtime_info_of(false, super::memdump_enabled()), "ok")
    }

    /// 未启用运行时：插件没有 fs 能力，作用域为空（对话框一律拒绝）
    pub fn fs_roots(_id: &str) -> Result<FsRoots, String> {
        Ok(FsRoots::default())
    }
}

#[cfg(not(feature = "wasm-runtime"))]
pub use stub::{call, drop_instance, fs_roots, last_stats, probe, runtime_info};

// ==================== 单元测试（两种构建共用） ====================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn runtime_info_reports_build_variant() {
        let info = runtime_info_of(true, false);
        assert!(info.available);
        assert!(!info.runtime.is_empty());
        assert!(info.timeout_ms > 0 && info.fuel > 0);
        assert!(info.max_memory_mb > 0);
        assert!(!info.memdump_on_error);

        let off = runtime_info_of(false, true);
        assert!(!off.available);
        assert!(off.runtime.is_empty());
        // 限制值与是否启用无关（UI 仍要展示「开启后会是什么样」）
        assert_eq!(off.fuel, info.fuel);
        // 调试开关与是否启用无关，照原样透出
        assert!(off.memdump_on_error);
    }

    #[test]
    fn permission_list_is_documented() {
        let info = runtime_info_of(true, false);
        for expect in ["log", "kv"] {
            assert!(
                info.permissions.iter().any(|p| p.starts_with(expect)),
                "缺少权限说明: {}",
                expect
            );
        }
        assert!(info.permissions.iter().any(|p| p.starts_with("fs:read")));
        assert!(info.permissions.iter().any(|p| p.starts_with("net:")));
    }

    /// `fs:read:*` 是任意路径，但跨插件隔离的排除区仍然拦得住
    ///
    /// 判定只有 `fs_permitted` 一处，所以测它就等于测读写两条通道（含流式句柄的打开）。
    #[test]
    fn fs_wildcard_scope_blocks_foreign_plugin_dirs() {
        let root = PathBuf::from(if cfg!(windows) { "D:/app" } else { "/app" });
        let join = |parts: &[&str]| {
            let mut p = root.clone();
            for s in parts {
                p.push(s);
            }
            p
        };
        let deny = vec![
            join(&["plugin_data_ext", "other-plugin"]),
            join(&["plugin_data"]),
            join(&["plugins_ext"]),
        ];
        let own = join(&["plugin_data_ext", "me"]);

        // 通配：任意位置一律放行（能读 SSH 私钥是它的字面语义，防不住也不该假装防得住）
        assert!(fs_permitted(true, &[], &deny, &join(&["Users", "x", ".ssh", "id_rsa"])));
        // 自己的数据目录不受排除区影响
        assert!(fs_permitted(true, &[], &deny, &own.join("state.json")));
        // 别人的数据目录：即使声明了 `*` 也拒绝
        assert!(!fs_permitted(
            true,
            &[],
            &deny,
            &join(&["plugin_data_ext", "other-plugin", "state.json"])
        ));
        // 内置插件数据 + 插件安装目录（写权限尤其不能放开：否则能改别人的 plugin.wasm）
        assert!(!fs_permitted(
            true,
            &[],
            &deny,
            &join(&["plugin_data", "ws", "database", "x.json"])
        ));
        assert!(!fs_permitted(
            true,
            &[],
            &deny,
            &join(&["plugins_ext", "other-plugin", "plugin.wasm"])
        ));
        // 排除区优先级**高于**显式作用域：显式声明也不放开（故障半径按插件切分）
        assert!(!fs_permitted(
            false,
            &deny,
            &deny,
            &join(&["plugins_ext", "other-plugin", "plugin.wasm"])
        ));
        // 非通配：没声明就是拒绝，声明过的前缀才放行
        assert!(!fs_permitted(false, &[], &deny, &join(&["Users", "x", ".ssh", "id_rsa"])));
        assert!(fs_permitted(false, &[own.clone()], &deny, &own.join("a/b.txt")));
    }

    /// 通配标志与排除区要给前端对话框用，字段名必须是 camelCase（前端按 `readAny` / `deny` 读）
    #[test]
    fn fs_roots_serializes_wildcard_flags_in_camel_case() {
        let r = FsRoots {
            read: vec![],
            write: vec![],
            read_any: true,
            write_any: false,
            deny: vec!["/x".to_string()],
        };
        let v = serde_json::to_value(&r).expect("FsRoots 应可序列化");
        assert_eq!(v["readAny"], serde_json::json!(true));
        assert_eq!(v["writeAny"], serde_json::json!(false));
        assert_eq!(v["deny"][0], serde_json::json!("/x"));
    }

    /// 进程内存采样：Windows 上必须拿得到（否则 B 方案的数字全是 None 却没人发现）
    #[test]
    fn proc_mem_is_available_on_windows() {
        let m = proc_mem();
        if cfg!(target_os = "windows") {
            let m = m.expect("Windows 上应能读到进程内存");
            assert!(m.commit > 0, "提交内存应大于 0");
            assert!(m.working_set > 0, "驻留内存应大于 0");
        } else {
            assert!(m.is_none(), "未实现的平台应返回 None，而不是编造 0");
        }
    }

    /// 撞限额时的错误必须被翻成人话（直接透传会让插件页只看到「oom」）
    #[cfg(feature = "wasm-runtime")]
    #[test]
    fn limit_errors_are_explained() {
        use extism::Error;

        let oom = imp::explain(&Error::msg("oom"), "插件执行");
        assert!(oom.contains("线性内存上限"), "oom 应给出内存上限提示: {}", oom);
        assert!(oom.contains(&format!("{}MB", (MAX_MEMORY_PAGES as u64 * 64) / 1024)));
        // 关键：要指向出路，而不是只说“失败”
        assert!(oom.contains("host_fs_read_at"), "应提示流式通道: {}", oom);

        let timeout = imp::explain(&Error::msg("timeout"), "插件执行");
        assert!(timeout.contains("可续跑"), "超时应提示做成可续跑: {}", timeout);

        // 其它错误保持原样，不要吞掉细节
        let other = imp::explain(&Error::msg("boom"), "插件执行");
        assert!(other.contains("boom"), "普通错误应保留原始信息: {}", other);
    }

    /// 「上次调用资源占用」必须真的记下来（否则设置页上的数字永远是空）
    #[cfg(feature = "wasm-runtime")]
    #[test]
    fn call_stats_are_recorded() {
        use crate::plugin_host::host::parse_manifest;

        let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../examples/third-party-plugins/text-toolkit");
        if !dir.join("plugin.wasm").is_file() {
            return;
        }
        let manifest = parse_manifest(&dir).expect("示例插件清单应可解析");
        let out = call(
            "text-toolkit",
            &dir,
            &manifest,
            "handle",
            r#"{"action":"upper","text":"x"}"#,
        )
        .expect("调用示例插件失败");
        assert!(out.contains("X"), "输出不符合预期: {}", out);

        let resp = last_stats("text-toolkit");
        assert_eq!(resp.code, 200);
        let stats = resp.data.expect("调用之后应有资源统计");
        // 本测试与其它端到端测试共用同一插件 id，可能互相覆盖；所以只断言「与具体哪次无关」的性质
        assert!(stats.ok);
        assert_eq!(stats.plugin_id, "text-toolkit");
        assert!(stats.fuel_used.unwrap_or(0) > 0, "fuel 是白拿的口径，不该缺失");
        if cfg!(target_os = "windows") {
            assert!(stats.proc_commit_bytes.is_some(), "Windows 上应有进程内存采样");
            assert!(stats.mem_delta_bytes.is_some(), "Windows 上应有内存增量");
        }
    }

    /// 端到端冒烟：把仓库里的示例插件真的加载进 Extism 跑一遍。
    ///
    /// 覆盖的是最容易出错、又最难靠静态检查发现的一段——Extism 与 extism-pdk 的 ABI
    /// （宿主函数命名空间、字符串的「offset ↔ 内存」转换、导出名与清单是否一致）。
    /// 「纯计算」动作不需要任何宿主权限，所以这里不涉及权限与环境相关的分支。
    #[cfg(feature = "wasm-runtime")]
    #[test]
    fn example_plugin_runs_end_to_end() {
        use crate::plugin_host::host::parse_manifest;

        let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../examples/third-party-plugins/text-toolkit");
        // 示例包随仓库分发；缺失时静默跳过（不把「没带 examples」变成测试失败）
        if !dir.join("plugin.wasm").is_file() {
            return;
        }
        let manifest = parse_manifest(&dir).expect("示例插件清单应可解析");
        assert!(manifest.wasm.is_some(), "示例插件应声明 wasm");
        assert_eq!(manifest.init.as_deref(), Some("init"));

        let input = r#"{"action":"upper","text":"abc"}"#;
        let out = call("text-toolkit", &dir, &manifest, "handle", input).expect("调用示例插件失败");
        assert!(out.contains("ABC"), "输出不符合预期: {}", out);

        // 反转：验证 Unicode 走的是字符而不是字节
        let input = r#"{"action":"reverse","text":"你好ab"}"#;
        let out = call("text-toolkit", &dir, &manifest, "handle", input).expect("调用示例插件失败");
        assert!(out.contains("ba好你"), "输出不符合预期: {}", out);

        // 未知动作走 fail 分支（guest 侧自己返回的 JSON，不是宿主错误）
        let out = call("text-toolkit", &dir, &manifest, "handle", r#"{"action":"nope"}"#)
            .expect("未知动作不应变成调用错误");
        assert!(out.contains("未知动作"), "输出不符合预期: {}", out);

        // 内存自报：guest 读自己的线性内存页数（`wasm32::memory_size(0)`），
        // 上限从 host_info 拿。这条路径同时钉住两件事：
        //   1. 自报口径是「按页提交」，所以一定是 64KiB 的整数倍；
        //   2. 宿主下发的 memoryLimitPages 与 MAX_MEMORY_PAGES 一致
        //      （两面口径一旦漂移，插件算出来的占用比就是错的）。
        let out = call("text-toolkit", &dir, &manifest, "handle", r#"{"action":"mem"}"#)
            .expect("内存自报不应变成调用错误");
        let v: serde_json::Value = serde_json::from_str(&out).expect("内存自报应是 JSON");
        let data = v.get("data").cloned().unwrap_or_default();
        let used = data.get("usedBytes").and_then(|x| x.as_u64()).unwrap_or(0);
        assert!(used > 0, "线性内存用量应为正（已提交页 × 64KiB）: {}", out);
        assert_eq!(used % (64 * 1024), 0, "线性内存按页提交，必须是 64KiB 的倍数: {}", out);
        let limit = data.get("limitBytes").and_then(|x| x.as_u64()).unwrap_or(0);
        assert_eq!(
            limit,
            MAX_MEMORY_PAGES as u64 * 64 * 1024,
            "自报上限应与宿主 MAX_MEMORY_PAGES 一致: {}",
            out
        );
        assert!(used < limit, "示例插件不该用满线性内存上限: {}", out);
    }

    /// 流式通道的地基：按偏移读写真的「指哪读哪」；跨过末尾是**短读**，越过末尾是 0 字节。
    #[cfg(feature = "wasm-runtime")]
    #[test]
    fn stream_helpers_read_and_write_at_offset() {
        use super::imp::{read_full_at, write_full_at};

        let dir = std::env::temp_dir().join(format!("bdt-stream-unit-{}", std::process::id()));
        std::fs::create_dir_all(&dir).expect("建临时目录");
        let path = dir.join("payload.bin");

        {
            let f = std::fs::File::create(&path).expect("建文件");
            write_full_at(&f, b"0123456789", 0).expect("先写 10 字节");
            write_full_at(&f, b"abc", 5).expect("覆盖中间 3 字节");
            write_full_at(&f, b"xyz", 10).expect("写到末尾之外应当扩展文件");
        }
        assert_eq!(std::fs::metadata(&path).unwrap().len(), 13);

        let f = std::fs::File::open(&path).expect("打开");
        let mut whole = [0u8; 16];
        let n = read_full_at(&f, &mut whole, 0).expect("整读");
        assert_eq!(n, 13);
        assert_eq!(&whole[..n], b"01234abc89xyz");

        let mut part = [0u8; 4];
        let n = read_full_at(&f, &mut part, 5).expect("按偏移读");
        assert_eq!(&part[..n], b"abc8");

        let n = read_full_at(&f, &mut part, 10).expect("跨过末尾是短读");
        assert_eq!(&part[..n], b"xyz");

        let n = read_full_at(&f, &mut part, 99).expect("越过末尾读 0 字节");
        assert_eq!(n, 0);

        drop(f);
        std::fs::remove_file(&path).ok();
        std::fs::remove_dir(&dir).ok();
    }

    /// 端到端：示例插件用**流式通道**处理比单块上限大得多的文件，并且跨多次调用续跑。
    ///
    /// 一条测试覆盖「大文件」整条链路：句柄 → 按偏移读 → 字节通道（不经 base64）→ 游标续跑 →
    /// 写侧（首轮 write_at、后续 append）→ rw 模式下按偏移改 + 截断。
    /// 夹具写在插件私有数据目录里，走默认 fs 作用域，不需要额外授权。
    #[cfg(feature = "wasm-runtime")]
    #[test]
    fn example_plugin_streams_large_file_across_calls() {
        use crate::plugin_host::host::{parse_manifest, plugin_data_ext_dir};

        let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../examples/third-party-plugins/text-toolkit");
        if !dir.join("plugin.wasm").is_file() {
            return;
        }
        let manifest = parse_manifest(&dir).expect("示例插件清单应可解析");
        assert!(manifest.permissions.iter().any(|p| p == "fs:read"));

        let data = plugin_data_ext_dir("text-toolkit").expect("插件数据目录应可解析");
        std::fs::create_dir_all(&data).expect("建插件数据目录");

        // 夹具：比「单块上限」大 3 倍多，每 100 行有一个 NEEDLE
        const NEEDLE: &str = "NEEDLE";
        let mut body = String::new();
        let mut expect_matched = 0usize;
        let mut line_no = 0usize;
        while body.len() < MAX_FS_CHUNK as usize * 3 + 4096 {
            if line_no % 100 == 0 {
                body.push_str(NEEDLE);
                body.push(' ');
                expect_matched += 1;
            }
            body.push_str("filler-0123456789-abcdefghij\n");
            line_no += 1;
        }
        let total = body.len() as u64;
        let src_name = "stream-fixture.txt";
        std::fs::write(data.join(src_name), body.as_bytes()).expect("写夹具");
        assert!(total > MAX_FS_CHUNK * 3, "夹具要真的跨多块");

        // ---- 读侧：scan 分块扫，按游标续跑 ----
        let mut cursor = 0u64;
        let mut matched = 0u64;
        let mut rounds = 0;
        loop {
            let input = serde_json::json!({
                "action": "scan", "path": src_name, "needle": NEEDLE, "cursor": cursor,
            })
            .to_string();
            let out = call("text-toolkit", &dir, &manifest, "handle", &input).expect("调用失败");
            let v: serde_json::Value = serde_json::from_str(&out).expect("输出应是 JSON");
            assert_eq!(v["ok"], serde_json::json!(true), "插件报错: {}", out);
            let d = &v["data"];
            matched += d["matched"].as_u64().unwrap_or(0);
            let next = d["cursor"].as_u64().unwrap_or(0);
            assert!(next > cursor || d["done"] == serde_json::json!(true), "游标没有推进: {}", out);
            cursor = next;
            rounds += 1;
            assert!(rounds < 32, "续跑轮数异常（可能每次只扫了一块）");
            if d["done"].as_bool().unwrap_or(false) {
                break;
            }
        }
        assert_eq!(cursor, total, "游标应停在文件末尾");
        assert_eq!(matched as usize, expect_matched, "分块扫描漏了或重复计数");
        assert!(rounds >= 2, "夹具大于一块，必须跨调用才算测到续跑");

        // ---- 写侧：copy 分块复制（首轮 write_at，后续 append）----
        let dst_name = "stream-copy.txt";
        let mut cursor = 0u64;
        let mut rounds = 0;
        loop {
            let input = serde_json::json!({
                "action": "copy", "path": src_name, "dest": dst_name, "cursor": cursor,
            })
            .to_string();
            let out = call("text-toolkit", &dir, &manifest, "handle", &input).expect("调用失败");
            let v: serde_json::Value = serde_json::from_str(&out).expect("输出应是 JSON");
            assert_eq!(v["ok"], serde_json::json!(true), "插件报错: {}", out);
            cursor = v["data"]["cursor"].as_u64().unwrap_or(0);
            rounds += 1;
            assert!(rounds < 32, "续跑轮数异常");
            if v["data"]["done"].as_bool().unwrap_or(false) {
                break;
            }
        }
        let copied = std::fs::read(data.join(dst_name)).expect("读回副本");
        assert_eq!(copied.len() as u64, total, "副本长度不符（写侧丢了块？）");
        assert_eq!(copied, body.as_bytes(), "副本内容与源文件不一致");

        // ---- 读写 + 截断：patch 就地改几个字节再截断 ----
        let patched_name = "stream-patch.bin";
        std::fs::write(data.join(patched_name), b"AAAAAAAAAA").expect("写 patch 夹具");
        let input = serde_json::json!({
            "action": "patch", "path": patched_name, "offset": 3, "text": "XYZ", "truncateTo": 7,
        })
        .to_string();
        let out = call("text-toolkit", &dir, &manifest, "handle", &input).expect("调用失败");
        let v: serde_json::Value = serde_json::from_str(&out).expect("输出应是 JSON");
        assert_eq!(v["ok"], serde_json::json!(true), "插件报错: {}", out);
        assert_eq!(
            std::fs::read(data.join(patched_name)).expect("读回"),
            b"AAAXYZA",
            "rw 模式下的按偏移写 + 截断结果不符"
        );

        for name in [src_name, dst_name, patched_name] {
            std::fs::remove_file(data.join(name)).ok();
        }
    }
}
