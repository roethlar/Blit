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
use blit_core::remote::pull::RemotePullClient;
use blit_core::remote::transfer::operation_spec::NormalizedTransferOperation;
use tokio::sync::mpsc;
use tonic::Status;

use crate::delegation_gate::{
    validate_source, GateDenial, HostResolver, LocatorView, StdResolver,
};
use crate::metrics::TransferMetrics;
use crate::runtime::{ModuleConfig, RootExport};
use crate::service::util::{resolve_contained_path, resolve_module};

/// Per-request capabilities advertised by *this destination daemon* on
/// the spec it forwards to src. The CLI's value is overwritten — the
/// CLI is not in the byte path and cannot speak for what dst supports.
pub(crate) fn dst_capabilities() -> PeerCapabilities {
    PeerCapabilities {
        supports_resume: true,
        supports_tar_shards: true,
        supports_data_plane_tcp: true,
        supports_filter_spec: true,
    }
}

/// Mandatory `client_capabilities` override (R25-F2). The CLI is not
/// the byte recipient in delegation; the destination is. Whatever
/// `client_capabilities` the CLI put on the spec must be replaced
/// with the destination's actual capabilities before the spec leaves
/// for src. Unconditional — no merging, no field-level fallback.
pub(crate) fn apply_dst_capabilities_override(mut spec: TransferOperationSpec) -> TransferOperationSpec {
    spec.client_capabilities = Some(dst_capabilities());
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
pub(crate) async fn handle_delegated_pull(
    req: DelegatedPullRequest,
    modules: Arc<tokio::sync::Mutex<std::collections::HashMap<String, ModuleConfig>>>,
    default_root: Option<RootExport>,
    delegation: Arc<crate::delegation_gate::DelegationConfig>,
    metrics: Arc<TransferMetrics>,
    tx: mpsc::Sender<Result<DelegatedPullProgress, Status>>,
) {
    let resolver = StdResolver;
    let result =
        run_delegated_pull(req, modules, default_root, delegation, metrics, &tx, &resolver).await;

    if let Err(error_progress) = result {
        // Surface the phased error to the CLI. We use a one-shot
        // send-and-ignore here: if the CLI has already disconnected we
        // can't (and don't need to) report.
        let _ = tx.send(Ok(error_progress)).await;
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
    tx: &mpsc::Sender<Result<DelegatedPullProgress, Status>>,
    resolver: &R,
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
    let spec = validate_spec(spec)
        .map_err(|msg| err_progress(Phase::DelegationRejected as i32, msg))?;

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
            err_progress(Phase::DelegationRejected as i32, status.message().to_string())
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
    let dest_root = resolve_contained_path(&module, &dst_rel).map_err(|status| {
        err_progress(Phase::Apply as i32, status.message().to_string())
    })?;

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
    let mut pull_client = RemotePullClient::connect(endpoint).await.map_err(|err| {
        err_progress(
            Phase::ConnectSource as i32,
            format!("connecting to source {}:{}: {}", resolved.ip(), resolved.port(), err),
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
        .pull_sync_with_spec(&dest_root, local_manifest, spec, /* track_paths = */ false, None)
        .await
        .map_err(|err| {
            // Errors here can be from the negotiate/transfer/apply
            // phases; surface verbatim and tag with a best-guess
            // phase based on whether we got any summary.
            err_progress(Phase::Transfer as i32, format!("delegated pull: {err}"))
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
                    total_bytes: report.summary.as_ref().map(|s| s.bytes_transferred).unwrap_or(0),
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
/// destination tree. Every path is routed through `safe_join` before
/// the unlink, so a hostile source daemon's `..`-traversal entry is
/// rejected at the chokepoint (R5-F1 of the 2026-05-01 review).
///
/// Returns the count of files actually removed (after which the
/// caller may surface it via `entries_deleted` on the summary). On
/// error returns a phase-bearing message string for `Phase::Apply`.
async fn apply_delete_list(
    dest_root: &Path,
    relative_paths: &[String],
) -> std::result::Result<u64, String> {
    use blit_core::path_safety::safe_join;
    use std::collections::BTreeSet;

    let mut files_deleted: u64 = 0;
    let mut candidate_parents: BTreeSet<PathBuf> = BTreeSet::new();

    for rel in relative_paths {
        let target = safe_join(dest_root, rel)
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
    // the CLI's `delete_listed_paths`.
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
/// to decide which files need transfer. Mirror of the CLI's
/// `enumerate_local_manifest` (`crates/blit-cli/src/transfers/remote.rs`)
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
            spec_version: 1,
            module: "m".into(),
            source_path: ".".into(),
            filter: None,
            compare_mode: 0,
            mirror_mode: 0,
            resume: None,
            client_capabilities: Some(caps),
            force_grpc: false,
            ignore_existing: false,
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
    }

    #[test]
    fn dst_override_preserves_every_non_capabilities_field() {
        // Apart from client_capabilities, the spec must travel
        // verbatim — that's the R21-F1 contract. A regression that
        // accidentally rebuilt other fields would silently change
        // operator intent.
        let mut spec_in = TransferOperationSpec {
            spec_version: 1,
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
            }),
            force_grpc: true,
            ignore_existing: true,
        };
        let snapshot_module = spec_in.module.clone();
        let snapshot_source_path = spec_in.source_path.clone();
        let snapshot_compare = spec_in.compare_mode;
        let snapshot_mirror = spec_in.mirror_mode;
        let snapshot_resume = spec_in.resume.clone();
        let snapshot_force_grpc = spec_in.force_grpc;
        let snapshot_ignore_existing = spec_in.ignore_existing;
        // Move spec_in by value through the override.
        spec_in = apply_dst_capabilities_override(spec_in);
        assert_eq!(spec_in.module, snapshot_module);
        assert_eq!(spec_in.source_path, snapshot_source_path);
        assert_eq!(spec_in.compare_mode, snapshot_compare);
        assert_eq!(spec_in.mirror_mode, snapshot_mirror);
        assert_eq!(spec_in.resume, snapshot_resume);
        assert_eq!(spec_in.force_grpc, snapshot_force_grpc);
        assert_eq!(spec_in.ignore_existing, snapshot_ignore_existing);
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
}
