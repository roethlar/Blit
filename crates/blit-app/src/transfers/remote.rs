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
//! - [`run_delegated_pull`] + [`DelegatedPullExecution`] +
//!   [`DelegatedPullOutcome`] — delegated remote→remote
//!   orchestration. Builds the `DelegatedPullRequest`, connects
//!   to the destination's `BlitClient`, consumes the streamed
//!   payload (ManifestBatch / BytesProgress / Summary / Error),
//!   maps errors via [`map_delegated_error`], and returns the
//!   summary. The `on_started` callback fires once when the
//!   destination emits its `Started` event, giving the caller a
//!   live hook for verbose-mode diagnostics without baking
//!   presentation into the library.
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
use blit_core::generated::delegated_pull_error::Phase as DelegatedPullPhase;
use blit_core::generated::delegated_pull_progress::Payload as DelegatedPayload;
use blit_core::generated::{
    BytesProgress, DelegatedPullRequest, DelegatedPullStarted, DelegatedPullSummary, FileHeader,
    MirrorMode, RemoteSourceLocator,
};
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
use tonic::Code;

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

/// Inputs for [`run_delegated_pull`]. Primitive fields only —
/// no clap, no presentation. CLI builds this from
/// `&TransferArgs`; the future TUI builds it directly.
///
/// `relay_fallback_suggestable` is a CLI-side knob baked into
/// the error-mapping logic: when true (copy / mirror callers),
/// error messages mention `--relay-via-cli` as an escape hatch;
/// when false (move callers — `--relay-via-cli` is refused there
/// per R53-F2), the hint is omitted so users aren't sent to a
/// flag the same command rejects. Documented here because the
/// library now owns the error mapping.
pub struct DelegatedPullExecution {
    pub src: RemoteEndpoint,
    pub dst: RemoteEndpoint,
    pub options: PullSyncOptions,
    pub trace_data_plane: bool,
    pub relay_fallback_suggestable: bool,
    pub dst_label: String,
    /// Detach the transfer from the calling CLI. When true,
    /// the destination daemon's `tx.closed()` race disarms,
    /// so client disconnect no longer drops the transfer.
    /// The CLI can exit after observing the daemon's
    /// `Started` event. Only valid on remote→remote
    /// delegated transfers (push / pull / pull_sync have the
    /// CLI in the byte path and reject the flag upstream).
    pub detach: bool,
}

/// Output of [`run_delegated_pull`]. The `src` / `dst` endpoints
/// are echoed back so the caller's printer can reference them
/// without keeping its own copies.
pub struct DelegatedPullOutcome {
    pub summary: DelegatedPullSummary,
    pub src: RemoteEndpoint,
    pub dst: RemoteEndpoint,
}

/// Per-stream state tracked while consuming `BytesProgress`
/// messages. `files_completed` / `bytes_completed` are
/// monotonic counters from the daemon; we use them to compute
/// deltas against the CLI's [`RemoteTransferProgress`] channel.
#[derive(Default)]
struct DelegatedBytesProgressState {
    files_completed: u64,
    bytes_completed: u64,
}

fn report_bytes_progress(
    progress: Option<&RemoteTransferProgress>,
    state: &mut DelegatedBytesProgressState,
    bytes: &BytesProgress,
) {
    if let Some(progress) = progress {
        let file_delta = bytes
            .files_completed
            .saturating_sub(state.files_completed)
            .try_into()
            .unwrap_or(usize::MAX);
        let byte_delta = bytes.bytes_completed.saturating_sub(state.bytes_completed);
        state.files_completed = state.files_completed.max(bytes.files_completed);
        state.bytes_completed = state.bytes_completed.max(bytes.bytes_completed);
        if file_delta > 0 || byte_delta > 0 {
            progress.report_payload(file_delta, byte_delta);
        }
    }
}

/// Map a daemon-side `DelegatedPullError` to a human-readable
/// CLI-facing report. Behavior is parameterized by
/// `relay_fallback_suggestable` — see [`DelegatedPullExecution`]
/// for the policy.
pub fn map_delegated_error(
    phase: i32,
    message: &str,
    relay_fallback_suggestable: bool,
) -> eyre::Report {
    let phase = DelegatedPullPhase::try_from(phase).unwrap_or(DelegatedPullPhase::Unknown);
    let relay_clause = if relay_fallback_suggestable {
        ". Pass --relay-via-cli to route through the CLI host"
    } else {
        ""
    };
    let relay_clause_semi = if relay_fallback_suggestable {
        "; pass --relay-via-cli to route through the CLI host"
    } else {
        ""
    };
    match phase {
        DelegatedPullPhase::DelegationRejected => {
            eyre!("delegation rejected by destination daemon: {message}{relay_clause}")
        }
        DelegatedPullPhase::ConnectSource => {
            eyre!("destination daemon cannot reach source ({message}){relay_clause_semi}")
        }
        DelegatedPullPhase::Negotiate => eyre!("source refused delegated pull: {message}"),
        DelegatedPullPhase::Transfer => eyre!("delegated transfer failed: {message}"),
        DelegatedPullPhase::Apply => {
            eyre!("destination failed to apply delegated transfer: {message}")
        }
        DelegatedPullPhase::Unknown => eyre!("delegated transfer failed: {message}"),
    }
}

/// Extract the `(module, destination_path)` pair the
/// `DelegatedPullRequest` needs from a parsed
/// [`RemoteEndpoint`]. Errors on `RemotePath::Discovery` —
/// remote destinations always require an explicit module or
/// root.
pub fn destination_spec_fields(dst: &RemoteEndpoint) -> Result<(String, String)> {
    match &dst.path {
        RemotePath::Module { module, rel_path } => {
            Ok((module.clone(), normalize_for_request(rel_path)))
        }
        RemotePath::Root { rel_path } => Ok((String::new(), normalize_for_request(rel_path))),
        RemotePath::Discovery => bail!(
            "remote destination must include a module or root (e.g., server:/module/ or server://path)"
        ),
    }
}

fn normalize_for_request(path: &Path) -> String {
    if path.as_os_str().is_empty() {
        ".".to_string()
    } else {
        path.iter()
            .map(|component| component.to_string_lossy())
            .collect::<Vec<_>>()
            .join("/")
    }
}

/// Run a delegated remote→remote pull: build the request,
/// connect to the destination's `BlitClient`, stream the
/// payload, and return the destination's summary.
///
/// `progress` is borrowed for the duration of the call. The
/// library translates per-payload `BytesProgress` messages into
/// `report_payload` calls on the channel; CLI's printer
/// continues to consume `ProgressEvent` as before.
///
/// `on_started` fires exactly once if the destination emits a
/// `Started` event (it precedes the first byte). The callback
/// is the stopgap presentation hook: CLI prints
/// `[delegation] destination pulling from <ep> (<n> stream(s))`
/// in verbose mode; TUI passes a no-op. The M-C
/// `AppProgressEvent` reshape will replace the callback with
/// a stream variant that both consumers handle uniformly.
///
/// Errors from the destination's stream are mapped through
/// [`map_delegated_error`]; transport-level failures
/// (Unavailable, Unimplemented) get their own CLI-style hints
/// because they don't have a `phase`.
pub async fn run_delegated_pull<F>(
    execution: DelegatedPullExecution,
    progress: Option<&RemoteTransferProgress>,
    mut on_started: F,
) -> Result<DelegatedPullOutcome>
where
    F: FnMut(&DelegatedPullStarted),
{
    let spec = RemotePullClient::build_spec_from_options(&execution.src, &execution.options)?;
    let (dst_module, dst_destination_path) = destination_spec_fields(&execution.dst)?;

    let request = DelegatedPullRequest {
        dst_module,
        dst_destination_path,
        src: Some(RemoteSourceLocator {
            host: execution.src.host.clone(),
            port: execution.src.port as u32,
        }),
        spec: Some(spec),
        trace_data_plane: execution.trace_data_plane,
        detach: execution.detach,
    };

    let uri = execution.dst.control_plane_uri();
    let mut client = crate::client::connect_with_timeout(uri.clone())
        .await
        .with_context(|| format!("connecting to destination {}", execution.dst_label))?;

    let response = client.delegated_pull(request).await.map_err(|status| {
        let relay_hint = if execution.relay_fallback_suggestable {
            " or pass --relay-via-cli"
        } else {
            ""
        };
        let relay_clause = if execution.relay_fallback_suggestable {
            "; pass --relay-via-cli to route through the CLI host"
        } else {
            ""
        };
        if status.code() == Code::Unimplemented {
            eyre!(
                "destination daemon does not implement DelegatedPull; upgrade the destination \
                 daemon{relay_hint}"
            )
        } else if status.code() == Code::Unavailable {
            eyre!(
                "destination daemon is unavailable for delegated pull ({}){}",
                status.message(),
                relay_clause
            )
        } else {
            eyre!(
                "delegated remote-to-remote transfer failed: {}",
                status.message()
            )
        }
    })?;
    let mut stream = response.into_inner();

    let mut summary: Option<DelegatedPullSummary> = None;
    let mut failure: Option<eyre::Report> = None;
    let mut bytes_progress_state = DelegatedBytesProgressState::default();

    loop {
        let message = match stream.message().await {
            Ok(Some(message)) => message,
            Ok(None) => break,
            Err(status) => {
                failure = Some(if status.code() == Code::Unavailable {
                    let relay_clause = if execution.relay_fallback_suggestable {
                        "; pass --relay-via-cli to route through the CLI host"
                    } else {
                        ""
                    };
                    eyre!(
                        "delegation stream lost ({}){}",
                        status.message(),
                        relay_clause
                    )
                } else {
                    eyre!("delegation stream failed: {}", status.message())
                });
                break;
            }
        };
        match message.payload {
            Some(DelegatedPayload::Started(started)) => {
                on_started(&started);
            }
            Some(DelegatedPayload::ManifestBatch(batch)) => {
                if let Some(progress) = progress {
                    progress.report_manifest_batch(batch.file_count as usize);
                }
            }
            Some(DelegatedPayload::BytesProgress(bytes)) => {
                report_bytes_progress(progress, &mut bytes_progress_state, &bytes);
            }
            Some(DelegatedPayload::Summary(done)) => {
                summary = Some(done);
                break;
            }
            Some(DelegatedPayload::Error(error)) => {
                failure = Some(map_delegated_error(
                    error.phase,
                    &error.upstream_message,
                    execution.relay_fallback_suggestable,
                ));
                break;
            }
            None => {}
        }
    }

    if let Some(error) = failure {
        return Err(error);
    }

    let summary = summary.ok_or_else(|| eyre!("delegation ended before summary"))?;
    Ok(DelegatedPullOutcome {
        summary,
        src: execution.src,
        dst: execution.dst,
    })
}

/// "Fire and forget" variant of [`run_delegated_pull`] for the
/// CLI's `--detach` flow. Opens the delegated_pull RPC,
/// receives the first `Started` event (which now carries the
/// daemon-assigned `transfer_id` after m-jobs-3), and returns
/// without consuming the rest of the stream. Dropping the
/// returned tuple's response stream closes the receiver — but
/// the daemon-side spawn closure honors `execution.detach` and
/// completes the transfer regardless.
///
/// Returns the `DelegatedPullStarted` payload (which the CLI
/// uses to print the transfer id + cancel hint) plus the
/// destination endpoint so the caller can format display
/// strings without re-parsing.
///
/// Refuses to proceed if `execution.detach` is `false` — the
/// detached semantic is meaningless on a tx.closed-armed
/// daemon, and the caller would mistakenly return success
/// while the daemon drops the transfer the moment we drop the
/// stream.
pub async fn run_delegated_pull_until_started(
    execution: DelegatedPullExecution,
) -> Result<(DelegatedPullStarted, RemoteEndpoint)> {
    if !execution.detach {
        return Err(eyre!(
            "run_delegated_pull_until_started requires execution.detach=true"
        ));
    }

    let spec = RemotePullClient::build_spec_from_options(&execution.src, &execution.options)?;
    let (dst_module, dst_destination_path) = destination_spec_fields(&execution.dst)?;

    let request = DelegatedPullRequest {
        dst_module,
        dst_destination_path,
        src: Some(RemoteSourceLocator {
            host: execution.src.host.clone(),
            port: execution.src.port as u32,
        }),
        spec: Some(spec),
        trace_data_plane: execution.trace_data_plane,
        detach: execution.detach,
    };

    let uri = execution.dst.control_plane_uri();
    let mut client = crate::client::connect_with_timeout(uri.clone())
        .await
        .with_context(|| format!("connecting to destination {}", execution.dst_label))?;

    let response = client.delegated_pull(request).await.map_err(|status| {
        if status.code() == Code::Unimplemented {
            eyre!(
                "destination daemon does not implement DelegatedPull; \
                 cannot detach against this daemon"
            )
        } else if status.code() == Code::Unavailable {
            eyre!(
                "destination daemon is unavailable for delegated pull ({})",
                status.message()
            )
        } else {
            eyre!(
                "delegated remote-to-remote transfer failed: {}",
                status.message()
            )
        }
    })?;
    let mut stream = response.into_inner();

    // Read the first frame and resolve. Started is the
    // daemon's first emitted payload per the
    // DelegatedPullProgress protocol; anything else (or
    // stream end) is a clear error.
    //
    // Empty `transfer_id` is a daemon-too-old signal: the
    // `Started.transfer_id` field arrived in m-jobs-3 and
    // older daemons leave it empty (proto3 default). We
    // **must** refuse here rather than return success,
    // because an older daemon also doesn't honor the
    // `detach=true` we asked for — dropping `stream` after
    // Started would let its tx.closed() race drop the
    // transfer. The caller would print a detached-success
    // message with no usable id while the transfer was
    // already cancelled.
    match stream.message().await {
        Ok(Some(message)) => match message.payload {
            Some(DelegatedPayload::Started(started)) => {
                if started.transfer_id.is_empty() {
                    return Err(eyre!(
                        "destination daemon is older than m-jobs-3 and cannot detach \
                         this transfer (Started.transfer_id was empty, and dropping \
                         the stream would cancel the transfer on an older daemon). \
                         Upgrade the destination daemon, or retry without --detach."
                    ));
                }
                // Dropping `stream` here closes the receiver
                // → daemon's tx.closed() resolves. With
                // detach=true the daemon ignores that and
                // keeps the transfer running.
                drop(stream);
                Ok((started, execution.dst))
            }
            Some(DelegatedPayload::Error(error)) => Err(map_delegated_error(
                error.phase,
                &error.upstream_message,
                execution.relay_fallback_suggestable,
            )),
            _ => Err(eyre!(
                "delegated pull emitted a non-Started payload before Started"
            )),
        },
        Ok(None) => Err(eyre!("delegated pull stream closed before Started")),
        Err(status) => Err(eyre!(
            "delegation stream failed before Started: {}",
            status.message()
        )),
    }
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
    async fn run_delegated_pull_until_started_refuses_non_detach() {
        // Guard: if a caller asks for the "exit after Started"
        // path without setting `execution.detach = true`, the
        // function refuses synchronously instead of opening
        // the RPC. Otherwise dropping the stream after Started
        // would let the daemon's tx.closed() race drop the
        // transfer.
        use blit_core::remote::endpoint::RemoteEndpoint;
        use blit_core::remote::RemotePath;
        let endpoint = RemoteEndpoint {
            host: "127.0.0.1".to_string(),
            port: 1,
            path: RemotePath::Module {
                module: "m".to_string(),
                rel_path: PathBuf::new(),
            },
        };
        let execution = DelegatedPullExecution {
            src: endpoint.clone(),
            dst: endpoint,
            options: PullSyncOptions::default(),
            trace_data_plane: false,
            relay_fallback_suggestable: false,
            dst_label: "x".to_string(),
            detach: false,
        };
        let err = run_delegated_pull_until_started(execution)
            .await
            .expect_err("non-detach execution must be refused");
        assert!(
            err.to_string().contains("requires execution.detach=true"),
            "got: {err}"
        );
    }

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

    // Delegated-pull helper tests — moved from
    // `crates/blit-cli/src/transfers/remote_remote_direct.rs::tests`
    // in the a0-delegated-execution slice so the helpers and their
    // coverage live together.

    use blit_core::remote::transfer::{ProgressEvent, RemoteTransferProgress};
    use tokio::sync::mpsc;

    fn delegated_endpoint(path: RemotePath) -> RemoteEndpoint {
        RemoteEndpoint {
            host: "localhost".to_string(),
            port: 9031,
            path,
        }
    }

    #[test]
    fn destination_fields_for_module_root_use_dot_path() {
        let dst = delegated_endpoint(RemotePath::Module {
            module: "mod".to_string(),
            rel_path: PathBuf::new(),
        });
        let (module, path) = destination_spec_fields(&dst).unwrap();
        assert_eq!(module, "mod");
        assert_eq!(path, ".");
    }

    #[test]
    fn destination_fields_for_subpath_normalize_forward_slashes() {
        let dst = delegated_endpoint(RemotePath::Module {
            module: "mod".to_string(),
            rel_path: PathBuf::from("a").join("b"),
        });
        let (module, path) = destination_spec_fields(&dst).unwrap();
        assert_eq!(module, "mod");
        assert_eq!(path, "a/b");
    }

    #[test]
    fn bytes_progress_reports_cumulative_values_as_deltas() {
        let (tx, mut rx) = mpsc::unbounded_channel();
        let progress = RemoteTransferProgress::new(tx);
        let mut state = DelegatedBytesProgressState::default();

        report_bytes_progress(
            Some(&progress),
            &mut state,
            &BytesProgress {
                files_completed: 1,
                files_total: 3,
                bytes_completed: 1024,
                bytes_total: 4096,
            },
        );
        report_bytes_progress(
            Some(&progress),
            &mut state,
            &BytesProgress {
                files_completed: 2,
                files_total: 3,
                bytes_completed: 4096,
                bytes_total: 4096,
            },
        );

        assert!(matches!(
            rx.try_recv().unwrap(),
            ProgressEvent::Payload {
                files: 1,
                bytes: 1024
            }
        ));
        assert!(matches!(
            rx.try_recv().unwrap(),
            ProgressEvent::Payload {
                files: 1,
                bytes: 3072
            }
        ));
        assert!(rx.try_recv().is_err());
    }

    #[test]
    fn bytes_progress_duplicate_cumulative_update_is_not_counted_twice() {
        let (tx, mut rx) = mpsc::unbounded_channel();
        let progress = RemoteTransferProgress::new(tx);
        let mut state = DelegatedBytesProgressState::default();
        let update = BytesProgress {
            files_completed: 1,
            files_total: 1,
            bytes_completed: 2048,
            bytes_total: 2048,
        };

        report_bytes_progress(Some(&progress), &mut state, &update);
        report_bytes_progress(Some(&progress), &mut state, &update);

        assert!(matches!(
            rx.try_recv().unwrap(),
            ProgressEvent::Payload {
                files: 1,
                bytes: 2048
            }
        ));
        assert!(rx.try_recv().is_err());
    }
}
