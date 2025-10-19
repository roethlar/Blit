use std::path::{Path, PathBuf};

use eyre::Result;

use crate::enumeration::{EntryKind, FileEnumerator};
use crate::fs_enum::{CopyJob, FileEntry};
use crate::mirror_planner::MirrorPlanner;
use crate::perf_history::TransferMode;
use crate::perf_predictor::PerformancePredictor;

use super::LocalMirrorOptions;

pub(super) const TINY_FILE_LIMIT: usize = 8;
pub(super) const TINY_TOTAL_BYTES: u64 = 100 * 1024 * 1024;
pub(super) const HUGE_SINGLE_BYTES: u64 = 1024 * 1024 * 1024;
pub(super) const PREDICT_STREAMING_THRESHOLD_MS: f64 = 1_000.0;

#[derive(Clone, Debug)]
pub(super) enum FastPathDecision {
    NoWork,
    Tiny { files: Vec<(PathBuf, u64)> },
    Huge { file: PathBuf, size: u64 },
}

#[derive(Clone, Debug, Default)]
pub(super) struct FastPathOutcome {
    pub(super) decision: Option<FastPathDecision>,
    pub(super) prediction: Option<(f64, u64)>,
}

impl FastPathOutcome {
    pub(super) fn fast_path(decision: FastPathDecision, prediction: Option<(f64, u64)>) -> Self {
        Self {
            decision: Some(decision),
            prediction,
        }
    }

    pub(super) fn streaming(prediction: Option<(f64, u64)>) -> Self {
        Self {
            decision: None,
            prediction,
        }
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
    predictor: Option<&PerformancePredictor>,
) -> Result<FastPathOutcome> {
    if options.mirror || options.checksum || options.force_tar {
        return Ok(FastPathOutcome::streaming(None));
    }

    let mut enumerator = FileEnumerator::new(options.filter.clone_without_cache());
    if !options.preserve_symlinks {
        enumerator = enumerator.follow_symlinks(true);
    }
    if options.include_symlinks {
        enumerator = enumerator.include_symlinks(true);
    }

    let mode = if options.mirror {
        TransferMode::Mirror
    } else {
        TransferMode::Copy
    };

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
        return Ok(FastPathOutcome::streaming(None));
    }

    if files.is_empty() {
        return Ok(FastPathOutcome::fast_path(FastPathDecision::NoWork, None));
    }

    let prediction = predictor.and_then(|p| {
        p.predict_planner_ms(
            mode.clone(),
            None,
            options.skip_unchanged,
            options.checksum,
            files.len(),
            total_bytes,
        )
    });

    if files.len() <= TINY_FILE_LIMIT && total_bytes <= TINY_TOTAL_BYTES {
        let use_fast_path = prediction
            .map(|(ms, observations)| observations == 0 || ms > PREDICT_STREAMING_THRESHOLD_MS)
            .unwrap_or(true);
        if use_fast_path {
            return Ok(FastPathOutcome::fast_path(
                FastPathDecision::Tiny { files },
                prediction,
            ));
        }
    }

    if let Some((file, size)) = huge_candidate {
        if size >= HUGE_SINGLE_BYTES {
            return Ok(FastPathOutcome::fast_path(
                FastPathDecision::Huge { file, size },
                None,
            ));
        }
    }

    Ok(FastPathOutcome::streaming(prediction))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::perf_history::{OptionSnapshot, PerformanceRecord};
    use eyre::Result;
    use tempfile::tempdir;

    struct EnvGuard {
        key: &'static str,
        prev: Option<String>,
    }

    impl EnvGuard {
        fn set(key: &'static str, value: &str) -> Self {
            let prev = std::env::var(key).ok();
            std::env::set_var(key, value);
            Self { key, prev }
        }
    }

    impl Drop for EnvGuard {
        fn drop(&mut self) {
            if let Some(prev) = &self.prev {
                std::env::set_var(self.key, prev);
            } else {
                std::env::remove_var(self.key);
            }
        }
    }

    #[test]
    fn tiny_fast_path_without_history_prefers_fastpath() -> Result<()> {
        let _guard = EnvGuard::set("BLIT_DISABLE_PERF_HISTORY", "1");
        let temp = tempdir()?;
        let src = temp.path().join("src");
        let dest = temp.path().join("dest");
        std::fs::create_dir_all(&src)?;
        std::fs::create_dir_all(&dest)?;
        std::fs::write(src.join("file.txt"), b"hello")?;

        let options = LocalMirrorOptions::default();
        let outcome = maybe_select_fast_path(&src, &dest, &options, None)?;
        assert!(matches!(
            outcome.decision,
            Some(FastPathDecision::Tiny { .. })
        ));
        Ok(())
    }

    #[test]
    fn tiny_fast_path_uses_predictor_when_history_exists() -> Result<()> {
        let _guard = EnvGuard::set("BLIT_DISABLE_PERF_HISTORY", "1");
        let temp = tempdir()?;
        let src = temp.path().join("src");
        let dest = temp.path().join("dest");
        std::fs::create_dir_all(&src)?;
        std::fs::create_dir_all(&dest)?;
        std::fs::write(src.join("file.txt"), b"hello")?;

        let mut predictor = PerformancePredictor::for_tests(temp.path());
        let snapshot = OptionSnapshot {
            dry_run: false,
            preserve_symlinks: true,
            include_symlinks: true,
            skip_unchanged: true,
            checksum: false,
            workers: 4,
        };
        let record = PerformanceRecord::new(
            TransferMode::Copy,
            None,
            None,
            2,
            256,
            snapshot,
            None,
            100,
            1_000,
            0,
            0,
        );
        predictor.observe(&record);

        let options = LocalMirrorOptions::default();
        let outcome = maybe_select_fast_path(&src, &dest, &options, Some(&predictor))?;
        assert!(
            outcome.decision.is_none(),
            "predictor should keep streaming path when predicted planning is fast"
        );
        let (pred_ms, _) = outcome.prediction.expect("expected prediction");
        assert!(pred_ms <= PREDICT_STREAMING_THRESHOLD_MS);
        Ok(())
    }
}
