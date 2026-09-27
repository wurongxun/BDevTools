/*! # files 插件：文件与目录

- `file_command` —— 目录树、笔记/脚本文件读写、文件编码探测
- `file_utils` —— 路径与文件工具函数
- `file_association` —— Windows 文件关联（注册 / 打开系统默认应用设置）
*/
pub mod file_command;
pub mod file_utils;
pub(crate) mod file_association;
