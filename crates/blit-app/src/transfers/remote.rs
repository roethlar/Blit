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
//! - [`run_pull_sync`] + [`apply_pull_mirror_purge`] +
//!   [`PullSyncExecution`] / [`PullSyncOutcome`] /
//!   [`PullExecutionOutcome`] — pull entry-point orchestration,
//!   split into the pull_sync RPC half (returns intermediate
//!   state) and the mirror-purge half. The caller composes them
//!   so it can tear down its progress channel between the two
//!   steps. Round-1 of this slice bundled both halves into a
//!   single library call, which kept the progress monitor alive
//!   through purge — round-2 split fixes that regression.
//!   Presentation (progress monitor spawn, summary printing)
//!   stays in `blit-cli` until the M-C `AppProgressEvent`
//!   reshape lands.
//!
//! - [`run_remote_push`] + [`PushExecution`] +
//!   [`PushExecutionOutcome`] — push entry-point orchestration.
//!   The library builds the `Arc<dyn TransferSource>` from the
//!   [`Endpoint`] passed in (Local → `FsTransferSource`,
//!   Remote → `RemoteTransferSource` over a pull-client) and
//!   wraps it in the [`FilteredSource`] before invoking
//!   `RemotePushClient::push`. The CLI-side progress monitor
//!   stays in `blit-cli` (M-C `AppProgressEvent` reshape is
//!   its own pause point).
//!
//! No further `transfers/remote.rs` orchestration lives in
//! `blit-cli` after this slice — the CLI's `transfers/remote.rs`
//! retains only the clap-arg wrappers and presentation
//! (progress monitor + JSON / human printers).

use crate::endpoints::Endpoint;
use blit_core::fs_enum::FileFilter;
use blit_core::generated::{FileHeader, MirrorMode};
use blit_core::path_safety::{canonical_dest_root, safe_join_contained};
use blit_core::remote::pull::{PullSyncOptions, RemotePullReport};
use blit_core::remote::push::RemotePushReport;
use blit_core::remote::transfer::source::{
    FilteredSource, FsTransferSource, RemoteTransferSource, TransferSource,
};
use blit_core::remote::transfer::RemoteTransferProgress;
use blit_core::remote::{RemoteEndpoint, RemotePath, RemotePullClient, RemotePushClient};
use eyre::{bail, eyre, Context, Result};
use std::path::{Path, PathBuf};
use std::sync::Arc;

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

/// Inputs for [`run_pull_sync`]. Primitive fields only — no
/// clap, no presentation types — so the CLI and the future TUI
/// can both build it without sharing a dependency.
///
/// `remote_label` is the human-readable string used in error
/// context (e.g. `pulling from <label> into <dest>`). The CLI
/// passes `format_remote_endpoint(&remote)`; the TUI passes
/// whatever string it shows the user in the picker.
pub struct PullSyncExecution {
    pub remote: RemoteEndpoint,
    pub dest_root: PathBuf,
    pub options: PullSyncOptions,
    pub compute_checksums: bool,
    pub mirror_mode: bool,
    pub remote_label: String,
}

/// Output of [`run_pull_sync`]. The PullSync handshake is done
/// and the daemon's report (including any mirror-mode delete
/// list) is in hand, but no destination filesystem mutation
/// has happened yet. The caller is expected to tear down its
/// progress channel here and then call
/// [`apply_pull_mirror_purge`] to run the destructive half of
/// the flow — that ordering is the round-2 fix for the
/// behavior regression where purge ran while the progress
/// monitor was still alive.
pub struct PullSyncOutcome {
    pub report: RemotePullReport,
    pub actual_dest: PathBuf,
}

/// Full post-pull state for the CLI printer / TUI summary —
/// PullSync report + actual destination + (mirror-mode) purge
/// stats. Composed by the caller from [`PullSyncOutcome`] plus
/// the result of [`apply_pull_mirror_purge`].
pub struct PullExecutionOutcome {
    pub report: RemotePullReport,
    pub actual_dest: PathBuf,
    pub mirror_purge_stats: Option<LocalPurgeStats>,
}

/// Run the PullSync half of a remote pull: connect, enumerate
/// the local manifest, and run the PullSync handshake. Does
/// **not** apply any mirror-mode delete list — that's
/// [`apply_pull_mirror_purge`], called by the caller after it
/// has had a chance to tear down the progress channel.
///
/// `progress` is borrowed for the duration of the PullSync RPC
/// only. The split exists so the caller can run the lifecycle:
///
/// ```text
/// let (handle, task) = spawn_progress_monitor(...);
/// let sync = run_pull_sync(execution, handle.as_ref()).await?;
/// drop(handle);
/// if let Some(t) = task { let _ = t.await; }
/// let purge = apply_pull_mirror_purge(&sync, mirror_mode).await?;
/// ```
///
/// Round 2 of `a0-pull-execution` introduced this split. Round
/// 1 had a single `run_remote_pull` that did pull_sync **and**
/// purge before returning, which forced the progress monitor
/// to stay alive across the (potentially long) purge — a
/// regression vs the pre-Phase-5 CLI lifecycle that the
/// reviewer caught.
pub async fn run_pull_sync(
    execution: PullSyncExecution,
    progress: Option<&RemoteTransferProgress>,
) -> Result<PullSyncOutcome> {
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

    Ok(PullSyncOutcome {
        report,
        actual_dest,
    })
}

/// Apply the daemon-authored mirror-delete list when
/// `mirror_mode` is true. No-op (returns `Ok(None)`) for plain
/// copy pulls or when the report carries no paths to delete.
/// Splits the purge step out of the pull_sync RPC so the
/// caller's progress monitor can be torn down between the two
/// (see [`run_pull_sync`] doc comment).
///
/// R46-F6 ordering still holds at the caller level: the
/// printer is invoked **after** this returns, so the purge
/// stats appear in the same JSON document as the transfer
/// report. The R46-F6 fix was about ordering relative to
/// *printing*, not relative to the progress monitor — the
/// monitor lifecycle was lost in round 1 by trying to bundle
/// pull_sync + purge into a single library call.
pub async fn apply_pull_mirror_purge(
    outcome: &PullSyncOutcome,
    mirror_mode: bool,
) -> Result<Option<LocalPurgeStats>> {
    if !mirror_mode {
        return Ok(None);
    }
    let Some(ref delete_paths) = outcome.report.paths_to_delete else {
        return Ok(None);
    };
    if delete_paths.is_empty() {
        return Ok(None);
    }
    Ok(Some(
        delete_listed_paths(&outcome.actual_dest, delete_paths).await?,
    ))
}

/// Inputs for [`run_remote_push`]. Primitive fields only — no
/// clap, no presentation. CLI builds this from `&TransferArgs`;
/// the future TUI builds it directly.
///
/// `source` is the [`Endpoint`] the user picked (local path or
/// a remote module). The library handles the dispatch internally:
/// `Endpoint::Local(path)` → `FsTransferSource`,
/// `Endpoint::Remote(endpoint)` → `RemoteTransferSource` over a
/// pull-client connected at call time. The library then wraps
/// the inner source with [`FilteredSource`] before invoking
/// `RemotePushClient::push`, so the universal filter chokepoint
/// (R49) applies on push the same way it applies on
/// local→local and remote→remote.
///
/// `filter` is the runtime `FileFilter` (not the wire
/// `FilterSpec`); the CLI builds it via
/// `blit_app::transfers::filter::build`. `mirror_kind`
/// communicates the user's `--delete-scope` choice to the
/// daemon (R59 #1 F2: `--mirror --include …` deletes only
/// in-scope entries via `FilteredSubset`).
pub struct PushExecution {
    pub source: Endpoint,
    pub remote: RemoteEndpoint,
    pub filter: FileFilter,
    pub mirror_mode: bool,
    pub mirror_kind: MirrorMode,
    pub force_grpc: bool,
    pub trace_data_plane: bool,
    pub require_complete_scan: bool,
    pub remote_label: String,
}

/// Output of [`run_remote_push`]. `destination` is the
/// caller-supplied `remote_label` echoed back — the printer
/// consumes it. `show_progress` is intentionally **not** here;
/// it's a CLI-side presentation hint that the CLI threads
/// directly into its own `DeferredPushState`.
pub struct PushExecutionOutcome {
    pub report: RemotePushReport,
    pub destination: String,
}

/// Run a remote push end-to-end: connect to the destination,
/// build the `Arc<dyn TransferSource>` from the [`Endpoint`]
/// (resolving any pull-client connection for remote sources),
/// wrap it in the universal [`FilteredSource`], and invoke
/// `RemotePushClient::push`. No mirror-purge step exists on
/// the push side — mirror deletes happen on the daemon and
/// surface through the returned [`RemotePushReport`].
///
/// `progress` is borrowed for the duration of the push RPC.
/// The caller owns the channel + monitor task; this function
/// never spawns or awaits the monitor. Standard lifecycle:
///
/// ```text
/// let (handle, task) = spawn_progress_monitor(...);
/// let outcome = run_remote_push(execution, handle.as_ref()).await?;
/// drop(handle);
/// if let Some(t) = task { let _ = t.await; }
/// ```
///
/// Unlike the pull side, there is no need to split this into
/// pre-/post-purge halves — push has no post-RPC destructive
/// step on the caller's filesystem, so the monitor's lifetime
/// already lines up cleanly with the RPC.
pub async fn run_remote_push(
    execution: PushExecution,
    progress: Option<&RemoteTransferProgress>,
) -> Result<PushExecutionOutcome> {
    let mut client = RemotePushClient::connect(execution.remote.clone())
        .await
        .with_context(|| format!("connecting to {}", execution.remote.control_plane_uri()))?;

    let inner: Arc<dyn TransferSource> = match execution.source {
        Endpoint::Local(path) => Arc::new(FsTransferSource::new(path)),
        Endpoint::Remote(endpoint) => {
            let pull_client = RemotePullClient::connect(endpoint.clone())
                .await
                .with_context(|| {
                    format!("connecting to source {}", endpoint.control_plane_uri())
                })?;
            // Resolve the source's root path from its `RemotePath`
            // variant. Mirrors the pre-A.0 CLI code; semantics unchanged.
            let root = match &endpoint.path {
                RemotePath::Module { rel_path, .. } => rel_path.clone(),
                RemotePath::Root { rel_path } => rel_path.clone(),
                RemotePath::Discovery => PathBuf::from("."),
            };
            Arc::new(RemoteTransferSource::new(pull_client, root))
        }
    };

    let transfer_source: Arc<dyn TransferSource> =
        Arc::new(FilteredSource::new(inner, execution.filter.clone()));

    let push_result = client
        .push(
            transfer_source.clone(),
            &execution.filter,
            execution.mirror_mode,
            execution.mirror_kind,
            execution.force_grpc,
            execution.require_complete_scan,
            progress,
            execution.trace_data_plane,
        )
        .await
        .with_context(|| {
            format!(
                "negotiating push manifest for {} -> {}",
                transfer_source.root().display(),
                execution.remote_label
            )
        })?;

    Ok(PushExecutionOutcome {
        report: push_result,
        destination: execution.remote_label,
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
