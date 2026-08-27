/*! # 元数据查询模块
获取数据库、表、字段、索引等元数据信息。
*/
use crate::response::response::Response;
use anyhow::Result;
use serde::{Deserialize, Serialize};
use sqlx::Row;
use super::pool::get_pool;
use super::operations::ColumnInfo;

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

    let result = match &pool {
        super::pool::DbPool::MySQL(p) => {
            sqlx::query("SELECT SCHEMA_NAME AS name, DEFAULT_CHARACTER_SET_NAME AS charset, DEFAULT_COLLATION_NAME AS collation FROM information_schema.SCHEMATA ORDER BY SCHEMA_NAME")
                .fetch_all(p)
                .await
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
                .map(|rows| {
                    rows.iter().map(|row| DatabaseInfo {
                        name: row.get("name"),
                        charset: None,
                        collation: None,
                    }).collect()
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

    let result = match &pool {
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

    let result = match &pool {
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

    let result = match &pool {
        super::pool::DbPool::MySQL(p) => {
            let sql = format!(
                "SELECT INDEX_NAME AS name, COLUMN_NAME AS column_name, NON_UNIQUE AS non_unique, INDEX_TYPE AS index_type \
                 FROM information_schema.STATISTICS WHERE TABLE_SCHEMA = '{}' AND TABLE_NAME = '{}' ORDER BY INDEX_NAME, SEQ_IN_INDEX",
                database, table
            );
            sqlx::query(&sql)
                .fetch_all(p)
                .await
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

    let result = match &pool {
        super::pool::DbPool::MySQL(p) => {
            let sql = format!("SHOW CREATE TABLE `{}`.`{}`", database, table);
            let row = sqlx::query(&sql).fetch_one(p).await;
            match row {
                Ok(r) => {
                    let ddl: String = r.try_get(1).unwrap_or_default();
                    Ok(ddl)
                }
                Err(e) => Err(e),
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
                Err(e) => Err(e),
            }
        }
    };

    match result {
        Ok(ddl) => Response::new(200, ddl, "查询成功"),
        Err(e) => Response::new(300, String::new(), &format!("查询失败: {}", e)),
    }
}
