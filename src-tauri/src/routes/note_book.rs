use crate::response::resp_data::{ResData, ResDataNoData};
use crate::utils::fileUtil::read_dir_info;
use std::path::{Path, PathBuf};
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
}

// 读取本地笔记文件
#[tauri::command]
pub async fn read_note_file(
    app_handle: tauri::AppHandle,
    path: String,
) -> Result<ResData<String>, String> {
    let contents = tokio::fs::read_to_string(path).await;
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

// 创建本地文件夹
#[tauri::command]
pub async fn create_folder(
    app_handle: tauri::AppHandle,
    father_path: String,
    new_folder_name: String,
) -> Result<ResData<String>, String> {
    if father_path.is_empty() {
        return Ok(ResData {
            code: 500,
            msg: "父路径为空！".to_string(),
            data: String::new(),
        });
    }
    //检查路径是否存在
    if !Path::new(&father_path).exists() {
        return Ok(ResData {
            code: 500,
            msg: "路径不存在！".to_string(),
            data: String::new(),
        });
    }
    let mut path_ = PathBuf::from(father_path);
    path_.push(new_folder_name);
    // 方法2: 递归创建目录（推荐）
    match tokio::fs::create_dir_all(path_).await {
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

/**
 * @param notebook 文件夹/笔记本
 * @param newNoteMdFileName 笔记名 文件名
 */
#[tauri::command]
pub async fn create_note_file(
    app_handle: tauri::AppHandle,
    father_path: String,
    new_note_md_file_name: String,
) -> Result<ResData<String>, String> {
    if father_path.is_empty() {
        return Ok(ResData {
            code: 500,
            msg: "父路径为空！".to_string(),
            data: String::new(),
        });
    }
    //检查路径是否存在
    if !Path::new(&father_path).exists() {
        return Ok(ResData {
            code: 500,
            msg: "选择不存在！".to_string(),
            data: String::new(),
        });
    }
    let mut path_ = PathBuf::from(father_path);
    let filename_clone = new_note_md_file_name.clone();
    path_.push(filename_clone + ".md");
    let res = std::fs::File::create(path_);
    match res {
        Ok(_) => {
            return Ok(ResData {
                code: 200,
                msg: "创建成功".to_string(),
                data: new_note_md_file_name,
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
    format!("save_note1 {}", path);
    println!("save_note1 {}", path);
    // 克隆path以避免所有权问题
    let path_clone = path.clone();
    //判断文件是否存在
    match tokio::fs::metadata(path_clone).await {
        Ok(_) => {
            let wres = std::fs::write(&path, md_text);
            match wres {
                Ok(_) => {
                    let mut _filename = String::new();
                    let path_ = PathBuf::from(&path);
                    if path_.is_file() {
                        if let Some(filename) = path_.file_stem() {
                            _filename = filename.to_string_lossy().to_string();
                        }
                    }
                    if (_filename.to_lowercase() != md_title.to_lowercase()) {
                        //将文件或目录重命名为新名称，如果 to 已存在，则替换原始文件。如果新名称位于不同的挂载点，这将不起作用。
                        format!("save_note2 {}", path);
                        println!(
                            "save_note2 {}  md_title {} _filename {}",
                            path, md_title, _filename
                        );
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
}

/**
 * 删除本地文件夹
 * @param app_handle
 * @param path 文件夹
 */
#[tauri::command]
pub async fn remove_folder(
    app_handle: tauri::AppHandle,
    path: String,
) -> Result<ResData<String>, String> {
    //检查路径是否存在
    if !Path::new(&path).exists() {
        return Ok(ResData {
            code: 500,
            msg: "选择路径不存在！".to_string(),
            data: String::new(),
        });
    }
    let mut path_ = PathBuf::from(path);
    let remove_res = std::fs::remove_dir_all(path_);
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

/**
 * 删除本地笔记
 * @param current_note 笔记
 *
 */
#[tauri::command]
pub async fn remove_file(
    app_handle: tauri::AppHandle,
    path: String,
) -> Result<ResData<String>, String> {
    //检查路径是否存在
    if !Path::new(&path).exists() {
        return Ok(ResData {
            code: 500,
            msg: "选择文件不存在！".to_string(),
            data: String::new(),
        });
    }
    let mut path_ = PathBuf::from(path);

    let remove_res = std::fs::remove_file(path_);
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

/**
 * 获取目录下所有文件及文件夹名称
 * @param app_handle
 * @param current_notebook 文件夹
 */
#[tauri::command]
pub async fn get_path_childrenFoldersAndFiles(
    app_handle: tauri::AppHandle,
    path: String,
) -> Result<ResData<Vec<String>>, String> {
    // 检查路径是否存在
    if !Path::new(&path).exists() {
        return Ok(ResData {
            code: 500,
            msg: "路径不存在".to_string(),
            data: Vec::new(),
        });
    }
    // 读取目录内容
    let entries = tokio::fs::read_dir(&path).await;
    match entries {
        Ok(entries) => {
            let mut names: Vec<String> = Vec::new();

            let mut ps = entries; // 将 ps 声明为可变变量
            while let Some(entry) = ps.next_entry().await.map_err(|e| e.to_string())? {
                let path = entry.path();

                if let Some(filename) = path.file_stem() {
                    names.push(filename.to_string_lossy().to_string());
                }
            }
            Ok(ResData {
                code: 200,
                msg: "success".to_string(),
                data: names,
            })
        }
        Err(_) => Ok(ResData {
            code: 500,
            msg: "读取目录失败".to_string(),
            data: Vec::new(),
        }),
    }
}
