use crate::{
    entity::structs::DirAndFileInfo,
    response::resp_data::ResData,
    utils::fileUtil::{all_path, get_document_notebooks_data, read_dir_info},
};
use std::error::Error;
use tauri::{AppHandle, Manager};
#[tauri::command]
pub async fn my_read_file(path: std::path::PathBuf) -> String {
    // 读取文件内容，以文本字符串形式返回
    std::fs::read_to_string(path).unwrap()
}

// 获取本地笔记本列表
#[tauri::command]
pub async fn get_home_notebooks(
    app_handle: tauri::AppHandle,
) -> Result<ResData<Vec<String>>, String> {
    let app_dir = app_handle.path().document_dir();
    match app_dir {
        Ok(dir) => {
            // 本地文件目录
            let local_data_dir = dir.as_path().join("notebooks");
            println!("local_data_dir： {:?}", local_data_dir);
            let local_data_dir_2 = local_data_dir.clone();
            all_path(&local_data_dir_2.as_os_str().to_str().unwrap().to_string());
            read_dir_info(&local_data_dir);
            // 文件夹是否存在
            if local_data_dir.exists() {
                let mut notebooks: Vec<String> = vec![];
                let paths = std::fs::read_dir(local_data_dir);
                match paths {
                    Ok(ps) => {
                        // 遍历目录
                        for path in ps {
                            let entry = path.unwrap();
                            let path = entry.path();
                            if path.is_dir() {
                                let name = path.file_stem().unwrap();
                                let n = name.to_str().unwrap().to_string();
                                notebooks.push(n);
                            }
                        }
                        return Ok(ResData {
                            code: 200,
                            msg: "success".to_string(),
                            data: notebooks,
                        });
                    }
                    Err(_) => {
                        return {
                            Ok(ResData {
                                code: 500,
                                msg: "读取目录失败".to_string(),
                                data: Vec::new(),
                            })
                        }
                    }
                }
            }
            // 目录不存在则创建
            let a = std::fs::create_dir_all(local_data_dir);
            match a {
                Ok(_) => println!("创建目录成功"),
                Err(_) => return Err("创建目录失败".to_string()),
            }
        }
        Err(e) => {
            println!("{}", e.to_string());
        }
        _ => {}
    }

    Ok(ResData {
        code: 500,
        msg: "读取目录失败".to_string(),
        data: Vec::new(),
    })
}

// 获取本地笔记本列表
#[tauri::command]
pub async fn get_document_notebooks(app_handle: tauri::AppHandle) -> ResData<String> {
    let app_dir = app_handle.path().document_dir();
    let mut children_dir_and_file_info_1 = DirAndFileInfo::create(
        "".to_string(),
        "".to_string(),
        "".to_string(),
        "notebooks".to_string(),
        false,
    );
    match app_dir {
        Ok(dir) => {
            // 本地文件目录
            let local_data_dir = dir.as_path().join("notebooks");
            let local_data_dir_ = local_data_dir.clone();
            // println!("{:?}", local_data_dir);
            // 文件夹是否存在
            if local_data_dir.exists() {
                return get_document_notebooks_data(
                    &local_data_dir.as_os_str().to_str().unwrap().to_string(),
                );
            }

            // 目录不存在则创建
            let a = std::fs::create_dir_all(local_data_dir);
            match a {
                Ok(_) => {
                    return get_document_notebooks_data(
                        &local_data_dir_.as_os_str().to_str().unwrap().to_string(),
                    )
                }
                Err(_) => {
                    let j = serde_json::to_string(&children_dir_and_file_info_1).unwrap();
                    return ResData {
                        code: 500,
                        msg: "读取文档目录失败".to_string(),
                        data: j,
                    };
                }
            }
        }
        Err(e) => {
            return ResData {
                code: 500,
                msg: "读取文档目录失败".to_string(),
                data: serde_json::to_string(&children_dir_and_file_info_1).unwrap(),
            }
        }
        _ => {}
    }
    ResData {
        code: 500,
        msg: "读取文档目录失败".to_string(),
        data: serde_json::to_string(&children_dir_and_file_info_1).unwrap(),
    }
}
