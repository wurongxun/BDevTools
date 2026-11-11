/*! # MySQL 目录实现

全部 MySQL 系统表 / `SHOW` 语句集中在本文件。约定：

- **能参数化的一律参数化**（`?` 绑定）；`SHOW ... IN` 这类只能拼标识符的走 `Dialect::quote_ident`。
- **版本兼容**：`mysql.user` 的字段在 5.7 / 8.0 之间有增删，因此取用户只查稳定字段
  （`user` / `host` / `plugin`），权限位只在管理员判定里用，失败则回退到「按 user=root 兜底」。
- 取列统一走 `s()`：NULL 与空串都收敛为 `None`，避免前端显示 `null`。
*/
use async_trait::async_trait;
use sqlx::mysql::{MySqlPool, MySqlRow};
use sqlx::Row;

use super::*;

pub struct MySqlCatalog {
    pool: MySqlPool,
}

impl MySqlCatalog {
    pub fn new(pool: MySqlPool) -> Self {
        Self { pool }
    }

    /// 执行查询并收集行（内部统一入口）
    async fn rows(&self, sql: &'static str) -> Result<Vec<MySqlRow>> {
        Ok(sqlx::query(sql).fetch_all(&self.pool).await?)
    }

    /// 执行带一个绑定参数的查询
    async fn rows_one(&self, sql: &'static str, p1: &str) -> Result<Vec<MySqlRow>> {
        Ok(sqlx::query(sql).bind(p1.to_string()).fetch_all(&self.pool).await?)
    }

    /// 执行带两个绑定参数的查询
    async fn rows_two(&self, sql: &'static str, p1: &str, p2: &str) -> Result<Vec<MySqlRow>> {
        Ok(sqlx::query(sql)
            .bind(p1.to_string())
            .bind(p2.to_string())
            .fetch_all(&self.pool)
            .await?)
    }

    /// MySQL 直到 8.0 都没有 `SHOW CREATE TRIGGER`，只能拿 `information_schema.TRIGGERS`
    /// 的字段拼回去。
    ///
    /// 拼出来的语句**保证看得懂、能照着重建**，但不保证与服务端内部表示逐字节一致
    /// （比如 sql_mode 这段服务端是会带上的，这里没写）。与其假装拿到了官方源码，
    /// 不如把「这是拼的」写进注释 —— 反正触发器的可执行主体 `ACTION_STATEMENT` 是真的。
    async fn trigger_source(&self, database: &str, name: &str) -> Result<String> {
        let rows = self
            .rows_two(
                "SELECT TRIGGER_NAME AS name, ACTION_TIMING AS timing, \
                 EVENT_MANIPULATION AS evt, EVENT_OBJECT_TABLE AS tbl, \
                 ACTION_ORIENTATION AS scope, ACTION_STATEMENT AS body, \
                 IFNULL(DEFINER,'') AS definer \
                 FROM information_schema.TRIGGERS \
                 WHERE TRIGGER_SCHEMA = ? AND TRIGGER_NAME = ?",
                database,
                name,
            )
            .await?;
        let r = rows.first().ok_or_else(|| anyhow::anyhow!("未找到触发器 {}", name))?;

        // DEFINER 形如 `'root'@'localhost'`，要按 SHOW CREATE 的写法拆成两段各自加反引号
        let head = match s(r, "definer") {
            Some(d) if !d.is_empty() => {
                let (u, h) = d.split_once('@').unwrap_or((d.as_str(), "%"));
                format!(
                    "CREATE DEFINER={}@{} ",
                    self.quote_ident(u),
                    self.quote_ident(h)
                )
            }
            _ => "CREATE ".to_string(),
        };
        let scope = if req(r, "scope").eq_ignore_ascii_case("STATEMENT") {
            "STATEMENT"
        } else {
            "ROW"
        };
        Ok(format!(
            "{}TRIGGER {} {} {} ON {}.{} FOR EACH {}\n{}",
            head,
            self.quote_ident(&req(r, "name")),
            req(r, "timing"),
            req(r, "evt"),
            self.quote_ident(database),
            self.quote_ident(&req(r, "tbl")),
            scope,
            req(r, "body")
        ))
    }
}

/// 取可空字符串：NULL / 空串都收敛为 None
fn s(row: &MySqlRow, col: &str) -> Option<String> {
    row.try_get::<String, _>(col).ok().filter(|v| !v.is_empty())
}

/// 取必填字符串
fn req(row: &MySqlRow, col: &str) -> String {
    row.try_get::<String, _>(col).unwrap_or_default()
}

/// MySQL 的 `'Y' / 'N'` 枚举转布尔
fn yn(v: Option<String>) -> Option<bool> {
    v.map(|x| matches!(x.as_str(), "Y" | "y" | "1"))
}

#[async_trait]
impl CatalogProvider for MySqlCatalog {
    fn db_type(&self) -> &'static str {
        "MySQL"
    }

    fn dialect(&self) -> Dialect {
        Dialect::MySQL
    }

    fn capabilities(&self) -> CatalogCapabilities {
        CatalogCapabilities {
            views: true,
            procedures: true,
            functions: true,
            triggers: true,
            events: true,
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
        let rows = self
            .rows(
                "SELECT SCHEMA_NAME AS name, DEFAULT_CHARACTER_SET_NAME AS charset, \
                 DEFAULT_COLLATION_NAME AS collation FROM information_schema.SCHEMATA ORDER BY SCHEMA_NAME",
            )
            .await?;
        Ok(rows
            .iter()
            .map(|r| DatabaseInfo {
                name: req(r, "name"),
                charset: s(r, "charset"),
                collation: s(r, "collation"),
            })
            .collect())
    }

    async fn list_tables(&self, database: &str) -> Result<Vec<TableInfo>> {
        let rows = self
            .rows_one(
                "SELECT TABLE_NAME AS name, ENGINE AS engine, \
                 CAST(TABLE_ROWS AS SIGNED) AS row_count, \
                 TABLE_COMMENT AS comment, \
                 CAST(DATA_LENGTH AS SIGNED) AS data_length \
                 FROM information_schema.TABLES \
                 WHERE TABLE_SCHEMA = ? AND TABLE_TYPE = 'BASE TABLE' ORDER BY TABLE_NAME",
                database,
            )
            .await?;
        Ok(rows
            .iter()
            .map(|r| TableInfo {
                name: req(r, "name"),
                engine: s(r, "engine"),
                row_count: r.try_get::<i64, _>("row_count").ok(),
                comment: s(r, "comment"),
                data_length: r.try_get::<i64, _>("data_length").ok(),
            })
            .collect())
    }

    async fn table_columns(&self, database: &str, table: &str) -> Result<Vec<ColumnInfo>> {
        let rows = self
            .rows_two(
                "SELECT COLUMN_NAME AS name, DATA_TYPE AS data_type, IS_NULLABLE AS is_nullable, \
                 COLUMN_KEY AS column_key, COLUMN_DEFAULT AS default_value, COLUMN_COMMENT AS comment, \
                 ORDINAL_POSITION AS ord, EXTRA AS extra, \
                 NUMERIC_PRECISION AS num_prec, NUMERIC_SCALE AS num_scale, \
                 CHARACTER_MAXIMUM_LENGTH AS char_len \
                 FROM information_schema.COLUMNS \
                 WHERE TABLE_SCHEMA = ? AND TABLE_NAME = ? ORDER BY ORDINAL_POSITION",
                database,
                table,
            )
            .await?;
        Ok(rows
            .iter()
            .map(|r| {
                let key = s(r, "column_key");
                let nullable = s(r, "is_nullable");
                // MySQL 的 EXTRA 同时承载自增与生成列两类标记（`auto_increment` /
                // `VIRTUAL GENERATED` / `STORED GENERATED`），读它比再查 statistics 便宜
                let extra = req(r, "extra").to_lowercase();
                ColumnInfo {
                    name: req(r, "name"),
                    data_type: req(r, "data_type"),
                    is_nullable: nullable.map(|v| v == "YES").unwrap_or(true),
                    is_primary_key: key.as_deref() == Some("PRI"),
                    default_value: s(r, "default_value"),
                    comment: s(r, "comment"),
                    ordinal_position: r.try_get::<i64, _>("ord").unwrap_or(0) as i32,
                    is_auto_increment: extra.contains("auto_increment"),
                    is_generated: extra.contains("generated"),
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
                "SELECT INDEX_NAME AS name, COLUMN_NAME AS column_name, NON_UNIQUE AS non_unique, \
                 INDEX_TYPE AS index_type \
                 FROM information_schema.STATISTICS \
                 WHERE TABLE_SCHEMA = ? AND TABLE_NAME = ? ORDER BY INDEX_NAME, SEQ_IN_INDEX",
                database,
                table,
            )
            .await?;
        Ok(rows
            .iter()
            .map(|r| {
                let non_unique: i32 = r.try_get("non_unique").unwrap_or(1);
                let name = req(r, "name");
                IndexInfo {
                    is_unique: non_unique == 0,
                    is_primary: name == "PRIMARY",
                    name,
                    column_name: req(r, "column_name"),
                    index_type: s(r, "index_type"),
                }
            })
            .collect())
    }

    async fn table_schema(&self, database: &str, table: &str) -> Result<String> {
        let sql = format!(
            "SHOW CREATE TABLE {}.{}",
            self.quote_ident(database),
            self.quote_ident(table)
        );
        let row = sqlx::query(&sql).fetch_one(&self.pool).await?;
        // 第 2 列是 `Create Table` 的 DDL（第 1 列是 Table 名）
        Ok(row.try_get::<String, _>(1).unwrap_or_default())
    }

    // ===== 库级对象 =====

    async fn list_views(&self, database: &str) -> Result<Vec<DatabaseObjectInfo>> {
        let rows = self
            .rows_one(
                "SELECT TABLE_NAME AS name, 'VIEW' AS object_type, TABLE_SCHEMA AS database_name, \
                 IFNULL(DEFINER,'') AS definer, '' AS created, '' AS modified, \
                 '' AS comment_txt, '' AS status_txt, '' AS event_txt, '' AS table_name \
                 FROM information_schema.VIEWS WHERE TABLE_SCHEMA = ? ORDER BY TABLE_NAME",
                database,
            )
            .await?;
        Ok(rows.iter().map(|r| object_from_row(r)).collect())
    }

    async fn list_procedures(&self, database: &str) -> Result<Vec<DatabaseObjectInfo>> {
        let rows = self
            .rows_one(
                "SELECT ROUTINE_NAME AS name, ROUTINE_TYPE AS object_type, ROUTINE_SCHEMA AS database_name, \
                 IFNULL(DEFINER,'') AS definer, IFNULL(CREATED,'') AS created, IFNULL(LAST_ALTERED,'') AS modified, \
                 IFNULL(ROUTINE_COMMENT,'') AS comment_txt, '' AS status_txt, '' AS event_txt, '' AS table_name \
                 FROM information_schema.ROUTINES \
                 WHERE ROUTINE_SCHEMA = ? AND ROUTINE_TYPE = 'PROCEDURE' ORDER BY ROUTINE_NAME",
                database,
            )
            .await?;
        Ok(rows.iter().map(|r| object_from_row(r)).collect())
    }

    async fn list_functions(&self, database: &str) -> Result<Vec<DatabaseObjectInfo>> {
        let rows = self
            .rows_one(
                "SELECT ROUTINE_NAME AS name, ROUTINE_TYPE AS object_type, ROUTINE_SCHEMA AS database_name, \
                 IFNULL(DEFINER,'') AS definer, IFNULL(CREATED,'') AS created, IFNULL(LAST_ALTERED,'') AS modified, \
                 IFNULL(ROUTINE_COMMENT,'') AS comment_txt, '' AS status_txt, '' AS event_txt, '' AS table_name \
                 FROM information_schema.ROUTINES \
                 WHERE ROUTINE_SCHEMA = ? AND ROUTINE_TYPE = 'FUNCTION' ORDER BY ROUTINE_NAME",
                database,
            )
            .await?;
        Ok(rows.iter().map(|r| object_from_row(r)).collect())
    }

    /// 取对象的定义源码。
    ///
    /// **列位置按语句各不相同**，集中在这里核对一次，别照抄 `table_schema` 的下标：
    /// - `SHOW CREATE VIEW`      → View      | Create View  | character_set_client | …
    /// - `SHOW CREATE PROCEDURE` → Procedure | **sql_mode** | Create Procedure     | …
    /// - `SHOW CREATE FUNCTION`  → Function  | **sql_mode** | Create Function      | …
    ///
    /// 例程那条比 view 多夹了一个 sql_mode，源码在**第 3 列（下标 2）**；
    /// 用下标 1 会把 sql_mode 当成源码显示给用户。
    async fn object_source(
        &self,
        kind: ObjectSourceKind,
        database: &str,
        name: &str,
    ) -> Result<String> {
        let full = format!("{}.{}", self.quote_ident(database), self.quote_ident(name));
        let (sql, idx): (String, usize) = match kind {
            ObjectSourceKind::View => (format!("SHOW CREATE VIEW {}", full), 1),
            ObjectSourceKind::Procedure => (format!("SHOW CREATE PROCEDURE {}", full), 2),
            ObjectSourceKind::Function => (format!("SHOW CREATE FUNCTION {}", full), 2),
            ObjectSourceKind::Trigger => return self.trigger_source(database, name).await,
        };
        let row = sqlx::query(&sql).fetch_one(&self.pool).await?;
        let src = row.try_get::<String, _>(idx).unwrap_or_default();
        if src.trim().is_empty() {
            return Err(anyhow::anyhow!("未取到 {}（对象不存在或没有权限）", kind.label()));
        }
        Ok(src)
    }

    async fn list_triggers(&self, database: &str) -> Result<Vec<DatabaseObjectInfo>> {
        let rows = self
            .rows_one(
                "SELECT TRIGGER_NAME AS name, 'TRIGGER' AS object_type, TRIGGER_SCHEMA AS database_name, \
                 IFNULL(DEFINER,'') AS definer, IFNULL(CREATED,'') AS created, '' AS modified, \
                 '' AS comment_txt, IFNULL(ACTION_STATUS,'') AS status_txt, \
                 IFNULL(EVENT_MANIPULATION,'') AS event_txt, IFNULL(EVENT_OBJECT_TABLE,'') AS table_name \
                 FROM information_schema.TRIGGERS WHERE TRIGGER_SCHEMA = ? ORDER BY TRIGGER_NAME",
                database,
            )
            .await?;
        Ok(rows.iter().map(|r| object_from_row(r)).collect())
    }

    async fn list_events(&self, database: &str) -> Result<Vec<DatabaseObjectInfo>> {
        let rows = self
            .rows_one(
                "SELECT EVENT_NAME AS name, 'EVENT' AS object_type, EVENT_SCHEMA AS database_name, \
                 IFNULL(DEFINER,'') AS definer, IFNULL(CREATED,'') AS created, \
                 IFNULL(LAST_ALTERED,'') AS modified, IFNULL(EVENT_COMMENT,'') AS comment_txt, \
                 IFNULL(STATUS,'') AS status_txt, '' AS event_txt, '' AS table_name \
                 FROM information_schema.EVENTS WHERE EVENT_SCHEMA = ? ORDER BY EVENT_NAME",
                database,
            )
            .await?;
        Ok(rows.iter().map(|r| object_from_row(r)).collect())
    }

    async fn list_database_indexes(&self, database: &str) -> Result<Vec<DatabaseIndexInfo>> {
        let rows = self
            .rows_one(
                "SELECT TABLE_NAME AS table_name, INDEX_NAME AS name, COLUMN_NAME AS column_name, \
                 NON_UNIQUE AS non_unique, IFNULL(INDEX_TYPE,'') AS index_type \
                 FROM information_schema.STATISTICS \
                 WHERE TABLE_SCHEMA = ? ORDER BY TABLE_NAME, INDEX_NAME, SEQ_IN_INDEX",
                database,
            )
            .await?;
        Ok(rows
            .iter()
            .map(|r| {
                let non_unique: i32 = r.try_get("non_unique").unwrap_or(1);
                let name = req(r, "name");
                DatabaseIndexInfo {
                    is_unique: non_unique == 0,
                    is_primary: name == "PRIMARY",
                    name,
                    table_name: req(r, "table_name"),
                    column_name: req(r, "column_name"),
                    index_type: s(r, "index_type"),
                }
            })
            .collect())
    }

    // ===== 实例级 =====

    async fn list_users(&self) -> Result<Vec<DbUserInfo>> {
        // 只查 5.7 / 8.0 都存在的稳定字段，避免版本差异导致整条查询失败
        let rows = self
            .rows(
                "SELECT IFNULL(user,'') AS user, IFNULL(host,'') AS host, IFNULL(plugin,'') AS plugin \
                 FROM mysql.user ORDER BY user, host",
            )
            .await?;
        Ok(rows
            .iter()
            .map(|r| DbUserInfo {
                user: req(r, "user"),
                host: req(r, "host"),
                plugin: s(r, "plugin"),
                account_locked: None,
                password_expired: None,
                privileges: None,
            })
            .collect())
    }

    /// 管理员判定（实用档）：能杀连接/改全局参数（SUPER）、能给别人授权（GRANT OPTION）、
    /// 或能建账号（CREATE USER）的账号都算管理员。
    ///
    /// 权限位一并返回，用户可自行核对判定是否准确。若 `mysql.user` 无权限列（权限不足或版本差异），
    /// 回退到「按用户名兜底」，保证节点不至于整块打不开。
    async fn list_admins(&self) -> Result<Vec<DbAdminInfo>> {
        let full = sqlx::query(
            "SELECT IFNULL(user,'') AS user, IFNULL(host,'') AS host, IFNULL(plugin,'') AS plugin, \
             IFNULL(Super_priv,'N') AS super_priv, IFNULL(Grant_priv,'N') AS grant_priv, \
             IFNULL(Create_user_priv,'N') AS create_user_priv \
             FROM mysql.user \
             WHERE Super_priv = 'Y' OR Grant_priv = 'Y' OR Create_user_priv = 'Y' \
             ORDER BY user, host",
        )
        .fetch_all(&self.pool)
        .await;

        match full {
            Ok(rows) => Ok(rows
                .iter()
                .map(|r| {
                    let su = yn(s(r, "super_priv")).unwrap_or(false);
                    let gr = yn(s(r, "grant_priv")).unwrap_or(false);
                    let cu = yn(s(r, "create_user_priv")).unwrap_or(false);
                    let mut reasons = Vec::new();
                    if su {
                        reasons.push("SUPER");
                    }
                    if gr {
                        reasons.push("GRANT");
                    }
                    if cu {
                        reasons.push("CREATE USER");
                    }
                    let mut privs = Vec::new();
                    if su {
                        privs.push("Super_priv=Y");
                    }
                    if gr {
                        privs.push("Grant_priv=Y");
                    }
                    if cu {
                        privs.push("Create_user_priv=Y");
                    }
                    DbAdminInfo {
                        user: req(r, "user"),
                        host: req(r, "host"),
                        plugin: s(r, "plugin"),
                        reason: reasons.join(" / "),
                        privileges: if privs.is_empty() {
                            None
                        } else {
                            Some(privs.join(", "))
                        },
                    }
                })
                .collect()),
            // 权限列不可用（权限不足或版本差异）→ 退化为「只列 root 等常见管理员账号」
            Err(_) => {
                let rows = self
                    .rows(
                        "SELECT IFNULL(user,'') AS user, IFNULL(host,'') AS host, IFNULL(plugin,'') AS plugin \
                         FROM mysql.user WHERE user IN ('root', 'admin') ORDER BY user, host",
                    )
                    .await?;
                Ok(rows
                    .iter()
                    .map(|r| DbAdminInfo {
                        user: req(r, "user"),
                        host: req(r, "host"),
                        plugin: s(r, "plugin"),
                        reason: "按账号名兜底（无法读取权限列）".to_string(),
                        privileges: None,
                    })
                    .collect())
            }
        }
    }

    async fn show_status(&self, scope: Scope) -> Result<Vec<KeyValueRow>> {
        let sql = match scope {
            Scope::Session => "SHOW SESSION STATUS",
            Scope::Global => "SHOW GLOBAL STATUS",
        };
        Ok(self.key_values(sql).await?)
    }

    async fn show_variables(&self, scope: Scope) -> Result<Vec<KeyValueRow>> {
        let sql = match scope {
            Scope::Session => "SHOW SESSION VARIABLES",
            Scope::Global => "SHOW GLOBAL VARIABLES",
        };
        Ok(self.key_values(sql).await?)
    }
}

impl MySqlCatalog {
    /// `SHOW STATUS / VARIABLES` 统一取「名-值」两列（用下标取，避免列名大小写差异）
    async fn key_values(&self, sql: &str) -> Result<Vec<KeyValueRow>> {
        let rows = sqlx::query(sql).fetch_all(&self.pool).await?;
        Ok(rows
            .iter()
            .map(|r| KeyValueRow {
                name: r.try_get::<String, _>(0).unwrap_or_default(),
                value: r.try_get::<String, _>(1).unwrap_or_default(),
            })
            .collect())
    }
}

/// 由统一的列别名构造 `DatabaseObjectInfo`
fn object_from_row(r: &MySqlRow) -> DatabaseObjectInfo {
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
