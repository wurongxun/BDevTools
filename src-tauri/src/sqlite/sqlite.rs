/*! # sqlite连接管理模块*/
use crate::{entity::structs::Payload, response::response::Response};
use anyhow::{anyhow, Result};
use log::info;
use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};
use std::{path::PathBuf, result::Result::Ok};
use tauri::{command, window, Manager, Window};
use tauri_plugin_notification::{NotificationExt, PermissionState};
use uuid::Uuid;
/// 创建数据库连接
fn create_sqlite_connection() -> Result<Connection> {
    println!("create_sqlite_connection start");
    let app_dir = PathBuf::from(&crate::app::app_store::APP.lock().app_dir);
    // 确保目录存在
    if !app_dir.exists() {
        std::fs::create_dir_all(&app_dir)?;
    }
    let database = app_dir.join("app.db");
    println!("create_sqlite_connection database: {:?}", database);
    let conn = Connection::open(database)?;
    // 初始化表结构
    init_tables(&conn)?;
    println!("create_sqlite_connection end");
    Ok(conn)
}

/// 初始化数据库表
fn init_tables(conn: &Connection) -> Result<()> {
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS connections (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            name TEXT NOT NULL,
            address TEXT NOT NULL,
            port INTEGER NOT NULL DEFAULT 22,
            username TEXT NOT NULL,
            password TEXT NOT NULL,
            agreement TEXT NOT NULL DEFAULT 'ssh',
            favorites_name TEXT NOT NULL DEFAULT 'default',
            affix INTEGER NOT NULL DEFAULT 0,
            auth_type TEXT NOT NULL DEFAULT 'password',
            private_key_path TEXT NOT NULL DEFAULT ''
        );"
    )?;
    // 迁移：为已有表添加新列（忽略已存在错误）
    let _ = conn.execute_batch("ALTER TABLE connections ADD COLUMN auth_type TEXT NOT NULL DEFAULT 'password';");
    let _ = conn.execute_batch("ALTER TABLE connections ADD COLUMN private_key_path TEXT NOT NULL DEFAULT '';");
    // 初始化数据库连接配置表
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS db_connections (
            id TEXT PRIMARY KEY,
            name TEXT NOT NULL,
            db_type TEXT NOT NULL,
            host TEXT NOT NULL,
            port INTEGER NOT NULL,
            username TEXT NOT NULL,
            password TEXT NOT NULL,
            database_name TEXT,
            ssh_enabled INTEGER NOT NULL DEFAULT 0,
            ssh_host TEXT,
            ssh_port INTEGER,
            ssh_username TEXT,
            ssh_password TEXT,
            ssh_key_path TEXT,
            created_at TEXT,
            updated_at TEXT
        );"
    )?;
    Ok(())
}
/// 封装一个方法，获取连接
pub fn exec<F, T>(func: F) -> Result<T>
where
    F: FnOnce(&mut Connection) -> Result<T>,
{
    match create_sqlite_connection() {
        Ok(mut conn) => func(&mut conn),
        Err(e) => Err(e),
    }
}
//查看是否已经建表
fn table_exists(conn: &mut Connection, table_name: &str) -> Result<bool> {
    let mut stmt = conn.prepare("SELECT name FROM sqlite_master WHERE type='table' AND name=?1")?;
    let exists = stmt.exists([table_name])?;
    Ok(exists)
}
/// 创建表
pub fn init_table() {
    println!("=====初始化表单=====");
    let res = exec(|mut conn: &mut Connection| {
        let connections_re = table_exists(&mut conn, "sys_info")?;
        if !connections_re {
            // 初始化connections信息表
            println!("初始化connections信息表");
            conn.execute(
                "CREATE TABLE IF NOT EXISTS connections (
                    id INTEGER  PRIMARY KEY AUTOINCREMENT,
                    name TEXT NOT NULL,
                    address TEXT NOT NULL,
                    port INTEGER DEFAULT 22,
                    username TEXT DEFAULT '',
                    password TEXT DEFAULT '',
                    agreement TEXT NOT NULL ,
                    favorites_name TEXT NOT NULL ,
                    affix bool NOT null default false)",
                [],
            )?;
        }

        let sys_info_re = table_exists(&mut conn, "sys_info")?;
        if !sys_info_re {
            // 初始化系统信息表
            conn.execute(
                "create table if not exists sys_info(
                        id text primary key,
                        auto_refresh bool not null default false,
                        auto_refresh_time number not null default 3,
                        pubsub bool not null default true
                    )",
                [],
            )?;
            println!("初始化系统信息表结束");
        }

        let app_setting_info_re = table_exists(&mut conn, "app_setting_info")?;
        if !app_setting_info_re {
            // 初始化设置信息表
            conn.execute(
                "create table if not exists app_setting_info(
                        id TEXT primary key,
                        app_setting_item_name TEXT not null ,
                        app_setting_item_value TEXT not null 
                    )",
                [],
            )?;
            println!("初始化设置信息表结束");
        }
        //schedule 日程表
        let re_schedule = table_exists(&mut conn, "schedule_from")?;
        if !re_schedule {
            conn.execute(
                "CREATE TABLE IF NOT EXISTS schedule_from (
                     id INTEGER  PRIMARY KEY AUTOINCREMENT,
                     schedule_start_date TEXT NOT NULL,
                     schedule_end_date TEXT NOT NULL,
                     schedule_start_time TEXT NOT NULL,
                     schedule_end_time TEXT NOT NULL,
                     schedule_title TEXT NOT NULL,
                     schedule_content TEXT NOT NULL,
                     schedule_address TEXT NOT NULL,
                     schedule_alarm number not null default 3)",
                [],
            )?;
            println!("初始化日程表结束");
        }

        // 查询初始化数据是否存在
        let res: rusqlite::Result<String> =
            conn.query_row("select * from sys_info where id = '1'", [], |row| {
                row.get(0)
            });
        // 报错代表初始化数据不存在
        match res {
            Err(_) => {
                let res = conn.execute("insert into sys_info(id,auto_refresh,auto_refresh_time,pubsub) values('1',false,3,true)",[])?;
                if res == 0 {
                    panic!("init system info error")
                }
            }
            _ => {}
        }
        // 查询app_setting_info初始化数据是否存在
        let res2: rusqlite::Result<String> =
            conn.query_row("SELECT COUNT(*) FROM app_setting_info", [], |row| {
                row.get(0)
            });
        // 报错代表app_setting_info初始化数据不存在
        match res2 {
            Err(_) => {
                let tx = conn.transaction()?;
                tx.execute("insert into app_setting_info (id,app_setting_item_name,app_setting_item_value) values (?1,?2,?3)", &[&"1", &"language", &"CN"])?;
                tx.execute("insert into app_setting_info (id,app_setting_item_name,app_setting_item_value) values (?1,?2,?3)", &[&"2", &"autostart", &"true"])?;
                tx.execute("insert into app_setting_info (id,app_setting_item_name,app_setting_item_value) values (?1,?2,?3)", &[&"3", &"update_reminder", &"true"])?;
                tx.execute("insert into app_setting_info (id,app_setting_item_name,app_setting_item_value) values (?1,?2,?3)", &[&"4", &"use_dark", &"false"])?;
                let res = tx.commit();
                //assert!(res.is_err());
            }
            _ => {}
        }
        // 查询connections初始化数据是否存在
        let res3: rusqlite::Result<String> =
            conn.query_row("SELECT COUNT(*) FROM connections", [], |row| row.get(0));
        // 报错代表connections初始化数据不存在
        match res3 {
            Err(_) => {
                let tx = conn.transaction()?;
                //tx.execute("insert into connections (name,address,port,username,password,agreement,favorites_name,affix) values (?1,?2,?3,?4,?5,?6,?7,?8)",
                //&[&"Edit", &"Edit", &"22", &"Edit", &"Edit", &"Edit", &"Edit", &"true"])?;
                let r1 = tx.execute("insert into connections (name,address,port,username,password,agreement,favorites_name,affix) values ('Edit', 'Edit', 22, 'Edit', 'Edit', 'Edit', 'Edit', false)",[]);
                let r2 = tx.execute("insert into connections (name,address,port,username,password,agreement,favorites_name,affix) values ('Add', 'Add', 22, 'Edit', 'Edit', 'Edit', 'Edit', false)",[]);

                let res = tx.commit();
                match res {
                    Ok(_) => {
                        println!("connections table_name init OK");
                    }

                    Err(e) => {
                        println!("connections table_name init error{:?}", e);
                    }
                };
            }
            _ => {
                println!("connections table 已初始化");
            }
        }

        Ok(())
    });
    info!("init table 结束 {:?}", res);
}

/// 更新系统信息
#[command]
pub async fn update_sys_info(
    window: Window,
    id: String,
    auto_refresh: bool,
    auto_refresh_time: isize,
    pubsub: bool,
) -> Response<bool> {
    let res = exec(|conn| {
        let res = conn.execute(
            "update sys_info set auto_refresh=?1,auto_refresh_time=?2,pubsub=?3 where id=?4",
            params![auto_refresh, auto_refresh_time, pubsub, id],
        )?;
        if res == 0 {
            return Err(anyhow!("操作失败！"));
        }
        Ok(())
    });

    match res {
        Ok(_) => {
            // let _ = window.emit_all("sys_info_update", true);
            Response::new(200, true, "操作成功！")
        }
        Err(e) => Response::new(300, false, &format!("{}", e)),
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SysInfo {
    pub id: String,
    #[serde(rename = "autoRefresh")]
    pub auto_refresh: bool,
    #[serde(rename = "autoRefreshTime")]
    pub auto_refresh_time: isize,
    pub pubsub: bool,
}
/// 查询系统信息
#[command]
pub async fn query_sys_info() -> Response<Option<SysInfo>> {
    let res = exec(|conn| {
        let res = conn.query_row("select * from sys_info", [], |row| {
            Ok(SysInfo {
                id: row.get("id")?,
                auto_refresh: row.get("auto_refresh")?,
                auto_refresh_time: row.get("auto_refresh_time")?,
                pubsub: row.get("pubsub")?,
            })
        })?;
        Ok(res)
    });
    match res {
        Ok(res) => Response::new(200, Some(res), "查询成功！"),
        Err(e) => Response::new(200, None, &format!("{}", e)),
    }
}
