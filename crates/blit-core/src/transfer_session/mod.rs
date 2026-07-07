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

mod data_plane;
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
    session_error, ComparisonMode, DataPlaneResize, DataPlaneResizeAck, DataPlaneResizeOp,
    FileData, FileHeader, FilterSpec, ManifestComplete, MirrorMode, NeedBatch, NeedComplete,
    NeedEntry, SessionAccept, SessionError, SessionHello, SessionOpen, SourceDone,
    TarShardComplete, TarShardHeader, TransferFrame, TransferRole, TransferSummary,
};
use crate::manifest::{header_transfer_status, CompareOptions, FileStatus};
use crate::remote::transfer::diff_planner;
use crate::remote::transfer::payload::PreparedPayload;
use crate::remote::transfer::sink::{FsSinkConfig, FsTransferSink, TransferSink};
use crate::remote::transfer::source::{FsTransferSource, TransferSource};
use crate::remote::transfer::stall_guard::TRANSFER_STALL_TIMEOUT;
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
    /// Host to dial the granted TCP data plane on (otp-4b). The
    /// initiator connected the control plane to this host; the data
    /// plane rides the same host on the granted port (contract
    /// §Transport: the initiator always dials). `None` disables the
    /// data plane at this end — a grant then faults, since the responder
    /// is waiting to accept sockets that would never arrive.
    pub data_plane_host: Option<String>,
}

pub struct DestinationSessionConfig {
    pub hello: HelloConfig,
    pub endpoint: SessionEndpoint,
    /// Host to dial the granted TCP data plane on when this end is the
    /// **initiator** (pull-equivalent, otp-5b): the DESTINATION initiator
    /// dials the SOURCE responder's granted sockets on the same host it
    /// reached the control plane on (contract §Transport: the initiator
    /// always dials). `None` — or a DESTINATION responder, which binds
    /// rather than dials — falls back to the in-stream carrier. Symmetric
    /// with [`SourceSessionConfig::data_plane_host`].
    pub data_plane_host: Option<String>,
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

/// Build a `SessionError` frame with the given code and message — the
/// wire form an end sends to tell its peer why it is aborting. Public
/// so the daemon dispatcher can emit `CANCELLED` when a `CancelJob`
/// fires mid-session (the session future is aborted by the select and
/// cannot send it itself — otp-4a codex F1); blit-core stays the one
/// owner of the frame grammar. The build-id fields are left empty:
/// they are only meaningful for `BUILD_MISMATCH`.
pub fn session_error_frame(code: session_error::Code, message: impl Into<String>) -> TransferFrame {
    frame(Frame::Error(SessionError {
        code: code as i32,
        message: message.into(),
        local_build_id: String::new(),
        peer_build_id: String::new(),
    }))
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

/// Where a SOURCE responder reads from. Symmetric with
/// [`DestinationTarget`]: `Fixed` is a source known up front (an
/// initiator's own tree, or a test), `Resolve` defers to the same
/// [`OpenResolver`] the destination path uses to map a received
/// `SessionOpen`'s module name to a local root, from which a
/// [`FsTransferSource`] is built inside blit-core (so callers stay free
/// of the concrete source type, exactly as `run_destination` builds its
/// sink from `dst_root`). A `Resolve` target is meaningful only on a
/// Responder; an Initiator always knows its own source. Used by
/// [`run_responder`] for the daemon-as-SOURCE (pull-equivalent, otp-5).
pub enum SourceResponderTarget {
    Fixed(Arc<dyn TransferSource>),
    Resolve(Box<OpenResolver>),
}

/// What a served session produced, tagged by which role the responder
/// played. `run_responder` dispatches on the initiator's declared role,
/// so the caller (the daemon) learns after the fact which half ran.
pub enum ResponderOutcome {
    /// The initiator was SOURCE; this end received (push-equivalent).
    Destination(DestinationOutcome),
    /// The initiator was DESTINATION; this end sent (pull-equivalent).
    Source(TransferSummary),
}

fn source_open_validator(open: &SessionOpen) -> std::result::Result<(), SessionFault> {
    if open.resume.as_ref().is_some_and(|r| r.enabled) {
        return Err(SessionFault::internal(
            "resume is not implemented on the unified session yet (otp-7)",
        ));
    }
    // otp-6a: filters are honored on the source scan (see
    // `source_send_half`). Validate the globs here so a malformed pattern
    // from a peer is refused at OPEN — peer-notified on the responder —
    // rather than faulting mid-scan once bytes are already moving.
    if let Some(filter) = open.filter.as_ref() {
        if *filter != FilterSpec::default() {
            crate::remote::transfer::operation_spec::filter_from_spec(filter.clone())
                .map_err(|e| SessionFault::protocol_violation(format!("invalid filter: {e:#}")))?;
        }
    }
    Ok(())
}

fn destination_open_validator(open: &SessionOpen) -> std::result::Result<(), SessionFault> {
    // otp-6b: mirror is executed on the DESTINATION (the end that owns the
    // dest tree). An enabled mirror needs a concrete scope; reject the
    // contradictory "enabled but OFF/unspecified kind" combination here.
    if open.mirror_enabled {
        let kind = MirrorMode::try_from(open.mirror_kind).unwrap_or(MirrorMode::Unspecified);
        if !matches!(kind, MirrorMode::FilteredSubset | MirrorMode::All) {
            return Err(SessionFault::protocol_violation(
                "mirror_enabled requires mirror_kind FILTERED_SUBSET or ALL",
            ));
        }
    }
    // The dest enumerates its tree through this filter when scoping a
    // FilteredSubset mirror, so its globs must be valid — validate at OPEN
    // (peer-notified refusal), symmetric with `source_open_validator`.
    if let Some(filter) = open.filter.as_ref() {
        if *filter != FilterSpec::default() {
            crate::remote::transfer::operation_spec::filter_from_spec(filter.clone())
                .map_err(|e| SessionFault::protocol_violation(format!("invalid filter: {e:#}")))?;
        }
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
    /// The responder's reply. The SOURCE initiator reads
    /// `accept.data_plane` to decide dial-vs-in-stream (otp-4b).
    accept: SessionAccept,
    /// The write root a Responder's [`OpenResolver`] produced from the
    /// received open, if one was supplied; `None` for an Initiator or a
    /// fixed-root Responder (the caller supplies the root then).
    resolved_root: Option<PathBuf>,
    /// The bound data-plane listener + credentials a DESTINATION
    /// Responder prepared before its `SessionAccept` (otp-4b). `None`
    /// on an Initiator, or when the responder granted no data plane
    /// (in-stream carrier). Consumed by the DESTINATION accept loop.
    responder_data_plane: Option<data_plane::ResponderDataPlane>,
}

/// HELLO both ways, exact match (D-2026-07-05-2). First frame each
/// direction; no ordering between the two directions. Factored out so a
/// serving end (`run_responder`) can exchange HELLO, then read the OPEN
/// and dispatch on the declared role before running a role driver.
async fn exchange_hello(transport: &mut FrameTransport, hello: &HelloConfig) -> Result<()> {
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
    Ok(())
}

/// The responder half of establish AFTER the `SessionOpen` is read:
/// complement check, `validate_open`, endpoint resolution, data-plane
/// prepare, and `SessionAccept`. Factored out so both `establish` (which
/// reads the open then calls this) and `run_responder` (which reads the
/// open, dispatches on the declared role, then calls this with the
/// resolved local role) share one implementation. Sends the refusal
/// `SessionError` itself; returned faults are `peer_notified`.
async fn responder_finish(
    transport: &mut FrameTransport,
    open: SessionOpen,
    local_role: TransferRole,
    validate_open: &OpenValidator,
    resolve_open: Option<&OpenResolver>,
) -> Result<Negotiated> {
    // The initiator declares ITS role; this responder end must
    // hold the complement.
    let declared = TransferRole::try_from(open.initiator_role).unwrap_or(TransferRole::Unspecified);
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
                        SessionFault::read_only("destination module is read-only".to_string()),
                    )
                    .await);
                }
                Some(resolved.root)
            }
            Err(fault) => return Err(notify_and_wrap(transport, fault).await),
        },
        None => None,
    };
    // Data plane (otp-4b/5b): a responder binds a TCP listener and grants
    // it, unless the initiator requested the in-stream carrier or the bind
    // fails (grant-less accept ⇒ in-stream fallback). This is role-agnostic
    // (otp-5b): the RESPONDER binds+accepts and the INITIATOR dials, while
    // byte direction is set by role — a DESTINATION responder accepts+
    // receives (push, otp-4b), a SOURCE responder accepts+sends (pull,
    // otp-5b). The bound listener travels in `Negotiated.responder_data_plane`
    // and is consumed by whichever role's driver runs.
    let responder_data_plane = if open.in_stream_bytes {
        None
    } else {
        data_plane::prepare_responder_data_plane().await
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
        // Grant present ⇒ TCP data plane; absent ⇒ in-stream.
        data_plane: responder_data_plane.as_ref().map(|dp| dp.grant()),
    };
    transport.send(frame(Frame::Accept(accept.clone()))).await?;
    Ok(Negotiated {
        open,
        accept,
        resolved_root,
        responder_data_plane,
    })
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
    exchange_hello(transport, hello).await?;

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
                responder_data_plane: None,
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
            responder_finish(transport, open, local_role, validate_open, resolve_open).await
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
    /// The destination's ack of a `DataPlaneResize{ADD}` (otp-4b-2). The
    /// send half dials the epoch-N socket on `accepted`.
    ResizeAck(DataPlaneResizeAck),
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
        // run_source only ever resolves nothing: a SOURCE *initiator*
        // owns its own root, and a SOURCE *responder* driven directly
        // (the in-process role suite) is handed a Fixed source. The
        // daemon SOURCE responder resolves module→root inside
        // `run_responder`, not here (otp-5).
        None,
    )
    .await?;

    drive_source(
        cfg.plan_options,
        cfg.data_plane_host,
        negotiated,
        transport,
        source,
    )
    .await
}

/// The SOURCE session body after establish: spawn the receive half,
/// run the send half, and map a fault to a peer-notified report. Shared
/// by [`run_source`] (initiator or direct-responder) and
/// [`run_responder`] (the daemon SOURCE responder), so the send/receive
/// choreography is single-sourced.
async fn drive_source(
    plan_options: PlanOptions,
    data_plane_host: Option<String>,
    mut negotiated: Negotiated,
    transport: FrameTransport,
    source: Arc<dyn TransferSource>,
) -> Result<TransferSummary> {
    // A SOURCE responder (pull, otp-5b) carries a bound listener to accept
    // its send sockets on; a SOURCE initiator (push) has none and dials the
    // grant it received instead. Take it here so the send half owns it.
    let responder_data_plane = negotiated.responder_data_plane.take();
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
        plan_options,
        data_plane_host.as_deref(),
        &negotiated,
        responder_data_plane,
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
            Some(Frame::ResizeAck(ack)) => {
                // The destination's response to a shape-resize proposal
                // (otp-4b-2). Forward it to the send half, which owns the
                // dial and dials the epoch-N socket on `accepted`.
                let _ = events.send(SourceEvent::ResizeAck(ack));
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

#[allow(clippy::too_many_arguments)]
async fn source_send_half(
    plan_options: PlanOptions,
    data_plane_host: Option<&str>,
    negotiated: &Negotiated,
    responder_data_plane: Option<data_plane::ResponderDataPlane>,
    tx: &mut Box<dyn FrameTx>,
    source: Arc<dyn TransferSource>,
    sent: Arc<StdMutex<HashMap<String, FileHeader>>>,
    manifest_sent: &AtomicBool,
    mut events: mpsc::UnboundedReceiver<SourceEvent>,
) -> Result<TransferSummary> {
    let mut pending: Vec<FileHeader> = Vec::new();
    let mut need_complete = false;

    // Data plane (otp-4b/5b): set up the send sockets up front — BEFORE
    // streaming the manifest — so the peer sees the connections promptly
    // rather than waiting out a bounded-accept/connect timeout while a long
    // manifest streams. Which end connects depends on connection role
    // (otp-5b): a SOURCE **responder** (pull) accepts sockets off its bound
    // listener; a SOURCE **initiator** (push) dials the grant it received.
    // Byte direction is the same either way (SOURCE sends), so both yield a
    // `SourceDataPlane` driven identically below. `None` on both ⇒ the
    // in-stream carrier (fallback), which needs no early setup.
    let mut data_plane = match responder_data_plane {
        // SOURCE responder (pull, otp-5b): accept + send. The DESTINATION
        // initiator advertised its capacity in the open (byte RECEIVER
        // advertises, wherever it initiates); the accept plane is single-
        // stream (otp-5b-1).
        Some(bound) => Some(
            data_plane::accept_source_data_plane(
                bound,
                negotiated.open.receiver_capacity.as_ref(),
                Arc::clone(&source),
            )
            .await?,
        ),
        // SOURCE initiator (push, otp-4b): dial the grant if the responder
        // granted a data plane; else in-stream.
        None => match &negotiated.accept.data_plane {
            Some(grant) => {
                let host = data_plane_host.ok_or_else(|| {
                    eyre::Report::new(SessionFault::internal(
                        "responder granted a TCP data plane but this initiator has no host to dial",
                    ))
                })?;
                Some(
                    data_plane::dial_source_data_plane(
                        host,
                        grant,
                        negotiated.accept.receiver_capacity.as_ref(),
                        Arc::clone(&source),
                    )
                    .await?,
                )
            }
            None => None,
        },
    };

    // sf-2 shape correction (otp-4b-2): running totals of the need list,
    // fed to the shape table so the SOURCE grows the data-plane stream
    // count as the workload's shape becomes known. Append-only (a need is
    // counted once, when it arrives), and the in-flight resize record the
    // ack is matched against (at most one — the dial enforces it).
    let mut needed_bytes: u64 = 0;
    let mut needed_count: usize = 0;
    let mut pending_resize: Option<data_plane::PendingResize> = None;

    // Streaming manifest: entries go out as enumeration produces them
    // (immediate start in every direction — plan §Design 2). The open
    // carries no source path (the source end owns its local endpoint) but
    // does carry the include/exclude/size/age filter (otp-6a): only
    // matching files are manifested and transferred. The filter MUST ride
    // the wire (not be pre-wrapped by a local caller) because for pull the
    // SOURCE is the remote daemon responder — it, not the client, owns the
    // scan. Apply it through the universal `FilteredSource` decorator, the
    // single filter chokepoint every source impl routes through, rather
    // than the per-impl `scan(filter)` arg which only `FsTransferSource`
    // honors (`RemoteTransferSource` ignores it — codex otp-6a F1). A
    // default/absent filter scans everything (unchanged from otp-3). Globs
    // were validated at OPEN (`source_open_validator`), so the conversion
    // cannot fail on a validated open; map any error to a fault regardless.
    let scan_source: Arc<dyn TransferSource> = match negotiated.open.filter.as_ref() {
        Some(spec) if *spec != FilterSpec::default() => {
            let filter = crate::remote::transfer::operation_spec::filter_from_spec(spec.clone())
                .map_err(|e| {
                    eyre::Report::new(SessionFault::internal(format!("invalid filter: {e:#}")))
                })?;
            Arc::new(crate::remote::transfer::source::FilteredSource::new(
                Arc::clone(&source),
                filter,
            ))
        }
        _ => Arc::clone(&source),
    };
    let unreadable: Arc<StdMutex<Vec<String>>> = Arc::default();
    let (mut header_rx, scan_handle) = scan_source.scan(None, Arc::clone(&unreadable));
    while let Some(header) = header_rx.recv().await {
        sent.lock()
            .expect("sent-manifest lock poisoned")
            .insert(header.relative_path.clone(), header.clone());
        tx.send(frame(Frame::ManifestEntry(header))).await?;
        // Faults detected by the receive half abort the stream now,
        // not after the full scan; needs just accumulate. (Resize acks
        // cannot arrive yet — none is proposed before the payload phase.)
        drain_ready_source_events(
            &mut events,
            &mut pending,
            &mut need_complete,
            &mut needed_bytes,
            &mut needed_count,
            data_plane.as_ref(),
            tx,
            &mut pending_resize,
        )
        .await?;
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

    // Payload phase. The byte carrier is either the TCP data plane
    // (dialed above) or the in-stream record grammar (fallback). Needs
    // accumulated while a batch was being sent become the next planner
    // batch (contract §Transport selection); payloads only flow after
    // ManifestComplete.
    // The in-stream carrier reuses one read buffer across records; the
    // data plane owns its own pooled buffers, so skip that allocation.
    let mut read_buf = if data_plane.is_none() {
        vec![0u8; IN_STREAM_CHUNK]
    } else {
        Vec::new()
    };
    loop {
        drain_ready_source_events(
            &mut events,
            &mut pending,
            &mut need_complete,
            &mut needed_bytes,
            &mut needed_count,
            data_plane.as_ref(),
            tx,
            &mut pending_resize,
        )
        .await?;
        if !pending.is_empty() {
            let batch = std::mem::take(&mut pending);
            match &mut data_plane {
                Some(dp) => {
                    // sf-2: correct the stream count toward the shape the
                    // accumulated need list implies before queueing this
                    // batch (one ADD per epoch; a no-op while one is in
                    // flight or the shape wants no more).
                    maybe_propose_resize(dp, tx, needed_bytes, needed_count, &mut pending_resize)
                        .await?;
                    let payloads =
                        diff_planner::plan_push_payloads(batch, source.root(), plan_options)?;
                    // A cancel while earlier batches are actively moving
                    // closes the send pipeline under backpressure, so this
                    // queue fails with a data-plane error — prefer the
                    // peer's framed reason (CANCELLED) the same way the
                    // finish() drain does (otp-4b-3 codex F1). Not raced
                    // against events like finish(): live `Need`s still
                    // arrive here, and `recv_peer_fault` would consume them.
                    if let Err(dp_err) = dp.queue(payloads).await {
                        return Err(prefer_peer_fault(&mut events, dp_err).await);
                    }
                }
                None => {
                    send_payload_records(tx, &source, plan_options, batch, &mut read_buf).await?;
                }
            }
            continue;
        }
        if need_complete {
            break;
        }
        match events.recv().await {
            Some(event) => {
                process_source_event(
                    event,
                    &mut pending,
                    &mut need_complete,
                    &mut needed_bytes,
                    &mut needed_count,
                    data_plane.as_ref(),
                    tx,
                    &mut pending_resize,
                )
                .await?;
            }
            None => {
                return Err(eyre::Report::new(SessionFault::internal(
                    "source receive half ended before NeedComplete",
                )))
            }
        }
    }

    // A resize proposed on the last batch may still be in flight. Resolve
    // it BEFORE finishing so the destination's armed slot is consumed by
    // the dialed socket — an armed-but-never-dialed credential would hang
    // its accept loop (which waits for every arm to be claimed). We do not
    // propose further here: exactly the one in-flight resize is drained.
    if let Some(dp) = &data_plane {
        if let Some(pending) = pending_resize.take() {
            resolve_in_flight_resize(&mut events, dp, pending).await?;
        }
    }

    // Close the data plane BEFORE SourceDone so the destination's receive
    // pipeline sees each socket's END record and completes; SourceDone on
    // the control lane then lets the destination score and summarize.
    //
    // The drain is the byte-transfer phase's wall-time sink, so a
    // mid-transfer cancel almost always lands here. Race it against a
    // peer-framed fault on the control lane (otp-4b-3): a `CancelJob` on
    // the served session frames `SessionError{CANCELLED}`, and the source
    // must surface THAT — not the data-plane transport break it also
    // causes. Two orderings, both covered:
    //   * fault arrives while the drain is still pending (e.g. a worker
    //     blocked reading a slow file, so the socket break never unblocks
    //     it) → the `recv_peer_fault` arm wins; dropping the unfinished
    //     `finish()` future drops the data plane, and its `AbortOnDrop`
    //     stops the in-flight workers.
    //   * the socket break makes `finish()` return `Err` first → prefer
    //     the framed reason if the control lane delivers one within the
    //     stall window (`prefer_peer_fault`).
    if let Some(dp) = data_plane.take() {
        tokio::select! {
            biased;
            fault = recv_peer_fault(&mut events) => {
                return Err(eyre::Report::new(fault));
            }
            res = dp.finish() => {
                if let Err(dp_err) = res {
                    return Err(prefer_peer_fault(&mut events, dp_err).await);
                }
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
        Some(SourceEvent::ResizeAck(_)) => Err(eyre::Report::new(
            SessionFault::protocol_violation("DataPlaneResizeAck after SourceDone"),
        )),
        None => Err(eyre::Report::new(SessionFault::internal(
            "source receive half ended before TransferSummary",
        ))),
    }
}

/// Process every event ready right now (needs accumulating, resize acks
/// dialing their epoch-N socket) without blocking. Called between
/// manifest sends and at the top of the payload loop.
#[allow(clippy::too_many_arguments)]
async fn drain_ready_source_events(
    events: &mut mpsc::UnboundedReceiver<SourceEvent>,
    pending: &mut Vec<FileHeader>,
    need_complete: &mut bool,
    needed_bytes: &mut u64,
    needed_count: &mut usize,
    data_plane: Option<&data_plane::SourceDataPlane>,
    tx: &mut Box<dyn FrameTx>,
    pending_resize: &mut Option<data_plane::PendingResize>,
) -> Result<()> {
    while let Ok(event) = events.try_recv() {
        process_source_event(
            event,
            pending,
            need_complete,
            needed_bytes,
            needed_count,
            data_plane,
            tx,
            pending_resize,
        )
        .await?;
    }
    Ok(())
}

/// Handle one source event. Needs accumulate into `pending` and the
/// shape totals; a resize ack dials its epoch-N socket and proposes the
/// next ADD (the one-per-epoch ramp).
#[allow(clippy::too_many_arguments)]
async fn process_source_event(
    event: SourceEvent,
    pending: &mut Vec<FileHeader>,
    need_complete: &mut bool,
    needed_bytes: &mut u64,
    needed_count: &mut usize,
    data_plane: Option<&data_plane::SourceDataPlane>,
    tx: &mut Box<dyn FrameTx>,
    pending_resize: &mut Option<data_plane::PendingResize>,
) -> Result<()> {
    match event {
        SourceEvent::Need(header) => {
            if *need_complete {
                return Err(eyre::Report::new(SessionFault::protocol_violation(
                    format!("need for '{}' after NeedComplete", header.relative_path),
                )));
            }
            *needed_bytes = needed_bytes.saturating_add(header.size);
            *needed_count += 1;
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
        SourceEvent::ResizeAck(ack) => {
            let dp = data_plane.ok_or_else(|| {
                eyre::Report::new(SessionFault::protocol_violation(
                    "DataPlaneResizeAck on a session with no data plane",
                ))
            })?;
            // Match the ack to the in-flight proposal; stale/unsolicited
            // acks (wrong epoch, or none pending) are ignored, matching
            // old push. `take()` + restore keeps the borrow simple.
            let pending_r = match pending_resize.take() {
                Some(p) if p.epoch == ack.epoch => p,
                restored => {
                    *pending_resize = restored;
                    return Ok(());
                }
            };
            if ack.accepted {
                dp.add_stream(&pending_r.sub_token).await?;
                dp.dial()
                    .resize_settled(pending_r.epoch, pending_r.target_streams as usize, true);
            } else {
                dp.dial()
                    .resize_settled(pending_r.epoch, dp.dial().live_streams(), false);
            }
            // Ramp one stream per accepted epoch: propose the next ADD.
            maybe_propose_resize(dp, tx, *needed_bytes, *needed_count, pending_resize).await
        }
        SourceEvent::Summary(_) => Err(eyre::Report::new(SessionFault::protocol_violation(
            "TransferSummary before SourceDone",
        ))),
        SourceEvent::Fault(fault) => Err(eyre::Report::new(fault)),
    }
}

/// Propose one shape-correction resize (`DataPlaneResize{ADD}`) toward
/// the stream count the accumulated need list implies, if none is in
/// flight. A no-op when the shape wants no more than the live count (the
/// dial returns `None`). Sends the frame and records the in-flight
/// proposal for the ack to match.
async fn maybe_propose_resize(
    dp: &data_plane::SourceDataPlane,
    tx: &mut Box<dyn FrameTx>,
    needed_bytes: u64,
    needed_count: usize,
    pending_resize: &mut Option<data_plane::PendingResize>,
) -> Result<()> {
    if pending_resize.is_some() {
        return Ok(());
    }
    if let Some(proposal) = dp.propose_resize(needed_bytes, needed_count)? {
        tx.send(frame(Frame::Resize(DataPlaneResize {
            op: DataPlaneResizeOp::Add as i32,
            epoch: proposal.epoch,
            target_stream_count: proposal.target_streams,
            sub_token: proposal.sub_token.clone(),
        })))
        .await?;
        *pending_resize = Some(proposal);
    }
    Ok(())
}

/// Block for the ack of the one in-flight resize and dial its socket (or
/// settle it refused). Does NOT propose further — it resolves exactly the
/// pending proposal so the destination's armed slot is consumed before we
/// finish the data plane.
async fn resolve_in_flight_resize(
    events: &mut mpsc::UnboundedReceiver<SourceEvent>,
    dp: &data_plane::SourceDataPlane,
    pending: data_plane::PendingResize,
) -> Result<()> {
    loop {
        match events.recv().await {
            Some(SourceEvent::ResizeAck(ack)) if ack.epoch == pending.epoch => {
                if ack.accepted {
                    dp.add_stream(&pending.sub_token).await?;
                    dp.dial()
                        .resize_settled(pending.epoch, pending.target_streams as usize, true);
                } else {
                    dp.dial()
                        .resize_settled(pending.epoch, dp.dial().live_streams(), false);
                }
                return Ok(());
            }
            // A stale ack for an already-settled epoch: ignore, keep
            // waiting for ours.
            Some(SourceEvent::ResizeAck(_)) => continue,
            Some(SourceEvent::Fault(fault)) => return Err(eyre::Report::new(fault)),
            Some(SourceEvent::Need(h)) => {
                return Err(eyre::Report::new(SessionFault::protocol_violation(
                    format!("need for '{}' after NeedComplete", h.relative_path),
                )))
            }
            Some(SourceEvent::NeedComplete) => {
                return Err(eyre::Report::new(SessionFault::protocol_violation(
                    "duplicate NeedComplete",
                )))
            }
            Some(SourceEvent::Summary(_)) => {
                return Err(eyre::Report::new(SessionFault::protocol_violation(
                    "TransferSummary before SourceDone",
                )))
            }
            None => {
                return Err(eyre::Report::new(SessionFault::internal(
                    "source receive half ended with a resize in flight",
                )))
            }
        }
    }
}

/// Await the next terminal signal the receive half forwards while the
/// data-plane drain is in progress (otp-4b-3). Used to race the drain: a
/// mid-transfer `SessionError` (e.g. a `CancelJob` → `CANCELLED`) must
/// abort the send and surface as the fault.
///
/// The drain runs after `resolve_in_flight_resize` and before `SourceDone`
/// goes out, so the event channel is drained and the peer sends nothing
/// but (possibly) an abort frame — no `Need`, `NeedComplete`, `ResizeAck`,
/// or `Summary` is legitimate here. So a `Fault` is returned as-is and any
/// OTHER event is surfaced as a protocol violation rather than silently
/// dropped (codex otp-4b-3 F3): dropping it would defer or lose a
/// fail-fast error and, if the drain is itself stuck, hang. Parks forever
/// once the channel closes with no event so the data-plane future it
/// races decides the outcome instead.
async fn recv_peer_fault(events: &mut mpsc::UnboundedReceiver<SourceEvent>) -> SessionFault {
    match events.recv().await {
        Some(SourceEvent::Fault(fault)) => fault,
        Some(SourceEvent::Need(h)) => SessionFault::protocol_violation(format!(
            "need for '{}' during the data-plane drain (after NeedComplete)",
            h.relative_path
        )),
        Some(SourceEvent::NeedComplete) => {
            SessionFault::protocol_violation("duplicate NeedComplete during the data-plane drain")
        }
        Some(SourceEvent::ResizeAck(_)) => SessionFault::protocol_violation(
            "DataPlaneResizeAck during the data-plane drain (no resize is in flight)",
        ),
        Some(SourceEvent::Summary(_)) => {
            SessionFault::protocol_violation("TransferSummary before SourceDone")
        }
        None => std::future::pending().await,
    }
}

/// A data-plane operation (`queue`/`finish`) failed mid-transfer. The
/// break is usually the *symptom* of a peer abort — within
/// `TRANSFER_STALL_TIMEOUT` the peer (which runs the same stall guard on
/// its receive workers) always frames the real reason on the control
/// lane. Prefer that framed fault; fall back to the raw data-plane error
/// if the channel closes first or none arrives in that window.
///
/// Unlike `recv_peer_fault` (the finish()-drain select arm, which fails
/// fast on any stray event), this is called from BOTH error sites,
/// including the `queue()` error inside the payload loop — where a
/// legitimate `Need`/`NeedComplete`/`ResizeAck` may already be queued
/// ahead of the peer's `SessionError` (codex otp-4b-3 pass-2 F1). So it
/// SKIPS non-fault events rather than treating them as violations: we are
/// already unwinding on a data-plane error, and the framed fault (or the
/// dp error) is the correct outcome, never a spurious protocol violation.
async fn prefer_peer_fault(
    events: &mut mpsc::UnboundedReceiver<SourceEvent>,
    dp_err: eyre::Report,
) -> eyre::Report {
    let framed = async {
        loop {
            match events.recv().await {
                Some(SourceEvent::Fault(fault)) => break Some(fault),
                // Skip a still-in-flight need/ack/complete: on this error
                // path the transfer is aborting, so the framed reason (or
                // the dp error) wins, not a stray-event violation.
                Some(_) => continue,
                // Receive half ended without framing a fault → the raw
                // data-plane error is the best available cause.
                None => break None,
            }
        }
    };
    match tokio::time::timeout(TRANSFER_STALL_TIMEOUT, framed).await {
        Ok(Some(fault)) => eyre::Report::new(fault),
        Ok(None) | Err(_) => dp_err,
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
    /// The settled data-plane stream count this end observed (epoch-0 +
    /// accepted resizes), or `None` for the in-stream carrier. The sf-2
    /// pin (otp-4b-2) reads it to assert shape correction grew the
    /// stream set past the zero-knowledge single-stream grant.
    pub data_plane_streams: Option<usize>,
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

    drive_destination(
        &mut transport,
        negotiated,
        &dst_root,
        cfg.data_plane_host.as_deref(),
    )
    .await
}

/// The DESTINATION session body: run the diff/receive loop and map a
/// fault to a peer-notified report. Shared by [`run_destination`] and
/// [`run_responder`] (the daemon DESTINATION responder), so the receive
/// choreography is single-sourced.
async fn drive_destination(
    transport: &mut FrameTransport,
    negotiated: Negotiated,
    dst_root: &Path,
    data_plane_host: Option<&str>,
) -> Result<DestinationOutcome> {
    match destination_session(transport, negotiated, dst_root, data_plane_host).await {
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

/// Serve one transfer session as the RESPONDER, dispatching on the
/// initiator's declared role — the daemon's single serving entry
/// (contract §Invariants 3: one handshake, roles not directions). A
/// client that declares SOURCE makes this end the DESTINATION
/// (push-equivalent, otp-4); a client that declares DESTINATION makes
/// this end the SOURCE (pull-equivalent, otp-5). The two targets carry
/// the endpoint resolution for each role; only the one the initiator
/// selects is used. Returns a [`ResponderOutcome`] tagged with the role
/// that ran.
pub async fn run_responder(
    hello: HelloConfig,
    transport: FrameTransport,
    source_target: SourceResponderTarget,
    dest_target: DestinationTarget,
) -> Result<ResponderOutcome> {
    let mut transport = transport;
    exchange_hello(&mut transport, &hello).await?;
    let open = match expect_frame(&mut transport).await? {
        Frame::Open(o) => o,
        other => {
            return Err(notify_and_wrap(
                &mut transport,
                SessionFault::protocol_violation(format!(
                    "expected SessionOpen, got {}",
                    frame_name(&Some(other))
                )),
            )
            .await)
        }
    };
    let declared = TransferRole::try_from(open.initiator_role).unwrap_or(TransferRole::Unspecified);
    match declared {
        // Initiator SOURCE ⇒ this end is DESTINATION (push-equivalent).
        TransferRole::Source => {
            let resolve = match &dest_target {
                DestinationTarget::Resolve(resolver) => Some(resolver.as_ref()),
                DestinationTarget::Fixed(_) => None,
            };
            let negotiated = responder_finish(
                &mut transport,
                open,
                TransferRole::Destination,
                &destination_open_validator,
                resolve,
            )
            .await?;
            let dst_root = match negotiated.resolved_root.clone() {
                Some(root) => root,
                None => match &dest_target {
                    DestinationTarget::Fixed(root) => root.clone(),
                    DestinationTarget::Resolve(_) => {
                        return Err(eyre::Report::new(SessionFault::internal(
                            "resolver target produced no destination root",
                        )));
                    }
                },
            };
            // A DESTINATION responder (push) binds+accepts its receive
            // sockets — it never dials, so it needs no data-plane host.
            let outcome = drive_destination(&mut transport, negotiated, &dst_root, None).await?;
            Ok(ResponderOutcome::Destination(outcome))
        }
        // Initiator DESTINATION ⇒ this end is SOURCE (pull-equivalent).
        TransferRole::Destination => {
            let resolve = match &source_target {
                SourceResponderTarget::Resolve(resolver) => Some(resolver.as_ref()),
                SourceResponderTarget::Fixed(_) => None,
            };
            let negotiated = responder_finish(
                &mut transport,
                open,
                TransferRole::Source,
                &source_open_validator,
                resolve,
            )
            .await?;
            let source: Arc<dyn TransferSource> = match source_target {
                SourceResponderTarget::Fixed(source) => source,
                SourceResponderTarget::Resolve(_) => {
                    // A Resolve target always yields a root on the
                    // Responder branch (establish only skips resolution
                    // on the Initiator branch, which uses Fixed).
                    let root = negotiated.resolved_root.clone().ok_or_else(|| {
                        eyre::Report::new(SessionFault::internal(
                            "resolver target produced no source root",
                        ))
                    })?;
                    Arc::new(FsTransferSource::new(root))
                }
            };
            // The SOURCE owns its planner knobs; a daemon-served source
            // has no client-supplied ones (§Transport selection). A SOURCE
            // responder binds+accepts its send sockets (otp-5b) — it never
            // dials, so it needs no data-plane host.
            let summary =
                drive_source(PlanOptions::default(), None, negotiated, transport, source).await?;
            Ok(ResponderOutcome::Source(summary))
        }
        TransferRole::Unspecified => Err(notify_and_wrap(
            &mut transport,
            SessionFault::protocol_violation(
                "initiator declared no role (TRANSFER_ROLE_UNSPECIFIED)",
            ),
        )
        .await),
    }
}

fn violation(message: String) -> eyre::Report {
    eyre::Report::new(SessionFault::protocol_violation(message))
}

/// otp-6b: the DESTINATION's mirror delete pass — the session's single
/// delete rule. Plans (enumerate dest + diff against the complete source
/// file set) and executes the extraneous deletions, all blocking FS work,
/// so it runs on the blocking pool. Returns the count deleted.
///
/// Every target is containment-checked against the canonical destination
/// root before any filesystem op (the same chokepoint the sink write paths
/// use). Missing entries are tolerated — the pass is idempotent. Deletion
/// order is files then dirs deepest-first (the plan sorts them). `remove_dir`
/// (not `remove_dir_all`) is used so out-of-scope content is never removed:
/// under `FilteredSubset` an extraneous dir that still holds filter-excluded
/// files fails with ENOTEMPTY and is left alone; under `All` the tree was
/// enumerated unfiltered, so a dir reaching here is empty and a non-empty one
/// is a genuine error.
fn mirror_delete_pass(
    dst_root: &Path,
    source_files: &HashSet<String>,
    filter: &crate::fs_enum::FileFilter,
    tolerate_nonempty_dirs: bool,
    canonical_dst_root: Option<&Path>,
) -> Result<u64> {
    let plan = crate::mirror_planner::MirrorPlanner::new(false).plan_session_deletions(
        dst_root,
        source_files,
        filter,
    )?;

    let contained = |target: &Path| -> Result<()> {
        if let Some(root) = canonical_dst_root {
            crate::path_safety::verify_contained(root, target).map_err(|e| {
                eyre::eyre!("mirror delete containment {}: {e:#}", target.display())
            })?;
        }
        Ok(())
    };

    let mut deleted = 0u64;
    for file in &plan.files {
        contained(file)?;
        // Windows refuses to delete a read-only file; clear the attribute
        // first, matching the daemon purge (admin.rs) and local mirror
        // (engine/mirror.rs) executors (codex otp-6b F2).
        #[cfg(windows)]
        crate::win_fs::clear_readonly_recursive(file);
        match std::fs::remove_file(file) {
            Ok(()) => deleted += 1,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
            Err(e) => return Err(eyre::eyre!("mirror delete {}: {e}", file.display())),
        }
    }
    for dir in &plan.dirs {
        contained(dir)?;
        #[cfg(windows)]
        crate::win_fs::clear_readonly_recursive(dir);
        match std::fs::remove_dir(dir) {
            Ok(()) => deleted += 1,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
            // FilteredSubset: the dir still holds out-of-scope files the
            // filter excluded from enumeration; leaving it is the scope
            // contract, not a failure (engine/mirror.rs R58-F6). `Some(66)`
            // is ENOTEMPTY on macOS/BSD, which maps to a different ErrorKind.
            Err(e)
                if tolerate_nonempty_dirs
                    && (e.kind() == std::io::ErrorKind::DirectoryNotEmpty
                        || e.raw_os_error() == Some(66)) => {}
            Err(e) => return Err(eyre::eyre!("mirror delete dir {}: {e}", dir.display())),
        }
    }
    Ok(deleted)
}

async fn destination_session(
    transport: &mut FrameTransport,
    negotiated: Negotiated,
    dst_root: &Path,
    data_plane_host: Option<&str>,
) -> Result<DestinationOutcome> {
    let compare_mode = ComparisonMode::try_from(negotiated.open.compare_mode)
        .unwrap_or(ComparisonMode::Unspecified);
    let compare_opts = CompareOptions {
        mode: compare_mode.into(),
        ignore_existing: negotiated.open.ignore_existing,
        // Session deletions run via the otp-6b mirror pass (a whole-tree diff
        // at SourceDone), not the per-entry transfer-status diff below.
        include_deletions: false,
    };
    // src_root is only consumed by local File payloads, which never
    // occur on a session destination (payload bytes arrive as records
    // and go through the stream/tar write paths). `Arc` so the data-plane
    // receive task (otp-4b) can share the one sink across sockets.
    let sink = Arc::new(FsTransferSink::new(
        PathBuf::new(),
        dst_root.to_path_buf(),
        FsSinkConfig {
            preserve_times: true,
            dry_run: false,
            checksum: None,
            resume: false,
            compare_mode,
        },
    ));
    // Same canonical-containment chokepoint the sink write paths use
    // (R46-F3), applied to diff stats so a hostile manifest path can't
    // make the destination stat outside its root.
    let canonical_dst_root = crate::path_safety::canonical_dest_root(dst_root).ok();

    // otp-6b: mirror config. The DESTINATION owns the delete pass (it holds
    // the tree). `mirror_filter` scopes the dest enumeration — the user
    // filter for FilteredSubset (out-of-scope dest entries are never
    // candidates), the whole-tree default for All. Globs were validated at
    // OPEN. `source_files` accumulates the COMPLETE source file set (only
    // when mirroring) so the pass can diff it against the dest at SourceDone.
    let mirror_enabled = negotiated.open.mirror_enabled;
    let mirror_kind = MirrorMode::try_from(negotiated.open.mirror_kind).unwrap_or(MirrorMode::Off);
    let mirror_filter: crate::fs_enum::FileFilter = if mirror_enabled
        && mirror_kind == MirrorMode::FilteredSubset
    {
        match negotiated.open.filter.as_ref() {
            Some(spec) if *spec != FilterSpec::default() => {
                crate::remote::transfer::operation_spec::filter_from_spec(spec.clone()).map_err(
                    |e| eyre::Report::new(SessionFault::internal(format!("invalid filter: {e:#}"))),
                )?
            }
            _ => crate::fs_enum::FileFilter::default(),
        }
    } else {
        crate::fs_enum::FileFilter::default()
    };
    let mut source_files: HashSet<String> = HashSet::new();

    // Two sets, deliberately separate (codex otp-4b-1 fix-review F1):
    // `granted` is the ever-granted DEDUP set — control-loop-local,
    // insert-only, never removed, so a concurrent data-plane claim can
    // never re-open a grant (a duplicate manifest path is granted at
    // most once regardless of delivery timing). `outstanding` is the
    // not-yet-delivered COMPLETION set — inserted for each freshly
    // granted path before its NeedBatch, claimed by both carriers (the
    // in-stream arms inline, the data-plane NeedListSink as payloads
    // land), and empty at SourceDone. A count proxy was insufficient
    // (F1); merging the two into one set raced the data-plane claim
    // against the diff (fix-review F1).
    let mut granted: HashSet<String> = HashSet::new();
    let outstanding: data_plane::OutstandingNeeds = Arc::new(StdMutex::new(HashSet::new()));

    // Data plane (otp-4b/5b): when a TCP data plane is in play, payload
    // bytes arrive on sockets (not the control lane). Set it up NOW —
    // concurrent with the diff loop below, and before the peer sends — so
    // the connections are established promptly. Which end connects depends
    // on connection role (otp-5b): a DESTINATION **responder** (push)
    // accepts sockets off its bound listener; a DESTINATION **initiator**
    // (pull) dials the grant it received on `data_plane_host`. Byte
    // direction is the same either way (DESTINATION receives). The
    // NeedListSink gives the socket receive the same need-list strictness
    // the in-stream control loop applies inline; AbortOnDrop (inside the
    // responder run) bounds the accept task to this future. `resize_live`
    // tracks the stream count this end has grown to (epoch-0 plus each
    // accepted resize ADD) and `resize_ceiling` the receiver's advertised
    // max_streams — both directions resize (push arms+accepts, otp-4b-2;
    // pull dials, otp-5b-2), so both seed these from their epoch-0 streams.
    let recv_sink: Arc<dyn TransferSink> = Arc::new(data_plane::NeedListSink::new(
        Arc::clone(&sink) as Arc<dyn TransferSink>,
        Arc::clone(&outstanding),
    ));
    let (mut data_plane_recv, mut resize_live, resize_ceiling) = match negotiated
        .responder_data_plane
    {
        // DESTINATION responder (push, otp-4b): accept + receive.
        Some(rdp) => {
            let initial = rdp.initial_streams() as usize;
            let run = rdp.spawn(recv_sink);
            let ceiling = run.ceiling;
            (
                Some(data_plane::DestRecvPlane::Responder(run)),
                initial,
                ceiling,
            )
        }
        // DESTINATION initiator (pull, otp-5b): dial + receive when the
        // SOURCE responder granted a data plane and we have a host to dial.
        None => match (&negotiated.accept.data_plane, data_plane_host) {
            (Some(grant), Some(host)) => {
                let initial = grant.initial_streams.max(1) as usize;
                let run = data_plane::dial_destination_data_plane(host, grant, recv_sink).await?;
                // otp-5b-2: the pull data plane resizes too. Seed
                // `resize_live` from the epoch-0 streams dialed and bound
                // growth by the capacity THIS end advertised in its open
                // (it is the byte receiver) — the exact ceiling the SOURCE
                // responder's dial already clamps to, so both ends agree
                // even when the caller advertised a max_streams below this
                // host's fresh local reading (codex otp-5b-2 F1). On a
                // Resize frame the initiator dials the epoch-N socket (vs
                // the responder path's arm).
                let ceiling = negotiated
                    .open
                    .receiver_capacity
                    .as_ref()
                    .map(|c| c.max_streams)
                    .unwrap_or(0)
                    .max(1) as usize;
                (
                    Some(data_plane::DestRecvPlane::Initiator(run)),
                    initial,
                    ceiling,
                )
            }
            // A grant with no host to dial is an inconsistent initiator
            // config: fail fast, mirroring the SOURCE initiator
            // (`source_send_half`). The SOURCE responder has already bound
            // and blocks accepting the socket this end would dial, so
            // silently taking the in-stream branch cannot fall back — it
            // would deadlock until the responder's accept times out. A
            // grant means the initiator MUST dial (contract §Transport).
            // (codex otp-5b-1 finding.)
            (Some(_), None) => {
                return Err(eyre::Report::new(SessionFault::internal(
                    "responder granted a TCP data plane but this DESTINATION \
                     initiator has no host to dial",
                )))
            }
            // No grant (the responder could not bind, or the initiator
            // asked for in-stream): the in-stream carrier.
            (None, _) => (None, 0usize, 0usize),
        },
    };

    let mut pending: Vec<FileHeader> = Vec::new();
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
                // otp-6b: retain the full source path set for the mirror
                // diff (the need list keeps only files needing transfer).
                if mirror_enabled {
                    source_files.insert(header.relative_path.clone());
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
                        &mut granted,
                        &outstanding,
                        &mut needed_paths,
                    )
                    .await?;
                }
            }
            Some(Frame::ManifestComplete(complete)) => {
                if manifest_complete {
                    return Err(violation("duplicate ManifestComplete".into()));
                }
                // otp-6b: mirror deletions are data-loss-dangerous when the
                // source scan was incomplete — a source file missing from an
                // aborted scan would be misclassified extraneous and deleted
                // at the dest. Refuse here (before any transfer or deletion)
                // rather than partial-mirror. Matches the old paths'
                // require-complete-scan guard.
                if mirror_enabled && !complete.scan_complete {
                    return Err(eyre::Report::new(SessionFault::internal(
                        "mirror refused: the source scan did not complete \
                         (unreadable paths) — deleting now could remove files \
                         the source still has",
                    )));
                }
                let chunk = std::mem::take(&mut pending);
                diff_chunk_and_send_needs(
                    transport,
                    chunk,
                    dst_root,
                    canonical_dst_root.as_deref(),
                    &compare_opts,
                    &mut granted,
                    &outstanding,
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
                // Payload records ride the control lane only under the
                // in-stream carrier; with a TCP data plane active they
                // flow over the sockets, so one here is a violation.
                if data_plane_recv.is_some() {
                    return Err(violation(format!(
                        "file record '{}' on the control lane while a TCP data plane is active",
                        header.relative_path
                    )));
                }
                if !manifest_complete {
                    return Err(violation(format!(
                        "payload record for '{}' before ManifestComplete",
                        header.relative_path
                    )));
                }
                if !outstanding
                    .lock()
                    .expect("outstanding-needs lock poisoned")
                    .remove(&header.relative_path)
                {
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
                if data_plane_recv.is_some() {
                    return Err(violation(
                        "tar shard record on the control lane while a TCP data plane is active"
                            .into(),
                    ));
                }
                if !manifest_complete {
                    return Err(violation("tar shard record before ManifestComplete".into()));
                }
                {
                    let mut out = outstanding.lock().expect("outstanding-needs lock poisoned");
                    for h in &shard.files {
                        if !out.remove(&h.relative_path) {
                            return Err(violation(format!(
                                "tar shard entry '{}' which is not on the need list",
                                h.relative_path
                            )));
                        }
                    }
                }
                let outcome = receive_tar_record(transport, &sink, shard).await?;
                files_written += outcome.files_written as u64;
                bytes_written += outcome.bytes_written;
            }
            Some(Frame::Resize(resize)) => {
                // sf-2 shape correction (otp-4b-2 push, otp-5b-2 pull): the
                // SOURCE proposes one ADD; the DESTINATION grows its receive
                // set (bump `resize_live`) and acks so the SOURCE completes
                // the epoch-N socket. The control-lane frames are identical
                // in both directions — only the transport action flips: a
                // DESTINATION **responder** (push) ARMS a credential its
                // accept loop then accepts; a DESTINATION **initiator**
                // (pull) DIALS the epoch-N socket itself. Only ADD occurs
                // (REMOVE is a tuner concern, future work); anything else
                // fails fast.
                if data_plane_recv.is_none() {
                    return Err(violation(
                        "DataPlaneResize on a session with no data plane".into(),
                    ));
                }
                let op = DataPlaneResizeOp::try_from(resize.op)
                    .unwrap_or(DataPlaneResizeOp::Unspecified);
                if op != DataPlaneResizeOp::Add {
                    return Err(violation(format!(
                        "unsupported data-plane resize op {}",
                        op.as_str_name()
                    )));
                }
                if resize.sub_token.len() != crate::remote::transfer::SUB_TOKEN_LEN {
                    return Err(violation(
                        "DataPlaneResize sub_token must be 16 bytes".into(),
                    ));
                }
                // Cumulative ceiling bound (defense in depth — the source's
                // dial already clamps to the same profile). Under the
                // ceiling, grow per connection role: arm the credential
                // (responder) or dial the epoch-N socket (initiator). A
                // dial failure is fatal (`add_dialed_stream`); a gone accept
                // loop returns false (arm). The initiator dials BEFORE the
                // ack so the SOURCE responder — which accepts on the ack —
                // never commits to an accept the DESTINATION did not dial.
                let accepted = if resize_live < resize_ceiling {
                    match data_plane_recv
                        .as_mut()
                        .expect("data plane present (checked above)")
                    {
                        data_plane::DestRecvPlane::Responder(run) => {
                            run.arm(resize.sub_token.clone())
                        }
                        data_plane::DestRecvPlane::Initiator(run) => {
                            run.add_dialed_stream(&resize.sub_token).await?;
                            true
                        }
                    }
                } else {
                    false
                };
                if accepted {
                    resize_live += 1;
                }
                let effective = if accepted {
                    resize.target_stream_count
                } else {
                    resize_live as u32
                };
                transport
                    .send(frame(Frame::ResizeAck(DataPlaneResizeAck {
                        epoch: resize.epoch,
                        effective_stream_count: effective,
                        accepted,
                    })))
                    .await?;
            }
            Some(Frame::SourceDone(_)) => {
                if !manifest_complete {
                    return Err(violation("SourceDone before ManifestComplete".into()));
                }
                // Completion, both carriers: the shared `outstanding`
                // set must be empty (every granted need claimed exactly
                // once). In-stream claims inline above; the data-plane
                // NeedListSink claims as payloads land, so joining the
                // receive task first drains the last of them (and
                // surfaces any receive error / stall). Set membership —
                // not a file count — is the contract (codex F1: a count
                // proxy let a peer substitute or duplicate paths).
                // `finish()` drops the arm sender (no more resizes), joins
                // the accept loop, and reports the settled stream count.
                let (in_stream_carrier_used, data_plane_streams) = match data_plane_recv.take() {
                    Some(run) => {
                        let totals = run.finish().await?;
                        files_written = totals.outcome.files_written as u64;
                        bytes_written = totals.outcome.bytes_written;
                        (false, Some(totals.streams))
                    }
                    None => (true, None),
                };
                let unfulfilled = outstanding
                    .lock()
                    .expect("outstanding-needs lock poisoned")
                    .len();
                if unfulfilled != 0 {
                    return Err(violation(format!(
                        "SourceDone with {unfulfilled} needed file(s) never delivered"
                    )));
                }
                // otp-6b: run the mirror delete pass now — after every payload
                // is written, so the dest tree is final and no about-to-arrive
                // file is misjudged extraneous. All blocking FS work (enumerate
                // + delete) runs on the blocking pool.
                let entries_deleted: u64 = if mirror_enabled {
                    let dst = dst_root.to_path_buf();
                    let canonical = canonical_dst_root.clone();
                    let files = std::mem::take(&mut source_files);
                    let filter = mirror_filter.clone_without_cache();
                    let tolerate_nonempty = mirror_kind == MirrorMode::FilteredSubset;
                    tokio::task::spawn_blocking(move || {
                        mirror_delete_pass(
                            &dst,
                            &files,
                            &filter,
                            tolerate_nonempty,
                            canonical.as_deref(),
                        )
                    })
                    .await
                    .map_err(|e| {
                        eyre::Report::new(SessionFault::internal(format!(
                            "mirror delete task panicked: {e}"
                        )))
                    })?
                    .map_err(|e| {
                        eyre::Report::new(SessionFault::internal(format!(
                            "mirror delete failed: {e:#}"
                        )))
                    })?
                } else {
                    0
                };
                let summary = TransferSummary {
                    files_transferred: files_written,
                    bytes_transferred: bytes_written,
                    entries_deleted,
                    in_stream_carrier_used,
                    files_resumed: 0, // resume lands at otp-7
                };
                transport.send(frame(Frame::Summary(summary))).await?;
                return Ok(DestinationOutcome {
                    summary,
                    needed_paths,
                    data_plane_streams,
                });
            }
            Some(Frame::Error(err)) => {
                return Err(eyre::Report::new(SessionFault::from_wire(err)));
            }
            other => {
                // Everything else is off-lane or off-phase here:
                // destination-lane frames echoed back (a ResizeAck the
                // destination would never receive), resume frames in a
                // non-resume session (otp-7), stray handshake frames,
                // bare FileData/TarShardChunk outside a record. Fail
                // fast, no tolerant parsing.
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
    // Ever-granted DEDUP set (control-loop-local, insert-only): a path
    // the source manifests twice is granted at most once, and because it
    // is never removed, a concurrent data-plane claim can't re-open the
    // grant (fix-review F1).
    granted: &mut HashSet<String>,
    // Not-yet-delivered COMPLETION set (shared with the receive).
    outstanding: &data_plane::OutstandingNeeds,
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

    // Dedup on the ever-granted set (no lock — control-loop-local), then
    // insert the freshly granted paths into the shared `outstanding`
    // completion set BEFORE the NeedBatch goes out. The source can only
    // send a payload after receiving its need, so insert-before-send
    // orders the data-plane receive's `claim` strictly after this insert.
    let fresh: Vec<String> = needed
        .into_iter()
        .filter(|path| granted.insert(path.clone()))
        .collect();
    let entries: Vec<NeedEntry> = {
        let mut out = outstanding.lock().expect("outstanding-needs lock poisoned");
        fresh
            .into_iter()
            .map(|relative_path| {
                needed_paths.push(relative_path.clone());
                out.insert(relative_path.clone());
                NeedEntry {
                    relative_path,
                    resume: false, // resume lands at otp-7
                }
            })
            .collect()
    };
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

    /// otp-4b-3: a data-plane break during the drain prefers the peer's
    /// framed reason. When the receive half has forwarded a
    /// `SessionError{CANCELLED}` on the control lane, `prefer_peer_fault`
    /// returns THAT fault, not the raw data-plane transport error — the
    /// non-timeout half of the mid-transfer-cancel guard (the e2e in
    /// `blit-daemon` guards the still-pending-drain half).
    #[tokio::test]
    async fn prefer_peer_fault_prefers_a_framed_fault() {
        let (tx, mut rx) = mpsc::unbounded_channel::<SourceEvent>();
        // The peer framed CANCELLED on the control lane before we ask.
        tx.send(SourceEvent::Fault(SessionFault {
            code: session_error::Code::Cancelled,
            message: "transfer cancelled via CancelJob".into(),
            local_build_id: String::new(),
            peer_build_id: String::new(),
            peer_notified: true,
        }))
        .expect("send fault");

        let dp_err = eyre::Report::new(SessionFault::refusal(
            session_error::Code::DataPlaneFailed,
            "Broken pipe (os error 32)",
        ));
        let chosen = prefer_peer_fault(&mut rx, dp_err).await;
        let fault = chosen
            .downcast_ref::<SessionFault>()
            .expect("a SessionFault");
        assert_eq!(
            fault.code,
            session_error::Code::Cancelled,
            "the framed CANCELLED must win over the data-plane break"
        );
    }

    /// otp-4b-3 pass-2 F1: on the `queue()` error path (payload phase) a
    /// legitimate `Need` may be queued ahead of the peer's `CANCELLED`.
    /// `prefer_peer_fault` must SKIP it and still surface CANCELLED — not
    /// mistake the in-flight need for a protocol violation (the strict
    /// finish()-drain `recv_peer_fault` would).
    #[tokio::test]
    async fn prefer_peer_fault_skips_inflight_needs_to_reach_the_fault() {
        let (tx, mut rx) = mpsc::unbounded_channel::<SourceEvent>();
        // A still-in-flight need queued before the abort frame.
        tx.send(SourceEvent::Need(FileHeader {
            relative_path: "still-needed.bin".into(),
            ..Default::default()
        }))
        .expect("send need");
        tx.send(SourceEvent::Fault(SessionFault {
            code: session_error::Code::Cancelled,
            message: "transfer cancelled via CancelJob".into(),
            local_build_id: String::new(),
            peer_build_id: String::new(),
            peer_notified: true,
        }))
        .expect("send fault");

        let dp_err = eyre::Report::new(SessionFault::refusal(
            session_error::Code::DataPlaneFailed,
            "pipeline closed",
        ));
        let chosen = prefer_peer_fault(&mut rx, dp_err).await;
        let fault = chosen
            .downcast_ref::<SessionFault>()
            .expect("a SessionFault");
        assert_eq!(
            fault.code,
            session_error::Code::Cancelled,
            "an in-flight need must be skipped, not surfaced as a violation"
        );
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
