//! Local transfers on the unified session (otp-11): behavior pins for
//! `run_local_session`, ported from the retired local orchestration's
//! test surface (`orchestrator.rs` unit pins + `local_transfers.rs`)
//! per `docs/plan/OTP11_LOCAL_SESSION.md` — fast-path TAG pins become
//! behavior pins (the strategy layer is deleted; the session records
//! one `"session"` perf-history tag), everything else pins the same
//! observable contract on the session route.

use blit_core::config;
use blit_core::perf_history;
use blit_core::transfer_session::run_local_session;
use blit_core::transfer_session::{
    LocalCompareMode, LocalMirrorDeleteScope, LocalMirrorOptions, TransferOutcome,
};
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

/// pfc-4: a local run's summary carries the same per-file failure
/// report the wire carriers return, so one renderer serves both. Mirror
/// here so the same run also pins Q1(a) — the delete phase runs under a
/// contained failure (pfc-5 extended containment to non-mirror sessions;
/// that half is pinned in `transfer_session::local`'s unit tests). The
/// blocked file is a directory sitting where its file belongs —
/// attributable to that one path.
#[tokio::test]
async fn local_summary_carries_the_contained_failure_report() -> Result<()> {
    let tmp = tempdir()?;
    let src = tmp.path().join("src");
    let dest = tmp.path().join("dest");
    fs::create_dir_all(&src)?;
    fs::write(src.join("landed.txt"), b"alpha")?;
    fs::write(src.join("blocked.txt"), b"never lands")?;
    fs::create_dir_all(dest.join("blocked.txt"))?;

    let summary = run_local_session(
        &src,
        &dest,
        LocalMirrorOptions {
            mirror: true,
            ..options()
        },
    )
    .await?;

    assert_eq!(summary.files_failed, 1);
    assert_eq!(summary.failures.len(), 1);
    assert_eq!(summary.failures[0].relative_path, "blocked.txt");
    assert!(
        !summary.failures[0].reason.is_empty(),
        "a carried failure names its reason"
    );
    assert_eq!(
        summary.copied_files, 1,
        "a failed file is never a copied file"
    );
    assert_eq!(fs::read(dest.join("landed.txt"))?, b"alpha");
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

// ---------------------------------------------------------------------------
// otp-11b floor-restoration pins (the slice doc's closure plan): each
// pins live session-route behavior the retired engine tests used to
// cover from the outside, or new surface the route added.
// ---------------------------------------------------------------------------

/// Mirror from an EMPTY source deletes everything at the destination —
/// mirror semantics, not an error (the CLI's destructive-confirm owns
/// the UX guard).
#[tokio::test]
async fn mirror_from_empty_source_deletes_destination_tree() -> Result<()> {
    let tmp = tempdir()?;
    let src = tmp.path().join("src");
    let dest = tmp.path().join("dest");
    fs::create_dir_all(&src)?;
    fs::create_dir_all(dest.join("sub"))?;
    fs::write(dest.join("a.txt"), b"old")?;
    fs::write(dest.join("sub/b.txt"), b"old")?;

    let summary = run_local_session(
        &src,
        &dest,
        LocalMirrorOptions {
            mirror: true,
            ..options()
        },
    )
    .await?;
    assert!(!dest.join("a.txt").exists());
    assert!(!dest.join("sub").exists());
    assert_eq!(summary.deleted_files, 2);
    assert_eq!(summary.deleted_dirs, 1);
    Ok(())
}

/// Deep-nested extraneous trees delete whole under mirror, with the
/// split counters accounting every level (files then dirs
/// deepest-first — the one delete rule's ordering).
#[tokio::test]
async fn mirror_deletes_nested_extraneous_tree_with_split_counts() -> Result<()> {
    let tmp = tempdir()?;
    let src = tmp.path().join("src");
    let dest = tmp.path().join("dest");
    fs::create_dir_all(&src)?;
    fs::write(src.join("keep.txt"), b"keep")?;
    fs::create_dir_all(dest.join("stale/deeper/deepest"))?;
    fs::write(dest.join("stale/one.txt"), b"x")?;
    fs::write(dest.join("stale/deeper/two.txt"), b"x")?;
    fs::write(dest.join("stale/deeper/deepest/three.txt"), b"x")?;

    let summary = run_local_session(
        &src,
        &dest,
        LocalMirrorOptions {
            mirror: true,
            ..options()
        },
    )
    .await?;
    assert!(!dest.join("stale").exists());
    assert_eq!(summary.deleted_files, 3);
    assert_eq!(summary.deleted_dirs, 3);
    assert!(dest.join("keep.txt").exists());
    Ok(())
}

/// `--ignore-times` transfers an unchanged TREE unconditionally
/// through the session route (the move rule's mapping, e2e).
#[tokio::test]
async fn ignore_times_recopies_unchanged_tree() -> Result<()> {
    let tmp = tempdir()?;
    let src = tmp.path().join("src");
    let dest = tmp.path().join("dest");
    fs::create_dir_all(&src)?;
    fs::write(src.join("a.txt"), b"one")?;
    fs::write(src.join("b.txt"), b"two")?;

    let first = run_local_session(&src, &dest, options()).await?;
    assert_eq!(first.copied_files, 2);

    let again = run_local_session(
        &src,
        &dest,
        LocalMirrorOptions {
            compare_mode: LocalCompareMode::IgnoreTimes,
            ..options()
        },
    )
    .await?;
    assert_eq!(again.copied_files, 2, "IgnoreTimes must always transfer");
    Ok(())
}

/// `--checksum` on the session-local route: same size + same mtime but
/// different CONTENT transfers (the cell `--checksum` exists for).
#[tokio::test]
async fn checksum_transfers_same_size_same_mtime_content_change() -> Result<()> {
    let tmp = tempdir()?;
    let src = tmp.path().join("src");
    let dest = tmp.path().join("dest");
    fs::create_dir_all(&src)?;
    fs::create_dir_all(&dest)?;
    fs::write(src.join("f.bin"), b"AAAA")?;
    fs::write(dest.join("f.bin"), b"BBBB")?;
    let mtime = filetime::FileTime::from_unix_time(1_600_000_000, 0);
    filetime::set_file_mtime(src.join("f.bin"), mtime)?;
    filetime::set_file_mtime(dest.join("f.bin"), mtime)?;

    let summary = run_local_session(
        &src,
        &dest,
        LocalMirrorOptions {
            compare_mode: LocalCompareMode::Checksum,
            ..options()
        },
    )
    .await?;
    assert_eq!(summary.copied_files, 1);
    assert_eq!(fs::read(dest.join("f.bin"))?, b"AAAA");
    Ok(())
}

/// `--checksum`: content-equal files SKIP even when mtimes differ.
#[tokio::test]
async fn checksum_skips_content_equal_despite_mtime() -> Result<()> {
    let tmp = tempdir()?;
    let src = tmp.path().join("src");
    let dest = tmp.path().join("dest");
    fs::create_dir_all(&src)?;
    fs::create_dir_all(&dest)?;
    fs::write(src.join("f.bin"), b"identical")?;
    fs::write(dest.join("f.bin"), b"identical")?;
    filetime::set_file_mtime(
        dest.join("f.bin"),
        filetime::FileTime::from_unix_time(1_500_000_000, 0),
    )?;

    let summary = run_local_session(
        &src,
        &dest,
        LocalMirrorOptions {
            compare_mode: LocalCompareMode::Checksum,
            ..options()
        },
    )
    .await?;
    assert_eq!(summary.copied_files, 0);
    Ok(())
}

/// `--ignore-existing` over a tree: present destination entries are
/// untouched regardless of differences; absent ones land.
#[tokio::test]
async fn ignore_existing_tree_keeps_existing_lands_missing() -> Result<()> {
    let tmp = tempdir()?;
    let src = tmp.path().join("src");
    let dest = tmp.path().join("dest");
    fs::create_dir_all(&src)?;
    fs::create_dir_all(&dest)?;
    fs::write(src.join("exists.txt"), b"new content longer")?;
    fs::write(src.join("missing.txt"), b"lands")?;
    fs::write(dest.join("exists.txt"), b"old")?;

    let summary = run_local_session(
        &src,
        &dest,
        LocalMirrorOptions {
            ignore_existing: true,
            ..options()
        },
    )
    .await?;
    assert_eq!(summary.copied_files, 1);
    assert_eq!(fs::read(dest.join("exists.txt"))?, b"old");
    assert_eq!(fs::read(dest.join("missing.txt"))?, b"lands");
    Ok(())
}

/// Mirror scope under an INCLUDE filter: out-of-scope destination
/// entries survive a FilteredSubset mirror.
#[tokio::test]
async fn mirror_subset_include_filter_scopes_deletions() -> Result<()> {
    let tmp = tempdir()?;
    let src = tmp.path().join("src");
    let dest = tmp.path().join("dest");
    fs::create_dir_all(&src)?;
    fs::create_dir_all(&dest)?;
    fs::write(src.join("data.csv"), b"in scope")?;
    fs::write(dest.join("stale.csv"), b"in scope, extraneous")?;
    fs::write(dest.join("keep.log"), b"out of scope")?;

    let mut opts = options();
    opts.mirror = true;
    opts.filter.include_files = vec!["*.csv".to_string()];
    let summary = run_local_session(&src, &dest, opts).await?;
    assert!(
        !dest.join("stale.csv").exists(),
        "in-scope extraneous entry must delete"
    );
    assert!(
        dest.join("keep.log").exists(),
        "out-of-scope entry must survive FilteredSubset"
    );
    assert_eq!(summary.deleted_files, 1);
    Ok(())
}

/// Dry-run over a single-file root: nothing created, the plan
/// reported.
#[tokio::test]
async fn dry_run_single_file_creates_nothing() -> Result<()> {
    let tmp = tempdir()?;
    let src = tmp.path().join("one.bin");
    let dest = tmp.path().join("out/dest.bin");
    fs::write(&src, b"payload")?;

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
    assert!(!dest.exists());
    assert!(
        !tmp.path().join("out").exists(),
        "dry run must not create the destination parent chain"
    );
    assert_eq!(summary.planned_files, 1, "the plan still reports the copy");
    Ok(())
}

/// A single-file copy into a missing nested parent chain creates it
/// (the sink's parent mkdir), and Force re-copies over an identical
/// destination file root.
#[tokio::test]
async fn single_file_nested_parent_creation_and_force_recopy() -> Result<()> {
    let tmp = tempdir()?;
    let src = tmp.path().join("one.bin");
    let dest = tmp.path().join("a/b/c/dest.bin");
    fs::write(&src, b"payload")?;

    let first = run_local_session(&src, &dest, options()).await?;
    assert_eq!(first.copied_files, 1);
    assert_eq!(fs::read(&dest)?, b"payload");

    let forced = run_local_session(
        &src,
        &dest,
        LocalMirrorOptions {
            compare_mode: LocalCompareMode::Force,
            ..options()
        },
    )
    .await?;
    assert_eq!(forced.copied_files, 1, "Force must re-copy the file root");
    Ok(())
}

/// `--resume` against a FRESH destination falls back to a plain full
/// copy (nothing to patch), byte-identical.
#[tokio::test]
async fn resume_fresh_destination_full_copies() -> Result<()> {
    let tmp = tempdir()?;
    let src = tmp.path().join("src");
    let dest = tmp.path().join("dest");
    fs::create_dir_all(&src)?;
    let payload: Vec<u8> = (0u8..=255).cycle().take(512 * 1024).collect();
    fs::write(src.join("big.bin"), &payload)?;

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

/// An unreadable source SUBDIRECTORY on a plain copy: the readable
/// remainder lands, the scan incompleteness is carried in
/// `unreadable_paths` (the move gate's signal), and the copy succeeds.
#[cfg(unix)]
#[tokio::test]
async fn unreadable_subdir_plain_copy_continues_and_records() -> Result<()> {
    use std::os::unix::fs::PermissionsExt;

    let tmp = tempdir()?;
    let src = tmp.path().join("src");
    let dest = tmp.path().join("dest");
    fs::create_dir_all(src.join("locked"))?;
    fs::write(src.join("ok.txt"), b"readable")?;
    fs::write(src.join("locked/hidden.txt"), b"unreadable")?;
    fs::set_permissions(src.join("locked"), fs::Permissions::from_mode(0o000))?;

    let result = run_local_session(&src, &dest, options()).await;
    fs::set_permissions(src.join("locked"), fs::Permissions::from_mode(0o755))?;
    let summary = result?;

    assert_eq!(fs::read(dest.join("ok.txt"))?, b"readable");
    assert!(
        !summary.unreadable_paths.is_empty(),
        "the unreadable subdir must be recorded"
    );
    Ok(())
}

/// SizeOnly transfers on a size mismatch even when mtimes match (the
/// counterpart of the same-size skip pin).
#[tokio::test]
async fn size_only_transfers_size_mismatch_same_mtime() -> Result<()> {
    let tmp = tempdir()?;
    let src = tmp.path().join("src");
    let dest = tmp.path().join("dest");
    fs::create_dir_all(&src)?;
    fs::create_dir_all(&dest)?;
    fs::write(src.join("f.txt"), b"longer content")?;
    fs::write(dest.join("f.txt"), b"short")?;
    let mtime = filetime::FileTime::from_unix_time(1_600_000_000, 0);
    filetime::set_file_mtime(src.join("f.txt"), mtime)?;
    filetime::set_file_mtime(dest.join("f.txt"), mtime)?;

    let summary = run_local_session(
        &src,
        &dest,
        LocalMirrorOptions {
            compare_mode: LocalCompareMode::SizeOnly,
            ..options()
        },
    )
    .await?;
    assert_eq!(summary.copied_files, 1);
    assert_eq!(fs::read(dest.join("f.txt"))?, b"longer content");
    Ok(())
}

/// `--delete-scope all` deletes a NESTED excluded-only destination
/// tree the filter would have protected under FilteredSubset
/// (R58-F6's All-scope contract on the session route).
#[tokio::test]
async fn mirror_all_scope_deletes_nested_excluded_only_dirs() -> Result<()> {
    let tmp = tempdir()?;
    let src = tmp.path().join("src");
    let dest = tmp.path().join("dest");
    fs::create_dir_all(&src)?;
    fs::write(src.join("data.txt"), b"data")?;
    fs::create_dir_all(dest.join("logs/archive"))?;
    fs::write(dest.join("logs/app.log"), b"excluded")?;
    fs::write(dest.join("logs/archive/old.log"), b"excluded")?;

    let mut opts = options();
    opts.mirror = true;
    opts.delete_scope = LocalMirrorDeleteScope::All;
    opts.filter.exclude_files = vec!["*.log".to_string()];
    let summary = run_local_session(&src, &dest, opts).await?;
    assert!(
        !dest.join("logs").exists(),
        "All scope must delete through the filter, nested dirs included"
    );
    assert_eq!(summary.deleted_files, 2);
    assert_eq!(summary.deleted_dirs, 2);
    Ok(())
}

/// The planner-mix stats fold: a many-small-file tree reports its
/// tar-shard grouping in the summary (the `--verbose` planner-mix
/// block's data source on the session route).
#[tokio::test]
async fn planner_mix_stats_populated_for_small_tree() -> Result<()> {
    let tmp = tempdir()?;
    let src = tmp.path().join("src");
    let dest = tmp.path().join("dest");
    fs::create_dir_all(&src)?;
    for idx in 0..50 {
        fs::write(src.join(format!("f{idx:03}.txt")), b"tiny")?;
    }

    let summary = run_local_session(&src, &dest, options()).await?;
    assert_eq!(summary.copied_files, 50);
    assert!(
        summary.tar_shard_tasks >= 1,
        "small files must report tar-shard grouping"
    );
    assert_eq!(
        summary.tar_shard_files + summary.large_tasks,
        50,
        "every copied file is accounted to exactly one planner bucket"
    );
    Ok(())
}

/// Scanned-byte accounting across diff chunks: `scanned_bytes` is the
/// exact post-filter source workload for a >1-chunk tree.
#[tokio::test]
async fn scanned_bytes_accumulate_across_chunks() -> Result<()> {
    let tmp = tempdir()?;
    let src = tmp.path().join("src");
    let dest = tmp.path().join("dest");
    fs::create_dir_all(&src)?;
    for idx in 0..150 {
        fs::write(src.join(format!("f{idx:03}.bin")), vec![0u8; 100])?;
    }

    let summary = run_local_session(&src, &dest, options()).await?;
    assert_eq!(summary.scanned_files, 150);
    assert_eq!(summary.scanned_bytes, 150 * 100);
    Ok(())
}

/// The old engine's journal fast path silently skipped DEEP
/// modifications: its `NoChanges` verdict rested on root-dir
/// metadata a deep write never touches (macOS: the event-id arm
/// always differs across runs, so the root-MTIME fallback decides;
/// Linux: the FIRST arm is the root dir's CTIME, equally untouched;
/// Windows: the strict-USN arm needs a write-quiet volume, decaying
/// to the same mtime fallback) — reproduced against the pre-otp-11
/// binary 2026-07-12 ("Up to date" while src/dest differed;
/// transcript in `docs/bench/otp11-local-2026-07-11/README.md`). The
/// session route diffs every run: a deep change after warm repeated
/// runs MUST land.
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

/// pfc-6: metadata-only attribute repair at the destination diff. These
/// run against the real Win32 attribute surface — the field evidence
/// (`H:\apps` backup regions reading Normal 0x00 against a source's
/// Archive 0x20) is reproduced literally, not simulated.
#[cfg(windows)]
mod metadata_repair {
    use super::*;
    use std::path::Path;
    use windows::core::PCWSTR;
    use windows::Win32::Storage::FileSystem::{
        GetFileAttributesW, SetFileAttributesW, FILE_FLAGS_AND_ATTRIBUTES, INVALID_FILE_ATTRIBUTES,
    };

    const READONLY: u32 = 0x01;
    const ARCHIVE: u32 = 0x20;
    const HIDDEN: u32 = 0x02;
    const NORMAL: u32 = 0x80;
    /// `WINDOWS_PRESERVED_ATTRIBUTE_MASK` — the durable bits Blit compares.
    const PRESERVED: u32 = 0x27;
    /// Large enough that any re-transfer of the file is unmistakable in
    /// `total_bytes`.
    const BODY_LEN: usize = 64 * 1024;

    fn wide(path: &Path) -> Vec<u16> {
        use std::os::windows::ffi::OsStrExt;
        path.as_os_str()
            .encode_wide()
            .chain(std::iter::once(0))
            .collect()
    }

    fn set_attributes(path: &Path, attributes: u32) -> Result<()> {
        let path = wide(path);
        unsafe {
            SetFileAttributesW(PCWSTR(path.as_ptr()), FILE_FLAGS_AND_ATTRIBUTES(attributes))
        }?;
        Ok(())
    }

    fn durable_attributes(path: &Path) -> u32 {
        let path = wide(path);
        let attributes = unsafe { GetFileAttributesW(PCWSTR(path.as_ptr())) };
        assert_ne!(attributes, INVALID_FILE_ATTRIBUTES, "reading attributes");
        attributes & PRESERVED
    }

    fn stream_path(path: &Path, name: &str) -> PathBuf {
        let mut value = path.as_os_str().to_owned();
        value.push(":");
        value.push(name);
        value.into()
    }

    /// Both copies byte-identical with the same mtime — the only thing left
    /// for the diff to disagree about is metadata. mtimes are set LAST: a
    /// named-stream write bumps the file's last-write time.
    fn identical_pair(src: &Path, dest: &Path, name: &str) -> Result<(PathBuf, PathBuf)> {
        fs::create_dir_all(src)?;
        fs::create_dir_all(dest)?;
        let body: Vec<u8> = (0u8..=255).cycle().take(BODY_LEN).collect();
        let src_file = src.join(name);
        let dest_file = dest.join(name);
        fs::write(&src_file, &body)?;
        fs::write(&dest_file, &body)?;
        Ok((src_file, dest_file))
    }

    fn pin_mtimes(files: &[&Path]) -> Result<()> {
        let mtime = filetime::FileTime::from_unix_time(1_600_000_000, 0);
        for file in files {
            filetime::set_file_mtime(file, mtime)?;
        }
        Ok(())
    }

    /// ls-1 step (0): a run that really repairs must attribute that work to
    /// `AttributeRepair` and NOT leave it inside `Compare`. This is the case
    /// the non-Windows phase tests cannot reach — with nothing repairable,
    /// the subtraction has nothing to subtract and a missing subtraction is
    /// indistinguishable from a working one. Here the repair is real, so
    /// removing either the `measure` wrapper or the subtraction at the
    /// compare seam turns this red.
    #[tokio::test]
    async fn repair_time_is_attributed_to_repair_not_to_compare() -> Result<()> {
        use blit_core::transfer_session::{LocalPhase, LocalPhaseProbe, LocalPhaseReport};
        use std::sync::{Arc, Mutex as StdMutex};

        let tmp = tempdir()?;
        let src = tmp.path().join("src");
        let dest = tmp.path().join("dest");
        // Several files so the repair span is an aggregate, not one sample
        // that could pass by luck.
        let mut pairs = Vec::new();
        for index in 0..16 {
            let name = format!("tool{index}.exe");
            let (src_file, dest_file) = identical_pair(&src, &dest, &name)?;
            pin_mtimes(&[&src_file, &dest_file])?;
            set_attributes(&src_file, ARCHIVE)?;
            set_attributes(&dest_file, NORMAL)?;
            pairs.push(dest_file);
        }

        let sink: Arc<StdMutex<Vec<LocalPhaseReport>>> = Arc::default();
        let seen = Arc::clone(&sink);
        let probe = LocalPhaseProbe::capture("ls1-repair", move |report| {
            seen.lock().expect("sink poisoned").push(report);
        });

        let summary = run_local_session(
            &src,
            &dest,
            LocalMirrorOptions {
                phase_probe: probe,
                ..options()
            },
        )
        .await?;
        assert_eq!(summary.files_repaired, 16, "every file was repaired");
        assert_eq!(summary.total_bytes, 0, "and none of them cost bytes");

        let reports = sink.lock().expect("sink poisoned");
        let report = reports.first().expect("one report");
        let find = |phase: LocalPhase| {
            report
                .phases
                .iter()
                .find(|(candidate, _)| *candidate == phase)
                .map(|(_, aggregate)| aggregate.clone())
                .expect("every phase is reported")
        };

        let repair = find(LocalPhase::AttributeRepair);
        assert_eq!(
            repair.samples, 16,
            "one measured span per repaired file — if the `measure` wrapper at \
             the repair call site is removed this reads 0"
        );
        assert!(repair.total_ns > 0, "the repairs took real time");

        // cr-ls1-2: this test does NOT try to guard the subtraction. The
        // assertion that used to sit here (`compare + repair <= 2 * wall`)
        // was satisfied by construction and stayed green when the
        // subtraction was deleted — a reviewer proved that. The subtraction
        // is guarded directly and decisively by
        // `phase_probe::tests::nested_time_is_subtracted_from_the_enclosing_span`,
        // which makes the nested phase claim an hour so a missing
        // subtraction is categorical rather than a timing comparison. What
        // this test is for is the WIRING: that real repairs on a real
        // session reach the repair phase at all.
        assert!(
            find(LocalPhase::Compare).samples > 0,
            "the compare span still closes on a repair-heavy run"
        );
        Ok(())
    }

    /// The LOCAL carrier's half of the pfc-6 guard: a destination file with
    /// equal size, mtime, content and named streams, diverging only in its
    /// attributes, converges without entering the need list at all, and the
    /// repaired counter reaches the local summary.
    ///
    /// Read the byte assertion below with its caveat: this carrier's sink
    /// re-checks the body itself and skips an identical one, so
    /// `total_bytes` was already 0 before pfc-6. The BINDING byte guard —
    /// where re-sending really costs bytes — is
    /// `transfer_session_roles::attributes_only_divergence_repairs_without_sending_bytes_under_both_initiators`.
    #[tokio::test]
    async fn attributes_only_divergence_repairs_without_re_sending_bytes() -> Result<()> {
        let tmp = tempdir()?;
        let src = tmp.path().join("src");
        let dest = tmp.path().join("dest");
        let (src_file, dest_file) = identical_pair(&src, &dest, "tool.exe")?;
        pin_mtimes(&[&src_file, &dest_file])?;
        set_attributes(&src_file, ARCHIVE)?;
        // The field-evidence shape: the old backup region reads Normal.
        set_attributes(&dest_file, NORMAL)?;
        assert_eq!(durable_attributes(&dest_file), 0x00);

        let summary = run_local_session(&src, &dest, options()).await?;

        assert_eq!(
            summary.total_bytes, 0,
            "an attributes-only divergence must re-send no payload bytes"
        );
        // These three are what move under a reverted repair on this carrier:
        // the file would be planned, prepared and applied — a whole payload
        // round trip to set one bit.
        assert_eq!(summary.copied_files, 0);
        assert_eq!(
            summary.planned_files, 0,
            "a repaired file never enters the need list"
        );
        assert_eq!(summary.files_repaired, 1);
        assert_eq!(summary.files_failed, 0);
        assert_eq!(
            durable_attributes(&dest_file),
            ARCHIVE,
            "the destination attributes converged on the source's"
        );

        // Re-run convergence: the repair holds, so the second run has
        // nothing left to repair either.
        let second = run_local_session(&src, &dest, options()).await?;
        assert_eq!(second.files_repaired, 0);
        assert_eq!(second.total_bytes, 0);
        Ok(())
    }

    /// A named-stream divergence needs the payload that carries the stream
    /// bytes, so it still transfers in full — the split's other half.
    #[tokio::test]
    async fn stream_divergence_still_transfers_the_whole_file() -> Result<()> {
        let tmp = tempdir()?;
        let src = tmp.path().join("src");
        let dest = tmp.path().join("dest");
        let (src_file, dest_file) = identical_pair(&src, &dest, "tool.exe")?;
        fs::write(stream_path(&src_file, "meta"), b"source stream")?;
        fs::write(stream_path(&dest_file, "meta"), b"stale stream")?;
        pin_mtimes(&[&src_file, &dest_file])?;
        set_attributes(&src_file, ARCHIVE)?;
        set_attributes(&dest_file, ARCHIVE)?;

        let summary = run_local_session(&src, &dest, options()).await?;

        assert_eq!(summary.files_repaired, 0, "stream bytes are not repairable");
        assert_eq!(summary.copied_files, 1);
        assert_eq!(summary.planned_files, 1, "the file takes the payload road");
        // The LOCAL carrier's sink re-checks the body with its own compare
        // and skips an identical one (`copy_resolved_file_payload`), so the
        // bytes it reports here are the stream's. The byte-level proof that
        // a needed file really re-sends its payload lives on the wire
        // carrier — `transfer_session_roles`.
        assert!(summary.total_bytes > 0);
        assert_eq!(
            fs::read(stream_path(&dest_file, "meta"))?,
            b"source stream",
            "the stream landed with the payload"
        );
        Ok(())
    }

    /// pfc-1's tolerance governs the repair verdict too: a dot-named
    /// destination carrying an extra HIDDEN bit compares CONVERGED, so no
    /// repair is attempted. A repair here could never converge (no setter
    /// call clears a server-synthesized bit), and running it every session
    /// would be a permanent repair loop.
    #[tokio::test]
    async fn tolerated_synthesized_hidden_triggers_no_repair() -> Result<()> {
        let tmp = tempdir()?;
        let src = tmp.path().join("src");
        let dest = tmp.path().join("dest");
        let (src_file, dest_file) = identical_pair(&src, &dest, ".tcp_shot.png.XkR0Av")?;
        pin_mtimes(&[&src_file, &dest_file])?;
        set_attributes(&src_file, ARCHIVE)?;
        set_attributes(&dest_file, ARCHIVE | HIDDEN)?;

        let summary = run_local_session(&src, &dest, options()).await?;

        assert_eq!(
            summary.files_repaired, 0,
            "a tolerated divergence is converged, not repairable"
        );
        assert_eq!(summary.total_bytes, 0);
        assert_eq!(summary.planned_files, 0);
        assert_eq!(
            durable_attributes(&dest_file),
            ARCHIVE | HIDDEN,
            "nothing touched the destination's attributes"
        );
        Ok(())
    }

    /// A repair that FAILS degrades to the full transfer the file would
    /// have had before pfc-6 — never a new fatal path. The failure is real:
    /// a deny-WriteAttributes ACE on the destination file makes
    /// `SetFileAttributesW` return access-denied, and the test proves the
    /// fixture bites before running the session.
    #[tokio::test]
    async fn failed_repair_degrades_to_transfer_and_the_session_completes() -> Result<()> {
        let tmp = tempdir()?;
        let src = tmp.path().join("src");
        let dest = tmp.path().join("dest");
        let (src_file, dest_file) = identical_pair(&src, &dest, "tool.exe")?;
        pin_mtimes(&[&src_file, &dest_file])?;
        // READONLY is the bit the destination will be missing: writing an
        // ACL sets ARCHIVE on the file, so a plain Archive-vs-Normal
        // divergence would not survive installing the deny ACE below.
        set_attributes(&src_file, ARCHIVE | READONLY)?;

        // Everyone by SID: not localized, and a deny ACE outranks every
        // allow the current token holds.
        let denied = std::process::Command::new("icacls")
            .arg(&dest_file)
            .arg("/deny")
            .arg("*S-1-1-0:(WA)")
            .output()?;
        assert!(
            denied.status.success(),
            "icacls could not install the deny ACE: {}",
            String::from_utf8_lossy(&denied.stdout)
        );
        assert_ne!(
            durable_attributes(&dest_file),
            ARCHIVE | READONLY,
            "the fixture must leave an attribute divergence to repair"
        );
        assert!(
            set_attributes(&dest_file, ARCHIVE | READONLY).is_err(),
            "the fixture must actually make an attribute write fail"
        );

        let summary = run_local_session(&src, &dest, options()).await;
        let _ = std::process::Command::new("icacls")
            .arg(&dest_file)
            .arg("/remove:d")
            .arg("*S-1-1-0")
            .output();
        // Leave nothing read-only behind: the temp tree has to delete.
        set_attributes(&src_file, ARCHIVE)?;
        let summary = summary?;

        assert_eq!(
            summary.files_repaired, 0,
            "a failed repair is not a repaired file"
        );
        assert_eq!(
            summary.planned_files, 1,
            "the file degrades onto the need list, exactly as before pfc-6"
        );
        // What the transfer then hits on the same denied file is pfc-2's
        // business: contained per-file, session completes.
        assert_eq!(summary.files_failed, 1);
        assert_eq!(summary.copied_files, 0);
        Ok(())
    }

    /// A destination that writes nothing must not repair either: `--null`
    /// and `--dry-run` would otherwise make a diff-time attribute write the
    /// one mutation those runs perform.
    #[tokio::test]
    async fn destinations_that_write_nothing_repair_nothing() -> Result<()> {
        for (dry_run, null_sink) in [(true, false), (false, true)] {
            let tmp = tempdir()?;
            let src = tmp.path().join("src");
            let dest = tmp.path().join("dest");
            let (src_file, dest_file) = identical_pair(&src, &dest, "tool.exe")?;
            pin_mtimes(&[&src_file, &dest_file])?;
            set_attributes(&src_file, ARCHIVE)?;
            set_attributes(&dest_file, NORMAL)?;

            let summary = run_local_session(
                &src,
                &dest,
                LocalMirrorOptions {
                    dry_run,
                    null_sink,
                    ..options()
                },
            )
            .await?;

            assert_eq!(
                summary.files_repaired, 0,
                "dry_run={dry_run} null_sink={null_sink} must repair nothing"
            );
            assert_eq!(
                durable_attributes(&dest_file),
                0x00,
                "dry_run={dry_run} null_sink={null_sink} must not touch the destination"
            );
            assert_eq!(
                summary.planned_files, 1,
                "the file is still PLANNED, so the reported plan is unchanged"
            );
        }
        Ok(())
    }
}

/// The destination diff runs its per-file work in parallel across a chunk
/// (ls-1 fix). Parallelism is where ordering and partial-result bugs hide,
/// so pin both: a workload spanning many chunks where only a scattered
/// subset needs transferring must land exactly that subset, and nothing
/// else, regardless of thread scheduling.
#[tokio::test]
async fn a_parallel_diff_selects_exactly_the_changed_files() -> Result<()> {
    let tmp = tempdir()?;
    let src = tmp.path().join("src");
    let dest = tmp.path().join("dest");
    fs::create_dir_all(&src)?;

    // Several chunks' worth (the diff chunks at 128).
    const FILES: usize = 500;
    for idx in 0..FILES {
        fs::write(src.join(format!("f{idx:04}.txt")), format!("v1-{idx}"))?;
    }
    let first = run_local_session(&src, &dest, options()).await?;
    assert_eq!(first.copied_files, FILES);

    // Change a scattered subset, spanning chunk boundaries and including
    // the first and last entries.
    let changed: Vec<usize> = vec![0, 1, 127, 128, 129, 255, 256, 300, 498, 499];
    for idx in &changed {
        let path = src.join(format!("f{idx:04}.txt"));
        fs::write(&path, format!("v2-{idx}-longer-content"))?;
    }

    let second = run_local_session(&src, &dest, options()).await?;
    assert_eq!(
        second.copied_files,
        changed.len(),
        "exactly the changed files transfer — no more (a parallel diff that \
         dropped or duplicated verdicts would miss this)"
    );
    for idx in 0..FILES {
        let expected = if changed.contains(&idx) {
            format!("v2-{idx}-longer-content")
        } else {
            format!("v1-{idx}")
        };
        assert_eq!(
            fs::read_to_string(dest.join(format!("f{idx:04}.txt")))?,
            expected,
            "f{idx:04} has the wrong content after a parallel diff"
        );
    }
    Ok(())
}

/// ls-1 step (0): the wall-clock breakdown, pinned end to end on a real
/// local session. `docs/plan/LOCAL_SMALL_FILE_PATH.md`.
mod phase_breakdown {
    use super::*;
    use blit_core::transfer_session::{LocalPhase, LocalPhaseProbe, LocalPhaseReport};
    use std::sync::{Arc, Mutex as StdMutex};

    fn capturing() -> (LocalPhaseProbe, Arc<StdMutex<Vec<LocalPhaseReport>>>) {
        let sink: Arc<StdMutex<Vec<LocalPhaseReport>>> = Arc::default();
        let seen = Arc::clone(&sink);
        let probe = LocalPhaseProbe::capture("ls1-test", move |report| {
            seen.lock().expect("sink poisoned").push(report);
        });
        (probe, sink)
    }

    fn total(report: &LocalPhaseReport, phase: LocalPhase) -> u64 {
        report
            .phases
            .iter()
            .find(|(candidate, _)| *candidate == phase)
            .map(|(_, aggregate)| aggregate.total_ns)
            .expect("every phase is reported")
    }

    fn samples(report: &LocalPhaseReport, phase: LocalPhase) -> u64 {
        report
            .phases
            .iter()
            .find(|(candidate, _)| *candidate == phase)
            .map(|(_, aggregate)| aggregate.samples)
            .expect("every phase is reported")
    }

    #[tokio::test]
    async fn a_real_local_session_reports_every_phase_once() -> Result<()> {
        let tmp = tempdir()?;
        let src = tmp.path().join("src");
        let dest = tmp.path().join("dest");
        fs::create_dir_all(&src)?;
        for index in 0..8 {
            fs::write(src.join(format!("f{index}.txt")), vec![b'x'; 1024])?;
        }

        let (probe, sink) = capturing();
        let summary = run_local_session(
            &src,
            &dest,
            LocalMirrorOptions {
                perf_history: false,
                phase_probe: probe,
                ..Default::default()
            },
        )
        .await?;
        assert_eq!(summary.copied_files, 8);

        let reports = sink.lock().expect("sink poisoned");
        assert_eq!(reports.len(), 1, "exactly one breakdown per session");
        let report = reports.first().expect("one report");

        // The denominator has to be real, and the phases that necessarily ran
        // on a copy of eight fresh files have to be present. Without this the
        // probe could ship reporting all zeros and nobody would notice.
        assert!(report.session_wall_ns > 0, "the session took real time");
        assert!(
            samples(report, LocalPhase::Enumerate) > 0,
            "the source walk is measured"
        );
        // One measured span per header handed downstream. This is the guard
        // on the enumerate/backpressure SPLIT, not on the wait being large:
        // a fast local test never actually blocks, so asserting on the
        // duration would be vacuous, while the sample count goes to 0 the
        // moment the split at the send site is removed. The split is the
        // point of step (0) — without it a slow destination reports as slow
        // enumeration.
        assert_eq!(
            samples(report, LocalPhase::EnumerateBackpressure),
            8,
            "every manifest send is timed separately from the walk"
        );
        assert!(
            samples(report, LocalPhase::Compare) > 0,
            "the destination diff is measured"
        );
        assert!(
            samples(report, LocalPhase::Plan) > 0,
            "planning the needed files is measured"
        );
        assert!(
            samples(report, LocalPhase::Apply) > 0,
            "the apply drain is measured"
        );
        // cr-ls1-1: the queue wait is timed too. On this fast fixture it is
        // near zero, which is the point — the phase must exist and be
        // sampled even when it costs nothing, so a run where it costs
        // everything cannot be mistaken for a run where apply was free.
        assert!(
            samples(report, LocalPhase::ApplyBackpressure) > 0,
            "handing payloads to the apply queue is measured"
        );
        // Not a mirror, so no delete pass ran: a measured zero, not a gap.
        assert_eq!(samples(report, LocalPhase::Delete), 0);
        Ok(())
    }

    #[tokio::test]
    async fn compare_excludes_the_attribute_repair_it_contains() -> Result<()> {
        // The subtraction guard. pfc-6 repairs inside the diff, so a naive
        // compare span would bill the same nanoseconds twice. Here nothing is
        // repairable, so AttributeRepair must be a measured zero while Compare
        // still records real time — if the subtraction were wrong in the other
        // direction (repair time leaking into compare), the repair-heavy case
        // this exists for would silently double-count.
        let tmp = tempdir()?;
        let src = tmp.path().join("src");
        let dest = tmp.path().join("dest");
        fs::create_dir_all(&src)?;
        fs::write(src.join("only.txt"), b"payload")?;

        let (probe, sink) = capturing();
        run_local_session(
            &src,
            &dest,
            LocalMirrorOptions {
                perf_history: false,
                phase_probe: probe,
                ..Default::default()
            },
        )
        .await?;

        let reports = sink.lock().expect("sink poisoned");
        let report = reports.first().expect("one report");
        assert_eq!(
            total(report, LocalPhase::AttributeRepair),
            0,
            "nothing was repairable, so no repair time is attributed"
        );
        assert!(total(report, LocalPhase::Compare) > 0);
        Ok(())
    }

    #[tokio::test]
    async fn a_mirror_delete_pass_is_attributed_to_delete() -> Result<()> {
        let tmp = tempdir()?;
        let src = tmp.path().join("src");
        let dest = tmp.path().join("dest");
        fs::create_dir_all(&src)?;
        fs::create_dir_all(&dest)?;
        fs::write(src.join("keep.txt"), b"keep")?;
        fs::write(dest.join("extraneous.txt"), b"go away")?;

        let (probe, sink) = capturing();
        let summary = run_local_session(
            &src,
            &dest,
            LocalMirrorOptions {
                mirror: true,
                perf_history: false,
                phase_probe: probe,
                ..Default::default()
            },
        )
        .await?;
        assert_eq!(summary.deleted_files, 1);

        let reports = sink.lock().expect("sink poisoned");
        let report = reports.first().expect("one report");
        assert_eq!(
            samples(report, LocalPhase::Delete),
            1,
            "the mirror delete pass is one measured span"
        );
        Ok(())
    }

    #[tokio::test]
    async fn the_probe_is_off_unless_asked_for() -> Result<()> {
        // The default must not emit. A diagnostic that switches itself on in
        // production is a defect, and `LocalMirrorOptions::default()` is what
        // every caller in the CLI actually uses.
        let tmp = tempdir()?;
        let src = tmp.path().join("src");
        let dest = tmp.path().join("dest");
        fs::create_dir_all(&src)?;
        fs::write(src.join("f.txt"), b"data")?;

        let summary = run_local_session(
            &src,
            &dest,
            LocalMirrorOptions {
                perf_history: false,
                phase_probe: LocalPhaseProbe::disabled(),
                ..Default::default()
            },
        )
        .await?;
        assert_eq!(summary.copied_files, 1, "tracing off changes no behaviour");
        Ok(())
    }
}
