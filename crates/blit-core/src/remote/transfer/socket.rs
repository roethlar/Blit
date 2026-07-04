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

use socket2::SockRef;
use tokio::net::TcpStream;

/// Apply the data-plane socket policy to a connected or accepted
/// stream, in place (no `into_std`/`from_std` round trip):
///
/// - `TCP_NODELAY` on — **hard error**. Nagle on a data-plane socket
///   silently serializes small records behind ACKs; a socket we cannot
///   configure is a socket we do not use.
/// - `SO_KEEPALIVE` on — best-effort, logged. Keeps idle connections
///   alive during long transfers on sibling streams; the kernel can
///   refuse on exotic socket types (POST_REVIEW_FIXES §1.1).
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
    if let Err(e) = socket.set_keepalive(true) {
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
