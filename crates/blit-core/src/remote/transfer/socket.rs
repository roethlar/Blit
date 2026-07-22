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
//! design-3 added [`dial_data_plane`]: the client-side dial (bounded
//! connect + policy + bounded handshake write) lives here too, so
//! both data-plane connect sites share one owner and neither can
//! regress to an unbounded `TcpStream::connect`.

use std::io;
use std::time::Duration;

use eyre::Context as _;
use socket2::{SockRef, TcpKeepalive};
use tokio::io::{AsyncWrite, AsyncWriteExt};
use tokio::net::TcpStream;

/// Bounded wait for a data-plane accept (w1-4: one shared pair — this
/// and [`DATA_PLANE_TOKEN_TIMEOUT`] — replacing three per-file
/// declarations of the same two values). R46-F7 lineage: pre-fix the
/// daemon called `listener.accept().await` with no timeout — a peer
/// that opened the control connection but never opened the data
/// connection (or hung mid-handshake) would pin the daemon's stream
/// task indefinitely, holding the listener and the queued work. 30 s
/// gives a generous margin for slow networks while still bounding the
/// worst case.
pub const DATA_PLANE_ACCEPT_TIMEOUT: Duration = Duration::from_secs(30);
/// Bounded wait for the handshake-token bytes after a TCP accept.
/// R46-F7: pre-fix `read_exact(&mut token_buf).await` had no timeout —
/// a peer that opened the socket and stalled would hold the stream
/// worker forever. 15 s is enough for a healthy peer to send a few
/// dozen bytes; anything slower is a stuck or hostile peer.
pub const DATA_PLANE_TOKEN_TIMEOUT: Duration = Duration::from_secs(15);

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
///   [`TransferDial::tcp_buffer_bytes`](crate::dial::TransferDial::tcp_buffer_bytes)
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

/// design-3: dial a data-plane endpoint with the shared bounds — the
/// client-side mirror of the daemon's bounded accept. Connect is
/// bounded by [`DATA_PLANE_ACCEPT_TIMEOUT`] (the audit-2 wave bounded
/// every control-plane connect at the same 30 s but never reached the
/// TCP data plane: a firewalled or black-holed data port — the daemon
/// advertises a fresh ephemeral port per transfer, and asymmetric
/// firewalls that pass the control port but block ephemerals are
/// common — hung for the kernel SYN timeout, 60–127 s, with no
/// message). The handshake-token write is bounded by
/// [`DATA_PLANE_TOKEN_TIMEOUT`], mirroring the acceptor's bounded
/// token read. Applies [`configure_data_socket`] in between.
///
/// On timeout the error chain carries an `io::ErrorKind::TimedOut`
/// source so `remote::retry::is_retryable` classifies it as a
/// transient transport failure (`--retry` re-dials instead of giving
/// up on a deterministic-looking error).
pub async fn dial_data_plane(
    addr: &str,
    handshake: &[u8],
    tcp_buffer_size: Option<usize>,
) -> eyre::Result<TcpStream> {
    dial_data_plane_with_timeouts(
        addr,
        handshake,
        tcp_buffer_size,
        DATA_PLANE_ACCEPT_TIMEOUT,
        DATA_PLANE_TOKEN_TIMEOUT,
    )
    .await
}

/// Timeout-parameterized core of [`dial_data_plane`], so tests can pin
/// the bounded-failure shape without waiting out the production 30 s.
async fn dial_data_plane_with_timeouts(
    addr: &str,
    handshake: &[u8],
    tcp_buffer_size: Option<usize>,
    connect_timeout: Duration,
    token_timeout: Duration,
) -> eyre::Result<TcpStream> {
    let mut stream = match tokio::time::timeout(connect_timeout, TcpStream::connect(addr)).await {
        Ok(connected) => connected.with_context(|| format!("connecting data plane {addr}"))?,
        Err(_) => {
            return Err(eyre::Report::new(io::Error::new(
                io::ErrorKind::TimedOut,
                format!("connect did not complete within {connect_timeout:?}"),
            ))
            .wrap_err(format!(
                "data-plane connect to {addr} timed out after {connect_timeout:?} — the \
                 port is likely unreachable (the daemon advertises a fresh ephemeral \
                 data port per transfer; a firewall that passes the control port but \
                 blocks ephemeral ports produces exactly this failure)"
            )));
        }
    };
    configure_data_socket(&stream, tcp_buffer_size).context("setting TCP_NODELAY")?;
    write_handshake_with_timeout(&mut stream, handshake, addr, token_timeout).await?;
    Ok(stream)
}

/// Write the data-plane handshake under the production timeout policy.
///
/// Keeping the timeout around a generic async writer lets the guard use a
/// bounded in-memory pipe whose capacity is known, instead of guessing how
/// much a particular operating system will buffer on loopback. Production
/// still calls this exact helper with its configured [`TcpStream`].
async fn write_handshake_with_timeout<W>(
    writer: &mut W,
    handshake: &[u8],
    addr: &str,
    token_timeout: Duration,
) -> eyre::Result<()>
where
    W: AsyncWrite + Unpin,
{
    match tokio::time::timeout(token_timeout, writer.write_all(handshake)).await {
        Ok(written) => {
            written.with_context(|| format!("writing data-plane handshake token to {addr}"))?
        }
        Err(_) => {
            return Err(eyre::Report::new(io::Error::new(
                io::ErrorKind::TimedOut,
                format!("handshake write did not complete within {token_timeout:?}"),
            ))
            .wrap_err(format!(
                "data-plane handshake to {addr} stalled for {token_timeout:?} — the peer \
                 accepted the connection but is not reading"
            )));
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

    // ── design-3: bounded dial ────────────────────────────────────

    fn chain_has_timed_out(err: &eyre::Report) -> bool {
        err.chain().any(|cause| {
            cause
                .downcast_ref::<io::Error>()
                .is_some_and(|io_err| io_err.kind() == io::ErrorKind::TimedOut)
        })
    }

    /// Happy path: the dial connects, applies the socket policy, and
    /// delivers the handshake bytes to the peer.
    #[tokio::test]
    async fn dial_connects_applies_policy_and_sends_handshake() {
        let listener = TcpListener::bind("127.0.0.1:0").await.expect("bind");
        let addr = listener.local_addr().expect("addr").to_string();

        let (dialed, accepted) = tokio::join!(
            dial_data_plane_with_timeouts(
                &addr,
                b"tok-123",
                None,
                Duration::from_secs(5),
                Duration::from_secs(5),
            ),
            listener.accept(),
        );
        let stream = dialed.expect("dial succeeds");
        let (mut server, _) = accepted.expect("accept");

        assert!(
            SockRef::from(&stream).tcp_nodelay().expect("read nodelay"),
            "dial must apply the shared socket policy"
        );
        let mut buf = [0u8; 7];
        tokio::io::AsyncReadExt::read_exact(&mut server, &mut buf)
            .await
            .expect("handshake bytes arrive");
        assert_eq!(&buf, b"tok-123");
    }

    /// A stalled handshake (peer holds the reader open but never reads)
    /// fails within the token bound — with
    /// `io::ErrorKind::TimedOut` in the chain so the retry classifier
    /// treats it as transient. The one-byte in-memory capacity makes
    /// the two-byte `write_all` block deterministically on every OS;
    /// production uses this exact timeout helper with its TCP stream.
    #[tokio::test]
    async fn dial_token_write_stall_times_out_bounded_and_retryable() {
        let (mut writer, _held_reader) = tokio::io::duplex(1);
        let start = std::time::Instant::now();
        let err = write_handshake_with_timeout(
            &mut writer,
            &[0xA5, 0x5A],
            "deterministic-test-writer",
            Duration::from_millis(50),
        )
        .await
        .expect_err("stalled handshake must time out");
        assert!(
            start.elapsed() < Duration::from_secs(2),
            "failure must arrive within the bound, not the OS timeout"
        );
        assert!(
            chain_has_timed_out(&err),
            "chain must carry io::ErrorKind::TimedOut: {err:#}"
        );
        assert!(
            crate::remote::retry::is_retryable(&err),
            "a dial timeout is a transient transport failure"
        );
    }

    /// A black-holed connect fails within the connect bound instead of
    /// hanging for the kernel SYN timeout (60–127 s). RFC 5737
    /// TEST-NET-1 is reserved and never routable; most stacks
    /// black-hole it (timeout arm — assert the TimedOut chain), some
    /// networks reject it fast (unreachable — the dial still failed
    /// bounded, which is the invariant under test either way).
    #[tokio::test]
    async fn dial_connect_to_black_hole_fails_within_bound() {
        let start = std::time::Instant::now();
        let result = dial_data_plane_with_timeouts(
            "192.0.2.1:9",
            b"tok",
            None,
            Duration::from_millis(250),
            Duration::from_millis(250),
        )
        .await;
        let elapsed = start.elapsed();
        let err = result.expect_err("TEST-NET dial must fail");
        assert!(
            elapsed < Duration::from_secs(10),
            "dial must fail within the bound (took {elapsed:?}) — an unbounded \
             connect rides the kernel SYN timeout for 60-127s"
        );
        // Only the black-hole arm produces our TimedOut shape; a fast
        // OS rejection (some networks) is also a bounded failure.
        if elapsed >= Duration::from_millis(250) {
            assert!(
                chain_has_timed_out(&err),
                "black-holed connect must surface the timeout shape: {err:#}"
            );
            assert!(crate::remote::retry::is_retryable(&err));
        }
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
