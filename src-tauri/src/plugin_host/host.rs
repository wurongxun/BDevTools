/*! # 第三方插件宿主（安装 / 卸载 / 排序 / 调用预留）

与内置插件（`src/plugins/` 下各插件目录，随应用编译）相对，第三方插件以「包目录」形式放在
`{app_dir}/plugins_ext/<plugin_id>/`，由宿主在运行时发现、校验并渲染：

- 包结构：`plugin.json`（清单）+ 入口 HTML + 可选静态资源 + 可选 `plugin.wasm`
- 渲染：入口页面经 `plugin://` 自定义协议加载进**沙箱 iframe**（`sandbox="allow-scripts"`，
  不透明源），第三方插件**不提供 `sidebarPanel`、不接管宿主 `tabBar`**，只占 `mainPanel`
- 逻辑：**WASM 运行时（Extism）已接入**（cargo feature `wasm-runtime`，默认开启）。
  声明了 `wasm` 的插件，其逻辑跑在 `plugin_wasm.rs` 的 Extism 沙箱里，
  宿主能力（日志 / KV / 文件 / HTTP）全部经 host function 注入并在函数内校验权限。
  构建时用 `--no-default-features` 关掉该 feature，`plugin_call` 会回落为 501，
  插件退化成「纯前端沙箱页面」。

## 安全边界（都在本模块内强制）
- id 规范 + 内置保留字：第三方不能顶替内置插件
- 清单校验：入口必须是包内 `.html/.htm` 相对路径，权限必须是白名单内的声明
- 资产读取：canonicalize 后校验仍在包目录内（防目录穿越），且限制静态资源扩展名
- 安装：zip 解包走 `enclosed_name()` 防 zip slip，并限制总体积与文件数
*/
use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::app::app_store::{read_settings, write_settings, APP};
use crate::response::response::Response;

/// 内置插件 ID（第三方插件禁止占用，防顶替与数据目录冲突）
/// 注意：新增内置插件时必须同步这张表，否则第三方插件可能顶替内置插件的 id 与数据目录。
/// 这张表同时是「内置插件启用/禁用」的取值域（见 `BUILTIN_DEFAULT_ENABLED` / `HOST_PINNED_IDS`），
/// 漏登记的内置插件会既拦不住第三方顶替、也拿不到默认关闭策略。
pub const BUILTIN_IDS: [&str; 9] = [
    "files",
    "ssh",
    "database",
    "logs",
    "mat",
    "terminal",
    "placeholder",
    "settings",
    "web3d",
];

/// 内置插件里**默认开启**的（首次运行时其余内置一律默认关闭）。
///
/// 为什么默认关：内置插件里有终端（会起 PTY）、内存分析（可能拉起 jvm/jhat）、SSH、数据库连接
/// 这些重资源入口，全部默认打开意味着首屏要同时准备一堆多数人当下用不到的东西。
/// 改成「按需启用」后，第一次打开应用只看到「文件」，其余从「设置 → 插件」自行开启。
pub const BUILTIN_DEFAULT_ENABLED: [&str; 1] = ["files"];

/// 宿主常驻入口（**不可禁用**）。
///
/// 与前端 `ToolPlugin.pinnedBottom` 同一语义：它是活动栏底部的齿轮，同时也是「插件管理」界面本身的入口。
/// 一旦允许禁用，用户就再也进不去设置页、无法启用任何插件 —— 一个自锁的死循环。
pub const HOST_PINNED_IDS: [&str; 1] = ["settings"];

/// settings.json：被禁用插件 ID 列表的键（内置与第三方**共用同一集合**）
const KEY_DISABLED: &str = "plugin_disabled";

/// settings.json：「内置插件默认状态已写入」标记的键（幂等开关）
const KEY_DEFAULTS_APPLIED: &str = "plugin_defaults_applied";

/// settings.json：插件设置值的键（`{ "<pluginId>": { "<key>": value } }`）
///
/// 放在宿主 settings.json 里而不是插件自己的 KV：设置要能随宿主设置一起导出 / 重置，
/// 也要能在**设置页**被统一渲染 —— KV 是插件私有存储，宿主不该去解读它的结构。
const KEY_SETTINGS: &str = "plugin_settings";

/// settings.json：**用户主动收紧掉的页面能力**的键（`{ "<pluginId>": ["wasm", ...] }`）
///
/// 语义是「从清单声明里**减去**」，不是「授予」——用户永远不能给出作者没声明的能力
/// （那样插件升级后行为不可预期，也等于替作者改写契约）。
/// 与 `plugin_settings` 一样放宿主 settings.json：要随宿主设置导出 / 重置。
const KEY_CAPS_DENIED: &str = "plugin_caps_denied";

/// 单个插件的设置项数量上限
const MAX_SETTINGS: usize = 32;
/// 设置项 key 长度上限
const MAX_SETTING_KEY: usize = 64;
/// `enum` 类型设置项的候选值数量上限
const MAX_ENUM_OPTIONS: usize = 32;

/// 安装根目录名（config_dir 下）
/// 注意：不能叫 `plugins`——该目录已被 KV 存储用作业内插件配置命名空间（`plugins/<id>.json`）
const INSTALL_ROOT: &str = "plugins_ext";

/// 第三方插件的 KV 文件名（与 `kv_store::THIRD_PARTY_PREFIX` 一致，迁移时需要）
const PLUGIN_KV_DIR: &str = "plugins_ext_kv";

/// 安装元数据文件名（宿主生成，不属于插件包内容）
const META_FILE: &str = ".bdt-install.json";

/// 允许的页面入口扩展名
const ENTRY_EXTS: [&str; 2] = ["html", "htm"];

/// 单包安装体积上限（64MB）
const MAX_PACKAGE_BYTES: u64 = 64 * 1024 * 1024;
/// 单包文件数上限
const MAX_PACKAGE_FILES: usize = 5000;
/// plugin_call 单次入参上限（1MB）
const MAX_CALL_INPUT: usize = 1024 * 1024;
/// plugin_call 单次输出上限（8MB），防止插件把宿主内存打满
const MAX_CALL_OUTPUT: usize = 8 * 1024 * 1024;
/// 函数名长度上限
const MAX_FUNC_NAME: usize = 64;
/// 第三方插件私有数据目录名（config 之外，位于 app_dir 下）
const PLUGIN_DATA_DIR: &str = "plugin_data_ext";

// ==================== 清单 ====================

/// 插件设置项类型（清单 `contributes.configuration[].type`）
///
/// 刻意只有四种：覆盖绝大多数场景，且**不需要做半个 JSON Schema 编辑器**。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SettingType {
    String,
    Boolean,
    Number,
    /// 单选：候选值见 `options`
    Enum,
}

/// 一个插件设置项的定义（清单 `contributes.configuration[]`）
///
/// 由**宿主**校验并渲染：插件不用再自己画一套设置页，设置也能随宿主设置统一导出 / 重置。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SettingDef {
    /// 设置项键名：同一插件内唯一
    pub key: String,
    /// 类型；JSON 里写 `type`（Rust 里 `type` 是关键字，故字段名用 `kind`）
    #[serde(rename = "type")]
    pub kind: SettingType,
    /// 设置页展示的标题
    pub title: String,
    /// 说明文字（可选）
    #[serde(default)]
    pub description: String,
    /// 默认值（类型必须与 `kind` 一致，安装时校验）
    pub default: serde_json::Value,
    /// `number` 的下界（可选）
    #[serde(default)]
    pub min: Option<f64>,
    /// `number` 的上界（可选）
    #[serde(default)]
    pub max: Option<f64>,
    /// `enum` 的候选值；JSON 里写 `enum`（Rust 关键字）
    #[serde(default, rename = "enum")]
    pub options: Option<Vec<String>>,
}

/// 插件贡献点（`contributes`）
///
/// 与 `permissions` 的分工：`permissions` 是**能力**（WASM 能用哪些宿主能力），
/// 这里是**集成**（宿主 UI 上如何呈现这个插件）。后者不需要权限——它不给插件任何新能力。
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Contributes {
    /// 设置项定义（宿主设置页渲染）
    #[serde(default)]
    pub configuration: Vec<SettingDef>,
}

/// 第三方插件清单（plugin.json）
///
/// ⚠️ **JSON 键名一律 camelCase**（`rename_all`）。清单是**人手写**的，Rust 字段名与 JSON 键名
/// 只靠这个属性对齐；漏了它 + 字段带 `#[serde(default)]` = 用户写了却不生效、且**不报任何错**
/// （曾因此让 `pageCapabilities` 静默失效，见 `manifest_json_keys_are_camel_case` 测试与
/// `check_manifest_keys`，后者会在安装时就把这类键名写错拦下来）。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PluginManifest {
    /// 插件 ID（小写字母/数字/./_/-，不与内置插件冲突）
    pub id: String,
    /// 显示名称
    pub name: String,
    /// 版本号
    pub version: String,
    #[serde(default)]
    pub description: String,
    /// 图标：内置图标名（白名单）或包内资源相对路径
    #[serde(default)]
    pub icon: Option<String>,
    /// 入口页面（包内相对路径，默认 page.html）
    #[serde(default = "default_entry")]
    pub entry: String,
    /// 后端 WASM 文件名（可选；声明后由 Extism 沙箱加载）
    #[serde(default)]
    pub wasm: Option<String>,
    /// 实例化后先调用的入口函数名（可选，如 `init`）；与目标函数同一次调用内共享实例
    #[serde(default)]
    pub init: Option<String>,
    /// 权限声明（白名单内）
    #[serde(default)]
    pub permissions: Vec<String>,
    /// 插件页能力声明（白名单内，见 `PAGE_CAPABILITIES`）
    ///
    /// 与 `permissions` 的分工：`permissions` 管 WASM 沙箱里的**宿主能力**（日志 / KV / 文件 / HTTP），
    /// 这里管**插件页自己**在 webview 里的能力（能否编译 WASM、能否 fetch 包内资源、能否用音视频与
    /// Worker、能否进入跨源隔离）。宿主据此决定 `plugin://` 返回哪一套 CSP，**默认空 = 最严**。
    ///
    /// 规范写法是 `pageCapabilities`；snake_case 的 `page_capabilities` 一并接受（历史/手误兜底）。
    #[serde(default, alias = "page_capabilities")]
    pub page_capabilities: Vec<String>,
    /// 本插件的**主界面在独立窗口里**（默认 false = 面板内沙箱 iframe）。
    ///
    /// **必须显式声明，不能由 `pageCapabilities` 推断**：独立窗口的插件页拿不到宿主命令
    /// （窗口 label 不在任何 capability 里），所以「带 WASM 后端 + 需要 `plugin_call`」的插件
    /// 一旦被自动判定成独立窗口就会直接废掉。是否牺牲宿主桥由插件作者自己决定。
    ///
    /// 声明后：活动栏图标 = 「打开（或前置）它的独立窗口」，主面板换成启动卡片（不再是 iframe）。
    #[serde(default)]
    pub standalone_window: bool,
    /// 贡献点（设置项 / 命令…）：宿主据此在设置页与命令面板里呈现插件能力。
    ///
    /// 与 `permissions` 是两套东西：这里**不授予任何能力**，只是告诉宿主「把我放在哪、
    /// 给我哪些入口」。所以未声明 = 没有集成入口，而不是报错。
    #[serde(default)]
    pub contributes: Option<Contributes>,
}

/// 清单允许出现的 JSON 键名（`rename_all = "camelCase"` 之后的**规范名**，加别名）。
///
/// 用途只有一个：把「键名写错」从**静默失效**变成可见告警。serde 默认忽略未知字段，而清单字段
/// 又几乎都带 `#[serde(default)]`，所以 `pagecapabilities` / `PageCapabilities` / `permission`
/// 这类错法**不会报错，只会表现为「我明明声明了却没生效」**。
/// **新增清单字段时必须同步这里**，否则新字段的正确写法会被误报为「未识别」。
const MANIFEST_KEYS: [&str; 13] = [
    "id",
    "name",
    "version",
    "description",
    "icon",
    "entry",
    "wasm",
    "init",
    "permissions",
    "pageCapabilities",
    "page_capabilities",
    "standaloneWindow",
    "contributes",
];

/// 键名校验：**未识别字段一律报错**（安装被拒），只放行 `x-` / `_` 前缀的扩展字段。
///
/// 为什么值得为此报错：serde 默认忽略未知字段，清单字段又几乎都带 `#[serde(default)]`，
/// 于是 `pagecapabilities` / `PageCapabilities` / `permission` 这类写法**不报任何错**，
/// 只表现为「我明明声明了却没生效」——排查成本极高（`pageCapabilities` 曾因此静默失效，
/// 表现为「插件页仍跑在最严 CSP 下」）。装的时候就报错，比装完再猜便宜得多。
///
/// 想放自定义元数据用 `x-` 或 `_` 前缀（如 `_comment` / `x-author`），静默忽略。
fn check_manifest_keys(raw: &str) -> Result<(), String> {
    let Ok(serde_json::Value::Object(map)) = serde_json::from_str::<serde_json::Value>(raw) else {
        // JSON 本身坏掉的情况交给后面正常的解析报错
        return Ok(());
    };
    let mut unknown: Vec<String> = Vec::new();
    for k in map.keys() {
        if MANIFEST_KEYS.contains(&k.as_str()) || k.starts_with("x-") || k.starts_with('_') {
            continue;
        }
        // 大小写写错是最常见的错法，单独给出「→ 正确写法」
        match MANIFEST_KEYS
            .iter()
            .find(|good| good.eq_ignore_ascii_case(k))
        {
            Some(good) => unknown.push(format!("{} → {}", k, good)),
            None => unknown.push(k.to_string()),
        }
    }
    if !unknown.is_empty() {
        return Err(format!(
            "plugin.json 含未识别字段: {}（合法字段：{}；自定义元数据请用 x- 或 _ 前缀）",
            unknown.join("，"),
            MANIFEST_KEYS.join(" / ")
        ));
    }
    Ok(())
}

fn default_entry() -> String {
    "page.html".to_string()
}

/// 安装元数据（宿主写入 `.bdt-install.json`）
#[derive(Debug, Clone, Serialize, Deserialize)]
struct InstallMeta {
    installed_at: String,
    #[serde(default)]
    source: String,
}

/// 已安装插件（前端展示用）
#[derive(Debug, Clone, Serialize)]
pub struct InstalledPlugin {
    pub id: String,
    pub name: String,
    pub version: String,
    pub description: String,
    /// 图标：内置图标名或包内相对路径（空串表示用默认图标）
    pub icon: String,
    pub entry: String,
    /// 包内是否存在清单声明的 wasm 文件
    pub has_wasm: bool,
    pub permissions: Vec<String>,
    /// 插件页能力声明（决定 `plugin://` 用哪套 CSP；空 = 最严）
    pub page_capabilities: Vec<String>,
    /// 用户从上面那份声明里**收紧掉**的项（settings.json `plugin_caps_denied`）
    ///
    /// 取值限定在 `PAGE_CAPABILITIES` 内；生效口径 = 声明 − 收紧。
    /// 收紧一个插件压根没声明的项是空操作（不报错，也不会凭空授予什么）。
    pub denied_page_capabilities: Vec<String>,
    /// 主界面是否在独立窗口里（清单 `standaloneWindow`；前端据此改活动栏图标的入口语义）
    pub standalone_window: bool,
    /// 是否启用（false 时不注册到前端插件列表）
    pub enabled: bool,
    /// 安装时间（RFC3339，缺失时为空串）
    pub installed_at: String,
    /// 包目录占用字节数
    pub size_bytes: u64,
    /// 恒为 false（第三方）；内置插件由前端自行提供
    pub builtin: bool,
    /// 设置项定义（清单 `contributes.configuration`；空 = 该插件没有设置）
    pub settings: Vec<SettingDef>,
}

// ==================== 校验 ====================

/// 合法插件 ID：非空、≤64、仅小写字母/数字/./_/-，且不以 `.` 开头、不含 `..`
pub fn is_valid_id(id: &str) -> bool {
    !id.is_empty()
        && id.len() <= 64
        && !id.starts_with('.')
        && !id.contains("..")
        && id
            .chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '.' || c == '_' || c == '-')
}

/// 安全相对路径：非空、非绝对、不含 `..`、不含 `:`（Windows 盘符）
fn is_safe_rel_path(p: &str) -> bool {
    !p.is_empty()
        && !p.starts_with('/')
        && !p.starts_with('\\')
        && !p.contains("..")
        && !p.contains(':')
}

/// 权限声明是否在白名单内：
/// `log` / `kv` / `fs:read[:作用域]` / `fs:write[:作用域]` / `net:<host>`
///
/// `fs:read` / `fs:write` 不带作用域 = 只限插件私有数据目录；
/// 带作用域时必须是绝对路径或 `$HOME` 前缀（`..`、`*`、空字节一律拒绝）。
fn is_permission_allowed(p: &str) -> bool {
    if matches!(p, "log" | "kv" | "fs:read" | "fs:write") {
        return true;
    }
    if let Some(scope) = p
        .strip_prefix("fs:read:")
        .or_else(|| p.strip_prefix("fs:write:"))
    {
        return is_valid_fs_scope(scope);
    }
    match p.strip_prefix("net:") {
        Some(host) => !host.is_empty() && !host.contains('/') && host.len() <= 253,
        None => false,
    }
}

/// fs 作用域是否合法：`$HOME` / `$HOME/...` / `~/...` / 绝对路径（含 Windows 盘符）
fn is_valid_fs_scope(scope: &str) -> bool {
    if scope.is_empty() || scope.len() > 1024 {
        return false;
    }
    if scope.contains("..") || scope.contains('*') || scope.contains('\0') {
        return false;
    }
    if scope == "$HOME" {
        return true;
    }
    if let Some(rest) = scope.strip_prefix("$HOME/") {
        return !rest.is_empty();
    }
    if let Some(rest) = scope.strip_prefix("~/") {
        return !rest.is_empty();
    }
    if scope.starts_with('/') {
        return true;
    }
    // Windows 盘符：形如 C:/ 或 C:\\
    let bytes: Vec<char> = scope.chars().take(3).collect();
    bytes.len() >= 2 && bytes[0].is_ascii_alphabetic() && bytes[1] == ':'
}

// ==================== 插件页能力（CSP profile） ====================

/// 插件页能力白名单（清单 `pageCapabilities`）。
///
/// 每一项都对应一条**明确**的 CSP 指令，便于逐条审计；默认（不声明）＝历史最严 CSP 原样不变：
/// - `wasm`    → `script-src` 追加 `'wasm-unsafe-eval'`（允许编译 WASM，**不**放开 JS `eval`）
/// - `assets`  → `connect-src 'self'`（允许 fetch **自己包内**的 .glb / .data / .pck 等，仍到不了外网）
/// - `media`   → `media-src 'self' data: blob:`（`<audio>` / `<video>`；Web Audio 本来不受 CSP 管）
/// - `worker`  → `worker-src 'self' blob:`（引擎的 Web Worker / 从 Blob 建 Worker）
/// - `isolate` → 不走 CSP，而是给响应加 COOP/COEP，让**顶层窗口**进入跨源隔离
///               （解锁 `SharedArrayBuffer` → 多线程 WASM 引擎；iframe 里加了也无效，见 web3d.md）
///
/// 刻意**不提供**任何能拿到宿主权限的项：这个白名单只影响插件页自身的 webview 能力，
/// 与 `permissions`（WASM 沙箱的宿主能力）互不相干，也不会让插件页拿到 Tauri IPC。
pub const PAGE_CAPABILITIES: [&str; 5] = ["wasm", "assets", "media", "worker", "isolate"];

/// 页面能力是否在白名单内
pub fn is_page_capability_allowed(c: &str) -> bool {
    PAGE_CAPABILITIES.contains(&c)
}

/// 未声明任何页面能力时的严格 CSP（与历史行为逐字一致，改动它会放宽所有第三方插件）
pub const CSP_STRICT: &str = "default-src 'none'; script-src 'self'; style-src 'self' 'unsafe-inline'; img-src 'self' data:; font-src 'self'; connect-src 'none'";

/// 按页面能力声明拼 CSP：**未声明的指令保持严格值**，不存在「声明 A 顺带放开 B」
pub fn page_csp(caps: &[String]) -> String {
    let has = |c: &str| caps.iter().any(|x| x == c);
    let mut script = String::from("'self'");
    if has("wasm") {
        script.push_str(" 'wasm-unsafe-eval'");
    }
    let img = if has("media") {
        "img-src 'self' data: blob:"
    } else {
        "img-src 'self' data:"
    };
    let connect = if has("assets") {
        "connect-src 'self'"
    } else {
        "connect-src 'none'"
    };
    let mut parts = vec![
        "default-src 'none'".to_string(),
        format!("script-src {}", script),
        "style-src 'self' 'unsafe-inline'".to_string(),
        img.to_string(),
        "font-src 'self'".to_string(),
        connect.to_string(),
    ];
    if has("media") {
        parts.push("media-src 'self' data: blob:".to_string());
    }
    if has("worker") {
        parts.push("worker-src 'self' blob:".to_string());
    }
    parts.join("; ")
}

/// 一次页面请求该用的策略
#[derive(Debug, Clone)]
pub struct PagePolicy {
    /// 响应头里的 CSP
    pub csp: String,
    /// 是否需要 COOP/COEP（跨源隔离）
    pub isolate: bool,
}

impl PagePolicy {
    /// 最严默认（清单读不到 / 解析失败 / 未声明任何页面能力）
    fn strict() -> Self {
        PagePolicy {
            csp: CSP_STRICT.to_string(),
            isolate: false,
        }
    }
}

/// 读某插件清单得出页面策略。
///
/// 只应在 **HTML 文档响应**上调用（子资源响应的 CSP 头会被浏览器忽略，没必要多读一次清单）；
/// 清单缺失或非法时**回落最严**——安全默认，不因为读不到就放宽。
pub fn page_policy(plugin_id: &str) -> PagePolicy {
    let Ok(dir) = plugin_dir(plugin_id) else {
        return PagePolicy::strict();
    };
    page_policy_in(&dir, &denied_page_caps(plugin_id))
}

/// 策略计算本体（纯函数，便于拿**真实包目录**做测试，不依赖全局 APP 路径）
///
/// `denied` 是用户在设置里收紧掉的项（settings.json `plugin_caps_denied`）。
/// 生效口径 = **声明 − 收紧**：只能减，不能加。
fn page_policy_in(dir: &Path, denied: &[String]) -> PagePolicy {
    let Ok(manifest) = parse_manifest(dir) else {
        return PagePolicy::strict();
    };
    // 减掉被收紧的项；某个能力被整体收紧后 isolate 也随之关掉
    let caps: Vec<String> = manifest
        .page_capabilities
        .into_iter()
        .filter(|c| !denied.iter().any(|d| d == c))
        .collect();
    if caps.is_empty() {
        return PagePolicy::strict();
    }
    PagePolicy {
        isolate: caps.iter().any(|c| c == "isolate"),
        csp: page_csp(&caps),
    }
}

/// 某插件被用户收紧掉的页面能力（读不到即空 = 完全按清单声明执行）
pub fn denied_page_caps(id: &str) -> Vec<String> {
    read_settings()
        .get(KEY_CAPS_DENIED)
        .and_then(|v| v.get(id))
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|v| v.as_str())
                // 只认白名单内的值：settings.json 是**人手可编辑**的，脏值必须被忽略而不是放宽
                .filter(|c| is_page_capability_allowed(c))
                .map(str::to_string)
                .collect()
        })
        .unwrap_or_default()
}

/// 设置某插件被收紧的页面能力。**只能收紧，不能授予**
///
/// 传空数组 = 恢复插件自己的声明（把该插件的键删掉，不在 settings.json 里留空对象）。
/// 取值必须落在 `PAGE_CAPABILITIES` 内：用户不该替插件声明它没要过的东西，
/// 否则插件升级改了声明之后，宿主这边多出来的授权会让行为完全不可预期。
pub fn set_denied_page_caps(id: &str, denied: &[String]) -> Result<(), String> {
    for c in denied {
        if !is_page_capability_allowed(c) {
            return Err(format!(
                "未知的页面能力: {}（合法值：{}）",
                c,
                PAGE_CAPABILITIES.join(" / ")
            ));
        }
    }
    let mut clean: Vec<String> = Vec::new();
    for c in denied {
        if !clean.contains(c) {
            clean.push(c.clone());
        }
    }

    let mut settings = read_settings();
    let Some(obj) = settings.as_object_mut() else {
        return Err("settings.json 结构异常（不是对象）".to_string());
    };
    let mut all = obj
        .get(KEY_CAPS_DENIED)
        .and_then(|v| v.as_object())
        .cloned()
        .unwrap_or_default();
    if clean.is_empty() {
        all.remove(id);
    } else {
        all.insert(id.to_string(), serde_json::json!(clean));
    }
    if all.is_empty() {
        obj.remove(KEY_CAPS_DENIED);
    } else {
        obj.insert(KEY_CAPS_DENIED.to_string(), serde_json::Value::Object(all));
    }
    write_settings(&settings);
    Ok(())
}

/// 清单校验（不依赖安装目录名）
fn validate_manifest(m: &PluginManifest) -> Result<(), String> {
    if !is_valid_id(&m.id) {
        return Err(format!(
            "插件 id 非法（仅允许小写字母/数字/./_/-，不以 . 开头）: {}",
            m.id
        ));
    }
    if BUILTIN_IDS.contains(&m.id.as_str()) {
        return Err(format!("插件 id 与内置插件冲突: {}", m.id));
    }
    if m.name.trim().is_empty() || m.name.len() > 64 {
        return Err("插件 name 不能为空且不超过 64 字符".to_string());
    }
    if m.version.trim().is_empty() || m.version.len() > 32 {
        return Err("插件 version 不能为空且不超过 32 字符".to_string());
    }
    if !is_safe_rel_path(&m.entry) {
        return Err(format!("插件 entry 不是包内安全相对路径: {}", m.entry));
    }
    let ext = m.entry.rsplit('.').next().unwrap_or("").to_ascii_lowercase();
    if !ENTRY_EXTS.contains(&ext.as_str()) {
        return Err("插件 entry 必须是 .html/.htm 页面".to_string());
    }
    if let Some(w) = &m.wasm {
        if !is_safe_rel_path(w) {
            return Err(format!("插件 wasm 路径非法: {}", w));
        }
    }
    for p in &m.permissions {
        if !is_permission_allowed(p) {
            return Err(format!("不支持的权限声明: {}", p));
        }
    }
    // 与 permissions 同一原则：页面能力也必须显式授权且在白名单内，未知项直接拒绝安装，
    // 不允许「静默降级」——否则插件作者会以为能力生效了，实际被 CSP 挡着却查不出原因。
    for c in &m.page_capabilities {
        if !is_page_capability_allowed(c) {
            return Err(format!(
                "不支持的页面能力声明: {}（可选值：{}）",
                c,
                PAGE_CAPABILITIES.join(" / ")
            ));
        }
    }
    // 设置项同样「未知 / 不合法一律拒装」：装的时候报错，比装完发现设置页渲染不出来便宜
    if let Some(c) = &m.contributes {
        validate_configuration(&c.configuration)?;
    }
    Ok(())
}

/// 设置项键名：非空、≤64、仅字母/数字/./_/-
fn is_valid_setting_key(k: &str) -> bool {
    !k.is_empty()
        && k.len() <= MAX_SETTING_KEY
        && k
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '.' || c == '-')
}

/// 值是否符合设置项的类型（**纯函数**，写入与读取两处共用同一判定）
pub fn setting_value_matches(v: &serde_json::Value, kind: SettingType) -> bool {
    match kind {
        SettingType::String => v.is_string(),
        SettingType::Boolean => v.is_boolean(),
        SettingType::Number => v.is_number(),
        SettingType::Enum => v.is_string(),
    }
}

/// 值是否落在 min / max / enum 约束内（**纯函数**）
fn setting_in_range(v: &serde_json::Value, d: &SettingDef) -> bool {
    match d.kind {
        SettingType::Number => {
            let Some(x) = v.as_f64() else {
                return false;
            };
            if let Some(lo) = d.min {
                if x < lo {
                    return false;
                }
            }
            if let Some(hi) = d.max {
                if x > hi {
                    return false;
                }
            }
            true
        }
        SettingType::Enum => {
            let Some(s) = v.as_str() else {
                return false;
            };
            d.options
                .as_ref()
                .map(|o| o.iter().any(|x| x == s))
                .unwrap_or(false)
        }
        _ => true,
    }
}

/// 校验一份设置项定义（安装时调用；不合格拒装）
///
/// 为什么必须校验而不是「渲染时跳过」：设置项是**宿主替插件呈现**的东西，
/// 一条非法定义会让设置页渲染不出来、或渲染出一个改不动的控件，
/// 而作者只会看到「我的设置项消失了」—— 排查成本极高。
fn validate_configuration(defs: &[SettingDef]) -> Result<(), String> {
    if defs.len() > MAX_SETTINGS {
        return Err(format!("设置项数量超过上限（{}）", MAX_SETTINGS));
    }
    let mut seen: HashSet<String> = HashSet::new();
    for d in defs {
        if !is_valid_setting_key(&d.key) {
            return Err(format!(
                "设置项 key 非法（仅允许字母/数字/./_/-，不超过 {} 字符）: {}",
                MAX_SETTING_KEY, d.key
            ));
        }
        if !seen.insert(d.key.clone()) {
            return Err(format!("设置项 key 重复: {}", d.key));
        }
        if d.title.trim().is_empty() || d.title.len() > 64 {
            return Err(format!("设置项 {} 的 title 不能为空且不超过 64 字符", d.key));
        }
        if !setting_value_matches(&d.default, d.kind) {
            return Err(format!("设置项 {} 的 default 与 type 不符", d.key));
        }
        match d.kind {
            SettingType::Number => {
                let v = d.default.as_f64().unwrap_or(0.0);
                if let (Some(lo), Some(hi)) = (d.min, d.max) {
                    if lo > hi {
                        return Err(format!("设置项 {} 的 min 大于 max", d.key));
                    }
                }
                if let Some(lo) = d.min {
                    if v < lo {
                        return Err(format!("设置项 {} 的 default 小于 min", d.key));
                    }
                }
                if let Some(hi) = d.max {
                    if v > hi {
                        return Err(format!("设置项 {} 的 default 大于 max", d.key));
                    }
                }
            }
            SettingType::Enum => {
                let opts = d.options.as_ref().ok_or_else(|| {
                    format!("enum 类型的设置项 {} 必须给出 enum 列表", d.key)
                })?;
                if opts.is_empty() || opts.len() > MAX_ENUM_OPTIONS {
                    return Err(format!(
                        "设置项 {} 的 enum 列表为空或超过 {} 项",
                        d.key, MAX_ENUM_OPTIONS
                    ));
                }
                if opts.iter().collect::<HashSet<_>>().len() != opts.len() {
                    return Err(format!("设置项 {} 的 enum 列表有重复项", d.key));
                }
                let cur = d.default.as_str().unwrap_or("");
                if !opts.iter().any(|o| o == cur) {
                    return Err(format!("设置项 {} 的 default 不在 enum 列表内", d.key));
                }
            }
            // string / boolean 不接受 min / max / enum：写了说明作者理解错了类型，
            // 报错而不是静默忽略（静默忽略 = 作者以为约束生效了）
            _ => {
                if d.min.is_some() || d.max.is_some() || d.options.is_some() {
                    return Err(format!(
                        "设置项 {} 是 {:?} 类型，不接受 min / max / enum",
                        d.key, d.kind
                    ));
                }
            }
        }
    }
    Ok(())
}

// ==================== 路径与存储 ====================

/// 安装根目录：`{app_dir}/plugins_ext`
///
/// **必须落在 `app_dir` 而不是 `config_dir`**：NSIS 卸载脚本在用户勾选「删除应用数据」时会
/// `RmDir /r "$APPDATA\${BUNDLEID}"`，而 `config_dir` 正是 `$APPDATA\com.backend-development-tools.app`
/// —— 插件放在那里，卸载/重装一次就全没了。第三方插件是用户资产，不随应用卸载消失。
fn install_root() -> PathBuf {
    install_root_in(&APP.lock().app_dir)
}

/// [`install_root`] 的纯函数版（不碰全局单例，便于单测）
fn install_root_in(app_dir: &Path) -> PathBuf {
    app_dir.join(INSTALL_ROOT)
}

/// 一次性迁移：把第三方插件存储从 `config_dir` 搬到 `app_dir`
///
/// 必须在 `app_store::App::update_dirs` **之后**调用（依赖已算好的两个目录）。
/// 迁移失败**不阻断启动**：数据仍在旧位置，下次启动会再试一次。
pub fn migrate_third_party_storage() {
    let (config_dir, app_dir) = {
        let app = APP.lock();
        (app.config_dir.clone(), app.app_dir.clone())
    };
    // 路径没初始化好时不做任何猜测，避免拿相对路径搬错目录
    if config_dir.as_os_str().is_empty() || app_dir.as_os_str().is_empty() {
        return;
    }
    migrate_plugin_dirs(&config_dir, &app_dir);
}

/// [`migrate_third_party_storage`] 的纯函数版（不碰全局单例，便于单测）
fn migrate_plugin_dirs(config_dir: &Path, app_dir: &Path) {
    use crate::app::app_store::{migrate_dir_once, MigrateOutcome};
    for name in [INSTALL_ROOT, PLUGIN_KV_DIR] {
        let from = config_dir.join(name);
        let to = app_dir.join(name);
        match migrate_dir_once(&from, &to) {
            MigrateOutcome::Migrated => {
                println!("[PluginHost] 第三方插件数据已迁移: {:?} -> {:?}", from, to);
            }
            MigrateOutcome::Failed => {
                println!("[PluginHost] 迁移失败，数据保留在 {:?}", from);
            }
            MigrateOutcome::NotNeeded => {}
        }
    }
}

/// 插件包目录（含 id 合法性校验）
pub(crate) fn plugin_dir(id: &str) -> Result<PathBuf, String> {
    if !is_valid_id(id) {
        return Err(format!("非法插件 ID: {}", id));
    }
    Ok(install_root().join(id))
}

/// 插件私有数据根目录（`{app_dir}/plugin_data`，与内置插件同一套约定）
fn plugin_data_root() -> PathBuf {
    APP.lock().app_dir.join("plugin_data")
}

/// 第三方插件私有数据目录：`{app_dir}/plugin_data_ext/<plugin_id>/`
///
/// 与内置插件的 `plugin_data/{workspace}/{plugin_id}/` 分开：WASM 调用不带工作区上下文，
/// 插件状态按「插件」而非「插件+工作区」组织，语义更简单，也不会随切换工作区丢失。
pub(crate) fn plugin_data_ext_dir(id: &str) -> Result<PathBuf, String> {
    if !is_valid_id(id) {
        return Err(format!("非法插件 ID: {}", id));
    }
    Ok(APP.lock().app_dir.join(PLUGIN_DATA_DIR).join(id))
}

/// 删除第三方插件私有数据目录（卸载时 `remove_data=true` 调用）
fn purge_plugin_ext_data(id: &str) {
    let Ok(dir) = plugin_data_ext_dir(id) else {
        return;
    };
    if dir.is_dir() {
        if let Err(e) = fs::remove_dir_all(&dir) {
            println!("[PluginHost] 清理插件数据失败 {}: {}", dir.display(), e);
        }
    }
}

/// 从一份 settings 值里取被禁用的插件 ID 集合（**纯函数**，便于测试）
fn disabled_ids_of(settings: &serde_json::Value) -> HashSet<String> {
    settings
        .get(KEY_DISABLED)
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|v| v.as_str().map(str::to_string))
                .collect()
        })
        .unwrap_or_default()
}

/// settings.json 中被禁用的插件 ID 集合（内置 + 第三方）
fn disabled_ids() -> HashSet<String> {
    disabled_ids_of(&read_settings())
}

/// 写入被禁用插件 ID 集合。
///
/// **排序后写**：`HashSet` 的迭代顺序不稳定，直接写会让 settings.json 每次保存都抖动
/// —— 既让人看不出真实变更，也会白白触发基于文件变化的逻辑。
fn set_disabled_ids(settings: &mut serde_json::Value, ids: &HashSet<String>) {
    let mut sorted: Vec<String> = ids.iter().cloned().collect();
    sorted.sort();
    if let Some(obj) = settings.as_object_mut() {
        obj.insert(KEY_DISABLED.to_string(), serde_json::json!(sorted));
    }
}

/// 按「内置插件默认状态」补全 settings，返回本次新增禁用的插件 ID（**纯函数**）。
///
/// 规则：
/// - 只有 `BUILTIN_DEFAULT_ENABLED` 里的内置插件默认开启，其余内置一律写入禁用集合；
/// - **合并而非覆盖**：老用户的 settings 里已经存着 `plugin_disabled`（第三方插件的记录），
///   整体覆盖会把它们禁用过的第三方插件悄悄全部启用回来；
/// - 宿主常驻入口（`HOST_PINNED_IDS`）永远不写进禁用集合，历史数据里若误有则顺手清掉；
/// - **幂等**：由 `plugin_defaults_applied` 标记把关，用户自己改过的状态不会被每次启动纠正。
pub fn apply_plugin_defaults(settings: &mut serde_json::Value) -> Vec<String> {
    if !settings.is_object() || settings.get(KEY_DEFAULTS_APPLIED).is_some() {
        return Vec::new();
    }
    let mut disabled = disabled_ids_of(settings);
    let mut newly: Vec<String> = Vec::new();
    for id in BUILTIN_IDS {
        if HOST_PINNED_IDS.contains(&id) {
            disabled.remove(id);
            continue;
        }
        if !BUILTIN_DEFAULT_ENABLED.contains(&id) && disabled.insert(id.to_string()) {
            newly.push(id.to_string());
        }
    }
    set_disabled_ids(settings, &disabled);
    if let Some(obj) = settings.as_object_mut() {
        obj.insert(KEY_DEFAULTS_APPLIED.to_string(), serde_json::json!(true));
    }
    newly.sort();
    newly
}

/// 启动时写入内置插件默认状态（只开 `BUILTIN_DEFAULT_ENABLED` 里的，其余关闭）。
///
/// 刻意**不做**任何失败兜底以外的动作：插件默认状态是「一次性初始化」，
/// 用户之后在设置页的选择才是权威（包括把 files 也关掉的决定）。
pub fn ensure_plugin_defaults() {
    let mut settings = read_settings();
    let newly = apply_plugin_defaults(&mut settings);
    if newly.is_empty() {
        return;
    }
    write_settings(&settings);
    println!(
        "[PluginHost] 已应用内置插件默认状态：默认关闭 {} 个（{}）。可在「设置 → 插件」按需开启。",
        newly.len(),
        newly.join(", ")
    );
}

/// 解析包目录内的 plugin.json（不做目录名一致性检查）
pub(crate) fn parse_manifest(dir: &Path) -> Result<PluginManifest, String> {
    let raw = fs::read_to_string(dir.join("plugin.json"))
        .map_err(|e| format!("读取 plugin.json 失败: {}", e))?;
    check_manifest_keys(&raw)?;
    let m: PluginManifest =
        serde_json::from_str(&raw).map_err(|e| format!("plugin.json 解析失败: {}", e))?;
    validate_manifest(&m)?;
    Ok(m)
}

/// 递归统计目录占用（跳过无法访问的条目）
fn dir_size(dir: &Path) -> u64 {
    let mut total = 0u64;
    let Ok(entries) = fs::read_dir(dir) else {
        return 0;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if let Ok(meta) = entry.metadata() {
            if meta.is_file() {
                total += meta.len();
            } else if meta.is_dir() {
                total += dir_size(&path);
            }
        }
    }
    total
}

/// 读取一个已安装插件的完整信息
fn load_installed(dir: &Path) -> Result<InstalledPlugin, String> {
    let id = dir
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or("")
        .to_string();
    let m = parse_manifest(dir)?;
    // 目录名必须与清单 id 一致：防止手工改名后资产路径与 id 错位
    if m.id != id {
        return Err(format!(
            "plugin.json 的 id({}) 与安装目录名({}) 不一致",
            m.id, id
        ));
    }
    if !dir.join(&m.entry).is_file() {
        return Err(format!("入口页面不存在: {}", m.entry));
    }
    let meta: InstallMeta = fs::read_to_string(dir.join(META_FILE))
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or(InstallMeta {
            installed_at: String::new(),
            source: String::new(),
        });
    let has_wasm = m
        .wasm
        .as_ref()
        .map(|w| dir.join(w).is_file())
        .unwrap_or(false);
    let enabled = !disabled_ids().contains(&m.id);
    // 必须在构造 InstalledPlugin **之前**取：字面量里 `id: m.id` 会把 id 移走
    let denied_page_capabilities = denied_page_caps(&m.id);
    Ok(InstalledPlugin {
        id: m.id,
        name: m.name,
        version: m.version,
        description: m.description,
        icon: m.icon.unwrap_or_default(),
        entry: m.entry,
        has_wasm,
        enabled,
        permissions: m.permissions,
        page_capabilities: m.page_capabilities,
        denied_page_capabilities,
        standalone_window: m.standalone_window,
        installed_at: meta.installed_at,
        size_bytes: dir_size(dir),
        builtin: false,
        settings: m.contributes.map(|c| c.configuration).unwrap_or_default(),
    })
}

// ==================== 列表 / 启用 / 排序 ====================

/// 列出全部已安装第三方插件（跳过非法包并记录日志）
pub fn plugin_list() -> Response<Vec<InstalledPlugin>> {
    let root = install_root();
    let Ok(entries) = fs::read_dir(&root) else {
        return Response::new(200, Vec::new(), "ok");
    };
    let mut out: Vec<InstalledPlugin> = Vec::new();
    for entry in entries.flatten() {
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }
        let Some(id) = path.file_name().and_then(|s| s.to_str()).map(str::to_string) else {
            continue;
        };
        if id.starts_with('.') {
            continue; // .staging 等宿主内部目录
        }
        match load_installed(&path) {
            Ok(item) => out.push(item),
            Err(e) => println!("[PluginHost] 跳过非法插件包 {}: {}", id, e),
        }
    }
    out.sort_by(|a, b| a.name.cmp(&b.name));
    Response::new(200, out, "ok")
}

/// 启用 / 禁用插件（内置与第三方统一处理）。
///
/// 两种形态的差别只在**前置校验**：内置插件没有安装目录，写禁用集合即可；
/// 第三方插件必须先装着。禁用之后的善后是共用的 —— 回收 WASM 编译产物，
/// 既省内存，也让「禁用」在后端真正生效（见 `plugin_call` 的 enabled 校验）。
pub fn plugin_set_enabled(id: String, enabled: bool) -> Response<bool> {
    // 只拦「禁用」：宿主常驻入口一旦关掉，用户就进不去设置页，也就再也开不回来
    if !enabled && HOST_PINNED_IDS.contains(&id.as_str()) {
        return Response::new(500, false, "宿主常驻入口不能禁用");
    }
    if !BUILTIN_IDS.contains(&id.as_str()) {
        let dir = match plugin_dir(&id) {
            Ok(d) => d,
            Err(e) => return Response::new(500, false, &e),
        };
        if !dir.exists() {
            return Response::new(500, false, "插件未安装");
        }
    }
    let mut settings = read_settings();
    apply_enabled_toggle(&mut settings, &id, enabled);
    write_settings(&settings);
    if !enabled {
        super::wasm::drop_instance(&id);
    }
    Response::new(200, true, if enabled { "已启用" } else { "已禁用" })
}

/// 把一个插件的启用意图写进禁用集合（**纯函数**，便于脱离全局单例测试）
fn apply_enabled_toggle(settings: &mut serde_json::Value, id: &str, enabled: bool) {
    let mut disabled = disabled_ids_of(settings);
    if enabled {
        disabled.remove(id);
    } else {
        disabled.insert(id.to_string());
    }
    set_disabled_ids(settings, &disabled);
}

/// 被禁用的插件 ID 列表（内置 + 第三方同一集合，排序后返回）。
///
/// 前端拿的是这一份「关掉了谁」，而不是给每个插件发一个 enabled 字段：
/// 内置插件的元数据只存在于前端，后端只维护这一份事实，两边不会各存一半。
pub fn plugin_get_disabled() -> Response<Vec<String>> {
    let mut ids: Vec<String> = disabled_ids().into_iter().collect();
    ids.sort();
    Response::new(200, ids, "ok")
}

/// 读取统一排序（内置 + 第三方 ID 的全量顺序；未记录的插件排在末尾）
pub fn plugin_get_order() -> Response<Vec<String>> {
    let order = read_settings()
        .get("plugin_order")
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|v| v.as_str().map(str::to_string))
                .collect()
        })
        .unwrap_or_default();
    Response::new(200, order, "ok")
}

/// 保存统一排序（去重、限长；只存 ID 序列，内置与第三方一视同仁）
pub fn plugin_set_order(order: Vec<String>) -> Response<bool> {
    if order.len() > 200 {
        return Response::new(500, false, "排序列表过长");
    }
    let mut seen: HashSet<String> = HashSet::new();
    let mut cleaned: Vec<String> = Vec::with_capacity(order.len());
    for id in order {
        if !is_valid_id(&id) {
            return Response::new(500, false, &format!("非法插件 ID: {}", id));
        }
        if seen.insert(id.clone()) {
            cleaned.push(id);
        }
    }
    let mut settings = read_settings();
    if let Some(obj) = settings.as_object_mut() {
        obj.insert("plugin_order".to_string(), serde_json::json!(cleaned));
    }
    write_settings(&settings);
    Response::new(200, true, "排序已保存")
}

// ==================== 安装 / 卸载 ====================

/// 递归复制目录（跳过符号链接，防越权读取）
fn copy_dir_all(src: &Path, dst: &Path) -> Result<(), String> {
    fs::create_dir_all(dst).map_err(|e| format!("创建目录失败 {}: {}", dst.display(), e))?;
    for entry in fs::read_dir(src)
        .map_err(|e| format!("读取目录失败 {}: {}", src.display(), e))?
        .flatten()
    {
        let path = entry.path();
        let Ok(meta) = fs::symlink_metadata(&path) else {
            continue;
        };
        if meta.file_type().is_symlink() {
            continue;
        }
        let target = dst.join(entry.file_name());
        if meta.is_dir() {
            copy_dir_all(&path, &target)?;
        } else if meta.is_file() {
            fs::copy(&path, &target)
                .map_err(|e| format!("复制文件失败 {}: {}", path.display(), e))?;
        }
    }
    Ok(())
}

/// 解压 zip 到目标目录（用 `enclosed_name()` 防 zip slip，并限制总体积与文件数）
fn unzip(src: &Path, dest: &Path) -> Result<(), String> {
    let file = fs::File::open(src).map_err(|e| format!("打开压缩包失败: {}", e))?;
    let mut archive =
        zip::ZipArchive::new(file).map_err(|e| format!("解析压缩包失败: {}", e))?;
    let mut total: u64 = 0;
    let mut count: usize = 0;
    for i in 0..archive.len() {
        let mut entry = archive
            .by_index(i)
            .map_err(|e| format!("读取压缩包条目失败: {}", e))?;
        // enclosed_name 会过滤 ../ 与绝对路径，返回 None 即不安全条目
        let Some(out_rel) = entry.enclosed_name().map(|p| p.to_path_buf()) else {
            continue;
        };
        let out_path = dest.join(&out_rel);
        if entry.is_dir() {
            fs::create_dir_all(&out_path)
                .map_err(|e| format!("创建目录失败 {}: {}", out_path.display(), e))?;
            continue;
        }
        count += 1;
        if count > MAX_PACKAGE_FILES {
            return Err(format!("包内文件数超过上限 {}", MAX_PACKAGE_FILES));
        }
        if let Some(parent) = out_path.parent() {
            fs::create_dir_all(parent)
                .map_err(|e| format!("创建目录失败 {}: {}", parent.display(), e))?;
        }
        let mut out = fs::File::create(&out_path)
            .map_err(|e| format!("创建文件失败 {}: {}", out_path.display(), e))?;
        let written = std::io::copy(&mut entry, &mut out)
            .map_err(|e| format!("写入文件失败 {}: {}", out_path.display(), e))?;
        total += written;
        if total > MAX_PACKAGE_BYTES {
            return Err(format!("包体积超过上限 {}MB", MAX_PACKAGE_BYTES / 1024 / 1024));
        }
    }
    Ok(())
}

/// 在解包目录内定位「含 plugin.json 的根」：允许 zip 内多包一层目录
fn locate_manifest_root(staging: &Path) -> Result<PathBuf, String> {
    if staging.join("plugin.json").is_file() {
        return Ok(staging.to_path_buf());
    }
    let Ok(entries) = fs::read_dir(staging) else {
        return Err("解包目录为空".to_string());
    };
    let mut found: Option<PathBuf> = None;
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() && path.join("plugin.json").is_file() {
            if found.is_some() {
                return Err("包内存在多个 plugin.json，无法确定入口".to_string());
            }
            found = Some(path);
        }
    }
    found.ok_or_else(|| "未找到 plugin.json".to_string())
}

/// 安装插件：`source` 可以是插件目录或 .zip 包。
/// 已存在同名插件时覆盖安装（升级），插件私有数据目录保留。
pub fn plugin_install(source: String) -> Response<InstalledPlugin> {
    let src = PathBuf::from(&source);
    if !src.exists() {
        return Response::new(500, InstalledPlugin::empty(), "安装源不存在");
    }
    let root = install_root();
    let staging = root.join(".staging").join(uuid::Uuid::new_v4().to_string());
    if let Err(e) = fs::create_dir_all(&staging) {
        return Response::new(500, InstalledPlugin::empty(), &format!("创建临时目录失败: {}", e));
    }
    // 无论后续成败都清理 staging
    let result = (|| -> Result<InstalledPlugin, String> {
        if src.is_dir() {
            copy_dir_all(&src, &staging)?;
        } else {
            let ext = src
                .extension()
                .and_then(|s| s.to_str())
                .unwrap_or("")
                .to_ascii_lowercase();
            if ext != "zip" {
                return Err("安装源必须是插件目录或 .zip 包".to_string());
            }
            unzip(&src, &staging)?;
        }
        let pkg_root = locate_manifest_root(&staging)?;
        let manifest = parse_manifest(&pkg_root)?;
        // 声明了 wasm 的就地预检：编不过 / 不是 Extism PDK 模块 / 缺导入，
        // 在安装阶段就报错，避免「装上了但一调用就崩」
        if manifest.wasm.is_some() {
            super::wasm::probe(&manifest.id, &pkg_root, &manifest)?;
        }
        let target = plugin_dir(&manifest.id)?;
        if target.exists() {
            fs::remove_dir_all(&target)
                .map_err(|e| format!("移除旧版本失败: {}", e))?;
        }
        fs::create_dir_all(&root).map_err(|e| format!("创建安装目录失败: {}", e))?;
        copy_dir_all(&pkg_root, &target)?;
        // 写安装元数据（宿主生成，不覆盖包内同名文件以外的任何内容）
        let meta = InstallMeta {
            installed_at: chrono::Utc::now().to_rfc3339(),
            source: source.clone(),
        };
        if let Ok(text) = serde_json::to_string_pretty(&meta) {
            let _ = fs::write(target.join(META_FILE), text);
        }
        // 覆盖安装（升级）后旧实例必须丢弃，否则仍跑旧代码
        super::wasm::drop_instance(&manifest.id);
        // 新装插件在已显式指定插件清单的窗口预设中默认启用，避免「装了却看不见」
        update_window_presets(&manifest.id, false);
        load_installed(&target)
    })();
    let _ = fs::remove_dir_all(&staging);
    match result {
        Ok(item) => Response::new(200, item, "安装成功"),
        Err(e) => Response::new(500, InstalledPlugin::empty(), &e),
    }
}

/// 新装插件在「已显式指定插件清单」的窗口预设中默认启用（空清单 = 显示全部，无需追加）。
/// `remove=true` 时反向执行：把 id 从各预设的清单里摘掉（卸载后清理残留）。
fn update_window_presets(id: &str, remove: bool) {
    let mut settings = read_settings();
    let Some(presets) = settings
        .get_mut("window_presets")
        .and_then(|v| v.as_array_mut())
    else {
        return;
    };
    let mut changed = false;
    for preset in presets.iter_mut() {
        let Some(list) = preset
            .get_mut("enabledPlugins")
            .and_then(|v| v.as_array_mut())
        else {
            continue;
        };
        if remove {
            let before = list.len();
            list.retain(|v| v.as_str() != Some(id));
            if list.len() != before {
                changed = true;
            }
        } else {
            if list.is_empty() || list.iter().any(|v| v.as_str() == Some(id)) {
                continue;
            }
            list.push(serde_json::json!(id));
            changed = true;
        }
    }
    if changed {
        write_settings(&settings);
    }
}

/// 卸载插件（默认保留插件私有数据；`remove_data=true` 时一并清理）
pub fn plugin_uninstall(id: String, remove_data: bool) -> Response<bool> {
    if BUILTIN_IDS.contains(&id.as_str()) {
        return Response::new(500, false, "内置插件不可卸载");
    }
    let dir = match plugin_dir(&id) {
        Ok(d) => d,
        Err(e) => return Response::new(500, false, &e),
    };
    if !dir.exists() {
        return Response::new(500, false, "插件未安装");
    }
    if let Err(e) = fs::remove_dir_all(&dir) {
        return Response::new(500, false, &format!("卸载失败: {}", e));
    }
    // 清理排序与禁用记录，避免残留指向已卸载插件
    let mut settings = read_settings();
    if let Some(obj) = settings.as_object_mut() {
        for key in ["plugin_order", "plugin_disabled"] {
            if let Some(arr) = obj.get_mut(key).and_then(|v| v.as_array_mut()) {
                arr.retain(|v| v.as_str() != Some(id.as_str()));
            }
        }
        // 这两个是**按插件 id 索引的对象**，同样要清（plugin_settings 以前没清，属同一类残留）
        for key in [KEY_SETTINGS, KEY_CAPS_DENIED] {
            if let Some(map) = obj.get_mut(key).and_then(|v| v.as_object_mut()) {
                map.remove(id.as_str());
                if map.is_empty() {
                    obj.remove(key);
                }
            }
        }
    }
    write_settings(&settings);
    // 丢弃运行时实例（否则已卸载的插件仍可能被调用到内存里的旧实例）
    super::wasm::drop_instance(&id);
    // 清理窗口预设里的残留引用
    update_window_presets(&id, true);
    if remove_data {
        purge_plugin_data(&id);
        purge_plugin_ext_data(&id);
    }
    Response::new(200, true, "已卸载")
}

/// 清理插件在所有工作区下的私有数据目录
fn purge_plugin_data(id: &str) {
    let root = plugin_data_root();
    let Ok(entries) = fs::read_dir(&root) else {
        return;
    };
    for entry in entries.flatten() {
        let dir = entry.path().join(id);
        if dir.is_dir() {
            if let Err(e) = fs::remove_dir_all(&dir) {
                println!("[PluginHost] 清理插件数据失败 {}: {}", dir.display(), e);
            }
        }
    }
}

// ==================== 调用 ====================

/// 调用插件后端逻辑（唯一入口，见模块头）。
///
/// 校验顺序：插件已安装 → 函数名/入参合规 → 清单有效 → 声明了 wasm → 进 Extism 沙箱。
/// 关闭 `wasm-runtime` feature 时，最后一步回落为 501 结构化错误。
pub fn plugin_call(id: String, func: String, input: String) -> Response<String> {
    if !is_valid_id(&id) {
        return Response::new(500, String::new(), "非法插件 ID");
    }
    let dir = match plugin_dir(&id) {
        Ok(d) => d,
        Err(e) => return Response::new(500, String::new(), &e),
    };
    if !dir.exists() {
        return Response::new(500, String::new(), "插件未安装");
    }
    // 禁用不只是前端不注册：后端也拒绝调用，避免「禁用后旧页面仍能跑」
    if disabled_ids().contains(&id) {
        return Response::new(500, String::new(), "插件已被禁用");
    }
    if func.trim().is_empty() || func.len() > MAX_FUNC_NAME {
        return Response::new(500, String::new(), "函数名非法");
    }
    if input.len() > MAX_CALL_INPUT {
        return Response::new(500, String::new(), "调用入参超过 1MB 上限");
    }
    let manifest = match parse_manifest(&dir) {
        Ok(m) => m,
        Err(e) => return Response::new(500, String::new(), &format!("插件清单无效: {}", e)),
    };
    call_wasm(&id, &dir, &manifest, &func, &input)
}

/// 进 WASM 沙箱执行（运行时在 feature 关闭时由 plugin_wasm 返回结构化错误，此处无需分支）
fn call_wasm(
    id: &str,
    dir: &Path,
    manifest: &PluginManifest,
    func: &str,
    input: &str,
) -> Response<String> {
    if manifest.wasm.is_none() {
        return Response::new(
            501,
            String::new(),
            "该插件未声明 WASM 后端：它只有沙箱页面，没有可调用的后端逻辑",
        );
    }
    match super::wasm::call(id, dir, manifest, func, input) {
        Ok(out) => {
            if out.len() > MAX_CALL_OUTPUT {
                return Response::new(
                    500,
                    String::new(),
                    &format!("插件输出超过 {}MB 上限", MAX_CALL_OUTPUT / 1024 / 1024),
                );
            }
            Response::new(200, out, "ok")
        }
        Err(e) => Response::new(500, String::new(), &e),
    }
}

/// 运行时信息（设置页展示 + 出问题时排查）
pub fn plugin_runtime_info() -> Response<super::wasm::RuntimeInfo> {
    super::wasm::runtime_info()
}

/// 某插件上一次调用的资源占用（进程内存增量 / fuel / 出错快照）
///
/// 口径见 `plugin_wasm::CallStats`：内存是**宿主进程级**的近似值，不是插件的精确线性内存
/// （那个要插件用 `memory_size` 自报）。纯内存读取，不碰磁盘。
pub fn plugin_last_stats(id: String) -> Response<Option<super::wasm::CallStats>> {
    super::wasm::last_stats(&id)
}

/// 设置「插件调用出错时导出线性内存快照」的调试开关
///
/// 开关本身是**编译期**选项（Extism 的 `with_memdump` 作用于 PluginBuilder），
/// 已折进编译缓存指纹 ⇒ 切换后下一次调用自动重编，不需要手工清缓存。
/// 开启后每次出错都会把整块线性内存（≤ 32MB）写进该插件的私有数据目录，别长期开着。
pub fn plugin_set_memdump(enabled: bool) -> Response<bool> {
    let mut settings = read_settings();
    if let Some(obj) = settings.as_object_mut() {
        obj.insert(
            "plugin_memdump_on_error".to_string(),
            serde_json::json!(enabled),
        );
    }
    write_settings(&settings);
    Response::new(
        200,
        enabled,
        if enabled { "已开启出错内存快照" } else { "已关闭出错内存快照" },
    )
}

// ==================== 资产读取（plugin:// 协议） ====================

/// 解析 `plugin://` 请求的 URI，得到 `(插件 ID, 包内相对路径)`。
///
/// URL 形态固定为 `plugin://localhost/<plugin_id>/<相对路径>`，host 必须是 `localhost`：
/// Windows / Android 下 wry 会把 `{scheme}://localhost/*` 改写成 `http://{scheme}.localhost/*`，
/// 只有 host 为 localhost 时该改写才生效，所以插件 ID 只能走**路径首段**（跨平台一致）。
/// macOS / Linux 下 wry 原生支持自定义 scheme，host 可直接承载插件 ID，这里一并兼容。
pub fn split_plugin_uri(host: &str, path: &str) -> (String, String) {
    let p = path.trim_start_matches('/').to_string();
    if host.is_empty() || host == "localhost" {
        match p.split_once('/') {
            Some((id, rest)) => (id.to_string(), rest.to_string()),
            None => (p, String::new()),
        }
    } else {
        (host.to_string(), p)
    }
}

/// 静态资源扩展名 → MIME（未列出的扩展名一律拒绝，避免把包内任意文件喂给 webview）
fn mime_for(ext: &str) -> Option<&'static str> {
    Some(match ext {
        "html" | "htm" => "text/html; charset=utf-8",
        "js" | "mjs" => "text/javascript; charset=utf-8",
        "css" => "text/css; charset=utf-8",
        "json" => "application/json; charset=utf-8",
        "svg" => "image/svg+xml",
        "png" => "image/png",
        "jpg" | "jpeg" => "image/jpeg",
        "gif" => "image/gif",
        "ico" => "image/x-icon",
        "woff2" => "font/woff2",
        "woff" => "font/woff",
        "txt" | "md" => "text/plain; charset=utf-8",
        // ---- Web3D / WASM 引擎资源 ----
        // 取得到 ≠ 用得上：没有在 `pageCapabilities` 里声明 `wasm` / `assets` / `media` 的插件，
        // 即使这里放行了扩展名，也会被 CSP 挡在编译 / fetch / 播放那一步（见 page_csp）。
        // 所以这里按扩展名放行是安全的：它只能读到**自己包内**的文件。
        "wasm" => "application/wasm", // 必须是这个 MIME，否则 compileStreaming 直接拒收
        "data" | "bin" | "pck" | "unityweb" | "br" | "gz" => "application/octet-stream",
        "glb" => "model/gltf-binary",
        "gltf" => "model/gltf+json",
        "ktx2" => "image/ktx2",
        "hdr" => "image/vnd.radiance",
        "mp3" => "audio/mpeg",
        "ogg" | "oga" => "audio/ogg",
        "wav" => "audio/wav",
        // 私有数据目录（`__data__/`）里常见的音频格式：下载的音乐要靠这些扩展名才能播
        "flac" => "audio/flac",
        "m4a" => "audio/mp4",
        "aac" => "audio/aac",
        "opus" => "audio/opus",
        "mp4" => "video/mp4",
        "webm" => "video/webm",
        _ => return None,
    })
}

/// `plugin://` 下「插件私有数据目录」的路径前缀。
///
/// `plugin://<id>/__data__/<rel>` → `{app_dir}/plugin_data_ext/<id>/<rel>`
///
/// 选**同协议 + 路径前缀**而不是新协议：协议与 host 都不变 ⇒ 与插件页**同源**，
/// CSP 的 `'self'` 天然覆盖（`media-src 'self' data: blob:` 立即生效），一行 CSP 都不用改。
/// 换成一个新协议（如 `plugin-data://`）就会变成跨源，反而要回头改 CSP。
pub const DATA_URL_PREFIX: &str = "__data__";

/// 一次资源响应的全部信息（含分段相关头，交给协议层直接拼响应）
pub struct AssetResponse {
    /// 200 全量 / 206 分段 / 416 区间不满足
    pub status: u16,
    pub mime: &'static str,
    pub body: Vec<u8>,
    /// `Content-Range` 头的值；仅 206 与 416 时给出（200 时给它是协议噪音）
    pub content_range: Option<String>,
}

/// 解析 HTTP `Range` 头（**纯函数**，便于测试）。
///
/// 只支持 `bytes=` 的**单段**区间，返回**闭区间** `(start, end)`。
/// 语法非法 / 非 bytes 单位 / 多段 / 起点越界 → `None`（由调用方决定回 416 还是 200 全量）。
pub fn parse_range(header: &str, len: u64) -> Option<(u64, u64)> {
    if len == 0 {
        return None;
    }
    let rest = header.trim().strip_prefix("bytes=")?;
    // 多段（`bytes=0-9,20-29`）按不支持处理：实现成本（multipart/byteranges）与收益不成比例
    if rest.contains(',') {
        return None;
    }
    let (s, e) = rest.split_once('-')?;
    let last = len - 1;
    if s.trim().is_empty() {
        // 后缀式 `bytes=-N`：取最后 N 字节；N 超过文件长度则从头开始（按 RFC 7233 是合法的）
        let n: u64 = e.trim().parse().ok()?;
        if n == 0 {
            return None;
        }
        Some((last.saturating_sub(n - 1), last))
    } else {
        let start: u64 = s.trim().parse().ok()?;
        if start > last {
            return None; // 起点越界 → 416（不是静默截断成空）
        }
        let end = if e.trim().is_empty() {
            last
        } else {
            // 终点越界按 RFC 截断到末尾（这是合法的，只有**起点**越界才是错误）
            let end: u64 = e.trim().parse().ok()?;
            end.min(last)
        };
        if end < start {
            return None;
        }
        Some((start, end))
    }
}

/// 已安装插件清单里的权限声明（读不到清单时返回空 = 默认全拒）
fn manifest_permissions(plugin_id: &str) -> Vec<String> {
    let Ok(dir) = plugin_dir(plugin_id) else {
        return Vec::new();
    };
    parse_manifest(&dir).map(|m| m.permissions).unwrap_or_default()
}

/// 从 `start` 读到 `end`（**闭区间**）
fn read_span(path: &PathBuf, start: u64, end: u64) -> Result<Vec<u8>, std::io::Error> {
    use std::io::{Read, Seek, SeekFrom};
    let mut f = fs::File::open(path)?;
    f.seek(SeekFrom::Start(start))?;
    let mut buf = Vec::new();
    f.take(end - start + 1).read_to_end(&mut buf)?;
    Ok(buf)
}

/// 读取插件资源（`plugin://<id>/<rel>`），支持 `__data__/` 前缀与 Range。
///
/// 两个根目录，越权校验是同一套（canonicalize + starts_with）：
/// - **包内**：`{install_root}/<id>/<rel>` —— 插件自带文件
/// - **私有数据**：`{app_dir}/plugin_data_ext/<id>/<rel>` —— 前缀 `__data__/`，
///   这是 `host_fs_*` 写下的文件能被**页面**用到的唯一通道（例：下载的音乐交给 `<audio>` 播）
pub fn read_plugin_asset_response(
    plugin_id: &str,
    rel_path: &str,
    range: Option<&str>,
) -> Result<AssetResponse, String> {
    if !is_valid_id(plugin_id) {
        return Err(format!("非法插件 ID: {}", plugin_id));
    }
    let rel = rel_path.trim_start_matches('/');
    let (dir, inner, is_data) = match rel.strip_prefix(DATA_URL_PREFIX) {
        Some(rest) => {
            let inner = rest.trim_start_matches('/');
            if !is_safe_rel_path(inner) {
                return Err(format!("非法资源路径: {}", rel_path));
            }
            // 权限门控：与 WASM 侧 `host_fs_read` 同一口径 —— 没声明 fs:read 就看不到自己的数据目录
            let perms = manifest_permissions(plugin_id);
            if !perms.iter().any(|p| p == "fs:read" || p.starts_with("fs:read:")) {
                return Err("插件未声明 fs:read 权限".to_string());
            }
            (plugin_data_ext_dir(plugin_id)?, inner.to_string(), true)
        }
        None => {
            if !is_safe_rel_path(rel) {
                return Err(format!("非法资源路径: {}", rel_path));
            }
            (install_root().join(plugin_id), rel.to_string(), false)
        }
    };
    let full = dir.join(&inner);
    // canonicalize 后校验仍在根目录内（防 ../ 穿越与软链逃逸）
    let canonical = fs::canonicalize(&full).map_err(|_| format!("资源不存在: {}", inner))?;
    let base = fs::canonicalize(&dir).map_err(|_| {
        if is_data {
            "插件数据目录不可访问".to_string()
        } else {
            "插件目录不可访问".to_string()
        }
    })?;
    if !canonical.starts_with(&base) {
        return Err(if is_data {
            "越权访问插件数据目录".to_string()
        } else {
            "越权访问插件资源".to_string()
        });
    }
    // 内存快照不该被页面读到：它是 WASM 出错那刻的**线性内存**，里面可能有任何东西
    if is_data
        && canonical.file_name().and_then(|s| s.to_str())
            == Some(crate::plugin_host::wasm::MEMDUMP_FILE)
    {
        return Err("不允许通过 URL 读取内存快照".to_string());
    }
    let ext = canonical
        .extension()
        .and_then(|s| s.to_str())
        .unwrap_or("")
        .to_ascii_lowercase();
    let mime = match mime_for(&ext) {
        Some(m) => m,
        None => return Err(format!("不允许的资源类型: .{}", ext)),
    };

    let len = fs::metadata(&canonical)
        .map_err(|e| format!("读取资源失败: {}", e))?
        .len();
    if len == 0 {
        return Ok(AssetResponse {
            status: 200,
            mime,
            body: Vec::new(),
            content_range: None,
        });
    }
    let (start, end, status) = match range.and_then(|r| parse_range(r, len)) {
        Some((s, e)) => (s, e, 206u16),
        None => {
            // Range 头存在却无法满足 → 416，并给出合法总长度（浏览器据此重试）
            if range.is_some() {
                return Ok(AssetResponse {
                    status: 416,
                    mime: "text/plain; charset=utf-8",
                    body: Vec::new(),
                    content_range: Some(format!("bytes */{}", len)),
                });
            }
            (0, len - 1, 200u16)
        }
    };
    let body = read_span(&canonical, start, end).map_err(|e| format!("读取资源失败: {}", e))?;
    Ok(AssetResponse {
        status,
        mime,
        body,
        content_range: if status == 206 {
            Some(format!("bytes {}-{}/{}", start, end, len))
        } else {
            None
        },
    })
}

/// 读取插件包内静态资源：`plugin://<plugin_id>/<相对路径>`（**整份**，不支持 Range）
///
/// 协议层请改用 `read_plugin_asset_response`：`<audio>`/`<video>` 拖动进度条会发 Range，
/// 一律 200 全量返回会让大文件无法 seek（包内 `.data` 与私有数据目录里的音视频都受影响）。
pub fn read_plugin_asset(plugin_id: &str, rel_path: &str) -> Result<(Vec<u8>, &'static str), String> {
    read_plugin_asset_response(plugin_id, rel_path, None).map(|r| (r.body, r.mime))
}

// ==================== 插件设置（contributes.configuration） ====================

/// 已安装插件的设置项定义（读不到清单时为空 = 没有设置项）
pub fn plugin_settings_defs(id: &str) -> Vec<SettingDef> {
    let Ok(dir) = plugin_dir(id) else {
        return Vec::new();
    };
    parse_manifest(&dir)
        .ok()
        .and_then(|m| m.contributes)
        .map(|c| c.configuration)
        .unwrap_or_default()
}

/// 插件设置的当前值：已存的用已存的，**没有才回落到清单 default**
///
/// 「已存的不被新 default 覆盖」是刻意的：插件升级改默认值时，用户在旧版本里
/// 显式改过的值必须保留（否则一次升级就会静默重置用户的偏好）。
///
/// 类型/范围校验也在读取时做一次：插件可能拿到上个版本留下的脏值，
/// 回落成 default 比把脏值喂给插件要好。
pub fn plugin_settings_of(id: &str) -> serde_json::Value {
    let defs = plugin_settings_defs(id);
    let stored = read_settings()
        .get(KEY_SETTINGS)
        .and_then(|v| v.get(id))
        .cloned()
        .unwrap_or_else(|| serde_json::Value::Object(Default::default()));
    let mut out = serde_json::Map::new();
    for d in &defs {
        let v = stored
            .get(&d.key)
            .cloned()
            .filter(|v| setting_value_matches(v, d.kind) && setting_in_range(v, d))
            .unwrap_or_else(|| d.default.clone());
        out.insert(d.key.clone(), v);
    }
    serde_json::Value::Object(out)
}

/// 写入插件设置值：按清单定义校验类型与范围，**只覆盖提交上来的键**
pub fn plugin_set_settings(id: &str, values: &serde_json::Value) -> Result<(), String> {
    let defs = plugin_settings_defs(id);
    if defs.is_empty() {
        return Err("插件未声明任何设置项".to_string());
    }
    let Some(obj) = values.as_object() else {
        return Err("设置值必须是对象".to_string());
    };
    let mut patch = serde_json::Map::new();
    for (k, v) in obj {
        let d = defs
            .iter()
            .find(|d| d.key == *k)
            .ok_or_else(|| format!("未声明的设置项: {}", k))?;
        if !setting_value_matches(v, d.kind) {
            return Err(format!("设置项 {} 的值类型不符（应为 {:?}）", k, d.kind));
        }
        if !setting_in_range(v, d) {
            return Err(format!("设置项 {} 的值超出允许范围", k));
        }
        patch.insert(k.clone(), v.clone());
    }
    let mut settings = read_settings();
    let mut all = settings
        .get(KEY_SETTINGS)
        .cloned()
        .unwrap_or_else(|| serde_json::Value::Object(Default::default()));
    let mut merged = all
        .get(id)
        .and_then(|v| v.as_object())
        .cloned()
        .unwrap_or_default();
    for (k, v) in patch {
        merged.insert(k, v);
    }
    if let Some(map) = all.as_object_mut() {
        map.insert(id.to_string(), serde_json::Value::Object(merged));
    }
    if let Some(map) = settings.as_object_mut() {
        map.insert(KEY_SETTINGS.to_string(), all);
    }
    write_settings(&settings);
    Ok(())
}

impl InstalledPlugin {
    /// 失败时的空占位（保证 Response<InstalledPlugin> 始终有合法 data）
    pub(crate) fn empty() -> Self {
        InstalledPlugin {
            id: String::new(),
            name: String::new(),
            version: String::new(),
            description: String::new(),
            icon: String::new(),
            entry: String::new(),
            has_wasm: false,
            permissions: Vec::new(),
            page_capabilities: Vec::new(),
            denied_page_capabilities: Vec::new(),
            standalone_window: false,
            enabled: false,
            installed_at: String::new(),
            size_bytes: 0,
            builtin: false,
            settings: Vec::new(),
        }
    }
}

#[cfg(test)]
mod settings_tests {
    use super::{setting_value_matches, validate_configuration, SettingDef, SettingType};
    use serde_json::json;

    fn def(key: &str, kind: SettingType, default: serde_json::Value) -> SettingDef {
        SettingDef {
            key: key.to_string(),
            kind,
            title: key.to_string(),
            description: String::new(),
            default,
            min: None,
            max: None,
            options: None,
        }
    }

    #[test]
    fn accepts_well_formed_definitions() {
        let defs = vec![
            def("server.host", SettingType::String, json!("127.0.0.1")),
            def("autoRefresh", SettingType::Boolean, json!(false)),
            def("interval", SettingType::Number, json!(30)),
        ];
        assert!(validate_configuration(&defs).is_ok());
        assert!(validate_configuration(&[]).is_ok(), "空声明合法");
    }

    #[test]
    fn rejects_bad_definitions() {
        // key 非法 / 重复
        assert!(validate_configuration(&[def("has space", SettingType::String, json!("x"))]).is_err());
        assert!(validate_configuration(&[
            def("a", SettingType::String, json!("x")),
            def("a", SettingType::String, json!("y")),
        ])
        .is_err());
        // default 与 type 不符
        assert!(validate_configuration(&[def("a", SettingType::Boolean, json!("yes"))]).is_err());
        assert!(validate_configuration(&[def("a", SettingType::Number, json!("30"))]).is_err());
        // 越界 / min>max
        let mut n = def("a", SettingType::Number, json!(0));
        n.min = Some(1.0);
        assert!(validate_configuration(&[n]).is_err());
        let mut bad = def("a", SettingType::Number, json!(5));
        bad.min = Some(10.0);
        bad.max = Some(1.0);
        assert!(validate_configuration(&[bad]).is_err());
        // enum：缺列表 / default 不在列表里
        assert!(validate_configuration(&[def("a", SettingType::Enum, json!("x"))]).is_err());
        let mut e = def("a", SettingType::Enum, json!("x"));
        e.options = Some(vec!["p".into(), "q".into()]);
        assert!(validate_configuration(&[e]).is_err());
        // string 类型不接受 min/max/enum（避免作者以为约束生效了）
        let mut s = def("a", SettingType::String, json!("x"));
        s.min = Some(0.0);
        assert!(validate_configuration(&[s]).is_err());
    }

    #[test]
    fn accepts_enum_in_list() {
        let mut e = def("view", SettingType::Enum, json!("auto"));
        e.options = Some(vec!["auto".into(), "list".into(), "tree".into()]);
        assert!(validate_configuration(&[e]).is_ok());
    }

    #[test]
    fn value_matches_kind() {
        assert!(setting_value_matches(&json!("x"), SettingType::String));
        assert!(!setting_value_matches(&json!(1), SettingType::String));
        assert!(setting_value_matches(&json!(true), SettingType::Boolean));
        assert!(setting_value_matches(&json!(1.5), SettingType::Number));
        assert!(!setting_value_matches(&json!("1"), SettingType::Number));
    }
}

#[cfg(test)]
mod asset_range_tests {
    use super::{parse_range, DATA_URL_PREFIX};

    #[test]
    fn parses_single_range() {
        // 全闭区间
        assert_eq!(parse_range("bytes=0-9", 100), Some((0, 9)));
        // 开尾：到文件末尾
        assert_eq!(parse_range("bytes=10-", 100), Some((10, 99)));
        // 后缀式：最后 5 字节
        assert_eq!(parse_range("bytes=-5", 100), Some((95, 99)));
        // 后缀式超过文件长度 → 从头开始（RFC 7233 合法）
        assert_eq!(parse_range("bytes=-500", 100), Some((0, 99)));
        // 终点越界 → 截断到末尾（合法）
        assert_eq!(parse_range("bytes=90-1000", 100), Some((90, 99)));
        // 单字节文件
        assert_eq!(parse_range("bytes=0-0", 1), Some((0, 0)));
    }

    #[test]
    fn rejects_bad_ranges() {
        // 起点越界 → None（调用方回 416）
        assert_eq!(parse_range("bytes=100-", 100), None);
        assert_eq!(parse_range("bytes=101-", 100), None);
        // 终点小于起点
        assert_eq!(parse_range("bytes=9-5", 100), None);
        // 非 bytes 单位 / 语法非法 / 多段 / 空文件
        assert_eq!(parse_range("items=0-9", 100), None);
        assert_eq!(parse_range("0-9", 100), None);
        assert_eq!(parse_range("bytes=abc-def", 100), None);
        assert_eq!(parse_range("bytes=0-9,20-29", 100), None);
        assert_eq!(parse_range("bytes=0-9", 0), None);
        assert_eq!(parse_range("bytes=-0", 100), None);
    }

    #[test]
    fn data_prefix_is_stable() {
        // 前缀改名会静默弄坏所有已发布插件的 URL，这里钉住
        assert_eq!(DATA_URL_PREFIX, "__data__");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn id_rules() {
        assert!(is_valid_id("demo-plugin"));
        assert!(is_valid_id("acme.tool_v2"));
        assert!(!is_valid_id("Demo")); // 大写
        assert!(!is_valid_id(""));
        assert!(!is_valid_id(".hidden"));
        assert!(!is_valid_id("a..b"));
        // 内置保留字不是 id 格式问题，由 validate_manifest / plugin_uninstall 单独拦截
        assert!(is_valid_id("files"));
        assert!(BUILTIN_IDS.contains(&"files"));
    }

    #[test]
    fn builtin_ids_are_rejected() {
        for id in BUILTIN_IDS {
            let m = PluginManifest {
                contributes: None,
                id: id.to_string(),
                name: "x".to_string(),
                version: "1".to_string(),
                description: String::new(),
                icon: None,
                entry: "page.html".to_string(),
                wasm: None,
                init: None,
                permissions: Vec::new(),
                page_capabilities: Vec::new(),
                standalone_window: false,
            };
            assert!(validate_manifest(&m).is_err(), "{} 应被拒绝", id);
        }
    }

    /// 首次运行的内置插件默认状态：只有 `BUILTIN_DEFAULT_ENABLED` 里的开启，其余关闭。
    #[test]
    fn builtin_defaults_enable_only_files() {
        let mut settings = serde_json::json!({});
        let newly = apply_plugin_defaults(&mut settings);
        let disabled = disabled_ids_of(&settings);

        for id in BUILTIN_IDS {
            if BUILTIN_DEFAULT_ENABLED.contains(&id) || HOST_PINNED_IDS.contains(&id) {
                assert!(!disabled.contains(id), "{} 不该出现在禁用集合里", id);
            } else {
                assert!(disabled.contains(id), "{} 应默认禁用", id);
            }
        }
        assert!(
            disabled.contains("files") == false,
            "files 是内置插件里唯一默认开启的"
        );
        // 宿主常驻入口（设置）必须始终开着，否则用户进不了设置页、无法启用任何插件
        assert!(HOST_PINNED_IDS.iter().all(|id| !disabled.contains(*id)));
        assert_eq!(
            newly.len(),
            BUILTIN_IDS.len() - BUILTIN_DEFAULT_ENABLED.len() - HOST_PINNED_IDS.len()
        );
    }

    /// 默认状态**只写一次**：既不能覆盖已有的第三方禁用记录，也不能纠正用户后来的选择。
    #[test]
    fn builtin_defaults_are_applied_once_and_merge_existing() {
        // 老用户的 settings：已经禁用过一个第三方插件
        let mut settings = serde_json::json!({ "plugin_disabled": ["acme-tool"] });
        apply_plugin_defaults(&mut settings);
        assert!(
            disabled_ids_of(&settings).contains("acme-tool"),
            "已有记录不能被默认值覆盖"
        );

        // 用户随后把 ssh 打开、把 files 关掉
        let mut after = settings.clone();
        let mut d = disabled_ids_of(&after);
        d.remove("ssh");
        d.insert("files".to_string());
        set_disabled_ids(&mut after, &d);

        // 再次启动：标记已在，不再纠正用户的选择
        assert!(apply_plugin_defaults(&mut after).is_empty());
        let final_disabled = disabled_ids_of(&after);
        assert!(
            !final_disabled.contains("ssh"),
            "用户开启过的插件不能被默认值关回来"
        );
        assert!(
            final_disabled.contains("files"),
            "用户关掉的 files 也不该被默认值强行打开"
        );
    }

    /// 开关的读写闭环：禁用 → 进集合；启用 → 出集合；重复操作幂等。
    #[test]
    fn enabled_toggle_round_trip() {
        let mut settings = serde_json::json!({});
        apply_enabled_toggle(&mut settings, "ssh", false);
        assert!(disabled_ids_of(&settings).contains("ssh"));
        apply_enabled_toggle(&mut settings, "ssh", false); // 幂等：不该出现重复项
        assert_eq!(disabled_ids_of(&settings).len(), 1);
        apply_enabled_toggle(&mut settings, "ssh", true);
        assert!(
            disabled_ids_of(&settings).is_empty(),
            "启用后必须从禁用集合里移除"
        );
        apply_enabled_toggle(&mut settings, "ssh", true); // 幂等
        assert!(disabled_ids_of(&settings).is_empty());
    }

    /// 宿主常驻入口即使**历史数据**里被禁用了，初始化时也要清掉（它关掉就再也进不了设置页）。
    #[test]
    fn host_pinned_is_cleared_from_disabled() {
        let mut settings = serde_json::json!({ "plugin_disabled": ["settings"] });
        apply_plugin_defaults(&mut settings);
        assert!(
            !disabled_ids_of(&settings).contains("settings"),
            "宿主常驻入口不允许留在禁用集合里"
        );
    }

    /// 禁用集合落盘时是稳定排序的（否则 settings.json 每次保存都抖动）。
    #[test]
    fn disabled_ids_are_written_sorted() {
        let mut settings = serde_json::json!({});
        let mut d = HashSet::new();
        d.insert("zeta".to_string());
        d.insert("alpha".to_string());
        d.insert("mid".to_string());
        set_disabled_ids(&mut settings, &d);
        let arr: Vec<String> = settings[KEY_DISABLED]
            .as_array()
            .expect("应是数组")
            .iter()
            .map(|v| v.as_str().unwrap_or_default().to_string())
            .collect();
        assert_eq!(arr, vec!["alpha", "mid", "zeta"]);
    }

    /// 宿主常驻入口必须是内置插件（否则常量表与前端清单不同步，禁用它就会锁死设置入口）。
    #[test]
    fn host_pinned_entries_are_builtin() {
        for id in HOST_PINNED_IDS {
            assert!(
                BUILTIN_IDS.contains(&id),
                "{} 必须登记在 BUILTIN_IDS 里",
                id
            );
        }
        for id in BUILTIN_DEFAULT_ENABLED {
            assert!(BUILTIN_IDS.contains(&id), "{} 必须登记在 BUILTIN_IDS 里", id);
        }
    }

    #[test]
    fn entry_must_be_html() {
        let base = PluginManifest {
            contributes: None,
            id: "demo".to_string(),
            name: "demo".to_string(),
            version: "1.0.0".to_string(),
            description: String::new(),
            icon: None,
            entry: "page.html".to_string(),
            wasm: None,
            init: None,
            permissions: Vec::new(),
            page_capabilities: Vec::new(),
            standalone_window: false,
        };
        assert!(validate_manifest(&base).is_ok());
        let mut bad = base.clone();
        bad.entry = "../etc/passwd".to_string();
        assert!(validate_manifest(&bad).is_err());
        let mut bad2 = base.clone();
        bad2.entry = "main.js".to_string();
        assert!(validate_manifest(&bad2).is_err());
    }

    #[test]
    fn permission_whitelist() {
        assert!(is_permission_allowed("log"));
        assert!(is_permission_allowed("kv"));
        assert!(is_permission_allowed("fs:read"));
        assert!(is_permission_allowed("net:api.example.com"));
        assert!(!is_permission_allowed("net"));
        assert!(!is_permission_allowed("exec"));
        assert!(!is_permission_allowed("fs:read:*"));
    }

    #[test]
    fn fs_scope_rules() {
        assert!(is_permission_allowed("fs:read:$HOME"));
        assert!(is_permission_allowed("fs:read:$HOME/docs"));
        assert!(is_permission_allowed("fs:write:~/out"));
        assert!(is_permission_allowed("fs:read:/tmp/demo"));
        assert!(is_permission_allowed("fs:write:C:/tmp/demo"));
        // 穿越 / 通配 / 空白 / 相对路径一律拒绝
        assert!(!is_permission_allowed("fs:read:$HOME/../secret"));
        assert!(!is_permission_allowed("fs:read:/tmp/*"));
        assert!(!is_permission_allowed("fs:read:"));
        assert!(!is_permission_allowed("fs:read:relative/path"));
    }

    #[test]
    fn page_capability_whitelist() {
        for c in PAGE_CAPABILITIES {
            assert!(is_page_capability_allowed(c), "{} 应被允许", c);
        }
        // 大小写敏感；且必须是「只影响插件页自身」的项
        assert!(!is_page_capability_allowed("Wasm"));
        assert!(!is_page_capability_allowed("eval"));
        assert!(!is_page_capability_allowed("unsafe-eval"));
        // 明确不提供：加了它插件页就能拿到真实源，进而导航到 tauri.localhost 逃逸出沙箱
        assert!(!is_page_capability_allowed("same-origin"));
        assert!(!is_page_capability_allowed("native"));
    }

    #[test]
    fn page_csp_profiles() {
        // 默认必须与历史严格 CSP 逐字一致 —— 改这里等于放宽**所有**第三方插件
        assert_eq!(page_csp(&[]), CSP_STRICT);
        assert!(!CSP_STRICT.contains("wasm-unsafe-eval"));
        assert!(CSP_STRICT.contains("connect-src 'none'"));

        // 只声明 wasm：放开 WASM 编译，但不放开 JS eval，也不放开 fetch
        let wasm_only = page_csp(&["wasm".to_string()]);
        assert!(wasm_only.contains("'wasm-unsafe-eval'"));
        assert!(!wasm_only.contains("'unsafe-eval'"));
        assert!(wasm_only.contains("connect-src 'none'"));

        // 每项独立生效，互不牵连
        let assets = page_csp(&["assets".to_string()]);
        assert!(assets.contains("connect-src 'self'"));
        assert!(!assets.contains("wasm-unsafe-eval"));

        let media = page_csp(&["media".to_string()]);
        assert!(media.contains("media-src 'self' data: blob:"));
        assert!(media.contains("img-src 'self' data: blob:"));

        let worker = page_csp(&["worker".to_string()]);
        assert!(worker.contains("worker-src 'self' blob:"));
        assert!(!worker.contains("media-src"));

        // isolate 走 COOP/COEP 响应头，不进 CSP
        let all: Vec<String> = ["wasm", "assets", "media", "worker", "isolate"]
            .iter()
            .map(|s| s.to_string())
            .collect();
        let csp = page_csp(&all);
        assert!(!csp.contains("isolate"));
        assert!(csp.contains("script-src 'self' 'wasm-unsafe-eval'"));
        assert!(csp.contains("connect-src 'self'"));
    }

    #[test]
    fn manifest_validates_page_capabilities() {
        let base = PluginManifest {
            contributes: None,
            id: "game".to_string(),
            name: "game".to_string(),
            version: "1.0.0".to_string(),
            description: String::new(),
            icon: None,
            entry: "page.html".to_string(),
            wasm: None,
            init: None,
            permissions: Vec::new(),
            page_capabilities: Vec::new(),
            standalone_window: false,
        };
        assert!(validate_manifest(&base).is_ok());
        let mut ok = base.clone();
        ok.page_capabilities = vec!["wasm".to_string(), "isolate".to_string()];
        assert!(validate_manifest(&ok).is_ok());
        // 未知项必须拒绝安装（与 permissions 一致，不静默降级）
        let mut bad = base.clone();
        bad.page_capabilities = vec!["eval".to_string()];
        assert!(validate_manifest(&bad).is_err());
        let mut bad2 = base.clone();
        bad2.page_capabilities = vec!["wasm".to_string(), "same-origin".to_string()];
        assert!(validate_manifest(&bad2).is_err());
    }

    /// 回归：清单 JSON 的键名必须是 camelCase。
    ///
    /// 曾经的 bug：`PluginManifest` 漏了 `rename_all`，字段却是 `page_capabilities` 且带
    /// `#[serde(default)]` → 文档 / 示例 / 面板教学里写的 `pageCapabilities` 被 serde
    /// **静默忽略**，表现为「明明声明了页面能力，插件页却仍跑在最严 CSP 下」，且一路不报错。
    ///
    /// 这条测试**故意从真实 JSON 字符串反序列化** —— 直接构造结构体的写法永远抓不到这类
    /// 「键名对不上」的 bug（既有几条用例就是这样漏掉的）。
    #[test]
    fn manifest_json_keys_are_camel_case() {
        let raw = r#"{
            "id": "game",
            "name": "游戏",
            "version": "1.0.0",
            "entry": "index.html",
            "pageCapabilities": ["wasm", "assets"]
        }"#;
        let m: PluginManifest = serde_json::from_str(raw).expect("清单应能解析");
        assert_eq!(m.page_capabilities, vec!["wasm", "assets"]);
        assert_eq!(m.entry, "index.html");
        assert!(validate_manifest(&m).is_ok());

        // 大小写写错时 serde 会按「未声明」处理（严格），且不能串到别的字段。
        // 注意：这种写法在**安装时**就会被 check_manifest_keys 拒掉（见下一条测试），
        // 这里只锁定「万一漏到运行期也不会串位」。
        let wrong = raw.replace("pageCapabilities", "PageCapabilities");
        let m2: PluginManifest = serde_json::from_str(&wrong).expect("未知字段不该导致解析失败");
        assert!(m2.page_capabilities.is_empty());
        assert_eq!(m2.id, "game");
        assert_eq!(m2.entry, "index.html");

        // snake_case 别名兜底
        let snake = raw.replace("pageCapabilities", "page_capabilities");
        let m3: PluginManifest = serde_json::from_str(&snake).expect("别名应可解析");
        assert_eq!(m3.page_capabilities, vec!["wasm", "assets"]);

        // rename_all 不能波及单字字段（既有插件包必须原样兼容）
        let legacy = r#"{"id":"a","name":"b","version":"1.0.0","description":"d",
            "icon":"Monitor","wasm":"plugin.wasm","init":"init","permissions":["log"]}"#;
        let m4: PluginManifest = serde_json::from_str(legacy).expect("既有清单必须保持兼容");
        assert_eq!(m4.icon.as_deref(), Some("Monitor"));
        assert_eq!(m4.wasm.as_deref(), Some("plugin.wasm"));
        assert_eq!(m4.init.as_deref(), Some("init"));
        assert_eq!(m4.permissions, vec!["log"]);
    }

    /// `standaloneWindow` 的解析与「不能由 pageCapabilities 推断」这条约定。
    ///
    /// 为什么单独立一条：入口形态决定了插件页**有没有宿主桥**（独立窗口的 label 不在任何
    /// capability 里），代价不可逆。所以它必须是作者显式写的开关，不能因为「声明了 wasm」
    /// 就被自动判成独立窗口 —— 否则「带 WASM 后端 + 需要 plugin_call」的插件会被静默废掉。
    /// 插件包根目录必须落在 `app_dir`（用户资产），
    /// 绝不能回到 `config_dir`（会被 NSIS 卸载脚本 `RmDir /r` 删掉）
    #[test]
    fn plugin_storage_root_is_app_dir() {
        let app = Path::new("/data/bdt");
        assert_eq!(install_root_in(app), app.join("plugins_ext"));
    }

    /// 一次性迁移要把包目录与 KV 目录**都**搬走，且不能留下旧目录
    #[test]
    fn plugin_storage_migration_moves_both_dirs() {
        let tmp = std::env::temp_dir().join(format!("bdt-plugin-migrate-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&tmp);
        let cfg = tmp.join("config");
        let app = tmp.join("data");
        std::fs::create_dir_all(cfg.join("plugins_ext").join("demo")).expect("造旧目录");
        std::fs::write(cfg.join("plugins_ext").join("demo").join("plugin.json"), "{}")
            .expect("写旧包");
        std::fs::create_dir_all(cfg.join("plugins_ext_kv")).expect("造旧 KV 目录");
        std::fs::write(cfg.join("plugins_ext_kv").join("demo.json"), "{}").expect("写旧 KV");
        std::fs::create_dir_all(&app).expect("造新目录");

        migrate_plugin_dirs(&cfg, &app);

        assert!(
            app.join("plugins_ext").join("demo").join("plugin.json").is_file(),
            "包目录应已迁到 app_dir"
        );
        assert!(
            app.join("plugins_ext_kv").join("demo.json").is_file(),
            "KV 目录应已迁到 app_dir"
        );
        assert!(!cfg.join("plugins_ext").exists(), "旧包目录不应残留");
        assert!(!cfg.join("plugins_ext_kv").exists(), "旧 KV 目录不应残留");

        let _ = std::fs::remove_dir_all(&tmp);
    }

    #[test]
    fn standalone_window_is_explicit() {
        // 只声明页面能力 ≠ 独立窗口（两者正交）
        let caps_only = r#"{"id":"a","name":"b","version":"1","pageCapabilities":["wasm"]}"#;
        let m: PluginManifest = serde_json::from_str(caps_only).expect("可解析");
        assert!(!m.standalone_window, "仅声明 pageCapabilities 不得推断为独立窗口");

        // 显式声明才算
        let raw = r#"{"id":"a","name":"b","version":"1","standaloneWindow":true}"#;
        let m2: PluginManifest = serde_json::from_str(raw).expect("可解析");
        assert!(m2.standalone_window);
        assert!(validate_manifest(&m2).is_ok());
        // 不声明时序列化也要看得见（前端按 false 走面板内 iframe）
        let m3: PluginManifest = serde_json::from_str(r#"{"id":"a","name":"b","version":"1"}"#)
            .expect("可解析");
        let v = serde_json::to_value(&m3).expect("可序列化");
        assert_eq!(v.get("standaloneWindow"), Some(&serde_json::json!(false)));
    }

    /// 未识别 / 大小写写错的键名必须**在解析阶段就报错**，不能静默忽略。
    #[test]
    fn manifest_key_typos_are_rejected() {
        // 正常写法（camelCase）与 snake_case 别名都放行
        assert!(check_manifest_keys(r#"{"id":"a","pageCapabilities":["wasm"]}"#).is_ok());
        assert!(check_manifest_keys(r#"{"id":"a","page_capabilities":["wasm"]}"#).is_ok());
        // 自定义元数据用 x- / _ 前缀，静默放行
        assert!(check_manifest_keys(r#"{"id":"a","_comment":"随便写","x-author":"me"}"#).is_ok());
        // 空对象 / 非对象 JSON 不在这一层判错（交给必填字段校验与解析报错）
        assert!(check_manifest_keys("{}").is_ok());
        assert!(check_manifest_keys("[1,2]").is_ok());
        assert!(check_manifest_keys("{ 坏 json").is_ok());

        // 大小写写错：报错里必须给出正确写法
        let e = check_manifest_keys(r#"{"PageCapabilities":["wasm"]}"#)
            .expect_err("大小写写错必须被拒");
        assert!(e.contains("PageCapabilities → pageCapabilities"), "{}", e);
        // 全小写同样给出「→ 正确写法」（camelCase 的字段最容易这么写错）
        let e3 = check_manifest_keys(r#"{"standalonewindow":true}"#)
            .expect_err("全小写必须被拒");
        assert!(e3.contains("standalonewindow → standaloneWindow"), "{}", e3);
        // 单复数写错（permission）与拼错的字段名同样被拒
        assert!(check_manifest_keys(r#"{"permission":["log"]}"#).is_err());
        assert!(check_manifest_keys(r#"{"author":"me"}"#).is_err());
        // 报错要列出合法字段，便于直接照抄
        let e2 = check_manifest_keys(r#"{"nope":1}"#).expect_err("未知字段必须被拒");
        assert!(e2.contains("pageCapabilities") && e2.contains("x-"), "{}", e2);
    }

    /// 仓库里的示例包必须能被**真实解析器**吃下去。
    ///
    /// 文档和示例是用户照抄的对象，它们跑不通就等于文档在骗人 —— `pageCapabilities` 静默失效
    /// 期间，`webgl-demo` 正是这个状态（清单写着能力、宿主读到的却是空）。这条测试直接拿仓库
    /// 里的 `plugin.json` 过一遍 `parse_manifest`，把「文档 / 示例 / 实现」三者钉在一起。
    #[test]
    fn example_packages_parse() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../examples/third-party-plugins");
        for name in ["json-toolkit", "text-toolkit", "webgl-demo"] {
            let m = parse_manifest(&root.join(name))
                .unwrap_or_else(|e| panic!("示例包 {} 清单不合法: {}", name, e));
            assert_eq!(m.id, name, "示例包目录名应与 id 一致");
        }
        // 页面能力的示范包必须真的读到了声明（五项全声明），否则示例没有任何示范作用
        let demo = parse_manifest(&root.join("webgl-demo")).expect("webgl-demo 清单");
        assert_eq!(demo.page_capabilities.len(), 5, "webgl-demo 应声明全部五项");
        assert!(page_csp(&demo.page_capabilities).contains("wasm-unsafe-eval"));

        // 直接走协议层用的那个函数（page_policy 去掉全局路径依赖后的本体）：
        // 这条就是本次 bug 的正面结论 —— 清单在磁盘上，策略就该是放松的那套
        let policy = page_policy_in(&root.join("webgl-demo"), &[]);
        assert!(policy.csp.contains("'wasm-unsafe-eval'"), "{}", policy.csp);
        assert!(policy.csp.contains("connect-src 'self'"), "{}", policy.csp);
        assert!(policy.csp.contains("worker-src 'self' blob:"), "{}", policy.csp);
        assert!(policy.csp.contains("media-src 'self' data: blob:"), "{}", policy.csp);
        assert!(policy.isolate, "webgl-demo 声明了 isolate，应带 COOP/COEP");

        // 没声明页面能力的包必须仍是**逐字**的最严 CSP（放宽只能由声明触发）
        let plain = page_policy_in(&root.join("text-toolkit"), &[]);
        assert_eq!(plain.csp, CSP_STRICT);
        assert!(!plain.isolate);
        // 清单读不到（目录不存在）也必须回落最严，不能因为出错就放行
        assert_eq!(page_policy_in(&root.join("no-such-plugin"), &[]).csp, CSP_STRICT);
    }

    /// 用户在设置里收紧的能力必须真的从 CSP 里消失（**只能减，不能加**）。
    ///
    /// 这条守的是"开关是不是真的有效"：收紧是浏览器侧 CSP 强制执行的，
    /// 与 `permissions` 那种调用期校验不同，这里必须表现为响应头逐字变化。
    #[test]
    fn denied_page_caps_tighten_csp() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../examples/third-party-plugins");
        let demo = root.join("webgl-demo");

        // 收紧 isolate：CSP 不变，但 COOP/COEP 必须关掉（否则多线程 WASM 仍然解锁）
        let no_iso = page_policy_in(&demo, &["isolate".to_string()]);
        assert!(!no_iso.isolate, "收紧 isolate 后不该再带跨源隔离头");
        assert!(no_iso.csp.contains("'wasm-unsafe-eval'"), "其它项不受影响");

        // 收紧 wasm：编译 WASM 的指令必须消失
        let no_wasm = page_policy_in(&demo, &["wasm".to_string()]);
        assert!(
            !no_wasm.csp.contains("wasm-unsafe-eval"),
            "收紧 wasm 后仍放行了 WASM 编译: {}",
            no_wasm.csp
        );
        assert!(no_wasm.isolate, "isolate 没被收紧就不该跟着关");

        // 全部收紧 = 回到最严（与"压根没声明"完全等价）
        let all = page_policy_in(
            &demo,
            &[
                "wasm".to_string(),
                "assets".to_string(),
                "media".to_string(),
                "worker".to_string(),
                "isolate".to_string(),
            ],
        );
        assert_eq!(all.csp, CSP_STRICT);
        assert!(!all.isolate);

        // 收紧一个插件没声明的项是空操作：既不报错，也不凭空多出授权
        let unchanged = page_policy_in(&root.join("text-toolkit"), &["wasm".to_string()]);
        assert_eq!(unchanged.csp, CSP_STRICT);
    }

    /// 收紧集合的取值必须落在白名单内：settings.json 人手可编辑，
    /// 写错了要在**保存时**报错，而不是静默存一个永远匹配不上的字符串。
    #[test]
    fn set_denied_page_caps_rejects_unknown_value() {
        let e = is_page_capability_allowed("eval");
        assert!(!e, "eval 不在白名单内（它会放开 JS eval，不能作为页面能力）");
        assert!(is_page_capability_allowed("wasm"));
        assert!(is_page_capability_allowed("isolate"));
    }

    /// `MANIFEST_KEYS` 必须覆盖清单的全部字段，否则「忘了同步」会把正确写法误报成未识别字段。
    #[test]
    fn manifest_keys_list_covers_all_fields() {
        let m = PluginManifest {
            contributes: None,
            id: "a".to_string(),
            name: "b".to_string(),
            version: "1.0.0".to_string(),
            description: "d".to_string(),
            icon: Some("Monitor".to_string()),
            entry: "page.html".to_string(),
            wasm: Some("plugin.wasm".to_string()),
            init: Some("init".to_string()),
            permissions: vec!["log".to_string()],
            page_capabilities: vec!["wasm".to_string()],
            standalone_window: true,
        };
        let v = serde_json::to_value(&m).expect("可序列化");
        for k in v.as_object().expect("对象").keys() {
            assert!(
                MANIFEST_KEYS.contains(&k.as_str()),
                "MANIFEST_KEYS 缺字段（新增清单字段后要同步它）: {}",
                k
            );
        }
    }

    #[test]
    fn engine_asset_mimes() {
        // .wasm 的 MIME 必须是 application/wasm，否则 compileStreaming 直接拒收
        assert_eq!(mime_for("wasm"), Some("application/wasm"));
        assert_eq!(mime_for("glb"), Some("model/gltf-binary"));
        assert_eq!(mime_for("gltf"), Some("model/gltf+json"));
        assert_eq!(mime_for("data"), Some("application/octet-stream"));
        assert_eq!(mime_for("pck"), Some("application/octet-stream"));
        assert_eq!(mime_for("mp3"), Some("audio/mpeg"));
        // 放行图标/引擎资源不等于放行可执行文件：这些仍必须是 None
        assert_eq!(mime_for("exe"), None);
        assert_eq!(mime_for("dll"), None);
        assert_eq!(mime_for("sh"), None);
        assert_eq!(mime_for("ps1"), None);
    }

    #[test]
    fn settings_is_reserved() {
        // 「设置」已剥离为内置插件，第三方不能顶替
        assert!(BUILTIN_IDS.contains(&"settings"));
        let m = PluginManifest {
            contributes: None,
            id: "settings".to_string(),
            name: "x".to_string(),
            version: "1".to_string(),
            description: String::new(),
            icon: None,
            entry: "page.html".to_string(),
            wasm: None,
            init: None,
            permissions: Vec::new(),
            page_capabilities: Vec::new(),
            standalone_window: false,
        };
        assert!(validate_manifest(&m).is_err());
    }

    #[test]
    fn uri_split() {
        // 标准形态（Windows/Android 与 macOS/Linux 通用）
        assert_eq!(
            split_plugin_uri("localhost", "/demo/page.html"),
            ("demo".to_string(), "page.html".to_string())
        );
        assert_eq!(
            split_plugin_uri("localhost", "/demo/assets/app.js"),
            ("demo".to_string(), "assets/app.js".to_string())
        );
        // macOS/Linux 原生形态
        assert_eq!(
            split_plugin_uri("demo", "/page.html"),
            ("demo".to_string(), "page.html".to_string())
        );
    }

    #[test]
    fn safe_rel_paths() {
        assert!(is_safe_rel_path("page.html"));
        assert!(is_safe_rel_path("assets/app.js"));
        assert!(!is_safe_rel_path("/abs/page.html"));
        assert!(!is_safe_rel_path("../outside.html"));
        assert!(!is_safe_rel_path("C:\\x\\y.html"));
    }
}
