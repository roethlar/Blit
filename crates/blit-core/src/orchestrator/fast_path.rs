use std::path::{Path, PathBuf};

use eyre::Result;

use crate::enumeration::{EntryKind, FileEnumerator};
use crate::fs_enum::{CopyJob, FileEntry};
use crate::mirror_planner::MirrorPlanner;

use super::LocalMirrorOptions;

pub(super) const TINY_FILE_LIMIT: usize = 256;
pub(super) const TINY_TOTAL_BYTES: u64 = 256 * 1024 * 1024;
pub(super) const HUGE_SINGLE_BYTES: u64 = 1024 * 1024 * 1024;

#[derive(Clone, Debug)]
pub(super) enum FastPathDecision {
    /// Enumeration produced zero file entries to consider for copying.
    /// `examined` distinguishes "source was empty / had no enumerable
    /// files" (examined=0) from "source had N files but all already
    /// matched the destination under skip_unchanged" (examined>0).
    NoWork {
        examined: usize,
    },
    Tiny {
        files: Vec<(PathBuf, u64)>,
    },
    Huge {
        file: PathBuf,
        size: u64,
    },
}

#[derive(Clone, Debug, Default)]
pub(super) struct FastPathOutcome {
    pub(super) decision: Option<FastPathDecision>,
    /// R47-F4: suppressed walkdir errors observed during the
    /// fast-path scan. Propagated into `LocalMirrorSummary.
    /// unreadable_paths` so the CLI's source-delete step (move)
    /// can refuse to remove a source it couldn't fully scan.
    /// Empty on a clean walk.
    pub(super) unreadable_paths: Vec<String>,
}

impl FastPathOutcome {
    pub(super) fn fast_path(decision: FastPathDecision) -> Self {
        Self {
            decision: Some(decision),
            unreadable_paths: Vec::new(),
        }
    }

    pub(super) fn streaming() -> Self {
        Self {
            decision: None,
            unreadable_paths: Vec::new(),
        }
    }

    pub(super) fn with_unreadable(mut self, paths: Vec<String>) -> Self {
        self.unreadable_paths = paths;
        self
    }
}

#[derive(Debug)]
struct FastPathAbort;

impl std::fmt::Display for FastPathAbort {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "fast-path aborted")
    }
}

impl std::error::Error for FastPathAbort {}

pub(super) fn maybe_select_fast_path(
    src_root: &Path,
    dest_root: &Path,
    options: &LocalMirrorOptions,
) -> Result<FastPathOutcome> {
    if options.mirror || options.checksum || options.force_tar {
        return Ok(FastPathOutcome::streaming());
    }
    // R58-F7: the fast-path's tiny/huge planners route through
    // MirrorPlanner::should_copy_entry, which only understands
    // SizeMtime (and Checksum via the checksum bool). SizeOnly /
    // Force / IgnoreTimes silently became SizeMtime here, so a
    // tiny-manifest copy with --size-only would still re-copy when
    // mtimes differed but sizes matched. Route through the
    // streaming planner, which honors all five ComparisonMode
    // variants via plan_local_mirror.
    if !matches!(options.compare_mode, super::LocalCompareMode::SizeMtime) {
        return Ok(FastPathOutcome::streaming());
    }

    let mut enumerator = FileEnumerator::new(options.filter.clone_without_cache());
    if !options.preserve_symlinks {
        enumerator = enumerator.follow_symlinks(true);
    }
    if options.include_symlinks {
        enumerator = enumerator.include_symlinks(true);
    }

    let planner = MirrorPlanner::new(options.checksum);
    let mut files: Vec<(PathBuf, u64)> = Vec::new();
    let mut total_bytes: u64 = 0;
    let mut aborted = false;
    let mut huge_candidate: Option<(PathBuf, u64)> = None;
    let mut examined: usize = 0;

    // R47-F4: capture suppressed walk errors so the fast-path
    // summary can carry them into `summary.unreadable_paths` —
    // otherwise a move with an unreadable source subdir would
    // route through Tiny (or NoWork on an incremental run),
    // produce a summary with empty unreadable_paths, and the
    // CLI's source-delete step would proceed without seeing the
    // partial-scan signal.
    let scan_result = enumerator.enumerate_local_streaming_capturing(src_root, |entry| {
        if let EntryKind::File { size } = entry.kind {
            examined += 1;
            let should_copy = if options.skip_unchanged {
                let job = CopyJob {
                    entry: FileEntry {
                        path: entry.absolute_path.clone(),
                        size,
                        is_directory: false,
                    },
                };
                planner.should_copy_entry(&job, src_root, dest_root)
            } else {
                true
            };

            if should_copy {
                if files.is_empty() {
                    huge_candidate = Some((entry.relative_path.clone(), size));
                } else {
                    huge_candidate = None;
                }

                files.push((entry.relative_path.clone(), size));
                total_bytes += size;

                if files.len() > TINY_FILE_LIMIT {
                    aborted = true;
                    return Err(FastPathAbort.into());
                }

                if total_bytes > TINY_TOTAL_BYTES && files.len() > 1 {
                    aborted = true;
                    return Err(FastPathAbort.into());
                }
            }
        }

        Ok(())
    });

    let suppressed = match scan_result {
        Ok(outcome) => outcome
            .suppressed_errors
            .into_iter()
            .map(|e| format!("{} ({})", e.path, e.message))
            .collect::<Vec<_>>(),
        Err(err) => {
            if err.downcast_ref::<FastPathAbort>().is_none() {
                return Err(err);
            }
            // FastPathAbort means we threw mid-walk to escape the
            // tiny-budget tripwire. The capturing-enumerator's
            // outcome isn't returned in that case, but the abort
            // path always switches to streaming-planner, which
            // does its own (capturing) source.scan() and gets a
            // proper unreadable list — so leaving it empty here
            // is correct.
            Vec::new()
        }
    };

    if aborted {
        return Ok(FastPathOutcome::streaming().with_unreadable(suppressed));
    }

    if files.is_empty() {
        return Ok(
            FastPathOutcome::fast_path(FastPathDecision::NoWork { examined })
                .with_unreadable(suppressed),
        );
    }

    if files.len() <= TINY_FILE_LIMIT && total_bytes <= TINY_TOTAL_BYTES {
        return Ok(FastPathOutcome::fast_path(FastPathDecision::Tiny { files })
            .with_unreadable(suppressed));
    }

    if let Some((file, size)) = huge_candidate {
        if size >= HUGE_SINGLE_BYTES {
            return Ok(
                FastPathOutcome::fast_path(FastPathDecision::Huge { file, size })
                    .with_unreadable(suppressed),
            );
        }
    }

    Ok(FastPathOutcome::streaming().with_unreadable(suppressed))
}

#[cfg(test)]
mod tests {
    use super::*;
    use eyre::Result;
    use tempfile::tempdir;

    #[test]
    fn tiny_fast_path_single_file() -> Result<()> {
        let temp = tempdir()?;
        let src = temp.path().join("src");
        let dest = temp.path().join("dest");
        std::fs::create_dir_all(&src)?;
        std::fs::create_dir_all(&dest)?;
        std::fs::write(src.join("file.txt"), b"hello")?;

        let options = LocalMirrorOptions {
            perf_history: false,
            ..Default::default()
        };
        let outcome = maybe_select_fast_path(&src, &dest, &options)?;
        assert!(matches!(
            outcome.decision,
            Some(FastPathDecision::Tiny { .. })
        ));
        Ok(())
    }

    #[test]
    fn tiny_fast_path_many_small_files() -> Result<()> {
        let temp = tempdir()?;
        let src = temp.path().join("src");
        let dest = temp.path().join("dest");
        std::fs::create_dir_all(&src)?;
        std::fs::create_dir_all(&dest)?;
        for i in 0..100 {
            std::fs::write(src.join(format!("file_{i}.txt")), b"data")?;
        }

        let options = LocalMirrorOptions {
            perf_history: false,
            ..Default::default()
        };
        let outcome = maybe_select_fast_path(&src, &dest, &options)?;
        assert!(
            matches!(outcome.decision, Some(FastPathDecision::Tiny { .. })),
            "100 small files should use fast path"
        );
        Ok(())
    }

    #[test]
    fn streaming_path_when_over_file_limit() -> Result<()> {
        let temp = tempdir()?;
        let src = temp.path().join("src");
        let dest = temp.path().join("dest");
        std::fs::create_dir_all(&src)?;
        std::fs::create_dir_all(&dest)?;
        for i in 0..300 {
            std::fs::write(src.join(format!("file_{i}.txt")), b"data")?;
        }

        let options = LocalMirrorOptions {
            perf_history: false,
            ..Default::default()
        };
        let outcome = maybe_select_fast_path(&src, &dest, &options)?;
        assert!(
            outcome.decision.is_none(),
            "300 files should fall through to streaming path"
        );
        Ok(())
    }
}
