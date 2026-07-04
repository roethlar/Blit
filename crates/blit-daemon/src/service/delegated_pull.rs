//! Destination-side handler for the `DelegatedPull` RPC.
//!
//! The CLI calls this on the destination daemon when both endpoints in
//! a `blit copy` are remote. The destination daemon validates the
//! request through the delegation gate, opens its own pull against the
//! named source, and streams progress back to the CLI. Bytes flow
//! source→dst directly.
//!
//! See `docs/plan/REMOTE_REMOTE_DELEGATION_PLAN.md` §4.2 for the
//! ordered handler steps and §4.3 for the gate.

use std::path::{Path, PathBuf};
use std::sync::Arc;

use blit_core::generated::{
    delegated_pull_progress::Payload as ProgressPayload, ComparisonMode, DelegatedPullError,
    DelegatedPullProgress, DelegatedPullRequest, DelegatedPullStarted, DelegatedPullSummary,
    FileHeader, ManifestBatch as ProtoManifestBatch, MirrorMode, PeerCapabilities, PullSummary,
    TransferOperationSpec,
};
use blit_core::remote::endpoint::{RemoteEndpoint, RemotePath};
use blit_core::remote::pull::{PullSyncError, RemotePullClient};
use blit_core::remote::transfer::operation_spec::NormalizedTransferOperation;
use tokio::sync::mpsc;
use tonic::Status;

use crate::delegation_gate::{validate_source, GateDenial, HostResolver, LocatorView, StdResolver};
use crate::metrics::TransferMetrics;
use crate::runtime::{ModuleConfig, RootExport};
use crate::service::util::{resolve_contained_path, resolve_module};

/// audit-1: deadline for the dst→src TCP connect. Bounds the OS SYN
/// timeout (60-180s) against a firewalled/black-holed source; matches
/// the data-plane accept timeout's 30s margin for slow networks.
const SOURCE_CONNECT_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(30);

/// Per-request capabilities advertised by *this destination daemon* on
/// the spec it forwards to src. The CLI's value is overwritten — the
/// CLI is not in the byte path and cannot speak for what dst supports.
pub(crate) fn dst_capabilities() -> PeerCapabilities {
    PeerCapabilities {
        supports_resume: true,
        supports_tar_shards: true,
        supports_data_plane_tcp: true,
        supports_filter_spec: true,
        // ue-r2-2: the delegated byte path IS `pull_sync_with_spec`,
        // whose receive worker set is growable — the dst daemon (the
        // byte receiver and dialer in delegation) advertises resize
        // and inherits the whole client-side implementation.
        supports_stream_resize: true,
    }
}

/// Mandatory `client_capabilities` override (R25-F2). The CLI is not
/// the byte recipient in delegation; the destination is. Whatever
/// `client_capabilities` the CLI put on the spec must be replaced
/// with the destination's actual capabilities before the spec leaves
/// for src. Unconditional — no merging, no field-level fallback.
///
/// ue-r2-1b extends the same boundary to `receiver_capacity`: the byte
/// recipient is this destination daemon, so a CLI-supplied profile is
/// non-authoritative and is REPLACED with this daemon's own profile
/// (ue-r2-1e — dst is the receiver in delegation, so its capacity is
/// what the source's dial must respect).
pub(crate) fn apply_dst_capabilities_override(
    mut spec: TransferOperationSpec,
) -> TransferOperationSpec {
    spec.client_capabilities = Some(dst_capabilities());
    spec.receiver_capacity = Some(blit_core::engine::local_receiver_capacity());
    spec
}

/// Validate the wire spec via the same `NormalizedTransferOperation::from_spec`
/// boundary that push and pull_sync use (R30-F3). Catches bad
/// spec_version, malformed FilterSpec globs, and contradictory flag
/// combinations before the handler does any DNS resolution, outbound
/// connect, or manifest enumeration. The normalized form is
/// discarded — the wire spec travels onward verbatim — but the
/// validation side-effect protects src from surfaces an
/// already-rejected-by-this-receiver shape would let through.
///
/// Returns the original spec unchanged on success so callers can keep
/// using the wire shape. Returns a phase-bearing error string on
/// failure.
pub(crate) fn validate_spec(spec: TransferOperationSpec) -> Result<TransferOperationSpec, String> {
    NormalizedTransferOperation::from_spec(spec.clone()).map_err(|e| format!("{e:#}"))?;
    // ue-r2-1h review (self-review panel F1): metadata_only is a
    // header-scan session shape for the relay's direct PullSync use
    // ONLY. Forwarded through delegation it would make the source
    // stream bare FileHeaders that this daemon's pull_sync client
    // loop answers with File::create — truncating every enumerated
    // destination file to zero bytes and then reporting success.
    // Fail closed at the same boundary that validates everything
    // else, before any outbound connect.
    if spec.metadata_only {
        return Err(
            "metadata_only is not valid on a delegated pull: the destination \
             would materialize the source's headers as empty files"
                .to_string(),
        );
    }
    Ok(spec)
}

/// True iff the validated `mirror_mode` field on a `TransferOperationSpec`
/// authorizes deletions on the destination (R32-F1).
///
/// Only `MirrorMode::FilteredSubset` and `MirrorMode::All` cause
/// dst-side deletions. `Off` and `Unspecified` (the proto3 default
/// for an omitted field) must NOT trigger any unlink, even if the
/// source daemon attaches a non-empty `paths_to_delete` to its
/// pull report — a buggy or hostile source could otherwise cause
/// in-scope destination files to vanish during a plain copy.
///
/// The CLI applies the same gate at
/// `crates/blit-cli/src/transfers/remote.rs:304`. This helper exists
/// so the daemon's gate is testable without mocking the full
/// `pull_sync_with_spec` flow.
pub(crate) fn delete_list_authorized(mirror_mode_proto: i32) -> bool {
    mirror_mode_proto == MirrorMode::FilteredSubset as i32
        || mirror_mode_proto == MirrorMode::All as i32
}

/// Build a CLI-bound `DelegatedPullProgress` carrying a phased error.
fn err_progress(phase: i32, message: impl Into<String>) -> DelegatedPullProgress {
    DelegatedPullProgress {
        payload: Some(ProgressPayload::Error(DelegatedPullError {
            upstream_message: message.into(),
            phase,
        })),
    }
}

/// Spawn the destination-side delegated pull. Drains a one-shot RPC
/// request into a streaming response. The `tx` is the reply channel
/// the gRPC layer hands us; the handler is responsible for emitting
/// progress events and final summary/error onto it.
///
/// Returns `true` if `run_delegated_pull` completed without error,
/// `false` if it failed (and the phased error was sent on `tx`).
/// §3.1 followup: the caller uses this to flip the `--metrics`
/// completion line `ok` bit and increment `errors`. Pre-fix the
/// function returned `()` so a handler failure looked identical to
/// success on the operator-facing metric line.
pub(crate) async fn handle_delegated_pull(
    req: DelegatedPullRequest,
    modules: Arc<tokio::sync::Mutex<std::collections::HashMap<String, ModuleConfig>>>,
    default_root: Option<RootExport>,
    delegation: Arc<crate::delegation_gate::DelegationConfig>,
    metrics: Arc<TransferMetrics>,
    tx: mpsc::Sender<Result<DelegatedPullProgress, Status>>,
    transfer_id: String,
    byte_progress: blit_core::remote::transfer::ByteProgressSink,
) -> bool {
    let resolver = StdResolver;
    let result = run_delegated_pull(
        req,
        modules,
        default_root,
        delegation,
        metrics,
        transfer_id,
        &tx,
        &resolver,
        &byte_progress,
    )
    .await;

    match result {
        Ok(()) => true,
        Err(error_progress) => {
            // Surface the phased error to the CLI. We use a one-shot
            // send-and-ignore here: if the CLI has already disconnected we
            // can't (and don't need to) report.
            let _ = tx.send(Ok(error_progress)).await;
            false
        }
    }
}

/// Inner driver. Splitting the error-emit path out of the public
/// entry point lets us write `?` on every step and have the dispatcher
/// route a single error onto the response channel.
async fn run_delegated_pull<R: HostResolver + ?Sized>(
    req: DelegatedPullRequest,
    modules: Arc<tokio::sync::Mutex<std::collections::HashMap<String, ModuleConfig>>>,
    default_root: Option<RootExport>,
    delegation: Arc<crate::delegation_gate::DelegationConfig>,
    metrics: Arc<TransferMetrics>,
    transfer_id: String,
    tx: &mpsc::Sender<Result<DelegatedPullProgress, Status>>,
    resolver: &R,
    byte_progress: &blit_core::remote::transfer::ByteProgressSink,
) -> Result<(), DelegatedPullProgress> {
    use blit_core::generated::delegated_pull_error::Phase;

    // Step 1: parse RemoteSourceLocator.
    let src = req
        .src
        .as_ref()
        .ok_or_else(|| err_progress(Phase::DelegationRejected as i32, "missing source locator"))?;
    if src.host.trim().is_empty() {
        return Err(err_progress(
            Phase::DelegationRejected as i32,
            "source locator host is empty",
        ));
    }
    let port_u16: u16 = src
        .port
        .try_into()
        .map_err(|_| err_progress(Phase::DelegationRejected as i32, "source port out of range"))?;
    let locator = LocatorView {
        host: &src.host,
        port: port_u16,
    };

    // Step 2: spec validation. Routes through the same
    // NormalizedTransferOperation::from_spec boundary push/pull_sync
    // use — validates spec_version, FilterSpec globs, and
    // contradictory flag combinations BEFORE we do any DNS, outbound
    // connect, or manifest enumeration (R30-F3).
    let spec = req
        .spec
        .ok_or_else(|| err_progress(Phase::DelegationRejected as i32, "missing transfer spec"))?;
    let spec =
        validate_spec(spec).map_err(|msg| err_progress(Phase::DelegationRejected as i32, msg))?;

    // Step 3: daemon-wide gate (master switch + allowlist matching +
    // DNS-rebinding mitigation by binding to the resolved IP).
    let resolved = validate_source(&delegation, &locator, resolver)
        .await
        .map_err(|denial: GateDenial| {
            err_progress(Phase::DelegationRejected as i32, denial.reason())
        })?;

    // Step 4: module metadata lookup. resolve_module honors empty-name
    // → default-root and rejects unknown modules.
    let module = resolve_module(&modules, default_root.as_ref(), &req.dst_module)
        .await
        .map_err(|status| {
            err_progress(
                Phase::DelegationRejected as i32,
                status.message().to_string(),
            )
        })?;

    // Step 5: per-module narrowing override.
    if !module.delegation_allowed {
        return Err(err_progress(
            Phase::DelegationRejected as i32,
            format!(
                "module '{}' opts out of being a delegation destination",
                module.name
            ),
        ));
    }

    if module.read_only {
        return Err(err_progress(
            Phase::DelegationRejected as i32,
            format!("module '{}' is read-only", module.name),
        ));
    }

    // Step 6: F2 canonical-path containment on dst_destination_path.
    let dst_rel = if req.dst_destination_path.trim().is_empty() {
        PathBuf::from(".")
    } else {
        PathBuf::from(&req.dst_destination_path)
    };
    let dest_root = resolve_contained_path(&module, &dst_rel)
        .map_err(|status| err_progress(Phase::Apply as i32, status.message().to_string()))?;

    // Step 7: metrics RAII. inc_pull because — from this daemon's
    // perspective — the body of work is a pull from src.
    metrics.inc_pull();
    let _guard = Arc::clone(&metrics).enter_transfer();

    // Step 8: mandatory client_capabilities override (R25-F2). The
    // CLI is not the byte recipient — dst is. Whatever the CLI sent
    // here is non-authoritative; we rewrite unconditionally.
    let spec = apply_dst_capabilities_override(spec);

    // Step 9: outbound connect. The endpoint host is the validated
    // IP literal — no further DNS resolution between check and
    // connect. The gRPC channel takes the URI directly.
    let endpoint_host = match resolved.ip() {
        std::net::IpAddr::V4(v4) => v4.to_string(),
        std::net::IpAddr::V6(v6) => format!("[{}]", v6),
    };
    let endpoint = RemoteEndpoint {
        host: endpoint_host,
        port: resolved.port(),
        // The endpoint is transport-only (R25-F1). The spec carries
        // the authoritative module + source_path. We set
        // RemotePath::Discovery here as a defensive sentinel: any
        // accidental read of self.endpoint.path inside
        // pull_sync_with_spec would bail loudly on the Discovery
        // variant rather than silently work with stale identity.
        path: RemotePath::Discovery,
    };
    // audit-1: bound the TCP connect to the source daemon. A firewalled
    // or black-holed source would otherwise block the handler for the OS
    // TCP SYN timeout (60-180s on Linux), pinning the ActiveJobs row and
    // resources. 30s matches the data-plane accept timeout.
    let mut pull_client =
        crate::net_timeout::within(SOURCE_CONNECT_TIMEOUT, RemotePullClient::connect(endpoint))
            .await
            .ok_or_else(|| {
                err_progress(
                    Phase::ConnectSource as i32,
                    format!(
                        "connecting to source {}:{} timed out after {:?}",
                        resolved.ip(),
                        resolved.port(),
                        SOURCE_CONNECT_TIMEOUT
                    ),
                )
            })?
            .map_err(|err| {
                err_progress(
                    Phase::ConnectSource as i32,
                    format!(
                        "connecting to source {}:{}: {}",
                        resolved.ip(),
                        resolved.port(),
                        err
                    ),
                )
            })?;

    // Send the "started" progress event so CLI knows the dst→src
    // handshake is underway. The diagnostic source_data_plane_endpoint
    // is filled in once data-plane negotiation completes; for now we
    // surface the validated source IP/port. (CLI tests rely on the
    // CLI-side byte counter for the load-bearing isolation assertion;
    // this field is informational only.)
    let _ = tx
        .send(Ok(DelegatedPullProgress {
            payload: Some(ProgressPayload::Started(DelegatedPullStarted {
                source_data_plane_endpoint: format!("tcp:{}:{}", resolved.ip(), resolved.port()),
                stream_count: 0,
                transfer_id: transfer_id.clone(),
            })),
        }))
        .await;

    // Step 10: build a local manifest of the destination tree (so
    // src can decide what to send) and run pull_sync_with_spec. The
    // checksum_required hint follows the spec — if compare_mode is
    // Checksum, the source will need authoritative dest checksums to
    // decide which files to transfer.
    let want_checksums = spec.compare_mode == ComparisonMode::Checksum as i32;
    let local_manifest = enumerate_local_manifest(&dest_root, want_checksums)
        .await
        .map_err(|err| {
            err_progress(
                Phase::Apply as i32,
                format!("enumerating destination manifest: {err}"),
            )
        })?;

    // Step 11/12: progress forwarding + cancellation.
    //
    // pull_sync_with_spec doesn't currently expose a streaming progress
    // adapter that maps to DelegatedPullProgress directly. For 0.1.0 we
    // call it as a single await (no per-chunk progress passthrough);
    // the final summary is the load-bearing event. A bounded
    // RemoteTransferProgress sink can be added later.
    //
    // Cancellation: if the CLI disconnects, this future is dropped by
    // the gRPC server's stream cancellation; the inner pull is then
    // dropped too, propagating cleanup through the existing pull
    // cancellation paths.
    //
    // Capture mirror_mode before moving `spec` into pull_sync_with_spec
    // — we need it after the call to gate `apply_delete_list` (R32-F1).
    let spec_mirror_mode = spec.mirror_mode;
    let report = pull_client
        .pull_sync_with_spec(
            &dest_root,
            local_manifest,
            spec,
            /* track_paths = */ false,
            None,
            Some(byte_progress),
        )
        .await
        .map_err(|err| {
            // Errors here can be from the negotiate/transfer/apply
            // phases. Preserve pull_sync_with_spec's typed
            // negotiation boundary so source-side refusal surfaces as
            // `NEGOTIATE` instead of a generic transfer failure (R37-F1).
            if err
                .downcast_ref::<PullSyncError>()
                .is_some_and(PullSyncError::is_negotiation)
            {
                err_progress(Phase::Negotiate as i32, err.to_string())
            } else {
                err_progress(Phase::Transfer as i32, format!("delegated pull: {err}"))
            }
        })?;

    // R30-F1 + R32-F1: apply the daemon-authoritative mirror delete
    // list, but ONLY when this transfer is actually a mirror. The CLI
    // path at remote.rs:304 gates on `mirror_mode`; the delegated
    // handler must match. Without the gate, a buggy or hostile source
    // daemon could attach a non-empty `paths_to_delete` to a plain
    // copy and we would dutifully unlink in-scope destination files.
    //
    // We read the mirror mode off the validated wire spec — it has
    // already passed `NormalizedTransferOperation::from_spec`, so
    // unknown values are rejected at the boundary. Only the active
    // deletion modes (FilteredSubset / All) trigger
    // `apply_delete_list`; Off / Unspecified silently ignore any
    // delete list the source attached.
    let entries_deleted_locally = if delete_list_authorized(spec_mirror_mode) {
        if let Some(ref delete_paths) = report.paths_to_delete {
            apply_delete_list(&dest_root, delete_paths)
                .await
                .map_err(|err| err_progress(Phase::Apply as i32, err))?
        } else {
            0
        }
    } else {
        // Plain copy: we ignore any delete list. Don't surface it as
        // entries_deleted on the summary either.
        0
    };

    // Optional manifest_batch event for symmetry with normal pull
    // progress shape (CLIs may render an aggregate count).
    if let Some(batch_count) = report.summary.as_ref().map(|s| s.files_transferred) {
        let _ = tx
            .send(Ok(DelegatedPullProgress {
                payload: Some(ProgressPayload::ManifestBatch(ProtoManifestBatch {
                    file_count: batch_count,
                    total_bytes: report
                        .summary
                        .as_ref()
                        .map(|s| s.bytes_transferred)
                        .unwrap_or(0),
                })),
            }))
            .await;
    }

    let summary = build_summary(&report.summary, &resolved, entries_deleted_locally);
    let _ = tx
        .send(Ok(DelegatedPullProgress {
            payload: Some(ProgressPayload::Summary(summary)),
        }))
        .await;
    Ok(())
}

/// Apply the daemon's authoritative mirror delete list against the
/// destination tree. Every path is routed through
/// `safe_join_contained` (R5-F1 lexical check + R46-F3 canonical
/// containment) before the unlink. R58-F3 closed the symmetry gap:
/// the non-delegated `blit_app::transfers::remote::delete_listed_paths`
/// upgraded to `safe_join_contained` in R46-F3 (the helper lived in
/// `blit-cli` at the time of that fix; Phase 5 A.0 moved it to
/// `blit-app`), but this daemon-side delegated path was still on bare
/// `safe_join`. With `dest_root/link → /outside` a peer-controlled
/// delete-list entry like `link/victim` would have removed an
/// outside file.
///
/// Returns the count of files actually removed (after which the
/// caller may surface it via `entries_deleted` on the summary). On
/// error returns a phase-bearing message string for `Phase::Apply`.
async fn apply_delete_list(
    dest_root: &Path,
    relative_paths: &[String],
) -> std::result::Result<u64, String> {
    use blit_core::path_safety::{canonical_dest_root, safe_join_contained};
    use std::collections::BTreeSet;

    // R58-F3: capture the canonical destination root once. Fail
    // closed if it can't be canonicalized — on the destructive
    // side, lexical-only fallback would be the bug we're closing.
    let canonical = canonical_dest_root(dest_root).map_err(|e| {
        format!(
            "cannot canonicalize destination '{}' for mirror-purge containment: {e:#}",
            dest_root.display()
        )
    })?;

    let mut files_deleted: u64 = 0;
    let mut candidate_parents: BTreeSet<PathBuf> = BTreeSet::new();

    for rel in relative_paths {
        let target = safe_join_contained(&canonical, dest_root, rel)
            .map_err(|e| format!("source delete list contained unsafe path '{rel}': {e:#}"))?;
        // safe_join("") returns dest_root itself; we never delete it.
        if target == dest_root {
            return Err("source delete list referenced the destination root itself".to_string());
        }
        match tokio::fs::remove_file(&target).await {
            Ok(()) => {
                files_deleted += 1;
                let mut p = target.parent();
                while let Some(parent) = p {
                    if parent == dest_root {
                        break;
                    }
                    candidate_parents.insert(parent.to_path_buf());
                    p = parent.parent();
                }
            }
            // Already gone: source's view may lag. Not an error.
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
            Err(e) => {
                return Err(format!("failed to delete {}: {e}", target.display()));
            }
        }
    }

    // Prune empty directories deepest-first. Failures here are
    // ignored (dir not empty / dir doesn't exist) — same posture as
    // `blit_app::transfers::remote::delete_listed_paths`.
    let mut dirs: Vec<_> = candidate_parents.into_iter().collect();
    dirs.sort_by_key(|p| std::cmp::Reverse(p.components().count()));
    for dir in dirs {
        let _ = tokio::fs::remove_dir(&dir).await;
    }

    Ok(files_deleted)
}

fn build_summary(
    inner: &Option<PullSummary>,
    resolved: &std::net::SocketAddr,
    entries_deleted_locally: u64,
) -> DelegatedPullSummary {
    let s = inner.as_ref();
    // R34-F1: report what *this destination* actually deleted —
    // never the source-attested count. The source-side
    // `PullSummary.entries_deleted` is what the source thought we
    // should delete; in plain copy mode (gated post-R32-F1) we do
    // not delete anything locally, so falling back to the source
    // count would lie on the summary. The dst is now the only
    // authority on what happened on the dst filesystem.
    DelegatedPullSummary {
        files_transferred: s.map(|x| x.files_transferred).unwrap_or(0),
        bytes_transferred: s.map(|x| x.bytes_transferred).unwrap_or(0),
        bytes_zero_copy: s.map(|x| x.bytes_zero_copy).unwrap_or(0),
        tcp_fallback_used: s.map(|x| x.tcp_fallback_used).unwrap_or(false),
        entries_deleted: entries_deleted_locally,
        // Diagnostic only (R23-F4). The destination's view; not
        // load-bearing for byte-path isolation.
        source_peer_observed: format!("{}:{}", resolved.ip(), resolved.port()),
    }
}

/// Walk the destination tree and emit a manifest the source can use
/// to decide which files need transfer. Mirror of
/// `blit_app::transfers::remote::enumerate_local_manifest` (which was
/// the CLI-side helper pre-Phase 5 A.0, now a shared library helper)
/// but lives on the daemon side because the destination owns this
/// view in the delegated path. Sequential walk; for a 0.1.0 release
/// targeting modest module sizes this is fine. Parallelize later if
/// the manifest enumeration becomes the bottleneck.
async fn enumerate_local_manifest(
    root: &Path,
    compute_checksums: bool,
) -> std::result::Result<Vec<FileHeader>, std::io::Error> {
    use blit_core::checksum::{hash_file, ChecksumType};
    use walkdir::WalkDir;

    if !root.exists() {
        return Ok(Vec::new());
    }

    let root_path = root.to_path_buf();
    let manifest = tokio::task::spawn_blocking(move || -> std::io::Result<Vec<FileHeader>> {
        let mut out = Vec::new();
        for entry in WalkDir::new(&root_path).into_iter().filter_map(|e| e.ok()) {
            if !entry.file_type().is_file() {
                continue;
            }
            let path = entry.path();
            let rel = match path.strip_prefix(&root_path) {
                Ok(rel) => rel,
                Err(_) => continue,
            };
            let relative_path = rel
                .iter()
                .map(|c| c.to_string_lossy())
                .collect::<Vec<_>>()
                .join("/");
            let meta = match std::fs::metadata(path) {
                Ok(m) => m,
                Err(_) => continue,
            };
            let mtime_seconds = meta
                .modified()
                .ok()
                .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
                .map(|d| d.as_secs() as i64)
                .unwrap_or(0);
            let checksum = if compute_checksums {
                hash_file(path, ChecksumType::Blake3).unwrap_or_default()
            } else {
                Vec::new()
            };
            out.push(FileHeader {
                relative_path,
                size: meta.len(),
                mtime_seconds,
                permissions: 0,
                checksum,
            });
        }
        Ok(out)
    })
    .await
    .map_err(|e| std::io::Error::other(format!("manifest task panicked: {e}")))??;

    Ok(manifest)
}

#[cfg(test)]
mod tests {
    //! Unit tests pinning the contracts called out in
    //! `docs/plan/REMOTE_REMOTE_DELEGATION_PLAN.md` Phase 1.6:
    //!
    //!   * R25-F2 — `client_capabilities` is unconditionally
    //!     overridden by the destination, regardless of what the CLI
    //!     supplied.
    //!   * R21-F6 — `spec_version` rejection is explicit, not
    //!     dependent on protobuf unknown-field detection.
    //!
    //! Allowlist matching, DNS-rebinding mitigation, IDNA, and the
    //! loopback IP-form rule are tested in
    //! `crates/blit-daemon/src/delegation_gate.rs`. Wire-equivalence
    //! and endpoint-isolation for the spec extraction live in
    //! `crates/blit-core/src/remote/pull.rs::spec_extraction_tests`.
    //! End-to-end byte-path isolation lands in Phase 2 integration.

    use super::*;
    use blit_core::generated::PeerCapabilities;

    fn spec_with_caps(caps: PeerCapabilities) -> TransferOperationSpec {
        TransferOperationSpec {
            spec_version: 2,
            module: "m".into(),
            source_path: ".".into(),
            filter: None,
            compare_mode: 0,
            mirror_mode: 0,
            resume: None,
            client_capabilities: Some(caps),
            force_grpc: false,
            ignore_existing: false,
            require_complete_scan: false,
            receiver_capacity: None,
            metadata_only: false,
        }
    }

    // ── R25-F2: client_capabilities mandatory override ───────────────

    #[test]
    fn dst_override_replaces_cli_supplied_caps_unconditionally() {
        // CLI tries to claim the destination doesn't support tar
        // shards. The destination supports them. The override must
        // rewrite the field — silently honoring the CLI's claim
        // would let a misbehaving CLI degrade the wire shape.
        let cli_caps = PeerCapabilities {
            supports_resume: false,
            supports_tar_shards: false,
            supports_data_plane_tcp: false,
            supports_filter_spec: false,
            // The CLI under-claims resize support; dst DOES resize
            // (ue-r2-2), and dst — not the CLI — is the byte
            // recipient, so the override must assert dst's own truth
            // in both directions.
            supports_stream_resize: false,
        };
        let spec_in = spec_with_caps(cli_caps);
        let spec_out = apply_dst_capabilities_override(spec_in);

        let caps_out = spec_out
            .client_capabilities
            .as_ref()
            .expect("override populates client_capabilities");
        assert!(caps_out.supports_resume);
        assert!(caps_out.supports_tar_shards);
        assert!(caps_out.supports_data_plane_tcp);
        assert!(caps_out.supports_filter_spec);
        assert!(
            caps_out.supports_stream_resize,
            "ue-r2-2: dst advertises its real resize support"
        );
    }

    #[test]
    fn dst_override_replaces_cli_supplied_receiver_capacity() {
        // ue-r2-1b/1e: the byte recipient in delegation is dst, so a
        // CLI-supplied receiver profile is non-authoritative — the
        // override must replace it with dst's OWN profile; leaking the
        // CLI value through would hand the src sender a fabricated
        // capacity ceiling.
        let mut spec_in = spec_with_caps(dst_capabilities());
        spec_in.receiver_capacity = Some(blit_core::generated::CapacityProfile {
            cpu_cores: 999,
            drain_class: 0,
            load_percent: 0,
            max_streams: 4096,
            drain_rate_bytes_per_sec: u64::MAX,
            max_chunk_bytes: u64::MAX,
            max_inflight_bytes: u64::MAX,
        });
        let spec_out = apply_dst_capabilities_override(spec_in);
        let profile = spec_out
            .receiver_capacity
            .expect("dst stamps its own profile");
        assert_eq!(profile, blit_core::engine::local_receiver_capacity());
        assert_ne!(profile.max_streams, 4096, "CLI ceiling must not leak");
    }

    #[test]
    fn dst_override_preserves_every_non_capabilities_field() {
        // Apart from client_capabilities, the spec must travel
        // verbatim — that's the R21-F1 contract. A regression that
        // accidentally rebuilt other fields would silently change
        // operator intent.
        let mut spec_in = TransferOperationSpec {
            spec_version: 2,
            module: "alpha".into(),
            source_path: "x/y".into(),
            filter: None,
            compare_mode: ComparisonMode::Checksum as i32,
            mirror_mode: 3, // MirrorMode::All
            resume: Some(blit_core::generated::ResumeSettings {
                enabled: true,
                block_size: 4096,
            }),
            client_capabilities: Some(PeerCapabilities {
                supports_resume: false,
                supports_tar_shards: false,
                supports_data_plane_tcp: false,
                supports_filter_spec: false,
                supports_stream_resize: false,
            }),
            force_grpc: true,
            ignore_existing: true,
            require_complete_scan: false,
            receiver_capacity: None,
            // ue-r2-1h: non-default so preservation is provable below.
            metadata_only: true,
        };
        let snapshot_module = spec_in.module.clone();
        let snapshot_source_path = spec_in.source_path.clone();
        let snapshot_compare = spec_in.compare_mode;
        let snapshot_mirror = spec_in.mirror_mode;
        let snapshot_resume = spec_in.resume;
        let snapshot_force_grpc = spec_in.force_grpc;
        let snapshot_ignore_existing = spec_in.ignore_existing;
        let snapshot_metadata_only = spec_in.metadata_only;
        // Move spec_in by value through the override.
        spec_in = apply_dst_capabilities_override(spec_in);
        assert_eq!(spec_in.module, snapshot_module);
        assert_eq!(spec_in.source_path, snapshot_source_path);
        assert_eq!(spec_in.compare_mode, snapshot_compare);
        assert_eq!(spec_in.mirror_mode, snapshot_mirror);
        assert_eq!(spec_in.resume, snapshot_resume);
        assert_eq!(spec_in.force_grpc, snapshot_force_grpc);
        assert_eq!(spec_in.ignore_existing, snapshot_ignore_existing);
        assert_eq!(spec_in.metadata_only, snapshot_metadata_only);
    }

    #[test]
    fn dst_override_populates_when_cli_left_field_unset() {
        // The CLI is allowed to leave client_capabilities unset
        // entirely (the field is non-authoritative anyway). The
        // destination must still emit a populated value on the wire.
        let mut spec_in = spec_with_caps(PeerCapabilities::default());
        spec_in.client_capabilities = None;
        let spec_out = apply_dst_capabilities_override(spec_in);
        assert!(spec_out.client_capabilities.is_some());
        let caps = spec_out.client_capabilities.as_ref().unwrap();
        // `dst_capabilities()` is the source of truth.
        assert_eq!(*caps, dst_capabilities());
    }

    // ── R21-F6 / R25 spec_version: explicit allowlist ────────────────

    #[test]
    fn validate_spec_accepts_known_version_and_clean_fields() {
        let spec = spec_with_caps(PeerCapabilities::default());
        let out = validate_spec(spec.clone()).expect("clean spec validates");
        // Wire spec must travel verbatim through validation.
        assert_eq!(out, spec);
    }

    #[test]
    fn validate_spec_rejects_unknown_spec_version_explicitly() {
        // Important: the rejection comes from the explicit
        // NormalizedTransferOperation::from_spec check, not from
        // protobuf unknown-field detection. Per R21-F6 we do not
        // rely on prost dropping unknown fields.
        let mut spec = spec_with_caps(PeerCapabilities::default());
        spec.spec_version = 99;
        let err = validate_spec(spec).unwrap_err();
        assert!(
            err.contains("spec_version") && err.contains("99"),
            "expected version-rejection message mentioning 99, got: {err}"
        );
    }

    #[test]
    fn validate_spec_rejects_zero_spec_version() {
        // spec_version 0 is what proto3 sees when the sender omits
        // the field. We require explicit version setting.
        let mut spec = spec_with_caps(PeerCapabilities::default());
        spec.spec_version = 0;
        let err = validate_spec(spec).unwrap_err();
        assert!(err.contains("spec_version"));
    }

    #[test]
    fn validate_spec_rejects_metadata_only() {
        // ue-r2-1h review (panel F1): a forwarded metadata_only spec
        // would make the source stream bare headers that this daemon's
        // pull_sync client loop materializes as zero-byte files —
        // silent destination truncation reported as success. Fail
        // closed at the validation boundary.
        let mut spec = spec_with_caps(PeerCapabilities::default());
        spec.metadata_only = true;
        let err = validate_spec(spec).unwrap_err();
        assert!(
            err.contains("metadata_only"),
            "expected metadata_only rejection, got: {err}"
        );
    }

    #[test]
    fn validate_spec_rejects_contradictory_force_plus_ignore_existing() {
        // Per from_spec at operation_spec.rs:104: ignore_existing=true
        // with Force compare_mode is contradictory and rejected. The
        // delegated handler must catch this at the boundary, before
        // any DNS/connect/manifest work — exactly like push and
        // pull_sync. (R30-F3.)
        let mut spec = spec_with_caps(PeerCapabilities::default());
        spec.compare_mode = blit_core::generated::ComparisonMode::Force as i32;
        spec.ignore_existing = true;
        let err = validate_spec(spec).unwrap_err();
        assert!(
            err.contains("ignore_existing") || err.contains("Force"),
            "expected contradictory-flag rejection, got: {err}"
        );
    }

    // ── R34-F1: build_summary reports only the local delete count ────

    #[test]
    fn build_summary_reports_local_entries_deleted_count_not_source_side() {
        // Plain copy: local count is 0 (delete list never applied).
        // The source-attested count must NOT leak through. Pre-R34-F1
        // we fell back to source-side when local was 0, which would
        // mis-report on a copy if the source attached a non-zero
        // entries_deleted.
        let resolved: std::net::SocketAddr = "127.0.0.1:9031".parse().unwrap();
        let inner = Some(PullSummary {
            files_transferred: 5,
            bytes_transferred: 1024,
            bytes_zero_copy: 0,
            tcp_fallback_used: false,
            entries_deleted: 7, // source claims 7 — must be ignored
        });
        let summary = build_summary(&inner, &resolved, /* local = */ 0);
        assert_eq!(summary.entries_deleted, 0);
        assert_eq!(summary.files_transferred, 5);
        assert_eq!(summary.bytes_transferred, 1024);
    }

    #[test]
    fn build_summary_reports_local_entries_deleted_count_in_mirror_mode() {
        // Mirror: local count is what was actually unlinked. The
        // source-side count is informational at best (it tells us
        // what was supposed to happen, not what did).
        let resolved: std::net::SocketAddr = "127.0.0.1:9031".parse().unwrap();
        let inner = Some(PullSummary {
            files_transferred: 0,
            bytes_transferred: 0,
            bytes_zero_copy: 0,
            tcp_fallback_used: false,
            entries_deleted: 99, // source claim, ignored
        });
        let summary = build_summary(&inner, &resolved, /* local = */ 3);
        assert_eq!(summary.entries_deleted, 3);
    }

    #[test]
    fn build_summary_zero_when_no_inner_and_no_local() {
        let resolved: std::net::SocketAddr = "127.0.0.1:9031".parse().unwrap();
        let summary = build_summary(&None, &resolved, 0);
        assert_eq!(summary.entries_deleted, 0);
        assert_eq!(summary.files_transferred, 0);
    }

    // ── R32-F1: delete-list gating on validated MirrorMode ───────────

    #[test]
    fn delete_list_authorized_only_for_active_mirror_modes() {
        // The CLI gates delete_listed_paths on mirror_mode at
        // remote.rs:304. The delegated daemon handler must match —
        // otherwise a buggy or hostile source daemon attaching a
        // non-empty paths_to_delete to a plain copy would cause the
        // destination to delete in-scope files.
        assert!(!delete_list_authorized(
            blit_core::generated::MirrorMode::Unspecified as i32
        ));
        assert!(!delete_list_authorized(
            blit_core::generated::MirrorMode::Off as i32
        ));
        assert!(delete_list_authorized(
            blit_core::generated::MirrorMode::FilteredSubset as i32
        ));
        assert!(delete_list_authorized(
            blit_core::generated::MirrorMode::All as i32
        ));
    }

    #[test]
    fn delete_list_authorized_rejects_unknown_mirror_mode_value() {
        // Any other proto3 value — e.g. a future MirrorMode variant
        // we don't yet recognize — must not authorize deletes. The
        // matching is allowlist-style on the named variants we know
        // are deletion-active. (NormalizedTransferOperation::from_spec
        // rejects unknown variants outright at the validate boundary;
        // this defense-in-depth check guards against a future
        // refactor accidentally widening the set.)
        assert!(!delete_list_authorized(99));
        assert!(!delete_list_authorized(-1));
    }

    #[test]
    fn validate_spec_rejects_malformed_filter_glob() {
        // A FilterSpec with a malformed include glob must be rejected
        // by from_spec's filter normalization. This is the kind of
        // surface the handler must NOT pass through to the source
        // daemon — silently sending a malformed filter would let the
        // source's own from_spec reject it later, but only after the
        // delegation gate / outbound connect has already burned
        // resources. (R30-F3.)
        let mut spec = spec_with_caps(PeerCapabilities::default());
        spec.filter = Some(blit_core::generated::FilterSpec {
            include: vec!["[invalid-glob".into()],
            exclude: vec![],
            min_size: None,
            max_size: None,
            min_age_secs: None,
            max_age_secs: None,
            files_from: vec![],
        });
        // We deliberately do not depend on a specific message here —
        // the contract is "from_spec rejects malformed globs", and
        // its message wording can vary across glob crate versions.
        assert!(validate_spec(spec).is_err());
    }

    // ── R58-F3: apply_delete_list canonical containment ─────────────

    /// A delete-list entry that traverses a pre-existing escape
    /// symlink under `dest_root` MUST be rejected by the
    /// destination daemon's mirror-purge step. Pre-fix this path
    /// used bare `safe_join` (lexical-only); a peer-controlled
    /// `link/victim` entry would have removed
    /// `/outside/victim`.
    #[cfg(unix)]
    #[tokio::test]
    async fn apply_delete_list_rejects_symlink_escape() {
        use std::os::unix::fs::symlink;
        let tmp = tempfile::tempdir().unwrap();
        let dest_root = tmp.path().join("dst");
        let outside = tmp.path().join("outside");
        std::fs::create_dir_all(&dest_root).unwrap();
        std::fs::create_dir_all(&outside).unwrap();
        std::fs::write(outside.join("victim.txt"), b"do not lose").unwrap();
        // Pre-existing escape symlink under dest_root.
        symlink(&outside, dest_root.join("link")).unwrap();

        let err = apply_delete_list(&dest_root, &["link/victim.txt".to_string()])
            .await
            .expect_err("R58-F3: delete through escape symlink must reject");
        assert!(
            err.contains("escape") || err.contains("escapes"),
            "expected canonical-escape rejection, got: {err}"
        );
        // The outside file must be intact.
        assert!(
            outside.join("victim.txt").exists(),
            "victim file must survive — apply_delete_list rejected the unsafe entry"
        );
    }

    /// Sanity: in-scope deletes still work after R58-F3.
    #[tokio::test]
    async fn apply_delete_list_removes_in_scope_entries() {
        let tmp = tempfile::tempdir().unwrap();
        let dest_root = tmp.path().join("dst");
        std::fs::create_dir_all(&dest_root).unwrap();
        std::fs::write(dest_root.join("victim.txt"), b"goodbye").unwrap();

        let count = apply_delete_list(&dest_root, &["victim.txt".to_string()])
            .await
            .expect("in-scope delete should succeed");
        assert_eq!(count, 1);
        assert!(!dest_root.join("victim.txt").exists());
    }

    /// R-followup: `handle_delegated_pull` must return `false` when
    /// the inner pipeline fails so the caller (`service/core.rs`)
    /// can `inc_error` and log `ok=false`. Pre-fix the function
    /// returned `()` and a real failure showed `ok=true` on the
    /// `--metrics` line. Drive the failure with the cheapest path:
    /// a request with a blank source locator host, which trips the
    /// `DelegationRejected` phase before any DNS/connect/IO.
    #[tokio::test]
    async fn handle_delegated_pull_returns_false_on_handler_failure() {
        use blit_core::generated::{
            DelegatedPullRequest, RemoteSourceLocator, TransferOperationSpec,
        };

        let req = DelegatedPullRequest {
            dst_module: String::new(),
            dst_destination_path: String::new(),
            src: Some(RemoteSourceLocator {
                host: String::new(), // blank → DelegationRejected at step 1
                port: 0,
            }),
            spec: Some(TransferOperationSpec {
                spec_version: 1,
                ..Default::default()
            }),
            trace_data_plane: false,
            detach: false,
        };
        let modules = Arc::new(tokio::sync::Mutex::new(std::collections::HashMap::<
            String,
            ModuleConfig,
        >::new()));
        let delegation = Arc::new(crate::delegation_gate::DelegationConfig::default());
        let metrics = TransferMetrics::disabled();
        let (tx, _rx) = mpsc::channel(8);

        let ok = handle_delegated_pull(
            req,
            modules,
            None,
            delegation,
            metrics,
            tx,
            "t-test".to_string(),
            blit_core::remote::transfer::ByteProgressSink::new(),
        )
        .await;
        assert!(
            !ok,
            "handler with blank source locator must return false so caller can inc_error"
        );
    }
}
