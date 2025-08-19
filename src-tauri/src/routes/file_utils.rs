use crate::{
    entity::structs::DirAndFileInfo,
    response::resp_data::ResData,
    utils::{
        fileUtil::{all_path, get_document_notebooks_data, read_dir_info},
        regexUtil,
    },
};
use hex;
use md5::Digest;
use std::{error::Error, path::{Path, PathBuf}};
use tauri::{AppHandle, Manager};
use tauri_plugin_opener::OpenerExt;
use tauri_plugin_store::StoreExt;
#[tauri::command]
pub async fn my_read_file(path: std::path::PathBuf) -> String {
    // 读取文件内容，以文本字符串形式返回
    std::fs::read_to_string(path).unwrap()
}

// 获取本地笔记本列表  未使用
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
// 获取本地笔记本列表
#[tauri::command]
pub async fn get_document_notebooks(app_handle: tauri::AppHandle) -> ResData<String> {
    let app_dir = app_handle.path().document_dir();
    let store_ = app_handle.store("settings.json");
    let mut children_dir_and_file_info_1 = DirAndFileInfo::create(
        "notebooks".to_string(),
        "".to_string(),
        "notebooks".to_string(),
        "".to_string(),
        "notebooks".to_string(),
        false,
    );
    match store_ {
        Ok(store) => match store.get("md_working_directory") {
            Some(value) => {
                let md_working_directory = value.as_str().unwrap();
                return notebooks(&app_handle, md_working_directory);
            }
            None => match app_dir {
                Ok(app_dir_) => {
                    let path_op = app_dir_.to_str();
                    match path_op {
                        Some(path) => {
                            return notebooks(&app_handle, path);
                        }
                        None => {
                            return ResData {
                                code: 500,
                                msg: "读取文档目录失败".to_string(),
                                data: serde_json::to_string(&children_dir_and_file_info_1).unwrap(),
                            };
                        }
                    }
                }
                Err(err) => {
                    return ResData {
                        code: 500,
                        msg: "读取文档目录失败".to_string(),
                        data: serde_json::to_string(&children_dir_and_file_info_1).unwrap(),
                    };
                }
            },
        },
        Err(err) => match app_dir {
            Ok(app_dir_) => {
                let path_op = app_dir_.to_str();
                match path_op {
                    Some(path) => {
                        return notebooks(&app_handle, path);
                    }
                    None => {
                        return ResData {
                            code: 500,
                            msg: "读取文档目录失败".to_string(),
                            data: serde_json::to_string(&children_dir_and_file_info_1).unwrap(),
                        };
                    }
                }
            }
            Err(err) => {
                return ResData {
                    code: 500,
                    msg: "读取文档目录失败".to_string(),
                    data: serde_json::to_string(&children_dir_and_file_info_1).unwrap(),
                };
            }
        },
    }

    ResData {
        code: 500,
        msg: "读取文档目录失败".to_string(),
        data: serde_json::to_string(&children_dir_and_file_info_1).unwrap(),
    }
}

pub fn notebooks(app: &AppHandle, path_parameter: &str) -> ResData<String>  {
    let mut children_dir_and_file_info_1 = DirAndFileInfo::create(
        "notebooks".to_string(),
        "".to_string(),
        "notebooks".to_string(),
        "".to_string(),
        "notebooks".to_string(),
        false,
    );
    let dir = PathBuf::from(path_parameter);
    //let dir: &Path = Path::new(path_parameter);
    // match app_dir {
    //     Ok(dir) => {
    //         // 本地文件目录
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
        // }
    //     Err(e) => {
    //         return ResData {
    //             code: 500,
    //             msg: "读取文档目录失败".to_string(),
    //             data: serde_json::to_string(&children_dir_and_file_info_1).unwrap(),
    //         }
    //     }
    //     _ => {}
    // }
}
// 生成字符串的MD5哈希值
pub fn generate_md5_hash(input: &str) -> String {
    let digest = md5::compute(input);
    hex::encode(digest.as_ref())
}
// 读取用户选择的目录内容
// #[tauri::command]
// pub async fn read_selected_directory(app_handle: tauri::AppHandle) -> Result<ResData<Vec<String>>, String> {
//     let dialog_result = app_handle.dialog().file().blocking_pick_folder();

//     match dialog_result {
//         Some(path) => {
//             // 用户选择了目录
//             let dir_path = path;
//             let paths = std::fs::read_dir(&dir_path);

//             match paths {
//                 Ok(entries) => {
//                     let mut files: Vec<String> = vec![];
//                     for entry in entries {
//                         match entry {
//                             Ok(entry) => {
//                                 let path = entry.path();
//                                 if let Some(file_name) = path.file_name() {
//                                     if let Some(name_str) = file_name.to_str() {
//                                         files.push(name_str.to_string());
//                                     }
//                                 }
//                             }
//                             Err(_) => continue,
//                         }
//                     }

//                     Ok(ResData {
//                         code: 200,
//                         msg: "读取成功".to_string(),
//                         data: files,
//                     })
//                 }
//                 Err(_) => {
//                     Ok(ResData {
//                         code: 500,
//                         msg: "无法读取目录".to_string(),
//                         data: Vec::new(),
//                     })
//                 }
//             }
//         }
//         None => {
//             // 用户取消了选择
//             Ok(ResData {
//                 code: 400,
//                 msg: "未选择目录".to_string(),
//                 data: Vec::new(),
//             })
//         }
//     }
// }
