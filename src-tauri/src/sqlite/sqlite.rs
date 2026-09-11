/*! # SQLite 连接管理（app.db 统一读写入口）*/
use anyhow::Result;
use rusqlite::Connection;
use std::path::PathBuf;
/// 创建数据库连接
fn create_sqlite_connection() -> Result<Connection> {
    println!("create_sqlite_connection start");
    let app_dir = PathBuf::from(&crate::app::app_store::APP.lock().app_dir);
    // 确保目录存在
    if !app_dir.exists() {
        std::fs::create_dir_all(&app_dir)?;
    }
    let database = app_dir.join("app.db");
    println!("create_sqlite_connection database: {:?}", database);
    let conn = Connection::open(database)?;
    // 初始化表结构
    init_tables(&conn)?;
    println!("create_sqlite_connection end");
    Ok(conn)
}

/// 初始化数据库表
fn init_tables(conn: &Connection) -> Result<()> {
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS connections (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            name TEXT NOT NULL,
            address TEXT NOT NULL,
            port INTEGER NOT NULL DEFAULT 22,
            username TEXT NOT NULL,
            password TEXT NOT NULL,
            agreement TEXT NOT NULL DEFAULT 'ssh',
            favorites_name TEXT NOT NULL DEFAULT 'default',
            affix INTEGER NOT NULL DEFAULT 0,
            auth_type TEXT NOT NULL DEFAULT 'password',
            private_key_path TEXT NOT NULL DEFAULT ''
        );"
    )?;
    // 迁移：为已有表添加新列（忽略已存在错误）
    let _ = conn.execute_batch("ALTER TABLE connections ADD COLUMN auth_type TEXT NOT NULL DEFAULT 'password';");
    let _ = conn.execute_batch("ALTER TABLE connections ADD COLUMN private_key_path TEXT NOT NULL DEFAULT '';");
    // 初始化数据库连接配置表
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
}
/// 封装一个方法，获取连接
pub fn exec<F, T>(func: F) -> Result<T>
where
    F: FnOnce(&mut Connection) -> Result<T>,
{
    match create_sqlite_connection() {
        Ok(mut conn) => func(&mut conn),
        Err(e) => Err(e),
    }
}
