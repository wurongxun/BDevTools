/*! # logs 插件：日志索引与查询

- `log_index` —— 两级索引（L1 块级聚合 / L1.5 级别位图 / L2 行偏移）+ 合并查询
  （筛选 + 关键词一次扫描）。设计见 `docs/design/log-filter-search-unification.md`。
- 行读取工具已提升为核心 `crate::utils::line_reader`（files 的 TextView 也用它，
  不能跟着 logs 一起被裁）。
- `log_stream` —— 日志文件流式读取（远程 / 本地）。
*/
pub mod log_index;
pub mod log_stream;
