/*! # SQL 方言抽象层
把各数据库在「标识符引用 / 分页语法 / 切库语句 / 默认值」上的差异收敛到一处，
避免业务模块里散落 `if db_type == ...` 与硬编码的反引号。

支持：MySQL、PostgreSQL、SQL Server（TDS）、Oracle（ODBC）。

注：部分方法（from_db_type / default_port / default_database / qualified_table /
bool_literal / is_select_query）当前由上层以各自方式实现，属于方言库预留的公共 API，
后续 connection 层与更多方言接入时复用，故允许 dead_code。
*/
#![allow(dead_code)]
use anyhow::{anyhow, Result};
use super::pool::DbPool;

/// 数据库方言
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Dialect {
    MySQL,
    PostgreSQL,
    SqlServer,
    Oracle,
}

/// 由连接池反推方言（CRUD / 编辑写回都需要按方言拼标识符与字面量）。
/// 放在这里而不是各处 `match pool`，是为了让「这一句该怎么写」只有一个答案。
pub fn dialect_of(pool: &DbPool) -> Dialect {
    match pool {
        DbPool::MySQL(_) => Dialect::MySQL,
        DbPool::PostgreSQL(_) => Dialect::PostgreSQL,
        DbPool::SqlServer(_) => Dialect::SqlServer,
        DbPool::Oracle(_) => Dialect::Oracle,
    }
}

impl Dialect {
    /// 由连接配置里的 db_type 字符串解析方言（大小写/常见别名均兼容）
    pub fn from_db_type(db_type: &str) -> Result<Dialect> {
        match db_type.trim().to_lowercase().as_str() {
            "mysql" | "mariadb" => Ok(Dialect::MySQL),
            "postgresql" | "postgres" | "pg" => Ok(Dialect::PostgreSQL),
            "sqlserver" | "sql server" | "mssql" => Ok(Dialect::SqlServer),
            "oracle" => Ok(Dialect::Oracle),
            other => Err(anyhow!("不支持的数据库类型: {}", other)),
        }
    }

    /// 默认端口
    pub fn default_port(&self) -> i32 {
        match self {
            Dialect::MySQL => 3306,
            Dialect::PostgreSQL => 5432,
            Dialect::SqlServer => 1433,
            Dialect::Oracle => 1521,
        }
    }

    /// 未指定库名时连接串里使用的默认库（空串表示不指定）
    pub fn default_database(&self) -> &'static str {
        match self {
            Dialect::MySQL => "",
            Dialect::PostgreSQL => "postgres",
            Dialect::SqlServer => "master",
            Dialect::Oracle => "",
        }
    }

    /// 标识符引用（同时做转义，避免标识符本身含引号/方括号导致注入或语法错误）
    /// - MySQL：反引号
    /// - PostgreSQL / Oracle：双引号（Oracle 未加引号的标识符存为大写，故统一转大写）
    /// - SQL Server：方括号
    pub fn quote_ident(&self, ident: &str) -> String {
        match self {
            Dialect::MySQL => format!("`{}`", ident.replace('`', "``")),
            Dialect::PostgreSQL => format!("\"{}\"", ident.replace('"', "\"\"")),
            Dialect::SqlServer => format!("[{}]", ident.replace(']', "]]")),
            Dialect::Oracle => format!("\"{}\"", ident.to_uppercase().replace('"', "\"\"")),
        }
    }

    /// 库名 + 表名限定；库名为空时只返回表名
    pub fn qualified_table(&self, database: &str, table: &str) -> String {
        if database.is_empty() {
            return self.quote_ident(table);
        }
        format!("{}.{}", self.quote_ident(database), self.quote_ident(table))
    }

    /// 切换当前库 / schema 的语句（在同一连接上先执行，再跑业务 SQL）
    pub fn use_database_sql(&self, database: &str) -> String {
        match self {
            Dialect::MySQL | Dialect::SqlServer => format!("USE {}", self.quote_ident(database)),
            Dialect::PostgreSQL => format!("SET search_path TO {}", self.quote_ident(database)),
            Dialect::Oracle => format!(
                "ALTER SESSION SET CURRENT_SCHEMA = {}",
                self.quote_ident(database)
            ),
        }
    }

    /// 把查询体包装成分页查询（body 已去掉尾部分号）。
    /// 各库语法差异：
    /// - MySQL / PostgreSQL：`LIMIT n OFFSET m`
    /// - SQL Server：`OFFSET m ROWS FETCH NEXT n ROWS ONLY`，且必须带 ORDER BY（用常量排序占位）
    /// - Oracle：同上，但派生表别名不能写 AS
    pub fn wrap_paged_sql(&self, body: &str, limit: usize, offset: u64) -> String {
        match self {
            Dialect::MySQL | Dialect::PostgreSQL => format!(
                "SELECT * FROM ( {} ) AS __bdt_page LIMIT {} OFFSET {}",
                body, limit, offset
            ),
            Dialect::SqlServer => format!(
                "SELECT * FROM ( {} ) AS __bdt_page ORDER BY (SELECT NULL) OFFSET {} ROWS FETCH NEXT {} ROWS ONLY",
                body, offset, limit
            ),
            Dialect::Oracle => format!(
                "SELECT * FROM ( {} ) __bdt_page ORDER BY NULL OFFSET {} ROWS FETCH NEXT {} ROWS ONLY",
                body, offset, limit
            ),
        }
    }

    /// 包装成总数统计查询
    pub fn wrap_count_sql(&self, body: &str) -> String {
        match self {
            Dialect::MySQL | Dialect::PostgreSQL | Dialect::SqlServer => format!(
                "SELECT COUNT(*) AS __bdt_cnt FROM ( {} ) AS __bdt_cnt_sub",
                body
            ),
            Dialect::Oracle => format!(
                "SELECT COUNT(*) AS __bdt_cnt FROM ( {} ) __bdt_cnt_sub",
                body
            ),
        }
    }

    /// 布尔字面量（Oracle 的 SQL 层没有 BOOLEAN 类型，沿用 1/0）
    pub fn bool_literal(&self, value: bool) -> &'static str {
        match self {
            Dialect::PostgreSQL => {
                if value {
                    "TRUE"
                } else {
                    "FALSE"
                }
            }
            _ => {
                if value {
                    "1"
                } else {
                    "0"
                }
            }
        }
    }

    /// 字符串字面量里的转义（不含外层引号）。
    /// MySQL 默认 `\` 是转义符，只把 `'` 翻倍的话，值里的 `\` 会把后面的引号吃掉；
    /// 其余三种在字符串字面量里反斜杠就是普通字符（PG 的 standard_conforming_strings
    /// 默认开启），翻倍反而会写错数据 —— 所以这里按方言区分，不是偷懒。
    pub fn escape_string(&self, s: &str) -> String {
        match self {
            Dialect::MySQL => s.replace('\\', "\\\\").replace('\'', "''"),
            _ => s.replace('\'', "''"),
        }
    }

    /// 字符串 → 带引号的字面量
    pub fn string_literal(&self, s: &str) -> String {
        format!("'{}'", self.escape_string(s))
    }

    /// JSON 值 → SQL 字面量（编辑写回的唯一出口）。
    /// - `null` → `NULL`
    /// - bool → 方言布尔字面量
    /// - number → 原样
    /// - 其余（含嵌套对象）→ JSON 文本。
    ///   注意：网格里的「标量契约」默认把 DECIMAL / 时间 / JSON 都序列化成了字符串，
    ///   这里只要原样包引号即可，**不要**为了「看起来像数字」去尝试 parse —— 那会
    ///   丢掉 `12.30` 的标度，也会把 `007` 变成 `7`。
    pub fn literal(&self, v: &serde_json::Value) -> String {
        match v {
            serde_json::Value::Null => "NULL".to_string(),
            serde_json::Value::Bool(b) => self.bool_literal(*b).to_string(),
            serde_json::Value::Number(n) => n.to_string(),
            serde_json::Value::String(s) => self.string_literal(s),
            other => self.string_literal(&other.to_string()),
        }
    }
}

/// 去掉 SQL 前导注释（支持 -- 单行注释和 /* */ 块注释）
pub fn strip_leading_comments(sql: &str) -> &str {
    let mut s = sql.trim_start();
    loop {
        if s.starts_with("--") {
            if let Some(pos) = s.find('\n') {
                s = s[pos + 1..].trim_start();
            } else {
                return "";
            }
        } else if s.starts_with("/*") {
            if let Some(pos) = s.find("*/") {
                s = s[pos + 2..].trim_start();
            } else {
                return s;
            }
        } else {
            break;
        }
    }
    s
}

/// 是否为返回结果集的语句（跳过前导注释后判断）
pub fn is_select_query(sql: &str) -> bool {
    let stripped = strip_leading_comments(sql).trim().to_uppercase();
    stripped.starts_with("SELECT")
        || stripped.starts_with("SHOW")
        || stripped.starts_with("DESCRIBE")
        || stripped.starts_with("EXPLAIN")
}

/// 是否为可安全包装成子查询的纯 SELECT。
/// SHOW / DESCRIBE / EXPLAIN 以及以 WITH 开头的 CTE 无法安全地套一层
/// `SELECT * FROM (...) `，只能原样执行并限制读取行数（因此不支持翻页）。
pub fn is_wrappable_select(sql: &str) -> bool {
    strip_leading_comments(sql).to_uppercase().starts_with("SELECT")
}
