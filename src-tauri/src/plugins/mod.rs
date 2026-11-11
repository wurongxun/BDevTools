/*! # 插件业务模块

按「前端插件」划分业务目录，与前端 `src/plugins/<id>/` 一一对应：

| 目录 | 前端插件 | 职责 |
|---|---|---|
| `files` | files | 文件/目录树、脚本文件、文件关联 |
| `ssh` | ssh | SSH / SFTP 远程连接与会话（**已拆到 `crates/plugin-ssh`**，此处仅 re-export） |
| `logs` | logs | 日志索引、查询、流式读取 |
| `database` | database | 数据库连接、查询、元数据、导入导出 |
| `mat` | mat | Java 堆转储（HPROF）分析 |
| `terminal` | terminal | 本地终端（PTY） |

**宿主机制不在这里**：`plugin_host`、`plugin_wasm` 是插件机制本身（不属于任何插件），
放在顶层 `crate::plugin_host`。

命名与前端保持一致，跨端定位代码时目录名直接对得上，不需要记映射表。

## 编译期裁剪

每个目录都可由同名 Cargo feature 关掉（`plugin-files` / `plugin-ssh` / `plugin-logs` /
`plugin-database` / `plugin-mat` / `plugin-terminal`），关掉后模块与它的命令都不进产物，
专属重依赖也不再链接。**必须与前端的 `BUILTIN_PLUGINS` 同开同关**，否则会出现
「前端有入口、后端无命令」—— 由 `verify-plugin-selection.mjs` 守着。

两个前提（已就位，改动前先确认没被破坏）：
- 通用 fs 能力（`ensure_dir` / `create_folder`）已下沉到 `crate::utils::fs`，
  纯 `create_dir_all` 不该被 files 的开关绑住（database 的外部脚本目录也要建目录）。
- 前端 `documents` store 已提升到核心（见 `src/core/documents.ts`），
  否则 App.vue / 会话恢复会因 files 被裁而连带崩掉。
*/
#[cfg(feature = "plugin-database")]
pub use bdevtools_plugin_database as database;
#[cfg(feature = "plugin-files")]
pub mod files;
#[cfg(feature = "plugin-logs")]
pub use bdevtools_plugin_logs as logs;
#[cfg(feature = "plugin-mat")]
pub use bdevtools_plugin_mat as mat;
#[cfg(feature = "plugin-ssh")]
pub use bdevtools_plugin_ssh as ssh;
#[cfg(feature = "plugin-terminal")]
pub mod terminal;
