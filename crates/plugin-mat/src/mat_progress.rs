//! 分析进度汇（主进程 / sidecar 子进程共用一套打点）。
//!
//! ## 为什么是「全局汇」而不是给每个函数加回调参数
//!
//! 打点要埋在 `mat_graph` 的两遍扫描、`mat_dom` 的 Lengauer-Tarjan 里 —— 这些函数
//! 已经被直方图 / OQL / 报告 / 泄漏检测共 10 余处调用。给它们逐个加 `&dyn Fn` 参数
//! 会传染整条调用链，而其中 99% 的调用方根本不需要进度。
//!
//! 这里改成「进程级汇」：默认 `None`，此时 `tick` 只做**一次原子读**就返回，
//! 老路径零开销；只有 sidecar 子进程启动时装一个「写 stdout NDJSON」的汇。
//!
//! ## 节流
//!
//! 5000 万对象的堆会调 `tick` 几千万次，直接发事件会把 pipe 打满、把前端主线程淹掉。
//! 因此按**数量**节流（默认每 5 万条记录发一次）；阶段切换用 `stage()`，它总是立即发出
//! （阶段只有个位数，不存在洪泛）。

use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};

/// 一条进度事件
#[derive(Debug, Clone)]
pub struct Progress {
    /// 阶段标识（`sniff` / `graph-meta` / `graph-edges` / `dom` / `index`）
    pub stage: &'static str,
    /// 已处理记录数（同阶段内单调递增）
    pub done: u64,
    /// 预计总量；`None` = 未知（HPROF 不记录记录数，只能给「不确定」进度条）
    pub total: Option<u64>,
    /// 人读文案（阶段说明 / 明细）
    pub msg: String,
}

/// 发送间隔（记录条数）
const THROTTLE_UNITS: u64 = 50_000;

lazy_static::lazy_static! {
    static ref SINK: parking_lot::Mutex<Option<Box<dyn Fn(Progress) + Send + Sync>>> =
        parking_lot::Mutex::new(None);
}

/// 是否装了汇。**热路径只读它**，装了汇才去抢锁。
static ENABLED: AtomicBool = AtomicBool::new(false);
/// 上一次已发出的 `done`（按阶段重置，故与 `STAGE_SEQ` 配合使用）
static LAST_SENT: AtomicU64 = AtomicU64::new(0);
/// 阶段序号：阶段切换时 +1，使节流基准失效
static STAGE_SEQ: AtomicU64 = AtomicU64::new(0);

/// 装汇（sidecar 用）。重复装会覆盖。
pub fn set_sink(sink: Box<dyn Fn(Progress) + Send + Sync>) {
    *SINK.lock() = Some(sink);
    LAST_SENT.store(0, Ordering::Relaxed);
    ENABLED.store(true, Ordering::Release);
}

/// 卸汇（测试与 sidecar 收尾用）
pub fn clear_sink() {
    ENABLED.store(false, Ordering::Release);
    *SINK.lock() = None;
}

/// 是否正在采集（前端 / 测试可据此跳过无谓计算）
pub fn enabled() -> bool {
    ENABLED.load(Ordering::Acquire)
}

/// 阶段切换：**总是立即发出**（阶段个位数，不会洪泛）
pub fn stage(stage: &'static str, total: Option<u64>, msg: impl Into<String>) {
    if !enabled() {
        return;
    }
    STAGE_SEQ.fetch_add(1, Ordering::Relaxed);
    LAST_SENT.store(0, Ordering::Relaxed);
    emit(Progress { stage, done: 0, total, msg: msg.into() });
}

/// 进度打点（按数量节流；`force=true` 强制发出，用于阶段收尾）
#[inline]
pub fn tick(stage: &'static str, done: u64, total: Option<u64>, msg: impl Into<String>) {
    if !enabled() {
        return;
    }
    let last = LAST_SENT.load(Ordering::Relaxed);
    if done < last + THROTTLE_UNITS {
        return;
    }
    // 抢不到就跳过本次（另一个线程刚发过）—— 进度丢帧不影响正确性
    if LAST_SENT
        .compare_exchange(last, done, Ordering::Relaxed, Ordering::Relaxed)
        .is_err()
    {
        return;
    }
    emit(Progress { stage, done, total, msg: msg.into() });
}

/// 强制发一条（不受节流影响）。阶段收尾用，保证「最后一条」不会因为
/// 没跨过 5 万的门槛而丢失。
pub fn flush(stage: &'static str, done: u64, total: Option<u64>, msg: impl Into<String>) {
    if !enabled() {
        return;
    }
    LAST_SENT.store(done, Ordering::Relaxed);
    emit(Progress { stage, done, total, msg: msg.into() });
}

fn emit(p: Progress) {
    // 锁内不发用户代码：把闭包取出来（闭包本身是 `&dyn Fn`，只在锁内调用会阻塞
    // 其它线程打点，但汇只有 sidecar 一个，不存在竞争热点）。
    let guard = SINK.lock();
    if let Some(f) = guard.as_ref() {
        f(p);
    }
}

/// 打点序号（阶段内自增，节流用不到，仅供测试断言用）
#[cfg(test)]
pub(crate) fn stage_seq() -> u64 {
    STAGE_SEQ.load(Ordering::Relaxed)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;

    /// 串行化：`set_sink` 是进程级全局状态，两个用例并行会互相覆盖。
    fn guard() -> parking_lot::MutexGuard<'static, ()> {
        lazy_static::lazy_static! {
            static ref L: parking_lot::Mutex<()> = parking_lot::Mutex::new(());
        }
        L.lock()
    }

    #[test]
    fn no_sink_is_zero_cost_noop() {
        let _g = guard();
        clear_sink();
        assert!(!enabled());
        // 没装汇时这些调用必须什么都不做、也不 panic
        stage("s", None, "x");
        tick("s", 10_000_000, None, "x");
        flush("s", 1, None, "x");
    }

    #[test]
    fn throttles_then_flush_always_sends() {
        let _g = guard();
        clear_sink();
        let got = Arc::new(parking_lot::Mutex::new(Vec::<u64>::new()));
        let g2 = got.clone();
        set_sink(Box::new(move |p| g2.lock().push(p.done)));

        tick("s", 10, None, ""); // 未过门槛 → 不发
        tick("s", 50_000, None, ""); // 正好跨过 → 发
        tick("s", 50_001, None, ""); // 未过下一次门槛 → 不发
        flush("s", 50_002, None, ""); // 强制 → 发

        clear_sink();
        assert_eq!(*got.lock(), vec![50_000, 50_002]);
    }

    #[test]
    fn stage_resets_throttle_baseline() {
        let _g = guard();
        clear_sink();
        let n = Arc::new(AtomicU64::new(0));
        let n2 = n.clone();
        set_sink(Box::new(move |_| {
            n2.fetch_add(1, Ordering::Relaxed);
        }));

        tick("a", 100_000, None, "");
        assert_eq!(n.load(Ordering::Relaxed), 1);
        stage("b", None, "切阶段"); // 立即发出
        assert_eq!(n.load(Ordering::Relaxed), 2);
        // 新阶段的小数值也能发（基准已重置）
        tick("b", 60_000, None, "");
        assert_eq!(n.load(Ordering::Relaxed), 3);

        clear_sink();
        assert_eq!(stage_seq(), 1);
    }
}
