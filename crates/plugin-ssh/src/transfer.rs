/*! # SFTP 传输层（P1-C）

`remote_conn.rs` 负责会话与对外命令，本模块只负责「把一份文件从一端搬到另一端」。
上传与下载共用同一套：取消标志、断点续传、滑动窗口测速、进度事件节流。
这样可以避免历史上「下载有测速、上传没有」「两条链路取消行为不一致」的分叉。

三条硬约束：

1. **取消是协作式的**：`CANCELLED` 表按 `taskId` 存标志，传输循环每 64KB 检查一次。
   前端重试复用同一个 taskId，因此每次开传先 `clear_cancel`；取消按钮只对
   `transferring` 状态开放，不存在「还没开传就点取消」的竞态。
2. **续传只在「目标已存在且比源文件小」时生效**。上传查远端 stat，下载查本地
   metadata；两端各自 seek 到同一 offset。目标不存在 / 已完整则退化为全量传输。
3. **进度事件带 totalSize**，前端不必再猜大小；只在整数百分比变化时 emit，
   大文件不会产生上万条 IPC（会把命令响应挤到消息队列尾部）。
*/
use anyhow::Result;
use bdevtools_core::ctx::AppContext;
use bdevtools_core::response::response::Response;
use ssh2::{Session, Sftp};
use std::collections::{HashSet, VecDeque};
use std::fs::{File, OpenOptions};
use std::io::{Read, Seek, SeekFrom, Write};
use std::path::Path;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

/// 用户主动取消 / 暂停：前端把任务标成 cancelled / paused，不当作失败重试
pub const TRANSFER_CANCELLED: usize = 499;

lazy_static::lazy_static! {
    static ref CANCELLED: Mutex<HashSet<String>> = Mutex::new(HashSet::new());
}

pub fn request_cancel(task_id: &str) {
    CANCELLED.lock().unwrap().insert(task_id.to_string());
}

pub fn clear_cancel(task_id: &str) {
    CANCELLED.lock().unwrap().remove(task_id);
}

fn is_cancelled(task_id: &str) -> bool {
    CANCELLED.lock().unwrap().contains(task_id)
}

pub async fn sftp_cancel_transfer(task_id: String) -> Response<bool> {
    if task_id.is_empty() {
        return Response::new(300, false, "缺少 taskId");
    }
    request_cancel(&task_id);
    Response::new(200, true, "已请求取消")
}

/// 一次传输的结束原因
#[derive(Debug, PartialEq, Eq)]
pub enum StopReason {
    Done,
    Cancelled,
}

/// 3 秒滑动窗口测速。上传和下载都必须用它，不能只给下载加速度显示。
struct SpeedMeter {
    start: Instant,
    window: VecDeque<(Instant, u64)>,
    written: u64,
}

impl SpeedMeter {
    fn new() -> Self {
        Self { start: Instant::now(), window: VecDeque::new(), written: 0 }
    }

    fn record(&mut self, bytes: u64) -> f64 {
        let now = Instant::now();
        self.written += bytes;
        self.window.push_back((now, bytes));
        while let Some(&(t, _)) = self.window.front() {
            if now - t > Duration::from_secs(3) {
                self.window.pop_front();
            } else {
                break;
            }
        }
        if self.window.len() >= 2 {
            let total: u64 = self.window.iter().map(|(_, b)| *b).sum();
            let span = (self.window.back().unwrap().0 - self.window.front().unwrap().0).as_secs_f64();
            if span > 0.0 { return total as f64 / span; }
        }
        self.average()
    }

    /// 整体平均速度（窗口样本不足或收尾时用）
    fn average(&self) -> f64 {
        let elapsed = self.start.elapsed().as_secs_f64();
        if elapsed > 0.0 { self.written as f64 / elapsed } else { 0.0 }
    }
}

fn emit_progress(
    ctx: &AppContext,
    task_id: &str,
    progress: u32,
    transferred: u64,
    total: u64,
    speed: String,
) {
    ctx.emit(
        "transfer-progress",
        serde_json::json!({
            "tabId": "",
            "taskId": task_id,
            "progress": progress,
            "transferredSize": transferred,
            "totalSize": total,
            "speed": speed
        }),
    );
}

/// 计算目标端已写入的字节数（严格小于源大小才允许续传，避免接到一个更新的文件后面）
fn resume_offset(existing: Option<u64>, total: u64) -> u64 {
    match existing {
        Some(n) if n > 0 && n < total => n,
        _ => 0,
    }
}

/// 上传：本地 → 远端
#[allow(clippy::too_many_arguments)]
pub fn upload(
    ctx: &AppContext,
    sess: Arc<Session>,
    local_path: &str,
    remote_path: &str,
    task_id: &str,
    resume: bool,
) -> Result<StopReason> {
    clear_cancel(task_id);
    let sftp = sess.sftp()?;
    let local = Path::new(local_path);
    let remote = Path::new(remote_path);
    let total = std::fs::metadata(local)?.len();

    let existing = if resume {
        sftp.stat(remote).ok().and_then(|st| st.size)
    } else {
        None
    };
    let offset = resume_offset(existing, total);

    let mut remote_file = if offset > 0 {
        // 不带 TRUNCATE / CREATE：已存在的文件从 offset 继续写
        let mut f = sftp.open_mode(remote, ssh2::OpenFlags::WRITE, 0o644, ssh2::OpenType::File)?;
        f.seek(SeekFrom::Start(offset))?;
        f
    } else {
        sftp.create(remote)?
    };
    let mut local_file = File::open(local)?;
    if offset > 0 {
        local_file.seek(SeekFrom::Start(offset))?;
    }

    let mut buffer = [0u8; 65536];
    let mut transferred = offset;
    let mut last_progress: i64 = -1;
    let mut meter = SpeedMeter::new();

    loop {
        if is_cancelled(task_id) {
            return Ok(StopReason::Cancelled);
        }
        let n = local_file.read(&mut buffer)?;
        if n == 0 {
            break;
        }
        remote_file.write_all(&buffer[..n])?;
        transferred += n as u64;
        let speed = meter.record(n as u64);
        let progress = if total > 0 {
            ((transferred as f64 / total as f64) * 100.0).min(100.0) as u32
        } else {
            100
        };
        if (progress as i64) != last_progress {
            last_progress = progress as i64;
            emit_progress(
                ctx,
                task_id,
                progress,
                transferred,
                total,
                crate::remote_conn::format_speed(speed),
            );
        }
    }
    // 收尾：节流可能刚好跳过 100，最终值必须送达
    emit_progress(ctx, task_id, 100, transferred, total, crate::remote_conn::format_speed(meter.average()));
    Ok(StopReason::Done)
}

/// 下载：远端 → 本地
#[allow(clippy::too_many_arguments)]
pub fn download(
    ctx: &AppContext,
    sess: Arc<Session>,
    remote_path: &str,
    local_path: &str,
    task_id: &str,
    resume: bool,
) -> Result<StopReason> {
    clear_cancel(task_id);
    let sftp = sess.sftp()?;
    let remote = Path::new(remote_path);
    let stat = sftp.stat(remote)?;
    let total = stat.size.unwrap_or(0);

    let existing = if resume {
        std::fs::metadata(local_path).ok().map(|m| m.len())
    } else {
        None
    };
    let offset = resume_offset(existing, total);

    let mut remote_file = sftp.open(remote)?;
    if offset > 0 {
        remote_file.seek(SeekFrom::Start(offset))?;
    }
    let mut local_file = if offset > 0 {
        OpenOptions::new().append(true).open(local_path)?
    } else {
        File::create(local_path)?
    };

    let mut buffer = [0u8; 65536];
    let mut transferred = offset;
    let mut last_progress: i64 = -1;
    let mut meter = SpeedMeter::new();

    loop {
        if is_cancelled(task_id) {
            return Ok(StopReason::Cancelled);
        }
        let n = remote_file.read(&mut buffer)?;
        if n == 0 {
            break;
        }
        local_file.write_all(&buffer[..n])?;
        transferred += n as u64;
        let speed = meter.record(n as u64);
        let progress = if total > 0 {
            ((transferred as f64 / total as f64) * 100.0).min(100.0) as u32
        } else {
            100
        };
        if (progress as i64) != last_progress {
            last_progress = progress as i64;
            emit_progress(
                ctx,
                task_id,
                progress,
                transferred,
                total,
                crate::remote_conn::format_speed(speed),
            );
        }
    }
    emit_progress(ctx, task_id, 100, transferred, total, crate::remote_conn::format_speed(meter.average()));
    Ok(StopReason::Done)
}

/// 修改远端文件权限。`setstat` 会把 stat 里所有「有值」的字段一并下发，
/// 因此先清空 size/uid/gid/时间，只留 permissions，避免误改文件大小与时间戳。
/// 权限里的文件类型位必须保留，否则服务器可能把它当成普通位组合。
pub fn apply_chmod(sftp: &Sftp, path: &Path, mode: u32) -> Result<()> {
    let mut st = sftp.stat(path)?;
    let type_bits = st.perm.unwrap_or(0) & 0o170000;
    st.size = None;
    st.uid = None;
    st.gid = None;
    st.atime = None;
    st.mtime = None;
    st.perm = Some(type_bits | (mode & 0o7777));
    sftp.setstat(path, st)?;
    Ok(())
}
