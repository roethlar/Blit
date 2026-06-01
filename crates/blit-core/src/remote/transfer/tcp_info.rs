//! Best-effort per-socket TCP statistics for the adaptive stream
//! controller.
//!
//! On Linux the controller reads `TCP_INFO` via `getsockopt(2)` to see
//! retransmits and smoothed RTT — the cleanest "the link is congesting"
//! signal available without a userspace congestion model. Everywhere
//! else the syscall has no portable equivalent, so [`sample_stream`]
//! returns `None` and the controller falls back to its
//! throughput-slope + `write_blocked_nanos` signals (which are
//! cross-platform). Keeping the platform split behind one function lets
//! the controller stay platform-agnostic.

/// A point-in-time read of kernel TCP state for one stream. Fields are
/// cumulative counters / current estimates; the controller diffs
/// successive samples to derive a per-interval retransmit rate.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct TcpInfoSample {
    /// Total retransmitted segments over the life of the connection
    /// (`tcpi_total_retrans`). Monotonic; diff across samples.
    pub total_retransmits: u64,
    /// Smoothed round-trip time in microseconds (`tcpi_rtt`).
    pub rtt_micros: u64,
}

/// Read `TCP_INFO` for `stream`. Returns `None` when the platform has no
/// equivalent or the syscall fails (the controller then leans on its
/// portable signals). Never panics.
#[cfg(target_os = "linux")]
pub fn sample_stream(stream: &tokio::net::TcpStream) -> Option<TcpInfoSample> {
    use std::os::fd::AsRawFd;
    let fd = stream.as_raw_fd();
    // SAFETY: `getsockopt` writes at most `len` bytes into `info`, which
    // is a fully-owned zeroed `tcp_info`; `len` is initialised to its
    // size and updated by the kernel. We read only after a success
    // return. `fd` is borrowed from a live `TcpStream` for the duration
    // of the call.
    let mut info: libc::tcp_info = unsafe { std::mem::zeroed() };
    let mut len = std::mem::size_of::<libc::tcp_info>() as libc::socklen_t;
    let ret = unsafe {
        libc::getsockopt(
            fd,
            libc::IPPROTO_TCP,
            libc::TCP_INFO,
            &mut info as *mut libc::tcp_info as *mut libc::c_void,
            &mut len,
        )
    };
    if ret != 0 {
        return None;
    }
    Some(TcpInfoSample {
        total_retransmits: info.tcpi_total_retrans as u64,
        rtt_micros: info.tcpi_rtt as u64,
    })
}

/// Non-Linux stub: no portable `TCP_INFO`, so the controller uses
/// throughput + `write_blocked_nanos` instead.
#[cfg(not(target_os = "linux"))]
pub fn sample_stream(_stream: &tokio::net::TcpStream) -> Option<TcpInfoSample> {
    None
}

#[cfg(all(test, target_os = "linux"))]
mod tests {
    use super::*;

    /// A live loopback connection should yield a `TCP_INFO` read with a
    /// plausible (non-huge) RTT and zero-ish retransmits. This proves
    /// the `getsockopt` wiring works end-to-end on Linux.
    #[tokio::test]
    async fn samples_live_loopback_socket() {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        let (client, _server) = tokio::join!(
            async { tokio::net::TcpStream::connect(addr).await.unwrap() },
            async { listener.accept().await.unwrap() },
        );
        let sample = sample_stream(&client).expect("TCP_INFO available on loopback");
        // Loopback RTT is microseconds-to-low-milliseconds; assert it's
        // not absurd rather than pinning a value.
        assert!(
            sample.rtt_micros < 5_000_000,
            "loopback rtt should be well under 5s, got {} us",
            sample.rtt_micros
        );
    }
}
