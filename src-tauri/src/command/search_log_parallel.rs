use crate::response::resp_data::ResData;
use serde::Serialize;
use std::fs::File;
use std::io::{BufRead, BufReader, Read, Seek, SeekFrom};
use std::path::Path;
use encoding_rs::Encoding;
// ==================== 行读取工具（log_index 模块复用） ====================

pub(crate) fn read_line_lossy<R: BufRead>(
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

#[derive(Serialize)]
pub struct FileLinesResult {
    pub content: String,
    pub first_line_number: u64,
    pub total_lines: u64,
    pub has_more: bool,
}

/// 编码名 → encoding_rs 静态编码（log_index 模块复用）
pub(crate) fn resolve_encoding(name: &str) -> &'static Encoding {
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

pub async fn read_file_lines(
    file_path: String,
    start_line: Option<u64>,
    line_count: Option<u64>,
    encoding: Option<String>,
) -> Result<ResData<FileLinesResult>, String> {
    if !Path::new(&file_path).exists() {
        return Err("文件不存在".to_string());
    }

    let start_line = start_line.unwrap_or(1);
    let line_count = line_count.unwrap_or(1000);
    let enc_name = encoding.unwrap_or_else(|| "UTF-8".to_string());
    let encoding = resolve_encoding(&enc_name);

    let file_size = std::fs::metadata(&file_path)
        .map_err(|e| format!("获取文件大小失败: {}", e))?
        .len();

    if file_size == 0 {
        return Ok(ResData {
            code: 200,
            msg: "文件为空".to_string(),
            data: FileLinesResult {
                content: String::new(),
                first_line_number: 1,
                total_lines: 0,
                has_more: false,
            },
        });
    }

    let file = File::open(&file_path).map_err(|e| format!("打开文件失败: {}", e))?;
    let mut reader = BufReader::with_capacity(256 * 1024, file);

    // L2 行偏移命中 ⇒ 直接 seek 到目标行，把"从文件头数换行"的 O(N) 降为 O(段长)。
    // 无索引（未建 / 已过期 / 通用文本文件）时保持原有顺序扫描路径，行为完全不变。
    let seek_hit = if start_line > 1 {
        match crate::command::log_index::lookup_line_offset(&file_path, &enc_name, start_line) {
            Some(off) => {
                reader
                    .seek(SeekFrom::Start(off))
                    .map_err(|e| format!("seek 失败: {}", e))?;
                Some(off)
            }
            None => None,
        }
    } else {
        None
    };

    let mut current_line: u64 = if seek_hit.is_some() { start_line } else { 1 };
    let mut line_bytes: Vec<(u64, u64)> = Vec::with_capacity(line_count as usize);
    let mut byte_pos: u64 = seek_hit.unwrap_or(0);
    let mut line_start: u64 = byte_pos;
    let mut found_end = false;

    let mut buf = [0u8; 8192];

    loop {
        let n = reader
            .read(&mut buf)
            .map_err(|e| format!("读取失败: {}", e))?;
        if n == 0 {
            if line_start < byte_pos {
                if current_line >= start_line && current_line < start_line + line_count {
                    line_bytes.push((line_start, byte_pos));
                }
            }
            found_end = true;
            break;
        }

        for i in 0..n {
            if buf[i] == b'\n' {
                if current_line >= start_line && current_line < start_line + line_count {
                    line_bytes.push((line_start, byte_pos + i as u64 + 1));
                }
                current_line += 1;
                line_start = byte_pos + i as u64 + 1;

                if current_line >= start_line + line_count {
                    found_end = true;
                    break;
                }
            }
        }
        byte_pos += n as u64;

        if found_end {
            break;
        }
    }

    if line_bytes.is_empty() {
        return Ok(ResData {
            code: 200,
            msg: "success".to_string(),
            data: FileLinesResult {
                content: String::new(),
                first_line_number: start_line,
                total_lines: 0,
                has_more: false,
            },
        });
    }

    let first_byte = line_bytes.first().unwrap().0;
    let last_byte = line_bytes.last().unwrap().1;
    let read_size = (last_byte - first_byte) as usize;

    let mut file2 = File::open(&file_path).map_err(|e| format!("打开文件失败: {}", e))?;
    file2
        .seek(SeekFrom::Start(first_byte))
        .map_err(|e| format!("seek 失败: {}", e))?;
    let mut buffer = vec![0u8; read_size];
    file2
        .read_exact(&mut buffer)
        .map_err(|e| format!("读取失败: {}", e))?;

    let (cow, _encoding_used, _had_errors) = encoding.decode(&buffer);
    let content = cow.into_owned();
    let has_more = !found_end || current_line > start_line + line_count;

    Ok(ResData {
        code: 200,
        msg: "success".to_string(),
        data: FileLinesResult {
            content,
            first_line_number: start_line,
            total_lines: 0,
            has_more,
        },
    })
}

// ==================== 命令7：快速统计文件总行数 ====================

pub async fn count_file_lines(
    file_path: String,
) -> Result<ResData<u64>, String> {
    if !Path::new(&file_path).exists() {
        return Err("文件不存在".to_string());
    }

    let file = File::open(&file_path).map_err(|e| format!("打开文件失败: {}", e))?;
    let mut reader = BufReader::with_capacity(512 * 1024, file);
    let mut count: u64 = 0;
    let mut buf = [0u8; 65536];

    loop {
        let n = reader
            .read(&mut buf)
            .map_err(|e| format!("读取失败: {}", e))?;
        if n == 0 {
            break;
        }
        count += buf[..n].iter().filter(|&&b| b == b'\n').count() as u64;
    }

    Ok(ResData {
        code: 200,
        msg: "success".to_string(),
        data: count,
    })
}
