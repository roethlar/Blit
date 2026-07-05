//! Unified transfer session — the ONE block of transfer code
//! (docs/plan/ONE_TRANSFER_PATH.md, D-2026-07-05-1).
//!
//! A transfer has a SOURCE role and a DESTINATION role; which end
//! initiated and which CLI verb was used select roles, never code.
//! Both roles run the drivers below over a [`transport::FrameTransport`];
//! the wire contract they implement — phases, frame table, record
//! grammar, error semantics — is `docs/TRANSFER_SESSION.md` (otp-1).
//!
//! otp-3 scope: the role-parameterized state machine over the existing
//! engine with the in-process transport and the in-stream byte
//! carrier. The TCP data plane, daemon serving, ActiveJobs/cancel and
//! progress wiring land at otp-4; mirror otp-6; resume otp-7;
//! delegated otp-9 (see the slice list in the plan).

pub mod transport;

use std::collections::{HashMap, HashSet};
use std::fmt;
use std::future::Future;
use std::path::{Path, PathBuf};
use std::pin::Pin;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex as StdMutex};

use eyre::Result;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::sync::mpsc;

use crate::generated::transfer_frame::Frame;
use crate::generated::{
    session_error, ComparisonMode, FileData, FileHeader, FilterSpec, ManifestComplete, NeedBatch,
    NeedComplete, NeedEntry, SessionAccept, SessionError, SessionHello, SessionOpen, SourceDone,
    TarShardComplete, TarShardHeader, TransferFrame, TransferRole, TransferSummary,
};
use crate::manifest::{header_transfer_status, CompareOptions, FileStatus};
use crate::remote::transfer::diff_planner;
use crate::remote::transfer::payload::PreparedPayload;
use crate::remote::transfer::sink::{FsSinkConfig, FsTransferSink, TransferSink};
use crate::remote::transfer::source::TransferSource;
use crate::remote::transfer::tar_safety::MAX_TAR_SHARD_BYTES;
use crate::remote::transfer::{AbortOnDrop, CONTROL_PLANE_CHUNK_SIZE};
use crate::transfer_plan::PlanOptions;
use transport::{FrameRx, FrameTransport, FrameTx};

/// Belt-and-braces wire-shape version, bumped on any change to the
/// frame set or grammar. Exchanged (and exact-matched) in
/// `SessionHello` alongside the build id (D-2026-07-05-2).
pub const CONTRACT_VERSION: u32 = 1;

/// Payload chunk size on the in-stream carrier. Same unit the gRPC
/// control plane uses today; the data plane (otp-4) has its own.
const IN_STREAM_CHUNK: usize = CONTROL_PLANE_CHUNK_SIZE;

/// Manifest entries buffered per destination diff batch. Mirrors the
/// daemon push handler's `MANIFEST_CHECK_CHUNK` rationale (w4-4): the
/// per-entry check is 2+ blocking syscalls, so it runs chunked on the
/// blocking pool instead of inline per entry.
const DEST_DIFF_CHUNK: usize = 128;

/// Buffer of the in-memory pipe that feeds wire file-record bytes
/// into `FsTransferSink::write_file_stream`. Bounds destination-side
/// buffering per file record.
const FILE_RECORD_PIPE_BYTES: usize = 256 * 1024;

/// This build's session identity: `<crate version>+<git sha>[.dirty]`
/// (contract §Invariants 2). `BLIT_GIT_SHA` is emitted by build.rs;
/// "unknown" when git was unavailable at compile time.
pub fn session_build_id() -> &'static str {
    concat!(env!("CARGO_PKG_VERSION"), "+", env!("BLIT_GIT_SHA"))
}

/// The identity this end presents in `SessionHello`. Defaults to the
/// real compile-time identity; tests inject mismatches.
#[derive(Debug, Clone)]
pub struct HelloConfig {
    pub build_id: String,
    pub contract_version: u32,
}

impl Default for HelloConfig {
    fn default() -> Self {
        Self {
            build_id: session_build_id().to_string(),
            contract_version: CONTRACT_VERSION,
        }
    }
}

/// Which handshake part this end plays. Orthogonal to role: all four
/// initiator/role combinations run the same state machine (contract
/// §Invariants 3).
pub enum SessionEndpoint {
    /// This end opened the transport; it sends `SessionOpen`.
    /// (Boxed: `SessionOpen` dwarfs the bare `Responder` variant.)
    Initiator { open: Box<SessionOpen> },
    /// This end answers `SessionOpen` with `SessionAccept`. Daemon
    /// module/path/read-only validation attaches here at otp-4.
    Responder,
}

impl SessionEndpoint {
    /// Convenience constructor so callers don't spell the `Box`.
    pub fn initiator(open: SessionOpen) -> Self {
        SessionEndpoint::Initiator {
            open: Box::new(open),
        }
    }
}

pub struct SourceSessionConfig {
    pub hello: HelloConfig,
    pub endpoint: SessionEndpoint,
    /// Engine planner knobs (tar/large/raw thresholds). Local to the
    /// source end — strategy selection is planner-owned and never
    /// crosses the wire (contract §Transport selection).
    pub plan_options: PlanOptions,
}

pub struct DestinationSessionConfig {
    pub hello: HelloConfig,
    pub endpoint: SessionEndpoint,
}

/// A session-terminating fault: either end refusing, aborting, or
/// catching the peer in a protocol violation. Carried as the error
/// payload of the drivers' `eyre::Report`s — downcast to inspect the
/// wire code.
#[derive(Debug, Clone)]
pub struct SessionFault {
    pub code: session_error::Code,
    pub message: String,
    /// Both build ids on BUILD_MISMATCH so the operator sees exactly
    /// which end is stale (contract §Errors).
    pub local_build_id: String,
    pub peer_build_id: String,
    /// True when the peer already knows about this fault — it sent
    /// the `SessionError` frame itself, or this end already emitted
    /// one. Drivers must not send another.
    pub peer_notified: bool,
}

impl SessionFault {
    fn new(code: session_error::Code, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
            local_build_id: String::new(),
            peer_build_id: String::new(),
            peer_notified: false,
        }
    }

    fn protocol_violation(message: impl Into<String>) -> Self {
        Self::new(session_error::Code::ProtocolViolation, message)
    }

    fn internal(message: impl Into<String>) -> Self {
        Self::new(session_error::Code::Internal, message)
    }

    fn read_only(message: impl Into<String>) -> Self {
        Self::new(session_error::Code::ReadOnly, message)
    }

    /// Public constructor for a caller-side refusal (e.g. the daemon's
    /// [`OpenResolver`] mapping a `tonic::Status` to a `SessionError`
    /// code). blit-core stays free of `tonic::Status`, so the caller
    /// picks the wire code.
    pub fn refusal(code: session_error::Code, message: impl Into<String>) -> Self {
        Self::new(code, message)
    }

    fn from_wire(err: SessionError) -> Self {
        Self {
            code: session_error::Code::try_from(err.code)
                .unwrap_or(session_error::Code::SessionErrorUnspecified),
            message: err.message,
            // The peer reports its view: its "local" is our peer.
            local_build_id: err.peer_build_id,
            peer_build_id: err.local_build_id,
            peer_notified: true,
        }
    }

    fn to_wire(&self) -> SessionError {
        SessionError {
            code: self.code as i32,
            message: self.message.clone(),
            local_build_id: self.local_build_id.clone(),
            peer_build_id: self.peer_build_id.clone(),
        }
    }
}

impl fmt::Display for SessionFault {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "session {}: {}", self.code.as_str_name(), self.message)
    }
}

impl std::error::Error for SessionFault {}

/// Downcast a driver-internal error back to its fault, wrapping
/// non-fault failures (fs errors, planner errors, transport failures)
/// as INTERNAL — an end that aborts says why before closing.
fn fault_from_report(report: eyre::Report) -> SessionFault {
    match report.downcast::<SessionFault>() {
        Ok(fault) => fault,
        Err(other) => SessionFault::internal(format!("{other:#}")),
    }
}

fn frame(f: Frame) -> TransferFrame {
    TransferFrame { frame: Some(f) }
}

fn error_frame(fault: &SessionFault) -> TransferFrame {
    frame(Frame::Error(fault.to_wire()))
}

/// Short frame identifier for protocol-violation messages.
fn frame_name(f: &Option<Frame>) -> &'static str {
    match f {
        Some(Frame::Hello(_)) => "SessionHello",
        Some(Frame::Open(_)) => "SessionOpen",
        Some(Frame::Accept(_)) => "SessionAccept",
        Some(Frame::ManifestEntry(_)) => "ManifestEntry",
        Some(Frame::ManifestComplete(_)) => "ManifestComplete",
        Some(Frame::NeedBatch(_)) => "NeedBatch",
        Some(Frame::NeedComplete(_)) => "NeedComplete",
        Some(Frame::BlockHashes(_)) => "BlockHashList",
        Some(Frame::FileBegin(_)) => "FileBegin",
        Some(Frame::FileData(_)) => "FileData",
        Some(Frame::TarShardHeader(_)) => "TarShardHeader",
        Some(Frame::TarShardChunk(_)) => "TarShardChunk",
        Some(Frame::TarShardComplete(_)) => "TarShardComplete",
        Some(Frame::Block(_)) => "BlockTransfer",
        Some(Frame::BlockComplete(_)) => "BlockTransferComplete",
        Some(Frame::Resize(_)) => "DataPlaneResize",
        Some(Frame::ResizeAck(_)) => "DataPlaneResizeAck",
        Some(Frame::SourceDone(_)) => "SourceDone",
        Some(Frame::Summary(_)) => "TransferSummary",
        Some(Frame::Error(_)) => "SessionError",
        None => "empty frame",
    }
}

fn complement(role: TransferRole) -> TransferRole {
    match role {
        TransferRole::Source => TransferRole::Destination,
        TransferRole::Destination => TransferRole::Source,
        TransferRole::Unspecified => TransferRole::Unspecified,
    }
}

/// Per-role capability check of the operation a `SessionOpen`
/// describes. otp-3 refuses what later slices implement rather than
/// silently ignoring it (fail-fast; contract §Errors).
type OpenValidator = dyn Fn(&SessionOpen) -> std::result::Result<(), SessionFault> + Send + Sync;

/// The local endpoint a Responder resolves a received `SessionOpen`
/// to. The daemon maps the wire module name + path here; a test can
/// hand a fixed root with no module semantics via
/// [`DestinationTarget::Fixed`] instead.
#[derive(Debug, Clone)]
pub struct ResolvedEndpoint {
    /// Absolute local root this end targets.
    pub root: PathBuf,
    /// Whether the resolved module forbids writes. A DESTINATION
    /// responder refuses `READ_ONLY`; a SOURCE responder (otp-5,
    /// daemon-send) does not care — reading a read-only module is fine.
    pub read_only: bool,
}

/// Async callback a Responder uses to turn a received (and
/// capability-validated) `SessionOpen` into its local endpoint. It
/// lives caller-side — the daemon resolves modules and maps its own
/// `tonic::Status` errors to [`SessionFault`], so blit-core stays free
/// of module/Status types. A returned fault (unknown module,
/// containment failure) becomes a `SessionError` at OPEN, never a
/// silent close (contract §Phase state machine).
pub type OpenResolver = dyn Fn(
        &SessionOpen,
    )
        -> Pin<Box<dyn Future<Output = std::result::Result<ResolvedEndpoint, SessionFault>> + Send>>
    + Send
    + Sync;

/// Where a DESTINATION driver writes. `Fixed` is a root known up front
/// (an initiator's own local root, or a test's temp dir). `Resolve`
/// defers to a caller callback that maps the received `SessionOpen` to
/// a local root — the daemon path, where the root depends on the wire
/// module name and so can only be resolved mid-handshake (after HELLO,
/// before SessionAccept). A `Resolve` target is meaningful only on a
/// Responder; an Initiator always knows its own root.
pub enum DestinationTarget {
    Fixed(PathBuf),
    Resolve(Box<OpenResolver>),
}

fn source_open_validator(open: &SessionOpen) -> std::result::Result<(), SessionFault> {
    if open.resume.as_ref().is_some_and(|r| r.enabled) {
        return Err(SessionFault::internal(
            "resume is not implemented on the unified session yet (otp-7)",
        ));
    }
    if open
        .filter
        .as_ref()
        .is_some_and(|f| *f != FilterSpec::default())
    {
        return Err(SessionFault::internal(
            "filters are not implemented on the unified session yet (otp-6)",
        ));
    }
    Ok(())
}

fn destination_open_validator(open: &SessionOpen) -> std::result::Result<(), SessionFault> {
    if open.mirror_enabled {
        return Err(SessionFault::internal(
            "mirror is not implemented on the unified session yet (otp-6)",
        ));
    }
    if open.resume.as_ref().is_some_and(|r| r.enabled) {
        return Err(SessionFault::internal(
            "resume is not implemented on the unified session yet (otp-7)",
        ));
    }
    Ok(())
}

/// Outcome of the HELLO + OPEN phases.
struct Negotiated {
    open: SessionOpen,
    #[allow(dead_code)] // capacity/grant consumed from otp-4b (data plane) on
    accept: SessionAccept,
    /// The write root a Responder's [`OpenResolver`] produced from the
    /// received open, if one was supplied; `None` for an Initiator or a
    /// fixed-root Responder (the caller supplies the root then).
    resolved_root: Option<PathBuf>,
}

/// HELLO + OPEN/ACCEPT, one implementation both roles call (otp-3
/// scoping requirement). Sends the refusal `SessionError` itself when
/// it detects the fault locally; returned faults are `peer_notified`.
async fn establish(
    transport: &mut FrameTransport,
    hello: &HelloConfig,
    endpoint: &SessionEndpoint,
    local_role: TransferRole,
    validate_open: &OpenValidator,
    // Consulted only on the Responder branch, after the received open
    // passes `validate_open` and before SessionAccept. `None` = the
    // caller supplies the root itself (Initiator, or fixed-root test).
    resolve_open: Option<&OpenResolver>,
) -> Result<Negotiated> {
    // HELLO both ways, exact match (D-2026-07-05-2). First frame each
    // direction; no ordering between the two directions.
    transport
        .send(frame(Frame::Hello(SessionHello {
            build_id: hello.build_id.clone(),
            contract_version: hello.contract_version,
        })))
        .await?;

    let peer_hello = match expect_frame(transport).await? {
        Frame::Hello(h) => h,
        other => {
            return Err(notify_and_wrap(
                transport,
                SessionFault::protocol_violation(format!(
                    "expected SessionHello, got {}",
                    frame_name(&Some(other))
                )),
            )
            .await)
        }
    };

    if peer_hello.build_id != hello.build_id
        || peer_hello.contract_version != hello.contract_version
    {
        let fault = SessionFault {
            code: session_error::Code::BuildMismatch,
            message: format!(
                "same-build peers required (D-2026-07-05-2): local {} (contract v{}) vs peer {} (contract v{})",
                hello.build_id, hello.contract_version,
                peer_hello.build_id, peer_hello.contract_version,
            ),
            local_build_id: hello.build_id.clone(),
            peer_build_id: peer_hello.build_id.clone(),
            peer_notified: false,
        };
        return Err(notify_and_wrap(transport, fault).await);
    }

    match endpoint {
        SessionEndpoint::Initiator { open } => {
            let open = open.as_ref().clone();
            transport.send(frame(Frame::Open(open.clone()))).await?;
            let accept = match expect_frame(transport).await? {
                Frame::Accept(a) => a,
                other => {
                    return Err(notify_and_wrap(
                        transport,
                        SessionFault::protocol_violation(format!(
                            "expected SessionAccept, got {}",
                            frame_name(&Some(other))
                        )),
                    )
                    .await)
                }
            };
            Ok(Negotiated {
                open,
                accept,
                resolved_root: None,
            })
        }
        SessionEndpoint::Responder => {
            let open = match expect_frame(transport).await? {
                Frame::Open(o) => o,
                other => {
                    return Err(notify_and_wrap(
                        transport,
                        SessionFault::protocol_violation(format!(
                            "expected SessionOpen, got {}",
                            frame_name(&Some(other))
                        )),
                    )
                    .await)
                }
            };
            // The initiator declares ITS role; this responder end must
            // hold the complement.
            let declared =
                TransferRole::try_from(open.initiator_role).unwrap_or(TransferRole::Unspecified);
            if declared != complement(local_role) {
                return Err(notify_and_wrap(
                    transport,
                    SessionFault::protocol_violation(format!(
                        "initiator declared role {} but this responder is {}",
                        declared.as_str_name(),
                        local_role.as_str_name()
                    )),
                )
                .await);
            }
            if let Err(fault) = validate_open(&open) {
                // Refusal is a SessionError instead of SessionAccept,
                // never a silent close (contract §Phase state machine).
                return Err(notify_and_wrap(transport, fault).await);
            }
            // Responder endpoint resolution (otp-4): map the wire
            // module/path to a local root and enforce read-only, both
            // BEFORE SessionAccept so a refusal replaces the accept
            // (never follows it). The resolver is caller-supplied
            // (daemon module lookup); a fixed-root responder passes
            // None and resolves nothing here.
            let resolved_root = match resolve_open {
                Some(resolve) => match resolve(&open).await {
                    Ok(resolved) => {
                        // A read-only module is fatal only for a
                        // DESTINATION (it would write); a SOURCE
                        // responder (otp-5, daemon-send) reads happily.
                        if local_role == TransferRole::Destination && resolved.read_only {
                            return Err(notify_and_wrap(
                                transport,
                                SessionFault::read_only(
                                    "destination module is read-only".to_string(),
                                ),
                            )
                            .await);
                        }
                        Some(resolved.root)
                    }
                    Err(fault) => return Err(notify_and_wrap(transport, fault).await),
                },
                None => None,
            };
            let accept = SessionAccept {
                // The byte RECEIVER advertises capacity at session
                // open (D-2026-06-20-1/-2); consumed by the dial when
                // the data plane lands (otp-4b).
                receiver_capacity: if local_role == TransferRole::Destination {
                    Some(crate::engine::local_receiver_capacity())
                } else {
                    None
                },
                // No grant = in-stream byte carrier, otp-4a's only one.
                data_plane: None,
            };
            transport.send(frame(Frame::Accept(accept.clone()))).await?;
            Ok(Negotiated {
                open,
                accept,
                resolved_root,
            })
        }
    }
}

/// Receive one frame during establish; peer errors and closes become
/// terminal faults.
async fn expect_frame(transport: &mut FrameTransport) -> Result<Frame> {
    match transport.recv().await? {
        Some(TransferFrame {
            frame: Some(Frame::Error(err)),
        }) => Err(eyre::Report::new(SessionFault::from_wire(err))),
        Some(TransferFrame { frame: Some(f) }) => Ok(f),
        Some(TransferFrame { frame: None }) => Err(eyre::Report::new(
            SessionFault::protocol_violation("frame with empty oneof"),
        )),
        None => Err(eyre::Report::new(SessionFault::internal(
            "peer closed during session establish",
        ))),
    }
}

/// Send the fault to the peer (best effort), mark it notified, and
/// wrap it for return.
async fn notify_and_wrap(transport: &mut FrameTransport, mut fault: SessionFault) -> eyre::Report {
    let _ = transport.send(error_frame(&fault)).await;
    fault.peer_notified = true;
    eyre::Report::new(fault)
}

// ---------------------------------------------------------------------------
// SOURCE driver
// ---------------------------------------------------------------------------

/// Events the source's receive half forwards to its send half. The
/// channel is unbounded but bounded by construction: every `Need`
/// consumes a distinct sent-manifest entry (unknown or repeated paths
/// fault the session), so the queue never exceeds the source's own
/// manifest size — the contract's bounded-buffering rule holds.
enum SourceEvent {
    Need(FileHeader),
    NeedComplete,
    Summary(TransferSummary),
    Fault(SessionFault),
}

/// Run the SOURCE role of one transfer session over `transport`.
/// Returns the destination-computed `TransferSummary` (contract: the
/// end that wrote the bytes is the end that attests to them).
pub async fn run_source(
    cfg: SourceSessionConfig,
    transport: FrameTransport,
    source: Arc<dyn TransferSource>,
) -> Result<TransferSummary> {
    let mut transport = transport;
    if let SessionEndpoint::Initiator { open } = &cfg.endpoint {
        // Own-config coherence: a source initiator declares SOURCE.
        let declared = TransferRole::try_from(open.initiator_role);
        if declared != Ok(TransferRole::Source) {
            eyre::bail!("run_source initiator must declare TRANSFER_ROLE_SOURCE in SessionOpen");
        }
        if let Err(fault) = source_open_validator(open) {
            eyre::bail!("run_source initiator config unsupported: {fault}");
        }
    }

    let negotiated = establish(
        &mut transport,
        &cfg.hello,
        &cfg.endpoint,
        TransferRole::Source,
        &source_open_validator,
        // A SOURCE responder's endpoint resolution (module→root for a
        // daemon-send) lands with otp-5; otp-4a's daemon is always the
        // DESTINATION responder, so the source never resolves here.
        None,
    )
    .await?;

    let (mut tx, rx) = transport.split();
    let sent: Arc<StdMutex<HashMap<String, FileHeader>>> = Arc::default();
    // Set by the send half the moment ManifestComplete goes out. On
    // an ordered transport, a NeedComplete arriving while this is
    // still false is provably premature — the peer cannot have
    // received what we have not sent (contract: NeedComplete only
    // after ManifestComplete received + all entries diffed).
    let manifest_sent = Arc::new(AtomicBool::new(false));
    let (event_tx, event_rx) = mpsc::unbounded_channel();
    // AbortOnDrop: an early error return below must abort the receive
    // half instead of leaking it (same rationale as design-2 / w4-1).
    let _recv_guard = AbortOnDrop::new(tokio::spawn(source_recv_half(
        rx,
        Arc::clone(&sent),
        Arc::clone(&manifest_sent),
        event_tx,
    )));

    match source_send_half(
        &cfg,
        &negotiated,
        &mut tx,
        source,
        sent,
        &manifest_sent,
        event_rx,
    )
    .await
    {
        Ok(summary) => Ok(summary),
        Err(report) => {
            let mut fault = fault_from_report(report);
            if !fault.peer_notified {
                let _ = tx.send(error_frame(&fault)).await;
                fault.peer_notified = true;
            }
            Err(eyre::Report::new(fault))
        }
    }
}

/// Receive half of the source driver: drains the transport for the
/// whole session so destination sends can never deadlock against a
/// blocked source send, and routes the destination lane to the send
/// half. Terminates on summary, error, close, or violation.
async fn source_recv_half(
    mut rx: Box<dyn FrameRx>,
    sent: Arc<StdMutex<HashMap<String, FileHeader>>>,
    manifest_sent: Arc<AtomicBool>,
    events: mpsc::UnboundedSender<SourceEvent>,
) {
    loop {
        let received = match rx.recv().await {
            Ok(Some(f)) => f,
            Ok(None) => {
                let _ = events.send(SourceEvent::Fault(SessionFault::internal(
                    "peer closed before TransferSummary",
                )));
                return;
            }
            Err(err) => {
                let _ = events.send(SourceEvent::Fault(SessionFault::internal(format!(
                    "transport receive failed: {err:#}"
                ))));
                return;
            }
        };
        match received.frame {
            Some(Frame::NeedBatch(batch)) => {
                for entry in batch.entries {
                    if entry.resume {
                        let _ = events.send(SourceEvent::Fault(SessionFault::protocol_violation(
                            format!(
                                "resume-flagged need for '{}' in a session opened without resume",
                                entry.relative_path
                            ),
                        )));
                        return;
                    }
                    let header = sent
                        .lock()
                        .expect("sent-manifest lock poisoned")
                        .remove(&entry.relative_path);
                    match header {
                        Some(h) => {
                            let _ = events.send(SourceEvent::Need(h));
                        }
                        None => {
                            let _ = events.send(SourceEvent::Fault(
                                SessionFault::protocol_violation(format!(
                                    "need for unknown or already-needed path '{}'",
                                    entry.relative_path
                                )),
                            ));
                            return;
                        }
                    }
                }
            }
            Some(Frame::NeedComplete(_)) => {
                if !manifest_sent.load(Ordering::Acquire) {
                    // Fail fast at arrival time (otp-3 codex F2): the
                    // event queue would otherwise let an early
                    // NeedComplete be processed late and pass as
                    // legitimate.
                    let _ = events.send(SourceEvent::Fault(SessionFault::protocol_violation(
                        "NeedComplete before the source's ManifestComplete",
                    )));
                    return;
                }
                let _ = events.send(SourceEvent::NeedComplete);
            }
            Some(Frame::Summary(summary)) => {
                let _ = events.send(SourceEvent::Summary(summary));
                return;
            }
            Some(Frame::Error(err)) => {
                let _ = events.send(SourceEvent::Fault(SessionFault::from_wire(err)));
                return;
            }
            other => {
                let _ = events.send(SourceEvent::Fault(SessionFault::protocol_violation(
                    format!("{} on the source's receive lane", frame_name(&other)),
                )));
                return;
            }
        }
    }
}

async fn source_send_half(
    cfg: &SourceSessionConfig,
    negotiated: &Negotiated,
    tx: &mut Box<dyn FrameTx>,
    source: Arc<dyn TransferSource>,
    sent: Arc<StdMutex<HashMap<String, FileHeader>>>,
    manifest_sent: &AtomicBool,
    mut events: mpsc::UnboundedReceiver<SourceEvent>,
) -> Result<TransferSummary> {
    let mut pending: Vec<FileHeader> = Vec::new();
    let mut need_complete = false;

    // Streaming manifest: entries go out as enumeration produces them
    // (immediate start in every direction — plan §Design 2). The open
    // carries no source path: the source end owns its local endpoint.
    let _ = &negotiated.open;
    let unreadable: Arc<StdMutex<Vec<String>>> = Arc::default();
    let (mut header_rx, scan_handle) = source.scan(None, Arc::clone(&unreadable));
    while let Some(header) = header_rx.recv().await {
        sent.lock()
            .expect("sent-manifest lock poisoned")
            .insert(header.relative_path.clone(), header.clone());
        tx.send(frame(Frame::ManifestEntry(header))).await?;
        // Faults detected by the receive half abort the stream now,
        // not after the full scan; needs just accumulate.
        drain_source_events(&mut events, &mut pending, &mut need_complete)?;
    }
    let scanned = scan_handle
        .await
        .map_err(|err| eyre::eyre!("manifest scan task panicked: {err}"))??;
    let scan_complete = unreadable
        .lock()
        .expect("unreadable list lock poisoned")
        .is_empty();
    log::debug!("session source manifest complete: {scanned} entries, complete={scan_complete}");
    tx.send(frame(Frame::ManifestComplete(ManifestComplete {
        scan_complete,
    })))
    .await?;
    manifest_sent.store(true, Ordering::Release);

    // Payload phase. In-stream record grammar: payload records only
    // after ManifestComplete, strictly serialized per record
    // (contract §Transport selection). Needs accumulated while a
    // record batch was being sent become the next planner batch.
    let mut read_buf = vec![0u8; IN_STREAM_CHUNK];
    loop {
        drain_source_events(&mut events, &mut pending, &mut need_complete)?;
        if !pending.is_empty() {
            let batch = std::mem::take(&mut pending);
            send_payload_records(tx, &source, cfg.plan_options, batch, &mut read_buf).await?;
            continue;
        }
        if need_complete {
            break;
        }
        match events.recv().await {
            Some(event) => {
                handle_source_event(event, &mut pending, &mut need_complete)?;
            }
            None => {
                return Err(eyre::Report::new(SessionFault::internal(
                    "source receive half ended before NeedComplete",
                )))
            }
        }
    }

    tx.send(frame(Frame::SourceDone(SourceDone {}))).await?;

    // CLOSING: the destination is the scorer; the next event must be
    // its summary (the receive half ends after forwarding it).
    match events.recv().await {
        Some(SourceEvent::Summary(summary)) => Ok(summary),
        Some(SourceEvent::Fault(fault)) => Err(eyre::Report::new(fault)),
        Some(SourceEvent::Need(h)) => Err(eyre::Report::new(SessionFault::protocol_violation(
            format!("need for '{}' after NeedComplete", h.relative_path),
        ))),
        Some(SourceEvent::NeedComplete) => Err(eyre::Report::new(
            SessionFault::protocol_violation("duplicate NeedComplete"),
        )),
        None => Err(eyre::Report::new(SessionFault::internal(
            "source receive half ended before TransferSummary",
        ))),
    }
}

fn drain_source_events(
    events: &mut mpsc::UnboundedReceiver<SourceEvent>,
    pending: &mut Vec<FileHeader>,
    need_complete: &mut bool,
) -> Result<()> {
    while let Ok(event) = events.try_recv() {
        handle_source_event(event, pending, need_complete)?;
    }
    Ok(())
}

fn handle_source_event(
    event: SourceEvent,
    pending: &mut Vec<FileHeader>,
    need_complete: &mut bool,
) -> Result<()> {
    match event {
        SourceEvent::Need(header) => {
            if *need_complete {
                return Err(eyre::Report::new(SessionFault::protocol_violation(
                    format!("need for '{}' after NeedComplete", header.relative_path),
                )));
            }
            pending.push(header);
            Ok(())
        }
        SourceEvent::NeedComplete => {
            if *need_complete {
                return Err(eyre::Report::new(SessionFault::protocol_violation(
                    "duplicate NeedComplete",
                )));
            }
            *need_complete = true;
            Ok(())
        }
        SourceEvent::Summary(_) => Err(eyre::Report::new(SessionFault::protocol_violation(
            "TransferSummary before SourceDone",
        ))),
        SourceEvent::Fault(fault) => Err(eyre::Report::new(fault)),
    }
}

/// Plan one batch of needed headers with the engine planner and emit
/// the resulting payload records per the in-stream grammar.
async fn send_payload_records(
    tx: &mut Box<dyn FrameTx>,
    source: &Arc<dyn TransferSource>,
    plan_options: PlanOptions,
    batch: Vec<FileHeader>,
    read_buf: &mut [u8],
) -> Result<()> {
    let payloads = diff_planner::plan_push_payloads(batch, source.root(), plan_options)?;
    for payload in payloads {
        match source.prepare_payload(payload).await? {
            PreparedPayload::File(header) => {
                tx.send(frame(Frame::FileBegin(header.clone()))).await?;
                if header.size == 0 {
                    continue; // record complete at 0 cumulative bytes
                }
                let mut reader = source.open_file(&header).await?;
                let mut remaining = header.size;
                while remaining > 0 {
                    let want = read_buf.len().min(remaining as usize);
                    let got = reader.read(&mut read_buf[..want]).await?;
                    if got == 0 {
                        // Shorter on disk than the manifest promised —
                        // the record can no longer complete at
                        // header.size; abort rather than pad.
                        eyre::bail!(
                            "'{}' hit EOF with {} bytes still promised",
                            header.relative_path,
                            remaining
                        );
                    }
                    tx.send(frame(Frame::FileData(FileData {
                        content: read_buf[..got].to_vec(),
                    })))
                    .await?;
                    remaining -= got as u64;
                }
            }
            PreparedPayload::TarShard { headers, data } => {
                tx.send(frame(Frame::TarShardHeader(TarShardHeader {
                    files: headers,
                    archive_size: data.len() as u64,
                })))
                .await?;
                for chunk in data.chunks(IN_STREAM_CHUNK) {
                    tx.send(frame(Frame::TarShardChunk(
                        crate::generated::TarShardChunk {
                            content: chunk.to_vec(),
                        },
                    )))
                    .await?;
                }
                tx.send(frame(Frame::TarShardComplete(TarShardComplete {})))
                    .await?;
            }
            PreparedPayload::FileBlock { .. } | PreparedPayload::FileBlockComplete { .. } => {
                // The outbound planner never emits these (resume is
                // receive-originated and lands at otp-7).
                eyre::bail!("resume payload planned in a non-resume session");
            }
        }
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// DESTINATION driver
// ---------------------------------------------------------------------------

/// What the destination end can report after a completed session.
#[derive(Debug, Clone)]
pub struct DestinationOutcome {
    /// The summary this end computed and sent (contract: DESTINATION
    /// is the scorer).
    pub summary: TransferSummary,
    /// Paths this end put on the need list, in emission order. The
    /// role suite pins these identical across role assignments — the
    /// executable form of the owner's invariance requirement.
    pub needed_paths: Vec<String>,
}

/// Run the DESTINATION role of one transfer session over `transport`,
/// writing under the root named by `target`. Diffs the streamed
/// manifest against its own filesystem (the destination is the one
/// diff owner — plan §Design 3), returns the summary it computed and
/// sent.
///
/// `target` is [`DestinationTarget::Fixed`] when the root is known up
/// front (an Initiator's own local root, or a test), or
/// [`DestinationTarget::Resolve`] when the root must be resolved from
/// the received `SessionOpen` mid-handshake (the daemon Responder,
/// where the wire module name selects the root).
pub async fn run_destination(
    cfg: DestinationSessionConfig,
    transport: FrameTransport,
    target: DestinationTarget,
) -> Result<DestinationOutcome> {
    let mut transport = transport;
    let endpoint = match cfg.endpoint {
        SessionEndpoint::Initiator { mut open } => {
            let declared = TransferRole::try_from(open.initiator_role);
            if declared != Ok(TransferRole::Destination) {
                eyre::bail!(
                    "run_destination initiator must declare TRANSFER_ROLE_DESTINATION in SessionOpen"
                );
            }
            if let Err(fault) = destination_open_validator(&open) {
                eyre::bail!("run_destination initiator config unsupported: {fault}");
            }
            // Dial contract: the byte receiver advertises capacity in
            // its open when it is the initiator (contract §Invariants 5).
            if open.receiver_capacity.is_none() {
                open.receiver_capacity = Some(crate::engine::local_receiver_capacity());
            }
            SessionEndpoint::Initiator { open }
        }
        SessionEndpoint::Responder => SessionEndpoint::Responder,
    };

    let resolve_open: Option<&OpenResolver> = match &target {
        DestinationTarget::Resolve(resolver) => Some(resolver.as_ref()),
        DestinationTarget::Fixed(_) => None,
    };

    let negotiated = establish(
        &mut transport,
        &cfg.hello,
        &endpoint,
        TransferRole::Destination,
        &destination_open_validator,
        resolve_open,
    )
    .await?;

    // The resolver's root (Responder + Resolve) wins; otherwise the
    // caller-supplied Fixed root.
    let dst_root = match negotiated.resolved_root.clone() {
        Some(root) => root,
        None => match &target {
            DestinationTarget::Fixed(root) => root.clone(),
            // Unreachable: a Resolve target always yields a root on the
            // Responder branch, and establish only skips resolution on
            // the Initiator branch (which pairs with a Fixed root).
            DestinationTarget::Resolve(_) => {
                return Err(eyre::Report::new(SessionFault::internal(
                    "resolver target produced no destination root",
                )));
            }
        },
    };

    match destination_session(&mut transport, &negotiated, &dst_root).await {
        Ok(outcome) => Ok(outcome),
        Err(report) => {
            let mut fault = fault_from_report(report);
            if !fault.peer_notified {
                let _ = transport.send(error_frame(&fault)).await;
                fault.peer_notified = true;
            }
            Err(eyre::Report::new(fault))
        }
    }
}

fn violation(message: String) -> eyre::Report {
    eyre::Report::new(SessionFault::protocol_violation(message))
}

async fn destination_session(
    transport: &mut FrameTransport,
    negotiated: &Negotiated,
    dst_root: &Path,
) -> Result<DestinationOutcome> {
    let compare_mode = ComparisonMode::try_from(negotiated.open.compare_mode)
        .unwrap_or(ComparisonMode::Unspecified);
    let compare_opts = CompareOptions {
        mode: compare_mode.into(),
        ignore_existing: negotiated.open.ignore_existing,
        include_deletions: false, // mirror lands at otp-6
    };
    // src_root is only consumed by local File payloads, which never
    // occur on a session destination (payload bytes arrive as records
    // and go through the stream/tar write paths).
    let sink = FsTransferSink::new(
        PathBuf::new(),
        dst_root.to_path_buf(),
        FsSinkConfig {
            preserve_times: true,
            dry_run: false,
            checksum: None,
            resume: false,
            compare_mode,
        },
    );
    // Same canonical-containment chokepoint the sink write paths use
    // (R46-F3), applied to diff stats so a hostile manifest path can't
    // make the destination stat outside its root.
    let canonical_dst_root = crate::path_safety::canonical_dest_root(dst_root).ok();

    let mut pending: Vec<FileHeader> = Vec::new();
    let mut outstanding: HashSet<String> = HashSet::new();
    let mut needed_paths: Vec<String> = Vec::new();
    let mut manifest_complete = false;
    let mut files_written: u64 = 0;
    let mut bytes_written: u64 = 0;

    loop {
        let received = match transport.recv().await? {
            Some(f) => f,
            None => {
                return Err(eyre::Report::new(SessionFault::internal(
                    "peer closed mid-session",
                )))
            }
        };
        match received.frame {
            Some(Frame::ManifestEntry(header)) => {
                if manifest_complete {
                    return Err(violation(format!(
                        "manifest entry '{}' after ManifestComplete",
                        header.relative_path
                    )));
                }
                pending.push(header);
                if pending.len() >= DEST_DIFF_CHUNK {
                    let chunk = std::mem::take(&mut pending);
                    diff_chunk_and_send_needs(
                        transport,
                        chunk,
                        dst_root,
                        canonical_dst_root.as_deref(),
                        &compare_opts,
                        &mut outstanding,
                        &mut needed_paths,
                    )
                    .await?;
                }
            }
            Some(Frame::ManifestComplete(_complete)) => {
                if manifest_complete {
                    return Err(violation("duplicate ManifestComplete".into()));
                }
                // (scan_complete gates mirror purges from otp-6 on;
                // nothing consumes it in otp-3.)
                let chunk = std::mem::take(&mut pending);
                diff_chunk_and_send_needs(
                    transport,
                    chunk,
                    dst_root,
                    canonical_dst_root.as_deref(),
                    &compare_opts,
                    &mut outstanding,
                    &mut needed_paths,
                )
                .await?;
                // NeedComplete only after ManifestComplete received
                // AND every entry diffed — both true here.
                transport
                    .send(frame(Frame::NeedComplete(NeedComplete {})))
                    .await?;
                manifest_complete = true;
            }
            Some(Frame::FileBegin(header)) => {
                if !manifest_complete {
                    return Err(violation(format!(
                        "payload record for '{}' before ManifestComplete",
                        header.relative_path
                    )));
                }
                if !outstanding.remove(&header.relative_path) {
                    return Err(violation(format!(
                        "payload for '{}' which is not on the need list",
                        header.relative_path
                    )));
                }
                let outcome = receive_file_record(transport, &sink, &header).await?;
                files_written += outcome.files_written as u64;
                bytes_written += outcome.bytes_written;
            }
            Some(Frame::TarShardHeader(shard)) => {
                if !manifest_complete {
                    return Err(violation("tar shard record before ManifestComplete".into()));
                }
                for h in &shard.files {
                    if !outstanding.remove(&h.relative_path) {
                        return Err(violation(format!(
                            "tar shard entry '{}' which is not on the need list",
                            h.relative_path
                        )));
                    }
                }
                let outcome = receive_tar_record(transport, &sink, shard).await?;
                files_written += outcome.files_written as u64;
                bytes_written += outcome.bytes_written;
            }
            Some(Frame::SourceDone(_)) => {
                if !manifest_complete {
                    return Err(violation("SourceDone before ManifestComplete".into()));
                }
                if !outstanding.is_empty() {
                    return Err(violation(format!(
                        "SourceDone with {} needed file(s) never sent",
                        outstanding.len()
                    )));
                }
                let summary = TransferSummary {
                    files_transferred: files_written,
                    bytes_transferred: bytes_written,
                    entries_deleted: 0, // mirror lands at otp-6
                    in_stream_carrier_used: true,
                    files_resumed: 0, // resume lands at otp-7
                };
                transport.send(frame(Frame::Summary(summary))).await?;
                return Ok(DestinationOutcome {
                    summary,
                    needed_paths,
                });
            }
            Some(Frame::Error(err)) => {
                return Err(eyre::Report::new(SessionFault::from_wire(err)));
            }
            other => {
                // Everything else is off-lane or off-phase here:
                // destination-lane frames echoed back, resume frames
                // in a non-resume session (otp-7), resize with no
                // data plane to resize (otp-4), stray handshake
                // frames, bare FileData/TarShardChunk outside a
                // record. Fail fast, no tolerant parsing.
                return Err(violation(format!(
                    "{} not valid on the destination's receive lane in this phase",
                    frame_name(&other)
                )));
            }
        }
    }
}

/// Stat-and-compare one chunk of manifest entries on the blocking
/// pool (2+ syscalls per entry — same rationale as the daemon's
/// w4-4 chunked checks), then stream the resulting need batch.
async fn diff_chunk_and_send_needs(
    transport: &mut FrameTransport,
    chunk: Vec<FileHeader>,
    dst_root: &Path,
    canonical_dst_root: Option<&Path>,
    compare_opts: &CompareOptions,
    outstanding: &mut HashSet<String>,
    needed_paths: &mut Vec<String>,
) -> Result<()> {
    if chunk.is_empty() {
        return Ok(());
    }
    let dst_root = dst_root.to_path_buf();
    let canonical = canonical_dst_root.map(Path::to_path_buf);
    let opts = compare_opts.clone();
    let needed: Vec<String> = tokio::task::spawn_blocking(move || -> Result<Vec<String>> {
        let mut needed = Vec::new();
        for header in &chunk {
            if destination_needs(header, &dst_root, canonical.as_deref(), &opts)? {
                needed.push(header.relative_path.clone());
            }
        }
        Ok(needed)
    })
    .await
    .map_err(|err| eyre::eyre!("destination diff task panicked: {err}"))??;

    let entries: Vec<NeedEntry> = needed
        .into_iter()
        // A path the source manifests twice is diffed twice but
        // needed at most once.
        .filter(|path| outstanding.insert(path.clone()))
        .map(|relative_path| {
            needed_paths.push(relative_path.clone());
            NeedEntry {
                relative_path,
                resume: false, // resume lands at otp-7
            }
        })
        .collect();
    if entries.is_empty() {
        return Ok(());
    }
    transport
        .send(frame(Frame::NeedBatch(NeedBatch { entries })))
        .await?;
    Ok(())
}

/// Does the destination need this manifest entry? Stats its own file
/// and delegates the verdict to `manifest::header_transfer_status` —
/// the same mode-aware owner `compare_manifests` uses, fed from a
/// live stat instead of a materialized target manifest.
fn destination_needs(
    header: &FileHeader,
    dst_root: &Path,
    canonical_dst_root: Option<&Path>,
    opts: &CompareOptions,
) -> Result<bool> {
    let dst = match canonical_dst_root {
        Some(canonical) => {
            crate::path_safety::safe_join_contained(canonical, dst_root, &header.relative_path)
        }
        None => crate::path_safety::safe_join(dst_root, &header.relative_path),
    }
    .map_err(|err| {
        SessionFault::protocol_violation(format!(
            "manifest path '{}' escapes the destination root: {err:#}",
            header.relative_path
        ))
    })?;

    let target = match std::fs::metadata(&dst) {
        Ok(meta) if meta.is_file() => {
            let mtime = match meta.modified() {
                Ok(t) => match t.duration_since(std::time::UNIX_EPOCH) {
                    Ok(d) => d.as_secs() as i64,
                    Err(e) => -(e.duration().as_secs() as i64),
                },
                Err(_) => 0,
            };
            Some((meta.len(), mtime))
        }
        // Absent — or present as a directory/other, which a file
        // write must replace: both diff as "target does not have it"
        // (matches the push daemon's file_requires_upload).
        _ => None,
    };
    let status = header_transfer_status(
        header,
        // Destination-side checksums are never precomputed; Checksum
        // mode therefore transfers (the conservative arm of
        // compare_file), matching what push does today.
        target.map(|(size, mtime)| (size, mtime, &[] as &[u8])),
        opts,
    );
    Ok(matches!(status, FileStatus::New | FileStatus::Modified))
}

/// Receive one strictly-serialized file record (`file_begin` already
/// consumed) and stream its bytes into the sink through a bounded
/// in-memory pipe — record completion is exactly `header.size`
/// cumulative bytes (contract §Transport selection).
async fn receive_file_record(
    transport: &mut FrameTransport,
    sink: &FsTransferSink,
    header: &FileHeader,
) -> Result<crate::remote::transfer::SinkOutcome> {
    let (mut pipe_wr, mut pipe_rd) = tokio::io::duplex(FILE_RECORD_PIPE_BYTES);
    let write = sink.write_file_stream(header, &mut pipe_rd);
    let feed = async {
        let mut remaining = header.size;
        while remaining > 0 {
            let received = match transport.recv().await? {
                Some(f) => f,
                None => {
                    return Err(eyre::Report::new(SessionFault::internal(format!(
                        "peer closed inside file record '{}'",
                        header.relative_path
                    ))))
                }
            };
            match received.frame {
                Some(Frame::FileData(data)) => {
                    let len = data.content.len() as u64;
                    if len > remaining {
                        return Err(violation(format!(
                            "file record '{}' overran its size by {} byte(s)",
                            header.relative_path,
                            len - remaining
                        )));
                    }
                    pipe_wr.write_all(&data.content).await?;
                    remaining -= len;
                }
                other => {
                    // Strict serialization: nothing may interleave
                    // with an open record on the source lane.
                    return Err(violation(format!(
                        "{} inside file record '{}' ({} byte(s) short)",
                        frame_name(&other),
                        header.relative_path,
                        remaining
                    )));
                }
            }
        }
        pipe_wr.shutdown().await?;
        Ok(())
    };
    let (outcome, ()) = tokio::try_join!(write, feed)?;
    Ok(outcome)
}

/// Receive one tar-shard record (`tar_shard_header` already consumed):
/// buffer to exactly `archive_size` (bounded by the shared tar cap)
/// and hand the archive to the sink's tar-safety unpack path.
async fn receive_tar_record(
    transport: &mut FrameTransport,
    sink: &FsTransferSink,
    shard: TarShardHeader,
) -> Result<crate::remote::transfer::SinkOutcome> {
    if shard.archive_size > MAX_TAR_SHARD_BYTES {
        return Err(violation(format!(
            "tar shard of {} bytes exceeds the {} byte cap",
            shard.archive_size, MAX_TAR_SHARD_BYTES
        )));
    }
    let mut data: Vec<u8> = Vec::new();
    data.try_reserve_exact(shard.archive_size as usize)
        .map_err(|err| eyre::eyre!("allocating {} byte tar shard: {err}", shard.archive_size))?;
    loop {
        let received = match transport.recv().await? {
            Some(f) => f,
            None => {
                return Err(eyre::Report::new(SessionFault::internal(
                    "peer closed inside tar shard record",
                )))
            }
        };
        match received.frame {
            Some(Frame::TarShardChunk(chunk)) => {
                if data.len() as u64 + chunk.content.len() as u64 > shard.archive_size {
                    return Err(violation(format!(
                        "tar shard record overran its declared {} bytes",
                        shard.archive_size
                    )));
                }
                data.extend_from_slice(&chunk.content);
            }
            Some(Frame::TarShardComplete(_)) => {
                if data.len() as u64 != shard.archive_size {
                    return Err(violation(format!(
                        "tar shard record completed at {} of {} declared bytes",
                        data.len(),
                        shard.archive_size
                    )));
                }
                return sink
                    .write_payload(PreparedPayload::TarShard {
                        headers: shard.files,
                        data,
                    })
                    .await;
            }
            other => {
                return Err(violation(format!(
                    "{} inside tar shard record",
                    frame_name(&other)
                )));
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn build_id_has_version_and_git_components() {
        let id = session_build_id();
        let (version, git) = id.split_once('+').expect("build id must be version+git");
        assert_eq!(version, env!("CARGO_PKG_VERSION"));
        assert!(!git.is_empty(), "git component must be non-empty");
    }

    #[test]
    fn fault_round_trips_the_wire_shape() {
        let fault = SessionFault {
            code: session_error::Code::BuildMismatch,
            message: "boom".into(),
            local_build_id: "1.0+aaa".into(),
            peer_build_id: "1.0+bbb".into(),
            peer_notified: false,
        };
        let wire = fault.to_wire();
        let back = SessionFault::from_wire(wire);
        assert_eq!(back.code, session_error::Code::BuildMismatch);
        assert_eq!(back.message, "boom");
        // from_wire swaps perspective: the sender's local is our peer.
        assert_eq!(back.peer_build_id, "1.0+aaa");
        assert_eq!(back.local_build_id, "1.0+bbb");
        assert!(back.peer_notified);
    }
}
