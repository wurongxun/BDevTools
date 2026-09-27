/*! # Oracle 目录实现（走系统 ODBC）

要点：

- **「数据库」= schema（owner）**：Oracle 没有 MySQL 那种多库概念，树上的库节点即 `owner`，
  所有 `all_*` 视图都按 `OWNER = 大写库名` 过滤（未加引号的标识符在 Oracle 里存为大写）。
- **事件不支持**：Oracle 的调度是 `DBMS_SCHEDULER`，与 MySQL EVENT 语义差别过大，
  `capabilities().events = false`。
- **管理员判定带多级回退**：`DBA` 角色 → `V$PWFILE_USERS.SYSDBA` → 内置账号兜底。
  前两者都需要额外权限，普通账号看不到；逐级回退保证节点不会直接打不开。
- **状态 / 变量**：`V$MYSTAT`/`V$SYSSTAT`（会话/全局统计）、`V$PARAMETER`（参数，
  会话级只列 `ISSES_MODIFIABLE` 的项）。
*/
use async_trait::async_trait;
use serde_json::Value;
use std::sync::Arc;

use super::*;
use crate::plugins::database::odbc::OraclePool;

const MAX_ROWS: usize = 10000;

pub struct OracleCatalog {
    pool: Arc<OraclePool>,
}

impl OracleCatalog {
    pub fn new(pool: Arc<OraclePool>) -> Self {
        Self { pool }
    }

    async fn q(&self, sql: &str) -> Result<Vec<Vec<Value>>> {
        let (_, rows) = self.pool.query_rows("", sql, MAX_ROWS).await?;
        Ok(rows)
    }

    /// 字符串字面量（Oracle 里未加引号的标识符存为大写，故统一大写后转义）
    fn owner_literal(&self, database: &str) -> String {
        self.escape_literal(&database.to_uppercase())
    }
}

fn cell(row: &[Value], i: usize) -> String {
    row.get(i).and_then(str_of).unwrap_or_default()
}

fn opt_cell(row: &[Value], i: usize) -> Option<String> {
    row.get(i).and_then(str_of).filter(|v| !v.is_empty())
}

#[async_trait]
impl CatalogProvider for OracleCatalog {
    fn db_type(&self) -> &'static str {
        "Oracle"
    }

    fn dialect(&self) -> Dialect {
        Dialect::Oracle
    }

    fn capabilities(&self) -> CatalogCapabilities {
        CatalogCapabilities {
            views: true,
            procedures: true,
            triggers: true,
            events: false, // 调度是 DBMS_SCHEDULER，语义不同
            database_indexes: true,
            users: true,
            admins: true,
            session_status: true,
            global_status: true,
            session_variables: true,
            global_variables: true,
        }
    }

    async fn list_databases(&self) -> Result<Vec<DatabaseInfo>> {
        let rows = self
            .q("SELECT username AS name FROM all_users ORDER BY username")
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
            "SELECT table_name AS name, NULL AS engine, num_rows AS row_count, \
             NULL AS comment, NULL AS data_length \
             FROM all_tables WHERE owner = '{}' ORDER BY table_name",
            self.owner_literal(database)
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
        let owner = self.owner_literal(database);
        let tbl = self.escape_literal(&table.to_uppercase());
        let sql = format!(
            "SELECT c.column_name AS name, c.data_type AS data_type, c.nullable AS is_nullable, \
             c.data_default AS default_value, \
             CASE WHEN pk.column_name IS NOT NULL THEN 'PRI' ELSE '' END AS column_key, \
             '' AS comment_txt \
             FROM all_tab_columns c \
             LEFT JOIN (SELECT acc.column_name FROM all_cons_columns acc \
                JOIN all_constraints ac ON acc.constraint_name = ac.constraint_name \
                WHERE ac.constraint_type = 'P' AND ac.owner = '{}' AND ac.table_name = '{}') pk \
                ON c.column_name = pk.column_name \
             WHERE c.owner = '{}' AND c.table_name = '{}' ORDER BY c.column_id",
            owner, tbl, owner, tbl
        );
        let rows = self.q(&sql).await?;
        Ok(rows
            .iter()
            .map(|r| {
                let nullable = cell(r, 2);
                ColumnInfo {
                    name: cell(r, 0),
                    data_type: cell(r, 1),
                    is_nullable: nullable != "N",
                    is_primary_key: cell(r, 4) == "PRI",
                    default_value: opt_cell(r, 3),
                    comment: opt_cell(r, 5),
                }
            })
            .collect())
    }

    async fn table_indexes(&self, database: &str, table: &str) -> Result<Vec<IndexInfo>> {
        let owner = self.owner_literal(database);
        let tbl = self.escape_literal(&table.to_uppercase());
        let sql = format!(
            "SELECT i.index_name AS name, ic.column_name AS column_name, i.uniqueness AS is_unique, \
             CASE WHEN c.constraint_type = 'P' THEN 1 ELSE 0 END AS is_primary, \
             i.index_type AS index_type \
             FROM all_indexes i \
             JOIN all_ind_columns ic ON i.index_name = ic.index_name AND i.table_owner = ic.table_owner \
             LEFT JOIN all_constraints c ON c.constraint_name = i.index_name AND c.constraint_type = 'P' \
             WHERE i.table_owner = '{}' AND i.table_name = '{}' \
             ORDER BY i.index_name, ic.column_position",
            owner, tbl
        );
        let rows = self.q(&sql).await?;
        Ok(rows
            .iter()
            .map(|r| IndexInfo {
                name: cell(r, 0),
                column_name: cell(r, 1),
                is_unique: cell(r, 2).eq_ignore_ascii_case("UNIQUE"),
                is_primary: r.get(3).and_then(num_of).map(|n| n != 0).unwrap_or(false),
                index_type: opt_cell(r, 4),
            })
            .collect())
    }

    async fn table_schema(&self, database: &str, table: &str) -> Result<String> {
        let owner = self.owner_literal(database);
        let tbl = self.escape_literal(&table.to_uppercase());
        let sql = format!(
            "SELECT column_name, data_type, data_length, data_precision, data_scale, nullable, data_default \
             FROM all_tab_columns WHERE owner = '{}' AND table_name = '{}' ORDER BY column_id",
            owner, tbl
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
            let data_len = r.get(2).and_then(num_of).unwrap_or(0);
            let precision = r.get(3).and_then(num_of);
            let scale = r.get(4).and_then(num_of);
            let nullable = cell(r, 5) != "N";
            let default = opt_cell(r, 6);
            let type_str = type_str(&type_name, data_len, precision, scale);
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
        let owner = self.owner_literal(database);
        let sql = format!(
            "SELECT view_name AS name, 'VIEW' AS object_type, owner AS database_name, \
             '' AS definer, '' AS created, '' AS modified, '' AS comment_txt, \
             '' AS status_txt, '' AS event_txt, '' AS table_name \
             FROM all_views WHERE owner = '{}' ORDER BY view_name",
            owner
        );
        let rows = self.q(&sql).await?;
        Ok(rows.iter().map(|r| object_from_row(r)).collect())
    }

    async fn list_procedures(&self, database: &str) -> Result<Vec<DatabaseObjectInfo>> {
        let owner = self.owner_literal(database);
        let sql = format!(
            "SELECT object_name AS name, object_type AS object_type, owner AS database_name, \
             '' AS definer, TO_CHAR(created, 'YYYY-MM-DD HH24:MI:SS') AS created, \
             TO_CHAR(last_ddl_time, 'YYYY-MM-DD HH24:MI:SS') AS modified, \
             '' AS comment_txt, status AS status_txt, '' AS event_txt, '' AS table_name \
             FROM all_objects \
             WHERE owner = '{}' AND object_type IN ('PROCEDURE', 'FUNCTION') \
             ORDER BY object_name",
            owner
        );
        let rows = self.q(&sql).await?;
        Ok(rows.iter().map(|r| object_from_row(r)).collect())
    }

    async fn list_triggers(&self, database: &str) -> Result<Vec<DatabaseObjectInfo>> {
        let owner = self.owner_literal(database);
        let sql = format!(
            "SELECT trigger_name AS name, 'TRIGGER' AS object_type, owner AS database_name, \
             '' AS definer, '' AS created, '' AS modified, '' AS comment_txt, \
             status AS status_txt, triggering_event AS event_txt, table_name AS table_name \
             FROM all_triggers WHERE owner = '{}' ORDER BY trigger_name",
            owner
        );
        let rows = self.q(&sql).await?;
        Ok(rows.iter().map(|r| object_from_row(r)).collect())
    }

    async fn list_database_indexes(&self, database: &str) -> Result<Vec<DatabaseIndexInfo>> {
        let owner = self.owner_literal(database);
        let sql = format!(
            "SELECT i.index_name AS name, i.table_name AS table_name, ic.column_name AS column_name, \
             i.uniqueness AS is_unique, \
             CASE WHEN c.constraint_type = 'P' THEN 1 ELSE 0 END AS is_primary, \
             i.index_type AS index_type \
             FROM all_indexes i \
             JOIN all_ind_columns ic ON i.index_name = ic.index_name AND i.table_owner = ic.table_owner \
             LEFT JOIN all_constraints c ON c.constraint_name = i.index_name AND c.constraint_type = 'P' \
             WHERE i.table_owner = '{}' ORDER BY i.table_name, i.index_name, ic.column_position",
            owner
        );
        let rows = self.q(&sql).await?;
        Ok(rows
            .iter()
            .map(|r| DatabaseIndexInfo {
                name: cell(r, 0),
                table_name: cell(r, 1),
                column_name: cell(r, 2),
                is_unique: cell(r, 3).eq_ignore_ascii_case("UNIQUE"),
                is_primary: r.get(4).and_then(num_of).map(|n| n != 0).unwrap_or(false),
                index_type: opt_cell(r, 5),
            })
            .collect())
    }

    // ===== 实例级 =====

    async fn list_users(&self) -> Result<Vec<DbUserInfo>> {
        // all_users 对普通账号也可读；账号状态需要 DBA_USERS（权限不足时留空）
        let rows = self
            .q("SELECT username AS user_name, created AS created_txt FROM all_users ORDER BY username")
            .await?;
        Ok(rows
            .iter()
            .map(|r| DbUserInfo {
                user: cell(r, 0),
                host: String::new(),
                plugin: None,
                account_locked: None,
                password_expired: None,
                privileges: opt_cell(r, 1).map(|c| format!("创建于 {}", c)),
            })
            .collect())
    }

    /// 管理员：DBA 角色 → SYSDBA → 内置账号兜底（逐级回退，见文件头说明）
    async fn list_admins(&self) -> Result<Vec<DbAdminInfo>> {
        // ① 已授予 DBA 角色（需要 DBA 权限才能查）
        let by_role = self
            .q(
                "SELECT grantee AS user_name, 'DBA 角色' AS reason_txt \
                 FROM dba_role_privs WHERE granted_role = 'DBA' ORDER BY grantee",
            )
            .await;
        if let Ok(rows) = by_role {
            if !rows.is_empty() {
                return Ok(rows
                    .iter()
                    .map(|r| DbAdminInfo {
                        user: cell(r, 0),
                        host: String::new(),
                        plugin: None,
                        reason: cell(r, 1),
                        privileges: Some("已授予 DBA 角色".to_string()),
                    })
                    .collect());
            }
        }

        // ② 有 SYSDBA 权限（口令文件里登记）
        let by_sysdba = self
            .q(
                "SELECT username AS user_name, 'SYSDBA' AS reason_txt \
                 FROM v$pwfile_users WHERE sysdba = 'TRUE' ORDER BY username",
            )
            .await;
        if let Ok(rows) = by_sysdba {
            if !rows.is_empty() {
                return Ok(rows
                    .iter()
                    .map(|r| DbAdminInfo {
                        user: cell(r, 0),
                        host: String::new(),
                        plugin: None,
                        reason: cell(r, 1),
                        privileges: Some("SYSDBA".to_string()),
                    })
                    .collect());
            }
        }

        // ③ 兜底：内置管理员账号（无上述视图权限时至少给出常见账号）
        let rows = self
            .q(
                "SELECT username AS user_name FROM all_users \
                 WHERE username IN ('SYS', 'SYSTEM') ORDER BY username",
            )
            .await?;
        Ok(rows
            .iter()
            .map(|r| DbAdminInfo {
                user: cell(r, 0),
                host: String::new(),
                plugin: None,
                reason: "内置管理员账号（无法读取角色/权限视图）".to_string(),
                privileges: None,
            })
            .collect())
    }

    async fn show_status(&self, scope: Scope) -> Result<Vec<KeyValueRow>> {
        let sql = match scope {
            // 会话统计
            Scope::Session => {
                "SELECT n.name, TO_CHAR(m.value) AS value_txt \
                 FROM v$mystat m JOIN v$statname n ON m.statistic# = n.statistic# \
                 ORDER BY n.name"
                    .to_string()
            }
            // 实例级统计
            Scope::Global => {
                "SELECT n.name, TO_CHAR(s.value) AS value_txt \
                 FROM v$sysstat s JOIN v$statname n ON s.statistic# = n.statistic# \
                 ORDER BY n.name"
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
        let sql = match scope {
            // 会话可修改的参数（Oracle 参数主要是实例级，这里只列会话可调项）
            Scope::Session => {
                "SELECT name, TO_CHAR(value) AS value_txt FROM v$parameter \
                 WHERE isses_modifiable = 'TRUE' ORDER BY name"
                    .to_string()
            }
            Scope::Global => {
                "SELECT name, TO_CHAR(value) AS value_txt FROM v$parameter ORDER BY name".to_string()
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
}

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

/// 拼 Oracle 列类型（含长度/精度/小数位）
fn type_str(type_name: &str, data_len: i64, precision: Option<i64>, scale: Option<i64>) -> String {
    let tn = type_name.to_lowercase();
    match tn.as_str() {
        "varchar2" | "char" | "nvarchar2" | "nchar" | "raw" => format!("{}({})", type_name, data_len),
        "number" => match (precision, scale) {
            (Some(p), Some(s)) if s > 0 => format!("NUMBER({},{})", p, s),
            (Some(p), _) => format!("NUMBER({})", p),
            _ => "NUMBER".to_string(),
        },
        _ => type_name.to_string(),
    }
}
