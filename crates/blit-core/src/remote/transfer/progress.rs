use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, AtomicU8, Ordering};
use std::sync::{Arc, Mutex};
use tokio::sync::mpsc::UnboundedSender;

/// One progress observation from a transfer producer.
///
/// **Contract (w6-1)** — every producer, every direction, both
/// transports:
///
/// - **Bytes ride `Payload` only.** Transferred bytes are reported
///   exclusively as `Payload { bytes, .. }` deltas (chunk- or
///   file-granular, producer's choice). `FileComplete` carries no byte
///   field at all, so no fold can double-count a file's bytes — the
///   class of bug filed as design-1.
/// - **Files are counted exactly once**, through exactly one of two
///   lanes per file:
///   - *per-file lane*: one `FileComplete { path }` per finished file,
///     with `Payload.files == 0` on that producer — used wherever the
///     producer sees individual files (receive pipelines, send
///     pipelines);
///   - *aggregate lane*: `Payload { files: delta, .. }` — used where
///     only counters are visible (the delegated `BytesProgress` bridge,
///     tar-shard batch appliers). Files counted this way get no
///     `FileComplete`.
/// - `FileComplete.path` is the source-relative wire path (POSIX
///   separators), never an absolute local path.
/// - `ManifestBatch { files }` is the enumeration denominator ("N of M
///   files"); it never adds to transferred totals. Its meaning is
///   direction-flavored (pull: full source manifest; push: need-list
///   batches; delegated: post-hoc summary) — consumers must treat it
///   as "expected files", nothing stronger.
///
/// [`ProgressTotals`] is the single shared fold for this contract;
/// consumers must not re-derive per-direction folding rules.
#[derive(Debug, Clone)]
pub enum ProgressEvent {
    /// Enumeration denominator: `files` more files are expected.
    ManifestBatch { files: usize },
    /// Transfer delta: `bytes` more bytes moved; `files` more files
    /// finished on the aggregate lane (0 on per-file-lane producers).
    Payload { files: usize, bytes: u64 },
    /// Per-file lane: the file at the source-relative wire `path`
    /// finished. Deliberately carries no byte count — bytes ride
    /// [`ProgressEvent::Payload`] only.
    FileComplete { path: String },
}

/// Running totals folded from a [`ProgressEvent`] stream under the
/// contract documented on the enum. This is the one shared
/// accumulator (w6-1) — the CLI progress monitor and all three TUI
/// transfer footers fold through it; per-direction folding rules no
/// longer exist.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct ProgressTotals {
    /// Files announced by enumeration (`ManifestBatch`) — the
    /// denominator, never part of transferred totals.
    pub manifest_files: u64,
    /// Files finished, counted once each via either lane.
    pub files: u64,
    /// Bytes transferred (`Payload` only).
    pub bytes: u64,
}

impl ProgressTotals {
    /// Fold one event into the running totals.
    pub fn apply(&mut self, event: &ProgressEvent) {
        match event {
            ProgressEvent::ManifestBatch { files } => {
                self.manifest_files = self.manifest_files.saturating_add(*files as u64);
            }
            ProgressEvent::Payload { files, bytes } => {
                self.files = self.files.saturating_add(*files as u64);
                self.bytes = self.bytes.saturating_add(*bytes);
            }
            ProgressEvent::FileComplete { .. } => {
                self.files = self.files.saturating_add(1);
            }
        }
    }

    /// True once any transfer work (not mere enumeration) has been
    /// observed — the "show live totals" gate consumers share.
    pub fn started(&self) -> bool {
        self.files > 0 || self.bytes > 0
    }
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
mod progress_totals_tests {
    use super::*;

    /// TCP data-plane pull shape after w6-1: the receive pipeline
    /// emits `Payload { 0, bytes_written }` then `FileComplete` per
    /// file — totals must count the bytes exactly once (design-1's
    /// class, now unrepresentable because FileComplete has no byte
    /// field).
    #[test]
    fn tcp_pull_pair_counts_bytes_once_and_one_file() {
        let mut totals = ProgressTotals::default();
        totals.apply(&ProgressEvent::Payload {
            files: 0,
            bytes: 1024,
        });
        totals.apply(&ProgressEvent::FileComplete {
            path: "f.txt".into(),
        });
        assert_eq!(totals.bytes, 1024);
        assert_eq!(totals.files, 1);
    }

    /// gRPC pull shape: chunk-granular `Payload`s then a
    /// `FileComplete` — chunks sum, the file counts once.
    #[test]
    fn grpc_pull_chunks_sum_then_file_counts_once() {
        let mut totals = ProgressTotals::default();
        for chunk in [4096u64, 4096, 2000] {
            totals.apply(&ProgressEvent::Payload {
                files: 0,
                bytes: chunk,
            });
        }
        totals.apply(&ProgressEvent::FileComplete {
            path: "big.bin".into(),
        });
        assert_eq!(totals.bytes, 10192);
        assert_eq!(totals.files, 1);
    }

    /// Push send shape after w6-1: `Payload { 0, size }` +
    /// `FileComplete` per file (bytes moved off FileComplete by the
    /// contract).
    #[test]
    fn push_send_pairs_accumulate_per_file() {
        let mut totals = ProgressTotals::default();
        for (i, size) in [100u64, 200, 300].iter().enumerate() {
            totals.apply(&ProgressEvent::Payload {
                files: 0,
                bytes: *size,
            });
            totals.apply(&ProgressEvent::FileComplete {
                path: format!("f{i}"),
            });
        }
        assert_eq!(totals.files, 3);
        assert_eq!(totals.bytes, 600);
    }

    /// Aggregate lane (delegated bridge, tar-shard appliers):
    /// `Payload` carries both deltas; no FileComplete arrives for
    /// those files.
    #[test]
    fn aggregate_lane_counts_files_and_bytes_from_payload() {
        let mut totals = ProgressTotals::default();
        totals.apply(&ProgressEvent::Payload {
            files: 2,
            bytes: 500,
        });
        totals.apply(&ProgressEvent::Payload {
            files: 1,
            bytes: 250,
        });
        assert_eq!(totals.files, 3);
        assert_eq!(totals.bytes, 750);
    }

    /// ManifestBatch is the denominator: it moves `manifest_files`
    /// only and never flips `started`.
    #[test]
    fn manifest_batch_is_denominator_only() {
        let mut totals = ProgressTotals::default();
        totals.apply(&ProgressEvent::ManifestBatch { files: 12 });
        assert_eq!(totals.manifest_files, 12);
        assert_eq!(totals.files, 0);
        assert_eq!(totals.bytes, 0);
        assert!(!totals.started());
        totals.apply(&ProgressEvent::Payload { files: 0, bytes: 1 });
        assert!(totals.started());
    }

    /// Totals saturate instead of wrapping on pathological inputs.
    #[test]
    fn totals_saturate_at_u64_max() {
        let mut totals = ProgressTotals::default();
        totals.apply(&ProgressEvent::Payload {
            files: 0,
            bytes: u64::MAX,
        });
        totals.apply(&ProgressEvent::Payload {
            files: 0,
            bytes: u64::MAX,
        });
        assert_eq!(totals.bytes, u64::MAX);
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

/// Exact live-membership registry for per-stream telemetry.
///
/// Stream identity, rather than insertion order, is the contract: an
/// elastic-pipeline retirement names the [`StreamId`] whose probe must leave
/// the tuner view. Duplicate registration is refused without replacing the
/// probe already bound to that member.
#[derive(Debug, Default)]
pub struct StreamProbeRegistry {
    probes: HashMap<StreamId, StreamProbe>,
}

impl StreamProbeRegistry {
    /// Build a registry from a fixed initial stream set.
    ///
    /// Initial member IDs are an internal invariant, so a duplicate is a
    /// construction bug and fails immediately instead of silently replacing
    /// the first stream's telemetry.
    pub fn from_probes(probes: Vec<StreamProbe>) -> Self {
        let mut registry = Self::default();
        for probe in probes {
            let id = probe.id();
            assert!(registry.register(probe), "duplicate stream probe id {id:?}");
        }
        registry
    }

    /// Register one live member. Returns `false` on a duplicate and leaves
    /// the existing probe untouched.
    pub fn register(&mut self, probe: StreamProbe) -> bool {
        let id = probe.id();
        match self.probes.entry(id) {
            std::collections::hash_map::Entry::Occupied(_) => false,
            std::collections::hash_map::Entry::Vacant(entry) => {
                entry.insert(probe);
                true
            }
        }
    }

    /// Remove exactly the named member's probe.
    pub fn unregister(&mut self, id: StreamId) -> Option<StreamProbe> {
        self.probes.remove(&id)
    }

    pub fn contains(&self, id: StreamId) -> bool {
        self.probes.contains_key(&id)
    }

    pub fn len(&self) -> usize {
        self.probes.len()
    }

    pub fn is_empty(&self) -> bool {
        self.probes.is_empty()
    }

    pub fn values(&self) -> impl Iterator<Item = &StreamProbe> {
        self.probes.values()
    }
}

/// Growable per-transfer probe registry shared by the sampler and elastic
/// membership controller. The mutex is held only for a snapshot fold or one
/// exact membership update.
pub type SharedStreamProbes = Arc<Mutex<StreamProbeRegistry>>;

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

    /// Announce `files` more expected files (the denominator). Never
    /// adds to transferred totals.
    pub fn report_manifest_batch(&self, files: usize) {
        let _ = self.sender.send(ProgressEvent::ManifestBatch { files });
    }

    /// Report a transfer delta. `bytes` is the only byte channel in
    /// the contract; `files` is nonzero only on the aggregate lane
    /// (producers with no per-file visibility — see the enum docs).
    pub fn report_payload(&self, files: usize, bytes: u64) {
        let _ = self.sender.send(ProgressEvent::Payload { files, bytes });
    }

    /// Report one finished file on the per-file lane. `path` is the
    /// source-relative wire path. Carries no bytes by construction —
    /// report those via [`report_payload`](Self::report_payload).
    pub fn report_file_complete(&self, path: String) {
        let _ = self.sender.send(ProgressEvent::FileComplete { path });
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

    #[test]
    fn registry_refuses_duplicate_without_replacing_existing_probe() {
        let existing = StreamProbe::new(StreamId(7));
        existing.record_bytes(11);
        let duplicate = StreamProbe::new(StreamId(7));
        duplicate.record_bytes(99);

        let mut registry = StreamProbeRegistry::default();
        assert!(registry.is_empty());
        assert!(registry.register(existing));
        assert!(!registry.register(duplicate));
        assert_eq!(registry.len(), 1);

        let retained = registry
            .unregister(StreamId(7))
            .expect("the first probe remains registered");
        assert_eq!(retained.snapshot().bytes_sent, 11);
        assert!(registry.is_empty());
    }

    #[test]
    fn registry_unregisters_exact_non_tail_member() {
        let mut registry = StreamProbeRegistry::from_probes(vec![
            StreamProbe::new(StreamId(10)),
            StreamProbe::new(StreamId(20)),
            StreamProbe::new(StreamId(30)),
        ]);

        let removed = registry
            .unregister(StreamId(20))
            .expect("middle member is present");
        assert_eq!(removed.id(), StreamId(20));
        assert!(registry.contains(StreamId(10)));
        assert!(!registry.contains(StreamId(20)));
        assert!(registry.contains(StreamId(30)));
        assert_eq!(registry.len(), 2);

        let remaining: std::collections::HashSet<_> =
            registry.values().map(StreamProbe::id).collect();
        assert_eq!(
            remaining,
            std::collections::HashSet::from([StreamId(10), StreamId(30)])
        );
    }
}
