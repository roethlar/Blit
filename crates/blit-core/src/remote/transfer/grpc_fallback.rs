//! audit-h3c slice 1: gRPC fallback frame contract.
//!
//! The TCP data plane observes byte-level progress via `AsyncRead::poll_read`
//! — every `poll_read` event resets the [`super::stall_guard::StallGuard`]
//! deadline, so a steady byte trickle on a slow link keeps the guard fresh.
//!
//! The gRPC fallback path cannot do that. `tonic::Streaming::message()`
//! only resolves after a full protobuf message is decoded; nothing inside
//! the message is observable from the receive side. So if the daemon
//! sends TCP-sized chunks (16-64 MiB from [`crate::remote::tuning`]) over
//! the gRPC fallback, a 1 Mbps link takes ~128 seconds per `message()`
//! await — defeating any per-message stall guard, and giving the slice-2
//! progress watchdog nothing observable to measure.
//!
//! The fix is to size gRPC fallback frames separately from the TCP
//! tuning, capped to a value small enough that even a deeply slow link
//! still produces messages at observable cadence. That's
//! [`GRPC_FALLBACK_CHUNK_BYTES`] below.
//!
//! Slice 1 (this slice) caps the chunk size at the send sinks
//! (`GrpcFallbackSink`, `GrpcServerStreamingSink`) and routes the
//! CLI-side pull-receive sites named by the audit chain through
//! [`recv_fallback_message`]. The helper is a thin pass-through today;
//! slice 2 will add the dynamic progress watchdog.
//!
//! Whether slice 2 can be a single-file change inside this module
//! depends on whether the watchdog needs per-call cadence state. If it
//! does, the helper signature will likely grow (e.g. an extra
//! `&mut FallbackRecvState` parameter or a wrapper struct constructed
//! at the call sites), which is a small touch at each of the three
//! sites. That's acceptable scope creep for slice 2 — the slice-1
//! payoff is the named chokepoint, not perfect future-proofing.
//!
//! ## In scope for slice 1
//!
//! - `crates/blit-core/src/remote/pull.rs:316` (plain `pull()` entry —
//!   CLI receives `PullChunk`s from the daemon).
//! - `crates/blit-core/src/remote/pull.rs:484` (`scan_remote_files` —
//!   metadata-only force-gRPC scan).
//! - `crates/blit-core/src/remote/pull.rs:752` (`pull_sync_with_spec`
//!   — the load-bearing gRPC-fallback control + data loop named by
//!   GPT-12 / R3 §H3).
//! - Send-side cap: `GrpcFallbackSink` (push, client → daemon) and
//!   `GrpcServerStreamingSink` (pull, daemon → client) in
//!   `crates/blit-core/src/remote/transfer/sink.rs`.
//! - Defense-in-depth cap: `transfer_payloads_via_control_plane` in
//!   `crates/blit-core/src/remote/transfer/payload.rs` (dead-code today
//!   but pub-reachable from external crates).
//!
//! ## Out of scope for slice 1 / 2 (named explicitly to prevent drift)
//!
//! - `crates/blit-core/src/remote/pull.rs:~1210` — `RemoteFileStream`'s
//!   `poll_read` consumes pull-stream messages via `Stream::poll_next`
//!   inside an `AsyncRead` impl. Structurally different from the bare
//!   `Streaming::message().await` shape; needs its own adapter and is
//!   not on the audit chain's named h3c surface.
//! - `crates/blit-core/src/remote/push/client/helpers.rs:245` — CLI
//!   push response forwarder (`spawn_response_task`). Symmetric PUSH
//!   analog; tracked as a potential h3-sibling slice if owner extends
//!   coverage.
//! - `crates/blit-app/src/transfers/remote.rs:734, :878` — app-side
//!   delegated-pull progress consumer. Same DoS pattern but a different
//!   surface (control stream observability) and not on the h3c chain.
//! - `crates/blit-daemon/src/service/push/data_plane.rs:347` —
//!   daemon-side gRPC push fallback receive (CLI → daemon direction).
//!   Out of h3c's CLI-receive scope; daemon-side stuck-peer is handled
//!   by the existing cancel-on-disconnect + active-job lifecycle.
//! - `crates/blit-daemon/src/service/pull_sync.rs:307, :341, :798, :966`
//!   and `crates/blit-daemon/src/service/push/control.rs:62` — all
//!   daemon-side receives of CLI-driven messages. Different concern,
//!   handled by HTTP/2 keepalive + cancel-on-disconnect; out of h3c.
//!
//! ## Why not a hardcoded timeout
//!
//! The principle (from the project's plan): Blit performs at maximum
//! per-hardware throughput under any conditions. Hardcoded wall-clock
//! constants violate that. The send-side cap is a structural shape —
//! frames are sized so progress is observable at floor throughput; the
//! actual stall policy in slice 2 will be progress-cadence-derived,
//! not a magic number.

use async_trait::async_trait;
use tonic::Streaming;

/// Maximum size of a single gRPC fallback data frame.
///
/// Sized so that even a deeply slow link (floor throughput around
/// 100 KB/s — mobile / satellite / heavily-throttled WAN) still produces
/// messages at observable cadence: 1 MiB / 100 KB/s ≈ 10 s per message,
/// which gives slice 2's watchdog something tangible to measure cadence
/// against.
///
/// The TCP data plane uses [`crate::remote::tuning`]-derived chunk sizes
/// (16-64 MiB for large transfers) because its byte-level progress
/// stream lets the read-side `StallGuard` observe every successful
/// `poll_read`. gRPC fallback can only observe whole decoded messages,
/// so it needs its own ceiling here regardless of TCP tuning.
///
/// Matches [`super::data_plane::CONTROL_PLANE_CHUNK_SIZE`] (1 MiB) by
/// design: both control-plane and gRPC-fallback data frames share the
/// same protobuf path and benefit from the same predictable frame size.
pub const GRPC_FALLBACK_CHUNK_BYTES: usize = 1024 * 1024;

/// Clamp a TCP-tuning-derived chunk size to the gRPC fallback ceiling.
///
/// Used by [`super::sink::GrpcFallbackSink`] and
/// [`super::sink::GrpcServerStreamingSink`] to ensure their emitted
/// `FileData` / `TarShardChunk` messages stay observable in size, even
/// when the TCP tuning would call for 16-64 MiB chunks.
pub fn clamp_fallback_chunk_size(tcp_tuned_chunk_bytes: usize) -> usize {
    tcp_tuned_chunk_bytes.min(GRPC_FALLBACK_CHUNK_BYTES)
}

/// Minimal abstraction over a gRPC stream-of-messages receive.
///
/// Implemented for `tonic::Streaming<T>` (production) and a test stub
/// inside `tests` below. Letting the helper take any `FallbackRecv`
/// instead of a concrete `Streaming<T>` is what makes
/// `recv_fallback_message` directly unit-testable without spinning up a
/// real gRPC server (slice 1 round 2: addresses the test-coverage
/// reviewer's pass-through concern).
#[async_trait]
pub(crate) trait FallbackRecv<T> {
    /// Return the next message, `None` on clean EOF, or a tonic Status
    /// on error. Same shape as `Streaming<T>::message`.
    async fn recv(&mut self) -> Result<Option<T>, tonic::Status>;
}

#[async_trait]
impl<T: Send + 'static> FallbackRecv<T> for Streaming<T> {
    async fn recv(&mut self) -> Result<Option<T>, tonic::Status> {
        self.message().await
    }
}

/// Receive the next message on a gRPC fallback stream.
///
/// Slice 1: a thin pass-through over [`FallbackRecv::recv`]. The
/// callers at `pull.rs:316`, `pull.rs:484`, and `pull.rs:752` (the three
/// CLI-side gRPC fallback receive loops named by the audit chain) all
/// route through this helper so slice 2's progress watchdog has a single
/// chokepoint to wrap.
///
/// The named helper is the auditable surface. The audit chain's
/// recurring lesson (memory `feedback-server-await-timeouts`) is that
/// every long-lived `.await` needs to be auditable; a named helper
/// makes the fallback-receive surface explicit instead of three
/// independent bare awaits — easy to grep, easy to refactor when slice
/// 2 lands.
pub(crate) async fn recv_fallback_message<T, S>(stream: &mut S) -> Result<Option<T>, tonic::Status>
where
    S: FallbackRecv<T> + ?Sized + Send,
    T: Send,
{
    stream.recv().await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fallback_chunk_size_caps_oversized_tcp_tuning() {
        // 64 MiB TCP tuning (the largest currently-emitted size) is
        // clamped down to the 1 MiB fallback ceiling.
        let big = 64 * 1024 * 1024;
        assert_eq!(clamp_fallback_chunk_size(big), GRPC_FALLBACK_CHUNK_BYTES);
    }

    #[test]
    fn fallback_chunk_size_preserves_already_small_tuning() {
        // A small TCP tuning value passes through unchanged — the clamp
        // is a ceiling, not a normalization.
        let small = 256 * 1024;
        assert_eq!(clamp_fallback_chunk_size(small), small);
    }

    #[test]
    fn fallback_chunk_size_handles_zero_and_one() {
        // Degenerate inputs don't panic and don't blow up to MAX.
        assert_eq!(clamp_fallback_chunk_size(0), 0);
        assert_eq!(clamp_fallback_chunk_size(1), 1);
    }

    #[test]
    fn fallback_chunk_size_equal_to_ceiling_is_unchanged() {
        assert_eq!(
            clamp_fallback_chunk_size(GRPC_FALLBACK_CHUNK_BYTES),
            GRPC_FALLBACK_CHUNK_BYTES
        );
    }

    // ─── recv_fallback_message smoke tests ──────────────────────────────
    //
    // Pins that slice 1's helper passes through the three result shapes
    // of FallbackRecv unchanged: Ok(Some(_)), Ok(None), Err(_). A future
    // refactor that adds a filter or swallows an error would fail these.

    /// Test stub: feeds a fixed sequence of `recv()` results.
    struct TestStream<T> {
        results: std::collections::VecDeque<Result<Option<T>, tonic::Status>>,
    }

    impl<T> TestStream<T> {
        fn new(results: Vec<Result<Option<T>, tonic::Status>>) -> Self {
            Self {
                results: results.into_iter().collect(),
            }
        }
    }

    #[async_trait]
    impl<T: Send + 'static> FallbackRecv<T> for TestStream<T> {
        async fn recv(&mut self) -> Result<Option<T>, tonic::Status> {
            self.results
                .pop_front()
                .unwrap_or_else(|| panic!("TestStream exhausted: caller polled too many times"))
        }
    }

    #[tokio::test]
    async fn recv_fallback_message_passes_through_ok_some() {
        let mut stream = TestStream::new(vec![Ok(Some(42u32))]);
        let result = recv_fallback_message(&mut stream).await;
        assert!(matches!(result, Ok(Some(42))));
    }

    #[tokio::test]
    async fn recv_fallback_message_passes_through_ok_none_clean_eof() {
        let mut stream = TestStream::<u32>::new(vec![Ok(None)]);
        let result = recv_fallback_message(&mut stream).await;
        // Critical: Ok(None) means clean stream end, NOT a stall. Slice
        // 1 must never map it to an error. Slice 2's watchdog has the
        // same invariant — TimedOut comes from cadence, not from EOF.
        assert!(matches!(result, Ok(None)));
    }

    #[tokio::test]
    async fn recv_fallback_message_propagates_tonic_status_err() {
        let mut stream = TestStream::<u32>::new(vec![Err(tonic::Status::unavailable("test"))]);
        let result = recv_fallback_message(&mut stream).await;
        match result {
            Err(status) => {
                assert_eq!(status.code(), tonic::Code::Unavailable);
                assert_eq!(status.message(), "test");
            }
            other => panic!("expected Err(Status), got {other:?}"),
        }
    }
}
