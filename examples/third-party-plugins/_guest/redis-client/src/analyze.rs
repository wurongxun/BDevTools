// P2 运维分析：在 **guest 侧**对真实 Redis 做聚合，返回与前端 MockEngine 完全一致的 Reply JSON 形状。
//
// 真实 Redis 不认识 `RCANALYZE` 这类自定义命令，所以这些分析由本插件用标准命令聚合：
// SCAN + MEMORY USAGE（内存）、SLOWLOG GET（慢查询）、CLUSTER INFO/NODES（集群）、
// INFO replication（复制）、SENTINEL MASTERS（哨兵）。聚合结果按前端 types.ts 的
// MemoryAnalysis / SlowQueryAnalysis / ClusterTopology / ReplicationInfo / SentinelMaster
// 形状返回，前端 `RedisClient.memoryAnalyze()` 等直接消费。

use std::collections::HashMap;

use serde_json::{json, Value};

use crate::host::Conn;
use crate::resp::{send_cmd, RespReply};

/// 拼一个 `bulk(object)` 形状的 Reply（v 为对象，非字符串）。
fn reply_bulk_obj(v: Value) -> Value {
    json!({ "t": "bulk", "v": v })
}

/// 拼一个 `array([...])` 形状的 Reply。
fn reply_array_val(v: Value) -> Value {
    json!({ "t": "array", "v": v })
}

/// 在连接上执行一条命令（`&str` 参数便捷版）。
fn cmd(conn: &mut Conn, args: &[&str]) -> Result<RespReply, String> {
    let a: Vec<String> = args.iter().map(|s| s.to_string()).collect();
    send_cmd(conn, &a)
}

/// 单个 RESP 元素折叠为字符串（数组内层 / 顶层字符串元素通用）。
fn elem_to_string(r: &RespReply) -> String {
    match r {
        RespReply::Bulk(Some(s)) => s.clone(),
        RespReply::Status(s) => s.clone(),
        RespReply::Int(n) => n.to_string(),
        RespReply::Bulk(None) => String::new(),
        RespReply::Error(s) => s.clone(),
        RespReply::Array(_) => String::new(),
    }
}

/// 把 `Array(Some(items))` 折叠为字符串数组。
fn as_str_arr(r: &RespReply) -> Option<Vec<String>> {
    if let RespReply::Array(Some(items)) = r {
        Some(items.iter().map(elem_to_string).collect())
    } else {
        None
    }
}

// ============================================================ 内存分析
pub fn analyze_memory(conn: &mut Conn, sample_limit: i64) -> Result<Value, String> {
    let limit = if sample_limit > 0 { Some(sample_limit as usize) } else { None };

    // SCAN 全量收集 key（MATCH *）
    let mut cursor = "0".to_string();
    let mut names: Vec<String> = Vec::new();
    loop {
        let reply = cmd(conn, &["SCAN", &cursor, "COUNT", "200"])?;
        if let RespReply::Array(Some(items)) = &reply {
            if items.len() >= 2 {
                if let RespReply::Bulk(Some(c)) = &items[0] {
                    cursor = c.clone();
                }
                if let Some(keys) = as_str_arr(&items[1]) {
                    names.extend(keys);
                }
            }
        }
        if cursor == "0" {
            break;
        }
        if let Some(lim) = limit {
            if names.len() >= lim {
                break;
            }
        }
    }

    // 逐 key 取 TYPE / TTL / MEMORY USAGE
    let mut items: Vec<Value> = Vec::new();
    for name in &names {
        let t = match cmd(conn, &["TYPE", name])? {
            RespReply::Status(s) => s,
            RespReply::Bulk(Some(s)) => s,
            _ => "none".to_string(),
        };
        if t == "none" {
            continue;
        }
        let ttl = match cmd(conn, &["TTL", name])? {
            RespReply::Int(n) => n,
            _ => -2,
        };
        let size = match cmd(conn, &["MEMORY", "USAGE", name])? {
            RespReply::Int(n) => n,
            RespReply::Bulk(Some(s)) => s.parse::<i64>().unwrap_or(0),
            _ => 0,
        };
        items.push(json!({ "name": name, "type": t, "size": size, "ttl": ttl }));
    }

    let total_bytes: i64 = items.iter().map(|x| x["size"].as_i64().unwrap_or(0)).sum();
    let total_keys = items.len() as i64;

    let mut sorted = items.clone();
    sorted.sort_by(|a, b| b["size"].as_i64().unwrap_or(0).cmp(&a["size"].as_i64().unwrap_or(0)));
    let big_keys: Vec<Value> = sorted.iter().take(20).cloned().collect();

    let mut pmap: HashMap<String, (i64, i64)> = HashMap::new(); // 前缀 -> (count, size)
    let mut tmap: HashMap<String, (i64, i64)> = HashMap::new(); // 类型 -> (count, size)
    for it in &items {
        let name = it["name"].as_str().unwrap_or("");
        let size = it["size"].as_i64().unwrap_or(0);
        let p = if let Some(i) = name.find(':') { &name[..i] } else { "(root)" };
        let e = pmap.entry(p.to_string()).or_insert((0, 0));
        e.0 += 1;
        e.1 += size;
        let ty = it["type"].as_str().unwrap_or("none").to_string();
        let te = tmap.entry(ty).or_insert((0, 0));
        te.0 += 1;
        te.1 += size;
    }

    let mut prefixes: Vec<Value> = pmap
        .iter()
        .map(|(p, (c, s))| {
            json!({
                "prefix": p,
                "count": c,
                "size": s,
                "pct": if total_bytes > 0 { (*s * 100) / total_bytes } else { 0 }
            })
        })
        .collect();
    prefixes.sort_by(|a, b| b["size"].as_i64().unwrap_or(0).cmp(&a["size"].as_i64().unwrap_or(0)));

    let mut types: Vec<Value> = tmap
        .iter()
        .map(|(t, (c, s))| {
            json!({
                "type": t,
                "count": c,
                "size": s,
                "pct": if total_bytes > 0 { (*s * 100) / total_bytes } else { 0 }
            })
        })
        .collect();
    types.sort_by(|a, b| b["size"].as_i64().unwrap_or(0).cmp(&a["size"].as_i64().unwrap_or(0)));

    Ok(reply_bulk_obj(json!({
        "totalBytes": total_bytes,
        "totalKeys": total_keys,
        "bigKeys": big_keys,
        "prefixes": prefixes,
        "types": types
    })))
}

// ============================================================ 慢查询分析
fn normalize_slow(command: &str) -> String {
    let parts: Vec<&str> = command.split_whitespace().collect();
    if parts.is_empty() {
        return String::new();
    }
    let tail: Vec<String> = parts[1..]
        .iter()
        .map(|tok| {
            if *tok == "*" || *tok == "$" || tok.parse::<i64>().is_ok() {
                (*tok).to_string()
            } else if let Some(i) = tok.find(':') {
                format!("{}:*", &tok[..i])
            } else {
                (*tok).to_string()
            }
        })
        .collect();
    format!("{} {}", parts[0], tail.join(" "))
}

fn empty_slow() -> Value {
    json!({
        "items": [],
        "count": 0,
        "maxUs": 0,
        "minUs": 0,
        "avgUs": 0,
        "p95Us": 0,
        "byCommand": []
    })
}

pub fn slowlog(conn: &mut Conn, count: i64) -> Result<Value, String> {
    let count = count.max(1);
    let raw = match cmd(conn, &["SLOWLOG", "GET", &count.to_string()])? {
        RespReply::Array(Some(items)) => items,
        // 未开启慢日志 / 命令报错时返回空分析
        _ => return Ok(reply_bulk_obj(empty_slow())),
    };

    let mut items: Vec<Value> = Vec::new();
    for e in &raw {
        if let RespReply::Array(Some(parts)) = e {
            if parts.len() >= 4 {
                let id = match &parts[0] {
                    RespReply::Int(n) => *n,
                    _ => 0,
                };
                let ts = match &parts[1] {
                    RespReply::Int(n) => *n,
                    _ => 0,
                };
                let dur = match &parts[2] {
                    RespReply::Int(n) => *n,
                    _ => 0,
                };
                let command = as_str_arr(&parts[3]).unwrap_or_default().join(" ");
                let pattern = normalize_slow(&command);
                items.push(json!({
                    "id": id,
                    "timestamp": ts,
                    "durationUs": dur,
                    "command": command,
                    "pattern": pattern
                }));
            }
        }
    }

    let n = items.len();
    let (count_, max_, min_, avg_, p95_) = if n > 0 {
        let mut sd: Vec<i64> = items.iter().map(|x| x["durationUs"].as_i64().unwrap_or(0)).collect();
        sd.sort_unstable();
        let max = sd[n - 1];
        let min = sd[0];
        let avg = sd.iter().sum::<i64>() / n as i64;
        let p95i = (0.95 * (n as f64)).ceil() as usize;
        let p95 = sd[p95i.min(n - 1)];
        (n, max, min, avg, p95)
    } else {
        (0, 0, 0, 0, 0)
    };

    let mut gmap: HashMap<String, (i64, i64, i64)> = HashMap::new(); // pattern -> (count, total, max)
    for it in &items {
        let p = it["pattern"].as_str().unwrap_or("").to_string();
        let d = it["durationUs"].as_i64().unwrap_or(0);
        let e = gmap.entry(p).or_insert((0, 0, 0));
        e.0 += 1;
        e.1 += d;
        e.2 = e.2.max(d);
    }
    let mut by_command: Vec<Value> = gmap
        .iter()
        .map(|(p, (c, t, m))| json!({ "pattern": p, "count": c, "totalUs": t, "maxUs": m }))
        .collect();
    by_command.sort_by(|a, b| b["totalUs"].as_i64().unwrap_or(0).cmp(&a["totalUs"].as_i64().unwrap_or(0)));

    Ok(reply_bulk_obj(json!({
        "items": items,
        "count": count_,
        "maxUs": max_,
        "minUs": min_,
        "avgUs": avg_,
        "p95Us": p95_,
        "byCommand": by_command
    })))
}

// ============================================================ 集群拓扑
fn parse_cluster_node(line: &str) -> Option<Value> {
    let p: Vec<&str> = line.split_whitespace().collect();
    if p.len() < 8 {
        return None;
    }
    let id = p[0];
    let addr = p[1].split('@').next().unwrap_or(p[1]).to_string();
    let flags = p[2];
    let master = if p[3] == "-" { None } else { Some(p[3].to_string()) };
    let link = p[7];
    let myself = flags.contains("myself");
    let role = if flags.contains("master") {
        "master"
    } else if flags.contains("slave") {
        "slave"
    } else {
        "master"
    };
    let role_out = if myself { "myself" } else { role };
    let slots: Vec<&str> = p[8..].to_vec();
    let slot_desc = if slots.is_empty() || slots[0] == "-" {
        "-".to_string()
    } else {
        slots.join(" ")
    };
    Some(json!({
        "id": id,
        "role": role_out,
        "addr": addr,
        "flags": flags,
        "masterId": master,
        "slots": slot_desc,
        "linkState": link
    }))
}

pub fn cluster(conn: &mut Conn) -> Result<Value, String> {
    // 用 CLUSTER INFO 判断是否集群模式（未开启时命令报错）
    let is_cluster = match cmd(conn, &["CLUSTER", "INFO"]) {
        Ok(RespReply::Bulk(Some(ci))) => {
            ci.contains("cluster_enabled:1") || ci.to_uppercase().contains("CLUSTER_STATE:OK")
        }
        _ => false,
    };

    let mut nodes: Vec<Value> = Vec::new();
    if is_cluster {
        if let Ok(RespReply::Bulk(Some(text))) = cmd(conn, &["CLUSTER", "NODES"]) {
            for line in text.lines() {
                if let Some(node) = parse_cluster_node(line) {
                    nodes.push(node);
                }
            }
        }
    } else {
        nodes.push(json!({
            "id": "self",
            "role": "myself",
            "addr": "",
            "flags": "master",
            "masterId": null,
            "slots": "-",
            "linkState": "connected"
        }));
    }

    let mode = if is_cluster { "cluster" } else { "standalone" };
    let myself_id = nodes
        .iter()
        .find(|n| n["role"] == "myself")
        .and_then(|n| n["id"].as_str())
        .unwrap_or("")
        .to_string();
    let slots_covered = if is_cluster { 16384 } else { 0 };

    Ok(reply_bulk_obj(json!({
        "mode": mode,
        "nodeCount": nodes.len() as i64,
        "slotsCovered": slots_covered,
        "myselfId": myself_id,
        "nodes": nodes
    })))
}

// ============================================================ 主从复制
pub fn replication(conn: &mut Conn) -> Result<Value, String> {
    let info = match cmd(conn, &["INFO", "replication"]) {
        Ok(RespReply::Bulk(Some(s))) => s,
        _ => {
            return Ok(reply_bulk_obj(json!({
                "role": "master",
                "connectedSlaves": 0,
                "masterReplOffset": 0,
                "slaves": [],
                "masterAddr": null
            })));
        }
    };

    let mut role = "master".to_string();
    let mut connected_slaves = 0i64;
    let mut offset = 0i64;
    let mut slaves: Vec<Value> = Vec::new();
    for line in info.lines() {
        if let Some(eq) = line.find(':') {
            let k = &line[..eq];
            let v = &line[eq + 1..];
            match k {
                "role" => role = v.to_string(),
                "connected_slaves" => connected_slaves = v.parse().unwrap_or(0),
                "master_repl_offset" => offset = v.parse().unwrap_or(0),
                _ => {}
            }
            if k.starts_with("slave") {
                let mut ip = "";
                let mut port = 0i64;
                let mut lag = 0i64;
                let mut state = "";
                let mut soff = 0i64;
                for kv in v.split(',') {
                    let pp: Vec<&str> = kv.splitn(2, '=').collect();
                    if pp.len() == 2 {
                        match pp[0] {
                            "ip" => ip = pp[1],
                            "port" => port = pp[1].parse().unwrap_or(0),
                            "lag" => lag = pp[1].parse().unwrap_or(0),
                            "state" => state = pp[1],
                            "offset" => soff = pp[1].parse().unwrap_or(0),
                            _ => {}
                        }
                    }
                }
                slaves.push(json!({
                    "id": k,
                    "addr": format!("{ip}:{port}"),
                    "offset": soff,
                    "lag": lag,
                    "state": state
                }));
            }
        }
    }

    let master_addr = if role == "slave" {
        let mut mh = String::new();
        let mut mp = 0i64;
        for line in info.lines() {
            if let Some(eq) = line.find(':') {
                let k = &line[..eq];
                let v = &line[eq + 1..];
                if k == "master_host" {
                    mh = v.to_string();
                }
                if k == "master_port" {
                    mp = v.parse().unwrap_or(0);
                }
            }
        }
        Some(format!("{mh}:{mp}"))
    } else {
        None
    };

    Ok(reply_bulk_obj(json!({
        "role": role,
        "connectedSlaves": connected_slaves,
        "masterReplOffset": offset,
        "slaves": slaves,
        "masterAddr": master_addr
    })))
}

// ============================================================ Sentinel
pub fn sentinel(conn: &mut Conn, sub: &str, name: &str) -> Result<Value, String> {
    let sub = if sub.is_empty() { "MASTERS".to_string() } else { sub.to_uppercase() };

    if sub == "GET-MASTER-ADDR-BY-NAME" {
        let reply = cmd(conn, &["SENTINEL", "GET-MASTER-ADDR-BY-NAME", name])?;
        let arr = as_str_arr(&reply).unwrap_or_default();
        let addr = if arr.len() >= 2 {
            format!("{}:{}", arr[0], arr[1])
        } else {
            String::new()
        };
        return Ok(reply_bulk_obj(json!(addr)));
    }

    // SENTINEL MASTERS：返回 [ [field, value, field, value, ...], ... ]
    let reply = cmd(conn, &["SENTINEL", "MASTERS"])?;
    let mut masters: Vec<Value> = Vec::new();
    if let RespReply::Array(Some(items)) = &reply {
        for m in items {
            let mut map: HashMap<String, String> = HashMap::new();
            if let RespReply::Array(Some(pairs)) = m {
                let mut i = 0;
                while i + 1 < pairs.len() {
                    map.insert(elem_to_string(&pairs[i]), elem_to_string(&pairs[i + 1]));
                    i += 2;
                }
            }
            masters.push(json!({
                "name": map.get("name").cloned().unwrap_or_default(),
                "ip": map.get("ip").cloned().unwrap_or_default(),
                "port": map.get("port").and_then(|s| s.parse::<i64>().ok()).unwrap_or(0),
                "flags": map.get("flags").cloned().unwrap_or_default(),
                "numSlaves": map.get("num-slaves").and_then(|s| s.parse::<i64>().ok()).unwrap_or(0),
                "numOtherSentinels": map.get("num-other-sentinels").and_then(|s| s.parse::<i64>().ok()).unwrap_or(0),
                "quorum": map.get("quorum").and_then(|s| s.parse::<i64>().ok()).unwrap_or(0)
            }));
        }
    }
    Ok(reply_array_val(json!(masters)))
}
