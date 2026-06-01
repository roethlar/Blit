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
