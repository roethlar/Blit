//! High-volume, aggregate timing probe for the otp-12 small-file finding.
//!
//! This probe is deliberately separate from `session_phase`: the phase trace
//! stays low-frequency, while this one counts every small-file bookkeeping and
//! sink operation but emits only one bounded summary per endpoint. Production
//! activation requires `BLIT_TRACE_SMALL_FILE_PROBE=1` and a non-empty
//! `BLIT_TRACE_RUN_ID`. Tests can inject an in-memory emitter.

use serde::Serialize;
use std::collections::HashMap;
use std::io::Write;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex, OnceLock};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use crate::generated::FileHeader;

use super::session_phase::SessionPhaseRole;

const TRACE_ENV: &str = "BLIT_TRACE_SMALL_FILE_PROBE";
const RUN_ID_ENV: &str = "BLIT_TRACE_RUN_ID";
const MAX_SHARD_RECORDS: usize = 16_384;

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum SmallFileCarrier {
    Tcp,
    InStream,
}

#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize)]
pub struct TimingAggregate {
    pub samples: u64,
    pub total_ns: u64,
    pub max_ns: u64,
}

impl TimingAggregate {
    pub(crate) fn record(&mut self, duration: Duration) {
        let ns = duration_ns(duration);
        self.samples = self.samples.saturating_add(1);
        self.total_ns = self.total_ns.saturating_add(ns);
        self.max_ns = self.max_ns.max(ns);
    }
}

#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize)]
pub struct SourceBookkeepingReport {
    pub manifest_entries_inserted: u64,
    /// Synchronization wait before the manifest map operation. Zero on an
    /// unsynchronized/task-local historical implementation.
    pub manifest_insert_sync_wait: TimingAggregate,
    /// The map insertion itself, excluding synchronization wait.
    pub manifest_insert_map_op: TimingAggregate,
    pub need_entries_resolved: u64,
    /// Synchronization wait before resolving a need in the manifest map.
    pub need_resolve_sync_wait: TimingAggregate,
    /// Map lookup/remove/clone work that resolves a need, excluding wait.
    pub need_resolve_map_op: TimingAggregate,
    pub need_event_send: TimingAggregate,
    /// Enqueue-attempt to handler-start latency. This includes the
    /// (separately measured) enqueue call itself.
    pub need_event_hop: TimingAggregate,
    /// Common per-need handler work after map resolution. On the current
    /// path this is the event handler; an inline historical path records the
    /// equivalent inline work here.
    pub need_handler_work: TimingAggregate,
    pub planner: TimingAggregate,
    pub planner_input_entries: u64,
    pub planned_payloads: u64,
    pub planned_tar_shards: u64,
    pub planned_tar_members: u64,
    pub tar_queue: TimingAggregate,
    pub tar_shards_queued: u64,
    pub tar_members_queued: u64,
}

#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize)]
pub struct ClaimReport {
    pub members: u64,
    pub lock_acquisitions: u64,
    pub successful_removes: u64,
    pub lock_wait: TimingAggregate,
    pub lock_hold: TimingAggregate,
}

#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize)]
pub struct MemberTimingReport {
    pub mkdir: TimingAggregate,
    pub open: TimingAggregate,
    pub write: TimingAggregate,
    /// File-descriptor drop time. This is not an fsync/durability claim.
    pub close: TimingAggregate,
    pub metadata: TimingAggregate,
    pub total: TimingAggregate,
}

impl MemberTimingReport {
    pub(crate) fn record(
        &mut self,
        mkdir: Duration,
        open: Duration,
        write: Duration,
        close: Duration,
        metadata: Duration,
        total: Duration,
    ) {
        self.mkdir.record(mkdir);
        self.open.record(open);
        self.write.record(write);
        self.close.record(close);
        self.metadata.record(metadata);
        self.total.record(total);
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct ShardReceiveReport {
    pub seq: u64,
    /// Opaque keyed digest used only to join this report's receive and sink
    /// records; it is not a member path.
    pub shard_id: String,
    pub carrier: SmallFileCarrier,
    pub members: u64,
    pub archive_bytes: u64,
    pub start_elapsed_ns: u64,
    /// Carrier record-body receive plus decode after its discriminator.
    /// This includes transport wait/allocation and is not a CPU-only span.
    /// TCP includes its tar header; in-stream starts after TarShardHeader
    /// validation/claim and covers the following chunk frames.
    pub record_receive_ns: u64,
    /// Observer-only keyed shard-correlation work, excluded from `total_ns`.
    pub correlation_ns: u64,
    pub sink_ns: u64,
    pub total_ns: u64,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct ShardSinkReport {
    pub seq: u64,
    /// Opaque keyed digest used only to join this report's receive and sink
    /// records; it is not a member path.
    pub shard_id: String,
    pub carrier: SmallFileCarrier,
    pub members: u64,
    pub archive_bytes: u64,
    pub start_elapsed_ns: u64,
    pub blocking_pool_wait_ns: u64,
    pub parse_validate_ns: u64,
    pub member_parallel_wall_ns: u64,
    pub total_ns: u64,
    pub member: MemberTimingReport,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct SmallFileProbeReport {
    pub schema: u8,
    pub run_id: String,
    /// TCP: session-token-derived. In-stream: run-ID-derived, requiring a
    /// unique run ID for each observed session.
    pub correlation_id: String,
    pub endpoint_role: SessionPhaseRole,
    pub initiator_role: SessionPhaseRole,
    pub carrier: SmallFileCarrier,
    pub event: &'static str,
    pub success: bool,
    pub unix_ns: u128,
    pub elapsed_ns: u64,
    pub source_bookkeeping: SourceBookkeepingReport,
    pub tcp_claims: ClaimReport,
    pub in_stream_claims: ClaimReport,
    pub shard_receive: Vec<ShardReceiveReport>,
    pub shard_receive_dropped: u64,
    pub shard_sink: Vec<ShardSinkReport>,
    pub shard_sink_dropped: u64,
}

type ReportEmitter = dyn Fn(SmallFileProbeReport) + Send + Sync + 'static;

#[derive(Clone)]
struct ProbeEmitter {
    run_id: Arc<str>,
    emit: Arc<ReportEmitter>,
}

/// Unbound small-file probe carried by SOURCE/DESTINATION instruments.
#[derive(Clone)]
pub struct SmallFileProbe {
    emitter: Option<ProbeEmitter>,
    allow_env: bool,
}

impl Default for SmallFileProbe {
    fn default() -> Self {
        Self {
            emitter: None,
            allow_env: true,
        }
    }
}

impl SmallFileProbe {
    pub fn capture(
        run_id: impl Into<String>,
        emit: impl Fn(SmallFileProbeReport) + Send + Sync + 'static,
    ) -> Self {
        Self {
            emitter: Some(ProbeEmitter {
                run_id: Arc::from(run_id.into()),
                emit: Arc::new(emit),
            }),
            allow_env: false,
        }
    }

    pub fn disabled() -> Self {
        Self {
            emitter: None,
            allow_env: false,
        }
    }

    pub(crate) fn or_from_env(self) -> Self {
        self.or_from_env_with(|name| std::env::var(name).ok(), Self::stderr_writer)
    }

    fn or_from_env_with(
        self,
        mut read: impl FnMut(&str) -> Option<String>,
        writer: impl FnOnce(String) -> Self,
    ) -> Self {
        if self.emitter.is_some() || !self.allow_env {
            return self;
        }
        let enabled = read(TRACE_ENV).is_some_and(|value| {
            matches!(
                value.trim().to_ascii_lowercase().as_str(),
                "1" | "true" | "yes" | "on"
            )
        });
        if !enabled {
            return self;
        }
        let Some(run_id) = read(RUN_ID_ENV).filter(|value| !value.trim().is_empty()) else {
            eprintln!("[small-file-probe] configuration_error={RUN_ID_ENV} must be non-empty");
            return Self::disabled();
        };
        writer(run_id)
    }

    fn stderr_writer(run_id: String) -> Self {
        Self::capture(run_id, |report| {
            let line = match serde_json::to_string(&report) {
                Ok(json) => format!("[small-file-probe] {json}"),
                Err(err) => format!("[small-file-probe] serialization_error={err}"),
            };
            let stderr = std::io::stderr();
            let mut stderr = stderr.lock();
            let _ = writeln!(stderr, "{line}");
            let _ = stderr.flush();
        })
    }

    pub(crate) fn bind(
        &self,
        session_token: Option<&[u8]>,
        endpoint_role: SessionPhaseRole,
        initiator_role: SessionPhaseRole,
        carrier: SmallFileCarrier,
    ) -> Option<BoundSmallFileProbe> {
        let emitter = self.emitter.clone()?;
        let digest = match session_token {
            Some(token) => blake3::hash(token),
            None => blake3::hash(emitter.run_id.as_bytes()),
        };
        let digest_hex = digest.to_hex();
        Some(BoundSmallFileProbe {
            inner: Arc::new(BoundSmallFileProbeInner {
                emitter,
                correlation_id: Arc::from(&digest_hex.as_str()[..16]),
                shard_key: *digest.as_bytes(),
                endpoint_role,
                initiator_role,
                carrier,
                origin: Instant::now(),
                stats: ProbeStats::default(),
                finished: AtomicBool::new(false),
            }),
        })
    }
}

#[derive(Default)]
struct AtomicTiming {
    samples: AtomicU64,
    total_ns: AtomicU64,
    max_ns: AtomicU64,
}

impl AtomicTiming {
    fn record(&self, duration: Duration) {
        let ns = duration_ns(duration);
        self.samples.fetch_add(1, Ordering::Relaxed);
        self.total_ns.fetch_add(ns, Ordering::Relaxed);
        self.max_ns.fetch_max(ns, Ordering::Relaxed);
    }

    fn report(&self) -> TimingAggregate {
        TimingAggregate {
            samples: self.samples.load(Ordering::Relaxed),
            total_ns: self.total_ns.load(Ordering::Relaxed),
            max_ns: self.max_ns.load(Ordering::Relaxed),
        }
    }
}

#[derive(Default)]
struct SourceStats {
    manifest_entries_inserted: AtomicU64,
    manifest_insert_sync_wait: AtomicTiming,
    manifest_insert_map_op: AtomicTiming,
    need_entries_resolved: AtomicU64,
    need_resolve_sync_wait: AtomicTiming,
    need_resolve_map_op: AtomicTiming,
    need_event_send: AtomicTiming,
    need_event_hop: AtomicTiming,
    need_handler_work: AtomicTiming,
    need_event_stamps: Mutex<HashMap<String, Arc<OnceLock<Instant>>>>,
    planner: AtomicTiming,
    planner_input_entries: AtomicU64,
    planned_payloads: AtomicU64,
    planned_tar_shards: AtomicU64,
    planned_tar_members: AtomicU64,
    tar_queue: AtomicTiming,
    tar_shards_queued: AtomicU64,
    tar_members_queued: AtomicU64,
}

#[derive(Default)]
struct ClaimStats {
    members: AtomicU64,
    lock_acquisitions: AtomicU64,
    successful_removes: AtomicU64,
    lock_wait: AtomicTiming,
    lock_hold: AtomicTiming,
}

impl ClaimStats {
    fn report(&self) -> ClaimReport {
        ClaimReport {
            members: self.members.load(Ordering::Relaxed),
            lock_acquisitions: self.lock_acquisitions.load(Ordering::Relaxed),
            successful_removes: self.successful_removes.load(Ordering::Relaxed),
            lock_wait: self.lock_wait.report(),
            lock_hold: self.lock_hold.report(),
        }
    }
}

#[derive(Default)]
struct ProbeStats {
    source: SourceStats,
    tcp_claims: ClaimStats,
    in_stream_claims: ClaimStats,
    receive_seq: AtomicU64,
    shard_receive: Mutex<Vec<ShardReceiveReport>>,
    shard_receive_dropped: AtomicU64,
    sink_seq: AtomicU64,
    shard_sink: Mutex<Vec<ShardSinkReport>>,
    shard_sink_dropped: AtomicU64,
}

#[derive(Clone)]
pub(crate) struct BoundSmallFileProbe {
    inner: Arc<BoundSmallFileProbeInner>,
}

pub(crate) struct BoundSmallFileProbeInner {
    emitter: ProbeEmitter,
    correlation_id: Arc<str>,
    shard_key: [u8; 32],
    endpoint_role: SessionPhaseRole,
    initiator_role: SessionPhaseRole,
    carrier: SmallFileCarrier,
    origin: Instant,
    stats: ProbeStats,
    finished: AtomicBool,
}

impl std::ops::Deref for BoundSmallFileProbe {
    type Target = BoundSmallFileProbeInner;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl BoundSmallFileProbe {
    pub(crate) fn start(&self) -> Instant {
        Instant::now()
    }

    pub(crate) fn note_manifest_insert(&self, wait: Duration, map_op: Duration) {
        let source = &self.stats.source;
        source
            .manifest_entries_inserted
            .fetch_add(1, Ordering::Relaxed);
        source.manifest_insert_sync_wait.record(wait);
        source.manifest_insert_map_op.record(map_op);
    }

    pub(crate) fn note_need_resolve(&self, wait: Duration, map_op: Duration, resolved: bool) {
        let source = &self.stats.source;
        if resolved {
            source.need_entries_resolved.fetch_add(1, Ordering::Relaxed);
        }
        source.need_resolve_sync_wait.record(wait);
        source.need_resolve_map_op.record(map_op);
    }

    pub(crate) fn note_need_event_send(&self, duration: Duration) {
        self.stats.source.need_event_send.record(duration);
    }

    /// Install the enqueue timestamp in probe-owned state. This table does
    /// not alter `SourceEvent`, so the disabled event layout stays unchanged.
    pub(crate) fn note_need_event_enqueue(&self, path: &str) {
        let stamp = Arc::new(OnceLock::new());
        {
            self.stats
                .source
                .need_event_stamps
                .lock()
                .expect("small-file event-stamp lock poisoned")
                .insert(path.to_owned(), Arc::clone(&stamp));
        }
        // Table allocation/hash/insert/unlock are observer setup, not channel
        // hop latency. The event cannot be sent until this has been set.
        let _ = stamp.set(Instant::now());
    }

    /// Record enqueue-to-handler-start without charging the timestamp-table
    /// lookup to the handler-work span.
    pub(crate) fn note_need_event_hop(&self, path: &str, handler_started: Instant) {
        let enqueued_at = self
            .stats
            .source
            .need_event_stamps
            .lock()
            .expect("small-file event-stamp lock poisoned")
            .remove(path);
        if let Some(enqueued_at) = enqueued_at.and_then(|stamp| stamp.get().copied()) {
            self.stats
                .source
                .need_event_hop
                .record(handler_started.saturating_duration_since(enqueued_at));
        }
    }

    pub(crate) fn note_need_handler_work(&self, duration: Duration) {
        self.stats.source.need_handler_work.record(duration);
    }

    pub(crate) fn note_planner(
        &self,
        duration: Duration,
        input_entries: usize,
        payloads: usize,
        tar_shards: usize,
        tar_members: usize,
    ) {
        let source = &self.stats.source;
        source.planner.record(duration);
        source
            .planner_input_entries
            .fetch_add(input_entries as u64, Ordering::Relaxed);
        source
            .planned_payloads
            .fetch_add(payloads as u64, Ordering::Relaxed);
        source
            .planned_tar_shards
            .fetch_add(tar_shards as u64, Ordering::Relaxed);
        source
            .planned_tar_members
            .fetch_add(tar_members as u64, Ordering::Relaxed);
    }

    pub(crate) fn note_tar_queue(&self, duration: Duration, members: usize) {
        let source = &self.stats.source;
        source.tar_queue.record(duration);
        source.tar_shards_queued.fetch_add(1, Ordering::Relaxed);
        source
            .tar_members_queued
            .fetch_add(members as u64, Ordering::Relaxed);
    }

    pub(crate) fn note_claim(
        &self,
        carrier: SmallFileCarrier,
        members: usize,
        lock_acquisitions: usize,
        successful_removes: usize,
        wait: Duration,
        hold: Duration,
    ) {
        let claims = match carrier {
            SmallFileCarrier::Tcp => &self.stats.tcp_claims,
            SmallFileCarrier::InStream => &self.stats.in_stream_claims,
        };
        claims.members.fetch_add(members as u64, Ordering::Relaxed);
        claims
            .lock_acquisitions
            .fetch_add(lock_acquisitions as u64, Ordering::Relaxed);
        claims
            .successful_removes
            .fetch_add(successful_removes as u64, Ordering::Relaxed);
        claims.lock_wait.record(wait);
        claims.lock_hold.record(hold);
    }

    pub(crate) fn note_shard_receive(
        &self,
        shard_id: String,
        carrier: SmallFileCarrier,
        members: usize,
        archive_bytes: u64,
        started: Instant,
        decoded: Instant,
        correlated: Instant,
        sink_started: Instant,
        finished: Instant,
    ) {
        let record_receive = decoded.saturating_duration_since(started);
        let correlation = correlated.saturating_duration_since(decoded);
        let downstream = finished.saturating_duration_since(sink_started);
        let record = ShardReceiveReport {
            seq: self.stats.receive_seq.fetch_add(1, Ordering::Relaxed),
            shard_id,
            carrier,
            members: members as u64,
            archive_bytes,
            start_elapsed_ns: instant_offset_ns(self.origin, started),
            record_receive_ns: duration_ns(record_receive),
            correlation_ns: duration_ns(correlation),
            sink_ns: duration_ns(downstream),
            total_ns: duration_ns(record_receive.saturating_add(downstream)),
        };
        push_bounded(
            &self.stats.shard_receive,
            &self.stats.shard_receive_dropped,
            record,
        );
    }

    #[allow(clippy::too_many_arguments)]
    pub(crate) fn note_shard_sink(
        &self,
        shard_id: String,
        carrier: SmallFileCarrier,
        members: usize,
        archive_bytes: u64,
        started: Instant,
        blocking_pool_wait: Duration,
        parse_validate: Duration,
        member_parallel_wall: Duration,
        total: Duration,
        member: MemberTimingReport,
    ) {
        let record = ShardSinkReport {
            seq: self.stats.sink_seq.fetch_add(1, Ordering::Relaxed),
            shard_id,
            carrier,
            members: members as u64,
            archive_bytes,
            start_elapsed_ns: instant_offset_ns(self.origin, started),
            blocking_pool_wait_ns: duration_ns(blocking_pool_wait),
            parse_validate_ns: duration_ns(parse_validate),
            member_parallel_wall_ns: duration_ns(member_parallel_wall),
            total_ns: duration_ns(total),
            member,
        };
        push_bounded(
            &self.stats.shard_sink,
            &self.stats.shard_sink_dropped,
            record,
        );
    }

    pub(crate) fn carrier(&self) -> SmallFileCarrier {
        self.carrier
    }

    /// The ordered, length-delimited paths make concurrent receive and sink
    /// records independently joinable without emitting those paths. TCP uses
    /// the session-token digest as its key; in-stream uses the required run-ID
    /// digest, so that run ID must be unique per observed in-stream session.
    pub(crate) fn shard_id(&self, headers: &[FileHeader]) -> String {
        let mut hasher = blake3::Hasher::new_keyed(&self.shard_key);
        for header in headers {
            let path = header.relative_path.as_bytes();
            hasher.update(&(path.len() as u64).to_le_bytes());
            hasher.update(path);
        }
        hasher.finalize().to_hex().to_string()
    }

    pub(crate) fn finish(&self) {
        if self.finished.swap(true, Ordering::AcqRel) {
            return;
        }
        let source = &self.stats.source;
        let mut shard_receive = self
            .stats
            .shard_receive
            .lock()
            .expect("small-file receive records lock poisoned")
            .clone();
        shard_receive.sort_by_key(|record| record.seq);
        let mut shard_sink = self
            .stats
            .shard_sink
            .lock()
            .expect("small-file sink records lock poisoned")
            .clone();
        shard_sink.sort_by_key(|record| record.seq);
        let report = SmallFileProbeReport {
            schema: 1,
            run_id: self.emitter.run_id.to_string(),
            correlation_id: self.correlation_id.to_string(),
            endpoint_role: self.endpoint_role,
            initiator_role: self.initiator_role,
            carrier: self.carrier,
            event: "summary",
            success: true,
            unix_ns: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_nanos(),
            elapsed_ns: duration_ns(self.origin.elapsed()),
            source_bookkeeping: SourceBookkeepingReport {
                manifest_entries_inserted: source.manifest_entries_inserted.load(Ordering::Relaxed),
                manifest_insert_sync_wait: source.manifest_insert_sync_wait.report(),
                manifest_insert_map_op: source.manifest_insert_map_op.report(),
                need_entries_resolved: source.need_entries_resolved.load(Ordering::Relaxed),
                need_resolve_sync_wait: source.need_resolve_sync_wait.report(),
                need_resolve_map_op: source.need_resolve_map_op.report(),
                need_event_send: source.need_event_send.report(),
                need_event_hop: source.need_event_hop.report(),
                need_handler_work: source.need_handler_work.report(),
                planner: source.planner.report(),
                planner_input_entries: source.planner_input_entries.load(Ordering::Relaxed),
                planned_payloads: source.planned_payloads.load(Ordering::Relaxed),
                planned_tar_shards: source.planned_tar_shards.load(Ordering::Relaxed),
                planned_tar_members: source.planned_tar_members.load(Ordering::Relaxed),
                tar_queue: source.tar_queue.report(),
                tar_shards_queued: source.tar_shards_queued.load(Ordering::Relaxed),
                tar_members_queued: source.tar_members_queued.load(Ordering::Relaxed),
            },
            tcp_claims: self.stats.tcp_claims.report(),
            in_stream_claims: self.stats.in_stream_claims.report(),
            shard_receive,
            shard_receive_dropped: self.stats.shard_receive_dropped.load(Ordering::Relaxed),
            shard_sink,
            shard_sink_dropped: self.stats.shard_sink_dropped.load(Ordering::Relaxed),
        };
        (self.emitter.emit)(report);
    }
}

fn push_bounded<T>(records: &Mutex<Vec<T>>, dropped: &AtomicU64, record: T) {
    let mut records = records.lock().expect("small-file records lock poisoned");
    if records.len() < MAX_SHARD_RECORDS {
        records.push(record);
    } else {
        dropped.fetch_add(1, Ordering::Relaxed);
    }
}

fn duration_ns(duration: Duration) -> u64 {
    duration.as_nanos().min(u64::MAX as u128) as u64
}

fn instant_offset_ns(origin: Instant, instant: Instant) -> u64 {
    duration_ns(instant.saturating_duration_since(origin))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn env_activation_requires_the_canonical_nonempty_run_id() {
        let writer_called = std::cell::Cell::new(false);
        let absent = SmallFileProbe::default().or_from_env_with(
            |name| (name == RUN_ID_ENV).then(|| "must-not-activate".into()),
            |run_id| {
                writer_called.set(true);
                SmallFileProbe::capture(run_id, |_| {})
            },
        );
        assert!(absent.emitter.is_none());
        assert!(!writer_called.get());

        let missing = SmallFileProbe::default().or_from_env_with(
            |name| (name == TRACE_ENV).then(|| "1".into()),
            |run_id| SmallFileProbe::capture(run_id, |_| {}),
        );
        assert!(missing.emitter.is_none());

        let enabled = SmallFileProbe::default().or_from_env_with(
            |name| match name {
                TRACE_ENV => Some("true".into()),
                RUN_ID_ENV => Some("p2-guard".into()),
                _ => None,
            },
            |run_id| SmallFileProbe::capture(run_id, |_| {}),
        );
        assert_eq!(
            enabled
                .emitter
                .as_ref()
                .map(|emitter| emitter.run_id.as_ref()),
            Some("p2-guard")
        );
    }

    #[test]
    fn shard_ids_are_stable_ordered_and_key_isolated() {
        let headers = vec![
            FileHeader {
                relative_path: "a/one".into(),
                ..Default::default()
            },
            FileHeader {
                relative_path: "b/two".into(),
                ..Default::default()
            },
        ];
        let bind = |run_id| {
            SmallFileProbe::capture(run_id, |_| {})
                .bind(
                    None,
                    SessionPhaseRole::Destination,
                    SessionPhaseRole::Source,
                    SmallFileCarrier::InStream,
                )
                .unwrap()
        };
        let first = bind("run-one");
        let second = bind("run-two");
        let first_id = first.shard_id(&headers);
        assert_eq!(first_id, first.shard_id(&headers));
        assert_ne!(first_id, second.shard_id(&headers));
        let mut reversed = headers;
        reversed.reverse();
        assert_ne!(first_id, first.shard_id(&reversed));
    }

    #[test]
    fn finish_is_single_shot_and_preserves_zero_fields() {
        let reports: Arc<Mutex<Vec<SmallFileProbeReport>>> = Arc::default();
        let sink = Arc::clone(&reports);
        let probe = SmallFileProbe::capture("run", move |report| {
            sink.lock().unwrap().push(report);
        });
        let bound = probe
            .bind(
                None,
                SessionPhaseRole::Source,
                SessionPhaseRole::Source,
                SmallFileCarrier::InStream,
            )
            .unwrap();
        bound.finish();
        bound.finish();
        let reports = reports.lock().unwrap();
        assert_eq!(reports.len(), 1);
        assert_eq!(reports[0].source_bookkeeping, Default::default());
        assert_eq!(reports[0].tcp_claims, Default::default());
        assert!(reports[0].shard_receive.is_empty());
    }
}
