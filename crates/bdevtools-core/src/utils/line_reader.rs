//! 行读取 / 编码解析的纯函数部分（不依赖 Tauri 与任何插件）。
//! 带 logs 行索引加速的 read_file_lines / count_file_lines 留在宿主 crate。
use encoding_rs::Encoding;
use std::io::BufRead;

pub fn read_line_lossy<R: BufRead>(
    reader: &mut R,
    buf: &mut Vec<u8>,
    encoding: Option<&'static Encoding>,
) -> std::io::Result<Option<(String, usize)>> {
    buf.clear();
    let bytes_read = reader.read_until(b'\n', buf)?;
    if bytes_read == 0 {
        return Ok(None);
    }
    let mut end = bytes_read;
    if end > 0 && buf[end - 1] == b'\n' {
        end -= 1;
    }
    if end > 0 && buf[end - 1] == b'\r' {
        end -= 1;
    }
    let line = match encoding {
        Some(enc) if enc.name() != "utf-8" => {
            let (cow, _, _) = enc.decode(&buf[..end]);
            cow.into_owned()
        }
        _ => String::from_utf8_lossy(&buf[..end]).to_string(),
    };
    Ok(Some((line, bytes_read)))
}

/// 编码名 → encoding_rs 静态编码（log_index 模块复用）
pub fn resolve_encoding(name: &str) -> &'static Encoding {
    match name.to_uppercase().as_str() {
        "UTF-8" | "UTF8" => &encoding_rs::UTF_8,
        "GBK" | "GB2312" | "GB18030" | "CP936" => &encoding_rs::GBK,
        "BIG5" => &encoding_rs::BIG5,
        "UTF-16LE" => &encoding_rs::UTF_16LE,
        "UTF-16BE" => &encoding_rs::UTF_16BE,
        "SHIFT_JIS" | "SHIFT-JIS" => &encoding_rs::SHIFT_JIS,
        "EUC-JP" => &encoding_rs::EUC_JP,
        "EUC-KR" => &encoding_rs::EUC_KR,
        "ASCII" => &encoding_rs::UTF_8,
        _ => {
            Encoding::for_label(name.as_bytes()).unwrap_or(&encoding_rs::UTF_8)
        }
    }
}

