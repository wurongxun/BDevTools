/*! # TCP ↔ SSH channel 双向中继

端口转发（-L/-R/-D）与 ProxyJump 回环桥共用这一段。

**为什么是「单线程 nonblocking 轮询」而不是两线程 `io::copy`：**
ssh2 0.9.6 的 `Stream::read` 在读取期间一直持有 `MutexGuard<SessionInner>`；
blocking 模式下这个锁会一直等到对端来数据，写线程拿不到同一把锁 ⇒
转发看起来「连上了但不通」。把 session 置为非阻塞后 read 立即返回 WouldBlock，
单线程交替读写即可，既无锁竞争，也不需要 clone `Channel`（它不实现 Clone）。
*/
use std::collections::VecDeque;
use std::io::{ErrorKind, Read, Write};
use std::net::{Shutdown, TcpStream};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Arc;
use std::time::Duration;

/// 单个转发任务累计流量（字节）。UI 轮询 `ssh_tunnel_list` 时读取。
#[derive(Default)]
pub struct RelayStats {
    pub bytes_up: AtomicU64,
    pub bytes_down: AtomicU64,
}

const MAX_BUFFER: usize = 1024 * 1024;
const CHUNK: usize = 16 * 1024;

/// 把一条本地 TCP 连接与一个 SSH channel 双向打通，直到任一端关闭或 stop 置位。
///
/// 调用前必须：① session 已 `set_blocking(false)`；② local socket 上的应用层握手
/// （如 SOCKS5 greeting）已完成。本函数会把 tcp 设成 nonblocking。
pub fn relay_pair(
    mut tcp: TcpStream,
    mut chan: ssh2::Channel,
    stop: Arc<AtomicBool>,
    stats: Option<Arc<RelayStats>>,
) {
    let _ = tcp.set_nodelay(true);
    let _ = tcp.set_nonblocking(true);
    let mut up: VecDeque<u8> = VecDeque::new();
    let mut down: VecDeque<u8> = VecDeque::new();
    let mut buf = [0u8; CHUNK];
    let mut tcp_open = true;
    let mut chan_open = true;

    while !stop.load(Ordering::Relaxed) {
        let mut progressed = false;

        // 本地 → 远端
        if tcp_open && up.len() < MAX_BUFFER {
            match tcp.read(&mut buf) {
                Ok(0) => tcp_open = false,
                Ok(n) => {
                    up.extend(&buf[..n]);
                    if let Some(s) = &stats {
                        s.bytes_up.fetch_add(n as u64, Ordering::Relaxed);
                    }
                    progressed = true;
                }
                Err(e) if e.kind() == ErrorKind::WouldBlock => {}
                Err(e) if e.kind() == ErrorKind::Interrupted => {}
                Err(_) => tcp_open = false,
            }
        }
        // 远端 → 本地
        if chan_open && down.len() < MAX_BUFFER {
            match chan.read(&mut buf) {
                Ok(0) => {
                    // libssh2 在非阻塞下偶尔以 0 表示「暂时无数据」，只有 eof 才是真关闭
                    if chan.eof() {
                        chan_open = false;
                    }
                }
                Ok(n) => {
                    down.extend(&buf[..n]);
                    if let Some(s) = &stats {
                        s.bytes_down.fetch_add(n as u64, Ordering::Relaxed);
                    }
                    progressed = true;
                }
                Err(e) if e.kind() == ErrorKind::WouldBlock => {}
                Err(e) if e.kind() == ErrorKind::Interrupted => {}
                Err(_) => chan_open = false,
            }
        }

        // 冲刷缓冲：非阻塞写可能只写一部分，用 drain(..n) 保留未发完的字节
        if !up.is_empty() && chan_open {
            let (a, b) = up.as_slices();
            let head = if a.is_empty() { b } else { a };
            match chan.write(head) {
                Ok(0) => {}
                Ok(n) => {
                    up.drain(..n);
                    progressed = true;
                }
                Err(e) if e.kind() == ErrorKind::WouldBlock => {}
                Err(e) if e.kind() == ErrorKind::Interrupted => {}
                Err(_) => chan_open = false,
            }
            let _ = chan.flush();
        }
        if !down.is_empty() && tcp_open {
            let (a, b) = down.as_slices();
            let head = if a.is_empty() { b } else { a };
            match tcp.write(head) {
                Ok(0) => {}
                Ok(n) => {
                    down.drain(..n);
                    progressed = true;
                }
                Err(e) if e.kind() == ErrorKind::WouldBlock => {}
                Err(e) if e.kind() == ErrorKind::Interrupted => {}
                Err(_) => tcp_open = false,
            }
            let _ = tcp.flush();
        }

        // 一端已关但缓冲还没发完时继续冲刷，否则立即退出
        if (!tcp_open && up.is_empty()) || (!chan_open && down.is_empty()) {
            break;
        }
        if !tcp_open && !chan_open {
            break;
        }
        if !progressed {
            std::thread::sleep(Duration::from_millis(2));
        }
    }

    let _ = tcp.shutdown(Shutdown::Both);
    let _ = chan.send_eof();
    let _ = chan.close();
}
