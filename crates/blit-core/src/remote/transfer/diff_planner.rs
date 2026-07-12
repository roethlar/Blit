//! Payload planning for the session's SOURCE send half.
//!
//! [`plan_push_payloads`] shapes already-diffed need headers into
//! payloads (whole-file `File`, batched `TarShard`) via
//! `plan_transfer_payloads`. The diff itself is destination-owned
//! (`transfer_session::destination_needs` →
//! `manifest::header_transfer_status`) on every carrier; this module's
//! own local-mirror diff stage (`plan_local_mirror`/`filter_unchanged`)
//! died at otp-11b with the engine. The per-mode comparison semantics
//! (R2-F1: every `ComparisonMode` variant honored, no silent
//! fall-through) live in `copy::file_needs_copy_with_mode` — the sink's
//! defense layer — pinned in this module's tests.

use std::path::Path;

use eyre::{Context, Result};

use crate::generated::FileHeader;
use crate::remote::transfer::payload::{plan_transfer_payloads, TransferPayload};
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
) -> Result<Vec<TransferPayload>> {
    plan_transfer_payloads(headers, source_root, plan_options).context("planning push payloads")
}

// (`LocalDiffInputs` / `plan_local_mirror` / `filter_unchanged` died at
// otp-11b with their last caller, the engine's streaming plan — the
// local route diffs through the session's `destination_needs` and
// plans through `plan_transfer_payloads` like every other carrier. The
// per-mode decision tree they delegated to lives on in
// `copy::file_needs_copy_with_mode` — the sink's defense layer —
// pinned directly below.)

#[cfg(test)]
mod tests {
    use super::*;
    use crate::generated::ComparisonMode;

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

    fn needs_copy(src_root: &Path, dst_root: &Path, rel: &str, mode: ComparisonMode) -> bool {
        crate::copy::file_needs_copy_with_mode(&src_root.join(rel), &dst_root.join(rel), mode)
            .unwrap()
    }

    // Direct pins on `copy::file_needs_copy_with_mode` — the one
    // per-mode decision tree the sink's defense layer runs (converted
    // from the retired `filter_unchanged` pins at otp-11b).

    #[test]
    fn size_mtime_drops_matching_keeps_changed() {
        let (src, dst, _tmp) = make_trees(
            &[("same.txt", b"matching content"), ("diff.txt", b"new")],
            &[
                ("same.txt", b"matching content"),
                ("diff.txt", b"old content"),
            ],
        );
        sync_mtimes(&src, &dst, "same.txt");
        assert!(!needs_copy(
            &src,
            &dst,
            "same.txt",
            ComparisonMode::SizeMtime
        ));
        assert!(needs_copy(
            &src,
            &dst,
            "diff.txt",
            ComparisonMode::SizeMtime
        ));
    }

    #[test]
    fn size_mtime_keeps_missing_dest() {
        let (src, dst, _tmp) = make_trees(&[("only.txt", b"hi")], &[]);
        assert!(needs_copy(
            &src,
            &dst,
            "only.txt",
            ComparisonMode::SizeMtime
        ));
    }

    #[test]
    fn size_only_ignores_mtime_when_sizes_match() {
        let (src, dst, _tmp) = make_trees(&[("same.txt", b"abcdef")], &[("same.txt", b"abcdef")]);
        // Mtimes deliberately unsynced — SizeOnly must not care.
        assert!(
            !needs_copy(&src, &dst, "same.txt", ComparisonMode::SizeOnly),
            "SizeOnly must skip files with matching size regardless of mtime"
        );
    }

    #[test]
    fn size_only_keeps_size_mismatch() {
        let (src, dst, _tmp) = make_trees(&[("file.txt", b"longer")], &[("file.txt", b"short")]);
        assert!(needs_copy(&src, &dst, "file.txt", ComparisonMode::SizeOnly));
    }

    #[test]
    fn ignore_times_always_copies() {
        let (src, dst, _tmp) = make_trees(&[("a.txt", b"x")], &[("a.txt", b"x")]);
        sync_mtimes(&src, &dst, "a.txt");
        assert!(
            needs_copy(&src, &dst, "a.txt", ComparisonMode::IgnoreTimes),
            "IgnoreTimes must always copy"
        );
    }

    #[test]
    fn force_always_copies() {
        let (src, dst, _tmp) = make_trees(&[("a.txt", b"x")], &[("a.txt", b"x")]);
        sync_mtimes(&src, &dst, "a.txt");
        assert!(needs_copy(&src, &dst, "a.txt", ComparisonMode::Force));
    }

    #[test]
    fn checksum_drops_byte_identical_files_with_diff_mtime() {
        let (src, dst, _tmp) = make_trees(
            &[("same.txt", b"identical bytes")],
            &[("same.txt", b"identical bytes")],
        );
        // Mtimes deliberately unsynced — Checksum hashes content.
        assert!(
            !needs_copy(&src, &dst, "same.txt", ComparisonMode::Checksum),
            "Checksum should skip byte-identical files regardless of mtime"
        );
    }

    #[test]
    fn checksum_keeps_content_diff() {
        let (src, dst, _tmp) =
            make_trees(&[("a.txt", b"hello world")], &[("a.txt", b"goodbye foo")]);
        assert!(needs_copy(&src, &dst, "a.txt", ComparisonMode::Checksum));
    }

    #[test]
    fn unspecified_folds_to_size_mtime() {
        // Callers normalize Unspecified away, but the decision tree
        // accepts it defensively as the historical default.
        let (src, dst, _tmp) = make_trees(&[("same.txt", b"x")], &[("same.txt", b"x")]);
        sync_mtimes(&src, &dst, "same.txt");
        assert!(!needs_copy(
            &src,
            &dst,
            "same.txt",
            ComparisonMode::Unspecified
        ));
    }

    // Direct pins on `plan_transfer_payloads` — the one payload
    // planner every carrier uses (converted from the retired
    // `plan_local_mirror` pins at otp-11b; the diff half of those
    // compositions is pinned on the session route).

    #[test]
    fn planner_keeps_every_header() {
        let tmp = tempfile::tempdir().unwrap();
        let src = tmp.path().join("src");
        std::fs::create_dir_all(&src).unwrap();
        std::fs::write(src.join("a.txt"), b"x").unwrap();
        std::fs::write(src.join("b.txt"), b"y").unwrap();
        let headers = vec![header("a.txt", 1), header("b.txt", 1)];
        let planned = plan_transfer_payloads(headers, &src, PlanOptions::default()).unwrap();
        assert_eq!(
            crate::remote::transfer::payload::payload_file_count(&planned),
            2,
            "the planner must keep every header it is given"
        );
    }

    #[test]
    fn planner_batches_many_small_files_into_tar_shard() {
        // Tar-shard batching boundary: 50 tiny files should produce at
        // least one TarShard payload. Only "some tar shard exists" is
        // the contract — the exact mix is adaptive tuning.
        let tmp = tempfile::tempdir().unwrap();
        let src = tmp.path().join("src");
        std::fs::create_dir_all(&src).unwrap();
        let mut headers = Vec::with_capacity(50);
        for i in 0..50 {
            let name = format!("f{:03}.txt", i);
            std::fs::write(src.join(&name), b"tiny").unwrap();
            headers.push(header(&name, 4));
        }
        let planned = plan_transfer_payloads(headers, &src, PlanOptions::default()).unwrap();
        let tar_shards = planned
            .iter()
            .filter(|p| matches!(p, TransferPayload::TarShard { .. }))
            .count();
        assert!(
            tar_shards >= 1,
            "expected at least one TarShard payload for 50 small files, got {} payloads: {:?}",
            planned.len(),
            planned
        );
    }

    #[test]
    fn planner_force_tar_groups_even_a_few_files() {
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
        let planned = plan_transfer_payloads(headers, &src, plan_options).unwrap();
        let has_tar = planned
            .iter()
            .any(|p| matches!(p, TransferPayload::TarShard { .. }));
        assert!(has_tar, "force_tar must produce a TarShard payload");
    }
}
