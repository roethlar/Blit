use crate::fs_enum::FileFilter;

/// Scope of mirror deletions. Matches the wire-side `MirrorMode` enum
/// (FilteredSubset / All) plus a `false`/`true` flag form. R58-F6:
/// pre-fix, local mirror had no plumbing for this — `apply_mirror_deletions`
/// always operated on whatever the transfer filter let through. The
/// remote pull path already supports both modes via
/// `PullSyncOptions.delete_all_scope`; this brings local up to parity.
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
/// for the pull / remote-remote-direct paths so local copy/mirror
/// behaves the same as a same-options remote run.
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

/// Options for executing a local mirror/copy operation.
#[derive(Clone, Debug)]
pub struct LocalMirrorOptions {
    pub filter: FileFilter,
    pub mirror: bool,
    pub dry_run: bool,
    pub progress: bool,
    pub verbose: bool,
    pub perf_history: bool,
    pub force_tar: bool,
    pub preserve_symlinks: bool,
    pub include_symlinks: bool,
    pub skip_unchanged: bool,
    /// Skip any file the destination already has, regardless of
    /// comparison mode. Orthogonal to `checksum`/`skip_unchanged`;
    /// matches the `ignore_existing` field on `TransferOperationSpec`
    /// for full pipeline parity across local/push/pull paths.
    pub ignore_existing: bool,
    pub checksum: bool,
    /// R58-F7: comparison policy. The orchestrator picks
    /// `compare_mode` based on this rather than just the `checksum`
    /// bool, so `--size-only` / `--ignore-times` / `--force` get
    /// honored on local copy/mirror the same way the pull path
    /// honors them.
    pub compare_mode: LocalCompareMode,
    /// R58-F6: delete-scope policy for mirror. Only consulted when
    /// `mirror == true`. Defaults to FilteredSubset so a
    /// `mirror --exclude '*.log'` doesn't delete the destination's
    /// `*.log` files just because they were out of scope for the
    /// source filter.
    pub delete_scope: LocalMirrorDeleteScope,
    pub workers: usize,
    pub preserve_times: bool,
    pub debug_mode: bool,
    /// Resume interrupted transfers using block-level comparison.
    pub resume: bool,
    /// Discard writes (NullSink). Measures source read + pipeline throughput.
    pub null_sink: bool,
}

impl Default for LocalMirrorOptions {
    fn default() -> Self {
        Self {
            filter: FileFilter::default(),
            mirror: false,
            dry_run: false,
            progress: false,
            verbose: false,
            perf_history: true,
            force_tar: false,
            preserve_symlinks: true,
            include_symlinks: true,
            skip_unchanged: true,
            ignore_existing: false,
            checksum: false,
            compare_mode: LocalCompareMode::default(),
            delete_scope: LocalMirrorDeleteScope::default(),
            workers: num_cpus::get().max(1),
            preserve_times: true,
            debug_mode: false,
            resume: false,
            null_sink: false,
        }
    }
}
