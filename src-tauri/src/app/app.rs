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

use tauri_plugin_store::StoreExt;

use tauri::{path::BaseDirectory, Manager};
lazy_static! {
    pub static ref APP: Mutex<App> = Mutex::new(App::new());
}

///初始化app文件夹
pub fn init_app_dir() -> bool {
    if !Path::new(&crate::app::app::APP.lock().app_dir).exists() {
        if let Ok(_) = create_dir_all(&crate::app::app::APP.lock().app_dir) {
            return true;
        }
        return false;
    }
    true
}
pub fn app_settings(app: &tauri::AppHandle) {
    let store_ = app.store("settings.json");
    match store_ {
        Ok(store) => {
            let current_theme = store
                .get("current_theme");
            match current_theme {
                Some(_) => todo!(),
                None => {
                    store.set("current_theme", json!({ "value": "light" }));
                },
            };
            
            let autostart_value = store
                .get("auto_start");

            match autostart_value {
                Some(_) => todo!(),
                None => {
                    store.set(" auto_start", json!({ "value": "false" }));
                },
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
