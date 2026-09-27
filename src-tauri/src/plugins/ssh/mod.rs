/*! # ssh 插件：SSH / SFTP 远程连接

`remote_conn` 同时承载 SSH 会话、SFTP 文件操作与连接配置读写。
连接配置的持久化实体在 `crate::sqlite::ssh_connection`。
*/
pub mod remote_conn;
