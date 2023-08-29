use anyhow::Error;
use serde::{Deserialize,Serialize};

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

#[derive(Debug,Deserialize,Clone,Serialize)]
pub struct DirAndFileInfo {
    pub id:String,
    pub parent_id:String,
    pub path: String,
    pub name: String,
    pub isfile: bool,
    pub children: Vec<DirAndFileInfo> ,
}

impl DirAndFileInfo {
    pub fn create(id:String,parent_id:String,path: String,name: String, isfile: bool) -> DirAndFileInfo {
        DirAndFileInfo {id,parent_id, path,name, isfile, children: Vec::new() }
    }
    pub fn set_children(&mut self,children:DirAndFileInfo) {
        self.children.push(children);
    }
    pub fn get_path(&mut self)->String {
        return self.path.clone();
    }
    pub fn get_id(&mut self)->String {
        return self.id.clone();
    }
    pub fn set_parent_id(&mut self,parent_id:String){
        self.parent_id=parent_id;
    }
}

