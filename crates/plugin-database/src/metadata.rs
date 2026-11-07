/*! # 元数据查询模块（薄封装层）

职责只有一条：**取连接池 → 交给 `catalog_for()` → 包 `Response`**。

这里**不再出现任何具体 SQL**，各数据库的语句都在 `database/catalog/{mysql,postgres,mssql,oracle}.rs`
各自的 `CatalogProvider` 实现里。新增一种数据库的成本 = 加一个实现文件 + 工厂一行分支，
本文件与命令层都不用动。
*/
use bdevtools_core::response::response::Response;

pub use super::catalog::{
    catalog_for, CatalogCapabilities, DatabaseIndexInfo, DatabaseObjectInfo, DbAdminInfo, DbUserInfo,
    KeyValueRow, ObjectSourceKind, Scope,
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
///
/// 带一道**主键兜底**：部分库 / 版本的列元数据不带主键标记（明明 DDL 里有 `PRIMARY KEY`，
/// `is_primary_key` 却全是 false），此时用索引里的 `PRIMARY` 把主键补出来。
/// 少了这一步，前端会判定「没有列能唯一定位一行」而把整张表判成只读。
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
        Ok(mut list) => {
            if !list.iter().any(|c| c.is_primary_key) {
                if let Ok(indexes) = catalog_for(&pool).table_indexes(&database, &table).await {
                    fill_primary_key_from_index(&mut list, &indexes);
                }
            }
            Response::new(200, list, "查询成功")
        }
        Err(e) => Response::new(300, vec![], &format!("查询失败: {}", e)),
    }
}

/// 用索引里的 `PRIMARY` 把列的主键标记补上（**只在整张表没有任何主键标记时调用**）。
///
/// 抽成纯函数是为了能直接单测：这类兜底一旦写反（比如把普通 KEY 也算成主键），
/// 症状是「照着非唯一列去 UPDATE 一堆行」，只有真跑生产库才会发现。
fn fill_primary_key_from_index(cols: &mut [ColumnInfo], indexes: &[IndexInfo]) {
    let pk: Vec<String> = indexes
        .iter()
        .filter(|i| i.is_primary)
        .map(|i| i.column_name.to_lowercase())
        .collect();
    if pk.is_empty() {
        return;
    }
    for c in cols.iter_mut() {
        if pk.contains(&c.name.to_lowercase()) {
            c.is_primary_key = true;
        }
    }
}

#[cfg(test)]
mod pk_fallback_tests {
    use super::*;

    fn col(name: &str) -> ColumnInfo {
        ColumnInfo {
            name: name.to_string(),
            data_type: "int".to_string(),
            is_nullable: false,
            is_primary_key: false,
            default_value: None,
            comment: None,
            ordinal_position: 0,
            is_auto_increment: false,
            is_generated: false,
            numeric_precision: None,
            numeric_scale: None,
            character_max_length: None,
        }
    }

    fn idx(name: &str, column: &str, is_unique: bool, is_primary: bool) -> IndexInfo {
        IndexInfo {
            name: name.to_string(),
            column_name: column.to_string(),
            is_unique,
            is_primary,
            index_type: None,
        }
    }

    /// 用户现场：DDL 里有 `PRIMARY KEY (id)`，但列元数据一个主键标记都没带。
    #[test]
    fn primary_index_marks_pk_column() {
        let mut cols = vec![col("id"), col("requestId"), col("lcbh")];
        let indexes = vec![
            idx("PRIMARY", "id", true, true),
            // 普通 KEY 不是唯一的，绝不能被当成主键
            idx("ft_main_198_req_index", "requestId", false, false),
        ];
        fill_primary_key_from_index(&mut cols, &indexes);
        assert!(cols[0].is_primary_key);
        assert!(!cols[1].is_primary_key, "非唯一索引的列不能算主键");
        assert!(!cols[2].is_primary_key);
    }

    /// 复合主键：PRIMARY 索引有几列就标几列。
    #[test]
    fn composite_primary_marks_all_key_columns() {
        let mut cols = vec![col("a"), col("b"), col("c")];
        let indexes = vec![idx("PRIMARY", "a", true, true), idx("PRIMARY", "b", true, true)];
        fill_primary_key_from_index(&mut cols, &indexes);
        assert!(cols[0].is_primary_key && cols[1].is_primary_key);
        assert!(!cols[2].is_primary_key);
    }

    /// 列名大小写不一致时也要对得上（各库返回的列名大小写政策不同）。
    #[test]
    fn column_name_match_is_case_insensitive() {
        let mut cols = vec![col("ID")];
        let indexes = vec![idx("PRIMARY", "id", true, true)];
        fill_primary_key_from_index(&mut cols, &indexes);
        assert!(cols[0].is_primary_key);
    }

    /// 索引里没有 PRIMARY 就一个都别标 —— 宁可判只读，也不能拿非唯一列去定位行。
    #[test]
    fn nothing_marked_without_primary_index() {
        let mut cols = vec![col("id"), col("requestId")];
        let indexes = vec![idx("uk_request", "requestId", true, false)];
        fill_primary_key_from_index(&mut cols, &indexes);
        assert!(cols.iter().all(|c| !c.is_primary_key));
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

/// 取某个对象的**定义源码**（视图定义 / 存储过程体 / 函数体 / 触发器体）
///
/// 与 `db_table_schema` 的区别见 `CatalogProvider::object_source` 的注释：
/// 那条路返回的是「可读的 DDL」，这条返回的是数据库自己承认的源文本。
pub async fn db_object_source(
    config_id: String,
    database: String,
    kind: String,
    name: String,
) -> Response<String> {
    let Some(kind) = ObjectSourceKind::parse(&kind) else {
        return Response::new(300, String::new(), &format!("未知的对象类型: {}", kind));
    };
    let pool = match get_pool(&config_id) {
        Some(p) => p,
        None => return not_connected("未连接到数据库"),
    };
    match catalog_for(&pool)
        .object_source(kind, &database, &name)
        .await
    {
        Ok(src) => Response::new(200, src, "查询成功"),
        Err(e) => Response::new(300, String::new(), &format!("查询失败: {}", e)),
    }
}

/// 获取指定库的函数列表
pub async fn db_list_functions(
    config_id: String,
    database: String,
) -> Response<Vec<DatabaseObjectInfo>> {
    let pool = match get_pool(&config_id) {
        Some(p) => p,
        None => return not_connected("未连接到数据库"),
    };
    match catalog_for(&pool).list_functions(&database).await {
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
