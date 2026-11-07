/*! # 数据库操作模块
SQL 执行、表数据增删改查。
*/
use bdevtools_core::response::response::Response;
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
/// 内部调用（`execute_sql`，只跑内部辅助语句）的行数上限。
///
/// **不是对外的硬上限**：用户发起的查询一次拉多少行由前端指定（`db_execute_sql` 的
/// `page_size`），这里刻意不 clamp。表格已换成虚拟滚动，行数不再决定渲染开销，
/// 再靠后端偷偷裁 ±1000 行只会造成「填了 5000 却只回来 1000」的假象。
pub const INTERNAL_ROW_LIMIT: usize = 1000;

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
///
/// 后 6 个字段是**结果网格编辑**需要的元数据（列序号让结果列与表列能按位置对齐，
/// 自增 / 生成列决定能不能写入，精度标度与字符长度用于前端校验与选编辑器类型）。
/// 各 catalog 查不到时按「未知」处理（序号 0、长度 None）——宁可少一次校验，
/// 也不要因为元数据缺失把可写的列判成只读。
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ColumnInfo {
    pub name: String,
    pub data_type: String,
    pub is_nullable: bool,
    pub is_primary_key: bool,
    pub default_value: Option<String>,
    pub comment: Option<String>,
    /// 列在表中的序号（1 起）；0 = 未知
    #[serde(default)]
    pub ordinal_position: i32,
    /// 自增 / identity 列（写入通常没有意义）
    #[serde(default)]
    pub is_auto_increment: bool,
    /// 计算列 / 虚拟列（不可写）
    #[serde(default)]
    pub is_generated: bool,
    #[serde(default)]
    pub numeric_precision: Option<i32>,
    #[serde(default)]
    pub numeric_scale: Option<i32>,
    #[serde(default)]
    pub character_max_length: Option<i64>,
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

    // 前端指定行数以前端为准，**不设上限**：一次拉多少行由使用者决定（表格已有虚拟滚动）。
    // 只保留下界 1 —— LIMIT 0 在 MySQL / PG 里是空结果集，不是「不限制」。
    // 上界 `MAX-1` 纯粹是给后面 `limit + 1`（多取一行判断是否还有下一页）留出余量，不是行数限制。
    let limit = page_size
        .map(|v| v as usize)
        .unwrap_or(DEFAULT_PAGE_SIZE)
        .clamp(1, usize::MAX - 1);
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
    execute_sql_paged(pool, sql, INTERNAL_ROW_LIMIT, 0).await
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

    // 这里刻意走 `execute_sql_paged` 而不是 `execute_sql`：SQL 自带 LIMIT/OFFSET 已完成分页，
    // 再经内部接口会被二次包装成 1000 行上限（等于把 caller 指定的 page_size 悄悄改小）。
    let limit = (page_size as usize).clamp(1, usize::MAX - 1);
    match execute_sql_paged(&pool, &data_sql, limit, 0).await {
        Ok(res) => {
            let columns = res.columns.iter().map(|c| ColumnInfo {
                name: c.clone(),
                data_type: String::new(),
                is_nullable: true,
                is_primary_key: false,
                default_value: None,
                comment: None,
                ordinal_position: 0,
                is_auto_increment: false,
                is_generated: false,
                numeric_precision: None,
                numeric_scale: None,
                character_max_length: None,
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

// ==================== 内部辅助函数 ====================

// ===================================================================
// 单元格：时间/日期/JSON 的序列化格式
//
// **统一约定：时间值与 JSON 一律序列化成「字符串」**，不落成 JSON 的结构化值。
// 表格是二维标量结构 —— 单元格要能被排序、过滤、复制、导出成 CSV/Excel；
// 一旦某个格变成对象/数组，这些能力全线退化。DECIMAL 同理（见下方 try_get 分支）。
// 小数秒统一用 `%.f`：整秒时不输出多余的 `.000`，非零时才带。
// ===================================================================

/// DATE → `2026-10-09`
fn fmt_date(d: &chrono::NaiveDate) -> String {
    d.format("%Y-%m-%d").to_string()
}

/// DATETIME → `2026-10-09 08:27:32`（小数秒非零时补 `.123`）
fn fmt_datetime(d: &chrono::NaiveDateTime) -> String {
    d.format("%Y-%m-%d %H:%M:%S%.f").to_string()
}

/// MySQL TIMESTAMP → 与 DATETIME 同格式，**刻意不带时区后缀**。
///
/// 为什么不能标 `+00:00`：sqlx 的 `impl Decode for DateTime<Utc>` 源码注释写着
/// `Note: assumes the connection's time_zone is set to +00:00 (UTC)`，实现是
/// `Utc.from_utc_datetime(&naive)` —— **它不做任何时区换算**，只是把服务端返回的
/// 墙钟值贴上 UTC 标签。而 MySQL 返回 TIMESTAMP 时是按**会话时区**渲染的；会话时区
/// 是 +08:00 时这里拿到的其实是本地墙钟时间，再标 UTC 就成**撒谎**（误导 8 小时）。
/// 不带后缀 = 展示服务端给的墙钟时间，与 mysql CLI 及其它 GUI 看到的一致。
fn fmt_mysql_timestamp(t: &chrono::DateTime<chrono::Utc>) -> String {
    t.format("%Y-%m-%d %H:%M:%S%.f").to_string()
}

/// PostgreSQL TIMESTAMPTZ → `2026-10-09 08:27:32+00:00`。
///
/// 与 MySQL 恰恰相反，这里**必须带偏移**：Pg 二进制协议传的是「距 epoch 的 UTC 微秒」，
/// sqlx 解出的 `DateTime<Utc>` 是真·UTC 时刻，把偏移标出来才是准确的。
fn fmt_pg_timestamptz(t: &chrono::DateTime<chrono::Utc>) -> String {
    t.format("%Y-%m-%d %H:%M:%S%.f%:z").to_string()
}

/// MySQL TIME → `-838:59:59` ~ `838:59:59`。
///
/// MySQL 的 TIME 是「时长」不是「时刻」：可为负、可超 24h，所以要用 `TimeDelta`
/// 兜住全范围（`NaiveTime` 遇到越界值会直接报错，源码注释里写明了）。
/// PostgreSQL 的 TIME 是纯时刻，没有这个分支。
fn fmt_time_delta(d: chrono::TimeDelta) -> String {
    let neg = d < chrono::TimeDelta::zero();
    let total = d.num_seconds().abs();
    let h = total / 3600;
    let m = (total % 3600) / 60;
    let s = total % 60;
    let us = d.subsec_micros().abs();
    let sign = if neg { "-" } else { "" };
    if us > 0 {
        format!("{}{:02}:{:02}:{:02}.{:06}", sign, h, m, s, us)
    } else {
        format!("{}{:02}:{:02}:{:02}", sign, h, m, s)
    }
}

/// 将结果行单元格转为 JSON 值。
/// try_get::<String> 无法解码整型列（MySQL 二进制协议下会静默失败变 null），
/// 因此按原生类型依次尝试：i64 → u64（unsigned）→ f64 → bool → DECIMAL → 时间 → String（文本）。
///
/// **DECIMAL 为什么必须单独接**：`Type<MySql> for f64` 的 compatible 白名单**显式排除了
/// DECIMAL**（sqlx 源码注释原文：floating-point numbers have different semantics），
/// i64 / u64 / String 的白名单同样不含它 ⇒ 整列会一路落到兜底显示「类型转换失败」。
/// 而把它塞进 f64 又会静默丢精度 —— 那正是 sqlx 想避免的。这里走 `rust_decimal` 并按
/// **字符串**输出：标度与有效数字全部保留（`12.30` 不会变成 `12.3`）。
pub(crate) fn mysql_cell_to_value(row: &sqlx::mysql::MySqlRow, i: usize) -> serde_json::Value {
    // 用 crate `chrono` 本身，不用 `sqlx::types::chrono` —— 后者是挑选式 re-export，不含 TimeDelta。
    use chrono::{DateTime, NaiveDate, NaiveDateTime, NaiveTime, TimeDelta, Utc};
    use sqlx::types::{Json, Uuid};

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
    // DECIMAL / NUMERIC（原因见函数头注释）
    if let Ok(Some(v)) = row.try_get::<Option<sqlx::types::Decimal>, _>(i) {
        return serde_json::Value::String(v.to_string());
    }
    if let Ok(Some(v)) = row.try_get::<Option<bool>, _>(i) {
        return serde_json::Value::Bool(v);
    }
    if let Ok(Some(v)) = row.try_get::<Option<String>, _>(i) {
        return serde_json::Value::String(v);
    }

    // ---- 时间日期（需要 sqlx 的 chrono 特性） ----
    // MySQL 日期列的 charsetnr 是 binary(63)，不在 `str` 的 compatible 白名单里
    // （str.rs 要求 `!flags.contains(ColumnFlags::BINARY)`），所以**必须显式接**，
    // 否则整列退化成「类型转换失败」。
    if let Ok(Some(v)) = row.try_get::<Option<NaiveDate>, _>(i) {
        return serde_json::Value::String(fmt_date(&v));
    }
    // DATETIME：compatible 只匹配 ColumnType::Datetime（不含 Timestamp）
    if let Ok(Some(v)) = row.try_get::<Option<NaiveDateTime>, _>(i) {
        return serde_json::Value::String(fmt_datetime(&v));
    }
    // TIME 两段：NaiveTime 覆盖常见的 00:00:00~23:59:59，越界（负 / 超 24h）会报错，
    // 交给 TimeDelta 兜住 MySQL 的完整区间。
    if let Ok(Some(v)) = row.try_get::<Option<NaiveTime>, _>(i) {
        return serde_json::Value::String(v.format("%H:%M:%S%.f").to_string());
    }
    if let Ok(Some(v)) = row.try_get::<Option<TimeDelta>, _>(i) {
        return serde_json::Value::String(fmt_time_delta(v));
    }
    // TIMESTAMP：必须排在 NaiveDateTime **之后** —— `DateTime<Utc>` 的 compatible 同时
    // 匹配 Datetime | Timestamp，放前面会把 DATETIME 列也一并截胡。
    if let Ok(Some(v)) = row.try_get::<Option<DateTime<Utc>>, _>(i) {
        return serde_json::Value::String(fmt_mysql_timestamp(&v));
    }

    // ---- JSON（需要 sqlx 的 json 特性） ----
    // 必须排在 String 之后：`Json<T>` 的 compatible 是 `ColumnType::Json || str兼容 || bytes兼容`，
    // 放到 String 前面时，VARCHAR 列里恰好存着合法 JSON 文本（`123`、`"abc"`）会被反序列
    // 化成数字/字符串，与普通文本的展示不一致。而 MySQL 的 `ColumnType::Json` **不在**
    // str / [u8] 的白名单里 ⇒ 它不会被 String 分支截胡，排在这后面照样命中。
    if let Ok(Some(v)) = row.try_get::<Option<Json<serde_json::Value>>, _>(i) {
        return serde_json::Value::String(v.0.to_string());
    }

    // ---- UUID（需要 sqlx 的 uuid 特性） ----
    // 必须**最后**：`Uuid` 的 compatible 委托给 `&[u8]`，会把 VarChar / Blob / String / Enum
    // 统统判为兼容（只要值正好 16 字节就解成一个 Uuid）。排在所有明确类型之后，
    // 才能保证只有「没有别的类型接得住」的 BINARY(16) 才走到这里。
    if let Ok(Some(v)) = row.try_get::<Option<Uuid>, _>(i) {
        return serde_json::Value::String(v.to_string());
    }

    // 区分真正的 NULL 与解码失败，避免把解码失败伪装成空值。
    // 已知的不可解情形：MySQL 的「零值日期」`0000-00-00` —— chrono 表示不了，
    // MySQL 侧拿不到原始字节（MySqlValueRef::as_bytes 是 pub(crate)），无法像 Pg 那样文本兜底。
    match row.try_get_raw(i) {
        Ok(raw) if raw.is_null() => serde_json::Value::Null,
        _ => serde_json::Value::String("[类型转换失败]".to_string()),
    }
}

/// PostgreSQL 版本：Pg 的整数均带符号，u64 分支仅作兼容保留。
///
/// 相比 MySQL 多一层兜底：Pg 的 `PgValueRef::as_bytes()` 是公开的，能把 UUID / JSON /
/// INTERVAL / 时间等未开 feature 的类型也读出来。**MySQL 做不到这件事** —— 它的
/// `MySqlValueRef::as_bytes()` 是 `pub(crate)`，owned 类型也没有公开访问入口，
/// 那里只能逐个类型开 sqlx feature。
pub(crate) fn pg_cell_to_value(row: &sqlx::postgres::PgRow, i: usize) -> serde_json::Value {
    use chrono::{DateTime, NaiveDate, NaiveDateTime, NaiveTime, Utc};
    use sqlx::types::{Json, Uuid};

    if let Ok(Some(v)) = row.try_get::<Option<i64>, _>(i) {
        return serde_json::Value::Number(v.into());
    }
    if let Ok(Some(v)) = row.try_get::<Option<f64>, _>(i) {
        if let Some(n) = serde_json::Number::from_f64(v) {
            return serde_json::Value::Number(n);
        }
    }
    // DECIMAL / NUMERIC（原因见 mysql_cell_to_value 注释）
    if let Ok(Some(v)) = row.try_get::<Option<sqlx::types::Decimal>, _>(i) {
        return serde_json::Value::String(v.to_string());
    }
    if let Ok(Some(v)) = row.try_get::<Option<bool>, _>(i) {
        return serde_json::Value::Bool(v);
    }
    if let Ok(Some(v)) = row.try_get::<Option<String>, _>(i) {
        return serde_json::Value::String(v);
    }

    // ---- 时间日期（需要 sqlx 的 chrono 特性） ----
    // Pg 的各类型按 OID 精确匹配（NaiveDate→DATE / NaiveTime→TIME / NaiveDateTime→TIMESTAMP /
    // DateTime<Tz>→TIMESTAMPTZ），不像 MySQL 的 TIMESTAMP 那样存在时区假设问题。
    if let Ok(Some(v)) = row.try_get::<Option<NaiveDate>, _>(i) {
        return serde_json::Value::String(fmt_date(&v));
    }
    // Pg 的 TIME 是纯时刻（MySQL 的 TIME 是时长），NaiveTime 足够
    if let Ok(Some(v)) = row.try_get::<Option<NaiveTime>, _>(i) {
        return serde_json::Value::String(v.format("%H:%M:%S%.f").to_string());
    }
    if let Ok(Some(v)) = row.try_get::<Option<NaiveDateTime>, _>(i) {
        return serde_json::Value::String(fmt_datetime(&v));
    }
    if let Ok(Some(v)) = row.try_get::<Option<DateTime<Utc>>, _>(i) {
        return serde_json::Value::String(fmt_pg_timestamptz(&v));
    }

    // ---- JSON / JSONB（需要 sqlx 的 json 特性） ----
    // compatible 只匹配 JSON | JSONB，不掺 str / bytes 的宽松判定，比 MySQL 那边精准得多。
    // 仍然按全局约定输出字符串（理由见本文件顶部「统一约定」）：
    // 保持单元格是标量，过滤/排序/复制/导出才不会退化。
    if let Ok(Some(v)) = row.try_get::<Option<Json<serde_json::Value>>, _>(i) {
        return serde_json::Value::String(v.0.to_string());
    }

    // ---- UUID（需要 sqlx 的 uuid 特性） ----
    if let Ok(Some(v)) = row.try_get::<Option<Uuid>, _>(i) {
        return serde_json::Value::String(v.to_string());
    }

    match row.try_get_raw(i) {
        Ok(raw) if raw.is_null() => serde_json::Value::Null,
        // 只有**文本格式**能按 UTF-8 读：simple query（sqlx::query 无参数）返回的就是文本格式。
        // 二进制格式下原始字节是二进制编码（不是值的文本形态），读出来会是一串乱码 ⇒ 不兜底。
        Ok(raw) if matches!(raw.format(), sqlx::postgres::PgValueFormat::Text) => {
            match std::str::from_utf8(raw.as_bytes().unwrap_or(&[])) {
                Ok(s) => serde_json::Value::String(s.to_string()),
                Err(_) => serde_json::Value::String("[类型转换失败]".to_string()),
            }
        }
        // 区分真正的 NULL 与解码失败，避免把解码失败伪装成空值
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

// 这里原先有个 `format_value`（只把 ' 翻倍、按 1/0 输出布尔），现已删除：
// 写库面讲过的两个坑它都踩了 —— ① MySQL 没转义 `\`；② PG 的布尔字面量是 TRUE/FALSE。
// 现在一律走 `Dialect::literal`（`dialect.rs`），字面量规则只有一个出处。

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

#[cfg(test)]
mod decimal_tests {
    /// 锁住 DECIMAL 的输出契约：走 `rust_decimal` 转字符串，**保留标度与全部有效数字**。
    ///
    /// 单元格值最后落地的是 `Decimal::to_string()`；若哪天改成经 f64 中转，
    /// 这里会先炸 —— 尾部零被吃掉（`12.30` → `12.3`）、大数丢精度，
    /// 而这两点正是 sqlx 刻意不让 f64 兼容 DECIMAL 的原因。
    #[test]
    fn decimal_display_keeps_scale_and_precision() {
        let cases = [
            ("12.30", "12.30"),            // 标度不许被吃掉
            ("0.00", "0.00"),              // 全零小数位同样保留
            ("-0.001", "-0.001"),          // 负号与小数位
            ("123456789012345.678901234", "123456789012345.678901234"), // 大数不丢精度、不转科学计数法
            ("0", "0"),                    // 整值无小数位时不要补 .0
        ];
        for (input, want) in cases {
            let d: sqlx::types::Decimal = input
                .parse()
                .unwrap_or_else(|e| panic!("DECIMAL 解析失败 {}: {}", input, e));
            assert_eq!(d.to_string(), want, "输入 {}", input);
            // 序列化后必须是带引号的字符串而不是数字：JSON number 会把值退化成 f64
            assert_eq!(
                serde_json::Value::String(d.to_string()),
                serde_json::to_value(d.to_string()).expect("字符串序列化不应失败")
            );
        }
    }
}

#[cfg(test)]
mod value_format_tests {
    use super::*;
    use chrono::{DateTime, NaiveDate, NaiveTime, TimeDelta, TimeZone, Utc};

    /// 时间日期的落地形态。
    ///
    /// 统一成字符串是为了保住「单元格是标量」这个前提（排序/过滤/复制/导出都依赖它）；
    /// 小数秒用 `%.f`：整秒时不拖一条没意义的 `.000`。
    #[test]
    fn temporal_values_serialize_as_scalar_strings() {
        let d = NaiveDate::from_ymd_opt(2026, 10, 9).expect("合法日期");
        assert_eq!(fmt_date(&d), "2026-10-09");

        // 整秒：不带小数部分
        let dt = d.and_hms_opt(8, 27, 32).expect("合法时刻");
        assert_eq!(fmt_datetime(&dt), "2026-10-09 08:27:32");

        // 有毫秒：保留，不做四舍五入到秒
        let dt_ms = d
            .and_hms_micro_opt(8, 27, 32, 123_000)
            .expect("合法时刻");
        assert_eq!(fmt_datetime(&dt_ms), "2026-10-09 08:27:32.123");

        assert_eq!(
            NaiveTime::from_hms_micro_opt(8, 27, 32, 123_000)
                .expect("合法时间")
                .format("%H:%M:%S%.f")
                .to_string(),
            "08:27:32.123"
        );
    }

    /// **MySQL TIMESTAMP 不能带时区后缀** —— 这是最容易被人"改正确"的一处，务必保留测试。
    ///
    /// sqlx 的 `Decode<Utc> for MySql` 只是 `Utc.from_utc_datetime(&naive)`，
    /// 它**不换算时区**，把服务端按会话时区返回的墙钟值原样贴上 UTC 标签。
    /// 所以这里的 DateTime<Utc> 里的 Utc 是**假的**，带 "+00:00" 就是撒谎；
    /// 不带后缀 = 显示服务端给的墙钟值，与 mysql CLI 一致。
    #[test]
    fn mysql_timestamp_has_no_timezone_suffix() {
        let naive = NaiveDate::from_ymd_opt(2026, 10, 9)
            .expect("合法日期")
            .and_hms_opt(8, 27, 32)
            .expect("合法时刻");
        let ts: DateTime<Utc> = Utc.from_utc_datetime(&naive);

        let s = fmt_mysql_timestamp(&ts);
        assert_eq!(s, "2026-10-09 08:27:32");
        assert!(!s.contains('+'), "MySQL TIMESTAMP 不得带时区后缀：{}", s);
        // 与同一时刻的 DATETIME 显示完全一致 —— 两者本来就没差别
        assert_eq!(s, fmt_datetime(&naive));
    }

    /// PG TIMESTAMPTZ 恰恰相反：它传的是真·UTC 微秒，必须把偏移标出来。
    #[test]
    fn pg_timestamptz_keeps_utc_offset() {
        let naive = NaiveDate::from_ymd_opt(2026, 10, 9)
            .expect("合法日期")
            .and_hms_opt(8, 27, 32)
            .expect("合法时刻");
        let tstz: DateTime<Utc> = Utc.from_utc_datetime(&naive);

        let s = fmt_pg_timestamptz(&tstz);
        assert_eq!(s, "2026-10-09 08:27:32+00:00");
    }

    /// MySQL TIME 是「时长」不是「时刻」：可为负、可超 24h（区间 -838:59:59 ~ 838:59:59）。
    /// NaiveTime 表示不了越界值，这部分交给 TimeDelta 兜住。
    #[test]
    fn mysql_time_delta_covers_full_range() {
        let h = |h: i64, m: i64, s: i64| TimeDelta::try_hours(0).expect("0") + TimeDelta::try_seconds(h * 3600 + m * 60 + s).expect("秒数合法");

        assert_eq!(fmt_time_delta(h(0, 0, 0)), "00:00:00");
        assert_eq!(fmt_time_delta(h(8, 27, 32)), "08:27:32");
        // 超过 24h —— NaiveTime 到不了的范围
        assert_eq!(fmt_time_delta(h(838, 59, 59)), "838:59:59");
        // 负值：MySQL TIME 允许负时长
        assert_eq!(fmt_time_delta(h(-838, -59, -59)), "-838:59:59");

        // 微秒
        let with_us = TimeDelta::try_seconds(1).expect("1s") + TimeDelta::microseconds(500);
        assert_eq!(fmt_time_delta(with_us), "00:00:01.000500");
    }

    /// JSON 列按「字符串」落地而不是嵌套对象 —— 表格的标量契约。
    #[test]
    fn json_value_lands_as_string() {
        let v: serde_json::Value =
            serde_json::from_str(r#"{"a":1,"b":["x","y"]}"#).expect("合法 JSON");
        let cell = serde_json::Value::String(v.to_string());

        // 必须是 JSON string，不是一个嵌套对象
        assert!(cell.is_string(), "JSON 单元格必须是标量字符串");
        // 紧凑形式（不带多余空格/换行），方便复制粘贴与导出
        assert_eq!(cell.as_str().expect("是字符串"), r#"{"a":1,"b":["x","y"]}"#);
    }
}
