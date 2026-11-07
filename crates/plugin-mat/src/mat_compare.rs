//! # 多 dump 对比（histogram diff）
//!
//! 对应 Eclipse MAT 的「Histogram → Compare With Another Heap Dump」。本模块只做
//! **类 / 包维度**的 `Δcount` 与 `Δshallow`，两侧都复用 `mat_hprof::build_histogram`，
//! 保证「同一个 dump 单独看直方图」与「参与对比」的数字完全一致（各写一份聚合必然漂移）。
//!
//! ## 为什么没有 Δretained（明确的能力边界）
//!
//! retained 需要支配树，而支配树是**每个 dump 一次 Lengauer-Tarjan**。在两个 GB 级
//! dump 上这是分钟级操作，且会把 `mat_refs` 会话 LRU(3) 的两个槽位同时占满 ——
//! 用户本来能在 A/B 之间来回切着看，对比一跑反而把两个会话都挤掉。
//! 因此这里的口径是：**Δretained 请分别在两个 dump 的「支配树 → 按类」对照**，
//! 返回结果的 `note` 里也这么写，绝不用 count/shallow 的变化量去暗示 retained 的变化。
//!
//! ## 排序与分页
//!
//! 默认按 `|Δshallow|` 降序 —— 对比的用途是「找出变化最大的类」，不是「按名字浏览」。
//! 分页在服务端做，行数上限沿用 `QueryLimits::report_rows`（不新开配置项）。

use crate::mat_cfg;
use crate::mat_hprof::{build_histogram, catch_parse, read_head, sniff_format};
use crate::mat_refs::class_matches;
use bdevtools_core::response::resp_data::ResData;
use serde::Serialize;
use std::collections::HashMap;

/// 默认返回行数（前端分页条与之对齐）
const DEFAULT_LIMIT: usize = 200;

#[derive(Debug, Serialize)]
pub struct CompareRow {
    /// 分组名（类名 / 包名）
    pub name: String,
    /// class / package
    pub kind: String,
    pub count_a: u64,
    pub count_b: u64,
    /// B − A（可为负）
    pub delta_count: i64,
    pub shallow_a: u64,
    pub shallow_b: u64,
    /// B − A（可为负；字节，前端格式化）
    pub delta_shallow: i64,
    /// added（只在 B）/ removed（只在 A）/ changed / same
    pub status: String,
}

#[derive(Debug, Serialize)]
pub struct CompareResult {
    pub group_by: String,
    /// 过滤后、分页前的分组总数
    pub total: u64,
    pub rows: Vec<CompareRow>,
    pub only_changed: bool,
    /// 实际生效的排序键
    pub sort: String,
    pub objects_a: u64,
    pub objects_b: u64,
    pub shallow_a: u64,
    pub shallow_b: u64,
    pub delta_objects: i64,
    pub delta_shallow: i64,
    /// 只在 B / 只在 A / 两侧都有但数值变化 的分组数（统计的是过滤后的全量，不受分页影响）
    pub added: u64,
    pub removed: u64,
    pub changed: u64,
    pub truncated: bool,
    pub note: String,
    pub elapsed_ms: u64,
}

/// 类名 → 分组名。
///
/// 数组类（`[B` / `[[Ljava.lang.String;`）统一归 `(arrays)`：它们的「包」在 JVM 里
/// 没有意义，硬拆出来的 `[Ljava.lang` 只会污染包排行。没有 `.` 的类归 `(default)`。
fn group_name(class_name: &str, group_by: &str) -> String {
    if group_by != "package" {
        return class_name.to_string();
    }
    if class_name.starts_with('[') {
        return "(arrays)".to_string();
    }
    match class_name.rsplit_once('.') {
        Some((pkg, _)) if !pkg.is_empty() => pkg.to_string(),
        _ => "(default)".to_string(),
    }
}

/// 单个 dump 的分组统计：`分组名 → (count, shallow)` + 该 dump 的合计。
fn load_groups(
    path: &str,
    group_by: &str,
) -> Result<(HashMap<String, (u64, u64)>, u64, u64), String> {
    // 先过格式闸：PHD / 随机文件不该被当成 HPROF 硬解析（解析器 panic 靠 catch_parse 兜，
    // 但错误信息会变成「记录解析失败」，不如这里直接说清是什么格式）
    let head = read_head(path, 64)?;
    let (_format, supported) = sniff_format(&head);
    if !supported {
        return Err("不支持的堆转储格式（当前仅支持 JAVA PROFILE 1.0.x）".to_string());
    }

    let rows = catch_parse(|| build_histogram(path))?;
    let mut map: HashMap<String, (u64, u64)> = HashMap::new();
    let mut objects = 0u64;
    let mut shallow = 0u64;
    for r in rows {
        objects += r.count;
        shallow += r.shallow_bytes;
        let e = map.entry(group_name(&r.class_name, group_by)).or_insert((0, 0));
        e.0 += r.count;
        e.1 += r.shallow_bytes;
    }
    Ok((map, objects, shallow))
}

/// 两侧合计 → (A 值, B 值, Δ)，Δ 用饱和减法避免 u64 溢出成天文数字。
fn delta(a: u64, b: u64) -> i64 {
    (b as i128 - a as i128).clamp(i64::MIN as i128, i64::MAX as i128) as i64
}

/// 多 dump 直方图对比。`sort` 可取 `delta_shallow`（默认）/ `delta_count` / `shallow_b` / `name`。
pub fn mat_compare_histograms(
    path_a: &str,
    path_b: &str,
    group_by: &str,
    query: &str,
    only_changed: bool,
    sort: &str,
    offset: usize,
    limit: usize,
) -> Result<ResData<CompareResult>, String> {
    let started = std::time::Instant::now();
    if path_a == path_b {
        return Err("两侧是同一个文件，对比结果恒为 0；请选择不同的堆转储".to_string());
    }
    let group_by = if group_by == "package" { "package" } else { "class" };

    let (map_a, objects_a, shallow_a) =
        load_groups(path_a, group_by).map_err(|e| format!("A 侧失败：{e}"))?;
    let (map_b, objects_b, shallow_b) =
        load_groups(path_b, group_by).map_err(|e| format!("B 侧失败：{e}"))?;

    let q = query.trim().to_lowercase();
    let mut keys: Vec<String> = map_a.keys().chain(map_b.keys()).cloned().collect();
    keys.sort_unstable();
    keys.dedup();

    let mut rows: Vec<CompareRow> = Vec::with_capacity(keys.len());
    let (mut added, mut removed, mut changed) = (0u64, 0u64, 0u64);
    for name in keys {
        if !class_matches(&name, &q) {
            continue;
        }
        let (ca, sa) = map_a.get(&name).copied().unwrap_or((0, 0));
        let (cb, sb) = map_b.get(&name).copied().unwrap_or((0, 0));
        let status = if ca == 0 && cb > 0 {
            "added"
        } else if cb == 0 && ca > 0 {
            "removed"
        } else if ca != cb || sa != sb {
            "changed"
        } else {
            "same"
        };
        match status {
            "added" => added += 1,
            "removed" => removed += 1,
            "changed" => changed += 1,
            _ => {}
        }
        if only_changed && status == "same" {
            continue;
        }
        rows.push(CompareRow {
            kind: group_by.to_string(),
            name,
            count_a: ca,
            count_b: cb,
            delta_count: delta(ca, cb),
            shallow_a: sa,
            shallow_b: sb,
            delta_shallow: delta(sa, sb),
            status: status.to_string(),
        });
    }

    // 排序：默认「变化量绝对值」降序 —— 对比的目的是先看差异最大的分组
    match sort {
        "delta_count" => rows.sort_by_key(|r| std::cmp::Reverse(r.delta_count.abs())),
        "shallow_b" => rows.sort_by_key(|r| std::cmp::Reverse(r.shallow_b)),
        "name" => rows.sort_by(|a, b| a.name.cmp(&b.name)),
        _ => rows.sort_by_key(|r| std::cmp::Reverse(r.delta_shallow.abs())),
    }
    let sort = match sort {
        "delta_count" | "shallow_b" | "name" => sort,
        _ => "delta_shallow",
    };

    let total = rows.len() as u64;
    let cap = mat_cfg::query_limits().report_rows;
    let page = if limit == 0 { DEFAULT_LIMIT } else { limit.min(cap) };
    let start = offset.min(rows.len());
    let end = (start + page).min(rows.len());
    let truncated = end < rows.len();
    rows.truncate(end);
    rows.drain(..start);

    let mut note = String::from(
        "Δ 只在类/包维度比 count 与 shallow；Δretained 需要两份支配树（各一次 Lengauer-Tarjan），\
         成本分钟级且会同时占满两个会话槽位 —— 请分别在两个 dump 的「支配树 → 按类」对照。",
    );
    if truncated {
        note.push_str(&format!(
            " 已按 |Δ{}| 排序截取第 {} 页（每页 {page} 行），完整排行请改排序或导出。",
            if sort == "delta_count" { "count" } else { "shallow" },
            offset / page.max(1) + 1,
        ));
    }

    Ok(ResData {
        code: 200,
        msg: "ok".to_string(),
        data: CompareResult {
            group_by: group_by.to_string(),
            total,
            rows,
            only_changed,
            sort: sort.to_string(),
            objects_a,
            objects_b,
            shallow_a,
            shallow_b,
            delta_objects: delta(objects_a, objects_b),
            delta_shallow: delta(shallow_a, shallow_b),
            added,
            removed,
            changed,
            truncated,
            note,
            elapsed_ms: started.elapsed().as_millis() as u64,
        },
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn package_grouping_keeps_arrays_apart() {
        assert_eq!(group_name("java.util.HashMap", "package"), "java.util");
        assert_eq!(group_name("Foo", "package"), "(default)");
        assert_eq!(group_name("[B", "package"), "(arrays)");
        assert_eq!(group_name("[[Ljava.lang.String;", "package"), "(arrays)");
        // 嵌套类的包仍是外部包，不是 java.util.Map
        assert_eq!(group_name("java.util.Map$Entry", "package"), "java.util");
        // class 维度原样返回，不做任何归一
        assert_eq!(group_name("java.util.HashMap", "class"), "java.util.HashMap");
    }

    #[test]
    fn delta_is_signed_and_saturating() {
        assert_eq!(delta(10, 3), -7);
        assert_eq!(delta(3, 10), 7);
        assert_eq!(delta(u64::MAX, 0), i64::MIN);
    }

    #[test]
    fn same_file_is_rejected() {
        // 用 match 而非 unwrap_err：ResData 没有实现 Debug，unwrap_err 需要 T: Debug
        match mat_compare_histograms("/tmp/a.hprof", "/tmp/a.hprof", "class", "", false, "", 0, 0) {
            Err(e) => assert!(e.contains("同一个文件"), "unexpected: {e}"),
            Ok(_) => panic!("同一文件应被拒绝"),
        }
    }
}
