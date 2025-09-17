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
    // 递归创建目录
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
    //format!("save_note1 {}", path);
    println!("save_note1 {}", path);
    //判断文件名是否为空
    if md_title.is_empty() {
        return Ok(ResData {
            code: 500,
            msg: "文件名不能为空！".to_string(),
            data: String::new(),
        });
    }

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
                    // 替换原来的比较代码
                    let filename_lower = _filename.to_lowercase();
                    let title_lower = md_title.to_lowercase();
                    println!(
                        "save_note2 path {}  md_title {} _filename {}  比较结果 {}",
                        path,
                        md_title,
                        _filename,
                        _filename.to_lowercase() != md_title.to_lowercase()
                    );
                    let bb: bool = _filename.to_lowercase() != md_title.to_lowercase();

                    if bb {
                        let mut full_name = "";
                        if let Some(last_dot) = md_title.rfind('.') {
                            full_name = &md_title[..last_dot];
                        } else {
                            full_name = &md_title;
                        }
                        let new_file_path = rename_file_path(&path, full_name, ".md");
                        let new_name_res = tokio::fs::rename(path, new_file_path).await;
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
 * 更新文件名
 */
#[tauri::command]
pub async fn up_file_name(
    app_handle: tauri::AppHandle,
    path: String,
    up_file_name: String,
) -> Result<ResData<String>, String> {
    println!("up_file_name {},--------{}", path, up_file_name);
    //检查路径是否存在
    if !Path::new(&path).exists() {
        return Ok(ResData {
            code: 500,
            msg: "选择文件不存在！".to_string(),
            data: String::new(),
        });
    }
    let new_file_path = rename_file_path(&path, &up_file_name, ".md");
    println!("new_file_path {}", new_file_path);
    let new_name_res = tokio::fs::rename(&path, &new_file_path).await;
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
fn rename_file_path(original_path: &str, new_filename: &str, suffix: &str) -> String {
    let path = Path::new(original_path);
    let mut new_path = path.to_path_buf();
    // 获取目录部分
    let parent = new_path.parent().unwrap_or_else(|| Path::new(""));
    // 添加新的文件名
    let new_file_path = parent.join(format!("{}{}", new_filename, suffix));
    new_file_path.to_string_lossy().to_string()
}
