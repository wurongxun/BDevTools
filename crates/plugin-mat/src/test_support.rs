//! 测试公共工具（`#[cfg(test)]`）。
//!
//! ## 为什么要有这个模块
//!
//! 真实 dump 探针原先写死了开发机绝对路径（`D:/ideaProject/.../leak_demo.hprof`），
//! 因此只能用 `#[ignore]` 挂着 —— 别人 clone 下来既跑不了、也不知道该怎么跑。
//! 现在改成**环境变量注入**：
//!
//! ```text
//! BDT_MAT_SAMPLE=/path/to/leak_demo.hprof cargo test -p bdevtools-plugin-mat --lib real_dump -- --nocapture
//! ```
//!
//! 约定：
//! - **未设置** ⇒ 用例打印 skip 后返回（测试仍然通过，CI 没有样例也能全绿）；
//! - **设置了但文件不存在** ⇒ 直接 panic。这是刻意的：环境变量是你主动给的，
//!   静默跳过会让人误以为「回归跑过了」。
//!
//! 样例来源见 `mat-realcheck/README.md`（LeakDemo.java + `jmap -dump`）。

/// 真实堆转储样例路径（来自 `BDT_MAT_SAMPLE`）。
///
/// 返回 `None` 表示未配置样例，调用方应打印 skip 并返回。
pub(crate) fn sample_dump() -> Option<String> {
    let raw = std::env::var("BDT_MAT_SAMPLE").ok()?;
    let p = raw.trim();
    if p.is_empty() {
        return None;
    }
    if !std::path::Path::new(p).is_file() {
        panic!("BDT_MAT_SAMPLE 指向的文件不存在：{p}");
    }
    Some(p.to_string())
}
