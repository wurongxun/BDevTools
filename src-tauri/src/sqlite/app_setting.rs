use anyhow::{anyhow, Result};
use log::info;
use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};
use std::{path::PathBuf, result::Result::Ok};
use uuid::Uuid;
use crate::{entity::structs::SettingMap, response::response::Response, sqlite::sqlite::exec};
use crate::core::ctx::AppContext;
/**
 *
 * 获取开机启动设置状态
 */
pub async fn get_auto_start_state(ctx: &AppContext) -> bool {
    let res1 = exec(|conn| {
        let mut stmt = conn.prepare("select * from app_setting_info where app_setting_item_name='autostart'")?;
        let mut res = stmt.query(params![])?;
        let mut app_auto_sapp_auto_start_state = false; // 默认为false
        while let Some(row) = res.next()? {
            let a: String = row.get("app_setting_item_value")?;
            if a == "true" {
                app_auto_sapp_auto_start_state= true;
            }
        }
        Ok(app_auto_sapp_auto_start_state)
    });
    println!(
        "检查autostart状态 get_auto_start_state ? {:?}",
        res1
    );
    match res1 {
        Ok(res) => {
            // 通过上下文设置开机自启（平台能力由门面层注入）
            ctx.set_autostart(res);
            if res {
                println!("检查 autostart 已启用");
            } else {
                println!("检查 autostart 已禁用");
            }
            return res;
        }
        Err(_) => {
            return false;
        }
    }
}

/// 查询设置数据
pub async fn query_setting_item(id:String) -> Response<Vec<SettingMap>> {
    println!("query_setting_item");
    let res = exec(|conn| {
        let mut stmt = conn.prepare("select * from app_setting_info where id=?1")?;
        let mut res = stmt.query(params![id])?;
       
        let mut vec = vec![];
        while let Some(row) = res.next()? {
            vec.push(SettingMap {
                id: row.get("id")?,
                app_setting_item_name: row.get("app_setting_item_name")?,
                app_setting_item_value: row.get("app_setting_item_value")?,
            })
        }
       Ok(vec)
    });
    println!("ELECT COUNT(*) FROM app_setting_info end: {:?}", res);
    if let Ok(records) = res {
        Response::new(200, records, "操作成功！")
    } else {
        Response::new(400, Vec::new(), "操作成功！")
    }
}
/// 查询全部设置数据
pub async fn query_setting_list() -> Response<Vec<SettingMap>> {
    println!("setting_list");
    let res = exec(|conn| {
       // let res2: rusqlite::Result<String> =
        let count: u32 = conn.query_row("SELECT COUNT(*) FROM app_setting_info", [], |row| {
            row.get(0)
        }).expect("Unable to count rows");
        println!("ELECT COUNT(*) FROM app_setting_info res2: {}",count);
        let mut stmt = conn.prepare("select * from app_setting_info")?;
        let mut res = stmt.query(params![])?;
        let mut vec = vec![];
        while let Some(row) = res.next()? {
            vec.push(SettingMap {
                id: row.get("id")?,
                app_setting_item_name: row.get("app_setting_item_name")?,
                app_setting_item_value: row.get("app_setting_item_value")?,
            })
        }
        Ok(vec)
    });
    println!("ELECT COUNT(*) FROM app_setting_info end: {:?}", res);
    if let Ok(records) = res {
        Response::new(200, records, "操作成功！")
    } else {
        Response::new(400, Vec::new(), "操作成功！")
    }
}
/// 更新设置数据
pub async fn updatate_setting(ctx: &AppContext, id: String, appSettingItemValue: String, appSettingtemName: String) -> Response<String> {
    println!("updatate_setting_list");
    let res = exec(|conn| {
        let res = conn.execute(
            "update app_setting_info set app_setting_item_value=?1 where id=?2",
            params![appSettingItemValue, id],
        )?;
        if res == 0 {
            return Err(anyhow!("操作失败！"));
        }
        Ok(())
    });
    get_auto_start_state(ctx).await;
    match res {
        Ok(_) => {
            ctx.emit("app_info_notification", serde_json::json!({
                "message": "操作成功！"
            }));
            ctx.notify("设置成功!");
            let res1 = exec(|conn| {
              
                let mut stmt = conn.prepare("select * from app_setting_info")?;
                let mut res = stmt.query(params![])?;
                let mut vec = vec![];
                while let Some(row) = res.next()? {
                    vec.push(SettingMap {
                        id: row.get("id")?,
                        app_setting_item_name: row.get("app_setting_item_name")?,
                        app_setting_item_value: row.get("app_setting_item_value")?,
                    })
                }
                Ok(vec)
            });
            println!("ELECT COUNT(*) FROM app_setting_info end: {:?}", res1);

            Response::new(200, "OK".to_string(), "操作成功！")
        }
        Err(e) => {
            ctx.emit("app_info_notification", serde_json::json!({
                "message": "操作失败！"
            }));
            ctx.notify("设置成功!");
            Response::new(300, "OK".to_string(), &format!("{}", e))
        }
    }
}
