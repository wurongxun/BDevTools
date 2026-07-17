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
use std::fs;
use std::io::Read;
use chardetng::EncodingDetector;

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
/**
 * 更新文件夹名称
 * @param path 文件夹
 * @param up_folder_name 文件夹名称
 */

#[tauri::command]
pub async fn up_folder_name(
    app_handle: AppHandle,
    path: String,
    up_folder_name: String,
) -> Result<ResData<String>, String> {
    let dir = PathBuf::from(&path);
    // 文件夹是否存在
    if !dir.exists() {
        return Ok(ResData {
            code: 500,
            msg: "选择文件夹不存在！".to_string(),
            data: String::new(),
        });
    }
    //判断文件夹名称是否为空
    if up_folder_name.is_empty() {
        return Ok(ResData {
            code: 500,
            msg: "文件夹名称不能为空！".to_string(),
            data: String::new(),
        });
    }
    //文件路径 转换  linux and mac  "//"  windows "\\"
    //将最后的文件夹名称替换为 up_folder_name
    let up_folder_name = regexUtil::replace_last_path_component(&path, &up_folder_name);

    //文件夹拼接 更换"/"追后的字符
    let new_name_res = tokio::fs::rename(&path, &up_folder_name).await;
    match new_name_res {
        Ok(_) => {
            return Ok(ResData {
                code: 200,
                msg: "操作成功".to_string(),
                data: String::new(),
            });
        }
        Err(e) => {
            return Ok(ResData {
                code: 500,
                msg: e.to_string(),
                data: String::new(),
            });
        }
    }
}
// 检测文件编码
#[tauri::command]
pub async fn detect_file_encoding(path: &str) -> Result<ResData<String>, Box<dyn std::error::Error>> {
    let mut file = fs::File::open(path)?;
    let mut sample = [0u8; 8192];
    let bytes_read = file.read(&mut sample)?;

    let mut detector = EncodingDetector::new(chardetng::Iso2022JpDetection::Allow);
    detector.feed(&sample[..bytes_read], bytes_read == 0);
    let encoding = detector.guess(None, chardetng::Utf8Detection::Allow);

    Ok(ResData {
        code: 200,
        msg: "检测成功".to_string(),
        data: encoding.name().to_string(),
    })
}

