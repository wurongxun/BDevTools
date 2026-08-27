use crate::response::resp_data::ResData;
use lazy_static::lazy_static;
use parking_lot::Mutex;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs::File;
use std::io::{BufRead, BufReader, Read, Seek, SeekFrom};
use std::path::Path;
use encoding_rs::Encoding;
// ==================== 数据结构 ====================

#[derive(Serialize, Clone)]
pub struct LogMatch {
    pub line_number: u64,
    pub content: String,
}

#[derive(Serialize, Clone)]
pub struct LogMatchLite {
    pub line_number: u64,
}

#[derive(Serialize)]
pub struct LogSearchResult {
    pub matches: Vec<LogMatchLite>,
    pub total_matches: u64,
    pub scanned_lines: u64,
    pub elapsed_ms: u64,
}

#[derive(Serialize, Clone)]
pub struct ChunkMeta {
    pub chunk_index: usize,
    pub byte_start: u64,
    pub byte_end: u64,
    pub line_start: u64,
    pub line_count: u64,
    pub match_count: u64,
}

#[derive(Serialize, Clone)]
pub struct SearchIndex {
    pub search_id: String,
    pub total_matches: u64,
    pub scanned_lines: u64,
    pub elapsed_ms: u64,
    pub chunks: Vec<ChunkMeta>,
    pub match_line_numbers: Vec<u64>,
}

#[derive(Deserialize)]
pub struct PageRequest {
    pub search_id: String,
    pub page: u64,
    pub page_size: u64,
}

#[derive(Serialize)]
pub struct PageResult {
    pub matches: Vec<LogMatch>,
    pub page: u64,
    pub page_size: u64,
    pub total_pages: u64,
    pub total_matches: u64,
}

#[derive(Clone)]
struct CachedSearch {
    file_path: String,
    keyword: String,
    chunks: Vec<ChunkMeta>,
    total_matches: u64,
}

lazy_static! {
    static ref SEARCH_CACHE: Mutex<HashMap<String, CachedSearch>> = Mutex::new(HashMap::new());
}

const INDEX_CHUNK_LINES: u64 = 50000;
fn read_line_lossy<R: BufRead>(
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
// ==================== 命令1：流式搜索（只存行号，低内存） ====================

pub async fn search_log_parallel(
    file_path: String,
    keyword: String,
    max_results: Option<u64>,
) -> Result<ResData<LogSearchResult>, String> {
    if !Path::new(&file_path).exists() {
        return Err("文件不存在".to_string());
    }

    let max_results = max_results.unwrap_or(1000) as usize;
    let start = std::time::Instant::now();

    let file_size = std::fs::metadata(&file_path)
        .map_err(|e| format!("获取文件大小失败: {}", e))?
        .len();

    if file_size == 0 {
        return Ok(ResData {
            code: 200,
            msg: "文件为空".to_string(),
            data: LogSearchResult {
                matches: vec![],
                total_matches: 0,
                scanned_lines: 0,
                elapsed_ms: 0,
            },
        });
    }

    let keyword_lower = keyword.to_lowercase();
    let file = File::open(&file_path).map_err(|e| format!("打开文件失败: {}", e))?;
    let mut reader = BufReader::with_capacity(128 * 1024, file);
    let mut line_buf = Vec::with_capacity(1024);

    let mut matches: Vec<LogMatchLite> = Vec::new();
    let mut total_matches: u64 = 0;
    let mut line_number: u64 = 0;

    while let Some((line, _bytes_read)) =
        read_line_lossy(&mut reader, &mut line_buf, None).map_err(|e| format!("读取行失败: {}", e))?
    {
        line_number += 1;
        if line.to_lowercase().contains(&keyword_lower) {
            total_matches += 1;
            if matches.len() < max_results {
                matches.push(LogMatchLite { line_number });
            }
        }
    }

    let elapsed = start.elapsed().as_millis() as u64;

    Ok(ResData {
        code: 200,
        msg: format!("搜索完成，共匹配 {} 处", total_matches),
        data: LogSearchResult {
            matches,
            total_matches,
            scanned_lines: line_number,
            elapsed_ms: elapsed,
        },
    })
}

// ==================== 命令2：建立搜索索引（单线程流式，低内存） ====================

pub async fn search_log_index(
    file_path: String,
    keyword: String,
    encoding: Option<String>,
) -> Result<ResData<SearchIndex>, String> {
    if !Path::new(&file_path).exists() {
        return Err("文件不存在".to_string());
    }

    let start = std::time::Instant::now();
    let file_size = std::fs::metadata(&file_path)
        .map_err(|e| format!("获取文件大小失败: {}", e))?
        .len();

    if file_size == 0 {
        return Ok(ResData {
            code: 200,
            msg: "文件为空".to_string(),
            data: SearchIndex {
                search_id: String::new(),
                total_matches: 0,
                scanned_lines: 0,
                elapsed_ms: 0,
                chunks: vec![],
                match_line_numbers: vec![],
            },
        });
    }

    let keyword_lower = keyword.to_lowercase();
    let file = File::open(&file_path).map_err(|e| format!("打开文件失败: {}", e))?;
    let mut reader = BufReader::with_capacity(128 * 1024, file);
    let mut line_buf = Vec::with_capacity(1024);
    let file_encoding = encoding.as_deref().map(|e| resolve_encoding(e));

    let mut chunks_meta: Vec<ChunkMeta> = Vec::new();
    let mut total_matches: u64 = 0;
    let mut total_lines: u64 = 0;
    let mut current_chunk_lines: u64 = 0;
    let mut current_chunk_matches: u64 = 0;
    let mut current_chunk_byte_start: u64 = 0;
    let mut current_line: u64 = 1;
    let mut current_chunk_line_start: u64 = 1;
    let mut current_byte: u64 = 0;
    let mut match_line_numbers: Vec<u64> = Vec::new();

    while let Some((line, raw_bytes)) =
        read_line_lossy(&mut reader, &mut line_buf, file_encoding).map_err(|e| format!("读取行失败: {}", e))?
    {
        current_byte += raw_bytes as u64;
        current_chunk_lines += 1;
        total_lines += 1;

        if line.to_lowercase().contains(&keyword_lower) {
            current_chunk_matches += 1;
            total_matches += 1;
            match_line_numbers.push(total_lines);
        }

        if current_chunk_lines >= INDEX_CHUNK_LINES {
            chunks_meta.push(ChunkMeta {
                chunk_index: chunks_meta.len(),
                byte_start: current_chunk_byte_start,
                byte_end: current_byte,
                line_start: current_chunk_line_start,
                line_count: current_chunk_lines,
                match_count: current_chunk_matches,
            });
            current_chunk_byte_start = current_byte;
            current_chunk_line_start = current_line + 1;
            current_chunk_lines = 0;
            current_chunk_matches = 0;
        }

        current_line += 1;
    }

    if current_chunk_lines > 0 {
        chunks_meta.push(ChunkMeta {
            chunk_index: chunks_meta.len(),
            byte_start: current_chunk_byte_start,
            byte_end: current_byte,
            line_start: current_chunk_line_start,
            line_count: current_chunk_lines,
            match_count: current_chunk_matches,
        });
    }

    let elapsed = start.elapsed().as_millis() as u64;
    let search_id = format!("{}|{}|{}", file_path, keyword_lower, elapsed);

    {
        let mut cache = SEARCH_CACHE.lock();
        cache.insert(
            search_id.clone(),
            CachedSearch {
                file_path,
                keyword: keyword_lower,
                chunks: chunks_meta.clone(),
                total_matches,
            },
        );
    }

    Ok(ResData {
        code: 200,
        msg: format!("索引完成，共 {} 条匹配", total_matches),
        data: SearchIndex {
            search_id,
            total_matches,
            scanned_lines: total_lines,
            elapsed_ms: elapsed,
            chunks: chunks_meta,
            match_line_numbers,
        },
    })
}

// ==================== 命令3：分页获取搜索结果 ====================

pub async fn search_log_page(
    search_id: String,
    page: u64,
    page_size: Option<u64>,
) -> Result<ResData<PageResult>, String> {
    let page_size = page_size.unwrap_or(50);

    let cached = {
        let cache = SEARCH_CACHE.lock();
        cache
            .get(&search_id)
            .ok_or("搜索已过期，请重新搜索")?
            .clone()
    };

    let total_pages = if cached.total_matches == 0 {
        0
    } else {
        (cached.total_matches + page_size - 1) / page_size
    };

    if page < 1 || (total_pages > 0 && page > total_pages) {
        return Err(format!("页码超出范围，共 {} 页", total_pages));
    }

    let global_start = (page - 1) * page_size + 1;
    let global_end = global_start + page_size - 1;

    let matches = extract_matches_range(&cached, global_start, global_end)?;

    Ok(ResData {
        code: 200,
        msg: "success".to_string(),
        data: PageResult {
            matches,
            page,
            page_size,
            total_pages,
            total_matches: cached.total_matches,
        },
    })
}

// ==================== 命令4：跳转到第 N 条搜索结果 ====================

pub async fn search_log_jump_to(
    search_id: String,
    target_index: u64,
    context_count: Option<u64>,
) -> Result<ResData<Vec<LogMatch>>, String> {
    let context_count = context_count.unwrap_or(5);

    let cached = {
        let cache = SEARCH_CACHE.lock();
        cache
            .get(&search_id)
            .ok_or("搜索已过期，请重新搜索")?
            .clone()
    };

    if target_index < 1 || target_index > cached.total_matches {
        return Err(format!("索引超出范围，共 {} 条匹配", cached.total_matches));
    }

    let start_idx = target_index.saturating_sub(context_count);
    let end_idx = target_index + context_count;
    let matches = extract_matches_range(&cached, start_idx, end_idx)?;

    Ok(ResData {
        code: 200,
        msg: "success".to_string(),
        data: matches,
    })
}

// ==================== 内部工具函数 ====================

struct ChunkInfo {
    byte_start: u64,
    byte_end: u64,
    line_start: u64,
}

fn compute_chunk_infos(
    file_path: &str,
    num_chunks: usize,
    chunk_size: u64,
    file_size: u64,
) -> Result<Vec<ChunkInfo>, String> {
    let file = File::open(file_path).map_err(|e| format!("打开文件失败: {}", e))?;
    let mut reader = BufReader::with_capacity(256 * 1024, file);

    let mut infos: Vec<ChunkInfo> = Vec::with_capacity(num_chunks);
    let mut current_byte: u64 = 0;
    let mut current_line: u64 = 1;

    for i in 0..num_chunks {
        let byte_start = current_byte;
        let line_start = current_line;

        let rough_end = if i == num_chunks - 1 {
            file_size
        } else {
            (i as u64 + 1) * chunk_size
        };

        let actual_end = align_to_newline(&mut reader, rough_end, file_size)?;
        let newline_count = count_newlines(&mut reader, byte_start, actual_end)?;

        current_byte = actual_end;
        current_line += newline_count;

        infos.push(ChunkInfo {
            byte_start,
            byte_end: actual_end,
            line_start,
        });
    }

    Ok(infos)
}

fn align_to_newline(reader: &mut BufReader<File>, pos: u64, file_size: u64) -> Result<u64, String> {
    if pos >= file_size {
        return Ok(file_size);
    }

    reader
        .seek(SeekFrom::Start(pos))
        .map_err(|e| format!("seek 失败: {}", e))?;

    let mut buf = [0u8; 1];
    let mut current = pos;
    while current < file_size {
        reader
            .read_exact(&mut buf)
            .map_err(|e| format!("读取失败: {}", e))?;
        current += 1;
        if buf[0] == b'\n' {
            return Ok(current);
        }
    }
    Ok(file_size)
}

fn count_newlines(reader: &mut BufReader<File>, start: u64, end: u64) -> Result<u64, String> {
    reader
        .seek(SeekFrom::Start(start))
        .map_err(|e| format!("seek 失败: {}", e))?;

    let size = (end - start) as usize;
    let mut buffer = vec![0u8; size];
    reader
        .read_exact(&mut buffer)
        .map_err(|e| format!("读取失败: {}", e))?;

    let count = buffer.iter().filter(|&&b| b == b'\n').count() as u64;
    Ok(count)
}

fn search_chunk_with_line_offset(
    file_path: &str,
    info: ChunkInfo,
    keyword: &str,
) -> Result<(Vec<LogMatch>, u64, u64), String> {
    let mut file = File::open(file_path).map_err(|e| format!("打开文件失败: {}", e))?;
    file.seek(SeekFrom::Start(info.byte_start))
        .map_err(|e| format!("seek 失败: {}", e))?;

    let size = (info.byte_end - info.byte_start) as usize;
    let limited = file.take(size as u64);
    let mut buf_reader = BufReader::with_capacity(64 * 1024, limited);
    let mut line_buf = Vec::with_capacity(1024);

    let mut matches = Vec::new();
    let mut line_number = info.line_start;
    let mut total_matches: u64 = 0;
    let mut total_lines: u64 = 0;

    while let Some((line, _)) =
        read_line_lossy(&mut buf_reader, &mut line_buf, None).map_err(|e| format!("读取行失败: {}", e))?
    {
        if line.to_lowercase().contains(keyword) {
            total_matches += 1;
            matches.push(LogMatch {
                line_number,
                content: line,
            });
        }
        line_number += 1;
        total_lines += 1;
    }

    Ok((matches, total_matches, total_lines))
}

fn extract_matches_range(
    cached: &CachedSearch,
    global_start: u64,
    global_end: u64,
) -> Result<Vec<LogMatch>, String> {
    let mut results: Vec<LogMatch> = Vec::new();
    let mut cumulative: u64 = 0;

    for chunk_meta in &cached.chunks {
        if chunk_meta.match_count == 0 {
            continue;
        }

        let chunk_match_start = cumulative + 1;
        let chunk_match_end = cumulative + chunk_meta.match_count;
        cumulative += chunk_meta.match_count;

        if global_end < chunk_match_start || global_start > chunk_match_end {
            continue;
        }

        let local_start = if global_start > chunk_match_start {
            global_start - chunk_match_start + 1
        } else {
            1
        };
        let local_end = if global_end < chunk_match_end {
            global_end - chunk_match_start + 1
        } else {
            chunk_meta.match_count
        };

        let chunk_matches = extract_matches_from_chunk(
            &cached.file_path,
            chunk_meta,
            &cached.keyword,
            local_start,
            local_end,
        )?;

        results.extend(chunk_matches);
    }

    Ok(results)
}

fn extract_matches_from_chunk(
    file_path: &str,
    chunk_meta: &ChunkMeta,
    keyword: &str,
    local_start: u64,
    local_end: u64,
) -> Result<Vec<LogMatch>, String> {
    let mut file = File::open(file_path).map_err(|e| format!("打开文件失败: {}", e))?;
    file.seek(SeekFrom::Start(chunk_meta.byte_start))
        .map_err(|e| format!("seek 失败: {}", e))?;

    let size = (chunk_meta.byte_end - chunk_meta.byte_start) as usize;
    let limited = file.take(size as u64);
    let mut buf_reader = BufReader::with_capacity(64 * 1024, limited);
    let mut line_buf = Vec::with_capacity(1024);

    let mut matches = Vec::new();
    let mut line_number = chunk_meta.line_start;
    let mut match_index: u64 = 0;

    while let Some((line, _)) =
        read_line_lossy(&mut buf_reader, &mut line_buf, None).map_err(|e| format!("读取行失败: {}", e))?
    {
        if line.to_lowercase().contains(keyword) {
            match_index += 1;
            if match_index >= local_start && match_index <= local_end {
                matches.push(LogMatch {
                    line_number,
                    content: line,
                });
            }
            if match_index > local_end {
                break;
            }
        }
        line_number += 1;
    }

    Ok(matches)
}



#[derive(Serialize)]
pub struct TextPreviewBlock {
    pub content: String,
    pub first_line_number: u64,
    pub highlight_lines: Vec<u64>,
    pub target_line_number: u64,
    pub target_line_in_block: u64,
    pub current_match_index: u64,
    pub total_matches: u64,
}

// ==================== 命令5：文本预览（流式读取，低内存） ====================

pub async fn search_log_preview(
    search_id: String,
    target_index: u64,
    context_lines: Option<u64>,
    encoding: Option<String>,
) -> Result<ResData<TextPreviewBlock>, String> {
    let context_lines = context_lines.unwrap_or(50);
    let file_encoding = encoding.as_deref().map(|e| resolve_encoding(e));

    let cached = {
        let cache = SEARCH_CACHE.lock();
        cache
            .get(&search_id)
            .ok_or("搜索已过期，请重新搜索")?
            .clone()
    };

    if target_index < 1 || target_index > cached.total_matches {
        return Err(format!("索引超出范围，共 {} 条匹配", cached.total_matches));
    }

    let preview = find_match_and_read_context(&cached, target_index, context_lines, file_encoding)?;
    Ok(ResData {
        code: 200,
        msg: "success".to_string(),
        data: preview,
    })
}



fn find_match_and_read_context(
    cached: &CachedSearch,
    target_index: u64,
    context_lines: u64,
    encoding: Option<&'static Encoding>,
) -> Result<TextPreviewBlock, String> {
    let mut cumulative: u64 = 0;

    for chunk_meta in &cached.chunks {
        if chunk_meta.match_count == 0 {
            continue;
        }

        let chunk_match_start = cumulative + 1;
        let chunk_match_end = cumulative + chunk_meta.match_count;

        if target_index < chunk_match_start || target_index > chunk_match_end {
            cumulative += chunk_meta.match_count;
            continue;
        }

        let local_target = target_index - cumulative;
        return read_preview_from_chunk(
            &cached.file_path,
            chunk_meta,
            &cached.keyword,
            local_target,
            target_index,
            cached.total_matches,
            context_lines,
            encoding,
        );
    }

    Err("未找到目标匹配所在的块".to_string())
}

fn read_preview_from_chunk(
    file_path: &str,
    chunk_meta: &ChunkMeta,
    keyword: &str,
    local_target: u64,
    global_target: u64,
    total_matches: u64,
    context_lines: u64,
    encoding: Option<&'static Encoding>,
) -> Result<TextPreviewBlock, String> {
    let mut file = File::open(file_path).map_err(|e| format!("打开文件失败: {}", e))?;
    file.seek(SeekFrom::Start(chunk_meta.byte_start))
        .map_err(|e| format!("seek 失败: {}", e))?;

    let size = (chunk_meta.byte_end - chunk_meta.byte_start) as usize;
    let limited = file.take(size as u64);
    let mut buf_reader = BufReader::with_capacity(64 * 1024, limited);
    let mut line_buf = Vec::with_capacity(1024);

    let mut match_line_number: u64 = 0;
    let mut match_index: u64 = 0;
    let mut all_lines: Vec<String> = Vec::new();

    while let Some((line, _)) =
        read_line_lossy(&mut buf_reader, &mut line_buf, encoding).map_err(|e| format!("读取行失败: {}", e))?
    {
        if line.to_lowercase().contains(keyword) {
            match_index += 1;
            if match_index == local_target {
                match_line_number = chunk_meta.line_start + all_lines.len() as u64;
            }
        }
        all_lines.push(line);
    }

    if match_line_number == 0 {
        return Err("块内未找到目标匹配行".to_string());
    }

    let total_lines = all_lines.len() as u64;
    let window_start_in_chunk = if match_line_number >= chunk_meta.line_start + context_lines {
        match_line_number - chunk_meta.line_start - context_lines
    } else {
        0
    };
    let window_end_in_chunk =
        ((match_line_number - chunk_meta.line_start) + context_lines + 1).min(total_lines);

    let first_line_number = chunk_meta.line_start + window_start_in_chunk;
    let mut block_lines: Vec<String> = Vec::new();
    let mut highlight_lines: Vec<u64> = Vec::new();
    let mut target_line_in_block: u64 = 0;

    for i in window_start_in_chunk..window_end_in_chunk {
        let abs_line = chunk_meta.line_start + i;
        block_lines.push(all_lines[i as usize].clone());

        if all_lines[i as usize].to_lowercase().contains(keyword) {
            highlight_lines.push(abs_line);
        }

        if abs_line == match_line_number {
            target_line_in_block = block_lines.len() as u64;
        }
    }

    let block_content = block_lines.join("\n");

    Ok(TextPreviewBlock {
        content: block_content,
        first_line_number,
        highlight_lines,
        target_line_number: match_line_number,
        target_line_in_block,
        current_match_index: global_target,
        total_matches,
    })
}



// ==================== 命令6：按行范围读取文件内容 ====================


#[derive(Serialize)]
pub struct FileLinesResult {
    pub content: String,
    pub first_line_number: u64,
    pub total_lines: u64,
    pub has_more: bool,
}

fn resolve_encoding(name: &str) -> &'static Encoding {
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
    let encoding = resolve_encoding(&encoding.unwrap_or_else(|| "UTF-8".to_string()));

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

    let mut current_line: u64 = 1;
    let mut line_bytes: Vec<(u64, u64)> = Vec::with_capacity(line_count as usize);
    let mut byte_pos: u64 = 0;
    let mut line_start: u64 = 0;
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
