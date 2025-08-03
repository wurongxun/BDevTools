use std::{
    error::Error,
    fs::{metadata, read_dir},
    path::Path,
};

use crate::{
    entity::structs::DirAndFileInfo,
    response::{resp_data::ResData, response::Response}, routes::file_utils::generate_md5_hash,
};

use super::regexUtil;

pub fn all_path(root_path: &String) -> Result<Vec<String>, Box<dyn Error>> {
    let id = chrono::prelude::Local::now().timestamp_millis().to_string();
    let folder = get_directory_path(&root_path).unwrap();
    let mut children_dir_and_file_info_1 = DirAndFileInfo::create(
        id,
        "".to_string(),
        root_path.to_string(),
        folder,
        "notebooks".to_string(),
        false,
    );
    let mut i: u32 = 0;
    get_dir_and_file(&mut i, &mut children_dir_and_file_info_1);
    let sdata = serde_json::to_string(&children_dir_and_file_info_1);
    if sdata.is_err() {
        println!(
            "Error, failed to serialize structure: {}",
            sdata.unwrap_err()
        );
    } else {
        let sdata2 = sdata.unwrap();
        println!("Serialized data:\n{}", sdata2);
    }

    println!("all_path:  {}", root_path);
    let mut path_list = vec![String::from(root_path)];
    let mut start_index = 0;
    loop {
        let list_len = path_list.len();
        println!("all_path list_len  {}", list_len);
        for index in start_index..path_list.len() {
            let path = &path_list[index];
            println!("all_path path {} - {}", index, path);
            if metadata(path)?.is_dir() {
                for child_dir in read_dir(&path)? {
                    println!("all_path path child_dir - {:?}", child_dir);
                    path_list.push(String::from(
                        child_dir?.path().as_os_str().to_str().expect(""),
                    ));
                }
            }
        }
        if list_len == start_index {
            break;
        }
        start_index = list_len;
    }
    println!("all_path path path_list - {:?}", path_list);
    return Ok(path_list);
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
    Ok(ResData {
        code: 500,
        msg: "读取文档目录失败".to_string(),
        data: Vec::new(),
    })
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
    let path_2 = path_.clone();

    if metadata(path_)?.is_dir() {
        for child_dir in read_dir(&path_2)? {
            let path_dir_entry = child_dir?.path();
            let path_1 = path_dir_entry.clone();
            let mut st: String = num.clone().to_string();

            let id: String =
                chrono::prelude::Local::now().timestamp_millis().to_string() + "-" + &st;
            let mut name = "";
            #[cfg(target_os = "windows")]
            let pos: Vec<&str> = path_dir_entry
                .as_os_str()
                .to_str()
                .expect("")
                .split("\\")
                .collect();
            #[cfg(any(target_os = "linux", target_os = "macos"))]
            let pos: Vec<&str> = path_dir_entry
                .as_os_str()
                .to_str()
                .expect("")
                .split("/")
                .collect();
            if pos.len() > 0 {
               name = pos[pos.len() - 1];
            }
            //  let mut full_name = "";
            //  if let Some(last_dot) = name.rfind('.') {
            //         full_name = &name[..last_dot];
            //     } else {
            //         full_name = name;
            //     }
            let a: u32 = 1;
            let c: u32 = num.clone();
            let mut d = a + c;
            *num = d;
            let b: bool = regexUtil::validate(r#"(?:md)$"#, name);
            let path = String::from(path_dir_entry.as_os_str().to_str().expect(""));
            let folder = get_directory_path(&path).unwrap();
            let md5=generate_md5_hash(&path);
            let mut children_dir_and_file_info_3 = DirAndFileInfo::create(
                String::from(md5),
                children_dirAndFileInfo.get_id(),
                path,
                folder,
                String::from(name),
                path_dir_entry.is_file(),
            );
            if (!path_1.is_file()) {
                get_dir_and_file(num, &mut children_dir_and_file_info_3);
                children_dirAndFileInfo.set_children(children_dir_and_file_info_3);
            } else if b && path_1.is_file() {
                children_dirAndFileInfo.set_children(children_dir_and_file_info_3);
            }
        }
    }
    Ok(ResData {
        code: 500,
        msg: "文件夹不存在".to_string(),
        data: String::new(),
    })
}

pub fn get_document_notebooks_data(root_path: &String) -> ResData<String> {
    let id: String = chrono::prelude::Local::now().timestamp_millis().to_string();
    let folder = get_directory_path(root_path).unwrap();
    let mut children_dir_and_file_info_1 = DirAndFileInfo::create(
        id,
        "".to_string(),
        root_path.to_string(),
        folder,
        "notebooks".to_string(),
        false,
    );
    let mut i: u32 = 0;
    get_dir_and_file(&mut i, &mut children_dir_and_file_info_1);
    println!(
        "get_document_notebooks_data {}",
        serde_json::to_string(&children_dir_and_file_info_1).unwrap()
    );
    ResData {
        code: 200,
        msg: "OK".to_string(),
        data: serde_json::to_string(&children_dir_and_file_info_1).unwrap(),
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
