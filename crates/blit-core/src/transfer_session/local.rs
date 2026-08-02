//! Local transfers on the unified session (otp-11,
//! `docs/plan/OTP11_LOCAL_SESSION.md`).
//!
//! [`run_local_session`] joins both role drivers over
//! [`super::transport::in_process_pair`] — the same choreography as every
//! remote session (manifest streaming, destination-owned diff, the one
//! mirror delete rule, the destination-computed summary) — with the
//! LOCAL byte-carrier: a [`LocalApply`] extension on the destination
//! config under which needed files are applied in-process through the
//! shared payload planner and [`FsTransferSink`] (clonefile /
//! block-clone / copy_file_range where the platform has them), so no
//! payload byte rides any transport. `LocalApply` is process-local
//! config with no wire representation: only a caller holding BOTH
//! roots — this entry — can construct it (D-2026-07-05-3's
//! capability-selected write strategy, never role or initiator).

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex as StdMutex};
use std::time::Instant;

use async_trait::async_trait;
use eyre::{eyre, Context, Result};
use tokio::sync::mpsc;

use crate::fs_enum::FileFilter;
use crate::generated::{FileHeader, MirrorMode, SessionOpen, TransferRole};
use crate::path_posix::relative_path_to_posix;
use crate::remote::transfer::payload::{TransferPayload, DEFAULT_PAYLOAD_PREFETCH};
use crate::remote::transfer::pipeline::execute_sink_pipeline_streaming;
use crate::remote::transfer::sink::{
    FileFailure, FsSinkConfig, FsTransferSink, NullSink, SinkOutcome, TransferSink,
};
use crate::remote::transfer::source::{
    FilteredSource, FsTransferSource, SourceScan, TransferSource,
};
use crate::remote::transfer::{RemoteTransferProgress, SmallFileProbe};
use crate::transfer_plan::PlanOptions;

use super::checkers::CheckerPool;
use super::phase_probe::{LocalPhase, LocalPhaseProbe};

/// Apply-pipeline workers for a normal run.
///
/// **This was 1**, which cost 2.7× on a local destination. Measured on the
/// owner's real tree (46,041 files / 7.36 GiB, `D:\Apps`) — the phase probe
/// put **81.7% of a local-to-local copy's wall clock in APPLY_BACKPRESSURE**,
/// i.e. the diff loop blocked handing payloads to the one sink worker:
///
/// | workers | local NVMe→NVMe | SMB destination |
/// |---|---|---|
/// | 1 | 17.68 s | 35.03 s |
/// | 2 | 9.50 s | — |
/// | 4 | 7.73 s | 35.22 s |
/// | 8 | 7.19 s | — |
/// | 16 | 6.58 s | 35.17 s |
///
/// 8 captures 2.46× of the available 2.69× and sits just past the knee.
///
/// Deliberately a FIXED value rather than the runtime-discovered treatment
/// `--checkers` got (D-2026-08-01-1). That decision applies to tuning the
/// program can work out at runtime, and here there is nothing to work out:
/// the network measurement is FLAT — concurrency neither helps nor hurts —
/// so no destination seen so far has an optimum below this. Adding an
/// adaptive throttle to the WRITE path, where per-file containment and
/// failure classification live, would be real risk bought for no measured
/// gain.
///
/// Known gap, stated rather than implied: only NVMe and SMB were measured.
/// A spinning disk or a heavily contended share could plausibly prefer
/// fewer, and no evidence either way exists yet. The hidden `--workers` pin
/// remains the diagnostic escape hatch if that turns up.
pub const DEFAULT_SINK_WORKERS: usize = 8;
use super::transport::in_process_pair;
use super::{
    run_destination, run_source, DestinationInstruments, DestinationSessionConfig,
    DestinationTarget, HelloConfig, SessionEndpoint, SourceInstruments, SourceSessionConfig,
};

// ---------------------------------------------------------------------------
// The local option/summary surface (re-homed from the deleted
// engine/options.rs + engine/summary.rs at otp-11b — the engine died;
// these types are the app-facing local contract, D2 of the slice doc).
// ---------------------------------------------------------------------------

/// Scope of mirror deletions. Matches the wire-side `MirrorMode` enum
/// (FilteredSubset / All). R58-F6 brought local up to parity with the
/// remote paths' wire `MirrorMode` scope.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum LocalMirrorDeleteScope {
    /// Default: only delete destination entries that the source-side
    /// filter would have allowed. Files matching `--exclude` patterns
    /// at the destination are left alone, because they're not in
    /// scope for this mirror operation.
    #[default]
    FilteredSubset,
    /// Delete every destination entry not present at the source,
    /// regardless of filter scope. Selected via `--delete-scope all`.
    All,
}

/// Local comparison policy. Mirrors the wire-side `ComparisonMode` enum
/// so local copy/mirror behaves the same as a same-options remote run.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum LocalCompareMode {
    /// Default size + mtime. Skip if both match.
    #[default]
    SizeMtime,
    /// Compare by Blake3 checksum. Slow but content-accurate.
    Checksum,
    /// Compare by size only. Mtime differences are ignored.
    SizeOnly,
    /// Transfer regardless of target state.
    Force,
    /// Transfer all files unconditionally (--ignore-times). Same
    /// outcome as Force at the planner level; kept as a separate
    /// variant so the user's intent is preserved in summaries.
    IgnoreTimes,
}

impl LocalCompareMode {
    /// Resolve onto the unified wire-side `ComparisonMode`, honoring
    /// the legacy `checksum: bool` under the default `SizeMtime`
    /// (back-compat: `--checksum` callers that haven't migrated to
    /// `compare_mode` keep their behavior). ue-r2-1c: the single home
    /// for this translation.
    pub fn resolve_comparison_mode(
        self,
        legacy_checksum: bool,
    ) -> crate::generated::ComparisonMode {
        use crate::generated::ComparisonMode;
        match self {
            LocalCompareMode::Checksum => ComparisonMode::Checksum,
            LocalCompareMode::SizeOnly => ComparisonMode::SizeOnly,
            LocalCompareMode::Force => ComparisonMode::Force,
            LocalCompareMode::IgnoreTimes => ComparisonMode::IgnoreTimes,
            LocalCompareMode::SizeMtime => {
                if legacy_checksum {
                    ComparisonMode::Checksum
                } else {
                    ComparisonMode::SizeMtime
                }
            }
        }
    }

    /// Same resolution, onto the perf-history snapshot enum (tuning
    /// buckets key on the full comparison policy — R59 finding #5).
    pub(crate) fn resolve_compare_snapshot(
        self,
        legacy_checksum: bool,
    ) -> crate::perf_history::CompareModeSnapshot {
        use crate::perf_history::CompareModeSnapshot;
        match self {
            LocalCompareMode::Checksum => CompareModeSnapshot::Checksum,
            LocalCompareMode::SizeOnly => CompareModeSnapshot::SizeOnly,
            LocalCompareMode::Force => CompareModeSnapshot::Force,
            LocalCompareMode::IgnoreTimes => CompareModeSnapshot::IgnoreTimes,
            LocalCompareMode::SizeMtime => {
                if legacy_checksum {
                    CompareModeSnapshot::Checksum
                } else {
                    CompareModeSnapshot::SizeMtime
                }
            }
        }
    }
}

/// Options for executing a local mirror/copy operation. The dead
/// engine-era axes (`force_tar`, `preserve_symlinks`,
/// `include_symlinks`, `skip_unchanged`) retired with the engine at
/// otp-11b — none was reachable from any production caller (slice doc
/// D2/F6 adjudication).
#[derive(Clone, Debug)]
pub struct LocalMirrorOptions {
    pub filter: FileFilter,
    pub mirror: bool,
    pub dry_run: bool,
    /// Presentation intent only (the caller decided to show progress).
    /// The event lane is `progress_events`; a caller that wants live
    /// events must attach one.
    pub progress: bool,
    /// clp-1: the caller's live-progress lane. A local session runs BOTH
    /// roles in this process, so ONE sink covers both: source-side
    /// enumeration liveness and the destination's diff/apply events land
    /// on the caller's single channel, with the caller as the only
    /// consumer. `None` (TUI, tests, every non-`-p` run) keeps the
    /// session event-free and the enumeration lines on stderr.
    pub progress_events: Option<RemoteTransferProgress>,
    pub verbose: bool,
    pub perf_history: bool,
    /// Skip any file the destination already has, regardless of
    /// comparison mode. Orthogonal to `checksum`; matches the wire
    /// `ignore_existing` for full route parity.
    pub ignore_existing: bool,
    /// Explicitly discard Windows attributes and named data streams at the
    /// SOURCE. False preserves strictly.
    pub drop_windows_metadata: bool,
    pub checksum: bool,
    /// R58-F7: comparison policy — `--size-only` / `--ignore-times` /
    /// `--force` honored on local copy/mirror the same way the remote
    /// routes honor them.
    pub compare_mode: LocalCompareMode,
    /// R58-F6: delete-scope policy for mirror. Only consulted when
    /// `mirror == true`.
    pub delete_scope: LocalMirrorDeleteScope,
    /// The hidden `--workers` debug limiter (always paired with
    /// `debug_mode`); bounds the apply pipeline's worker count.
    pub workers: usize,
    pub preserve_times: bool,
    pub debug_mode: bool,
    /// Resume interrupted transfers using block-level comparison (the
    /// local carrier's sink-level block phase).
    pub resume: bool,
    /// Discard writes (NullSink). Measures source read + pipeline
    /// throughput.
    pub null_sink: bool,
    /// Destination-comparison concurrency. **`0` — the value every real run
    /// uses — discovers it at runtime.**
    ///
    /// A non-zero value pins it and exists only for diagnostic comparison
    /// runs; there is deliberately no advertised CLI flag, because tuning the
    /// program can work out for itself must not become user-facing surface
    /// (FAST, SIMPLE, RELIABLE — see `.agents/repo-guidance.md`).
    pub checkers: usize,
    /// Test-injection seam for the write backend — see [`SinkOverride`].
    /// Absent from production builds entirely.
    #[cfg(test)]
    pub sink_override: Option<SinkOverride>,
    /// Test-injection seam for the destination directory-sweep cache
    /// (ls-5): the test holds the very cache the session answers from and
    /// asserts it DID answer — the cr-ls1-9/cr-ls4-1 lesson, applied at
    /// build time rather than after a reviewer runs the revert. Absent
    /// from production builds entirely.
    #[cfg(test)]
    pub dir_stat_probe: Option<Arc<super::dir_stat::DirStatCache>>,
    /// Pre-built comparison pool. `None` (production) builds one from
    /// [`LocalMirrorOptions::checkers`].
    ///
    /// cr-ls1-9: injectable for the same reason `phase_probe` is — so a test
    /// can hold the very pool the session uses and assert the production diff
    /// actually dispatched onto it. Without that, removing the wiring at the
    /// call site leaves every checker test green.
    pub checker_pool: Option<CheckerPool>,
    /// ls-1 wall-clock breakdown. Default permits environment activation
    /// (`BLIT_TRACE_LOCAL_PHASES=1` + `BLIT_TRACE_RUN_ID`) and is otherwise
    /// inert; a caller that needs deterministic behaviour installs its own
    /// with [`LocalPhaseProbe::capture`] or [`LocalPhaseProbe::disabled`].
    pub phase_probe: LocalPhaseProbe,
}

impl LocalMirrorOptions {
    /// Apply-pipeline workers this configuration will actually use.
    ///
    /// The session reads it from here rather than inlining the expression, so
    /// a test can assert the effective count through the SAME path production
    /// takes. Asserting on [`DEFAULT_SINK_WORKERS`] directly is a
    /// compile-time constant comparison — clippy rejects it, correctly, as
    /// proving nothing about what a session does.
    pub fn effective_sink_workers(&self) -> usize {
        if self.debug_mode {
            self.workers.max(1)
        } else {
            DEFAULT_SINK_WORKERS
        }
    }
}

/// A caller-supplied write backend, replacing the sink the session would
/// build (`FsTransferSink`, or `NullSink` under `--null`).
///
/// ls-4 (r10 `ls4-guard`): this exists so a test can prove the session's
/// apply pipeline RUNS CONCURRENTLY, not merely that it is configured to.
/// The reviewer forced the worker wiring back to one and every ls-4 test
/// stayed green — the same implementation-not-the-seam gap as cr-ls1-9,
/// which `checker_pool` injection closed for the diff. A wrapping sink that
/// measures peak in-flight `write_payload` calls is the only observer that
/// can see the difference, and it needs this seam to get installed.
///
/// `cfg(test)` (r11 `test-seam-api`): the seam is COMPILED OUT of production
/// rather than merely unused there. A public override would let any caller
/// bypass the `FsTransferSink`/`NullSink` construction that enforces
/// dry-run, checksum and resume behaviour, and would change the public
/// struct for a test's benefit. Only this crate's unit tests can name it,
/// and the release binary carries neither the field nor the branch.
#[cfg(test)]
#[derive(Clone)]
pub struct SinkOverride(pub Arc<dyn TransferSink>);

/// Hand-written because `dyn TransferSink` is not `Debug`. Reports presence,
/// not contents — the same shape as `LocalPhaseProbe`'s.
#[cfg(test)]
impl std::fmt::Debug for SinkOverride {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SinkOverride").finish_non_exhaustive()
    }
}

impl Default for LocalMirrorOptions {
    fn default() -> Self {
        Self {
            filter: FileFilter::default(),
            mirror: false,
            dry_run: false,
            progress: false,
            progress_events: None,
            verbose: false,
            perf_history: true,
            ignore_existing: false,
            drop_windows_metadata: false,
            checksum: false,
            compare_mode: LocalCompareMode::default(),
            delete_scope: LocalMirrorDeleteScope::default(),
            workers: num_cpus::get().max(1),
            preserve_times: true,
            debug_mode: false,
            resume: false,
            null_sink: false,
            checkers: 0,
            #[cfg(test)]
            sink_override: None,
            #[cfg(test)]
            dir_stat_probe: None,
            checker_pool: None,
            phase_probe: LocalPhaseProbe::default(),
        }
    }
}

/// Why a transfer copied zero files. `JournalSkip` retired at otp-11b
/// with the unsound engine journal fast path (proven silent data loss
/// — see `docs/bench/otp11-local-2026-07-11/README.md`).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub enum TransferOutcome {
    /// Normal case: some work was attempted (files examined, possibly copied).
    #[default]
    Transferred,
    /// The run examined the source and found it up to date with the dest.
    UpToDate,
    /// The run examined the source and it contained zero files.
    SourceEmpty,
}

/// Summary of a local transfer execution.
///
///   - `scanned_files` / `scanned_bytes`: the source-side workload
///     observed by enumeration (post-filter, pre-diff).
///   - `planned_files`: entries the diff decided to transfer.
///   - `copied_files`: what the apply pipeline actually wrote.
///   - `total_bytes`: bytes the pipeline wrote — distinct from
///     `scanned_bytes` on incremental runs.
#[derive(Clone, Debug, Default)]
pub struct LocalMirrorSummary {
    pub planned_files: usize,
    pub copied_files: usize,
    pub total_bytes: u64,
    pub scanned_files: usize,
    pub scanned_bytes: u64,
    pub deleted_files: usize,
    pub deleted_dirs: usize,
    pub dry_run: bool,
    pub duration: std::time::Duration,
    pub tar_shard_tasks: usize,
    pub tar_shard_files: usize,
    pub tar_shard_bytes: u64,
    pub raw_bundle_tasks: usize,
    pub raw_bundle_files: usize,
    pub raw_bundle_bytes: u64,
    pub large_tasks: usize,
    pub large_bytes: u64,
    /// Classifier for the CLI summary line.
    pub outcome: TransferOutcome,
    /// R47-F4: source-side paths that couldn't be scanned or read.
    /// Destructive follow-ups in the caller — most importantly
    /// `blit move`'s source-side delete — MUST inspect this and
    /// refuse when non-empty.
    pub unreadable_paths: Vec<String>,
    /// Destination-side per-file failures the session contained instead
    /// of aborting (pfc-4, D-2026-07-30-1). Exact count, including any
    /// past the reported-detail cap.
    pub files_failed: u64,
    /// The named subset of `files_failed`, capped at
    /// [`crate::remote::transfer::MAX_REPORTED_FILE_FAILURES`] — the
    /// same bounded report the wire carries, so local and remote runs
    /// render one shape.
    pub failures: Vec<FileFailure>,
    /// Files whose destination attributes were repaired in place at diff
    /// time, with no payload bytes re-sent (pfc-6). Counted by the
    /// destination end, so a remote session repairs the same way without
    /// an initiator-visible count — the wire summary is not extended.
    pub files_repaired: u64,
}

/// Process-local destination extension: apply needed files in-process
/// instead of requesting them from the source. Constructed only by
/// [`run_local_session`] — the fields are crate-private, so no caller
/// outside this crate (and no wire peer, which has no representation
/// for it at all) can select the local carrier.
pub struct LocalApply {
    /// Source root for the payload planner (absolute paths =
    /// `src_root.join(relative_path)`).
    pub(super) src_root: PathBuf,
    /// The pre-built local write backend (FsTransferSink with the full
    /// user config, or NullSink under `--null`).
    pub(super) sink: Arc<dyn TransferSink>,
    /// Unfiltered source used by the apply pipeline to prepare
    /// payloads (tar builds, availability checks). Filtering already
    /// happened at scan time; prepare only reads planned entries.
    pub(super) prepare_source: Arc<dyn TransferSource>,
    /// Planner knobs for grouping needed headers into payloads —
    /// the same planner the session source uses for its needs.
    pub(super) plan_options: PlanOptions,
    /// Mirror delete scope under `MirrorMode::FilteredSubset`: the
    /// user's `FileFilter` directly (process-local twin of deriving it
    /// from the wire `SessionOpen.filter` — same type, same delete
    /// pass).
    pub(super) mirror_scope_filter: FileFilter,
    /// `--dry-run`: the sink already refuses writes; the mirror delete
    /// pass runs in plan-only mode (counts, deletes nothing).
    pub(super) dry_run: bool,
    /// `--null`: the sink discards every payload. Read only by
    /// [`LocalApply::applies_changes`] — a diagnostics run must not become
    /// the one thing that mutates the destination (pfc-6).
    pub(super) null_sink: bool,
    /// Pipeline worker count: 1 (the old streaming pipeline's default
    /// shape) unless the hidden `--workers` debug limiter set
    /// `debug_mode` (codex otp-11a F7).
    pub(super) sink_workers: usize,
    /// Shared unreadable-path accumulator (same Arc the source scan
    /// feeds): apply-side availability failures land here too, so
    /// `blit move`'s source-delete gate sees one merged list.
    pub(super) unreadable: Arc<StdMutex<Vec<String>>>,
    /// Counters the entry folds into [`LocalMirrorSummary`] afterward.
    pub(super) stats: Arc<LocalApplyStats>,
    /// ls-1 wall-clock breakdown, resolved once by the session entry. Rides
    /// here because `LocalApply` is the one value threaded through every
    /// destination-side local phase (diff, plan, apply, delete).
    pub(super) phase_probe: LocalPhaseProbe,
    /// The session's dedicated destination-comparison pool (`--checkers`),
    /// built once and shared by every chunk so threads are not respawned per
    /// chunk.
    pub(super) checker_pool: CheckerPool,
    /// ls-5: the session's destination directory-sweep cache — one
    /// `read_dir` per directory answers most per-file target resolutions
    /// (see [`super::dir_stat::DirStatCache`]). Carried here so the
    /// session drive loop shares the instance a test may have injected.
    pub(super) dir_stats: Arc<super::dir_stat::DirStatCache>,
}

/// Destination-side counters for the local summary. Atomics because
/// the diff loop (control lane) and the delete pass (SourceDone arm)
/// write them at different points of the session.
#[derive(Default)]
pub struct LocalApplyStats {
    pub(super) scanned_files: AtomicU64,
    pub(super) scanned_bytes: AtomicU64,
    pub(super) tar_shard_tasks: AtomicU64,
    pub(super) tar_shard_files: AtomicU64,
    pub(super) tar_shard_bytes: AtomicU64,
    pub(super) large_tasks: AtomicU64,
    pub(super) large_bytes: AtomicU64,
    pub(super) deleted_files: AtomicU64,
    pub(super) deleted_dirs: AtomicU64,
}

/// A running local-apply pipeline: the destination diff queues
/// payloads, `finish()` closes the queue and joins the pipeline for
/// the write totals (the same join discipline as the data-plane
/// receive). A run dropped WITHOUT `finish()` — a session error or a
/// cancelled future — aborts the pipeline task at its next payload
/// boundary (codex otp-11a F3): the in-flight `spawn_blocking` write
/// completes, queued payloads are dropped, and no write continues
/// behind an operation that already returned.
pub(super) struct LocalApplyRun {
    payload_tx: Option<mpsc::Sender<TransferPayload>>,
    pipeline: Option<tokio::task::JoinHandle<Result<SinkOutcome>>>,
    /// ls-1: times the drain in [`LocalApplyRun::finish`].
    phase_probe: LocalPhaseProbe,
}

impl Drop for LocalApplyRun {
    fn drop(&mut self) {
        if let Some(handle) = &self.pipeline {
            handle.abort();
        }
    }
}

impl LocalApply {
    /// Does this destination write files in this session? False for
    /// `--dry-run` (which must not mutate at all — its mirror pass is
    /// plan-only too) and `--null` (a throughput diagnostic whose sink
    /// discards payloads; its mirror delete pass has always executed,
    /// but its copy path must stay write-free). The destination diff's
    /// in-place attribute repair (pfc-6) is off for both.
    pub(super) fn applies_changes(&self) -> bool {
        !self.dry_run && !self.null_sink
    }

    /// Spawn the apply pipeline — the shared streaming sink pipeline
    /// (prefetched prepares, blocking-pool writes) over this config's
    /// sink.
    pub(super) fn start(&self, progress: Option<RemoteTransferProgress>) -> LocalApplyRun {
        let (payload_tx, payload_rx) =
            mpsc::channel::<TransferPayload>(DEFAULT_PAYLOAD_PREFETCH.max(1));
        let source = Arc::clone(&self.prepare_source);
        // One pipeline worker per sink handle — the old streaming
        // pipeline's default shape is one; the hidden `--workers`
        // debug limiter (which always sets debug_mode) widens it
        // (codex otp-11a F7).
        let sinks: Vec<Arc<dyn TransferSink>> = (0..self.sink_workers.max(1))
            .map(|_| Arc::clone(&self.sink))
            .collect();
        let pipeline = tokio::spawn(async move {
            execute_sink_pipeline_streaming(
                source,
                sinks,
                payload_rx,
                DEFAULT_PAYLOAD_PREFETCH,
                progress.as_ref(),
            )
            .await
        });
        LocalApplyRun {
            payload_tx: Some(payload_tx),
            pipeline: Some(pipeline),
            phase_probe: self.phase_probe.clone(),
        }
    }

    /// Group one diff chunk's needed headers into payloads, folding
    /// the planner-mix counters. Unavailable (unreadable) entries are
    /// dropped into the shared accumulator and skipped — the old local
    /// pipeline's copy-what-is-readable posture; the caller-side move
    /// gate refuses the source delete when the list is non-empty.
    pub(super) async fn plan_chunk(&self, needed: Vec<FileHeader>) -> Result<Vec<TransferPayload>> {
        if needed.is_empty() {
            return Ok(Vec::new());
        }
        let available = self
            .prepare_source
            .check_availability(needed, Arc::clone(&self.unreadable))
            .await?;
        let payloads = crate::remote::transfer::payload::plan_transfer_payloads(
            available,
            &self.src_root,
            self.plan_options,
        )?;
        for payload in &payloads {
            match payload {
                TransferPayload::TarShard { headers } => {
                    self.stats.tar_shard_tasks.fetch_add(1, Ordering::Relaxed);
                    self.stats
                        .tar_shard_files
                        .fetch_add(headers.len() as u64, Ordering::Relaxed);
                    self.stats.tar_shard_bytes.fetch_add(
                        headers.iter().map(|h| h.size).sum::<u64>(),
                        Ordering::Relaxed,
                    );
                }
                TransferPayload::File(header) => {
                    self.stats.large_tasks.fetch_add(1, Ordering::Relaxed);
                    self.stats
                        .large_bytes
                        .fetch_add(header.size, Ordering::Relaxed);
                }
                // The local planner emits only File/TarShard; resume
                // block payloads are receive-side wire shapes.
                _ => {}
            }
        }
        Ok(payloads)
    }
}

impl LocalApplyRun {
    /// Queue one payload (bounded — the diff loop inherits the
    /// pipeline's backpressure, exactly as the wire carriers lean on
    /// transport backpressure).
    pub(super) async fn queue(&self, payload: TransferPayload) -> Result<()> {
        self.payload_tx
            .as_ref()
            .expect("local apply queue used after finish")
            .send(payload)
            .await
            .map_err(|_| eyre!("local apply pipeline stopped early"))
    }

    /// Close the queue and join the pipeline. Returns the write
    /// totals; surfaces the pipeline's own error as the root cause.
    pub(super) async fn finish(mut self) -> Result<SinkOutcome> {
        // ls-1: dropping the sender closes the queue, so everything after
        // this point is the pipeline draining work the diff already handed
        // it — see `LocalPhase::Apply` for what that does and does not mean.
        self.payload_tx.take();
        let drain_started = self.phase_probe.is_enabled().then(Instant::now);
        let pipeline = self
            .pipeline
            .take()
            .expect("local apply pipeline joined twice");
        let outcome = pipeline
            .await
            .map_err(|err| eyre!("local apply pipeline panicked: {err}"));
        if let Some(started) = drain_started {
            self.phase_probe
                .record(LocalPhase::Apply, started.elapsed());
        }
        outcome?
    }
}

/// Source wrapper that drops manifest entries under the destination
/// subtree when the destination sits inside the source — the session
/// twin of the old engine's `exclude_dest_subtree` (pinned by
/// `nested_destination_does_not_self_copy`: without it, each run
/// re-copies the destination into itself one level deeper).
struct DestSubtreeExcludedSource {
    inner: Arc<dyn TransferSource>,
    /// POSIX-form relative path of the destination under the source
    /// root (no trailing slash).
    exclude_rel: String,
}

#[async_trait]
impl TransferSource for DestSubtreeExcludedSource {
    fn scan(
        &self,
        filter: Option<FileFilter>,
        unreadable_paths: Arc<StdMutex<Vec<String>>>,
    ) -> (mpsc::Receiver<FileHeader>, SourceScan) {
        self.scan_with_metadata_policy(filter, unreadable_paths, true)
    }

    fn scan_without_windows_metadata(
        &self,
        filter: Option<FileFilter>,
        unreadable_paths: Arc<StdMutex<Vec<String>>>,
    ) -> (mpsc::Receiver<FileHeader>, SourceScan) {
        self.scan_with_metadata_policy(filter, unreadable_paths, false)
    }

    async fn prepare_payload(
        &self,
        payload: TransferPayload,
    ) -> Result<crate::remote::transfer::payload::PreparedPayload> {
        self.inner.prepare_payload(payload).await
    }

    async fn check_availability(
        &self,
        headers: Vec<FileHeader>,
        unreadable_paths: Arc<StdMutex<Vec<String>>>,
    ) -> Result<Vec<FileHeader>> {
        self.inner
            .check_availability(headers, unreadable_paths)
            .await
    }

    async fn open_file(
        &self,
        header: &FileHeader,
    ) -> Result<Box<dyn tokio::io::AsyncRead + Unpin + Send>> {
        self.inner.open_file(header).await
    }

    fn root(&self) -> &Path {
        self.inner.root()
    }
}

impl DestSubtreeExcludedSource {
    fn scan_with_metadata_policy(
        &self,
        filter: Option<FileFilter>,
        unreadable_paths: Arc<StdMutex<Vec<String>>>,
        preserve_windows_metadata: bool,
    ) -> (mpsc::Receiver<FileHeader>, SourceScan) {
        let (mut inner_rx, mut scan) = if preserve_windows_metadata {
            self.inner.scan(filter, unreadable_paths)
        } else {
            self.inner
                .scan_without_windows_metadata(filter, unreadable_paths)
        };
        let (tx, rx) = mpsc::channel(64);
        let exact = self.exclude_rel.clone();
        let prefix = format!("{}/", self.exclude_rel);
        let handle = tokio::spawn(async move {
            let mut forwarded = 0u64;
            while let Some(header) = inner_rx.recv().await {
                if header.relative_path == exact || header.relative_path.starts_with(&prefix) {
                    continue;
                }
                forwarded += 1;
                if tx.send(header).await.is_err() {
                    break;
                }
            }
            Ok(forwarded)
        });
        scan.replace_primary(handle);
        (rx, scan)
    }
}

/// The destination's POSIX relative path under the source root, when
/// (and only when) it nests inside it. Same lexical check the old
/// engine used.
fn dest_subtree_rel(src_root: &Path, dst_root: &Path) -> Option<String> {
    match dst_root.strip_prefix(src_root) {
        Ok(rel) if !rel.as_os_str().is_empty() => Some(relative_path_to_posix(rel)),
        _ => None,
    }
}

/// Run one LOCAL transfer as a full session: both role drivers joined
/// over the in-process pair, bytes applied through [`LocalApply`].
/// This is the ONLY local transfer entry (D-2026-07-05-1) — the
/// `blit_app::transfers::local::run` chokepoint (CLI + TUI) rides it.
pub async fn run_local_session(
    src_root: &Path,
    dst_root: &Path,
    options: LocalMirrorOptions,
) -> Result<LocalMirrorSummary> {
    let started = Instant::now();
    // ls-1: resolve the probe once, here, so every phase folds into one
    // accumulator and the environment is read exactly once per session.
    let phase_probe = options.phase_probe.clone().or_from_env();
    // One dedicated comparison pool per session, built before any chunk so
    // its threads are not respawned per chunk.
    let checker_pool = match options.checker_pool.clone() {
        Some(pool) => pool,
        None => CheckerPool::new(options.checkers)?,
    };

    if !src_root.exists() {
        return Err(eyre!("source path does not exist: {}", src_root.display()));
    }
    if !options.dry_run {
        if let Some(parent) = dst_root.parent() {
            std::fs::create_dir_all(parent).with_context(|| {
                format!("failed to create destination parent {}", parent.display())
            })?;
        }
    }

    let compare_mode = options
        .compare_mode
        .resolve_comparison_mode(options.checksum);
    let mirror_kind = if options.mirror {
        match options.delete_scope {
            LocalMirrorDeleteScope::FilteredSubset => MirrorMode::FilteredSubset,
            LocalMirrorDeleteScope::All => MirrorMode::All,
        }
    } else {
        MirrorMode::Off
    };
    let open = SessionOpen {
        initiator_role: TransferRole::Source as i32,
        compare_mode: compare_mode as i32,
        ignore_existing: options.ignore_existing,
        drop_windows_metadata: options.drop_windows_metadata,
        // The local carrier moves no bytes on any lane; in-stream keeps
        // the responder from binding a TCP data plane.
        in_stream_bytes: true,
        mirror_enabled: options.mirror,
        mirror_kind: mirror_kind as i32,
        ..Default::default()
    };

    // One merged unreadable list: scan-side (source instruments) and
    // apply-side (availability checks) — the move gate reads it whole.
    let unreadable: Arc<StdMutex<Vec<String>>> = Arc::default();

    // Source chain: fs source → user filter (the universal
    // FilteredSource chokepoint, same as push/pull) → dest-subtree
    // exclusion when dst nests inside src.
    //
    // clp-1: the fs source carries the caller's progress lane, so its
    // enumeration liveness reports as events instead of raw stderr
    // lines that would fight the caller's live row. `prepare_source`
    // shares this instance and never scans, so the handle is inert
    // there.
    let fs_source: Arc<dyn TransferSource> = Arc::new(
        FsTransferSource::new(src_root.to_path_buf())
            .with_progress(options.progress_events.clone())
            .with_phase_probe(phase_probe.clone())
            // audit-16: a sink-less run (no progress lane attached) only
            // prints the raw enumeration heartbeat under `--verbose`, or
            // when `-p` was requested at all (clp-2 residue c: `-p` on a
            // redirected/non-TTY stderr can't draw a row, so the sink
            // never attaches either — that fallback keeps the heartbeat
            // as the only liveness signal and predates this gate).
            .with_verbose(options.verbose || options.progress),
    );
    let filtered: Arc<dyn TransferSource> = Arc::new(FilteredSource::new(
        Arc::clone(&fs_source),
        options.filter.clone_without_cache(),
    ));
    let scan_source: Arc<dyn TransferSource> = match dest_subtree_rel(src_root, dst_root) {
        Some(exclude_rel) => Arc::new(DestSubtreeExcludedSource {
            inner: filtered,
            exclude_rel,
        }),
        None => filtered,
    };

    // Local write backend — the old orchestrator's exact construction.
    // ls-4/r11: in TEST builds only, an injected override wins so a test can
    // observe the pipeline's real concurrency through this exact entry point
    // (see `SinkOverride`). Production builds compile neither the field nor
    // this branch.
    #[cfg(test)]
    let test_override: Option<Arc<dyn TransferSink>> = options
        .sink_override
        .as_ref()
        .map(|SinkOverride(sink)| Arc::clone(sink));
    #[cfg(not(test))]
    let test_override: Option<Arc<dyn TransferSink>> = None;
    let sink: Arc<dyn TransferSink> = if let Some(sink) = test_override {
        sink
    } else if options.null_sink {
        Arc::new(NullSink::new())
    } else {
        Arc::new(FsTransferSink::new(
            src_root.to_path_buf(),
            dst_root.to_path_buf(),
            FsSinkConfig {
                preserve_times: options.preserve_times,
                dry_run: options.dry_run,
                checksum: if options.checksum {
                    Some(crate::checksum::ChecksumType::Blake3)
                } else {
                    None
                },
                resume: options.resume,
                compare_mode,
            },
        ))
    };

    // ls-5: the session's destination directory-sweep cache. In TEST
    // builds an injected instance wins for the same reason `SinkOverride`
    // exists — so a test can hold the very cache the session answers from
    // and assert it DID answer, which no tree comparison can see.
    #[cfg(test)]
    let dir_stats = options.dir_stat_probe.clone().unwrap_or_default();
    #[cfg(not(test))]
    let dir_stats = Arc::new(super::dir_stat::DirStatCache::default());

    let stats = Arc::new(LocalApplyStats::default());
    let local_apply = LocalApply {
        src_root: src_root.to_path_buf(),
        sink,
        prepare_source: Arc::clone(&fs_source),
        plan_options: PlanOptions::default(),
        mirror_scope_filter: options.filter.clone_without_cache(),
        dry_run: options.dry_run,
        null_sink: options.null_sink,
        sink_workers: options.effective_sink_workers(),
        unreadable: Arc::clone(&unreadable),
        stats: Arc::clone(&stats),
        phase_probe: phase_probe.clone(),
        checker_pool: checker_pool.clone(),
        dir_stats,
    };

    let source_cfg = SourceSessionConfig {
        hello: HelloConfig::default(),
        endpoint: SessionEndpoint::initiator(open),
        plan_options: PlanOptions::default(),
        data_plane_host: None,
        instruments: SourceInstruments {
            // The LOCAL carrier's source sends no payload record, so it
            // has no per-file completion to report — the destination's
            // apply pipeline owns the whole per-file lane. Keeping this
            // `None` is also what keeps pfc-4's source-side completion
            // retraction (`source_send_half`, cr-pfc2-2) out of a lane
            // that never completed anything.
            progress: None,
            unreadable: Some(Arc::clone(&unreadable)),
            trace_data_plane: false,
            session_phase_trace: Default::default(),
            lifecycle_trace: Default::default(),
            small_file_probe: SmallFileProbe::disabled(),
            #[cfg(test)]
            dial_test_samples: None,
            #[cfg(test)]
            dial_terminal_test_gate: None,
            #[cfg(test)]
            dial_proposal_test_gate: None,
            #[cfg(test)]
            dial_membership_test_gate: None,
        },
    };
    let dest_cfg = DestinationSessionConfig {
        hello: HelloConfig::default(),
        endpoint: SessionEndpoint::Responder,
        data_plane_host: None,
        receiver_capacity: None,
        instruments: DestinationInstruments {
            small_file_probe: SmallFileProbe::disabled(),
            // clp-1: the destination end of the caller's one lane — its
            // diff reports the files-to-transfer denominator and the
            // local apply pipeline reports bytes and per-file
            // completions (a file the sink contained as failed never
            // completes here, D-2026-07-30-1).
            progress: options.progress_events.clone(),
            ..Default::default()
        },
        local_apply: Some(local_apply),
    };

    let (a, b) = in_process_pair();
    let (source_result, dest_result) = tokio::join!(
        run_source(source_cfg, a, scan_source),
        run_destination(
            dest_cfg,
            b,
            DestinationTarget::Fixed(dst_root.to_path_buf())
        ),
    );
    // ls-1: emit before the fault match, so a failed session still yields its
    // breakdown — a run that died slowly is exactly the one worth timing.
    // cr-ls1-4: and say so in the artifact. On a failure the spans are a
    // floor, not a measurement, because the work inside them stopped early;
    // an unflagged truncated phase would read as a fast one.
    phase_probe.emit(
        started.elapsed(),
        dest_result.is_err() || source_result.is_err(),
    );
    // The destination is the scorer and holds the primary fault
    // (refusals, apply failures, delete failures); a source-only
    // failure (scan abort) surfaces when the destination succeeded.
    let outcome = match dest_result {
        Ok(outcome) => {
            source_result?;
            outcome
        }
        Err(err) => return Err(err),
    };

    let scanned_files = stats.scanned_files.load(Ordering::Relaxed) as usize;
    let scanned_bytes = stats.scanned_bytes.load(Ordering::Relaxed);
    let unreadable_paths = unreadable
        .lock()
        .map_err(|err| eyre!("unreadable-path accumulator poisoned: {err}"))?
        .clone();

    // Outcome classification mirrors the old fast-path gate (strategy
    // gate: mirror / checksum / non-SizeMtime compare all
    // forced streaming, which always reported Transferred).
    let fast_path_shape = !options.mirror
        && !options.checksum
        && matches!(options.compare_mode, LocalCompareMode::SizeMtime);
    let copied_files = outcome.summary.files_transferred as usize;
    let outcome_class = if fast_path_shape && scanned_files == 0 {
        TransferOutcome::SourceEmpty
    } else if fast_path_shape && copied_files == 0 {
        TransferOutcome::UpToDate
    } else {
        TransferOutcome::Transferred
    };

    let summary = LocalMirrorSummary {
        planned_files: outcome.needed_paths.len(),
        copied_files,
        total_bytes: outcome.summary.bytes_transferred,
        scanned_files,
        scanned_bytes,
        deleted_files: stats.deleted_files.load(Ordering::Relaxed) as usize,
        deleted_dirs: stats.deleted_dirs.load(Ordering::Relaxed) as usize,
        dry_run: options.dry_run,
        duration: started.elapsed(),
        tar_shard_tasks: stats.tar_shard_tasks.load(Ordering::Relaxed) as usize,
        tar_shard_files: stats.tar_shard_files.load(Ordering::Relaxed) as usize,
        tar_shard_bytes: stats.tar_shard_bytes.load(Ordering::Relaxed),
        raw_bundle_tasks: 0,
        raw_bundle_files: 0,
        raw_bundle_bytes: 0,
        large_tasks: stats.large_tasks.load(Ordering::Relaxed) as usize,
        large_bytes: stats.large_bytes.load(Ordering::Relaxed),
        outcome: outcome_class,
        unreadable_paths,
        // pfc-4: the destination's failure report, taken from the same
        // summary the wire carriers return, so the local surface pfc-5
        // renders is the remote one.
        files_failed: outcome.summary.files_failed,
        failures: outcome
            .summary
            .failures
            .iter()
            .map(FileFailure::from_wire)
            .collect(),
        // pfc-6: destination-local, so it comes off the outcome rather
        // than the wire summary the carriers exchange.
        files_repaired: outcome.files_repaired,
    };

    record_local_history(&summary, &options);

    Ok(summary)
}

/// Perf-history row for a local session run (D3 in the slice doc:
/// `blit profile` keeps its local data feed; the predictor and its
/// planner/transfer split retired with the engine, so the whole wall
/// time lands in `transfer_duration_ms`).
fn record_local_history(summary: &LocalMirrorSummary, options: &LocalMirrorOptions) {
    if !options.perf_history {
        return;
    }
    let record = build_local_record(summary, options);
    if let Err(err) = crate::perf_history::append_local_record(&record) {
        if options.verbose {
            // Through the log facade so a live progress row can route it
            // above the row instead of being scrolled away by it.
            log::warn!("failed to update performance history: {err:?}");
        }
    }
}

/// Construct the local session's [`PerformanceRecord`] without
/// touching disk — split from the writer so the record-shape contract
/// (R44-F1's "train and query on the same feature vector" invariant,
/// carried forward as "record scanned features") stays unit-testable,
/// the same rationale the engine's `build_performance_record` had.
fn build_local_record(
    summary: &LocalMirrorSummary,
    options: &LocalMirrorOptions,
) -> crate::perf_history::PerformanceRecord {
    use crate::perf_history::{OptionSnapshot, PerformanceRecord, TransferMode};
    let snapshot = OptionSnapshot {
        dry_run: options.dry_run,
        // The engine-era option axes retired at otp-11b; the persisted
        // snapshot schema keeps the fields — record the historical
        // defaults (the only values production ever produced).
        preserve_symlinks: true,
        include_symlinks: true,
        skip_unchanged: true,
        checksum: options.checksum,
        compare_mode: options
            .compare_mode
            .resolve_compare_snapshot(options.checksum),
        workers: options.workers,
    };
    let mode = if options.mirror {
        TransferMode::Mirror
    } else {
        TransferMode::Copy
    };
    // `--null` runs keep the old `null_sink` tag: RunKind derivation
    // keys on it (perf_history.rs), and a `"session"` tag would
    // classify diagnostics runs as Real and contaminate profiling
    // (codex otp-11a F9).
    let fast_path = if options.null_sink {
        "null_sink"
    } else {
        "session"
    };
    let mut record = PerformanceRecord::new(
        mode,
        None,
        None,
        summary.scanned_files,
        summary.scanned_bytes,
        snapshot,
        Some(fast_path.to_string()),
        0,
        summary.duration.as_millis(),
        0,
        0,
    );
    record.tar_shard_tasks = summary.tar_shard_tasks as u32;
    record.tar_shard_files = summary.tar_shard_files as u32;
    record.tar_shard_bytes = summary.tar_shard_bytes;
    record.large_tasks = summary.large_tasks as u32;
    record.large_bytes = summary.large_bytes;
    record
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::generated::{ComparisonMode, TransferSummary};
    use crate::transfer_session::DestinationOutcome;

    /// Delegates scan/prepare/open to a real fs source but drops one
    /// path at `check_availability`, recording it unreadable — the
    /// deterministic stand-in for a file vanishing between a CLEAN
    /// scan and the apply (the window the SourceDone mirror guard
    /// exists for; a mode-000 fixture is caught at scan time instead).
    struct VanishingSource {
        inner: Arc<dyn TransferSource>,
        vanish: String,
    }

    #[async_trait]
    impl TransferSource for VanishingSource {
        fn scan(
            &self,
            filter: Option<FileFilter>,
            unreadable_paths: Arc<StdMutex<Vec<String>>>,
        ) -> (mpsc::Receiver<FileHeader>, SourceScan) {
            self.inner.scan(filter, unreadable_paths)
        }

        async fn prepare_payload(
            &self,
            payload: TransferPayload,
        ) -> eyre::Result<crate::remote::transfer::payload::PreparedPayload> {
            self.inner.prepare_payload(payload).await
        }

        async fn check_availability(
            &self,
            headers: Vec<FileHeader>,
            unreadable_paths: Arc<StdMutex<Vec<String>>>,
        ) -> eyre::Result<Vec<FileHeader>> {
            let (gone, available): (Vec<_>, Vec<_>) = headers
                .into_iter()
                .partition(|h| h.relative_path == self.vanish);
            if !gone.is_empty() {
                unreadable_paths
                    .lock()
                    .expect("accumulator lock")
                    .push(self.vanish.clone());
            }
            Ok(available)
        }

        async fn open_file(
            &self,
            header: &FileHeader,
        ) -> eyre::Result<Box<dyn tokio::io::AsyncRead + Unpin + Send>> {
            self.inner.open_file(header).await
        }

        fn root(&self) -> &Path {
            self.inner.root()
        }
    }

    /// R46-F2 carried onto the local carrier (codex otp-11a F4): a
    /// source entry that vanishes AFTER a clean scan (recorded
    /// unreadable by the apply's availability check) must refuse the
    /// mirror at SourceDone, before any deletion — the old engine
    /// refused mirror deletions on ANY unreadable entry.
    #[tokio::test]
    async fn mirror_refuses_when_availability_drops_after_clean_scan() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let src_root = tmp.path().join("src");
        let dst_root = tmp.path().join("dst");
        std::fs::create_dir_all(&src_root).expect("mkdir src");
        std::fs::create_dir_all(&dst_root).expect("mkdir dst");
        std::fs::write(src_root.join("ok.txt"), b"fine").expect("write");
        std::fs::write(src_root.join("gone.txt"), b"vanishes").expect("write");
        std::fs::write(dst_root.join("extraneous.txt"), b"would die").expect("write");

        let open = SessionOpen {
            initiator_role: TransferRole::Source as i32,
            compare_mode: ComparisonMode::SizeMtime as i32,
            in_stream_bytes: true,
            mirror_enabled: true,
            mirror_kind: MirrorMode::All as i32,
            ..Default::default()
        };
        let unreadable: Arc<StdMutex<Vec<String>>> = Arc::default();
        let fs_source: Arc<dyn TransferSource> = Arc::new(FsTransferSource::new(src_root.clone()));
        let sink: Arc<dyn TransferSink> = Arc::new(FsTransferSink::new(
            src_root.clone(),
            dst_root.clone(),
            FsSinkConfig::default(),
        ));
        let local_apply = LocalApply {
            src_root: src_root.clone(),
            sink,
            prepare_source: Arc::new(VanishingSource {
                inner: fs_source,
                vanish: "gone.txt".to_string(),
            }),
            plan_options: PlanOptions::default(),
            mirror_scope_filter: FileFilter::default(),
            dry_run: false,
            null_sink: false,
            sink_workers: 1,
            unreadable: Arc::clone(&unreadable),
            stats: Arc::new(LocalApplyStats::default()),
            phase_probe: LocalPhaseProbe::disabled(),
            checker_pool: CheckerPool::new(1).expect("checker pool"),
            dir_stats: Arc::default(),
        };
        let source_cfg = SourceSessionConfig {
            hello: HelloConfig::default(),
            endpoint: SessionEndpoint::initiator(open),
            plan_options: PlanOptions::default(),
            data_plane_host: None,
            instruments: SourceInstruments {
                progress: None,
                unreadable: Some(Arc::clone(&unreadable)),
                trace_data_plane: false,
                session_phase_trace: Default::default(),
                lifecycle_trace: Default::default(),
                small_file_probe: SmallFileProbe::disabled(),
                #[cfg(test)]
                dial_test_samples: None,
                #[cfg(test)]
                dial_terminal_test_gate: None,
                #[cfg(test)]
                dial_proposal_test_gate: None,
                #[cfg(test)]
                dial_membership_test_gate: None,
            },
        };
        let dest_cfg = DestinationSessionConfig {
            hello: HelloConfig::default(),
            endpoint: SessionEndpoint::Responder,
            data_plane_host: None,
            receiver_capacity: None,
            instruments: DestinationInstruments {
                small_file_probe: SmallFileProbe::disabled(),
                ..Default::default()
            },
            local_apply: Some(local_apply),
        };
        let (a, b) = in_process_pair();
        let scan_source: Arc<dyn TransferSource> =
            Arc::new(FsTransferSource::new(src_root.clone()));
        let (_, dest_result): (
            eyre::Result<TransferSummary>,
            eyre::Result<DestinationOutcome>,
        ) = tokio::join!(
            run_source(source_cfg, a, scan_source),
            run_destination(dest_cfg, b, DestinationTarget::Fixed(dst_root.clone())),
        );

        let err = dest_result.expect_err("apply-time unreadable must refuse the mirror");
        assert!(
            format!("{err:#}").contains("could not be read during the transfer"),
            "unexpected error: {err:#}"
        );
        assert!(
            dst_root.join("extraneous.txt").exists(),
            "a refused mirror must not have deleted anything"
        );
    }

    /// Wraps a real sink, holds each `write_payload` open for a fixed delay,
    /// and records the PEAK number of simultaneously in-flight calls.
    ///
    /// ls-4 (r10 `ls4-guard`): the only observer that can distinguish "the
    /// pipeline is configured for 8 workers" from "the pipeline ran 8
    /// workers". The first ls-4 guard asserted configuration plus tree
    /// equality, and the reviewer forced the production wiring back to one
    /// worker with every test staying green — a single worker produces the
    /// same correct tree, just slower.
    struct ConcurrencyProbeSink {
        inner: Arc<dyn TransferSink>,
        delay: std::time::Duration,
        in_flight: std::sync::atomic::AtomicUsize,
        peak: std::sync::atomic::AtomicUsize,
    }

    #[async_trait]
    impl TransferSink for ConcurrencyProbeSink {
        async fn write_payload(
            &self,
            payload: crate::remote::transfer::payload::PreparedPayload,
        ) -> eyre::Result<SinkOutcome> {
            use std::sync::atomic::Ordering;
            let now = self.in_flight.fetch_add(1, Ordering::SeqCst) + 1;
            self.peak.fetch_max(now, Ordering::SeqCst);
            // Long enough that overlap is unavoidable when workers exist,
            // and impossible when they do not.
            tokio::time::sleep(self.delay).await;
            let out = self.inner.write_payload(payload).await;
            self.in_flight.fetch_sub(1, Ordering::SeqCst);
            out
        }

        async fn write_file_stream(
            &self,
            header: &FileHeader,
            reader: &mut (dyn tokio::io::AsyncRead + Unpin + Send),
        ) -> eyre::Result<SinkOutcome> {
            self.inner.write_file_stream(header, reader).await
        }

        fn root(&self) -> &Path {
            self.inner.root()
        }
    }

    /// ls-4, the guard that observes EXECUTION: driven through
    /// `run_local_session` — the public entry every CLI copy takes — with an
    /// injected probe sink, this asserts more than one payload was in flight
    /// at once. Forcing the `sink_workers` wiring back to `1` reds it, which
    /// is precisely the mutation the r10 reviewer showed the configuration
    /// assertions could not see.
    #[tokio::test]
    async fn a_normal_session_holds_multiple_payloads_in_flight() {
        use std::sync::atomic::Ordering;

        let tmp = tempfile::tempdir().expect("tempdir");
        let src_root = tmp.path().join("src");
        let dst_root = tmp.path().join("dst");
        std::fs::create_dir_all(&src_root).expect("mkdir src");
        // Each file exceeds the tar-shard threshold so it plans as its own
        // `File` payload; enough of them that overlap is not a coin flip.
        let body = vec![b'p'; 1024 * 1024 + 1];
        for index in 0..12 {
            std::fs::write(src_root.join(format!("big{index}.bin")), &body).expect("write");
        }

        let probe = Arc::new(ConcurrencyProbeSink {
            inner: Arc::new(FsTransferSink::new(
                src_root.clone(),
                dst_root.clone(),
                FsSinkConfig::default(),
            )),
            delay: std::time::Duration::from_millis(40),
            in_flight: std::sync::atomic::AtomicUsize::new(0),
            peak: std::sync::atomic::AtomicUsize::new(0),
        });

        let summary = run_local_session(
            &src_root,
            &dst_root,
            LocalMirrorOptions {
                perf_history: false,
                sink_override: Some(SinkOverride(Arc::clone(&probe) as Arc<dyn TransferSink>)),
                ..Default::default()
            },
        )
        .await
        .expect("session");
        assert_eq!(summary.copied_files, 12, "the probe must not drop work");

        let peak = probe.peak.load(Ordering::SeqCst);
        assert!(
            peak > 1,
            "peak in-flight payloads was {peak}; the apply pipeline executed \
             sequentially, whatever its configuration says"
        );
    }

    /// ls-5 guard: a converged mirror ANSWERS from directory sweeps. The
    /// tree comparison cannot see this — a per-file-stat session produces
    /// the identical tree, just slower — so the test holds the very cache
    /// the session uses (`dir_stat_probe`, the cr-ls1-9 injection lesson)
    /// and asserts every resolution came from it. Reverting the
    /// `resolve_destination_target` wiring back to `std::fs::metadata`
    /// zeroes the hit counter and reds this.
    #[tokio::test]
    async fn a_converged_mirror_answers_from_directory_sweeps() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let src_root = tmp.path().join("src");
        let dst_root = tmp.path().join("dst");
        for dir in ["alpha", "beta", "gamma"] {
            std::fs::create_dir_all(src_root.join(dir)).expect("mkdir");
            for file in 0..3 {
                std::fs::write(src_root.join(dir).join(format!("f{file}.txt")), b"stable")
                    .expect("write");
            }
        }
        run_local_session(&src_root, &dst_root, LocalMirrorOptions::default())
            .await
            .expect("seed session");

        let cache = Arc::new(super::super::dir_stat::DirStatCache::default());
        let summary = run_local_session(
            &src_root,
            &dst_root,
            LocalMirrorOptions {
                perf_history: false,
                dir_stat_probe: Some(Arc::clone(&cache)),
                ..Default::default()
            },
        )
        .await
        .expect("converged session");
        assert_eq!(summary.copied_files, 0, "the tree was already converged");
        assert_eq!(
            cache.fallbacks(),
            0,
            "a converged tree of plain files needs no authoritative per-file stat"
        );
        assert!(cache.sweeps() >= 3, "one sweep per destination directory");
        assert_eq!(
            cache.hits(),
            9,
            "every manifest entry must resolve from the sweep cache; zero hits \
             means the session went back to per-file stats"
        );
    }

    /// ls-5 guard for the trusted-absent arm: a fresh copy into a
    /// destination that does not exist yet must not pay a per-file
    /// fallback storm — the sweep's NotFound answers every child.
    /// Reverting AbsentDir to Unsweepable reds the fallback count.
    #[tokio::test]
    async fn a_fresh_copy_trusts_the_sweeps_absent_answer() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let src_root = tmp.path().join("src");
        let dst_root = tmp.path().join("dst");
        std::fs::create_dir_all(src_root.join("nested")).expect("mkdir");
        for file in 0..4 {
            std::fs::write(src_root.join("nested").join(format!("f{file}.txt")), b"new")
                .expect("write");
        }

        let cache = Arc::new(super::super::dir_stat::DirStatCache::default());
        let summary = run_local_session(
            &src_root,
            &dst_root,
            LocalMirrorOptions {
                perf_history: false,
                dir_stat_probe: Some(Arc::clone(&cache)),
                ..Default::default()
            },
        )
        .await
        .expect("fresh session");
        assert_eq!(summary.copied_files, 4);
        assert_eq!(
            cache.fallbacks(),
            0,
            "an absent destination directory is a trusted absent for every \
             child, never a per-file stat storm"
        );
    }

    /// ls-5 trust boundary: a destination name that differs from the
    /// manifest's only by case must NEVER be judged absent from the sweep
    /// alone. On this case-sensitive filesystem the authoritative fallback
    /// stat misses and the file copies; trusting the sweep's absent would
    /// also copy — so the FALLBACK COUNTER is the assertion that matters,
    /// and the case-insensitive half of the argument (where trusting
    /// absent means a permanent re-copy loop, or worse a false skip on a
    /// case-sensitive SMB backend) rides the same counter.
    #[cfg(unix)]
    #[tokio::test]
    async fn a_case_folded_near_miss_is_never_trusted_as_absent() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let src_root = tmp.path().join("src");
        let dst_root = tmp.path().join("dst");
        std::fs::create_dir_all(&src_root).expect("mkdir src");
        std::fs::create_dir_all(&dst_root).expect("mkdir dst");
        std::fs::write(src_root.join("readme.txt"), b"source body").expect("write src");
        std::fs::write(dst_root.join("README.txt"), b"other file").expect("write dst");

        let cache = Arc::new(super::super::dir_stat::DirStatCache::default());
        let summary = run_local_session(
            &src_root,
            &dst_root,
            LocalMirrorOptions {
                perf_history: false,
                dir_stat_probe: Some(Arc::clone(&cache)),
                ..Default::default()
            },
        )
        .await
        .expect("session");
        assert_eq!(
            summary.copied_files, 1,
            "the lowercase file is genuinely absent"
        );
        assert!(
            cache.fallbacks() >= 1,
            "a folded-name near-miss must consult the authoritative stat, \
             not the sweep's absence"
        );
        assert_eq!(
            std::fs::read(dst_root.join("readme.txt")).expect("copied file"),
            b"source body"
        );
    }

    /// A sink that takes a known, large amount of time per payload. Wraps a
    /// real sink so the transfer still succeeds and the rest of the session
    /// behaves normally — only the writer is slow.
    struct SlowSink {
        inner: Arc<dyn TransferSink>,
        delay: std::time::Duration,
    }

    #[async_trait]
    impl TransferSink for SlowSink {
        async fn write_payload(
            &self,
            payload: crate::remote::transfer::payload::PreparedPayload,
        ) -> eyre::Result<SinkOutcome> {
            tokio::time::sleep(self.delay).await;
            self.inner.write_payload(payload).await
        }

        async fn write_file_stream(
            &self,
            header: &FileHeader,
            reader: &mut (dyn tokio::io::AsyncRead + Unpin + Send),
        ) -> eyre::Result<SinkOutcome> {
            self.inner.write_file_stream(header, reader).await
        }

        fn root(&self) -> &Path {
            self.inner.root()
        }
    }

    /// cr-ls1-1: a sink slower than the planner must show up as
    /// APPLY_BACKPRESSURE, not vanish.
    ///
    /// This is the guard the first cut of ls-1 lacked. With the queue await
    /// untimed, this exact shape — writer slower than reader — put its cost
    /// in no phase at all, which is the misattribution the whole slice
    /// exists to prevent. The payload queue holds `DEFAULT_PAYLOAD_PREFETCH`
    /// entries, so with more files than that and a real per-payload delay,
    /// the diff loop MUST block on the queue.
    #[tokio::test]
    async fn a_slow_sink_is_attributed_to_apply_backpressure() {
        use super::super::phase_probe::LocalPhaseReport;

        let tmp = tempfile::tempdir().expect("tempdir");
        let src_root = tmp.path().join("src");
        let dst_root = tmp.path().join("dst");
        std::fs::create_dir_all(&src_root).expect("mkdir src");
        std::fs::create_dir_all(&dst_root).expect("mkdir dst");
        // Each file is >= 1 MiB so it plans as its own `File` payload rather
        // than being amortised into a tar shard, and there are several times
        // `DEFAULT_PAYLOAD_PREFETCH` of them — otherwise every payload fits
        // in the queue, the diff loop never blocks, and the slow sink shows
        // up as tail drain instead. That was the original defect's blind
        // spot, so the fixture has to be past the queue depth to test it.
        const FILES: usize = DEFAULT_PAYLOAD_PREFETCH * 4;
        let body = vec![b'z'; 1024 * 1024 + 1];
        for index in 0..FILES {
            std::fs::write(src_root.join(format!("big{index}.bin")), &body).expect("write");
        }

        let delay = std::time::Duration::from_millis(40);
        let reports: Arc<StdMutex<Vec<LocalPhaseReport>>> = Arc::default();
        let seen = Arc::clone(&reports);
        let probe = LocalPhaseProbe::capture("cr-ls1-1", move |report| {
            seen.lock().expect("sink poisoned").push(report);
        });

        let summary = run_local_session(
            &src_root,
            &dst_root,
            LocalMirrorOptions {
                perf_history: false,
                phase_probe: probe.clone(),
                ..Default::default()
            },
        )
        .await
        .expect("session");
        assert_eq!(summary.copied_files, FILES);

        // The session above uses the real sink; the point of the fixture is
        // the ASSERTION SHAPE, so re-run the same tree through a slow sink
        // at the LocalApply seam where a custom sink can be injected.
        let dst_slow = tmp.path().join("dst-slow");
        std::fs::create_dir_all(&dst_slow).expect("mkdir dst-slow");
        let unreadable: Arc<StdMutex<Vec<String>>> = Arc::default();
        let real: Arc<dyn TransferSink> = Arc::new(FsTransferSink::new(
            src_root.clone(),
            dst_slow.clone(),
            FsSinkConfig::default(),
        ));
        let slow_reports: Arc<StdMutex<Vec<LocalPhaseReport>>> = Arc::default();
        let slow_seen = Arc::clone(&slow_reports);
        let slow_probe = LocalPhaseProbe::capture("cr-ls1-1-slow", move |report| {
            slow_seen.lock().expect("sink poisoned").push(report);
        });
        let local_apply = LocalApply {
            src_root: src_root.clone(),
            sink: Arc::new(SlowSink { inner: real, delay }),
            prepare_source: Arc::new(FsTransferSource::new(src_root.clone())),
            plan_options: PlanOptions::default(),
            mirror_scope_filter: FileFilter::default(),
            dry_run: false,
            null_sink: false,
            sink_workers: 1,
            unreadable: Arc::clone(&unreadable),
            stats: Arc::new(LocalApplyStats::default()),
            phase_probe: slow_probe.clone(),
            checker_pool: CheckerPool::new(1).expect("checker pool"),
            dir_stats: Arc::default(),
        };
        let open = SessionOpen {
            initiator_role: TransferRole::Source as i32,
            compare_mode: ComparisonMode::SizeMtime as i32,
            in_stream_bytes: true,
            ..Default::default()
        };
        let source_cfg = SourceSessionConfig {
            hello: HelloConfig::default(),
            endpoint: SessionEndpoint::initiator(open),
            plan_options: PlanOptions::default(),
            data_plane_host: None,
            instruments: SourceInstruments {
                progress: None,
                unreadable: Some(Arc::clone(&unreadable)),
                trace_data_plane: false,
                session_phase_trace: Default::default(),
                lifecycle_trace: Default::default(),
                small_file_probe: SmallFileProbe::disabled(),
                #[cfg(test)]
                dial_test_samples: None,
                #[cfg(test)]
                dial_terminal_test_gate: None,
                #[cfg(test)]
                dial_proposal_test_gate: None,
                #[cfg(test)]
                dial_membership_test_gate: None,
            },
        };
        let dest_cfg = DestinationSessionConfig {
            hello: HelloConfig::default(),
            endpoint: SessionEndpoint::Responder,
            data_plane_host: None,
            receiver_capacity: None,
            instruments: DestinationInstruments {
                small_file_probe: SmallFileProbe::disabled(),
                ..Default::default()
            },
            local_apply: Some(local_apply),
        };
        let (a, b) = in_process_pair();
        let scan_source: Arc<dyn TransferSource> =
            Arc::new(FsTransferSource::new(src_root.clone()).with_phase_probe(slow_probe.clone()));
        let (_, dest_result): (
            eyre::Result<TransferSummary>,
            eyre::Result<DestinationOutcome>,
        ) = tokio::join!(
            run_source(source_cfg, a, scan_source),
            run_destination(dest_cfg, b, DestinationTarget::Fixed(dst_slow.clone())),
        );
        dest_result.expect("slow-sink session still succeeds");
        slow_probe.emit(std::time::Duration::from_secs(1), false);

        let captured = slow_reports.lock().expect("sink poisoned");
        let report = captured.first().expect("one report");
        let phase = |wanted: LocalPhase| {
            report
                .phases
                .iter()
                .find(|(candidate, _)| *candidate == wanted)
                .map(|(_, aggregate)| aggregate.clone())
                .expect("every phase is reported")
        };

        let backpressure = phase(LocalPhase::ApplyBackpressure);
        assert!(
            backpressure.samples > 0,
            "every queue push is timed — this reads 0 if the span is removed"
        );
        // The binding half. With FILES payloads at `delay` each, one sink
        // worker, and a queue only DEFAULT_PAYLOAD_PREFETCH deep, the diff
        // loop must block for roughly (FILES - depth) * delay. Assert a
        // conservative fraction of that so the guard is decisive without
        // being timing-flaky: anything at or below one delay means the queue
        // never actually backed up and the fixture stopped testing the
        // defect.
        let blocked_pushes = (FILES - DEFAULT_PAYLOAD_PREFETCH) as u64;
        let expected_floor = delay.as_nanos() as u64 * blocked_pushes / 2;
        assert!(
            backpressure.total_ns >= expected_floor,
            "a slow sink must cost measurable APPLY_BACKPRESSURE: got {} ns \
             across {} samples, expected at least {} ns",
            backpressure.total_ns,
            backpressure.samples,
            expected_floor
        );
    }

    /// A source tree with one file whose destination position is blocked
    /// by a directory — the portable way to fail exactly one file's write.
    /// Both files are sized past the planner's 1 MiB small-file cut so each
    /// is planned as its own single-file payload: tar-shard members are a
    /// separate containment slice, and a shard here would prove nothing
    /// about the single-file write paths.
    fn one_blocked_file_fixture(tmp: &Path) -> (PathBuf, PathBuf) {
        let src_root = tmp.join("src");
        let dst_root = tmp.join("dst");
        std::fs::create_dir_all(&src_root).expect("mkdir src");
        std::fs::create_dir_all(&dst_root).expect("mkdir dst");
        let body = vec![b'x'; 1_048_576 + 1];
        std::fs::write(src_root.join("ok.bin"), &body).expect("write");
        std::fs::write(src_root.join("blocked.bin"), &body).expect("write");
        std::fs::create_dir_all(dst_root.join("blocked.bin")).expect("mkdir blocker");
        (src_root, dst_root)
    }

    /// pfc-5, replacing pfc-2's interim `!mirror_enabled` refusal (which
    /// this test previously pinned): containment now applies to NON-mirror
    /// sessions too — audit-17's closure shape, a plain `copy` surviving one
    /// file the destination filesystem rejects. Q1(b) moved to the caller's
    /// source-delete gate, which reads the `files_failed` this summary
    /// carries; the session no longer guesses whether a delete follows.
    #[tokio::test]
    async fn non_mirror_session_contains_a_per_file_failure_and_reports_it() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let (src_root, dst_root) = one_blocked_file_fixture(tmp.path());

        let summary = run_local_session(
            &src_root,
            &dst_root,
            LocalMirrorOptions {
                mirror: false,
                perf_history: false,
                ..LocalMirrorOptions::default()
            },
        )
        .await
        .expect("one file's write failure must not fault a non-mirror session");

        assert_eq!(summary.files_failed, 1);
        assert_eq!(summary.failures.len(), 1);
        assert_eq!(summary.failures[0].relative_path, "blocked.bin");
        assert_eq!(
            summary.copied_files, 1,
            "the failed file is never a copied file"
        );
        assert_eq!(
            std::fs::metadata(dst_root.join("ok.bin"))
                .expect("the rest of the manifest landed")
                .len(),
            1_048_577
        );
    }

    /// The same failure in a mirror is contained the same way, and the
    /// delete phase still runs (Q1(a)): the session completes and the rest
    /// of the manifest lands.
    #[tokio::test]
    async fn mirror_contains_a_per_file_failure_and_transfers_the_rest() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let (src_root, dst_root) = one_blocked_file_fixture(tmp.path());

        let summary = run_local_session(
            &src_root,
            &dst_root,
            LocalMirrorOptions {
                mirror: true,
                perf_history: false,
                ..LocalMirrorOptions::default()
            },
        )
        .await
        .expect("one file's write failure must not fault a mirror");

        assert_eq!(
            summary.copied_files, 1,
            "the failed file is never a copied file"
        );
        assert_eq!(
            std::fs::metadata(dst_root.join("ok.bin"))
                .expect("the rest of the manifest landed")
                .len(),
            1_048_577
        );
    }

    /// clp-1 wiring: a local session with a progress sink attached
    /// delivers BOTH roles' events onto the caller's one channel — the
    /// destination diff's transfer denominator and the apply pipeline's
    /// per-file completions and bytes. Without the wiring the session
    /// runs with `progress: None` and the caller's row has nothing to
    /// render.
    #[tokio::test]
    async fn progress_sink_receives_both_roles_events() {
        use crate::remote::transfer::{ProgressEvent, ProgressTotals};

        let tmp = tempfile::tempdir().expect("tempdir");
        let src_root = tmp.path().join("src");
        let dst_root = tmp.path().join("dst");
        std::fs::create_dir_all(&src_root).expect("mkdir src");
        std::fs::write(src_root.join("a.txt"), b"first").expect("write");
        std::fs::write(src_root.join("b.txt"), b"second").expect("write");

        let (tx, mut rx) = mpsc::unbounded_channel::<ProgressEvent>();
        let summary = run_local_session(
            &src_root,
            &dst_root,
            LocalMirrorOptions {
                progress: true,
                progress_events: Some(RemoteTransferProgress::new(tx)),
                perf_history: false,
                ..LocalMirrorOptions::default()
            },
        )
        .await
        .expect("local session with progress attached");
        assert_eq!(summary.copied_files, 2);

        let mut totals = ProgressTotals::default();
        let mut completed: Vec<String> = Vec::new();
        while let Ok(event) = rx.try_recv() {
            if let ProgressEvent::FileComplete { path } = &event {
                completed.push(path.clone());
            }
            totals.apply(&event);
        }
        assert_eq!(
            totals.manifest_files, 2,
            "the destination diff reports the files it needs"
        );
        assert_eq!(
            totals.enumerated_files, 2,
            "the source-side enumeration heartbeat reports through the same sink"
        );
        completed.sort();
        assert_eq!(
            completed,
            vec!["a.txt".to_string(), "b.txt".to_string()],
            "the apply pipeline reports each finished file"
        );
        assert_eq!(totals.files, 2);
        assert_eq!(totals.bytes, 11, "bytes ride the Payload lane only");
    }

    /// clp-2: the two phase facts the counters cannot express ride the
    /// same one lane. The diff finishing is what tells an up-to-date
    /// tree from a scan still running (both sit at zero needed files),
    /// and the delete pass is what tells a mirror's purge from the copy
    /// that preceded it. The purge signal is necessarily last: the pass
    /// runs only after every apply has joined.
    #[tokio::test]
    async fn progress_sink_receives_the_phase_signals() {
        use crate::remote::transfer::ProgressEvent;

        let tmp = tempfile::tempdir().expect("tempdir");
        let src_root = tmp.path().join("src");
        let dst_root = tmp.path().join("dst");
        std::fs::create_dir_all(&src_root).expect("mkdir src");
        std::fs::create_dir_all(&dst_root).expect("mkdir dst");
        std::fs::write(src_root.join("a.txt"), b"first").expect("write");
        std::fs::write(dst_root.join("extraneous.txt"), b"stale").expect("write");

        let (tx, mut rx) = mpsc::unbounded_channel::<ProgressEvent>();
        let summary = run_local_session(
            &src_root,
            &dst_root,
            LocalMirrorOptions {
                mirror: true,
                progress: true,
                progress_events: Some(RemoteTransferProgress::new(tx)),
                perf_history: false,
                ..LocalMirrorOptions::default()
            },
        )
        .await
        .expect("local mirror with progress attached");
        assert_eq!(summary.deleted_files, 1, "the extraneous file is purged");

        let mut diff_complete = 0usize;
        let mut delete_begin = 0usize;
        let mut last: Option<ProgressEvent> = None;
        while let Ok(event) = rx.try_recv() {
            match &event {
                ProgressEvent::DiffComplete => diff_complete += 1,
                ProgressEvent::DeleteBegin => delete_begin += 1,
                _ => {}
            }
            last = Some(event);
        }
        assert_eq!(
            diff_complete, 1,
            "the destination reports its diff finishing exactly once"
        );
        assert_eq!(
            delete_begin, 1,
            "the mirror-delete pass announces itself exactly once"
        );
        assert!(
            matches!(last, Some(ProgressEvent::DeleteBegin)),
            "the purge is the last thing the lane reports, not a stale copy \
             event: {last:?}"
        );
    }

    /// A dry run plans the delete pass without removing anything, so the
    /// row must never claim it is deleting.
    #[tokio::test]
    async fn dry_run_mirror_does_not_announce_deletion() {
        use crate::remote::transfer::ProgressEvent;

        let tmp = tempfile::tempdir().expect("tempdir");
        let src_root = tmp.path().join("src");
        let dst_root = tmp.path().join("dst");
        std::fs::create_dir_all(&src_root).expect("mkdir src");
        std::fs::create_dir_all(&dst_root).expect("mkdir dst");
        std::fs::write(src_root.join("a.txt"), b"first").expect("write");
        std::fs::write(dst_root.join("extraneous.txt"), b"stale").expect("write");

        let (tx, mut rx) = mpsc::unbounded_channel::<ProgressEvent>();
        run_local_session(
            &src_root,
            &dst_root,
            LocalMirrorOptions {
                mirror: true,
                dry_run: true,
                progress: true,
                progress_events: Some(RemoteTransferProgress::new(tx)),
                perf_history: false,
                ..LocalMirrorOptions::default()
            },
        )
        .await
        .expect("dry-run mirror with progress attached");
        assert!(
            dst_root.join("extraneous.txt").exists(),
            "a dry run deletes nothing"
        );

        let mut delete_begin = 0usize;
        while let Ok(event) = rx.try_recv() {
            if matches!(event, ProgressEvent::DeleteBegin) {
                delete_begin += 1;
            }
        }
        assert_eq!(
            delete_begin, 0,
            "a dry run must not announce a deletion it will not perform"
        );
    }

    #[test]
    fn dest_subtree_rel_detects_nesting() {
        assert_eq!(
            dest_subtree_rel(Path::new("/a/src"), Path::new("/a/src/nested/dst")),
            Some("nested/dst".to_string())
        );
        assert_eq!(
            dest_subtree_rel(Path::new("/a/src"), Path::new("/a/dst")),
            None
        );
        // dst == src is not a nested-subtree shape (strip yields empty).
        assert_eq!(
            dest_subtree_rel(Path::new("/a/src"), Path::new("/a/src")),
            None
        );
    }

    /// R44-F1 carried forward: the record's `(file_count, total_bytes)`
    /// are the SCANNED features, not the copied counts.
    #[test]
    fn local_record_uses_scanned_features_not_copied() {
        let summary = LocalMirrorSummary {
            scanned_files: 1000,
            scanned_bytes: 10 * 1024 * 1024,
            planned_files: 5,
            copied_files: 5,
            total_bytes: 100 * 1024,
            duration: std::time::Duration::from_millis(200),
            ..LocalMirrorSummary::default()
        };
        let record = build_local_record(&summary, &LocalMirrorOptions::default());
        assert_eq!(record.file_count, 1000);
        assert_eq!(record.total_bytes, summary.scanned_bytes);
        assert_eq!(record.transfer_duration_ms, 200);
        assert_eq!(record.fast_path.as_deref(), Some("session"));
    }

    /// Bucket-shape fields still reflect actual apply activity.
    #[test]
    fn local_record_carries_bucket_counters() {
        let summary = LocalMirrorSummary {
            scanned_files: 100,
            scanned_bytes: 1_000_000,
            tar_shard_tasks: 2,
            tar_shard_files: 7,
            tar_shard_bytes: 30_000,
            large_tasks: 1,
            large_bytes: 5_000,
            ..LocalMirrorSummary::default()
        };
        let record = build_local_record(&summary, &LocalMirrorOptions::default());
        assert_eq!(record.tar_shard_tasks, 2);
        assert_eq!(record.tar_shard_files, 7);
        assert_eq!(record.tar_shard_bytes, 30_000);
        assert_eq!(record.large_tasks, 1);
        assert_eq!(record.large_bytes, 5_000);
    }

    /// codex otp-11a F9: `--null` runs keep the `null_sink` tag so
    /// RunKind derivation classifies them as diagnostics, and dry-run
    /// records carry `dry_run` for the same lane split.
    #[test]
    fn local_record_null_and_dry_run_lanes() {
        use crate::perf_history::RunKind;
        let summary = LocalMirrorSummary::default();
        let null = build_local_record(
            &summary,
            &LocalMirrorOptions {
                null_sink: true,
                ..LocalMirrorOptions::default()
            },
        );
        assert_eq!(null.fast_path.as_deref(), Some("null_sink"));
        assert_eq!(null.run_kind, RunKind::NullSink);
        let dry = build_local_record(
            &summary,
            &LocalMirrorOptions {
                dry_run: true,
                ..LocalMirrorOptions::default()
            },
        );
        assert_eq!(dry.run_kind, RunKind::DryRun);
    }

    /// ue-r2-1c single-home mapping: every `LocalCompareMode` variant
    /// resolves onto its wire `ComparisonMode`, and the legacy
    /// `--checksum` bool upgrades the SizeMtime default only.
    #[test]
    fn compare_mode_resolves_onto_wire_enum() {
        assert_eq!(
            LocalCompareMode::SizeMtime.resolve_comparison_mode(false),
            ComparisonMode::SizeMtime
        );
        assert_eq!(
            LocalCompareMode::SizeMtime.resolve_comparison_mode(true),
            ComparisonMode::Checksum
        );
        assert_eq!(
            LocalCompareMode::Checksum.resolve_comparison_mode(false),
            ComparisonMode::Checksum
        );
        assert_eq!(
            LocalCompareMode::SizeOnly.resolve_comparison_mode(true),
            ComparisonMode::SizeOnly,
            "legacy checksum must not override an explicit non-default mode"
        );
        assert_eq!(
            LocalCompareMode::Force.resolve_comparison_mode(false),
            ComparisonMode::Force
        );
        assert_eq!(
            LocalCompareMode::IgnoreTimes.resolve_comparison_mode(false),
            ComparisonMode::IgnoreTimes
        );
    }

    /// The perf-history snapshot mapping mirrors the wire mapping
    /// (tuning buckets key on the full comparison policy, R59 #5).
    #[test]
    fn compare_mode_resolves_onto_snapshot_enum() {
        use crate::perf_history::CompareModeSnapshot;
        assert_eq!(
            LocalCompareMode::SizeMtime.resolve_compare_snapshot(true),
            CompareModeSnapshot::Checksum
        );
        assert_eq!(
            LocalCompareMode::IgnoreTimes.resolve_compare_snapshot(false),
            CompareModeSnapshot::IgnoreTimes
        );
        assert_eq!(
            LocalCompareMode::SizeMtime.resolve_compare_snapshot(false),
            CompareModeSnapshot::SizeMtime
        );
    }

    /// The dest-subtree exclusion wrapper forwards everything outside
    /// the excluded prefix and drops everything under it (the manifest
    /// the destination diff sees never contains the destination).
    #[tokio::test]
    async fn dest_subtree_excluded_source_filters_the_stream() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let src_root = tmp.path().join("src");
        std::fs::create_dir_all(src_root.join("backup")).expect("mkdir");
        std::fs::write(src_root.join("a.txt"), b"keep").expect("write");
        std::fs::write(src_root.join("b.txt"), b"keep").expect("write");
        std::fs::write(src_root.join("backup/old.txt"), b"drop").expect("write");

        let wrapper = DestSubtreeExcludedSource {
            inner: Arc::new(FsTransferSource::new(src_root.clone())),
            exclude_rel: "backup".to_string(),
        };
        let (mut rx, mut scan) = wrapper.scan(None, Arc::default());
        let mut forwarded = Vec::new();
        while let Some(h) = rx.recv().await {
            forwarded.push(h.relative_path);
        }
        forwarded.sort();
        assert_eq!(forwarded, vec!["a.txt".to_string(), "b.txt".to_string()]);
        let count = scan.finish().await.expect("scan");
        assert_eq!(count, 2, "the forwarded count excludes the subtree");
    }

    /// The streaming-overlap property, ported from the engine's
    /// `first_work_lands_before_enumeration_completes`: with more than
    /// one diff chunk of files, the first destination writes land
    /// while the source scan is still running. A gating source holds
    /// the manifest stream open after `DEST_DIFF_CHUNK` + a few
    /// entries until the test observes a file at the destination.
    #[tokio::test]
    async fn first_apply_lands_before_enumeration_completes() {
        use tokio::sync::oneshot;

        struct GatedSource {
            inner: Arc<dyn TransferSource>,
            gate: StdMutex<Option<oneshot::Receiver<()>>>,
        }

        #[async_trait]
        impl TransferSource for GatedSource {
            fn scan(
                &self,
                filter: Option<FileFilter>,
                unreadable_paths: Arc<StdMutex<Vec<String>>>,
            ) -> (mpsc::Receiver<FileHeader>, SourceScan) {
                let (mut inner_rx, mut scan) = self.inner.scan(filter, unreadable_paths);
                let (tx, rx) = mpsc::channel(8);
                let gate = self
                    .gate
                    .lock()
                    .expect("gate lock")
                    .take()
                    .expect("scan called once");
                let handle = tokio::spawn(async move {
                    let mut forwarded = 0u64;
                    let mut gate = Some(gate);
                    while let Some(h) = inner_rx.recv().await {
                        forwarded += 1;
                        if tx.send(h).await.is_err() {
                            break;
                        }
                        // Hold the manifest open once a full diff chunk
                        // (plus slack) is out, until the gate fires.
                        if forwarded == 160 {
                            if let Some(g) = gate.take() {
                                let _ = g.await;
                            }
                        }
                    }
                    Ok(forwarded)
                });
                scan.replace_primary(handle);
                (rx, scan)
            }

            async fn prepare_payload(
                &self,
                payload: TransferPayload,
            ) -> eyre::Result<crate::remote::transfer::payload::PreparedPayload> {
                self.inner.prepare_payload(payload).await
            }

            async fn check_availability(
                &self,
                headers: Vec<FileHeader>,
                unreadable_paths: Arc<StdMutex<Vec<String>>>,
            ) -> eyre::Result<Vec<FileHeader>> {
                self.inner
                    .check_availability(headers, unreadable_paths)
                    .await
            }

            async fn open_file(
                &self,
                header: &FileHeader,
            ) -> eyre::Result<Box<dyn tokio::io::AsyncRead + Unpin + Send>> {
                self.inner.open_file(header).await
            }

            fn root(&self) -> &Path {
                self.inner.root()
            }
        }

        let tmp = tempfile::tempdir().expect("tempdir");
        let src_root = tmp.path().join("src");
        let dst_root = tmp.path().join("dst");
        std::fs::create_dir_all(&src_root).expect("mkdir");
        std::fs::create_dir_all(&dst_root).expect("mkdir");
        for i in 0..200 {
            std::fs::write(src_root.join(format!("f{i:03}.txt")), b"payload").expect("write");
        }

        let (gate_tx, gate_rx) = oneshot::channel();
        // Watcher: fire the gate as soon as ANY file lands at the dest —
        // proof that apply work started before the scan completed.
        let dst_watch = dst_root.clone();
        let watcher = tokio::spawn(async move {
            for _ in 0..1000 {
                let landed = std::fs::read_dir(&dst_watch)
                    .map(|d| d.count())
                    .unwrap_or(0);
                if landed > 0 {
                    let _ = gate_tx.send(());
                    return true;
                }
                tokio::time::sleep(std::time::Duration::from_millis(10)).await;
            }
            false
        });

        let scan_source: Arc<dyn TransferSource> = Arc::new(GatedSource {
            inner: Arc::new(FsTransferSource::new(src_root.clone())),
            gate: StdMutex::new(Some(gate_rx)),
        });
        let sink: Arc<dyn TransferSink> = Arc::new(FsTransferSink::new(
            src_root.clone(),
            dst_root.clone(),
            FsSinkConfig::default(),
        ));
        let local_apply = LocalApply {
            src_root: src_root.clone(),
            sink,
            prepare_source: Arc::new(FsTransferSource::new(src_root.clone())),
            plan_options: PlanOptions::default(),
            mirror_scope_filter: FileFilter::default(),
            dry_run: false,
            null_sink: false,
            sink_workers: 1,
            unreadable: Arc::default(),
            stats: Arc::new(LocalApplyStats::default()),
            phase_probe: LocalPhaseProbe::disabled(),
            checker_pool: CheckerPool::new(1).expect("checker pool"),
            dir_stats: Arc::default(),
        };
        let open = SessionOpen {
            initiator_role: TransferRole::Source as i32,
            compare_mode: ComparisonMode::SizeMtime as i32,
            in_stream_bytes: true,
            ..Default::default()
        };
        let source_cfg = SourceSessionConfig {
            hello: HelloConfig::default(),
            endpoint: SessionEndpoint::initiator(open),
            plan_options: PlanOptions::default(),
            data_plane_host: None,
            instruments: SourceInstruments::default(),
        };
        let dest_cfg = DestinationSessionConfig {
            hello: HelloConfig::default(),
            endpoint: SessionEndpoint::Responder,
            data_plane_host: None,
            receiver_capacity: None,
            instruments: DestinationInstruments::default(),
            local_apply: Some(local_apply),
        };
        let (a, b) = in_process_pair();
        let (source_result, dest_result) =
            tokio::time::timeout(std::time::Duration::from_secs(30), async {
                tokio::join!(
                    run_source(source_cfg, a, scan_source),
                    run_destination(dest_cfg, b, DestinationTarget::Fixed(dst_root.clone())),
                )
            })
            .await
            .expect("session timed out — apply never overlapped the gated scan");
        source_result.expect("source");
        let outcome = dest_result.expect("destination");
        assert_eq!(outcome.summary.files_transferred, 200);
        assert!(
            watcher.await.expect("watcher"),
            "a destination write must land before enumeration completes"
        );
    }
}
