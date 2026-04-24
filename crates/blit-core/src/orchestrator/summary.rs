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
}
