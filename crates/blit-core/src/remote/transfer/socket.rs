//! Shared socket policy for data-plane TCP streams (w1-2).
//!
//! Every data-plane socket — the push client connect, the pull client
//! connect, and all daemon accept paths (push epoch-0/resize,
//! pull_sync epoch-0/resize/resume) — routes through
//! [`configure_data_socket`], the single owner of the
//! NODELAY/keepalive/tuned-buffer policy. Before this module the
//! policy existed on push sockets only; the pull direction ran with
//! Nagle enabled and the tuner's `tcp_buffer_bytes` was computed and
//! discarded (design map §1.1, finding
//! boundaries-pull-direction-bypasses-socket-policy).
//!
//! Connect *timeouts* are deliberately not owned here: this helper
//! configures an already-established stream. The missing data-plane
//! connect bounds are design-3's slice and live at the call sites.

use std::io;
use std::time::Duration;

use socket2::{SockRef, TcpKeepalive};
use tokio::net::TcpStream;

/// Idle time before the first keepalive probe (w1-3). Before this the
/// sockets ran `SO_KEEPALIVE` with OS-default timing (~2 h idle on
/// every supported platform) — useless on transfer timescales, while
/// the comments claimed it prevented idle stream timeouts. With
/// 60 s + 5 probes at 10 s, a vanished peer on an idle data socket
/// (an armed resize slot, a stream waiting for work while siblings
/// transfer) is detected in ~2 minutes. The complementary case — a
/// stalled peer with data in flight — is StallGuard's 30 s, not
/// keepalive's.
pub const TCP_KEEPALIVE_IDLE: Duration = Duration::from_secs(60);
/// Interval between keepalive probes once idle has elapsed.
pub const TCP_KEEPALIVE_INTERVAL: Duration = Duration::from_secs(10);
/// Unanswered probes before the connection is declared dead.
pub const TCP_KEEPALIVE_RETRIES: u32 = 5;

/// Apply the data-plane socket policy to a connected or accepted
/// stream, in place (no `into_std`/`from_std` round trip):
///
/// - `TCP_NODELAY` on — **hard error**. Nagle on a data-plane socket
///   silently serializes small records behind ACKs; a socket we cannot
///   configure is a socket we do not use.
/// - `SO_KEEPALIVE` on with explicit timing
///   ([`TCP_KEEPALIVE_IDLE`]/[`TCP_KEEPALIVE_INTERVAL`]/
///   [`TCP_KEEPALIVE_RETRIES`]) — best-effort, logged. Detects a
///   vanished peer on an idle data socket within ~2 minutes instead of
///   the OS-default ~2 hours; the kernel can refuse on exotic socket
///   types (POST_REVIEW_FIXES §1.1 lineage — failure is loud, never
///   fatal).
/// - Send/receive buffers sized to `tcp_buffer_size` when `Some` —
///   best-effort, logged. The knobs are advisory (the kernel can
///   clamp); a failure here should be visible to operators chasing a
///   sysctl/rlimit mismatch, never fatal. `None` = kernel default —
///   the value is a connect-time snapshot of
///   [`TransferDial::tcp_buffer_bytes`](crate::engine::TransferDial::tcp_buffer_bytes)
///   where a dial is in scope (epoch-0 sockets therefore run kernel
///   defaults; resize-ADD sockets get the ramped size), and `None`
///   where none is (the pull client and the daemon push receiver hold
///   no dial).
///
/// Errors only if `TCP_NODELAY` cannot be set (or the fd/socket
/// handle is unusable, which the same call surfaces).
pub fn configure_data_socket(stream: &TcpStream, tcp_buffer_size: Option<usize>) -> io::Result<()> {
    let socket = SockRef::from(stream);
    socket.set_tcp_nodelay(true)?;
    // `set_tcp_keepalive` also flips SO_KEEPALIVE on, so this is the
    // whole keepalive story in one call.
    let keepalive = TcpKeepalive::new()
        .with_time(TCP_KEEPALIVE_IDLE)
        .with_interval(TCP_KEEPALIVE_INTERVAL)
        .with_retries(TCP_KEEPALIVE_RETRIES);
    if let Err(e) = socket.set_tcp_keepalive(&keepalive) {
        log::warn!("set TCP keepalive on data-plane socket: {}", e);
    }
    if let Some(size) = tcp_buffer_size {
        if let Err(e) = socket.set_send_buffer_size(size) {
            log::warn!("set TCP send buffer to {} bytes: {}", size, e);
        }
        if let Err(e) = socket.set_recv_buffer_size(size) {
            log::warn!("set TCP recv buffer to {} bytes: {}", size, e);
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::net::TcpListener;

    async fn loopback_pair() -> (TcpStream, TcpStream) {
        let listener = TcpListener::bind("127.0.0.1:0").await.expect("bind");
        let addr = listener.local_addr().expect("addr");
        let (client, accepted) = tokio::join!(TcpStream::connect(addr), listener.accept());
        let (server, _) = accepted.expect("accept");
        (client.expect("connect"), server)
    }

    /// The full policy lands on the socket: nodelay and keepalive read
    /// back true, and both buffer directions honor (at least) the
    /// requested size — kernels may round up (Linux doubles), never
    /// silently ignore a size this small.
    #[tokio::test]
    async fn applies_nodelay_keepalive_and_buffers() {
        let (client, _server) = loopback_pair().await;
        let requested = 256 * 1024;
        configure_data_socket(&client, Some(requested)).expect("configure");

        let sock = SockRef::from(&client);
        assert!(
            sock.tcp_nodelay().expect("read nodelay"),
            "TCP_NODELAY must be on"
        );
        assert!(
            sock.keepalive().expect("read keepalive"),
            "SO_KEEPALIVE must be on"
        );
        assert!(
            sock.send_buffer_size().expect("read sndbuf") >= requested,
            "send buffer must be at least the requested size"
        );
        assert!(
            sock.recv_buffer_size().expect("read rcvbuf") >= requested,
            "recv buffer must be at least the requested size"
        );
    }

    /// w1-3: the keepalive is configured with explicit timing, not
    /// just switched on — OS-default timing (~2 h idle) is useless on
    /// transfer timescales. Read back through the kernel so the test
    /// pins what a peer actually experiences, not what we asked for.
    /// The socket2 getters are unix-only; Windows exercises the set
    /// path through every other test in this module.
    #[cfg(unix)]
    #[tokio::test]
    async fn keepalive_timing_is_explicit() {
        let (client, _server) = loopback_pair().await;
        configure_data_socket(&client, None).expect("configure");

        let sock = SockRef::from(&client);
        assert_eq!(
            sock.tcp_keepalive_time().expect("read keepalive time"),
            TCP_KEEPALIVE_IDLE,
            "idle time before the first probe must be the policy value"
        );
        assert_eq!(
            sock.tcp_keepalive_interval()
                .expect("read keepalive interval"),
            TCP_KEEPALIVE_INTERVAL,
            "probe interval must be the policy value"
        );
        assert_eq!(
            sock.tcp_keepalive_retries()
                .expect("read keepalive retries"),
            TCP_KEEPALIVE_RETRIES,
            "probe retry count must be the policy value"
        );
    }

    /// `None` = kernel-default buffers: nodelay/keepalive still land,
    /// and the buffer sizes are left exactly where the kernel put them.
    #[tokio::test]
    async fn none_leaves_kernel_default_buffers() {
        let (client, _server) = loopback_pair().await;
        let sock = SockRef::from(&client);
        let default_send = sock.send_buffer_size().expect("read sndbuf");
        let default_recv = sock.recv_buffer_size().expect("read rcvbuf");

        configure_data_socket(&client, None).expect("configure");

        assert!(sock.tcp_nodelay().expect("read nodelay"));
        assert!(sock.keepalive().expect("read keepalive"));
        assert_eq!(
            sock.send_buffer_size().expect("read sndbuf"),
            default_send,
            "None must not touch the send buffer"
        );
        assert_eq!(
            sock.recv_buffer_size().expect("read rcvbuf"),
            default_recv,
            "None must not touch the recv buffer"
        );
    }
}
