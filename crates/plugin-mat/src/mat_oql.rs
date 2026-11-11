//! # OQL 安全子集（对象查询语言）
//!
//! Eclipse MAT 的 OQL 是「类 SQL + JavaScript 表达式」的完整语言，能力来自**任意方法
//! 调用**——这在只读分析器里不可接受（可执行静态初始化、可反射、可死循环）。本模块只
//! 实现一个**封闭子集**，语法面小到可以逐条核对安全性：
//!
//! ```text
//! SELECT * | @id, @class, @shallow, @kind, <字段名> [, …]
//! FROM INSTANCEOF <类名通配>
//! [WHERE <条件>]
//! [LIMIT <n>]
//! ```
//!
//! 条件只允许「字段/内建列 与 字面量」的比较，布尔组合用 `and` / `or` / `not` 与括号：
//!
//! - 运算符：`= == != <> < <= > >=`，字符串列（`@class` / `@kind`）额外支持
//!   `contains` / `startsWith` / `endsWith`；
//! - 字面量：整数 / 浮点 / `'字符串'` / `"字符串"` / `true` / `false` / `null`；
//! - 内建列：`@id`、`@class`、`@shallow`、`@kind`。
//!
//! **明确拒绝**：任意函数调用（`toString()`、`retainedSet()`、`dominators()`）、
//! 反射、子查询、`;` / 注释符 / 反引号、`sizeof()` 之类未列出的内建。拒绝策略是
//! 「白名单外的词一律报错」，不是「黑名单过滤」——后者永远漏。
//!
//! ## 为什么不能比较对象的字符串值
//! `s.value = 'abc'` 需要沿 `String.value → char[]` 再解一次数组；一次查询里做这件事
//! 等于把值表全量物化。因此本子集**不解析引用字段的字符串值**：引用字段只能与 `null`
//! 比较。要按内容找 String，用「浪费分析」报告（那里本来就按值建索引）。
//!
//! ## 实现方式
//! 直扫 HPROF（与会话图解耦）：不建图、不占常驻内存，代价是每次查询一遍解析。
//! 候选与扫描量走 `mat_cfg::QueryLimits`（`oql_rows` / `oql_scan`），超限返回
//! 「部分结果 + note」，不返回 Err。

use crate::mat_graph::{field_type_tag, field_width, FT_OBJECT};
use crate::mat_hprof::{catch_parse, read_head, sniff_format};
use crate::mat_cfg;
use bdevtools_core::response::resp_data::ResData;
use jvm_hprof::heap_dump::SubRecord;
use serde::Serialize;
use std::collections::HashMap;

// ==================== 返回结构 ====================

#[derive(Debug, Serialize, Clone)]
pub struct OqlCell {
    pub name: String,
    pub value: String,
}

#[derive(Debug, Serialize)]
pub struct OqlRow {
    pub obj_id: String,
    pub class_name: String,
    pub kind: String,
    pub shallow_bytes: u64,
    /// 请求的列（含内建列的展开值）；`obj_id/class_name/shallow_bytes` 恒有，便于前端固定列
    pub cells: Vec<OqlCell>,
}

#[derive(Debug, Serialize)]
pub struct OqlResult {
    /// 归一化后的查询文本（前端回显 + 复制）
    pub query: String,
    /// 结果列名（`*` 时为内建列 + 对象字段名）
    pub columns: Vec<String>,
    pub rows: Vec<OqlRow>,
    /// 扫描过的实例数
    pub scanned: u64,
    /// 命中总数（可能 > rows.len()，rows 受 LIMIT 与行上限约束）
    pub matched: u64,
    pub limit: u64,
    pub truncated: bool,
    pub note: String,
    pub elapsed_ms: u64,
}

// ==================== 语法树 ====================

#[derive(Debug, Clone, PartialEq)]
enum Lit {
    Int(i64),
    Float(f64),
    Str(String),
    Bool(bool),
    Null,
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum Op {
    Eq,
    Ne,
    Lt,
    Le,
    Gt,
    Ge,
    Contains,
    StartsWith,
    EndsWith,
}

#[derive(Debug, Clone)]
enum Expr {
    And(Box<Expr>, Box<Expr>),
    Or(Box<Expr>, Box<Expr>),
    Not(Box<Expr>),
    Cmp { field: String, op: Op, lit: Lit },
}

#[derive(Debug, Clone)]
struct Query {
    columns: Vec<String>,
    wildcard: bool,
    class_pattern: String,
    filter: Option<Expr>,
    limit: usize,
}

// ==================== 词法 ====================

#[derive(Debug, Clone, PartialEq)]
enum Tok {
    Ident(String),
    Str(String),
    Num(String),
    Sym(String),
    LParen,
    RParen,
    Comma,
}

/// 把查询切成词法单元。任何不认识的字符直接报错（含 `;`、`#`、`` ` ``、`--`）。
fn lex(src: &str) -> Result<Vec<Tok>, String> {
    let chars: Vec<char> = src.chars().collect();
    let mut out: Vec<Tok> = Vec::new();
    let mut i = 0usize;
    while i < chars.len() {
        let c = chars[i];
        if c.is_whitespace() {
            i += 1;
            continue;
        }
        match c {
            '(' => {
                out.push(Tok::LParen);
                i += 1;
            }
            ')' => {
                out.push(Tok::RParen);
                i += 1;
            }
            ',' => {
                out.push(Tok::Comma);
                i += 1;
            }
            '\'' | '"' => {
                let quote = c;
                i += 1;
                let mut buf = String::new();
                let mut closed = false;
                while i < chars.len() {
                    let ch = chars[i];
                    if ch == '\\' && i + 1 < chars.len() {
                        // 只支持 \' \" \\ \n \t，其余按字面量
                        let next = chars[i + 1];
                        buf.push(match next {
                            'n' => '\n',
                            't' => '\t',
                            other => other,
                        });
                        i += 2;
                        continue;
                    }
                    if ch == quote {
                        closed = true;
                        i += 1;
                        break;
                    }
                    buf.push(ch);
                    i += 1;
                }
                if !closed {
                    return Err("字符串字面量缺少结束引号".to_string());
                }
                out.push(Tok::Str(buf));
            }
            ';' => return Err("不允许出现分号（只支持单条只读查询）".to_string()),
            '#' | '`' => return Err(format!("不允许的字符 `{c}`")),
            '-' if i + 1 < chars.len() && chars[i + 1] == '-' => {
                return Err("不允许 SQL 注释（--）".to_string())
            }
            '/' if i + 1 < chars.len() && chars[i + 1] == '*' => {
                return Err("不允许块注释（/* */）".to_string())
            }
            '=' | '!' | '<' | '>' => {
                let mut sym = c.to_string();
                i += 1;
                if i < chars.len() {
                    let n = chars[i];
                    if (c == '=' && n == '=') || (c == '!' && n == '=') || (c == '<' && n == '>')
                        || (c == '<' && n == '=') || (c == '>' && n == '=')
                    {
                        sym.push(n);
                        i += 1;
                    }
                }
                if c == '!' && sym == "!" {
                    return Err("不支持的运算符 `!`（请用 `!=` 或 `not`）".to_string());
                }
                out.push(Tok::Sym(sym));
            }
            _ if c.is_ascii_digit()
                || (c == '-' && i + 1 < chars.len() && chars[i + 1].is_ascii_digit())
                || (c == '.' && i + 1 < chars.len() && chars[i + 1].is_ascii_digit()) =>
            {
                let start = i;
                if c == '-' {
                    i += 1;
                }
                while i < chars.len()
                    && (chars[i].is_ascii_digit()
                        || chars[i] == '.'
                        || chars[i] == 'e'
                        || chars[i] == 'E'
                        || ((chars[i] == '+' || chars[i] == '-') && i > start && matches!(chars[i - 1], 'e' | 'E')))
                {
                    i += 1;
                }
                out.push(Tok::Num(chars[start..i].iter().collect()));
            }
            _ if c.is_alphanumeric() || c == '_' || c == '@' || c == '$' || c == '.' || c == '/'
                || c == '[' || c == ']' || c == '*' || c == '?' =>
            {
                let start = i;
                while i < chars.len() {
                    let ch = chars[i];
                    if ch.is_alphanumeric()
                        || matches!(ch, '_' | '@' | '$' | '.' | '/' | '[' | ']' | '*' | '?' | ';')
                    {
                        if ch == ';' {
                            return Err("不允许出现分号（只支持单条只读查询）".to_string());
                        }
                        i += 1;
                    } else {
                        break;
                    }
                }
                out.push(Tok::Ident(chars[start..i].iter().collect()));
            }
            _ => return Err(format!("不支持的字符 `{c}`")),
        }
    }
    Ok(out)
}

// ==================== 语法分析 ====================

struct Parser {
    toks: Vec<Tok>,
    pos: usize,
}

impl Parser {
    fn peek(&self) -> Option<&Tok> {
        self.toks.get(self.pos)
    }

    /// 取下一个标识符（小写比较关键字，但保留原文给字段名）
    fn next_ident(&mut self) -> Option<String> {
        match self.toks.get(self.pos) {
            Some(Tok::Ident(s)) => {
                self.pos += 1;
                Some(s.clone())
            }
            _ => None,
        }
    }

    fn eat_keyword(&mut self, kw: &str) -> bool {
        if let Some(Tok::Ident(s)) = self.toks.get(self.pos) {
            if s.eq_ignore_ascii_case(kw) {
                self.pos += 1;
                return true;
            }
        }
        false
    }

    fn expect_keyword(&mut self, kw: &str) -> Result<(), String> {
        if self.eat_keyword(kw) {
            Ok(())
        } else {
            Err(format!("语法错误：此处应为 `{kw}`"))
        }
    }

    fn eat_sym(&mut self, sym: &str) -> bool {
        if let Some(Tok::Sym(s)) = self.toks.get(self.pos) {
            if s == sym {
                self.pos += 1;
                return true;
            }
        }
        false
    }

    fn parse_query(&mut self) -> Result<Query, String> {
        // ---- SELECT 列 ----
        self.expect_keyword("select")?;
        let mut columns: Vec<String> = Vec::new();
        let mut wildcard = false;
        if self.eat_sym("*") {
            wildcard = true;
        } else if let Some(Tok::Ident(s)) = self.toks.get(self.pos).cloned() {
            if s == "*" {
                self.pos += 1;
                wildcard = true;
            } else {
                columns.push(self.parse_column()?);
                while self.peek() == Some(&Tok::Comma) {
                    self.pos += 1;
                    columns.push(self.parse_column()?);
                }
            }
        } else {
            return Err("语法错误：SELECT 后应为列名或 `*`".to_string());
        }

        // ---- FROM INSTANCEOF <类名> ----
        self.expect_keyword("from")?;
        self.expect_keyword("instanceof")?;
        let class_pattern = match self.next_ident() {
            Some(v) => v,
            None => return Err("语法错误：FROM INSTANCEOF 后应为类名（支持 * ? 通配）".to_string()),
        };
        if class_pattern.is_empty() {
            return Err("类名不能为空".to_string());
        }

        // ---- 可选 WHERE ----
        let mut filter = None;
        if self.eat_keyword("where") {
            let e = self.parse_or()?;
            filter = Some(e);
        }

        // ---- 可选 LIMIT ----
        let mut limit = mat_cfg::query_limits().oql_rows;
        if self.eat_keyword("limit") {
            match self.toks.get(self.pos).cloned() {
                Some(Tok::Num(n)) => {
                    self.pos += 1;
                    let v: usize = n
                        .parse()
                        .map_err(|_| format!("LIMIT 需要正整数：{n}"))?;
                    limit = v.clamp(1, mat_cfg::query_limits().oql_rows);
                }
                _ => return Err("语法错误：LIMIT 后应为数字".to_string()),
            }
        }

        if self.pos != self.toks.len() {
            return Err(format!(
                "语法错误：无法解析的尾部内容 `{}`",
                self.toks[self.pos..]
                    .iter()
                    .map(tok_text)
                    .collect::<Vec<_>>()
                    .join(" ")
            ));
        }
        Ok(Query { columns, wildcard, class_pattern, filter, limit })
    }

    fn parse_column(&mut self) -> Result<String, String> {
        let name = match self.toks.get(self.pos).cloned() {
            Some(Tok::Ident(s)) => s,
            _ => return Err("语法错误：列名无效".to_string()),
        };
        self.pos += 1;
        // 列后面紧跟 `(` 说明用户想调用函数（toString()/retainedSet() …）——一律拒绝
        if self.peek() == Some(&Tok::LParen) {
            return Err(format!(
                "不支持函数调用 `{name}(…)`：OQL 子集只允许列名与内建列（@id/@class/@shallow/@kind）"
            ));
        }
        Ok(name)
    }

    fn parse_or(&mut self) -> Result<Expr, String> {
        let mut left = self.parse_and()?;
        while self.eat_keyword("or") {
            let right = self.parse_and()?;
            left = Expr::Or(Box::new(left), Box::new(right));
        }
        Ok(left)
    }

    fn parse_and(&mut self) -> Result<Expr, String> {
        let mut left = self.parse_unary()?;
        while self.eat_keyword("and") {
            let right = self.parse_unary()?;
            left = Expr::And(Box::new(left), Box::new(right));
        }
        Ok(left)
    }

    fn parse_unary(&mut self) -> Result<Expr, String> {
        if self.eat_keyword("not") {
            return Ok(Expr::Not(Box::new(self.parse_unary()?)));
        }
        if self.peek() == Some(&Tok::LParen) {
            self.pos += 1;
            let e = self.parse_or()?;
            if self.peek() != Some(&Tok::RParen) {
                return Err("语法错误：缺少右括号".to_string());
            }
            self.pos += 1;
            return Ok(e);
        }
        self.parse_cmp()
    }

    fn parse_cmp(&mut self) -> Result<Expr, String> {
        let field = match self.toks.get(self.pos).cloned() {
            Some(Tok::Ident(s)) => s,
            _ => return Err("语法错误：条件左侧应为字段名或内建列".to_string()),
        };
        self.pos += 1;
        let op = match self.toks.get(self.pos).cloned() {
            Some(Tok::Sym(s)) => {
                self.pos += 1;
                match s.as_str() {
                    "=" | "==" => Op::Eq,
                    "!=" | "<>" => Op::Ne,
                    "<" => Op::Lt,
                    "<=" => Op::Le,
                    ">" => Op::Gt,
                    ">=" => Op::Ge,
                    other => return Err(format!("不支持的运算符 `{other}`")),
                }
            }
            Some(Tok::Ident(kw)) => {
                let lower = kw.to_ascii_lowercase();
                let op = match lower.as_str() {
                    "contains" => Op::Contains,
                    "startswith" => Op::StartsWith,
                    "endswith" => Op::EndsWith,
                    _ => return Err(format!("不支持的条件关键字 `{kw}`（可用 and/or/not）")),
                };
                self.pos += 1;
                op
            }
            _ => return Err("语法错误：条件缺少比较运算符".to_string()),
        };
        let lit = match self.toks.get(self.pos).cloned() {
            Some(Tok::Num(n)) => {
                self.pos += 1;
                if n.contains('.') || n.contains('e') || n.contains('E') {
                    Lit::Float(n.parse::<f64>().map_err(|_| format!("无效数字：{n}"))?)
                } else {
                    Lit::Int(n.parse::<i64>().map_err(|_| format!("无效整数：{n}"))?)
                }
            }
            Some(Tok::Str(s)) => {
                self.pos += 1;
                Lit::Str(s)
            }
            Some(Tok::Ident(w)) => {
                let lower = w.to_ascii_lowercase();
                self.pos += 1;
                match lower.as_str() {
                    "true" => Lit::Bool(true),
                    "false" => Lit::Bool(false),
                    "null" => Lit::Null,
                    _ => {
                        return Err(format!(
                            "字面量 `{w}` 无效：只允许数字 / 字符串 / true / false / null"
                        ))
                    }
                }
            }
            _ => return Err("语法错误：条件缺少字面量".to_string()),
        };
        Ok(Expr::Cmp { field, op, lit })
    }
}

fn tok_text(t: &Tok) -> String {
    match t {
        Tok::Ident(s) => s.clone(),
        Tok::Str(s) => format!("'{s}'"),
        Tok::Num(n) => n.clone(),
        Tok::Sym(s) => s.clone(),
        Tok::LParen => "(".to_string(),
        Tok::RParen => ")".to_string(),
        Tok::Comma => ",".to_string(),
    }
}

fn parse_oql(src: &str) -> Result<Query, String> {
    let toks = lex(src)?;
    if toks.is_empty() {
        return Err("查询为空".to_string());
    }
    let mut p = Parser { toks, pos: 0 };
    p.parse_query()
}

// ==================== 值模型与求值 ====================

#[derive(Debug, Clone, PartialEq)]
enum Val {
    Null,
    Bool(bool),
    Int(i64),
    Float(f64),
    Ref(u64),
    Str(String),
}

impl Val {
    fn as_text(&self) -> String {
        match self {
            Val::Null => "null".to_string(),
            Val::Bool(b) => b.to_string(),
            Val::Int(i) => i.to_string(),
            Val::Float(f) => f.to_string(),
            Val::Ref(id) => format!("0x{id:x}"),
            Val::Str(s) => s.clone(),
        }
    }

    fn as_num(&self) -> Option<f64> {
        match self {
            Val::Int(i) => Some(*i as f64),
            Val::Float(f) => Some(*f),
            Val::Bool(b) => Some(if *b { 1.0 } else { 0.0 }),
            _ => None,
        }
    }
}

fn class_name_matches(name: &str, pattern: &str) -> bool {
    let name = name.to_lowercase();
    let pattern = pattern.to_lowercase();
    if pattern.contains('*') || pattern.contains('?') {
        // 复用类名搜索的通配语义（`*` 任意串、`?` 单字符）
        crate::mat_refs::glob_match(&name, &pattern)
    } else {
        // 不写通配时按「类名末尾匹配」处理：`java.lang.String` 与 `String` 都能命中
        name == pattern || name.ends_with(&format!("/{pattern}")) || name.ends_with(&format!(".{pattern}"))
    }
}

/// 求值：`Ok(Some(true))` 命中；`Ok(None)` 表示字段不存在/类型不匹配（按 false 处理）；
/// `Err` 只用于内建列写错这类**查询级**错误（应当整条查询失败并提示）。
fn eval(expr: &Expr, row: &RowValues) -> Result<bool, String> {
    match expr {
        Expr::And(a, b) => Ok(eval(a, row)? && eval(b, row)?),
        Expr::Or(a, b) => Ok(eval(a, row)? || eval(b, row)?),
        Expr::Not(a) => Ok(!eval(a, row)?),
        Expr::Cmp { field, op, lit } => {
            let v = match row.lookup(field) {
                Some(v) => v,
                None => return Ok(false), // 该对象没有这个字段：条件不成立（而不是报错）
            };
            Ok(compare(&v, *op, lit))
        }
    }
}

fn compare(v: &Val, op: Op, lit: &Lit) -> bool {
    match op {
        Op::Contains | Op::StartsWith | Op::EndsWith => {
            let s = v.as_text().to_lowercase();
            let needle = match lit {
                Lit::Str(s) => s.to_lowercase(),
                _ => return false,
            };
            match op {
                Op::Contains => s.contains(&needle),
                Op::StartsWith => s.starts_with(&needle),
                _ => s.ends_with(&needle),
            }
        }
        Op::Eq | Op::Ne => {
            let eq = match (v, lit) {
                (Val::Null, Lit::Null) => true,
                (Val::Null, _) | (_, Lit::Null) => false,
                (Val::Bool(b), Lit::Bool(l)) => b == l,
                (Val::Str(s), Lit::Str(l)) => s.eq_ignore_ascii_case(l),
                (Val::Ref(id), Lit::Int(l)) => *id as i64 == *l,
                (Val::Ref(_), Lit::Str(_)) | (Val::Str(_), Lit::Int(_)) => return false,
                _ => match (v.as_num(), lit_num(lit)) {
                    (Some(a), Some(b)) => (a - b).abs() < f64::EPSILON,
                    _ => false,
                },
            };
            if op == Op::Eq {
                eq
            } else {
                !eq
            }
        }
        _ => {
            // 数值比较：引用 / 字符串没有序关系（`@class > 'a'` 这类写法无意义）
            let (a, b) = match (v.as_num(), lit_num(lit)) {
                (Some(a), Some(b)) => (a, b),
                _ => return false,
            };
            match op {
                Op::Lt => a < b,
                Op::Le => a <= b,
                Op::Gt => a > b,
                _ => a >= b,
            }
        }
    }
}

fn lit_num(l: &Lit) -> Option<f64> {
    match l {
        Lit::Int(i) => Some(*i as f64),
        Lit::Float(f) => Some(*f),
        Lit::Bool(b) => Some(if *b { 1.0 } else { 0.0 }),
        _ => None,
    }
}

struct RowValues {
    obj_id: u64,
    class_name: String,
    kind: String,
    shallow: u64,
    fields: Vec<(String, Val)>,
}

impl RowValues {
    fn lookup(&self, name: &str) -> Option<Val> {
        match name.to_ascii_lowercase().as_str() {
            "@id" => Some(Val::Int(self.obj_id as i64)),
            "@class" => Some(Val::Str(self.class_name.clone())),
            "@kind" => Some(Val::Str(self.kind.clone())),
            "@shallow" => Some(Val::Int(self.shallow as i64)),
            _ => self
                .fields
                .iter()
                .find(|(n, _)| n.eq_ignore_ascii_case(name))
                .map(|(_, v)| v.clone()),
        }
    }
}

/// 列名合法性：内建列或普通标识符。用于在扫描前把明显写错的列报出来。
fn is_builtin_col(name: &str) -> bool {
    matches!(
        name.to_ascii_lowercase().as_str(),
        "@id" | "@class" | "@kind" | "@shallow"
    )
}

// ==================== HPROF 扫描 ====================

struct ClsInfo {
    name: String,
    super_id: Option<u64>,
    instance_size: u64,
    fields: Vec<(u8, String)>,
}

const MAX_STRING_PREVIEW: usize = 96;

fn be_u(data: &[u8], off: usize, width: usize) -> Option<u64> {
    if off + width > data.len() {
        return None;
    }
    let mut v = 0u64;
    for &b in &data[off..off + width] {
        v = (v << 8) | b as u64;
    }
    Some(v)
}

/// 解一个实例的字段值（只解该查询要用的字段名；`want` 为空表示全解）
fn decode_fields(
    data: &[u8],
    chain: &[(u8, String)],
    id_width: usize,
    want: &[String],
) -> Vec<(String, Val)> {
    let mut out = Vec::new();
    let mut off = 0usize;
    for (tag, name) in chain {
        let w = if *tag == FT_OBJECT { id_width } else { field_width(*tag) };
        if w == 0 {
            break;
        }
        let raw = match be_u(data, off, w) {
            Some(v) => v,
            None => break,
        };
        off += w;
        // 只解查询真正用到的字段（`*` 只回内建列时不解析任何字段值）
        if !want.iter().any(|w| w.eq_ignore_ascii_case(name)) {
            continue;
        }
        let v = match *tag {
            FT_OBJECT => {
                if raw == 0 {
                    Val::Null
                } else {
                    Val::Ref(raw)
                }
            }
            1 => Val::Bool(raw != 0),
            2 => Val::Int(raw as i64),
            3 => Val::Float(f32::from_bits(raw as u32) as f64),
            4 => Val::Float(f64::from_bits(raw)),
            5 => Val::Int(raw as i8 as i64),
            6 => Val::Int(raw as i16 as i64),
            7 => Val::Int(raw as i32 as i64),
            8 => Val::Int(raw as i64),
            _ => Val::Int(raw as i64),
        };
        out.push((name.clone(), v));
    }
    out
}

/// 条件里出现的字段名（用于只解必要字段）
fn expr_fields(e: &Expr, out: &mut Vec<String>) {
    match e {
        Expr::And(a, b) | Expr::Or(a, b) => {
            expr_fields(a, out);
            expr_fields(b, out);
        }
        Expr::Not(a) => expr_fields(a, out),
        Expr::Cmp { field, .. } => {
            if !is_builtin_col(field) && !out.iter().any(|f| f.eq_ignore_ascii_case(field)) {
                out.push(field.clone());
            }
        }
    }
}

fn chain_of(cid: u64, classes: &HashMap<u64, ClsInfo>) -> Option<Vec<(u8, String)>> {
    let mut out: Vec<(u8, String)> = Vec::new();
    let mut cur = Some(cid);
    let mut guard = 0;
    while let Some(id) = cur {
        guard += 1;
        if guard > 64 {
            return None;
        }
        let ci = classes.get(&id)?;
        out.extend(ci.fields.iter().cloned());
        cur = ci.super_id;
    }
    Some(out)
}

/// 类继承链上任一名字命中通配/后缀 即算命中（`INSTANCEOF java.lang.Throwable` 能命中子类）
fn chain_matches(
    cid: u64,
    classes: &HashMap<u64, ClsInfo>,
    pattern: &str,
    memo: &mut HashMap<u64, bool>,
) -> bool {
    if let Some(v) = memo.get(&cid) {
        return *v;
    }
    let mut cur = Some(cid);
    let mut guard = 0;
    let mut hit = false;
    while let Some(id) = cur {
        guard += 1;
        if guard > 64 {
            break;
        }
        match classes.get(&id) {
            Some(ci) => {
                if class_name_matches(&ci.name, pattern) {
                    hit = true;
                    break;
                }
                cur = ci.super_id;
            }
            None => break,
        }
    }
    memo.insert(cid, hit);
    hit
}

// ==================== 命令入口 ====================


/// 执行一条 OQL 子集查询（纯同步，调用方需 spawn_blocking）
pub fn mat_oql_query(path: &str, query: &str) -> Result<ResData<OqlResult>, String> {
    let started = std::time::Instant::now();
    let q = parse_oql(query)?;
    let limits = mat_cfg::query_limits();

    // 列白名单：内建列 + 普通标识符；函数调用已在解析阶段拒绝
    let mut columns: Vec<String> = q.columns.clone();
    if q.wildcard {
        columns = vec!["@id".into(), "@class".into(), "@shallow".into(), "@kind".into()];
    }
    for c in &columns {
        if c.contains('(') || c.contains(')') {
            return Err(format!("列 `{c}` 非法：不支持函数调用"));
        }
    }

    let head = read_head(path, 64)?;
    let (_fmt, supported) = sniff_format(&head);
    if !supported {
        return Err("不支持的堆转储格式（当前仅支持 JAVA PROFILE 1.0.x）".to_string());
    }
    let mm = crate::mat_source::map_file(path)?;
    let hp = catch_parse(|| {
        jvm_hprof::parse_hprof(&mm[..]).map_err(|e| format!("解析失败: {e:?}"))
    })?;
    let id_size = hp.header().id_size();
    let id_width = match id_size {
        jvm_hprof::IdSize::U32 => 4,
        jvm_hprof::IdSize::U64 => 8,
    };
    // `@shallow` 与直方图 / 支配树同口径（mat_layout 为唯一真源）
    let layout = crate::mat_layout::HeapLayout::from_id_width(id_width as u64);

    // ---- Pass 1：字符串表 + LoadClass ----
    let mut utf8: HashMap<u64, String> = HashMap::new();
    let mut load_names: HashMap<u64, String> = HashMap::new();
    for rec in hp.records_iter() {
        let rec = rec.map_err(|e| format!("记录解析失败: {e:?}"))?;
        match rec.tag() {
            jvm_hprof::RecordTag::Utf8 => {
                if let Some(u) = rec.as_utf_8() {
                    let u = u.map_err(|e| format!("Utf8 解析失败: {e:?}"))?;
                    utf8.insert(u.name_id().id(), String::from_utf8_lossy(u.text()).into_owned());
                }
            }
            jvm_hprof::RecordTag::LoadClass => {
                if let Some(lc) = rec.as_load_class() {
                    let lc = lc.map_err(|e| format!("LoadClass 解析失败: {e:?}"))?;
                    let name = utf8.get(&lc.class_name_id().id()).cloned().unwrap_or_default();
                    load_names.insert(lc.class_obj_id().id(), name);
                }
            }
            _ => {}
        }
    }

    // ---- Pass 2：类元数据 ----
    let mut classes: HashMap<u64, ClsInfo> = HashMap::new();
    for rec in hp.records_iter() {
        let rec = rec.map_err(|e| format!("记录解析失败: {e:?}"))?;
        if !matches!(
            rec.tag(),
            jvm_hprof::RecordTag::HeapDump | jvm_hprof::RecordTag::HeapDumpSegment
        ) {
            continue;
        }
        let Some(seg) = rec.as_heap_dump_segment() else { continue };
        let seg = seg.map_err(|e| format!("HeapDump 解析失败: {e:?}"))?;
        for sr in seg.sub_records() {
            let Ok(SubRecord::Class(c)) = sr else { continue };
            let cid = c.obj_id().id();
            let mut fields = Vec::new();
            for fd in c.instance_field_descriptors() {
                let Ok(fd) = fd else { continue };
                let nm = utf8
                    .get(&fd.name_id().id())
                    .cloned()
                    .unwrap_or_else(|| format!("#{:x}", fd.name_id().id()));
                fields.push((field_type_tag(fd.field_type()), nm));
            }
            classes.insert(
                cid,
                ClsInfo {
                    name: load_names.get(&cid).cloned().unwrap_or_default(),
                    super_id: c.super_class_obj_id().map(|i| i.id()),
                    instance_size: c.instance_size_bytes() as u64,
                    fields,
                },
            );
        }
    }

    // ---- Pass 3：扫描实例、求值 ----
    let mut want_fields: Vec<String> = Vec::new();
    if let Some(e) = &q.filter {
        expr_fields(e, &mut want_fields);
    }
    let mut chain_memo: HashMap<u64, Option<Vec<(u8, String)>>> = HashMap::new();
    let mut match_memo: HashMap<u64, bool> = HashMap::new();
    let mut rows: Vec<OqlRow> = Vec::new();
    let mut scanned: u64 = 0;
    let mut matched: u64 = 0;
    let mut truncated = false;
    let out_columns: Vec<String> = columns.clone();

    'outer: for rec in hp.records_iter() {
        let rec = rec.map_err(|e| format!("记录解析失败: {e:?}"))?;
        if !matches!(
            rec.tag(),
            jvm_hprof::RecordTag::HeapDump | jvm_hprof::RecordTag::HeapDumpSegment
        ) {
            continue;
        }
        let Some(seg) = rec.as_heap_dump_segment() else { continue };
        let seg = seg.map_err(|e| format!("HeapDump 解析失败: {e:?}"))?;
        for sr in seg.sub_records() {
            let Ok(SubRecord::Instance(d)) = sr else { continue };
            let cid = d.class_obj_id().id();
            if !chain_matches(cid, &classes, &q.class_pattern, &mut match_memo) {
                continue;
            }
            scanned += 1;
            if scanned as usize > limits.oql_scan {
                truncated = true;
                break 'outer;
            }
            let chain = match chain_memo.get(&cid) {
                Some(Some(v)) => v.clone(),
                Some(None) => continue,
                None => {
                    let v = chain_of(cid, &classes);
                    chain_memo.insert(cid, v.clone());
                    match v {
                        Some(v) => v,
                        None => continue,
                    }
                }
            };
            let vals = RowValues {
                obj_id: d.obj_id().id(),
                class_name: classes.get(&cid).map(|c| c.name.clone()).unwrap_or_default(),
                kind: "instance".to_string(),
                shallow: layout.instance_bytes(
                    classes.get(&cid).map(|c| c.instance_size).unwrap_or(0),
                ),
                fields: decode_fields(d.fields(), &chain, id_width, &want_fields),
            };
            if let Some(e) = &q.filter {
                if !eval(e, &vals)? {
                    continue;
                }
            }
            matched += 1;
            if rows.len() < q.limit {
                let mut cells: Vec<OqlCell> = Vec::with_capacity(out_columns.len());
                for c in &out_columns {
                    let v = vals.lookup(c);
                    cells.push(OqlCell {
                        name: c.clone(),
                        value: match v {
                            Some(v) => {
                                let text = v.as_text();
                                if text.chars().count() > MAX_STRING_PREVIEW {
                                    text.chars().take(MAX_STRING_PREVIEW).collect::<String>() + "…"
                                } else {
                                    text
                                }
                            }
                            None => "(无此字段)".to_string(),
                        },
                    });
                }
                rows.push(OqlRow {
                    obj_id: format!("0x{:x}", vals.obj_id),
                    class_name: vals.class_name.clone(),
                    kind: "instance".to_string(),
                    shallow_bytes: vals.shallow,
                    cells,
                });
            }
        }
    }

    let mut note = String::new();
    if truncated {
        note.push_str(&format!(
            "扫描实例超过上限 {}，结果可能不完整；收窄 FROM 的类名或用 WHERE 过滤",
            limits.oql_scan
        ));
    }
    if matched > rows.len() as u64 {
        if !note.is_empty() {
            note.push_str("；");
        }
        note.push_str(&format!(
            "命中 {} 个对象，仅返回前 {} 行（可用 LIMIT 调整）",
            matched,
            rows.len()
        ));
    }

    Ok(ResData {
        code: 200,
        msg: "ok".to_string(),
        data: OqlResult {
            query: query.trim().to_string(),
            columns: out_columns,
            rows,
            scanned,
            matched,
            limit: q.limit as u64,
            truncated,
            note,
            elapsed_ms: started.elapsed().as_millis() as u64,
        },
    })
}

/// 查询语句的静态校验（供前端「执行」前的即时提示，不读文件）
pub fn mat_oql_check(query: &str) -> Result<ResData<OqlCheck>, String> {
    let q = parse_oql(query)?;
    Ok(ResData {
        code: 200,
        msg: "ok".to_string(),
        data: OqlCheck {
            columns: if q.wildcard {
                vec!["@id".into(), "@class".into(), "@shallow".into(), "@kind".into()]
            } else {
                q.columns
            },
            class_pattern: q.class_pattern,
            has_filter: q.filter.is_some(),
            limit: q.limit as u64,
        },
    })
}

#[derive(Debug, Serialize)]
pub struct OqlCheck {
    pub columns: Vec<String>,
    pub class_pattern: String,
    pub has_filter: bool,
    pub limit: u64,
}

// ==================== 测试 ====================

#[cfg(test)]
mod tests {
    use super::*;

    fn u1(v: &mut Vec<u8>, x: u8) {
        v.push(x);
    }
    fn u2(v: &mut Vec<u8>, x: u16) {
        v.extend_from_slice(&x.to_be_bytes());
    }
    fn u4(v: &mut Vec<u8>, x: u32) {
        v.extend_from_slice(&x.to_be_bytes());
    }
    fn u8_(v: &mut Vec<u8>, x: u64) {
        v.extend_from_slice(&x.to_be_bytes());
    }
    fn rec(v: &mut Vec<u8>, tag: u8, payload: &[u8]) {
        u1(v, tag);
        u4(v, 0);
        u4(v, payload.len() as u32);
        v.extend_from_slice(payload);
    }

    /// 最小 dump：mem/Box{int size, long tag} 三个实例 + mem/Other{int x}
    fn write_dump() -> String {
        let mut f: Vec<u8> = Vec::new();
        f.extend_from_slice(b"JAVA PROFILE 1.0.2\0");
        u4(&mut f, 8);
        u8_(&mut f, 0);
        let mut name_id = 0x1000u64;
        let mut utf8 = |f: &mut Vec<u8>, s: &str| -> u64 {
            let id = name_id;
            name_id += 1;
            let mut p = Vec::new();
            u8_(&mut p, id);
            p.extend_from_slice(s.as_bytes());
            rec(f, 0x01, &p);
            id
        };
        let n_box = utf8(&mut f, "mem/Box");
        let n_other = utf8(&mut f, "mem/Other");
        let n_size = utf8(&mut f, "size");
        let n_tag = utf8(&mut f, "tag");
        // LoadClass
        let mut serial = 1u32;
        let mut load = |f: &mut Vec<u8>, cid: u64, nid: u64| {
            let mut p = Vec::new();
            u4(&mut p, serial);
            serial += 1;
            u8_(&mut p, cid);
            u4(&mut p, 0);
            u8_(&mut p, nid);
            rec(f, 0x02, &p);
        };
        load(&mut f, 0x200, n_box);
        load(&mut f, 0x300, n_other);

        let mut h: Vec<u8> = Vec::new();
        // Class mem/Box：instance_size=16，字段 size(int) + tag(long)
        u1(&mut h, 0x20);
        u8_(&mut h, 0x200);
        u4(&mut h, 0);
        u8_(&mut h, 0);
        for _ in 0..5 {
            u8_(&mut h, 0);
        }
        u4(&mut h, 16);
        u2(&mut h, 0);
        u2(&mut h, 0);
        u2(&mut h, 2);
        u8_(&mut h, n_size);
        u1(&mut h, 0x0A); // int
        u8_(&mut h, n_tag);
        u1(&mut h, 0x0B); // long
        // Class mem/Other
        u1(&mut h, 0x20);
        u8_(&mut h, 0x300);
        u4(&mut h, 0);
        u8_(&mut h, 0);
        for _ in 0..5 {
            u8_(&mut h, 0);
        }
        u4(&mut h, 8);
        u2(&mut h, 0);
        u2(&mut h, 0);
        u2(&mut h, 1);
        u8_(&mut h, n_size);
        u1(&mut h, 0x0A);

        let inst = |h: &mut Vec<u8>, oid: u64, cid: u64, size: i32, tag: i64| {
            let mut p = Vec::new();
            u1(&mut p, 0x21);
            u8_(&mut p, oid);
            u4(&mut p, 0);
            u8_(&mut p, cid);
            let mut data = Vec::new();
            u4(&mut data, size as u32);
            u8_(&mut data, tag as u64);
            u4(&mut p, data.len() as u32);
            p.extend_from_slice(&data);
            h.extend_from_slice(&p);
        };
        inst(&mut h, 0x201, 0x200, 10, 100);
        inst(&mut h, 0x202, 0x200, 500, 7);
        inst(&mut h, 0x203, 0x200, 3000, 42);
        // mem/Other 一个实例（字段布局不同，验证 FROM 过滤）
        let mut p = Vec::new();
        u1(&mut p, 0x21);
        u8_(&mut p, 0x301);
        u4(&mut p, 0);
        u8_(&mut p, 0x300);
        let mut d = Vec::new();
        u4(&mut d, 999);
        u4(&mut p, d.len() as u32);
        p.extend_from_slice(&d);
        h.extend_from_slice(&p);

        rec(&mut f, 0x0C, &h);
        let dir = std::env::temp_dir().join("mat_oql_case.hprof");
        std::fs::write(&dir, f).unwrap();
        dir.to_str().unwrap().to_string()
    }

    #[test]
    fn parse_accepts_subset() {
        let q = parse_oql("select * from INSTANCEOF mem/Box where size > 100 and not (tag = 1) limit 3").unwrap();
        assert!(q.wildcard);
        assert_eq!(q.class_pattern, "mem/Box");
        assert_eq!(q.limit, 3);
        assert!(q.filter.is_some());

        let q = parse_oql("SELECT @id, size FROM INSTANCEOF *Box WHERE size != 0").unwrap();
        assert_eq!(q.columns, vec!["@id".to_string(), "size".to_string()]);
        assert!(q.filter.is_some());
    }

    #[test]
    fn parse_rejects_injection_and_functions() {
        // 多语句
        assert!(parse_oql("select * from INSTANCEOF mem/Box; drop table x").is_err());
        // 注释
        assert!(parse_oql("select * from INSTANCEOF mem/Box -- x").is_err());
        assert!(parse_oql("select * from INSTANCEOF mem/Box /* x */").is_err());
        // 函数调用（MAT 的 retainedSet()/toString()）
        assert!(parse_oql("select toString() from INSTANCEOF mem/Box").is_err());
        assert!(parse_oql("select * from INSTANCEOF mem/Box where retainedSet() > 1").is_err());
        // 未知关键字
        assert!(parse_oql("select * from CLASSES mem/Box").is_err());
        assert!(parse_oql("delete from INSTANCEOF mem/Box").is_err());
        // 字符串字面量未闭合
        assert!(parse_oql("select * from INSTANCEOF mem/Box where size = 'x").is_err());
    }

    #[test]
    fn query_end_to_end() {
        let p = write_dump();
        // 通配 + 数值过滤
        let r = mat_oql_query(&p, "select @id, size, tag from INSTANCEOF mem/Box where size > 100")
            .unwrap()
            .data;
        assert_eq!(r.matched, 2, "size>100 应有 2 个");
        assert_eq!(r.rows.len(), 2);
        assert_eq!(r.columns.len(), 3);
        let first = &r.rows[0];
        assert_eq!(first.class_name, "mem/Box");
        assert!(first.obj_id.starts_with("0x"));

        // 逻辑组合 + not
        let r = mat_oql_query(
            &p,
            "select * from INSTANCEOF mem/Box where size > 100 and not (tag = 100)",
        )
        .unwrap()
        .data;
        assert_eq!(r.matched, 2);

        // LIMIT 生效
        let r = mat_oql_query(&p, "select * from INSTANCEOF mem/Box limit 1").unwrap().data;
        assert_eq!(r.matched, 3);
        assert_eq!(r.rows.len(), 1);
        assert!(!r.note.is_empty());

        // 类名后缀匹配 + 不命中另一类
        let r = mat_oql_query(&p, "select * from INSTANCEOF mem/Other").unwrap().data;
        assert_eq!(r.matched, 1);
        assert_eq!(r.rows[0].cells.len(), 4);

        // 通配不命中
        let r = mat_oql_query(&p, "select * from INSTANCEOF *Nope").unwrap().data;
        assert_eq!(r.matched, 0);

        // 静态校验
        let c = mat_oql_check("select * from INSTANCEOF mem/Box where size >= 1").unwrap().data;
        assert!(c.has_filter);
        assert_eq!(c.class_pattern, "mem/Box");

        let _ = std::fs::remove_file(&p);
    }
}
