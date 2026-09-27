/*! # logs 插件：日志索引与查询

- `log_index` —— 两级索引（L1 块级聚合 / L1.5 级别位图 / L2 行偏移）+ 合并查询
  （筛选 + 关键词一次扫描）。设计见 `docs/design/log-filter-search-unification.md`。
- `search_log_parallel` —— 行读取工具（`read_file_lines` / `count_file_lines`），
  被 `log_index` 复用，保留原名以免与历史命令名混淆。
- `log_stream` —— 日志文件流式读取（远程 / 本地）。
*/
pub mod log_index;
pub mod log_stream;
pub(crate) mod search_log_parallel;
