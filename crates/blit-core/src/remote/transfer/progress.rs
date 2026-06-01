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
