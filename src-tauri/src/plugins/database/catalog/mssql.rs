/*! # SQL Server 目录实现

要点：

- **跨库查询用三段式命名** `[库].sys.xxx`（`SqlServerPool::query_rows` 只接受单条语句，
  不能用 `USE db; SELECT ...`），这样树上点到哪个库就查哪个库，不受当前连接库限制。
- **事件不支持**：SQL Server 的定时作业是 SQL Agent（Job），与 MySQL EVENT 语义差别过大，
  `capabilities().events = false`，不显示该节点。
- **变量**：只有实例级配置（`sys.configurations`），故 `session_variables = false`、
  `global_variables = true`；状态则当前会话走 `sys.dm_exec_sessions`、实例走 `sys.dm_os_sys_info`。
- **管理员**：`sysadmin` 固定服务器角色成员。
*/
use async_trait::async_trait;
use serde_json::Value;
use std::sync::Arc;

use super::*;
use crate::plugins::database::mssql::SqlServerPool;

/// 单次查询最多读取的行数（与现有元数据查询保持一致的上限）
const MAX_ROWS: usize = 10000;

pub struct SqlServerCatalog {
    pool: Arc<SqlServerPool>,
}

impl SqlServerCatalog {
    pub fn new(pool: Arc<SqlServerPool>) -> Self {
        Self { pool }
    }

    async fn q(&self, sql: &str) -> Result<Vec<Vec<Value>>> {
        let (_, rows) = self.pool.query_rows(sql, MAX_ROWS).await?;
        Ok(rows)
    }

    /// 三段式命名前缀：库名为空时不加前缀（退回当前库上下文）
    fn p(&self, database: &str) -> String {
        if database.is_empty() {
            String::new()
        } else {
            format!("{}.", self.quote_ident(database))
        }
    }
}

/// 按下标取字符串（越界 / NULL → 空串）
fn cell(row: &[Value], i: usize) -> String {
    row.get(i).and_then(str_of).unwrap_or_default()
}

/// 按下标取可空字符串
fn opt_cell(row: &[Value], i: usize) -> Option<String> {
    row.get(i).and_then(str_of).filter(|v| !v.is_empty())
}

/// 按下标取布尔
fn cell_bool(row: &[Value], i: usize) -> bool {
    row.get(i).and_then(bool_of).unwrap_or(false)
}

#[async_trait]
impl CatalogProvider for SqlServerCatalog {
    fn db_type(&self) -> &'static str {
        "SQL Server"
    }

    fn dialect(&self) -> Dialect {
        Dialect::SqlServer
    }

    fn capabilities(&self) -> CatalogCapabilities {
        CatalogCapabilities {
            views: true,
            procedures: true,
            triggers: true,
            events: false, // 定时作业是 SQL Agent，语义不同
            database_indexes: true,
            users: true,
            admins: true,
            session_status: true,
            global_status: true,
            session_variables: false, // 只有实例级配置，无会话级变量概念
            global_variables: true,
        }
    }

    async fn list_databases(&self) -> Result<Vec<DatabaseInfo>> {
        let rows = self
            .q("SELECT name FROM sys.databases WHERE database_id > 4 ORDER BY name")
            .await?;
        Ok(rows
            .iter()
            .map(|r| DatabaseInfo {
                name: cell(r, 0),
                charset: None,
                collation: None,
            })
            .collect())
    }

    async fn list_tables(&self, database: &str) -> Result<Vec<TableInfo>> {
        let sql = format!(
            "SELECT t.name AS name, NULL AS engine, p.rows AS row_count, NULL AS comment, NULL AS data_length \
             FROM {}sys.tables t \
             LEFT JOIN {}sys.partitions p ON t.object_id = p.object_id AND p.index_id IN (0, 1) \
             ORDER BY t.name",
            self.p(database),
            self.p(database)
        );
        let rows = self.q(&sql).await?;
        Ok(rows
            .iter()
            .map(|r| TableInfo {
                name: cell(r, 0),
                engine: None,
                row_count: r.get(2).and_then(num_of),
                comment: None,
                data_length: None,
            })
            .collect())
    }

    async fn table_columns(&self, database: &str, table: &str) -> Result<Vec<ColumnInfo>> {
        let p = self.p(database);
        let lit = format!("{}", self.escape_literal(&format!("{}.{}", database, table)));
        let sql = format!(
            "SELECT c.name AS name, ty.name AS data_type, c.is_nullable AS is_nullable, \
             dc.definition AS default_value, \
             CASE WHEN pk.column_id IS NOT NULL THEN 'PRI' ELSE '' END AS column_key, '' AS comment_txt \
             FROM {}sys.columns c \
             JOIN {}sys.types ty ON c.user_type_id = ty.user_type_id \
             LEFT JOIN {}sys.default_constraints dc ON c.default_object_id = dc.object_id \
             LEFT JOIN (SELECT ic.column_id FROM {}sys.indexes i \
                JOIN {}sys.index_columns ic ON i.object_id = ic.object_id AND i.index_id = ic.index_id \
                WHERE i.is_primary_key = 1 AND i.object_id = OBJECT_ID(N'{}')) pk ON c.column_id = pk.column_id \
             WHERE c.object_id = OBJECT_ID(N'{}') ORDER BY c.column_id",
            p, p, p, p, p, lit, lit
        );
        let rows = self.q(&sql).await?;
        Ok(rows
            .iter()
            .map(|r| ColumnInfo {
                name: cell(r, 0),
                data_type: cell(r, 1),
                is_nullable: cell_bool(r, 2),
                is_primary_key: cell(r, 4) == "PRI",
                default_value: opt_cell(r, 3),
                comment: opt_cell(r, 5),
            })
            .collect())
    }

    async fn table_indexes(&self, database: &str, table: &str) -> Result<Vec<IndexInfo>> {
        let p = self.p(database);
        let lit = self.escape_literal(&format!("{}.{}", database, table));
        let sql = format!(
            "SELECT i.name AS name, c.name AS column_name, i.is_unique AS is_unique, \
             i.is_primary_key AS is_primary, i.type_desc AS index_type \
             FROM {}sys.indexes i \
             JOIN {}sys.index_columns ic ON i.object_id = ic.object_id AND i.index_id = ic.index_id \
             JOIN {}sys.columns c ON ic.object_id = c.object_id AND ic.column_id = c.column_id \
             WHERE i.object_id = OBJECT_ID(N'{}') AND i.name IS NOT NULL \
             ORDER BY i.name, ic.key_ordinal",
            p, p, p, lit
        );
        let rows = self.q(&sql).await?;
        Ok(rows
            .iter()
            .map(|r| IndexInfo {
                name: cell(r, 0),
                column_name: cell(r, 1),
                is_unique: cell_bool(r, 2),
                is_primary: cell_bool(r, 3),
                index_type: opt_cell(r, 4),
            })
            .collect())
    }

    async fn table_schema(&self, database: &str, table: &str) -> Result<String> {
        let p = self.p(database);
        let lit = self.escape_literal(&format!("{}.{}", database, table));
        let sql = format!(
            "SELECT c.name, ty.name, c.max_length, c.precision, c.scale, c.is_nullable, dc.definition \
             FROM {}sys.columns c \
             JOIN {}sys.types ty ON c.user_type_id = ty.user_type_id \
             LEFT JOIN {}sys.default_constraints dc ON c.default_object_id = dc.object_id \
             WHERE c.object_id = OBJECT_ID(N'{}') ORDER BY c.column_id",
            p, p, p, lit
        );
        let rows = self.q(&sql).await?;
        let mut ddl = format!(
            "CREATE TABLE {}.{} (\n",
            self.quote_ident(database),
            self.quote_ident(table)
        );
        for (i, r) in rows.iter().enumerate() {
            let col_name = cell(r, 0);
            let type_name = cell(r, 1);
            let max_len = r.get(2).and_then(num_of).unwrap_or(0);
            let precision = r.get(3).and_then(num_of).unwrap_or(0);
            let scale = r.get(4).and_then(num_of).unwrap_or(0);
            let nullable = cell_bool(r, 5);
            let default = opt_cell(r, 6);
            let type_str = type_str(&type_name, max_len, precision, scale);
            let null_str = if nullable { "NULL" } else { "NOT NULL" };
            let default_str = default.map(|d| format!(" DEFAULT {}", d)).unwrap_or_default();
            if i > 0 {
                ddl.push_str(",\n");
            }
            ddl.push_str(&format!(
                "  {} {} {}{}",
                self.quote_ident(&col_name),
                type_str,
                null_str,
                default_str
            ));
        }
        ddl.push_str("\n);");
        Ok(ddl)
    }

    // ===== 库级对象 =====

    async fn list_views(&self, database: &str) -> Result<Vec<DatabaseObjectInfo>> {
        let p = self.p(database);
        let sql = format!(
            "SELECT v.name AS name, 'VIEW' AS object_type, DB_NAME() AS database_name, \
             '' AS definer, CONVERT(varchar(64), v.create_date, 120) AS created, \
             CONVERT(varchar(64), v.modify_date, 120) AS modified, '' AS comment_txt, \
             '' AS status_txt, '' AS event_txt, '' AS table_name \
             FROM {}sys.views v ORDER BY v.name",
            p
        );
        let rows = self.q(&sql).await?;
        Ok(rows.iter().map(|r| object_from_row(r)).collect())
    }

    async fn list_procedures(&self, database: &str) -> Result<Vec<DatabaseObjectInfo>> {
        let p = self.p(database);
        let sql = format!(
            "SELECT pr.name AS name, 'PROCEDURE' AS object_type, DB_NAME() AS database_name, \
             '' AS definer, CONVERT(varchar(64), pr.create_date, 120) AS created, \
             CONVERT(varchar(64), pr.modify_date, 120) AS modified, '' AS comment_txt, \
             '' AS status_txt, '' AS event_txt, '' AS table_name \
             FROM {}sys.procedures pr WHERE pr.is_ms_shipped = 0 ORDER BY pr.name",
            p
        );
        let rows = self.q(&sql).await?;
        Ok(rows.iter().map(|r| object_from_row(r)).collect())
    }

    async fn list_triggers(&self, database: &str) -> Result<Vec<DatabaseObjectInfo>> {
        let p = self.p(database);
        let sql = format!(
            "SELECT tr.name AS name, 'TRIGGER' AS object_type, DB_NAME() AS database_name, \
             '' AS definer, CONVERT(varchar(64), tr.create_date, 120) AS created, '' AS modified, \
             '' AS comment_txt, \
             CASE WHEN tr.is_disabled = 1 THEN 'DISABLED' ELSE 'ENABLED' END AS status_txt, \
             '' AS event_txt, ISNULL(tbl.name, '') AS table_name \
             FROM {}sys.triggers tr \
             LEFT JOIN {}sys.tables tbl ON tr.parent_id = tbl.object_id \
             WHERE tr.is_ms_shipped = 0 ORDER BY tr.name",
            p, p
        );
        let rows = self.q(&sql).await?;
        Ok(rows.iter().map(|r| object_from_row(r)).collect())
    }

    async fn list_database_indexes(&self, database: &str) -> Result<Vec<DatabaseIndexInfo>> {
        let p = self.p(database);
        let sql = format!(
            "SELECT ix.name AS name, tbl.name AS table_name, c.name AS column_name, \
             ix.is_unique AS is_unique, ix.is_primary_key AS is_primary, ix.type_desc AS index_type \
             FROM {}sys.indexes ix \
             JOIN {}sys.index_columns ic ON ix.object_id = ic.object_id AND ix.index_id = ic.index_id \
             JOIN {}sys.columns c ON ic.object_id = c.object_id AND ic.column_id = c.column_id \
             JOIN {}sys.tables tbl ON ix.object_id = tbl.object_id \
             WHERE ix.name IS NOT NULL \
             ORDER BY tbl.name, ix.name, ic.key_ordinal",
            p, p, p, p
        );
        let rows = self.q(&sql).await?;
        Ok(rows
            .iter()
            .map(|r| DatabaseIndexInfo {
                name: cell(r, 0),
                table_name: cell(r, 1),
                column_name: cell(r, 2),
                is_unique: cell_bool(r, 3),
                is_primary: cell_bool(r, 4),
                index_type: opt_cell(r, 5),
            })
            .collect())
    }

    // ===== 实例级 =====

    async fn list_users(&self) -> Result<Vec<DbUserInfo>> {
        let rows = self
            .q(
                "SELECT ISNULL(name,'') AS user_name, '' AS host_name, ISNULL(type_desc,'') AS plugin_txt, \
                 is_disabled AS account_locked, '' AS password_expired, '' AS privileges_txt \
                 FROM sys.server_principals \
                 WHERE type IN ('S','U','G','E','X') ORDER BY name",
            )
            .await?;
        Ok(rows
            .iter()
            .map(|r| DbUserInfo {
                user: cell(r, 0),
                host: String::new(),
                plugin: opt_cell(r, 2),
                account_locked: Some(cell_bool(r, 3)),
                password_expired: None,
                privileges: None,
            })
            .collect())
    }

    /// 管理员 = `sysadmin` 固定服务器角色成员
    async fn list_admins(&self) -> Result<Vec<DbAdminInfo>> {
        let rows = self
            .q(
                "SELECT ISNULL(sp.name,'') AS user_name, '' AS host_name, ISNULL(sp.type_desc,'') AS plugin_txt \
                 FROM sys.server_principals sp \
                 JOIN sys.server_role_members srm ON sp.principal_id = srm.member_principal_id \
                 JOIN sys.server_principals rol ON srm.role_principal_id = rol.principal_id \
                 WHERE rol.name = 'sysadmin' ORDER BY sp.name",
            )
            .await?;
        Ok(rows
            .iter()
            .map(|r| DbAdminInfo {
                user: cell(r, 0),
                host: String::new(),
                plugin: opt_cell(r, 2),
                reason: "sysadmin".to_string(),
                privileges: Some("sysadmin 固定服务器角色成员".to_string()),
            })
            .collect())
    }

    async fn show_status(&self, scope: Scope) -> Result<Vec<KeyValueRow>> {
        let sql = match scope {
            // 会话状态：当前连接会话
            Scope::Session => {
                "SELECT kv.[name], kv.[value] FROM sys.dm_exec_sessions s \
                 CROSS APPLY (VALUES \
                   ('session_id', CONVERT(varchar(64), s.session_id)), \
                   ('login_name', ISNULL(s.login_name, '')), \
                   ('host_name', ISNULL(s.host_name, '')), \
                   ('program_name', ISNULL(s.program_name, '')), \
                   ('status', ISNULL(s.status, '')), \
                   ('login_time', CONVERT(varchar(64), s.login_time, 120)), \
                   ('cpu_time', CONVERT(varchar(64), s.cpu_time)), \
                   ('memory_usage', CONVERT(varchar(64), s.memory_usage)), \
                   ('reads', CONVERT(varchar(64), s.reads)), \
                   ('writes', CONVERT(varchar(64), s.writes)), \
                   ('transaction_isolation_level', CONVERT(varchar(64), s.transaction_isolation_level)) \
                 ) AS kv([name], [value]) \
                 WHERE s.session_id = @@SPID ORDER BY kv.[name]"
                    .to_string()
            }
            // 全局状态：实例级信息
            Scope::Global => {
                "SELECT kv.[name], kv.[value] FROM sys.dm_os_sys_info i \
                 CROSS APPLY (VALUES \
                   ('cpu_count', CONVERT(varchar(64), i.cpu_count)), \
                   ('physical_memory_kb', CONVERT(varchar(64), i.physical_memory_kb)), \
                   ('max_workers_count', CONVERT(varchar(64), i.max_workers_count)), \
                   ('ms_ticks', CONVERT(varchar(64), i.ms_ticks)), \
                   ('sqlserver_start_time', CONVERT(varchar(64), i.sqlserver_start_time, 120)) \
                 ) AS kv([name], [value]) ORDER BY kv.[name]"
                    .to_string()
            }
        };
        let rows = self.q(&sql).await?;
        Ok(rows
            .iter()
            .map(|r| KeyValueRow {
                name: cell(r, 0),
                value: cell(r, 1),
            })
            .collect())
    }

    async fn show_variables(&self, scope: Scope) -> Result<Vec<KeyValueRow>> {
        match scope {
            // SQL Server 只有实例级配置，会话级变量不显示（capabilities 里已标 false）
            Scope::Session => Err(unsupported_scope(self.db_type())),
            Scope::Global => {
                let rows = self
                    .q(
                        "SELECT name, CONVERT(varchar(4000), value_in_use) AS value_txt \
                         FROM sys.configurations ORDER BY name",
                    )
                    .await?;
                Ok(rows
                    .iter()
                    .map(|r| KeyValueRow {
                        name: cell(r, 0),
                        value: cell(r, 1),
                    })
                    .collect())
            }
        }
    }
}

fn unsupported_scope(db_type: &str) -> anyhow::Error {
    anyhow!("{} 没有会话级变量概念（配置是实例级的）", db_type)
}

/// 由统一的列下标构造 `DatabaseObjectInfo`（列序见各 SQL 的 SELECT 列表）
fn object_from_row(r: &[Value]) -> DatabaseObjectInfo {
    DatabaseObjectInfo {
        name: cell(r, 0),
        object_type: opt_cell(r, 1),
        database: opt_cell(r, 2),
        definer: opt_cell(r, 3),
        created: opt_cell(r, 4),
        modified: opt_cell(r, 5),
        comment: opt_cell(r, 6),
        status: opt_cell(r, 7),
        event: opt_cell(r, 8),
        table_name: opt_cell(r, 9),
    }
}

/// 拼 SQL Server 列类型（含长度/精度/小数位）
fn type_str(type_name: &str, max_len: i64, precision: i64, scale: i64) -> String {
    let tn = type_name.to_lowercase();
    match tn.as_str() {
        "varchar" | "char" | "nvarchar" | "nchar" | "varbinary" | "binary" => {
            let len = if max_len < 0 { 0 } else { max_len };
            if tn.starts_with('n') {
                format!("{}({})", type_name, len / 2)
            } else {
                format!("{}({})", type_name, len)
            }
        }
        "decimal" | "numeric" => format!("{}({},{})", type_name, precision, scale),
        _ => type_name.to_string(),
    }
}
