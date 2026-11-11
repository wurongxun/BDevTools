/*! # ssh_exec：单条远程命令执行

与交互式 PTY shell 分开：不开 shell、不分配 PTY，stdout / stderr 分两条流读，
返回退出码。供脚本、健康检查、远程日志之外的「跑一条命令」场景复用。

安全口径与终端完全一致 —— 走 `remote_conn::establish_session`，
所以代理、跳板机、known_hosts、2FA 全部自动生效。
*/
use crate::options::parse_target;
use crate::remote_conn::{connect_failure, establish_session, params_from_target, ConnectError};
use bdevtools_core::response::response::Response;
use serde::Serialize;
use ssh2::ExtendedData;
use std::io::{ErrorKind, Read};
use std::time::{Duration, Instant};

#[derive(Serialize, Default, Clone)]
#[serde(rename_all = "camelCase")]
pub struct ExecResult {
    /// stdout / stderr 是命令输出，按 UTF-8 lossy 解码（脚本场景；终端流仍走 base64）
    pub stdout: String,
    pub stderr: String,
    pub exit_code: i32,
}

/// `target` 为 `options::ConnectTarget` 的 JSON 字符串。
pub async fn ssh_exec(
    target: String,
    command: String,
    timeout_ms: Option<u64>,
) -> Response<ExecResult> {
    let t = match parse_target(&target) {
        Ok(t) => t,
        Err(e) => return Response::new(300, ExecResult::default(), &e.to_string()),
    };
    if command.trim().is_empty() {
        return Response::new(300, ExecResult::default(), "命令不能为空");
    }
    let params = params_from_target(t);
    let timeout = Duration::from_millis(timeout_ms.unwrap_or(30_000).clamp(1_000, 600_000));

    let outcome = tokio::task::spawn_blocking(
        move || -> std::result::Result<ExecResult, ConnectError> {
            let session = establish_session(&params)?;
            let mut channel = session.channel_session()?;
            // Normal = stderr 单独排队，用 channel.stderr() 读；默认 Merge 会把两股流混在一起
            channel.handle_extended_data(ExtendedData::Normal)?;
            channel.exec(&command)?;
            // 非阻塞后交替读两条流：blocking 模式下 ssh2 的 Stream::read 会一直持有
            // SessionInner 锁，另一条流（以及写入）会被卡死
            session.set_blocking(false);

            let mut stdout = channel.stream(0);
            let mut stderr = channel.stderr();
            let mut out: Vec<u8> = Vec::new();
            let mut err: Vec<u8> = Vec::new();
            let mut buf = [0u8; 8192];
            let start = Instant::now();

            loop {
                let mut got = false;
                match stdout.read(&mut buf) {
                    Ok(0) => {}
                    Ok(n) => {
                        out.extend_from_slice(&buf[..n]);
                        got = true;
                    }
                    Err(e) if matches!(e.kind(), ErrorKind::WouldBlock | ErrorKind::Interrupted) => {}
                    Err(_) => break,
                }
                match stderr.read(&mut buf) {
                    Ok(0) => {}
                    Ok(n) => {
                        err.extend_from_slice(&buf[..n]);
                        got = true;
                    }
                    Err(e) if matches!(e.kind(), ErrorKind::WouldBlock | ErrorKind::Interrupted) => {}
                    Err(_) => break,
                }
                if channel.eof() && !got {
                    break;
                }
                if start.elapsed() >= timeout {
                    return Err(ConnectError::Other(anyhow::anyhow!(
                        "命令执行超时（{} 秒）",
                        timeout.as_secs()
                    )));
                }
                if !got {
                    std::thread::sleep(Duration::from_millis(2));
                }
                // 防止单条命令把内存吃穿
                if out.len() + err.len() > 16 * 1024 * 1024 {
                    return Err(ConnectError::Other(anyhow::anyhow!(
                        "命令输出超过 16 MB，请改用终端或重定向到文件"
                    )));
                }
            }

            // 收尾：两条流读完后再切回阻塞等 exit-status。非阻塞下 wait_close 会立刻返回，
            // exit_status 可能还是 libssh2 的默认值 0，会把失败命令误报成成功。
            drop(stdout);
            drop(stderr);
            session.set_blocking(true);
            let _ = channel.close();
            let _ = channel.wait_close();
            let exit_code = channel.exit_status().unwrap_or(-1);
            Ok(ExecResult {
                stdout: String::from_utf8_lossy(&out).to_string(),
                stderr: String::from_utf8_lossy(&err).to_string(),
                exit_code,
            })
        },
    )
    .await;

    match outcome {
        Ok(Ok(r)) => Response::new(200, r, "执行完成"),
        Ok(Err(e)) => connect_failure(e, "执行失败: "),
        Err(e) => Response::new(300, ExecResult::default(), &format!("执行异常: {}", e)),
    }
}
