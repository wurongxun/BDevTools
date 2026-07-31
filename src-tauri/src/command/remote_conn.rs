/*! # 远程连接管理模块 - SSH终端 + SFTP文件操作 */
use crate::response::response::Response;
use anyhow::Result;
use serde::{Deserialize, Serialize};
use ssh2::Session;
use std::collections::HashMap;
use std::io::{Read, Write};
use std::net::TcpStream;
use std::path::Path;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;
use tauri::{command, AppHandle, Emitter};

// 全局会话管理
lazy_static::lazy_static! {
    static ref SSH_SESSIONS: Mutex<HashMap<String, Arc<Mutex<SshSessionInner>>>> = Mutex::new(HashMap::new());
    static ref SFTP_SESSIONS: Mutex<HashMap<String, Arc<Session>>> = Mutex::new(HashMap::new());
}

/// SSH 会话内部状态
struct SshSessionInner {
    session: Session,
    channel: Option<ssh2::Channel>,
}

/// SSH 文件项
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct RemoteFileItem {
    pub name: String,
    pub path: String,
    pub is_directory: bool,
    pub size: u64,
    pub modified_time: String,
    pub permissions: String,
}

// ===================== SSH 终端操作 =====================

/// SSH 连接
#[command]
pub async fn ssh_connect(
    app: AppHandle,
    tab_id: String,
    host: String,
    port: u16,
    username: String,
    password: String,
) -> Response<Option<String>> {
    let session_id = format!("ssh-{}", uuid::Uuid::new_v4());

    let result = tokio::task::spawn_blocking({
        let session_id = session_id.clone();
        let app = app.clone();
        move || -> Result<String> {
            let tcp = TcpStream::connect(format!("{}:{}", host, port))?;
            tcp.set_read_timeout(Some(Duration::from_secs(10)))?;

            let mut session = Session::new()?;
            session.set_tcp_stream(tcp);
            session.handshake()?;
            session.userauth_password(&username, &password)?;

            if !session.authenticated() {
                return Err(anyhow::anyhow!("认证失败，请检查用户名和密码"));
            }

            let mut channel = session.channel_session()?;
            channel.request_pty("xterm-256color", None, Some((120, 40, 0, 0)))?;
            channel.shell()?;
            // ssh2::Channel 不支持 set_blocking，通过 session 级别设置
            // 非阻塞读取在后台线程中通过超时机制实现

            let inner = Arc::new(Mutex::new(SshSessionInner {
                session,
                channel: Some(channel),
            }));

            SSH_SESSIONS
                .lock()
                .unwrap()
                .insert(session_id.clone(), inner.clone());

            // 启动后台读取线程
            let inner_read = inner.clone();
            let sid = session_id.clone();
            let app_read = app.clone();
            thread::spawn(move || {
                let mut buf = [0u8; 4096];
                loop {
                    {
                        let guard = inner_read.lock().unwrap();
                        if guard.channel.is_none() {
                            break;
                        }
                    }
                    // 尝试读取
                    let data = {
                        let mut guard = inner_read.lock().unwrap();
                        if let Some(ref mut ch) = guard.channel {
                            let mut s = String::new();
                            match ch.read_to_string(&mut s) {
                                Ok(_) if !s.is_empty() => Some(s),
                                _ => None,
                            }
                        } else {
                            None
                        }
                    };

                    if let Some(output) = data {
                        let _ = app_read.emit("ssh-output", serde_json::json!({
                            "tabId": tab_id,
                            "data": output
                        }));
                    } else {
                        thread::sleep(Duration::from_millis(50));
                    }
                }
            });

            Ok(session_id)
        }
    })
    .await;

    match result {
        Ok(Ok(sid)) => Response::new(200, Some(sid), "连接成功"),
        Ok(Err(e)) => Response::new(300, None, &format!("连接失败: {}", e)),
        Err(e) => Response::new(300, None, &format!("连接异常: {}", e)),
    }
}

/// SSH 写入（用户输入）
#[command]
pub async fn ssh_write(session_id: String, data: String) -> Response<bool> {
    let sessions = SSH_SESSIONS.lock().unwrap();
    if let Some(inner) = sessions.get(&session_id) {
        let mut guard = inner.lock().unwrap();
        if let Some(ref mut ch) = guard.channel {
            match ch.write_all(data.as_bytes()) {
                Ok(_) => Response::new(200, true, "写入成功"),
                Err(e) => Response::new(300, false, &format!("写入失败: {}", e)),
            }
        } else {
            Response::new(300, false, "通道不存在")
        }
    } else {
        Response::new(300, false, "会话不存在")
    }
}

/// SSH 断开连接
#[command]
pub async fn ssh_disconnect(session_id: String) -> Response<bool> {
    let mut sessions = SSH_SESSIONS.lock().unwrap();
    if let Some(inner) = sessions.remove(&session_id) {
        let mut guard = inner.lock().unwrap();
        if let Some(mut ch) = guard.channel.take() {
            let _ = ch.close();
        }
        Response::new(200, true, "已断开")
    } else {
        Response::new(300, false, "会话不存在")
    }
}

/// SSH 调整 PTY 大小
#[command]
pub async fn ssh_resize_pty(
    session_id: String,
    cols: u32,
    rows: u32,
) -> Response<bool> {
    let sessions = SSH_SESSIONS.lock().unwrap();
    if let Some(inner) = sessions.get(&session_id) {
        let mut guard = inner.lock().unwrap();
        if let Some(ref mut ch) = guard.channel {
            let _ = ch.request_pty_size(cols, rows, Some(0), Some(0));
            Response::new(200, true, "调整成功")
        } else {
            Response::new(300, false, "通道不存在")
        }
    } else {
        Response::new(300, false, "会话不存在")
    }
}

// ===================== SFTP 文件操作 =====================

/// SFTP 连接
#[command]
pub async fn sftp_connect(
    tab_id: String,
    host: String,
    port: u16,
    username: String,
    password: String,
) -> Response<Option<String>> {
    let session_id = format!("sftp-{}", uuid::Uuid::new_v4());

    let result = tokio::task::spawn_blocking(move || -> Result<String> {
        let tcp = TcpStream::connect(format!("{}:{}", host, port))?;
        let mut session = Session::new()?;
        session.set_tcp_stream(tcp);
        session.handshake()?;
        session.userauth_password(&username, &password)?;

        if !session.authenticated() {
            return Err(anyhow::anyhow!("SFTP 认证失败"));
        }

        let sess = Arc::new(session);
        SFTP_SESSIONS
            .lock()
            .unwrap()
            .insert(session_id.clone(), sess);

        Ok(session_id)
    })
    .await;

    match result {
        Ok(Ok(sid)) => Response::new(200, Some(sid), "SFTP 连接成功"),
        Ok(Err(e)) => Response::new(300, None, &format!("连接失败: {}", e)),
        Err(e) => Response::new(300, None, &format!("连接异常: {}", e)),
    }
}

/// SFTP 断开连接
#[command]
pub async fn sftp_disconnect(session_id: String) -> Response<bool> {
    let mut sessions = SFTP_SESSIONS.lock().unwrap();
    sessions.remove(&session_id);
    Response::new(200, true, "已断开")
}

/// SFTP 列出目录
#[command]
pub async fn sftp_list_dir(
    session_id: String,
    path: String,
) -> Response<Vec<RemoteFileItem>> {
    let sess = {
        let sessions = SFTP_SESSIONS.lock().unwrap();
        sessions.get(&session_id).cloned()
    };
    if let Some(sess) = sess {

        let result = tokio::task::spawn_blocking(move || -> Result<Vec<RemoteFileItem>> {
            let sftp = sess.sftp()?;
            let entries = sftp.readdir(Path::new(&path))?;

            let mut files = Vec::new();
            for entry in entries {
                let (path_buf, stat) = entry;
                let name = path_buf
                    .file_name()
                    .unwrap_or_default()
                    .to_string_lossy()
                    .to_string();
                let file_path = path_buf.to_string_lossy().to_string();
                let is_dir = stat.is_dir();
                let size = stat.size.unwrap_or(0);

                let modified_time = stat
                    .mtime
                    .map(|t| {
                        chrono::DateTime::from_timestamp(t as i64, 0)
                            .map(|dt| dt.format("%Y-%m-%d %H:%M:%S").to_string())
                            .unwrap_or_default()
                    })
                    .unwrap_or_default();

                let permissions = stat
                    .perm
                    .map(|p| format!("{:o}", p))
                    .unwrap_or_default();

                files.push(RemoteFileItem {
                    name,
                    path: file_path,
                    is_directory: is_dir,
                    size,
                    modified_time,
                    permissions,
                });
            }

            // 排序：目录在前，文件在后
            files.sort_by(|a, b| {
                if a.is_directory != b.is_directory {
                    b.is_directory.cmp(&a.is_directory)
                } else {
                    a.name.to_lowercase().cmp(&b.name.to_lowercase())
                }
            });

            Ok(files)
        })
        .await;

        match result {
            Ok(Ok(files)) => Response::new(200, files, "查询成功"),
            Ok(Err(e)) => Response::new(300, Vec::new(), &format!("列出文件失败: {}", e)),
            Err(e) => Response::new(300, Vec::new(), &format!("执行异常: {}", e)),
        }
    } else {
        Response::new(300, Vec::new(), "会话不存在")
    }
}

/// SFTP 创建目录
#[command]
pub async fn sftp_mkdir(session_id: String, path: String) -> Response<bool> {
    let sess = {
        let sessions = SFTP_SESSIONS.lock().unwrap();
        sessions.get(&session_id).cloned()
    };
    if let Some(sess) = sess {

        let result = tokio::task::spawn_blocking(move || -> Result<bool> {
            let sftp = sess.sftp()?;
            sftp.mkdir(Path::new(&path), 0o755)?;
            Ok(true)
        })
        .await;

        match result {
            Ok(Ok(_)) => Response::new(200, true, "创建成功"),
            Ok(Err(e)) => Response::new(300, false, &format!("创建失败: {}", e)),
            Err(e) => Response::new(300, false, &format!("执行异常: {}", e)),
        }
    } else {
        Response::new(300, false, "会话不存在")
    }
}

/// SFTP 删除文件/目录
#[command]
pub async fn sftp_remove(session_id: String, path: String) -> Response<bool> {
    let sess = {
        let sessions = SFTP_SESSIONS.lock().unwrap();
        sessions.get(&session_id).cloned()
    };
    if let Some(sess) = sess {

        let result = tokio::task::spawn_blocking(move || -> Result<bool> {
            let sftp = sess.sftp()?;
            let p = Path::new(&path);

            if let Ok(stat) = sftp.stat(p) {
                if stat.is_dir() {
                    sftp.rmdir(p)?;
                } else {
                    sftp.unlink(p)?;
                }
                Ok(true)
            } else {
                Err(anyhow::anyhow!("文件不存在"))
            }
        })
        .await;

        match result {
            Ok(Ok(_)) => Response::new(200, true, "删除成功"),
            Ok(Err(e)) => Response::new(300, false, &format!("删除失败: {}", e)),
            Err(e) => Response::new(300, false, &format!("执行异常: {}", e)),
        }
    } else {
        Response::new(300, false, "会话不存在")
    }
}

/// SFTP 上传文件
#[command]
pub async fn sftp_upload(
    app: AppHandle,
    session_id: String,
    local_path: String,
    remote_path: String,
    task_id: String,
) -> Response<bool> {
    let sess = {
        let sessions = SFTP_SESSIONS.lock().unwrap();
        sessions.get(&session_id).cloned()
    };
    if let Some(sess) = sess {
        let result = tokio::task::spawn_blocking(move || -> Result<bool> {
            let sftp = sess.sftp()?;
            let local = Path::new(&local_path);
            let file_size = std::fs::metadata(local)?.len();

            let mut remote_file = sftp.create(Path::new(&remote_path))?;
            let mut local_file = std::fs::File::open(local)?;

            let mut buffer = [0u8; 8192];
            let mut transferred: u64 = 0;

            loop {
                let bytes_read = local_file.read(&mut buffer)?;
                if bytes_read == 0 {
                    break;
                }
                remote_file.write_all(&buffer[..bytes_read])?;
                transferred += bytes_read as u64;

                let progress = if file_size > 0 {
                    (transferred as f64 / file_size as f64 * 100.0) as u32
                } else {
                    0
                };

                let _ = app.emit("transfer-progress", serde_json::json!({
                    "tabId": "",
                    "taskId": task_id,
                    "progress": progress,
                    "transferredSize": transferred,
                    "speed": ""
                }));
            }

            Ok(true)
        })
        .await;

        match result {
            Ok(Ok(_)) => Response::new(200, true, "上传成功"),
            Ok(Err(e)) => Response::new(300, false, &format!("上传失败: {}", e)),
            Err(e) => Response::new(300, false, &format!("执行异常: {}", e)),
        }
    } else {
        Response::new(300, false, "会话不存在")
    }
}

/// SFTP 下载文件
#[command]
pub async fn sftp_download(
    app: AppHandle,
    session_id: String,
    remote_path: String,
    local_path: String,
    task_id: String,
) -> Response<bool> {
    let sess = {
        let sessions = SFTP_SESSIONS.lock().unwrap();
        sessions.get(&session_id).cloned()
    };
    if let Some(sess) = sess {
        let result = tokio::task::spawn_blocking(move || -> Result<bool> {
            let sftp = sess.sftp()?;
            let remote = Path::new(&remote_path);

            let stat = sftp.stat(remote)?;
            let file_size = stat.size.unwrap_or(0);

            let mut remote_file = sftp.open(remote)?;
            let mut local_file = std::fs::File::create(Path::new(&local_path))?;

            let mut buffer = [0u8; 8192];
            let mut transferred: u64 = 0;

            loop {
                let bytes_read = remote_file.read(&mut buffer)?;
                if bytes_read == 0 {
                    break;
                }
                local_file.write_all(&buffer[..bytes_read])?;
                transferred += bytes_read as u64;

                let progress = if file_size > 0 {
                    (transferred as f64 / file_size as f64 * 100.0) as u32
                } else {
                    0
                };

                let _ = app.emit("transfer-progress", serde_json::json!({
                    "tabId": "",
                    "taskId": task_id,
                    "progress": progress,
                    "transferredSize": transferred,
                    "speed": ""
                }));
            }

            Ok(true)
        })
        .await;

        match result {
            Ok(Ok(_)) => Response::new(200, true, "下载成功"),
            Ok(Err(e)) => Response::new(300, false, &format!("下载失败: {}", e)),
            Err(e) => Response::new(300, false, &format!("执行异常: {}", e)),
        }
    } else {
        Response::new(300, false, "会话不存在")
    }
}

/// 本地目录列出
#[command]
pub async fn local_list_dir(path: String) -> Response<Vec<RemoteFileItem>> {
    let result = tokio::task::spawn_blocking(move || -> Result<Vec<RemoteFileItem>> {
        let dir_path = if path.is_empty() {
            std::env::current_dir()?
        } else {
            Path::new(&path).to_path_buf()
        };

        let mut files = Vec::new();
        for entry in std::fs::read_dir(&dir_path)? {
            let entry = entry?;
            let metadata = entry.metadata()?;
            let name = entry.file_name().to_string_lossy().to_string();
            let file_path = entry.path().to_string_lossy().to_string();
            let is_dir = metadata.is_dir();
            let size = metadata.len();

            let modified_time = metadata
                .modified()
                .ok()
                .and_then(|t| {
                    let duration = t.duration_since(std::time::UNIX_EPOCH).ok()?;
                    chrono::DateTime::from_timestamp(duration.as_secs() as i64, 0)
                        .map(|dt| dt.format("%Y-%m-%d %H:%M:%S").to_string())
                })
                .unwrap_or_default();

            files.push(RemoteFileItem {
                name,
                path: file_path,
                is_directory: is_dir,
                size,
                modified_time,
                permissions: String::new(),
            });
        }

        files.sort_by(|a, b| {
            if a.is_directory != b.is_directory {
                b.is_directory.cmp(&a.is_directory)
            } else {
                a.name.to_lowercase().cmp(&b.name.to_lowercase())
            }
        });

        Ok(files)
    })
    .await;

    match result {
        Ok(Ok(files)) => Response::new(200, files, "查询成功"),
        Ok(Err(e)) => Response::new(300, Vec::new(), &format!("列出文件失败: {}", e)),
        Err(e) => Response::new(300, Vec::new(), &format!("执行异常: {}", e)),
    }
}
