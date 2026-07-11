//! Destination-side handler for the `DelegatedPull` RPC.
//!
//! The CLI calls this on the destination daemon when both endpoints in
//! a `blit copy` are remote. The destination daemon validates the
//! request through the delegation gate, then (otp-9b) initiates the
//! unified `Transfer` session against the named source as DESTINATION
//! — the same choreography every transfer runs — and relays progress
//! back to the CLI. Bytes flow source→dst directly; this RPC carries
//! trigger + progress only, never payload bytes.
//!
//! See `docs/plan/REMOTE_REMOTE_DELEGATION_PLAN.md` §4.2 for the
//! ordered handler steps and §4.3 for the gate;
//! `docs/plan/ONE_TRANSFER_PATH.md` §Design (Delegated transfer) for
//! the session reroute.

use std::path::PathBuf;
use std::sync::Arc;

use blit_core::generated::{
    delegated_pull_progress::Payload as ProgressPayload, session_error, ComparisonMode,
    DelegatedPullError, DelegatedPullProgress, DelegatedPullRequest, DelegatedPullStarted,
    DelegatedPullSummary, ManifestBatch as ProtoManifestBatch, MirrorMode, TransferOperationSpec,
};
use blit_core::remote::endpoint::{RemoteEndpoint, RemotePath};
use blit_core::remote::transfer::operation_spec::NormalizedTransferOperation;
use blit_core::remote::transfer::session_client::{
    connect_transfer_client, run_pull_session_with_client, PullSessionOptions, TransferOpenRefusal,
};
use blit_core::transfer_session::SessionFault;
use tokio::sync::mpsc;
use tonic::Status;

use crate::delegation_gate::{validate_source, GateDenial, HostResolver, LocatorView, StdResolver};
use crate::metrics::TransferMetrics;
use crate::runtime::{ModuleConfig, RootExport};
use crate::service::util::{resolve_contained_path, resolve_module};

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
    // (The ue-r2-1h metadata_only rejection that used to follow the
    // normalization died with the field at otp-10c-2 — the relay scan
    // shape it guarded against no longer exists on the wire.)
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

/// Classify a session error onto the wire error phase (codex otp-9b
/// F3): every `Transfer`-open failure is a NEGOTIATE-phase failure —
/// structurally, via [`TransferOpenRefusal`], exactly as the old typed
/// `PullSyncError` boundary treated every pre-response RPC failure
/// (R37-F1) — and a mid-session `SessionFault` phases by its code
/// (handshake refusals → NEGOTIATE; everything else → TRANSFER).
fn session_error_phase(err: &eyre::Report) -> blit_core::generated::delegated_pull_error::Phase {
    use blit_core::generated::delegated_pull_error::Phase;
    if err.downcast_ref::<TransferOpenRefusal>().is_some() {
        return Phase::Negotiate;
    }
    match err.downcast_ref::<SessionFault>().map(|f| f.code) {
        Some(
            session_error::Code::BuildMismatch
            | session_error::Code::ModuleUnknown
            | session_error::Code::ReadOnly
            | session_error::Code::DelegationRefused
            | session_error::Code::ScanIncomplete
            // otp-10b-1: refused at OPEN, before any transfer work.
            | session_error::Code::ChecksumDisabled,
        ) => Phase::Negotiate,
        _ => Phase::Transfer,
    }
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

    // Step 8 (RETIRED at otp-9b): the R25-F2 capabilities override and
    // the ue-r2-1b receiver-capacity override are satisfied by
    // construction on the session path — `run_destination` advertises
    // THIS end's own `local_receiver_capacity()` in its `SessionOpen`
    // (contract §Invariants 5: the byte receiver advertises, wherever
    // it initiates), and same-build peers make capability bits moot
    // (D-2026-07-05-2). Nothing capability-shaped from the CLI's spec
    // is consulted.

    // Step 9: outbound connect. The endpoint host is the validated
    // IP literal — no further DNS resolution between check and
    // connect. The session client's connect policy bounds it (30s,
    // the audit-1 posture — a black-holed source cannot pin this
    // handler for the OS SYN timeout).
    let endpoint_host = match resolved.ip() {
        std::net::IpAddr::V4(v4) => v4.to_string(),
        std::net::IpAddr::V6(v6) => format!("[{}]", v6),
    };
    let endpoint = RemoteEndpoint {
        host: endpoint_host,
        port: resolved.port(),
        // The spec is authoritative for module + source path (R25-F1);
        // the session client derives `SessionOpen.module/path` from
        // the endpoint's `RemotePath`.
        path: RemotePath::Module {
            module: spec.module.clone(),
            rel_path: PathBuf::from(&spec.source_path),
        },
    };
    let client = connect_transfer_client(&endpoint).await.map_err(|err| {
        err_progress(
            Phase::ConnectSource as i32,
            format!(
                "connecting to source {}:{}: {err:#}",
                resolved.ip(),
                resolved.port()
            ),
        )
    })?;

    // Send the "started" progress event so CLI knows the dst→src
    // handshake is underway. The diagnostic source_data_plane_endpoint
    // surfaces the validated source IP/port. (CLI tests rely on the
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

    // Step 10 (otp-9b): this daemon initiates the unified `Transfer`
    // session as DESTINATION against src — the same choreography as
    // every other transfer (plan §Design, Delegated transfer). The
    // source streams its manifest and THIS end diffs incrementally
    // against its module tree (the old pre-enumerated local manifest
    // dies with the bespoke driver); payload bytes ride the session's
    // TCP data plane, or the in-stream carrier when the spec asks
    // (`force_grpc`); mirror deletions run HERE via the one delete
    // rule (otp-6b). R30-F1/R32-F1 are satisfied by construction:
    // there is no source-attested delete list anymore — this end
    // deletes only what its own scan-complete-guarded diff says, and
    // only when the validated mirror mode authorizes deletions.
    //
    // Cancellation: unchanged — core.rs races this future against
    // tx.closed() and the row's CancelJob token; dropping it tears the
    // transport down and the src daemon's served session cleans up.
    let mirror_active = delete_list_authorized(spec.mirror_mode);
    let options = PullSessionOptions {
        compare_mode: ComparisonMode::try_from(spec.compare_mode)
            .unwrap_or(ComparisonMode::SizeMtime),
        ignore_existing: spec.ignore_existing,
        require_complete_scan: spec.require_complete_scan,
        in_stream_bytes: spec.force_grpc,
        resume: spec.resume.as_ref().is_some_and(|r| r.enabled),
        resume_block_size: spec.resume.as_ref().map_or(0, |r| r.block_size),
        filter: spec.filter.clone(),
        mirror_enabled: mirror_active,
        mirror_kind: if mirror_active {
            MirrorMode::try_from(spec.mirror_mode).unwrap_or(MirrorMode::Off)
        } else {
            MirrorMode::Off
        },
        byte_progress: Some(byte_progress.clone()),
        // The delegated dst daemon has no per-file progress consumer —
        // its live lane is the jobs-row byte counter above — and its
        // stderr is the daemon log, not an operator terminal, so dial
        // traces stay off (otp-10b-2 fields; verb-side only).
        progress: None,
        trace_data_plane: false,
    };
    let outcome = run_pull_session_with_client(client, &endpoint, dest_root, options)
        .await
        .map_err(|err| {
            err_progress(
                session_error_phase(&err) as i32,
                format!("delegated transfer: {err:#}"),
            )
        })?;

    // Optional manifest_batch event for symmetry with normal pull
    // progress shape (CLIs may render an aggregate count).
    let _ = tx
        .send(Ok(DelegatedPullProgress {
            payload: Some(ProgressPayload::ManifestBatch(ProtoManifestBatch {
                file_count: outcome.summary.files_transferred,
                total_bytes: outcome.summary.bytes_transferred,
            })),
        }))
        .await;

    // Summary: the session's DESTINATION-computed record IS this end's
    // authoritative account (R34-F1 by construction — the session
    // scored what this filesystem did, deletions included; the old
    // source-attested count problem cannot recur).
    let s = &outcome.summary;
    let _ = tx
        .send(Ok(DelegatedPullProgress {
            payload: Some(ProgressPayload::Summary(DelegatedPullSummary {
                files_transferred: s.files_transferred,
                bytes_transferred: s.bytes_transferred,
                bytes_zero_copy: 0,
                // Wire-compat: the old field means "the gRPC byte
                // fallback carried the payload" — on the session that
                // is the in-stream carrier.
                tcp_fallback_used: s.in_stream_carrier_used,
                entries_deleted: s.entries_deleted,
                // Diagnostic only (R23-F4). The destination's view; not
                // load-bearing for byte-path isolation.
                source_peer_observed: format!("{}:{}", resolved.ip(), resolved.port()),
            })),
        }))
        .await;
    Ok(())
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
    //! `crates/blit-daemon/src/delegation_gate.rs`. Spec-extraction
    //! coverage lives with the builder in
    //! `remote/transfer/operation_spec.rs` (relocated at otp-10c-2).
    //! End-to-end byte-path isolation: the remote_remote.rs pins.

    use super::*;

    fn wire_spec() -> TransferOperationSpec {
        TransferOperationSpec {
            spec_version: 2,
            module: "m".into(),
            source_path: ".".into(),
            filter: None,
            compare_mode: 0,
            mirror_mode: 0,
            resume: None,
            force_grpc: false,
            ignore_existing: false,
            require_complete_scan: false,
        }
    }

    // ── R21-F6 / R25 spec_version: explicit allowlist ────────────────

    #[test]
    fn validate_spec_accepts_known_version_and_clean_fields() {
        let spec = wire_spec();
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
        let mut spec = wire_spec();
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
        let mut spec = wire_spec();
        spec.spec_version = 0;
        let err = validate_spec(spec).unwrap_err();
        assert!(err.contains("spec_version"));
    }

    // (validate_spec_rejects_metadata_only died with the wire field at
    // otp-10c-2 — the rejected shape is unrepresentable.)

    #[test]
    fn validate_spec_rejects_contradictory_force_plus_ignore_existing() {
        // Per from_spec at operation_spec.rs:104: ignore_existing=true
        // with Force compare_mode is contradictory and rejected. The
        // delegated handler must catch this at the boundary, before
        // any DNS/connect/manifest work — exactly like push and
        // pull_sync. (R30-F3.)
        let mut spec = wire_spec();
        spec.compare_mode = blit_core::generated::ComparisonMode::Force as i32;
        spec.ignore_existing = true;
        let err = validate_spec(spec).unwrap_err();
        assert!(
            err.contains("ignore_existing") || err.contains("Force"),
            "expected contradictory-flag rejection, got: {err}"
        );
    }

    // ── R34-F1: build_summary reports only the local delete co
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
        let mut spec = wire_spec();
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

    /// codex otp-9b F3: phase classification is structural. EVERY
    /// Transfer-open failure — whatever its inner code — is NEGOTIATE
    /// (the old typed boundary's contract for pre-response failures);
    /// mid-session faults phase by code; a plain transport error is
    /// TRANSFER.
    #[test]
    fn session_error_phase_classifies_structurally() {
        use blit_core::generated::delegated_pull_error::Phase;

        // Open-time failure with a non-special inner code (e.g. the
        // peer returned Unavailable at open) → still NEGOTIATE.
        let open_err = eyre::Report::new(TransferOpenRefusal(SessionFault::refusal(
            session_error::Code::Internal,
            "opening Transfer RPC: unavailable",
        )));
        assert_eq!(session_error_phase(&open_err), Phase::Negotiate);

        // Mid-session handshake refusal → NEGOTIATE by code.
        let refusal = eyre::Report::new(SessionFault::refusal(
            session_error::Code::ReadOnly,
            "module is read-only",
        ));
        assert_eq!(session_error_phase(&refusal), Phase::Negotiate);

        // otp-10b-1: an operator-policy refusal at OPEN → NEGOTIATE.
        let checksum = eyre::Report::new(SessionFault::refusal(
            session_error::Code::ChecksumDisabled,
            "checksum comparison is disabled on this daemon",
        ));
        assert_eq!(session_error_phase(&checksum), Phase::Negotiate);

        // Mid-session abort (cancel) → TRANSFER.
        let cancel = eyre::Report::new(SessionFault::refusal(
            session_error::Code::Cancelled,
            "cancelled",
        ));
        assert_eq!(session_error_phase(&cancel), Phase::Transfer);

        // A bare transport error with no session identity → TRANSFER.
        let plain = eyre::eyre!("connection reset by peer");
        assert_eq!(session_error_phase(&plain), Phase::Transfer);
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
