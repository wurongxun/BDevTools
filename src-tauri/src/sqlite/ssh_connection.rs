/*! # sqlite连接管理模块*/
use crate::sqlite::sqlite::exec;
use crate::{entity::structs::Payload, response::response::Response};
use anyhow::Result;
use log::info;
use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};
use std::{path::PathBuf, result::Result::Ok};
use tauri::{command, window, Manager, Window};
use tauri_plugin_notification::{NotificationExt, PermissionState};
use uuid::Uuid;

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
}

/// 创建一个连接
#[command]
pub async fn connection_create(mut map: SshSubMenu) -> Response<Option<bool>> {
    let res = exec(|conn| {
        let count: u32 = conn
        .query_row("SELECT COUNT(*) FROM connections where favorites_name=?1 and name=?2",  params![map.favorites_name,map.name], |row| row.get(0))
        .expect("Unable to count rows");
        println!("ELECT COUNT(*) FROM connections res2: {}", count);
        if count > 0 {
            return Err(anyhow!("连接已存在！")); 
        }
        let conns = conn.execute(
      "insert into connections(name,address,port,username,password,agreement,favorites_name,affix) values(?1,?2,?3,?4,?5,?6,?7,?8)",
      params![
        map.name,
        map.address,
        map.port,
        map.username,
        map.password,
        map.agreement,
        map.favorites_name,
        map.affix,
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
#[command]
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
#[command]
pub async fn connection_edit(map: SshSubMenu) -> Response<bool> {
    let _ = exec(|conn| {
        let _ = conn.execute(
            "replace into connections(id,name,address,port,password) values(?1,?2,?3,?4,?5,?6,?7,?8)",
            params![map.id, map.name, map.address, map.port, map.password, map.agreement,
            map.favorites_name,map.affix]
        )?;
        Ok(())
    });
    Response::new(200, true, "修改成功！")
}

/// 查询连接列表
#[command]
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
