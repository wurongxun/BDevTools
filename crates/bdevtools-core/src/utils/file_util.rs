use serde::{Deserialize, Serialize};
use std::{
    error::Error,
    fs::{metadata, read_dir},
    path::{Path, PathBuf},
};

use super::regex_util;
use crate::response::resp_data::ResData;

/// 目录与文件信息（仅文件模块使用）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DirAndFileInfo {
    pub id: String,
    pub parent_id: String,
    pub path: String,
    pub folder: String,
    pub name: String,
    pub isfile: bool,
    pub data_type: String,
    pub children: Vec<DirAndFileInfo>,
}

impl DirAndFileInfo {
    pub fn create(
        id: String,
        parent_id: String,
        path: String,
        folder: String,
        name: String,
        isfile: bool,
        data_type: String,
    ) -> DirAndFileInfo {
        DirAndFileInfo {
            id,
            parent_id,
            path,
            folder,
            name,
            isfile,
            data_type,
            children: Vec::new(),
        }
    }

    pub fn set_children(&mut self, children: DirAndFileInfo) {
        self.children.push(children);
    }

    pub fn get_path(&self) -> String {
        self.path.clone()
    }

    pub fn get_id(&self) -> String {
        self.id.clone()
    }
}

/**
 *
 * 递归获取目录下全部文件以及文件夹（最大层级限制）
 * num 编号
 * dir_info 父文件夹信息
 * max_depth 最大递归深度
 * current_depth 当前递归深度
 */
pub fn get_dir_and_file_with_depth(
    num: &mut u32,
    dir_info: &mut DirAndFileInfo,
    max_depth: usize,
    current_depth: usize,
) -> Result<ResData<String>, Box<dyn Error>> {
    // 限制目录层级最多5层
    if current_depth > max_depth {
        return Ok(ResData {
            code: 200,
            msg: "达到最大目录层级限制".to_string(),
            data: String::new(),
        });
    }

    let path_ = dir_info.get_path();
    if metadata(&path_)?.is_dir() {
        for child_dir in read_dir(&path_)? {
            let path_dir_entry = child_dir?.path();
            let path_1 = path_dir_entry.clone();
            *num += 1;
            // 获取文件或目录名
            let name = path_dir_entry
                .file_name()
                .map_or(String::new(), |n| n.to_string_lossy().to_string());
            // 判断文件后缀（md/pdf/txt 作为文件节点）
            let is_md = regex_util::validate(r#"(?:md)$"#, &name);
            let is_pdf = regex_util::validate(r#"(?:pdf)$"#, &name);
            let is_txt = regex_util::validate(r#"(?:txt)$"#, &name);
            let path = path_dir_entry.to_string_lossy().to_string();
            let folder = get_directory_path(&path).unwrap_or_default();
            let md5 = generate_md5_hash(&path);

            let data_type = if is_md {
                "md".to_string()
            } else if is_pdf {
                "pdf".to_string()
            } else if is_txt {
                "txt".to_string()
            } else {
                "folder".to_string()
            };
            let mut child_info = DirAndFileInfo::create(
                md5,
                dir_info.get_id(),
                path.clone(),
                folder,
                name,
                path_dir_entry.is_file(),
                data_type,
            );
            if !path_1.is_file() {
                // 递归处理子目录，增加层级限制
                get_dir_and_file_with_depth(num, &mut child_info, max_depth, current_depth + 1)?;
            }
            dir_info.set_children(child_info);
        }
        Ok(ResData {
            code: 200,
            msg: "成功".to_string(),
            data: String::new(),
        })
    } else {
        Ok(ResData {
            code: 500,
            msg: "文件夹不存在".to_string(),
            data: String::new(),
        })
    }
}

/**
 * 获取目录下所有文件信息
 */
pub fn get_dir_notebooks_data(root_path: &String) -> ResData<String> {
    let id: String = chrono::prelude::Local::now().timestamp_millis().to_string();
    let folder = get_directory_path(root_path).unwrap();
    let mut name = "";
    #[cfg(target_os = "windows")]
    let pos: Vec<&str> = Path::new(&folder)
        .as_os_str()
        .to_str()
        .expect("")
        .split("\\")
        .collect();
    #[cfg(any(target_os = "linux", target_os = "macos"))]
    let pos: Vec<&str> = Path::new(&folder)
        .as_os_str()
        .to_str()
        .expect("")
        .split("/")
        .collect();
    if !pos.is_empty() {
        name = pos[pos.len() - 1];
    }

    let mut dir_info = DirAndFileInfo::create(
        id,
        "".to_string(),
        root_path.to_string(),
        folder.clone(),
        name.to_string(),
        false,
        "folder".to_string(),
    );
    let mut i: u32 = 0;
    //最大层级5 当前深度0
    let _ = get_dir_and_file_with_depth(&mut i, &mut dir_info, 5, 0);
    // println!(
    //     "get_dir_notebooks_data {}",
    //     serde_json::to_string(&dir_info).unwrap()
    // );
    ResData {
        code: 200,
        msg: "OK".to_string(),
        data: serde_json::to_string(&dir_info).unwrap(),
    }
}
/**
 * 判断字符串是文件路径还是文件夹路径
 * 如果是文件夹路径直接返回
 * 如果是文件路径则返回文件所在文件夹路径
 */
pub fn get_directory_path(path: &str) -> Option<String> {
    match metadata(path) {
        Ok(metadata) => {
            if metadata.is_dir() {
                // 如果是文件夹，直接返回原路径
                Some(path.to_string())
            } else {
                // 如果是文件，返回所在文件夹路径
                let path_obj = Path::new(path);
                if let Some(parent) = path_obj.parent() {
                    Some(parent.to_string_lossy().to_string())
                } else {
                    Some(".".to_string())
                }
            }
        }
        Err(_) => None,
    }
}

/**
 * 获取默认文档目录数据
 * @param document_dir 系统文档目录（不可用时返回失败）
 */
pub fn default_document_data(document_dir: Option<std::path::PathBuf>) -> ResData<String> {
    match document_dir {
        Some(app_dir_) => {
            let path_op = app_dir_.to_str();
            match path_op {
                Some(path) => {
                    return md_notebooks(path);
                }
                None => {
                    return ResData {
                        code: 500,
                        msg: "读取文档目录失败".to_string(),
                        data: String::new(),
                    };
                }
            }
        }
        None => {
            return ResData {
                code: 500,
                msg: "读取文档目录失败".to_string(),
                data: String::new(),
            };
        }
    }
}

pub fn md_notebooks(path_parameter: &str) -> ResData<String> {
    let dir = PathBuf::from(path_parameter);
    let local_data_dir = dir.clone();
    let local_data_dir_ = dir.clone();

    // 文件夹是否存在
    if local_data_dir.exists() {
        return get_dir_notebooks_data(&local_data_dir.as_os_str().to_str().unwrap().to_string());
    }

    // 目录不存在则创建
    let _dir = std::fs::create_dir_all(local_data_dir);
    match _dir {
        Ok(_) => {
            return get_dir_notebooks_data(
                &local_data_dir_.as_os_str().to_str().unwrap().to_string(),
            )
        }
        Err(_) => {
            return ResData {
                code: 500,
                msg: "读取文档目录失败".to_string(),
                data: String::new(),
            };
        }
    }
}

/// 生成字符串的 MD5 哈希值
pub fn generate_md5_hash(input: &str) -> String {
    let digest = md5::compute(input);
    hex::encode(digest.as_ref())
}
