//! 通用工具：file_util / fs / regex_util 来自 bdevtools-core，重导出保持调用路径不变。
//! `line_reader` 里带 logs 行索引加速的部分依赖宿主裁剪开关，仍留在宿主 crate。
pub mod line_reader;
pub use bdevtools_core::utils::{file_util, fs, regex_util};
