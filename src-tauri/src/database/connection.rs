/*! # 数据库连接配置管理
连接配置的 CRUD 操作，存储在 SQLite 的 db_connections 表中。
*/
use crate::sqlite::sqlite::exec;
use crate::response::response::Response;
use anyhow::{anyhow, Result};
use rusqlite::params;
use serde::{Deserialize, Serialize};
use sqlx::mysql::MySqlPool;
use sqlx::postgres::PgPool;
use tauri::command;
use uuid::Uuid;
use super::pool::{DbPool, set_pool, remove_pool};

/// 数据库连接配置
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct DbConnectionConfig {
    pub id: Option<String>,
    pub name: String,
    pub db_type: String,       // mysql / postgresql
    pub host: String,
    pub port: i32,
    pub username: String,
    pub password: String,
    pub database_name: Option<String>,
    pub ssh_enabled: bool,
    pub ssh_host: Option<String>,
    pub ssh_port: Option<i32>,
    pub ssh_username: Option<String>,
    pub ssh_password: Option<String>,
    pub ssh_key_path: Option<String>,
    pub created_at: Option<String>,
    pub updated_at: Option<String>,
}

/// 初始化 db_connections 表
pub fn init_db_connections_table() {
    let _ = exec(|conn| {
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS db_connections (
                id TEXT PRIMARY KEY,
                name TEXT NOT NULL,
                db_type TEXT NOT NULL,
                host TEXT NOT NULL,
                port INTEGER NOT NULL,
                username TEXT NOT NULL,
                password TEXT NOT NULL,
                database_name TEXT,
                ssh_enabled INTEGER NOT NULL DEFAULT 0,
                ssh_host TEXT,
                ssh_port INTEGER,
                ssh_username TEXT,
                ssh_password TEXT,
                ssh_key_path TEXT,
                created_at TEXT,
                updated_at TEXT
            );"
        )?;
        Ok(())
    });
    println!("[database] db_connections table initialized");
}

/// 创建数据库连接配置
#[command]
pub async fn db_connection_create(config: DbConnectionConfig) -> Response<Option<String>> {
    let id = Uuid::new_v4().to_string();
    let now = chrono::Local::now().format("%Y-%m-%d %H:%M:%S").to_string();
    let res = exec(|conn| {
        conn.execute(
            "INSERT INTO db_connections (id, name, db_type, host, port, username, password, database_name, ssh_enabled, ssh_host, ssh_port, ssh_username, ssh_password, ssh_key_path, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16)",
            params![
                id,
                config.name,
                config.db_type,
                config.host,
                config.port,
                config.username,
                config.password,
                config.database_name,
                config.ssh_enabled as i32,
                config.ssh_host,
                config.ssh_port,
                config.ssh_username,
                config.ssh_password,
                config.ssh_key_path,
                now,
                now,
            ],
        )?;
        Ok(())
    });
    match res {
        Ok(_) => Response::new(200, Some(id), "连接配置创建成功"),
        Err(e) => Response::new(300, None, &format!("创建失败: {}", e)),
    }
}

/// 删除数据库连接配置
#[command]
pub async fn db_connection_delete(id: String) -> Response<bool> {
    // 先断开连接
    remove_pool(&id);
    let res = exec(|conn| {
        let affected = conn.execute("DELETE FROM db_connections WHERE id = ?1", params![id])?;
        Ok(affected > 0)
    });
    match res {
        Ok(true) => Response::new(200, true, "删除成功"),
        Ok(false) => Response::new(300, false, "连接配置不存在"),
        Err(e) => Response::new(300, false, &format!("删除失败: {}", e)),
    }
}

/// 更新数据库连接配置
#[command]
pub async fn db_connection_update(config: DbConnectionConfig) -> Response<bool> {
    let id = match &config.id {
        Some(id) => id.clone(),
        None => return Response::new(300, false, "缺少连接ID"),
    };
    let now = chrono::Local::now().format("%Y-%m-%d %H:%M:%S").to_string();
    // 更新后需要重新连接
    remove_pool(&id);
    let res = exec(|conn| {
        let affected = conn.execute(
            "UPDATE db_connections SET name=?1, db_type=?2, host=?3, port=?4, username=?5, password=?6, database_name=?7, ssh_enabled=?8, ssh_host=?9, ssh_port=?10, ssh_username=?11, ssh_password=?12, ssh_key_path=?13, updated_at=?14 WHERE id=?15",
            params![
                config.name,
                config.db_type,
                config.host,
                config.port,
                config.username,
                config.password,
                config.database_name,
                config.ssh_enabled as i32,
                config.ssh_host,
                config.ssh_port,
                config.ssh_username,
                config.ssh_password,
                config.ssh_key_path,
                now,
                id,
            ],
        )?;
        Ok(affected > 0)
    });
    match res {
        Ok(true) => Response::new(200, true, "更新成功"),
        Ok(false) => Response::new(300, false, "连接配置不存在"),
        Err(e) => Response::new(300, false, &format!("更新失败: {}", e)),
    }
}

/// 获取所有连接配置列表
#[command]
pub async fn db_connection_list() -> Response<Vec<DbConnectionConfig>> {
    let res = exec(|conn| {
        let mut stmt = conn.prepare("SELECT * FROM db_connections ORDER BY created_at DESC")?;
        let mut rows = stmt.query(params![])?;
        let mut list = Vec::new();
        while let Some(row) = rows.next()? {
            list.push(DbConnectionConfig {
                id: row.get("id")?,
                name: row.get("name")?,
                db_type: row.get("db_type")?,
                host: row.get("host")?,
                port: row.get("port")?,
                username: row.get("username")?,
                password: row.get("password")?,
                database_name: row.get("database_name")?,
                ssh_enabled: row.get::<_, i32>("ssh_enabled")? != 0,
                ssh_host: row.get("ssh_host")?,
                ssh_port: row.get("ssh_port")?,
                ssh_username: row.get("ssh_username")?,
                ssh_password: row.get("ssh_password")?,
                ssh_key_path: row.get("ssh_key_path")?,
                created_at: row.get("created_at")?,
                updated_at: row.get("updated_at")?,
            });
        }
        Ok(list)
    });
    match res {
        Ok(list) => Response::new(200, list, "查询成功"),
        Err(e) => Response::new(300, Vec::new(), &format!("查询失败: {}", e)),
    }
}

/// 测试数据库连接连通性
#[command]
pub async fn db_connection_test(config: DbConnectionConfig) -> Response<bool> {
    let result = test_connection(&config).await;
    match result {
        Ok(_) => Response::new(200, true, "连接成功"),
        Err(e) => Response::new(300, false, &format!("连接失败: {}", e)),
    }
}

/// 建立数据库连接（创建连接池）
#[command]
pub async fn db_connect(config_id: String) -> Response<bool> {
    // 从 SQLite 读取配置
    let config_res = exec(|conn| {
        let mut stmt = conn.prepare("SELECT * FROM db_connections WHERE id = ?1")?;
        let mut rows = stmt.query(params![config_id])?;
        if let Some(row) = rows.next()? {
            Ok(Some(DbConnectionConfig {
                id: row.get("id")?,
                name: row.get("name")?,
                db_type: row.get("db_type")?,
                host: row.get("host")?,
                port: row.get("port")?,
                username: row.get("username")?,
                password: row.get("password")?,
                database_name: row.get("database_name")?,
                ssh_enabled: row.get::<_, i32>("ssh_enabled")? != 0,
                ssh_host: row.get("ssh_host")?,
                ssh_port: row.get("ssh_port")?,
                ssh_username: row.get("ssh_username")?,
                ssh_password: row.get("ssh_password")?,
                ssh_key_path: row.get("ssh_key_path")?,
                created_at: row.get("created_at")?,
                updated_at: row.get("updated_at")?,
            }))
        } else {
            Ok(None)
        }
    });

    match config_res {
        Ok(Some(config)) => {
            match create_pool(&config).await {
                Ok(pool) => {
                    set_pool(config_id, pool);
                    Response::new(200, true, "连接成功")
                }
                Err(e) => Response::new(300, false, &format!("连接失败: {}", e)),
            }
        }
        Ok(None) => Response::new(300, false, "连接配置不存在"),
        Err(e) => Response::new(300, false, &format!("读取配置失败: {}", e)),
    }
}

/// 断开数据库连接
#[command]
pub async fn db_disconnect(config_id: String) -> Response<bool> {
    if let Some(pool) = remove_pool(&config_id) {
        match pool {
            DbPool::MySQL(p) => { p.close().await; }
            DbPool::PostgreSQL(p) => { p.close().await; }
        }
        Response::new(200, true, "已断开连接")
    } else {
        Response::new(300, false, "连接不存在")
    }
}

/// 内部：测试连接
async fn test_connection(config: &DbConnectionConfig) -> Result<()> {
    match config.db_type.as_str() {
        "mysql" => {
            let url = format!(
                "mysql://{}:{}@{}:{}/{}",
                config.username,
                config.password,
                config.host,
                config.port,
                config.database_name.as_deref().unwrap_or(""),
            );
            let pool = MySqlPool::connect(&url).await?;
            pool.close().await;
            Ok(())
        }
        "postgresql" => {
            let url = format!(
                "postgres://{}:{}@{}:{}/{}",
                config.username,
                config.password,
                config.host,
                config.port,
                config.database_name.as_deref().unwrap_or("postgres"),
            );
            let pool = PgPool::connect(&url).await?;
            pool.close().await;
            Ok(())
        }
        _ => Err(anyhow!("不支持的数据库类型: {}", config.db_type)),
    }
}

/// 内部：创建连接池
pub async fn create_pool(config: &DbConnectionConfig) -> Result<DbPool> {
    match config.db_type.as_str() {
        "mysql" => {
            let url = format!(
                "mysql://{}:{}@{}:{}/{}",
                config.username,
                config.password,
                config.host,
                config.port,
                config.database_name.as_deref().unwrap_or(""),
            );
            let pool = MySqlPool::connect(&url).await?;
            Ok(DbPool::MySQL(pool))
        }
        "postgresql" => {
            let url = format!(
                "postgres://{}:{}@{}:{}/{}",
                config.username,
                config.password,
                config.host,
                config.port,
                config.database_name.as_deref().unwrap_or("postgres"),
            );
            let pool = PgPool::connect(&url).await?;
            Ok(DbPool::PostgreSQL(pool))
        }
        _ => Err(anyhow!("不支持的数据库类型: {}", config.db_type)),
    }
}
