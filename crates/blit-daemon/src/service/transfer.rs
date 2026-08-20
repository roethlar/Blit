//! ONE_TRANSFER_PATH unified `Transfer` session — daemon side.
//!
//! otp-1 (D-2026-07-05-4) landed the wire surface (the RPC, the frame
//! set, `docs/TRANSFER_SESSION.md`) with the handler refusing
//! UNIMPLEMENTED. otp-4a landed the behavior; otp-5a makes the daemon
//! serve BOTH roles: it runs `blit_core::transfer_session::run_responder`,
//! which dispatches on the client's declared initiator role — a SOURCE
//! initiator makes the daemon the DESTINATION (push-equivalent), a
//! DESTINATION initiator makes it the SOURCE (pull-equivalent, streaming
//! its module tree). The dispatcher in `core.rs::transfer` registers a
//! jobs row and races the session against cancel/hangup via
//! `resolve_transfer_session_outcome`, returning the response
//! `ReceiverStream`. This is the ONLY transfer dispatch since cutover
//! (otp-10c-2 deleted the push/pull_sync arms).
//!
//! This module owns the two daemon-specific pieces the session driver
//! in blit-core cannot: (1) the [`OpenResolver`] that maps a wire
//! module/path to a local root and read-only decision (blit-core stays
//! free of module config and `tonic::Status`), and (2) the transport
//! assembly + outcome mapping.
//!
//! Both roles ride the TCP data plane by default (otp-4b/5b), with the
//! in-stream carrier as the fallback. The dispatcher's one jobs-row byte
//! sink is attached to destination writes directly and to source payload
//! progress through a small relay, so either served role reports live bytes.

use std::collections::HashMap;
use std::sync::Arc;

use tokio::sync::mpsc;
use tokio::sync::Mutex;
use tonic::{Status, Streaming};

use blit_core::generated::session_error::Code;
use blit_core::generated::{ComparisonMode, SessionOpen, TransferFrame};
use blit_core::perf_history::{HistoryStore, Initiator, LocalRole, RecordSink, RouteTag, Topology};
use blit_core::remote::transfer::session_client::{record_session_history, SessionRecordFacts};
use blit_core::remote::transfer::{ByteProgressSink, RemoteTransferProgress};
use blit_core::transfer_session::transport::grpc_daemon_transport;
use blit_core::transfer_session::{
    run_responder, DestinationInstruments, DestinationTarget, HelloConfig, OpenResolver,
    ResolvedEndpoint, ResponderInstruments, ResponderPolicy, SessionFault, SourceInstruments,
    SourceResponderTarget,
};

use super::util::{resolve_contained_path, resolve_module, resolve_relative_path};
use crate::active_jobs::ActiveJobKind;
use crate::active_jobs::ActiveJobProgress;
use crate::runtime::{ModuleConfig, RootExport};

/// The dispatcher's open hook (review otp-10b-2 F4): called exactly once
/// per session, at the moment the received `SessionOpen` resolves
/// successfully — the first point the daemon knows what kind of
/// transfer it is serving (the initiator's declared role) and which
/// module/path it targets. `core.rs::transfer` uses it to fix up the
/// jobs row, count the right metric, and emit the `TransferStarted`
/// event with real values instead of the Push/empty placeholders.
pub(crate) type OnSessionOpen = dyn Fn(ActiveJobKind, &str, &str) + Send + Sync;

/// Wrap an [`OpenResolver`] so a successful resolve also fires the
/// dispatcher's open hook with this role's job kind and the open's
/// wire module/path. A refused open never fires it — the session dies
/// in the handshake and the placeholder row drains as before.
fn with_open_hook(
    inner: Box<OpenResolver>,
    hook: Arc<OnSessionOpen>,
    kind: ActiveJobKind,
) -> Box<OpenResolver> {
    Box::new(move |open: &SessionOpen| {
        let fut = inner(open);
        let hook = Arc::clone(&hook);
        let module = open.module.clone();
        let path = open.path.clone();
        Box::pin(async move {
            let resolved = fut.await?;
            hook(kind, &module, &path);
            Ok(resolved)
        })
    })
}

/// The open-time facts the daemon's own performance record needs
/// (ph-1c): the policy the initiator declared in `SessionOpen` and the
/// local root the open resolved to. Captured by [`with_perf_capture`]
/// at the same moment the dispatcher's open hook fires. A session
/// whose open never resolves records nothing — there is no honest row
/// to write.
#[derive(Debug, Clone)]
struct OpenPerfFacts {
    compare_mode: ComparisonMode,
    mirror_enabled: bool,
    resolved_root: std::path::PathBuf,
}

/// One served session's observer, owned by the DISPATCHER task — not
/// the raced responder future (ph-1c). The initiator legitimately
/// hangs up the moment it has what it needs, which can drop the
/// responder mid-teardown (w4-3 race); anything the daemon must still
/// know afterward — "did the session reach its terminal summary?",
/// the facts for its own performance row — lives here, filled from
/// inside the session via the open resolver and the
/// `on_terminal_summary` instruments, and read after the race settles.
pub(crate) struct ServedSessionRecorder {
    store: Option<HistoryStore>,
    peer: String,
    started: std::time::Instant,
    open: std::sync::Mutex<Option<OpenPerfFacts>>,
    terminal: std::sync::Mutex<Option<(LocalRole, blit_core::generated::TransferSummary)>>,
}

impl ServedSessionRecorder {
    pub(crate) fn new(store: Option<HistoryStore>, peer: String) -> Arc<Self> {
        Arc::new(Self {
            store,
            peer,
            started: std::time::Instant::now(),
            open: std::sync::Mutex::new(None),
            terminal: std::sync::Mutex::new(None),
        })
    }

    fn note_open(&self, facts: OpenPerfFacts) {
        *self.open.lock().unwrap_or_else(|e| e.into_inner()) = Some(facts);
    }

    fn note_terminal(&self, role: LocalRole, summary: &blit_core::generated::TransferSummary) {
        *self.terminal.lock().unwrap_or_else(|e| e.into_inner()) = Some((role, summary.clone()));
    }

    /// Whether the session reached its terminal summary — the contract
    /// point past which a peer hangup is a normal close, not a cancel.
    pub(crate) fn completed(&self) -> bool {
        self.terminal
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .is_some()
    }

    /// The terminal summary, if the session reached one — the
    /// dispatcher uses it to pin final jobs-row counts when the raced
    /// responder future was dropped before its own `finish`.
    pub(crate) fn terminal_summary(&self) -> Option<blit_core::generated::TransferSummary> {
        self.terminal
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .as_ref()
            .map(|(_, summary)| summary.clone())
    }

    /// Append this session's row to the daemon's own store (ruling
    /// R5), once the dispatcher race has settled and only for a
    /// session scored `ok`. The role is the one the responder actually
    /// played; `peer_key` follows the ph-1b client conventions viewed
    /// from this side of the wire: serving a push (DESTINATION) keys
    /// `peer_host:local_root` — the mirror image of the pull client's
    /// `src_host:dest_root` — while serving a pull (SOURCE) degrades
    /// to the host-level key, because the peer's destination root
    /// never crosses the wire. No resolvable peer host → no key: the
    /// record still lands and counts in aggregates, but a shared
    /// `unknown` bucket must never teach seeds (contamination rule,
    /// R56-F1 spirit). Recording never fails or delays the transfer
    /// (ph-1 posture).
    pub(crate) fn record(&self, ok: bool) {
        if !ok {
            return;
        }
        let Some(store) = self.store.as_ref() else {
            return;
        };
        let Some(facts) = self.open.lock().unwrap_or_else(|e| e.into_inner()).clone() else {
            return;
        };
        let Some((local_role, summary)) = self
            .terminal
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .clone()
        else {
            return;
        };
        let peer_key = peer_host(&self.peer).map(|host| match local_role {
            // `components()` renders `root/.` and `root` as one key —
            // same target, one seed bucket.
            LocalRole::Destination => format!(
                "{host}:{}",
                facts
                    .resolved_root
                    .components()
                    .collect::<std::path::PathBuf>()
                    .display()
            ),
            _ => host.to_string(),
        });
        let sink = RecordSink {
            store: store.clone(),
            route: RouteTag {
                topology: Topology::Remote,
                local_role,
                // Best wire knowledge: the session protocol does not
                // identify the peer's process kind (see the enum docs).
                initiator: Initiator::Cli,
                peer_key,
            },
        };
        record_session_history(
            Some(&sink),
            SessionRecordFacts {
                mirror_enabled: facts.mirror_enabled,
                compare_mode: facts.compare_mode,
                files: summary.files_transferred,
                bytes: summary.bytes_transferred,
                files_failed: summary.files_failed,
                elapsed: self.started.elapsed(),
            },
        );
    }
}

/// Wrap an [`OpenResolver`] so a successful resolve also deposits the
/// perf facts into the recorder. Kept separate from [`with_open_hook`]
/// so the dispatcher-event contract (otp-10b-2 F4) stays untouched.
fn with_perf_capture(
    inner: Box<OpenResolver>,
    recorder: Arc<ServedSessionRecorder>,
) -> Box<OpenResolver> {
    Box::new(move |open: &SessionOpen| {
        let compare_mode = open.compare_mode();
        let mirror_enabled = open.mirror_enabled;
        let fut = inner(open);
        let recorder = Arc::clone(&recorder);
        Box::pin(async move {
            let resolved = fut.await?;
            recorder.note_open(OpenPerfFacts {
                compare_mode,
                mirror_enabled,
                resolved_root: resolved.root.clone(),
            });
            Ok(resolved)
        })
    })
}

/// The stable host portion of the dispatcher's peer string
/// (`SocketAddr` rendering or `"unknown"`): the port churns per
/// connection and must not fragment seed keys.
fn peer_host(peer: &str) -> Option<&str> {
    peer.rsplit_once(':').map(|(host, _)| host)
}

/// Map a resolver `tonic::Status` onto a `SessionError` code. blit-core
/// is deliberately `Status`-free, so the daemon picks the wire code:
/// an unknown module is `MODULE_UNKNOWN`, a bad or escaping path is a
/// `PROTOCOL_VIOLATION` (the initiator sent an unusable request),
/// anything else is `INTERNAL`.
fn status_to_fault(status: Status) -> SessionFault {
    let code = match status.code() {
        tonic::Code::NotFound => Code::ModuleUnknown,
        tonic::Code::InvalidArgument | tonic::Code::PermissionDenied => Code::ProtocolViolation,
        _ => Code::Internal,
    };
    SessionFault::refusal(code, status.message().to_string())
}

/// Build the daemon's [`OpenResolver`]: given a received `SessionOpen`,
/// resolve its module + path to an absolute local root and report the
/// module's read-only flag. Mirrors the push Header sequence
/// (`resolve_module` → path validation → F2 canonical containment via
/// `resolve_contained_path`), refusing with a `SessionError` instead of
/// a `tonic::Status`. The closure is `Fn` (callable once per session)
/// and clones its captured handles per call so it stays `Send + Sync`.
pub(crate) fn make_open_resolver(
    modules: Arc<Mutex<HashMap<String, ModuleConfig>>>,
    default_root: Option<RootExport>,
) -> Box<OpenResolver> {
    Box::new(move |open: &SessionOpen| {
        let modules = Arc::clone(&modules);
        let default_root = default_root.clone();
        let module_name = open.module.clone();
        let wire_path = open.path.clone();
        Box::pin(async move {
            let config = resolve_module(&modules, default_root.as_ref(), &module_name)
                .await
                .map_err(status_to_fault)?;
            // Empty path targets the module root; a non-empty path is
            // validated and contained against the module's canonical
            // root (F2 symlink-escape protection — the same chokepoint
            // the per-file write path uses).
            let root = if wire_path.is_empty() {
                config.path.clone()
            } else {
                let rel = resolve_relative_path(&wire_path).map_err(status_to_fault)?;
                resolve_contained_path(&config, &rel).map_err(status_to_fault)?
            };
            Ok(ResolvedEndpoint {
                root,
                read_only: config.read_only,
            })
        })
    })
}

/// Run one daemon-side transfer session to completion, dispatching on
/// the client's declared initiator role via [`run_responder`]: a SOURCE
/// initiator makes the daemon the DESTINATION (push-equivalent, otp-4);
/// a DESTINATION initiator makes the daemon the SOURCE (pull-equivalent,
/// otp-5). Returns `Ok(())` on a clean transfer or `Err(Status)`
/// carrying the session fault's message for the jobs record. The session
/// communicates its own refusals to the peer as `SessionError` *frames*
/// (via the response stream); this `Status` is for the daemon's outcome
/// record and `resolve_streaming_outcome`'s terminal handling, not the
/// primary error channel.
pub(crate) async fn run_transfer_session(
    modules: Arc<Mutex<HashMap<String, ModuleConfig>>>,
    default_root: Option<RootExport>,
    inbound: Streaming<TransferFrame>,
    tx: mpsc::Sender<Result<TransferFrame, Status>>,
    // Operator policy from the daemon runtime config: `--force-grpc-data`
    // (review otp-10a F3) and `--no-server-checksums` (otp-10b-1) apply
    // to served sessions exactly as they did to the old handlers.
    policy: ResponderPolicy,
    byte_progress: ByteProgressSink,
    job_progress: ActiveJobProgress,
    // Fires once at a successful open resolve with this session's job
    // kind + endpoint (review otp-10b-2 F4).
    on_open: Arc<OnSessionOpen>,
    // ph-1c: dispatcher-owned session observer; this function fills it
    // from inside the session, the dispatcher reads it after the race.
    recorder: Arc<ServedSessionRecorder>,
) -> Result<(), Status> {
    let transport = grpc_daemon_transport(tx, inbound);
    let (source_progress_tx, mut source_progress_rx) = mpsc::unbounded_channel();
    let source_job_progress = job_progress.clone();
    let relay_source_bytes = async move {
        while let Some(event) = source_progress_rx.recv().await {
            source_job_progress.report_source_event(&event);
        }
    };
    let (destination_progress_tx, mut destination_progress_rx) = mpsc::unbounded_channel();
    let destination_job_progress = job_progress.clone();
    let relay_destination_progress = async move {
        while let Some(event) = destination_progress_rx.recv().await {
            destination_job_progress.report_destination_event(&event);
        }
    };
    // The same module→root resolver serves both roles; only the one the
    // initiator's declared role selects is consulted. Two clones so each
    // target owns its resolver (the closure clones its captured handles
    // per call, so this is cheap). Which resolver runs IS the kind: a
    // consulted source-resolver means the daemon serves SOURCE (the
    // client pulls — the old PullSync verbs' kind), a consulted
    // dest-resolver means the daemon receives (push-equivalent).
    let source_resolver = with_perf_capture(
        with_open_hook(
            make_open_resolver(Arc::clone(&modules), default_root.clone()),
            Arc::clone(&on_open),
            ActiveJobKind::PullSync,
        ),
        Arc::clone(&recorder),
    );
    let dest_resolver = with_perf_capture(
        with_open_hook(
            make_open_resolver(modules, default_root),
            on_open,
            ActiveJobKind::Push,
        ),
        Arc::clone(&recorder),
    );
    // ph-1c: terminal-summary hooks fire inside the session at the
    // contract's closing point, so the dispatcher still knows the
    // session completed even if the initiator's immediate hangup drops
    // this future mid-teardown. Which hook fires IS the role.
    let source_terminal = {
        let recorder = Arc::clone(&recorder);
        Arc::new(move |summary: &blit_core::generated::TransferSummary| {
            recorder.note_terminal(LocalRole::Source, summary);
        })
    };
    let destination_terminal = {
        let recorder = Arc::clone(&recorder);
        Arc::new(move |summary: &blit_core::generated::TransferSummary| {
            recorder.note_terminal(LocalRole::Destination, summary);
        })
    };
    let instruments = ResponderInstruments {
        source: SourceInstruments {
            progress: Some(RemoteTransferProgress::new(source_progress_tx)),
            on_terminal_summary: Some(source_terminal),
            ..Default::default()
        },
        destination: DestinationInstruments {
            progress: Some(RemoteTransferProgress::new(destination_progress_tx)),
            byte_progress: Some(byte_progress),
            on_terminal_summary: Some(destination_terminal),
            ..Default::default()
        },
    };
    let (outcome, (), ()) = tokio::join!(
        run_responder(
            HelloConfig::default(),
            transport,
            SourceResponderTarget::Resolve(source_resolver),
            DestinationTarget::Resolve(dest_resolver),
            instruments,
            policy,
        ),
        relay_source_bytes,
        relay_destination_progress,
    );
    match outcome {
        // Either role completing cleanly is a successful transfer; the
        // daemon record does not distinguish push- from pull-equivalent
        // (the jobs kind stays Push until the taxonomy is revisited at
        // cutover — see the dispatcher).
        Ok(outcome) => {
            // ph-1c: the daemon's own record is appended by the
            // dispatcher AFTER the outcome race settles
            // (`ServedSessionRecorder::record`), never here — this
            // future is the raced one.
            let summary = match outcome {
                blit_core::transfer_session::ResponderOutcome::Source(summary) => summary,
                blit_core::transfer_session::ResponderOutcome::Destination(outcome) => {
                    outcome.summary
                }
            };
            job_progress.finish(
                summary.files_transferred,
                summary.bytes_transferred,
                summary.in_stream_carrier_used,
            );
            Ok(())
        }
        Err(report) => {
            // run_responder already emitted a SessionError frame to the
            // peer; surface the reason for the record.
            let msg = report
                .downcast_ref::<SessionFault>()
                .map(|f| f.message.clone())
                .unwrap_or_else(|| format!("{report:#}"));
            Err(Status::internal(msg))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn status_to_fault_maps_not_found_to_module_unknown() {
        let fault = status_to_fault(Status::not_found("module 'x' not found"));
        assert_eq!(fault.code, Code::ModuleUnknown);
        assert!(fault.message.contains("not found"));
    }

    #[test]
    fn status_to_fault_maps_permission_denied_to_protocol_violation() {
        let fault = status_to_fault(Status::permission_denied("path containment: escape"));
        assert_eq!(fault.code, Code::ProtocolViolation);
    }

    #[test]
    fn status_to_fault_maps_invalid_argument_to_protocol_violation() {
        let fault = status_to_fault(Status::invalid_argument("path not allowed"));
        assert_eq!(fault.code, Code::ProtocolViolation);
    }
}
