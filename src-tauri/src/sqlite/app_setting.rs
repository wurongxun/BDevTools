use anyhow::Result;
use log::info;
use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};
use tauri_plugin_autostart::ManagerExt;
use std::{path::PathBuf, result::Result::Ok};
use tauri::{command, window, Manager, Window};
use uuid::Uuid;
use crate::{entity::structs::{Payload, SettingMap}, response::response::Response, sqlite::sqlite::exec};
use tauri_plugin_notification::{NotificationExt, PermissionState};
use tauri::{AppHandle, Emitter};
/**
 *
 * 获取开机启动设置状态
 */
pub async fn get_auto_start_state(app: &tauri::AppHandle) -> bool {
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
            // Get the autostart manager
            let autostart_manager = app.autolaunch();
            // 启用 autostart
            if res {
                let _ = autostart_manager.enable();
                  // 检查 enable 状态
                println!(
                    "检查 enable 状态 registered for autostart? {}",
                    autostart_manager.is_enabled().unwrap()
                );
            }else{
                // 禁用 autostart
                let _ = autostart_manager.disable();
            }
            
            return res;
        }
        Err(_) => {
            return false;
        }
    }
}

/// 查询设置数据
#[command]
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
#[command]
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
#[command]
pub async fn updatate_setting(app_handle: tauri::AppHandle,id:String,appSettingItemValue:String,appSettingtemName:String) -> Response<String> {
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
    get_auto_start_state(&app_handle).await;
    match res {
        Ok(_) => {
            let _ = app_handle.emit("app_info_notification",  Payload {
                message: "操作成功！".into(),
            },);
            // if app_handle.notification().permission_state().unwrap() == PermissionState::Unknown {
            //     app_handle.notification().request_permission().unwrap();
            // }
            if app_handle.notification().permission_state().unwrap() == PermissionState::Granted {
                app_handle.notification()
                    .builder()
                    .body("设置成功!")
                    .show().unwrap();
            }
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
            let _ = app_handle.emit("app_info_notification",  Payload {
                message: "操作失败！".into(),
            },);
            // if app_handle.notification().permission_state().unwrap() == PermissionState::Unknown {
            //     app_handle.notification().request_permission().unwrap();
            // }
            if app_handle.notification().permission_state().unwrap() == PermissionState::Granted {
                app_handle.notification()
                    .builder()
                    .body("设置成功!")
                    .show().unwrap();
            }
            Response::new(300, "OK".to_string(), &format!("{}", e))
        }
    }
}
