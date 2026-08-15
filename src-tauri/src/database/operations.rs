/*! # 数据库操作模块
SQL 执行、表数据增删改查。
*/
use crate::response::response::Response;
use anyhow::Result;
use serde::{Deserialize, Serialize};
use sqlx::mysql::MySqlPool;
use sqlx::postgres::PgPool;
use sqlx::{Row, Column, Executor};
use tauri::command;
use super::pool::{get_pool, DbPool};

/// SQL 执行结果
#[derive(Serialize, Deserialize, Debug)]
pub struct SqlResult {
    pub columns: Vec<String>,
    pub rows: Vec<Vec<serde_json::Value>>,
    pub affected_rows: u64,
    pub execution_time_ms: u64,
    pub is_query: bool,
}

/// 表数据查询结果
#[derive(Serialize, Deserialize, Debug)]
pub struct TableDataResult {
    pub columns: Vec<ColumnInfo>,
    pub rows: Vec<Vec<serde_json::Value>>,
    pub total: u64,
    pub page: u64,
    pub page_size: u64,
}

/// 字段信息
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ColumnInfo {
    pub name: String,
    pub data_type: String,
    pub is_nullable: bool,
    pub is_primary_key: bool,
    pub default_value: Option<String>,
    pub comment: Option<String>,
}

/// 执行 SQL 语句
#[command]
pub async fn db_execute_sql(config_id: String, sql: String, database: Option<String>) -> Response<SqlResult> {
    let pool = match get_pool(&config_id) {
        Some(p) => p,
        None => return Response::new(300, SqlResult { columns: vec![], rows: vec![], affected_rows: 0, execution_time_ms: 0, is_query: false }, "未连接到数据库"),
    };

    let start = std::time::Instant::now();

    // 如果指定了数据库，获取专用连接先切换数据库再执行 SQL
    let result = if let Some(db) = &database {
        if !db.is_empty() {
            execute_with_database(&pool, db, &sql).await
        } else {
            execute_sql(&pool, &sql).await
        }
    } else {
        execute_sql(&pool, &sql).await
    };

    let elapsed = start.elapsed().as_millis() as u64;

    match result {
        Ok(res) => {
            let mut res = res;
            res.execution_time_ms = elapsed;
            Response::new(200, res, "执行成功")
        }
        Err(e) => Response::new(300, SqlResult { columns: vec![], rows: vec![], affected_rows: 0, execution_time_ms: elapsed, is_query: false }, &format!("执行失败: {}", e)),
    }
}

/// 在专用连接上先切换数据库再执行 SQL（确保同一连接）
async fn execute_with_database(pool: &DbPool, database: &str, sql: &str) -> Result<SqlResult> {
    let is_query = is_select_query(sql);
    match pool {
        DbPool::MySQL(p) => {
            let mut conn = p.acquire().await?;
            // 在同一连接上先 USE 再执行 SQL（PoolConnection Deref 到底层连接）
            let use_sql = format!("USE `{}`", database);
            Executor::execute(&mut *conn, use_sql.as_str()).await?;
            if is_query {
                let rows = sqlx::query(sql).fetch_all(&mut *conn).await?;
                Ok(build_query_result(rows))
            } else {
                let result = sqlx::query(sql).execute(&mut *conn).await?;
                Ok(SqlResult { columns: vec![], rows: vec![], affected_rows: result.rows_affected(), execution_time_ms: 0, is_query: false })
            }
        }
        DbPool::PostgreSQL(p) => {
            let mut conn = p.acquire().await?;
            let use_sql = format!("SET search_path TO \"{}\"", database);
            Executor::execute(&mut *conn, use_sql.as_str()).await?;
            if is_query {
                let rows = sqlx::query(sql).fetch_all(&mut *conn).await?;
                Ok(build_pg_query_result(rows))
            } else {
                let result = sqlx::query(sql).execute(&mut *conn).await?;
                Ok(SqlResult { columns: vec![], rows: vec![], affected_rows: result.rows_affected(), execution_time_ms: 0, is_query: false })
            }
        }
    }
}

/// 将 MySQL 查询结果行转换为 SqlResult
fn build_query_result(rows: Vec<sqlx::mysql::MySqlRow>) -> SqlResult {
    let mut columns = Vec::new();
    let mut data_rows = Vec::new();
    if let Some(first_row) = rows.first() {
        for col in first_row.columns() {
            columns.push(col.name().to_string());
        }
    }
    for row in &rows {
        let mut values = Vec::new();
        for (i, _) in columns.iter().enumerate() {
            let val: Option<String> = row.try_get(i).ok();
            values.push(match val {
                Some(v) => serde_json::Value::String(v),
                None => serde_json::Value::Null,
            });
        }
        data_rows.push(values);
    }
    SqlResult { columns, rows: data_rows, affected_rows: 0, execution_time_ms: 0, is_query: true }
}

/// 将 PostgreSQL 查询结果行转换为 SqlResult
fn build_pg_query_result(rows: Vec<sqlx::postgres::PgRow>) -> SqlResult {
    let mut columns = Vec::new();
    let mut data_rows = Vec::new();
    if let Some(first_row) = rows.first() {
        for col in first_row.columns() {
            columns.push(col.name().to_string());
        }
    }
    for row in &rows {
        let mut values = Vec::new();
        for (i, _) in columns.iter().enumerate() {
            let val: Option<String> = row.try_get(i).ok();
            values.push(match val {
                Some(v) => serde_json::Value::String(v),
                None => serde_json::Value::Null,
            });
        }
        data_rows.push(values);
    }
    SqlResult { columns, rows: data_rows, affected_rows: 0, execution_time_ms: 0, is_query: true }
}

/// 分页查询表数据
#[command]
pub async fn db_query_table_data(
    config_id: String,
    database: String,
    table: String,
    page: u64,
    page_size: u64,
    where_clause: Option<String>,
    order_by: Option<String>,
) -> Response<TableDataResult> {
    let pool = match get_pool(&config_id) {
        Some(p) => p,
        None => return Response::new(300, TableDataResult { columns: vec![], rows: vec![], total: 0, page: 1, page_size: 20 }, "未连接到数据库"),
    };

    let where_sql = where_clause.as_deref().unwrap_or("");
    let order_sql = order_by.as_deref().unwrap_or("");

    // 查询总数
    let count_sql = if where_sql.is_empty() {
        format!("SELECT COUNT(*) FROM `{}`.`{}`", database, table)
    } else {
        format!("SELECT COUNT(*) FROM `{}`.`{}` WHERE {}", database, table, where_sql)
    };

    let total = match query_count(&pool, &count_sql).await {
        Ok(n) => n,
        Err(e) => return Response::new(300, TableDataResult { columns: vec![], rows: vec![], total: 0, page, page_size }, &format!("查询失败: {}", e)),
    };

    // 查询数据
    let offset = (page - 1) * page_size;
    let data_sql = if where_sql.is_empty() {
        if order_sql.is_empty() {
            format!("SELECT * FROM `{}`.`{}` LIMIT {} OFFSET {}", database, table, page_size, offset)
        } else {
            format!("SELECT * FROM `{}`.`{}` ORDER BY {} LIMIT {} OFFSET {}", database, table, order_sql, page_size, offset)
        }
    } else {
        if order_sql.is_empty() {
            format!("SELECT * FROM `{}`.`{}` WHERE {} LIMIT {} OFFSET {}", database, table, where_sql, page_size, offset)
        } else {
            format!("SELECT * FROM `{}`.`{}` WHERE {} ORDER BY {} LIMIT {} OFFSET {}", database, table, where_sql, order_sql, page_size, offset)
        }
    };

    match execute_sql(&pool, &data_sql).await {
        Ok(res) => {
            let columns = res.columns.iter().map(|c| ColumnInfo {
                name: c.clone(),
                data_type: String::new(),
                is_nullable: true,
                is_primary_key: false,
                default_value: None,
                comment: None,
            }).collect();
            Response::new(200, TableDataResult {
                columns,
                rows: res.rows,
                total,
                page,
                page_size,
            }, "查询成功")
        }
        Err(e) => Response::new(300, TableDataResult { columns: vec![], rows: vec![], total: 0, page, page_size }, &format!("查询失败: {}", e)),
    }
}

/// 插入单行数据
#[command]
pub async fn db_insert_row(
    config_id: String,
    database: String,
    table: String,
    data: serde_json::Value,
) -> Response<bool> {
    let pool = match get_pool(&config_id) {
        Some(p) => p,
        None => return Response::new(300, false, "未连接到数据库"),
    };

    let obj = match data.as_object() {
        Some(o) => o,
        None => return Response::new(300, false, "数据格式错误"),
    };

    let columns: Vec<&str> = obj.keys().map(|k| k.as_str()).collect();
    let values: Vec<&serde_json::Value> = obj.values().collect();

    let col_str = columns.iter().map(|c| format!("`{}`", c)).collect::<Vec<_>>().join(", ");
    let val_placeholders: Vec<String> = values.iter().map(|v| format_value(v)).collect();
    let val_str = val_placeholders.join(", ");

    let sql = format!("INSERT INTO `{}`.`{}` ({}) VALUES ({})", database, table, col_str, val_str);

    match execute_sql(&pool, &sql).await {
        Ok(_) => Response::new(200, true, "插入成功"),
        Err(e) => Response::new(300, false, &format!("插入失败: {}", e)),
    }
}

/// 更新单行数据
#[command]
pub async fn db_update_row(
    config_id: String,
    database: String,
    table: String,
    primary_key: String,
    primary_value: serde_json::Value,
    data: serde_json::Value,
) -> Response<bool> {
    let pool = match get_pool(&config_id) {
        Some(p) => p,
        None => return Response::new(300, false, "未连接到数据库"),
    };

    let obj = match data.as_object() {
        Some(o) => o,
        None => return Response::new(300, false, "数据格式错误"),
    };

    let set_clauses: Vec<String> = obj.iter()
        .map(|(k, v)| format!("`{}` = {}", k, format_value(v)))
        .collect();
    let set_str = set_clauses.join(", ");
    let pk_val = format_value(&primary_value);

    let sql = format!("UPDATE `{}`.`{}` SET {} WHERE `{}` = {}", database, table, set_str, primary_key, pk_val);

    match execute_sql(&pool, &sql).await {
        Ok(_) => Response::new(200, true, "更新成功"),
        Err(e) => Response::new(300, false, &format!("更新失败: {}", e)),
    }
}

/// 删除单行数据
#[command]
pub async fn db_delete_row(
    config_id: String,
    database: String,
    table: String,
    primary_key: String,
    primary_value: serde_json::Value,
) -> Response<bool> {
    let pool = match get_pool(&config_id) {
        Some(p) => p,
        None => return Response::new(300, false, "未连接到数据库"),
    };

    let pk_val = format_value(&primary_value);
    let sql = format!("DELETE FROM `{}`.`{}` WHERE `{}` = {}", database, table, primary_key, pk_val);

    match execute_sql(&pool, &sql).await {
        Ok(_) => Response::new(200, true, "删除成功"),
        Err(e) => Response::new(300, false, &format!("删除失败: {}", e)),
    }
}

// ==================== 内部辅助函数 ====================

/// 执行 SQL 并返回结果（自动判断是否为查询语句）
pub async fn execute_sql(pool: &DbPool, sql: &str) -> Result<SqlResult> {
    let is_query = is_select_query(sql);
    execute_sql_with_mode(pool, sql, is_query).await
}

/// 执行 SQL 并返回结果（显式指定是否为查询语句）
async fn execute_sql_with_mode(pool: &DbPool, sql: &str, is_query: bool) -> Result<SqlResult> {
    match pool {
        DbPool::MySQL(p) => execute_mysql(p, sql, is_query).await,
        DbPool::PostgreSQL(p) => execute_postgres(p, sql, is_query).await,
    }
}

async fn execute_mysql(pool: &MySqlPool, sql: &str, is_query: bool) -> Result<SqlResult> {
    if is_query {
        let rows = sqlx::query(sql).fetch_all(pool).await?;
        let mut columns = Vec::new();
        let mut data_rows = Vec::new();

        if let Some(first_row) = rows.first() {
            for col in first_row.columns() {
                columns.push(col.name().to_string());
            }
        }

        for row in &rows {
            let mut values = Vec::new();
            for (i, _) in columns.iter().enumerate() {
                let val: Option<String> = row.try_get(i).ok();
                values.push(match val {
                    Some(v) => serde_json::Value::String(v),
                    None => serde_json::Value::Null,
                });
            }
            data_rows.push(values);
        }

        Ok(SqlResult {
            columns,
            rows: data_rows,
            affected_rows: 0,
            execution_time_ms: 0,
            is_query: true,
        })
    } else {
        let result = sqlx::query(sql).execute(pool).await?;
        Ok(SqlResult {
            columns: vec![],
            rows: vec![],
            affected_rows: result.rows_affected(),
            execution_time_ms: 0,
            is_query: false,
        })
    }
}

async fn execute_postgres(pool: &PgPool, sql: &str, is_query: bool) -> Result<SqlResult> {
    if is_query {
        let rows = sqlx::query(sql).fetch_all(pool).await?;
        let mut columns = Vec::new();
        let mut data_rows = Vec::new();

        if let Some(first_row) = rows.first() {
            for col in first_row.columns() {
                columns.push(col.name().to_string());
            }
        }

        for row in &rows {
            let mut values = Vec::new();
            for (i, _) in columns.iter().enumerate() {
                let val: Option<String> = row.try_get(i).ok();
                values.push(match val {
                    Some(v) => serde_json::Value::String(v),
                    None => serde_json::Value::Null,
                });
            }
            data_rows.push(values);
        }

        Ok(SqlResult {
            columns,
            rows: data_rows,
            affected_rows: 0,
            execution_time_ms: 0,
            is_query: true,
        })
    } else {
        let result = sqlx::query(sql).execute(pool).await?;
        Ok(SqlResult {
            columns: vec![],
            rows: vec![],
            affected_rows: result.rows_affected(),
            execution_time_ms: 0,
            is_query: false,
        })
    }
}

async fn query_count(pool: &DbPool, sql: &str) -> Result<u64> {
    match pool {
        DbPool::MySQL(p) => {
            let row: (i64,) = sqlx::query_as(sql).fetch_one(p).await?;
            Ok(row.0 as u64)
        }
        DbPool::PostgreSQL(p) => {
            let row: (i64,) = sqlx::query_as(sql).fetch_one(p).await?;
            Ok(row.0 as u64)
        }
    }
}

/// 判断是否为查询语句（跳过前导注释）
fn is_select_query(sql: &str) -> bool {
    let stripped = strip_leading_comments(sql).trim().to_uppercase();
    stripped.starts_with("SELECT") || stripped.starts_with("SHOW") || stripped.starts_with("DESCRIBE") || stripped.starts_with("EXPLAIN")
}

/// 去除 SQL 前导注释（支持 -- 单行注释和 /* */ 块注释）
fn strip_leading_comments(sql: &str) -> &str {
    let mut s = sql.trim_start();
    loop {
        if s.starts_with("--") {
            // 单行注释：跳到行尾
            if let Some(pos) = s.find('\n') {
                s = s[pos + 1..].trim_start();
            } else {
                return "";
            }
        } else if s.starts_with("/*") {
            // 块注释：跳到 */
            if let Some(pos) = s.find("*/") {
                s = s[pos + 2..].trim_start();
            } else {
                return s;
            }
        } else {
            break;
        }
    }
    s
}

/// 格式化 JSON 值为 SQL 值字符串
pub fn format_value(val: &serde_json::Value) -> String {
    match val {
        serde_json::Value::Null => "NULL".to_string(),
        serde_json::Value::Bool(b) => if *b { "1".to_string() } else { "0".to_string() },
        serde_json::Value::Number(n) => n.to_string(),
        serde_json::Value::String(s) => format!("'{}'", s.replace('\'', "''")),
        _ => format!("'{}'", val.to_string().replace('\'', "''")),
    }
}
