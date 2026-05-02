//! Unified diff + payload planning stage.
//!
//! Sits between `TransferSource::scan` (which emits headers from the
//! origin's filesystem) and `execute_sink_pipeline_streaming` (which
//! dispatches payloads to one or more sinks). Decides:
//!
//!   1. Which source headers represent files that genuinely need to
//!      transfer (against the target's destination state).
//!   2. What payload shapes the surviving files become (whole-file
//!      `File` payloads, batched `TarShard`, or — once step 4 lands —
//!      block-level resume `FileBlock` + `FileBlockComplete` pairs).
//!
//! Step 3a of `docs/plan/PIPELINE_UNIFICATION.md`. Today this module
//! consolidates the local-mirror path that lived in `orchestrator.rs`
//! (`filter_headers_for_copy` + the call to `plan_transfer_payloads`).
//! Push and pull will adopt the same module in 3b and step 4.
//!
//! `ComparisonMode` in `proto/blit.proto` is the canonical input
//! shape; today we honor `SizeMtime` (default) and `Checksum`. The
//! other variants (`SizeOnly`, `IgnoreTimes`, `IgnoreExisting`,
//! `Force`) are accepted by the API and documented but mapped to the
//! historical defaults until step 4 brings them in alongside the
//! `pull_sync.rs` migration that introduced them.

use std::path::Path;

use eyre::{Context, Result};

use crate::checksum::ChecksumType;
use crate::copy::file_needs_copy_with_checksum_type;
use crate::generated::{ComparisonMode, FileHeader};
use crate::remote::transfer::payload::{plan_transfer_payloads, PlannedPayloads};
use crate::transfer_plan::PlanOptions;

/// Input bundle for the local-mirror diff stage. Origin and target
/// are co-located (both on the same filesystem), so the comparison
/// can stat the destination directly without a wire roundtrip.
pub struct LocalDiffInputs<'a> {
    /// Source-rooted absolute path. Headers' `relative_path` is
    /// joined under this to find the source bytes.
    pub src_root: &'a Path,
    /// Destination-rooted absolute path. Headers' `relative_path` is
    /// joined under this to compare against existing target state.
    pub dst_root: &'a Path,
    /// How to decide whether a target-existing file matches.
    pub compare_mode: ComparisonMode,
    /// Knobs for the tar / large / raw planner (unchanged from the
    /// pre-extraction call site).
    pub plan_options: PlanOptions,
    /// When false, every source header passes the comparison stage —
    /// equivalent to `--ignore-times`/`--force` in user-facing terms.
    /// Used by the orchestrator when its `skip_unchanged` flag is off.
    pub skip_unchanged: bool,
}

/// Filter source headers down to those that need transferring against
/// a local destination, then plan the surviving headers into payloads.
///
/// This is the single entry point the local-mirror path uses. Future
/// origin paths (push client, pull daemon) will gain their own entry
/// points on this module — same diff + planning algorithm, different
/// "where the destination lives" assumption.
pub fn plan_local_mirror(
    source_headers: Vec<FileHeader>,
    inputs: LocalDiffInputs<'_>,
) -> Result<PlannedPayloads> {
    let headers_to_copy = if inputs.skip_unchanged {
        filter_unchanged(&source_headers, inputs.src_root, inputs.dst_root, inputs.compare_mode)
    } else {
        source_headers
    };

    plan_transfer_payloads(headers_to_copy, inputs.src_root, inputs.plan_options)
        .context("planning payloads after diff stage")
}

/// Drop headers whose destination file already matches the source
/// under the chosen comparison mode. Keeps headers that need transfer.
///
/// This is the local-mirror flavor: it stats the destination directly.
/// Remote-source variants (where the destination manifest arrives over
/// the wire) live in their own helpers — TBD step 4.
pub fn filter_unchanged(
    headers: &[FileHeader],
    src_root: &Path,
    dst_root: &Path,
    compare_mode: ComparisonMode,
) -> Vec<FileHeader> {
    let checksum = checksum_for_mode(compare_mode);

    headers
        .iter()
        .filter(|h| {
            let src = src_root.join(&h.relative_path);
            let dst = dst_root.join(&h.relative_path);
            file_needs_copy_with_checksum_type(&src, &dst, checksum).unwrap_or(true)
        })
        .cloned()
        .collect()
}

/// Translate a `ComparisonMode` enum into the `Option<ChecksumType>`
/// that the existing comparison primitives consume.
///
/// Only `SizeMtime` (default) and `Checksum` map to existing behavior;
/// the other variants (`SizeOnly`, `IgnoreTimes`, `IgnoreExisting`,
/// `Force`) are accepted at the protocol surface but currently fall
/// back to size+mtime semantics. Step 4 (pull_sync.rs migration) is
/// where the alternate comparison strategies that today live as
/// PullSyncHeader bools get wired in.
fn checksum_for_mode(mode: ComparisonMode) -> Option<ChecksumType> {
    match mode {
        ComparisonMode::Checksum => Some(ChecksumType::Blake3),
        // Unspecified, SizeMtime, SizeOnly, IgnoreTimes, IgnoreExisting,
        // Force — all default to historical size+mtime comparison.
        // The non-SizeMtime variants are silently treated as SizeMtime
        // until step 4 brings their proper behavior over.
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn checksum_mode_enables_blake3() {
        assert_eq!(checksum_for_mode(ComparisonMode::Checksum), Some(ChecksumType::Blake3));
    }

    #[test]
    fn other_modes_default_to_size_mtime() {
        assert_eq!(checksum_for_mode(ComparisonMode::Unspecified), None);
        assert_eq!(checksum_for_mode(ComparisonMode::SizeMtime), None);
        // The variants below are protocol-level placeholders today;
        // they should still produce a valid (size+mtime) result.
        assert_eq!(checksum_for_mode(ComparisonMode::SizeOnly), None);
        assert_eq!(checksum_for_mode(ComparisonMode::IgnoreTimes), None);
        assert_eq!(checksum_for_mode(ComparisonMode::IgnoreExisting), None);
        assert_eq!(checksum_for_mode(ComparisonMode::Force), None);
    }

    #[test]
    fn filter_unchanged_drops_matching_files() {
        let tmp = tempfile::tempdir().unwrap();
        let src = tmp.path().join("src");
        let dst = tmp.path().join("dst");
        std::fs::create_dir_all(&src).unwrap();
        std::fs::create_dir_all(&dst).unwrap();

        // Two files: one identical on both sides, one different.
        std::fs::write(src.join("same.txt"), b"matching content").unwrap();
        std::fs::write(dst.join("same.txt"), b"matching content").unwrap();
        std::fs::write(src.join("diff.txt"), b"new content").unwrap();
        std::fs::write(dst.join("diff.txt"), b"old content").unwrap();
        // Sync mtimes for the matching pair so size+mtime equality holds.
        let src_mtime = std::fs::metadata(src.join("same.txt"))
            .unwrap()
            .modified()
            .unwrap();
        let _ = filetime::set_file_mtime(
            dst.join("same.txt"),
            filetime::FileTime::from_system_time(src_mtime),
        );

        let headers = vec![
            FileHeader {
                relative_path: "same.txt".into(),
                size: 16,
                mtime_seconds: 0,
                permissions: 0,
                checksum: vec![],
            },
            FileHeader {
                relative_path: "diff.txt".into(),
                size: 11,
                mtime_seconds: 0,
                permissions: 0,
                checksum: vec![],
            },
        ];
        let kept = filter_unchanged(&headers, &src, &dst, ComparisonMode::SizeMtime);
        // diff.txt has different size on disk → comparison says "needs copy".
        // same.txt has matching size+mtime → dropped.
        let kept_paths: Vec<_> = kept.iter().map(|h| h.relative_path.as_str()).collect();
        assert_eq!(kept_paths, vec!["diff.txt"]);
    }

    #[test]
    fn filter_unchanged_keeps_missing_dest() {
        let tmp = tempfile::tempdir().unwrap();
        let src = tmp.path().join("src");
        let dst = tmp.path().join("dst");
        std::fs::create_dir_all(&src).unwrap();
        std::fs::create_dir_all(&dst).unwrap();

        std::fs::write(src.join("only.txt"), b"hi").unwrap();

        let headers = vec![FileHeader {
            relative_path: "only.txt".into(),
            size: 2,
            mtime_seconds: 0,
            permissions: 0,
            checksum: vec![],
        }];
        let kept = filter_unchanged(&headers, &src, &dst, ComparisonMode::SizeMtime);
        assert_eq!(kept.len(), 1, "missing-on-dest file must survive filter");
    }
}
