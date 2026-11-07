/*! # Oracle（ODBC）连接与执行封装
基于 `odbc-api` 走系统 ODBC：本机不引入 OCI 原生库，
但要求用户机器已安装 Oracle ODBC 驱动（Instant Client / ODAC 均可）。

ODBC 接口是同步的，且连接句柄不能安全地在线程间共享，
因此这里采用「每次操作开一条连接 → 执行 → 关闭」的简单策略，
并把同步调用放进 `spawn_blocking`，避免阻塞 Tauri 的异步运行时。
*/
use anyhow::{anyhow, Result};
use odbc_api::buffers::{AnyColumnBufferSlice, BufferDesc, ColumnarDynBuffer};
use odbc_api::{ColumnDescription, ConnectionOptions, Cursor, Environment, Nullability, ResultSetMetadata};
use serde_json::Value;

use super::connection::DbConnectionConfig;

/// 单批最多缓冲的行数（列存缓冲，过大会浪费内存）
const MAX_BATCH: usize = 500;
/// 文本列缓冲的最大字符长度
const MAX_STR_LEN: usize = 4000;

/// 进程唯一的 ODBC 环境（odbc-api 要求一个进程只能有一个环境）。
/// `Environment` 是 `Send` 但非 `Sync`，故用 `Mutex` 包裹使其可跨线程共享；
/// 实际建连时在 `spawn_blocking` 内短暂加锁取 `&Environment`。
fn env() -> &'static parking_lot::Mutex<Environment> {
    use lazy_static::lazy_static;
    lazy_static! {
        static ref ENV: parking_lot::Mutex<Environment> =
            parking_lot::Mutex::new(Environment::new().expect("初始化 ODBC 环境失败"));
    }
    &ENV
}

/// Oracle 连接句柄容器（只保存连接串，实际连接按需创建）
pub struct OraclePool {
    conn_str: String,
}

impl OraclePool {
    pub fn new(config: &DbConnectionConfig) -> Result<Self> {
        Ok(Self {
            conn_str: build_connection_string(config)?,
        })
    }

    /// 执行查询，最多读取 max_rows 行；setup 为空表示不执行前置语句
    pub async fn query_rows(
        &self,
        setup: &str,
        sql: &str,
        max_rows: usize,
    ) -> Result<(Vec<String>, Vec<Vec<Value>>)> {
        let cs = self.conn_str.clone();
        let setup = setup.to_string();
        let sql = sql.to_string();
        tokio::task::spawn_blocking(move || query_rows_blocking(&cs, &setup, &sql, max_rows)).await?
    }

    /// 执行非查询语句，返回受影响行数
    pub async fn execute(&self, setup: &str, sql: &str) -> Result<u64> {
        let cs = self.conn_str.clone();
        let setup = setup.to_string();
        let sql = sql.to_string();
        tokio::task::spawn_blocking(move || execute_blocking(&cs, &setup, &sql)).await?
    }

    /// 在一个事务里顺序执行多条语句（共用**同一条 ODBC 连接**）。
    ///
    /// Oracle 侧默认是 autocommit，不开的话每条语句各自提交 —— 那就没有回滚可言了。
    /// 所以这里 `set_autocommit(false)` → 逐条执行 → commit / rollback，最后恢复 autocommit
    /// （连接虽然即用即关，但显式恢复可以避免将来改成池化时踩坑）。
    pub async fn execute_batch_transaction(
        &self,
        setup: &str,
        stmts: &[anyhow::Result<String>],
        expect_one: &[bool],
    ) -> Result<Vec<Result<u64>>> {
        let cs = self.conn_str.clone();
        let setup = setup.to_string();
        let expect_one = expect_one.to_vec();
        let stmts: Vec<anyhow::Result<String>> = stmts
            .iter()
            .map(|r| match r {
                Ok(s) => Ok(s.clone()),
                Err(e) => Err(anyhow!("{}", e)),
            })
            .collect();
        tokio::task::spawn_blocking(move || {
            execute_batch_blocking(&cs, &setup, stmts, &expect_one)
        })
        .await?
    }

    /// 查询单个数值（用于 COUNT 统计）
    pub async fn query_count(&self, setup: &str, sql: &str) -> Result<u64> {
        let (_, rows) = self.query_rows(setup, sql, 1).await?;
        let first = rows.into_iter().next().and_then(|r| r.into_iter().next());
        match first {
            Some(Value::Number(n)) => Ok(n.as_u64().unwrap_or(0)),
            Some(Value::String(s)) => Ok(s.parse::<u64>().unwrap_or(0)),
            _ => Ok(0),
        }
    }

    /// 无需释放资源（连接即用即关）
    pub async fn close(&self) -> Result<()> {
        Ok(())
    }

    /// 测试连通性：建连后执行一条最简查询
    pub async fn test_connection(config: &DbConnectionConfig) -> Result<()> {
        let cs = build_connection_string(config)?;
        tokio::task::spawn_blocking(move || {
            with_connection(&cs, |conn| {
                conn.execute("SELECT 1 FROM DUAL", (), None)?;
                Ok(())
            })
        })
        .await?
    }
}

/// 在进程唯一 Environment 的锁作用域内建立连接并执行闭包。
/// `Connection` 借用 `Environment`，故连接与操作必须在持锁期间完成；
/// 闭包返回后连接与锁一起释放（连接即用即关）。
fn with_connection<T>(conn_str: &str, f: impl FnOnce(&mut odbc_api::Connection) -> Result<T>) -> Result<T> {
    let guard = env().lock();
    let mut conn = guard.connect_with_connection_string(conn_str, ConnectionOptions::default())?;
    f(&mut conn)
}

/// 构建 Oracle 的 ODBC 连接串。
/// 若「数据库名」里填的是含 '=' 的完整连接串（DSN 或自定义属性），则原样使用；
/// 否则自动挑选本机已安装的 Oracle ODBC 驱动，并用 EZConnect 形式的 DBQ 拼接。
fn build_connection_string(config: &DbConnectionConfig) -> Result<String> {
    if let Some(db) = &config.database_name {
        let trimmed = db.trim();
        if trimmed.contains('=') {
            return Ok(trimmed.to_string());
        }
    }
    let driver = find_oracle_driver()?;
    let service = config.database_name.clone().unwrap_or_default();
    let dbq = if service.is_empty() {
        format!("{}:{}", config.host, config.port)
    } else {
        format!("{}:{}/{}", config.host, config.port, service)
    };
    // 密码里的 ; { } 等字符需要按 ODBC 规则转义
    let pwd = odbc_api::escape_attribute_value(&config.password);
    Ok(format!(
        "DRIVER={{{}}};DBQ={};UID={};PWD={};",
        driver, dbq, config.username, pwd
    ))
}

/// 从系统 ODBC 驱动列表里找出第一个 Oracle 驱动
fn find_oracle_driver() -> Result<String> {
    let drivers = env().lock().drivers()?;
    drivers
        .into_iter()
        .map(|d| d.description)
        .find(|name| name.to_lowercase().contains("oracle"))
        .ok_or_else(|| {
            anyhow!(
                "未检测到 Oracle ODBC 驱动。请安装 Oracle Instant Client 或 ODAC 后重试；\
                 若驱动名特殊，可在「数据库名」中直接填写完整 ODBC 连接串（例如 DSN=xxx;UID=...;PWD=...）"
            )
        })
}

fn query_rows_blocking(
    conn_str: &str,
    setup: &str,
    sql: &str,
    max_rows: usize,
) -> Result<(Vec<String>, Vec<Vec<Value>>)> {
    with_connection(conn_str, |conn| {
        if !setup.trim().is_empty() {
            conn.execute(setup, (), None)?;
        }
        let mut cursor = match conn.execute(sql, (), None)? {
            Some(c) => c,
            None => return Ok((Vec::new(), Vec::new())),
        };

        let ncols = cursor.num_result_cols()?.max(0) as usize;
        if ncols == 0 {
            return Ok((Vec::new(), Vec::new()));
        }

        let mut columns = Vec::with_capacity(ncols);
        let mut descs = Vec::with_capacity(ncols);
        for idx in 0..ncols {
            let mut cd = ColumnDescription::default();
            cursor.describe_col(idx as u16 + 1, &mut cd)?;
            columns.push(
                cd.name_to_string()
                    .unwrap_or_else(|_| format!("COL{}", idx + 1)),
            );
            let nullable = matches!(cd.nullability, Nullability::Unknown | Nullability::Nullable);
            descs.push(pick_buffer_desc(BufferDesc::from_data_type(cd.data_type, nullable)));
        }

        let capacity = max_rows.clamp(1, MAX_BATCH);
        let buffer = ColumnarDynBuffer::from_descs(capacity, descs);
        let mut block = cursor.bind_buffer(buffer)?;

        let mut rows: Vec<Vec<Value>> = Vec::new();
        while let Some(batch) = block.fetch()? {
            let batch_rows = batch.num_rows();
            for r in 0..batch_rows {
                if rows.len() >= max_rows {
                    break;
                }
                let mut values = Vec::with_capacity(ncols);
                for c in 0..ncols {
                    values.push(cell_value(batch.column(c), r));
                }
                rows.push(values);
            }
            if rows.len() >= max_rows {
                break;
            }
        }
        Ok((columns, rows))
    })
}

fn execute_batch_blocking(
    conn_str: &str,
    setup: &str,
    stmts: Vec<anyhow::Result<String>>,
    expect_one: &[bool],
) -> Result<Vec<Result<u64>>> {
    with_connection(conn_str, |conn| {
        if !setup.trim().is_empty() {
            conn.execute(setup, (), None)?;
        }
        conn.set_autocommit(false)?;
        let mut out: Vec<Result<u64>> = Vec::with_capacity(stmts.len());
        let mut rolled_back = false;
        for (i, item) in stmts.into_iter().enumerate() {
            match item {
                Err(e) => {
                    out.push(Err(e));
                    rolled_back = true;
                    break;
                }
                Ok(sql) => {
                    match conn.prepare(sql.as_str()) {
                        Ok(mut prepared) => {
                            // execute 返回的游标借住了 prepared，必须先把它消耗掉再取行数；
                            // 用立即执行闭包把这段作用域收住，否则 row_count 会被判成二次可变借用
                            let res: std::result::Result<u64, String> = (|| {
                                prepared.execute(()).map_err(|e| e.to_string())?;
                                Ok(prepared.row_count().map_err(|e| e.to_string())?.unwrap_or(0) as u64)
                            })();
                            match res {
                                Ok(n) => {
                                    // 要求恰好命中一行却不是一行 ⇒ 与执行失败同等对待：整批回滚
                                    if expect_one.get(i) == Some(&true) && n != 1 {
                                        out.push(Err(anyhow!(
                                            "{}",
                                            crate::editing::exact_one_violation(n)
                                        )));
                                        rolled_back = true;
                                        break;
                                    }
                                    out.push(Ok(n));
                                }
                                Err(msg) => {
                                    out.push(Err(anyhow!("{}", msg)));
                                    rolled_back = true;
                                    break;
                                }
                            }
                        }
                        Err(e) => {
                            out.push(Err(anyhow!("{}", e)));
                            rolled_back = true;
                            break;
                        }
                    }
                }
            }
        }
        if rolled_back {
            conn.rollback()?;
        } else if let Err(e) = conn.commit() {
            // 提交失败不能假装成功；前面的语句已经回滚不了，如实抛出
            return Err(anyhow!("提交失败: {}", e));
        }
        let _ = conn.set_autocommit(true);
        Ok(out)
    })
}

fn execute_blocking(conn_str: &str, setup: &str, sql: &str) -> Result<u64> {
    with_connection(conn_str, |conn| {
        if !setup.trim().is_empty() {
            conn.execute(setup, (), None)?;
        }
        let mut prepared = conn.prepare(sql)?;
        prepared.execute(())?;
        Ok(prepared.row_count()?.unwrap_or(0) as u64)
    })
}

/// 选定列缓冲类型：数值列保留原始类型，其余（日期、二进制、CLOB 等）
/// 一律按文本读取，由驱动负责渲染，避免为每种类型写一套解码分支。
fn pick_buffer_desc(desc: Option<BufferDesc>) -> BufferDesc {
    match desc {
        Some(d @ BufferDesc::I64 { .. }) => d,
        Some(d @ BufferDesc::I32 { .. }) => d,
        Some(d @ BufferDesc::I16 { .. }) => d,
        Some(d @ BufferDesc::F64 { .. }) => d,
        Some(d @ BufferDesc::F32 { .. }) => d,
        _ => BufferDesc::Text {
            max_str_len: MAX_STR_LEN,
        },
    }
}

/// 取第 row 行的单元格值（列缓冲可能是数值或文本，逐个尝试）
fn cell_value(slice: AnyColumnBufferSlice, row: usize) -> Value {
    if let Some(col) = slice.as_text() {
        return match col.get(row) {
            Some(bytes) => Value::String(String::from_utf8_lossy(bytes).into_owned()),
            None => Value::Null,
        };
    }
    if let Some(col) = slice.as_wide_text() {
        return match col.get(row) {
            Some(units) => Value::String(String::from_utf16_lossy(units)),
            None => Value::Null,
        };
    }
    macro_rules! try_num {
        ($t:ty) => {
            if let Some(col) = slice.as_nullable_slice::<$t>() {
                return col
                    .get(row)
                    .map(|v| serde_json::json!(*v))
                    .unwrap_or(Value::Null);
            }
        };
    }
    try_num!(i64);
    try_num!(i32);
    try_num!(i16);
    try_num!(f64);
    try_num!(f32);
    Value::Null
}
