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
    NoWork,
    Tiny { files: Vec<(PathBuf, u64)> },
    Huge { file: PathBuf, size: u64 },
}

#[derive(Clone, Debug, Default)]
pub(super) struct FastPathOutcome {
    pub(super) decision: Option<FastPathDecision>,
}

impl FastPathOutcome {
    pub(super) fn fast_path(decision: FastPathDecision) -> Self {
        Self {
            decision: Some(decision),
        }
    }

    pub(super) fn streaming() -> Self {
        Self { decision: None }
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

    let scan_result = enumerator.enumerate_local_streaming(src_root, |entry| {
        if let EntryKind::File { size } = entry.kind {
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

    match scan_result {
        Ok(()) => {}
        Err(err) => {
            if err.downcast_ref::<FastPathAbort>().is_none() {
                return Err(err);
            }
        }
    }

    if aborted {
        return Ok(FastPathOutcome::streaming());
    }

    if files.is_empty() {
        return Ok(FastPathOutcome::fast_path(FastPathDecision::NoWork));
    }

    if files.len() <= TINY_FILE_LIMIT && total_bytes <= TINY_TOTAL_BYTES {
        return Ok(FastPathOutcome::fast_path(FastPathDecision::Tiny { files }));
    }

    if let Some((file, size)) = huge_candidate {
        if size >= HUGE_SINGLE_BYTES {
            return Ok(FastPathOutcome::fast_path(FastPathDecision::Huge { file, size }));
        }
    }

    Ok(FastPathOutcome::streaming())
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

        let mut options = LocalMirrorOptions::default();
        options.perf_history = false;
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

        let mut options = LocalMirrorOptions::default();
        options.perf_history = false;
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

        let mut options = LocalMirrorOptions::default();
        options.perf_history = false;
        let outcome = maybe_select_fast_path(&src, &dest, &options)?;
        assert!(
            outcome.decision.is_none(),
            "300 files should fall through to streaming path"
        );
        Ok(())
    }
}
