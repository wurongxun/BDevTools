# Redis 客户端（第三方插件 · P0 + P1 + P2）

基于 BackendDevelopmentTools 的**第三方插件框架**实现的 Redis 客户端。

- **P0 原型**：连接管理 + Key 浏览 + 5 种基础类型读写 + 命令行（能连上、能看、能改）。
- **P1 增强**：值编辑器增强（多格式视图 + 大值分页 + 值对比）+ 批量操作 + TTL 管理 + 基本监控。
- **P2 体验**：内存分析 + 慢查询分析 + 集群/复制支持 + 操作审计（运维也能用）。

代码用 **TypeScript + Vue3** 实现，Vite 构建为**单文件 IIFE 包**，可在 Tauri 插件的严格 CSP 沙箱内运行。P3 的扩展点以 `reserved()` 接缝显式预留，方便后续阶段直接实现。

---

## 约束：为什么 P0/P1 用「内存引擎」

第三方插件页面运行在 `sandbox="allow-scripts"` 的 iframe 里（不透明源），受到固定 CSP 约束：

```
default-src 'none'; script-src 'self'; style-src 'self' 'unsafe-inline';
img-src 'self' data:; connect-src 'none'
```

即：页面不能 `fetch`、不能开 WebSocket、拿不到任何 Tauri 对象。而 Extism 宿主提供的 `http_request` 只放行 HTTP，无法用于 Redis 的 RESP/TCP 协议。因此「在页面内直接连真实 Redis」在当前插件架构下**做不到**。

P0/P1 选择用页内 `MockEngine` 完整演示全部交互；真实连接已通过 `transport.ts` 里的 `HostRedisTransport`（经宿主 `redis_command` → `src-tauri/src/command/redis_backend/` 的 RESP 客户端）接入，**tcp 模式即连真实 Redis**（方案见下文）。

### Vue 为何必须预编译

CSP `script-src 'self'` 且无 `eval`，Vue 的运行时模板编译器（`new Vue({ template })`）需要 `eval`，会被 CSP 拦掉。因此改用 **Vite SFC + 构建期预编译模板**，产物为经典 `<script src>` 形式的 IIFE（而非 ESM 模块）——不透明源下 ESM 会因缺少 CORS 头而加载失败。

---

## 安装 / 运行

设置 → 插件 → **从目录安装**，选择本目录 `redis-client/`（需先构建，见下）。装上后活动栏出现「Redis 客户端」图标，点击即在主面板打开。

> 默认连接 `本地 Mock（演示）` 会注入一组演示数据（5 种类型 + 多库 + 大值 `big:text` + 过期 key），打开即可体验。

---

## 构建

依赖（vue、vite、@vitejs/plugin-vue、typescript、vue-tsc）复用**仓库根** `node_modules`，插件目录内不安装 `node_modules`（`verify.sh` 会拒绝）。

```bash
# 在插件目录执行（会让模块解析回落到仓库根 node_modules）
cd examples/third-party-plugins/redis-client
node ../../../node_modules/vite/bin/vite.js build      # 产出 assets/app.js + assets/app.css
node ../../../node_modules/vue-tsc/bin/vue-tsc.js --noEmit   # 类型检查
```

> ⚠️ `vite.config.ts` 中 `minify` 关闭：Vite 8 / rolldown 不再内置 esbuild 作为压缩器，仓库未单独安装 esbuild，故关闭压缩。功能等价，仅产物体积偏大（插件场景可接受）。

构建产物：
- `assets/app.js`（IIFE，预编译 Vue 应用）
- `assets/app.css`（自包含样式，主题跟随宿主 `data-theme`）

`page.html` 仅外链这两个文件，**无任何内联脚本/事件**，满足 CSP。

### 构建 WASM 后端（`plugin.wasm`）

后端是独立的 Extism guest crate（`examples/third-party-plugins/_guest/redis-client/`），编译为 `wasm32-unknown-unknown`：

```bash
cd examples/third-party-plugins/_guest/redis-client
cargo build --release --target wasm32-unknown-unknown
# 把产物放到插件包里（清单的 "wasm": "plugin.wasm" 指向它）
cp target/wasm32-unknown-unknown/release/redis_client_guest.wasm \
   ../../redis-client/plugin.wasm
```

> `_guest/redis-client` 是**独立 crate**（`[workspace]` 自成一档，不会被上层 workspace 吸入），只依赖 `extism-pdk` + `serde` + `serde_json`，可离线构建。改了后端源码必须重新构建并复制，插件再「从目录安装」一次才会生效。`_guest/*/target/` 已在 `.gitignore` 中，不会入库。

---

## 文件结构与职责

| 路径 | 职责 |
|---|---|
| `plugin.json` | 清单（id/name/entry，纯前端无 `wasm`） |
| `page.html` | 入口（仅外链 `assets/app.js` / `assets/app.css`） |
| `vite.config.ts` / `tsconfig.json` / `package.json` | 构建与类型配置 |
| `src/main.ts` | 引导：`ready` 握手 + 主题跟随 + 挂载 Vue 应用 |
| `src/bridge.ts` | 宿主桥（`postMessage`/`call`）、持久化抽象、传输注册表、`reserved()` |
| `src/engine.ts` | 内存版 Redis 引擎（命令入口 `execute(args)`，含 Stream / 合成 INFO·SLOWLOG·CLIENT / P2 分析命令 RCANALYZE·RCSLOW·RCCLUSTER·RCREPL·RCSENTINEL） |
| `src/transport.ts` | **传输层接缝**：`RedisTransport` 接口 + `MockRedisTransport`（内存）+ `WasmRedisTransport`（真实 TCP：`bridge.call('handle', {action,...})` 打给自带 WASM 后端）+ `ProxyRedisTransport`（预留） |
| `src/client.ts` | `RedisClient` 高层 API + 命令行解析/执行/补全 + P1 批量/导出/对比/监控 + P2 内存/慢查询/集群/审计 |
| `src/diff.ts` | 值对比：文本行 LCS diff + JSON 结构 diff |
| `src/monitor.ts` | INFO 文本解析、内存占用计算 |
| `src/utils.ts` / `src/types.ts` | 工具与类型定义（含 `maskSensitive` 脱敏、P2 全部类型） |
| `src/store.ts` | 全局响应式状态 + 业务动作（连接/浏览/选中/批量/监控/审计） |
| `src/ui.css` | 自包含样式（主题跟随 `data-theme`） |
| `src/components/*.vue` | Vue 组件：`AppRoot` `ConnBar` `ConnDialog` `DbSidebar` `KeyList` `KeyInfoBar` `BatchToolbar` `ValueEditor` `JsonNode` `Terminal` `MonitorPanel` `OpsView` `MemoryAnalysisPanel` `SlowQueryPanel` `ClusterPanel` `AuditPanel` `CompareDialog` `ImportExportDialog` `Modal` |

---

## 功能清单

**P0**
- 多连接管理（新增/编辑/删除/测试占位），连接配置跟随宿主主题与 `ready` 握手
- 16 个数据库网格 + 各库 key 数；前缀分组、类型筛选、通配搜索
- 5 种基础类型（string/hash/list/set/zset）+ 命令行终端（历史 ↑↓、Tab 补全）

**P1**
- **值编辑器增强**：Text / JSON / XML / YAML / Hex 多格式视图；JSON 折叠树；大值（如 400 行 `big:text`）按行分页加载；字符串一键保存
- **值对比**：对任意两个 key 做文本行 diff 或 JSON 结构 diff（`DiffLine` / `JsonChange`）
- **批量操作**：多选后批量删除 / 批量设过期 / 批量永久 / 导出选中
- **TTL 管理**：Key 信息条实时倒计时进度条（<60s 告警），设过期 / 永久 / 重命名 / 复制
- **基本监控**：概览（内存/QPS/时长/客户端/keyspace 卡片）+ INFO 分区表 + 慢日志 + 客户端列表
- **导入/导出**：JSON / CSV / Redis 命令脚本三种格式，导入回写、导出复制/下载

**P2**（新增「运维」Tab，含内存分析 / 慢查询分析 / 集群·复制 / 操作审计 四个子页）
- **内存分析**（`MemoryAnalysisPanel`）：大 Key 排名（Top 20 + 占比条）、按前缀内存占比、按类型内存分布（SVG 环形图 + 图例）。数据源 `engine.analyzeMemory` → `client.memoryAnalyze`。
- **慢查询分析**（`SlowQueryPanel`）：统计卡片（慢查询数/最大/平均/P95 μs）、命令热点（按累计耗时排序 + 占比条）、慢查询明细（原始命令 + 归一化模式，>10ms 高亮）。
- **集群/复制**（`ClusterPanel`）：主从复制信息（角色/偏移/从节点表）、集群拓扑（6 节点 + 槽位分布条 + 主从关系）、Sentinel 监控的主节点列表。数据源 `clusterTopology` / `replicationInfo` / `sentinelMasters`。
- **操作审计**（`AuditPanel`）：每次命令行、批量、导入导出、键变更均写入审计日志（持久化于 MemoryPersistence，不透明源无 localStorage）；敏感参数（密码 / token / secret）自动脱敏；支持按操作类型筛选、导出 JSON/CSV、一键清空。

> P2 的集群/复制/Sentinel 数据：在 **tcp 模式**已接真实后端（宿主 `redis_backend` 用 `SCAN + MEMORY USAGE` / `SLOWLOG GET` / `CLUSTER INFO·NODES` / `INFO replication` / `SENTINEL MASTERS` 聚合，形状与前端类型一致）；**mock 模式**仍为页内合成演示。`mode=tcp` 连接真实 Redis 后，内存分析/慢查询/集群/复制/哨兵五个子页均走真实数据。

---

## 真实 Redis 后端（已落地：方案 A）

`tcp` 模式已接入宿主真实 Redis 连接：在 `src/transport.ts` 实现 `HostRedisTransport`（经 `bridge.call('redis_*')` 转发），宿主侧在 `src-tauri/src/command/redis_backend/` 用纯 tokio 实现了 RESP 客户端（不依赖第三方 redis crate），由 `redis_command` Tauri 命令暴露给插件。UI 切到 `connection.mode = tcp` 即可连真实 Redis，**业务代码零改动**。

- **(A) 宿主原始 socket 能力（已实现）**：`src-tauri` 新增 `redis_command` 命令（RESP over tokio TcpStream），传输层通过 `bridge.call('redis_exec', { args })` 转发；P2 分析命令（`RCANALYZE`/`RCSLOW`/`RCCLUSTER`/`RCREPL`/`RCSENTINEL`）由传输层翻译成 `redis_analyze_memory` / `redis_slowlog` / `redis_cluster` / `redis_replication` / `redis_sentinel`，在宿主侧用 `SCAN + MEMORY USAGE` / `SLOWLOG GET` / `CLUSTER NODES` / `INFO replication` / `SENTINEL MASTERS` 聚合。
- **(B) 本地伴随代理**：未实现（备选方案）。

SSH 隧道 / SSL-TLS / Unix Socket 仍为预留：当前 `redis_connect` 检测到这些字段会返回明确错误，待后续扩展。

---

## 预留接口清单（检索）

全局检索 `reserved(` 可定位所有后期接缝，主要分布在：

- `src/transport.ts`：`HostRedisTransport`（已落地：tcp 模式真实连接，转发 `redis_*`）/ `ProxyRedisTransport`（预留：伴随代理方案 B）
- `src/bridge.ts`：`KvPersistence`（连接配置跨刷新持久化，当前为 `MemoryPersistence`）

> P2 的审计日志当前走 `MemoryPersistence`（不透明源无 `localStorage`）；接入 `KvPersistence` 后审计可跨会话保留。P2 的内存分析/慢查询/集群/复制/哨兵在 **tcp 模式已接真实后端**（宿主 `redis_backend` 聚合），仅 mock 模式为页内合成。

---

## 验证

```bash
# 类型检查（vite build 不报类型错误，仅做语法转换）
node ../../../node_modules/vue-tsc/bin/vue-tsc.js --noEmit

# 构建产出
node ../../../node_modules/vite/bin/vite.js build

# 入口无内联脚本 / 事件（CSP 要求）
grep -nE "on(click|load|change|input|submit)=" page.html && echo "发现内联事件" || echo "page.html 合规"
```

> 真机验证：从目录安装后在应用内点开，确认握手无超时提示、主题跟随、各 Tab 与按钮可用、大值分页与监控数据正常。
