//! Local transfers on the unified session (otp-11): behavior pins for
//! `run_local_session`, ported from the retired local orchestration's
//! test surface (`orchestrator.rs` unit pins + `local_transfers.rs`)
//! per `docs/plan/OTP11_LOCAL_SESSION.md` — fast-path TAG pins become
//! behavior pins (the strategy layer is deleted; the session records
//! one `"session"` perf-history tag), everything else pins the same
//! observable contract on the session route.

use blit_core::config;
use blit_core::orchestrator::{
    LocalCompareMode, LocalMirrorDeleteScope, LocalMirrorOptions, TransferOutcome,
};
use blit_core::perf_history;
use blit_core::transfer_session::run_local_session;
use eyre::Result;
use std::fs;
use std::path::PathBuf;
use tempfile::tempdir;
use tokio::sync::Mutex;

/// Tests that touch perf history mutate process-global state (the
/// config-dir override and the history file inside it); they must not
/// interleave. Same discipline as the retired `local_transfers.rs`,
/// with an async-aware mutex because these pins hold it across the
/// session await.
static SERIAL: Mutex<()> = Mutex::const_new(());

struct ConfigDirGuard {
    _temp: tempfile::TempDir,
    prev: Option<PathBuf>,
}

impl ConfigDirGuard {
    fn new() -> Result<Self> {
        let temp = tempdir()?;
        let prev = config::config_dir_override();
        config::set_config_dir(temp.path());
        Ok(Self { _temp: temp, prev })
    }
}

impl Drop for ConfigDirGuard {
    fn drop(&mut self) {
        if let Some(prev) = &self.prev {
            config::set_config_dir(prev);
        } else {
            config::clear_config_dir_override();
        }
    }
}

fn options() -> LocalMirrorOptions {
    LocalMirrorOptions {
        progress: false,
        perf_history: false,
        ..Default::default()
    }
}

// ---------------------------------------------------------------------------
// Ports of local_transfers.rs (fast-path tag pins → behavior pins)
// ---------------------------------------------------------------------------

/// Port of `tiny_manifest_records_fast_path`: a small tree copies
/// whole, and a perf-history row is recorded with the session tag
/// (the `tiny_manifest` strategy died with the engine).
#[tokio::test]
async fn small_tree_copies_and_records_session_history() -> Result<()> {
    let _serial = SERIAL.lock().await;
    let _guard = ConfigDirGuard::new()?;
    perf_history::set_perf_history_enabled(true)?;
    let _ = perf_history::clear_history()?;

    let tmp = tempdir()?;
    let src = tmp.path().join("src");
    let dest = tmp.path().join("dest");
    fs::create_dir_all(&src)?;
    fs::write(src.join("a.txt"), b"one")?;
    fs::write(src.join("b.txt"), b"two")?;
    fs::write(src.join("c.txt"), b"three")?;

    let summary = run_local_session(
        &src,
        &dest,
        LocalMirrorOptions {
            perf_history: true,
            ..options()
        },
    )
    .await?;
    assert_eq!(summary.copied_files, 3);
    assert_eq!(fs::read(dest.join("c.txt"))?, b"three");

    let records = perf_history::read_recent_records(0)?;
    let last = records.last().expect("expected perf history record");
    assert_eq!(last.fast_path.as_deref(), Some("session"));
    assert_eq!(last.file_count, 3);
    Ok(())
}

/// Port of `up_to_date_second_run_records_no_work`: a second run over
/// an already-synced tree copies nothing and reports `UpToDate` with
/// the examined count (the `no_work` journal strategy died with the
/// engine; the session diff produces the same observable outcome).
#[tokio::test]
async fn up_to_date_second_run_copies_nothing() -> Result<()> {
    let tmp = tempdir()?;
    let src = tmp.path().join("src");
    let dest = tmp.path().join("dest");
    fs::create_dir_all(&src)?;
    fs::write(src.join("a.txt"), b"one")?;
    fs::write(src.join("b.txt"), b"two")?;

    let first = run_local_session(&src, &dest, options()).await?;
    assert_eq!(first.copied_files, 2);

    let second = run_local_session(&src, &dest, options()).await?;
    assert_eq!(second.copied_files, 0);
    assert_eq!(second.outcome, TransferOutcome::UpToDate);
    assert!(
        second.scanned_files >= 2,
        "an up-to-date run must report examined files"
    );
    Ok(())
}

/// Port of `empty_source_dir_reports_source_empty`.
#[tokio::test]
async fn empty_source_dir_reports_source_empty() -> Result<()> {
    let tmp = tempdir()?;
    let src = tmp.path().join("src");
    let dest = tmp.path().join("dest");
    fs::create_dir_all(&src)?;

    let summary = run_local_session(&src, &dest, options()).await?;
    assert_eq!(summary.copied_files, 0);
    assert_eq!(summary.outcome, TransferOutcome::SourceEmpty);
    Ok(())
}

/// Port of `single_file_copy_records_history`: a file source root
/// (empty wire relative path) copies to the exact destination path
/// with scanned-feature accounting.
#[tokio::test]
async fn single_file_copy_lands_and_records_history() -> Result<()> {
    let _serial = SERIAL.lock().await;
    let _guard = ConfigDirGuard::new()?;
    perf_history::set_perf_history_enabled(true)?;
    let _ = perf_history::clear_history()?;

    let tmp = tempdir()?;
    let src = tmp.path().join("one.bin");
    let dest = tmp.path().join("dest.bin");
    fs::write(&src, b"payload-bytes")?;

    let summary = run_local_session(
        &src,
        &dest,
        LocalMirrorOptions {
            perf_history: true,
            ..options()
        },
    )
    .await?;
    assert_eq!(summary.copied_files, 1);
    assert_eq!(fs::read(&dest)?, b"payload-bytes");

    let records = perf_history::read_recent_records(0)?;
    let last = records.last().expect("expected perf history record");
    assert_eq!(last.fast_path.as_deref(), Some("session"));
    assert_eq!(last.file_count, 1);
    assert_eq!(last.total_bytes, b"payload-bytes".len() as u64);
    Ok(())
}

/// Port of `cross_batch_boundary_copies_every_file`: a workload
/// spanning multiple destination diff chunks copies every file exactly
/// once across every chunk boundary.
#[tokio::test]
async fn cross_chunk_boundary_copies_every_file() -> Result<()> {
    let tmp = tempdir()?;
    let src = tmp.path().join("src");
    let dest = tmp.path().join("dest");
    fs::create_dir_all(&src)?;
    for idx in 0..600 {
        fs::write(src.join(format!("f{idx}.txt")), format!("payload-{idx}"))?;
    }

    let summary = run_local_session(&src, &dest, options()).await?;
    assert_eq!(summary.copied_files, 600);
    assert_eq!(summary.scanned_files, 600);
    assert_eq!(fs::read(dest.join("f0.txt"))?, b"payload-0");
    assert_eq!(fs::read(dest.join("f511.txt"))?, b"payload-511");
    assert_eq!(fs::read(dest.join("f512.txt"))?, b"payload-512");
    assert_eq!(fs::read(dest.join("f599.txt"))?, b"payload-599");
    Ok(())
}

/// Port of `nested_destination_does_not_self_copy` (ue-r2-1d F1): a
/// destination nested inside the source is excluded from the scan —
/// the second run's walk definitely sees the pre-existing destination
/// directory, so the exclusion is exercised deterministically.
#[tokio::test]
async fn nested_destination_does_not_self_copy() -> Result<()> {
    let tmp = tempdir()?;
    let src = tmp.path().join("src");
    fs::create_dir_all(&src)?;
    for idx in 0..300 {
        fs::write(src.join(format!("f{idx}.txt")), format!("payload-{idx}"))?;
    }
    let dest = src.join("backup");

    let first = run_local_session(&src, &dest, options()).await?;
    assert_eq!(first.copied_files, 300);
    assert!(dest.join("f0.txt").exists());
    assert!(
        !dest.join("backup").exists(),
        "first run must not copy the destination into itself"
    );

    let second = run_local_session(&src, &dest, options()).await?;
    assert!(
        !dest.join("backup").exists(),
        "second run re-walks a tree that now contains the destination; \
         the scan must exclude it (got copied_files={})",
        second.copied_files
    );
    assert_eq!(second.copied_files, 0);
    Ok(())
}

/// Port of `larger_manifest_records_streaming_path`, reduced to its
/// behavior half: a 300-file tree copies whole (the streaming-vs-tiny
/// strategy distinction died with the engine).
#[tokio::test]
async fn larger_manifest_copies_whole() -> Result<()> {
    let tmp = tempdir()?;
    let src = tmp.path().join("src");
    let dest = tmp.path().join("dest");
    fs::create_dir_all(&src)?;
    for idx in 0..300 {
        fs::write(
            src.join(format!("file-{idx}.txt")),
            format!("payload-{idx}"),
        )?;
    }

    let summary = run_local_session(&src, &dest, options()).await?;
    assert_eq!(summary.copied_files, 300);
    Ok(())
}

// ---------------------------------------------------------------------------
// Ports of the orchestrator.rs behavior pins (R44–R58 contract)
// ---------------------------------------------------------------------------

/// Port of `incremental_run_total_bytes_excludes_skipped_files`.
#[tokio::test]
async fn incremental_run_total_bytes_excludes_skipped_files() -> Result<()> {
    let tmp = tempdir()?;
    let src = tmp.path().join("src");
    let dest = tmp.path().join("dest");
    fs::create_dir_all(&src)?;
    fs::write(src.join("keep.txt"), b"unchanged")?;
    fs::write(src.join("grow.txt"), b"v1")?;

    let first = run_local_session(&src, &dest, options()).await?;
    assert_eq!(first.copied_files, 2);

    fs::write(src.join("grow.txt"), b"v2-now-longer")?;
    let second = run_local_session(&src, &dest, options()).await?;
    assert_eq!(second.copied_files, 1);
    assert_eq!(
        second.total_bytes,
        b"v2-now-longer".len() as u64,
        "skipped files must not count toward transferred bytes"
    );
    Ok(())
}

/// Port of `mirror_refuses_when_source_scan_incomplete` (R46-F2): an
/// unreadable source subdir makes the scan incomplete; deleting at the
/// destination could remove files the source still has, so the session
/// refuses the mirror outright.
#[cfg(unix)]
#[tokio::test]
async fn mirror_refuses_when_source_scan_incomplete() -> Result<()> {
    use std::os::unix::fs::PermissionsExt;

    let tmp = tempdir()?;
    let src = tmp.path().join("src");
    let dest = tmp.path().join("dest");
    fs::create_dir_all(src.join("locked"))?;
    fs::write(src.join("a.txt"), b"readable")?;
    fs::write(src.join("locked/hidden.txt"), b"unreadable")?;
    fs::create_dir_all(&dest)?;
    fs::write(dest.join("extraneous.txt"), b"would be deleted")?;
    fs::set_permissions(src.join("locked"), fs::Permissions::from_mode(0o000))?;

    let result = run_local_session(
        &src,
        &dest,
        LocalMirrorOptions {
            mirror: true,
            ..options()
        },
    )
    .await;
    fs::set_permissions(src.join("locked"), fs::Permissions::from_mode(0o755))?;

    let err = result.expect_err("mirror over an incomplete scan must refuse");
    assert!(
        format!("{err:#}").contains("scan did not complete"),
        "unexpected error: {err:#}"
    );
    assert!(
        dest.join("extraneous.txt").exists(),
        "a refused mirror must not have deleted anything"
    );
    Ok(())
}

/// Port of `mirror_delete_failure_propagates_as_error` (R45): a delete
/// the filesystem refuses fails the mirror instead of being swallowed.
#[cfg(unix)]
#[tokio::test]
async fn mirror_delete_failure_propagates_as_error() -> Result<()> {
    use std::os::unix::fs::PermissionsExt;

    let tmp = tempdir()?;
    let src = tmp.path().join("src");
    let dest = tmp.path().join("dest");
    fs::create_dir_all(&src)?;
    fs::write(src.join("a.txt"), b"one")?;
    fs::create_dir_all(dest.join("locked"))?;
    fs::write(dest.join("locked/extraneous.txt"), b"stuck")?;
    fs::set_permissions(dest.join("locked"), fs::Permissions::from_mode(0o555))?;

    let result = run_local_session(
        &src,
        &dest,
        LocalMirrorOptions {
            mirror: true,
            ..options()
        },
    )
    .await;
    fs::set_permissions(dest.join("locked"), fs::Permissions::from_mode(0o755))?;

    assert!(
        result.is_err(),
        "a failed mirror deletion must propagate as an error"
    );
    Ok(())
}

/// Port of `mirror_with_subdir_does_not_treat_parent_dir_as_absent`:
/// a synced subdirectory tree mirrors clean — nothing re-copied,
/// nothing deleted.
#[tokio::test]
async fn mirror_with_subdir_does_not_treat_parent_dir_as_absent() -> Result<()> {
    let tmp = tempdir()?;
    let src = tmp.path().join("src");
    let dest = tmp.path().join("dest");
    fs::create_dir_all(src.join("sub"))?;
    fs::write(src.join("sub/file.txt"), b"content")?;

    let first = run_local_session(
        &src,
        &dest,
        LocalMirrorOptions {
            mirror: true,
            ..options()
        },
    )
    .await?;
    assert_eq!(first.copied_files, 1);

    let second = run_local_session(
        &src,
        &dest,
        LocalMirrorOptions {
            mirror: true,
            ..options()
        },
    )
    .await?;
    assert_eq!(second.copied_files, 0, "synced subdir must not re-copy");
    assert_eq!(second.deleted_files + second.deleted_dirs, 0);
    assert!(dest.join("sub/file.txt").exists());
    Ok(())
}

/// Port of `mirror_still_deletes_truly_unrelated_destination_dirs`,
/// also pinning the otp-11 split delete counters.
#[tokio::test]
async fn mirror_deletes_unrelated_destination_dirs_and_reports_split() -> Result<()> {
    let tmp = tempdir()?;
    let src = tmp.path().join("src");
    let dest = tmp.path().join("dest");
    fs::create_dir_all(&src)?;
    fs::write(src.join("keep.txt"), b"keep")?;
    fs::create_dir_all(dest.join("stale-dir"))?;
    fs::write(dest.join("stale-dir/old.txt"), b"old")?;
    fs::write(dest.join("stale.txt"), b"old")?;

    let summary = run_local_session(
        &src,
        &dest,
        LocalMirrorOptions {
            mirror: true,
            ..options()
        },
    )
    .await?;
    assert!(!dest.join("stale-dir").exists(), "extraneous dir must go");
    assert!(!dest.join("stale.txt").exists(), "extraneous file must go");
    assert!(dest.join("keep.txt").exists());
    assert_eq!(summary.deleted_files, 2, "stale.txt + stale-dir/old.txt");
    assert_eq!(summary.deleted_dirs, 1, "stale-dir itself");
    Ok(())
}

/// Port of `local_dry_run_does_not_create_destination` plus the
/// mirror half: dry-run writes nothing, deletes nothing, and still
/// reports the plan (would-copy and would-delete counts).
#[tokio::test]
async fn dry_run_creates_nothing_and_reports_the_plan() -> Result<()> {
    let tmp = tempdir()?;
    let src = tmp.path().join("src");
    let dest = tmp.path().join("dest");
    fs::create_dir_all(&src)?;
    fs::write(src.join("a.txt"), b"one")?;

    let summary = run_local_session(
        &src,
        &dest,
        LocalMirrorOptions {
            dry_run: true,
            ..options()
        },
    )
    .await?;
    assert!(summary.dry_run);
    assert!(
        !dest.exists(),
        "dry run must not create the destination root"
    );

    // Mirror dry-run: extraneous entries are counted, never deleted.
    fs::create_dir_all(&dest)?;
    fs::write(dest.join("stale.txt"), b"old")?;
    let mirror = run_local_session(
        &src,
        &dest,
        LocalMirrorOptions {
            dry_run: true,
            mirror: true,
            ..options()
        },
    )
    .await?;
    assert!(
        dest.join("stale.txt").exists(),
        "dry-run mirror must not delete"
    );
    assert_eq!(mirror.deleted_files, 1, "the plan still reports the count");
    Ok(())
}

/// Port of `single_file_copy_honors_filter_excludes` (R58-F5).
#[tokio::test]
async fn single_file_copy_honors_filter_excludes() -> Result<()> {
    let tmp = tempdir()?;
    let src = tmp.path().join("skip.log");
    let dest = tmp.path().join("dest.log");
    fs::write(&src, b"filtered out")?;

    let mut opts = options();
    opts.filter.exclude_files = vec!["*.log".to_string()];
    let summary = run_local_session(&src, &dest, opts).await?;
    assert_eq!(summary.copied_files, 0, "excluded file must not copy");
    assert!(!dest.exists());
    Ok(())
}

/// Port of `single_file_copy_honors_ignore_existing`.
#[tokio::test]
async fn single_file_copy_honors_ignore_existing() -> Result<()> {
    let tmp = tempdir()?;
    let src = tmp.path().join("src.bin");
    let dest = tmp.path().join("dest.bin");
    fs::write(&src, b"new content longer")?;
    fs::write(&dest, b"old")?;

    let summary = run_local_session(
        &src,
        &dest,
        LocalMirrorOptions {
            ignore_existing: true,
            ..options()
        },
    )
    .await?;
    assert_eq!(summary.copied_files, 0);
    assert_eq!(fs::read(&dest)?, b"old", "existing dest must be untouched");
    Ok(())
}

/// Port of `single_file_copy_size_only_skips_same_size` +
/// `local_copy_honors_size_only_compare_mode` (R58-F7): same size,
/// different content and mtime — SizeOnly skips.
#[tokio::test]
async fn size_only_skips_same_size_different_content() -> Result<()> {
    let tmp = tempdir()?;
    let src = tmp.path().join("src");
    let dest = tmp.path().join("dest");
    fs::create_dir_all(&src)?;
    fs::create_dir_all(&dest)?;
    fs::write(src.join("f.txt"), b"AAAA")?;
    fs::write(dest.join("f.txt"), b"BBBB")?;

    let summary = run_local_session(
        &src,
        &dest,
        LocalMirrorOptions {
            compare_mode: LocalCompareMode::SizeOnly,
            ..options()
        },
    )
    .await?;
    assert_eq!(summary.copied_files, 0);
    assert_eq!(fs::read(dest.join("f.txt"))?, b"BBBB");
    Ok(())
}

/// Port of `local_copy_honors_force_compare_mode` +
/// `directory_copy_force_overrides_sink_second_guess` (R58-F7/F11):
/// identical trees still copy whole under Force.
#[tokio::test]
async fn force_copies_identical_tree() -> Result<()> {
    let tmp = tempdir()?;
    let src = tmp.path().join("src");
    let dest = tmp.path().join("dest");
    fs::create_dir_all(&src)?;
    fs::write(src.join("same.txt"), b"identical")?;

    let first = run_local_session(&src, &dest, options()).await?;
    assert_eq!(first.copied_files, 1);

    let forced = run_local_session(
        &src,
        &dest,
        LocalMirrorOptions {
            compare_mode: LocalCompareMode::Force,
            ..options()
        },
    )
    .await?;
    assert_eq!(
        forced.copied_files, 1,
        "Force must transfer regardless of target state (sink second-guess included)"
    );
    Ok(())
}

/// Port of `local_mirror_subset_keeps_excluded_only_directories`
/// (R58-F6): under the default FilteredSubset scope, destination
/// entries the filter excludes are out of scope and survive.
#[tokio::test]
async fn mirror_subset_keeps_excluded_destination_entries() -> Result<()> {
    let tmp = tempdir()?;
    let src = tmp.path().join("src");
    let dest = tmp.path().join("dest");
    fs::create_dir_all(&src)?;
    fs::write(src.join("data.txt"), b"data")?;
    fs::create_dir_all(dest.join("logs"))?;
    fs::write(dest.join("logs/app.log"), b"out of scope")?;

    let mut opts = options();
    opts.mirror = true;
    opts.filter.exclude_files = vec!["*.log".to_string()];
    let summary = run_local_session(&src, &dest, opts).await?;
    assert!(
        dest.join("logs/app.log").exists(),
        "filter-excluded dest entries are out of mirror scope (FilteredSubset)"
    );
    assert_eq!(summary.deleted_files, 0);
    Ok(())
}

/// Port of `local_mirror_all_scope_deletes_through_filter` (R58-F6):
/// `--delete-scope all` deletes extraneous entries regardless of the
/// transfer filter.
#[tokio::test]
async fn mirror_all_scope_deletes_through_filter() -> Result<()> {
    let tmp = tempdir()?;
    let src = tmp.path().join("src");
    let dest = tmp.path().join("dest");
    fs::create_dir_all(&src)?;
    fs::write(src.join("data.txt"), b"data")?;
    fs::create_dir_all(dest.join("logs"))?;
    fs::write(dest.join("logs/app.log"), b"deleted under All")?;

    let mut opts = options();
    opts.mirror = true;
    opts.delete_scope = LocalMirrorDeleteScope::All;
    opts.filter.exclude_files = vec!["*.log".to_string()];
    let summary = run_local_session(&src, &dest, opts).await?;
    assert!(
        !dest.join("logs").exists(),
        "All scope deletes extraneous entries the filter would exclude"
    );
    assert!(summary.deleted_files >= 1);
    Ok(())
}

// ---------------------------------------------------------------------------
// New otp-11 pins (session-local route specifics)
// ---------------------------------------------------------------------------

/// `--null` diagnostics sink: the pipeline runs whole (reads, plans,
/// counts) but the destination is never touched.
#[tokio::test]
async fn null_sink_counts_but_writes_nothing() -> Result<()> {
    let tmp = tempdir()?;
    let src = tmp.path().join("src");
    let dest = tmp.path().join("dest");
    fs::create_dir_all(&src)?;
    fs::write(src.join("a.txt"), b"one")?;
    fs::write(src.join("b.txt"), b"two")?;

    let summary = run_local_session(
        &src,
        &dest,
        LocalMirrorOptions {
            null_sink: true,
            ..options()
        },
    )
    .await?;
    assert_eq!(summary.copied_files, 2);
    assert!(
        !dest.exists(),
        "the null sink must never create the destination"
    );
    Ok(())
}

/// The old engine's journal fast path silently skipped DEEP
/// modifications: its macOS/Linux `NoChanges` verdict decayed to
/// ROOT-dir mtime equality, which a write to `src/sub/deep.txt` never
/// touches — reproduced against the pre-otp-11 binary 2026-07-12
/// ("Up to date" while src/dest differed; transcript in
/// `docs/bench/otp11-local-2026-07-11/README.md`). The session route
/// diffs every run: a deep change after warm repeated runs MUST land.
#[tokio::test]
async fn deep_modification_after_warm_runs_syncs() -> Result<()> {
    let tmp = tempdir()?;
    let src = tmp.path().join("src");
    let dest = tmp.path().join("dest");
    fs::create_dir_all(src.join("sub"))?;
    fs::write(src.join("sub/deep.txt"), b"v1")?;
    fs::write(src.join("top.txt"), b"top")?;

    let opts = || LocalMirrorOptions {
        mirror: true,
        ..options()
    };
    for _ in 0..3 {
        run_local_session(&src, &dest, opts()).await?;
    }

    // A deep content write leaves the root dir's mtime untouched —
    // the exact shape the old fast path lost. Different length so the
    // diff verdict is deterministic within one mtime second.
    fs::write(src.join("sub/deep.txt"), b"v2-now-longer")?;
    let after = run_local_session(&src, &dest, opts()).await?;
    assert_eq!(after.copied_files, 1, "the deep change must transfer");
    assert_eq!(fs::read(dest.join("sub/deep.txt"))?, b"v2-now-longer");
    Ok(())
}

/// Local `--resume` rides the carrier's block phase — the shared
/// `resume_copy_file` primitive (design doc D2, codex design F5
/// adjudication): a stale partial at the destination is completed
/// byte-identical.
#[tokio::test]
async fn resume_completes_stale_partial_byte_identical() -> Result<()> {
    let tmp = tempdir()?;
    let src = tmp.path().join("src");
    let dest = tmp.path().join("dest");
    fs::create_dir_all(&src)?;
    fs::create_dir_all(&dest)?;
    let payload: Vec<u8> = (0u8..=255).cycle().take(4 * 1024 * 1024).collect();
    fs::write(src.join("big.bin"), &payload)?;
    // Stale partial: first half only, first byte drifted.
    let mut partial = payload[..2 * 1024 * 1024].to_vec();
    partial[0] = !partial[0];
    fs::write(dest.join("big.bin"), &partial)?;

    let summary = run_local_session(
        &src,
        &dest,
        LocalMirrorOptions {
            resume: true,
            ..options()
        },
    )
    .await?;
    assert_eq!(summary.copied_files, 1);
    assert_eq!(fs::read(dest.join("big.bin"))?, payload);
    Ok(())
}

/// An unreadable source file is skipped (readable siblings land) and
/// recorded in `unreadable_paths` — the summary signal `blit move`'s
/// caller-side source-delete gate (R47-F4) relies on.
#[cfg(unix)]
#[tokio::test]
async fn unreadable_source_file_lands_in_summary_and_copy_continues() -> Result<()> {
    use std::os::unix::fs::PermissionsExt;

    let tmp = tempdir()?;
    let src = tmp.path().join("src");
    let dest = tmp.path().join("dest");
    fs::create_dir_all(&src)?;
    fs::write(src.join("ok.txt"), b"readable")?;
    fs::write(src.join("locked.txt"), b"unreadable")?;
    fs::set_permissions(src.join("locked.txt"), fs::Permissions::from_mode(0o000))?;

    let result = run_local_session(&src, &dest, options()).await;
    fs::set_permissions(src.join("locked.txt"), fs::Permissions::from_mode(0o644))?;
    let summary = result?;

    assert_eq!(fs::read(dest.join("ok.txt"))?, b"readable");
    assert!(
        !summary.unreadable_paths.is_empty(),
        "the unreadable file must be recorded for the move gate"
    );
    Ok(())
}
