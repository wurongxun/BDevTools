use std::fmt::Display;

use serde::{Deserialize, Serialize};
//use tokio_tungstenite::tungstenite::Utf8Bytes;

#[derive(Debug, Clone)]
pub enum AuthType {
    Password(String),
    PublicKey(String),
    Unknown,
}

impl Default for AuthType {
    fn default() -> Self {
        AuthType::Unknown
    }
}

#[derive(Debug, Deserialize, Clone, Serialize)]
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
        return self.path.clone();
    }
    pub fn get_folder(&mut self) -> String {
        return self.folder.clone();
    }
    pub fn get_id(&mut self) -> String {
        return self.id.clone();
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
        return self.name.clone();
    }
    pub fn get_isfile(&mut self) -> bool {
        return self.isfile;
    }
    pub fn get_dataType(&mut self) -> String {
        return self.data_type.clone();
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct MyMessage {
    pub message_type: String,
    pub data: String,
}
impl MyMessage {
    pub fn get_message(&self) -> &str {
        &self.message_type
    }

    pub fn get_data(&self) -> &str {
        &self.data
    }
    pub fn set_data(&mut self, x: String) {
        self.data = x;
    }
    // pub fn to_utf8_bytes(&self) -> Utf8Bytes {
    //     // 这里假设我们将 message_type 和 data 拼接成一个字符串
    //     let message = format!("Type: {}, Data: {}", self.message_type, self.data);
    //     Utf8Bytes::from_static(&message);
    // }
}
// //反序列化
// impl<'de> Deserialize<'de> for Message {
//     fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
//     where
//         D: serde::Deserializer<'de>,
//     {
//         // 在这里实现自定义的反序列化逻辑
//         // 使用 deserializer 提供的方法来读取输入数据并解析为 Cat 结构体

//         // 例如，假设输入数据为 JSON 格式，可以使用 serde_json 库进行反序列化
//         let cat_json: serde_json::Value = serde::Deserialize::deserialize(deserializer)?;

//         // 提取 age 和 name 字段的值
//         let message_type = cat_json["message_type"].as_i64().ok_or_else(|| serde::de::Error::custom("Missing age"))?;
//         let data = cat_json["data"].as_str().ok_or_else(|| serde::de::Error::custom("Missing name"))?;

//         // 创建 Cat 结构体并返回
//         Ok(Message {
//             message_type,
//             data: data.to_string(),
//         })
//     }
//  }
#[derive(Clone, Serialize, Deserialize)]
pub struct Payload {
    pub message: String,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct SystemInfo {
    pub memory_total: f32,
    pub memory_used: f32,
    pub cpu_used: String,
    pub hostname: String,
    pub cpus: i32,
}

///响应结构
#[derive(Serialize, Deserialize)]
pub struct Response<T> {
    pub code: i32,
    pub data: T,
    pub msg: String,
}

impl<T> Response<T> {
    pub fn new(code: i32, data: T, msg: &str) -> Response<T> {
        Response {
            code,
            data,
            msg: msg.to_string(),
        }
    }
}

/// 数据对应字段
#[derive(Serialize, Deserialize, Debug)]
pub struct SettingMap {
    pub id: String,
    pub app_setting_item_name: String,
    pub app_setting_item_value: String,
}
