//! 文件元数据与可靠写回（files 插件业务层，纯 Rust，无 Tauri 依赖）
//!
//! 修复四类会导致数据损坏的问题：
//! 1. `std::fs::write` 直接整文件覆盖 —— 写一半失败会留下半截文件；
//! 2. 读时自动探测编码、写时固定 UTF-8 —— GBK / UTF-16 文件保存后乱码；
//! 3. 不识别 EOL —— CRLF 文件保存后被静默改成 LF；
//! 4. 没有编辑期冲突检测 —— 外部进程/AI 任务并发写会互相覆盖。
//!
//! 口径（M0 定案）：**默认按原值回写**，不做自动转码 / 自动换行转换；
//! 编码无法表示新内容时直接拒绝保存，绝不静默降级成 UTF-8。

use crate::response::resp_data::ResData;
use chardetng::EncodingDetector;
use encoding_rs::Encoding;
use serde::Serialize;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};
use tokio::io::{AsyncReadExt, AsyncWriteExt};

/// 文件已被外部修改，保存被阻止（前端据此提示刷新 / 另存副本，不静默覆盖）
pub const CODE_CONFLICT: u32 = 409;
/// 当前内容含目标编码无法表示的字符，保存被阻止（避免乱码或字符被替换成实体）
pub const CODE_UNMAPPABLE: u32 = 415;

/// 探测样本上限：只取文件头，避免为拿元数据把 50MB 日志整份读进内存。
const PROBE_BYTES: usize = 64 * 1024;

#[derive(Debug, Clone, Serialize)]
pub struct FileMeta {
    /// WHATWG 编码名（encoding_rs / chardetng 口径），如 UTF-8 / GBK / UTF-16LE
    pub encoding: String,
    /// LF / CRLF / CR
    pub eol: String,
    /// none / utf8 / utf16le / utf16be
    pub bom: String,
    /// 文件修改时间（毫秒），保存时回传用于冲突检测
    pub mtime_ms: i64,
    pub size: u64,
}

/// 保存选项。全部为 Option：旧调用点（草稿、脚本、数据库插件）不传时
/// 行为与改造前一致（UTF-8 + LF + 不检测冲突），保证向后兼容。
#[derive(Debug, Default)]
pub struct SaveMeta {
    pub encoding: Option<String>,
    pub eol: Option<String>,
    pub bom: Option<String>,
    pub expected_mtime_ms: Option<i64>,
    pub expected_size: Option<u64>,
}

fn modified_ms(t: SystemTime) -> i64 {
    t.duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0)
}

fn monotonic_stamp() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0)
}

fn encoding_from_name(name: &str) -> &'static Encoding {
    // for_label 覆盖全部 WHATWG 标签（GBK / windows-1252 / UTF-16LE ...），
    // 比 utils::line_reader::resolve_encoding 的手写白名单更全。
    Encoding::for_label(name.as_bytes()).unwrap_or(encoding_rs::UTF_8)
}

/// 把解码后的文本压成目标编码的字节。
///
/// **关键坑**：`encoding_rs` 的 `UTF_16LE` / `UTF_16BE` 是 decode-only 编码，
/// 官方文档明确写「The output encoding of this encoding is UTF-8」——
/// 直接调 `enc.encode()` 得到的是 UTF-8 字节。若照抄通用路径，保存 UTF-16 文件
/// 会变成「UTF-16 BOM + UTF-8 正文」，读取侧又因 BOM 仍按 UTF-16 解码 ⇒ 全篇乱码。
/// 因此 UTF-16 必须自己按 UTF-16 code unit 写字节（Rust String 无孤立代理项，可直接编）。
pub fn encode_text(text: &str, enc: &'static Encoding) -> (Vec<u8>, bool) {
    match enc.name() {
        "UTF-16LE" => (
            text.encode_utf16().flat_map(|u| u.to_le_bytes()).collect(),
            false,
        ),
        "UTF-16BE" => (
            text.encode_utf16().flat_map(|u| u.to_be_bytes()).collect(),
            false,
        ),
        _ => {
            let (cow, _, had_errors) = enc.encode(text);
            (cow.into_owned(), had_errors)
        }
    }
}

/// BOM 优先、chardetng 兜底地解码，返回 (编码名, BOM 种类, 去掉 BOM 的正文)。
/// BOM 必须在这里剥掉：否则 UTF-8 BOM 会以 U+FEFF 出现在编辑器首行，
/// 保存时又被写回去，反复叠加。
pub fn decode_bytes(bytes: &[u8]) -> (String, String, String) {
    let (encoding, bom, offset) = if bytes.starts_with(&[0xEF, 0xBB, 0xBF]) {
        (encoding_rs::UTF_8, "utf8", 3)
    } else if bytes.starts_with(&[0xFF, 0xFE]) {
        (encoding_rs::UTF_16LE, "utf16le", 2)
    } else if bytes.starts_with(&[0xFE, 0xFF]) {
        (encoding_rs::UTF_16BE, "utf16be", 2)
    } else {
        let mut detector = EncodingDetector::new(chardetng::Iso2022JpDetection::Allow);
        detector.feed(bytes, bytes.is_empty());
        (
            detector.guess(None, chardetng::Utf8Detection::Allow),
            "none",
            0,
        )
    };
    let (text, _, _) = encoding.decode(&bytes[offset..]);
    (
        encoding.name().to_string(),
        bom.to_string(),
        text.into_owned(),
    )
}

/// 在**解码后的文本**上统计换行符（不能在原始字节上做：UTF-16 的 `\n` 是 `0A 00`）。
pub fn detect_eol(text: &str) -> String {
    let chars: Vec<char> = text.chars().collect();
    let (mut crlf, mut lf, mut cr) = (0u64, 0u64, 0u64);
    let mut i = 0;
    while i < chars.len() {
        match chars[i] {
            '\r' if i + 1 < chars.len() && chars[i + 1] == '\n' => {
                crlf += 1;
                i += 2;
                continue;
            }
            '\r' => cr += 1,
            '\n' => lf += 1,
            _ => {}
        }
        i += 1;
    }
    if crlf > 0 && crlf >= lf && crlf >= cr {
        "CRLF".to_string()
    } else if cr > lf && cr > 0 {
        "CR".to_string()
    } else {
        "LF".to_string()
    }
}

/// 编辑器交给后端的字符串统一是 `\n`；按文件原 EOL 还原后再编码写回。
pub fn normalize_eol(text: &str, eol: &str) -> String {
    let unified = text.replace("\r\n", "\n").replace('\r', "\n");
    match eol {
        "CRLF" => unified.replace('\n', "\r\n"),
        "CR" => unified.replace('\n', "\r"),
        _ => unified,
    }
}

/// 读取文件元数据（编码 / EOL / BOM / mtime / size）。只读文件头 64KB。
pub async fn read_file_meta(path: &str) -> Result<FileMeta, String> {
    let stat = tokio::fs::metadata(path)
        .await
        .map_err(|e| format!("读取文件信息失败: {e}"))?;
    if !stat.is_file() {
        return Err(format!("不是文件: {path}"));
    }
    let mtime_ms = modified_ms(
        stat.modified()
            .map_err(|e| format!("读取修改时间失败: {e}"))?,
    );
    let mut file = tokio::fs::File::open(path)
        .await
        .map_err(|e| format!("打开文件失败: {e}"))?;
    let mut buf = vec![0u8; PROBE_BYTES];
    let n = file
        .read(&mut buf)
        .await
        .map_err(|e| format!("读取文件头失败: {e}"))?;
    buf.truncate(n);
    let (encoding, bom, text) = decode_bytes(&buf);
    Ok(FileMeta {
        encoding,
        eol: detect_eol(&text),
        bom,
        mtime_ms,
        size: stat.len(),
    })
}

/// 原子保存：临时文件 + sync + rename；Windows 上 rename 无法覆盖时
/// 先备份原文件、删除再 rename，任何失败都能从备份恢复，绝不留下半截文件。
pub(crate) async fn write_atomic(target: &Path, bytes: &[u8]) -> Result<(), String> {
    let parent = target.parent().unwrap_or_else(|| Path::new("."));
    let name = target
        .file_name()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_else(|| "file".to_string());
    let stamp = monotonic_stamp();
    let pid = std::process::id();
    let tmp: PathBuf = parent.join(format!(".{name}.bdt-{pid}-{stamp}.tmp"));
    let bak: PathBuf = parent.join(format!(".{name}.bdt-{pid}-{stamp}.bak"));

    let mut f = tokio::fs::File::create(&tmp)
        .await
        .map_err(|e| format!("创建临时文件失败: {e}"))?;
    f.write_all(bytes)
        .await
        .map_err(|e| format!("写入临时文件失败: {e}"))?;
    f.sync_all()
        .await
        .map_err(|e| format!("刷盘失败: {e}"))?;
    drop(f);

    match tokio::fs::rename(&tmp, target).await {
        Ok(_) => Ok(()),
        Err(first_err) => {
            // Windows：目标已存在时 rename 可能不覆盖。先备份，删旧，再 rename；
            // 中途任何一步失败都用备份把原文件放回去。
            if tokio::fs::copy(target, &bak).await.is_err() {
                let _ = tokio::fs::remove_file(&tmp).await;
                return Err(format!("保存失败（原文件未改动）: {first_err}"));
            }
            if let Err(e) = tokio::fs::remove_file(target).await {
                let _ = tokio::fs::remove_file(&tmp).await;
                let _ = tokio::fs::remove_file(&bak).await;
                return Err(format!("保存失败（原文件未改动）: {e}"));
            }
            match tokio::fs::rename(&tmp, target).await {
                Ok(_) => {
                    let _ = tokio::fs::remove_file(&bak).await;
                    Ok(())
                }
                Err(e) => {
                    let _ = tokio::fs::copy(&bak, target).await;
                    let _ = tokio::fs::remove_file(&bak).await;
                    let _ = tokio::fs::remove_file(&tmp).await;
                    Err(format!("保存失败，原文件已从备份恢复: {e}"))
                }
            }
        }
    }
}

/// 保存文件。`ResData<String>.data` 返回写入后的新 mtime，前端据此刷新冲突基线，
/// 避免「连续保存两次」被自己的第一次写入误判成外部修改。
pub async fn save_note(
    path: String,
    text: String,
    meta: SaveMeta,
) -> Result<ResData<String>, String> {
    let stat = match tokio::fs::metadata(&path).await {
        Ok(m) => m,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            return Ok(ResData {
                code: 500,
                msg: "文件不存在".to_string(),
                data: String::new(),
            })
        }
        Err(e) => {
            return Ok(ResData {
                code: 500,
                msg: format!("检查文件时出错: {e}"),
                data: String::new(),
            })
        }
    };
    if !stat.is_file() {
        return Ok(ResData {
            code: 500,
            msg: "目标不是文件".to_string(),
            data: String::new(),
        });
    }

    let disk_ms = modified_ms(
        stat
            .modified()
            .map_err(|e| format!("读取修改时间失败: {e}"))?,
    );
    if let Some(expected) = meta.expected_mtime_ms {
        let size_changed = meta.expected_size.map_or(false, |s| s != stat.len());
        if expected != disk_ms || size_changed {
            return Ok(ResData {
                code: CODE_CONFLICT,
                msg: "文件已被外部修改，已阻止保存。请刷新文件后重试，或另存为副本。".to_string(),
                data: String::new(),
            });
        }
    }

    let normalized = normalize_eol(&text, meta.eol.as_deref().unwrap_or("LF"));
    let enc_name = meta.encoding.as_deref().unwrap_or("UTF-8");
    let enc = encoding_from_name(enc_name);
    let (encoded, had_errors) = encode_text(&normalized, enc);
    if had_errors {
        return Ok(ResData {
            code: CODE_UNMAPPABLE,
            msg: format!(
                "当前内容包含 {} 无法表示的字符，已阻止保存以免乱码。请改用 UTF-8 保存。",
                enc.name()
            ),
            data: String::new(),
        });
    }
    let mut out = Vec::with_capacity(encoded.len() + 3);
    match meta.bom.as_deref() {
        Some("utf8") => out.extend_from_slice(&[0xEF, 0xBB, 0xBF]),
        Some("utf16le") => out.extend_from_slice(&[0xFF, 0xFE]),
        Some("utf16be") => out.extend_from_slice(&[0xFE, 0xFF]),
        _ => {}
    }
    out.extend_from_slice(&encoded);

    if let Err(e) = write_atomic(Path::new(&path), &out).await {
        // 写失败也要走 ResData 语义：原文件仍在，前端保持 dirty，让用户能重试
        return Ok(ResData {
            code: 500,
            msg: e,
            data: String::new(),
        });
    }

    let new_ms = modified_ms(
        tokio::fs::metadata(&path)
            .await
            .map_err(|e| format!("读取写入后信息失败: {e}"))?
            .modified()
            .map_err(|e| format!("读取写入后时间失败: {e}"))?,
    );
    Ok(ResData {
        code: 200,
        msg: "操作成功".to_string(),
        data: new_ms.to_string(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tmp_dir(tag: &str) -> PathBuf {
        let d = std::env::temp_dir().join(format!(
            "bdevtools_file_meta_{}_{}_{}",
            tag,
            std::process::id(),
            monotonic_stamp()
        ));
        std::fs::create_dir_all(&d).unwrap();
        d
    }

    #[test]
    fn decode_strips_utf8_bom() {
        let (_enc, bom, text) = decode_bytes(&[0xEF, 0xBB, 0xBF, b'a', b'b']);
        assert_eq!(bom, "utf8");
        assert_eq!(text, "ab");
    }

    #[test]
    fn detect_eol_variants() {
        assert_eq!(detect_eol("a\nb\nc"), "LF");
        assert_eq!(detect_eol("a\r\nb\r\nc"), "CRLF");
        assert_eq!(detect_eol("a\rb\rc"), "CR");
    }

    #[test]
    fn normalize_eol_roundtrip() {
        assert_eq!(normalize_eol("a\nb", "CRLF"), "a\r\nb");
        assert_eq!(normalize_eol("a\r\nb\rc", "LF"), "a\nb\nc");
    }

    #[tokio::test]
    async fn save_keeps_gbk_bytes_and_crlf() {
        let dir = tmp_dir("gbk");
        let path = dir.join("zh.txt");
        let (src, _, had_err) = encoding_rs::GBK.encode("中文\r\n第二行");
        assert!(!had_err);
        std::fs::write(&path, src.as_ref()).unwrap();

        let meta = read_file_meta(path.to_str().unwrap()).await.unwrap();
        assert_eq!(meta.encoding, "GBK");
        assert_eq!(meta.eol, "CRLF");

        let res = save_note(
            path.to_string_lossy().to_string(),
            "中文\r\n第三行".to_string().replace("\r\n", "\n"),
            SaveMeta {
                encoding: Some(meta.encoding.clone()),
                eol: Some(meta.eol.clone()),
                bom: Some(meta.bom.clone()),
                ..Default::default()
            },
        )
        .await
        .unwrap();
        assert_eq!(res.code, 200);

        let bytes = std::fs::read(&path).unwrap();
        // 仍是 GBK（不是 UTF-8），且换行仍是 CRLF
        assert!(bytes.windows(2).any(|w| w == b"\r\n"));
        let (text, _, _) = encoding_rs::GBK.decode(&bytes);
        assert_eq!(text, "中文\r\n第三行");
    }

    #[tokio::test]
    async fn conflict_blocks_write_and_keeps_original() {
        let dir = tmp_dir("conflict");
        let path = dir.join("a.txt");
        std::fs::write(&path, b"original").unwrap();

        let res = save_note(
            path.to_string_lossy().to_string(),
            "changed".to_string(),
            SaveMeta {
                expected_mtime_ms: Some(1), // 必然不等于真实 mtime
                ..Default::default()
            },
        )
        .await
        .unwrap();
        assert_eq!(res.code, CODE_CONFLICT);
        assert_eq!(std::fs::read(&path).unwrap(), b"original");
    }

    /// 回归：encoding_rs 的 UTF_16LE/BE 是 decode-only，`encode()` 会静默输出 UTF-8。
    /// 若保存链路没走 `encode_text` 的手动 UTF-16 分支，写盘后就成了
    ///「UTF-16 BOM + UTF-8 正文」，读取侧仍按 BOM 当 UTF-16 解码 ⇒ 全篇乱码。
    #[tokio::test]
    async fn utf16le_bom_roundtrip_keeps_bytes() {
        let dir = tmp_dir("utf16");
        let path = dir.join("u.txt");
        let original = "中文\r\n第二行";
        let mut bytes = vec![0xFF, 0xFE];
        for u in original.encode_utf16() {
            bytes.extend_from_slice(&u.to_le_bytes());
        }
        std::fs::write(&path, &bytes).unwrap();

        let meta = read_file_meta(path.to_str().unwrap()).await.unwrap();
        assert_eq!(meta.encoding, "UTF-16LE");
        assert_eq!(meta.bom, "utf16le");
        assert_eq!(meta.eol, "CRLF");

        let (enc_name, bom, text) = decode_bytes(&bytes);
        assert_eq!((enc_name.as_str(), bom.as_str(), text.as_str()), ("UTF-16LE", "utf16le", original));

        let res = save_note(
            path.to_string_lossy().to_string(),
            text,
            SaveMeta {
                encoding: Some(meta.encoding.clone()),
                eol: Some(meta.eol.clone()),
                bom: Some(meta.bom.clone()),
                ..Default::default()
            },
        )
        .await
        .unwrap();
        assert_eq!(res.code, 200, "{}", res.msg);
        assert_eq!(std::fs::read(&path).unwrap(), bytes, "逐字节必须完全一致");
    }

    #[tokio::test]
    async fn unmappable_char_blocks_write() {
        let dir = tmp_dir("unmappable");
        let path = dir.join("a.txt");
        std::fs::write(&path, b"ascii").unwrap();
        let res = save_note(
            path.to_string_lossy().to_string(),
            "emoji 😀".to_string(),
            SaveMeta {
                encoding: Some("GBK".to_string()),
                ..Default::default()
            },
        )
        .await
        .unwrap();
        assert_eq!(res.code, CODE_UNMAPPABLE);
        assert_eq!(std::fs::read(&path).unwrap(), b"ascii");
    }
}
