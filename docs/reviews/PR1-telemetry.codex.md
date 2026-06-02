OpenAI Codex v0.136.0
--------
workdir: C:\Users\michael\Dev\blit_v2
model: gpt-5.5
provider: openai
approval: never
sandbox: read-only
reasoning effort: xhigh
reasoning summaries: none
session id: 019e8607-2237-7e60-be39-679181e4e32b
--------
user
You are reviewing ONE commit of a phased, pre-approved Rust refactor. EVERYTHING you need is in THIS message. Do NOT run shell commands, call tools, read files, or modify anything — read the diff + files below and write your review as text only.

PROJECT: blit, high-performance Rust file-transfer tool (CLI + daemon; gRPC control + TCP data plane; targets 10-25 Gbps).

SETTLED DECISIONS — do not relitigate or raise as findings:
- Slice PR1 of adaptive-stream-concurrency. This commit adds per-stream telemetry substrate ONLY: StreamId/StreamTelemetry (cache-padded lock-free counters), a cloneable StreamProbe, and a zero-cost Probe trait (NoProbe ACTIVE=false compiles to no-ops; LiveProbe). PR2 added the work-queue; PR3+ add resize + an AIMD controller that CONSUMES this telemetry. Do NOT suggest the controller/sampler/resize logic — later slices. The telemetry having no live reader yet is expected.
- HARD INVARIANT: zero added cost on the byte-copy hot path. The design gates all instrumentation (incl. Instant::now reads) on the compile-time const P::ACTIVE so DataPlaneSession<NoProbe> folds to today's code. Judge whether this actually holds; do NOT propose adding runtime branches to the per-chunk loop.
- flume/existing deps only; no new dep. Default type params (P = NoProbe) keep existing call sites unchanged — intentional.

JUDGE ONLY this commit's diff for REAL issues:
1. Correctness: atomic ordering (Relaxed is intended for counters), cache-padding soundness, the Probe gating truly eliding work for NoProbe, no panics, the TCP_INFO getsockopt unsafe block on Linux being sound and the non-Linux stub correct.
2. Does the zero-cost claim hold (NoProbe truly compiles out, no clock reads / atomic ops on the NoProbe hot path)?
3. Real bugs only — no scope creep, no speculative features, no style nits, nothing assigned to a later slice.

OUTPUT: one-line verdict (ship / fix-then-ship / block), then findings ranked by severity with file:line and a concrete fix. If correct and in-scope, say so plainly.

===== COMMIT DIFF: git show e6ef095 =====
commit e6ef095ccd8e1f654c0d9f8d02c7f9399a4dd4ed
Author: Claude <noreply@anthropic.com>
Date:   Mon Jun 1 04:14:26 2026 +0000

    feat(transfer): per-stream telemetry with zero-cost Probe (adaptive PR1)
    
    First phase of the adaptive-stream-concurrency work. Adds the per-stream
    telemetry substrate the future AIMD controller needs, with no behavior
    change and zero added cost on the byte-copy hot path.
    
    - progress.rs: StreamId / StreamTelemetry (cache-padded lock-free
      counters for bytes_sent + write_blocked_nanos, state, generation),
      a cloneable StreamProbe handle mirroring ByteProgressSink, and a Copy
      snapshot type for the sampler.
    - A zero-cost `Probe` trait with an associated `const ACTIVE: bool`:
      NoProbe (ACTIVE=false, #[inline(always)] no-ops) and LiveProbe. The
      send loop gates all instrumentation (including Instant::now reads) on
      `P::ACTIVE`, so the NoProbe monomorphization folds to today's code.
      Pinned by `const _: () = assert!(!NoProbe::ACTIVE)`.
    - data_plane.rs: DataPlaneSession<P: Probe = NoProbe>; from_stream/
      connect stay NoProbe wrappers over a generic from_stream_with_probe.
      The hot loop records bytes + write-block time through the probe.
    - sink.rs: DataPlaneSink<P: Probe = NoProbe>. Default type params keep
      every existing call site unchanged (inferred NoProbe).
    - tcp_info.rs: best-effort getsockopt(TCP_INFO) on Linux (retransmits,
      rtt); None stub elsewhere (controller falls back to throughput +
      write_blocked signals).
    
    Local 64-byte CachePadded wrapper avoids a new dependency.
    
    https://claude.ai/code/session_01RoMGEDwefAjC789v6NbLLN

diff --git a/crates/blit-core/src/remote/transfer/data_plane.rs b/crates/blit-core/src/remote/transfer/data_plane.rs
index 5b4fa43..c55d16e 100644
--- a/crates/blit-core/src/remote/transfer/data_plane.rs
+++ b/crates/blit-core/src/remote/transfer/data_plane.rs
@@ -8,6 +8,7 @@ use crate::buffer::BufferPool;
 use crate::generated::FileHeader;
 
 use super::payload::{prepared_payload_stream, PreparedPayload, TransferPayload};
+use super::progress::{NoProbe, Probe};
 use crate::remote::transfer::source::TransferSource;
 use std::sync::Arc;
 
@@ -18,13 +19,24 @@ pub const DATA_PLANE_RECORD_BLOCK: u8 = 2;
 pub const DATA_PLANE_RECORD_BLOCK_COMPLETE: u8 = 3;
 pub const DATA_PLANE_RECORD_END: u8 = 0xFF;
 
-pub struct DataPlaneSession {
+/// A single data-plane TCP stream and its send loop.
+///
+/// Generic over a [`Probe`] so the byte-copy hot path can carry
+/// per-stream telemetry under adaptive mode at **zero cost** when the
+/// probe is [`NoProbe`] (the default): the instrumented branches are
+/// gated on `P::ACTIVE`, a compile-time constant, so they fold away
+/// entirely for `DataPlaneSession<NoProbe>`. Existing callers name the
+/// bare type and get the `NoProbe` default; the adaptive controller
+/// constructs `DataPlaneSession<LiveProbe>` via
+/// [`from_stream_with_probe`](DataPlaneSession::from_stream_with_probe).
+pub struct DataPlaneSession<P: Probe = NoProbe> {
     stream: TcpStream,
     pool: Arc<BufferPool>,
     trace: bool,
     chunk_bytes: usize,
     payload_prefetch: usize,
     bytes_sent: u64,
+    probe: P,
 }
 
 macro_rules! trace_client {
@@ -35,8 +47,10 @@ macro_rules! trace_client {
     };
 }
 
-impl DataPlaneSession {
+impl DataPlaneSession<NoProbe> {
     /// Create a session from an existing stream with buffer pooling.
+    /// Produces the un-instrumented `NoProbe` variant — the default for
+    /// every non-adaptive caller.
     pub async fn from_stream(
         stream: TcpStream,
         trace: bool,
@@ -44,16 +58,8 @@ impl DataPlaneSession {
         payload_prefetch: usize,
         pool: Arc<BufferPool>,
     ) -> Self {
-        let payload_prefetch = payload_prefetch.max(1);
-        let chunk_bytes = chunk_bytes.max(64 * 1024);
-        Self {
-            stream,
-            pool,
-            trace,
-            chunk_bytes,
-            payload_prefetch,
-            bytes_sent: 0,
-        }
+        Self::from_stream_with_probe(stream, trace, chunk_bytes, payload_prefetch, pool, NoProbe)
+            .await
     }
 
     /// Connect to a data plane endpoint with buffer pooling.
@@ -113,6 +119,33 @@ impl DataPlaneSession {
 
         Ok(Self::from_stream(stream, trace, chunk_bytes, payload_prefetch, pool).await)
     }
+}
+
+impl<P: Probe> DataPlaneSession<P> {
+    /// Create a session carrying an arbitrary [`Probe`]. The generic
+    /// primitive behind [`from_stream`](DataPlaneSession::from_stream);
+    /// the adaptive controller calls this with a `LiveProbe` to enable
+    /// per-stream telemetry.
+    pub async fn from_stream_with_probe(
+        stream: TcpStream,
+        trace: bool,
+        chunk_bytes: usize,
+        payload_prefetch: usize,
+        pool: Arc<BufferPool>,
+        probe: P,
+    ) -> Self {
+        let payload_prefetch = payload_prefetch.max(1);
+        let chunk_bytes = chunk_bytes.max(64 * 1024);
+        Self {
+            stream,
+            pool,
+            trace,
+            chunk_bytes,
+            payload_prefetch,
+            bytes_sent: 0,
+            probe,
+        }
+    }
 
     pub async fn send_payloads(
         &mut self,
@@ -292,6 +325,15 @@ impl DataPlaneSession {
 
         // Main loop: write buf_a while reading into buf_b
         while remaining > 0 {
+            // Per-stream telemetry: time the overlapped write+read step
+            // as a backpressure proxy. Gated on the compile-time
+            // `P::ACTIVE` constant so `DataPlaneSession<NoProbe>` reads
+            // no clock and folds this to nothing.
+            let step_start = if P::ACTIVE {
+                Some(std::time::Instant::now())
+            } else {
+                None
+            };
             // Overlap: write from buf_a, read into buf_b concurrently
             let (write_result, read_result) = tokio::join!(
                 self.stream.write_all(&buf_a.as_slice()[..bytes_a]),
@@ -299,6 +341,12 @@ impl DataPlaneSession {
             );
 
             write_result.with_context(|| format!("sending {}", rel))?;
+            if P::ACTIVE {
+                if let Some(t) = step_start {
+                    self.probe.note_write_blocked(t.elapsed().as_nanos() as u64);
+                }
+            }
+            self.probe.record_bytes(bytes_a as u64);
             crate::remote::instrumentation::record_cli_data_plane_outbound_bytes(bytes_a as u64);
 
             let bytes_b = read_result.with_context(|| format!("reading {}", rel))?;
@@ -321,12 +369,25 @@ impl DataPlaneSession {
             bytes_a = bytes_b;
         }
 
-        // Final write: send the last chunk in buf_a
+        // Final write: send the last chunk in buf_a. This is a pure
+        // write (no overlapped read), so the timing is cleanly
+        // attributable to socket-write backpressure.
         if bytes_a > 0 {
+            let tail_start = if P::ACTIVE {
+                Some(std::time::Instant::now())
+            } else {
+                None
+            };
             self.stream
                 .write_all(&buf_a.as_slice()[..bytes_a])
                 .await
                 .with_context(|| format!("sending {}", rel))?;
+            if P::ACTIVE {
+                if let Some(t) = tail_start {
+                    self.probe.note_write_blocked(t.elapsed().as_nanos() as u64);
+                }
+            }
+            self.probe.record_bytes(bytes_a as u64);
             crate::remote::instrumentation::record_cli_data_plane_outbound_bytes(bytes_a as u64);
         }
 
@@ -399,6 +460,7 @@ impl DataPlaneSession {
                 .write_all(chunk)
                 .await
                 .context("writing tar shard payload")?;
+            self.probe.record_bytes(chunk.len() as u64);
             crate::remote::instrumentation::record_cli_data_plane_outbound_bytes(chunk.len() as u64);
         }
         trace_client!(
diff --git a/crates/blit-core/src/remote/transfer/mod.rs b/crates/blit-core/src/remote/transfer/mod.rs
index d77f4ad..9deb905 100644
--- a/crates/blit-core/src/remote/transfer/mod.rs
+++ b/crates/blit-core/src/remote/transfer/mod.rs
@@ -8,6 +8,7 @@ pub mod sink;
 pub mod source;
 pub mod stall_guard;
 pub mod tar_safety;
+pub mod tcp_info;
 
 pub use data_plane::{
     receive_stream_double_buffered, DataPlaneSession, CONTROL_PLANE_CHUNK_SIZE,
@@ -21,9 +22,11 @@ pub use payload::{
 };
 pub use pipeline::{execute_sink_pipeline, execute_sink_pipeline_streaming};
 pub use progress::{
-    ByteProgressSink, Phase, ProgressEvent, RemoteTransferProgress, TransferProgress,
-    TransferProgressSnapshot,
+    ByteProgressSink, LiveProbe, NoProbe, Phase, Probe, ProgressEvent, RemoteTransferProgress,
+    StreamId, StreamProbe, StreamState, StreamTelemetry, StreamTelemetrySnapshot,
+    TransferProgress, TransferProgressSnapshot,
 };
+pub use tcp_info::{sample_stream as sample_tcp_info, TcpInfoSample};
 pub use sink::{
     DataPlaneSink, FsSinkConfig, FsTransferSink, GrpcFallbackSink, GrpcServerStreamingSink,
     NullSink, SinkOutcome, TransferSink,
diff --git a/crates/blit-core/src/remote/transfer/progress.rs b/crates/blit-core/src/remote/transfer/progress.rs
index 19909ed..d1717bf 100644
--- a/crates/blit-core/src/remote/transfer/progress.rs
+++ b/crates/blit-core/src/remote/transfer/progress.rs
@@ -298,6 +298,234 @@ mod tests {
     }
 }
 
+// =====================================================================
+// Per-stream telemetry (PR1 of the adaptive-streams work).
+//
+// The adaptive stream controller (added in a later PR) needs a live,
+// per-stream view of throughput and write-backpressure to steer
+// AIMD decisions. This module provides the lock-free counters plus a
+// zero-cost `Probe` abstraction so the byte-copy hot path pays nothing
+// when telemetry is off.
+//
+// Hot-path discipline mirrors `ByteProgressSink`: writers only do
+// `Relaxed` atomic adds; a sampler task reads `snapshot()` on a timer.
+// =====================================================================
+
+/// Cache-line-aligned wrapper so independent per-stream counters never
+/// share a cache line (false sharing would tax the hot path under high
+/// stream counts). A local 8-line equivalent of
+/// `crossbeam_utils::CachePadded`, kept here to avoid adding a
+/// dependency for one type. 64 bytes covers x86-64 / aarch64 lines.
+#[repr(align(64))]
+#[derive(Debug, Default)]
+struct CachePadded<T>(T);
+
+impl<T> CachePadded<T> {
+    fn new(value: T) -> Self {
+        Self(value)
+    }
+}
+
+impl<T> std::ops::Deref for CachePadded<T> {
+    type Target = T;
+    #[inline]
+    fn deref(&self) -> &T {
+        &self.0
+    }
+}
+
+/// Identifies one data-plane stream within a transfer. Stable for the
+/// life of the stream; an `ADD`'d stream gets a fresh id.
+#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
+pub struct StreamId(pub u32);
+
+/// Coarse lifecycle state of a single stream, stored as a `u8` atomic so
+/// the sampler can read it lock-free. The controller uses it to exclude
+/// draining/closed streams from marginal-gain math.
+#[derive(Clone, Copy, Debug, PartialEq, Eq)]
+#[repr(u8)]
+pub enum StreamState {
+    /// Connected, handshake done, no payload written yet.
+    Starting = 0,
+    /// Actively transferring payloads.
+    Active = 1,
+    /// `stop` requested; finishing in-flight payload then closing.
+    Draining = 2,
+    /// `RECORD_END` emitted, socket closed.
+    Closed = 3,
+}
+
+impl StreamState {
+    fn from_u8(v: u8) -> Self {
+        match v {
+            1 => StreamState::Active,
+            2 => StreamState::Draining,
+            3 => StreamState::Closed,
+            _ => StreamState::Starting,
+        }
+    }
+}
+
+/// Lock-free per-stream counters. Cache-padded so independent streams
+/// never share a cache line (false sharing would otherwise tax the hot
+/// path under high stream counts). Only the owning send loop writes
+/// `bytes_sent` / `write_blocked_nanos`; the sampler reads a snapshot.
+#[derive(Debug)]
+pub struct StreamTelemetry {
+    bytes_sent: CachePadded<AtomicU64>,
+    write_blocked_nanos: CachePadded<AtomicU64>,
+    state: AtomicU8,
+    /// Bumped each time the controller resizes; lets a stale snapshot be
+    /// discarded across a resize boundary.
+    generation: AtomicU64,
+}
+
+impl StreamTelemetry {
+    pub fn new() -> Self {
+        Self {
+            bytes_sent: CachePadded::new(AtomicU64::new(0)),
+            write_blocked_nanos: CachePadded::new(AtomicU64::new(0)),
+            state: AtomicU8::new(StreamState::Starting as u8),
+            generation: AtomicU64::new(0),
+        }
+    }
+}
+
+impl Default for StreamTelemetry {
+    fn default() -> Self {
+        Self::new()
+    }
+}
+
+/// Plain `Copy` view of a [`StreamTelemetry`], taken by the sampler each
+/// tick. Decoupled from the atomics so the sampler never holds a
+/// reference into the shared handle.
+#[derive(Clone, Copy, Debug)]
+pub struct StreamTelemetrySnapshot {
+    pub id: StreamId,
+    pub bytes_sent: u64,
+    pub write_blocked_nanos: u64,
+    pub state: StreamState,
+    pub generation: u64,
+}
+
+/// Cloneable handle to one stream's telemetry, held by the send loop.
+/// Clone is a cheap `Arc` bump; the increments are `Relaxed`. Mirrors
+/// the `ByteProgressSink` pattern so the data plane can carry it the
+/// same way it carries byte progress.
+#[derive(Clone, Debug)]
+pub struct StreamProbe {
+    id: StreamId,
+    inner: Arc<StreamTelemetry>,
+}
+
+impl StreamProbe {
+    pub fn new(id: StreamId) -> Self {
+        Self {
+            id,
+            inner: Arc::new(StreamTelemetry::new()),
+        }
+    }
+
+    pub fn from_telemetry(id: StreamId, inner: Arc<StreamTelemetry>) -> Self {
+        Self { id, inner }
+    }
+
+    pub fn id(&self) -> StreamId {
+        self.id
+    }
+
+    /// Shared `Arc` so a sampler can hold the telemetry independently of
+    /// the send loop's probe.
+    pub fn telemetry(&self) -> Arc<StreamTelemetry> {
+        Arc::clone(&self.inner)
+    }
+
+    /// Add `delta` bytes that just landed on the wire. `Relaxed` is
+    /// sufficient: the sampler only needs eventual visibility.
+    #[inline]
+    pub fn record_bytes(&self, delta: u64) {
+        self.inner.bytes_sent.fetch_add(delta, Ordering::Relaxed);
+    }
+
+    /// Add nanoseconds spent blocked on a socket write — the signal the
+    /// controller uses to tell "link-bound" from "source-bound".
+    #[inline]
+    pub fn add_write_blocked(&self, nanos: u64) {
+        self.inner
+            .write_blocked_nanos
+            .fetch_add(nanos, Ordering::Relaxed);
+    }
+
+    pub fn set_state(&self, state: StreamState) {
+        self.inner.state.store(state as u8, Ordering::Relaxed);
+    }
+
+    pub fn set_generation(&self, generation: u64) {
+        self.inner.generation.store(generation, Ordering::Relaxed);
+    }
+
+    pub fn snapshot(&self) -> StreamTelemetrySnapshot {
+        StreamTelemetrySnapshot {
+            id: self.id,
+            bytes_sent: self.inner.bytes_sent.load(Ordering::Relaxed),
+            write_blocked_nanos: self.inner.write_blocked_nanos.load(Ordering::Relaxed),
+            state: StreamState::from_u8(self.inner.state.load(Ordering::Relaxed)),
+            generation: self.inner.generation.load(Ordering::Relaxed),
+        }
+    }
+}
+
+/// Zero-cost telemetry abstraction for the byte-copy hot path.
+///
+/// The send loop is generic over `P: Probe`. The associated
+/// `const ACTIVE` lets the timing instrumentation (`Instant::now()`)
+/// be compile-time elided for the [`NoProbe`] monomorphization: an
+/// `if P::ACTIVE { … }` guarding the clock reads folds to nothing when
+/// `ACTIVE == false`, and the empty `#[inline(always)]` methods emit no
+/// code. The result is byte-identical codegen to the pre-telemetry hot
+/// loop — the hard "zero added cost on the byte-copy hot path"
+/// constraint, satisfied at compile time rather than via a runtime
+/// branch.
+pub trait Probe: Send + Sync + 'static {
+    /// When `false`, callers must skip all instrumentation work
+    /// (including clock reads) so the optimizer drops it entirely.
+    const ACTIVE: bool;
+    fn record_bytes(&self, delta: u64);
+    fn note_write_blocked(&self, nanos: u64);
+}
+
+/// The default probe: every method is an inlined no-op and `ACTIVE`
+/// is `false`, so a `DataPlaneSession<NoProbe>` send loop compiles to
+/// exactly today's code.
+#[derive(Clone, Copy, Debug, Default)]
+pub struct NoProbe;
+
+impl Probe for NoProbe {
+    const ACTIVE: bool = false;
+    #[inline(always)]
+    fn record_bytes(&self, _delta: u64) {}
+    #[inline(always)]
+    fn note_write_blocked(&self, _nanos: u64) {}
+}
+
+/// The instrumented probe, constructed only under adaptive mode. Wraps
+/// a [`StreamProbe`] and forwards into its lock-free counters.
+#[derive(Clone, Debug)]
+pub struct LiveProbe(pub StreamProbe);
+
+impl Probe for LiveProbe {
+    const ACTIVE: bool = true;
+    #[inline(always)]
+    fn record_bytes(&self, delta: u64) {
+        self.0.record_bytes(delta);
+    }
+    #[inline(always)]
+    fn note_write_blocked(&self, nanos: u64) {
+        self.0.add_write_blocked(nanos);
+    }
+}
+
 #[derive(Clone)]
 pub struct RemoteTransferProgress {
     sender: UnboundedSender<ProgressEvent>,
@@ -322,3 +550,61 @@ impl RemoteTransferProgress {
             .send(ProgressEvent::FileComplete { path, bytes });
     }
 }
+
+#[cfg(test)]
+mod stream_telemetry_tests {
+    use super::*;
+
+    #[test]
+    fn live_probe_accumulates_bytes_and_block_time() {
+        let probe = StreamProbe::new(StreamId(7));
+        let live = LiveProbe(probe.clone());
+        // Drive through the Probe trait, exactly as the hot loop does.
+        Probe::record_bytes(&live, 1000);
+        Probe::record_bytes(&live, 500);
+        Probe::note_write_blocked(&live, 250_000);
+        let snap = probe.snapshot();
+        assert_eq!(snap.id, StreamId(7));
+        assert_eq!(snap.bytes_sent, 1500);
+        assert_eq!(snap.write_blocked_nanos, 250_000);
+        assert_eq!(snap.state, StreamState::Starting);
+    }
+
+    // The optimizer relies on these constants to elide the
+    // instrumentation branches; pin them at compile time (a runtime
+    // `assert!` on a const is a clippy `assertions_on_constants` lint).
+    const _: () = assert!(!<NoProbe as Probe>::ACTIVE);
+    const _: () = assert!(<LiveProbe as Probe>::ACTIVE);
+
+    #[test]
+    fn no_probe_is_inert() {
+        // NoProbe must compile to a no-op; there is no observable state,
+        // so the contract is simply that the trait calls type-check and
+        // run without effect.
+        let n = NoProbe;
+        Probe::record_bytes(&n, 123);
+        Probe::note_write_blocked(&n, 456);
+    }
+
+    #[test]
+    fn state_and_generation_round_trip() {
+        let probe = StreamProbe::new(StreamId(0));
+        probe.set_state(StreamState::Draining);
+        probe.set_generation(42);
+        let snap = probe.snapshot();
+        assert_eq!(snap.state, StreamState::Draining);
+        assert_eq!(snap.generation, 42);
+    }
+
+    #[test]
+    fn clones_share_counters() {
+        let probe = StreamProbe::new(StreamId(1));
+        let clone = probe.clone();
+        probe.record_bytes(10);
+        clone.record_bytes(20);
+        assert_eq!(probe.snapshot().bytes_sent, 30);
+        // The telemetry Arc is shared.
+        let tel = probe.telemetry();
+        assert!(Arc::strong_count(&tel) >= 2);
+    }
+}
diff --git a/crates/blit-core/src/remote/transfer/sink.rs b/crates/blit-core/src/remote/transfer/sink.rs
index 875cf3c..18e87c7 100644
--- a/crates/blit-core/src/remote/transfer/sink.rs
+++ b/crates/blit-core/src/remote/transfer/sink.rs
@@ -17,7 +17,7 @@ use crate::copy::{copy_file, resume_copy_file};
 use crate::generated::{ComparisonMode, FileHeader};
 use crate::logger::NoopLogger;
 use crate::remote::transfer::payload::PreparedPayload;
-use crate::remote::transfer::progress::ByteProgressSink;
+use crate::remote::transfer::progress::{ByteProgressSink, NoProbe, Probe};
 use crate::remote::transfer::source::TransferSource;
 
 // Re-export for consumers.
@@ -771,15 +771,15 @@ async fn write_file_block_complete(
 ///
 /// Each instance wraps a single TCP stream (DataPlaneSession). For multi-stream
 /// transfers, the pipeline executor creates multiple DataPlaneSink instances.
-pub struct DataPlaneSink {
-    session: tokio::sync::Mutex<DataPlaneSession>,
+pub struct DataPlaneSink<P: Probe = NoProbe> {
+    session: tokio::sync::Mutex<DataPlaneSession<P>>,
     source: Arc<dyn TransferSource>,
     dst_root: PathBuf,
 }
 
-impl DataPlaneSink {
+impl<P: Probe> DataPlaneSink<P> {
     pub fn new(
-        session: DataPlaneSession,
+        session: DataPlaneSession<P>,
         source: Arc<dyn TransferSource>,
         dst_root: PathBuf,
     ) -> Self {
@@ -792,7 +792,7 @@ impl DataPlaneSink {
 }
 
 #[async_trait]
-impl TransferSink for DataPlaneSink {
+impl<P: Probe> TransferSink for DataPlaneSink<P> {
     async fn write_payload(&self, payload: PreparedPayload) -> Result<SinkOutcome> {
         let mut session = self.session.lock().await;
         match payload {
diff --git a/crates/blit-core/src/remote/transfer/tcp_info.rs b/crates/blit-core/src/remote/transfer/tcp_info.rs
new file mode 100644
index 0000000..6fc805a
--- /dev/null
+++ b/crates/blit-core/src/remote/transfer/tcp_info.rs
@@ -0,0 +1,88 @@
+//! Best-effort per-socket TCP statistics for the adaptive stream
+//! controller.
+//!
+//! On Linux the controller reads `TCP_INFO` via `getsockopt(2)` to see
+//! retransmits and smoothed RTT — the cleanest "the link is congesting"
+//! signal available without a userspace congestion model. Everywhere
+//! else the syscall has no portable equivalent, so [`sample_stream`]
+//! returns `None` and the controller falls back to its
+//! throughput-slope + `write_blocked_nanos` signals (which are
+//! cross-platform). Keeping the platform split behind one function lets
+//! the controller stay platform-agnostic.
+
+/// A point-in-time read of kernel TCP state for one stream. Fields are
+/// cumulative counters / current estimates; the controller diffs
+/// successive samples to derive a per-interval retransmit rate.
+#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
+pub struct TcpInfoSample {
+    /// Total retransmitted segments over the life of the connection
+    /// (`tcpi_total_retrans`). Monotonic; diff across samples.
+    pub total_retransmits: u64,
+    /// Smoothed round-trip time in microseconds (`tcpi_rtt`).
+    pub rtt_micros: u64,
+}
+
+/// Read `TCP_INFO` for `stream`. Returns `None` when the platform has no
+/// equivalent or the syscall fails (the controller then leans on its
+/// portable signals). Never panics.
+#[cfg(target_os = "linux")]
+pub fn sample_stream(stream: &tokio::net::TcpStream) -> Option<TcpInfoSample> {
+    use std::os::fd::AsRawFd;
+    let fd = stream.as_raw_fd();
+    // SAFETY: `getsockopt` writes at most `len` bytes into `info`, which
+    // is a fully-owned zeroed `tcp_info`; `len` is initialised to its
+    // size and updated by the kernel. We read only after a success
+    // return. `fd` is borrowed from a live `TcpStream` for the duration
+    // of the call.
+    let mut info: libc::tcp_info = unsafe { std::mem::zeroed() };
+    let mut len = std::mem::size_of::<libc::tcp_info>() as libc::socklen_t;
+    let ret = unsafe {
+        libc::getsockopt(
+            fd,
+            libc::IPPROTO_TCP,
+            libc::TCP_INFO,
+            &mut info as *mut libc::tcp_info as *mut libc::c_void,
+            &mut len,
+        )
+    };
+    if ret != 0 {
+        return None;
+    }
+    Some(TcpInfoSample {
+        total_retransmits: info.tcpi_total_retrans as u64,
+        rtt_micros: info.tcpi_rtt as u64,
+    })
+}
+
+/// Non-Linux stub: no portable `TCP_INFO`, so the controller uses
+/// throughput + `write_blocked_nanos` instead.
+#[cfg(not(target_os = "linux"))]
+pub fn sample_stream(_stream: &tokio::net::TcpStream) -> Option<TcpInfoSample> {
+    None
+}
+
+#[cfg(all(test, target_os = "linux"))]
+mod tests {
+    use super::*;
+
+    /// A live loopback connection should yield a `TCP_INFO` read with a
+    /// plausible (non-huge) RTT and zero-ish retransmits. This proves
+    /// the `getsockopt` wiring works end-to-end on Linux.
+    #[tokio::test]
+    async fn samples_live_loopback_socket() {
+        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
+        let addr = listener.local_addr().unwrap();
+        let (client, _server) = tokio::join!(
+            async { tokio::net::TcpStream::connect(addr).await.unwrap() },
+            async { listener.accept().await.unwrap() },
+        );
+        let sample = sample_stream(&client).expect("TCP_INFO available on loopback");
+        // Loopback RTT is microseconds-to-low-milliseconds; assert it's
+        // not absurd rather than pinning a value.
+        assert!(
+            sample.rtt_micros < 5_000_000,
+            "loopback rtt should be well under 5s, got {} us",
+            sample.rtt_micros
+        );
+    }
+}


===== FULL FILE: crates/blit-core/src/remote/transfer/progress.rs =====
use std::sync::atomic::{AtomicU64, AtomicU8, Ordering};
use std::sync::Arc;
use tokio::sync::mpsc::UnboundedSender;

#[derive(Debug, Clone)]
pub enum ProgressEvent {
    ManifestBatch { files: usize },
    Payload { files: usize, bytes: u64 },
    FileComplete { path: String, bytes: u64 },
}

/// Cumulative byte-progress reporter for data-plane write loops.
///
/// Lives here in `blit-core` so the data-plane functions
/// (`receive_stream_double_buffered`, &c.) can take an
/// `Option<&ByteProgressSink>` parameter without depending on
/// `blit-daemon`. The daemon constructs one per transfer
/// (cloned from the per-row `Arc<AtomicU64>` it stores in its
/// `ActiveJobs` registry) and threads it through the handler;
/// callers that don't need byte-level instrumentation pass
/// `None`.
///
/// Clone is cheap (`Arc` bump); `report` is cheap
/// (`AtomicU64::fetch_add` with `Relaxed` ordering). Outliving
/// the producer/consumer pair is benign — a stray report after
/// the daemon's row drains just bumps an orphaned atomic, no
/// row reappears.
#[derive(Clone)]
pub struct ByteProgressSink {
    counter: Arc<AtomicU64>,
}

impl ByteProgressSink {
    /// Construct a fresh sink backed by a new atomic. Daemon
    /// callers can ignore this and instead clone-from the
    /// counter their row already owns via `ByteProgressSink::from_counter`.
    #[allow(dead_code)]
    pub fn new() -> Self {
        Self {
            counter: Arc::new(AtomicU64::new(0)),
        }
    }

    /// Wrap an existing counter `Arc`. Daemon uses this so the
    /// sink it hands the data plane shares the atomic stored
    /// on the `ActiveJobs` row — `snapshot()` and Drop can read
    /// the same value the data plane is incrementing.
    pub fn from_counter(counter: Arc<AtomicU64>) -> Self {
        Self { counter }
    }

    /// Add `delta` bytes to the cumulative counter. Called by
    /// the data plane after each chunk write. `Relaxed`
    /// ordering is sufficient: readers only need eventual
    /// visibility, not synchronization with other memory
    /// operations.
    pub fn report(&self, delta: u64) {
        self.counter.fetch_add(delta, Ordering::Relaxed);
    }
}

impl Default for ByteProgressSink {
    fn default() -> Self {
        Self::new()
    }
}

/// Coarse lifecycle phase of a local transfer, used to switch the
/// CLI's progress display between a scan spinner and a byte bar.
/// Stored as a `u8` atomic on [`TransferProgress`] so the render
/// thread can read it lock-free.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u8)]
pub enum Phase {
    /// Walking the source tree, counting entries; total unknown.
    Enumerating = 0,
    /// Diffing + planning; total being computed.
    Planning = 1,
    /// Copying bytes to the destination; total known.
    Transferring = 2,
    /// Mirror cleanup (deleting extraneous destination entries).
    Deleting = 3,
    /// All work finished; render thread should clear and exit.
    Done = 4,
}

impl Phase {
    fn from_u8(v: u8) -> Self {
        match v {
            1 => Phase::Planning,
            2 => Phase::Transferring,
            3 => Phase::Deleting,
            4 => Phase::Done,
            _ => Phase::Enumerating,
        }
    }
}

/// Lock-free, shareable progress handle for a single local transfer.
///
/// The orchestrator and its copy workers only ever *write* to this
/// via `Relaxed` atomic adds/stores — there is no lock, no syscall,
/// and no allocation on the hot path. A single CLI-owned render
/// thread *reads* it via [`snapshot`](Self::snapshot) on a timer and
/// is the sole writer to the terminal. This split is what keeps the
/// progress display free of any measurable transfer-throughput cost
/// while also fixing the scrolling spinner (one terminal writer).
///
/// Cloning is a cheap `Arc` bump; every clone observes the same
/// counters. The byte-done counter is exposed to the existing
/// [`FsTransferSink`](crate::remote::transfer::FsTransferSink)
/// byte-reporting path via [`byte_sink`](Self::byte_sink), so no
/// changes to the sink write loops are required.
#[derive(Clone, Debug)]
pub struct TransferProgress {
    phase: Arc<AtomicU8>,
    scanned_files: Arc<AtomicU64>,
    scanned_bytes: Arc<AtomicU64>,
    total_files: Arc<AtomicU64>,
    total_bytes: Arc<AtomicU64>,
    done_files: Arc<AtomicU64>,
    done_bytes: Arc<AtomicU64>,
}

/// A plain `Copy` point-in-time view of [`TransferProgress`], taken
/// by the render thread each tick. Decoupled from the atomics so the
/// render code never holds a reference into the shared handle.
#[derive(Clone, Copy, Debug)]
pub struct TransferProgressSnapshot {
    pub phase: Phase,
    pub scanned_files: u64,
    pub scanned_bytes: u64,
    pub total_files: u64,
    pub total_bytes: u64,
    pub done_files: u64,
    pub done_bytes: u64,
}

impl TransferProgress {
    pub fn new() -> Self {
        Self {
            phase: Arc::new(AtomicU8::new(Phase::Enumerating as u8)),
            scanned_files: Arc::new(AtomicU64::new(0)),
            scanned_bytes: Arc::new(AtomicU64::new(0)),
            total_files: Arc::new(AtomicU64::new(0)),
            total_bytes: Arc::new(AtomicU64::new(0)),
            done_files: Arc::new(AtomicU64::new(0)),
            done_bytes: Arc::new(AtomicU64::new(0)),
        }
    }

    /// Set the lifecycle phase. Called by the orchestrator at phase
    /// boundaries (cheap, infrequent).
    pub fn set_phase(&self, phase: Phase) {
        self.phase.store(phase as u8, Ordering::Relaxed);
    }

    /// Add to the live scanned counters. Called once per discovered
    /// header in the orchestrator's collect loop — not in the
    /// filesystem walker, so the walk itself pays nothing.
    pub fn add_scanned(&self, files: u64, bytes: u64) {
        self.scanned_files.fetch_add(files, Ordering::Relaxed);
        self.scanned_bytes.fetch_add(bytes, Ordering::Relaxed);
    }

    /// Record the known denominators once planning has determined
    /// how many files / bytes the transfer will actually move.
    pub fn set_totals(&self, files: u64, bytes: u64) {
        self.total_files.store(files, Ordering::Relaxed);
        self.total_bytes.store(bytes, Ordering::Relaxed);
    }

    /// Add to the live completed counters. `done_bytes` is normally
    /// driven through [`byte_sink`](Self::byte_sink) by the sink's
    /// existing per-payload report; `done_files` is bumped alongside.
    pub fn add_done(&self, files: u64, bytes: u64) {
        self.done_files.fetch_add(files, Ordering::Relaxed);
        self.done_bytes.fetch_add(bytes, Ordering::Relaxed);
    }

    /// A [`ByteProgressSink`] that reports into this handle's
    /// `done_bytes` counter. Lets the orchestrator wire live byte
    /// progress through `FsTransferSink::with_byte_progress` without
    /// any new code in the sink write path — the sink's existing
    /// `bp.report(bytes_written)` call lands on `done_bytes`.
    pub fn byte_sink(&self) -> ByteProgressSink {
        ByteProgressSink::from_counter(self.done_bytes.clone())
    }

    /// The raw `done_files` counter, for the sink to `fetch_add` into
    /// per payload alongside its existing byte report. Mirrors the
    /// `byte_sink` bridge so live file progress needs no bespoke type
    /// in the sink. Non-CLI callers never wire this, so the sink's
    /// counter stays `None` and the add is skipped.
    pub fn done_files_counter(&self) -> Arc<AtomicU64> {
        self.done_files.clone()
    }

    /// Take a consistent-enough snapshot for rendering. The loads are
    /// `Relaxed` and independent, so fields may be momentarily skewed
    /// across a tick; that's acceptable for a progress display and
    /// avoids any synchronization cost on the writers.
    pub fn snapshot(&self) -> TransferProgressSnapshot {
        TransferProgressSnapshot {
            phase: Phase::from_u8(self.phase.load(Ordering::Relaxed)),
            scanned_files: self.scanned_files.load(Ordering::Relaxed),
            scanned_bytes: self.scanned_bytes.load(Ordering::Relaxed),
            total_files: self.total_files.load(Ordering::Relaxed),
            total_bytes: self.total_bytes.load(Ordering::Relaxed),
            done_files: self.done_files.load(Ordering::Relaxed),
            done_bytes: self.done_bytes.load(Ordering::Relaxed),
        }
    }
}

impl Default for TransferProgress {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn report_accumulates_on_single_sink() {
        let sink = ByteProgressSink::new();
        sink.report(100);
        sink.report(50);
        assert_eq!(sink.counter.load(Ordering::Relaxed), 150);
    }

    #[test]
    fn clones_share_underlying_counter() {
        let sink_a = ByteProgressSink::new();
        let sink_b = sink_a.clone();
        sink_a.report(10);
        sink_b.report(20);
        assert_eq!(sink_a.counter.load(Ordering::Relaxed), 30);
        assert_eq!(sink_b.counter.load(Ordering::Relaxed), 30);
    }

    #[test]
    fn from_counter_wraps_external_arc() {
        // Verifies the daemon-side construction path: an Arc
        // owned by the table row can be wrapped without copying
        // the atomic, and reports through the sink show up on
        // the original Arc.
        let counter = Arc::new(AtomicU64::new(0));
        let sink = ByteProgressSink::from_counter(Arc::clone(&counter));
        sink.report(4096);
        assert_eq!(counter.load(Ordering::Relaxed), 4096);
    }

    #[test]
    fn transfer_progress_tracks_phase_and_counters() {
        let prog = TransferProgress::new();
        assert_eq!(prog.snapshot().phase, Phase::Enumerating);

        prog.add_scanned(3, 300);
        prog.add_scanned(2, 200);
        prog.set_phase(Phase::Transferring);
        prog.set_totals(5, 500);
        prog.add_done(1, 100);

        let snap = prog.snapshot();
        assert_eq!(snap.phase, Phase::Transferring);
        assert_eq!(snap.scanned_files, 5);
        assert_eq!(snap.scanned_bytes, 500);
        assert_eq!(snap.total_files, 5);
        assert_eq!(snap.total_bytes, 500);
        assert_eq!(snap.done_files, 1);
        assert_eq!(snap.done_bytes, 100);
    }

    #[test]
    fn byte_sink_feeds_done_bytes() {
        // The bridge the orchestrator uses: the sink's existing
        // `report` lands on the handle's done_bytes counter, so
        // wiring it needs no change to the sink write loop.
        let prog = TransferProgress::new();
        let sink = prog.byte_sink();
        sink.report(2048);
        sink.report(1024);
        assert_eq!(prog.snapshot().done_bytes, 3072);
    }

    #[test]
    fn clones_share_counters() {
        let prog = TransferProgress::new();
        let clone = prog.clone();
        prog.add_done(1, 10);
        clone.add_done(2, 20);
        let snap = prog.snapshot();
        assert_eq!(snap.done_files, 3);
        assert_eq!(snap.done_bytes, 30);
    }
}

// =====================================================================
// Per-stream telemetry (PR1 of the adaptive-streams work).
//
// The adaptive stream controller (added in a later PR) needs a live,
// per-stream view of throughput and write-backpressure to steer
// AIMD decisions. This module provides the lock-free counters plus a
// zero-cost `Probe` abstraction so the byte-copy hot path pays nothing
// when telemetry is off.
//
// Hot-path discipline mirrors `ByteProgressSink`: writers only do
// `Relaxed` atomic adds; a sampler task reads `snapshot()` on a timer.
// =====================================================================

/// Cache-line-aligned wrapper so independent per-stream counters never
/// share a cache line (false sharing would tax the hot path under high
/// stream counts). A local 8-line equivalent of
/// `crossbeam_utils::CachePadded`, kept here to avoid adding a
/// dependency for one type. 64 bytes covers x86-64 / aarch64 lines.
#[repr(align(64))]
#[derive(Debug, Default)]
struct CachePadded<T>(T);

impl<T> CachePadded<T> {
    fn new(value: T) -> Self {
        Self(value)
    }
}

impl<T> std::ops::Deref for CachePadded<T> {
    type Target = T;
    #[inline]
    fn deref(&self) -> &T {
        &self.0
    }
}

/// Identifies one data-plane stream within a transfer. Stable for the
/// life of the stream; an `ADD`'d stream gets a fresh id.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct StreamId(pub u32);

/// Coarse lifecycle state of a single stream, stored as a `u8` atomic so
/// the sampler can read it lock-free. The controller uses it to exclude
/// draining/closed streams from marginal-gain math.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u8)]
pub enum StreamState {
    /// Connected, handshake done, no payload written yet.
    Starting = 0,
    /// Actively transferring payloads.
    Active = 1,
    /// `stop` requested; finishing in-flight payload then closing.
    Draining = 2,
    /// `RECORD_END` emitted, socket closed.
    Closed = 3,
}

impl StreamState {
    fn from_u8(v: u8) -> Self {
        match v {
            1 => StreamState::Active,
            2 => StreamState::Draining,
            3 => StreamState::Closed,
            _ => StreamState::Starting,
        }
    }
}

/// Lock-free per-stream counters. Cache-padded so independent streams
/// never share a cache line (false sharing would otherwise tax the hot
/// path under high stream counts). Only the owning send loop writes
/// `bytes_sent` / `write_blocked_nanos`; the sampler reads a snapshot.
#[derive(Debug)]
pub struct StreamTelemetry {
    bytes_sent: CachePadded<AtomicU64>,
    write_blocked_nanos: CachePadded<AtomicU64>,
    state: AtomicU8,
    /// Bumped each time the controller resizes; lets a stale snapshot be
    /// discarded across a resize boundary.
    generation: AtomicU64,
}

impl StreamTelemetry {
    pub fn new() -> Self {
        Self {
            bytes_sent: CachePadded::new(AtomicU64::new(0)),
            write_blocked_nanos: CachePadded::new(AtomicU64::new(0)),
            state: AtomicU8::new(StreamState::Starting as u8),
            generation: AtomicU64::new(0),
        }
    }
}

impl Default for StreamTelemetry {
    fn default() -> Self {
        Self::new()
    }
}

/// Plain `Copy` view of a [`StreamTelemetry`], taken by the sampler each
/// tick. Decoupled from the atomics so the sampler never holds a
/// reference into the shared handle.
#[derive(Clone, Copy, Debug)]
pub struct StreamTelemetrySnapshot {
    pub id: StreamId,
    pub bytes_sent: u64,
    pub write_blocked_nanos: u64,
    pub state: StreamState,
    pub generation: u64,
}

/// Cloneable handle to one stream's telemetry, held by the send loop.
/// Clone is a cheap `Arc` bump; the increments are `Relaxed`. Mirrors
/// the `ByteProgressSink` pattern so the data plane can carry it the
/// same way it carries byte progress.
#[derive(Clone, Debug)]
pub struct StreamProbe {
    id: StreamId,
    inner: Arc<StreamTelemetry>,
}

impl StreamProbe {
    pub fn new(id: StreamId) -> Self {
        Self {
            id,
            inner: Arc::new(StreamTelemetry::new()),
        }
    }

    pub fn from_telemetry(id: StreamId, inner: Arc<StreamTelemetry>) -> Self {
        Self { id, inner }
    }

    pub fn id(&self) -> StreamId {
        self.id
    }

    /// Shared `Arc` so a sampler can hold the telemetry independently of
    /// the send loop's probe.
    pub fn telemetry(&self) -> Arc<StreamTelemetry> {
        Arc::clone(&self.inner)
    }

    /// Add `delta` bytes that just landed on the wire. `Relaxed` is
    /// sufficient: the sampler only needs eventual visibility.
    #[inline]
    pub fn record_bytes(&self, delta: u64) {
        self.inner.bytes_sent.fetch_add(delta, Ordering::Relaxed);
    }

    /// Add nanoseconds spent blocked on a socket write — the signal the
    /// controller uses to tell "link-bound" from "source-bound".
    #[inline]
    pub fn add_write_blocked(&self, nanos: u64) {
        self.inner
            .write_blocked_nanos
            .fetch_add(nanos, Ordering::Relaxed);
    }

    pub fn set_state(&self, state: StreamState) {
        self.inner.state.store(state as u8, Ordering::Relaxed);
    }

    pub fn set_generation(&self, generation: u64) {
        self.inner.generation.store(generation, Ordering::Relaxed);
    }

    pub fn snapshot(&self) -> StreamTelemetrySnapshot {
        StreamTelemetrySnapshot {
            id: self.id,
            bytes_sent: self.inner.bytes_sent.load(Ordering::Relaxed),
            write_blocked_nanos: self.inner.write_blocked_nanos.load(Ordering::Relaxed),
            state: StreamState::from_u8(self.inner.state.load(Ordering::Relaxed)),
            generation: self.inner.generation.load(Ordering::Relaxed),
        }
    }
}

/// Zero-cost telemetry abstraction for the byte-copy hot path.
///
/// The send loop is generic over `P: Probe`. The associated
/// `const ACTIVE` lets the timing instrumentation (`Instant::now()`)
/// be compile-time elided for the [`NoProbe`] monomorphization: an
/// `if P::ACTIVE { … }` guarding the clock reads folds to nothing when
/// `ACTIVE == false`, and the empty `#[inline(always)]` methods emit no
/// code. The result is byte-identical codegen to the pre-telemetry hot
/// loop — the hard "zero added cost on the byte-copy hot path"
/// constraint, satisfied at compile time rather than via a runtime
/// branch.
pub trait Probe: Send + Sync + 'static {
    /// When `false`, callers must skip all instrumentation work
    /// (including clock reads) so the optimizer drops it entirely.
    const ACTIVE: bool;
    fn record_bytes(&self, delta: u64);
    fn note_write_blocked(&self, nanos: u64);
}

/// The default probe: every method is an inlined no-op and `ACTIVE`
/// is `false`, so a `DataPlaneSession<NoProbe>` send loop compiles to
/// exactly today's code.
#[derive(Clone, Copy, Debug, Default)]
pub struct NoProbe;

impl Probe for NoProbe {
    const ACTIVE: bool = false;
    #[inline(always)]
    fn record_bytes(&self, _delta: u64) {}
    #[inline(always)]
    fn note_write_blocked(&self, _nanos: u64) {}
}

/// The instrumented probe, constructed only under adaptive mode. Wraps
/// a [`StreamProbe`] and forwards into its lock-free counters.
#[derive(Clone, Debug)]
pub struct LiveProbe(pub StreamProbe);

impl Probe for LiveProbe {
    const ACTIVE: bool = true;
    #[inline(always)]
    fn record_bytes(&self, delta: u64) {
        self.0.record_bytes(delta);
    }
    #[inline(always)]
    fn note_write_blocked(&self, nanos: u64) {
        self.0.add_write_blocked(nanos);
    }
}

#[derive(Clone)]
pub struct RemoteTransferProgress {
    sender: UnboundedSender<ProgressEvent>,
}

impl RemoteTransferProgress {
    pub fn new(sender: UnboundedSender<ProgressEvent>) -> Self {
        Self { sender }
    }

    pub fn report_manifest_batch(&self, files: usize) {
        let _ = self.sender.send(ProgressEvent::ManifestBatch { files });
    }

    pub fn report_payload(&self, files: usize, bytes: u64) {
        let _ = self.sender.send(ProgressEvent::Payload { files, bytes });
    }

    pub fn report_file_complete(&self, path: String, bytes: u64) {
        let _ = self
            .sender
            .send(ProgressEvent::FileComplete { path, bytes });
    }
}

#[cfg(test)]
mod stream_telemetry_tests {
    use super::*;

    #[test]
    fn live_probe_accumulates_bytes_and_block_time() {
        let probe = StreamProbe::new(StreamId(7));
        let live = LiveProbe(probe.clone());
        // Drive through the Probe trait, exactly as the hot loop does.
        Probe::record_bytes(&live, 1000);
        Probe::record_bytes(&live, 500);
        Probe::note_write_blocked(&live, 250_000);
        let snap = probe.snapshot();
        assert_eq!(snap.id, StreamId(7));
        assert_eq!(snap.bytes_sent, 1500);
        assert_eq!(snap.write_blocked_nanos, 250_000);
        assert_eq!(snap.state, StreamState::Starting);
    }

    // The optimizer relies on these constants to elide the
    // instrumentation branches; pin them at compile time (a runtime
    // `assert!` on a const is a clippy `assertions_on_constants` lint).
    const _: () = assert!(!<NoProbe as Probe>::ACTIVE);
    const _: () = assert!(<LiveProbe as Probe>::ACTIVE);

    #[test]
    fn no_probe_is_inert() {
        // NoProbe must compile to a no-op; there is no observable state,
        // so the contract is simply that the trait calls type-check and
        // run without effect.
        let n = NoProbe;
        Probe::record_bytes(&n, 123);
        Probe::note_write_blocked(&n, 456);
    }

    #[test]
    fn state_and_generation_round_trip() {
        let probe = StreamProbe::new(StreamId(0));
        probe.set_state(StreamState::Draining);
        probe.set_generation(42);
        let snap = probe.snapshot();
        assert_eq!(snap.state, StreamState::Draining);
        assert_eq!(snap.generation, 42);
    }

    #[test]
    fn clones_share_counters() {
        let probe = StreamProbe::new(StreamId(1));
        let clone = probe.clone();
        probe.record_bytes(10);
        clone.record_bytes(20);
        assert_eq!(probe.snapshot().bytes_sent, 30);
        // The telemetry Arc is shared.
        let tel = probe.telemetry();
        assert!(Arc::strong_count(&tel) >= 2);
    }
}


===== FULL FILE: crates/blit-core/src/remote/transfer/tcp_info.rs =====
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


codex
fix-then-ship

**Findings**

Medium - [data_plane.rs](C:/Users/michael/Dev/blit_v2/crates/blit-core/src/remote/transfer/data_plane.rs:325): `write_blocked_nanos` measures the whole `tokio::join!(write_all, read_chunk_into)` step, so slow source reads inflate socket-write backpressure even when the write finished immediately. Fix by timing inside the write future itself, then joining that timed write future with the read future. Keep the `NoProbe` path on the existing direct join or behind `if P::ACTIVE` so the clock read still const-folds away.

Low - [data_plane.rs](C:/Users/michael/Dev/blit_v2/crates/blit-core/src/remote/transfer/data_plane.rs:460): tar shard chunk writes record bytes but never record write-block time. If `DataPlaneSession<LiveProbe>` sends this payload variant, backpressure is underreported for that stream. Fix by applying the same `P::ACTIVE` write-only timing around each `write_all(chunk)`.

Everything else reviewed looks in scope: `Relaxed` is appropriate for these counters, the cache padding is sound, the `NoProbe` clock gating/no-op probe calls should compile out on the default hot path, and the Linux `TCP_INFO` unsafe block plus non-Linux stub look correct.
fix-then-ship

**Findings**

Medium - [data_plane.rs](C:/Users/michael/Dev/blit_v2/crates/blit-core/src/remote/transfer/data_plane.rs:325): `write_blocked_nanos` measures the whole `tokio::join!(write_all, read_chunk_into)` step, so slow source reads inflate socket-write backpressure even when the write finished immediately. Fix by timing inside the write future itself, then joining that timed write future with the read future. Keep the `NoProbe` path on the existing direct join or behind `if P::ACTIVE` so the clock read still const-folds away.

Low - [data_plane.rs](C:/Users/michael/Dev/blit_v2/crates/blit-core/src/remote/transfer/data_plane.rs:460): tar shard chunk writes record bytes but never record write-block time. If `DataPlaneSession<LiveProbe>` sends this payload variant, backpressure is underreported for that stream. Fix by applying the same `P::ACTIVE` write-only timing around each `write_all(chunk)`.

Everything else reviewed looks in scope: `Relaxed` is appropriate for these counters, the cache padding is sound, the `NoProbe` clock gating/no-op probe calls should compile out on the default hot path, and the Linux `TCP_INFO` unsafe block plus non-Linux stub look correct.
tokens used
23,043
