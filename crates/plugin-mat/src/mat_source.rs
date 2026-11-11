//! # 堆转储来源解析（`mat_source`）—— 统一「路径 → 可 mmap 文件」的入口
//!
//! 目前支持：
//! - 原始 `.hprof`（直接 mmap，零拷贝，行为与历史完全一致）
//! - `.hprof.gz` / 任何 gzip 压缩的 HPROF（**先解压到缓存文件，再 mmap**）
//!
//! ## 为什么 gzip 必须落盘而不是解压进内存
//! 整个解析链路的前提是「mmap 零拷贝、按字节随机访问」（`jvm-hprof` 需要反复回跳读
//! ClassDump / 子记录）。gzip 是流式格式，无法随机访问：解压进 `Vec<u8>` 会让一个
//! 2GB 的 dump 立刻占用 2GB 常驻内存，与 `mat_cfg` 的「分析预算只算图结构」口径冲突，
//! 且多条命令各自解压一次会直接 OOM。落盘后各命令共享同一份缓存文件、各自 mmap
//! 文件页（由操作系统决定是否驻留），代价只有一次性解压时间与磁盘空间。
//!
//! ## 缓存键 = 源路径 + 源大小 + 修改时间
//! 三者任一变化即视为不同文件（`jmap` 覆盖写同名 dump 是很常见的操作，只按文件名
//! 做键会拿到上一次的内容）。键做成 **FNV-1a 哈希**而不是原路径，是为了避免长路径
//! 撞上 Windows 260 字符限制、以及类名/空格等字符进文件名后不好清理。
//!
//! ## 写入用「`.part` + rename」
//! 解压中途被杀（用户关窗口、磁盘满）时缓存目录里只会多一个 `.part`，绝不会出现
//! 一个长度不完整但名字正确的 `.hprof` 被后续命令当成功结果 mmap。
//! rename 失败且目标已存在 ⇒ 视为并发对手已解压完成，删掉自己的 `.part` 即可。

use bdevtools_core::response::resp_data::ResData;
use flate2::read::GzDecoder;
use memmap2::Mmap;
use serde::Serialize;
use std::fs::{self, File};
use std::io::{BufReader, Read, Write};
use std::path::{Path, PathBuf};

/// gzip 魔数（`1f 8b`）。HPROF 的魔数是 `JAVA PROFILE`，两者不会冲突。
const GZIP_MAGIC: [u8; 2] = [0x1f, 0x8b];

/// 解压上限（32 GiB）。
///
/// 不是「dump 不能更大」，而是防 gzip 炸弹：gzip 压缩比可以到 1000:1，
/// 一个几十 MB 的恶意/误生成的包能写满用户的磁盘。32 GiB 已覆盖常见的大堆场景，
/// 超过它更可能是异常输入，直接报错比默默写满 C 盘好。
const MAX_DECOMPRESSED_BYTES: u64 = 32 * 1024 * 1024 * 1024;

/// 缓存子目录名（落在 `app_dir` 下，不用 `config_dir`：它是用户资产/大文件，
/// 而 `config_dir` 会被卸载脚本整棵删除，放进去等于每次升级都要重解压）
const CACHE_DIR_NAME: &str = "mat_cache";

// ==================== 返回结构 ====================

/// 来源解析结果（供缓存管理 / 概览页展示）
#[derive(Debug, Serialize)]
pub struct SourceInfo {
    /// 实际用于 mmap 的 HPROF 路径（非压缩文件时等于源路径）
    pub hprof_path: String,
    /// 是否是 gzip 压缩包
    pub compressed: bool,
    /// 本次是否命中已解压缓存（true 表示没重新解压）
    pub cache_hit: bool,
    /// 源文件大小（字节）
    pub source_size: u64,
    /// 可用于 mmap 的 HPROF 大小（未压缩时 == source_size）
    pub hprof_size: u64,
    /// 补充说明（前端直接展示）
    pub note: String,
}

/// 缓存占用
#[derive(Debug, Serialize)]
pub struct CacheInfo {
    pub dir: String,
    /// 已完成的解压文件数
    pub files: usize,
    /// 解压文件总字节
    pub total_bytes: u64,
    /// 残留的半成品 `.part` 数（异常退出留下的，可安全清理）
    pub partial_files: usize,
}

/// 清理结果
#[derive(Debug, Serialize)]
pub struct CacheClearResult {
    pub removed: usize,
    pub removed_bytes: u64,
    /// 删不掉的文件（Windows 下仍被 mmap 占用时会失败）
    pub skipped: Vec<String>,
}

/// 解析后的来源
pub(crate) struct Resolved {
    pub hprof_path: PathBuf,
    pub compressed: bool,
    pub cache_hit: bool,
    pub source_size: u64,
    pub hprof_size: u64,
}

/// 人类可读体积（只在 note 里用，前端有自己的格式化，两边不必一致）
pub(crate) fn size_hint(bytes: u64) -> String {
    const MB: f64 = 1024.0 * 1024.0;
    const GB: f64 = MB * 1024.0;
    let b = bytes as f64;
    if b >= GB {
        format!("{:.2} GB", b / GB)
    } else if b >= MB {
        format!("{:.1} MB", b / MB)
    } else {
        format!("{bytes} B")
    }
}

impl Resolved {
    /// 对外展示的信息（note 由调用方按场景决定是否使用）
    pub fn info(&self) -> SourceInfo {
        let note = if !self.compressed {
            String::new()
        } else if self.cache_hit {
            format!(
                "已解压缓存命中（源 {} → HPROF {})",
                size_hint(self.source_size),
                size_hint(self.hprof_size)
            )
        } else {
            format!(
                "已从 gzip 解压（源 {} → HPROF {})",
                size_hint(self.source_size),
                size_hint(self.hprof_size)
            )
        };
        SourceInfo {
            hprof_path: self.hprof_path.to_string_lossy().to_string(),
            compressed: self.compressed,
            cache_hit: self.cache_hit,
            source_size: self.source_size,
            hprof_size: self.hprof_size,
            note,
        }
    }
}

// ==================== 基础工具 ====================

/// 只读前 2 字节判断是否 gzip（文件不存在时返回 false，让后续 open 报真正的错）
fn is_gzip(path: &str) -> bool {
    let mut buf = [0u8; 2];
    match File::open(path).and_then(|mut f| f.read_exact(&mut buf)) {
        Ok(()) => buf == GZIP_MAGIC,
        Err(_) => false,
    }
}

/// FNV-1a 64 位（只为拿一个稳定的短名，不做安全用途）
pub(crate) fn fnv1a64(bytes: &[u8]) -> u64 {
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    for &b in bytes {
        h ^= b as u64;
        h = h.wrapping_mul(0x0000_0100_0000_01b3);
    }
    h
}

/// 缓存根目录：`<app_dir>/mat_cache`。
///
/// - `BDT_MAT_CACHE_DIR` 可整体覆盖（手工调试用；单测走 `resolve_with` 注入，不依赖它）
/// - `app_dir` 未初始化时回退到系统临时目录（`cargo test` 下 APP 可能是空路径）
fn cache_root() -> PathBuf {
    if let Some(d) = std::env::var_os("BDT_MAT_CACHE_DIR") {
        return PathBuf::from(d);
    }
    let app_dir = bdevtools_core::app::app_store::APP.lock().app_dir.clone();
    let base = if app_dir.as_os_str().is_empty() {
        std::env::temp_dir().join("bdevtools")
    } else {
        app_dir
    };
    base.join(CACHE_DIR_NAME)
}

/// 源文件 mtime（毫秒）。拿不到就用 0 —— 仅影响缓存键区分度，不影响正确性
/// （大小不同的同名文件仍然分开）。
fn mtime_ms(meta: &std::fs::Metadata) -> i64 {
    meta.modified()
        .ok()
        .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0)
}

/// 缓存文件路径：`<root>/<fnv(path)>-<size>-<mtime>.hprof`
fn cache_path_for(root: &Path, src: &Path, size: u64, mtime: i64) -> PathBuf {
    let key = format!("{:016x}-{:x}-{:x}", fnv1a64(src.to_string_lossy().as_bytes()), size, mtime);
    root.join(format!("{key}.hprof"))
}

// ==================== 解压 ====================

/// 抢占 `.part` 的所有权。
///
/// **必须独占**：两个线程同时 `File::create` 同一个 `.part` 会各自拿到句柄、互相覆盖内容，
/// 先完成的那个 rename 成功后，另一个仍在往「已改名成 `.hprof` 的文件」里写 ⇒ 缓存直接写坏。
/// 所以用 `create_new`（原子创建）当锁：抢不到就等对方完成。
///
/// 返回 `Ok(None)` 表示「别人已完成，直接用 dst」。
fn acquire_part(part: &Path, dst: &Path) -> Result<Option<File>, String> {
    use std::time::{Duration, Instant};
    for _attempt in 0..2 {
        match fs::OpenOptions::new().write(true).create_new(true).open(part) {
            Ok(f) => return Ok(Some(f)),
            Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => {
                // 别人正在解压：等它 rename 出 dst
                let deadline = Instant::now() + Duration::from_secs(900);
                while Instant::now() < deadline {
                    if fs::metadata(dst).map(|m| m.len() > 0).unwrap_or(false) {
                        return Ok(None);
                    }
                    std::thread::sleep(Duration::from_millis(150));
                }
                // 等超时 ⇒ 几乎可以断定是上次崩溃留下的孤儿 .part，清掉后重抢一次
                let _ = fs::remove_file(part);
            }
            Err(e) => return Err(format!("创建缓存文件失败: {e}")),
        }
    }
    Err("解压缓存被占用且无法回收（请清理缓存目录后重试）".to_string())
}

/// 把 gzip 解压到 `dst`（先写 `.part`，成功后 rename）
fn decompress_to(src: &Path, dst: &Path) -> Result<u64, String> {
    if let Some(parent) = dst.parent() {
        fs::create_dir_all(parent).map_err(|e| format!("创建缓存目录失败: {e}"))?;
    }
    let part = dst.with_extension("hprof.part");
    let Some(mut out) = acquire_part(&part, dst)? else {
        // 并发对手已经解压好了
        return Ok(fs::metadata(dst).map(|m| m.len()).unwrap_or(0));
    };
    let src_file = File::open(src).map_err(|e| format!("无法打开压缩包: {e}"))?;
    let mut dec = GzDecoder::new(BufReader::with_capacity(1 << 20, src_file));

    let mut buf = vec![0u8; 4 << 20];
    let mut total: u64 = 0;
    loop {
        let n = match dec.read(&mut buf) {
            Ok(0) => break,
            Ok(n) => n,
            Err(e) => {
                drop(out);
                let _ = fs::remove_file(&part);
                return Err(format!("解压失败（数据损坏或不是 gzip）: {e}"));
            }
        };
        total += n as u64;
        if total > MAX_DECOMPRESSED_BYTES {
            drop(out);
            let _ = fs::remove_file(&part);
            return Err(format!(
                "解压后超过 {} GB 上限，已中止（疑似异常压缩包）",
                MAX_DECOMPRESSED_BYTES / (1024 * 1024 * 1024)
            ));
        }
        if let Err(e) = out.write_all(&buf[..n]) {
            drop(out);
            let _ = fs::remove_file(&part);
            return Err(format!("写入缓存失败（磁盘空间不足？）: {e}"));
        }
    }
    out.flush().map_err(|e| format!("写入缓存失败: {e}"))?;
    // sync_all 之后再 rename：保证断电时不会留下「名字正确、内容没落盘」的文件
    out.sync_all().map_err(|e| format!("写入缓存失败: {e}"))?;
    drop(out);

    match fs::rename(&part, dst) {
        Ok(()) => Ok(total),
        Err(e) => {
            let _ = fs::remove_file(&part);
            if dst.exists() {
                // 并发场景：另一个命令先完成了同一份解压，直接用它的结果
                Ok(fs::metadata(dst).map(|m| m.len()).unwrap_or(total))
            } else {
                Err(format!("缓存文件落盘失败: {e}"))
            }
        }
    }
}

// ==================== 对外入口 ====================

/// 把任意来源路径解析成「可直接 mmap 的 HPROF 路径」。
///
/// 非压缩文件零成本（只做一次 stat）；gzip 会解压到缓存并复用。
pub(crate) fn resolve(path: &str) -> Result<Resolved, String> {
    resolve_with(path, &cache_root())
}

/// `resolve` 的可注入版本（单测用它把缓存固定到临时目录，**不碰进程环境变量**
/// —— `set_var` 是进程级全局副作用，会把同进程里并行跑的其他用例一起改掉）
pub(crate) fn resolve_with(path: &str, root: &Path) -> Result<Resolved, String> {
    let src = Path::new(path);
    let meta = fs::metadata(src).map_err(|e| format!("无法读取文件: {e}"))?;
    let source_size = meta.len();

    if !is_gzip(path) {
        return Ok(Resolved {
            hprof_path: src.to_path_buf(),
            compressed: false,
            cache_hit: false,
            source_size,
            hprof_size: source_size,
        });
    }

    if source_size == 0 {
        return Err("压缩包为空，不是有效的堆转储".to_string());
    }
    let dst = cache_path_for(root, src, source_size, mtime_ms(&meta));
    if let Ok(m) = fs::metadata(&dst) {
        // 命中缓存：仍校验一次非空（写入用 rename，长度不完整的情况本不该出现，
        // 但磁盘满/被外部截断的历史残留不值得信任）
        if m.len() > 0 {
            return Ok(Resolved {
                hprof_path: dst,
                compressed: true,
                cache_hit: true,
                source_size,
                hprof_size: m.len(),
            });
        }
    }
    let hprof_size = decompress_to(src, &dst)?;
    Ok(Resolved {
        hprof_path: dst,
        compressed: true,
        cache_hit: false,
        source_size,
        hprof_size,
    })
}

/// 统一 mmap 入口：所有解析模块都走这里，压缩包支持因此是全链路的。
pub(crate) fn map_file(path: &str) -> Result<Mmap, String> {
    let r = resolve(path)?;
    let f = File::open(&r.hprof_path).map_err(|e| format!("无法打开文件: {e}"))?;
    unsafe { Mmap::map(&f).map_err(|e| format!("内存映射失败: {e}")) }
}

/// 读取逻辑文件前 `n` 字节。
///
/// gzip 时**不会**全量解压：解压器读到 `n` 字节即停 —— 打开对话框时识别格式只
/// 需要 1KB 头，不应该为此等几十秒。
pub(crate) fn read_head(path: &str, n: usize) -> Result<Vec<u8>, String> {
    let mut buf = vec![0u8; n];
    if is_gzip(path) {
        let f = File::open(path).map_err(|e| format!("无法打开文件: {e}"))?;
        let mut dec = GzDecoder::new(BufReader::new(f));
        let mut got = 0usize;
        while got < n {
            match dec.read(&mut buf[got..]) {
                Ok(0) => break,
                Ok(k) => got += k,
                Err(e) => return Err(format!("解压读取失败: {e}")),
            }
        }
        buf.truncate(got);
        return Ok(buf);
    }
    let mut f = File::open(path).map_err(|e| format!("无法打开文件: {e}"))?;
    let got = f.read(&mut buf).map_err(|e| format!("读取文件失败: {e}"))?;
    buf.truncate(got);
    Ok(buf)
}

// ==================== 命令 ====================

/// 解析来源信息（前端打开 .gz 后可展示「已解压」与体积对比）
pub fn mat_source_info(path: &str) -> Result<ResData<SourceInfo>, String> {
    let r = resolve(path)?;
    Ok(ResData {
        code: 200,
        msg: "ok".to_string(),
        data: r.info(),
    })
}

/// 解压缓存占用
pub fn mat_cache_info() -> Result<ResData<CacheInfo>, String> {
    let dir = cache_root();
    let mut files = 0usize;
    let mut total = 0u64;
    let mut partial = 0usize;
    if let Ok(rd) = fs::read_dir(&dir) {
        for e in rd.flatten() {
            let p = e.path();
            let Ok(m) = e.metadata() else { continue };
            if !m.is_file() {
                continue;
            }
            match p.extension().and_then(|s| s.to_str()) {
                Some("hprof") => {
                    files += 1;
                    total += m.len();
                }
                Some("part") => partial += 1,
                _ => {}
            }
        }
    }
    Ok(ResData {
        code: 200,
        msg: "ok".to_string(),
        data: CacheInfo {
            dir: dir.to_string_lossy().to_string(),
            files,
            total_bytes: total,
            partial_files: partial,
        },
    })
}

/// 清理解压缓存（含残留 `.part`）。
///
/// 正在被 mmap 的文件在 Windows 上删不掉 —— 不阻断整体清理，逐个记录跳过项，
/// 前端提示「重启应用后可清掉剩余 N 个」。
pub fn mat_cache_clear() -> Result<ResData<CacheClearResult>, String> {
    let dir = cache_root();
    let mut removed = 0usize;
    let mut removed_bytes = 0u64;
    let mut skipped: Vec<String> = Vec::new();
    if let Ok(rd) = fs::read_dir(&dir) {
        for e in rd.flatten() {
            let p = e.path();
            let Ok(m) = e.metadata() else { continue };
            if !m.is_file() {
                continue;
            }
            let ok_ext = matches!(p.extension().and_then(|s| s.to_str()), Some("hprof") | Some("part"));
            if !ok_ext {
                continue;
            }
            match fs::remove_file(&p) {
                Ok(()) => {
                    removed += 1;
                    removed_bytes += m.len();
                }
                Err(_) => skipped.push(
                    p.file_name().map(|s| s.to_string_lossy().to_string()).unwrap_or_default(),
                ),
            }
        }
    }
    Ok(ResData {
        code: 200,
        msg: "ok".to_string(),
        data: CacheClearResult {
            removed,
            removed_bytes,
            skipped,
        },
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn gzip_magic_detection() {
        let dir = std::env::temp_dir().join(format!("mat_src_{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let plain = dir.join("a_hprof.bin");
        std::fs::write(&plain, b"JAVA PROFILE 1.0.2\0").unwrap();
        assert!(!is_gzip(plain.to_str().unwrap()));
        let gz = dir.join("a_hprof.gz");
        std::fs::write(&gz, [0x1f, 0x8b, 0x08, 0x00]).unwrap();
        assert!(is_gzip(gz.to_str().unwrap()));
    }

    #[test]
    fn fnv_is_stable_and_path_sensitive() {
        let a = fnv1a64(b"a/b/c.hprof.gz");
        assert_eq!(a, fnv1a64(b"a/b/c.hprof.gz"));
        assert_ne!(a, fnv1a64(b"a/b/d.hprof.gz"));
    }

    #[test]
    fn resolve_gzip_decompresses_then_hits_cache() {
        let cache = std::env::temp_dir().join(format!("mat_src_cache_{}", std::process::id()));
        let dir = std::env::temp_dir().join(format!("mat_src_resolve_{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        // 造一个真实 gzip（内容不重要，只看解压/缓存语义）
        let gz = dir.join("dump.hprof.gz");
        {
            let f = File::create(&gz).unwrap();
            let mut enc = flate2::write::GzEncoder::new(f, flate2::Compression::fast());
            enc.write_all(b"JAVA PROFILE 1.0.2\0payload").unwrap();
            enc.finish().unwrap();
        }
        let r1 = resolve_with(gz.to_str().unwrap(), &cache).unwrap();
        assert!(r1.compressed && !r1.cache_hit);
        assert_eq!(r1.hprof_size, b"JAVA PROFILE 1.0.2\0payload".len() as u64);
        assert!(r1.hprof_path.starts_with(&cache), "缓存必须落在隔离目录内: {:?}", r1.hprof_path);
        // 第二次必须命中，且指向同一路径
        let r2 = resolve_with(gz.to_str().unwrap(), &cache).unwrap();
        assert!(r2.cache_hit);
        assert_eq!(r1.hprof_path, r2.hprof_path);
        assert_eq!(std::fs::read(&r2.hprof_path).unwrap(), b"JAVA PROFILE 1.0.2\0payload");
    }

    #[test]
    fn read_head_stops_early_on_gzip() {
        // 构造：1024 个 'A' 的 gzip。read_head(2) 必须只取 2 字节且不整包解压
        let dir = std::env::temp_dir().join(format!("mat_src_head_{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let gz = dir.join("head.gz");
        {
            let f = File::create(&gz).unwrap();
            let mut enc = flate2::write::GzEncoder::new(f, flate2::Compression::fast());
            enc.write_all(&vec![b'A'; 1024]).unwrap();
            enc.finish().unwrap();
        }
        let head = read_head(gz.to_str().unwrap(), 2).unwrap();
        assert_eq!(head, b"AA");
        let full = read_head(gz.to_str().unwrap(), 4096).unwrap();
        assert_eq!(full.len(), 1024);
    }
}
