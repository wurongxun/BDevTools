//! 数据库连接 / 操作 / 元数据 / 导入导出 命令（原 commands.rs 对应分区的机械拆分）。
// 分片内命令可能被 feature 整体裁掉，故 glob import 允许未使用。
#[allow(unused_imports)]
use super::*;

// ==================== 数据库连接配置 ====================

/// 创建数据库连接配置
#[cfg(feature = "plugin-database")]
#[tauri::command]
pub async fn db_connection_create(config: DbConnectionConfig) -> Response<Option<String>> {
    crate::plugins::database::connection::db_connection_create(config).await
}

/// 删除数据库连接配置
#[cfg(feature = "plugin-database")]
#[tauri::command]
pub async fn db_connection_delete(id: String) -> Response<bool> {
    crate::plugins::database::connection::db_connection_delete(id).await
}

/// 更新数据库连接配置
#[cfg(feature = "plugin-database")]
#[tauri::command]
pub async fn db_connection_update(config: DbConnectionConfig) -> Response<bool> {
    crate::plugins::database::connection::db_connection_update(config).await
}

/// 获取所有连接配置列表
#[cfg(feature = "plugin-database")]
#[tauri::command]
pub async fn db_connection_list() -> Response<Vec<DbConnectionConfig>> {
    crate::plugins::database::connection::db_connection_list().await
}

/// 测试数据库连接连通性
#[cfg(feature = "plugin-database")]
#[tauri::command]
pub async fn db_connection_test(config: DbConnectionConfig) -> Response<bool> {
    crate::plugins::database::connection::db_connection_test(config).await
}

/// 建立数据库连接（创建连接池）
#[cfg(feature = "plugin-database")]
#[tauri::command]
pub async fn db_connect(config_id: String) -> Response<bool> {
    crate::plugins::database::connection::db_connect(config_id).await
}

/// 断开数据库连接
#[cfg(feature = "plugin-database")]
#[tauri::command]
pub async fn db_disconnect(config_id: String) -> Response<bool> {
    crate::plugins::database::connection::db_disconnect(config_id).await
}

// ==================== 数据库操作 ====================

/// 执行 SQL 语句（分页）
#[cfg(feature = "plugin-database")]
#[tauri::command]
pub async fn db_execute_sql(
    config_id: String,
    sql: String,
    database: Option<String>,
    page_size: Option<u64>,
    offset: Option<u64>,
) -> Response<SqlResult> {
    crate::plugins::database::operations::db_execute_sql(config_id, sql, database, page_size, offset).await
}

/// 分页查询表数据
#[cfg(feature = "plugin-database")]
#[tauri::command]
pub async fn db_query_table_data(
    config_id: String,
    database: String,
    table: String,
    page: u64,
    page_size: u64,
    where_clause: Option<String>,
    order_by: Option<String>,
) -> Response<TableDataResult> {
    crate::plugins::database::operations::db_query_table_data(
        config_id, database, table, page, page_size, where_clause, order_by,
    )
    .await
}

/// 结果网格：批量写回（一个事务，逐条返回受影响行数）
#[cfg(feature = "plugin-database")]
#[tauri::command]
pub async fn db_apply_edits(
    config_id: String,
    database: String,
    ops: Vec<crate::plugins::database::editing::EditOp>,
) -> Response<crate::plugins::database::editing::EditBatchResult> {
    crate::plugins::database::editing::db_apply_edits(config_id, database, ops).await
}

/// 结果网格：预览批量写将要执行的 SQL（与 `db_apply_edits` 同源，不落地）
#[cfg(feature = "plugin-database")]
#[tauri::command]
pub async fn db_preview_edits(
    config_id: String,
    database: String,
    ops: Vec<crate::plugins::database::editing::EditOp>,
) -> Response<crate::plugins::database::editing::EditPreview> {
    crate::plugins::database::editing::db_preview_edits(config_id, database, ops).await
}

// ==================== 数据库元数据 ====================
/// 获取所有数据库列表
#[cfg(feature = "plugin-database")]
#[tauri::command]
pub async fn db_list_databases(config_id: String) -> Response<Vec<DatabaseInfo>> {
    crate::plugins::database::metadata::db_list_databases(config_id).await
}

/// 获取指定数据库的所有表
#[cfg(feature = "plugin-database")]
#[tauri::command]
pub async fn db_list_tables(config_id: String, database: String) -> Response<Vec<TableInfo>> {
    crate::plugins::database::metadata::db_list_tables(config_id, database).await
}

/// 获取表的字段信息
#[cfg(feature = "plugin-database")]
#[tauri::command]
pub async fn db_table_columns(
    config_id: String,
    database: String,
    table: String,
) -> Response<Vec<ColumnInfo>> {
    crate::plugins::database::metadata::db_table_columns(config_id, database, table).await
}

/// 获取表的索引信息
#[cfg(feature = "plugin-database")]
#[tauri::command]
pub async fn db_table_indexes(
    config_id: String,
    database: String,
    table: String,
) -> Response<Vec<IndexInfo>> {
    crate::plugins::database::metadata::db_table_indexes(config_id, database, table).await
}

/// 获取建表 DDL
#[cfg(feature = "plugin-database")]
#[tauri::command]
pub async fn db_table_schema(
    config_id: String,
    database: String,
    table: String,
) -> Response<String> {
    crate::plugins::database::metadata::db_table_schema(config_id, database, table).await
}

// ---- 目录能力声明与新增对象（走 catalog 抽象，各库按自身能力实现）----

/// 查询当前连接支持哪些目录能力（前端据此决定渲染哪些树节点）
#[cfg(feature = "plugin-database")]
#[tauri::command]
pub async fn db_catalog_capabilities(config_id: String) -> Response<CatalogCapabilities> {
    crate::plugins::database::metadata::db_catalog_capabilities(config_id).await
}

/// 获取指定库的视图列表
#[cfg(feature = "plugin-database")]
#[tauri::command]
pub async fn db_list_views(
    config_id: String,
    database: String,
) -> Response<Vec<DatabaseObjectInfo>> {
    crate::plugins::database::metadata::db_list_views(config_id, database).await
}

/// 获取指定库的存储过程列表
#[cfg(feature = "plugin-database")]
#[tauri::command]
pub async fn db_list_procedures(
    config_id: String,
    database: String,
) -> Response<Vec<DatabaseObjectInfo>> {
    crate::plugins::database::metadata::db_list_procedures(config_id, database).await
}

/// 获取指定库的函数列表
#[cfg(feature = "plugin-database")]
#[tauri::command]
pub async fn db_list_functions(
    config_id: String,
    database: String,
) -> Response<Vec<DatabaseObjectInfo>> {
    crate::plugins::database::metadata::db_list_functions(config_id, database).await
}

/// 取某个对象的定义源码（视图 / 存储过程 / 函数 / 触发器）
#[cfg(feature = "plugin-database")]
#[tauri::command]
pub async fn db_object_source(
    config_id: String,
    database: String,
    kind: String,
    name: String,
) -> Response<String> {
    crate::plugins::database::metadata::db_object_source(config_id, database, kind, name).await
}

/// 获取指定库的触发器列表
#[cfg(feature = "plugin-database")]
#[tauri::command]
pub async fn db_list_triggers(
    config_id: String,
    database: String,
) -> Response<Vec<DatabaseObjectInfo>> {
    crate::plugins::database::metadata::db_list_triggers(config_id, database).await
}

/// 获取指定库的事件列表（仅 MySQL 支持）
#[cfg(feature = "plugin-database")]
#[tauri::command]
pub async fn db_list_events(
    config_id: String,
    database: String,
) -> Response<Vec<DatabaseObjectInfo>> {
    crate::plugins::database::metadata::db_list_events(config_id, database).await
}

/// 获取指定库的索引聚合
#[cfg(feature = "plugin-database")]
#[tauri::command]
pub async fn db_list_database_indexes(
    config_id: String,
    database: String,
) -> Response<Vec<DatabaseIndexInfo>> {
    crate::plugins::database::metadata::db_list_database_indexes(config_id, database).await
}

/// 获取实例用户列表
#[cfg(feature = "plugin-database")]
#[tauri::command]
pub async fn db_list_users(config_id: String) -> Response<Vec<DbUserInfo>> {
    crate::plugins::database::metadata::db_list_users(config_id).await
}

/// 获取管理员列表（各库用自身原生语义判定）
#[cfg(feature = "plugin-database")]
#[tauri::command]
pub async fn db_list_admins(config_id: String) -> Response<Vec<DbAdminInfo>> {
    crate::plugins::database::metadata::db_list_admins(config_id).await
}

/// 获取运行状态（scope: session | global）
#[cfg(feature = "plugin-database")]
#[tauri::command]
pub async fn db_show_status(config_id: String, scope: String) -> Response<Vec<KeyValueRow>> {
    crate::plugins::database::metadata::db_show_status(config_id, scope).await
}

/// 获取系统变量（scope: session | global）
#[cfg(feature = "plugin-database")]
#[tauri::command]
pub async fn db_show_variables(config_id: String, scope: String) -> Response<Vec<KeyValueRow>> {
    crate::plugins::database::metadata::db_show_variables(config_id, scope).await
}

// ==================== 数据库导入导出 ====================

/// 导出表数据为 CSV
#[cfg(feature = "plugin-database")]
#[tauri::command]
pub async fn db_export_csv(
    config_id: String,
    database: String,
    table: String,
    file_path: String,
) -> Response<ExportResult> {
    crate::plugins::database::import_export::db_export_csv(config_id, database, table, file_path).await
}

/// 导出表数据为 Excel
#[cfg(feature = "plugin-database")]
#[tauri::command]
pub async fn db_export_excel(
    config_id: String,
    database: String,
    table: String,
    file_path: String,
) -> Response<ExportResult> {
    crate::plugins::database::import_export::db_export_excel(config_id, database, table, file_path).await
}

/// 按任意 SQL 语句导出结果为 CSV
#[cfg(feature = "plugin-database")]
#[tauri::command]
pub async fn db_export_sql_csv(
    config_id: String,
    sql: String,
    file_path: String,
) -> Response<ExportResult> {
    crate::plugins::database::import_export::db_export_sql_csv(config_id, sql, file_path).await
}

/// 按任意 SQL 语句导出结果为 Excel
#[cfg(feature = "plugin-database")]
#[tauri::command]
pub async fn db_export_sql_excel(
    config_id: String,
    sql: String,
    file_path: String,
) -> Response<ExportResult> {
    crate::plugins::database::import_export::db_export_sql_excel(config_id, sql, file_path).await
}

/// 从 CSV 导入数据到表
#[cfg(feature = "plugin-database")]
#[tauri::command]
pub async fn db_import_csv(
    config_id: String,
    database: String,
    table: String,
    file_path: String,
    has_header: bool,
) -> Response<ImportResult> {
    crate::plugins::database::import_export::db_import_csv(config_id, database, table, file_path, has_header)
        .await
}

/// 从 Excel 导入数据到表
#[cfg(feature = "plugin-database")]
#[tauri::command]
pub async fn db_import_excel(
    config_id: String,
    database: String,
    table: String,
    file_path: String,
) -> Response<ImportResult> {
    crate::plugins::database::import_export::db_import_excel(config_id, database, table, file_path).await
}
