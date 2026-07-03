use std::path::Path;
use std::sync::Arc;

use eyre::{eyre, Context, Result};
use tokio::runtime::Builder;

use crate::engine::{EngineRequest, TransferEngine};
use crate::remote::transfer::sink::{FsSinkConfig, FsTransferSink, NullSink, TransferSink};
use crate::remote::transfer::source::{FilteredSource, FsTransferSource, TransferSource};

use super::{LocalMirrorOptions, LocalMirrorSummary};

/// The LOCAL adapter for [`TransferEngine`] (ue-r2-1c, REV4 Design §1).
///
/// Owns exactly the path-specific boundary work: precondition checks,
/// construction of the local filesystem source (filter-wrapped) and
/// sink, and option translation. Everything else -- strategy selection
/// (journal / fast paths / single-file / streaming), execution, and
/// accounting -- lives in the engine. The public API is unchanged from
/// the pre-engine orchestrator.
pub struct TransferOrchestrator;

impl TransferOrchestrator {
    pub fn new() -> Self {
        Self
    }

    /// Sync wrapper around [`execute_local_mirror_async`]. Builds a
    /// new multi-thread Tokio runtime and blocks on it. Use this from
    /// non-async callers (CLI commands, tests). Callers already
    /// inside an async runtime must use `execute_local_mirror_async`
    /// directly -- calling this from inside a Tokio context will
    /// panic at `Runtime::new` (closes F9 of
    /// `docs/reviews/codebase_review_2026-05-01.md`).
    ///
    /// [`execute_local_mirror_async`]: Self::execute_local_mirror_async
    pub fn execute_local_mirror(
        &self,
        src_root: &Path,
        dest_root: &Path,
        options: LocalMirrorOptions,
    ) -> Result<LocalMirrorSummary> {
        let workers = options.workers.max(1);
        let runtime = Builder::new_multi_thread()
            .worker_threads(workers)
            .enable_all()
            .build()
            .context("build tokio runtime")?;
        runtime.block_on(self.execute_local_mirror_async(src_root, dest_root, options))
    }

    /// Async local-transfer entry point: validate the local
    /// preconditions, construct the local source/sink pair, and hand
    /// execution to the engine.
    pub async fn execute_local_mirror_async(
        &self,
        src_root: &Path,
        dest_root: &Path,
        options: LocalMirrorOptions,
    ) -> Result<LocalMirrorSummary> {
        if !src_root.exists() {
            return Err(eyre!("source path does not exist: {}", src_root.display()));
        }

        if !options.dry_run {
            if let Some(parent) = dest_root.parent() {
                std::fs::create_dir_all(parent).with_context(|| {
                    format!("failed to create destination parent {}", parent.display())
                })?;
            }
        }

        // Local source, wrapped in FilteredSource so the user filter
        // applies through the universal pipeline chokepoint (identical
        // to push/pull/remote-remote behavior -- full parity).
        let inner: Arc<dyn TransferSource> =
            Arc::new(FsTransferSource::new(src_root.to_path_buf()));
        let source: Arc<dyn TransferSource> = Arc::new(FilteredSource::new(
            inner,
            options.filter.clone_without_cache(),
        ));

        // Local sink. Construction is pure state (paths + config), so
        // building it up front -- even for runs the engine resolves via
        // a fast path that never touches it -- is behavior-neutral.
        let compare_mode = options
            .compare_mode
            .resolve_comparison_mode(options.checksum);
        let sink: Arc<dyn TransferSink> = if options.null_sink {
            Arc::new(NullSink::new())
        } else {
            Arc::new(FsTransferSink::new(
                src_root.to_path_buf(),
                dest_root.to_path_buf(),
                FsSinkConfig {
                    preserve_times: options.preserve_times,
                    dry_run: options.dry_run,
                    checksum: if options.checksum {
                        Some(crate::checksum::ChecksumType::Blake3)
                    } else {
                        None
                    },
                    resume: options.resume,
                    // R58-followup: thread the compare_mode into the
                    // sink. Pre-fix the sink hard-coded SizeMtime via
                    // file_needs_copy_with_checksum_type, defeating
                    // --force / --ignore-times: the planner emitted
                    // the file but the sink decided "skip" when
                    // mtime+size matched.
                    compare_mode,
                },
            ))
        };

        TransferEngine::new()
            .execute(EngineRequest {
                src_root: src_root.to_path_buf(),
                dest_root: dest_root.to_path_buf(),
                source,
                sink,
                options,
            })
            .await
    }
}

impl Default for TransferOrchestrator {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod async_runtime_tests {
    //! F9 regression: `execute_local_mirror_async` must be callable
    //! from inside an existing Tokio runtime without panicking. The
    //! sync `execute_local_mirror` wrapper builds its own runtime
    //! and would panic with "Cannot start a runtime from within a
    //! runtime" if called from `#[tokio::test]`.
    use super::*;
    use tempfile::tempdir;

    fn write_file(path: &std::path::Path, body: &[u8]) {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).unwrap();
        }
        std::fs::write(path, body).unwrap();
    }

    fn opts() -> LocalMirrorOptions {
        LocalMirrorOptions {
            workers: 2,
            preserve_times: false,
            dry_run: false,
            checksum: false,
            ..Default::default()
        }
    }

    #[tokio::test]
    async fn async_version_callable_from_async_context() {
        // The whole point of F9 — calling the async version from
        // within #[tokio::test]'s runtime must not build a nested
        // runtime or panic.
        let tmp = tempdir().unwrap();
        let src = tmp.path().join("src");
        let dst = tmp.path().join("dst");
        write_file(&src.join("a.txt"), b"hello");
        let orch = TransferOrchestrator::new();
        let summary = orch
            .execute_local_mirror_async(&src, &dst, opts())
            .await
            .unwrap();
        assert!(
            summary.copied_files >= 1,
            "expected at least one file copied, got {:?}",
            summary
        );
        assert!(dst.join("a.txt").exists());
    }

    #[test]
    fn sync_wrapper_still_works() {
        // The sync API must keep working for non-async callers
        // (CLI commands today).
        let tmp = tempdir().unwrap();
        let src = tmp.path().join("src");
        let dst = tmp.path().join("dst");
        write_file(&src.join("a.txt"), b"hello-sync");
        let orch = TransferOrchestrator::new();
        let summary = orch.execute_local_mirror(&src, &dst, opts()).unwrap();
        assert!(summary.copied_files >= 1);
        assert_eq!(std::fs::read(dst.join("a.txt")).unwrap(), b"hello-sync");
    }

    /// R45 regression: `summary.total_bytes` must report bytes the
    /// pipeline actually wrote, not bytes the source scan saw. The
    /// pre-fix R44 commit aliased `let total_bytes = scanned_bytes`
    /// and fed that into the summary — so on this skip-unchanged
    /// incremental run the second run would have reported the full
    /// scanned size as bytes-written even though zero bytes were
    /// actually written.
    ///
    /// The fast-path branches (NoWork / Tiny / Huge / JournalSkip)
    /// don't exhibit the bug because they construct their summary
    /// directly without going through the aliased local. We force
    /// the streaming-pipeline path by enabling `mirror = true`,
    /// which disables fast-path selection (see
    /// `maybe_select_fast_path`'s mirror short-circuit).
    #[tokio::test]
    async fn incremental_run_total_bytes_excludes_skipped_files() {
        let tmp = tempdir().unwrap();
        let src = tmp.path().join("src");
        let dst = tmp.path().join("dst");
        let body_a = vec![b'a'; 2 * 1024];
        let body_b = vec![b'b'; 2 * 1024];
        write_file(&src.join("a.txt"), &body_a);
        write_file(&src.join("b.txt"), &body_b);
        let total_payload = (body_a.len() + body_b.len()) as u64;

        // mirror=true forces the streaming pipeline (fast-path is
        // skipped for mirror runs); skip_unchanged=true means the
        // diff stage will mark both files unchanged on the second
        // run, so the pipeline writes 0 bytes.
        let mut run_opts = opts();
        run_opts.mirror = true;
        run_opts.skip_unchanged = true;

        let orch = TransferOrchestrator::new();
        let first = orch
            .execute_local_mirror_async(&src, &dst, run_opts.clone())
            .await
            .unwrap();
        assert_eq!(
            first.scanned_files, 2,
            "first run should hit streaming planner and scan both files (got summary {:?})",
            first
        );
        assert_eq!(first.scanned_bytes, total_payload);
        assert_eq!(
            first.total_bytes, total_payload,
            "from-scratch run: total_bytes equals bytes written"
        );
        assert_eq!(first.copied_files, 2);

        let second = orch
            .execute_local_mirror_async(&src, &dst, run_opts)
            .await
            .unwrap();
        assert_eq!(
            second.scanned_files, 2,
            "second run still scans both files in mirror mode (got summary {:?})",
            second
        );
        assert_eq!(second.scanned_bytes, total_payload);
        assert_eq!(
            second.total_bytes, 0,
            "incremental skip_unchanged run must report 0 bytes \
             written; R45 alias bug would have reported {} here \
             (full summary: {:?})",
            second.scanned_bytes, second
        );
        assert_eq!(second.copied_files, 0);
    }

    /// R46-F2 regression: a mirror with an unreadable source
    /// subdirectory must NOT delete the corresponding destination
    /// subtree. Pre-fix the walkdir error on the unreadable
    /// subdir was silently dropped at `enumeration.rs:90-95`, the
    /// orchestrator never checked `unreadable`, and
    /// `apply_mirror_deletions` would treat the unscanned subtree
    /// as "absent at source" and delete the matching destination
    /// path. Now the mirror branch refuses to delete with a clear
    /// error.
    ///
    /// Unix-only because we rely on `chmod 000` to make the
    /// subdirectory unreadable and that doesn't work the same way
    /// on Windows.
    #[cfg(unix)]
    #[tokio::test]
    async fn mirror_refuses_when_source_scan_incomplete() {
        use std::os::unix::fs::PermissionsExt;
        let tmp = tempdir().unwrap();
        let src = tmp.path().join("src");
        let dst = tmp.path().join("dst");

        // Source has a readable file and a subdirectory we'll make
        // unreadable so the walkdir can't enter it.
        write_file(&src.join("readable.txt"), b"keep");
        let blocked = src.join("blocked");
        std::fs::create_dir_all(&blocked).unwrap();
        write_file(&blocked.join("inner.txt"), b"unscannable");

        // Destination has the readable file already AND a
        // subdirectory matching the (now-unreadable) source
        // subdir. Pre-fix mirror would delete `dst/blocked/`
        // because the source scan never observed it.
        std::fs::create_dir_all(&dst).unwrap();
        write_file(&dst.join("readable.txt"), b"keep");
        std::fs::create_dir_all(dst.join("blocked")).unwrap();
        write_file(&dst.join("blocked/preserve_me.txt"), b"survivor");

        // Make src/blocked unreadable to the walkdir.
        let mut perms = std::fs::metadata(&blocked).unwrap().permissions();
        perms.set_mode(0o000);
        std::fs::set_permissions(&blocked, perms).unwrap();
        // Restore perms in a guard so cleanup works whatever the
        // assertion outcome.
        struct PermGuard(std::path::PathBuf);
        impl Drop for PermGuard {
            fn drop(&mut self) {
                let mut p = std::fs::metadata(&self.0).unwrap().permissions();
                p.set_mode(0o755);
                let _ = std::fs::set_permissions(&self.0, p);
            }
        }
        let _guard = PermGuard(blocked.clone());

        let mut opts = opts();
        opts.mirror = true;
        let orch = TransferOrchestrator::new();
        let result = orch.execute_local_mirror_async(&src, &dst, opts).await;

        // Mirror should refuse with an explicit error. Pre-fix it
        // would have returned Ok and deleted dst/blocked/.
        let err = match result {
            Err(e) => e,
            Ok(summary) => {
                panic!(
                    "expected mirror to refuse on incomplete scan, \
                     got Ok(summary={:?}); dst/blocked/preserve_me \
                     exists: {}",
                    summary,
                    dst.join("blocked/preserve_me.txt").exists()
                );
            }
        };
        let msg = format!("{err:#}");
        assert!(
            msg.contains("source scan was") && msg.contains("incomplete"),
            "expected scan-incomplete error, got: {msg}"
        );
        // The destination subtree must still be intact.
        assert!(
            dst.join("blocked/preserve_me.txt").exists(),
            "dst/blocked/preserve_me.txt was deleted (R46-F2 \
             incomplete-scan-mirror-delete regression)"
        );
    }

    /// R46-F5 regression: mirror-delete failures must surface as an
    /// Err on the orchestrator's return value, not be silently
    /// swallowed into a warning + Ok summary. We force a deletion
    /// failure by making the destination's "extra" file's parent
    /// non-writable; on unix `remove_file` then fails with EACCES.
    #[cfg(unix)]
    #[tokio::test]
    async fn mirror_delete_failure_propagates_as_error() {
        use std::os::unix::fs::PermissionsExt;
        let tmp = tempdir().unwrap();
        let src = tmp.path().join("src");
        let dst = tmp.path().join("dst");

        write_file(&src.join("kept.txt"), b"src");

        std::fs::create_dir_all(&dst).unwrap();
        write_file(&dst.join("kept.txt"), b"src");
        // The "extra" file mirror would try to delete. Lock its
        // parent dir so the unlink fails.
        let locked_parent = dst.join("locked_subdir");
        std::fs::create_dir_all(&locked_parent).unwrap();
        write_file(&locked_parent.join("extra.txt"), b"unwanted");
        let mut perms = std::fs::metadata(&locked_parent).unwrap().permissions();
        perms.set_mode(0o555); // r-xr-xr-x: contents listable, not writable
        std::fs::set_permissions(&locked_parent, perms).unwrap();

        struct PermGuard(std::path::PathBuf);
        impl Drop for PermGuard {
            fn drop(&mut self) {
                let mut p = std::fs::metadata(&self.0).unwrap().permissions();
                p.set_mode(0o755);
                let _ = std::fs::set_permissions(&self.0, p);
            }
        }
        let _g = PermGuard(locked_parent.clone());

        let mut opts = opts();
        opts.mirror = true;
        let orch = TransferOrchestrator::new();
        let result = orch.execute_local_mirror_async(&src, &dst, opts).await;

        let err = match result {
            Err(e) => e,
            Ok(summary) => panic!(
                "expected mirror-delete failure to propagate as Err, \
                 got Ok(summary={:?})",
                summary
            ),
        };
        let msg = format!("{err:#}");
        assert!(
            msg.contains("mirror-delete left") && msg.contains("in place"),
            "expected mirror-delete-left-in-place message, got: {msg}"
        );
    }

    /// R48-F1 regression: a normal mirror that contains a
    /// subdirectory with a source file must succeed. Pre-fix
    /// `source_paths` was a set of file paths only; every dest
    /// directory was "absent at source" and got queued for
    /// `remove_dir`, which (after R46-F5 promoted those failures
    /// to hard errors) failed the whole mirror with ENOTEMPTY on
    /// the parent dir that contained the freshly-copied file.
    #[tokio::test]
    async fn mirror_with_subdir_does_not_treat_parent_dir_as_absent() {
        let tmp = tempdir().unwrap();
        let src = tmp.path().join("src");
        let dst = tmp.path().join("dst");

        // Source: nested file under a subdir.
        write_file(&src.join("sub/file.txt"), b"payload");

        let mut opts = opts();
        opts.mirror = true;
        let orch = TransferOrchestrator::new();
        let summary = orch
            .execute_local_mirror_async(&src, &dst, opts)
            .await
            .unwrap_or_else(|e| panic!("mirror failed: {e:#}"));

        assert!(
            dst.join("sub/file.txt").exists(),
            "destination subdir file must exist after mirror, got summary: {:?}",
            summary
        );
        // The parent dir is implicitly in source — must not have
        // been counted as a deletion.
        assert_eq!(
            summary.deleted_dirs, 0,
            "mirror over a single nested file must not delete any \
             destination directory; got summary: {:?}",
            summary
        );
    }

    /// R48-F1 sibling: a destination dir that the source *doesn't*
    /// reference must still be deleted by mirror.
    #[tokio::test]
    async fn mirror_still_deletes_truly_unrelated_destination_dirs() {
        let tmp = tempdir().unwrap();
        let src = tmp.path().join("src");
        let dst = tmp.path().join("dst");

        write_file(&src.join("kept.txt"), b"src");
        // Pre-existing dest dir that's not part of source.
        std::fs::create_dir_all(dst.join("stale_dir")).unwrap();
        // Plus a stale file inside it, so the delete order has to
        // be deepest-first.
        write_file(&dst.join("stale_dir/extra.txt"), b"stale");

        let mut opts = opts();
        opts.mirror = true;
        let orch = TransferOrchestrator::new();
        let summary = orch
            .execute_local_mirror_async(&src, &dst, opts)
            .await
            .unwrap();

        assert!(dst.join("kept.txt").exists());
        assert!(
            !dst.join("stale_dir/extra.txt").exists(),
            "mirror must still delete files in unrelated dest subdirs"
        );
        assert!(
            !dst.join("stale_dir").exists(),
            "mirror must still delete unrelated dest dirs once empty"
        );
        assert!(summary.deleted_dirs >= 1);
        assert!(summary.deleted_files >= 1);
    }

    /// R58-F4 regression: local dry-run on a directory source must
    /// not create the destination directory. Pre-fix `blit copy
    /// src/ dst/ --dry-run` would create `dst/` on disk.
    #[tokio::test]
    async fn local_dry_run_does_not_create_destination() {
        let tmp = tempdir().unwrap();
        let src = tmp.path().join("src");
        let dst = tmp.path().join("brand_new_dst");
        write_file(&src.join("a.txt"), b"hello");

        let mut opts = opts();
        opts.dry_run = true;
        let orch = TransferOrchestrator::new();
        let _ = orch
            .execute_local_mirror_async(&src, &dst, opts)
            .await
            .unwrap();

        assert!(!dst.exists(), "dry-run must not create destination dir");
    }

    /// R58-F5 regression: single-file local copy must honor
    /// `options.filter`. Pre-fix `execute_single_file_copy`
    /// short-circuited around the enumerator/planner and copied
    /// regardless of filter rules.
    #[tokio::test]
    async fn single_file_copy_honors_filter_excludes() {
        let tmp = tempdir().unwrap();
        let src = tmp.path().join("src.txt");
        let dst = tmp.path().join("dst.txt");
        std::fs::write(&src, b"would-be-copied").unwrap();

        let mut opts = opts();
        // Build a filter that excludes `*.txt`. FileFilter has
        // private compiled-glob caches so we go through
        // clone_without_cache() to construct one cleanly.
        let mut filter = crate::fs_enum::FileFilter::default();
        filter.exclude_files = vec!["*.txt".to_string()];
        opts.filter = filter.clone_without_cache();

        let orch = TransferOrchestrator::new();
        let summary = orch
            .execute_local_mirror_async(&src, &dst, opts)
            .await
            .unwrap();

        assert_eq!(
            summary.copied_files, 0,
            "filter exclusion must skip the file"
        );
        assert!(!dst.exists(), "excluded file must not be copied (R58-F5)");
    }

    /// R58-F6 regression: local mirror with `--exclude '*.log'`
    /// must not try to remove an out-of-scope directory just
    /// because the filter hid its in-scope contents. Pre-fix
    /// `apply_mirror_deletions` enumerated the destination
    /// through the filter, saw the .log file as out-of-scope (so
    /// the dir looked empty), and queued the dir for
    /// `remove_dir` — which failed with ENOTEMPTY because the
    /// .log was actually still inside.
    #[tokio::test]
    async fn local_mirror_subset_keeps_excluded_only_directories() {
        let tmp = tempdir().unwrap();
        let src = tmp.path().join("src");
        let dst = tmp.path().join("dst");
        write_file(&src.join("keep.txt"), b"src-keep");
        // Pre-existing destination structure: a directory that
        // only contains an excluded `.log` file.
        std::fs::create_dir_all(dst.join("logs")).unwrap();
        write_file(&dst.join("logs/app.log"), b"excluded contents");

        let mut opts = opts();
        opts.mirror = true;
        // FilteredSubset is the default; spell it out for clarity.
        opts.delete_scope = crate::orchestrator::LocalMirrorDeleteScope::FilteredSubset;
        opts.filter.exclude_files = vec!["*.log".to_string()];

        let orch = TransferOrchestrator::new();
        let summary = orch
            .execute_local_mirror_async(&src, &dst, opts)
            .await
            .unwrap_or_else(|e| panic!("mirror failed: {e:#}"));

        // Mirror must succeed even though `dst/logs/` contains an
        // out-of-scope file. The `.log` survives, the dir
        // survives, the in-scope file transferred.
        assert!(dst.join("keep.txt").exists());
        assert!(
            dst.join("logs/app.log").exists(),
            "excluded .log file must not be deleted by mirror"
        );
        assert!(
            dst.join("logs").exists(),
            "dir containing only excluded files must survive mirror \
             (R58-F6 — pre-fix this failed with ENOTEMPTY)"
        );
        let _ = summary;
    }

    /// R58-F6 sibling: `--delete-scope=all` deletes through the
    /// filter, including dirs that only hold excluded files. The
    /// user explicitly opted out of subset semantics.
    #[tokio::test]
    async fn local_mirror_all_scope_deletes_through_filter() {
        let tmp = tempdir().unwrap();
        let src = tmp.path().join("src");
        let dst = tmp.path().join("dst");
        write_file(&src.join("keep.txt"), b"src-keep");
        std::fs::create_dir_all(dst.join("logs")).unwrap();
        write_file(&dst.join("logs/app.log"), b"deletable in All mode");

        let mut opts = opts();
        opts.mirror = true;
        opts.delete_scope = crate::orchestrator::LocalMirrorDeleteScope::All;
        opts.filter.exclude_files = vec!["*.log".to_string()];

        let orch = TransferOrchestrator::new();
        let _ = orch
            .execute_local_mirror_async(&src, &dst, opts)
            .await
            .unwrap();

        assert!(dst.join("keep.txt").exists());
        assert!(
            !dst.join("logs/app.log").exists(),
            "All scope must delete excluded files at destination"
        );
        assert!(
            !dst.join("logs").exists(),
            "All scope must delete the now-empty dir"
        );
    }

    /// R58-F7 regression: local copy honors `compare_mode =
    /// SizeOnly`. With a destination that has the same SIZE but
    /// different MTIME, default SizeMtime would re-copy
    /// (mtime differs); SizeOnly must skip.
    #[tokio::test]
    async fn local_copy_honors_size_only_compare_mode() {
        let tmp = tempdir().unwrap();
        let src = tmp.path().join("src");
        let dst = tmp.path().join("dst");
        write_file(&src.join("file.txt"), b"AAAA"); // 4 bytes
        write_file(&dst.join("file.txt"), b"BBBB"); // 4 bytes, different content

        // Bump the source mtime so SizeMtime would re-copy.
        let now = std::time::SystemTime::now();
        let later = now + std::time::Duration::from_secs(10);
        filetime::set_file_mtime(
            src.join("file.txt"),
            filetime::FileTime::from_system_time(later),
        )
        .unwrap();
        filetime::set_file_mtime(
            dst.join("file.txt"),
            filetime::FileTime::from_system_time(now),
        )
        .unwrap();

        let mut opts = opts();
        opts.compare_mode = crate::orchestrator::LocalCompareMode::SizeOnly;

        let orch = TransferOrchestrator::new();
        let _ = orch
            .execute_local_mirror_async(&src, &dst, opts)
            .await
            .unwrap();

        // SizeOnly: same size → skip → dst content unchanged.
        assert_eq!(
            std::fs::read(dst.join("file.txt")).unwrap(),
            b"BBBB",
            "SizeOnly compare must skip when sizes match (R58-F7)"
        );
    }

    /// R58-F7 regression: local copy honors `compare_mode = Force`.
    /// With matching size+mtime, default SizeMtime would skip;
    /// Force must always re-copy.
    #[tokio::test]
    async fn local_copy_honors_force_compare_mode() {
        let tmp = tempdir().unwrap();
        let src = tmp.path().join("src");
        let dst = tmp.path().join("dst");
        write_file(&src.join("file.txt"), b"CCCC");
        write_file(&dst.join("file.txt"), b"OLD!");

        // Match size+mtime so SizeMtime would skip.
        let t = filetime::FileTime::from_unix_time(1_700_000_000, 0);
        filetime::set_file_mtime(src.join("file.txt"), t).unwrap();
        filetime::set_file_mtime(dst.join("file.txt"), t).unwrap();

        let mut opts = opts();
        opts.compare_mode = crate::orchestrator::LocalCompareMode::Force;

        let orch = TransferOrchestrator::new();
        let _ = orch
            .execute_local_mirror_async(&src, &dst, opts)
            .await
            .unwrap();

        assert_eq!(
            std::fs::read(dst.join("file.txt")).unwrap(),
            b"CCCC",
            "Force compare must always copy even when size+mtime match (R58-F7)"
        );
    }

    /// R58-F5 regression: single-file local copy must honor
    /// `--ignore-existing`. Pre-fix the short-circuit overwrote
    /// the destination regardless.
    #[tokio::test]
    async fn single_file_copy_honors_ignore_existing() {
        let tmp = tempdir().unwrap();
        let src = tmp.path().join("src.txt");
        let dst = tmp.path().join("dst.txt");
        std::fs::write(&src, b"new-content").unwrap();
        std::fs::write(&dst, b"existing-pre-existing").unwrap();

        let mut opts = opts();
        opts.ignore_existing = true;

        let orch = TransferOrchestrator::new();
        let summary = orch
            .execute_local_mirror_async(&src, &dst, opts)
            .await
            .unwrap();

        assert_eq!(
            summary.copied_files, 0,
            "--ignore-existing must skip when destination exists"
        );
        assert_eq!(
            std::fs::read(&dst).unwrap(),
            b"existing-pre-existing",
            "destination content must be preserved (R58-F5)"
        );
    }

    /// R58-followup: single-file `--size-only` must skip when sizes
    /// match. Reviewer-reproduced: `blit copy src.txt dst.txt
    /// --size-only` was overwriting a same-size destination because
    /// the short-circuit called `file_needs_copy_with_checksum_type`
    /// (SizeMtime-or-Checksum) instead of routing through the new
    /// `file_needs_copy_with_mode(compare_mode)` helper.
    #[tokio::test]
    async fn single_file_copy_size_only_skips_same_size() {
        let tmp = tempdir().unwrap();
        let src = tmp.path().join("src.txt");
        let dst = tmp.path().join("dst.txt");
        std::fs::write(&src, b"AAAA").unwrap();
        std::fs::write(&dst, b"BBBB").unwrap(); // same size, different content
        let now = std::time::SystemTime::now();
        filetime::set_file_mtime(
            &src,
            filetime::FileTime::from_system_time(now + std::time::Duration::from_secs(10)),
        )
        .unwrap();
        filetime::set_file_mtime(&dst, filetime::FileTime::from_system_time(now)).unwrap();

        let mut opts = opts();
        opts.compare_mode = crate::orchestrator::LocalCompareMode::SizeOnly;

        let orch = TransferOrchestrator::new();
        let _ = orch
            .execute_local_mirror_async(&src, &dst, opts)
            .await
            .unwrap();

        assert_eq!(
            std::fs::read(&dst).unwrap(),
            b"BBBB",
            "single-file --size-only must skip when sizes match"
        );
    }

    /// R58-followup: `--force` must copy through the sink layer
    /// even when size+mtime match. Reviewer-reproduced: the
    /// planner queued the file but `write_file_payload`'s defensive
    /// `file_needs_copy_with_checksum_type` second-guess returned
    /// false, dropping the write — `blit copy src/ dst/ --force`
    /// reported 0 B and left the destination untouched.
    #[tokio::test]
    async fn directory_copy_force_overrides_sink_second_guess() {
        let tmp = tempdir().unwrap();
        let src = tmp.path().join("src");
        let dst = tmp.path().join("dst");
        // Two files (>1 so we route through the streaming pipeline,
        // not the tiny fast-path). The fast-path gate (R58-F7
        // followup) also bounces non-default compare_modes to
        // streaming, but having multiple files makes the intent
        // explicit.
        write_file(&src.join("a.txt"), b"NEW!");
        write_file(&src.join("b.txt"), b"NEW!");
        write_file(&dst.join("a.txt"), b"OLD!");
        write_file(&dst.join("b.txt"), b"OLD!");
        let t = filetime::FileTime::from_unix_time(1_700_000_000, 0);
        for name in ["a.txt", "b.txt"] {
            filetime::set_file_mtime(src.join(name), t).unwrap();
            filetime::set_file_mtime(dst.join(name), t).unwrap();
        }

        let mut opts = opts();
        opts.compare_mode = crate::orchestrator::LocalCompareMode::Force;

        let orch = TransferOrchestrator::new();
        let _ = orch
            .execute_local_mirror_async(&src, &dst, opts)
            .await
            .unwrap();

        for name in ["a.txt", "b.txt"] {
            assert_eq!(
                std::fs::read(dst.join(name)).unwrap(),
                b"NEW!",
                "--force must overwrite even when size+mtime match (sink-layer regression)"
            );
        }
    }
}
