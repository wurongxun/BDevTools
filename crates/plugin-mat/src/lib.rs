/*! # mat 插件：Java 堆转储分析

- `mat_hprof` —— HPROF 解析（mmap 零拷贝）、按类直方图
- `mat_graph` —— 对象图公共模块（泄漏检测与引用追踪共用）
- `mat_leak` —— Leak Suspects（Lengauer-Tarjan + Retained Heap）
- `mat_dom` —— 支配树会话缓存与 Dominator Tree / Retained Set / Top Consumers 查询
- `mat_refs` —— 会话式引用追踪（Path to GC Roots、实例分页、通配搜索）
- `mat_values` —— 按需扫描的字段值 / 数组内容 / String 值
- `mat_oql` —— OQL 安全子集（自写 lexer/parser，禁函数调用与反射）
- `mat_reports` —— 专用报告（线程 / 类加载器 / 内存浪费 / 不可达对象）
- `mat_compare` —— 多 dump 直方图对比（Δcount / Δshallow）
- `mat_index` —— 本地索引持久化（`.bdtidx` 图 / `.bdtidx.dom` 支配树，按指纹失效）
- `mat_layout` —— **浅堆口径唯一真源**（对象头 / 压缩指针 / 对齐，类对象不再按 0）
- `mat_source` —— **路径 → 可 mmap 文件的唯一入口**（`.hprof.gz` 透明解压 + 缓存）
- `mat_progress` —— 分析进度汇（默认空 = 零开销；sidecar 子进程装 NDJSON 汇）
- `mat_worker` —— **sidecar 分析子进程**（`__mat_worker --path <dump> [--dom]`）
- `mat_cfg` —— 分析预算 / 图规模闸 `current_limits()` / 查询上限 `query_limits()`
  （**不要写常量**，两类上限都要走这里）
*/
pub mod mat_cfg;
pub mod mat_compare;
pub mod mat_dom;
pub mod mat_hprof;
pub mod mat_index;
pub mod mat_layout;
pub mod mat_leak;
pub mod mat_refs;
pub mod mat_oql;
pub mod mat_reports;
pub mod mat_progress;
pub mod mat_source;
pub mod mat_values;
pub mod mat_worker;
pub(crate) mod mat_graph;
#[cfg(test)]
pub(crate) mod test_support;
