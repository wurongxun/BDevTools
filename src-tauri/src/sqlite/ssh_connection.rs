/*! # SSH/SFTP 连接配置持久化（sqlite 表）
存储 ssh/sftp 连接配置与收藏分组（favorites），供连接列表/右键子菜单使用。*/
use crate::response::response::Response;
use crate::sqlite::sqlite::exec;
use anyhow::anyhow;
use log::info;
use rusqlite::params;
use serde::{Deserialize, Serialize};

/// 数据对应字段
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
}

/// 创建一个连接
pub async fn connection_create(map: SshSubMenu) -> Response<Option<bool>> {
    let res = exec(|conn| {
        let count: u32 = conn
        .query_row("SELECT COUNT(*) FROM connections where favorites_name=?1 and name=?2",  params![map.favorites_name,map.name], |row| row.get(0))
        .expect("Unable to count rows");
        println!("ELECT COUNT(*) FROM connections res2: {}", count);
        if count > 0 {
            return Err(anyhow!("连接已存在！")); 
        }
        let conns = conn.execute(
      "insert into connections(name,address,port,username,password,agreement,favorites_name,affix,auth_type,private_key_path) values(?1,?2,?3,?4,?5,?6,?7,?8,?9,?10)",
      params![
        map.name,
        map.address,
        map.port,
        map.username,
        map.password,
        map.agreement,
        map.favorites_name,
        map.affix,
        map.auth_type,
        map.private_key_path,
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
    let res = exec(|conn| {
        let affected = conn.execute(
            "UPDATE connections SET name=?1, address=?2, port=?3, username=?4, password=?5, agreement=?6, favorites_name=?7, affix=?8, auth_type=?9, private_key_path=?10 WHERE id=?11",
            params![
                map.name,
                map.address,
                map.port,
                map.username,
                map.password,
                map.agreement,
                map.favorites_name,
                map.affix,
                map.auth_type,
                map.private_key_path,
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
        let count: u32 = conn
            .query_row("SELECT COUNT(*) FROM connections", [], |row| row.get(0))
            .expect("Unable to count rows");
        println!("ELECT COUNT(*) FROM connections res2: {}", count);

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
                password: row.get("password")?,
                agreement: row.get("agreement")?,
                favorites_name: row.get("favorites_name")?,
                affix: row.get("affix")?,
                auth_type: row.get("auth_type").unwrap_or_else(|_| "password".to_string()),
                private_key_path: row.get("private_key_path").unwrap_or_default(),
            })
        }
        Ok(vec)
    });
    println!("select * from connections end: {:?}", res);
    if let Ok(records) = res {
        println!("connection_list records -> {:?}", records);
        Response::new(200, records, "操作成功！")
    } else {
        Response::new(400, Vec::new(), "查询失败！")
    }
}
