/*! # files 插件：文件与目录

- `file_command` —— 目录树、笔记/脚本文件读写、文件编码探测、移动
- `file_utils` —— 路径与文件工具函数
- `file_meta` —— 编码/EOL/BOM/mtime 元数据与原子写回（防乱码、防半截文件、防静默覆盖）
- `file_search` —— 跨文件搜索 / 单文件按行查询 / 批量替换（预览·应用·撤销）
- `file_large` —— 大文件行级补丁与分块写回（复用 file_meta 的唯一写回通道）
- `file_convert` —— 编码 / EOL / BOM 显式转换（先备份，再走唯一写回通道）
- `file_association` —— Windows 文件关联（注册 / 打开系统默认应用设置）

整个 `plugins::files` 模块已由 `plugins/mod.rs` 的 `#[cfg(feature = "plugin-files")]` 整体裁剪，
子模块不再重复加 cfg（file_meta 上那一条是历史遗留，无害）。
*/
pub mod file_command;
pub mod file_utils;
#[cfg(feature = "plugin-files")]
pub mod file_meta;
pub mod file_search;
#[cfg(feature = "plugin-files")]
pub mod file_large;
#[cfg(feature = "plugin-files")]
pub mod file_convert;
pub(crate) mod file_association;
