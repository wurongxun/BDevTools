/*! 通用文件系统操作（**核心能力，不属于任何插件**）

这里只放「任何插件都可能用到、且不依赖插件业务模型」的纯 fs 操作。

为什么单独抽出来：这些能力原本放在 `plugins/files/file_command.rs`，但 `database`
插件（外部脚本目录）也要建目录。如果它们属于 `plugin-files` feature，那么
「只打包 database」的构建里 `create_folder` 命令会不存在 —— 一个纯 `create_dir_all`
的通用能力不该被某个插件的开关绑住。按架构约定（通用能力留在顶层 `utils/`）下沉到此处。
*/
use crate::core::ctx::AppContext;
use crate::response::resp_data::ResData;
use std::path::{Path, PathBuf};

/// 在系统文件管理器中打开路径（**核心能力**）。
///
/// 任何插件都可能要「打开所在目录」（第三方插件目录设置就用它），
/// 不能因为 files 插件被裁就让这个能力消失 —— 它只是 `AppContext::open_path` 的一层封装。
pub async fn open_path_in_file_manager(ctx: &AppContext, path: String) -> ResData<String> {
    match ctx.open_path(&path) {
        Ok(_) => ResData {
            code: 200,
            msg: "打开项目成功".to_string(),
            data: "".to_string(),
        },
        Err(e) => ResData {
            code: 500,
            msg: e,
            data: "".to_string(),
        },
    }
}

/// 确保目录存在（递归创建）
pub async fn ensure_dir(path: String) -> Result<ResData<String>, String> {
    match tokio::fs::create_dir_all(&path).await {
        Ok(_) => Ok(ResData {
            code: 200,
            msg: "ok".to_string(),
            data: String::new(),
        }),
        Err(e) => Ok(ResData {
            code: 500,
            msg: format!("创建目录失败: {}", e),
            data: String::new(),
        }),
    }
}

/// 在指定父目录下新建文件夹（父目录必须已存在）
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
