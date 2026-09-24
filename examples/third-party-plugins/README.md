# 第三方插件示例

三个可安装的插件包，覆盖第三方插件的三种形态。装法：设置 → 插件 → 安装，选择下面任意一个目录（或对应的 `.zip`）。

| 示例 | 形态 | 演示重点 |
|---|---|---|
| [`json-toolkit/`](./json-toolkit) | **纯前端沙箱页面**（清单里没有 `wasm`） | 逻辑全在插件页里；后端能力全部没有，`plugin_call` 返回 501 |
| [`text-toolkit/`](./text-toolkit) | **沙箱页面 + WASM 后端**（清单里有 `wasm`） | 逻辑跑在 Extism WASM 里；log / kv / fs / net 四类宿主能力的**授权与拒绝** |
| [`webgl-demo/`](./webgl-demo) | **独立窗口里的 3D 页面**（清单里有 `standaloneWindow` + `pageCapabilities`） | 零依赖 WebGL 立方体 + 五项页面能力自检；点活动栏图标直接开独立窗口 |

## 包结构（约定）

一个插件包就是一个目录，`plugin_install` 接受目录或 zip：

```
<plugin_id>/
├── plugin.json      # 清单：id / name / version / icon / entry / wasm? / init? / permissions / pageCapabilities? / standaloneWindow?
├── page.html        # 沙箱 iframe 的入口（清单 entry 指向它）
├── page.js          # 必须外链（插件页 CSP 是 script-src 'self'，禁内联）
├── page.css
├── assets/          # 可选：包内静态资源（声明 assets 后才能被 fetch）
└── plugin.wasm      # 可选：Extism WASM 后端（清单 wasm 指向它）
```

**别把构建产物放进包目录。** WASM 源码放在 `_guest/` 下（在包目录之外，也不会被 Cargo workspace 吸收——它自带 `[workspace]`），只把编出来的 `.wasm` 复制进包目录。否则 `cargo build` 的 `target/` 会被一起打包进去。

## `webgl-demo`：3D 与游戏（页面能力）

零依赖的 WebGL 旋转立方体，外加**五项页面能力自检** —— 它同时是示例也是自查工具：
每个探针都真做一次（编译一个 8 字节的空 WASM 模块、`fetch` 包内 `.json`、从 Blob 建 Worker、
读 `crossOriginIsolated`），把「声明了的能力到底生效没有」直接打在页面上。

```json
{
  "entry": "page.html",
  "standaloneWindow": true,
  "pageCapabilities": ["wasm", "assets", "media", "worker", "isolate"]
}
```

两个字段管两件事：`pageCapabilities` 决定**页面能做什么**，`standaloneWindow` 决定**界面在哪儿打开**
（声明后活动栏图标 = 开独立窗口，主面板变成启动卡）。

正确姿势是**对比两次**：未声明时五项全红（但 WebGL 照常可用）；全部声明后五项全绿，
且 `crossOriginIsolated === true` —— 这一项只有**独立窗口**里才可能为真，
面板内的沙箱 iframe 里必然为假（跨源隔离要求整条文档链带 COOP/COEP）。
把清单里的 `standaloneWindow` 删掉重装，就能看到「入口退回面板 iframe」的差别（`isolate` 变回 false）。

不想开应用也能验同一件事：`serve-csp-profile.mjs` 会用**与宿主相同的响应头**托管这个包，
再用 `preview-shot.mjs --eval-file probe-webgl-demo.js` 无头跑一遍即可，
命令见 [3D 与游戏](../../docs/plugin-dev/web3d.md)。

> ⚠️ 这个包**没有** WASM 后端。`pageCapabilities` 里的 `wasm` 管的是「**插件页**能不能编译 WASM」，
> 与清单里的 `wasm`（后端 Extism 模块）是两件事，别混。
>
> ⚠️ 反过来，`text-toolkit` **故意不声明** `standaloneWindow`：它的逻辑全在 WASM 后端里，
> 靠 `plugin_call` 调宿主能力，扔进独立窗口就废了（`verify.sh` 有断言把这条钉住）。

## `text-toolkit` 的 WASM 后端

`_guest/text-toolkit/` 是它的 Rust 源码（Extism PDK，`crate-type = ["cdylib"]`）。重新构建：

```bash
cd _guest/text-toolkit
rustup target add wasm32-unknown-unknown
cargo build --release --target wasm32-unknown-unknown
cp target/wasm32-unknown-unknown/release/text_toolkit_guest.wasm ../../text-toolkit/plugin.wasm
```

编完可以先用 `inspect-wasm.js` 自查 ABI（纯 JS，不需要装任何东西）：它会列出模块的导入导出，确认宿主函数落在 `extism:host/user` 命名空间、`plugin.json` 里声明的入口函数名真实存在。

```bash
node inspect-wasm.js text-toolkit/plugin.wasm
```

期望输出里有 `导出函数：handle, init` 与 `extism:host/user` 下的 `host_kv_get, host_fs_read, ...`。**导出名与清单/桥协议不一致是第三方插件最常见的「点了没反应」原因**——桥发的 `func` 必须等于 guest 侧 `#[plugin_fn]` 导出的名字。

装好后点「查看已授权能力」能看到宿主实际给了什么：

```json
{
  "id": "text-toolkit",
  "dataDir": "<app_dir>/plugin_data_ext/text-toolkit",
  "canLog": true,
  "canKv": true,
  "readRoots": ["<app_dir>/plugin_data_ext/text-toolkit"],
  "writeRoots": ["<app_dir>/plugin_data_ext/text-toolkit"],
  "memoryLimitMb": 32,
  "memoryLimitPages": 512
}
```

`dataDir` 与 `*Roots` 都不随工作区变化——第三方插件的私有数据是**全局单份**的（与内置插件的按工作区隔离策略刻意不同）。

### 大文件：`scan` / `copy` / `patch`（流式通道）

这三个动作演示「文件比 WASM 线性内存（32MB）大」时怎么写：**文件留在宿主侧**，插件每次只拿一块
（≤1MB），**游标**随请求进来、随响应出去，页面按游标循环调用。

| 动作 | 演示 |
|---|---|
| `scan` | `host_fs_open` + `host_fs_size` + `host_fs_read_at`：分块扫大文件、数子串出现次数 |
| `copy` | 首轮 `w` + `host_fs_write_at`，后续轮 `a` + `host_fs_append`：分块复制 |
| `patch` | `rw` 模式 + `host_fs_write_at` + `host_fs_truncate`：就地改几个字节，再截断 |

权限没有变化（还是 `fs:read` / `fs:write`），但**一次调用处理不完是常态**：15s / 10 亿 fuel 的
预算下，真实解析类工作一次只够几十 MB，所以任务必须能续跑。

### 内存：`mem` 自报（怎么知道自己占了多少）

点「本次占用内存」会调 `mem` 动作：guest 读自己的**线性内存页数**（`core::arch::wasm32::memory_size(0)`，
1 页 = 64KiB），上限从 `host_info` 的 `memoryLimitPages` 拿，返回用量 / 页数 / 上限 / 占用比。

这**不是**锦上添花的功能——它是「插件吃了多少内存」唯一精确的口径。宿主侧虽然也能给数字
（设置 → 插件 → 插件卡片上的「资源」按钮），但那是**宿主进程**的提交量增量，混着 wasmtime 元数据
与其他插件的活动，只能看趋势。想在自己插件里加一个，直接抄 `_guest/text-toolkit/src/lib.rs` 里的
`linear_memory_bytes()` + `mem()`（注意那两行 `#[cfg(...)]` 兜底，非 wasm32 目标必须能编译过）。

出错时想看插件内部状态：设置 → 插件 里打开「**出错时导出内存快照**」，复现一次错误，
宿主会把整块线性内存写进 `{HOME}/app/plugin_data_ext/<id>/last-error.mem`
（**文件大小就是出错那刻的线性内存字节数**），插件卡片上会显示大小与路径。

### 想试「被拒绝」是什么样

改 `plugin.json` 的 `permissions`，然后**重新安装**（覆盖安装即可）：

- 删掉 `"kv"` → 点「存到 KV」报 `插件 text-toolkit 未声明 kv 权限`
- 删掉 `"fs:write"` → 点「写文件」报 `无写入权限（插件未声明覆盖该路径的 fs:write 作用域）`
- 把 `"net:api.github.com"` 改成 `"net:example.com"` → 点「发起 HTTP 请求」报 HTTP 请求失败

`fs:read` / `fs:write` 还可以带作用域，比如 `"fs:read:$HOME/Documents"`：相对路径仍以插件数据目录为基准，绝对路径与 `~` 前缀则按作用域放行。**默认什么都没给**，所有 host function 都会拒绝。

## 桥协议速查（`page.js` 里用到的那套）

```
插件页 → 宿主   { v: 1, kind: 'ready' }
插件页 → 宿主   { v: 1, kind: 'call', reqId, func, input }
宿主 → 插件页   { v: 1, reqId, ok: true,  data }
宿主 → 插件页   { v: 1, reqId, ok: false, error }
宿主 → 插件页   { v: 1, kind: 'theme', dark, wasm }
```

- `v` 必须等于 `PLUGIN_BRIDGE_VERSION`（当前 `1`），不等会被宿主直接丢弃。
- `input` 原样 JSON 序列化后交给 WASM 的 `func`；`func` 就是 guest 侧 `#[plugin_fn]` 导出的名字。
- `kind: 'theme'` 是宿主推的，`dark` 跟主题、`wasm` 表示 WASM 运行时是否可用（插件页据此禁用依赖后端的按钮，而不是等调用失败）。
- host 也只在 iframe **自己**的 `contentWindow` 上收消息，所以插件页除了 `parent.postMessage` 没有第二条通道。
