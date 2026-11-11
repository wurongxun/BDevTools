/*! # ssh 插件：SSH / SFTP 远程连接（独立 crate，纯 Rust 业务层）

模块划分：
- `remote_conn`：连接建立（代理 / 跳板机 / known_hosts / 2FA）、交互式终端、SFTP、远程日志
- `options`：P2 高级选项与 SOCKS/HTTP 代理协议
- `relay`：TCP ↔ SSH channel 双向中继（转发与跳板机共用）
- `tunnel`：-L / -R / -D 端口转发任务表
- `exec`：单条命令执行
- `transfer`：上传/下载统一实现（取消、断点续传、测速、进度事件）
- `session_log`：P3 会话日志录制（带时间戳的终端输出落盘）

连接配置的持久化实体在 `bdevtools_core::sqlite::ssh_connection`。
本 crate 的 Cargo.toml 不允许出现 `tauri` / `tauri_plugin_*`：
Tauri 类型适配与 emit 注入一律留在宿主 `commands/`。
*/
pub mod exec;
pub mod options;
pub mod relay;
pub mod remote_conn;
pub mod session_log;
pub mod transfer;
pub mod tunnel;
