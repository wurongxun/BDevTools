/**
 * 第三方插件类型契约。
 *
 * 与后端 `command/plugin_host.rs` 中的 `PluginManifest` / `InstalledPlugin` 一一对应
 * （字段名保持后端原名，避免两侧命名不一致产生哑 bug）。
 */

/** 活动栏图标白名单：第三方清单只能用这些内置图标名，宿主不加载插件提供的组件 */
export const PLUGIN_ICON_WHITELIST: string[] = [
  'Grid', 'MagicStick', 'Monitor', 'Cpu', 'Platform', 'Compass', 'Aim',
  'Notebook', 'Postcard', 'Tickets', 'DataLine', 'Files', 'Connection',
  'Coin', 'Histogram', 'Calendar', 'Camera', 'Brush', 'Star', 'Setting',
  'HomeFilled', 'Box', 'Film', 'Printer', 'Promotion', 'Search',
]

/** 沙箱桥协议版本：宿主与插件页面必须一致，否则消息一律丢弃 */
export const PLUGIN_BRIDGE_VERSION = 1

/** 内置插件 ID（第三方禁止占用；与后端 `BUILTIN_IDS` 保持一致） */
/**
 * 内置插件 ID（契约表，必须与后端 `plugin_host::BUILTIN_IDS` 一致）。
 *
 * 用途是「第三方插件不得占用这些 id」的对照口径；内置插件的**启用状态**不在这里判断
 * （内置与第三方统一由 `registry.ts` 的 `disabledIds` 收口）。
 */
export const BUILTIN_PLUGIN_IDS: string[] = [
  'files', 'ssh', 'database', 'logs', 'mat', 'terminal', 'placeholder', 'settings', 'web3d',
]

/**
 * 插件页能力白名单（清单 `pageCapabilities`；与后端 `plugin_host::PAGE_CAPABILITIES` 一致）。
 *
 * 语义是「**插件页自己**在 webview 里能做什么」，宿主据此决定 `plugin://` 返回哪套 CSP；
 * 与 `permissions`（WASM 沙箱里的宿主能力）是两套东西。默认不声明 = 最严 CSP，逐项对应：
 * - `wasm`    → `script-src 'self' 'wasm-unsafe-eval'`（只放开 WASM 编译，**不**放开 JS eval）
 * - `assets`  → `connect-src 'self'`（能 fetch 自己包内的 .glb/.data，仍到不了外网）
 * - `media`   → `media-src 'self' data: blob:`
 * - `worker`  → `worker-src 'self' blob:`
 * - `isolate` → 加 COOP/COEP 进入跨源隔离（解锁 SharedArrayBuffer → 多线程 WASM）
 *
 * 刻意**没有**任何能拿到宿主权限的项；尤其没有 `allow-same-origin` 式的开关（见 docs/plugin-dev/web3d.md）。
 */
export const PAGE_CAPABILITIES: string[] = ['wasm', 'assets', 'media', 'worker', 'isolate']

/** 页面能力的中文说明（设置页 / 3D 与游戏容器展示用） */
export const PAGE_CAPABILITY_LABELS: Record<string, string> = {
  wasm: '编译 WASM（游戏引擎）',
  assets: '读取包内资源',
  media: '音视频',
  worker: 'Web Worker',
  isolate: '跨源隔离（多线程）',
}

/** 页面入口允许的扩展名（与后端 `ENTRY_EXTS` 一致） */
export const PLUGIN_ENTRY_EXTS = ['html', 'htm']

/** 包内资源图标允许的形态：纯相对路径 + 静态图片扩展名（拒绝协议头与绝对路径） */
export const PLUGIN_ICON_ASSET_RE =
  /^\.?\/?[A-Za-z0-9._-]+(\/[A-Za-z0-9._-]+)*\.(svg|png|jpe?g|gif|ico)$/i

/** 插件设置项类型（清单 `contributes.configuration[].type`） */
export type SettingType = 'string' | 'boolean' | 'number' | 'enum'

/**
 * 一个插件设置项的定义（清单 `contributes.configuration[]`）。
 *
 * 由**宿主**校验与渲染：插件不必自己画设置页，设置也能随宿主设置统一导出 / 重置。
 * 与 `permissions` 是两套东西 —— 这里**不授予任何能力**。
 */
export interface SettingDef {
  key: string
  type: SettingType
  title: string
  description?: string
  default: unknown
  /** `number` 的下界 */
  min?: number
  /** `number` 的上界 */
  max?: number
  /** `enum` 的候选值 */
  enum?: string[]
}

/** 第三方插件清单（包内 plugin.json） */
export interface PluginManifest {
  id: string
  name: string
  version: string
  description?: string
  /** 内置图标名（白名单）或包内资源相对路径 */
  icon?: string
  /** 入口页面（包内相对路径，默认 page.html） */
  entry?: string
  /** 后端 WASM 文件名（相对路径；声明后由 Extism 沙箱加载） */
  wasm?: string
  /** 实例化后先调用的入口函数名（可选；与目标函数共享同一实例） */
  init?: string
  /**
   * 权限声明（后端白名单）：
   * `log` / `kv` / `fs:read[:绝对路径|$HOME[/子路径]]` / `fs:write[:...]` / `net:<host>`
   *
   * 作用域位置写 `*` 是**通配**（`fs:read:*` = 任意路径），但其它插件的数据目录与插件安装
   * 目录仍是禁区 —— 见 `docs/plugin-dev/capabilities.md` 的「任意路径与跨插件隔离」。
   */
  permissions?: string[]
  /**
   * 插件页能力声明（后端白名单；决定 `plugin://` 用哪套 CSP）。
   * 可选值见 `PAGE_CAPABILITIES`；未声明 = 最严 CSP（不能编译 WASM / 不能 fetch 包内资源）。
   */
  pageCapabilities?: string[]
  /**
   * 本插件的**主界面在独立窗口里**（默认 false = 面板内沙箱 iframe）。
   *
   * 与 `pageCapabilities` **正交**，不能互相推断：独立窗口的插件页拿不到宿主命令，
   * 所以「带 WASM 后端 + 需要 `plugin_call`」的插件必须留在面板内。
   */
  standaloneWindow?: boolean
}

/** WASM 运行时信息（后端 `plugin_runtime_info` 返回） */
export interface PluginRuntimeInfo {
  /** 是否启用（构建期决定） */
  available: boolean
  /** 运行时名称（未启用时为空串） */
  runtime: string
  /** 单次调用指令数上限 */
  fuel: number
  /** 单次调用超时（毫秒） */
  timeout_ms: number
  /** 线性内存上限（MB） */
  max_memory_mb: number
  /** HTTP 响应上限（MB） */
  max_http_response_mb: number
  /** 单次 FS 读上限（MB） */
  max_fs_read_mb: number
  /** 流式通道的单块上限（MB） */
  max_fs_chunk_mb: number
  /** 可同时打开的 FS 句柄数 */
  max_fs_handles: number
  /** 支持的权限声明形态 */
  permissions: string[]
  /** 调试开关：插件调用出错时是否把线性内存快照导出到插件数据目录 */
  memdump_on_error: boolean
}

/**
 * 某插件「上一次调用」的资源占用（后端 `plugin_last_stats` 返回）。
 *
 * 口径说明（与 `docs/plugin-dev/capabilities.md` 一致）：
 * - `mem_delta_bytes` / `rss_delta_bytes` 是**宿主进程**在本次调用前后的增量，
 *   不是插件线性内存的精确值——WASM 线性内存是宿主进程内 mmap 的私有内存，
 *   所以进程增量是「不依赖插件配合」就能拿到的近似口径（负数按 0 计）。
 * - 想要精确值得让插件自报（guest 里读 `wasm32::memory_size(0)`，见示例插件 `mem` 动作）。
 * - `memdump_bytes` 是**出错那一刻**插件线性内存的字节数（等于快照文件大小），
 *   只有开启 `memdump_on_error` 且确实出过错、快照文件还在时才有值。
 */
export interface CallStats {
  plugin_id: string
  /** 被调用的函数名 */
  func: string
  /** 本次调用是否成功 */
  ok: boolean
  /** 消耗的指令数（Extism fuel；拿不到时为 null） */
  fuel_used: number | null
  /** 进程提交内存增量（字节，负增量按 0 计） */
  mem_delta_bytes: number | null
  /** 进程驻留内存增量（字节，负增量按 0 计） */
  rss_delta_bytes: number | null
  /** 采样时宿主进程提交内存总量（字节） */
  proc_commit_bytes: number | null
  /** 出错快照文件路径（仅开启 memdump 且文件存在时给出） */
  memdump_path: string | null
  /** 出错快照文件大小（字节）＝出错那刻的线性内存字节数 */
  memdump_bytes: number | null
  /** 连续「宿主级故障」次数（达到后端阈值就跳闸） */
  consec_failures: number
  /** 熔断剩余冷却毫秒（0 = 没在冷却）；冷却期内的调用会被直接拒掉 */
  circuit_cooldown_ms: number
}

/** 已安装第三方插件（后端 `plugin_list` 返回） */
export interface InstalledPlugin {
  id: string
  name: string
  version: string
  description: string
  /** 内置图标名或包内相对路径（空串表示用默认图标） */
  icon: string
  /** 入口页面（包内相对路径） */
  entry: string
  has_wasm: boolean
  permissions: string[]
  /** 设置项定义（清单 `contributes.configuration`；空 = 该插件没有设置） */
  settings: SettingDef[]
  /** 插件页能力声明（空 = 最严 CSP）；用于「3D 与游戏」容器与设置页展示 */
  page_capabilities: string[]
  /**
   * 用户在设置里**收紧掉**的页面能力（settings.json `plugin_caps_denied`）。
   * 生效口径 = `page_capabilities` − 本集合：只能减，不能加。
   */
  denied_page_capabilities?: string[]
  /** 主界面是否在独立窗口里（清单 `standaloneWindow`） */
  standalone_window: boolean
  /** 依赖哪些插件（清单 `dependencies` 原样透出） */
  dependencies: string[]
  enabled: boolean
  /** 安装时间（RFC3339，可能为空串） */
  installed_at: string
  size_bytes: number
  builtin: boolean
}

/** 后端统一响应包裹 */
export interface ApiResp<T> {
  code: number
  data: T
  msg: string
}

/** 插件来源 */
export type PluginSource = 'builtin' | 'third-party'

/** 插件管理统一列表项（内置 + 第三方同一视图，排序也用同一份序列） */
export interface PluginListItem {
  id: string
  label: string
  source: PluginSource
  description: string
  version: string
  permissions: string[]
  /** 设置项定义（内置插件恒为空） */
  settings: SettingDef[]
  /** 插件页能力声明（空 = 最严 CSP；内置插件恒为空） */
  pageCapabilities: string[]
  /**
   * 用户收紧掉的页面能力（内置插件恒为空：内置不是第三方包，没有 CSP 可收紧）。
   * 始终只可能 ⊆ `pageCapabilities`。
   */
  deniedPageCapabilities: string[]
  /** 主界面是否在独立窗口里（内置插件恒为 false） */
  standaloneWindow: boolean
  /**
   * 宿主常驻入口（活动栏底部齿轮，如「设置」）：不可禁用。
   *
   * 与 `enabled` 的区别是「能不能关」而不是「现在是开是关」——它同时也是插件管理界面的入口，
   * 允许关掉就等于把用户锁在门外（后端 `plugin_set_enabled` 也会拒绝）。
   */
  pinned: boolean
  enabled: boolean
  hasWasm: boolean
  sizeBytes: number
  installedAt: string
  /**
   * 上一次调用的资源占用（仅第三方且宿主本次会话真的调用过才有值）。
   * 放在列表项里而不是让视图自己去查：列表本身就是响应式的，视图不必再持一份映射。
   */
  lastStats: CallStats | null
  /** 本插件声明依赖哪些插件（空 = 不依赖任何插件） */
  dependencies: string[]
  /**
   * 声明了依赖但当前不可用的那些（没打包 / 没安装 / 被停用）。
   *
   * 单独列出而不是让视图自己算：「依赖缺失」是运行期事实，宿主判定口径要统一
   * （内置插件集由构建决定，第三方由安装状态决定）。
   */
  missingDependencies: string[]
  /**
   * 激活耗时（毫秒，各面板首次加载之和）；**未激活为 `null`**。
   * 懒激活之后「装了很多插件」不等于「启动很慢」，这个数字说明慢是谁造成的。
   */
  activationMs: number | null
}
