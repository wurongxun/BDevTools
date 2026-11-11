//! 编码 / EOL / BOM 手动转换（M4 遗留 E4/E5，纯 Rust，无 Tauri 依赖）。
//!
//! 与 `TextView` 里「换一种编码重新解码」（只改读取口径、不动磁盘）不同，本模块做的是
//! **真正把磁盘字节按目标编码/EOL 重写**。这是不可逆操作，因此四道保护缺一不可：
//! 1. 先整份备份原字节到 `.bdt-convert-*.bak`（可由统一的撤销清单恢复）；
//! 2. 走 `file_meta::save_note` 这一条唯一写回通道 ⇒ 原子写 + 409 冲突 + 415 不可表示字符；
//! 3. UTF-16LE/BE 目标**强制**带对应 BOM，否则下次打开时 `decode_bytes` 只能靠 chardetng 猜，
//!    会直接变成乱码；非 UTF-8/16 编码禁止带 BOM；
//! 4. 目标与现状一致时**不写盘、不生成备份**，避免用户点一次确认就留一堆无用 `.bak`。
//!
//! 转换的语料是「磁盘上的当前字节」：前端必须先确认文档没有未保存更改，否则转换会
//! 把内存里的编辑覆盖掉（由 `EncodingConvertDialog.vue` 拦截 isDirty）。

use super::file_meta::{self, SaveMeta};
use crate::response::resp_data::ResData;
use encoding_rs::Encoding;
use serde::Serialize;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

/// 与 `file_large::MAX_EDITABLE_BYTES` 同口径：整读整写不接超大文件。
pub const MAX_CONVERT_BYTES: u64 = 32 * 1024 * 1024;

#[derive(Debug, Clone, Serialize, Default)]
pub struct ConvertOutcome {
    pub path: String,
    /// 空串表示「目标与现状一致，未写盘」，此时不要往撤销清单里加条目
    pub backup_path: String,
    pub encoding: String,
    pub eol: String,
    pub bom: String,
    /// 写入后的新 mtime（毫秒字符串），供前端刷新 409 基线
    pub mtime_ms: String,
    pub size: u64,
    /// 转换前的元信息，用于在 UI 上回报「从什么改成什么」
    pub from_encoding: String,
    pub from_eol: String,
    pub from_bom: String,
}

#[derive(Debug, Clone)]
pub struct ConvertRequest {
    pub path: String,
    pub target_encoding: String,
    pub target_eol: String,
    pub target_bom: String,
    pub expected_mtime_ms: Option<i64>,
    pub expected_size: Option<u64>,
}

fn fail(code: u32, msg: impl Into<String>) -> ResData<ConvertOutcome> {
    ResData {
        code,
        msg: msg.into(),
        data: ConvertOutcome::default(),
    }
}

fn now_ns() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0)
}

fn backup_path(path: &Path) -> PathBuf {
    let parent = path.parent().unwrap_or_else(|| Path::new("."));
    let name = path
        .file_name()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_else(|| "file".to_string());
    parent.join(format!(
        ".{name}.bdt-convert-{}-{}.bak",
        std::process::id(),
        now_ns()
    ))
}

/// 纯函数：校验并归一化目标三元组，返回可直接喂给 `save_note` 的 (encoding, eol, bom)。
/// encoding 用 WHATWG label 反查真名（如 `gb18030` → `GB18030`），避免前端传别名后
/// `encoding_from_name` 静默回落成 UTF-8 这种最危险的失败方式。
pub fn normalize_target(
    encoding: &str,
    eol: &str,
    bom: &str,
) -> Result<(String, String, String), String> {
    let enc = Encoding::for_label(encoding.trim().as_bytes())
        .ok_or_else(|| format!("不支持的目标编码：{encoding}"))?;
    let name = enc.name().to_string();
    let eol_n = match eol.trim().to_ascii_uppercase().as_str() {
        "LF" => "LF",
        "CRLF" => "CRLF",
        "CR" => "CR",
        "" => "LF",
        _ => return Err(format!("不支持的目标换行符：{eol}（仅 LF / CRLF / CR）")),
    }
    .to_string();
    let bom_raw = bom.trim().to_ascii_lowercase();
    // UTF-16 没有 BOM 就无法被 decode_bytes 可靠识别，强制补齐；其余编码不允许带 BOM。
    let bom_n = match name.as_str() {
        "UTF-16LE" => "utf16le",
        "UTF-16BE" => "utf16be",
        "UTF-8" => match bom_raw.as_str() {
            "utf8" | "utf-8" => "utf8",
            "none" | "" => "none",
            _ => {
                return Err(format!(
                    "BOM 与目标编码不匹配：{name} 只支持 none / utf8，收到 {bom}"
                ))
            }
        },
        _ => match bom_raw.as_str() {
            "none" | "" => "none",
            _ => return Err(format!("BOM 与目标编码不匹配：{name} 不能带 BOM（{bom}）")),
        },
    }
    .to_string();
    Ok((name, eol_n, bom_n))
}

pub async fn convert_encoding_eol(
    req: ConvertRequest,
) -> Result<ResData<ConvertOutcome>, String> {
    let (enc, eol, bom_n) = match normalize_target(
        &req.target_encoding,
        &req.target_eol,
        &req.target_bom,
    ) {
        Ok(v) => v,
        Err(e) => return Ok(fail(400, e)),
    };
    let path = Path::new(&req.path);
    let stat = match tokio::fs::metadata(path).await {
        Ok(m) if m.is_file() => m,
        Ok(_) => return Ok(fail(500, "目标不是文件")),
        Err(e) => return Ok(fail(500, format!("读取文件信息失败: {e}"))),
    };
    if stat.len() > MAX_CONVERT_BYTES {
        return Ok(fail(413, "文件过大，不做整份转码；请改用分块编辑"));
    }
    let bytes = match tokio::fs::read(path).await {
        Ok(b) => b,
        Err(e) => return Ok(fail(500, format!("读取原文件失败: {e}"))),
    };
    // 按原编码解码并剥掉原 BOM；text 里的 EOL 保持原样，save_note 的 normalize_eol
    // 会先统一成 \n 再按目标还原，因此 CRLF → LF / CR 都是干净的一次性转换。
    let (src_enc, src_bom, text) = file_meta::decode_bytes(&bytes);
    let src_eol = file_meta::detect_eol(&text);
    if src_enc == enc && src_eol == eol && src_bom == bom_n {
        return Ok(ResData {
            code: 200,
            msg: "目标编码 / 换行符 / BOM 与当前文件一致，无需写盘".to_string(),
            data: ConvertOutcome {
                path: req.path,
                from_encoding: src_enc,
                from_eol: src_eol,
                from_bom: src_bom,
                ..Default::default()
            },
        });
    }

    let bak = backup_path(path);
    if let Err(e) = tokio::fs::copy(path, &bak).await {
        return Ok(fail(500, format!("备份失败，未改动原文件: {e}")));
    }
    let res = file_meta::save_note(
        req.path.clone(),
        text,
        SaveMeta {
            encoding: Some(enc.clone()),
            eol: Some(eol.clone()),
            bom: Some(bom_n.clone()),
            expected_mtime_ms: req.expected_mtime_ms,
            expected_size: req.expected_size,
        },
    )
    .await?;
    if res.code != 200 {
        // 409 / 415 都意味着原文件没被改动，备份必须一起清掉，不留孤儿
        let _ = tokio::fs::remove_file(&bak).await;
        return Ok(fail(res.code, res.msg));
    }
    let size = tokio::fs::metadata(path)
        .await
        .map(|m| m.len())
        .unwrap_or(0);
    Ok(ResData {
        code: 200,
        msg: "转换成功".to_string(),
        data: ConvertOutcome {
            path: req.path,
            backup_path: bak.to_string_lossy().to_string(),
            encoding: enc,
            eol,
            bom: bom_n,
            mtime_ms: res.data,
            size,
            from_encoding: src_enc,
            from_eol: src_eol,
            from_bom: src_bom,
        },
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tmp_dir(tag: &str) -> PathBuf {
        let d = std::env::temp_dir().join(format!(
            "bdevtools_convert_{}_{}_{}",
            tag,
            std::process::id(),
            now_ns()
        ));
        std::fs::create_dir_all(&d).unwrap();
        d
    }

    fn req(path: &Path, enc: &str, eol: &str, bom: &str) -> ConvertRequest {
        ConvertRequest {
            path: path.to_string_lossy().to_string(),
            target_encoding: enc.to_string(),
            target_eol: eol.to_string(),
            target_bom: bom.to_string(),
            expected_mtime_ms: None,
            expected_size: None,
        }
    }

    #[test]
    fn normalize_target_validates() {
        // encoding_rs 的 name() 对 GB18030 返回小写 "gb18030"，不是 UI 里的大写标签
        assert_eq!(
            normalize_target("gb18030", "crlf", "none").unwrap(),
            ("gb18030".into(), "CRLF".into(), "none".into())
        );
        assert_eq!(
            normalize_target("utf-16LE", "LF", "none").unwrap(),
            ("UTF-16LE".into(), "LF".into(), "utf16le".into())
        );
        assert_eq!(
            normalize_target("UTF-8", "lf", "utf8").unwrap(),
            ("UTF-8".into(), "LF".into(), "utf8".into())
        );
        assert!(normalize_target("no-such-enc", "LF", "none").is_err());
        assert!(normalize_target("UTF-8", "WINDOWS", "none").is_err());
        assert!(normalize_target("GBK", "LF", "utf8").is_err());
        assert!(normalize_target("UTF-8", "LF", "utf16le").is_err());
    }

    #[tokio::test]
    async fn gbk_crlf_to_utf8_lf_keeps_text_and_backup() {
        let dir = tmp_dir("gbk");
        let path = dir.join("zh.txt");
        let (src, _, had_err) = encoding_rs::GBK.encode("中文\r\n第二行");
        assert!(!had_err);
        std::fs::write(&path, src.as_ref()).unwrap();
        let original = std::fs::read(&path).unwrap();

        let res = convert_encoding_eol(req(&path, "UTF-8", "LF", "none"))
            .await
            .unwrap();
        assert_eq!(res.code, 200, "{}", res.msg);
        assert_eq!(res.data.from_encoding, "GBK");
        assert_eq!(res.data.from_eol, "CRLF");
        assert_eq!(res.data.encoding, "UTF-8");
        assert_eq!(res.data.eol, "LF");

        let out = std::fs::read(&path).unwrap();
        assert_eq!(String::from_utf8(out.clone()).unwrap(), "中文\n第二行");
        assert!(!out.windows(2).any(|w| w == b"\r\n"));
        let bak = std::fs::read(&res.data.backup_path).unwrap();
        assert_eq!(bak, original, "备份必须是转换前的原始字节");
    }

    #[tokio::test]
    async fn noop_does_not_write_or_leave_backup() {
        let dir = tmp_dir("noop");
        let path = dir.join("a.txt");
        std::fs::write(&path, b"plain\ntext").unwrap();
        let res = convert_encoding_eol(req(&path, "UTF-8", "LF", "none"))
            .await
            .unwrap();
        assert_eq!(res.code, 200);
        assert!(res.data.backup_path.is_empty());
        assert_eq!(std::fs::read_dir(&dir).unwrap().count(), 1);
    }

    #[tokio::test]
    async fn unmappable_char_blocks_and_cleans_backup() {
        let dir = tmp_dir("unmap");
        let path = dir.join("a.txt");
        std::fs::write(&path, "中文".as_bytes()).unwrap();
        let original = std::fs::read(&path).unwrap();
        let res = convert_encoding_eol(req(&path, "ASCII", "LF", "none"))
            .await
            .unwrap();
        assert_eq!(res.code, file_meta::CODE_UNMAPPABLE, "{}", res.msg);
        assert_eq!(std::fs::read(&path).unwrap(), original);
        assert_eq!(std::fs::read_dir(&dir).unwrap().count(), 1, "失败不能留孤儿 .bak");
    }

    #[tokio::test]
    async fn conflict_blocks_and_cleans_backup() {
        let dir = tmp_dir("conflict");
        let path = dir.join("a.txt");
        std::fs::write(&path, "a\nb".as_bytes()).unwrap();
        let original = std::fs::read(&path).unwrap();
        let mut r = req(&path, "UTF-8", "CRLF", "none");
        r.expected_mtime_ms = Some(1); // 必然与真实 mtime 不同
        let res = convert_encoding_eol(r).await.unwrap();
        assert_eq!(res.code, file_meta::CODE_CONFLICT);
        assert_eq!(std::fs::read(&path).unwrap(), original);
        assert_eq!(std::fs::read_dir(&dir).unwrap().count(), 1);
    }

    #[tokio::test]
    async fn utf16_target_forces_bom_and_stays_decodable() {
        let dir = tmp_dir("utf16");
        let path = dir.join("a.txt");
        std::fs::write(&path, "中文\n第二行".as_bytes()).unwrap();
        let res = convert_encoding_eol(req(&path, "UTF-16LE", "CRLF", "none"))
            .await
            .unwrap();
        assert_eq!(res.code, 200, "{}", res.msg);
        assert_eq!(res.data.bom, "utf16le");
        let out = std::fs::read(&path).unwrap();
        assert!(out.starts_with(&[0xFF, 0xFE]));
        let (enc, bom, text) = file_meta::decode_bytes(&out);
        assert_eq!(enc, "UTF-16LE");
        assert_eq!(bom, "utf16le");
        assert_eq!(text, "中文\r\n第二行");
        assert_eq!(file_meta::detect_eol(&text), "CRLF");
        // 正文必须是 UTF-16LE code unit（"中" = U+4E2D → 2D 4E），不是 UTF-8 的 E4 B8 AD
        assert_eq!(&out[2..6], &[0x2D, 0x4E, 0x87, 0x65]);
    }
}
