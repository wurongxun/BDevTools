/*! # 工作区 数据模型 */
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// 工作区配置
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct WorkspaceConfig {
    pub id: String,
    pub name: String,
    pub folders: Vec<String>,
    pub settings: HashMap<String, serde_json::Value>,
    pub created_at: i64,
    pub updated_at: i64,
}

/// 工作区列表项（摘要，不含 settings）
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct WorkspaceSummary {
    pub id: String,
    pub name: String,
    pub folders: Vec<String>,
    pub updated_at: i64,
}
