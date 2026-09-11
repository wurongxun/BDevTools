/*! # SQL Server（TDS）连接池与执行封装
基于 `tiberius`（纯 Rust TDS 协议实现，无原生依赖）。

tiberius 明确不提供连接池，这里用「tokio 信号量限流 + 空闲队列复用 Client」
实现一个轻量池：借用归还走 `Drop`，调用方无需手动释放。
*/
use std::ops::{Deref, DerefMut};
use std::sync::Arc;
use std::time::Duration;

use anyhow::{anyhow, Result};
use futures_util::TryStreamExt;
use parking_lot::Mutex;
use serde_json::Value;
use tiberius::{AuthMethod, Client, Config, QueryItem, Row};
use tokio::net::TcpStream;
use tokio_util::compat::{Compat, TokioAsyncWriteCompatExt};

use super::connection::DbConnectionConfig;

/// 池化后的 tiberius 客户端具体类型（tokio TcpStream 经 compat 适配为 futures IO）
pub type MssqlClient = Client<Compat<TcpStream>>;

/// 单连接池最大并发连接数
const MAX_CONNECTIONS: usize = 8;
/// 建连超时（秒）
const CONNECT_TIMEOUT_SECS: u64 = 10;

/// 建连参数（Config 不便于跨线程共享，故按需重建）
#[derive(Clone)]
struct MssqlParams {
    host: String,
    port: u16,
    username: String,
    password: String,
    database: String,
}

impl MssqlParams {
    fn from_config(config: &DbConnectionConfig) -> Self {
        Self {
            host: config.host.clone(),
            port: if config.port > 0 {
                config.port as u16
            } else {
                1433
            },
            username: config.username.clone(),
            password: config.password.clone(),
            database: config.database_name.clone().unwrap_or_default(),
        }
    }

    fn to_config(&self) -> Config {
        let mut cfg = Config::new();
        cfg.host(&self.host);
        cfg.port(self.port);
        cfg.authentication(AuthMethod::sql_server(&self.username, &self.password));
        if !self.database.is_empty() {
            cfg.database(&self.database);
        }
        // 信任服务器证书：内网 / 自建实例几乎都使用自签名证书，不信任会直接握手失败
        cfg.trust_cert();
        cfg
    }
}

/// SQL Server 连接池
pub struct SqlServerPool {
    params: MssqlParams,
    idle: Mutex<Vec<MssqlClient>>,
    permits: Arc<tokio::sync::Semaphore>,
}

impl SqlServerPool {
    pub fn new(config: &DbConnectionConfig) -> Self {
        Self {
            params: MssqlParams::from_config(config),
            idle: Mutex::new(Vec::new()),
            permits: Arc::new(tokio::sync::Semaphore::new(MAX_CONNECTIONS)),
        }
    }

    async fn connect(&self) -> Result<MssqlClient> {
        let cfg = self.params.to_config();
        let addr = cfg.get_addr();
        let tcp = match tokio::time::timeout(Duration::from_secs(CONNECT_TIMEOUT_SECS), TcpStream::connect(&addr)).await {
            Ok(r) => r?,
            Err(_) => return Err(anyhow!("连接 {addr} 超时")),
        };
        tcp.set_nodelay(true)?;
        Ok(Client::connect(cfg, tcp.compat_write()).await?)
    }

    /// 取一条连接：优先复用空闲连接，否则新建；达到并发上限时等待
    pub async fn acquire(&self) -> Result<SqlServerGuard<'_>> {
        let permit = self.permits.clone().acquire_owned().await?;
        // 先取出空闲连接（在锁作用域内完成，随后立即释放锁，避免跨 await 持有 MutexGuard）
        let idle_client = {
            let mut idle = self.idle.lock();
            idle.pop()
        };
        let client = match idle_client {
            Some(c) => c,
            None => self.connect().await?,
        };
        Ok(SqlServerGuard {
            pool: self,
            client: Some(client),
            _permit: permit,
        })
    }

    /// 关闭并清空所有空闲连接
    pub async fn close(&self) {
        // 先取出全部空闲连接并释放锁，再逐个 await 关闭（避免跨 await 持有 MutexGuard）
        let drained: Vec<MssqlClient> = self.idle.lock().drain(..).collect();
        for c in drained {
            let _ = c.close().await;
        }
    }

    /// 执行查询，最多读取 max_rows 行。
    /// 达到上限后仍会把流读完，避免残余结果集污染这条复用连接。
    pub async fn query_rows(&self, sql: &str, max_rows: usize) -> Result<(Vec<String>, Vec<Vec<Value>>)> {
        let mut conn = self.acquire().await?;
        let mut stream = conn.query(sql, &[]).await?;
        let mut columns: Vec<String> = Vec::new();
        let mut rows: Vec<Vec<Value>> = Vec::new();
        while let Some(item) = stream.try_next().await? {
            match item {
                QueryItem::Metadata(meta) => {
                    if columns.is_empty() {
                        columns = meta.columns().iter().map(|c| c.name().to_string()).collect();
                    }
                }
                QueryItem::Row(row) => {
                    if columns.is_empty() {
                        columns = row.columns().iter().map(|c| c.name().to_string()).collect();
                    }
                    if rows.len() >= max_rows {
                        continue;
                    }
                    let mut values = Vec::with_capacity(columns.len());
                    for i in 0..columns.len() {
                        values.push(cell_to_value(&row, i));
                    }
                    rows.push(values);
                }
            }
        }
        Ok((columns, rows))
    }

    /// 执行非查询语句，返回受影响行数（多语句时求和）
    pub async fn execute(&self, sql: &str) -> Result<u64> {
        let mut conn = self.acquire().await?;
        let result = conn.execute(sql, &[]).await?;
        Ok(result.rows_affected().iter().sum())
    }

    /// 查询单个数值（用于 COUNT 统计）
    pub async fn query_count(&self, sql: &str) -> Result<u64> {
        let (_, rows) = self.query_rows(sql, 1).await?;
        let first = rows.into_iter().next().and_then(|r| r.into_iter().next());
        match first {
            Some(Value::Number(n)) => Ok(n.as_u64().unwrap_or(0)),
            Some(Value::String(s)) => Ok(s.parse::<u64>().unwrap_or(0)),
            _ => Ok(0),
        }
    }

    /// 测试连通性：建连成功后立即关闭
    pub async fn test_connection(config: &DbConnectionConfig) -> Result<()> {
        let cfg = MssqlParams::from_config(config).to_config();
        let addr = cfg.get_addr();
        let tcp =
            tokio::time::timeout(Duration::from_secs(CONNECT_TIMEOUT_SECS), TcpStream::connect(&addr))
                .await??;
        tcp.set_nodelay(true)?;
        let client = Client::connect(cfg, tcp.compat_write()).await?;
        client.close().await?;
        Ok(())
    }
}

/// 连接借用句柄：Drop 时把连接归还到空闲队列
pub struct SqlServerGuard<'a> {
    pool: &'a SqlServerPool,
    client: Option<MssqlClient>,
    _permit: tokio::sync::OwnedSemaphorePermit,
}

impl Deref for SqlServerGuard<'_> {
    type Target = MssqlClient;
    fn deref(&self) -> &MssqlClient {
        self.client.as_ref().expect("连接已归还")
    }
}

impl DerefMut for SqlServerGuard<'_> {
    fn deref_mut(&mut self) -> &mut MssqlClient {
        self.client.as_mut().expect("连接已归还")
    }
}

impl Drop for SqlServerGuard<'_> {
    fn drop(&mut self) {
        if let Some(client) = self.client.take() {
            self.pool.idle.lock().push(client);
        }
    }
}

/// 单元格取值：tiberius 的 try_get::<R,_> 已返回 Result<Option<R>>（NULL → Ok(None)），
/// 按类型依次尝试，顺序不能随意调整（SQL Server 二进制协议下类型严格）。
/// 全部失败时回落到 Null，避免把解码失败伪装成具体值。
fn cell_to_value(row: &Row, i: usize) -> Value {
    if let Ok(v) = row.try_get::<bool, usize>(i) {
        return v.map(Value::Bool).unwrap_or(Value::Null);
    }
    if let Ok(v) = row.try_get::<i64, usize>(i) {
        return v.map(|n| Value::Number(n.into())).unwrap_or(Value::Null);
    }
    if let Ok(v) = row.try_get::<f64, usize>(i) {
        return v
            .and_then(|n| serde_json::Number::from_f64(n))
            .map(Value::Number)
            .unwrap_or(Value::Null);
    }
    if let Ok(v) = row.try_get::<&str, usize>(i) {
        return v.map(|s| Value::String(s.to_string())).unwrap_or(Value::Null);
    }
    if let Ok(v) = row.try_get::<chrono::NaiveDateTime, usize>(i) {
        return v
            .map(|d| Value::String(d.format("%Y-%m-%d %H:%M:%S").to_string()))
            .unwrap_or(Value::Null);
    }
    if let Ok(v) = row.try_get::<chrono::NaiveDate, usize>(i) {
        return v
            .map(|d| Value::String(d.format("%Y-%m-%d").to_string()))
            .unwrap_or(Value::Null);
    }
    if let Ok(v) = row.try_get::<chrono::NaiveTime, usize>(i) {
        return v
            .map(|d| Value::String(d.format("%H:%M:%S").to_string()))
            .unwrap_or(Value::Null);
    }
    Value::Null
}
