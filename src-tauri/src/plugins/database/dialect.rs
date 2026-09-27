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

/// 数据库方言
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Dialect {
    MySQL,
    PostgreSQL,
    SqlServer,
    Oracle,
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
