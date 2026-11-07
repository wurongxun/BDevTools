/*! # SSH/SFTP 连接配置持久化（sqlite 表）
存储 ssh/sftp 连接配置与收藏分组（favorites），供连接列表/右键子菜单使用。*/
use crate::response::response::Response;
use crate::sqlite::sqlite::exec;
use crate::sqlite::vault;
use anyhow::anyhow;
use log::info;
use rusqlite::params;
use serde::{Deserialize, Serialize};

fn default_keepalive() -> i32 {
    30
}

/// 数据对应字段
///
/// 新增列一律带 `#[serde(default)]`：老前端/其它调用方（如 logs 插件）仍可只传基础字段，
/// 缺字段不会让整条 `connection_create` / `connection_edit` 反序列化失败。
#[derive(Serialize, Deserialize, Debug)]
pub struct SshSubMenu {
    pub id: i32,
    pub name: String,
    pub address: String,
    pub port: i32,
    pub username: String,
    pub password: String,
    pub agreement: String,
    pub favorites_name: String,
    pub affix: bool,
    pub auth_type: String,
    pub private_key_path: String,
    /// 加密私钥的口令（仅 pubkey 认证使用），与登录密码分开存储
    #[serde(default)]
    pub passphrase: String,
    /// 终端字符编码：utf-8 / gbk / gb18030 / big5 / shift_jis
    #[serde(default)]
    pub encoding: String,
    /// SSH keepalive 间隔（秒），0 表示不启用
    #[serde(default = "default_keepalive")]
    pub keepalive_interval: i32,
    /// PTY 请求的终端类型
    #[serde(default)]
    pub term_type: String,
    /// 会话备注
    #[serde(default)]
    pub description: String,
    /// P2 高级连接选项（JSON 文本）：代理 / 跳板机 / 压缩 / 算法 / 2FA / agent / DISPLAY
    #[serde(default)]
    pub advanced_options: String,
    /// P3 会话颜色标签（#RRGGBB，空串表示不标色）
    #[serde(default)]
    pub color: String,
}

/// 创建一个连接
pub async fn connection_create(map: SshSubMenu) -> Response<Option<bool>> {
    // 保险库锁定期间禁止写入：否则编辑一条会话会把「解密失败得到的空密码」写回库里。
    if let Err(msg) = vault::ensure_writable() {
        return Response::new(423, None, &msg);
    }
    let password = vault::encrypt_secret(&map.password);
    let passphrase = vault::encrypt_secret(&map.passphrase);
    let advanced_options = vault::encrypt_advanced(&map.advanced_options);
    let res = exec(|conn| {
        let count: u32 = conn
        .query_row("SELECT COUNT(*) FROM connections where favorites_name=?1 and name=?2",  params![map.favorites_name,map.name], |row| row.get(0))
        .expect("Unable to count rows");
        if count > 0 {
            return Err(anyhow!("连接已存在！"));
        }
        let conns = conn.execute(
      "insert into connections(name,address,port,username,password,agreement,favorites_name,affix,auth_type,private_key_path,passphrase,encoding,keepalive_interval,term_type,description,advanced_options,color) values(?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?14,?15,?16,?17)",
      params![
        map.name,
        map.address,
        map.port,
        map.username,
        password,
        map.agreement,
        map.favorites_name,
        map.affix,
        map.auth_type,
        map.private_key_path,
        passphrase,
        map.encoding,
        map.keepalive_interval,
                map.term_type,
                map.description,
                advanced_options,
                map.color,
            ],
        )?;
        Ok(conns > 0)
    });
    match res {
        Ok(res) => {
            if res {
                Response::new(200, Some(true), "添加成功！")
            } else {
                Response::new(300, None, "添加失败！")
            }
        }
        Err(e) => {
            Response::new(300, None, &format!("{}", e))
        }
    }
    // if let Ok(res) = res {
    //     if res {
    //         Response::new(200, Some(true), "操作成功！")
    //     } else {
    //         Response::new(300, None, "操作失败！")
    //     }
    // } else {
    //     Response::new(300, None, "操作失败！")
    // }
}

/// 删除一个连接
pub async fn connection_delete(id: String) -> Response<bool> {
    let res = exec(|conn| {
        let mut stmt = conn.prepare("delete from connections where id =?1")?;
        let res = stmt.execute(params![id])?;
        Ok(res > 0)
    });
    info!("res -> {:?}", res);
    if let Ok(res) = res {
        if res {
            Response::new(200, true, "删除成功！")
        } else {
            Response::new(300, false, "删除失败！")
        }
    } else {
        Response::new(300, false, "删除失败！")
    }
}

/// 编辑一个连接
pub async fn connection_edit(map: SshSubMenu) -> Response<bool> {
    if let Err(msg) = vault::ensure_writable() {
        return Response::new(423, false, &msg);
    }
    let password = vault::encrypt_secret(&map.password);
    let passphrase = vault::encrypt_secret(&map.passphrase);
    let advanced_options = vault::encrypt_advanced(&map.advanced_options);
    let res = exec(|conn| {
        let affected = conn.execute(
            "UPDATE connections SET name=?1, address=?2, port=?3, username=?4, password=?5, agreement=?6, favorites_name=?7, affix=?8, auth_type=?9, private_key_path=?10, passphrase=?11, encoding=?12, keepalive_interval=?13, term_type=?14, description=?15, advanced_options=?16, color=?17 WHERE id=?18",
            params![
                map.name,
                map.address,
                map.port,
                map.username,
                password,
                map.agreement,
                map.favorites_name,
                map.affix,
                map.auth_type,
                map.private_key_path,
                passphrase,
                map.encoding,
                map.keepalive_interval,
                map.term_type,
                map.description,
                advanced_options,
                map.color,
                map.id,
            ]
        )?;
        Ok(affected > 0)
    });
    match res {
        Ok(_) => Response::new(200, true, "修改成功！"),
        Err(e) => Response::new(300, false, &format!("修改失败：{}", e)),
    }
}

/// 查询连接列表
pub async fn connection_list() -> Response<Vec<SshSubMenu>> {
    let res = exec(|conn| {
        let mut stmt = conn.prepare("select * from connections")?;
        let mut res = stmt.query(params![])?;
        let mut vec = vec![];
        while let Some(row) = res.next()? {
            vec.push(SshSubMenu {
                id: row.get("id")?,
                name: row.get("name")?,
                address: row.get("address")?,
                port: row.get("port")?,
                username: row.get("username")?,
                password: vault::decrypt_secret(&row.get::<_, String>("password")?),
                agreement: row.get("agreement")?,
                favorites_name: row.get("favorites_name")?,
                affix: row.get("affix")?,
                auth_type: row.get("auth_type").unwrap_or_else(|_| "password".to_string()),
                private_key_path: row.get("private_key_path").unwrap_or_default(),
                passphrase: vault::decrypt_secret(&row.get::<_, String>("passphrase").unwrap_or_default()),
                encoding: row.get("encoding").unwrap_or_else(|_| "utf-8".to_string()),
                keepalive_interval: row.get("keepalive_interval").unwrap_or(30),
                term_type: row.get("term_type").unwrap_or_else(|_| "xterm-256color".to_string()),
                description: row.get("description").unwrap_or_default(),
                advanced_options: vault::decrypt_advanced(&row.get::<_, String>("advanced_options").unwrap_or_default()),
                color: row.get("color").unwrap_or_default(),
            })
        }
        Ok(vec)
    });
    // 不要打印查询结果：SshSubMenu 含密码与私钥口令，会明文落到 stdout / 日志。
    if let Ok(records) = res {
        Response::new(200, records, "操作成功！")
    } else {
        Response::new(400, Vec::new(), "查询失败！")
    }
}


// ==================== P3 会话保险库（命令层只用下面 4 个入口） ====================

pub async fn vault_status() -> Response<vault::VaultStatus> {
    Response::new(200, vault::status(), "操作成功！")
}

pub async fn vault_lock() -> Response<bool> {
    vault::lock();
    Response::new(200, true, "保险库已锁定")
}

pub async fn vault_unlock(password: String) -> Response<bool> {
    match vault::unlock(&password) {
        Ok(()) => Response::new(200, true, "解锁成功"),
        Err(msg) => Response::new(300, false, &msg),
    }
}

/// 启用保险库并（可选）把已有明文凭据一次性加密。
/// 先写 meta 再迁移：迁移失败时 meta 已在，用户仍可解锁后重试「加密现有明文」。
pub async fn vault_enable(password: String, migrate: bool) -> Response<usize> {
    if let Err(msg) = vault::setup(&password) {
        return Response::new(300, 0, &msg);
    }
    if migrate {
        migrate_secrets(true).await
    } else {
        Response::new(200, 0, "保险库已启用（现有明文凭据未加密，可在设置里补做迁移）")
    }
}

/// 取消保险库：验证主密码 → 把密文全部还原为明文 → 删除 meta。
/// 顺序不能反，否则会话会变成永远解不开的孤儿密文。
pub async fn vault_disable(password: String) -> Response<usize> {
    if let Err(msg) = vault::unlock(&password) {
        return Response::new(300, 0, &msg);
    }
    let res = migrate_secrets(false).await;
    if res.code != 200 {
        return res;
    }
    if let Err(msg) = vault::purge() {
        return Response::new(300, 0, &msg);
    }
    Response::new(200, res.data, "保险库已移除，凭据已恢复明文")
}

pub async fn vault_encrypt_existing() -> Response<usize> {
    migrate_secrets(true).await
}

/// 逐行迁移，只改「确实需要改」的行；`encrypt=false` 时把密文还原为明文。
async fn migrate_secrets(encrypt: bool) -> Response<usize> {
    if !vault::is_enabled() {
        return Response::new(300, 0, "保险库尚未创建");
    }
    if !vault::is_unlocked() {
        return Response::new(423, 0, "请先解锁保险库");
    }
    let res = exec(|conn| {
        let rows: Vec<(i32, String, String, String)> = {
            let mut stmt = conn.prepare(
                "SELECT id, password, passphrase, advanced_options FROM connections",
            )?;
            let collected = stmt
                .query_map([], |row| {
                    Ok((
                        row.get::<_, i32>(0)?,
                        row.get::<_, String>(1)?,
                        row.get::<_, String>(2)?,
                        row.get::<_, String>(3)?,
                    ))
                })?
                .collect::<rusqlite::Result<Vec<_>>>()?;
            collected
        };
        let mut changed = 0usize;
        for (id, password, passphrase, advanced) in rows {
            let next = if encrypt {
                (
                    vault::encrypt_secret(&password),
                    vault::encrypt_secret(&passphrase),
                    vault::encrypt_advanced(&advanced),
                )
            } else {
                (
                    vault::decrypt_secret(&password),
                    vault::decrypt_secret(&passphrase),
                    vault::decrypt_advanced(&advanced),
                )
            };
            if next.0 == password && next.1 == passphrase && next.2 == advanced {
                continue;
            }
            conn.execute(
                "UPDATE connections SET password=?1, passphrase=?2, advanced_options=?3 WHERE id=?4",
                params![next.0, next.1, next.2, id],
            )?;
            changed += 1;
        }
        Ok(changed)
    });
    match res {
        Ok(n) => Response::new(200, n, "迁移完成"),
        Err(e) => Response::new(300, 0, &format!("迁移失败：{}", e)),
    }
}
