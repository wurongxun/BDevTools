//! # 本地交互式终端（PTY）
//!
//! 本模块是 terminal 插件的后端：为前端（xterm.js）提供真实的伪终端会话。
//! **没有网络层**——PTY 直接落在本机，跨平台由 `portable-pty` 统一（Windows 走 ConPTY，
//! Unix 走 openpty），读写 / resize / kill / 进程退出都不需要平台相关的 unsafe 代码。
//!
//! ## 数据流
//! ```text
//! 前端按键 ──terminal_write──▶ PTY master ──▶ shell 进程
//!                                                 │
//! 前端 xterm ◀──Channel(TermEvent::Data)◀─ 读线程 ◀─┘
//! ```
//!
//! ## 与框架的边界
//! 本模块**不依赖任何 tauri 类型**：输出经 [`TermSink`]（一个中性闭包）推给调用方，
//! `commands.rs` 负责把 `tauri::ipc::Channel` 适配成这个闭包。
//!
//! ## 转义序列解析放在前端
//! OSC 7（cwd）/ OSC 0-2（标题）/ OSC 133（命令边界 + 退出码）**不在后端解析**：
//! xterm.js 自带 `parser.registerOscHandler`，由前端注册处理器，避免在 Rust 侧再写一遍
//! 需要跨读块拼接状态机的 OSC 解析器。后端只负责把原始字节原样透传。

use std::collections::HashMap;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread;

use base64::Engine;
use parking_lot::Mutex;
use portable_pty::{native_pty_system, ChildKiller, CommandBuilder, MasterPty, PtySize};
use serde::{Deserialize, Serialize};

use crate::response::response::Response;

/// 单次读取的最大字节数（一次读取 = 一个 `Data` 事件）
const READ_BUF_SIZE: usize = 16 * 1024;

/// 输出流编码器（PTY 吐任意字节，含跨块截断的 UTF-8；base64 原样透传给前端）
const B64: base64::engine::general_purpose::GeneralPurpose = base64::engine::general_purpose::STANDARD;

/// 事件推送出口。由调用方（`commands.rs`）把 `tauri::ipc::Channel` 包成闭包注入，
/// 使本模块不感知任何 UI 框架类型。
pub type TermSink = Arc<dyn Fn(TermEvent) + Send + Sync + 'static>;

// ==================== 事件与数据结构 ====================

/// 后端 → 前端的事件（经 Tauri v2 Channel 推送，带序号保证顺序）
#[derive(Debug, Clone, Serialize)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum TermEvent {
    /// 终端输出：`data` 是 **base64** 编码的原始字节。
    /// 前端解码成 `Uint8Array` 直接喂给 `term.write()`，
    /// 避免在分块边界把多字节字符切断导致乱码。
    Data { data: String },
    /// 子进程退出（唯一一次），`code` 为退出码；被信号终止时 `signal` 有值
    Exit {
        code: Option<i32>,
        signal: Option<String>,
    },
    /// 会话级错误（读线程异常等），不影响已有输出
    Error { message: String },
}

/// 打开会话的入参（前端 → 后端；字段名与 TS 侧 camelCase 对齐）
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TermOpenOptions {
    /// 初始列数（由前端 FitAddon 量出）
    pub cols: u16,
    /// 初始行数
    pub rows: u16,
    /// 工作目录，缺省用 `$HOME`（不存在时回退到进程当前目录）
    pub cwd: Option<String>,
    /// 指定 shell 可执行文件路径；缺省按平台挑默认 shell
    pub shell: Option<String>,
    /// 追加参数（拼在 shell 默认参数之后）
    pub args: Option<Vec<String>>,
    /// 额外环境变量
    pub env: Option<HashMap<String, String>>,
    /// 关闭 shell 集成注入（排障用：集成脚本出问题时可用纯 shell 验证 PTY 通路）
    #[serde(default)]
    pub no_integration: bool,
}

/// 会话建立结果
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TermOpenResult {
    pub session_id: String,
    /// 实际启动的 shell 可执行文件路径
    pub shell: String,
    /// shell 类型（powershell / pwsh / cmd / bash / zsh / fish / nu / unknown）
    pub shell_kind: String,
    /// 实际工作目录
    pub cwd: String,
    /// 子进程 pid（拿不到时为 None）
    pub pid: Option<u32>,
    /// shell 集成程度：
    /// - `full`：注入了 OSC 133（A/B/C/D + 退出码）与 OSC 7（cwd）→ 命令边界/退出码/cwd 全可用
    /// - `partial`：只有 OSC 133;A/B 与 OSC 7（如 cmd.exe 拿不到退出码）
    /// - `none`：未注入（`noIntegration`、或该 shell 无集成脚本）
    pub integration: String,
}

/// 可选 shell 描述（前端「新建会话」下拉用）
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ShellInfo {
    pub id: String,
    pub label: String,
    pub path: String,
    pub kind: String,
    /// 是否为本平台默认 shell
    pub is_default: bool,
    /// 该 shell 能拿到的 shell 集成程度：`full`（命令边界 + 退出码 + cwd）
    /// / `partial`（只有命令边界 + cwd，如 cmd.exe）
    /// / `none`（未注入，如 WSL、fish —— WSL 内部跑什么 shell 未知，注不进去）
    pub integration: String,
}

// ==================== shell 类型判定 ====================

/// 由可执行文件名推导 shell 类型（纯函数，便于单测）
pub fn detect_shell_kind(shell_path: &str) -> &'static str {
    let name = Path::new(shell_path)
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or(shell_path)
        .to_ascii_lowercase();
    match name.as_str() {
        "pwsh" | "pwsh-preview" => "pwsh",
        "powershell" => "powershell",
        "cmd" => "cmd",
        "bash" => "bash",
        "zsh" => "zsh",
        "fish" => "fish",
        "nu" => "nu",
        "sh" | "dash" => "sh",
        // WSL 是「Windows 侧的启动器」，内部跑哪个发行版/shell 直到运行起来才知道，
        // 归为独立类型：不能当 unknown（要在 UI 上正确标为「无集成」并给出解释）
        "wsl" => "wsl",
        _ => "unknown",
    }
}

/// 该 shell 类型**能拿到**的 shell 集成程度（`integration_plan()` 的实际产出等级）。
///
/// 注意别写成「要么完整、要么部分」的二元判断：cmd.exe 的 PROMPT 里取不到 errorlevel，
/// 只能算 partial；WSL / fish / nu / sh 完全没有注入，是 none。
/// 早先 `ShellInfo` 用的是 `full_integration: bool`，于是「非 full」一律被 UI 显示成
/// 「部分集成」——cmd 被误标成完整集成、WSL 被误标成部分集成，两个方向都错。
pub fn shell_integration_level(kind: &str) -> &'static str {
    if kind_has_full_integration(kind) {
        "full"
    } else if kind == "cmd" {
        "partial"
    } else {
        "none"
    }
}

/// 该 shell 类型能否拿到「退出码」（OSC 133;D）。
/// cmd.exe 的 PROMPT 里取不到 errorlevel，只能做部分集成。
fn kind_has_full_integration(kind: &str) -> bool {
    matches!(kind, "powershell" | "pwsh" | "bash" | "zsh")
}

// ==================== shell 集成注入计划 ====================

/// 一次 shell 集成注入需要做的事（纯数据，便于单测）
#[derive(Debug, Clone, PartialEq)]
pub struct IntegrationPlan {
    /// 追加到 shell 命令行的参数
    pub args: Vec<String>,
    /// 需要设置的环境变量
    pub env: Vec<(String, String)>,
    /// 需要写盘的临时文件（路径，内容）；由调用方创建
    pub files: Vec<(PathBuf, String)>,
    /// 集成程度：full / partial / none
    pub level: &'static str,
}

/// PowerShell 集成脚本（powershell.exe / pwsh.exe 通用）
fn powershell_integration_script() -> String {
    // `prompt` 每次显示提示符前被调用，正好是「上条命令结束」的时机：
    // 先吐 133;D;<exit> 与 133;A，再吐 OSC 7 与 OSC 0，最后提示符文本后面跟 133;B。
    let mut s = String::new();
    s.push_str("# BDTerminal shell integration — 自动生成，请勿手改\n");
    s.push_str("$global:__bdt_esc = [char]27\n");
    s.push_str("$global:__bdt_bel = [char]7\n");
    s.push_str("# 统一 UTF-8 输出，避免中文乱码\n");
    s.push_str("try {\n");
    s.push_str("  [Console]::OutputEncoding = [System.Text.UTF8Encoding]::new($false)\n");
    s.push_str("  $OutputEncoding = [Console]::OutputEncoding\n");
    s.push_str("} catch {}\n");
    s.push_str("function global:prompt {\n");
    s.push_str("  $ec = 0\n");
    s.push_str("  if ($global:LASTEXITCODE -is [int]) { $ec = $global:LASTEXITCODE }\n");
    s.push_str("  $global:LASTEXITCODE = 0\n");
    s.push_str("  $path = $ExecutionContext.SessionState.Path.CurrentLocation.Path\n");
    s.push_str("  $uri = 'file:///' + ($path -replace '\\\\', '/')\n");
    s.push_str("  $leaf = Split-Path -Leaf $path\n");
    s.push_str("  if ([string]::IsNullOrEmpty($leaf)) { $leaf = $path }\n");
    s.push_str("  $pre = \"$global:__bdt_esc]133;D;$ec$global:__bdt_bel\"\n");
    s.push_str("  $pre += \"$global:__bdt_esc]133;A$global:__bdt_bel\"\n");
    s.push_str("  $pre += \"$global:__bdt_esc]7;$uri$global:__bdt_bel\"\n");
    s.push_str("  $pre += \"$global:__bdt_esc]0;$leaf$global:__bdt_bel\"\n");
    s.push_str("  return $pre + \"PS $path> \" + \"$global:__bdt_esc]133;B$global:__bdt_bel\"\n");
    s.push_str("}\n");
    s
}

/// bash 集成脚本（作为 `--rcfile`）
fn bash_integration_script() -> String {
    let mut s = String::new();
    s.push_str("# BDTerminal shell integration — 自动生成，请勿手改\n");
    s.push_str("if [ -f \"$HOME/.bashrc\" ]; then . \"$HOME/.bashrc\"; fi\n");
    s.push_str("__bdt_prompt() {\n");
    s.push_str("  local ec=$?\n");
    s.push_str("  printf '\\033]133;D;%s\\007' \"$ec\"\n");
    s.push_str("  printf '\\033]133;A\\007'\n");
    s.push_str("  printf '\\033]7;file://%s%s\\007' \"${HOSTNAME:-localhost}\" \"$PWD\"\n");
    s.push_str("}\n");
    s.push_str("# PS0：读取到命令、执行之前输出 → 133;C（输出开始）\n");
    s.push_str("PS0='\\033]133;C\\007'\n");
    s.push_str("case \"$PROMPT_COMMAND\" in\n");
    s.push_str("  *__bdt_prompt*) ;;\n");
    s.push_str("  \"\") PROMPT_COMMAND=\"__bdt_prompt\" ;;\n");
    s.push_str("  *) PROMPT_COMMAND=\"__bdt_prompt;$PROMPT_COMMAND\" ;;\n");
    s.push_str("esac\n");
    s.push_str("PS1='\\[\\033]133;B\\007\\]'\"$PS1\"\n");
    s.push_str("export PS1 PROMPT_COMMAND PS0\n");
    s
}

/// zsh 集成脚本（作为临时 ZDOTDIR 下的 `.zshrc`）
fn zsh_integration_script() -> String {
    let mut s = String::new();
    s.push_str("# BDTerminal shell integration — 自动生成，请勿手改\n");
    s.push_str("if [ -f \"$HOME/.zshrc\" ]; then source \"$HOME/.zshrc\"; fi\n");
    s.push_str("autoload -Uz add-zsh-hook 2>/dev/null\n");
    s.push_str("__bdt_precmd() {\n");
    s.push_str("  local ec=$?\n");
    s.push_str("  printf '\\033]133;D;%s\\007' \"$ec\"\n");
    s.push_str("  printf '\\033]133;A\\007'\n");
    s.push_str("  printf '\\033]7;file://%s%s\\007' \"${HOST:-localhost}\" \"$PWD\"\n");
    s.push_str("}\n");
    s.push_str("__bdt_preexec() { printf '\\033]133;C\\007' }\n");
    s.push_str("add-zsh-hook precmd __bdt_precmd 2>/dev/null\n");
    s.push_str("add-zsh-hook preexec __bdt_preexec 2>/dev/null\n");
    s.push_str("PS1=$'\\033]133;B\\007'\"$PS1\"\n");
    s
}

/// cmd.exe 的 PROMPT（cmd 支持 `$E` = ESC、`$P` = 当前路径、`$G` = `>`）
/// 只能做到 133;A/B + OSC 7，**拿不到退出码** → partial。
fn cmd_prompt_value() -> String {
    // 生成的字符串形如：ESC]133;A BEL ESC]7;file:///$P BEL $P$G ESC]133;B BEL
    const ESC: char = '\u{1b}';
    const BEL: char = '\u{7}';
    format!("{ESC}]133;A{BEL}{ESC}]7;file:///$P{BEL}$P$G{ESC}]133;B{BEL}")
}

/// 构造某个 shell 的集成注入计划（纯函数，便于单测）
///
/// `tmp_dir` 为该会话独占的临时目录（调用方保证目录存在）；未使用时不写文件。
pub fn integration_plan(kind: &str, tmp_dir: &Path, no_integration: bool) -> IntegrationPlan {
    if no_integration || !kind_has_full_integration(kind) && kind != "cmd" {
        return IntegrationPlan {
            args: Vec::new(),
            env: Vec::new(),
            files: Vec::new(),
            level: "none",
        };
    }

    match kind {
        "powershell" | "pwsh" => {
            let script = powershell_integration_script();
            IntegrationPlan {
                args: vec![
                    "-NoLogo".into(),
                    "-NoExit".into(),
                    "-ExecutionPolicy".into(),
                    "Bypass".into(),
                    "-File".into(),
                    tmp_dir.join("bdt-integration.ps1").to_string_lossy().into_owned(),
                ],
                env: Vec::new(),
                files: vec![(tmp_dir.join("bdt-integration.ps1"), script)],
                level: "full",
            }
        }
        "bash" => IntegrationPlan {
            args: vec![
                "--rcfile".into(),
                tmp_dir.join("bdt-bashrc").to_string_lossy().into_owned(),
                "-i".into(),
            ],
            env: Vec::new(),
            files: vec![(tmp_dir.join("bdt-bashrc"), bash_integration_script())],
            level: "full",
        },
        "zsh" => IntegrationPlan {
            args: vec!["-i".into()],
            env: vec![("ZDOTDIR".into(), tmp_dir.to_string_lossy().into_owned())],
            files: vec![(tmp_dir.join(".zshrc"), zsh_integration_script())],
            level: "full",
        },
        "cmd" => IntegrationPlan {
            args: Vec::new(),
            env: vec![("PROMPT".into(), cmd_prompt_value())],
            files: Vec::new(),
            level: "partial",
        },
        _ => IntegrationPlan {
            args: Vec::new(),
            env: Vec::new(),
            files: Vec::new(),
            level: "none",
        },
    }
}

// ==================== shell 探测 ====================

/// 在 PATH 中查找可执行文件（Windows 追加 PATHEXT 后缀）
fn which(program: &str) -> Option<PathBuf> {
    let path_var = std::env::var_os("PATH")?;
    let exts: Vec<String> = if cfg!(windows) {
        std::env::var("PATHEXT")
            .unwrap_or_else(|_| ".COM;.EXE;.BAT;.CMD".into())
            .split(';')
            .map(|s| s.to_ascii_lowercase())
            .collect()
    } else {
        vec![String::new()]
    };

    for dir in std::env::split_paths(&path_var) {
        // 先按原样试，再按扩展名试（Windows）
        let direct = dir.join(program);
        if direct.is_file() {
            return Some(direct);
        }
        for ext in &exts {
            let candidate = dir.join(format!("{program}{ext}"));
            if candidate.is_file() {
                return Some(candidate);
            }
        }
    }
    None
}

/// 系统目录下的固定路径（Windows）
fn system_file(name: &str) -> Option<PathBuf> {
    let root = std::env::var("SystemRoot").unwrap_or_else(|_| "C:\\Windows".into());
    let p = Path::new(&root).join("System32").join(name);
    if p.is_file() {
        Some(p)
    } else {
        None
    }
}

/// 探测本机可用的 shell 列表（跨平台）
pub fn list_shells() -> Vec<ShellInfo> {
    let mut out: Vec<ShellInfo> = Vec::new();
    let mut push = |id: &str, label: &str, path: PathBuf| {
        let path_str = path.to_string_lossy().into_owned();
        let kind = detect_shell_kind(&path_str).to_string();
        if out.iter().any(|s| s.path == path_str) {
            return;
        }
        out.push(ShellInfo {
            id: id.to_string(),
            label: label.to_string(),
            path: path_str,
            integration: shell_integration_level(&kind).to_string(),
            kind,
            is_default: false,
        });
    };

    if cfg!(windows) {
        if let Some(p) = system_file("WindowsPowerShell\\v1.0\\powershell.exe") {
            push("powershell", "Windows PowerShell", p);
        }
        if let Some(p) = which("pwsh") {
            push("pwsh", "PowerShell 7+", p);
        }
        if let Some(p) = std::env::var_os("COMSPEC").map(PathBuf::from).or_else(|| system_file("cmd.exe")) {
            push("cmd", "命令提示符 (cmd)", p);
        }
        if let Some(p) = which("bash") {
            push("git-bash", "Git Bash", p);
        }
        if let Some(p) = system_file("wsl.exe") {
            push("wsl", "WSL", p);
        }
    } else {
        if let Some(p) = std::env::var_os("SHELL").map(PathBuf::from) {
            if p.is_file() {
                push("default", "默认 shell", p);
            }
        }
        for (id, label, cand) in [
            ("bash", "Bash", "/bin/bash"),
            ("zsh", "Zsh", "/bin/zsh"),
            ("fish", "Fish", "/usr/bin/fish"),
            ("sh", "sh", "/bin/sh"),
        ] {
            if let Some(p) = which(cand) {
                push(id, label, p);
            }
        }
    }

    // 标记默认项
    let default_path = default_shell_path();
    if let Some(d) = out.iter_mut().find(|s| s.path == default_path) {
        d.is_default = true;
    } else if let Some(first) = out.first_mut() {
        first.is_default = true;
    }
    out
}

/// 本平台默认 shell 路径
pub fn default_shell_path() -> String {
    if cfg!(windows) {
        // 优先 PowerShell 7，其次 Windows PowerShell，最后 cmd
        if let Some(p) = which("pwsh") {
            return p.to_string_lossy().into_owned();
        }
        if let Some(p) = system_file("WindowsPowerShell\\v1.0\\powershell.exe") {
            return p.to_string_lossy().into_owned();
        }
        if let Some(p) = std::env::var_os("COMSPEC").map(PathBuf::from) {
            return p.to_string_lossy().into_owned();
        }
        return "cmd.exe".to_string();
    }
    if let Ok(sh) = std::env::var("SHELL") {
        if Path::new(&sh).is_file() {
            return sh;
        }
    }
    for c in ["/bin/bash", "/bin/zsh", "/bin/sh"] {
        if Path::new(c).is_file() {
            return c.to_string();
        }
    }
    "/bin/sh".to_string()
}

/// 用户主目录（作为终端默认 cwd）
pub fn home_dir() -> String {
    std::env::var_os("HOME")
        .or_else(|| std::env::var_os("USERPROFILE"))
        .map(|v| PathBuf::from(v).to_string_lossy().into_owned())
        .unwrap_or_else(|| std::env::current_dir().map(|p| p.to_string_lossy().into_owned()).unwrap_or_default())
}

// ==================== 会话表 ====================

struct Session {
    master: Box<dyn MasterPty + Send>,
    writer: Box<dyn Write + Send>,
    killer: Box<dyn ChildKiller + Send + Sync>,
    /// 该会话独占的临时目录（shell 集成脚本），close 时递归删除
    tmp_dir: Option<PathBuf>,
    alive: Arc<AtomicBool>,
}

static SESSIONS: std::sync::LazyLock<Mutex<HashMap<String, Arc<Mutex<Session>>>>> =
    std::sync::LazyLock::new(|| Mutex::new(HashMap::new()));

/// 删除会话临时目录（尽力而为，失败只记日志）
fn cleanup_tmp(dir: &Path) {
    if let Err(e) = std::fs::remove_dir_all(dir) {
        log::warn!("[terminal] 清理临时目录失败 {}: {e}", dir.display());
    }
}

// ==================== 对外命令实现 ====================

/// 打开一个新的 PTY 会话
pub fn terminal_open(options: TermOpenOptions, sink: TermSink) -> Response<TermOpenResult> {
    let cols = options.cols.max(2);
    let rows = options.rows.max(1);

    let shell_path = options
        .shell
        .clone()
        .filter(|s| !s.trim().is_empty())
        .unwrap_or_else(default_shell_path);
    let kind = detect_shell_kind(&shell_path);

    // cwd：显式指定 → 存在才用；否则 HOME；都不行则用进程 cwd
    let cwd = options
        .cwd
        .clone()
        .map(PathBuf::from)
        .filter(|p| p.is_dir())
        .or_else(|| {
            let h = home_dir();
            let p = PathBuf::from(&h);
            if p.is_dir() {
                Some(p)
            } else {
                None
            }
        })
        .or_else(|| std::env::current_dir().ok())
        .unwrap_or_else(|| PathBuf::from("."));

    let session_id = format!("term-{}", uuid::Uuid::new_v4());
    let tmp_dir = std::env::temp_dir().join("bdt-terminal").join(&session_id);

    let plan = integration_plan(kind, &tmp_dir, options.no_integration);
    // 集成脚本落盘（写失败则退化为无集成，不影响 PTY 本身可用）
    let mut level = plan.level;
    if !plan.files.is_empty() {
        if let Err(e) = std::fs::create_dir_all(&tmp_dir) {
            log::warn!("[terminal] 无法创建集成脚本目录 {e}，将不注入 shell 集成");
            level = "none";
        } else {
            for (path, content) in &plan.files {
                if let Err(e) = std::fs::write(path, content) {
                    log::warn!("[terminal] 写入 {} 失败: {e}", path.display());
                    level = "none";
                }
            }
        }
    }

    // ---- 打开 PTY ----
    let pty_system = native_pty_system();
    let pair = match pty_system.openpty(PtySize {
        rows,
        cols,
        pixel_width: 0,
        pixel_height: 0,
    }) {
        Ok(p) => p,
        Err(e) => {
            return Response::new(500, empty_open_result(&session_id, &shell_path, kind, &cwd), &format!("创建伪终端失败: {e}"));
        }
    };

    let mut cmd = CommandBuilder::new(&shell_path);
    if level != "none" {
        for a in &plan.args {
            cmd.arg(a);
        }
    }
    if let Some(extra) = &options.args {
        for a in extra {
            cmd.arg(a);
        }
    }
    cmd.cwd(&cwd);
    // 终端能力声明（Unix 下 TERM 影响程序着色/光标控制；Windows ConPTY 不依赖它但无害）
    cmd.env("COLORTERM", "truecolor");
    cmd.env("TERM", "xterm-256color");
    cmd.env("TERM_PROGRAM", "BDTools");
    cmd.env("TERM_PROGRAM_VERSION", env!("CARGO_PKG_VERSION"));
    for (k, v) in &plan.env {
        cmd.env(k, v);
    }
    if let Some(extra) = &options.env {
        for (k, v) in extra {
            cmd.env(k, v);
        }
    }

    let mut child = match pair.slave.spawn_command(cmd) {
        Ok(c) => c,
        Err(e) => {
            if !plan.files.is_empty() {
                cleanup_tmp(&tmp_dir);
            }
            return Response::new(
                500,
                empty_open_result(&session_id, &shell_path, kind, &cwd),
                &format!("启动 shell 失败（{shell_path}）: {e}"),
            );
        }
    };
    // slave 必须立即释放：否则 pty 不会在子进程退出时关闭（Unix 上表现为永不 EOF）
    drop(pair.slave);

    let pid = child.process_id();
    let killer = child.clone_killer();

    let writer = match pair.master.take_writer() {
        Ok(w) => w,
        Err(e) => {
            let _ = child.kill();
            if !plan.files.is_empty() {
                cleanup_tmp(&tmp_dir);
            }
            return Response::new(
                500,
                empty_open_result(&session_id, &shell_path, kind, &cwd),
                &format!("获取 PTY 写入端失败: {e}"),
            );
        }
    };

    let reader = match pair.master.try_clone_reader() {
        Ok(r) => r,
        Err(e) => {
            let _ = child.kill();
            if !plan.files.is_empty() {
                cleanup_tmp(&tmp_dir);
            }
            return Response::new(
                500,
                empty_open_result(&session_id, &shell_path, kind, &cwd),
                &format!("获取 PTY 读取端失败: {e}"),
            );
        }
    };

    let alive = Arc::new(AtomicBool::new(true));

    SESSIONS.lock().insert(
        session_id.clone(),
        Arc::new(Mutex::new(Session {
            master: pair.master,
            writer,
            killer,
            tmp_dir: if plan.files.is_empty() { None } else { Some(tmp_dir) },
            alive: alive.clone(),
        })),
    );

    // ---- 读线程：PTY → Channel ----
    spawn_reader(session_id.clone(), reader, sink.clone(), alive.clone());

    // ---- 等待线程：子进程退出 → Exit 事件 + 回收会话 ----
    {
        let sid = session_id.clone();
        let sink = sink.clone();
        let alive = alive.clone();
        thread::spawn(move || {
            let status = child.wait();
            alive.store(false, Ordering::SeqCst);
            match status {
                Ok(st) => {
                    let signal = st.signal().map(|s| s.to_string());
                    let code = if signal.is_some() { None } else { Some(st.exit_code() as i32) };
                    sink(TermEvent::Exit { code, signal });
                }
                Err(e) => {
                    sink(TermEvent::Error {
                        message: format!("等待 shell 退出失败: {e}"),
                    });
                    sink(TermEvent::Exit {
                        code: None,
                        signal: None,
                    });
                }
            }
            // 会话已死：从表中移除并清理临时目录
            let removed = SESSIONS.lock().remove(&sid);
            if let Some(sess) = removed {
                let guard = sess.lock();
                if let Some(dir) = &guard.tmp_dir {
                    cleanup_tmp(dir);
                }
            }
        });
    }

    Response::new(
        200,
        TermOpenResult {
            session_id,
            shell: shell_path,
            shell_kind: kind.to_string(),
            cwd: cwd.to_string_lossy().into_owned(),
            pid,
            integration: level.to_string(),
        },
        "ok",
    )
}

fn empty_open_result(session_id: &str, shell: &str, kind: &str, cwd: &Path) -> TermOpenResult {
    TermOpenResult {
        session_id: session_id.to_string(),
        shell: shell.to_string(),
        shell_kind: kind.to_string(),
        cwd: cwd.to_string_lossy().into_owned(),
        pid: None,
        integration: "none".to_string(),
    }
}

/// 命令层在「连 spawn_blocking 都没跑起来」这类异常路径上构造占位结果用
pub fn empty_open_result_default() -> TermOpenResult {
    let shell = default_shell_path();
    let kind = detect_shell_kind(&shell).to_string();
    TermOpenResult {
        session_id: String::new(),
        shell,
        shell_kind: kind,
        cwd: home_dir(),
        pid: None,
        integration: "none".to_string(),
    }
}

/// 命令层在「连 spawn_blocking 都没跑起来」这类异常路径上构造占位结果用
pub fn empty_defaults() -> TermDefaults {
    let shell = default_shell_path();
    TermDefaults {
        shell_kind: detect_shell_kind(&shell).to_string(),
        shell,
        cwd: home_dir(),
        platform: std::env::consts::OS.to_string(),
    }
}

/// 读线程：把 PTY 输出按块 base64 后推给前端
fn spawn_reader(session_id: String, mut reader: Box<dyn Read + Send>, sink: TermSink, alive: Arc<AtomicBool>) {
    thread::spawn(move || {
        let mut buf = vec![0u8; READ_BUF_SIZE];
        loop {
            match reader.read(&mut buf) {
                Ok(0) => break, // EOF：子进程退出
                Ok(n) => {
                    if !alive.load(Ordering::SeqCst) {
                        break;
                    }
                    let data = B64.encode(&buf[..n]);
                    sink(TermEvent::Data { data });
                }
                Err(ref e) if e.kind() == std::io::ErrorKind::Interrupted => continue,
                Err(e) => {
                    // 会话被主动关闭时读端报错是预期行为，不当作错误上报
                    if alive.load(Ordering::SeqCst) {
                        sink(TermEvent::Error {
                            message: format!("读取终端输出失败: {e}"),
                        });
                    }
                    break;
                }
            }
        }
        log::debug!("[terminal] 读线程结束: {session_id}");
    });
}

/// 向 PTY 写入用户按键 / 粘贴内容（UTF-8 字节）
pub fn terminal_write(session_id: String, data: String) -> Response<bool> {
    let sessions = SESSIONS.lock();
    let Some(sess) = sessions.get(&session_id).cloned() else {
        return Response::new(404, false, "会话不存在或已结束");
    };
    drop(sessions);

    let mut guard = sess.lock();
    match guard.writer.write_all(data.as_bytes()).and_then(|_| guard.writer.flush()) {
        Ok(()) => Response::new(200, true, "ok"),
        Err(e) => Response::new(500, false, &format!("写入终端失败: {e}")),
    }
}

/// 调整终端尺寸（内核会向 shell 发 SIGWINCH，Windows ConPTY 由 resize 内部处理）
pub fn terminal_resize(session_id: String, cols: u16, rows: u16) -> Response<bool> {
    let sessions = SESSIONS.lock();
    let Some(sess) = sessions.get(&session_id).cloned() else {
        return Response::new(404, false, "会话不存在或已结束");
    };
    drop(sessions);

    let guard = sess.lock();
    match guard.master.resize(PtySize {
        rows: rows.max(1),
        cols: cols.max(2),
        pixel_width: 0,
        pixel_height: 0,
    }) {
        Ok(()) => Response::new(200, true, "ok"),
        Err(e) => Response::new(500, false, &format!("调整终端尺寸失败: {e}")),
    }
}

/// 关闭会话：kill 子进程、从表中移除、清理临时目录。
/// 读线程与等待线程会自然退出（killer 触发 EOF）。
///
/// **PTY 的释放放在独立线程里做，不在调用线程上**：`MasterPty` 落地会关掉
/// ConPTY 句柄，Windows 上 `ClosePseudoConsole` 会等 conhost 退出——如果 shell 起了
/// 孙进程（分页器 / 长驻子进程）仍挂着这个控制台，它会阻塞相当久。
/// 这个调用链是从「关标签」进来的（`terminal_close` → 释放 Session），
/// 早先它是同步命令（跑在主线程上），表现就是**关掉终端标签后整个页面无响应**。
/// 现在命令层已改为 `async + spawn_blocking`，这里再兜一层：
/// 即便 detach 的线程真的卡住，也只影响它自己。
pub fn terminal_close(session_id: String) -> Response<bool> {
    let removed = SESSIONS.lock().remove(&session_id);
    let Some(sess) = removed else {
        return Response::new(200, true, "会话已不存在");
    };

    let dir = {
        let mut guard = sess.lock();
        guard.alive.store(false, Ordering::SeqCst);
        let kill_result = guard.killer.kill();
        if let Err(e) = kill_result {
            log::warn!("[terminal] kill shell 失败: {e}");
        }
        guard.tmp_dir.take()
    };
    if let Some(d) = dir {
        cleanup_tmp(&d);
    }

    // 会话已从表里摘掉，不再有任何命令能碰到它；剩下的只是释放句柄。
    thread::spawn(move || drop(sess));

    Response::new(200, true, "ok")
}

/// 会话是否还活着（前端重连/自检用）
pub fn terminal_alive(session_id: String) -> Response<bool> {
    let sessions = SESSIONS.lock();
    match sessions.get(&session_id) {
        Some(s) => Response::new(200, s.lock().alive.load(Ordering::SeqCst), "ok"),
        None => Response::new(200, false, "会话不存在"),
    }
}

/// 当前进程可见的 shell 列表
pub fn terminal_list_shells() -> Response<Vec<ShellInfo>> {
    Response::new(200, list_shells(), "ok")
}

/// 终端默认启动信息（默认 shell / 默认 cwd），前端首次建会话前展示用
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TermDefaults {
    pub shell: String,
    pub shell_kind: String,
    pub cwd: String,
    pub platform: String,
}

pub fn terminal_defaults() -> Response<TermDefaults> {
    let shell = default_shell_path();
    let kind = detect_shell_kind(&shell).to_string();
    Response::new(
        200,
        TermDefaults {
            shell,
            shell_kind: kind,
            cwd: home_dir(),
            platform: std::env::consts::OS.to_string(),
        },
        "ok",
    )
}

// ==================== 单测 ====================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detect_kind_covers_common_shells() {
        assert_eq!(detect_shell_kind("C:\\Windows\\System32\\WindowsPowerShell\\v1.0\\powershell.exe"), "powershell");
        assert_eq!(detect_shell_kind("/usr/local/bin/pwsh"), "pwsh");
        assert_eq!(detect_shell_kind("C:\\Windows\\System32\\cmd.exe"), "cmd");
        assert_eq!(detect_shell_kind("/bin/bash"), "bash");
        assert_eq!(detect_shell_kind("/bin/zsh"), "zsh");
        assert_eq!(detect_shell_kind("/usr/bin/fish"), "fish");
        assert_eq!(detect_shell_kind("C:\\Windows\\System32\\wsl.exe"), "wsl");
        assert_eq!(detect_shell_kind("/opt/weird/thing"), "unknown");
    }

    /// UI 上的集成标签必须等于实际注入结果。
    /// 两者曾经漂移：`full_integration: bool` 把 cmd 标成「完整集成」（实际 partial，拿不到退出码）、
    /// 把 WSL 标成「部分集成」（实际 none，压根没注入）。
    #[test]
    fn integration_level_matches_plan_output() {
        let tmp = PathBuf::from("/tmp/x");
        for kind in ["powershell", "pwsh", "bash", "zsh", "cmd", "wsl", "fish", "nu", "sh", "unknown"] {
            assert_eq!(
                shell_integration_level(kind),
                integration_plan(kind, &tmp, false).level,
                "kind={kind} 的集成标签与实际注入等级不一致"
            );
        }
        // 两个曾经出错的点，钉死
        assert_eq!(shell_integration_level("cmd"), "partial");
        assert_eq!(shell_integration_level("wsl"), "none");
    }

    #[test]
    fn integration_plan_respects_no_integration() {
        let tmp = PathBuf::from("/tmp/x");
        let plan = integration_plan("bash", &tmp, true);
        assert_eq!(plan.level, "none");
        assert!(plan.args.is_empty() && plan.files.is_empty());
    }

    #[test]
    fn bash_plan_uses_rcfile() {
        let tmp = PathBuf::from("/tmp/x");
        let plan = integration_plan("bash", &tmp, false);
        assert_eq!(plan.level, "full");
        assert_eq!(plan.args[0], "--rcfile");
        assert!(plan.args[2] == "-i");
        assert_eq!(plan.files.len(), 1);
        let script = &plan.files[0].1;
        // 关键标记齐全：退出码(D)、提示符(A/B)、cwd(OSC 7)
        assert!(script.contains("]133;D;"));
        assert!(script.contains("]133;A"));
        assert!(script.contains("]133;B"));
        assert!(script.contains("]7;file://"));
    }

    #[test]
    fn powershell_plan_writes_script_with_all_markers() {
        let tmp = PathBuf::from("C:\\tmp\\x");
        let plan = integration_plan("pwsh", &tmp, false);
        assert_eq!(plan.level, "full");
        assert!(plan.args.contains(&"-NoExit".to_string()));
        let script = &plan.files[0].1;
        assert!(script.contains("]133;D;"));
        assert!(script.contains("]133;A"));
        assert!(script.contains("]133;B"));
        assert!(script.contains("]7;"));
    }

    #[test]
    fn zsh_plan_sets_zdotdir_and_dot_zshrc() {
        let tmp = PathBuf::from("/tmp/z");
        let plan = integration_plan("zsh", &tmp, false);
        assert_eq!(plan.level, "full");
        assert_eq!(plan.env[0].0, "ZDOTDIR");
        assert!(plan.files[0].0.ends_with(".zshrc"));
        let script = &plan.files[0].1;
        assert!(script.contains("add-zsh-hook precmd"));
        assert!(script.contains("]133;C"));
    }

    #[test]
    fn cmd_plan_is_partial_and_only_env() {
        let plan = integration_plan("cmd", PathBuf::from("C:\\t").as_path(), false);
        assert_eq!(plan.level, "partial");
        assert!(plan.args.is_empty());
        assert_eq!(plan.files.len(), 0);
        assert_eq!(plan.env[0].0, "PROMPT");
        // PROMPT 里必须有 ESC 与 133;A/B 标记
        let prompt = &plan.env[0].1;
        assert!(prompt.contains('\u{1b}'));
        assert!(prompt.contains("]133;A"));
        assert!(prompt.contains("]133;B"));
        assert!(prompt.contains("]7;file://"));
    }

    #[test]
    fn unknown_shell_gets_no_integration() {
        let plan = integration_plan("fish", PathBuf::from("/tmp/f").as_path(), false);
        assert_eq!(plan.level, "none");
    }

    #[test]
    fn defaults_are_usable() {
        let shell = default_shell_path();
        assert!(!shell.is_empty());
        assert!(!home_dir().is_empty());
        let shells = list_shells();
        assert!(!shells.is_empty(), "至少应探测到一个 shell");
        assert_eq!(shells.iter().filter(|s| s.is_default).count(), 1);
    }

    /// 端到端：真开一个 PTY 会话 → 从 sink 收到输出 → 关闭会话快速返回。
    ///
    /// 这条钉的是「关标签」真正走的整条链：
    /// `terminal_open`（PTY → 读线程 → sink，即前端的 Tauri Channel，base64 透传）
    /// 与 `terminal_close`（kill shell + 释放 PTY）。
    ///
    /// **为什么关闭这一步必须单独断言耗时**：在 Windows 上释放 `MasterPty` 会走
    /// `ClosePseudoConsole`，它要等 conhost 退出；shell 是被 `TerminateProcess` 杀掉、
    /// 而读端还持有输出管时，这个调用实测可以一直不返回（本机曾有测试进程因此挂住 6 小时）。
    /// 早先 `terminal_close` 是同步命令（跑在主线程上），于是「关掉终端标签」就等于
    /// 整个界面无响应 —— 所以这里要求：关闭必须在几秒内把控制权还给调用方。
    #[test]
    fn pty_roundtrip_streams_output_and_closes_fast() {
        use std::sync::Mutex;
        use std::time::{Duration, Instant};

        // 收集读线程推过来的 data（用 Mutex 而不是 mpsc：TermSink 要求 Send + Sync）
        let seen: Arc<Mutex<Vec<String>>> = Arc::new(Mutex::new(Vec::new()));
        let sink: TermSink = {
            let seen = seen.clone();
            Arc::new(move |ev| {
                if let TermEvent::Data { data } = ev {
                    if let Ok(mut v) = seen.lock() {
                        v.push(data);
                    }
                }
            })
        };

        let opened = terminal_open(
            TermOpenOptions {
                cols: 80,
                rows: 24,
                cwd: None,
                shell: None,
                args: None,
                env: None,
                no_integration: true,
            },
            sink,
        );
        if opened.code != 200 {
            eprintln!("跳过：本机无法打开 PTY（{}）", opened.msg);
            return;
        }
        let sid = opened.data.session_id.clone();

        // shell 启动横幅 / 提示符应经 sink 透传回来（base64 字符串非空）
        let deadline = Instant::now() + Duration::from_secs(30);
        let mut chunks = 0usize;
        while Instant::now() < deadline && chunks == 0 {
            thread::sleep(Duration::from_millis(50));
            chunks = seen.lock().map(|v| v.len()).unwrap_or(0);
        }
        assert!(chunks > 0, "PTY 输出应经 sink 透传回来，实际一块都没收到");

        // 用「另起线程 + 超时」而不是直接调用：一旦关闭又变回阻塞，这条测试要**失败**
        // 而不是把整个测试进程挂住（真有那一天，看失败信息比等 6 小时强）。
        let (done_tx, done_rx) = std::sync::mpsc::channel::<usize>();
        let closing = sid.clone();
        thread::spawn(move || {
            let code = terminal_close(closing).code;
            let _ = done_tx.send(code);
        });
        match done_rx.recv_timeout(Duration::from_secs(10)) {
            Ok(code) => assert_eq!(code, 200, "关闭会话应成功"),
            Err(_) => panic!(
                "terminal_close 超过 10s 未返回：PTY 的释放又落回调用线程了 \
                 （Windows 的 ClosePseudoConsole 会一直等 conhost，主线程上就是界面无响应）"
            ),
        }

        assert!(!terminal_alive(sid.clone()).data, "关闭后会话表里不应再有它");
        assert_eq!(terminal_close(sid).code, 200, "重复关闭应幂等");
    }
}
