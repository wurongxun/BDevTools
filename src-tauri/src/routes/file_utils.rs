use crate::{
    entity::structs::DirAndFileInfo,
    response::resp_data::ResData,
    utils::{
        fileUtil::{default_document_data, md_notebooks, read_dir_info},
        regexUtil,
    },
};
use hex;
use md5::Digest;
use std::{
    error::Error,
    path::{Path, PathBuf},
};
use tauri::{AppHandle, Manager};
use tauri_plugin_opener::OpenerExt;
use tauri_plugin_store::StoreExt;

#[tauri::command]
pub async fn my_read_file(path: std::path::PathBuf) -> String {
    // 读取文件内容，以文本字符串形式返回
    std::fs::read_to_string(path).unwrap()
}

// 打开目录
#[tauri::command]
pub async fn file_manager_open_path(app_handle: tauri::AppHandle, path: String) -> ResData<String> {
    let re = app_handle.opener().open_path(path, None::<String>);
    match re {
        Ok(_) => {
            return ResData {
                code: 200,
                msg: "打开目录成功".to_string(),
                data: "".to_string(),
            };
        }
        Err(e) => {
            return ResData {
                code: 500,
                msg: e.to_string(),
                data: "".to_string(),
            };
        }
    }
}
#[tauri::command]
pub async fn get_document_notebooks(app_handle: tauri::AppHandle) -> ResData<String> {
    //查看配置
    let store_ = app_handle.store("settings.json");
    match store_ {
        Ok(store) => {
            let md_working_directory_val = store.get("md_working_directory");
            match md_working_directory_val {
                Some(md_working_directory) => {
                    // 修复：正确获取 JSON 对象中的 "value" 字段
                    if let Some(directory_obj) = md_working_directory.as_object() {
                        if let Some(value_field) = directory_obj.get("value") {
                            if let Some(directory_path) = value_field.as_str() {
                                // 添加判断是否为空字符串
                                if !directory_path.is_empty() {
                                    // 获取目录下的所有文件和文件夹
                                    return md_notebooks(&app_handle, directory_path);
                                }
                            }
                        }
                    }
                    return default_document_data(app_handle);
                }
                None => {
                    return default_document_data(app_handle);
                }
            }
        }
        Err(e) => {
            return default_document_data(app_handle);
        }
    }
}
