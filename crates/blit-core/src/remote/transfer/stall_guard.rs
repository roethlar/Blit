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
//! Scope: audit-1c shipped this guard on the CLI pull-receive TCP path.
//! audit-h3 (R2/R3 finding H3) extends it to the symmetric receive paths
//! audit-1c missed — daemon push-receive (h3a, this slice) and daemon
//! pull-data-plane accepts (h3b). The gRPC-fallback receive (h3c) is
//! separately scoped because it sits below `Streaming<T>` rather than
//! `AsyncRead`.

use std::future::Future;
use std::io;
use std::pin::Pin;
use std::task::{Context, Poll};
use std::time::Duration;

use tokio::io::{AsyncRead, ReadBuf};
use tokio::time::{Instant, Sleep};

/// Idle/stall timeout applied to a transfer's receive path: if no bytes
/// arrive for this long, the transfer is aborted with `TimedOut` rather
/// than pinning resources forever. Owner-decided 30s, every receive path.
///
/// Applied by:
/// - CLI pull-receive TCP (`remote::pull` — audit-1c)
/// - Daemon push-receive TCP (`daemon::service::push::data_plane` —
///   audit-h3a)
/// - Daemon pull-data-plane accepts
///   (`daemon::service::{pull, pull_sync}` — audit-h3b)
///
/// The gRPC-fallback receive paths sit below `tonic::Streaming<T>` rather
/// than an `AsyncRead` and are covered separately (audit-h3c).
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
}
