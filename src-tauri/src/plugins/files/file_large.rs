//! 大文件（>1MB）行级编辑与分块写回（M4 / E7，纯 Rust，无 Tauri 依赖）
//!
//! 背景：M1–M3 里 `doc.content` 为 null 的大文件只能走 `TextView` 分块只读查看（D-8）。
//! 本模块把「按行区间打补丁」下沉到宿主：**前端只提交脏行区间，不整读、不拼整份文本**，
//! 宿主读一次原文件、应用补丁后仍然走 `file_meta::save_note` 这一条唯一写回通道，
//! 因此编码 / EOL / BOM / 原子写 / 409 冲突 / 415 不可表示字符的保护全部自动继承。
//!
//! 口径（M4 定案）：
//! - 编辑粒度是**行区间**（1-based 闭区间），`text` 里允许 `\n` 展开成多行；
//! - `delete=true` 表示删除整段行；空 `text` 且非 delete 表示把该行清空；
//! - 多个区间必须**升序且互不重叠**，否则直接拒绝 —— 静默错位比不写更危险；
//! - 超过 `MAX_EDITABLE_BYTES` 仍保持只读，返回 413（不整读、不整写）；
//! - 读原文件与写回之间若被外部进程改动，`save_note` 的 mtime/size 基线会拦住（409）。

use super::file_meta;
use crate::response::resp_data::ResData;
use serde::{Deserialize, Serialize};
use std::path::Path;

/// 32MB：整读整写的上限。更大文件只提供分块只读，避免一次保存把内存打满。
pub const MAX_EDITABLE_BYTES: u64 = 32 * 1024 * 1024;

/// 前端提交的单个行区间补丁。字段名保持 snake_case，与后端其它 ResData 契约一致。
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct LineEdit {
    /// 起始行（1-based，含）
    pub start_line: usize,
    /// 结束行（1-based，含）；必须 >= start_line
    pub end_line: usize,
    /// 替换文本，编辑器侧统一用 `\n`
    pub text: String,
    /// true 时删除 [start_line, end_line] 整段行，忽略 text
    #[serde(default)]
    pub delete: bool,
}

#[derive(Debug, Clone, Serialize, Default)]
pub struct LineEditOutcome {
    /// 写入后的新 mtime（毫秒字符串），前端据此刷新冲突基线
    pub mtime_ms: String,
    pub total_lines: usize,
    pub encoding: String,
    pub eol: String,
    pub bom: String,
}

fn fail(msg: impl Into<String>, code: u32) -> ResData<LineEditOutcome> {
    ResData {
        code,
        msg: msg.into(),
        data: LineEditOutcome::default(),
    }
}

/// 纯函数：把补丁应用到「已解码、已剥 BOM」的文本上。
/// 文本内部统一按 `\n` 处理；磁盘换行由 `save_note` 的 normalize_eol 还原。
pub fn apply_line_edits(text: &str, edits: &[LineEdit]) -> Result<String, String> {
    if edits.is_empty() {
        return Ok(text.to_string());
    }
    let unified = text.replace("\r\n", "\n").replace('\r', "\n");
    let mut lines: Vec<String> = unified.split('\n').map(|s| s.to_string()).collect();
    let total = lines.len();

    // 先全量校验再动手：任何一个非法区间都不能留下「写了一半」的中间状态。
    let mut prev_end = 0usize;
    for (i, e) in edits.iter().enumerate() {
        if e.start_line == 0 || e.end_line < e.start_line {
            return Err(format!(
                "第 {} 个编辑区间非法：start_line={} end_line={}",
                i + 1,
                e.start_line,
                e.end_line
            ));
        }
        if e.end_line > total {
            return Err(format!(
                "第 {} 个编辑区间越界：文件共 {} 行，end_line={}",
                i + 1,
                total,
                e.end_line
            ));
        }
        if e.start_line <= prev_end {
            return Err(format!(
                "第 {} 个编辑区间与前一个重叠或未按行号升序排列",
                i + 1
            ));
        }
        prev_end = e.end_line;
    }

    // 从后往前替换，前面的下标才不会因为行数变化而平移。
    for e in edits.iter().rev() {
        let start = e.start_line - 1;
        let end = e.end_line; // exclusive
        let replacement: Vec<String> = if e.delete {
            Vec::new()
        } else {
            e.text
                .replace("\r\n", "\n")
                .replace('\r', "\n")
                .split('\n')
                .map(|s| s.to_string())
                .collect()
        };
        lines.splice(start..end, replacement);
    }
    Ok(lines.join("\n"))
}

/// 读一次原文件 → 应用行补丁 → 交给唯一写回通道 `file_meta::save_note`。
pub async fn apply_line_edits_to_file(
    path: String,
    edits: Vec<LineEdit>,
    meta: file_meta::SaveMeta,
) -> Result<ResData<LineEditOutcome>, String> {
    if edits.is_empty() {
        return Ok(fail("没有需要保存的行改动", 400));
    }
    let stat = tokio::fs::metadata(&path)
        .await
        .map_err(|e| format!("读取文件信息失败: {e}"))?;
    if !stat.is_file() {
        return Ok(fail("目标不是文件", 500));
    }
    if stat.len() > MAX_EDITABLE_BYTES {
        return Ok(fail(
            format!(
                "文件超过 {} MB，暂不支持行编辑（仍可分块只读查看）",
                MAX_EDITABLE_BYTES / 1024 / 1024
            ),
            413,
        ));
    }

    let bytes = tokio::fs::read(Path::new(&path))
        .await
        .map_err(|e| format!("读取原文件失败: {e}"))?;
    let (detected_encoding, detected_bom, text) = file_meta::decode_bytes(&bytes);
    let detected_eol = file_meta::detect_eol(&text);
    let new_text = match apply_line_edits(&text, &edits) {
        Ok(t) => t,
        Err(e) => return Ok(fail(e, 400)),
    };

    let encoding = meta.encoding.clone().unwrap_or(detected_encoding);
    let eol = meta.eol.clone().unwrap_or(detected_eol);
    let bom = meta.bom.clone().unwrap_or(detected_bom);
    let res = file_meta::save_note(
        path,
        new_text.clone(),
        file_meta::SaveMeta {
            encoding: Some(encoding.clone()),
            eol: Some(eol.clone()),
            bom: Some(bom.clone()),
            expected_mtime_ms: meta.expected_mtime_ms,
            expected_size: meta.expected_size,
        },
    )
    .await?;
    if res.code != 200 {
        return Ok(fail(res.msg, res.code));
    }
    Ok(ResData {
        code: 200,
        msg: "操作成功".to_string(),
        data: LineEditOutcome {
            mtime_ms: res.data,
            total_lines: new_text.split('\n').count(),
            encoding,
            eol,
            bom,
        },
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn edit(start: usize, end: usize, text: &str, delete: bool) -> LineEdit {
        LineEdit {
            start_line: start,
            end_line: end,
            text: text.to_string(),
            delete,
        }
    }

    #[test]
    fn replace_single_line_and_expand_multiline() {
        let out = apply_line_edits(
            "alpha\nbeta\ngamma",
            &[edit(2, 2, "BETA\nBETA2", false), edit(3, 3, "G", false)],
        )
        .unwrap();
        assert_eq!(out, "alpha\nBETA\nBETA2\nG");
    }

    #[test]
    fn delete_lines_and_keep_trailing_newline() {
        // 末尾换行的空串必须保留，否则每次保存都会吃掉文件最后一行换行
        let out = apply_line_edits("a\nb\nc\n", &[edit(2, 2, "", true)]).unwrap();
        assert_eq!(out, "a\nc\n");
        let cleared = apply_line_edits("a\nb\nc", &[edit(1, 1, "", false)]).unwrap();
        assert_eq!(cleared, "\nb\nc");
    }

    #[test]
    fn crlf_is_normalized_internally() {
        let out = apply_line_edits("a\r\nb\r\nc", &[edit(2, 2, "X", false)]).unwrap();
        assert_eq!(out, "a\nX\nc");
        assert_eq!(file_meta::normalize_eol(&out, "CRLF"), "a\r\nX\r\nc");
    }

    #[test]
    fn rejects_overlap_out_of_order_and_bounds() {
        assert!(apply_line_edits("a\nb\nc", &[edit(1, 2, "x", false), edit(2, 3, "y", false)]).is_err());
        assert!(apply_line_edits("a\nb\nc", &[edit(3, 3, "y", false), edit(1, 1, "x", false)]).is_err());
        assert!(apply_line_edits("a\nb\nc", &[edit(9, 9, "x", false)]).is_err());
        assert!(apply_line_edits("a\nb\nc", &[edit(0, 1, "x", false)]).is_err());
        assert!(apply_line_edits("a\nb\nc", &[edit(3, 2, "x", false)]).is_err());
        // 拒绝时绝不改动原文本
        assert_eq!(apply_line_edits("a\nb\nc", &[]).unwrap(), "a\nb\nc");
    }

    fn tmp_dir(tag: &str) -> PathBuf {
        let d = std::env::temp_dir().join(format!(
            "bdevtools_file_large_{}_{}_{}",
            tag,
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir_all(&d).unwrap();
        d
    }

    #[tokio::test]
    async fn gbk_crlf_roundtrip_goes_through_save_note() {
        let dir = tmp_dir("gbk");
        let path = dir.join("zh.txt");
        let (src, _, had_err) = encoding_rs::GBK.encode("中文\r\n第二行\r\n第三行");
        assert!(!had_err);
        std::fs::write(&path, src.as_ref()).unwrap();

        let res = apply_line_edits_to_file(
            path.to_string_lossy().to_string(),
            vec![edit(2, 2, "改过的第二行", false)],
            file_meta::SaveMeta::default(),
        )
        .await
        .unwrap();
        assert_eq!(res.code, 200, "{}", res.msg);
        assert_eq!(res.data.encoding, "GBK");
        assert_eq!(res.data.eol, "CRLF");
        assert_eq!(res.data.total_lines, 3);

        let bytes = std::fs::read(&path).unwrap();
        assert!(bytes.windows(2).any(|w| w == b"\r\n"), "EOL 被改写");
        let (text, _, _) = encoding_rs::GBK.decode(&bytes);
        assert_eq!(text, "中文\r\n改过的第二行\r\n第三行");
    }

    #[tokio::test]
    async fn conflict_blocks_patch_and_keeps_original() {
        let dir = tmp_dir("conflict");
        let path = dir.join("a.txt");
        std::fs::write(&path, b"one\ntwo").unwrap();

        let res = apply_line_edits_to_file(
            path.to_string_lossy().to_string(),
            vec![edit(1, 1, "ONE", false)],
            file_meta::SaveMeta {
                expected_mtime_ms: Some(1), // 必然与真实 mtime 不同
                ..Default::default()
            },
        )
        .await
        .unwrap();
        assert_eq!(res.code, file_meta::CODE_CONFLICT);
        assert_eq!(std::fs::read(&path).unwrap(), b"one\ntwo");
    }
}
