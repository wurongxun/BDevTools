/*! # 数据导入导出模块
支持 CSV 和 Excel 格式的单表数据导入导出。
*/
use crate::response::response::Response;
use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};
use tauri::command;
use super::pool::get_pool;
use super::operations::execute_sql;

/// 导出结果
#[derive(Serialize, Deserialize, Debug)]
pub struct ExportResult {
    pub file_path: String,
    pub row_count: u64,
}

/// 导入结果
#[derive(Serialize, Deserialize, Debug)]
pub struct ImportResult {
    pub success_count: u64,
    pub error_count: u64,
    pub errors: Vec<String>,
}

/// 导出表数据为 CSV
#[command]
pub async fn db_export_csv(
    config_id: String,
    database: String,
    table: String,
    file_path: String,
) -> Response<ExportResult> {
    let pool = match get_pool(&config_id) {
        Some(p) => p,
        None => return Response::new(300, ExportResult { file_path: String::new(), row_count: 0 }, "未连接到数据库"),
    };

    let sql = format!("SELECT * FROM `{}`.`{}`", database, table);
    match export_to_csv(&pool, &sql, &file_path).await {
        Ok(count) => Response::new(200, ExportResult { file_path, row_count: count }, "导出成功"),
        Err(e) => Response::new(300, ExportResult { file_path: String::new(), row_count: 0 }, &format!("导出失败: {}", e)),
    }
}

/// 导出表数据为 Excel
#[command]
pub async fn db_export_excel(
    config_id: String,
    database: String,
    table: String,
    file_path: String,
) -> Response<ExportResult> {
    let pool = match get_pool(&config_id) {
        Some(p) => p,
        None => return Response::new(300, ExportResult { file_path: String::new(), row_count: 0 }, "未连接到数据库"),
    };

    let sql = format!("SELECT * FROM `{}`.`{}`", database, table);
    match export_to_excel(&pool, &sql, &file_path, &table).await {
        Ok(count) => Response::new(200, ExportResult { file_path, row_count: count }, "导出成功"),
        Err(e) => Response::new(300, ExportResult { file_path: String::new(), row_count: 0 }, &format!("导出失败: {}", e)),
    }
}

/// 从 CSV 导入数据到表
#[command]
pub async fn db_import_csv(
    config_id: String,
    database: String,
    table: String,
    file_path: String,
    has_header: bool,
) -> Response<ImportResult> {
    let pool = match get_pool(&config_id) {
        Some(p) => p,
        None => return Response::new(300, ImportResult { success_count: 0, error_count: 0, errors: vec![] }, "未连接到数据库"),
    };

    match import_from_csv(&pool, &database, &table, &file_path, has_header).await {
        Ok(result) => Response::new(200, result, "导入成功"),
        Err(e) => Response::new(300, ImportResult { success_count: 0, error_count: 0, errors: vec![e.to_string()] }, &format!("导入失败: {}", e)),
    }
}

/// 从 Excel 导入数据到表
#[command]
pub async fn db_import_excel(
    config_id: String,
    database: String,
    table: String,
    file_path: String,
) -> Response<ImportResult> {
    let pool = match get_pool(&config_id) {
        Some(p) => p,
        None => return Response::new(300, ImportResult { success_count: 0, error_count: 0, errors: vec![] }, "未连接到数据库"),
    };

    match import_from_excel(&pool, &database, &table, &file_path).await {
        Ok(result) => Response::new(200, result, "导入成功"),
        Err(e) => Response::new(300, ImportResult { success_count: 0, error_count: 0, errors: vec![e.to_string()] }, &format!("导入失败: {}", e)),
    }
}

// ==================== 内部辅助函数 ====================

async fn export_to_csv(pool: &super::pool::DbPool, sql: &str, file_path: &str) -> Result<u64> {
    let result = super::operations::execute_sql(pool, sql).await?;
    let mut wtr = csv::Writer::from_path(file_path)?;

    // 写入表头
    wtr.write_record(&result.columns)?;

    // 写入数据行
    for row in &result.rows {
        let str_row: Vec<String> = row.iter().map(|v| match v {
            serde_json::Value::Null => String::new(),
            serde_json::Value::String(s) => s.clone(),
            other => other.to_string(),
        }).collect();
        wtr.write_record(&str_row)?;
    }
    wtr.flush()?;
    Ok(result.rows.len() as u64)
}

async fn export_to_excel(pool: &super::pool::DbPool, sql: &str, file_path: &str, sheet_name: &str) -> Result<u64> {
    let result = super::operations::execute_sql(pool, sql).await?;
    let mut workbook = rust_xlsxwriter::Workbook::new();
    let worksheet = workbook.add_worksheet();
    worksheet.set_name(sheet_name)?;

    // 写入表头
    for (col_idx, col_name) in result.columns.iter().enumerate() {
        worksheet.write_string(0, col_idx as u16, col_name)?;
    }

    // 写入数据
    for (row_idx, row) in result.rows.iter().enumerate() {
        for (col_idx, val) in row.iter().enumerate() {
            match val {
                serde_json::Value::Null => { worksheet.write_string(row_idx as u32 + 1, col_idx as u16, "")?; }
                serde_json::Value::String(s) => { worksheet.write_string(row_idx as u32 + 1, col_idx as u16, s)?; }
                serde_json::Value::Number(n) => {
                    if let Some(i) = n.as_i64() {
                        worksheet.write_number(row_idx as u32 + 1, col_idx as u16, i as f64)?;
                    } else if let Some(f) = n.as_f64() {
                        worksheet.write_number(row_idx as u32 + 1, col_idx as u16, f)?;
                    }
                }
                serde_json::Value::Bool(b) => { worksheet.write_boolean(row_idx as u32 + 1, col_idx as u16, *b)?; }
                _ => { worksheet.write_string(row_idx as u32 + 1, col_idx as u16, &val.to_string())?; }
            }
        }
    }

    workbook.save(file_path)?;
    Ok(result.rows.len() as u64)
}

async fn import_from_csv(pool: &super::pool::DbPool, database: &str, table: &str, file_path: &str, has_header: bool) -> Result<ImportResult> {
    let mut rdr = csv::Reader::from_path(file_path)?;
    let headers: Vec<String> = if has_header {
        rdr.headers()?.iter().map(|s| s.to_string()).collect()
    } else {
        // 无表头时，从数据库获取字段名
        let sql = format!("SELECT * FROM `{}`.`{}` LIMIT 0", database, table);
        let result = super::operations::execute_sql(pool, &sql).await?;
        result.columns
    };

    let mut success_count = 0u64;
    let mut error_count = 0u64;
    let mut errors = Vec::new();

    for (idx, result) in rdr.records().enumerate() {
        match result {
            Ok(record) => {
                let values: Vec<String> = record.iter().map(|v| {
                    if v.is_empty() { "NULL".to_string() }
                    else { format!("'{}'", v.replace('\'', "''")) }
                }).collect();
                let col_str = headers.iter().map(|c| format!("`{}`", c)).collect::<Vec<_>>().join(", ");
                let val_str = values.join(", ");
                let sql = format!("INSERT INTO `{}`.`{}` ({}) VALUES ({})", database, table, col_str, val_str);

                match super::operations::execute_sql(pool, &sql).await {
                    Ok(_) => success_count += 1,
                    Err(e) => {
                        error_count += 1;
                        if errors.len() < 10 {
                            errors.push(format!("行 {}: {}", idx + 1, e));
                        }
                    }
                }
            }
            Err(e) => {
                error_count += 1;
                errors.push(format!("行 {}: {}", idx + 1, e));
            }
        }
    }

    Ok(ImportResult { success_count, error_count, errors })
}

async fn import_from_excel(pool: &super::pool::DbPool, database: &str, table: &str, file_path: &str) -> Result<ImportResult> {
    use calamine::{open_workbook, Reader, Xlsx};

    let mut workbook: Xlsx<_> = open_workbook(file_path).map_err(|e| anyhow!("打开Excel失败: {}", e))?;
    let sheet_name = workbook.sheet_names().first().cloned().ok_or_else(|| anyhow!("Excel无工作表"))?;
    let range = workbook.worksheet_range(&sheet_name).map_err(|e| anyhow!("读取工作表失败: {}", e))?;

    let mut rows_iter = range.rows();
    let headers: Vec<String> = match rows_iter.next() {
        Some(row) => row.iter().map(|c| c.to_string()).collect(),
        None => return Err(anyhow!("Excel为空")),
    };

    let mut success_count = 0u64;
    let mut error_count = 0u64;
    let mut errors = Vec::new();

    for (idx, row) in rows_iter.enumerate() {
        let values: Vec<String> = row.iter().map(|c| {
            match c {
                calamine::Data::Empty => "NULL".to_string(),
                calamine::Data::String(s) if s.is_empty() => "NULL".to_string(),
                calamine::Data::String(s) => format!("'{}'", s.replace('\'', "''")),
                calamine::Data::Int(i) => i.to_string(),
                calamine::Data::Float(f) => f.to_string(),
                calamine::Data::Bool(b) => if *b { "1".to_string() } else { "0".to_string() },
                _ => format!("'{}'", c.to_string().replace('\'', "''")),
            }
        }).collect();

        if values.iter().all(|v| v == "NULL") { continue; } // 跳过空行

        let col_str = headers.iter().map(|c| format!("`{}`", c)).collect::<Vec<_>>().join(", ");
        let val_str = values.join(", ");
        let sql = format!("INSERT INTO `{}`.`{}` ({}) VALUES ({})", database, table, col_str, val_str);

        match super::operations::execute_sql(pool, &sql).await {
            Ok(_) => success_count += 1,
            Err(e) => {
                error_count += 1;
                if errors.len() < 10 {
                    errors.push(format!("行 {}: {}", idx + 2, e));
                }
            }
        }
    }

    Ok(ImportResult { success_count, error_count, errors })
}
