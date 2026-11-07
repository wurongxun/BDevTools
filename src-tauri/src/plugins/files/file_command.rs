use crate::response::resp_data::ResData;
use std::path::{Path, PathBuf};
use chardetng::EncodingDetector;

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

/// 单次 `read_file_bytes` 的字节上限。图片几乎不会到这个量级，
/// 设上限是为了防止误把超大文件（镜像、视频、dump）整个读进内存再过一遍 IPC。
const MAX_READ_FILE_BYTES: u64 = 64 * 1024 * 1024;

/**
 * 读取文件原始字节（图片查看用）。
 *
 * 刻意**不**包 `ResData`：`ResData<Vec<u8>>` 会被序列化成 JSON 数字数组，
 * 一张 5MB 的图会变成几百万个数字，既慢又爆内存。业务层只返回裸 `Vec<u8>`，
 * 由 `commands.rs` 的 wrapper 包成 `tauri::ipc::Response` 走裸字节通道 ——
 * 前端 `invoke` 直接拿到 `ArrayBuffer`（≥1KB 时 tauri 走 fetch 通道投递，
 * 既不 base64 也不展开成 JSON 数组）。
 */
pub async fn read_file_bytes(path: String) -> Result<Vec<u8>, String> {
    let meta = tokio::fs::metadata(&path)
        .await
        .map_err(|e| format!("读取文件信息失败: {}", e))?;
    if !meta.is_file() {
        return Err(format!("不是文件: {}", path));
    }
    if meta.len() > MAX_READ_FILE_BYTES {
        return Err(format!(
            "文件过大（{} 字节），超过 {} 字节上限",
            meta.len(),
            MAX_READ_FILE_BYTES
        ));
    }
    let bytes = tokio::fs::read(&path)
        .await
        .map_err(|e| format!("读取文件失败: {}", e))?;
    Ok(bytes)
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

#[cfg(test)]
mod tests {
    use super::*;

    fn tmp_dir(name: &str) -> std::path::PathBuf {
        let d = std::env::temp_dir().join(name);
        std::fs::create_dir_all(&d).unwrap();
        d
    }

    /// 业务层必须返回**裸字节**：一旦有人改回 ResData/JSON，前端拿到的是
    /// 几百万个数字，图片直接卡死。裸字节通道本身由 commands.rs 的用例守着。
    #[tokio::test]
    async fn read_file_bytes_returns_plain_bytes() {
        let path = tmp_dir("bdevtools_read_bytes").join("sample.bin");
        std::fs::write(&path, b"\x89PNG\r\n\x1a\n\x00\x01\x02").unwrap();

        let bytes = read_file_bytes(path.to_string_lossy().to_string()).await.unwrap();
        assert_eq!(bytes, b"\x89PNG\r\n\x1a\n\x00\x01\x02".to_vec());
    }

    #[tokio::test]
    async fn read_file_bytes_rejects_directory() {
        let dir = tmp_dir("bdevtools_read_bytes");
        let res = read_file_bytes(dir.to_string_lossy().to_string()).await;
        // Response 没实现 Debug，不能用 unwrap_err()
        match res {
            Ok(_) => panic!("目录不该被当成文件读出来"),
            Err(e) => assert!(e.contains("不是文件"), "实际错误: {}", e),
        }
    }

    #[tokio::test]
    async fn read_file_bytes_rejects_missing_file() {
        let missing = tmp_dir("bdevtools_read_bytes").join("__not_exists__.bin");
        assert!(read_file_bytes(missing.to_string_lossy().to_string())
            .await
            .is_err());
    }
}
