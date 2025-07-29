use crate::response::resp_data::{ResData, ResDataNoData};
use crate::utils::fileUtil::read_dir_info;
use std::path::PathBuf;
use tauri::{AppHandle, Manager};
use tokio::fs::File;
use tokio::io::AsyncReadExt;
// 获取目录下笔记
#[tauri::command]
pub async fn get_note_list(
    app_handle: AppHandle,
    path: String,
) -> Result<ResData<Vec<String>>, String> {
    let paths = tokio::fs::read_dir(PathBuf::from(path)).await;
    //文件夹下所有文件
    let mut notes: Vec<String> = vec![];
    match paths {
        Ok(ps) => {
            let mut ps = ps; // 将 ps 声明为可变变量
            while let Some(entry) = ps.next_entry().await.map_err(|e| e.to_string())? {
                let path = entry.path();
                if path.is_file() {
                    if let Some(filename) = path.file_stem() {
                        notes.push(filename.to_string_lossy().to_string());
                    }
                }
            }
            return Ok(ResData {
                code: 200,
                msg: "success".to_string(),
                data: notes,
            });
        }
        Err(_) => {
            return Ok(ResData {
                code: 500,
                msg: "读取目录失败".to_string(),
                data: Vec::new(),
            })
        }
    }

    // if let Some(dir) = app_dir {}
    Ok(ResData {
        code: 500,
        msg: "文件不存在".to_string(),
        data: Vec::new(),
    })
}

// 读取本地笔记文件
#[tauri::command]
pub async fn read_note_file(
    app_handle: tauri::AppHandle,
    current_notebook: String,
) -> Result<ResData<String>, String> {
    let contents = tokio::fs::read_to_string(current_notebook).await;
    match contents {
        Ok(_contents) => {
            return Ok(ResData {
                code: 200,
                msg: "读取成功".to_string(),
                data: _contents,
            })
        }
        Err(err) => {
            return Ok(ResData {
                code: 500,
                msg: "笔记读取失败".to_string(),
                data: err.to_string(),
            })
        }
    }
}

// 创建本地 笔记本/文件夹
#[tauri::command]
pub async fn create_notebook(
    app_handle: tauri::AppHandle,
    notebook_name: String,
) -> Result<ResData<String>, String> {
    let app_dir = app_handle.path().document_dir();
    match app_dir {
        Ok(dir) => {
            // 本地文件目录
            let local_data_dir = dir.as_path().join("notebooks");
            // 文件夹是否存在
            if local_data_dir.exists() {
                let dir = local_data_dir.join(notebook_name);
                // 创建文件夹
                let res = std::fs::create_dir_all(dir);
                match res {
                    Ok(_) => {
                        return Ok(ResData {
                            code: 200,
                            msg: "创建成功".to_string(),
                            data: String::new(),
                        })
                    }
                    Err(err) => {
                        return Ok(ResData {
                            code: 500,
                            msg: format!("创建失败：{0}", err.to_string()),
                            data: String::new(),
                        })
                    }
                }
            }
            return Ok(ResData {
                code: 500,
                msg: "文件夹不存在".to_string(),
                data: String::new(),
            });
        }
        Err(_) => {
            return Ok(ResData {
                code: 500,
                msg: "获取路径失败".to_string(),
                data: String::new(),
            })
        }
    }
}

/**
 * @param notebook 文件夹/笔记本
 * @param newNoteMdFileName 笔记名 文件名
 */
#[tauri::command]
pub async fn create_note_file(
    app_handle: tauri::AppHandle,
    notebook: String,
    newNoteMdFileName: String,
) -> Result<ResData<String>, String> {
    let app_dir = app_handle.path().document_dir();
    match app_dir {
        Ok(dir) => {
            let local_data_dir = dir.as_path().join("notebooks");
            if local_data_dir.exists() {
                let newNoteMdFileName_: String = newNoteMdFileName.clone();
                let name = chrono::prelude::Local::now().timestamp_millis().to_string();
                let temp_file = local_data_dir
                    .join(notebook)
                    .join(newNoteMdFileName + ".md");
                // 创建文件
                let res = std::fs::File::create(temp_file);
                match res {
                    Ok(_) => {
                        return Ok(ResData {
                            code: 200,
                            msg: "创建成功".to_string(),
                            data: newNoteMdFileName_,
                        })
                    }
                    Err(err) => {
                        return Ok(ResData {
                            code: 500,
                            msg: format!("创建失败：{0}", err.to_string()),
                            data: String::new(),
                        })
                    }
                }
            }
            return Ok(ResData {
                code: 500,
                msg: "文件夹不存在".to_string(),
                data: String::new(),
            });
        }
        Err(_) => {
            return Ok(ResData {
                code: 500,
                msg: "获取路径失败".to_string(),
                data: String::new(),
            })
        }
    }
}

/**
 * @param path 全路径名
 * @param md_title 标题/文件名
 * @param md_text 内容
 */
#[tauri::command]
pub async fn save_note(
    app_handle: tauri::AppHandle,
    path: String,
    md_title: String,
    md_text: String,
) -> Result<ResData<String>, String> {
    let path_clone = path.clone(); // 克隆path以避免所有权问题
                                   //判断文件是否存在
    match tokio::fs::metadata(path_clone).await {
        Ok(_) => {
            let wres = std::fs::write(&path, md_text);
            match wres {
                Ok(_) => {
                    //tokio::fs::rename(from, to)
                    let mut _filename = String::new();
                    let path_ = PathBuf::from(&path);
                    if path_.is_file() {
                        if let Some(filename) = path_.file_stem() {
                            _filename = filename.to_string_lossy().to_string();
                        }
                    }
                    if (_filename.to_lowercase() != md_title.to_lowercase()) {
                        println!("忽略大小写相等");

                        //将文件或目录重命名为新名称，如果 to 已存在，则替换原始文件。如果新名称位于不同的挂载点，这将不起作用。
                        let new_name_res = tokio::fs::rename(path, md_title).await;
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
                    } else {
                        return Ok(ResData {
                            code: 200,
                            msg: "操作成功".to_string(),
                            data: String::new(),
                        });
                    }
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
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            println!("文件不存在");
            return Ok(ResData {
                code: 500,
                msg: "文件不存在".to_string(),
                data: String::new(),
            });
        }
        Err(e) => {
            println!("检查文件时出错: {}", e);
            return Ok(ResData {
                code: 500,
                msg: "检查文件时出错".to_string(),
                data: String::new(),
            });
        }
    }

    // let app_dir = app_handle.path().document_dir();
    // match app_dir {
    //     Ok(dir) => {
    //         // 本地文件目录
    //         let local_data_dir = dir.as_path().join("notebooks");
    //         // 文件夹是否存在
    //         if local_data_dir.exists() {
    //             let path = local_data_dir
    //                 .join(current_notebook.clone())

    //             let new_path = local_data_dir.join(current_notebook).join(md_title + ".md");
    //             //写入文件
    //             let wres = std::fs::write(path.clone(), md_text);
    //             match wres {
    //                 Ok(_) => {
    //                     //将文件或目录重命名为新名称，如果 to 已存在，则替换原始文件。如果新名称位于不同的挂载点，这将不起作用。
    //                     let new_name_res = std::fs::rename(path, new_path);
    //                     match new_name_res {
    //                         Ok(_) => {
    //                             return Ok(ResData {
    //                                 code: 200,
    //                                 msg: "操作成功".to_string(),
    //                                 data: String::new(),
    //                             });
    //                         }
    //                         Err(e) => {
    //                             return Ok(ResData {
    //                                 code: 500,
    //                                 msg: e.to_string(),
    //                                 data: String::new(),
    //                             });
    //                         }
    //                     }
    //                 }
    //                 Err(e) => {
    //                     return Ok(ResData {
    //                         code: 500,
    //                         msg: e.to_string(),
    //                         data: String::new(),
    //                     });
    //                 }
    //             }
    //         }
    //     }
    //     Err(_) => {
    //         return Ok(ResData {
    //             code: 500,
    //             msg: "获取路径失败".to_string(),
    //             data: String::new(),
    //         })
    //     }
    // }
}

/**
 * 删除本地笔记本/文件夹
 * @param app_handle
 * @param current_notebook 文件夹
 */
#[tauri::command]
pub async fn remove_notebook(
    app_handle: tauri::AppHandle,
    current_notebook: String,
) -> Result<ResData<String>, String> {
    let app_dir = app_handle.path().document_dir();
    match app_dir {
        Ok(dir) => {
            let local_data_dir = dir.as_path().join("notebooks");
            if local_data_dir.exists() {
                let dir = local_data_dir.join(current_notebook);
                let remove_res = std::fs::remove_dir_all(dir);
                match remove_res {
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
        }
        Err(_) => {
            return Ok(ResData {
                code: 500,
                msg: "获取路径失败".to_string(),
                data: String::new(),
            })
        }
    }
    Ok(ResData {
        code: 500,
        msg: "文件夹不存在".to_string(),
        data: String::new(),
    })
}

/**
 * 删除本地笔记
 * @param current_note 笔记
 *
 */
#[tauri::command]
pub async fn remove_note(
    app_handle: tauri::AppHandle,
    current_notebook: String,
    current_note: String,
) -> Result<ResData<String>, String> {
    let app_dir = app_handle.path().document_dir();
    match app_dir {
        Ok(dir) => {
            let local_data_dir = dir.as_path().join("notebooks");
            if local_data_dir.exists() {
                let file = local_data_dir.join(current_notebook).join(current_note);
                let remove_res = std::fs::remove_file(file);
                match remove_res {
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
        }
        Err(_) => {
            return Ok(ResData {
                code: 500,
                msg: "获取路径失败".to_string(),
                data: String::new(),
            })
        }
    }
    Ok(ResData {
        code: 500,
        msg: "文件夹不存在".to_string(),
        data: String::new(),
    })
}
