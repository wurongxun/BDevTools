/*! # MAT 分析规模上限：内存预算自适应换算

「分析上限」不该是一组与机器无关的写死常量，而应回答一个更本质的问题：
**当前机器愿意为这次分析付出多少内存？** 本模块把用户（或自动探测得到的）
「分析内存预算（GB）」换算成各环节的对象 / 边上限，让同一份 dump 在
64GB 机器上能分析、在 8GB 机器上自动更保守，而不是一概 20M/25M 一刀切。

## 预算 → 上限模型（以历史常量校准）

历史常量（引用追踪 20M 节点 / 100M 边、泄漏支配树 25M 可达 / 120M 边）是在约
16GB 物理内存、自动预算 ≈ 7.7GB 的桌面环境实测校准的。视其为「预算 7.7GB →
系数 1.0」，按线性单耗模型外推（对象/边结构内存与数量近似线性）：

- **引用追踪会话**（对象图 + id 索引 + 反向引用 + 类索引，LRU 常驻）约占预算 45%，
  173.25 B/节点（含约 5 边/节点）→ 20M 节点 @ 7.7GB；
- **泄漏支配树**（在图之上叠加 Lengauer-Tarjan / retained 等 u32 数组）约占预算 35%，
  107.8 B/可达节点（含约 4.8 边/节点）→ 25M 可达 @ 7.7GB。

注意泄漏检测与引用追踪现在**共享同一张会话图**：「能否建图」的闸门两边一致
（= 引用追踪节点上限）；支配树再按自己的上限独立截断——因此「堆里垃圾多、
可达少」的大堆至少能拿到精确的可达统计，而不是被全零截断。

## 预算来源（优先级）

1. `plugins/mat.json` 的 `analysisBudgetGb`（用户在前端「分析预算」设置里手动指定，
   单位 GB；数字则用该值，删除 / 置空 = 回到自动）—— 经 `kv_store::kv_read_value`
   读取，容错：文件缺失 / 损坏 / 数值非法时回退自动；
2. 自动：物理内存探测 → `(物理 − 2GB) × 55%`，夹在 [2, 48] GB。
*/

use crate::response::resp_data::ResData;
use serde::Serialize;
use std::sync::OnceLock;

/// 参考机（16GB 物理内存）自动预算 ≈ 7.7GB → 系数 1.0（对齐历史常量校准点）
const REF_BUDGET_GB: f64 = 7.7;
/// 系统 + 桌面壳 + 前端预留（GB）
const RESERVED_GB: f64 = 2.0;
/// 自动预算取可用物理内存的比例
const AUTO_RATIO: f64 = 0.55;
/// 预算可设范围（GB）
const BUDGET_MIN_GB: f64 = 2.0;
const BUDGET_MAX_GB: f64 = 48.0;
/// 会话预算占比与单耗（见模块注释；单耗取校准点反解值，使 7.7GB 时精确回到 20M/25M）
const REFS_BUDGET_SHARE: f64 = 0.45;
const REFS_BYTES_PER_NODE: f64 = 173.25;
const REFS_EDGES_PER_NODE: f64 = 5.0; // 100M / 20M
const DOM_BUDGET_SHARE: f64 = 0.35;
const DOM_BYTES_PER_NODE: f64 = 107.8;
const DOM_EDGES_PER_NODE: f64 = 4.8; // 120M / 25M

/// 一次换算得到的各环节上限（同一份预算在本次运行内稳定；改预算后下次命令调用重算）
#[derive(Debug, Clone, Copy)]
pub(crate) struct MatLimits {
    /// 生效中的分析内存预算（GB，自动或用户覆盖后的值）
    pub budget_gb: f64,
    /// 引用追踪节点上限（也作泄漏检测「能否建图」的总数闸——两者共享会话图）
    pub refs_nodes: usize,
    /// 引用追踪引用边上限
    pub refs_edges: usize,
    /// 泄漏支配树可达对象上限
    pub dom_nodes: usize,
    /// 泄漏支配树可达引用边上限
    pub dom_edges: usize,
}

/// `mat_analysis_limits` 命令返回：预算与上限明细（前端设置弹窗展示 / 预填）
#[derive(Debug, Serialize)]
pub struct MatLimitInfo {
    /// 本机物理内存（GB）
    pub phys_gb: f64,
    /// 自动预算（GB）：(物理 − 2GB) × 55%，夹 [2, 48]
    pub auto_budget_gb: f64,
    /// 用户覆盖值（GB，来自 plugins/mat.json 的 analysisBudgetGb；未设置 = None）
    pub override_gb: Option<f64>,
    /// 生效预算（GB）= 覆盖值或自动值
    pub effective_budget_gb: f64,
    /// 换算出的各环节上限
    pub refs_nodes: u64,
    pub refs_edges: u64,
    pub dom_nodes: u64,
    pub dom_edges: u64,
}

// ==================== 物理内存探测 ====================

/// 探测失败 / 未知平台时的保守物理内存假设（GB）：
/// 宁可少放行分析，也不让自动预算高估导致 OOM（取 16GB 校准机的一半）。
const UNKNOWN_PHYS_GB: f64 = 8.0;

/// 物理内存（GB）。Windows 用 GlobalMemoryStatusEx；Linux 读 /proc/meminfo；
/// macOS 用 sysctlbyname("hw.memsize")；探测失败 / 未知平台回退 UNKNOWN_PHYS_GB。
/// 探测一次后缓存。
pub(crate) fn physical_mem_gb() -> f64 {
    static CACHE: OnceLock<f64> = OnceLock::new();
    *CACHE.get_or_init(|| {
        let bytes = probe_physical_bytes();
        if bytes > 0 {
            bytes as f64 / 1e9
        } else {
            UNKNOWN_PHYS_GB
        }
    })
}

fn probe_physical_bytes() -> u64 {
    #[cfg(windows)]
    {
        use windows::Win32::System::SystemInformation::{GlobalMemoryStatusEx, MEMORYSTATUSEX};
        unsafe {
            let mut ms: MEMORYSTATUSEX = std::mem::zeroed();
            ms.dwLength = std::mem::size_of::<MEMORYSTATUSEX>() as u32;
            if GlobalMemoryStatusEx(&mut ms).as_bool() && ms.ullTotalPhys > 0 {
                return ms.ullTotalPhys;
            }
        }
        0
    }
    #[cfg(all(not(windows), target_os = "linux"))]
    {
        if let Ok(s) = std::fs::read_to_string("/proc/meminfo") {
            for line in s.lines() {
                if let Some(rest) = line.strip_prefix("MemTotal:") {
                    if let Ok(kb) = rest.trim().trim_end_matches("kB").trim().parse::<u64>() {
                        return kb * 1024;
                    }
                }
            }
        }
        0
    }
    // macOS：sysctlbyname("hw.memsize") → 物理内存字节（libc 仅 macOS target 引入，无新常驻依赖）
    #[cfg(all(not(windows), target_os = "macos"))]
    {
        unsafe {
            let name = b"hw.memsize\0".as_ptr() as *const libc::c_char;
            let mut size: u64 = 0;
            let mut len = std::mem::size_of::<u64>();
            if libc::sysctlbyname(
                name,
                &mut size as *mut u64 as *mut libc::c_void,
                &mut len,
                std::ptr::null_mut(),
                0,
            ) == 0
                && len == std::mem::size_of::<u64>()
            {
                return size;
            }
        }
        0
    }
    #[cfg(not(any(windows, target_os = "linux", target_os = "macos")))]
    {
        // 未知平台：无统一内存 API，返回 0 交回退值（UNKNOWN_PHYS_GB）
        0
    }
}

// ==================== 预算与上限换算 ====================

/// 自动预算 = (物理 − 2GB) × 55%，夹 [2, 48]
pub(crate) fn auto_budget_gb() -> f64 {
    let phys = physical_mem_gb();
    ((phys - RESERVED_GB).max(BUDGET_MIN_GB) * AUTO_RATIO).clamp(BUDGET_MIN_GB, BUDGET_MAX_GB)
}

/// 用户覆盖值：plugins/mat.json 的 `analysisBudgetGb`（数字），夹 [2, 48]
pub(crate) fn user_override_gb() -> Option<f64> {
    crate::app::kv_store::kv_read_value("plugins/mat.json", "analysisBudgetGb")
        .and_then(|v| v.as_f64())
        .map(|g| g.clamp(BUDGET_MIN_GB, BUDGET_MAX_GB))
}

/// 生效预算 = 覆盖值或自动值
pub(crate) fn effective_budget_gb() -> f64 {
    user_override_gb().unwrap_or_else(auto_budget_gb)
}

/// 预算 → 各环节上限（floor，下限 1，防 0）
pub(crate) fn limits_for_budget(budget_gb: f64) -> MatLimits {
    let gb_to_b = |share: f64| budget_gb * share * 1e9;
    let refs_nodes = (gb_to_b(REFS_BUDGET_SHARE) / REFS_BYTES_PER_NODE).floor().max(1.0) as usize;
    let refs_edges = ((refs_nodes as f64) * REFS_EDGES_PER_NODE) as usize;
    let dom_nodes = (gb_to_b(DOM_BUDGET_SHARE) / DOM_BYTES_PER_NODE).floor().max(1.0) as usize;
    let dom_edges = ((dom_nodes as f64) * DOM_EDGES_PER_NODE) as usize;
    MatLimits { budget_gb, refs_nodes, refs_edges, dom_nodes, dom_edges }
}

/// 当前生效上限（按用户覆盖或自动预算即时换算；预算文件可能被前端改动）
pub(crate) fn current_limits() -> MatLimits {
    limits_for_budget(effective_budget_gb())
}

// ==================== 命令入口（纯业务函数，薄包装见 commands.rs） ====================

/// 查询当前预算与各环节上限（前端「分析预算」设置弹窗用；不依赖任何 dump）
pub fn mat_analysis_limits() -> Result<ResData<MatLimitInfo>, String> {
    let phys = physical_mem_gb();
    let auto = auto_budget_gb();
    let ov = user_override_gb();
    let eff = ov.unwrap_or(auto);
    let l = limits_for_budget(eff);
    Ok(ResData {
        code: 200,
        msg: "ok".to_string(),
        data: MatLimitInfo {
            phys_gb: phys,
            auto_budget_gb: auto,
            override_gb: ov,
            effective_budget_gb: eff,
            refs_nodes: l.refs_nodes as u64,
            refs_edges: l.refs_edges as u64,
            dom_nodes: l.dom_nodes as u64,
            dom_edges: l.dom_edges as u64,
        },
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 参考预算 7.7GB（16GB 参考机自动值）必须精确回到历史常量校准点，
    /// 保证默认行为与改动态上限前完全一致（±2 容差防浮点边界）。
    #[test]
    fn ref_budget_reproduces_legacy_constants() {
        let l = limits_for_budget(REF_BUDGET_GB);
        assert!((l.refs_nodes as i64 - 20_000_000).abs() <= 2, "refs_nodes={}", l.refs_nodes);
        assert!((l.refs_edges as i64 - 100_000_000).abs() <= 10, "refs_edges={}", l.refs_edges);
        assert!((l.dom_nodes as i64 - 25_000_000).abs() <= 2, "dom_nodes={}", l.dom_nodes);
        assert!((l.dom_edges as i64 - 120_000_000).abs() <= 10, "dom_edges={}", l.dom_edges);
    }

    /// 预算单调性：预算越大各环节上限越大（用户在设置里调高的直觉要成立）
    #[test]
    fn limits_grow_with_budget() {
        let small = limits_for_budget(4.0);
        let big = limits_for_budget(32.0);
        assert!(big.refs_nodes > small.refs_nodes);
        assert!(big.refs_edges > small.refs_edges);
        assert!(big.dom_nodes > small.dom_nodes);
        assert!(big.dom_edges > small.dom_edges);
        assert!((big.refs_nodes as f64 / small.refs_nodes as f64 - 8.0).abs() < 0.1);
    }
}
