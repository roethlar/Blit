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
//! `ComparisonMode` in `proto/blit.proto` is the canonical input shape.
//! As of R2-F1 (`docs/reviews/followup_review_2026-05-02.md`) we honor
//! every variant with concrete semantics — no silent fall-through to
//! size+mtime. This means callers passing `SizeOnly`, `IgnoreTimes`,
//! or `Force` get the behavior the wire enum
//! advertises, not whatever the historical default happened to do.

use std::path::Path;

use eyre::{Context, Result};

use crate::checksum::{self, ChecksumType};
use crate::copy::file_needs_copy_with_checksum_type;
use crate::generated::{ComparisonMode, FileHeader};
use crate::remote::transfer::payload::{plan_transfer_payloads, PlannedPayloads};
use crate::transfer_plan::PlanOptions;

/// Push origins outsource the diff to the daemon: the client sends its
/// source manifest, daemon returns a NeedList, client filters to the
/// intersection. By the time we plan payloads, the headers are already
/// filtered. This re-exports the existing payload planner under the
/// diff_planner module so the push-client call site goes through the
/// unified module — there's no separate comparison stage to consolidate
/// (the comparison happens on the daemon, not the client).
///
/// When step 4 lands and the daemon-side diff moves into this module
/// for the pull case, push could in principle use the same daemon-side
/// helper instead of the round-trip-via-NeedList protocol. That would
/// be a deeper protocol change tracked under remote→remote re-evaluation
/// (step 5 of `docs/plan/PIPELINE_UNIFICATION.md`).
pub fn plan_push_payloads(
    headers: Vec<FileHeader>,
    source_root: &Path,
    plan_options: PlanOptions,
) -> Result<PlannedPayloads> {
    plan_transfer_payloads(headers, source_root, plan_options).context("planning push payloads")
}

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
    /// When true, skip any file the destination already has,
    /// regardless of `compare_mode`. Orthogonal axis; matches the
    /// `ignore_existing` field on `TransferOperationSpec`.
    pub ignore_existing: bool,
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
        filter_unchanged(
            &source_headers,
            inputs.src_root,
            inputs.dst_root,
            inputs.compare_mode,
            inputs.ignore_existing,
        )
    } else {
        source_headers
    };

    plan_transfer_payloads(headers_to_copy, inputs.src_root, inputs.plan_options)
        .context("planning payloads after diff stage")
}

/// Drop headers whose destination file already matches the source
/// under the chosen comparison mode. Keeps headers that need transfer.
///
/// `ignore_existing` is the orthogonal "skip if dst exists" axis from
/// `TransferOperationSpec`: when true, present destination files are
/// dropped before `compare_mode` is consulted at all.
///
/// This is the local-mirror flavor: it stats the destination directly.
/// Remote-source variants (where the destination manifest arrives over
/// the wire) live in their own helpers — TBD step 4.
///
/// Every `ComparisonMode` variant is implemented (R2-F1). `Unspecified`
/// behaves as `SizeMtime` (the historical default) — callers should fold
/// `Unspecified` away via `NormalizedTransferOperation::from_spec`
/// before reaching this function, but we accept it defensively.
pub fn filter_unchanged(
    headers: &[FileHeader],
    src_root: &Path,
    dst_root: &Path,
    compare_mode: ComparisonMode,
    ignore_existing: bool,
) -> Vec<FileHeader> {
    headers
        .iter()
        .filter(|h| {
            let src = src_root.join(&h.relative_path);
            let dst = dst_root.join(&h.relative_path);
            if ignore_existing && dst.exists() {
                return false;
            }
            local_needs_copy(&src, &dst, compare_mode).unwrap_or(true)
        })
        .cloned()
        .collect()
}

/// Per-mode comparison predicate. Returns `true` when the source file
/// should be transferred to the destination given the comparison mode.
///
/// All variants are implemented:
///
///   - `SizeMtime` / `Unspecified`: copy when missing, when sizes
///     differ, or when source is newer than dest by >2s. The 2s
///     tolerance matches the historical `file_needs_copy` primitive
///     and FAT/exFAT mtime granularity.
///   - `Checksum`: copy when missing, when sizes differ, or when
///     Blake3 hashes differ. mtime is not consulted.
///   - `SizeOnly`: copy when missing or sizes differ; mtime ignored.
///   - `IgnoreTimes`: always copy. Equivalent to rsync's
///     `--ignore-times` — destination is unconditionally rewritten.
///   - `Force`: always copy, even if dest is newer than source.
///     Equivalent to `IgnoreTimes` for one-way mirroring; named
///     differently because the rsync semantic also disables some
///     "skip newer" guards in mirror flows.
///
/// "Skip if destination exists" is not a `ComparisonMode` variant —
/// it's the orthogonal `ignore_existing` flag handled in
/// `filter_unchanged` before this function runs.
fn local_needs_copy(src: &Path, dst: &Path, mode: ComparisonMode) -> Result<bool> {
    match mode {
        ComparisonMode::IgnoreTimes | ComparisonMode::Force => Ok(true),
        ComparisonMode::SizeOnly => {
            if !dst.exists() {
                return Ok(true);
            }
            let src_meta = src.metadata().context("stat source for size compare")?;
            let dst_meta = dst.metadata().context("stat dest for size compare")?;
            Ok(src_meta.len() != dst_meta.len())
        }
        ComparisonMode::Checksum => {
            if !dst.exists() {
                return Ok(true);
            }
            let src_meta = src.metadata().context("stat source for checksum compare")?;
            let dst_meta = dst.metadata().context("stat dest for checksum compare")?;
            if src_meta.len() != dst_meta.len() {
                return Ok(true);
            }
            let src_hash = checksum::hash_file(src, ChecksumType::Blake3)
                .with_context(|| format!("hashing source {}", src.display()))?;
            let dst_hash = checksum::hash_file(dst, ChecksumType::Blake3)
                .with_context(|| format!("hashing dest {}", dst.display()))?;
            Ok(src_hash != dst_hash)
        }
        // Unspecified folds to the historical default. Callers that
        // run NormalizedTransferOperation never hit this branch.
        ComparisonMode::Unspecified | ComparisonMode::SizeMtime => {
            file_needs_copy_with_checksum_type(src, dst, None)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Build src+dst trees with the given (relative_path, content)
    /// pairs on each side. Returns (src_root, dst_root, _tempdir).
    fn make_trees(
        src_files: &[(&str, &[u8])],
        dst_files: &[(&str, &[u8])],
    ) -> (std::path::PathBuf, std::path::PathBuf, tempfile::TempDir) {
        let tmp = tempfile::tempdir().unwrap();
        let src = tmp.path().join("src");
        let dst = tmp.path().join("dst");
        std::fs::create_dir_all(&src).unwrap();
        std::fs::create_dir_all(&dst).unwrap();
        for (path, content) in src_files {
            let full = src.join(path);
            if let Some(parent) = full.parent() {
                std::fs::create_dir_all(parent).unwrap();
            }
            std::fs::write(full, content).unwrap();
        }
        for (path, content) in dst_files {
            let full = dst.join(path);
            if let Some(parent) = full.parent() {
                std::fs::create_dir_all(parent).unwrap();
            }
            std::fs::write(full, content).unwrap();
        }
        (src, dst, tmp)
    }

    fn header(rel: &str, size: u64) -> FileHeader {
        FileHeader {
            relative_path: rel.into(),
            size,
            mtime_seconds: 0,
            permissions: 0,
            checksum: vec![],
        }
    }

    fn sync_mtimes(src_root: &Path, dst_root: &Path, rel: &str) {
        let src_mtime = std::fs::metadata(src_root.join(rel))
            .unwrap()
            .modified()
            .unwrap();
        let _ = filetime::set_file_mtime(
            dst_root.join(rel),
            filetime::FileTime::from_system_time(src_mtime),
        );
    }

    fn kept_paths(kept: &[FileHeader]) -> Vec<String> {
        let mut v: Vec<String> = kept.iter().map(|h| h.relative_path.clone()).collect();
        v.sort();
        v
    }

    #[test]
    fn size_mtime_drops_matching_files() {
        let (src, dst, _tmp) = make_trees(
            &[("same.txt", b"matching content"), ("diff.txt", b"new")],
            &[
                ("same.txt", b"matching content"),
                ("diff.txt", b"old content"),
            ],
        );
        sync_mtimes(&src, &dst, "same.txt");

        let headers = vec![header("same.txt", 16), header("diff.txt", 3)];
        let kept = filter_unchanged(&headers, &src, &dst, ComparisonMode::SizeMtime, false);
        assert_eq!(kept_paths(&kept), vec!["diff.txt"]);
    }

    #[test]
    fn size_mtime_keeps_missing_dest() {
        let (src, dst, _tmp) = make_trees(&[("only.txt", b"hi")], &[]);
        let headers = vec![header("only.txt", 2)];
        let kept = filter_unchanged(&headers, &src, &dst, ComparisonMode::SizeMtime, false);
        assert_eq!(kept.len(), 1);
    }

    #[test]
    fn size_only_ignores_mtime_when_sizes_match() {
        let (src, dst, _tmp) = make_trees(&[("same.txt", b"abcdef")], &[("same.txt", b"abcdef")]);
        // Don't sync mtimes — they'll differ. SizeOnly should still drop
        // the entry because content sizes match.
        let headers = vec![header("same.txt", 6)];
        let kept = filter_unchanged(&headers, &src, &dst, ComparisonMode::SizeOnly, false);
        assert!(
            kept.is_empty(),
            "SizeOnly must skip files with matching size regardless of mtime"
        );
    }

    #[test]
    fn size_only_keeps_size_mismatch() {
        let (src, dst, _tmp) = make_trees(&[("file.txt", b"longer")], &[("file.txt", b"short")]);
        let headers = vec![header("file.txt", 6)];
        let kept = filter_unchanged(&headers, &src, &dst, ComparisonMode::SizeOnly, false);
        assert_eq!(kept.len(), 1);
    }

    #[test]
    fn ignore_times_always_copies() {
        let (src, dst, _tmp) = make_trees(
            &[("a.txt", b"x"), ("b.txt", b"y")],
            &[("a.txt", b"x"), ("b.txt", b"y")],
        );
        sync_mtimes(&src, &dst, "a.txt");
        sync_mtimes(&src, &dst, "b.txt");
        let headers = vec![header("a.txt", 1), header("b.txt", 1)];
        let kept = filter_unchanged(&headers, &src, &dst, ComparisonMode::IgnoreTimes, false);
        assert_eq!(kept.len(), 2, "IgnoreTimes must always copy");
    }

    #[test]
    fn force_always_copies() {
        let (src, dst, _tmp) = make_trees(&[("a.txt", b"x")], &[("a.txt", b"x")]);
        sync_mtimes(&src, &dst, "a.txt");
        let headers = vec![header("a.txt", 1)];
        let kept = filter_unchanged(&headers, &src, &dst, ComparisonMode::Force, false);
        assert_eq!(kept.len(), 1);
    }

    #[test]
    fn ignore_existing_skips_existing_regardless_of_mode() {
        // ignore_existing is orthogonal to compare_mode: even Force,
        // which would otherwise always copy, must respect it.
        let (src, dst, _tmp) = make_trees(
            &[("a.txt", b"new"), ("b.txt", b"only-on-src")],
            &[("a.txt", b"old")],
        );
        let headers = vec![header("a.txt", 3), header("b.txt", 11)];
        // Use SizeMtime as the mode (Force+ignore_existing is rejected
        // at the spec normalizer); we still expect a.txt to be skipped.
        let kept = filter_unchanged(&headers, &src, &dst, ComparisonMode::SizeMtime, true);
        assert_eq!(
            kept_paths(&kept),
            vec!["b.txt"],
            "ignore_existing keeps only files missing on dest"
        );
    }

    #[test]
    fn checksum_drops_byte_identical_files_with_diff_mtime() {
        let (src, dst, _tmp) = make_trees(
            &[("same.txt", b"identical bytes")],
            &[("same.txt", b"identical bytes")],
        );
        // Don't sync mtimes — Checksum mode shouldn't care about mtime.
        let headers = vec![header("same.txt", 15)];
        let kept = filter_unchanged(&headers, &src, &dst, ComparisonMode::Checksum, false);
        assert!(
            kept.is_empty(),
            "Checksum should skip byte-identical files regardless of mtime"
        );
    }

    #[test]
    fn checksum_keeps_content_diff() {
        let (src, dst, _tmp) =
            make_trees(&[("a.txt", b"hello world")], &[("a.txt", b"goodbye foo")]);
        let headers = vec![header("a.txt", 11)];
        let kept = filter_unchanged(&headers, &src, &dst, ComparisonMode::Checksum, false);
        assert_eq!(kept.len(), 1);
    }

    #[test]
    fn plan_local_mirror_skip_unchanged_off_passes_all_headers() {
        // R2-F4: when skip_unchanged=false the comparison stage is
        // bypassed, so identical files still appear in the planned
        // payloads. Equivalent to user-side --ignore-times / --force.
        let (src, dst, _tmp) = make_trees(
            &[("a.txt", b"x"), ("b.txt", b"y")],
            &[("a.txt", b"x"), ("b.txt", b"y")],
        );
        sync_mtimes(&src, &dst, "a.txt");
        sync_mtimes(&src, &dst, "b.txt");
        let headers = vec![header("a.txt", 1), header("b.txt", 1)];
        let planned = plan_local_mirror(
            headers,
            LocalDiffInputs {
                src_root: &src,
                dst_root: &dst,
                compare_mode: ComparisonMode::SizeMtime,
                ignore_existing: false,
                plan_options: PlanOptions::default(),
                skip_unchanged: false,
            },
        )
        .unwrap();
        assert_eq!(
            crate::remote::transfer::payload::payload_file_count(&planned.payloads),
            2,
            "skip_unchanged=false must keep matching files in the plan"
        );
    }

    #[test]
    fn plan_local_mirror_skip_unchanged_on_drops_matching_files() {
        // Counterpart to the above — confirms skip_unchanged=true
        // does drop matching files (the historical behavior).
        let (src, dst, _tmp) = make_trees(
            &[("a.txt", b"x"), ("b.txt", b"y")],
            &[("a.txt", b"x"), ("b.txt", b"y")],
        );
        sync_mtimes(&src, &dst, "a.txt");
        sync_mtimes(&src, &dst, "b.txt");
        let headers = vec![header("a.txt", 1), header("b.txt", 1)];
        let planned = plan_local_mirror(
            headers,
            LocalDiffInputs {
                src_root: &src,
                dst_root: &dst,
                compare_mode: ComparisonMode::SizeMtime,
                ignore_existing: false,
                plan_options: PlanOptions::default(),
                skip_unchanged: true,
            },
        )
        .unwrap();
        assert_eq!(
            crate::remote::transfer::payload::payload_file_count(&planned.payloads),
            0,
            "skip_unchanged=true must drop matching files before planning"
        );
    }

    #[test]
    fn plan_local_mirror_batches_many_small_files_into_tar_shard() {
        // R2-F4 tar-shard batching boundary: 50 tiny files in the
        // small bucket (<64KiB) should produce at least one TarShard
        // payload from the planner. We only assert that *some* tar
        // shard exists — the exact mix depends on the planner's
        // adaptive thresholds, which are tuning concerns rather than
        // a contract.
        use crate::remote::transfer::payload::TransferPayload;
        let tmp = tempfile::tempdir().unwrap();
        let src = tmp.path().join("src");
        let dst = tmp.path().join("dst");
        std::fs::create_dir_all(&src).unwrap();
        let mut headers = Vec::with_capacity(50);
        for i in 0..50 {
            let name = format!("f{:03}.txt", i);
            std::fs::write(src.join(&name), b"tiny").unwrap();
            headers.push(header(&name, 4));
        }
        let planned = plan_local_mirror(
            headers,
            LocalDiffInputs {
                src_root: &src,
                dst_root: &dst, // doesn't exist; skip_unchanged=false avoids stat
                compare_mode: ComparisonMode::SizeMtime,
                ignore_existing: false,
                plan_options: PlanOptions::default(),
                skip_unchanged: false,
            },
        )
        .unwrap();
        let tar_shards = planned
            .payloads
            .iter()
            .filter(|p| matches!(p, TransferPayload::TarShard { .. }))
            .count();
        assert!(
            tar_shards >= 1,
            "expected at least one TarShard payload for 50 small files, got {} payloads: {:?}",
            planned.payloads.len(),
            planned.payloads
        );
    }

    #[test]
    fn plan_local_mirror_force_tar_groups_even_a_few_files() {
        // PlanOptions::force_tar=true should always produce tar shards
        // regardless of file size distribution.
        use crate::remote::transfer::payload::TransferPayload;
        let tmp = tempfile::tempdir().unwrap();
        let src = tmp.path().join("src");
        std::fs::create_dir_all(&src).unwrap();
        for i in 0..3 {
            let name = format!("f{i}.txt");
            std::fs::write(src.join(&name), b"x").unwrap();
        }
        let headers = vec![
            header("f0.txt", 1),
            header("f1.txt", 1),
            header("f2.txt", 1),
        ];
        let plan_options = PlanOptions {
            force_tar: true,
            ..PlanOptions::default()
        };
        let planned = plan_local_mirror(
            headers,
            LocalDiffInputs {
                src_root: &src,
                dst_root: &src.join("nope"),
                compare_mode: ComparisonMode::SizeMtime,
                ignore_existing: false,
                plan_options,
                skip_unchanged: false,
            },
        )
        .unwrap();
        let has_tar = planned
            .payloads
            .iter()
            .any(|p| matches!(p, TransferPayload::TarShard { .. }));
        assert!(has_tar, "force_tar must produce a TarShard payload");
    }

    #[test]
    fn unspecified_folds_to_size_mtime() {
        // The orchestrator never sends Unspecified after normalization,
        // but defensively the planner should treat it as the historical
        // default (matches what NormalizedTransferOperation::from_spec does).
        let (src, dst, _tmp) = make_trees(&[("same.txt", b"x")], &[("same.txt", b"x")]);
        sync_mtimes(&src, &dst, "same.txt");
        let headers = vec![header("same.txt", 1)];
        let kept = filter_unchanged(&headers, &src, &dst, ComparisonMode::Unspecified, false);
        assert!(kept.is_empty());
    }
}
