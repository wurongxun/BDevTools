/*! # 结果网格编辑：批量写回 + 事务

把「用户在网格里改了若干格 → 生成 SQL → 在一个事务里提交」这条链路单独抽出来的原因：

1. **原来的 `db_insert_row / db_update_row / db_delete_row` 是一条语句一次 autocommit**
   （现已删除，写库统一走这里）。
   批量编辑时「前 3 条成功、第 4 条失败」会留下无法回滚的半成品，这对写生产库是不可接受的。
2. **SQL 只有一份生成逻辑**，前端的「预览」与后端的「执行」必须看到完全一致的结果，
   所以这里同时提供 `db_preview_edits`（只生成语句、不执行）供前端预览复用，
   而不是让 TS 侧再照抄一遍拼串规则（抄两次必然出现「预览与实际不一样」）。
3. 每个子操作**单独返回受影响行数**，前端据此判断「行已被别处删掉」这类冲突
   （受影响 0 行 ≠ 成功）。

支持：MySQL / PostgreSQL 走 sqlx 事务；SQL Server 借一条连接 + `BEGIN TRANSACTION`；
Oracle 借 ODBC 的 `set_autocommit(false)` + `commit/rollback`（即开即关的连接模型下，
必须在**同一个连接**里顺序执行才谈得上事务）。
*/
use crate::response::response::Response;
use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};

use super::dialect::{dialect_of, Dialect};
use super::pool::{get_pool, DbPool};

/// 列名 + 值。
/// 刻意用 `Vec<KeyValue>` 而不是 `HashMap`：HashMap 的迭代顺序会让同一份数据
/// 每次生成的 SQL 字段顺序不同 —— 日志没法比对，预览与实际也可能「看着不一样」。
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct KeyValue {
    pub column: String,
    pub value: serde_json::Value,
}

/// 一次网格写操作
///
/// `expect_one`：这条语句**必须恰好命中一行**，否则整批失败回滚。
/// 它服务于「没有主键也没有唯一索引」的表（见 `require_exact_one` 的说明）：
/// 那种行的身份只能靠**一堆列的原值**拼出来，命中 0 行说明数据已被改过、
/// 命中多行说明这条 WHERE 根本不足以定位一行 —— 两种情况都不许提交。
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(tag = "op")]
pub enum EditOp {
    /// 修改已有行：`keys` 定位一行，`data` 是要改的字段
    #[serde(rename = "update")]
    Update {
        table: String,
        keys: Vec<KeyValue>,
        data: Vec<KeyValue>,
        #[serde(default)]
        expect_one: bool,
    },
    /// 新增行（网格里「新增」也会走这里，便于和改动一起在一个事务里提交）
    #[serde(rename = "insert")]
    Insert { table: String, data: Vec<KeyValue> },
    /// 删除行
    #[serde(rename = "delete")]
    Delete {
        table: String,
        keys: Vec<KeyValue>,
        #[serde(default)]
        expect_one: bool,
    },
}

/// 某条语句命中了 N 行却要求恰好 1 行时的报错文案（前端据此提示「定位到 N 行」）
pub fn exact_one_violation(affected: u64) -> String {
    if affected == 0 {
        "没有匹配到任何行：这行数据已被改过或已删除（已回滚）".to_string()
    } else {
        format!("这条条件匹配到 {} 行，无法唯一定位（已回滚）", affected)
    }
}

/// 单个子操作的结果
#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct EditOutcome {
    /// 对应传入 ops 的下标（前端据此把错误落回具体单元格）
    pub index: usize,
    pub ok: bool,
    pub affected: u64,
    pub error: Option<String>,
}

/// 批量写结果
#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct EditBatchResult {
    pub outcomes: Vec<EditOutcome>,
    /// true = 全部在一个事务里（要么全成功要么全回滚）；false = 已经没法保证原子性
    pub transactional: bool,
    /// 已提交的语句数
    pub committed: usize,
}

/// 预览结果：每个子操作一条 SQL（与实际执行完全同源）
#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct EditPreview {
    pub statements: Vec<EditStatement>,
}

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct EditStatement {
    pub index: usize,
    pub sql: String,
    pub error: Option<String>,
    /// 执行时是否要求「恰好命中一行」（预览里要标出来：这种 WHERE 是拼出来的，不是主键）
    #[serde(default)]
    pub expect_one: bool,
}

impl EditOp {
    fn table(&self) -> &str {
        match self {
            EditOp::Update { table, .. } | EditOp::Insert { table, .. } | EditOp::Delete { table, .. } => table,
        }
    }

    /// 这条语句是否要求「恰好命中一行」（见 `expect_one` 字段的说明）。
    pub fn require_exact_one(&self) -> bool {
        match self {
            EditOp::Update { expect_one, .. } | EditOp::Delete { expect_one, .. } => *expect_one,
            // INSERT 没有「命中几行」的概念
            EditOp::Insert { .. } => false,
        }
    }
}

/// keys → WHERE 片段。NULL 必须写成 `IS NULL`：`col = NULL` 恒不成立，会静默更新 0 行。
fn key_condition(dialect: &Dialect, keys: &[KeyValue]) -> Result<String> {
    if keys.is_empty() {
        return Err(anyhow!("缺少定位行的键列"));
    }
    Ok(keys
        .iter()
        .map(|k| {
            let col = dialect.quote_ident(&k.column);
            if k.value.is_null() {
                format!("{} IS NULL", col)
            } else {
                format!("{} = {}", col, dialect.literal(&k.value))
            }
        })
        .collect::<Vec<_>>()
        .join(" AND "))
}

/// 生成一条子操作的 SQL（预览与执行**共用**，保证两边永远一致）
fn build_statement(dialect: &Dialect, database: &str, op: &EditOp) -> Result<String> {
    let target = dialect.qualified_table(database, op.table());
    match op {
        EditOp::Update { keys, data, .. } => {
            if data.is_empty() {
                return Err(anyhow!("没有要更新的字段"));
            }
            // 按列名排序：同一份改动每次生成同样的语句，便于日志比对与复盘
            let mut items: Vec<&KeyValue> = data.iter().collect();
            items.sort_by(|a, b| a.column.cmp(&b.column));
            let set_str = items
                .iter()
                .map(|kv| format!("{} = {}", dialect.quote_ident(&kv.column), dialect.literal(&kv.value)))
                .collect::<Vec<_>>()
                .join(", ");
            let where_str = key_condition(dialect, keys)?;
            Ok(format!("UPDATE {} SET {} WHERE {}", target, set_str, where_str))
        }
        EditOp::Insert { data, .. } => {
            if data.is_empty() {
                return Err(anyhow!("没有要插入的字段"));
            }
            let mut items: Vec<&KeyValue> = data.iter().collect();
            items.sort_by(|a, b| a.column.cmp(&b.column));
            let cols = items.iter().map(|kv| dialect.quote_ident(&kv.column)).collect::<Vec<_>>().join(", ");
            let vals = items.iter().map(|kv| dialect.literal(&kv.value)).collect::<Vec<_>>().join(", ");
            Ok(format!("INSERT INTO {} ({}) VALUES ({})", target, cols, vals))
        }
        EditOp::Delete { keys, .. } => {
            let where_str = key_condition(dialect, keys)?;
            Ok(format!("DELETE FROM {} WHERE {}", target, where_str))
        }
    }
}

fn build_statements(dialect: &Dialect, database: &str, ops: &[EditOp]) -> Vec<Result<String>> {
    ops.iter().map(|op| build_statement(dialect, database, op)).collect()
}

/// 预览：只生成语句，不执行
pub async fn db_preview_edits(
    config_id: String,
    database: String,
    ops: Vec<EditOp>,
) -> Response<EditPreview> {
    let pool = match get_pool(&config_id) {
        Some(p) => p,
        None => return Response::new(300, EditPreview::default(), "未连接到数据库"),
    };
    let dialect = dialect_of(&pool);
    let statements = build_statements(&dialect, &database, &ops)
        .into_iter()
        .enumerate()
        .map(|(index, r)| {
            let expect_one = ops.get(index).map(|op| op.require_exact_one()).unwrap_or(false);
            match r {
                Ok(sql) => EditStatement { index, sql, error: None, expect_one },
                Err(e) => EditStatement { index, sql: String::new(), error: Some(e.to_string()), expect_one },
            }
        })
        .collect();
    Response::new(200, EditPreview { statements }, "生成成功")
}

/// 执行：一个事务里跑完所有语句，失败整体回滚；逐个子操作返回受影响行数
pub async fn db_apply_edits(
    config_id: String,
    database: String,
    ops: Vec<EditOp>,
) -> Response<EditBatchResult> {
    let pool = match get_pool(&config_id) {
        Some(p) => p,
        None => return Response::new(300, EditBatchResult::default(), "未连接到数据库"),
    };
    if ops.is_empty() {
        return Response::new(300, EditBatchResult::default(), "没有要提交的改动");
    }

    let dialect = dialect_of(&pool);
    let built = build_statements(&dialect, &database, &ops);
    let expect_one: Vec<bool> = ops.iter().map(|op| op.require_exact_one()).collect();

    match exec_batch(&pool, &dialect, &database, built, &expect_one).await {
        Ok(mut result) => {
            let failed = result.outcomes.iter().any(|o| !o.ok);
            let msg = if failed {
                "部分语句执行失败（已回滚）"
            } else {
                "已提交"
            };
            result.transactional = true;
            Response::new(if failed { 300 } else { 200 }, result, msg)
        }
        Err(e) => Response::new(300, EditBatchResult::default(), &format!("提交失败: {}", e)),
    }
}

/// 按连接池类型分发。四条分支的**语义必须一致**：任一条失败 ⇒ 前面的一律不算数。
///
/// `expect_one` 与 `built` 一一对应：为 true 的语句命中数必须恰好是 1，
/// 否则当作失败并回滚整批（见 `require_exact_one`）。
async fn exec_batch(
    pool: &DbPool,
    dialect: &Dialect,
    database: &str,
    built: Vec<Result<String>>,
    expect_one: &[bool],
) -> Result<EditBatchResult> {
    match pool {
        DbPool::MySQL(p) => exec_batch_sqlx_mysql(p, built, expect_one).await,
        DbPool::PostgreSQL(p) => exec_batch_sqlx_pg(p, built, expect_one).await,
        DbPool::SqlServer(p) => {
            let affects = p.execute_batch_transaction(&built, expect_one).await?;
            Ok(outcomes_from(&stringify_errors(affects), built.len()))
        }
        DbPool::Oracle(p) => {
            // Oracle 的每条语句前可带一段 ALTER SESSION 切 schema（与既有 single-op 一致）
            let setup = if database.is_empty() {
                String::new()
            } else {
                dialect.use_database_sql(database)
            };
            let affects = p.execute_batch_transaction(&setup, &built, expect_one).await?;
            Ok(outcomes_from(&stringify_errors(affects), built.len()))
        }
    }
}

/// 把「每条语句的执行结果」翻译成统一的 outcome。
/// `affected: None` 表示这条语句在遇到错误前还没跑到（被跳过）。
fn outcomes_from(affects: &[std::result::Result<u64, String>], total: usize) -> EditBatchResult {
    let mut outcomes = Vec::with_capacity(total);
    let mut committed = 0usize;
    for (index, item) in affects.iter().enumerate() {
        match item {
            Ok(n) => {
                committed += 1;
                outcomes.push(EditOutcome { index, ok: true, affected: *n, error: None });
            }
            Err(msg) => outcomes.push(EditOutcome {
                index,
                ok: false,
                affected: 0,
                error: Some(msg.clone()),
            }),
        }
    }
    EditBatchResult { outcomes, transactional: true, committed }
}

/// anyhow 错误不能直接喂给前端，统一转成字符串
fn stringify_errors(affects: Vec<Result<u64>>) -> Vec<std::result::Result<u64, String>> {
    affects.into_iter().map(|r| r.map_err(|e| e.to_string())).collect()
}

/// MySQL 事务
async fn exec_batch_sqlx_mysql(
    pool: &sqlx::MySqlPool,
    built: Vec<Result<String>>,
    expect_one: &[bool],
) -> Result<EditBatchResult> {
    use sqlx::Executor;
    let mut tx = pool.begin().await?;
    let mut affects: Vec<std::result::Result<u64, String>> = Vec::with_capacity(built.len());
    for (i, item) in built.iter().enumerate() {
        match item {
            Ok(sql) => match Executor::execute(&mut *tx, sql.as_str()).await {
                Ok(res) => {
                    let n = res.rows_affected();
                    // 要求恰好一行却不是一行 ⇒ 失败（事务内回滚，数据不会留下半成品）
                    if expect_one.get(i) == Some(&true) && n != 1 {
                        affects.push(Err(exact_one_violation(n)));
                        break;
                    }
                    affects.push(Ok(n));
                }
                Err(e) => {
                    affects.push(Err(e.to_string()));
                    break;
                }
            },
            Err(e) => {
                affects.push(Err(e.to_string()));
                break;
            }
        }
    }
    if affects.iter().any(|r| r.is_err()) || affects.len() < built.len() {
        tx.rollback().await?;
    } else {
        tx.commit().await?;
    }
    Ok(outcomes_from(&affects, built.len()))
}

/// PostgreSQL 事务
async fn exec_batch_sqlx_pg(
    pool: &sqlx::PgPool,
    built: Vec<Result<String>>,
    expect_one: &[bool],
) -> Result<EditBatchResult> {
    use sqlx::Executor;
    let mut tx = pool.begin().await?;
    let mut affects: Vec<std::result::Result<u64, String>> = Vec::with_capacity(built.len());
    for (i, item) in built.iter().enumerate() {
        match item {
            Ok(sql) => match Executor::execute(&mut *tx, sql.as_str()).await {
                Ok(res) => {
                    let n = res.rows_affected();
                    if expect_one.get(i) == Some(&true) && n != 1 {
                        affects.push(Err(exact_one_violation(n)));
                        break;
                    }
                    affects.push(Ok(n));
                }
                Err(e) => {
                    affects.push(Err(e.to_string()));
                    break;
                }
            },
            Err(e) => {
                affects.push(Err(e.to_string()));
                break;
            }
        }
    }
    if affects.iter().any(|r| r.is_err()) || affects.len() < built.len() {
        tx.rollback().await?;
    } else {
        tx.commit().await?;
    }
    Ok(outcomes_from(&affects, built.len()))
}
#[cfg(test)]
mod edit_sql_tests {
    use super::*;
    use serde_json::json;

    fn kv(column: &str, value: serde_json::Value) -> KeyValue {
        KeyValue {
            column: column.to_string(),
            value,
        }
    }

    fn update() -> EditOp {
        EditOp::Update {
            table: "users".to_string(),
            keys: vec![kv("id", json!(7))],
            data: vec![kv("name", json!("O'Brien")), kv("age", json!(30))],
            expect_one: false,
        }
    }

    /// 四种方言各自的标识符引用必须生效 —— 这正是原来 CRUD 硬编码反引号时唯一通不过的地方。
    #[test]
    fn update_uses_dialect_quoting() {
        let cases = [
            (
                Dialect::MySQL,
                "UPDATE `app`.`users` SET `age` = 30, `name` = 'O''Brien' WHERE `id` = 7",
            ),
            (
                Dialect::PostgreSQL,
                "UPDATE \"app\".\"users\" SET \"age\" = 30, \"name\" = 'O''Brien' WHERE \"id\" = 7",
            ),
            (
                Dialect::SqlServer,
                "UPDATE [app].[users] SET [age] = 30, [name] = 'O''Brien' WHERE [id] = 7",
            ),
            (
                Dialect::Oracle,
                "UPDATE \"APP\".\"USERS\" SET \"AGE\" = 30, \"NAME\" = 'O''Brien' WHERE \"ID\" = 7",
            ),
        ];
        for (dialect, want) in cases {
            let got = build_statement(&dialect, "app", &update()).expect("应生成成功");
            assert_eq!(got, want, "{:?} 方言", dialect);
        }
    }

    /// SET 列表按列名排序：同一份改动永远得到同一条语句，日志才好比对、事后才复盘得动。
    #[test]
    fn set_clause_is_stable() {
        let sql = build_statement(&Dialect::MySQL, "app", &update()).expect("生成成功");
        assert!(sql.contains("SET `age` = 30, `name` = 'O''Brien'"), "实际: {}", sql);
    }

    /// 键值为 NULL 必须写成 `IS NULL`：拼成 `= NULL` 恒不成立，会「成功」地更新 0 行。
    #[test]
    fn null_key_becomes_is_null() {
        let op = EditOp::Delete {
            table: "t".to_string(),
            keys: vec![kv("a", serde_json::Value::Null), kv("b", json!(1))],
            expect_one: false,
        };
        let sql = build_statement(&Dialect::MySQL, "", &op).expect("生成成功");
        assert_eq!(sql, "DELETE FROM `t` WHERE `a` IS NULL AND `b` = 1");
    }

    /// MySQL 的字符串字面量必须转义反斜杠；其余三种在标准字符串里 `\` 是普通字符，
    /// 照搬 MySQL 的转义会写错数据 —— 所以按方言区分，不是偷懒。
    #[test]
    fn mysql_escapes_backslash_unlike_others() {
        let op = EditOp::Insert {
            table: "t".to_string(),
            data: vec![kv("s", json!("a\\b"))],
        };
        let mysql = build_statement(&Dialect::MySQL, "", &op).expect("生成成功");
        assert_eq!(mysql, "INSERT INTO `t` (`s`) VALUES ('a\\\\b')");
        let pg = build_statement(&Dialect::PostgreSQL, "", &op).expect("生成成功");
        assert_eq!(pg, "INSERT INTO \"t\" (\"s\") VALUES ('a\\b')");
    }

    /// 布尔按方言落成字面量（Oracle 的 SQL 层没有 BOOLEAN 类型）。
    #[test]
    fn bool_literal_follows_dialect() {
        let op = EditOp::Insert {
            table: "t".to_string(),
            data: vec![kv("flag", json!(true))],
        };
        assert!(build_statement(&Dialect::PostgreSQL, "", &op)
            .expect("生成")
            .contains("VALUES (TRUE)"));
        assert!(build_statement(&Dialect::MySQL, "", &op)
            .expect("生成")
            .contains("VALUES (1)"));
    }

    /// 空操作就地报错，不许生成「没有 SET」或「没有 WHERE」的危险语句。
    #[test]
    fn empty_data_is_rejected() {
        let cases = [
            EditOp::Update {
                table: "t".to_string(),
                keys: vec![kv("id", json!(1))],
                data: vec![],
                expect_one: false,
            },
            EditOp::Insert {
                table: "t".to_string(),
                data: vec![],
            },
            EditOp::Delete {
                table: "t".to_string(),
                keys: vec![],
                expect_one: false,
            },
        ];
        for op in cases {
            assert!(build_statement(&Dialect::MySQL, "", &op).is_err(), "本应报错: {:?}", op);
        }
    }

    /// `expect_one` 默认关闭：老调用方（主键定位）不受影响，
    /// 而且「没带这个字段的 JSON」反序列化后必须是 false，不是「没校验」。
    #[test]
    fn expect_one_defaults_to_false() {
        let json = r#"{"op":"update","table":"t","keys":[{"column":"id","value":1}],"data":[{"column":"a","value":2}]}"#;
        let op: EditOp = serde_json::from_str(json).expect("应能反序列化");
        assert!(!op.require_exact_one());
        // INSERT 没有「命中几行」的概念，恒为 false
        let ins = EditOp::Insert { table: "t".to_string(), data: vec![kv("a", json!(1))] };
        assert!(!ins.require_exact_one());
    }

    /// 命中数不对时的文案要说清「为什么回滚」：0 行 = 数据已被改过，N 行 = 定位条件不够。
    #[test]
    fn exact_one_violation_explains_why() {
        assert!(exact_one_violation(0).contains("没有匹配到任何行"));
        assert!(exact_one_violation(3).contains("3 行"));
    }

    /// outcome 的下标必须与入参 ops 对齐，前端据此把错误落回具体单元格。
    #[test]
    fn outcomes_keep_index_alignment() {
        let affects = vec![Ok(1u64), Err("boom".to_string()), Ok(0u64)];
        let r = outcomes_from(&affects, 3);
        assert_eq!(r.outcomes.len(), 3);
        assert!(r.outcomes[0].ok && r.outcomes[0].index == 0);
        assert_eq!(r.outcomes[1].error.as_deref(), Some("boom"));
        // affected = 0 仍然算「执行成功」，是否算成功由前端判定（行已被别处删掉）
        assert!(r.outcomes[2].ok && r.outcomes[2].affected == 0);
        assert!(r.transactional);
    }
}
