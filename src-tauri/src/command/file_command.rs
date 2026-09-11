use crate::response::resp_data::ResData;
use std::path::{Path, PathBuf};
use chardetng::EncodingDetector;

/// 确保目录存在（递归创建）
pub async fn ensure_dir(path: String) -> Result<ResData<String>, String> {
    match tokio::fs::create_dir_all(&path).await {
        Ok(_) => Ok(ResData { code: 200, msg: "ok".to_string(), data: String::new() }),
        Err(e) => Ok(ResData { code: 500, msg: format!("创建目录失败: {}", e), data: String::new() }),
    }
}

// 获取目录下笔记
pub async fn get_note_list(path: String) -> Result<ResData<Vec<String>>, String> {
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
// ... existing code ...

// 读取本地笔记文件
pub async fn read_note_file(path: String) -> Result<ResData<String>, String> {
    // 首先读取原始字节
    let bytes = tokio::fs::read(&path)
        .await
        .map_err(|err| format!("读取文件失败: {}", err))?;

    let mut detector = EncodingDetector::new(chardetng::Iso2022JpDetection::Allow);
    detector.feed(&bytes[..], bytes.is_empty());
    let encoding = detector.guess(None, chardetng::Utf8Detection::Allow);

    // 根据检测到的编码解码文件内容
    let (cow_text, _encoding_used, _had_errors) = encoding.decode(&bytes);
    // 转换为 String
    let content = cow_text.into_owned();

    Ok(ResData {
        code: 200,
        msg: "读取成功".to_string(),
        data: content,
    })
}

// ... existing code ...

// 创建本地文件夹
pub async fn create_folder(
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
 * 获取文件大小（字节）。供前端打开文件前做"大文件分诊"（小文件走代码编辑器、大文件走分块只读查看器）。
 */
pub async fn get_file_size(path: String) -> Result<ResData<u64>, String> {
    match tokio::fs::metadata(&path).await {
        Ok(meta) => Ok(ResData {
            code: 200,
            msg: "success".to_string(),
            data: meta.len(),
        }),
        Err(e) => Ok(ResData {
            code: 500,
            msg: format!("获取文件大小失败: {}", e),
            data: 0,
        }),
    }
}

/**
 * 创建笔记文件
 * @param father_path 父文件夹路径
 * @param new_note_md_file_name 新笔记文件名
 */
pub async fn create_note_file(
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
pub async fn save_note(
    path: String,
    md_text: String,
) -> Result<ResData<String>, String> {
    //format!("save_note1 {}", path);
    println!("save_note1 {}", path);

    // 克隆path以避免所有权问题
    let path_clone = path.clone();
    //判断文件是否存在
    match tokio::fs::metadata(path_clone).await {
        Ok(_) => {
            let wres = std::fs::write(&path, md_text);
            match wres {
                Ok(_) => {
                    println!("文件已保存");
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
pub async fn up_file_name(
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
pub async fn remove_folder(
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
    let path_ = PathBuf::from(path);
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
pub async fn remove_file(
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
    let path_ = PathBuf::from(path);

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
pub async fn get_path_childrenFoldersAndFiles(
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
    let new_path = path.to_path_buf();
    // 获取目录部分
    let parent = new_path.parent().unwrap_or_else(|| Path::new(""));
    // 添加新的文件名
    let new_file_path = parent.join(format!("{}{}", new_filename, suffix));
    new_file_path.to_string_lossy().to_string()
}

/// 写入脚本文件（不存在则创建，存在则覆盖；供数据库插件查询文件管理器使用）
pub async fn write_script_file(path: String, content: String) -> Result<ResData<String>, String> {
    match tokio::fs::write(&path, content.as_bytes()).await {
        Ok(_) => Ok(ResData {
            code: 200,
            msg: "操作成功".to_string(),
            data: String::new(),
        }),
        Err(e) => Ok(ResData {
            code: 500,
            msg: format!("写入文件失败: {}", e),
            data: String::new(),
        }),
    }
}

/// 重命名脚本文件（保留原扩展名，供数据库插件查询文件管理器使用）
pub async fn rename_script_file(
    path: String,
    new_name: String,
) -> Result<ResData<String>, String> {
    let src = PathBuf::from(&path);
    if !src.exists() {
        return Ok(ResData {
            code: 500,
            msg: "选择文件不存在！".to_string(),
            data: String::new(),
        });
    }
    let extension = src
        .extension()
        .map(|e| format!(".{}", e.to_string_lossy()))
        .unwrap_or_default();
    let file_name = if new_name.ends_with(&extension) {
        new_name.clone()
    } else {
        format!("{}{}", new_name, extension)
    };
    let dest = src
        .parent()
        .unwrap_or_else(|| Path::new(""))
        .join(file_name);
    match tokio::fs::rename(&src, &dest).await {
        Ok(_) => Ok(ResData {
            code: 200,
            msg: "操作成功".to_string(),
            data: dest.to_string_lossy().to_string(),
        }),
        Err(e) => Ok(ResData {
            code: 500,
            msg: e.to_string(),
            data: String::new(),
        }),
    }
}

/// 脚本文件树节点（供数据库插件查询文件管理器使用）
#[derive(serde::Serialize)]
pub struct ScriptFileEntry {
    pub name: String,
    pub path: String,
    pub is_dir: bool,
    pub children: Vec<ScriptFileEntry>,
}

/// 递归收集目录下的文件/文件夹（目录优先、按名称排序）
async fn collect_script_entries(dir: &Path) -> Result<Vec<ScriptFileEntry>, String> {
    let mut entries: Vec<ScriptFileEntry> = Vec::new();
    let mut read_dir = tokio::fs::read_dir(dir).await.map_err(|e| e.to_string())?;
    while let Some(entry) = read_dir.next_entry().await.map_err(|e| e.to_string())? {
        let p = entry.path();
        let is_dir = p.is_dir();
        let children = if is_dir {
            Box::pin(collect_script_entries(&p)).await?
        } else {
            Vec::new()
        };
        entries.push(ScriptFileEntry {
            name: entry.file_name().to_string_lossy().to_string(),
            path: p.to_string_lossy().to_string(),
            is_dir,
            children,
        });
    }
    entries.sort_by(|a, b| {
        b.is_dir
            .cmp(&a.is_dir)
            .then_with(|| a.name.to_lowercase().cmp(&b.name.to_lowercase()))
    });
    Ok(entries)
}

/// 递归列出目录下所有文件与文件夹（数据库插件查询文件管理器）
pub async fn list_script_files(path: String) -> Result<ResData<Vec<ScriptFileEntry>>, String> {
    let dir = PathBuf::from(&path);
    if !dir.exists() || !dir.is_dir() {
        return Ok(ResData {
            code: 500,
            msg: "路径不存在或不是目录".to_string(),
            data: Vec::new(),
        });
    }
    match collect_script_entries(&dir).await {
        Ok(entries) => Ok(ResData {
            code: 200,
            msg: "success".to_string(),
            data: entries,
        }),
        Err(e) => Ok(ResData {
            code: 500,
            msg: format!("读取目录失败: {}", e),
            data: Vec::new(),
        }),
    }
}
