/*! # app初始化相关
> app 工作目录初始化
> 配置初始化等
 */
use lazy_static::lazy_static;
use parking_lot::Mutex;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::{
    fs::create_dir_all,
    path::{Path, PathBuf},
};
use tauri_plugin_autostart::MacosLauncher;
use tauri_plugin_autostart::ManagerExt;
use tauri_plugin_store::StoreExt;

use tauri::{path::BaseDirectory, Manager};
use crate::utils::fileUtil::generate_md5_hash;

lazy_static! {
    pub static ref APP: Mutex<App> = Mutex::new(App::new());
}

///初始化app文件夹
pub fn init_app_dir() -> bool {
    if !Path::new(&crate::app::app_store::APP.lock().app_dir).exists() {
        if let Ok(_) = create_dir_all(&crate::app::app_store::APP.lock().app_dir) {
            return true;
        }
        return false;
    }
    true
}
/**
* app “关联文件” 和 “打开方式” 打开文件时存储文件路径 key:文件路径md5 val:文件详细路径;保存在 store中，前端加载完成后请求全部数据 并打开
*/
pub fn app_openFile(file: &str, app: &tauri::AppHandle) {
    let store_ = app.store("openfile.json");
    let md5 = generate_md5_hash(file);
    match store_ {
        Ok(store) => {
            store.set(&md5, file );
        }
        Err(err) => {
            println!("Error opening store: {}", err);
        }
    }
}

pub fn app_settings(app: &tauri::AppHandle) {
    let store_ = app.store("settings.json");
    match store_ {
        Ok(store) => {
            let theme = store.get("theme");
            match theme {
                Some(val) => {
                    println!("theme {:?}", val);
                }
                None => {
                    store.set("theme", json!({ "value": "light" }));
                }
            };

            let autostart_value = store.get("auto_start");
            match autostart_value {
                Some(val) => {
                    println!("auto_start_value {:?}", val);
                }
                None => {
                    store.set("auto_start", json!({ "value": false }));
                }
            };

            let language_value = store.get("language");
            match language_value {
                Some(val) => {
                    println!("language_value {:?}", val);
                }
                None => {
                    store.set("language", json!({ "value": "zh_CN" }));
                }
            };

            let md_working_directory_value = store.get("md_working_directory");
            match md_working_directory_value {
                Some(val) => {
                    println!("md_working_directory_value {:?}", val);
                }
                None => {
                    // store.set("md_working_directory", json!({ "value": "" }));
                }
            };
        }
        Err(err) => {
            println!("Error opening store: {}", err);
        }
    }
}

pub fn app_autostart(app: &tauri::AppHandle) {
    // Get the autostart manager
    let autostart_manager = app.autolaunch();
    let store_ = app.store("settings.json");
    match store_ {
        Ok(store) => {
            let autostart_value = store.get("auto_start");
            match autostart_value {
                Some(val) => {
                    println!("auto_start_value {:?}", val);
                    let v = val.get("value");
                    match v {
                        Some(value) => {
                            if value.as_bool().unwrap() {
                                let _ = autostart_manager.enable();
                            } else {
                                let _ = autostart_manager.disable();
                            }
                        }
                        None => todo!(),
                    }
                }
                None => {
                    store.set("auto_start", json!({ "value": false }));
                    let _ = autostart_manager.disable();
                }
            };
        }
        Err(err) => {
            println!("Error opening store: {}", err);
        }
    }
}
/// app配置map
pub struct App {
    pub app_dir: PathBuf,
}

impl App {
    pub fn new() -> App {
        let cfg = tauri::Config::default();
        let default_path = PathBuf::from("com.bs"); // 占位路径
        App {
            app_dir: default_path,
        }
    }
    //更新app_dir的值
    pub fn update_app_dir(&mut self, app: &tauri::AppHandle) {
        println!("update_app_dir");
        let mut home_dir_path = app.path().home_dir().expect("failed to get home dir");
        home_dir_path.push("app");
        println!("update_app_dir1 {:?}", home_dir_path);
        //println!("update_app_dir1 APP.lock().app_dir {:?}",&crate::app::app::APP.lock().app_dir);
        // let path = app.path().resolve("path/to/something", BaseDirectory::Config);
        // match path {
        //     Ok(p) => App {
        //         app_dir:p.join("com.echosocket.bs"),
        //     },
        //     Err(_) => App {
        //         app_dir: PathBuf::new(),
        //     },

        // };

        //let mut _dir = &crate::app::app::APP.lock().app_dir;
        //_dir = &home_dir_path;
        self.app_dir = home_dir_path;
        println!("update_app_dir2 {:?}", self.app_dir);
    }
}
