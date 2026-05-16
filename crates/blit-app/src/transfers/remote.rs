//! Remote transfer orchestration helpers.
//!
//! Moved from `crates/blit-cli/src/transfers/remote.rs` in A.0
//! across two sub-slices:
//!
//! - [`enumerate_local_manifest`] — walks a local destination
//!   tree and produces the `Vec<FileHeader>` that PullSync
//!   sends to the daemon for comparison.
//! - [`delete_listed_paths`] + [`LocalPurgeStats`] — applies the
//!   daemon-authored delete list during a mirror pull, with
//!   canonical-containment safety.
//! - [`run_remote_pull`] + [`PullExecution`] + [`PullExecutionOutcome`]
//!   — pull entry-point orchestration. The CLI's
//!   `run_remote_pull_transfer_inner` builds a `PullExecution`
//!   from `&TransferArgs`, the future TUI builds it directly.
//!   Presentation (progress monitor spawn, summary printing)
//!   stays in `blit-cli` until the M-C `AppProgressEvent`
//!   reshape lands.
//!
//! The push entry-point functions
//! (`run_remote_push_transfer`) and the CLI-side progress
//! monitor stay in `blit-cli` for now and move in subsequent
//! A.0 sub-slices.

use blit_core::generated::FileHeader;
use blit_core::path_safety::{canonical_dest_root, safe_join_contained};
use blit_core::remote::pull::{PullSyncOptions, RemotePullReport};
use blit_core::remote::transfer::RemoteTransferProgress;
use blit_core::remote::{RemoteEndpoint, RemotePullClient};
use eyre::{bail, eyre, Context, Result};
use std::path::{Path, PathBuf};

/// Enumerate local files under `root` and build the manifest the
/// pull-sync handshake sends to the daemon. Returns an empty vec
/// (not an error) when `root` doesn't exist — matches the
/// daemon's "we'll send everything" contract for a fresh
/// destination.
///
/// When `compute_checksums` is `true`, Blake3 hashes are
/// computed in parallel via rayon. Metadata-only mode runs
/// sequentially (it's already I/O-bound on the stat calls).
pub async fn enumerate_local_manifest(
    root: &Path,
    compute_checksums: bool,
) -> Result<Vec<FileHeader>> {
    use blit_core::checksum::{hash_file, ChecksumType};
    use rayon::prelude::*;
    use walkdir::WalkDir;

    if !root.exists() {
        return Ok(Vec::new());
    }

    let root_path = root.to_path_buf();
    tokio::task::spawn_blocking(move || {
        // First, collect all file entries
        let entries: Vec<_> = WalkDir::new(&root_path)
            .into_iter()
            .filter_map(|e| e.ok())
            .filter(|e| e.file_type().is_file())
            .collect();

        // Process files in parallel when computing checksums
        let manifest: Vec<FileHeader> = if compute_checksums {
            entries
                .into_par_iter()
                .filter_map(|entry| {
                    let path = entry.path();
                    let rel = path.strip_prefix(&root_path).ok()?;
                    let relative_path = rel
                        .iter()
                        .map(|c| c.to_string_lossy())
                        .collect::<Vec<_>>()
                        .join("/");

                    let meta = std::fs::metadata(path).ok()?;
                    let mtime_seconds = meta
                        .modified()
                        .ok()
                        .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
                        .map(|d| d.as_secs() as i64)
                        .unwrap_or(0);

                    // Compute Blake3 checksum
                    let checksum = hash_file(path, ChecksumType::Blake3).ok()?;

                    Some(FileHeader {
                        relative_path,
                        size: meta.len(),
                        mtime_seconds,
                        permissions: 0,
                        checksum,
                    })
                })
                .collect()
        } else {
            // No checksums - use sequential iteration (faster for metadata-only)
            entries
                .into_iter()
                .filter_map(|entry| {
                    let path = entry.path();
                    let rel = path.strip_prefix(&root_path).ok()?;
                    let relative_path = rel
                        .iter()
                        .map(|c| c.to_string_lossy())
                        .collect::<Vec<_>>()
                        .join("/");

                    let meta = std::fs::metadata(path).ok()?;
                    let mtime_seconds = meta
                        .modified()
                        .ok()
                        .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
                        .map(|d| d.as_secs() as i64)
                        .unwrap_or(0);

                    Some(FileHeader {
                        relative_path,
                        size: meta.len(),
                        mtime_seconds,
                        permissions: 0,
                        checksum: vec![],
                    })
                })
                .collect()
        };

        Ok(manifest)
    })
    .await
    .map_err(|err| eyre!("manifest enumeration task failed: {}", err))?
}

/// Stats from [`delete_listed_paths`]. Fields widened from
/// private (pre-A.0 lived in `blit-cli` alongside the printer
/// code that read them) to `pub` since they now cross a crate
/// boundary; the CLI's pull printer reads them directly.
#[derive(Debug, Default)]
pub struct LocalPurgeStats {
    pub files_deleted: u64,
    pub dirs_deleted: u64,
}

/// Apply a delete list provided by the daemon. Each wire string
/// is routed through `path_safety::safe_join_contained` before
/// any filesystem op runs, so `..`, absolute paths, Windows
/// drive prefixes, UNC paths, and the like are rejected
/// uniformly with the rest of the receive pipeline. The prior
/// lexical `starts_with` check (R5-F1 of
/// `docs/reviews/followup_review_2026-05-02.md`) was insufficient:
/// `dest_root.join("../victim")` produces `dest_root/../victim`
/// which still starts with `dest_root` lexically and would have
/// passed.
///
/// Empty parent directories under `dest_root` are pruned
/// bottom-up after the file deletions.
///
/// R46-F3 (preserved verbatim from pre-A.0): captures the
/// canonical destination root once and fails closed if it
/// can't be canonicalized. We refuse to apply a delete list
/// rather than fall back to lexical-only on the destructive
/// side — lexical-only would expose mirror-purge to escape via
/// pre-existing dest symlinks, and unlike the write side a
/// delete failure here means data loss.
pub async fn delete_listed_paths(
    dest_root: &Path,
    relative_paths: &[String],
) -> Result<LocalPurgeStats> {
    use std::collections::BTreeSet;
    let mut stats = LocalPurgeStats {
        files_deleted: 0,
        dirs_deleted: 0,
    };
    let mut candidate_parents: BTreeSet<PathBuf> = BTreeSet::new();

    let canonical = canonical_dest_root(dest_root).map_err(|e| {
        eyre!(
            "cannot canonicalize destination '{}' for mirror-purge containment: {:#}",
            dest_root.display(),
            e
        )
    })?;

    for rel in relative_paths {
        let target = safe_join_contained(&canonical, dest_root, rel).map_err(|e| {
            eyre!(
                "daemon delete list contained unsafe path '{}': {:#}",
                rel,
                e
            )
        })?;
        // safe_join("") returns dest_root itself; we never delete the
        // destination root.
        if target == dest_root {
            bail!("daemon delete list referenced the destination root itself");
        }
        match tokio::fs::remove_file(&target).await {
            Ok(()) => {
                stats.files_deleted += 1;
                let mut p = target.parent();
                while let Some(parent) = p {
                    if parent == dest_root {
                        break;
                    }
                    candidate_parents.insert(parent.to_path_buf());
                    p = parent.parent();
                }
            }
            // Already gone is fine; daemon's view may lag behind.
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
            Err(e) => {
                return Err(eyre!("failed to delete {}: {}", target.display(), e));
            }
        }
    }

    // Prune empty directories deepest-first.
    let mut dirs: Vec<_> = candidate_parents.into_iter().collect();
    dirs.sort_by_key(|p| std::cmp::Reverse(p.components().count()));
    for dir in dirs {
        if tokio::fs::remove_dir(&dir).await.is_ok() {
            stats.dirs_deleted += 1;
        }
    }
    Ok(stats)
}

/// Inputs for [`run_remote_pull`]. Primitive fields only — no
/// clap, no presentation types — so the CLI and the future TUI
/// can both build it without sharing a dependency.
///
/// `remote_label` is the human-readable string used in error
/// context (e.g. `pulling from <label> into <dest>`). The CLI
/// passes `format_remote_endpoint(&remote)`; the TUI passes
/// whatever string it shows the user in the picker.
pub struct PullExecution {
    pub remote: RemoteEndpoint,
    pub dest_root: PathBuf,
    pub options: PullSyncOptions,
    pub compute_checksums: bool,
    pub mirror_mode: bool,
    pub remote_label: String,
}

/// Captured outcome of [`run_remote_pull`]. Carries everything
/// the CLI's pull printer (`print_deferred_pull_result`) needs
/// to render output; the TUI consumes the same struct.
pub struct PullExecutionOutcome {
    pub report: RemotePullReport,
    pub actual_dest: PathBuf,
    pub mirror_purge_stats: Option<LocalPurgeStats>,
}

/// Run a remote pull end-to-end: connect, enumerate the local
/// manifest, run the PullSync handshake, and (in mirror mode)
/// apply the daemon-authored delete list. Returns the captured
/// state without printing — presentation is the caller's job.
///
/// `progress` is borrowed for the duration of the PullSync RPC.
/// The caller (CLI) owns the channel + monitor task; this
/// function never spawns or awaits the monitor. Lifecycle:
///
/// ```text
/// let (handle, task) = spawn_progress_monitor(...);
/// let outcome = run_remote_pull(execution, handle.as_ref()).await?;
/// drop(handle);
/// if let Some(t) = task { let _ = t.await; }
/// ```
///
/// R46-F6 ordering is preserved: mirror-purge runs *before*
/// the function returns, so the caller can include
/// `mirror_purge_stats` in the same JSON document as the
/// transfer report (pre-fix, the success line was already on
/// stdout when purge ran, which broke single-document JSON).
pub async fn run_remote_pull(
    execution: PullExecution,
    progress: Option<&RemoteTransferProgress>,
) -> Result<PullExecutionOutcome> {
    let mut client = RemotePullClient::connect(execution.remote.clone())
        .await
        .with_context(|| format!("connecting to {}", execution.remote.control_plane_uri()))?;

    let actual_dest = execution.dest_root;
    let local_manifest =
        enumerate_local_manifest(&actual_dest, execution.compute_checksums).await?;

    let report = client
        .pull_sync(
            &actual_dest,
            local_manifest,
            &execution.options,
            execution.mirror_mode,
            progress,
        )
        .await
        .with_context(|| {
            format!(
                "pulling from {} into {}",
                execution.remote_label,
                actual_dest.display()
            )
        })?;

    let mirror_purge_stats = if execution.mirror_mode {
        if let Some(ref delete_paths) = report.paths_to_delete {
            if !delete_paths.is_empty() {
                Some(delete_listed_paths(&actual_dest, delete_paths).await?)
            } else {
                None
            }
        } else {
            None
        }
    } else {
        None
    };

    Ok(PullExecutionOutcome {
        report,
        actual_dest,
        mirror_purge_stats,
    })
}

#[cfg(test)]
mod tests {
    //! R46-F3 canonical-containment safety tests for
    //! `delete_listed_paths`. Moved from
    //! `crates/blit-cli/src/transfers/remote.rs::delete_list_safety_tests`
    //! in the a0-remote-helpers reopen pass so the public library
    //! function carries its own coverage — `cargo test -p blit-app`
    //! now exercises the safety property directly.

    use super::*;
    use tempfile::tempdir;

    #[tokio::test]
    async fn rejects_parent_traversal() {
        let tmp = tempdir().unwrap();
        let dest = tmp.path().join("dest");
        let outside = tmp.path().join("outside");
        std::fs::create_dir_all(&dest).unwrap();
        std::fs::write(tmp.path().join("victim.txt"), b"keep me").unwrap();
        std::fs::write(outside.parent().unwrap().join("victim.txt"), b"keep me").unwrap();

        let bad = vec!["../victim.txt".to_string()];
        let err = delete_listed_paths(&dest, &bad).await.unwrap_err();
        assert!(
            err.to_string().contains("unsafe path"),
            "expected unsafe-path error, got: {err}"
        );
        // The sibling file the daemon was trying to reach must still exist.
        assert!(tmp.path().join("victim.txt").exists());
    }

    #[tokio::test]
    async fn rejects_absolute_path() {
        let tmp = tempdir().unwrap();
        let dest = tmp.path().join("dest");
        std::fs::create_dir_all(&dest).unwrap();
        std::fs::write(tmp.path().join("victim.txt"), b"keep me").unwrap();

        let bad = vec!["/etc/passwd".to_string(), "/tmp/victim.txt".to_string()];
        let err = delete_listed_paths(&dest, &bad).await.unwrap_err();
        assert!(err.to_string().contains("unsafe path"));
        assert!(tmp.path().join("victim.txt").exists());
    }

    #[tokio::test]
    async fn deletes_in_scope_paths() {
        let tmp = tempdir().unwrap();
        let dest = tmp.path().join("dest");
        std::fs::create_dir_all(&dest).unwrap();
        std::fs::write(dest.join("ok.txt"), b"goodbye").unwrap();

        let good = vec!["ok.txt".to_string()];
        let stats = delete_listed_paths(&dest, &good).await.unwrap();
        assert_eq!(stats.files_deleted, 1);
        assert!(!dest.join("ok.txt").exists());
    }

    #[tokio::test]
    async fn rejects_root_self_reference() {
        let tmp = tempdir().unwrap();
        let dest = tmp.path().join("dest");
        std::fs::create_dir_all(&dest).unwrap();
        // Empty string normalizes to dest_root via safe_join.
        let bad = vec!["".to_string()];
        let err = delete_listed_paths(&dest, &bad).await.unwrap_err();
        assert!(err.to_string().contains("destination root"));
    }
}
