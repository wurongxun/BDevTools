/*! # 元数据查询模块
获取数据库、表、字段、索引等元数据信息。
*/
use crate::response::response::Response;
use anyhow::Error;
use serde::{Deserialize, Serialize};
use sqlx::Row;
use super::pool::get_pool;
use super::operations::ColumnInfo;
use serde_json::Value;

/// 数据库信息
#[derive(Serialize, Deserialize, Debug)]
pub struct DatabaseInfo {
    pub name: String,
    pub charset: Option<String>,
    pub collation: Option<String>,
}

/// 表信息
#[derive(Serialize, Deserialize, Debug)]
pub struct TableInfo {
    pub name: String,
    pub engine: Option<String>,
    pub row_count: Option<i64>,
    pub comment: Option<String>,
    pub data_length: Option<i64>,
}

/// 索引信息
#[derive(Serialize, Deserialize, Debug)]
pub struct IndexInfo {
    pub name: String,
    pub column_name: String,
    pub is_unique: bool,
    pub is_primary: bool,
    pub index_type: Option<String>,
}

/// 获取所有数据库列表
pub async fn db_list_databases(config_id: String) -> Response<Vec<DatabaseInfo>> {
    let pool = match get_pool(&config_id) {
        Some(p) => p,
        None => return Response::new(300, vec![], "未连接到数据库"),
    };

    let result: Result<Vec<DatabaseInfo>, Error> = match &pool {
        super::pool::DbPool::MySQL(p) => {
            sqlx::query("SELECT SCHEMA_NAME AS name, DEFAULT_CHARACTER_SET_NAME AS charset, DEFAULT_COLLATION_NAME AS collation FROM information_schema.SCHEMATA ORDER BY SCHEMA_NAME")
                .fetch_all(p)
                .await
                .map_err(Error::from)
                .map(|rows| {
                    rows.iter().map(|row| DatabaseInfo {
                        name: row.get("name"),
                        charset: row.get("charset"),
                        collation: row.get("collation"),
                    }).collect()
                })
        }
        super::pool::DbPool::PostgreSQL(p) => {
            sqlx::query("SELECT datname AS name FROM pg_database WHERE datistemplate = false ORDER BY datname")
                .fetch_all(p)
                .await
                .map_err(Error::from)
                .map(|rows| {
                    rows.iter().map(|row| DatabaseInfo {
                        name: row.get("name"),
                        charset: None,
                        collation: None,
                    }).collect()
                })
        }
        super::pool::DbPool::SqlServer(p) => {
            p.query_rows(
                "SELECT name FROM sys.databases WHERE database_id > 4 ORDER BY name",
                10000,
            )
            .await
            .map(|(_, rows)| {
                rows.into_iter()
                    .filter_map(|r| r.into_iter().next())
                    .filter_map(|v| match v {
                        Value::String(s) => Some(s),
                        _ => None,
                    })
                    .map(|name| DatabaseInfo { name, charset: None, collation: None })
                    .collect()
            })
        }
        super::pool::DbPool::Oracle(p) => {
            p.query_rows(
                "",
                "SELECT username AS name FROM all_users ORDER BY username",
                10000,
            )
            .await
            .map(|(_, rows)| {
                rows.into_iter()
                    .filter_map(|r| r.into_iter().next())
                    .filter_map(|v| match v {
                        Value::String(s) => Some(s),
                        _ => None,
                    })
                    .map(|name| DatabaseInfo { name, charset: None, collation: None })
                    .collect()
            })
        }
    };

    match result {
        Ok(list) => Response::new(200, list, "查询成功"),
        Err(e) => Response::new(300, vec![], &format!("查询失败: {}", e)),
    }
}

/// 获取指定数据库的所有表
pub async fn db_list_tables(config_id: String, database: String) -> Response<Vec<TableInfo>> {
    let pool = match get_pool(&config_id) {
        Some(p) => p,
        None => return Response::new(300, vec![], "未连接到数据库"),
    };

    let result: Result<Vec<TableInfo>, Error> = match &pool {
        super::pool::DbPool::MySQL(p) => {
            let sql = format!(
                "SELECT TABLE_NAME AS name, ENGINE AS engine, \
                 CAST(TABLE_ROWS AS SIGNED) AS row_count, \
                 TABLE_COMMENT AS comment, \
                 CAST(DATA_LENGTH AS SIGNED) AS data_length \
                 FROM information_schema.TABLES WHERE TABLE_SCHEMA = '{}' AND TABLE_TYPE = 'BASE TABLE' ORDER BY TABLE_NAME",
                database
            );
            sqlx::query(&sql)
                .fetch_all(p)
                .await
                .map_err(Error::from)
                .map(|rows| {
                    rows.iter().map(|row| TableInfo {
                        name: row.get("name"),
                        engine: row.get("engine"),
                        row_count: row.get("row_count"),
                        comment: row.get("comment"),
                        data_length: row.get("data_length"),
                    }).collect()
                })
        }
        super::pool::DbPool::PostgreSQL(p) => {
            let sql = format!(
                "SELECT tablename AS name FROM pg_tables WHERE schemaname = 'public' ORDER BY tablename"
            );
            sqlx::query(&sql)
                .fetch_all(p)
                .await
                .map_err(Error::from)
                .map(|rows| {
                    rows.iter().map(|row| TableInfo {
                        name: row.get("name"),
                        engine: None,
                        row_count: None,
                        comment: None,
                        data_length: None,
                    }).collect()
                })
        }
        super::pool::DbPool::SqlServer(p) => {
            let sql = format!(
                "SELECT t.name AS name, NULL AS engine, p.rows AS row_count, NULL AS comment, NULL AS data_length \
                 FROM sys.tables t LEFT JOIN sys.partitions p ON t.object_id = p.object_id AND p.index_id IN (0, 1) \
                 WHERE SCHEMA_NAME(t.schema_id) = '{}' ORDER BY t.name",
                database.replace('\'', "''")
            );
            p.query_rows(&sql, 10000).await.map(|(_, rows)| {
                rows.into_iter().map(|r| {
                    let name = r.get(0).and_then(str_of).unwrap_or_default();
                    let row_count = r.get(2).and_then(num_of);
                    TableInfo { name, engine: None, row_count, comment: None, data_length: None }
                }).collect()
            })
        }
        super::pool::DbPool::Oracle(p) => {
            let sql = format!(
                "SELECT table_name AS name, NULL AS engine, num_rows AS row_count, NULL AS comment, NULL AS data_length \
                 FROM all_tables WHERE owner = '{}' ORDER BY table_name",
                database.to_uppercase().replace('\'', "''")
            );
            p.query_rows("", &sql, 10000).await.map(|(_, rows)| {
                rows.into_iter().map(|r| {
                    let name = r.get(0).and_then(str_of).unwrap_or_default();
                    let row_count = r.get(2).and_then(num_of);
                    TableInfo { name, engine: None, row_count, comment: None, data_length: None }
                }).collect()
            })
        }
    };

    match result {
        Ok(list) => Response::new(200, list, "查询成功"),
        Err(e) => Response::new(300, vec![], &format!("查询失败: {}", e)),
    }
}

/// 获取表的字段信息
pub async fn db_table_columns(config_id: String, database: String, table: String) -> Response<Vec<ColumnInfo>> {
    let pool = match get_pool(&config_id) {
        Some(p) => p,
        None => return Response::new(300, vec![], "未连接到数据库"),
    };

    let result: Result<Vec<ColumnInfo>, Error> = match &pool {
        super::pool::DbPool::MySQL(p) => {
            let sql = format!(
                "SELECT COLUMN_NAME AS name, DATA_TYPE AS data_type, IS_NULLABLE AS is_nullable, \
                 COLUMN_KEY AS column_key, COLUMN_DEFAULT AS default_value, COLUMN_COMMENT AS comment \
                 FROM information_schema.COLUMNS WHERE TABLE_SCHEMA = '{}' AND TABLE_NAME = '{}' ORDER BY ORDINAL_POSITION",
                database, table
            );
            sqlx::query(&sql)
                .fetch_all(p)
                .await
                .map_err(Error::from)
                .map(|rows| {
                    rows.iter().map(|row| {
                        let key: Option<String> = row.get("column_key");
                        let nullable_str: Option<String> = row.get("is_nullable");
                        ColumnInfo {
                            name: row.get("name"),
                            data_type: row.get("data_type"),
                            is_nullable: nullable_str.map(|v| v == "YES").unwrap_or(true),
                            is_primary_key: key.as_deref() == Some("PRI"),
                            default_value: row.get("default_value"),
                            comment: row.get("comment"),
                        }
                    }).collect()
                })
        }
        super::pool::DbPool::PostgreSQL(p) => {
            let sql = format!(
                "SELECT c.column_name AS name, c.data_type AS data_type, c.is_nullable AS is_nullable, \
                 c.column_default AS default_value, \
                 CASE WHEN pk.column_name IS NOT NULL THEN 'PRI' ELSE '' END AS column_key \
                 FROM information_schema.columns c \
                 LEFT JOIN (SELECT ku.column_name FROM information_schema.table_constraints tc \
                 JOIN information_schema.key_column_usage ku ON tc.constraint_name = ku.constraint_name \
                 WHERE tc.constraint_type = 'PRIMARY KEY' AND tc.table_name = '{}') pk ON c.column_name = pk.column_name \
                 WHERE c.table_name = '{}' AND c.table_schema = 'public' ORDER BY c.ordinal_position",
                table, table
            );
            sqlx::query(&sql)
                .fetch_all(p)
                .await
                .map_err(Error::from)
                .map(|rows| {
                    rows.iter().map(|row| {
                        let key: Option<String> = row.get("column_key");
                        let nullable_str: Option<String> = row.get("is_nullable");
                        ColumnInfo {
                            name: row.get("name"),
                            data_type: row.get("data_type"),
                            is_nullable: nullable_str.map(|v| v == "YES").unwrap_or(true),
                            is_primary_key: key.as_deref() == Some("PRI"),
                            default_value: row.get("default_value"),
                            comment: None,
                        }
                    }).collect()
                })
        }
        super::pool::DbPool::SqlServer(p) => {
            let sql = format!(
                "SELECT c.name AS name, ty.name AS data_type, c.is_nullable AS is_nullable, \
                 dc.definition AS default_value, \
                 CASE WHEN pk.column_id IS NOT NULL THEN 'PRI' ELSE '' END AS column_key \
                 FROM sys.columns c \
                 JOIN sys.types ty ON c.user_type_id = ty.user_type_id \
                 LEFT JOIN sys.default_constraints dc ON c.default_object_id = dc.object_id \
                 LEFT JOIN (SELECT ic.column_id FROM sys.indexes i JOIN sys.index_columns ic ON i.object_id = ic.object_id AND i.index_id = ic.index_id \
                    WHERE i.is_primary_key = 1 AND i.object_id = OBJECT_ID('{}')) pk ON c.column_id = pk.column_id \
                 WHERE c.object_id = OBJECT_ID('{}') ORDER BY c.column_id",
                table.replace('\'', "''"),
                table.replace('\'', "''")
            );
            p.query_rows(&sql, 10000).await.map(|(_, rows)| {
                rows.into_iter().map(|r| {
                    let name = r.get(0).and_then(str_of).unwrap_or_default();
                    let data_type = r.get(1).and_then(str_of).unwrap_or_default();
                    let is_nullable = r.get(2).and_then(bool_of).unwrap_or(true);
                    let default_value = r.get(3).and_then(str_of);
                    let key = r.get(4).and_then(str_of);
                    ColumnInfo {
                        name,
                        data_type,
                        is_nullable,
                        is_primary_key: key.as_deref() == Some("PRI"),
                        default_value,
                        comment: None,
                    }
                }).collect()
            })
        }
        super::pool::DbPool::Oracle(p) => {
            let sql = format!(
                "SELECT c.column_name AS name, c.data_type AS data_type, c.nullable AS is_nullable, \
                 c.data_default AS default_value, \
                 CASE WHEN pk.column_name IS NOT NULL THEN 'PRI' ELSE '' END AS column_key \
                 FROM all_tab_columns c \
                 LEFT JOIN (SELECT acc.column_name FROM all_cons_columns acc JOIN all_constraints ac \
                    ON acc.constraint_name = ac.constraint_name \
                    WHERE ac.constraint_type = 'P' AND ac.owner = '{}' AND ac.table_name = '{}') pk \
                    ON c.column_name = pk.column_name \
                 WHERE c.owner = '{}' AND c.table_name = '{}' ORDER BY c.column_id",
                database.to_uppercase().replace('\'', "''"),
                table.to_uppercase().replace('\'', "''"),
                database.to_uppercase().replace('\'', "''"),
                table.to_uppercase().replace('\'', "''")
            );
            p.query_rows("", &sql, 10000).await.map(|(_, rows)| {
                rows.into_iter().map(|r| {
                    let name = r.get(0).and_then(str_of).unwrap_or_default();
                    let data_type = r.get(1).and_then(str_of).unwrap_or_default();
                    let is_nullable = r.get(2).and_then(str_of).map(|s| s == "Y").unwrap_or(true);
                    let default_value = r.get(3).and_then(str_of);
                    let key = r.get(4).and_then(str_of);
                    ColumnInfo {
                        name,
                        data_type,
                        is_nullable,
                        is_primary_key: key.as_deref() == Some("PRI"),
                        default_value,
                        comment: None,
                    }
                }).collect()
            })
        }
    };

    match result {
        Ok(list) => Response::new(200, list, "查询成功"),
        Err(e) => Response::new(300, vec![], &format!("查询失败: {}", e)),
    }
}

/// 获取表的索引信息
pub async fn db_table_indexes(config_id: String, database: String, table: String) -> Response<Vec<IndexInfo>> {
    let pool = match get_pool(&config_id) {
        Some(p) => p,
        None => return Response::new(300, vec![], "未连接到数据库"),
    };

    let result: Result<Vec<IndexInfo>, Error> = match &pool {
        super::pool::DbPool::MySQL(p) => {
            let sql = format!(
                "SELECT INDEX_NAME AS name, COLUMN_NAME AS column_name, NON_UNIQUE AS non_unique, INDEX_TYPE AS index_type \
                 FROM information_schema.STATISTICS WHERE TABLE_SCHEMA = '{}' AND TABLE_NAME = '{}' ORDER BY INDEX_NAME, SEQ_IN_INDEX",
                database, table
            );
            sqlx::query(&sql)
                .fetch_all(p)
                .await
                .map_err(Error::from)
                .map(|rows| {
                    rows.iter().map(|row| {
                        let non_unique: i32 = row.get("non_unique");
                        let idx_name: String = row.get("name");
                        IndexInfo {
                            name: idx_name.clone(),
                            column_name: row.get("column_name"),
                            is_unique: non_unique == 0,
                            is_primary: idx_name == "PRIMARY",
                            index_type: row.get("index_type"),
                        }
                    }).collect()
                })
        }
        super::pool::DbPool::PostgreSQL(p) => {
            let sql = format!(
                "SELECT i.relname AS name, a.attname AS column_name, ix.indisunique AS is_unique, ix.indisprimary AS is_primary, am.amname AS index_type \
                 FROM pg_class t, pg_class i, pg_index ix, pg_attribute a, pg_am am \
                 WHERE t.oid = ix.indrelid AND i.oid = ix.indexrelid AND a.attrelid = t.oid AND a.attnum = ANY(ix.indkey) \
                 AND i.relam = am.oid AND t.relkind = 'r' AND t.relname = '{}' ORDER BY i.relname, a.attnum",
                table
            );
            sqlx::query(&sql)
                .fetch_all(p)
                .await
                .map_err(Error::from)
                .map(|rows| {
                    rows.iter().map(|row| IndexInfo {
                        name: row.get("name"),
                        column_name: row.get("column_name"),
                        is_unique: row.get("is_unique"),
                        is_primary: row.get("is_primary"),
                        index_type: row.get("index_type"),
                    }).collect()
                })
        }
        super::pool::DbPool::SqlServer(p) => {
            let sql = format!(
                "SELECT i.name AS name, c.name AS column_name, i.is_unique AS is_unique, i.is_primary_key AS is_primary, i.type_desc AS index_type \
                 FROM sys.indexes i \
                 JOIN sys.index_columns ic ON i.object_id = ic.object_id AND i.index_id = ic.index_id \
                 JOIN sys.columns c ON ic.object_id = c.object_id AND ic.column_id = c.column_id \
                 WHERE i.object_id = OBJECT_ID('{}') ORDER BY i.name, ic.key_ordinal",
                table.replace('\'', "''")
            );
            p.query_rows(&sql, 10000).await.map(|(_, rows)| {
                rows.into_iter().map(|r| IndexInfo {
                    name: r.get(0).and_then(str_of).unwrap_or_default(),
                    column_name: r.get(1).and_then(str_of).unwrap_or_default(),
                    is_unique: r.get(2).and_then(bool_of).unwrap_or(false),
                    is_primary: r.get(3).and_then(bool_of).unwrap_or(false),
                    index_type: r.get(4).and_then(str_of),
                }).collect()
            })
        }
        super::pool::DbPool::Oracle(p) => {
            let sql = format!(
                "SELECT i.index_name AS name, ic.column_name AS column_name, i.uniqueness AS is_unique, \
                 CASE WHEN c.constraint_type = 'P' THEN 1 ELSE 0 END AS is_primary, i.index_type AS index_type \
                 FROM all_indexes i \
                 JOIN all_ind_columns ic ON i.index_name = ic.index_name AND i.table_owner = ic.table_owner \
                 LEFT JOIN all_constraints c ON c.constraint_name = i.index_name AND c.constraint_type = 'P' \
                 WHERE i.table_owner = '{}' AND i.table_name = '{}' ORDER BY i.index_name, ic.column_position",
                database.to_uppercase().replace('\'', "''"),
                table.to_uppercase().replace('\'', "''")
            );
            p.query_rows("", &sql, 10000).await.map(|(_, rows)| {
                rows.into_iter().map(|r| {
                    let uniqueness = r.get(2).and_then(str_of).unwrap_or_default();
                    IndexInfo {
                        name: r.get(0).and_then(str_of).unwrap_or_default(),
                        column_name: r.get(1).and_then(str_of).unwrap_or_default(),
                        is_unique: uniqueness.eq_ignore_ascii_case("UNIQUE"),
                        is_primary: r.get(3).and_then(num_of).map(|n| n != 0).unwrap_or(false),
                        index_type: r.get(4).and_then(str_of),
                    }
                }).collect()
            })
        }
    };

    match result {
        Ok(list) => Response::new(200, list, "查询成功"),
        Err(e) => Response::new(300, vec![], &format!("查询失败: {}", e)),
    }
}

/// 获取建表 DDL
pub async fn db_table_schema(config_id: String, database: String, table: String) -> Response<String> {
    let pool = match get_pool(&config_id) {
        Some(p) => p,
        None => return Response::new(300, String::new(), "未连接到数据库"),
    };

    let result: Result<String, Error> = match &pool {
        super::pool::DbPool::MySQL(p) => {
            let sql = format!("SHOW CREATE TABLE `{}`.`{}`", database, table);
            let row = sqlx::query(&sql).fetch_one(p).await;
            match row {
                Ok(r) => {
                    let ddl: String = r.try_get(1).unwrap_or_default();
                    Ok(ddl)
                }
                Err(e) => Err(e.into()),
            }
        }
        super::pool::DbPool::PostgreSQL(p) => {
            // PostgreSQL 没有直接的 SHOW CREATE TABLE，需要手动构建
            let sql = format!(
                "SELECT column_name, data_type, character_maximum_length, is_nullable, column_default \
                 FROM information_schema.columns WHERE table_name = '{}' AND table_schema = 'public' ORDER BY ordinal_position",
                table
            );
            match sqlx::query(&sql).fetch_all(p).await {
                Ok(rows) => {
                    let mut ddl = format!("CREATE TABLE {} (\n", table);
                    for (i, row) in rows.iter().enumerate() {
                        let col_name: String = row.get("column_name");
                        let data_type: String = row.get("data_type");
                        let max_len: Option<i32> = row.get("character_maximum_length");
                        let nullable: String = row.get("is_nullable");
                        let default: Option<String> = row.get("column_default");

                        let type_str = if let Some(len) = max_len {
                            format!("{}({})", data_type, len)
                        } else {
                            data_type
                        };

                        let null_str = if nullable == "YES" { "NULL" } else { "NOT NULL" };
                        let default_str = default.map(|d| format!(" DEFAULT {}", d)).unwrap_or_default();

                        if i > 0 { ddl.push_str(",\n"); }
                        ddl.push_str(&format!("  {} {} {}{}", col_name, type_str, null_str, default_str));
                    }
                    ddl.push_str("\n);");
                    Ok(ddl)
                }
                Err(e) => Err(e.into()),
            }
        }
        super::pool::DbPool::SqlServer(p) => {
            let sql = format!(
                "SELECT c.name, ty.name, c.max_length, c.precision, c.scale, c.is_nullable, dc.definition \
                 FROM sys.columns c \
                 JOIN sys.types ty ON c.user_type_id = ty.user_type_id \
                 LEFT JOIN sys.default_constraints dc ON c.default_object_id = dc.object_id \
                 WHERE c.object_id = OBJECT_ID('{}') ORDER BY c.column_id",
                table.replace('\'', "''")
            );
            match p.query_rows(&sql, 10000).await {
                Ok((_, rows)) => {
                    let mut ddl = format!("CREATE TABLE {} (\n", table);
                    for (i, r) in rows.into_iter().enumerate() {
                        let col_name = r.get(0).and_then(str_of).unwrap_or_default();
                        let type_name = r.get(1).and_then(str_of).unwrap_or_default();
                        let max_len = r.get(2).and_then(num_of).unwrap_or(0);
                        let precision = r.get(3).and_then(num_of).unwrap_or(0);
                        let scale = r.get(4).and_then(num_of).unwrap_or(0);
                        let nullable = r.get(5).and_then(bool_of).unwrap_or(true);
                        let default = r.get(6).and_then(str_of);

                        let type_str = mssql_type_str(&type_name, max_len, precision, scale);
                        let null_str = if nullable { "NULL" } else { "NOT NULL" };
                        let default_str = default.map(|d| format!(" DEFAULT {}", d)).unwrap_or_default();
                        if i > 0 { ddl.push_str(",\n"); }
                        ddl.push_str(&format!("  {} {} {}{}", col_name, type_str, null_str, default_str));
                    }
                    ddl.push_str("\n);");
                    Ok(ddl)
                }
                Err(e) => Err(e),
            }
        }
        super::pool::DbPool::Oracle(p) => {
            let sql = format!(
                "SELECT column_name, data_type, data_length, data_precision, data_scale, nullable, data_default \
                 FROM all_tab_columns WHERE owner = '{}' AND table_name = '{}' ORDER BY column_id",
                database.to_uppercase().replace('\'', "''"),
                table.to_uppercase().replace('\'', "''")
            );
            match p.query_rows("", &sql, 10000).await {
                Ok((_, rows)) => {
                    let mut ddl = format!("CREATE TABLE {} (\n", table);
                    for (i, r) in rows.into_iter().enumerate() {
                        let col_name = r.get(0).and_then(str_of).unwrap_or_default();
                        let type_name = r.get(1).and_then(str_of).unwrap_or_default();
                        let data_len = r.get(2).and_then(num_of).unwrap_or(0);
                        let precision = r.get(3).and_then(num_of);
                        let scale = r.get(4).and_then(num_of);
                        let nullable = r.get(5).and_then(str_of).map(|s| s == "Y").unwrap_or(true);
                        let default = r.get(6).and_then(str_of);

                        let type_str = oracle_type_str(&type_name, data_len, precision, scale);
                        let null_str = if nullable { "NULL" } else { "NOT NULL" };
                        let default_str = default.map(|d| format!(" DEFAULT {}", d)).unwrap_or_default();
                        if i > 0 { ddl.push_str(",\n"); }
                        ddl.push_str(&format!("  {} {} {}{}", col_name, type_str, null_str, default_str));
                    }
                    ddl.push_str("\n);");
                    Ok(ddl)
                }
                Err(e) => Err(e),
            }
        }
    };

    match result {
        Ok(ddl) => Response::new(200, ddl, "查询成功"),
        Err(e) => Response::new(300, String::new(), &format!("查询失败: {}", e)),
    }
}

// ==================== 内部辅助函数 ====================

/// 从 serde_json::Value 取字符串
fn str_of(v: &Value) -> Option<String> {
    match v {
        Value::String(s) => Some(s.clone()),
        _ => None,
    }
}

/// 从 serde_json::Value 取布尔
fn bool_of(v: &Value) -> Option<bool> {
    match v {
        Value::Bool(b) => Some(*b),
        Value::Number(n) => n.as_i64().map(|i| i != 0),
        Value::String(s) => s.parse::<i64>().ok().map(|i| i != 0),
        _ => None,
    }
}

/// 从 serde_json::Value 取整数
fn num_of(v: &Value) -> Option<i64> {
    match v {
        Value::Number(n) => n.as_i64(),
        Value::String(s) => s.parse::<i64>().ok(),
        _ => None,
    }
}

/// 拼 SQL Server 列类型（含长度/精度/小数位）
fn mssql_type_str(type_name: &str, max_len: i64, precision: i64, scale: i64) -> String {
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

/// 拼 Oracle 列类型（含长度/精度/小数位）
fn oracle_type_str(type_name: &str, data_len: i64, precision: Option<i64>, scale: Option<i64>) -> String {
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
