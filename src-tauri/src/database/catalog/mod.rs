/*! # 数据库目录（元数据）抽象层

把「每种数据库各写一套 SQL」从 `metadata.rs` 里剥离出来，收敛为一个 trait：

```text
命令层 (commands.rs) → metadata.rs 薄封装 → catalog_for(pool) → CatalogProvider 实现
```

新增一种数据库的成本 = 加一个 `catalog/xxx.rs` + 工厂里加一行分支，**命令层与业务层零改动**。

## 设计要点

- **能力声明优先**：`capabilities()` 声明本库支持哪些能力，前端据此决定渲染哪些树节点。
  不支持的节点是「根本不出现」，而不是「出现了点开报错」。
- **可选能力默认不支持**：trait 里给可选能力写默认实现返回 `Err`，各库只 override 自己支持的，
  新增数据库时只需写它真正有的能力，其余自动降级。
- **标识符一律走 `Dialect::quote_ident`**：`SHOW ... IN db` / `OBJECT_ID('...')` 之类无法参数化，
  必须转义后再拼进 SQL。
*/
use anyhow::{anyhow, Result};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use super::dialect::Dialect;
use super::operations::ColumnInfo;
use super::pool::DbPool;

pub mod mssql;
pub mod mysql;
pub mod oracle;
pub mod postgres;

pub use mssql::SqlServerCatalog;
pub use mysql::MySqlCatalog;
pub use oracle::OracleCatalog;
pub use postgres::PostgresCatalog;

// ==================== 通用数据结构 ====================

/// 数据库信息
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct DatabaseInfo {
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub charset: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub collation: Option<String>,
}

/// 表信息
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct TableInfo {
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub engine: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub row_count: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub comment: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data_length: Option<i64>,
}

/// 索引信息
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct IndexInfo {
    pub name: String,
    pub column_name: String,
    pub is_unique: bool,
    pub is_primary: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub index_type: Option<String>,
}

/// 库级索引聚合（比表级索引多一个所属表）
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct DatabaseIndexInfo {
    pub name: String,
    pub table_name: String,
    pub column_name: String,
    pub is_unique: bool,
    pub is_primary: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub index_type: Option<String>,
}

/// 键值行：状态 / 变量等「名-值」结果统一用这个结构，前端直接当两列表格渲染
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct KeyValueRow {
    pub name: String,
    pub value: String,
}

/// 用户信息
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct DbUserInfo {
    pub user: String,
    /// MySQL 是 host；PG / Oracle 等无主机概念的库填空串
    pub host: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub plugin: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub account_locked: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub password_expired: Option<bool>,
    /// 原始权限标记（各库语义不同，保留供人工核对判定是否准确）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub privileges: Option<String>,
}

/// 管理员信息
///
/// 判定用各库自己的**原生语义**（MySQL 看权限位、PG 看 rolsuper、SQL Server 看 sysadmin 角色、
/// Oracle 看 SYSDBA/DBA 角色），不做跨库硬编码布尔。`reason` 记录命中了哪条依据，
/// 便于用户核对「为什么这个账号算管理员」。
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct DbAdminInfo {
    pub user: String,
    pub host: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub plugin: Option<String>,
    /// 判定依据，如 `SUPER / GRANT / CREATE USER`、`SUPERUSER`、`sysadmin`、`SYSDBA`
    pub reason: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub privileges: Option<String>,
}

/// 数据库对象（视图 / 存储过程 / 触发器 / 事件）的共有信息
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct DatabaseObjectInfo {
    pub name: String,
    /// 对象类型：VIEW / PROCEDURE / FUNCTION / TRIGGER / EVENT …
    #[serde(skip_serializing_if = "Option::is_none")]
    pub object_type: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub database: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub definer: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub created: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub modified: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub comment: Option<String>,
    /// 启用状态：ENABLED / DISABLED
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status: Option<String>,
    /// 触发器专用：触发事件（INSERT / UPDATE / DELETE）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub event: Option<String>,
    /// 触发器 / 索引专用：所属表
    #[serde(skip_serializing_if = "Option::is_none")]
    pub table_name: Option<String>,
}

/// 作用域：会话级 / 全局级（状态与变量都分这两种）
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Scope {
    Session,
    Global,
}

impl Scope {
    /// 由命令层传来的字符串解析；无法识别时按会话级处理
    pub fn parse(raw: &str) -> Scope {
        if raw.eq_ignore_ascii_case("global") {
            Scope::Global
        } else {
            Scope::Session
        }
    }
}

/// 目录能力声明：前端拉一次，据此决定渲染哪些树节点
///
/// 为 `false` 的能力，对应节点根本不会出现在树里 —— 比「显示出来但点开报错」体验好。
#[derive(Serialize, Deserialize, Debug, Clone, Copy)]
pub struct CatalogCapabilities {
    pub views: bool,
    pub procedures: bool,
    pub triggers: bool,
    pub events: bool,
    pub database_indexes: bool,
    pub users: bool,
    pub admins: bool,
    pub session_status: bool,
    pub global_status: bool,
    pub session_variables: bool,
    pub global_variables: bool,
}

impl CatalogCapabilities {
    /// 全不支持（兜底：未实现任何可选能力的数据库）
    pub fn none() -> Self {
        CatalogCapabilities {
            views: false,
            procedures: false,
            triggers: false,
            events: false,
            database_indexes: false,
            users: false,
            admins: false,
            session_status: false,
            global_status: false,
            session_variables: false,
            global_variables: false,
        }
    }
}

// ==================== 抽象接口 ====================

/// 「本库不支持某能力」的统一错误文案
fn unsupported(db_type: &str, what: &str) -> anyhow::Error {
    anyhow!("{} 暂不支持{}", db_type, what)
}

/// 目录（元数据）访问抽象
///
/// 每种数据库实现一次；可选能力有默认「不支持」实现，实现方只写自己真正支持的。
#[async_trait]
pub trait CatalogProvider: Send + Sync {
    /// 数据库类型标识（与连接配置里的 db_type 保持一致的小写名）
    fn db_type(&self) -> &'static str;

    /// SQL 方言（标识符引用 / 限定名等差异都走这里）
    fn dialect(&self) -> Dialect;

    /// 本库支持哪些能力
    fn capabilities(&self) -> CatalogCapabilities;

    // ===== 基础能力（每种库都必须实现）=====

    async fn list_databases(&self) -> Result<Vec<DatabaseInfo>>;
    async fn list_tables(&self, database: &str) -> Result<Vec<TableInfo>>;
    async fn table_columns(&self, database: &str, table: &str) -> Result<Vec<ColumnInfo>>;
    async fn table_indexes(&self, database: &str, table: &str) -> Result<Vec<IndexInfo>>;
    async fn table_schema(&self, database: &str, table: &str) -> Result<String>;

    // ===== 可选能力（默认不支持，按需 override）=====

    async fn list_views(&self, _database: &str) -> Result<Vec<DatabaseObjectInfo>> {
        Err(unsupported(self.db_type(), "视图"))
    }

    async fn list_procedures(&self, _database: &str) -> Result<Vec<DatabaseObjectInfo>> {
        Err(unsupported(self.db_type(), "存储过程"))
    }

    async fn list_triggers(&self, _database: &str) -> Result<Vec<DatabaseObjectInfo>> {
        Err(unsupported(self.db_type(), "触发器"))
    }

    async fn list_events(&self, _database: &str) -> Result<Vec<DatabaseObjectInfo>> {
        Err(unsupported(self.db_type(), "事件"))
    }

    async fn list_database_indexes(&self, _database: &str) -> Result<Vec<DatabaseIndexInfo>> {
        Err(unsupported(self.db_type(), "库级索引"))
    }

    async fn list_users(&self) -> Result<Vec<DbUserInfo>> {
        Err(unsupported(self.db_type(), "用户列表"))
    }

    /// 管理员：各库用自己的原生语义判定，不写跨库硬编码
    async fn list_admins(&self) -> Result<Vec<DbAdminInfo>> {
        Err(unsupported(self.db_type(), "管理员列表"))
    }

    async fn show_status(&self, _scope: Scope) -> Result<Vec<KeyValueRow>> {
        Err(unsupported(self.db_type(), "运行状态"))
    }

    async fn show_variables(&self, _scope: Scope) -> Result<Vec<KeyValueRow>> {
        Err(unsupported(self.db_type(), "系统变量"))
    }

    // ===== 供实现方复用的小工具 =====

    /// 引用标识符（走 Dialect，各库引号规则不同）
    fn quote_ident(&self, ident: &str) -> String {
        self.dialect().quote_ident(ident)
    }

    /// 字符串字面量转义（SQL Server / Oracle 的拼接式 SQL 用得上）
    fn escape_literal(&self, s: &str) -> String {
        s.replace('\'', "''")
    }
}

/// 工厂：由连接池产出对应的目录实现
pub fn catalog_for(pool: &DbPool) -> Box<dyn CatalogProvider> {
    match pool {
        DbPool::MySQL(p) => Box::new(MySqlCatalog::new(p.clone())),
        DbPool::PostgreSQL(p) => Box::new(PostgresCatalog::new(p.clone())),
        DbPool::SqlServer(p) => Box::new(SqlServerCatalog::new(p.clone())),
        DbPool::Oracle(p) => Box::new(OracleCatalog::new(p.clone())),
    }
}

// ==================== 行值转换辅助（SQL Server / Oracle 走 JSON 行列）====================

/// 取字符串
pub fn str_of(v: &Value) -> Option<String> {
    match v {
        Value::String(s) => Some(s.clone()),
        Value::Number(n) => Some(n.to_string()),
        Value::Bool(b) => Some(b.to_string()),
        _ => None,
    }
}

/// 取布尔（兼容布尔 / 数字 / "0"|"1" / "Y"|"N"）
pub fn bool_of(v: &Value) -> Option<bool> {
    match v {
        Value::Bool(b) => Some(*b),
        Value::Number(n) => n.as_i64().map(|i| i != 0),
        Value::String(s) => match s.trim().to_lowercase().as_str() {
            "y" | "yes" | "true" | "t" => Some(true),
            "n" | "no" | "false" | "f" => Some(false),
            other => other.parse::<i64>().ok().map(|i| i != 0),
        },
        _ => None,
    }
}

/// 取整数
pub fn num_of(v: &Value) -> Option<i64> {
    match v {
        Value::Number(n) => n.as_i64(),
        Value::String(s) => s.trim().parse::<i64>().ok(),
        _ => None,
    }
}
