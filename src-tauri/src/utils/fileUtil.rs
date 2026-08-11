use serde::{Deserialize, Serialize};
use std::{
    error::Error,
    fs::{metadata, read_dir},
    path::{Path, PathBuf},
};

use super::regexUtil;
use crate::response::{resp_data::ResData, response::Response};
use tauri::{AppHandle, Manager};

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
    pub fn get_path(&mut self) -> String {
        self.path.clone()
    }
    pub fn get_folder(&mut self) -> String {
        self.folder.clone()
    }
    pub fn get_id(&mut self) -> String {
        self.id.clone()
    }
    pub fn set_parent_id(&mut self, parent_id: String) {
        self.parent_id = parent_id;
    }
    pub fn set_folder(&mut self, folder: String) {
        self.folder = folder;
    }
    pub fn set_name(&mut self, name: String) {
        self.name = name;
    }
    pub fn set_isfile(&mut self, isfile: bool) {
        self.isfile = isfile;
    }
    pub fn set_dataType(&mut self, dataType: String) {
        self.data_type = dataType;
    }
    pub fn get_name(&mut self) -> String {
        self.name.clone()
    }
    pub fn get_isfile(&mut self) -> bool {
        self.isfile
    }
    pub fn get_dataType(&mut self) -> String {
        self.data_type.clone()
    }
}

pub fn read_dir_info<P: AsRef<Path>>(local_data_dir: P) -> Result<ResData<Vec<String>>, String> {
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
        Err(_) => return Err("读取目录失败".to_string()),
    }
}

/**
 *
 * 获取目录下全部文件以及文件夹
 * num 编号
 * children_dirAndFileInfo 父文件夹信息
 */
pub fn get_dir_and_file(
    num: &mut u32,
    children_dirAndFileInfo: &mut DirAndFileInfo,
) -> Result<ResData<String>, Box<dyn Error>> {
    let path_ = children_dirAndFileInfo.get_path();
    if metadata(&path_)?.is_dir() {
        for child_dir in read_dir(&path_)? {
            let path_dir_entry = child_dir?.path();
            let path_1 = path_dir_entry.clone();
            *num += 1;
            // 获取文件或目录名
            let name = path_dir_entry
                .file_name()
                .map_or(String::new(), |n| n.to_string_lossy().to_string());
            // 判断文件后缀 是md或者是PDF 文件才加入
            let is_md = regexUtil::validate(r#"(?:md)$"#, &name);
            let is_pdf = regexUtil::validate(r#"(?:pdf)$"#, &name);
            let is_txt = regexUtil::validate(r#"(?:txt)$"#, &name);
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
            let mut children_dir_and_file_info = DirAndFileInfo::create(
                md5,
                children_dirAndFileInfo.get_id(),
                path.clone(),
                folder,
                name,
                path_dir_entry.is_file(),
                data_type,
            );
            if !path_1.is_file() {
                // 递归处理子目录
                get_dir_and_file(num, &mut children_dir_and_file_info)?;
                children_dirAndFileInfo.set_children(children_dir_and_file_info);
            } else if (is_md || is_pdf) && path_1.is_file() {
                // 只添加 md 或 pdf 文件
                children_dirAndFileInfo.set_children(children_dir_and_file_info);
            }
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
 *
 * 获取目录下全部文件以及文件夹
 * num 编号
 * children_dirAndFileInfo 父文件夹信息
 * max_depth 最大递归深度
 * current_depth 当前递归深度
 */
pub fn get_dir_and_file_with_depth(
    num: &mut u32,
    children_dirAndFileInfo: &mut DirAndFileInfo,
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

    let path_ = children_dirAndFileInfo.get_path();
    if metadata(&path_)?.is_dir() {
        for child_dir in read_dir(&path_)? {
            let path_dir_entry = child_dir?.path();
            let path_1 = path_dir_entry.clone();
            *num += 1;
            // 获取文件或目录名
            let name = path_dir_entry
                .file_name()
                .map_or(String::new(), |n| n.to_string_lossy().to_string());
            // 判断文件后缀 是md或者是PDF 文件才加入
            let is_md = regexUtil::validate(r#"(?:md)$"#, &name);
            let is_pdf = regexUtil::validate(r#"(?:pdf)$"#, &name);
            let is_txt = regexUtil::validate(r#"(?:txt)$"#, &name);
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
            let mut children_dir_and_file_info = DirAndFileInfo::create(
                md5,
                children_dirAndFileInfo.get_id(),
                path.clone(),
                folder,
                name,
                path_dir_entry.is_file(),
                data_type,
            );
            if !path_1.is_file() {
                // 递归处理子目录，增加层级限制
                get_dir_and_file_with_depth(
                    num,
                    &mut children_dir_and_file_info,
                    max_depth,
                    current_depth + 1,
                )?;
                children_dirAndFileInfo.set_children(children_dir_and_file_info);
            } else if  path_1.is_file() {
                // 只添加 md 或 pdf 文件
                children_dirAndFileInfo.set_children(children_dir_and_file_info);
            }
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
    if pos.len() > 0 {
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
    //let _ = get_dir_and_file(&mut i, &mut dir_info);
    //最大层级5 当前深度0
    let _ = get_dir_and_file_with_depth(&mut i, &mut dir_info, 5, 0);
    println!(
        "get_dir_notebooks_data {}",
        serde_json::to_string(&dir_info).unwrap()
    );
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
 */
pub fn default_document_data(app_handle: tauri::AppHandle) -> ResData<String> {
    let app_dir = app_handle.path().document_dir();
    match app_dir {
        Ok(app_dir_) => {
            let path_op = app_dir_.to_str();
            match path_op {
                Some(path) => {
                    return md_notebooks(&app_handle, path);
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
        Err(err) => {
            return ResData {
                code: 500,
                msg: "读取文档目录失败".to_string(),
                data: String::new(),
            };
        }
    }
}
pub fn md_notebooks(app: &AppHandle, path_parameter: &str) -> ResData<String> {
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
// 生成字符串的MD5哈希值
pub fn generate_md5_hash(input: &str) -> String {
    let digest = md5::compute(input);
    hex::encode(digest.as_ref())
}
