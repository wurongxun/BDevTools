/*! # 插件业务模块

按「前端插件」划分业务目录，与前端 `src/plugins/<id>/` 一一对应：

| 目录 | 前端插件 | 职责 |
|---|---|---|
| `files` | files | 文件/目录树、脚本文件、文件关联 |
| `ssh` | ssh | SSH / SFTP 远程连接与会话 |
| `logs` | logs | 日志索引、查询、流式读取 |
| `database` | database | 数据库连接、查询、元数据、导入导出 |
| `mat` | mat | Java 堆转储（HPROF）分析 |
| `terminal` | terminal | 本地终端（PTY） |

**宿主机制不在这里**：`plugin_host`、`plugin_wasm` 是插件机制本身（不属于任何插件），
放在顶层 `crate::plugin_host`。

命名与前端保持一致，跨端定位代码时目录名直接对得上，不需要记映射表。
*/
pub mod database;
pub mod files;
pub mod logs;
pub mod mat;
pub mod ssh;
pub mod terminal;
