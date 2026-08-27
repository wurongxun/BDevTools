// src-tauri/src/command/log_search.rs

use crate::response::resp_data::ResData;
use serde::Serialize;
use std::path::Path;
use tokio::fs::File;
use tokio::io::{AsyncBufReadExt, BufReader};

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

/// 在超大日志文件中搜索关键词（流式读取，内存占用极低）
pub async fn search_log_file(
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

    let max_results = max_results.unwrap_or(1000);
    let start = std::time::Instant::now();

    let file = File::open(&file_path)
        .await
        .map_err(|e| format!("打开文件失败: {}", e))?;
    // 使用较大的缓冲区（256KB），减少系统调用次数
    let reader = BufReader::with_capacity(256 * 1024, file);
    let mut lines = reader.lines();

    let mut matches: Vec<LogMatch> = Vec::new();
    let mut line_number: u64 = 0;
    let mut total_matches: u64 = 0;

    let keyword_lower = keyword.to_lowercase();

    while let Some(line) = lines
        .next_line()
        .await
        .map_err(|e| format!("读取失败: {}", e))?
    {
        line_number += 1;

        // 大小写不敏感匹配
        if line.to_lowercase().contains(&keyword_lower) {
            total_matches += 1;
            if matches.len() < max_results as usize {
                matches.push(LogMatch {
                    line_number,
                    content: line,
                });
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
