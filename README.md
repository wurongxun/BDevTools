# BDevTools

**_一切皆是插件_**,BDevTools一款跨平台桌面管理工具集合。以「**_插件化_**」为核心，集文件编辑、大日志分析、SSH/SFTP、数据库、Java 堆分析、本地终端与 3D/游戏窗口于一身，并开放**第三方插件框架**——任何人都可以在沙箱里装一个纯前端页面、或带 Rust/WASM 后端的工具。轻量高效，GB 级日志与堆转储分析无压力。

> 当前版本 **1.6.0**（`productName: BDevTools`，应用标识 `com.backend-development-tools.app`）。

## 功能概览

功能以「插件」为最小单元组织。下表是随应用编译进来的**内置插件**；第三方插件可在「设置 → 插件」里从目录或 zip 安装（见下文）。

### 内置插件

| 插件 | ID | 定位 | 能力要点 |
|------|----|------|----------|
| 文件管理 | `files` | 资源管理器 + 编辑器 | 目录树 / 收藏 / 最近打开 / 搜索过滤；Markdown 编辑与实时预览；文件编码自动检测（UTF-8 / GBK / UTF-16）；会话恢复；系统双击 `.md` 关联打开 |
| 远程连接 | `ssh` | SSH 终端 + SFTP | 多标签 SSH 终端（多终端主题）；SFTP 双栏管理（上传/下载/重命名/删除，含进度条与传输队列）；连接分组 / 书签；断线检测；远程日志拉取 |
| 日志工作台 | `logs` | 大文件日志分析 | 分块懒加载（GB 级不卡）；关键词 / 正则搜索与命中导航；级别配色（Java/Logback/Python/系统日志归一化）；自定义高亮规则；堆栈识别与悬停元信息 |
| 数据库 | `database` | 多数据库连接管理 | 支持 MySQL / PostgreSQL / SQL Server / Oracle；SQL 执行（语法高亮、计划查看）；表结构与数据网格浏览、分页；导入 / 导出 |
| 内存分析 | `mat` | Java 堆转储分析 | HPROF / PHD 格式识别；浅堆直方图（真解析）；泄漏检测 / 支配树 / GC Roots 引用追踪 / OQL（子能力分批迭代接入） |
| 本地终端 | `terminal` | 停靠面板（底部） | 多标签 / 分屏 / 可拖拽停靠；shell 集成（cwd、退出码）；活动栏图标即显隐开关，不占主区 |
| 3D / 游戏 | `web3d` | 独立窗口托管 | 在新窗口里运行声明了页面能力 / 独立窗口的第三方 3D / 游戏插件（Unity WebGL、Godot、three.js 等）；列表占满主区 |
| 设置 | `settings` | 宿主入口（固定底部） | 应用设置、窗口预设、插件管理；不可禁用 |
| 占位 | `placeholder` | 布局验证示例 | 只有主面板、无侧栏 / 标签条，用于验证「无侧栏插件」的布局适配（验证完可删） |

> 三种窗口形态：`mainPanel`（占主区）、`sidebarPanel`（活动栏图标 + 侧栏）、`dockPanel`（停靠面板，如终端）。插件可任意组合，窗口预设决定「这个窗口启用哪些插件、怎么排」。

### 第三方插件（可安装）

应用内置一套**沙箱插件框架**，已附带 4 个示例插件（`examples/third-party-plugins/`）。核心特性：

- **插件页永远在沙箱里**：以 `sandbox="allow-scripts"` 的 iframe 运行（无 `allow-same-origin`），固定 CSP（`default-src 'none'; script-src 'self'` 等），**拿不到任何 Tauri 接口**——所有能力都由宿主按权限放行。
- **两种形态**（正交于渲染位置）：
    - **纯前端插件**：逻辑全在页面 JS 里，无后端能力（`plugin_call` 返回 501）。
    - **带 WASM 后端的插件**：逻辑跑在 **Extism WASM 沙箱**（Rust / C / AssemblyScript 皆可），按 `permissions` 申请宿主能力：`log` / `kv` / `fs:read` / `fs:write` / `net:<host>`（默认全拒，调用点逐次校验）。
- **渲染位置可选**：声明 `pageCapabilities`（`wasm` / `assets` / `media` / `worker` / `isolate`）可把页面 CSP 从「最严」放宽到能跑 3D / 游戏引擎；声明 `standaloneWindow` 则入口改为**独立窗口**（跨源隔离 `crossOriginIsolated` 为真，可本地存档）。
- **页面 ↔ 宿主 通信走桥协议**（postMessage + 版本号 `PLUGIN_BRIDGE_VERSION`）：页面发 `ready` / `call`，宿主回 `data` / `error` / `theme`，所有 host function 走 `plugin_call`。

| 示例 | 形态 | 演示重点 |
|------|------|----------|
| `json-toolkit` | 纯前端沙箱 | JSON 格式化 / 压缩 / 校验 / 结构统计（逻辑全在页面，无后端） |
| `text-toolkit` | 沙箱 + WASM 后端 | Extism WASM 里的逻辑；`log` / `kv` / `fs` / `net` 四类宿主能力的授权与拒绝、大文件流式通道（游标循环）、内存自报 |
| `webgl-demo` | 独立窗口 3D | 零依赖 WebGL 旋转立方体 + 五项页面能力自检（`standaloneWindow` + `pageCapabilities`） |
| `redis-client` | 独立窗口 + WASM 后端 | 真实 Redis 客户端（P0 连接/Key/5 种类型 + P1 值编辑器/批量/TTL/监控 + P2 内存/慢查询/集群/审计）；WASM 后端经宿主 `net:*` 原始 socket 跑 RESP 协议 |

> 插件开发完整指南见 `docs/plugin-dev/`（快速开始 / 清单字段 / 桥协议 / 能力 API / 3D 与游戏 / 打包发布 / 调试 / 错误码）。第三方插件的私有数据是**全局单份**（与内置插件按工作区隔离刻意不同）。

## 软件架构

### 技术栈

- **后端**：Rust + Tauri 2.x
- **前端**：Vue 3 + TypeScript + Pinia + Vite
- **UI 组件**：Element Plus
- **终端**：xterm.js
- **编辑器**：v-md-editor（Markdown）
- **插件后端**：Extism PDK（WASM 沙箱）

### 架构核心原则：Tauri 边界隔离

应用刻意保持「前端 Vue3 + 后端 Rust」框架耦合最小，便于迁移 Electron 等其他框架：

- Rust 业务模块是**纯函数**，不依赖 Tauri 类型/插件；Tauri 相关代码收敛到三处：`src/core/runtime.ts`（前端能力 re-export + KV 适配）、`src/api/rpc.ts`（`invoke()` 边界）、`src-tauri/src/commands.rs` + `lib.rs`（命令薄封装与注册）。
- 新增 Tauri 命令走三层：业务模块写函数 → `commands.rs` 加 `#[tauri::command]` wrapper → `lib.rs` 导入并注册。**可能阻塞的命令一律 `async fn` + `spawn_blocking`**，避免卡死主线程（界面、菜单、所有插件一起僵住）。
- 应用数据走自研 KV 命令（`kv_get/set/delete/entries`），不引入 `localStorage` 存关键数据；目录按「卸载时该不该删」分层（`config_dir` 存配置、`app_dir` 存用户资产与第三方插件）。
- 跨窗口 / 后端推送走 `src/core/runtime.ts` 的 `subscribe()` / `publish()`；每个窗口各有独立的 `pluginManager` / 布局 / Pinia，涉及「别的窗口也该变」的动作（启停插件、增删窗口预设）必须广播。

### 后端模块布局（`src-tauri/src/`）

```
src-tauri/src/
├── commands.rs          # 所有 #[tauri::command] 薄 wrapper（注册集中点）
├── lib.rs               # 命令注册、插件初始化
├── window_manager/      # 窗口注册表、创建、聚焦、关闭；主窗口唯一创建点
├── session/             # 会话快照保存与恢复
├── workspace/           # 工作区管理
├── command/             # 文件 / SSH / SFTP / 日志搜索 / 数据库 / 插件宿主 / Redis 后端
├── database/            # 数据库
├── sqlite/              # SQLite（SSH 连接配置存储等）
├── app/                 # 应用初始化、托盘、自动启动、沉浸式快捷键
├── plugin_host.rs       # 第三方插件宿主
└── plugin_wasm.rs       # Extism WASM 运行时
```

### 前端模块布局（`src/`）

```
src/
├── plugins/             # 内置插件（见上表）+ _sandbox（渲染第三方插件的宿主组件，非注册插件）
│   └── index.ts         # BUILTIN_PLUGINS 静态清单（禁用开关的作用域，不可与「已注册」混淆）
├── core/                # 插件管理、运行时桥、KV、事件、窗口预设等核心能力
├── components/          # 通用组件（TabBar / MonacoEditor / GlobalDialog 等）
├── Layout/              # 应用外壳（活动栏 / 主区 / 状态栏 / 窗口控制 / 沉浸式）
├── store/               # 工作区、会话等全局 Pinia
└── styles/              # 设计令牌唯一来源（tokens / theme / element 适配）
```

## 多窗口与布局

- **多窗口**：主窗口 + 多个编辑器窗口；一个窗口预设同时只开一个窗口（后端硬约束，返回 409）。
- **窗口预设**：自定义插件组合、窗口尺寸、专注模式；主窗口与编辑器窗口**均可绑定工作区**——绑定则在启动时直开该工作区，未绑定则弹「打开时选择」。
- **会话恢复**：关闭后重开自动恢复编辑状态。
- **专注模式**：隐藏活动栏，固定显示第一个插件的面板。
- **沉浸式**：一键只留插件界面（侧栏面板 + 主面板），隐藏标题栏 / 活动栏 / 状态栏；鼠标移到窗口顶部弹出退出条（Esc 亦可退出）。
- 窗口 `decorations(false)` 时由 `WindowControls` 还回拖拽区与最小/最大/关闭，避免「拖不动也关不掉」。

## 界面与主题

- 亮色 / 暗黑双主题，全局主题变量驱动，主题跟随系统（首次运行无偏好时）。
- 插件化侧边栏：活动栏 + 面板结构，类似 VS Code。
- 自定义无边框窗口（标题栏 = 应用自绘：左侧为主题 / 侧边栏 / 插件面板 / 沉浸式四个快捷按钮，右侧为窗口控制）。

## 项目规划

- **数据库插件**：已落地（MySQL / PostgreSQL / SQL Server / Oracle + SQL 执行 / 表数据浏览 / 导入导出）。
- **MAT（Java 堆分析）**：首批能力已上线（直方图），泄漏检测 / 支配树 / 引用追踪 / OQL 分批迭代。
- **第三方插件生态**：完善插件市场与签名分发（当前为「从目录 / zip 安装」），扩展更多官方示例（消息队列、K8s、对象存储等）。

## 使用说明

### 开发环境

```bash
# 安装依赖
pnpm install

# 启动开发（前端热更新 + Tauri 调试）
pnpm tauri dev

# 仅启动前端开发服务
pnpm dev

# 类型检查
pnpm type-check

# 代码格式化
pnpm format
```

### 打包发布

```bash
# 完整构建（含类型检查）
pnpm build

# 本地打包（生成带签名的安装包；Windows 上只出 nsis）
# 注意：本项目开了 createUpdaterArtifacts，tauri 构建时强制要求签名私钥，
# 直接跑 `pnpm tauri build` 会报 "A public key has been found, but no private key" —— 用下面这个
pnpm release:build

# 一键发版：升版本 → 构建并签名 → 校验签名 → 上传 Gitee Release → 更新 latest.json
pnpm release

# 分步执行
pnpm release:build     # 只构建 + 签名 + 本地验签
pnpm release:publish   # 只发布（需要令牌）
pnpm release:verify    # 校验产物签名与配置里的公钥是否匹配
```

### 打包配置

- 目标格式：`deb`、`nsis`
  **不用 msi**：MSI 是机器级安装，`msiexec` 需要管理员权限，自动更新每次都会弹 UAC 提权框；
  NSIS 走 `currentUser` 安装（装到 `%LOCALAPPDATA%`），更新全程静默
- 支持语言：`zh-CN`、`en-US`、`pt-BR`、`fr-FR`

### 自动更新

- 用 `tauri-plugin-updater` 检查更新，清单是仓库根 `latest.json`，由 `pnpm release` 自动生成并提交回仓库
- 更新包经 **minisign 签名**，公钥内置于应用（`plugins.updater.pubkey`），签名不匹配直接拒绝安装
- **1.4.0 → 1.4.1 需要手动重装一次**：1.4.0 内置的是旧公钥和已废弃的更新端点，本身就无法自动升级；
  从 1.4.1 起才进入自动更新链路

## 许可声明

应用图标出处：https://www.flaticon.com/free-icon/substance_9814620

## 应用截图

![image](./img_0.png)
![image](./img_1.png)
![image](./img_2.png)
![image](./img_3.png)
