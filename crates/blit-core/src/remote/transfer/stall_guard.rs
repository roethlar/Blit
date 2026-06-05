//! audit-1c / audit-h3: a `StallGuard<R>` `AsyncRead` adapter that turns
//! a *stalled* transfer — no bytes received for `timeout` — into a clean
//! `io::ErrorKind::TimedOut`, while leaving a steadily-progressing
//! transfer untouched.
//!
//! Why an `AsyncRead` adapter and not a `tokio::time::timeout` around the
//! receive call: the receive pipeline reads each wire frame through many
//! separate socket awaits (record tag, file header, length-prefixed
//! fields, file-data streaming, tar shards). A stall can happen at *any*
//! of them, mid-frame. Sitting at the `AsyncRead` layer catches a stall
//! at every read without touching the parsing logic, and — crucially —
//! it is an **idle** timeout (re-armed on every read that makes progress)
//! NOT a total-duration deadline, so a legitimate large transfer that
//! keeps making progress is never aborted. (Owner decision, memory
//! `audit-owner-decisions`: no-bytes-for-30s.)
//!
//! Scope:
//! - audit-1c shipped [`StallGuard`] on the CLI pull-receive TCP path
//!   (the original AsyncRead idle adapter).
//! - audit-h3a extended [`StallGuard`] to the daemon push-receive socket
//!   — another receive path.
//! - audit-h3b adds [`StallGuardWriter`] (this slice), an AsyncWrite
//!   adapter mirroring [`StallGuard`] for **write** progress. The
//!   daemon-side pull data plane is a SENDER (daemon writes bytes to
//!   the puller), so the stall surface is a slow / wedged reader
//!   causing TCP write backpressure on the daemon. `StallGuardWriter`
//!   trips after `TRANSFER_STALL_TIMEOUT` of no successful write
//!   progress, with the same idle-vs-total-deadline semantics as the
//!   read side. The earlier R2/R3 wording for h3b ("daemon pull-data-
//!   plane accepts") was imprecise — the accept + token phases are
//!   already bounded by `PULL_ACCEPT_TIMEOUT` / `PULL_TOKEN_TIMEOUT`;
//!   the missing guard is daemon pull-data-plane **write progress
//!   after token acceptance**, addressed here by wiring this writer
//!   inside `DataPlaneSession`.
//! - audit-h3c is the gRPC-fallback class, re-scoped 2026-06-05 to a
//!   two-slice contract because message-granular timeouts can't be
//!   reused from `StallGuard`'s byte-level model. **Slice 1 shipped**
//!   (structural frame cap + unified receive helper at
//!   `crates/blit-core/src/remote/transfer/grpc_fallback.rs`); **slice
//!   2 pending** (dynamic progress watchdog + retryable `TimedOut`
//!   error). See that module for details.

use std::future::Future;
use std::io;
use std::pin::Pin;
use std::task::{Context, Poll};
use std::time::Duration;

use tokio::io::{AsyncRead, AsyncWrite, ReadBuf};
use tokio::time::{Instant, Sleep};

/// Idle/stall timeout applied to every data-plane transfer path: if no
/// data-plane progress (read or write) is observable for this long, the
/// transfer is aborted with `TimedOut` rather than pinning resources
/// forever. Owner-decided 30s.
///
/// Applied by:
/// - CLI pull-receive TCP (`remote::pull` — audit-1c) via [`StallGuard`].
/// - Daemon push-receive TCP (`daemon::service::push::data_plane`
///   — audit-h3a) via [`StallGuard`].
/// - Daemon pull-data-plane **write progress after token acceptance**
///   (`daemon::service::{pull, pull_sync}` — audit-h3b) via
///   [`StallGuardWriter`] inside `DataPlaneSession`. The accept + token
///   phases on those paths are separately bounded by
///   `PULL_ACCEPT_TIMEOUT` / `PULL_TOKEN_TIMEOUT`.
///
/// The gRPC-fallback paths sit below `tonic::Streaming<T>` rather than
/// `AsyncRead` / `AsyncWrite` and are covered separately (audit-h3c).
pub const TRANSFER_STALL_TIMEOUT: Duration = Duration::from_secs(30);

/// Wraps an `AsyncRead` so a read that makes no progress within `timeout`
/// resolves to `io::ErrorKind::TimedOut`. The deadline is re-armed on
/// every read that returns (data or clean EOF), so it is a per-gap idle
/// timeout, not a cap on the whole transfer.
pub struct StallGuard<R> {
    inner: R,
    timeout: Duration,
    deadline: Pin<Box<Sleep>>,
}

impl<R> StallGuard<R> {
    pub fn new(inner: R, timeout: Duration) -> Self {
        Self {
            inner,
            timeout,
            deadline: Box::pin(tokio::time::sleep(timeout)),
        }
    }
}

impl<R: AsyncRead + Unpin> AsyncRead for StallGuard<R> {
    fn poll_read(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut ReadBuf<'_>,
    ) -> Poll<io::Result<()>> {
        let this = self.get_mut();
        match Pin::new(&mut this.inner).poll_read(cx, buf) {
            Poll::Ready(Ok(())) => {
                // The read completed (delivered bytes, or a clean EOF) —
                // that's progress, so re-arm the idle deadline.
                this.deadline.as_mut().reset(Instant::now() + this.timeout);
                Poll::Ready(Ok(()))
            }
            Poll::Ready(Err(e)) => Poll::Ready(Err(e)),
            Poll::Pending => {
                // No data available yet. Trip only if the whole idle
                // window has elapsed since the last progress; otherwise
                // stay pending (the deadline poll registers our waker).
                match this.deadline.as_mut().poll(cx) {
                    Poll::Ready(()) => Poll::Ready(Err(io::Error::new(
                        io::ErrorKind::TimedOut,
                        format!("transfer stalled: no data received for {:?}", this.timeout),
                    ))),
                    Poll::Pending => Poll::Pending,
                }
            }
        }
    }
}

/// Wraps an `AsyncWrite` so a write that makes no progress within `timeout`
/// resolves to `io::ErrorKind::TimedOut`. The deadline is re-armed on every
/// successful `poll_write` (any byte count > 0 counts as progress), so it
/// is a per-gap idle timeout, not a cap on the whole transfer.
///
/// audit-h3b: the daemon-side pull data plane writes bytes to the puller.
/// If the puller stops reading mid-stream, TCP flow control fills the
/// kernel send buffer and `write_all` blocks indefinitely (until OS-level
/// TCP retransmit exhaustion, often 15+ minutes). Wrapping the inner
/// stream in this adapter turns that into a clean
/// `io::ErrorKind::TimedOut` after `TRANSFER_STALL_TIMEOUT` of no
/// observable write progress.
///
/// Symmetric in spirit with [`StallGuard`] on the read side: same idle-
/// timeout semantics, same load-bearing property that a steadily-
/// progressing transfer (any non-trivial network at all) is never
/// aborted.
pub struct StallGuardWriter<W> {
    inner: W,
    timeout: Duration,
    deadline: Pin<Box<Sleep>>,
}

impl<W> StallGuardWriter<W> {
    pub fn new(inner: W, timeout: Duration) -> Self {
        Self {
            inner,
            timeout,
            deadline: Box::pin(tokio::time::sleep(timeout)),
        }
    }

    pub fn into_inner(self) -> W {
        self.inner
    }
}

impl<W: AsyncWrite + Unpin> AsyncWrite for StallGuardWriter<W> {
    fn poll_write(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &[u8],
    ) -> Poll<io::Result<usize>> {
        let this = self.get_mut();
        match Pin::new(&mut this.inner).poll_write(cx, buf) {
            Poll::Ready(Ok(0)) => {
                // Per the doc contract above, "no progress" means zero
                // bytes accepted. A 0-byte poll_write does NOT reset
                // the deadline — otherwise a peer that accepts zero
                // bytes per poll would never trip the guard. The
                // caller (write_all loop) will keep polling; if real
                // progress doesn't show up within the window the
                // Pending arm below trips. (h3b round 2: GPT review
                // flagged Ok(0) as a doc/code mismatch.)
                Poll::Ready(Ok(0))
            }
            Poll::Ready(Ok(n)) => {
                // n > 0: real progress. Reset the idle deadline so a
                // steadily-progressing transfer is never aborted.
                this.deadline.as_mut().reset(Instant::now() + this.timeout);
                Poll::Ready(Ok(n))
            }
            Poll::Ready(Err(e)) => Poll::Ready(Err(e)),
            Poll::Pending => {
                // The inner stream is back-pressuring (kernel send
                // buffer full → no kernel space available). Trip only
                // if the whole idle window has elapsed since the last
                // progress; otherwise stay pending (the deadline poll
                // registers our waker).
                match this.deadline.as_mut().poll(cx) {
                    Poll::Ready(()) => Poll::Ready(Err(io::Error::new(
                        io::ErrorKind::TimedOut,
                        format!("transfer stalled: no write progress for {:?}", this.timeout),
                    ))),
                    Poll::Pending => Poll::Pending,
                }
            }
        }
    }

    fn poll_flush(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        // Flush is a no-op for most AsyncWrite impls; we don't gate it
        // on the deadline because a stuck flush manifests as a stuck
        // poll_write upstream, which IS gated. Pass through cleanly.
        Pin::new(&mut self.get_mut().inner).poll_flush(cx)
    }

    fn poll_shutdown(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        Pin::new(&mut self.get_mut().inner).poll_shutdown(cx)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::io::{AsyncReadExt, AsyncWriteExt};

    #[tokio::test]
    async fn times_out_when_reader_stalls() {
        // A duplex whose writer half is held open but never written: the
        // read side is perpetually Pending, so the idle deadline fires.
        let (rx, _tx) = tokio::io::duplex(64);
        let mut guard = StallGuard::new(rx, Duration::from_millis(20));
        let mut buf = [0u8; 8];
        let err = guard
            .read(&mut buf)
            .await
            .expect_err("a stalled read must time out");
        assert_eq!(err.kind(), io::ErrorKind::TimedOut, "{err}");
    }

    #[tokio::test]
    async fn passes_data_through_unchanged() {
        let (rx, mut tx) = tokio::io::duplex(64);
        tokio::spawn(async move {
            tx.write_all(b"hello").await.unwrap();
            // tx dropped here → EOF after the 5 bytes.
        });
        let mut guard = StallGuard::new(rx, Duration::from_secs(5));
        let mut buf = [0u8; 5];
        guard
            .read_exact(&mut buf)
            .await
            .expect("data should arrive");
        assert_eq!(&buf, b"hello");
    }

    /// The load-bearing property (owner: idle timeout, NOT a total
    /// deadline): a trickle whose total span exceeds the window must NOT
    /// trip, as long as no single gap exceeds it.
    #[tokio::test]
    async fn does_not_trip_on_steady_trickle_past_total_window() {
        let (rx, mut tx) = tokio::io::duplex(64);
        tokio::spawn(async move {
            for _ in 0..3 {
                tokio::time::sleep(Duration::from_millis(20)).await;
                tx.write_all(b"x").await.unwrap();
            }
            // tx dropped → EOF.
        });
        // 3 writes 20ms apart = ~60ms total, but each gap (20ms) is under
        // the 50ms window, so the idle timeout must not trip.
        let mut guard = StallGuard::new(rx, Duration::from_millis(50));
        let mut buf = Vec::new();
        let n = guard
            .read_to_end(&mut buf)
            .await
            .expect("a steady trickle must not trip the idle timeout");
        assert_eq!(n, 3);
        assert_eq!(buf, b"xxx");
    }

    // ----- audit-h3b: write-side StallGuardWriter tests -----

    /// audit-h3b: a writer whose peer stops reading must trip with
    /// TimedOut once the kernel send buffer fills and no further
    /// write progress is observable. We simulate this with a duplex
    /// whose reader half is held open but never drained — the small
    /// buffer fills on the first write, the second write goes
    /// Pending, and the StallGuardWriter's idle deadline trips.
    #[tokio::test]
    async fn write_times_out_when_reader_stalls() {
        // 64-byte duplex buffer. Reader half is held but never read.
        let (mut guarded, _rx) = {
            let (tx, rx) = tokio::io::duplex(64);
            (StallGuardWriter::new(tx, Duration::from_millis(20)), rx)
        };
        // First write fills the buffer.
        let _ = guarded.write_all(&[0u8; 64]).await;
        // Second write goes Pending because the buffer is full; the
        // StallGuardWriter must surface a TimedOut error from inside
        // the writer's poll_write path.
        let err = guarded
            .write_all(&[0u8; 16])
            .await
            .expect_err("a stalled writer must time out");
        assert_eq!(err.kind(), io::ErrorKind::TimedOut, "{err}");
    }

    /// audit-h3b: an actively-draining peer keeps writes progressing,
    /// so the StallGuardWriter must not trip on a fast healthy
    /// connection.
    #[tokio::test]
    async fn write_passes_data_through_unchanged() {
        let (tx, mut rx) = tokio::io::duplex(64);
        let drain = tokio::spawn(async move {
            let mut buf = Vec::new();
            rx.read_to_end(&mut buf).await.unwrap();
            buf
        });
        let mut guarded = StallGuardWriter::new(tx, Duration::from_secs(5));
        guarded
            .write_all(b"hello world")
            .await
            .expect("writes must succeed on a healthy peer");
        guarded.shutdown().await.expect("shutdown ok");
        drop(guarded); // close the writer half so the reader EOFs.
        let received = drain.await.unwrap();
        assert_eq!(received, b"hello world");
    }

    /// audit-h3b: the load-bearing property — an idle timeout, NOT a
    /// total deadline. A trickle of writes whose total span exceeds
    /// the window must NOT trip, as long as no single gap exceeds it.
    /// Mirrors `does_not_trip_on_steady_trickle_past_total_window`
    /// on the read side.
    #[tokio::test]
    async fn write_does_not_trip_on_steady_trickle_past_total_window() {
        let (tx, mut rx) = tokio::io::duplex(64);
        let drain = tokio::spawn(async move {
            let mut buf = Vec::new();
            rx.read_to_end(&mut buf).await.unwrap();
            buf
        });
        let mut guarded = StallGuardWriter::new(tx, Duration::from_millis(50));
        for _ in 0..3 {
            tokio::time::sleep(Duration::from_millis(20)).await;
            guarded
                .write_all(b"x")
                .await
                .expect("steady-trickle write must not trip");
        }
        guarded.shutdown().await.expect("shutdown ok");
        drop(guarded);
        let received = drain.await.unwrap();
        assert_eq!(received, b"xxx");
    }
}
