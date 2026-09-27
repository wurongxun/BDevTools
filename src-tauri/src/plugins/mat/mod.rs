/*! # mat 插件：Java 堆转储分析

- `mat_hprof` —— HPROF 解析（mmap 零拷贝）、按类直方图
- `mat_graph` —— 对象图公共模块（泄漏检测与引用追踪共用）
- `mat_leak` —— Leak Suspects（支配树 + Retained Heap）
- `mat_refs` —— 会话式引用追踪（Path to GC Roots）
- `mat_cfg` —— 分析预算 / 规模上限（**不要写常量，一律走 `current_limits()`**）
*/
pub mod mat_cfg;
pub mod mat_hprof;
pub mod mat_leak;
pub mod mat_refs;
pub(crate) mod mat_graph;
