/*! # 元数据查询模块（薄封装层）

职责只有一条：**取连接池 → 交给 `catalog_for()` → 包 `Response`**。

这里**不再出现任何具体 SQL**，各数据库的语句都在 `database/catalog/{mysql,postgres,mssql,oracle}.rs`
各自的 `CatalogProvider` 实现里。新增一种数据库的成本 = 加一个实现文件 + 工厂一行分支，
本文件与命令层都不用动。
*/
use crate::response::response::Response;

pub use super::catalog::{
    catalog_for, CatalogCapabilities, DatabaseIndexInfo, DatabaseObjectInfo, DbAdminInfo, DbUserInfo,
    KeyValueRow, Scope,
};
use super::pool::get_pool;

// 类型统一从 catalog 转出：命令层仍从本模块引用，避免既有 import 全部改动
pub use super::catalog::{DatabaseInfo, IndexInfo, TableInfo};
pub use super::operations::ColumnInfo;

/// 统一的「未连接」响应构造
fn not_connected<T: serde::Serialize + Default>(msg: &str) -> Response<T> {
    Response::new(300, T::default(), msg)
}

// ==================== 基础能力 ====================

/// 获取所有数据库列表
pub async fn db_list_databases(config_id: String) -> Response<Vec<DatabaseInfo>> {
    let pool = match get_pool(&config_id) {
        Some(p) => p,
        None => return not_connected("未连接到数据库"),
    };
    match catalog_for(&pool).list_databases().await {
        Ok(list) => Response::new(200, list, "查询成功"),
        Err(e) => Response::new(300, vec![], &format!("查询失败: {}", e)),
    }
}

/// 获取指定数据库的所有表
pub async fn db_list_tables(config_id: String, database: String) -> Response<Vec<TableInfo>> {
    let pool = match get_pool(&config_id) {
        Some(p) => p,
        None => return not_connected("未连接到数据库"),
    };
    match catalog_for(&pool).list_tables(&database).await {
        Ok(list) => Response::new(200, list, "查询成功"),
        Err(e) => Response::new(300, vec![], &format!("查询失败: {}", e)),
    }
}

/// 获取表的字段信息
pub async fn db_table_columns(
    config_id: String,
    database: String,
    table: String,
) -> Response<Vec<ColumnInfo>> {
    let pool = match get_pool(&config_id) {
        Some(p) => p,
        None => return not_connected("未连接到数据库"),
    };
    match catalog_for(&pool).table_columns(&database, &table).await {
        Ok(list) => Response::new(200, list, "查询成功"),
        Err(e) => Response::new(300, vec![], &format!("查询失败: {}", e)),
    }
}

/// 获取表的索引信息
pub async fn db_table_indexes(
    config_id: String,
    database: String,
    table: String,
) -> Response<Vec<IndexInfo>> {
    let pool = match get_pool(&config_id) {
        Some(p) => p,
        None => return not_connected("未连接到数据库"),
    };
    match catalog_for(&pool).table_indexes(&database, &table).await {
        Ok(list) => Response::new(200, list, "查询成功"),
        Err(e) => Response::new(300, vec![], &format!("查询失败: {}", e)),
    }
}

/// 获取建表 DDL
pub async fn db_table_schema(
    config_id: String,
    database: String,
    table: String,
) -> Response<String> {
    let pool = match get_pool(&config_id) {
        Some(p) => p,
        None => return not_connected("未连接到数据库"),
    };
    match catalog_for(&pool).table_schema(&database, &table).await {
        Ok(ddl) => Response::new(200, ddl, "查询成功"),
        Err(e) => Response::new(300, String::new(), &format!("查询失败: {}", e)),
    }
}

// ==================== 能力声明 ====================

/// 查询当前连接支持哪些目录能力（前端据此决定渲染哪些树节点）
///
/// 未连接时返回全 false，前端应当把所有可选分组隐藏掉。
pub async fn db_catalog_capabilities(config_id: String) -> Response<CatalogCapabilities> {
    let pool = match get_pool(&config_id) {
        Some(p) => p,
        None => return Response::new(300, CatalogCapabilities::none(), "未连接到数据库"),
    };
    Response::new(200, catalog_for(&pool).capabilities(), "查询成功")
}

// ==================== 库级对象 ====================

/// 获取指定库的视图列表
pub async fn db_list_views(
    config_id: String,
    database: String,
) -> Response<Vec<DatabaseObjectInfo>> {
    let pool = match get_pool(&config_id) {
        Some(p) => p,
        None => return not_connected("未连接到数据库"),
    };
    match catalog_for(&pool).list_views(&database).await {
        Ok(list) => Response::new(200, list, "查询成功"),
        Err(e) => Response::new(300, vec![], &format!("查询失败: {}", e)),
    }
}

/// 获取指定库的存储过程列表
pub async fn db_list_procedures(
    config_id: String,
    database: String,
) -> Response<Vec<DatabaseObjectInfo>> {
    let pool = match get_pool(&config_id) {
        Some(p) => p,
        None => return not_connected("未连接到数据库"),
    };
    match catalog_for(&pool).list_procedures(&database).await {
        Ok(list) => Response::new(200, list, "查询成功"),
        Err(e) => Response::new(300, vec![], &format!("查询失败: {}", e)),
    }
}

/// 获取指定库的触发器列表
pub async fn db_list_triggers(
    config_id: String,
    database: String,
) -> Response<Vec<DatabaseObjectInfo>> {
    let pool = match get_pool(&config_id) {
        Some(p) => p,
        None => return not_connected("未连接到数据库"),
    };
    match catalog_for(&pool).list_triggers(&database).await {
        Ok(list) => Response::new(200, list, "查询成功"),
        Err(e) => Response::new(300, vec![], &format!("查询失败: {}", e)),
    }
}

/// 获取指定库的事件列表（仅 MySQL 支持，其余库返回「暂不支持」）
pub async fn db_list_events(
    config_id: String,
    database: String,
) -> Response<Vec<DatabaseObjectInfo>> {
    let pool = match get_pool(&config_id) {
        Some(p) => p,
        None => return not_connected("未连接到数据库"),
    };
    match catalog_for(&pool).list_events(&database).await {
        Ok(list) => Response::new(200, list, "查询成功"),
        Err(e) => Response::new(300, vec![], &format!("查询失败: {}", e)),
    }
}

/// 获取指定库的索引聚合（跨表）
pub async fn db_list_database_indexes(
    config_id: String,
    database: String,
) -> Response<Vec<DatabaseIndexInfo>> {
    let pool = match get_pool(&config_id) {
        Some(p) => p,
        None => return not_connected("未连接到数据库"),
    };
    match catalog_for(&pool).list_database_indexes(&database).await {
        Ok(list) => Response::new(200, list, "查询成功"),
        Err(e) => Response::new(300, vec![], &format!("查询失败: {}", e)),
    }
}

// ==================== 实例级 ====================

/// 获取实例用户列表
pub async fn db_list_users(config_id: String) -> Response<Vec<DbUserInfo>> {
    let pool = match get_pool(&config_id) {
        Some(p) => p,
        None => return not_connected("未连接到数据库"),
    };
    match catalog_for(&pool).list_users().await {
        Ok(list) => Response::new(200, list, "查询成功"),
        Err(e) => Response::new(300, vec![], &format!("查询失败: {}", e)),
    }
}

/// 获取管理员列表（各库用自己的原生语义判定）
pub async fn db_list_admins(config_id: String) -> Response<Vec<DbAdminInfo>> {
    let pool = match get_pool(&config_id) {
        Some(p) => p,
        None => return not_connected("未连接到数据库"),
    };
    match catalog_for(&pool).list_admins().await {
        Ok(list) => Response::new(200, list, "查询成功"),
        Err(e) => Response::new(300, vec![], &format!("查询失败: {}", e)),
    }
}

/// 获取运行状态（session / global）
pub async fn db_show_status(config_id: String, scope: String) -> Response<Vec<KeyValueRow>> {
    let pool = match get_pool(&config_id) {
        Some(p) => p,
        None => return not_connected("未连接到数据库"),
    };
    match catalog_for(&pool).show_status(Scope::parse(&scope)).await {
        Ok(list) => Response::new(200, list, "查询成功"),
        Err(e) => Response::new(300, vec![], &format!("查询失败: {}", e)),
    }
}

/// 获取系统变量（session / global）
pub async fn db_show_variables(config_id: String, scope: String) -> Response<Vec<KeyValueRow>> {
    let pool = match get_pool(&config_id) {
        Some(p) => p,
        None => return not_connected("未连接到数据库"),
    };
    match catalog_for(&pool).show_variables(Scope::parse(&scope)).await {
        Ok(list) => Response::new(200, list, "查询成功"),
        Err(e) => Response::new(300, vec![], &format!("查询失败: {}", e)),
    }
}
