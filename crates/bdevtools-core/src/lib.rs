//! # BDevTools 公共基础层
//!
//! 纯 Rust，**Cargo.toml 里不允许出现 `tauri` / `tauri_plugin_*` 依赖**。
//! 宿主 crate 通过 `pub use` 把这里的模块重导出为原来的 `crate::xxx` 路径，
//! 因此业务代码里既有的 `crate::response::...` / `crate::utils::...` 调用无需改写。

pub mod app;
pub mod ctx;
pub mod response;
pub mod sqlite;
pub mod utils;
