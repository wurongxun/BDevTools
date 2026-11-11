//! # 平台无关的应用上下文
//!
//! 业务模块一律通过 [`AppContext`] 获取平台能力（路径、事件、系统调用），
//! **不直接依赖 tauri**。`tauri::AppHandle` 只在 `crate::commands` 门面层
//! 被转换为 [`AppContext`]。
//!
//! 将来更换 UI 框架：只需重写 `commands/mod.rs` 里的适配函数，
//! 所有业务模块零改动。

use std::path::PathBuf;
use std::sync::Arc;

/// 平台无关的应用上下文
#[derive(Clone)]
pub struct AppContext {
    /// 应用数据目录（业务数据：workspace / session / 数据库等）
    pub data_dir: PathBuf,
    /// 配置目录（settings.json / openfile.json 等，对应 tauri 的 app_config_dir）
    pub config_dir: PathBuf,
    /// 系统文档目录（笔记默认位置，可能不可用）
    pub document_dir: Option<PathBuf>,
    emitter: Option<Arc<dyn Fn(&str, serde_json::Value) + Send + Sync>>,
    opener: Option<Arc<dyn Fn(&str) -> Result<(), String> + Send + Sync>>,
    notifier: Option<Arc<dyn Fn(&str) + Send + Sync>>,
    autostart: Option<Arc<dyn Fn(bool) + Send + Sync>>,
}

impl AppContext {
    /// 仅凭数据目录创建上下文（其余能力后续通过 with_* 注入）
    pub fn new(data_dir: PathBuf) -> Self {
        Self {
            data_dir,
            config_dir: PathBuf::new(),
            document_dir: None,
            emitter: None,
            opener: None,
            notifier: None,
            autostart: None,
        }
    }

    /// 注入配置目录（settings.json / openfile.json 所在目录）
    pub fn with_config_dir(mut self, dir: PathBuf) -> Self {
        self.config_dir = dir;
        self
    }

    pub fn with_document_dir(mut self, dir: Option<PathBuf>) -> Self {
        self.document_dir = dir;
        self
    }

    /// 注入事件发射器（进程内广播，如 `ssh-output` / `transfer-progress`）
    pub fn with_emitter<E>(mut self, e: E) -> Self
    where
        E: Fn(&str, serde_json::Value) + Send + Sync + 'static,
    {
        self.emitter = Some(Arc::new(e));
        self
    }

    /// 注入路径打开器（用系统默认程序打开文件/目录/URL）
    pub fn with_opener<O>(mut self, o: O) -> Self
    where
        O: Fn(&str) -> Result<(), String> + Send + Sync + 'static,
    {
        self.opener = Some(Arc::new(o));
        self
    }

    /// 注入系统通知器
    pub fn with_notifier<N>(mut self, n: N) -> Self
    where
        N: Fn(&str) + Send + Sync + 'static,
    {
        self.notifier = Some(Arc::new(n));
        self
    }

    /// 注入开机自启设置器（set-and-forget）
    pub fn with_autostart<A>(mut self, a: A) -> Self
    where
        A: Fn(bool) + Send + Sync + 'static,
    {
        self.autostart = Some(Arc::new(a));
        self
    }

    /// 广播事件（无订阅者时静默忽略）
    pub fn emit(&self, event: &str, payload: serde_json::Value) {
        if let Some(emitter) = &self.emitter {
            emitter(event, payload);
        }
    }

    /// 用系统默认程序打开路径/URL
    pub fn open_path(&self, path: &str) -> Result<(), String> {
        match &self.opener {
            Some(opener) => opener(path),
            None => Err("打开能力未注入".to_string()),
        }
    }

    /// 发送系统通知（权限不足或未注入时静默忽略）
    pub fn notify(&self, body: &str) {
        if let Some(notifier) = &self.notifier {
            notifier(body);
        }
    }

    /// 设置开机自启
    pub fn set_autostart(&self, enabled: bool) {
        if let Some(autostart) = &self.autostart {
            autostart(enabled);
        }
    }
}
