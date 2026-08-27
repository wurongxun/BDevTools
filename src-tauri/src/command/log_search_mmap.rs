// src-tauri/src/command/log_search.rs

use crate::response::resp_data::ResData;
use memmap2::MmapOptions;
use serde::Serialize;
use std::fs::File;
use std::path::Path;

#[derive(Serialize, Clone)]
pub struct LogMatch {
    pub line_number: u64,
    pub content: String,
}

#[derive(Serialize)]
pub struct LogSearchResult {
    pub matches: Vec<LogMatch>,
    pub total_matches: u64,
    pub scanned_lines: u64,
    pub elapsed_ms: u64,
}

/// 使用 mmap 搜索超大日志文件（OS 管理内存分页，10GB+ 文件无压力）
pub fn search_log_mmap(
    file_path: String,
    keyword: String,
    max_results: Option<u64>,
) -> Result<ResData<LogSearchResult>, String> {
    if !Path::new(&file_path).exists() {
        return Ok(ResData {
            code: 500,
            msg: "文件不存在".to_string(),
            data: LogSearchResult {
                matches: vec![],
                total_matches: 0,
                scanned_lines: 0,
                elapsed_ms: 0,
            },
        });
    }

    let max_results = max_results.unwrap_or(1000) as usize;
    let start = std::time::Instant::now();

    let file = File::open(&file_path).map_err(|e| format!("打开文件失败: {}", e))?;
    // 安全：只读映射
    let mmap = unsafe {
        MmapOptions::new()
            .populate()
            .map(&file)
            .map_err(|e| format!("内存映射失败: {}", e))?
    };

    let keyword_lower = keyword.to_lowercase();
    let mut matches: Vec<LogMatch> = Vec::with_capacity(max_results.min(1000));
    let mut total_matches: u64 = 0;
    let mut line_number: u64 = 0;

    // mmap 按行扫描：找到每个 '\n' 就处理一行
    let mut line_start = 0;
    for (i, &byte) in mmap.iter().enumerate() {
        if byte == b'\n' || i == mmap.len() - 1 {
            line_number += 1;
            let line_end = if byte == b'\n' { i } else { i + 1 };
            let line_bytes = &mmap[line_start..line_end];

            // 尝试 UTF-8 解码，失败则跳过
            if let Ok(line_str) = std::str::from_utf8(line_bytes) {
                let line_trimmed = line_str.trim_end_matches('\r');
                if line_trimmed.to_lowercase().contains(&keyword_lower) {
                    total_matches += 1;
                    if matches.len() < max_results {
                        matches.push(LogMatch {
                            line_number,
                            content: line_trimmed.to_string(),
                        });
                    }
                }
            }
            line_start = i + 1;
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
