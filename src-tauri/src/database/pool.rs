/*! # 数据库连接池管理
全局管理 MySQL / PostgreSQL 连接池，通过连接ID索引。
*/
use sqlx::mysql::MySqlPool;
use sqlx::postgres::PgPool;
use std::collections::HashMap;
use parking_lot::Mutex;
use lazy_static::lazy_static;

/// 数据库连接池枚举
#[derive(Clone)]
pub enum DbPool {
    MySQL(MySqlPool),
    PostgreSQL(PgPool),
}

/// 全局连接池管理器
lazy_static! {
    pub static ref POOL_MANAGER: Mutex<HashMap<String, DbPool>> = Mutex::new(HashMap::new());
}

/// 获取指定连接的连接池
pub fn get_pool(conn_id: &str) -> Option<DbPool> {
    POOL_MANAGER.lock().get(conn_id).cloned()
}

/// 存储连接池
pub fn set_pool(conn_id: String, pool: DbPool) {
    POOL_MANAGER.lock().insert(conn_id, pool);
}

/// 移除连接池
pub fn remove_pool(conn_id: &str) -> Option<DbPool> {
    POOL_MANAGER.lock().remove(conn_id)
}
