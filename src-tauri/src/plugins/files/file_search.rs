//! 文件搜索与批量替换（files 插件业务层，纯 Rust，无 Tauri 依赖）。
//!
//! M2 定案：**files 自建搜索能力，不再借用 `logs_query_lines`**。
//! `logs_query_lines` 带 `#[cfg(feature = "plugin-logs")]`，files 前端直接调它会让
//! 「只开 plugin-files」的裁剪构建里大文件搜索静默失效；下沉 core 或声明
//! `plugin-files → plugin-logs` 依赖都会把 logs 的重依赖拖进来，违背编译期裁剪。
//!
//! 三条硬约束：
//! 1. **不新增任何 Cargo 依赖**（切 feature/依赖组合会改 fingerprint 触发全量重编）；
//! 2. 扫描走 `spawn_blocking` + 同步 IO，避免长任务冻住 tokio 运行时；
//! 3. 写回**只经 `file_meta::save_note`**（原子写 + 原编码/EOL/BOM + 409 冲突检测），
//!    批量替换绝不自己 `std::fs::write`，且写前先落 `.bak` 供撤销。
//!
//! 返回字段统一 snake_case，与 `log_index` 的既有契约一致。

use crate::response::resp_data::ResData;
use lazy_static::lazy_static;
use regex::{Regex, RegexBuilder};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};

use super::file_meta::{self, SaveMeta};

/// 默认单文件扫描上限：超过则跳过并计数，避免把 50MB 日志整份读进内存。
pub const DEFAULT_MAX_FILE_SIZE: u64 = 4 * 1024 * 1024;
/// 默认最多扫描多少文件 / 收集多少命中，防止一次搜索把 IPC 撑爆。
pub const DEFAULT_MAX_FILES: usize = 2000;
pub const DEFAULT_MAX_MATCHES: usize = 20_000;

/// 常见「不该搜」的目录。用户仍可用 include/exclude glob 精确控制。
const IGNORE_DIRS: &[&str] = &[
    "node_modules", "target", "dist", "build", "out", ".git", ".svn", ".hg",
    ".idea", ".vscode", ".workbuddy", ".cache", ".next", "__pycache__",
    ".venv", "venv", ".gradle", "bin", "obj",
];

// ==================== 取消令牌（复用 logs 范式：按 task_id 查一次，之后键即凭证） ====================

lazy_static! {
    static ref TASKS: Mutex<HashMap<String, Arc<AtomicBool>>> = Mutex::new(HashMap::new());
}

fn register_task(task_id: &Option<String>) -> Arc<AtomicBool> {
    let Some(id) = task_id.as_ref().filter(|s| !s.is_empty()) else {
        return Arc::new(AtomicBool::new(false));
    };
    let flag = Arc::new(AtomicBool::new(false));
    let mut m = TASKS.lock().unwrap_or_else(|p| p.into_inner());
    // 前端可能泄漏 task_id；表超过 64 时先清掉已结束/已取消的条目
    if m.len() > 64 {
        m.retain(|_, v| !v.load(Ordering::Relaxed));
    }
    m.insert(id.clone(), flag.clone());
    flag
}

fn unregister_task(task_id: &Option<String>) {
    if let Some(id) = task_id.as_ref().filter(|s| !s.is_empty()) {
        let mut m = TASKS.lock().unwrap_or_else(|p| p.into_inner());
        m.remove(id);
    }
}

/// 请求取消一个正在跑的搜索/替换任务；返回是否命中。
pub fn cancel_task(task_id: String) -> bool {
    let m = TASKS.lock().unwrap_or_else(|p| p.into_inner());
    match m.get(&task_id) {
        Some(f) => {
            f.store(true, Ordering::Relaxed);
            true
        }
        None => false,
    }
}

// ==================== 请求 / 结果数据结构 ====================

#[derive(Debug, Clone)]
pub struct SearchRequest {
    pub root_path: String,
    pub keyword: String,
    pub mode: String,
    pub ignore_case: bool,
    pub include: String,
    pub exclude: String,
    pub max_file_size: u64,
    pub max_files: usize,
    pub max_matches: usize,
    pub task_id: Option<String>,
}

#[derive(Debug, Clone)]
pub struct LineQueryRequest {
    pub file_path: String,
    pub keyword: String,
    pub mode: String,
    pub ignore_case: bool,
    pub max_matches: usize,
    pub task_id: Option<String>,
}

/// 替换目标由搜索结果直接带上 mtime/size 基线，写回时交给 save_note 做 409 检测。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReplaceTarget {
    pub path: String,
    pub mtime_ms: i64,
    pub size: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UndoEntry {
    pub path: String,
    pub backup_path: String,
}

#[derive(Debug, Clone)]
pub struct ReplaceRequest {
    pub keyword: String,
    pub replacement: String,
    pub mode: String,
    pub ignore_case: bool,
    pub targets: Vec<ReplaceTarget>,
    pub dry_run: bool,
    pub task_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Default)]
pub struct SearchHit {
    pub line: usize,
    pub text: String,
    pub start: usize,
    pub end: usize,
}

#[derive(Debug, Clone, Serialize, Default)]
pub struct FileHits {
    pub path: String,
    pub rel_path: String,
    pub hits: Vec<SearchHit>,
    pub encoding: String,
    pub eol: String,
    pub bom: String,
    pub mtime_ms: i64,
    pub size: u64,
}

#[derive(Debug, Clone, Serialize, Default)]
pub struct SearchOutcome {
    pub files: Vec<FileHits>,
    pub total_matches: usize,
    pub files_scanned: usize,
    pub files_skipped: usize,
    pub truncated: bool,
    pub cancelled: bool,
}

/// 单文件查询契约保持与旧 `logs_query_lines` 的 `matched / lines / matches_truncated` 同名，
/// TextView 只需换命令名即可零改动消费。
#[derive(Debug, Clone, Serialize, Default)]
pub struct LineQueryOutcome {
    pub lines: Vec<usize>,
    pub matched: usize,
    pub matches_truncated: bool,
    pub cancelled: bool,
}

#[derive(Debug, Clone, Serialize, Default)]
pub struct ReplaceSample {
    pub line: usize,
    pub before: String,
    pub after: String,
}

#[derive(Debug, Clone, Serialize, Default)]
pub struct ReplaceItem {
    pub path: String,
    pub rel_path: String,
    pub code: u32,
    pub msg: String,
    pub replacements: usize,
    pub samples: Vec<ReplaceSample>,
    pub backup_path: String,
    pub encoding: String,
    pub eol: String,
}

#[derive(Debug, Clone, Serialize, Default)]
pub struct ReplaceOutcome {
    pub dry_run: bool,
    pub items: Vec<ReplaceItem>,
    pub total_replacements: usize,
    pub files_changed: usize,
    pub cancelled: bool,
    pub backups: Vec<UndoEntry>,
}

#[derive(Debug, Clone, Serialize, Default)]
pub struct UndoOutcome {
    pub restored: usize,
    pub failed: Vec<String>,
}

// ==================== 通用工具 ====================

fn ok<T: Serialize>(data: T) -> ResData<T> {
    ResData {
        code: 200,
        msg: "success".to_string(),
        data,
    }
}

fn fail<T: Serialize + Default>(code: u32, msg: impl Into<String>) -> ResData<T> {
    ResData {
        code,
        msg: msg.into(),
        data: T::default(),
    }
}

fn now_ns() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0)
}

fn stamp_ms(t: SystemTime) -> i64 {
    t.duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0)
}

fn snapshot(path: &Path) -> (i64, u64) {
    match std::fs::metadata(path) {
        Ok(m) => (m.modified().map(stamp_ms).unwrap_or(0), m.len()),
        Err(_) => (0, 0),
    }
}

/// 相对路径统一成 `/`，glob 才有跨平台一致的行为。
fn rel_of(root: &Path, path: &Path) -> String {
    path.strip_prefix(root)
        .unwrap_or(path)
        .to_string_lossy()
        .replace('\\', "/")
}

fn base_name(path: &Path) -> String {
    path.file_name()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_default()
}

fn clamp(s: &str, max: usize) -> String {
    let mut out: String = s.chars().take(max).collect();
    if s.chars().count() > max {
        out.push('…');
    }
    out
}

fn looks_binary(bytes: &[u8]) -> bool {
    let n = bytes.len().min(8192);
    bytes[..n].contains(&0)
}

/// text 模式把关键词转义成字面量；regex 模式原样编译。
/// `regex` crate 本身是线性时间，不存在 PCRE 式灾难性回溯。
fn build_matcher(keyword: &str, mode: &str, ignore_case: bool) -> Result<(Regex, bool), String> {
    let is_regex = mode.eq_ignore_ascii_case("regex");
    let pattern = if is_regex {
        keyword.to_string()
    } else {
        regex::escape(keyword)
    };
    let re = RegexBuilder::new(&pattern)
        .case_insensitive(ignore_case)
        .size_limit(1 << 20)
        .build()
        .map_err(|e| format!("正则表达式无效: {e}"))?;
    Ok((re, is_regex))
}

/// 极简 glob：支持 `*` `?` `**`，多个模式用 `,` 或 `;` 分隔。
/// 不含 `/` 的模式自动允许匹配任意深度（`*.ts` 能命中 `src/a/b.ts`）。
fn glob_body(pattern: &str) -> String {
    let chars: Vec<char> = pattern.chars().collect();
    let mut out = String::new();
    let mut i = 0;
    while i < chars.len() {
        match chars[i] {
            '*' if i + 1 < chars.len() && chars[i + 1] == '*' => {
                out.push_str(".*");
                i += 2;
            }
            '*' => {
                out.push_str("[^/]*");
                i += 1;
            }
            '?' => {
                out.push_str("[^/]");
                i += 1;
            }
            c => {
                out.push_str(&regex::escape(&c.to_string()));
                i += 1;
            }
        }
    }
    out
}

fn compile_glob(input: &str) -> Option<Regex> {
    let parts: Vec<String> = input
        .split([',', ';'])
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(|p| {
            let normalized = p.replace('\\', "/");
            let body = glob_body(&normalized);
            if normalized.contains('/') {
                format!("(?:{body})")
            } else {
                format!("(?:.*/)?{body}")
            }
        })
        .collect();
    if parts.is_empty() {
        return None;
    }
    RegexBuilder::new(&format!("^(?:{})$", parts.join("|")))
        .case_insensitive(true)
        .build()
        .ok()
}

// ==================== 跨文件搜索 ====================

/// 同步遍历目录。显式栈避免深层目录递归爆栈；每个循环点检查取消标志。
fn collect_files(
    root: &Path,
    req: &SearchRequest,
    cancel: &AtomicBool,
) -> (Vec<PathBuf>, usize, bool, bool) {
    let include = compile_glob(&req.include);
    let exclude = compile_glob(&req.exclude);
    let mut stack = vec![root.to_path_buf()];
    let mut files = Vec::new();
    let mut skipped = 0usize;
    let mut truncated = false;
    let mut cancelled = false;

    while let Some(dir) = stack.pop() {
        if cancel.load(Ordering::Relaxed) {
            cancelled = true;
            break;
        }
        let entries = match std::fs::read_dir(&dir) {
            Ok(e) => e,
            Err(_) => continue,
        };
        for entry in entries.flatten() {
            if cancel.load(Ordering::Relaxed) {
                cancelled = true;
                break;
            }
            let path = entry.path();
            let name = entry.file_name().to_string_lossy().to_string();
            if path.is_dir() {
                if name.starts_with('.')
                    || name.starts_with(".tmp-")
                    || IGNORE_DIRS.contains(&name.as_str())
                {
                    continue;
                }
                let rel = rel_of(root, &path);
                if exclude
                    .as_ref()
                    .is_some_and(|r| r.is_match(&rel) || r.is_match(&format!("{rel}/")))
                {
                    continue;
                }
                stack.push(path);
                continue;
            }
            // files 自己生成的原子写临时文件 / 替换备份不能被搜到，
            // 否则「替换完再搜」会冒出一堆 .bdt-*.bak 幽灵结果。
            if name.starts_with('.') && name.contains(".bdt-") {
                continue;
            }
            let rel = rel_of(root, &path);
            if include.as_ref().is_some_and(|r| !r.is_match(&rel)) {
                continue;
            }
            if exclude.as_ref().is_some_and(|r| r.is_match(&rel)) {
                skipped += 1;
                continue;
            }
            let size = entry.metadata().map(|m| m.len()).unwrap_or(0);
            if size > req.max_file_size {
                skipped += 1;
                continue;
            }
            if files.len() >= req.max_files {
                truncated = true;
                break;
            }
            files.push(path);
        }
        if truncated || cancelled {
            break;
        }
    }
    (files, skipped, truncated, cancelled)
}

fn scan_files(
    root: &Path,
    paths: Vec<PathBuf>,
    req: &SearchRequest,
    matcher: &Regex,
    cancel: &AtomicBool,
) -> SearchOutcome {
    let mut out = SearchOutcome::default();
    for path in paths {
        if cancel.load(Ordering::Relaxed) {
            out.cancelled = true;
            break;
        }
        let bytes = match std::fs::read(&path) {
            Ok(b) => b,
            Err(_) => {
                out.files_skipped += 1;
                continue;
            }
        };
        if looks_binary(&bytes) {
            out.files_skipped += 1;
            continue;
        }
        out.files_scanned += 1;
        let (encoding, bom, text) = file_meta::decode_bytes(&bytes);
        let eol = file_meta::detect_eol(&text);
        let (mtime_ms, size) = snapshot(&path);
        let mut hits = Vec::new();

        for (idx, raw) in text.split('\n').enumerate() {
            let line = raw.strip_suffix('\r').unwrap_or(raw);
            if let Some(m) = matcher.find(line) {
                hits.push(SearchHit {
                    line: idx + 1,
                    text: clamp(line, 400),
                    start: m.start(),
                    end: m.end(),
                });
                out.total_matches += 1;
                if out.total_matches >= req.max_matches {
                    out.truncated = true;
                    break;
                }
            }
        }
        if !hits.is_empty() {
            out.files.push(FileHits {
                path: path.to_string_lossy().to_string(),
                rel_path: rel_of(root, &path),
                hits,
                encoding,
                eol,
                bom,
                mtime_ms,
                size,
            });
        }
        if out.truncated {
            break;
        }
    }
    out
}

/// 跨目录搜索。返回按文件分组的命中（带行号/上下文/编码/EOL/mtime），
/// mtime 与 size 直接作为后续批量替换的冲突基线，不必二次读盘。
pub async fn search_in_files(req: SearchRequest) -> Result<ResData<SearchOutcome>, String> {
    if req.keyword.is_empty() {
        return Ok(fail(400, "关键词不能为空"));
    }
    let root = PathBuf::from(&req.root_path);
    if !root.is_dir() {
        return Ok(fail(400, format!("搜索目录不存在: {}", req.root_path)));
    }
    let (matcher, _) = build_matcher(&req.keyword, &req.mode, req.ignore_case)
        .map_err(|e| format!("搜索条件无效: {e}"))?;
    let cancel = register_task(&req.task_id);
    let task_id = req.task_id.clone();
    let joined = tokio::task::spawn_blocking(move || {
        let (paths, skipped, truncated, cancelled) = collect_files(&root, &req, &cancel);
        let mut outcome = scan_files(&root, paths, &req, &matcher, &cancel);
        outcome.files_skipped += skipped;
        outcome.truncated |= truncated;
        outcome.cancelled |= cancelled;
        outcome
    })
    .await;
    // 即使任务 panic 也要摘掉令牌，否则表里会留下永远等不到结束的条目
    unregister_task(&task_id);
    let result = joined.map_err(|e| format!("搜索任务失败: {e}"))?;
    Ok(ok(result))
}

// ==================== 单文件按行查询（替代 logs_query_lines） ====================

pub async fn query_lines(req: LineQueryRequest) -> Result<ResData<LineQueryOutcome>, String> {
    if req.keyword.is_empty() {
        return Ok(fail(400, "关键词不能为空"));
    }
    let path = PathBuf::from(&req.file_path);
    if !path.is_file() {
        return Ok(fail(400, format!("文件不存在: {}", req.file_path)));
    }
    let (matcher, _) = build_matcher(&req.keyword, &req.mode, req.ignore_case)
        .map_err(|e| format!("搜索条件无效: {e}"))?;
    let cancel = register_task(&req.task_id);
    let task_id = req.task_id.clone();
    // closure 返回 Result：读文件失败必须变成 500，不能伪装成「零命中」静默吞掉。
    let joined = tokio::task::spawn_blocking(move || -> Result<LineQueryOutcome, String> {
        let bytes = std::fs::read(&path).map_err(|e| format!("读取文件失败: {e}"))?;
        let mut out = LineQueryOutcome::default();
        let (_enc, _bom, text) = file_meta::decode_bytes(&bytes);
        for (idx, raw) in text.split('\n').enumerate() {
            if cancel.load(Ordering::Relaxed) {
                out.cancelled = true;
                break;
            }
            let line = raw.strip_suffix('\r').unwrap_or(raw);
            if matcher.is_match(line) {
                out.matched += 1;
                if out.lines.len() < req.max_matches {
                    out.lines.push(idx + 1);
                } else {
                    out.matches_truncated = true;
                }
            }
        }
        Ok(out)
    })
    .await;
    unregister_task(&task_id);
    let result = joined.map_err(|e| format!("查询任务失败: {e}"))??;
    Ok(ok(result))
}

// ==================== 批量替换（预览 / 应用 / 撤销） ====================

fn backup_path(path: &Path) -> PathBuf {
    let parent = path.parent().unwrap_or_else(|| Path::new("."));
    let name = path
        .file_name()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_else(|| "file".to_string());
    parent.join(format!(
        ".{name}.bdt-replace-{}-{}.bak",
        std::process::id(),
        now_ns()
    ))
}

/// 把一行里的 `\r`（CRLF 文件）摘掉，交给 `normalize_eol` 统一还原，
/// 避免替换后 EOL 被静默改成 LF。
fn replace_in_text(text: &str, matcher: &Regex, replacement: &str, is_regex: bool) -> (String, usize, Vec<ReplaceSample>) {
    let mut lines = Vec::new();
    let mut total = 0usize;
    let mut samples = Vec::new();
    for (idx, raw) in text.split('\n').enumerate() {
        let line = raw.strip_suffix('\r').unwrap_or(raw);
        let n = matcher.find_iter(line).count();
        total += n;
        let after = if is_regex {
            matcher.replace_all(line, replacement).into_owned()
        } else {
            // text 模式下 replacement 里的 `$1` 必须当字面量，不能被当捕获组展开
            matcher
                .replace_all(line, regex::NoExpand(replacement))
                .into_owned()
        };
        if n > 0 && samples.len() < 3 {
            samples.push(ReplaceSample {
                line: idx + 1,
                before: clamp(line, 200),
                after: clamp(&after, 200),
            });
        }
        lines.push(after);
    }
    (lines.join("\n"), total, samples)
}

/// `dry_run=true` 只算预览、绝不落盘；`false` 时逐文件「备份 → 原子写 → 记 undo 条目」。
/// 任何 409（外部已改）/ 415（编码不可表示）/ 500 都只跳过该文件并回报，不影响其它文件。
pub async fn replace_in_files(req: ReplaceRequest) -> Result<ResData<ReplaceOutcome>, String> {
    if req.keyword.is_empty() {
        return Ok(fail(400, "关键词不能为空"));
    }
    let (matcher, is_regex) = build_matcher(&req.keyword, &req.mode, req.ignore_case)
        .map_err(|e| format!("替换条件无效: {e}"))?;
    let cancel = register_task(&req.task_id);
    let task_id = req.task_id.clone();
    // core 里任何 `?` 提前返回都要先摘令牌，故注销留在外层统一做。
    let result = replace_core(&req, &matcher, is_regex, &cancel).await;
    unregister_task(&task_id);
    result.map(ok)
}

async fn replace_core(
    req: &ReplaceRequest,
    matcher: &Regex,
    is_regex: bool,
    cancel: &AtomicBool,
) -> Result<ReplaceOutcome, String> {
    let mut out = ReplaceOutcome {
        dry_run: req.dry_run,
        ..Default::default()
    };

    for target in &req.targets {
        if cancel.load(Ordering::Relaxed) {
            out.cancelled = true;
            break;
        }
        let path = Path::new(&target.path);
        let bytes = match tokio::fs::read(path).await {
            Ok(b) => b,
            Err(e) => {
                out.items.push(ReplaceItem {
                    path: target.path.clone(),
                    rel_path: base_name(path),
                    code: 500,
                    msg: format!("读取失败: {e}"),
                    ..Default::default()
                });
                continue;
            }
        };
        let (encoding, bom, text) = file_meta::decode_bytes(&bytes);
        let eol = file_meta::detect_eol(&text);
        let (new_text, count, samples) =
            replace_in_text(&text, &matcher, &req.replacement, is_regex);

        let mut item = ReplaceItem {
            path: target.path.clone(),
            rel_path: base_name(path),
            code: 200,
            msg: "ok".to_string(),
            replacements: count,
            samples,
            encoding: encoding.clone(),
            eol: eol.clone(),
            ..Default::default()
        };
        if count == 0 {
            item.msg = "无匹配".to_string();
            out.items.push(item);
            continue;
        }

        if req.dry_run {
            out.total_replacements += count;
            out.items.push(item);
            continue;
        }

        let bak = backup_path(path);
        if let Err(e) = tokio::fs::copy(path, &bak).await {
            item.code = 500;
            item.msg = format!("备份失败，未改动原文件: {e}");
            out.items.push(item);
            continue;
        }
        // 唯一写回通道：原子写 + 原编码/EOL/BOM + mtime/size 冲突检测
        let res = file_meta::save_note(
            target.path.clone(),
            new_text,
            SaveMeta {
                encoding: Some(encoding),
                eol: Some(eol),
                bom: Some(bom),
                expected_mtime_ms: Some(target.mtime_ms),
                expected_size: Some(target.size),
            },
        )
        .await?;
        if res.code != 200 {
            let _ = tokio::fs::remove_file(&bak).await;
            item.code = res.code;
            item.msg = res.msg;
            out.items.push(item);
            continue;
        }
        item.backup_path = bak.to_string_lossy().to_string();
        out.backups.push(UndoEntry {
            path: target.path.clone(),
            backup_path: item.backup_path.clone(),
        });
        out.total_replacements += count;
        out.files_changed += 1;
        out.items.push(item);
        tokio::task::yield_now().await;
    }
    Ok(out)
}

/// 从 `.bak` 原样恢复字节（不经过编码转换），成功后删除备份。
pub async fn undo_replace(entries: Vec<UndoEntry>) -> Result<ResData<UndoOutcome>, String> {
    let mut out = UndoOutcome::default();
    for entry in entries {
        match tokio::fs::read(&entry.backup_path).await {
            Ok(bytes) => {
                if let Err(e) = file_meta::write_atomic(Path::new(&entry.path), &bytes).await {
                    out.failed.push(format!("{}: {e}", entry.path));
                } else {
                    out.restored += 1;
                    let _ = tokio::fs::remove_file(&entry.backup_path).await;
                }
            }
            Err(e) => out.failed.push(format!("{}: 备份读取失败 {e}", entry.path)),
        }
    }
    Ok(ok(out))
}

/// 重启恢复用：落盘的撤销清单里，只保留备份文件仍然存在的条目。
/// 用户可能手动清过 `.bak`、或备份写在临时目录后被系统回收，此时「撤销」按钮
/// 若仍亮着，点下去必然整批失败，属于明摆着的假可用状态。
pub async fn probe_undo_entries(entries: Vec<UndoEntry>) -> Vec<UndoEntry> {
    let mut out = Vec::new();
    for entry in entries {
        if tokio::fs::metadata(&entry.backup_path)
            .await
            .map(|m| m.is_file())
            .unwrap_or(false)
        {
            out.push(entry);
        }
    }
    out
}

/// 清理一批 `.bak`（撤销成功后或用户主动丢弃撤销清单时）。删不掉只记不报错，
/// 因为备份是隐藏文件、删不掉不影响主流程，但要把异常回报给前端避免静默堆积。
pub async fn discard_undo_entries(entries: Vec<UndoEntry>) -> Vec<String> {
    let mut failed = Vec::new();
    for entry in entries {
        match tokio::fs::metadata(&entry.backup_path).await {
            Ok(_) => {
                if let Err(e) = tokio::fs::remove_file(&entry.backup_path).await {
                    failed.push(format!("{}: {e}", entry.backup_path));
                }
            }
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
            Err(e) => failed.push(format!("{}: {e}", entry.backup_path)),
        }
    }
    failed
}

// ==================== 单测 ====================

#[cfg(test)]
mod tests {
    use super::*;

    fn tmp_dir(tag: &str) -> PathBuf {
        let d = std::env::temp_dir().join(format!(
            "bdevtools_file_search_{}_{}_{}",
            tag,
            std::process::id(),
            now_ns()
        ));
        std::fs::create_dir_all(&d).unwrap();
        d
    }

    fn sample_req(root: &Path, keyword: &str) -> SearchRequest {
        SearchRequest {
            root_path: root.to_string_lossy().to_string(),
            keyword: keyword.to_string(),
            mode: "text".to_string(),
            ignore_case: true,
            include: String::new(),
            exclude: String::new(),
            max_file_size: DEFAULT_MAX_FILE_SIZE,
            max_files: DEFAULT_MAX_FILES,
            max_matches: DEFAULT_MAX_MATCHES,
            task_id: None,
        }
    }

    #[test]
    fn glob_matches_basename_at_any_depth() {
        let re = compile_glob("*.ts, *.vue").unwrap();
        assert!(re.is_match("a.ts"));
        assert!(re.is_match("src/deep/b.ts"));
        assert!(re.is_match("src/c.vue"));
        assert!(!re.is_match("src/d.md"));
        let deep = compile_glob("src/**/*.rs").unwrap();
        assert!(deep.is_match("src/a/b.rs"));
    }

    #[test]
    fn scan_respects_glob_binary_and_cancel() {
        let root = tmp_dir("scan");
        std::fs::create_dir_all(root.join("node_modules")).unwrap();
        std::fs::create_dir_all(root.join("src")).unwrap();
        std::fs::write(root.join("src/a.ts"), "hello TODO one\nworld\n").unwrap();
        std::fs::write(root.join("src/b.md"), "TODO in md\n").unwrap();
        std::fs::write(root.join("node_modules/c.ts"), "TODO ignored\n").unwrap();
        std::fs::write(root.join("src/bin.dat"), b"TODO\0binary").unwrap();

        let mut req = sample_req(&root, "todo");
        req.include = "*.ts".to_string();
        let (re, _) = build_matcher(&req.keyword, &req.mode, req.ignore_case).unwrap();
        let flag = AtomicBool::new(false);
        let (paths, _, _, _) = collect_files(&root, &req, &flag);
        assert_eq!(paths.len(), 1, "只应命中 src/a.ts");
        let out = scan_files(&root, paths, &req, &re, &flag);
        assert_eq!(out.total_matches, 1);
        assert_eq!(out.files.len(), 1);
        assert_eq!(out.files[0].hits[0].line, 1);

        // 预置取消标志 ⇒ 一个文件都不扫
        let cancelled = AtomicBool::new(true);
        let (paths2, _, _, stopped) = collect_files(&root, &req, &cancelled);
        assert!(stopped);
        assert!(paths2.is_empty());
    }

    #[test]
    fn regex_mode_and_case_switch_share_one_matcher() {
        let (re, _) = build_matcher(r"TODO\s+\d+", "regex", true).unwrap();
        assert!(re.is_match("todo 42 done"));
        let (plain, _) = build_matcher("TODO", "text", false).unwrap();
        assert!(plain.is_match("x TODO y"));
        assert!(!plain.is_match("x todo y"));
    }

    #[tokio::test]
    async fn query_lines_returns_hit_line_numbers() {
        let root = tmp_dir("query");
        let file = root.join("a.txt");
        std::fs::write(&file, "alpha\nBeta\nalpha beta\n").unwrap();
        let res = query_lines(LineQueryRequest {
            file_path: file.to_string_lossy().to_string(),
            keyword: "alpha".to_string(),
            mode: "text".to_string(),
            ignore_case: false,
            max_matches: 100,
            task_id: None,
        })
        .await
        .unwrap();
        assert_eq!(res.code, 200);
        assert_eq!(res.data.lines, vec![1, 3]);
        assert_eq!(res.data.matched, 2);
    }

    fn target_for(path: &Path) -> ReplaceTarget {
        let (mtime_ms, size) = snapshot(path);
        ReplaceTarget {
            path: path.to_string_lossy().to_string(),
            mtime_ms,
            size,
        }
    }

    fn replace_req(targets: Vec<ReplaceTarget>, dry_run: bool) -> ReplaceRequest {
        ReplaceRequest {
            keyword: "TODO".to_string(),
            replacement: "DONE".to_string(),
            mode: "text".to_string(),
            ignore_case: false,
            targets,
            dry_run,
            task_id: None,
        }
    }

    #[tokio::test]
    async fn preview_does_not_touch_disk_and_apply_keeps_gbk_crlf() {
        let root = tmp_dir("replace");
        let file = root.join("zh.txt");
        let (gbk, _, _) = encoding_rs::GBK.encode("中文 TODO\r\n第二行 TODO");
        std::fs::write(&file, gbk.as_ref()).unwrap();
        let before = std::fs::read(&file).unwrap();

        let target = target_for(&file);
        let preview = replace_in_files(replace_req(vec![target.clone()], true))
            .await
            .unwrap();
        assert!(preview.data.dry_run);
        assert_eq!(preview.data.total_replacements, 2);
        assert_eq!(std::fs::read(&file).unwrap(), before, "预览绝不能落盘");
        assert!(preview.data.backups.is_empty());
        assert_eq!(preview.data.items[0].samples.len(), 2);

        let applied = replace_in_files(replace_req(vec![target], false))
            .await
            .unwrap();
        assert_eq!(applied.data.files_changed, 1);
        assert_eq!(applied.data.backups.len(), 1);
        let bytes = std::fs::read(&file).unwrap();
        assert!(bytes.windows(2).any(|w| w == b"\r\n"), "CRLF 必须保持");
        let (text, _, _) = encoding_rs::GBK.decode(&bytes);
        assert_eq!(text, "中文 DONE\r\n第二行 DONE", "GBK 必须保持");

        let undo = undo_replace(applied.data.backups).await.unwrap();
        assert_eq!(undo.data.restored, 1);
        assert_eq!(std::fs::read(&file).unwrap(), before, "撤销应逐字节还原");
    }

    #[tokio::test]
    async fn conflict_skips_file_without_overwriting() {
        let root = tmp_dir("conflict");
        let file = root.join("a.txt");
        std::fs::write(&file, b"TODO keep").unwrap();
        let target = ReplaceTarget {
            path: file.to_string_lossy().to_string(),
            mtime_ms: 1, // 必然与真实 mtime 不符 ⇒ 409
            size: 9,
        };
        let res = replace_in_files(replace_req(vec![target], false))
            .await
            .unwrap();
        assert_eq!(res.data.items[0].code, file_meta::CODE_CONFLICT);
        assert_eq!(res.data.files_changed, 0);
        assert_eq!(std::fs::read(&file).unwrap(), b"TODO keep");
        assert!(res.data.backups.is_empty(), "失败不应留下 undo 条目");
    }

    #[test]
    fn generated_backups_are_not_searched() {
        let root = tmp_dir("backupskip");
        std::fs::write(root.join("a.ts"), "TODO real\n").unwrap();
        std::fs::write(root.join(".a.ts.bdt-replace-1-2.bak"), "TODO backup\n").unwrap();
        let req = sample_req(&root, "TODO");
        let flag = AtomicBool::new(false);
        let (paths, _, _, _) = collect_files(&root, &req, &flag);
        assert_eq!(paths.len(), 1, "替换备份不能变成幽灵搜索结果");
        assert!(paths[0].ends_with("a.ts"));
    }

    #[tokio::test]
    async fn query_missing_file_is_not_a_silent_zero_hit() {
        let root = tmp_dir("missing");
        let res = query_lines(LineQueryRequest {
            file_path: root.join("nope.txt").to_string_lossy().to_string(),
            keyword: "x".to_string(),
            mode: "text".to_string(),
            ignore_case: true,
            max_matches: 10,
            task_id: None,
        })
        .await
        .unwrap();
        assert_eq!(res.code, 400, "读不到文件必须报错，不能伪装成零命中");
    }

    #[tokio::test]
    async fn regex_replacement_expands_capture_groups() {
        let root = tmp_dir("regexreplace");
        let file = root.join("a.txt");
        std::fs::write(&file, "foo=1\nfoo=2\n").unwrap();
        let req = ReplaceRequest {
            keyword: r"foo=(\d+)".to_string(),
            replacement: "bar[$1]".to_string(),
            mode: "regex".to_string(),
            ignore_case: false,
            targets: vec![target_for(&file)],
            dry_run: false,
            task_id: None,
        };
        let res = replace_in_files(req).await.unwrap();
        assert_eq!(res.data.total_replacements, 2);
        assert_eq!(std::fs::read_to_string(&file).unwrap(), "bar[1]\nbar[2]\n");
    }
}
