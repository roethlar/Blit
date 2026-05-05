use std::time::Duration;

/// Why a transfer copied zero files (or fewer than enumerated).
///
/// The orchestrator has multiple paths that can legitimately return a
/// "zero bytes moved" summary. Distinguishing them lets the CLI print
/// an honest message — specifically, catching the pathological case
/// where the source yielded no entries at all (a silent-noop bug class
/// that previously looked identical to "up to date").
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub enum TransferOutcome {
    /// Normal case: some work was attempted (files examined, possibly copied).
    #[default]
    Transferred,
    /// Filesystem journal reported no changes since the last run on both
    /// source and destination — fast-path skip.
    JournalSkip,
    /// Fast-path examined the source and found it up to date with the dest.
    UpToDate,
    /// Fast-path examined the source and it contained zero files.
    ///
    /// Usually a legitimate case (empty source dir), but under the old
    /// single-file bug this also fired when the enumerator silently
    /// skipped a file-root — hence worth distinguishing.
    SourceEmpty,
}

/// Adaptive predictor's pre-run estimate for the just-completed
/// transfer. Surfaced in `--verbose` output so operators can audit
/// the predictor against actual durations, and in `blit profile
/// --json` for programmatic consumption. `None` when the predictor
/// had no profile (or no profile with enough observations) for the
/// workload's `(mode, src_fs, dst_fs, fast_path, skip_unchanged,
/// checksum)` shape — see `MIN_OBSERVATIONS_FOR_CONFIDENCE` in
/// `perf_predictor.rs`. §2.8 of `RELEASE_PLAN_v2_2026-05-04.md`.
#[derive(Clone, Debug, Default)]
pub struct PredictorEstimate {
    pub planner_ms: u128,
    pub transfer_ms: u128,
    pub total_ms: u128,
    pub observations: u64,
    /// 0 = exact key match, increasing with each fallback step
    /// (drop fast_path, drop dest_fs, drop src_fs). Higher depths
    /// are still surfaced but tagged so consumers can discount.
    pub fallback_depth: usize,
}

/// Summary of a local transfer execution.
#[derive(Clone, Debug, Default)]
pub struct LocalMirrorSummary {
    pub planned_files: usize,
    pub copied_files: usize,
    pub total_bytes: u64,
    pub deleted_files: usize,
    pub deleted_dirs: usize,
    pub dry_run: bool,
    pub duration: Duration,
    pub tar_shard_tasks: usize,
    pub tar_shard_files: usize,
    pub tar_shard_bytes: u64,
    pub raw_bundle_tasks: usize,
    pub raw_bundle_files: usize,
    pub raw_bundle_bytes: u64,
    pub large_tasks: usize,
    pub large_bytes: u64,
    /// Classifier for the CLI summary line — lets the user tell apart
    /// "journal fast-path skip", "planner found nothing to do", "source
    /// was empty", and normal transfers.
    pub outcome: TransferOutcome,
    /// What the predictor estimated for this run before it executed.
    /// Surfaced in `--verbose` next to actual durations so operators
    /// can sanity-check the model. Populated only for runs that hit
    /// the streaming planner; fast-path runs (Tiny/Huge/NoWork)
    /// leave it None — those bypass the planner entirely so a
    /// prediction wouldn't be informative.
    pub predictor_estimate: Option<PredictorEstimate>,
}
