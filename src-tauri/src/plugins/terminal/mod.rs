/*! # terminal 插件：本地终端（PTY）

ConPTY 会话管理。**释放 PTY 必须 detach 到独立线程** —— Windows 上
「shell 已杀、读端仍持输出管」时 `ClosePseudoConsole` 会永久阻塞（实测 6h+）。
*/
pub mod terminal;
