/*! # PostgreSQL 目录实现

PG 与 MySQL 的概念差异在这里收敛掉，命令层看不到任何差异：

- **「数据库」下钻粒度**：PG 单条连接内可切换的是 **schema**，跨库查询做不到。
  因此 `list_databases` 列出的是当前库的**业务 schema**（排除 `pg_catalog` /
  `information_schema`），保证树能一路下钻可用；`list_tables` 按 `schemaname` 过滤。
- **事件**：PG 没有内建事件调度器（社区用 pg_cron / 外部调度），`capabilities().events = false`。
- **状态 / 变量**：无 `SHOW STATUS` 等价物，用最接近的替代——
  - 会话状态 → `pg_stat_activity` 当前后端
  - 全局状态 → `pg_stat_database` 当前库计数器
  - 会话变量 → `pg_settings.setting`（当前生效值）
  - 全局变量 → `pg_settings.boot_val`（配置文件/启动值）
- **管理员**：`rolsuper`（超级用户）或 `rolcreaterole`（能建角色＝能发权限）。
*/
use async_trait::async_trait;
use sqlx::postgres::{PgPool, PgRow};
use sqlx::Row;

use super::*;

pub struct PostgresCatalog {
    pool: PgPool,
}

impl PostgresCatalog {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    async fn rows(&self, sql: &'static str) -> Result<Vec<PgRow>> {
        Ok(sqlx::query(sql).fetch_all(&self.pool).await?)
    }

    async fn rows_one(&self, sql: &'static str, p1: &str) -> Result<Vec<PgRow>> {
        Ok(sqlx::query(sql).bind(p1.to_string()).fetch_all(&self.pool).await?)
    }

    async fn rows_two(&self, sql: &'static str, p1: &str, p2: &str) -> Result<Vec<PgRow>> {
        Ok(sqlx::query(sql)
            .bind(p1.to_string())
            .bind(p2.to_string())
            .fetch_all(&self.pool)
            .await?)
    }

    /// 名-值两列查询（状态 / 变量）
    async fn key_values(&self, sql: &'static str) -> Result<Vec<KeyValueRow>> {
        let rows = self.rows(sql).await?;
        Ok(rows
            .iter()
            .map(|r| KeyValueRow {
                name: r.try_get::<String, _>("name").unwrap_or_default(),
                value: r.try_get::<String, _>("value").unwrap_or_default(),
            })
            .collect())
    }
}

fn s(row: &PgRow, col: &str) -> Option<String> {
    row.try_get::<String, _>(col).ok().filter(|v| !v.is_empty())
}

fn req(row: &PgRow, col: &str) -> String {
    row.try_get::<String, _>(col).unwrap_or_default()
}

fn b(row: &PgRow, col: &str) -> bool {
    row.try_get::<bool, _>(col).unwrap_or(false)
}

#[async_trait]
impl CatalogProvider for PostgresCatalog {
    fn db_type(&self) -> &'static str {
        "PostgreSQL"
    }

    fn dialect(&self) -> Dialect {
        Dialect::PostgreSQL
    }

    fn capabilities(&self) -> CatalogCapabilities {
        CatalogCapabilities {
            views: true,
            procedures: true,
            functions: true,
            triggers: true,
            events: false, // PG 无内建事件调度器
            database_indexes: true,
            users: true,
            admins: true,
            session_status: true,
            global_status: true,
            session_variables: true,
            global_variables: true,
            object_source: true,
        }
    }

    async fn list_databases(&self) -> Result<Vec<DatabaseInfo>> {
        // 见文件头：PG 单连接内可下钻的是 schema，故这里列出业务 schema
        let rows = self
            .rows(
                "SELECT schema_name AS name FROM information_schema.schemata \
                 WHERE schema_name NOT LIKE 'pg\\_%' AND schema_name <> 'information_schema' \
                 ORDER BY schema_name",
            )
            .await?;
        Ok(rows
            .iter()
            .map(|r| DatabaseInfo {
                name: req(r, "name"),
                charset: None,
                collation: None,
            })
            .collect())
    }

    async fn list_tables(&self, database: &str) -> Result<Vec<TableInfo>> {
        let rows = self
            .rows_one(
                "SELECT tablename AS name FROM pg_tables \
                 WHERE schemaname = COALESCE(NULLIF($1, ''), 'public') ORDER BY tablename",
                database,
            )
            .await?;
        Ok(rows
            .iter()
            .map(|r| TableInfo {
                name: req(r, "name"),
                engine: None,
                row_count: None,
                comment: None,
                data_length: None,
            })
            .collect())
    }

    async fn table_columns(&self, database: &str, table: &str) -> Result<Vec<ColumnInfo>> {
        let rows = self
            .rows_two(
                "SELECT c.column_name AS name, c.data_type AS data_type, c.is_nullable AS is_nullable, \
                 c.column_default AS default_value, \
                 CASE WHEN pk.column_name IS NOT NULL THEN 'PRI' ELSE '' END AS column_key, \
                 CAST(c.ordinal_position AS BIGINT) AS ord, c.is_identity AS is_identity, \
                 c.is_generated AS is_generated, \
                 CAST(c.numeric_precision AS BIGINT) AS num_prec, CAST(c.numeric_scale AS BIGINT) AS num_scale, \
                 CAST(c.character_maximum_length AS BIGINT) AS char_len \
                 FROM information_schema.columns c \
                 LEFT JOIN (SELECT ku.column_name FROM information_schema.table_constraints tc \
                   JOIN information_schema.key_column_usage ku ON tc.constraint_name = ku.constraint_name \
                   WHERE tc.constraint_type = 'PRIMARY KEY' AND tc.table_name = $2) pk \
                   ON c.column_name = pk.column_name \
                 WHERE c.table_schema = COALESCE(NULLIF($1, ''), 'public') AND c.table_name = $2 \
                 ORDER BY c.ordinal_position",
                database,
                table,
            )
            .await?;
        Ok(rows
            .iter()
            .map(|r| {
                let key = s(r, "column_key");
                let nullable = s(r, "is_nullable");
                let identity = s(r, "is_identity").unwrap_or_else(|| "NO".to_string());
                let generated = s(r, "is_generated").unwrap_or_else(|| "NEVER".to_string());
                ColumnInfo {
                    name: req(r, "name"),
                    data_type: req(r, "data_type"),
                    is_nullable: nullable.map(|v| v == "YES").unwrap_or(true),
                    is_primary_key: key.as_deref() == Some("PRI"),
                    default_value: s(r, "default_value"),
                    comment: None,
                    ordinal_position: r.try_get::<i64, _>("ord").unwrap_or(0) as i32,
                    is_auto_increment: identity.eq_ignore_ascii_case("YES"),
                    is_generated: generated.eq_ignore_ascii_case("ALWAYS"),
                    numeric_precision: r.try_get::<i64, _>("num_prec").ok().map(|v| v as i32),
                    numeric_scale: r.try_get::<i64, _>("num_scale").ok().map(|v| v as i32),
                    character_max_length: r.try_get::<i64, _>("char_len").ok(),
                }
            })
            .collect())
    }

    async fn table_indexes(&self, database: &str, table: &str) -> Result<Vec<IndexInfo>> {
        let rows = self
            .rows_two(
                "SELECT i.relname AS name, a.attname AS column_name, ix.indisunique AS is_unique, \
                 ix.indisprimary AS is_primary, am.amname AS index_type \
                 FROM pg_class t \
                 JOIN pg_namespace n ON n.oid = t.relnamespace \
                 JOIN pg_index ix ON t.oid = ix.indrelid \
                 JOIN pg_class i ON i.oid = ix.indexrelid \
                 JOIN pg_attribute a ON a.attrelid = t.oid AND a.attnum = ANY(ix.indkey) \
                 JOIN pg_am am ON i.relam = am.oid \
                 WHERE t.relkind = 'r' AND n.nspname = COALESCE(NULLIF($1, ''), 'public') AND t.relname = $2 \
                 ORDER BY i.relname, a.attnum",
                database,
                table,
            )
            .await?;
        Ok(rows
            .iter()
            .map(|r| IndexInfo {
                name: req(r, "name"),
                column_name: req(r, "column_name"),
                is_unique: b(r, "is_unique"),
                is_primary: b(r, "is_primary"),
                index_type: s(r, "index_type"),
            })
            .collect())
    }

    async fn table_schema(&self, database: &str, table: &str) -> Result<String> {
        // PG 没有 SHOW CREATE TABLE，按列信息拼一份可读 DDL
        let rows = self
            .rows_two(
                "SELECT column_name, data_type, character_maximum_length, is_nullable, column_default \
                 FROM information_schema.columns \
                 WHERE table_schema = COALESCE(NULLIF($1, ''), 'public') AND table_name = $2 \
                 ORDER BY ordinal_position",
                database,
                table,
            )
            .await?;
        let mut ddl = format!(
            "CREATE TABLE {}.{} (\n",
            self.quote_ident(if database.is_empty() { "public" } else { database }),
            self.quote_ident(table)
        );
        for (i, row) in rows.iter().enumerate() {
            let col_name: String = req(row, "column_name");
            let data_type: String = req(row, "data_type");
            let max_len: Option<i32> = row.try_get("character_maximum_length").ok();
            let nullable: String = req(row, "is_nullable");
            let default: Option<String> = s(row, "column_default");
            let type_str = match max_len {
                Some(len) => format!("{}({})", data_type, len),
                None => data_type,
            };
            let null_str = if nullable == "YES" { "NULL" } else { "NOT NULL" };
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
        let rows = self
            .rows_one(
                "SELECT v.viewname AS name, 'VIEW' AS object_type, v.schemaname AS database_name, \
                 COALESCE(v.viewowner::text, '') AS definer, '' AS created, '' AS modified, \
                 '' AS comment_txt, '' AS status_txt, '' AS event_txt, '' AS table_name \
                 FROM pg_views v \
                 WHERE v.schemaname = COALESCE(NULLIF($1, ''), 'public') ORDER BY v.viewname",
                database,
            )
            .await?;
        Ok(rows.iter().map(object_from_row).collect())
    }

    async fn list_procedures(&self, database: &str) -> Result<Vec<DatabaseObjectInfo>> {
        let rows = self
            .rows_one(
                "SELECT DISTINCT r.routine_name AS name, r.routine_type AS object_type, \
                 r.routine_schema AS database_name, \
                 COALESCE(pg_get_userbyid(p.proowner), '') AS definer, \
                 '' AS created, '' AS modified, '' AS comment_txt, '' AS status_txt, \
                 '' AS event_txt, '' AS table_name \
                 FROM information_schema.routines r \
                 JOIN pg_namespace n ON n.nspname = r.routine_schema \
                 JOIN pg_proc p ON p.pronamespace = n.oid AND p.proname = r.routine_name \
                 WHERE r.routine_schema = COALESCE(NULLIF($1, ''), 'public') \
                   AND r.routine_type = 'PROCEDURE' \
                 ORDER BY r.routine_name",
                database,
            )
            .await?;
        Ok(rows.iter().map(object_from_row).collect())
    }

    async fn list_functions(&self, database: &str) -> Result<Vec<DatabaseObjectInfo>> {
        let rows = self
            .rows_one(
                "SELECT DISTINCT r.routine_name AS name, r.routine_type AS object_type, \
                 r.routine_schema AS database_name, \
                 COALESCE(pg_get_userbyid(p.proowner), '') AS definer, \
                 '' AS created, '' AS modified, '' AS comment_txt, '' AS status_txt, \
                 '' AS event_txt, '' AS table_name \
                 FROM information_schema.routines r \
                 JOIN pg_namespace n ON n.nspname = r.routine_schema \
                 JOIN pg_proc p ON p.pronamespace = n.oid AND p.proname = r.routine_name \
                 WHERE r.routine_schema = COALESCE(NULLIF($1, ''), 'public') \
                   AND r.routine_type = 'FUNCTION' \
                 ORDER BY r.routine_name",
                database,
            )
            .await?;
        Ok(rows.iter().map(|r| object_from_row(r)).collect())
    }

    /// 取对象源码。PG 的三个内置函数各司其职。
    ///
    /// **同名例程可能有多个重载**（参数类型不同），这里把全部定义返回、空行分隔 ——
    /// 漏给一份，用户就会以为另外那个版本的函数不存在。
    async fn object_source(
        &self,
        kind: ObjectSourceKind,
        database: &str,
        name: &str,
    ) -> Result<String> {
        // pg_get_viewdef 只给 SELECT 部分，CREATE 头要自己补；
        // pg_get_functiondef / pg_get_triggerdef 自带完整 CREATE 语句，不要再添头。
        let sql: &'static str = match kind {
            ObjectSourceKind::View => {
                "SELECT 'CREATE OR REPLACE VIEW ' || quote_ident(c.relname) || E' AS\n' \
                 || pg_get_viewdef(c.oid, true) AS src \
                 FROM pg_class c JOIN pg_namespace n ON n.oid = c.relnamespace \
                 WHERE c.relkind IN ('v', 'm') AND c.relname = $2 \
                   AND n.nspname = COALESCE(NULLIF($1, ''), 'public')"
            }
            ObjectSourceKind::Procedure | ObjectSourceKind::Function => {
                "SELECT pg_get_functiondef(p.oid) AS src \
                 FROM pg_proc p JOIN pg_namespace n ON n.oid = p.pronamespace \
                 WHERE p.proname = $2 AND n.nspname = COALESCE(NULLIF($1, ''), 'public') \
                 ORDER BY p.oid"
            }
            ObjectSourceKind::Trigger => {
                "SELECT pg_get_triggerdef(t.oid) AS src \
                 FROM pg_trigger t \
                 JOIN pg_class c ON c.oid = t.tgrelid \
                 JOIN pg_namespace n ON n.oid = c.relnamespace \
                 WHERE t.tgname = $2 AND NOT t.tgisinternal \
                   AND n.nspname = COALESCE(NULLIF($1, ''), 'public')"
            }
        };
        let rows = self.rows_two(sql, database, name).await?;
        let srcs: Vec<String> = rows
            .iter()
            .map(|r| req(r, "src"))
            .filter(|v| !v.trim().is_empty())
            .collect();
        if srcs.is_empty() {
            return Err(anyhow::anyhow!("未找到{}：{}", kind.label(), name));
        }
        Ok(srcs.join("\n\n"))
    }

    async fn list_triggers(&self, database: &str) -> Result<Vec<DatabaseObjectInfo>> {
        let rows = self
            .rows_one(
                "SELECT t.trigger_name AS name, 'TRIGGER' AS object_type, t.trigger_schema AS database_name, \
                 '' AS definer, '' AS created, '' AS modified, '' AS comment_txt, '' AS status_txt, \
                 COALESCE(t.event_manipulation, '') AS event_txt, \
                 COALESCE(t.event_object_table, '') AS table_name \
                 FROM information_schema.triggers t \
                 WHERE t.trigger_schema = COALESCE(NULLIF($1, ''), 'public') ORDER BY t.trigger_name",
                database,
            )
            .await?;
        Ok(rows.iter().map(object_from_row).collect())
    }

    async fn list_database_indexes(&self, database: &str) -> Result<Vec<DatabaseIndexInfo>> {
        let rows = self
            .rows_one(
                "SELECT t.relname AS table_name, i.relname AS name, a.attname AS column_name, \
                 ix.indisunique AS is_unique, ix.indisprimary AS is_primary, am.amname AS index_type \
                 FROM pg_class t \
                 JOIN pg_namespace n ON n.oid = t.relnamespace \
                 JOIN pg_index ix ON t.oid = ix.indrelid \
                 JOIN pg_class i ON i.oid = ix.indexrelid \
                 JOIN pg_attribute a ON a.attrelid = t.oid AND a.attnum = ANY(ix.indkey) \
                 JOIN pg_am am ON i.relam = am.oid \
                 WHERE t.relkind = 'r' AND n.nspname = COALESCE(NULLIF($1, ''), 'public') \
                 ORDER BY t.relname, i.relname, a.attnum",
                database,
            )
            .await?;
        Ok(rows
            .iter()
            .map(|r| DatabaseIndexInfo {
                name: req(r, "name"),
                table_name: req(r, "table_name"),
                column_name: req(r, "column_name"),
                is_unique: b(r, "is_unique"),
                is_primary: b(r, "is_primary"),
                index_type: s(r, "index_type"),
            })
            .collect())
    }

    // ===== 实例级 =====

    async fn list_users(&self) -> Result<Vec<DbUserInfo>> {
        let rows = self
            .rows(
                "SELECT r.rolname AS user, '' AS host, '' AS plugin, \
                 r.rolcanlogin AS can_login, r.rolsuper AS is_super, \
                 r.rolcreaterole AS can_create_role, r.rolcreatedb AS can_create_db, \
                 r.rolreplication AS can_replicate \
                 FROM pg_roles r WHERE r.rolname NOT LIKE 'pg\\_%' ORDER BY r.rolname",
            )
            .await?;
        Ok(rows
            .iter()
            .map(|r| DbUserInfo {
                user: req(r, "user"),
                host: String::new(),
                plugin: None,
                account_locked: None,
                password_expired: None,
                privileges: Some(role_privs(r)),
            })
            .collect())
    }

    async fn list_admins(&self) -> Result<Vec<DbAdminInfo>> {
        let rows = self
            .rows(
                "SELECT r.rolname AS user, '' AS host, '' AS plugin, \
                 r.rolcanlogin AS can_login, r.rolsuper AS is_super, \
                 r.rolcreaterole AS can_create_role, r.rolcreatedb AS can_create_db, \
                 r.rolreplication AS can_replicate \
                 FROM pg_roles r \
                 WHERE r.rolname NOT LIKE 'pg\\_%' AND (r.rolsuper OR r.rolcreaterole) \
                 ORDER BY r.rolname",
            )
            .await?;
        Ok(rows
            .iter()
            .map(|r| {
                let su = b(r, "is_super");
                let cr = b(r, "can_create_role");
                let mut reasons = Vec::new();
                if su {
                    reasons.push("SUPERUSER");
                }
                if cr {
                    reasons.push("CREATEROLE");
                }
                DbAdminInfo {
                    user: req(r, "user"),
                    host: String::new(),
                    plugin: None,
                    reason: reasons.join(" / "),
                    privileges: Some(role_privs(r)),
                }
            })
            .collect())
    }

    async fn show_status(&self, scope: Scope) -> Result<Vec<KeyValueRow>> {
        match scope {
            // 会话状态：当前后端连接的活动信息
            Scope::Session => self
                .key_values(
                    "SELECT kv.name, kv.value FROM pg_stat_activity a \
                     CROSS JOIN LATERAL (VALUES \
                       ('pid', a.pid::text), ('backend_type', COALESCE(a.backend_type,'')), \
                       ('client_addr', COALESCE(a.client_addr::text,'')), \
                       ('client_port', COALESCE(a.client_port::text,'')), \
                       ('state', COALESCE(a.state,'')), \
                       ('query_start', COALESCE(a.query_start::text,'')), \
                       ('state_change', COALESCE(a.state_change::text,'')), \
                       ('wait_event_type', COALESCE(a.wait_event_type,'')), \
                       ('wait_event', COALESCE(a.wait_event,'')), \
                       ('application_name', COALESCE(a.application_name,'')) \
                     ) AS kv(name, value) \
                     WHERE a.pid = pg_backend_pid() ORDER BY kv.name",
                )
                .await,
            // 全局状态：当前库的累计计数器
            Scope::Global => self
                .key_values(
                    "SELECT kv.name, kv.value FROM pg_stat_database d \
                     CROSS JOIN LATERAL (VALUES \
                       ('numbackends', d.numbackends::text), ('xact_commit', d.xact_commit::text), \
                       ('xact_rollback', d.xact_rollback::text), ('blks_read', d.blks_read::text), \
                       ('blks_hit', d.blks_hit::text), ('tup_returned', d.tup_returned::text), \
                       ('tup_fetched', d.tup_fetched::text), ('tup_inserted', d.tup_inserted::text), \
                       ('tup_updated', d.tup_updated::text), ('tup_deleted', d.tup_deleted::text), \
                       ('conflicts', d.conflicts::text), ('deadlocks', d.deadlocks::text), \
                       ('temp_files', d.temp_files::text), ('temp_bytes', d.temp_bytes::text) \
                     ) AS kv(name, value) \
                     WHERE d.datname = current_database() ORDER BY kv.name",
                )
                .await,
        }
    }

    async fn show_variables(&self, scope: Scope) -> Result<Vec<KeyValueRow>> {
        match scope {
            // 会话变量：当前生效值
            Scope::Session => self
                .key_values(
                    "SELECT name, COALESCE(setting,'') AS value FROM pg_settings ORDER BY name",
                )
                .await,
            // 全局变量：启动/配置文件里的值（boot_val）
            Scope::Global => self
                .key_values(
                    "SELECT name, COALESCE(boot_val,'') AS value FROM pg_settings ORDER BY name",
                )
                .await,
        }
    }
}

/// 角色权限拼成可读串（供人工核对管理员判定是否准确）
fn role_privs(r: &PgRow) -> String {
    let mut parts = Vec::new();
    if b(r, "is_super") {
        parts.push("SUPERUSER");
    }
    if b(r, "can_create_db") {
        parts.push("CREATEDB");
    }
    if b(r, "can_create_role") {
        parts.push("CREATEROLE");
    }
    if b(r, "can_replicate") {
        parts.push("REPLICATION");
    }
    if b(r, "can_login") {
        parts.push("LOGIN");
    }
    if parts.is_empty() {
        "NONE".to_string()
    } else {
        parts.join(", ")
    }
}

fn object_from_row(r: &PgRow) -> DatabaseObjectInfo {
    DatabaseObjectInfo {
        name: req(r, "name"),
        object_type: s(r, "object_type"),
        database: s(r, "database_name"),
        definer: s(r, "definer"),
        created: s(r, "created"),
        modified: s(r, "modified"),
        comment: s(r, "comment_txt"),
        status: s(r, "status_txt"),
        event: s(r, "event_txt"),
        table_name: s(r, "table_name"),
    }
}
