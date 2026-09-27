/*! # 数据库操作模块
SQL 执行、表数据增删改查。
*/
use crate::response::response::Response;
use anyhow::Result;
use serde::{Deserialize, Serialize};
use sqlx::{Row, Column, Executor, ValueRef};
use futures_util::TryStreamExt;
use super::pool::{get_pool, DbPool};
use super::dialect::{self, Dialect};
use super::mssql::SqlServerPool;
use super::odbc::OraclePool;
use std::sync::Arc;

/// 查询默认每页行数（前后端一致）
pub const DEFAULT_PAGE_SIZE: usize = 200;
/// 后端单次查询最大返回行数（硬上限）
pub const MAX_PAGE_SIZE: usize = 1000;

/// SQL 执行结果
#[derive(Serialize, Deserialize, Debug)]
pub struct SqlResult {
    pub columns: Vec<String>,
    pub rows: Vec<Vec<serde_json::Value>>,
    pub affected_rows: u64,
    pub execution_time_ms: u64,
    pub is_query: bool,
    /// 是否还有下一页数据（达到行数上限且探测到后续行）
    #[serde(default)]
    pub has_more: bool,
    /// 查询结果总行数（仅纯 SELECT 且为首页时统计；DML / 无法包装的语句为 None）
    #[serde(default)]
    pub total: Option<u64>,
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

/// 执行 SQL 语句（分页：默认 200 行/页，前端可指定，后端上限 1000）
pub async fn db_execute_sql(
    config_id: String,
    sql: String,
    database: Option<String>,
    page_size: Option<u64>,
    offset: Option<u64>,
) -> Response<SqlResult> {
    let pool = match get_pool(&config_id) {
        Some(p) => p,
        None => return Response::new(300, SqlResult { columns: vec![], rows: vec![], affected_rows: 0, execution_time_ms: 0, is_query: false, has_more: false, total: None }, "未连接到数据库"),
    };

    // 前端指定行数以前端为准，后端限制在 1..=1000；未指定时默认 200
    let limit = page_size.map(|v| v as usize).unwrap_or(DEFAULT_PAGE_SIZE).clamp(1, MAX_PAGE_SIZE);
    let off = offset.unwrap_or(0);

    let start = std::time::Instant::now();

    // 如果指定了数据库，获取专用连接先切换数据库再执行 SQL
    let result = if let Some(db) = &database {
        if !db.is_empty() {
            execute_with_database(&pool, db, &sql, limit, off).await
        } else {
            execute_sql_paged(&pool, &sql, limit, off).await
        }
    } else {
        execute_sql_paged(&pool, &sql, limit, off).await
    };

    let elapsed = start.elapsed().as_millis() as u64;

    match result {
        Ok(res) => {
            let mut res = res;
            res.execution_time_ms = elapsed;
            Response::new(200, res, "执行成功")
        }
        Err(e) => Response::new(300, SqlResult { columns: vec![], rows: vec![], affected_rows: 0, execution_time_ms: elapsed, is_query: false, has_more: false, total: None }, &format!("执行失败: {}", e)),
    }
}

/// 在专用连接上先切换数据库再分页执行 SQL（确保同一连接）
async fn execute_with_database(pool: &DbPool, database: &str, sql: &str, limit: usize, offset: u64) -> Result<SqlResult> {
    let is_query = is_select_query(sql);
    match pool {
        DbPool::MySQL(p) => {
            let mut conn = p.acquire().await?;
            // 在同一连接上先 USE 再执行 SQL（PoolConnection Deref 到底层连接）
            let use_sql = format!("USE `{}`", database);
            Executor::execute(&mut *conn, use_sql.as_str()).await?;
            if is_query {
                let (exec_sql, max_rows, wrapped) = paged_exec_parts(sql, limit, offset);
                // 总行数统计：仅首页且纯 SELECT 可包装时执行（best-effort，失败不影响主查询）
                let total = count_total_mysql(&mut *conn, sql, wrapped, offset).await;
                let (columns, mut rows) = fetch_mysql_rows(&mut *conn, &exec_sql, max_rows).await?;
                let has_more = wrapped && rows.len() > limit;
                if rows.len() > limit { rows.truncate(limit); }
                Ok(SqlResult { columns, rows, affected_rows: 0, execution_time_ms: 0, is_query: true, has_more, total })
            } else {
                let result = sqlx::query(sql).execute(&mut *conn).await?;
                Ok(SqlResult { columns: vec![], rows: vec![], affected_rows: result.rows_affected(), execution_time_ms: 0, is_query: false, has_more: false, total: None })
            }
        }
        DbPool::PostgreSQL(p) => {
            let mut conn = p.acquire().await?;
            let use_sql = format!("SET search_path TO \"{}\"", database);
            Executor::execute(&mut *conn, use_sql.as_str()).await?;
            if is_query {
                let (exec_sql, max_rows, wrapped) = paged_exec_parts(sql, limit, offset);
                // 总行数统计：仅首页且纯 SELECT 可包装时执行（best-effort，失败不影响主查询）
                let total = count_total_pg(&mut *conn, sql, wrapped, offset).await;
                let (columns, mut rows) = fetch_pg_rows(&mut *conn, &exec_sql, max_rows).await?;
                let has_more = wrapped && rows.len() > limit;
                if rows.len() > limit { rows.truncate(limit); }
                Ok(SqlResult { columns, rows, affected_rows: 0, execution_time_ms: 0, is_query: true, has_more, total })
            } else {
                let result = sqlx::query(sql).execute(&mut *conn).await?;
                Ok(SqlResult { columns: vec![], rows: vec![], affected_rows: result.rows_affected(), execution_time_ms: 0, is_query: false, has_more: false, total: None })
            }
        }
        DbPool::SqlServer(p) => {
            let setup = dialect::Dialect::SqlServer.use_database_sql(database);
            execute_sql_paged_generic(p, Dialect::SqlServer, &setup, sql, limit, offset).await
        }
        DbPool::Oracle(p) => {
            let setup = dialect::Dialect::Oracle.use_database_sql(database);
            execute_sql_paged_generic(p, Dialect::Oracle, &setup, sql, limit, offset).await
        }
    }
}

/// 构造分页执行要素：纯 SELECT 包装为子查询 + LIMIT/OFFSET（多读 1 行用于探测下一页）；
/// SHOW/DESCRIBE/EXPLAIN 等无法包装的语句直接限行读取（不支持翻页）。
/// 返回 (实际执行 SQL, 最大读取行数, 是否已包装分页)
fn paged_exec_parts(sql: &str, limit: usize, offset: u64) -> (String, usize, bool) {
    if let Some(wrapped) = wrap_paged_sql(sql, limit + 1, offset) {
        (wrapped, limit + 1, true)
    } else {
        (sql.to_string(), limit, false)
    }
}

/// 纯 SELECT 语句包装为分页子查询（去掉尾部分号，避免破坏子查询语法）
fn wrap_paged_sql(sql: &str, limit: usize, offset: u64) -> Option<String> {
    // 去掉前导注释与空白后，仅纯 SELECT 可安全作为子查询包装（CTE 以 WITH 开头不包装）
    let stripped = strip_leading_comments(sql);
    let upper = stripped.to_uppercase();
    if !upper.starts_with("SELECT") {
        return None;
    }
    let trimmed = stripped.trim_end_matches(';').trim_end();
    Some(format!("SELECT * FROM ( {} ) AS __bdt_page LIMIT {} OFFSET {}", trimmed, limit, offset))
}

/// 构造 COUNT 统计 SQL：仅纯 SELECT 可安全包装为子查询统计总数（判定与分页包装一致）
fn wrap_count_sql(sql: &str) -> Option<String> {
    let stripped = strip_leading_comments(sql);
    let upper = stripped.to_uppercase();
    if !upper.starts_with("SELECT") {
        return None;
    }
    let trimmed = stripped.trim_end_matches(';').trim_end();
    Some(format!("SELECT COUNT(*) AS __bdt_cnt FROM ( {} ) AS __bdt_cnt_sub", trimmed))
}

/// MySQL 总行数统计：仅首页（offset==0）且可包装时执行 COUNT 子查询（best-effort，失败返回 None）
async fn count_total_mysql<'c, E>(executor: E, sql: &str, wrapped: bool, offset: u64) -> Option<u64>
where
    E: sqlx::Executor<'c, Database = sqlx::MySql>,
{
    if !wrapped || offset != 0 { return None; }
    let cnt_sql = wrap_count_sql(sql)?;
    count_rows_mysql(executor, &cnt_sql).await.ok().flatten()
}

/// PostgreSQL 总行数统计：语义同 MySQL 版本
async fn count_total_pg<'c, E>(executor: E, sql: &str, wrapped: bool, offset: u64) -> Option<u64>
where
    E: sqlx::Executor<'c, Database = sqlx::Postgres>,
{
    if !wrapped || offset != 0 { return None; }
    let cnt_sql = wrap_count_sql(sql)?;
    count_rows_pg(executor, &cnt_sql).await.ok().flatten()
}

/// 执行 MySQL COUNT 查询取第一列数值
async fn count_rows_mysql<'c, E>(executor: E, sql: &str) -> Result<Option<u64>>
where
    E: sqlx::Executor<'c, Database = sqlx::MySql>,
{
    let row = sqlx::query(sql).fetch_one(executor).await?;
    let n: i64 = row.try_get(0)?;
    Ok(Some(n.max(0) as u64))
}

/// 执行 PostgreSQL COUNT 查询取第一列数值
async fn count_rows_pg<'c, E>(executor: E, sql: &str) -> Result<Option<u64>>
where
    E: sqlx::Executor<'c, Database = sqlx::Postgres>,
{
    let row = sqlx::query(sql).fetch_one(executor).await?;
    let n: i64 = row.try_get(0)?;
    Ok(Some(n.max(0) as u64))
}

/// 流式读取 MySQL 查询，最多取 max_rows 行。
/// 未读完时丢弃流（当前连接被丢弃，避免残余数据污染池中连接）。
pub(crate) async fn fetch_mysql_rows<'c, E>(executor: E, sql: &str, max_rows: usize) -> Result<(Vec<String>, Vec<Vec<serde_json::Value>>)>
where
    E: sqlx::Executor<'c, Database = sqlx::MySql>,
{
    let mut stream = sqlx::query(sql).fetch(executor);
    let mut columns: Vec<String> = Vec::new();
    let mut data_rows: Vec<Vec<serde_json::Value>> = Vec::new();
    while let Some(row) = stream.try_next().await? {
        if columns.is_empty() {
            columns = row.columns().iter().map(|c| c.name().to_string()).collect();
        }
        let mut values = Vec::with_capacity(columns.len());
        for i in 0..columns.len() {
            values.push(mysql_cell_to_value(&row, i));
        }
        data_rows.push(values);
        if data_rows.len() >= max_rows {
            // 已达读取上限：不再读取剩余行，流被 drop 后连接直接丢弃（不入池复用）
            break;
        }
    }
    drop(stream);
    Ok((columns, data_rows))
}

/// 流式读取 PostgreSQL 查询，最多取 max_rows 行（语义同 MySQL 版本）
pub(crate) async fn fetch_pg_rows<'c, E>(executor: E, sql: &str, max_rows: usize) -> Result<(Vec<String>, Vec<Vec<serde_json::Value>>)>
where
    E: sqlx::Executor<'c, Database = sqlx::Postgres>,
{
    let mut stream = sqlx::query(sql).fetch(executor);
    let mut columns: Vec<String> = Vec::new();
    let mut data_rows: Vec<Vec<serde_json::Value>> = Vec::new();
    while let Some(row) = stream.try_next().await? {
        if columns.is_empty() {
            columns = row.columns().iter().map(|c| c.name().to_string()).collect();
        }
        let mut values = Vec::with_capacity(columns.len());
        for i in 0..columns.len() {
            values.push(pg_cell_to_value(&row, i));
        }
        data_rows.push(values);
        if data_rows.len() >= max_rows {
            break;
        }
    }
    drop(stream);
    Ok((columns, data_rows))
}

/// 执行 SQL（分页版，用户查询入口）
pub async fn execute_sql_paged(pool: &DbPool, sql: &str, limit: usize, offset: u64) -> Result<SqlResult> {
    let is_query = is_select_query(sql);
    match pool {
        DbPool::MySQL(p) => {
            if !is_query {
                let result = sqlx::query(sql).execute(p).await?;
                return Ok(SqlResult { columns: vec![], rows: vec![], affected_rows: result.rows_affected(), execution_time_ms: 0, is_query: false, has_more: false, total: None });
            }
            let (exec_sql, max_rows, wrapped) = paged_exec_parts(sql, limit, offset);
            let total = count_total_mysql(p, sql, wrapped, offset).await;
            let (columns, mut rows) = fetch_mysql_rows(p, &exec_sql, max_rows).await?;
            let has_more = wrapped && rows.len() > limit;
            if rows.len() > limit { rows.truncate(limit); }
            Ok(SqlResult { columns, rows, affected_rows: 0, execution_time_ms: 0, is_query: true, has_more, total })
        }
        DbPool::PostgreSQL(p) => {
            if !is_query {
                let result = sqlx::query(sql).execute(p).await?;
                return Ok(SqlResult { columns: vec![], rows: vec![], affected_rows: result.rows_affected(), execution_time_ms: 0, is_query: false, has_more: false, total: None });
            }
            let (exec_sql, max_rows, wrapped) = paged_exec_parts(sql, limit, offset);
            let total = count_total_pg(p, sql, wrapped, offset).await;
            let (columns, mut rows) = fetch_pg_rows(p, &exec_sql, max_rows).await?;
            let has_more = wrapped && rows.len() > limit;
            if rows.len() > limit { rows.truncate(limit); }
            Ok(SqlResult { columns, rows, affected_rows: 0, execution_time_ms: 0, is_query: true, has_more, total })
        }
        DbPool::SqlServer(p) => {
            execute_sql_paged_generic(p, Dialect::SqlServer, "", sql, limit, offset).await
        }
        DbPool::Oracle(p) => {
            execute_sql_paged_generic(p, Dialect::Oracle, "", sql, limit, offset).await
        }
    }
}

/// 执行 SQL（内部调用：不做分页包装，仅 1000 行安全上限）
pub async fn execute_sql(pool: &DbPool, sql: &str) -> Result<SqlResult> {
    execute_sql_paged(pool, sql, MAX_PAGE_SIZE, 0).await
}

/// SQL Server / Oracle 的通用分页执行：两者后端都暴露了
/// `query_rows(setup, sql, max_rows)` / `execute(setup, sql)` / `query_count(setup, sql)`，
/// 仅分页包装语法随方言不同（由 `Dialect::wrap_paged_sql` 提供）。
/// 抽成泛型助手避免两份几乎相同的分支逻辑。
async fn execute_sql_paged_generic<P>(
    pool: &P,
    dialect: Dialect,
    setup: &str,
    sql: &str,
    limit: usize,
    offset: u64,
) -> Result<SqlResult>
where
    P: SqlServerOrOracle,
{
    let is_query = is_select_query(sql);
    if !is_query {
        let affected = pool.execute(setup, sql).await?;
        return Ok(SqlResult { columns: vec![], rows: vec![], affected_rows: affected, execution_time_ms: 0, is_query: false, has_more: false, total: None });
    }
    // 纯 SELECT 包装分页；SHOW/DESCRIBE/EXPLAIN 无法包装则直接限行读取
    let (exec_sql, max_rows, wrapped) = if dialect::is_wrappable_select(sql) {
        let body = dialect::strip_leading_comments(sql).trim_end_matches(';').trim_end();
        (dialect.wrap_paged_sql(body, limit + 1, offset), limit + 1, true)
    } else {
        (sql.to_string(), limit, false)
    };
    // 总行数统计：仅首页且可包装时执行（best-effort）
    let total = if wrapped && offset == 0 {
        let body = dialect::strip_leading_comments(sql).trim_end_matches(';').trim_end();
        pool.query_count(setup, &dialect.wrap_count_sql(body)).await.ok()
    } else {
        None
    };
    let (columns, mut rows) = pool.query_rows(setup, &exec_sql, max_rows).await?;
    let has_more = wrapped && rows.len() > limit;
    if rows.len() > limit { rows.truncate(limit); }
    Ok(SqlResult { columns, rows, affected_rows: 0, execution_time_ms: 0, is_query: true, has_more, total })
}

/// 分页查询表数据
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

/// 将结果行单元格转为 JSON 值。
/// try_get::<String> 无法解码整型列（MySQL 二进制协议下会静默失败变 null），
/// 因此按原生类型依次尝试：i64 → u64（unsigned）→ f64 → bool → String（日期/文本），
/// 全部失败且非 NULL 时返回占位文案，避免把解码失败伪装成空值。
pub(crate) fn mysql_cell_to_value(row: &sqlx::mysql::MySqlRow, i: usize) -> serde_json::Value {
    if let Ok(Some(v)) = row.try_get::<Option<i64>, _>(i) {
        return serde_json::Value::Number(v.into());
    }
    if let Ok(Some(v)) = row.try_get::<Option<u64>, _>(i) {
        return serde_json::Value::Number(v.into());
    }
    if let Ok(Some(v)) = row.try_get::<Option<f64>, _>(i) {
        if let Some(n) = serde_json::Number::from_f64(v) {
            return serde_json::Value::Number(n);
        }
    }
    if let Ok(Some(v)) = row.try_get::<Option<bool>, _>(i) {
        return serde_json::Value::Bool(v);
    }
    if let Ok(Some(v)) = row.try_get::<Option<String>, _>(i) {
        return serde_json::Value::String(v);
    }
    // 区分真正的 NULL 与解码失败，避免把解码失败伪装成空值
    match row.try_get_raw(i) {
        Ok(raw) if raw.is_null() => serde_json::Value::Null,
        _ => serde_json::Value::String("[类型转换失败]".to_string()),
    }
}

/// PostgreSQL 版本：Pg 的整数均带符号，u64 分支仅作兼容保留
pub(crate) fn pg_cell_to_value(row: &sqlx::postgres::PgRow, i: usize) -> serde_json::Value {
    if let Ok(Some(v)) = row.try_get::<Option<i64>, _>(i) {
        return serde_json::Value::Number(v.into());
    }
    if let Ok(Some(v)) = row.try_get::<Option<f64>, _>(i) {
        if let Some(n) = serde_json::Number::from_f64(v) {
            return serde_json::Value::Number(n);
        }
    }
    if let Ok(Some(v)) = row.try_get::<Option<bool>, _>(i) {
        return serde_json::Value::Bool(v);
    }
    if let Ok(Some(v)) = row.try_get::<Option<String>, _>(i) {
        return serde_json::Value::String(v);
    }
    // 区分真正的 NULL 与解码失败，避免把解码失败伪装成空值
    match row.try_get_raw(i) {
        Ok(raw) if raw.is_null() => serde_json::Value::Null,
        _ => serde_json::Value::String("[类型转换失败]".to_string()),
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
        DbPool::SqlServer(p) => {
            SqlServerPool::query_count(p, sql).await
        }
        DbPool::Oracle(p) => {
            OraclePool::query_count(p, "", sql).await
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

/// SQL Server / Oracle 后端共有的「带前置切库语句」执行接口。
/// SqlServerPool 的 TDS 协议建连时已绑定库，无需前置语句，故忽略 setup；
/// OraclePool 通过 ODBC 每次操作即开即关，需要 setup（ALTER SESSION）先切 schema。
trait SqlServerOrOracle {
    async fn query_rows(&self, setup: &str, sql: &str, max_rows: usize) -> Result<(Vec<String>, Vec<Vec<serde_json::Value>>)>;
    async fn execute(&self, setup: &str, sql: &str) -> Result<u64>;
    async fn query_count(&self, setup: &str, sql: &str) -> Result<u64>;
}

impl SqlServerOrOracle for Arc<SqlServerPool> {
    async fn query_rows(&self, _setup: &str, sql: &str, max_rows: usize) -> Result<(Vec<String>, Vec<Vec<serde_json::Value>>)> {
        self.as_ref().query_rows(sql, max_rows).await
    }
    async fn execute(&self, _setup: &str, sql: &str) -> Result<u64> {
        self.as_ref().execute(sql).await
    }
    async fn query_count(&self, _setup: &str, sql: &str) -> Result<u64> {
        self.as_ref().query_count(sql).await
    }
}

impl SqlServerOrOracle for Arc<OraclePool> {
    async fn query_rows(&self, setup: &str, sql: &str, max_rows: usize) -> Result<(Vec<String>, Vec<Vec<serde_json::Value>>)> {
        self.as_ref().query_rows(setup, sql, max_rows).await
    }
    async fn execute(&self, setup: &str, sql: &str) -> Result<u64> {
        self.as_ref().execute(setup, sql).await
    }
    async fn query_count(&self, setup: &str, sql: &str) -> Result<u64> {
        self.as_ref().query_count(setup, sql).await
    }
}
