//! Java 堆转储分析（mat 插件） 命令（原 commands.rs 对应分区的机械拆分）。
// 分片内命令可能被 feature 整体裁掉，故 glob import 允许未使用。
#[allow(unused_imports)]
use super::*;

// ==================== Java 堆转储分析（mat 插件） ====================

/// 读取堆转储文件头：格式识别（HPROF / PHD / 未知）+ 元信息
#[cfg(feature = "plugin-mat")]
#[tauri::command]
pub async fn mat_hprof_header(path: String) -> Result<ResData<MatHeaderInfo>, String> {
    let p = path.clone();
    tokio::task::spawn_blocking(move || mat_hprof::mat_hprof_header(&p))
        .await
        .map_err(|e| format!("解析线程异常: {e}"))?
}

/// 生成按类聚合的浅堆直方图（Shallow Heap）
#[cfg(feature = "plugin-mat")]
#[tauri::command]
pub async fn mat_hprof_histogram(
    path: String,
    limit: Option<usize>,
) -> Result<ResData<MatHistogramResult>, String> {
    let p = path.clone();
    tokio::task::spawn_blocking(move || mat_hprof::mat_hprof_histogram(&p, limit))
        .await
        .map_err(|e| format!("解析线程异常: {e}"))?
}

/// 分析预算 / 规模上限明细（前端「分析预算」设置弹窗，不依赖任何 dump）
#[cfg(feature = "plugin-mat")]
#[tauri::command]
pub async fn mat_analysis_limits() -> Result<ResData<MatLimitInfo>, String> {
    tokio::task::spawn_blocking(mat_cfg::mat_analysis_limits)
        .await
        .map_err(|e| format!("解析线程异常: {e}"))?
}

/// Leak Suspects：支配树 + Retained Heap 的泄漏嫌疑分析（对象多时耗时数十秒级）
#[cfg(feature = "plugin-mat")]
#[tauri::command]
pub async fn mat_leak_suspects(path: String) -> Result<ResData<MatLeakSuspectsResult>, String> {
    let p = path.clone();
    tokio::task::spawn_blocking(move || mat_leak::mat_leak_suspects(&p))
        .await
        .map_err(|e| format!("解析线程异常: {e}"))?
}

/// 引用追踪：构建（或复用）对象图会话，返回图规模与可达性汇总
#[cfg(feature = "plugin-mat")]
#[tauri::command]
pub async fn mat_refs_build(path: String) -> Result<ResData<RefBuildInfo>, String> {
    let p = path.clone();
    tokio::task::spawn_blocking(move || mat_refs::mat_refs_build(&p))
        .await
        .map_err(|e| format!("解析线程异常: {e}"))?
}

/// 引用追踪：释放某 dump 的会话缓存（关闭 dump 时调用）
#[cfg(feature = "plugin-mat")]
#[tauri::command]
pub async fn mat_refs_release(path: String) -> Result<ResData<bool>, String> {
    let p = path.clone();
    tokio::task::spawn_blocking(move || mat_refs::mat_refs_release(&p))
        .await
        .map_err(|e| format!("解析线程异常: {e}"))?
}

/// 引用追踪：类名搜索（大小写不敏感子串，按浅堆合计降序）
#[cfg(feature = "plugin-mat")]
#[tauri::command]
pub async fn mat_refs_search_classes(
    path: String,
    query: String,
    limit: Option<usize>,
) -> Result<ResData<RefSearchResult>, String> {
    let (p, q) = (path.clone(), query.clone());
    tokio::task::spawn_blocking(move || {
        mat_refs::mat_refs_search_classes(&p, &q, limit.unwrap_or(40))
    })
    .await
    .map_err(|e| format!("解析线程异常: {e}"))?
}

/// 引用追踪：取某类实例（**分页**，按浅堆降序或对象 id 升序）
#[cfg(feature = "plugin-mat")]
#[tauri::command]
pub async fn mat_refs_instances(
    path: String,
    class_name: String,
    offset: Option<usize>,
    limit: Option<usize>,
    sort: Option<String>,
) -> Result<ResData<RefInstances>, String> {
    let (p, c) = (path.clone(), class_name.clone());
    tokio::task::spawn_blocking(move || {
        mat_refs::mat_refs_instances(
            &p,
            &c,
            offset.unwrap_or(0),
            limit.unwrap_or(100),
            sort.as_deref().unwrap_or("shallow"),
        )
    })
    .await
    .map_err(|e| format!("解析线程异常: {e}"))?
}

/// 引用追踪：单个对象详情（outgoing / incoming / Path to GC Roots）
#[cfg(feature = "plugin-mat")]
#[tauri::command]
pub async fn mat_refs_inspect(
    path: String,
    obj_id: String,
) -> Result<ResData<RefObjectDetail>, String> {
    let (p, o) = (path.clone(), obj_id.clone());
    tokio::task::spawn_blocking(move || mat_refs::mat_refs_inspect(&p, &o))
        .await
        .map_err(|e| format!("解析线程异常: {e}"))?
}

/// Dominator Tree：对象级 / 按类 / 按包，支持子树展开与分页（支配树在会话内只算一次）
#[cfg(feature = "plugin-mat")]
#[tauri::command]
pub async fn mat_dom_tree(
    path: String,
    group_by: Option<String>,
    parent_obj_id: Option<String>,
    query: Option<String>,
    offset: Option<usize>,
    limit: Option<usize>,
) -> Result<ResData<mat_dom::DomTreeResult>, String> {
    let p = path.clone();
    tokio::task::spawn_blocking(move || {
        mat_dom::mat_dom_tree(
            &p,
            group_by.as_deref(),
            parent_obj_id.as_deref(),
            query.as_deref().unwrap_or(""),
            offset.unwrap_or(0),
            limit.unwrap_or(200),
        )
    })
    .await
    .map_err(|e| format!("解析线程异常: {e}"))?
}

/// Show Retained Set：某对象支配子树中的对象（按 retained 降序）
#[cfg(feature = "plugin-mat")]
#[tauri::command]
pub async fn mat_dom_retained_set(
    path: String,
    obj_id: String,
    limit: Option<usize>,
) -> Result<ResData<mat_dom::RetainedSetResult>, String> {
    let (p, o) = (path.clone(), obj_id.clone());
    tokio::task::spawn_blocking(move || {
        mat_dom::mat_dom_retained_set(&p, &o, limit.unwrap_or(200))
    })
    .await
    .map_err(|e| format!("解析线程异常: {e}"))?
}

/// 单对象字段值 / 数组内容 / String 值（按需扫描，不常驻内存）
#[cfg(feature = "plugin-mat")]
#[tauri::command]
pub async fn mat_object_values(
    path: String,
    obj_id: String,
    limit: Option<usize>,
) -> Result<ResData<mat_values::ObjectValues>, String> {
    let (p, o) = (path.clone(), obj_id.clone());
    tokio::task::spawn_blocking(move || {
        mat_values::mat_object_values(&p, &o, limit.unwrap_or(200))
    })
    .await
    .map_err(|e| format!("解析线程异常: {e}"))?
}

/// 多条最短 Path to GC Roots（可排除弱/软/虚/终结器引用，可翻页）
#[cfg(feature = "plugin-mat")]
#[tauri::command]
pub async fn mat_gc_root_paths(
    path: String,
    obj_id: String,
    exclude_weak: Option<bool>,
    offset: Option<usize>,
    limit: Option<usize>,
) -> Result<ResData<mat_refs::GcRootPathsResult>, String> {
    let (p, o) = (path.clone(), obj_id.clone());
    tokio::task::spawn_blocking(move || {
        mat_refs::mat_gc_root_paths(
            &p,
            &o,
            exclude_weak.unwrap_or(true),
            offset.unwrap_or(0),
            limit.unwrap_or(5),
        )
    })
    .await
    .map_err(|e| format!("解析线程异常: {e}"))?
}

/// OQL 安全子集查询（禁方法调用 / 反射）
#[cfg(feature = "plugin-mat")]
#[tauri::command]
pub async fn mat_oql_query(
    path: String,
    query: String,
) -> Result<ResData<mat_oql::OqlResult>, String> {
    let (p, q) = (path.clone(), query.clone());
    tokio::task::spawn_blocking(move || mat_oql::mat_oql_query(&p, &q))
        .await
        .map_err(|e| format!("解析线程异常: {e}"))?
}

/// OQL 语法静态校验（不读文件，供输入框即时提示）
#[cfg(feature = "plugin-mat")]
#[tauri::command]
pub async fn mat_oql_check(query: String) -> Result<ResData<mat_oql::OqlCheck>, String> {
    tokio::task::spawn_blocking(move || mat_oql::mat_oql_check(&query))
        .await
        .map_err(|e| format!("解析线程异常: {e}"))?
}

/// Top Consumers：支配树顶层按类 / 包聚合的 retained 排行
#[cfg(feature = "plugin-mat")]
#[tauri::command]
pub async fn mat_top_consumers(
    path: String,
    group_by: Option<String>,
    limit: Option<usize>,
) -> Result<ResData<mat_dom::TopConsumersResult>, String> {
    let p = path.clone();
    tokio::task::spawn_blocking(move || {
        mat_dom::mat_top_consumers(&p, group_by.as_deref().unwrap_or("class"), limit.unwrap_or(50))
    })
    .await
    .map_err(|e| format!("解析线程异常: {e}"))?
}

/// 线程报告（线程对象 + 名称 + 状态位；不含调用栈）
#[cfg(feature = "plugin-mat")]
#[tauri::command]
pub async fn mat_threads(
    path: String,
    limit: Option<usize>,
) -> Result<ResData<mat_reports::ThreadsResult>, String> {
    let p = path.clone();
    tokio::task::spawn_blocking(move || mat_reports::mat_threads(&p, limit.unwrap_or(500)))
        .await
        .map_err(|e| format!("解析线程异常: {e}"))?
}

/// 类加载器报告（loader → 类数 + duplicate classes）
#[cfg(feature = "plugin-mat")]
#[tauri::command]
pub async fn mat_classloaders(
    path: String,
    limit: Option<usize>,
) -> Result<ResData<mat_reports::ClassLoadersResult>, String> {
    let p = path.clone();
    tokio::task::spawn_blocking(move || mat_reports::mat_classloaders(&p, limit.unwrap_or(500)))
        .await
        .map_err(|e| format!("解析线程异常: {e}"))?
}

/// 内存浪费报告（重复字符串 / 集合填充率 / 空集合）
#[cfg(feature = "plugin-mat")]
#[tauri::command]
pub async fn mat_waste_report(
    path: String,
    limit: Option<usize>,
) -> Result<ResData<mat_reports::WasteResult>, String> {
    let p = path.clone();
    tokio::task::spawn_blocking(move || mat_reports::mat_waste_report(&p, limit.unwrap_or(200)))
        .await
        .map_err(|e| format!("解析线程异常: {e}"))?
}

/// 不可达对象直方图（与会话的可达标记同口径）
#[cfg(feature = "plugin-mat")]
#[tauri::command]
pub async fn mat_unreachable_histogram(
    path: String,
    limit: Option<usize>,
) -> Result<ResData<mat_reports::UnreachableResult>, String> {
    let p = path.clone();
    tokio::task::spawn_blocking(move || {
        mat_reports::mat_unreachable_histogram(&p, limit.unwrap_or(200))
    })
    .await
    .map_err(|e| format!("解析线程异常: {e}"))?
}

/// 多 dump 直方图对比（按类 / 包的 Δcount / Δshallow）
#[cfg(feature = "plugin-mat")]
#[tauri::command]
pub async fn mat_compare_histograms(
    path_a: String,
    path_b: String,
    group_by: Option<String>,
    query: Option<String>,
    only_changed: Option<bool>,
    sort: Option<String>,
    offset: Option<usize>,
    limit: Option<usize>,
) -> Result<ResData<mat_compare::CompareResult>, String> {
    let (pa, pb) = (path_a.clone(), path_b.clone());
    tokio::task::spawn_blocking(move || {
        mat_compare::mat_compare_histograms(
            &pa,
            &pb,
            group_by.as_deref().unwrap_or("class"),
            query.as_deref().unwrap_or(""),
            only_changed.unwrap_or(false),
            sort.as_deref().unwrap_or("delta_shallow"),
            offset.unwrap_or(0),
            limit.unwrap_or(0),
        )
    })
    .await
    .map_err(|e| format!("解析线程异常: {e}"))?
}

/// 解析堆转储来源（`.hprof.gz` 会在此解压到缓存，返回可 mmap 的实际路径）
#[cfg(feature = "plugin-mat")]
#[tauri::command]
pub async fn mat_source_info(path: String) -> Result<ResData<mat_source::SourceInfo>, String> {
    tokio::task::spawn_blocking(move || mat_source::mat_source_info(&path))
        .await
        .map_err(|e| format!("解压线程异常: {e}"))?
}

/// 解压缓存占用（目录 / 文件数 / 总字节 / 残留 .part）
#[cfg(feature = "plugin-mat")]
#[tauri::command]
pub async fn mat_cache_info() -> Result<ResData<mat_source::CacheInfo>, String> {
    tokio::task::spawn_blocking(mat_source::mat_cache_info)
        .await
        .map_err(|e| format!("缓存查询线程异常: {e}"))?
}

/// 清理解压缓存（正在被 mmap 的文件会跳过，不算失败）
#[cfg(feature = "plugin-mat")]
#[tauri::command]
pub async fn mat_cache_clear() -> Result<ResData<mat_source::CacheClearResult>, String> {
    tokio::task::spawn_blocking(mat_source::mat_cache_clear)
        .await
        .map_err(|e| format!("缓存清理线程异常: {e}"))?
}

/// 本地索引状态（图 `.bdtidx` / 支配树 `.bdtidx.dom` 是否存在且指纹匹配）
#[cfg(feature = "plugin-mat")]
#[tauri::command]
pub async fn mat_index_info(path: String) -> Result<ResData<mat_index::IndexInfo>, String> {
    tokio::task::spawn_blocking(move || mat_index::mat_index_info(&path))
        .await
        .map_err(|e| format!("索引查询线程异常: {e}"))?
}

/// 删除本地索引（重建成本 = 一次完整解析）
#[cfg(feature = "plugin-mat")]
#[tauri::command]
pub async fn mat_index_clear(path: String) -> Result<ResData<mat_index::IndexClearResult>, String> {
    tokio::task::spawn_blocking(move || mat_index::mat_index_clear(&path))
        .await
        .map_err(|e| format!("索引清理线程异常: {e}"))?
}

// ==================== MAT sidecar（独立分析进程） ====================

/// 启动后台分析进程：解析 + 建图 +（可选）支配树，结果落 `.bdtidx` 索引。
/// 主窗口不阻塞，进度经事件 `mat-worker` 推送。
#[cfg(feature = "plugin-mat")]
#[tauri::command]
pub fn mat_worker_start(
    app: tauri::AppHandle,
    path: String,
    dom: Option<bool>,
) -> Result<ResData<MatWorkerJob>, String> {
    crate::mat_worker_host::mat_worker_start(&app, &path, dom.unwrap_or(false))
}

/// 取消后台分析（杀进程）
#[cfg(feature = "plugin-mat")]
#[tauri::command]
pub fn mat_worker_cancel(job_id: String) -> Result<ResData<bool>, String> {
    crate::mat_worker_host::mat_worker_cancel(&job_id)
}

/// 作业状态（事件丢失时轮询兜底）
#[cfg(feature = "plugin-mat")]
#[tauri::command]
pub fn mat_worker_status(job_id: String) -> Result<ResData<MatWorkerJob>, String> {
    crate::mat_worker_host::mat_worker_status(&job_id)
}

/// 某 dump 最近一次作业（页面重挂载恢复进度条）
#[cfg(feature = "plugin-mat")]
#[tauri::command]
pub fn mat_worker_status_for_path(
    path: String,
) -> Result<ResData<Option<MatWorkerJob>>, String> {
    crate::mat_worker_host::mat_worker_status_for_path(&path)
}
