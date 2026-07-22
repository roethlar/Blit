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
pub mod local;
pub mod transport;

pub use local::{
    run_local_session, LocalCompareMode, LocalMirrorDeleteScope, LocalMirrorOptions,
    LocalMirrorSummary, TransferOutcome,
};

use std::collections::{HashMap, HashSet};
use std::fmt;
use std::future::Future;
use std::path::{Path, PathBuf};
use std::pin::Pin;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex as StdMutex};

use eyre::Result;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::sync::{mpsc, watch};

use crate::copy::DEFAULT_BLOCK_SIZE;
use crate::generated::transfer_frame::Frame;
use crate::generated::{
    session_error, BlockHashList, BlockTransfer, BlockTransferComplete, CapacityProfile,
    ComparisonMode, DataPlaneResize, DataPlaneResizeAck, DataPlaneResizeOp, FileData, FileHeader,
    FilterSpec, ManifestComplete, MirrorMode, NeedBatch, NeedComplete, NeedEntry, SessionAccept,
    SessionError, SessionHello, SessionOpen, SourceDone, TarShardComplete, TarShardHeader,
    TransferFrame, TransferRole, TransferSummary,
};
use crate::manifest::{header_transfer_status, CompareMode, CompareOptions, FileStatus};
use crate::remote::transfer::diff_planner;
use crate::remote::transfer::payload::{PreparedPayload, TransferPayload};
use crate::remote::transfer::session_phase::{
    BoundSessionPhaseTrace, SessionPhaseFields, SessionPhaseRole, SessionPhaseTrace,
};
use crate::remote::transfer::sink::{FsSinkConfig, FsTransferSink, TransferSink};
use crate::remote::transfer::small_file_probe::{
    BoundSmallFileProbe, SmallFileCarrier, SmallFileProbe,
};
#[cfg(test)]
use crate::remote::transfer::source::SourceScan;
use crate::remote::transfer::source::{FsTransferSource, TransferSource};
use crate::remote::transfer::stall_guard::TRANSFER_STALL_TIMEOUT;
use crate::remote::transfer::tar_safety::MAX_TAR_SHARD_BYTES;
use crate::remote::transfer::{
    AbortOnDrop, FaultedPath, MembershipOutcome, RemoteTransferProgress, CONTROL_PLANE_CHUNK_SIZE,
};
use crate::transfer_plan::PlanOptions;
use transport::{FrameRx, FrameTransport, FrameTx};

/// Belt-and-braces wire-shape version, bumped on any change to the
/// frame set or grammar. Exchanged (and exact-matched) in
/// `SessionHello` alongside the build id (D-2026-07-05-2).
/// v2: `SessionError.relative_path` (otp-7b-2, the D-2026-07-09-1 Q2
/// fault-summary rider).
/// v3: `SessionError.Code::CHECKSUM_DISABLED` + populated
/// `FileHeader.checksum` on a `COMPARISON_MODE_CHECKSUM` session
/// (otp-10b-1 — content compare on the session).
pub const CONTRACT_VERSION: u32 = 3;

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

/// otp-7a resume bounds (codex F1, D-2026-07-10-1; data-plane ceiling
/// otp-7b, D-2026-07-10-2). The in-stream carrier rides the gRPC
/// `Transfer` RPC when the daemon serves, and tonic's default 4 MiB
/// decode limit applies to every frame — so the DESTINATION's
/// block-size clamp (plan D5) must keep both resume frame shapes under
/// it. The TCP data plane carries blocks as binary records with no
/// protobuf envelope, so its ceiling is the wire record bound instead.
/// The ceiling is therefore PER CARRIER; both ends know the carrier
/// (grant present ⇒ data plane), so they agree without negotiation.
///
/// Floor: a `BlockHashList` costs 32 bytes per block, so absurdly small
/// blocks amplify — a block_size=1 list would be 32× the partial.
const MIN_RESUME_BLOCK_SIZE: usize = 64 * 1024;
/// Ceiling, in-stream carrier: one `BlockTransfer` frame carries one
/// whole block; 2 MiB of content plus the envelope stays well under the
/// 4 MiB frame limit.
const MAX_IN_STREAM_RESUME_BLOCK_SIZE: usize = 2 * 1024 * 1024;
/// Ceiling, in-stream carrier, for one `TarShardHeader` frame's encoded
/// member list (codex otp-8 F2). The planner bounds a shard's CONTENT
/// bytes and file count (≤ 4096), but not the encoded size of its
/// header list — 4096 legally long relative paths can push the single
/// protobuf frame past tonic's 4 MiB decode limit. The in-stream send
/// path splits an offending shard into consecutive smaller shard
/// records under this bound (same grammar, same planner decisions —
/// only the record boundaries move). Same 2 MiB posture as the resume
/// block ceiling: content plus envelope stays well under the frame
/// limit. The data plane is unaffected (binary records, 64 MiB cap).
const MAX_IN_STREAM_TAR_HEADER_BYTES: usize = 2 * 1024 * 1024;
/// Ceiling, TCP data plane (otp-7b): binary `BLOCK` records have no
/// protobuf envelope; the bound is the receive pipeline's per-record
/// allocation cap (= the old resume path's `MAX_BLOCK_SIZE`, 64 MiB).
/// The hash list still rides the control lane as protobuf, but its
/// size is governed by the 65_536-hash cap, not by block size.
const MAX_DATA_PLANE_RESUME_BLOCK_SIZE: usize =
    crate::remote::transfer::pipeline::MAX_WIRE_BLOCK_BYTES;
/// One `BlockHashList` frame carries a partial's whole list; capped at
/// 65_536 × 32 B = 2 MiB of hashes. A partial with more blocks than
/// this degrades to the empty list — the contract's full-transfer
/// fallback (plan D1) — never an oversized frame.
const MAX_RESUME_BLOCK_HASHES: u64 = 65_536;

/// Does a partial of `dst_len` bytes get a real hash list, or the empty
/// full-transfer fallback (cap rationale above)? Pure, so the cap is
/// unit-testable without a multi-GiB fixture.
fn resume_hash_list_fits(dst_len: u64, block_size: usize) -> bool {
    dst_len.div_ceil(block_size.max(1) as u64) <= MAX_RESUME_BLOCK_HASHES
}

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
    /// Caller-side observability hooks (otp-10a). All default-off; the
    /// daemon SOURCE responder runs with the defaults.
    pub instruments: SourceInstruments,
}

/// Observability hooks a SOURCE-side caller can attach to its session
/// (otp-10a — the push-shaped verb's progress line and `blit move`'s
/// unreadable-scan gate ride these). Everything is inactive by default
/// unless an explicit process-level probe flag enables it; the session's
/// behavior on the wire is identical either way.
#[cfg(test)]
pub(crate) struct DialTestSample {
    delta_bytes: u64,
    blocked_ratio: f64,
    reply: tokio::sync::oneshot::Sender<DialTestObservation>,
}

#[cfg(test)]
#[derive(Clone, Copy, Debug)]
pub(crate) struct DialTestObservation {
    proposal: Option<crate::dial::ResizeProposal>,
    live_streams: usize,
    settled_epoch: u32,
}

#[cfg(test)]
pub(crate) type DialTestSamples = Arc<StdMutex<Option<mpsc::UnboundedReceiver<DialTestSample>>>>;

#[cfg(test)]
pub(crate) struct DialTerminalTestGate {
    entered: AtomicBool,
    changed: tokio::sync::Notify,
    release: tokio::sync::Semaphore,
}

#[cfg(test)]
impl DialTerminalTestGate {
    fn new() -> Arc<Self> {
        Arc::new(Self {
            entered: AtomicBool::new(false),
            changed: tokio::sync::Notify::new(),
            release: tokio::sync::Semaphore::new(0),
        })
    }

    async fn hold(&self) {
        self.entered.store(true, Ordering::Release);
        self.changed.notify_waiters();
        self.release
            .acquire()
            .await
            .expect("terminal test gate remains open")
            .forget();
    }

    async fn wait_until_entered(&self) {
        loop {
            let changed = self.changed.notified();
            tokio::pin!(changed);
            changed.as_mut().enable();
            if self.entered.load(Ordering::Acquire) {
                return;
            }
            changed.await;
        }
    }

    fn release(&self) {
        self.release.add_permits(1);
    }
}

#[derive(Clone, Default)]
pub struct SourceInstruments {
    /// w6-1 progress events, reported exactly where the old push driver
    /// reported them: `ManifestBatch` per received need batch (the
    /// push-direction denominator — files the DESTINATION requested),
    /// `Payload`/`FileComplete` per payload sent on either carrier.
    pub progress: Option<RemoteTransferProgress>,
    /// Shared accumulator for source paths the manifest scan could not
    /// read. The session still streams what it can and reports
    /// `ManifestComplete{scan_complete=false}`; callers that must not
    /// treat a partial transfer as success (the push verb, `blit move`)
    /// inspect this after the session returns. `None` = the session
    /// keeps its own private accumulator (unchanged behavior).
    pub unreadable: Option<Arc<StdMutex<Vec<String>>>>,
    /// Emit `[data-plane-client]` connect traces on the data-plane
    /// sockets this SOURCE acquires (`--trace-data-plane`).
    pub trace_data_plane: bool,
    /// Low-frequency, structured session timing events used by pf-1.
    /// Disabled by default; production probes may also enable it with
    /// `BLIT_TRACE_SESSION_PHASES=1` on each endpoint.
    pub session_phase_trace: SessionPhaseTrace,
    /// High-volume aggregate observer for otp-12 small-file attribution.
    /// Separate from the low-frequency phase trace and disabled by default.
    pub small_file_probe: SmallFileProbe,
    /// Deterministic sample source used only by in-crate role guards. The
    /// production build has no such field and always samples live probes.
    #[cfg(test)]
    pub(crate) dial_test_samples: Option<DialTestSamples>,
    /// Pauses the SOURCE immediately after ManifestComplete is durably sent,
    /// allowing deterministic terminal resize races without production
    /// sleeps or scheduler assumptions.
    #[cfg(test)]
    pub(crate) dial_terminal_test_gate: Option<Arc<DialTerminalTestGate>>,
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
    /// Capacity this byte receiver advertises and enforces. `None` snapshots
    /// [`crate::dial::local_receiver_capacity`] once at session start. Tests
    /// and constrained callers may supply a lower honest receiver limit; the
    /// same profile drives the wire advertisement, epoch-0 floor, and resize
    /// admission in either connection layout.
    pub receiver_capacity: Option<CapacityProfile>,
    /// Caller-side observability hooks (otp-10b-2). All default-off; the
    /// daemon DESTINATION responder runs with the defaults. Symmetric
    /// with [`SourceSessionConfig::instruments`].
    pub instruments: DestinationInstruments,
    /// otp-11: the LOCAL byte-carrier. When set, this destination
    /// applies needed files in-process through the local sink instead
    /// of requesting them from the source — no payload byte rides any
    /// transport. Process-local config with no wire representation:
    /// only [`local::run_local_session`] (which holds BOTH roots in
    /// this process) can construct a [`local::LocalApply`], so no wire
    /// peer can ever select it. `None` (every remote caller and the
    /// daemon responder) keeps the wire carriers exactly as before.
    pub local_apply: Option<local::LocalApply>,
}

/// Observability hooks a DESTINATION-side caller can attach to its
/// session (otp-10b-2 — the pull-shaped verb's progress line rides
/// these; `byte_progress` predates them, otp-9a). Everything is inactive
/// by default unless an explicit process-level probe flag enables it; the
/// session's behavior on the wire is identical either way.
#[derive(Clone, Default)]
pub struct DestinationInstruments {
    /// w6-1 progress events from this end's receive side:
    /// `ManifestBatch` per NeedBatch emitted (the pull-direction
    /// denominator — files this DESTINATION requested, the same
    /// files-to-transfer semantic the push verb reports),
    /// `Payload`/`FileComplete` per record received on either carrier.
    pub progress: Option<RemoteTransferProgress>,
    /// Live byte counter for this DESTINATION's writes (otp-9a). The
    /// session sink reports applied payload bytes against it — the same
    /// `ByteProgressSink` contract the old drivers used, so a caller
    /// that owns a jobs row (the delegated dst daemon, otp-9) can watch
    /// bytes land while the session runs. `None` = no reporting.
    pub byte_progress: Option<crate::remote::transfer::ByteProgressSink>,
    /// Emit `[data-plane-client]` connect traces on the data-plane
    /// sockets this DESTINATION initiator dials (`--trace-data-plane`).
    /// A DESTINATION responder accepts rather than dials; the flag is
    /// inert there.
    pub trace_data_plane: bool,
    /// Low-frequency, structured session timing events used by pf-1.
    /// Kept separate from the per-file `trace_data_plane` output so the
    /// observer does not dominate a small-file timing run.
    pub session_phase_trace: SessionPhaseTrace,
    /// High-volume aggregate observer for otp-12 small-file attribution.
    /// Separate from the low-frequency phase trace and disabled by default.
    pub small_file_probe: SmallFileProbe,
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
    /// otp-7b-2 (D-2026-07-09-1 Q2 rider): the file this fault
    /// concerns, when one is known — a mid-record read/write failure
    /// names its file so the end-of-operation summary can, too.
    /// Carried on the wire (`SessionError.relative_path`), so BOTH
    /// ends can name it, wherever the fault originated. Structured
    /// identity, never scraped from the message.
    pub relative_path: Option<String>,
    /// codex otp-10a F5: the `io::ErrorKind` of the underlying I/O
    /// failure, when this fault stringified a report that carried one.
    /// `SessionFault` replaces the original error chain as the
    /// drivers' error payload, which would otherwise strip the signal
    /// the retry classifier (`remote::retry::is_retryable`) keys on —
    /// a mid-transfer socket reset must stay retryable under
    /// `--retry`. Local evidence only: faults received from the peer
    /// (`from_wire`) carry `None`.
    pub io_kind: Option<std::io::ErrorKind>,
}

impl SessionFault {
    fn new(code: session_error::Code, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
            local_build_id: String::new(),
            peer_build_id: String::new(),
            peer_notified: false,
            relative_path: None,
            io_kind: None,
        }
    }

    /// Capture the underlying `io::ErrorKind` from the report this
    /// fault is about to replace (codex otp-10a F5). Call at every
    /// site that stringifies an eyre chain into a fault.
    fn with_io_kind_from(mut self, report: &eyre::Report) -> Self {
        self.io_kind = report
            .chain()
            .find_map(|cause| cause.downcast_ref::<std::io::Error>())
            .map(|io_err| io_err.kind());
        self
    }

    /// Attach the file identity this fault concerns (otp-7b-2).
    fn with_path(mut self, relative_path: impl Into<String>) -> Self {
        self.relative_path = Some(relative_path.into());
        self
    }

    /// otp-7b-2, the D-2026-07-09-1 Q2 rider's mechanism: the
    /// END-OF-OPERATION summary block a reporting CLI appends after a
    /// faulted transfer — naming the affected file and suggesting a
    /// re-run to converge — or `None` when the fault names no file
    /// (nothing to converge on; the plain fault line suffices). The
    /// otp-10 verb switch prints this; until then the session-client
    /// tests pin its content.
    pub fn end_of_operation_summary(&self) -> Option<String> {
        let path = self.relative_path.as_deref()?;
        // "" is the single-file-root transfer's identity (the root IS
        // the file) — render it as such rather than a blank name
        // (codex 7b-2 G1).
        let shown = if path.is_empty() {
            "<the transfer root file>"
        } else {
            path
        };
        Some(format!(
            "transfer aborted: {}\naffected file: {shown} — partial data at the \
             destination is preserved; re-run the same command to converge \
             (resume transfers only what is still missing)",
            self.message
        ))
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
            // Explicit wire presence (codex 7b-2 G1): "" is the valid
            // identity of a single-file-root transfer, not absence.
            relative_path: err.relative_path,
            // Peer-reported fault: no local I/O evidence (codex
            // otp-10a F5 — io_kind is local-transport testimony only).
            io_kind: None,
        }
    }

    fn to_wire(&self) -> SessionError {
        SessionError {
            code: self.code as i32,
            message: self.message.clone(),
            local_build_id: self.local_build_id.clone(),
            peer_build_id: self.peer_build_id.clone(),
            relative_path: self.relative_path.clone(),
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
/// as INTERNAL — an end that aborts says why before closing. A
/// [`FaultedPath`] marker anywhere in the chain (otp-7b-2) becomes the
/// fault's structured file identity, so per-file read/write failures
/// keep naming their file across the eyre boundary.
fn fault_from_report(report: eyre::Report) -> SessionFault {
    match report.downcast::<SessionFault>() {
        Ok(fault) => fault,
        Err(other) => {
            // codex otp-10a F5: stringifying the chain would strip the
            // io::ErrorKind the retry classifier keys on — carry it.
            let fault = SessionFault::internal(format!("{other:#}")).with_io_kind_from(&other);
            match other.downcast_ref::<FaultedPath>() {
                Some(FaultedPath(path)) => fault.with_path(path.clone()),
                None => fault,
            }
        }
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
        relative_path: None,
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

/// otp-7a: whether this open negotiates the resume block phase. One
/// reading, both roles and both validators — the flag is in the open, so
/// resume runs identically whichever end initiated (plan D6).
fn resume_negotiated(open: &SessionOpen) -> bool {
    open.resume.as_ref().is_some_and(|r| r.enabled)
}

fn source_open_validator(open: &SessionOpen) -> std::result::Result<(), SessionFault> {
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
    Ok(())
}

/// Flips an abort flag when dropped, so a blocking-pool pass whose
/// awaiting future is dropped (client disconnect, CancelJob) stops at
/// its next flag check instead of running to completion behind a dead
/// session. Introduced for the mirror delete pass (codex otp-9b F2);
/// the destination diff's hash chunks share it (codex otp-10b-1 F3).
struct AbortFlagOnDrop(Arc<AtomicBool>);
impl Drop for AbortFlagOnDrop {
    fn drop(&mut self) {
        self.0.store(true, Ordering::Release);
    }
}

/// Operator policy a serving responder applies to every session it
/// accepts (otp-10a F3 / otp-10b-1). Defaults are the permissive
/// non-daemon posture; the daemon fills it from its runtime config.
#[derive(Clone, Copy, Default)]
pub struct ResponderPolicy {
    /// `--force-grpc-data`: never grant a TCP data plane — every
    /// served session rides the in-stream carrier regardless of what
    /// the initiator asked for.
    pub force_in_stream: bool,
    /// `--no-server-checksums`: refuse `COMPARISON_MODE_CHECKSUM`
    /// opens with `CHECKSUM_DISABLED` instead of hashing (or silently
    /// degrading the compare).
    pub refuse_checksum_compare: bool,
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

fn bind_session_phase_trace(
    trace: SessionPhaseTrace,
    negotiated: &Negotiated,
    endpoint_role: SessionPhaseRole,
) -> Option<BoundSessionPhaseTrace> {
    let session_token = negotiated
        .responder_data_plane
        .as_ref()
        .map(data_plane::ResponderDataPlane::session_token)
        .or_else(|| {
            negotiated
                .accept
                .data_plane
                .as_ref()
                .map(|grant| grant.session_token.as_slice())
        })?;
    let initiator_role = match TransferRole::try_from(negotiated.open.initiator_role).ok()? {
        TransferRole::Source => SessionPhaseRole::Source,
        TransferRole::Destination => SessionPhaseRole::Destination,
        TransferRole::Unspecified => return None,
    };
    trace
        .or_from_env()
        .bind(session_token, endpoint_role, initiator_role)
}

fn bind_small_file_probe(
    probe: SmallFileProbe,
    negotiated: &Negotiated,
    endpoint_role: SessionPhaseRole,
) -> Option<BoundSmallFileProbe> {
    let session_token = negotiated
        .responder_data_plane
        .as_ref()
        .map(data_plane::ResponderDataPlane::session_token)
        .or_else(|| {
            negotiated
                .accept
                .data_plane
                .as_ref()
                .map(|grant| grant.session_token.as_slice())
        });
    let initiator_role = match TransferRole::try_from(negotiated.open.initiator_role).ok()? {
        TransferRole::Source => SessionPhaseRole::Source,
        TransferRole::Destination => SessionPhaseRole::Destination,
        TransferRole::Unspecified => return None,
    };
    let carrier = if session_token.is_some() {
        SmallFileCarrier::Tcp
    } else {
        SmallFileCarrier::InStream
    };
    probe
        .or_from_env()
        .bind(session_token, endpoint_role, initiator_role, carrier)
}

async fn finish_small_file_probe(probe: Option<&BoundSmallFileProbe>) {
    let Some(probe) = probe.cloned() else {
        return;
    };
    let _ = tokio::task::spawn_blocking(move || probe.finish()).await;
}

async fn flush_session_phase_trace(trace: Option<&BoundSessionPhaseTrace>) {
    let Some(trace) = trace.cloned() else {
        return;
    };
    let _ = tokio::task::spawn_blocking(move || trace.flush()).await;
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
            relative_path: None,
            io_kind: None,
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
    policy: &ResponderPolicy,
    local_receiver_capacity: Option<&CapacityProfile>,
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
    // otp-10b-1: an operator who disabled server-side checksum hashing
    // refuses a content-compare session outright — the session never
    // silently degrades a `--checksum` request to a weaker compare.
    if policy.refuse_checksum_compare && open.compare_mode == ComparisonMode::Checksum as i32 {
        return Err(notify_and_wrap(
            transport,
            SessionFault::new(
                session_error::Code::ChecksumDisabled,
                "checksum comparison is disabled on this daemon \
                 (--no-server-checksums / server_checksums_enabled = false)",
            ),
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
    //
    //
    // otp-7b: resume sessions ride the data plane too — block records
    // travel as binary BLOCK/BLOCK_COMPLETE records on the sockets (the
    // otp-7a in-stream frames remain the fallback carrier), so the grant
    // is no longer suppressed for a resume session.
    let receiver_capacity = if local_role == TransferRole::Destination {
        local_receiver_capacity
    } else {
        open.receiver_capacity.as_ref()
    };
    let responder_data_plane = if open.in_stream_bytes || policy.force_in_stream {
        None
    } else {
        data_plane::prepare_responder_data_plane(receiver_capacity).await
    };
    let accept = SessionAccept {
        // The byte RECEIVER advertises capacity at session
        // open (D-2026-06-20-1/-2); consumed by the dial when
        // the data plane lands (otp-4b).
        receiver_capacity: local_receiver_capacity.cloned(),
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
    local_receiver_capacity: Option<&CapacityProfile>,
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
            // Direct-responder establish (the in-process role suite):
            // no daemon config in scope — permissive policy.
            responder_finish(
                transport,
                open,
                local_role,
                validate_open,
                resolve_open,
                &ResponderPolicy::default(),
                local_receiver_capacity,
            )
            .await
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
    /// A resume-flagged need (otp-7a). The send half HOLDS it until the
    /// destination's `BlockHashList` for the same path arrives — the
    /// contract's RELIABLE ordering guarantee: no byte of a resume file
    /// moves before its hash list.
    ResumeNeed(FileHeader),
    /// The destination's block hashes for a held resume need (otp-7a).
    BlockHashes(BlockHashList),
    NeedComplete,
    /// The destination's acknowledgement of one ADD or REMOVE epoch. The
    /// send half applies the accepted membership change through the common
    /// SOURCE data plane and settles the live dial from the actual count.
    ResizeAck(DataPlaneResizeAck),
    Summary(TransferSummary),
    Fault(SessionFault),
}

/// The receive half's event sender, mirroring every `Fault` onto a
/// `watch` signal as it is queued. The in-stream send path races this
/// signal against its (potentially blocked) record sends — codex otp-8
/// F1: a peer fault (CANCELLED above all) must interrupt a send half
/// stuck inside `reader.read()`/`tx.send()`, exactly as the data-plane
/// drain's `recv_peer_fault` arm does for socket sends. The mpsc queue
/// still carries the fault for the between-send paths; the watch is a
/// non-consuming side channel, so mid-send `Need`s stay queued.
struct SourceEventSender {
    tx: mpsc::UnboundedSender<SourceEvent>,
    fault_signal: watch::Sender<Option<SessionFault>>,
}

impl SourceEventSender {
    fn send(&self, event: SourceEvent) -> Result<(), mpsc::error::SendError<SourceEvent>> {
        if let SourceEvent::Fault(fault) = &event {
            let _ = self.fault_signal.send(Some(fault.clone()));
        }
        self.tx.send(event)
    }
}

/// Resolves to the peer/receive-half fault the moment one is signalled;
/// never resolves otherwise (the racing send future decides the
/// outcome, mirroring `recv_peer_fault`'s closed-channel posture).
async fn peer_fault_signalled(signal: &mut watch::Receiver<Option<SessionFault>>) -> SessionFault {
    loop {
        if let Some(fault) = signal.borrow_and_update().clone() {
            return fault;
        }
        if signal.changed().await.is_err() {
            // Sender dropped without ever signalling a fault: stay
            // pending so the send future's own result decides.
            std::future::pending::<()>().await;
        }
    }
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
        None,
    )
    .await?;

    drive_source(
        cfg.plan_options,
        cfg.data_plane_host,
        cfg.instruments,
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
    instruments: SourceInstruments,
    mut negotiated: Negotiated,
    transport: FrameTransport,
    source: Arc<dyn TransferSource>,
) -> Result<TransferSummary> {
    let phase_trace = bind_session_phase_trace(
        instruments.session_phase_trace.clone(),
        &negotiated,
        SessionPhaseRole::Source,
    );
    let small_file_probe = bind_small_file_probe(
        instruments.small_file_probe.clone(),
        &negotiated,
        SessionPhaseRole::Source,
    );
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
    // Fault side-channel (codex otp-8 F1): the in-stream send path
    // races this signal against blocked record sends; see
    // `SourceEventSender`.
    let (fault_tx, fault_rx) = watch::channel(None::<SessionFault>);
    // AbortOnDrop: an early error return below must abort the receive
    // half instead of leaking it (same rationale as design-2 / w4-1).
    let recv_guard = AbortOnDrop::new(tokio::spawn(source_recv_half(
        rx,
        Arc::clone(&sent),
        Arc::clone(&manifest_sent),
        resume_negotiated(&negotiated.open),
        // otp-10a: the recv half owns need-batch arrival, which is the
        // push-direction progress denominator (contract on
        // `ProgressEvent::ManifestBatch`: "push: need-list batches").
        instruments.progress.clone(),
        phase_trace.clone(),
        small_file_probe.clone(),
        SourceEventSender {
            tx: event_tx,
            fault_signal: fault_tx,
        },
    )));

    let send_result = source_send_half(
        plan_options,
        data_plane_host.as_deref(),
        instruments,
        &negotiated,
        responder_data_plane,
        &mut tx,
        source,
        sent,
        &manifest_sent,
        event_rx,
        fault_rx,
        phase_trace.clone(),
        small_file_probe,
    )
    .await;
    let mut result = match send_result {
        Ok(summary) => Ok(summary),
        Err(report) => {
            let mut fault = fault_from_report(report);
            if !fault.peer_notified {
                let _ = tx.send(error_frame(&fault)).await;
                fault.peer_notified = true;
            }
            Err(eyre::Report::new(fault))
        }
    };
    let recv_cleanup = recv_guard.abort_and_join().await;
    if result.is_ok() {
        if let Err(err) = recv_cleanup {
            if !err.is_cancelled() {
                result = Err(eyre::Report::new(SessionFault::internal(format!(
                    "source receive task panicked: {err}"
                ))));
            }
        }
    }
    flush_session_phase_trace(phase_trace.as_ref()).await;
    result
}

/// Receive half of the source driver: drains the transport for the
/// whole session so destination sends can never deadlock against a
/// blocked source send, and routes the destination lane to the send
/// half. Terminates on summary, error, close, or violation.
async fn source_recv_half(
    mut rx: Box<dyn FrameRx>,
    sent: Arc<StdMutex<HashMap<String, FileHeader>>>,
    manifest_sent: Arc<AtomicBool>,
    resume_session: bool,
    progress: Option<RemoteTransferProgress>,
    phase_trace: Option<BoundSessionPhaseTrace>,
    small_file_probe: Option<BoundSmallFileProbe>,
    events: SourceEventSender,
) {
    let mut need_batch_seq = 0u64;
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
                if let Some(trace) = &phase_trace {
                    trace.event(
                        "need_batch_received",
                        SessionPhaseFields {
                            batch: Some(need_batch_seq),
                            count: Some(batch.entries.len() as u64),
                            ..Default::default()
                        },
                    );
                    need_batch_seq += 1;
                }
                // otp-10a: the need list is the push-direction progress
                // denominator ("N of M files"). Entries are unique by
                // contract (a duplicate need faults below), so every
                // batch is newly-requested work — same semantics as the
                // old push driver's `report_manifest_batch`.
                if let Some(p) = &progress {
                    if !batch.entries.is_empty() {
                        p.report_manifest_batch(batch.entries.len());
                    }
                }
                for entry in batch.entries {
                    if entry.resume && !resume_session {
                        let _ = events.send(SourceEvent::Fault(SessionFault::protocol_violation(
                            format!(
                                "resume-flagged need for '{}' in a session opened without resume",
                                entry.relative_path
                            ),
                        )));
                        return;
                    }
                    let header = if let Some(probe) = &small_file_probe {
                        let wait_started = probe.start();
                        let mut sent = sent.lock().expect("sent-manifest lock poisoned");
                        let wait = wait_started.elapsed();
                        let map_op_started = probe.start();
                        let header = sent.remove(&entry.relative_path);
                        let map_op = map_op_started.elapsed();
                        drop(sent);
                        probe.note_need_resolve(wait, map_op, header.is_some());
                        header
                    } else {
                        sent.lock()
                            .expect("sent-manifest lock poisoned")
                            .remove(&entry.relative_path)
                    };
                    match header {
                        Some(h) if entry.resume => {
                            if let Some(probe) = &small_file_probe {
                                probe.note_need_event_enqueue(&h.relative_path);
                                let started = probe.start();
                                let _ = events.send(SourceEvent::ResumeNeed(h));
                                probe.note_need_event_send(started.elapsed());
                            } else {
                                let _ = events.send(SourceEvent::ResumeNeed(h));
                            }
                        }
                        Some(h) => {
                            if let Some(probe) = &small_file_probe {
                                probe.note_need_event_enqueue(&h.relative_path);
                                let started = probe.start();
                                let _ = events.send(SourceEvent::Need(h));
                                probe.note_need_event_send(started.elapsed());
                            } else {
                                let _ = events.send(SourceEvent::Need(h));
                            }
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
            Some(Frame::BlockHashes(list)) => {
                // otp-7a: the destination's hashes for a resume-flagged
                // need. The send half correlates it with the held need;
                // in a non-resume session the frame is off-contract.
                if !resume_session {
                    let _ = events.send(SourceEvent::Fault(SessionFault::protocol_violation(
                        format!(
                            "BlockHashList for '{}' in a session opened without resume",
                            list.relative_path
                        ),
                    )));
                    return;
                }
                let _ = events.send(SourceEvent::BlockHashes(list));
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
                if let Some(trace) = &phase_trace {
                    trace.event("need_complete_received", SessionPhaseFields::default());
                }
                let _ = events.send(SourceEvent::NeedComplete);
            }
            Some(Frame::ResizeAck(ack)) => {
                // Forward the destination's resize response to the SOURCE
                // send half, which owns dial and membership settlement.
                if let Some(trace) = &phase_trace {
                    trace.event(
                        "resize_ack_received",
                        SessionPhaseFields {
                            epoch: Some(ack.epoch),
                            live_streams: Some(ack.effective_stream_count),
                            accepted: Some(ack.accepted),
                            ..Default::default()
                        },
                    );
                }
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

/// otp-7a: the send half's resume bookkeeping. A resume-flagged need is
/// HELD until its `BlockHashList` arrives (the contract's strict
/// ordering — the source must not send a byte of that file first); the
/// correlated pair then queues for the block phase.
#[derive(Default)]
struct ResumeSendState {
    held: HashMap<String, FileHeader>,
    ready: Vec<(FileHeader, BlockHashList)>,
}

struct SourceResizeState {
    proposals: Option<mpsc::UnboundedReceiver<crate::dial::ResizeProposal>>,
    pending: Option<data_plane::PendingResize>,
    last_ack: Option<DataPlaneResizeAck>,
}

impl SourceResizeState {
    fn take_pending_for_ack(
        &mut self,
        ack: &DataPlaneResizeAck,
    ) -> Result<Option<data_plane::PendingResize>> {
        if self.last_ack.as_ref().is_some_and(|settled| settled == ack) {
            return Ok(None);
        }
        match self.pending.take() {
            Some(pending) if pending.epoch == ack.epoch => Ok(Some(pending)),
            Some(pending) => {
                let expected = pending.epoch;
                self.pending = Some(pending);
                Err(eyre::Report::new(SessionFault::protocol_violation(
                    format!(
                        "DataPlaneResizeAck epoch {} while epoch {expected} is pending",
                        ack.epoch
                    ),
                )))
            }
            None => Err(eyre::Report::new(SessionFault::protocol_violation(
                format!("unsolicited DataPlaneResizeAck epoch {}", ack.epoch),
            ))),
        }
    }
}

fn validate_source_resize_ack(
    pending: &data_plane::PendingResize,
    current_streams: usize,
    ack: &DataPlaneResizeAck,
) -> Result<()> {
    let effective = ack.effective_stream_count as usize;
    let expected = if ack.accepted {
        pending.target_streams as usize
    } else {
        current_streams
    };
    if effective != expected {
        let disposition = if ack.accepted { "accepted" } else { "refused" };
        return Err(eyre::Report::new(SessionFault::protocol_violation(
            format!(
                "{disposition} resize epoch {} reported effective {effective}, expected {expected}",
                pending.epoch
            ),
        )));
    }
    Ok(())
}

#[allow(clippy::too_many_arguments)]
async fn source_send_half(
    plan_options: PlanOptions,
    data_plane_host: Option<&str>,
    instruments: SourceInstruments,
    negotiated: &Negotiated,
    responder_data_plane: Option<data_plane::ResponderDataPlane>,
    tx: &mut Box<dyn FrameTx>,
    source: Arc<dyn TransferSource>,
    sent: Arc<StdMutex<HashMap<String, FileHeader>>>,
    manifest_sent: &AtomicBool,
    mut events: mpsc::UnboundedReceiver<SourceEvent>,
    mut fault_signal: watch::Receiver<Option<SessionFault>>,
    phase_trace: Option<BoundSessionPhaseTrace>,
    small_file_probe: Option<BoundSmallFileProbe>,
) -> Result<TransferSummary> {
    let mut pending: Vec<FileHeader> = Vec::new();
    let mut resume: ResumeSendState = ResumeSendState::default();
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
        // SOURCE responder: accept + send. The DESTINATION initiator
        // advertised its capacity in the open (the byte RECEIVER advertises,
        // wherever it initiates); epoch 0 uses the same receiver-bounded
        // floor as the dial layout and later epochs use the same controller.
        Some(bound) => Some(
            data_plane::accept_source_data_plane(
                bound,
                negotiated.open.receiver_capacity.as_ref(),
                Arc::clone(&source),
                &instruments,
                phase_trace.clone(),
                small_file_probe.clone(),
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
                        &instruments,
                        phase_trace.clone(),
                        small_file_probe.clone(),
                    )
                    .await?,
                )
            }
            None => None,
        },
    };

    let proposals = match data_plane.as_mut() {
        Some(dp) => Some(dp.take_resize_proposals()?),
        None => None,
    };
    let mut resize = SourceResizeState {
        proposals,
        pending: None,
        last_ack: None,
    };

    let result: Result<TransferSummary> = async {
        // Streaming manifest: entries go out as enumeration produces them
        // (immediate start in every direction — plan §Design 2). The open
        // carries no source path (the source end owns its local endpoint) but
        // does carry the include/exclude/size/age filter (otp-6a): only
        // matching files are manifested and transferred. The filter MUST ride
        // the wire (not be pre-wrapped by a local caller) because for pull the
        // SOURCE is the remote daemon responder — it, not the client, owns the
        // scan. Apply it through the universal `FilteredSource` decorator, the
        // single filter chokepoint every source impl routes through, rather
        // than the per-impl `scan(filter)` arg — a source impl is free to
        // ignore that arg (the since-deleted relay source did; codex otp-6a
        // F1), and the chokepoint makes filtering independent of it. A
        // default/absent filter scans everything (unchanged from otp-3). Globs
        // were validated at OPEN (`source_open_validator`), so the conversion
        // cannot fail on a validated open; map any error to a fault regardless.
        let scan_source: Arc<dyn TransferSource> = match negotiated.open.filter.as_ref() {
            Some(spec) if *spec != FilterSpec::default() => {
                let filter =
                    crate::remote::transfer::operation_spec::filter_from_spec(spec.clone())
                        .map_err(|e| {
                            eyre::Report::new(SessionFault::internal(format!(
                                "invalid filter: {e:#}"
                            )))
                        })?;
                Arc::new(crate::remote::transfer::source::FilteredSource::new(
                    Arc::clone(&source),
                    filter,
                ))
            }
            _ => Arc::clone(&source),
        };
        // otp-10b-1: a Checksum session fills each manifest header's
        // checksum so the DESTINATION can skip content-equal files
        // regardless of mtime. Wrapped OUTSIDE the filter so only
        // in-scope files pay the hash; a serving end that refuses to hash
        // never gets here (CHECKSUM_DISABLED at OPEN).
        let scan_source: Arc<dyn TransferSource> =
            if negotiated.open.compare_mode == ComparisonMode::Checksum as i32 {
                Arc::new(crate::remote::transfer::source::ChecksummingSource::new(
                    scan_source,
                ))
            } else {
                scan_source
            };
        // otp-10a: callers that must not treat a partial transfer as success
        // (the push verb, `blit move`'s source-delete gate) supply their own
        // accumulator via `SourceInstruments` and inspect it after the
        // session returns; the wire behavior is identical either way.
        let unreadable: Arc<StdMutex<Vec<String>>> =
            instruments.unreadable.clone().unwrap_or_default();
        let (mut header_rx, mut scan) = scan_source.scan(None, Arc::clone(&unreadable));
        // The TCP carrier owns a separate authenticated lane, so an ordinary
        // copy can restore the old push driver's scan/transfer overlap as soon
        // as DESTINATION has authorized a batch through NeedBatch. Mirror and
        // complete-scan operations intentionally retain their pre-write gate:
        // their refusal/deletion semantics depend on ManifestComplete.
        let early_tcp_payloads = data_plane.is_some()
            && !negotiated.open.mirror_enabled
            && !negotiated.open.require_complete_scan;
        let mut planner_batch_seq = 0u64;
        let scan_result: Result<u64> = async {
            loop {
                // NeedBatch can arrive while enumeration is blocked between
                // entries. Service the SOURCE controller independently of
                // scan progress; otherwise the data plane remains idle until
                // ManifestComplete and small-file TCP loses all overlap.
                drain_ready_source_events(
                    &mut events,
                    &mut pending,
                    &mut resume,
                    &mut need_complete,
                    data_plane.as_ref(),
                    tx,
                    &mut resize,
                    small_file_probe.as_ref(),
                )
                .await?;
                if early_tcp_payloads && !pending.is_empty() {
                    queue_tcp_payload_batch(
                        std::mem::take(&mut pending),
                        &source,
                        plan_options,
                        data_plane
                            .as_ref()
                            .expect("early TCP payloads require a data plane"),
                        &mut events,
                        &mut pending,
                        &mut resume,
                        &mut need_complete,
                        tx,
                        &mut resize,
                        phase_trace.as_ref(),
                        small_file_probe.as_ref(),
                        &mut planner_batch_seq,
                    )
                    .await?;
                    continue;
                }

                let next_header = tokio::select! {
                    biased;
                    input = wait_for_source_input(
                        &mut events,
                        &mut pending,
                        &mut resume,
                        &mut need_complete,
                        data_plane.as_ref(),
                        tx,
                        &mut resize,
                        small_file_probe.as_ref(),
                    ) => {
                        input?;
                        continue;
                    }
                    header = header_rx.recv() => header,
                };
                let Some(header) = next_header else { break };
                if let Some(probe) = &small_file_probe {
                    let wait_started = probe.start();
                    let mut sent = sent.lock().expect("sent-manifest lock poisoned");
                    let wait = wait_started.elapsed();
                    let map_op_started = probe.start();
                    sent.insert(header.relative_path.clone(), header.clone());
                    let map_op = map_op_started.elapsed();
                    drop(sent);
                    probe.note_manifest_insert(wait, map_op);
                } else {
                    sent.lock()
                        .expect("sent-manifest lock poisoned")
                        .insert(header.relative_path.clone(), header.clone());
                }
                tx.send(frame(Frame::ManifestEntry(header))).await?;
            }
            scan.finish().await
        }
        .await;
        let scanned = match scan_result {
            Ok(scanned) => scanned,
            Err(error) => {
                // A filesystem scan runs in `spawn_blocking`, so aborting its
                // JoinHandle cannot stop it once it has started. Close the
                // consumer first: that releases a producer blocked in
                // `blocking_send` before we reap the complete scan chain.
                header_rx.close();
                scan.abort_and_join().await;
                return Err(error);
            }
        };
        let scan_complete = unreadable
            .lock()
            .expect("unreadable list lock poisoned")
            .is_empty();
        log::debug!(
            "session source manifest complete: {scanned} entries, complete={scan_complete}"
        );
        if let Some(trace) = &phase_trace {
            trace.event(
                "manifest_complete_send_begin",
                SessionPhaseFields::default(),
            );
        }
        tx.send(frame(Frame::ManifestComplete(ManifestComplete {
            scan_complete,
        })))
        .await?;
        if let Some(trace) = &phase_trace {
            trace.event(
                "manifest_complete_sent",
                SessionPhaseFields {
                    count: Some(scanned as u64),
                    ..Default::default()
                },
            );
        }
        manifest_sent.store(true, Ordering::Release);
        #[cfg(test)]
        if let Some(gate) = &instruments.dial_terminal_test_gate {
            gate.hold().await;
        }

        // Payload phase. The byte carrier is either the TCP data plane
        // (dialed above) or the in-stream record grammar (fallback). Any TCP
        // needs not already queued during the scan, plus every in-stream need,
        // are handled here after ManifestComplete.
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
                &mut resume,
                &mut need_complete,
                data_plane.as_ref(),
                tx,
                &mut resize,
                small_file_probe.as_ref(),
            )
            .await?;
            if !pending.is_empty() {
                let batch = std::mem::take(&mut pending);
                match &mut data_plane {
                    Some(dp) => {
                        queue_tcp_payload_batch(
                            batch,
                            &source,
                            plan_options,
                            dp,
                            &mut events,
                            &mut pending,
                            &mut resume,
                            &mut need_complete,
                            tx,
                            &mut resize,
                            phase_trace.as_ref(),
                            small_file_probe.as_ref(),
                            &mut planner_batch_seq,
                        )
                        .await?;
                        // A cancel while earlier batches are actively moving
                        // closes the send pipeline under backpressure, so this
                        // queue fails with a data-plane error — prefer the
                        // peer's framed reason (CANCELLED) the same way the
                        // finish() drain does (otp-4b-3 codex F1). Not raced
                        // against events like finish(): live `Need`s still
                        // arrive here, and `recv_peer_fault` would consume them.
                    }
                    None => {
                        // codex otp-8 F1: race the record sends against the
                        // receive half's fault signal — the in-stream twin of
                        // the data-plane drain's `recv_peer_fault` arm. A peer
                        // cancel (framed CANCELLED, then RPC teardown) must
                        // interrupt a send blocked in `reader.read()` or in
                        // flow-controlled `tx.send()` and surface the framed
                        // reason, not hang or decay to INTERNAL. Biased:
                        // when both are ready, the framed fault wins.
                        tokio::select! {
                            biased;
                            fault = peer_fault_signalled(&mut fault_signal) => {
                                return Err(eyre::Report::new(fault));
                            }
                            res = send_payload_records(
                                tx,
                                &source,
                                plan_options,
                                batch,
                                &mut read_buf,
                                instruments.progress.as_ref(),
                                small_file_probe.as_ref(),
                            ) => {
                                res?;
                            }
                        }
                    }
                }
                continue;
            }
            if !resume.ready.is_empty() {
                // The block phase for correlated (need, hash-list) pairs.
                // Data plane (otp-7b): each pair becomes ONE composite
                // ResumeFile work item, so one pipeline worker runs the
                // whole record on one socket — strict per-file serialization
                // without cross-socket reorder hazards. In-stream (otp-7a):
                // control-lane BlockTransfer/Complete frames, as before.
                let ready = std::mem::take(&mut resume.ready);
                match &mut data_plane {
                    Some(dp) => {
                        let payloads = ready
                            .into_iter()
                            .map(|(header, hashes)| TransferPayload::ResumeFile {
                                header,
                                block_size: hashes.block_size,
                                dest_hashes: hashes.hashes,
                            })
                            .collect();
                        queue_payloads_while_servicing_events(
                            payloads,
                            &mut events,
                            &mut pending,
                            &mut resume,
                            &mut need_complete,
                            dp,
                            tx,
                            &mut resize,
                            small_file_probe.as_ref(),
                        )
                        .await?;
                        // Same cancel posture as the plain-batch queue above:
                        // prefer the peer's framed reason over the transport
                        // break a cancel also causes (otp-4b-3 codex F1).
                    }
                    None => {
                        for (header, hashes) in ready {
                            // codex 7b-2 G2: the whole in-stream record names
                            // its file on failure, matching the data-plane
                            // carrier's outer wrap. Same fault race as the
                            // plain-batch send above (codex otp-8 F1).
                            tokio::select! {
                                biased;
                                fault = peer_fault_signalled(&mut fault_signal) => {
                                    return Err(eyre::Report::new(fault));
                                }
                                res = send_resume_block_records(
                                    tx,
                                    &source,
                                    &header,
                                    &hashes,
                                    instruments.progress.as_ref(),
                                ) => {
                                    res.map_err(|e| tag_path(e, &header.relative_path))?;
                                }
                            }
                        }
                    }
                }
                continue;
            }
            if need_complete {
                break;
            }
            wait_for_source_input(
                &mut events,
                &mut pending,
                &mut resume,
                &mut need_complete,
                data_plane.as_ref(),
                tx,
                &mut resize,
                small_file_probe.as_ref(),
            )
            .await?;
        }

        // Demand is complete. Stop sampling and decline a proposal that was
        // claimed locally but never sent; then seal payload admission. A request
        // already on the wire still settles through ldt-1's terminal membership
        // outcomes, but no tail convergence opens idle sockets.
        if let Some(dp) = &mut data_plane {
            dp.stop_tuner().await?;
            stop_resize_intake(dp, &mut resize);
            dp.close_payloads()?;
            settle_inflight_resize(
                &mut events,
                &mut pending,
                &mut resume,
                &mut need_complete,
                dp,
                tx,
                &mut resize,
                small_file_probe.as_ref(),
            )
            .await?;
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
        //     it) → the `recv_peer_fault` arm wins; the data-plane owner is
        //     retained and the error epilogue cooperatively aborts and joins
        //     every in-flight worker before returning.
        //   * the socket break makes `finish()` return `Err` first → prefer
        //     the framed reason if the control lane delivers one within the
        //     stall window (`prefer_peer_fault`).
        if let Some(dp) = data_plane.as_mut() {
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
        data_plane = None;

        tx.send(frame(Frame::SourceDone(SourceDone {}))).await?;

        // CLOSING: the destination is the scorer; the next event must be
        // its summary (the receive half ends after forwarding it).
        match events.recv().await {
            Some(SourceEvent::Summary(summary)) => {
                if let Some(trace) = &phase_trace {
                    trace.event("summary_received", SessionPhaseFields::default());
                }
                finish_small_file_probe(small_file_probe.as_ref()).await;
                Ok(summary)
            }
            Some(SourceEvent::Fault(fault)) => Err(eyre::Report::new(fault)),
            Some(SourceEvent::Need(h) | SourceEvent::ResumeNeed(h)) => {
                Err(eyre::Report::new(SessionFault::protocol_violation(
                    format!("need for '{}' after NeedComplete", h.relative_path),
                )))
            }
            Some(SourceEvent::BlockHashes(l)) => {
                Err(eyre::Report::new(SessionFault::protocol_violation(
                    format!("BlockHashList for '{}' after SourceDone", l.relative_path),
                )))
            }
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
    .await;

    if result.is_err() {
        if let Some(dp) = data_plane.as_mut() {
            dp.abort_and_join().await;
        }
    }
    result
}

/// Drain ready peer events and live-dial proposals without blocking. Peer
/// events go first so an ACK always clears its epoch before the next sample
/// can be forwarded.
#[allow(clippy::too_many_arguments)]
async fn drain_ready_source_events(
    events: &mut mpsc::UnboundedReceiver<SourceEvent>,
    pending: &mut Vec<FileHeader>,
    resume: &mut ResumeSendState,
    need_complete: &mut bool,
    data_plane: Option<&data_plane::SourceDataPlane>,
    tx: &mut Box<dyn FrameTx>,
    resize: &mut SourceResizeState,
    small_file_probe: Option<&BoundSmallFileProbe>,
) -> Result<()> {
    loop {
        // Once the ordered terminal need marker is known and no payload work
        // remains to queue, do not consume a newly claimed tuner proposal.
        // Shutdown closes/drains the proposal channel and settles it
        // unchanged. Sending it here would cross a resize after demand was
        // already terminal merely because both events became ready together.
        if *need_complete && pending.is_empty() && resume.ready.is_empty() {
            break;
        }
        if let Ok(event) = events.try_recv() {
            process_source_event(
                event,
                pending,
                resume,
                need_complete,
                data_plane,
                resize,
                small_file_probe,
            )
            .await?;
            continue;
        }
        let proposal = match resize.proposals.as_mut() {
            Some(proposals) => match proposals.try_recv() {
                Ok(proposal) => Some(proposal),
                Err(mpsc::error::TryRecvError::Empty) => None,
                Err(mpsc::error::TryRecvError::Disconnected) => {
                    resize.proposals = None;
                    None
                }
            },
            None => None,
        };
        let Some(proposal) = proposal else { break };
        let dp = data_plane.ok_or_else(|| {
            eyre::Report::new(SessionFault::internal(
                "live dial proposed a resize without a TCP data plane",
            ))
        })?;
        send_resize_proposal(dp, tx, proposal, resize).await?;
    }
    Ok(())
}

#[allow(clippy::too_many_arguments)]
async fn wait_for_source_input(
    events: &mut mpsc::UnboundedReceiver<SourceEvent>,
    pending: &mut Vec<FileHeader>,
    resume: &mut ResumeSendState,
    need_complete: &mut bool,
    data_plane: Option<&data_plane::SourceDataPlane>,
    tx: &mut Box<dyn FrameTx>,
    resize: &mut SourceResizeState,
    small_file_probe: Option<&BoundSmallFileProbe>,
) -> Result<()> {
    loop {
        let proposal = async {
            match resize.proposals.as_mut() {
                Some(proposals) => proposals.recv().await,
                None => std::future::pending().await,
            }
        };
        tokio::select! {
            biased;
            event = events.recv() => {
                let event = event.ok_or_else(|| {
                    eyre::Report::new(SessionFault::internal(
                        "source receive half ended before NeedComplete",
                    ))
                })?;
                return process_source_event(
                    event,
                    pending,
                    resume,
                    need_complete,
                    data_plane,
                    resize,
                    small_file_probe,
                ).await;
            }
            proposal = proposal => {
                match proposal {
                    Some(proposal) => {
                        let dp = data_plane.ok_or_else(|| {
                            eyre::Report::new(SessionFault::internal(
                                "live dial proposed a resize without a TCP data plane",
                            ))
                        })?;
                        return send_resize_proposal(dp, tx, proposal, resize).await;
                    }
                    None => resize.proposals = None,
                }
            }
        }
    }
}

/// Handle one SOURCE control event. Need shape affects planning only; worker
/// count changes exclusively through the live dial proposal stream.
async fn process_source_event(
    event: SourceEvent,
    pending: &mut Vec<FileHeader>,
    resume: &mut ResumeSendState,
    need_complete: &mut bool,
    data_plane: Option<&data_plane::SourceDataPlane>,
    resize: &mut SourceResizeState,
    small_file_probe: Option<&BoundSmallFileProbe>,
) -> Result<()> {
    match event {
        SourceEvent::Need(header) => {
            if let Some(probe) = small_file_probe {
                let handler_started = probe.start();
                probe.note_need_event_hop(&header.relative_path, handler_started);
            }
            let process_started = small_file_probe.map(BoundSmallFileProbe::start);
            if *need_complete {
                return Err(eyre::Report::new(SessionFault::protocol_violation(
                    format!("need for '{}' after NeedComplete", header.relative_path),
                )));
            }
            pending.push(header);
            if let (Some(probe), Some(started)) = (small_file_probe, process_started) {
                probe.note_need_handler_work(started.elapsed());
            }
            Ok(())
        }
        SourceEvent::ResumeNeed(header) => {
            if let Some(probe) = small_file_probe {
                let handler_started = probe.start();
                probe.note_need_event_hop(&header.relative_path, handler_started);
            }
            let process_started = small_file_probe.map(BoundSmallFileProbe::start);
            if *need_complete {
                return Err(eyre::Report::new(SessionFault::protocol_violation(
                    format!(
                        "resume need for '{}' after NeedComplete",
                        header.relative_path
                    ),
                )));
            }
            // HELD until its BlockHashList arrives; no duplicate is
            // possible (the receive half's sent-map removal already
            // faults a second need for the same path).
            resume.held.insert(header.relative_path.clone(), header);
            if let (Some(probe), Some(started)) = (small_file_probe, process_started) {
                probe.note_need_handler_work(started.elapsed());
            }
            Ok(())
        }
        SourceEvent::BlockHashes(list) => {
            // Validate the wire block size at ARRIVAL (codex F5), not
            // when the record is eventually sent — pending plain files
            // go out first, and an already-invalid frame must fail fast.
            // A conforming destination clamps into this range (D5 /
            // D-2026-07-10-1); same-build peers make a mismatch a
            // violation, never a negotiation. The ceiling is the
            // CARRIER's (otp-7b, D-2026-07-10-2): binary data-plane
            // records take up to the wire block cap; in-stream frames
            // must stay under the gRPC frame limit.
            let ceiling = if data_plane.is_some() {
                MAX_DATA_PLANE_RESUME_BLOCK_SIZE
            } else {
                MAX_IN_STREAM_RESUME_BLOCK_SIZE
            };
            let bs = list.block_size as usize;
            if !(MIN_RESUME_BLOCK_SIZE..=ceiling).contains(&bs) {
                return Err(eyre::Report::new(SessionFault::protocol_violation(
                    format!(
                        "BlockHashList for '{}' block_size {bs} outside \
                         [{MIN_RESUME_BLOCK_SIZE}, {ceiling}]",
                        list.relative_path
                    ),
                )));
            }
            match resume.held.remove(&list.relative_path) {
                Some(header) => {
                    resume.ready.push((header, list));
                    Ok(())
                }
                None => Err(eyre::Report::new(SessionFault::protocol_violation(
                    format!(
                        "BlockHashList for '{}' without a held resume need",
                        list.relative_path
                    ),
                ))),
            }
        }
        SourceEvent::NeedComplete => {
            if *need_complete {
                return Err(eyre::Report::new(SessionFault::protocol_violation(
                    "duplicate NeedComplete",
                )));
            }
            // Ordered lane: the destination sends every BlockHashList
            // before its NeedComplete, so a still-held resume need here
            // means the peer broke the choreography — fail fast rather
            // than hang waiting for a list that can no longer arrive.
            if !resume.held.is_empty() {
                return Err(eyre::Report::new(SessionFault::protocol_violation(
                    format!(
                        "NeedComplete with {} resume need(s) missing their BlockHashList",
                        resume.held.len()
                    ),
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
            let Some(pending_r) = resize.take_pending_for_ack(&ack)? else {
                return Ok(());
            };
            let current = dp.dial().live_streams();
            validate_source_resize_ack(&pending_r, current, &ack)?;
            if ack.accepted {
                let membership = match pending_r.op {
                    DataPlaneResizeOp::Add => {
                        let token = pending_r.sub_token.as_deref().ok_or_else(|| {
                            eyre::Report::new(SessionFault::internal(
                                "pending ADD has no sub-token",
                            ))
                        })?;
                        dp.add_stream(pending_r.epoch, token).await?
                    }
                    DataPlaneResizeOp::Remove => {
                        if pending_r.sub_token.is_some() {
                            return Err(eyre::Report::new(SessionFault::internal(
                                "pending REMOVE carries a sub-token",
                            )));
                        }
                        dp.retire_stream().await?
                    }
                    DataPlaneResizeOp::Unspecified => {
                        return Err(eyre::Report::new(SessionFault::internal(
                            "pending resize has unspecified operation",
                        )))
                    }
                };
                let logical_count = match (pending_r.op, membership) {
                    (
                        DataPlaneResizeOp::Add,
                        MembershipOutcome::Joined { logical_count, .. }
                        | MembershipOutcome::JoinedThenEnded { logical_count, .. },
                    ) => logical_count,
                    (
                        DataPlaneResizeOp::Remove,
                        MembershipOutcome::RetireMarked { logical_count, .. }
                        | MembershipOutcome::AlreadyEnded { logical_count, .. },
                    ) => logical_count,
                    (_, other) => {
                        return Err(eyre::Report::new(SessionFault::internal(format!(
                            "accepted {} produced unexpected membership outcome {other:?}",
                            pending_r.op.as_str_name()
                        ))))
                    }
                };
                if logical_count != pending_r.target_streams as usize {
                    return Err(eyre::Report::new(SessionFault::internal(format!(
                        "accepted {} settled {logical_count} members, expected {}",
                        pending_r.op.as_str_name(),
                        pending_r.target_streams
                    ))));
                }
                dp.dial()
                    .resize_settled(pending_r.epoch, pending_r.target_streams as usize, true);
            } else {
                dp.dial().resize_settled(pending_r.epoch, current, false);
            }
            if let Some(trace) = dp.phase_trace() {
                trace.event(
                    "source_settled",
                    SessionPhaseFields {
                        action: Some(pending_r.op.as_str_name()),
                        epoch: Some(pending_r.epoch),
                        target_streams: Some(pending_r.target_streams),
                        live_streams: Some(dp.dial().live_streams() as u32),
                        accepted: Some(ack.accepted),
                        ..Default::default()
                    },
                );
            }
            resize.last_ack = Some(ack);
            Ok(())
        }
        SourceEvent::Summary(_) => Err(eyre::Report::new(SessionFault::protocol_violation(
            "TransferSummary before SourceDone",
        ))),
        SourceEvent::Fault(fault) => Err(eyre::Report::new(fault)),
    }
}

async fn send_resize_proposal(
    dp: &data_plane::SourceDataPlane,
    tx: &mut Box<dyn FrameTx>,
    proposal: crate::dial::ResizeProposal,
    resize: &mut SourceResizeState,
) -> Result<()> {
    if resize.pending.is_some() {
        dp.refuse_unsent_resize(proposal);
        return Err(eyre::Report::new(SessionFault::internal(
            "live dial emitted a second proposal while one is in flight",
        )));
    }
    if let Some(pending) = dp.prepare_resize(proposal)? {
        if let Some(trace) = dp.phase_trace() {
            trace.event(
                "resize_proposed",
                SessionPhaseFields {
                    action: Some(pending.op.as_str_name()),
                    epoch: Some(pending.epoch),
                    target_streams: Some(pending.target_streams),
                    live_streams: Some(dp.dial().live_streams() as u32),
                    ..Default::default()
                },
            );
        }
        if let Some(trace) = dp.phase_trace() {
            trace.event(
                "resize_send_begin",
                SessionPhaseFields {
                    action: Some(pending.op.as_str_name()),
                    epoch: Some(pending.epoch),
                    target_streams: Some(pending.target_streams),
                    live_streams: Some(dp.dial().live_streams() as u32),
                    ..Default::default()
                },
            );
        }
        let wire = DataPlaneResize {
            op: pending.op as i32,
            epoch: pending.epoch,
            target_stream_count: pending.target_streams,
            sub_token: pending.sub_token.clone().unwrap_or_default(),
        };
        if let Err(err) = tx.send(frame(Frame::Resize(wire))).await {
            dp.dial()
                .resize_settled(pending.epoch, dp.dial().live_streams(), false);
            return Err(err);
        }
        if let Some(trace) = dp.phase_trace() {
            trace.event(
                "resize_sent",
                SessionPhaseFields {
                    action: Some(pending.op.as_str_name()),
                    epoch: Some(pending.epoch),
                    target_streams: Some(pending.target_streams),
                    live_streams: Some(dp.dial().live_streams() as u32),
                    ..Default::default()
                },
            );
        }
        resize.pending = Some(pending);
    }
    Ok(())
}

/// Plan and admit one need-authorized batch to the TCP data plane. Shared by
/// the scan-overlap path and the post-manifest tail so batching, probes, phase
/// traces, and control-lane servicing cannot diverge between them.
#[allow(clippy::too_many_arguments)]
async fn queue_tcp_payload_batch(
    batch: Vec<FileHeader>,
    source: &Arc<dyn TransferSource>,
    plan_options: PlanOptions,
    data_plane: &data_plane::SourceDataPlane,
    events: &mut mpsc::UnboundedReceiver<SourceEvent>,
    pending: &mut Vec<FileHeader>,
    resume: &mut ResumeSendState,
    need_complete: &mut bool,
    tx: &mut Box<dyn FrameTx>,
    resize: &mut SourceResizeState,
    phase_trace: Option<&BoundSessionPhaseTrace>,
    small_file_probe: Option<&BoundSmallFileProbe>,
    planner_batch_seq: &mut u64,
) -> Result<()> {
    let batch_count = batch.len() as u64;
    if let Some(trace) = phase_trace {
        trace.event(
            "planner_begin",
            SessionPhaseFields {
                batch: Some(*planner_batch_seq),
                count: Some(batch_count),
                ..Default::default()
            },
        );
    }
    let planner_input = batch.len();
    let planner_started = small_file_probe.map(BoundSmallFileProbe::start);
    let payloads = diff_planner::plan_push_payloads(batch, source.root(), plan_options)?;
    if let (Some(probe), Some(started)) = (small_file_probe, planner_started) {
        let (tar_shards, tar_members) = tar_payload_shape(&payloads);
        probe.note_planner(
            started.elapsed(),
            planner_input,
            payloads.len(),
            tar_shards,
            tar_members,
        );
    }
    if let Some(trace) = phase_trace {
        trace.event(
            "planner_end",
            SessionPhaseFields {
                batch: Some(*planner_batch_seq),
                count: Some(payloads.len() as u64),
                ..Default::default()
            },
        );
    }
    *planner_batch_seq += 1;
    queue_payloads_while_servicing_events(
        payloads,
        events,
        pending,
        resume,
        need_complete,
        data_plane,
        tx,
        resize,
        small_file_probe,
    )
    .await
}

/// Feed one planned batch into the shared bounded data-plane queue while
/// continuing to service the SOURCE control lane. Queue readiness is biased
/// first so epoch-0 work starts immediately; once backpressure applies,
/// resize ACKs apply ADD or REMOVE to that same queue and newly arriving needs
/// are retained for the next planner batch.
#[allow(clippy::too_many_arguments)]
async fn queue_payloads_while_servicing_events(
    payloads: Vec<TransferPayload>,
    events: &mut mpsc::UnboundedReceiver<SourceEvent>,
    pending: &mut Vec<FileHeader>,
    resume: &mut ResumeSendState,
    need_complete: &mut bool,
    data_plane: &data_plane::SourceDataPlane,
    tx: &mut Box<dyn FrameTx>,
    resize: &mut SourceResizeState,
    small_file_probe: Option<&BoundSmallFileProbe>,
) -> Result<()> {
    for payload in payloads {
        let queued = data_plane.queue(payload);
        tokio::pin!(queued);
        loop {
            tokio::select! {
                biased;
                result = &mut queued => {
                    if let Err(dp_err) = result {
                        return Err(prefer_peer_fault(events, dp_err).await);
                    }
                    break;
                }
                event = events.recv() => {
                    let event = event.ok_or_else(|| {
                        eyre::Report::new(SessionFault::internal(
                            "source receive half ended while queueing data-plane payloads",
                        ))
                    })?;
                    process_source_event(
                        event,
                        pending,
                        resume,
                        need_complete,
                        Some(data_plane),
                        resize,
                        small_file_probe,
                    )
                    .await?;
                }
                proposal = async {
                    match resize.proposals.as_mut() {
                        Some(proposals) => proposals.recv().await,
                        None => std::future::pending().await,
                    }
                } => {
                    match proposal {
                        Some(proposal) => {
                            send_resize_proposal(data_plane, tx, proposal, resize).await?;
                        }
                        None => resize.proposals = None,
                    }
                }
            }
        }
    }
    Ok(())
}

fn stop_resize_intake(data_plane: &data_plane::SourceDataPlane, resize: &mut SourceResizeState) {
    if let Some(mut proposals) = resize.proposals.take() {
        proposals.close();
        while let Ok(proposal) = proposals.try_recv() {
            data_plane.refuse_unsent_resize(proposal);
        }
    }
}

#[allow(clippy::too_many_arguments)]
async fn settle_inflight_resize(
    events: &mut mpsc::UnboundedReceiver<SourceEvent>,
    pending: &mut Vec<FileHeader>,
    resume: &mut ResumeSendState,
    need_complete: &mut bool,
    data_plane: &data_plane::SourceDataPlane,
    tx: &mut Box<dyn FrameTx>,
    resize: &mut SourceResizeState,
    small_file_probe: Option<&BoundSmallFileProbe>,
) -> Result<()> {
    let _ = tx;
    while resize.pending.is_some() {
        let event = events.recv().await.ok_or_else(|| {
            eyre::Report::new(SessionFault::internal(
                "source receive half ended during data-plane resize",
            ))
        })?;
        process_source_event(
            event,
            pending,
            resume,
            need_complete,
            Some(data_plane),
            resize,
            small_file_probe,
        )
        .await?;
    }
    Ok(())
}

/// Await the next terminal signal the receive half forwards while the
/// data-plane drain is in progress (otp-4b-3). Used to race the drain: a
/// mid-transfer `SessionError` (e.g. a `CancelJob` → `CANCELLED`) must
/// abort the send and surface as the fault.
///
/// The drain runs after proposal intake stops and before `SourceDone` goes
/// out, so the event channel is drained and the peer sends nothing
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
        Some(SourceEvent::Need(h) | SourceEvent::ResumeNeed(h)) => {
            SessionFault::protocol_violation(format!(
                "need for '{}' during the data-plane drain (after NeedComplete)",
                h.relative_path
            ))
        }
        Some(SourceEvent::BlockHashes(l)) => SessionFault::protocol_violation(format!(
            "BlockHashList for '{}' during the data-plane drain",
            l.relative_path
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

/// Split any planned tar shard whose encoded `TarShardHeader` member
/// list would exceed `max_encoded` into consecutive smaller shards
/// (codex otp-8 F2 — the in-stream carrier's frame-limit bound; cap
/// rationale at [`MAX_IN_STREAM_TAR_HEADER_BYTES`]). Order and file
/// set are preserved; non-shard payloads pass through untouched. Pure,
/// so the bound is unit-testable without a 4 MiB fixture.
fn bound_in_stream_tar_headers(
    payloads: Vec<TransferPayload>,
    max_encoded: usize,
) -> Vec<TransferPayload> {
    use prost::Message;
    let mut out = Vec::with_capacity(payloads.len());
    for payload in payloads {
        match payload {
            TransferPayload::TarShard { headers } => {
                let mut cur: Vec<FileHeader> = Vec::new();
                let mut cur_bytes = 0usize;
                for header in headers {
                    // One repeated-field entry costs its message bytes
                    // plus tag + length delimiter; 5 covers any header
                    // a filesystem can produce.
                    let cost = header.encoded_len() + 5;
                    if !cur.is_empty() && cur_bytes + cost > max_encoded {
                        out.push(TransferPayload::TarShard {
                            headers: std::mem::take(&mut cur),
                        });
                        cur_bytes = 0;
                    }
                    cur_bytes += cost;
                    cur.push(header);
                }
                if !cur.is_empty() {
                    out.push(TransferPayload::TarShard { headers: cur });
                }
            }
            other => out.push(other),
        }
    }
    out
}

/// Plan one batch of needed headers with the engine planner and emit
/// the resulting payload records per the in-stream grammar.
async fn send_payload_records(
    tx: &mut Box<dyn FrameTx>,
    source: &Arc<dyn TransferSource>,
    plan_options: PlanOptions,
    batch: Vec<FileHeader>,
    read_buf: &mut [u8],
    progress: Option<&RemoteTransferProgress>,
    small_file_probe: Option<&BoundSmallFileProbe>,
) -> Result<()> {
    let planner_input = batch.len();
    let planner_started = small_file_probe.map(BoundSmallFileProbe::start);
    let payloads = diff_planner::plan_push_payloads(batch, source.root(), plan_options)?;
    if let (Some(probe), Some(started)) = (small_file_probe, planner_started) {
        let (tar_shards, tar_members) = tar_payload_shape(&payloads);
        probe.note_planner(
            started.elapsed(),
            planner_input,
            payloads.len(),
            tar_shards,
            tar_members,
        );
    }
    // In-stream only: every shard's header frame must clear the tonic
    // frame limit (codex otp-8 F2). The data-plane branch keeps the
    // planner's shards whole — its records are not protobuf frames.
    let payloads = bound_in_stream_tar_headers(payloads, MAX_IN_STREAM_TAR_HEADER_BYTES);
    // Progress convention (otp-10a): identical to the data-plane sink
    // pipeline — per-file lane, planned manifest sizes, one
    // `Payload{0, size}` + `FileComplete` pair per file after its
    // record completes. Both carriers therefore report the same shape.
    let report_files = |files: &[(String, u64)]| {
        if let Some(p) = progress {
            for (name, size) in files {
                p.report_payload(0, *size);
                p.report_file_complete(name.clone());
            }
        }
    };
    for payload in payloads {
        match source.prepare_payload(payload).await? {
            PreparedPayload::File(header) => {
                tx.send(frame(Frame::FileBegin(header.clone()))).await?;
                if header.size == 0 {
                    report_files(&[(header.relative_path.clone(), 0)]);
                    continue; // record complete at 0 cumulative bytes
                }
                let mut reader = source
                    .open_file(&header)
                    .await
                    .map_err(|e| tag_path(e, &header.relative_path))?;
                let mut remaining = header.size;
                while remaining > 0 {
                    let want = read_buf.len().min(remaining as usize);
                    let got = reader
                        .read(&mut read_buf[..want])
                        .await
                        .map_err(|e| tag_path(eyre::Report::new(e), &header.relative_path))?;
                    if got == 0 {
                        // Shorter on disk than the manifest promised —
                        // the record can no longer complete at
                        // header.size; abort rather than pad.
                        return Err(tag_path(
                            eyre::eyre!(
                                "'{}' hit EOF with {} bytes still promised",
                                header.relative_path,
                                remaining
                            ),
                            &header.relative_path,
                        ));
                    }
                    tx.send(frame(Frame::FileData(FileData {
                        content: read_buf[..got].to_vec(),
                    })))
                    .await?;
                    remaining -= got as u64;
                }
                report_files(&[(header.relative_path.clone(), header.size)]);
            }
            PreparedPayload::TarShard { headers, data } => {
                let shard_files: Vec<(String, u64)> = headers
                    .iter()
                    .map(|h| (h.relative_path.clone(), h.size))
                    .collect();
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
                report_files(&shard_files);
            }
            PreparedPayload::FileBlock { .. }
            | PreparedPayload::FileBlockComplete { .. }
            | PreparedPayload::ResumeFile { .. } => {
                // The outbound planner never emits these: resume block
                // records are choreography-originated (otp-7a in-stream,
                // otp-7b data plane), never planned.
                eyre::bail!("resume payload planned by the outbound planner");
            }
        }
    }
    Ok(())
}

fn tar_payload_shape(payloads: &[TransferPayload]) -> (usize, usize) {
    payloads.iter().fold((0, 0), |(shards, members), payload| {
        if let TransferPayload::TarShard { headers } = payload {
            (shards + 1, members + headers.len())
        } else {
            (shards, members)
        }
    })
}

/// otp-7a: the SOURCE-side block phase for one resume-flagged need over
/// the IN-STREAM carrier — a session free helper, deliberately not a
/// `TransferSource` method (plan D3: it needs only `open_file` + blake3,
/// and keeping it off the trait spares every future source impl from
/// re-implementing it, the same reasoning that made `FilteredSource`
/// the one filter chokepoint).
///
/// The diff itself — read sequentially at the block size the
/// DESTINATION chose (plan D5; range-validated at frame arrival),
/// blake3 each block, treat an index beyond the list, a differing hash,
/// or a MALFORMED hash as stale (plan D1, the graceful stale-partial
/// fallback) — is [`ResumeBlockDiff`], shared verbatim with the data
/// plane's `DataPlaneSink` (otp-7b) so both carriers keep one staleness
/// semantic. Ends with `BlockTransferComplete{total_bytes =
/// header.size}`; the manifest promised `header.size`, so EOF-short
/// aborts exactly as a whole-file record does.
async fn send_resume_block_records(
    tx: &mut Box<dyn FrameTx>,
    source: &Arc<dyn TransferSource>,
    header: &FileHeader,
    hashes: &BlockHashList,
    progress: Option<&RemoteTransferProgress>,
) -> Result<()> {
    use crate::remote::transfer::resume_diff::{ResumeBlockDiff, ResumeDiffEvent};
    // block_size was range-validated when the BlockHashList arrived
    // (`process_source_event`, codex F5) — use it directly. Keepalive
    // stays unarmed: the control lane carries no receive stall guard,
    // so a silent scan cannot trip one (codex 7b-1 F1 is a data-plane
    // concern; `DataPlaneSink` arms it there).
    let mut diff = ResumeBlockDiff::open(
        source,
        header,
        hashes.block_size as usize,
        hashes.hashes.clone(),
    )
    .await?;
    let mut stale_bytes: u64 = 0;
    while let Some(event) = diff.next_event().await? {
        match event {
            ResumeDiffEvent::Stale { offset, bytes } => {
                stale_bytes += bytes.len() as u64;
                tx.send(frame(Frame::Block(BlockTransfer {
                    relative_path: header.relative_path.clone(),
                    offset,
                    content: bytes.to_vec(),
                })))
                .await?;
            }
            ResumeDiffEvent::KeepAlive { .. } => {}
        }
    }
    tx.send(frame(Frame::BlockComplete(BlockTransferComplete {
        relative_path: header.relative_path.clone(),
        total_bytes: header.size,
    })))
    .await?;
    // codex otp-10a F6: a resumed file finishes like any other (w6-1:
    // per-file lane, counted once); its bytes are the stale blocks
    // actually sent — the same convention as the data-plane carrier.
    if let Some(p) = progress {
        p.report_payload(0, stale_bytes);
        p.report_file_complete(header.relative_path.clone());
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
    /// Final settled logical data-plane membership (epoch 0 plus accepted
    /// ADD/REMOVE epochs), or `None` for the in-stream carrier. This is not
    /// the cumulative number of sockets ever opened.
    pub data_plane_streams: Option<usize>,
}

#[derive(Clone, Debug, PartialEq)]
enum DestinationResizeDecision {
    Apply(DataPlaneResizeOp),
    Refuse,
    Replay(DataPlaneResizeAck),
}

/// Receiver-side logical membership authority shared by both connection
/// layouts. Socket acquisition remains role-specific, but epoch validation,
/// bounds, duplicate replay, terminal refusal, and the exposed final count do
/// not know which end opened the control connection.
#[derive(Debug)]
struct DestinationResizeState {
    settled_epoch: u32,
    live_streams: usize,
    peak_streams: usize,
    ceiling: usize,
    refused: bool,
    last_request: Option<DataPlaneResize>,
    last_ack: Option<DataPlaneResizeAck>,
}

impl DestinationResizeState {
    fn new(live_streams: usize, ceiling: usize) -> Self {
        debug_assert!((1..=ceiling).contains(&live_streams));
        Self {
            settled_epoch: 0,
            live_streams,
            peak_streams: live_streams,
            ceiling,
            refused: false,
            last_request: None,
            last_ack: None,
        }
    }

    fn classify(&self, request: &DataPlaneResize) -> Result<DestinationResizeDecision> {
        if request.epoch == self.settled_epoch {
            return match (&self.last_request, &self.last_ack) {
                (Some(last_request), Some(last_ack)) if last_request == request => {
                    Ok(DestinationResizeDecision::Replay(*last_ack))
                }
                _ => Err(violation(format!(
                    "changed or unsolicited duplicate DataPlaneResize epoch {}",
                    request.epoch
                ))),
            };
        }

        if request.epoch < self.settled_epoch {
            return Err(violation(format!(
                "stale DataPlaneResize epoch {} after settled epoch {}",
                request.epoch, self.settled_epoch
            )));
        }
        let expected_epoch = self
            .settled_epoch
            .checked_add(1)
            .ok_or_else(|| violation("data-plane resize epoch space exhausted".to_string()))?;
        if request.epoch != expected_epoch {
            return Err(violation(format!(
                "DataPlaneResize epoch {} while next epoch is {expected_epoch}",
                request.epoch
            )));
        }
        if self.refused {
            return Err(violation(format!(
                "DataPlaneResize epoch {} after terminal refusal",
                request.epoch
            )));
        }

        let op = DataPlaneResizeOp::try_from(request.op).map_err(|_| {
            violation(format!(
                "unsupported data-plane resize op value {}",
                request.op
            ))
        })?;
        match op {
            DataPlaneResizeOp::Add => {
                if request.sub_token.len() != crate::remote::transfer::SUB_TOKEN_LEN {
                    return Err(violation(
                        "DataPlaneResize ADD sub_token must be 16 bytes".to_string(),
                    ));
                }
                let expected_target = self
                    .live_streams
                    .checked_add(1)
                    .ok_or_else(|| violation("data-plane stream count overflow".to_string()))?;
                if request.target_stream_count as usize != expected_target {
                    return Err(violation(format!(
                        "DataPlaneResize ADD target {} from live {} must be {expected_target}",
                        request.target_stream_count, self.live_streams
                    )));
                }
                if expected_target > self.ceiling {
                    Ok(DestinationResizeDecision::Refuse)
                } else {
                    Ok(DestinationResizeDecision::Apply(op))
                }
            }
            DataPlaneResizeOp::Remove => {
                if !request.sub_token.is_empty() {
                    return Err(violation(
                        "DataPlaneResize REMOVE sub_token must be empty".to_string(),
                    ));
                }
                let expected_target = self.live_streams.saturating_sub(1);
                if request.target_stream_count as usize != expected_target {
                    return Err(violation(format!(
                        "DataPlaneResize REMOVE target {} from live {} must be {expected_target}",
                        request.target_stream_count, self.live_streams
                    )));
                }
                if expected_target < 1 {
                    Ok(DestinationResizeDecision::Refuse)
                } else {
                    Ok(DestinationResizeDecision::Apply(op))
                }
            }
            DataPlaneResizeOp::Unspecified => Err(violation(
                "unsupported data-plane resize op DATA_PLANE_RESIZE_OP_UNSPECIFIED".to_string(),
            )),
        }
    }

    fn settle(&mut self, request: DataPlaneResize, accepted: bool) -> DataPlaneResizeAck {
        if accepted {
            self.live_streams = request.target_stream_count as usize;
            self.peak_streams = self.peak_streams.max(self.live_streams);
        } else {
            self.refused = true;
        }
        self.settled_epoch = request.epoch;
        let ack = DataPlaneResizeAck {
            epoch: request.epoch,
            effective_stream_count: self.live_streams as u32,
            accepted,
        };
        self.last_request = Some(request);
        self.last_ack = Some(ack);
        ack
    }
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
    let mut receiver_capacity = cfg
        .receiver_capacity
        .unwrap_or_else(crate::dial::local_receiver_capacity);
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
            if let Some(advertised) = &open.receiver_capacity {
                receiver_capacity = *advertised;
            } else {
                open.receiver_capacity = Some(receiver_capacity);
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
        Some(&receiver_capacity),
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
        cfg.instruments,
        cfg.local_apply,
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
    instruments: DestinationInstruments,
    local_apply: Option<local::LocalApply>,
) -> Result<DestinationOutcome> {
    match destination_session(
        transport,
        negotiated,
        dst_root,
        data_plane_host,
        instruments,
        local_apply,
    )
    .await
    {
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
    // Operator policy from the serving daemon's runtime config
    // (`--force-grpc-data`, `--no-server-checksums`).
    policy: ResponderPolicy,
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
            let receiver_capacity = crate::dial::local_receiver_capacity();
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
                &policy,
                Some(&receiver_capacity),
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
            // Served destination (push-equivalent): no instruments — the
            // serving daemon has no progress line; wiring the daemon
            // row's byte counter through here is the core.rs jobs-row
            // follow-up.
            let outcome = drive_destination(
                &mut transport,
                negotiated,
                &dst_root,
                None,
                DestinationInstruments::default(),
                // The serving daemon never applies locally — the local
                // carrier exists only inside run_local_session's process.
                None,
            )
            .await?;
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
                &policy,
                None,
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
            // dials, so it needs no data-plane host. No instruments: the
            // serving daemon has no progress line, and an incomplete scan
            // already travels as `ManifestComplete{scan_complete}`.
            let summary = drive_source(
                PlanOptions::default(),
                None,
                SourceInstruments::default(),
                negotiated,
                transport,
                source,
            )
            .await?;
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

/// A protocol violation that names the file it concerns (otp-7b-2):
/// the path rides `SessionFault.relative_path` so the end-of-operation
/// summary can name it structurally.
fn violation_for(path: &str, message: String) -> eyre::Report {
    eyre::Report::new(SessionFault::protocol_violation(message).with_path(path))
}

/// Attach `path` to a non-fault error (otp-7b-2). A report already
/// carrying a `SessionFault` is left untouched — the fault owns its
/// own identity, and wrapping it would bury the downcast
/// `fault_from_report` depends on.
fn tag_path(report: eyre::Report, path: &str) -> eyre::Report {
    if report.downcast_ref::<SessionFault>().is_some() {
        report
    } else {
        report.wrap_err(FaultedPath(path.to_string()))
    }
}

/// otp-6b: the DESTINATION's mirror delete pass — the session's single
/// delete rule. Plans (enumerate dest + diff against the complete source
/// file set) and executes the extraneous deletions, all blocking FS work,
/// so it runs on the blocking pool. Returns `(files, dirs)` deleted —
/// split so the local carrier's summary (otp-11) can report both; wire
/// summaries carry the sum. `execute: false` (local `--dry-run` only)
/// plans and counts without touching the filesystem.
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
    abort: &AtomicBool,
    execute: bool,
) -> Result<(u64, u64)> {
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

    // codex otp-9b F2: a dropped session future (client disconnect,
    // CancelJob) cannot abort a running blocking task — the caller's
    // drop-guard flips this flag instead, and the pass stops deleting
    // at the next filesystem op rather than running to completion
    // behind a job already recorded cancelled.
    let check_abort = || -> Result<()> {
        if abort.load(Ordering::Acquire) {
            return Err(eyre::eyre!("mirror delete pass aborted: session cancelled"));
        }
        Ok(())
    };

    let mut deleted_files = 0u64;
    let mut deleted_dirs = 0u64;
    for file in &plan.files {
        check_abort()?;
        contained(file)?;
        if !execute {
            deleted_files += 1;
            continue;
        }
        // Windows refuses to delete a read-only file; clear the attribute
        // first, matching the daemon purge (admin.rs) and local mirror
        // (engine/mirror.rs) executors (codex otp-6b F2).
        #[cfg(windows)]
        crate::win_fs::clear_readonly_recursive(file);
        match std::fs::remove_file(file) {
            Ok(()) => deleted_files += 1,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
            Err(e) => return Err(eyre::eyre!("mirror delete {}: {e}", file.display())),
        }
    }
    for dir in &plan.dirs {
        check_abort()?;
        contained(dir)?;
        if !execute {
            deleted_dirs += 1;
            continue;
        }
        #[cfg(windows)]
        crate::win_fs::clear_readonly_recursive(dir);
        match std::fs::remove_dir(dir) {
            Ok(()) => deleted_dirs += 1,
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
    Ok((deleted_files, deleted_dirs))
}

async fn destination_session(
    transport: &mut FrameTransport,
    negotiated: Negotiated,
    dst_root: &Path,
    data_plane_host: Option<&str>,
    instruments: DestinationInstruments,
    local_apply: Option<local::LocalApply>,
) -> Result<DestinationOutcome> {
    let phase_trace = bind_session_phase_trace(
        instruments.session_phase_trace.clone(),
        &negotiated,
        SessionPhaseRole::Destination,
    );
    let small_file_probe = bind_small_file_probe(
        instruments.small_file_probe.clone(),
        &negotiated,
        SessionPhaseRole::Destination,
    );
    let result = destination_session_inner(
        transport,
        negotiated,
        dst_root,
        data_plane_host,
        instruments,
        local_apply,
        phase_trace.clone(),
        small_file_probe.clone(),
    )
    .await;
    flush_session_phase_trace(phase_trace.as_ref()).await;
    finish_small_file_probe(small_file_probe.as_ref()).await;
    result
}

#[allow(clippy::too_many_arguments)]
async fn destination_session_inner(
    transport: &mut FrameTransport,
    negotiated: Negotiated,
    dst_root: &Path,
    data_plane_host: Option<&str>,
    instruments: DestinationInstruments,
    local_apply: Option<local::LocalApply>,
    phase_trace: Option<BoundSessionPhaseTrace>,
    small_file_probe: Option<BoundSmallFileProbe>,
) -> Result<DestinationOutcome> {
    // otp-10b-2: the receive side's w6-1 progress lane. Need batches are
    // the denominator (reported where they're emitted, below); per-file
    // events ride each carrier's record handling.
    let progress = instruments.progress;
    let compare_mode = ComparisonMode::try_from(negotiated.open.compare_mode)
        .unwrap_or(ComparisonMode::Unspecified);
    // Session deletions run via the otp-6b mirror pass (a whole-tree
    // diff at SourceDone), never a per-entry flag.
    let compare_opts = CompareOptions {
        mode: compare_mode.into(),
        ignore_existing: negotiated.open.ignore_existing,
    };
    // src_root is only consumed by local File payloads, which never
    // occur on a WIRE session destination (payload bytes arrive as
    // records and go through the stream/tar write paths); the LOCAL
    // carrier (otp-11) brings its own fully-configured sink, where
    // File payloads are the point. `Arc` so the data-plane receive
    // task (otp-4b) can share the one sink across sockets.
    let sink: Arc<dyn TransferSink> = match &local_apply {
        Some(la) => Arc::clone(&la.sink),
        None => {
            let mut sink = FsTransferSink::new(
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
            // otp-9a: applied payload bytes report against the caller's live
            // counter (the delegated dst daemon's jobs row) through the sink's
            // existing ByteProgressSink contract.
            if let Some(bp) = instruments.byte_progress {
                sink = sink.with_byte_progress(bp);
            }
            sink = sink.with_small_file_probe(small_file_probe.clone());
            Arc::new(sink)
        }
    };
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
    let mirror_filter: crate::fs_enum::FileFilter =
        if mirror_enabled && mirror_kind == MirrorMode::FilteredSubset {
            // otp-11: the local carrier threads the user's FileFilter
            // directly (process-local; no wire FilterSpec round-trip) —
            // same type, same delete pass, same scope semantics.
            if let Some(la) = &local_apply {
                la.mirror_scope_filter.clone_without_cache()
            } else {
                match negotiated.open.filter.as_ref() {
                    Some(spec) if *spec != FilterSpec::default() => {
                        crate::remote::transfer::operation_spec::filter_from_spec(spec.clone())
                            .map_err(|e| {
                                eyre::Report::new(SessionFault::internal(format!(
                                    "invalid filter: {e:#}"
                                )))
                            })?
                    }
                    _ => crate::fs_enum::FileFilter::default(),
                }
            }
        } else {
            crate::fs_enum::FileFilter::default()
        };
    let mut source_files: HashSet<String> = HashSet::new();

    // otp-7a: resume. Headers of resume-granted needs are retained so a
    // record's completion can finalize with the manifest's
    // size/mtime/permissions and be validated against the grant. Both
    // the header map and the resumed counter are SHARED with the
    // data-plane receive (otp-7b) exactly as `outstanding` is: on the
    // data plane the control loop never sees block records, so the
    // NeedListSink claims resume grants and counts completions as they
    // land on the sockets. The block size is chosen below, once the
    // carrier is known (the ceiling is per carrier).
    let resume_enabled = resume_negotiated(&negotiated.open);
    let resume_headers: data_plane::ResumeHeaders = Arc::default();
    let files_resumed = Arc::new(std::sync::atomic::AtomicU64::new(0));

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
    // responder run) bounds the accept task to this future. The shared resize
    // state below owns logical membership and the receiver's advertised
    // ceiling for both connection layouts; only socket acquisition differs.
    let recv_sink: Arc<dyn TransferSink> = Arc::new(data_plane::NeedListSink::new(
        Arc::clone(&sink),
        Arc::clone(&outstanding),
        // otp-7b: only a resume session accepts block records on the
        // data plane; the sink validates + claims them against the same
        // shared grant state the in-stream arms use.
        resume_enabled.then(|| data_plane::ResumeRecv {
            headers: Arc::clone(&resume_headers),
            resumed: Arc::clone(&files_resumed),
        }),
        small_file_probe.clone(),
    ));
    let (mut data_plane_recv, resize_initial, resize_ceiling) =
        match negotiated.responder_data_plane {
            // DESTINATION responder (push, otp-4b): accept + receive.
            Some(rdp) => {
                let initial = rdp.initial_streams() as usize;
                let run = rdp.spawn(
                    recv_sink,
                    progress.clone(),
                    phase_trace.clone(),
                    small_file_probe.clone(),
                );
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
                    let initial = grant.initial_streams as usize;
                    let run = data_plane::dial_destination_data_plane(
                        host,
                        grant,
                        negotiated.open.receiver_capacity.as_ref(),
                        recv_sink,
                        progress.clone(),
                        instruments.trace_data_plane,
                        phase_trace.clone(),
                        small_file_probe.clone(),
                    )
                    .await?;
                    // The DESTINATION initiator seeds the same logical resize
                    // state from the exact epoch-0 sockets it dialed and uses
                    // the capacity it advertised in SessionOpen. The SOURCE
                    // responder's dial resolves that same ceiling, so both
                    // ends admit identical ADD/REMOVE targets even when the
                    // advertised limit is below this host's fresh local read.
                    // ADD dials an epoch-N socket here; REMOVE opens none.
                    let ceiling = crate::dial::receiver_stream_ceiling(
                        negotiated.open.receiver_capacity.as_ref(),
                    );
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
    let mut resize_state = data_plane_recv
        .as_ref()
        .map(|_| DestinationResizeState::new(resize_initial, resize_ceiling));

    // otp-7a/7b: the DESTINATION chooses the resume block size (plan D5
    // — it hashes first; the SOURCE reads the size from each
    // BlockHashList): 0 ⇒ default, clamped to THIS CARRIER's cap
    // (D-2026-07-10-1 in-stream, D-2026-07-10-2 data plane) — decided
    // here, after the carrier is settled.
    let resume_block_size = {
        let ceiling = if data_plane_recv.is_some() {
            MAX_DATA_PLANE_RESUME_BLOCK_SIZE
        } else {
            MAX_IN_STREAM_RESUME_BLOCK_SIZE
        };
        match negotiated
            .open
            .resume
            .as_ref()
            .map(|r| r.block_size as usize)
            .unwrap_or(0)
        {
            0 => DEFAULT_BLOCK_SIZE,
            bs => bs.clamp(MIN_RESUME_BLOCK_SIZE, ceiling),
        }
    };

    let mut pending: Vec<FileHeader> = Vec::new();
    let mut needed_paths: Vec<String> = Vec::new();
    let mut manifest_complete = false;
    let mut files_written: u64 = 0;
    let mut bytes_written: u64 = 0;
    let mut need_batch_seq = 0u64;

    // otp-11: the LOCAL carrier's apply pipeline — spawned before the
    // loop so applies run concurrent with the diff, exactly as the
    // data-plane receive does.
    let mut local_run = local_apply.as_ref().map(|la| la.start(progress.clone()));

    let result: Result<DestinationOutcome> = async {
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
                    if let Some(la) = &local_apply {
                        diff_chunk_and_apply_local(
                            la,
                            &mut local_run,
                            chunk,
                            dst_root,
                            canonical_dst_root.as_deref(),
                            &compare_opts,
                            &mut granted,
                            &mut needed_paths,
                            progress.as_ref(),
                        )
                        .await?;
                    } else {
                        diff_chunk_and_send_needs(
                            transport,
                            chunk,
                            dst_root,
                            canonical_dst_root.as_deref(),
                            &compare_opts,
                            resume_enabled,
                            resume_block_size,
                            &resume_headers,
                            &mut granted,
                            &outstanding,
                            &mut needed_paths,
                            progress.as_ref(),
                            phase_trace.as_ref(),
                            &mut need_batch_seq,
                        )
                        .await?;
                    }
                }
            }
            Some(Frame::ManifestComplete(complete)) => {
                if let Some(trace) = &phase_trace {
                    trace.event("manifest_complete_received", SessionPhaseFields::default());
                }
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
                // codex otp-9b F1 (R49-F2 on the session): an initiator
                // that declared "the source will be deleted after this
                // transfer" (`blit move`) must NOT get a success out of
                // an incomplete source scan — files the scan could not
                // read would be silently lost when the caller deletes
                // the source. Same abort point as the mirror guard.
                if negotiated.open.require_complete_scan && !complete.scan_complete {
                    return Err(eyre::Report::new(SessionFault::refusal(
                        session_error::Code::ScanIncomplete,
                        "transfer refused: the source scan did not complete \
                         (unreadable paths) and the operation requires a \
                         complete scan (move deletes the source afterwards)",
                    )));
                }
                let chunk = std::mem::take(&mut pending);
                if let Some(la) = &local_apply {
                    diff_chunk_and_apply_local(
                        la,
                        &mut local_run,
                        chunk,
                        dst_root,
                        canonical_dst_root.as_deref(),
                        &compare_opts,
                        &mut granted,
                        &mut needed_paths,
                        progress.as_ref(),
                    )
                    .await?;
                } else {
                    diff_chunk_and_send_needs(
                        transport,
                        chunk,
                        dst_root,
                        canonical_dst_root.as_deref(),
                        &compare_opts,
                        resume_enabled,
                        resume_block_size,
                        &resume_headers,
                        &mut granted,
                        &outstanding,
                        &mut needed_paths,
                        progress.as_ref(),
                        phase_trace.as_ref(),
                        &mut need_batch_seq,
                    )
                    .await?;
                }
                // NeedComplete only after ManifestComplete received
                // AND every entry diffed — both true here.
                transport
                    .send(frame(Frame::NeedComplete(NeedComplete {})))
                    .await?;
                if let Some(trace) = &phase_trace {
                    trace.event("need_complete_sent", SessionPhaseFields::default());
                }
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
                // A resume-flagged grant may be satisfied ONLY by its
                // block record — a whole-file record for it bypasses the
                // hash choreography this end committed to (codex F3).
                if resume_headers
                    .lock()
                    .expect("resume-headers lock poisoned")
                    .contains_key(&header.relative_path)
                {
                    return Err(violation(format!(
                        "file record for resume-flagged '{}' — the contract requires \
                         its block record",
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
                let outcome = receive_file_record(transport, sink.as_ref(), &header).await?;
                files_written += outcome.files_written as u64;
                bytes_written += outcome.bytes_written;
                // otp-10b-2: in-stream per-file progress, same convention
                // as the data-plane receive (`execute_receive_pipeline`):
                // bytes ride Payload, FileComplete is byteless.
                if let Some(p) = &progress {
                    p.report_payload(0, outcome.bytes_written);
                    p.report_file_complete(header.relative_path.clone());
                }
            }
            Some(Frame::Block(block)) => {
                // otp-7a: a resume block record opens with its first
                // BlockTransfer (no begin frame). Claim the need and run
                // the strictly-serialized record to its completion frame.
                let header = claim_resume_record(
                    &block.relative_path,
                    resume_enabled,
                    data_plane_recv.is_some(),
                    manifest_complete,
                    &resume_headers,
                    &outstanding,
                )?;
                let outcome =
                    receive_block_record(transport, sink.as_ref(), &header, block).await?;
                files_written += outcome.files_written as u64;
                bytes_written += outcome.bytes_written;
                files_resumed.fetch_add(1, Ordering::Relaxed);
                // The whole block record (patch bytes + completion) ran
                // to its completion frame — one resumed file done.
                if let Some(p) = &progress {
                    p.report_payload(0, outcome.bytes_written);
                    p.report_file_complete(header.relative_path.clone());
                }
            }
            Some(Frame::BlockComplete(complete)) => {
                // otp-7a: a zero-block record — every block matched
                // (identical content, e.g. an mtime-only touch), so the
                // completion frame arrives with no blocks before it and
                // finalization stamps size/mtime/perms.
                let header = claim_resume_record(
                    &complete.relative_path,
                    resume_enabled,
                    data_plane_recv.is_some(),
                    manifest_complete,
                    &resume_headers,
                    &outstanding,
                )?;
                let outcome = finish_block_record(sink.as_ref(), &header, &complete).await?;
                files_written += outcome.files_written as u64;
                bytes_written += outcome.bytes_written;
                files_resumed.fetch_add(1, Ordering::Relaxed);
                // Zero-block record: nothing transferred, the file is
                // complete (identical content, metadata stamped).
                if let Some(p) = &progress {
                    p.report_file_complete(header.relative_path.clone());
                }
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
                // Same rule as file records (codex F3): a resume-flagged
                // grant may not be satisfied through a tar shard.
                {
                    let held = resume_headers.lock().expect("resume-headers lock poisoned");
                    for h in &shard.files {
                        if held.contains_key(&h.relative_path) {
                            return Err(violation(format!(
                                "tar shard entry for resume-flagged '{}' — the contract \
                                 requires its block record",
                                h.relative_path
                            )));
                        }
                    }
                }
                if let Some(probe) = &small_file_probe {
                    let wait_started = probe.start();
                    let mut out = outstanding.lock().expect("outstanding-needs lock poisoned");
                    let wait = wait_started.elapsed();
                    let hold_started = probe.start();
                    let mut removed = 0usize;
                    for h in &shard.files {
                        if !out.remove(&h.relative_path) {
                            return Err(violation(format!(
                                "tar shard entry '{}' which is not on the need list",
                                h.relative_path
                            )));
                        }
                        removed += 1;
                    }
                    drop(out);
                    let hold = hold_started.elapsed();
                    probe.note_claim(
                        SmallFileCarrier::InStream,
                        shard.files.len(),
                        1,
                        removed,
                        wait,
                        hold,
                    );
                } else {
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
                // Capture member paths for the per-file progress lane
                // before the record consumes the shard (the data-plane
                // receive does the same); skip the allocation when no one
                // is listening.
                let member_paths: Option<Vec<String>> = progress.as_ref().map(|_| {
                    shard
                        .files
                        .iter()
                        .map(|h| h.relative_path.clone())
                        .collect()
                });
                let outcome =
                    receive_tar_record(transport, sink.as_ref(), shard, small_file_probe.as_ref())
                        .await?;
                files_written += outcome.files_written as u64;
                bytes_written += outcome.bytes_written;
                if let Some(p) = &progress {
                    p.report_payload(0, outcome.bytes_written);
                    for path in member_paths.unwrap_or_default() {
                        p.report_file_complete(path);
                    }
                }
            }
            Some(Frame::Resize(resize)) => {
                let state = resize_state.as_mut().ok_or_else(|| {
                    violation("DataPlaneResize on a session with no data plane".into())
                })?;
                if let Some(trace) = &phase_trace {
                    trace.event(
                        "resize_received",
                        SessionPhaseFields {
                            epoch: Some(resize.epoch),
                            target_streams: Some(resize.target_stream_count),
                            live_streams: Some(state.live_streams as u32),
                            ..Default::default()
                        },
                    );
                }
                let decision = state.classify(&resize)?;
                let (ack, action) = match decision {
                    DestinationResizeDecision::Replay(ack) => (ack, "replay"),
                    DestinationResizeDecision::Refuse => {
                        (state.settle(resize, false), "bound_refused")
                    }
                    DestinationResizeDecision::Apply(DataPlaneResizeOp::Remove) => {
                        // The SOURCE retires the logical worker and emits that
                        // stream's ordinary END. The DESTINATION closes no
                        // socket here; its receive worker drains END normally.
                        (state.settle(resize, true), "logical_remove")
                    }
                    DestinationResizeDecision::Apply(DataPlaneResizeOp::Add) => {
                        // ADD is the sole role-specific transport step. A
                        // responder arms an authenticated accept; an initiator
                        // dials and starts its receive worker before ACK. A
                        // preparation failure is a refusal because acceptance
                        // has not yet crossed the wire.
                        let accepted = match data_plane_recv
                            .as_mut()
                            .expect("resize state exists only with a data plane")
                        {
                            data_plane::DestRecvPlane::Responder(run) => {
                                if let Some(trace) = &phase_trace {
                                    trace.event(
                                        "resize_arm_queue_begin",
                                        SessionPhaseFields {
                                            epoch: Some(resize.epoch),
                                            target_streams: Some(resize.target_stream_count),
                                            ..Default::default()
                                        },
                                    );
                                }
                                run.arm(resize.epoch, resize.sub_token.clone())
                            }
                            data_plane::DestRecvPlane::Initiator(run) => {
                                match run.add_dialed_stream(resize.epoch, &resize.sub_token).await {
                                    Ok(()) => true,
                                    Err(err) => {
                                        if instruments.trace_data_plane {
                                            eprintln!(
                                                "[data-plane-client] refusing resize epoch {}: {err:#}",
                                                resize.epoch
                                            );
                                        }
                                        false
                                    }
                                }
                            }
                        };
                        if let Some(trace) = &phase_trace {
                            trace.event(
                                "destination_prepared",
                                SessionPhaseFields {
                                    action: Some(if accepted {
                                        "add_prepared"
                                    } else {
                                        "add_refused"
                                    }),
                                    epoch: Some(resize.epoch),
                                    target_streams: Some(resize.target_stream_count),
                                    accepted: Some(accepted),
                                    ..Default::default()
                                },
                            );
                        }
                        (state.settle(resize, accepted), "add")
                    }
                    DestinationResizeDecision::Apply(DataPlaneResizeOp::Unspecified) => {
                        unreachable!("classify rejects unspecified resize operations")
                    }
                };
                if let Some(trace) = &phase_trace {
                    trace.event(
                        "resize_ack_send_begin",
                        SessionPhaseFields {
                            action: Some(action),
                            epoch: Some(ack.epoch),
                            live_streams: Some(ack.effective_stream_count),
                            accepted: Some(ack.accepted),
                            ..Default::default()
                        },
                    );
                }
                transport.send(frame(Frame::ResizeAck(ack))).await?;
                if let Some(trace) = &phase_trace {
                    trace.event(
                        "resize_ack_sent",
                        SessionPhaseFields {
                            action: Some(action),
                            epoch: Some(ack.epoch),
                            live_streams: Some(ack.effective_stream_count),
                            accepted: Some(ack.accepted),
                            ..Default::default()
                        },
                    );
                }
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
                // `finish()` drops the arm sender (no more ADD sockets) and
                // joins every receive worker. The shared resize state below,
                // not cumulative socket completions, reports final logical
                // membership.
                //
                // otp-11: the LOCAL carrier joins its apply pipeline with
                // the same discipline (drain every write, surface its
                // error) and takes the write totals as this end's
                // counters — the scorer stays the destination.
                if let Some(run) = local_run.take() {
                    let totals = run.finish().await?;
                    files_written = totals.files_written as u64;
                    bytes_written = totals.bytes_written;
                }
                // R46-F2 on the local carrier (codex otp-11a F4): the
                // scan-complete guard fired at ManifestComplete, but the
                // local apply's availability checks can record
                // unreadables AFTER it (a file vanishing or losing
                // permissions between enumeration and apply). The old
                // engine refused mirror deletions on ANY unreadable
                // entry; carry that exact posture — checked here, after
                // the apply pipeline joined, before any deletion.
                if mirror_enabled {
                    if let Some(la) = &local_apply {
                        let unreadable_count = la.unreadable.lock().map(|g| g.len()).unwrap_or(0);
                        if unreadable_count != 0 {
                            return Err(eyre::Report::new(SessionFault::internal(format!(
                                "mirror refused: {unreadable_count} source entr{} could \
                                 not be read during the transfer — deleting now could \
                                 remove files the source still has",
                                if unreadable_count == 1 { "y" } else { "ies" }
                            ))));
                        }
                    }
                }
                let final_logical_streams = resize_state.as_ref().map(|state| state.live_streams);
                let peak_logical_streams = resize_state.as_ref().map(|state| state.peak_streams);
                let receiver_ceiling = resize_state.as_ref().map(|state| state.ceiling);
                let (in_stream_carrier_used, data_plane_streams) = match data_plane_recv.as_mut() {
                    Some(run) => {
                        let totals = run.finish().await?;
                        if let Some(trace) = &phase_trace {
                            trace.event(
                                "data_plane_complete",
                                SessionPhaseFields {
                                    live_streams: final_logical_streams.map(|value| value as u32),
                                    receiver_ceiling: receiver_ceiling.map(|value| value as u32),
                                    peak_streams: peak_logical_streams.map(|value| value as u32),
                                    ..Default::default()
                                },
                            );
                        }
                        files_written = totals.outcome.files_written as u64;
                        bytes_written = totals.outcome.bytes_written;
                        debug_assert!(
                            final_logical_streams.is_some(),
                            "receive data plane must have logical resize state"
                        );
                        data_plane_recv = None;
                        (false, final_logical_streams)
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
                // Belt-and-braces (codex F3): with the per-record claims
                // above (in-stream inline, data-plane in NeedListSink),
                // an empty outstanding set implies every resume grant
                // completed as a block record — but verify the stronger
                // invariant directly rather than infer it. The data
                // plane's finish() above drained every receive worker,
                // so all socket-side claims have landed.
                let unresumed = resume_headers
                    .lock()
                    .expect("resume-headers lock poisoned")
                    .len();
                if unresumed != 0 {
                    return Err(violation(format!(
                        "SourceDone with {unresumed} resume grant(s) never completed by a block record"
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
                    // otp-11: `--dry-run` (local carrier only) plans the
                    // pass without deleting; every wire session executes.
                    let execute = local_apply.as_ref().is_none_or(|la| !la.dry_run);
                    // codex otp-9b F2: if THIS future is dropped while the
                    // blocking pass runs (client disconnect, CancelJob),
                    // the guard's Drop flips the abort flag and the pass
                    // stops deleting instead of running to completion
                    // behind a cancelled job. (A completed await drops the
                    // guard too — harmless, the task is already done.)
                    let abort = Arc::new(AtomicBool::new(false));
                    let _abort_guard = AbortFlagOnDrop(Arc::clone(&abort));
                    let mut pass = tokio::task::spawn_blocking(move || {
                        mirror_delete_pass(
                            &dst,
                            &files,
                            &filter,
                            tolerate_nonempty,
                            canonical.as_deref(),
                            &abort,
                            execute,
                        )
                    });
                    // codex otp-10b-2 F1: a PEER fault mid-purge (a
                    // CancelJob on the serving source, a source-side
                    // abort) arrives as a control frame — a bare await
                    // here would leave it unread while deletions run to
                    // completion behind a cancelled session. Race ONE
                    // control-lane read against the pass (biased to the
                    // frame, so an already-queued cancel aborts the pass
                    // before its first delete); on any lane event, flip
                    // the abort flag, let the pass wind down at its next
                    // op, and surface the peer's fault instead of the
                    // aborted pass's error.
                    let mut peer_fault: Option<eyre::Report> = None;
                    let joined = tokio::select! {
                        biased;
                        received = transport.recv() => {
                            _abort_guard.0.store(true, Ordering::Release);
                            peer_fault = Some(match received {
                                Ok(Some(TransferFrame {
                                    frame: Some(Frame::Error(err)),
                                })) => eyre::Report::new(SessionFault::from_wire(err)),
                                Ok(Some(other)) => violation(format!(
                                    "unexpected {} during the mirror delete pass",
                                    frame_name(&other.frame)
                                )),
                                Ok(None) => eyre::Report::new(SessionFault::internal(
                                    "peer closed mid-session",
                                )),
                                Err(err) => err,
                            });
                            (&mut pass).await
                        }
                        joined = &mut pass => joined,
                    };
                    let pass_result = joined.map_err(|e| {
                        eyre::Report::new(SessionFault::internal(format!(
                            "mirror delete task panicked: {e}"
                        )))
                    })?;
                    if let Some(fault) = peer_fault {
                        // The peer's fault owns the outcome; the aborted
                        // pass's own "aborted" error is its consequence.
                        return Err(fault);
                    }
                    let (deleted_file_count, deleted_dir_count) = pass_result.map_err(|e| {
                        eyre::Report::new(SessionFault::internal(format!(
                            "mirror delete failed: {e:#}"
                        )))
                    })?;
                    // otp-11: the local summary reports the split; the
                    // wire summary keeps the one entries_deleted count.
                    if let Some(la) = &local_apply {
                        la.stats
                            .deleted_files
                            .store(deleted_file_count, Ordering::Relaxed);
                        la.stats
                            .deleted_dirs
                            .store(deleted_dir_count, Ordering::Relaxed);
                    }
                    deleted_file_count + deleted_dir_count
                } else {
                    0
                };
                let summary = TransferSummary {
                    files_transferred: files_written,
                    bytes_transferred: bytes_written,
                    entries_deleted,
                    in_stream_carrier_used,
                    files_resumed: files_resumed.load(Ordering::Relaxed),
                };
                if let Some(trace) = &phase_trace {
                    trace.event("summary_send_begin", SessionPhaseFields::default());
                }
                transport.send(frame(Frame::Summary(summary))).await?;
                if let Some(trace) = &phase_trace {
                    trace.event("summary_sent", SessionPhaseFields::default());
                }
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
                // destination-lane frames echoed back (a ResizeAck or
                // BlockHashList the destination would never receive),
                // stray handshake frames, bare FileData/TarShardChunk
                // outside a record. Fail fast, no tolerant parsing.
                return Err(violation(format!(
                    "{} not valid on the destination's receive lane in this phase",
                    frame_name(&other)
                )));
            }
        }
    }
    }
    .await;

    if result.is_err() {
        let data_plane_was_live = data_plane_recv.is_some();
        if let Some(run) = data_plane_recv.as_mut() {
            run.abort_and_join().await;
        }
        if data_plane_was_live {
            if let (Some(trace), Some(state)) = (&phase_trace, &resize_state) {
                trace.event(
                    "data_plane_aborted",
                    SessionPhaseFields {
                        live_streams: Some(state.live_streams as u32),
                        receiver_ceiling: Some(state.ceiling as u32),
                        peak_streams: Some(state.peak_streams as u32),
                        ..Default::default()
                    },
                );
            }
        }
    }
    result
}

/// The LOCAL carrier's twin of [`diff_chunk_and_send_needs`] (otp-11):
/// identical per-entry verdicts (the same [`destination_needs`] compare,
/// the same `granted` dedup, the same `needed_paths` record), but the
/// needed headers are planned into payloads and queued onto the
/// in-process apply pipeline instead of being granted to the source —
/// no frame is sent and nothing enters `outstanding`. Resume is
/// sink-level on the local carrier (`FsSinkConfig.resume`), so no need
/// is ever resume-flagged here.
#[allow(clippy::too_many_arguments)]
async fn diff_chunk_and_apply_local(
    local: &local::LocalApply,
    run: &mut Option<local::LocalApplyRun>,
    chunk: Vec<FileHeader>,
    dst_root: &Path,
    canonical_dst_root: Option<&Path>,
    compare_opts: &CompareOptions,
    granted: &mut HashSet<String>,
    needed_paths: &mut Vec<String>,
    progress: Option<&RemoteTransferProgress>,
) -> Result<()> {
    if chunk.is_empty() {
        return Ok(());
    }
    // Scanned workload (post-filter, pre-diff) — the summary's
    // scanned_files/scanned_bytes, folded where every manifest entry
    // passes through.
    local
        .stats
        .scanned_files
        .fetch_add(chunk.len() as u64, Ordering::Relaxed);
    local
        .stats
        .scanned_bytes
        .fetch_add(chunk.iter().map(|h| h.size).sum::<u64>(), Ordering::Relaxed);

    // ONE diff core, both carriers (codex otp-11a F1): only the
    // dispatch differs — the wire twin grants these to the source,
    // this one plans and applies them in-process. The resume flag is
    // meaningless here (the local carrier's block phase is
    // sink-level).
    let needed = diff_chunk_verdicts(chunk, dst_root, canonical_dst_root, compare_opts).await?;

    let fresh: Vec<FileHeader> = needed
        .into_iter()
        .map(|(header, _)| header)
        .filter(|header| granted.insert(header.relative_path.clone()))
        .collect();
    if fresh.is_empty() {
        return Ok(());
    }
    for header in &fresh {
        needed_paths.push(header.relative_path.clone());
    }
    if let Some(p) = progress {
        p.report_manifest_batch(fresh.len());
    }
    let payloads = local.plan_chunk(fresh).await?;
    for payload in payloads {
        let queued = match run.as_ref() {
            Some(r) => r.queue(payload).await.is_ok(),
            None => false,
        };
        if !queued {
            // The pipeline died mid-run: surface ITS error as the root
            // cause — a bare "stopped early" would hide the write
            // failure that killed it.
            if let Some(r) = run.take() {
                return Err(r
                    .finish()
                    .await
                    .err()
                    .unwrap_or_else(|| eyre::eyre!("local apply pipeline stopped early")));
            }
            return Err(eyre::eyre!("local apply pipeline stopped early"));
        }
    }
    Ok(())
}

/// Stat-and-compare one chunk of manifest entries on the blocking
/// pool (2+ syscalls per entry — same rationale as the daemon's
/// w4-4 chunked checks), then stream the resulting need batch, followed
/// by a `BlockHashList` for each resume-flagged entry in it (otp-7a).
#[allow(clippy::too_many_arguments)]
async fn diff_chunk_and_send_needs(
    transport: &mut FrameTransport,
    chunk: Vec<FileHeader>,
    dst_root: &Path,
    canonical_dst_root: Option<&Path>,
    compare_opts: &CompareOptions,
    resume_enabled: bool,
    resume_block_size: usize,
    // Headers of resume-granted needs, retained for record finalization
    // (shared with the data-plane receive, otp-7b).
    resume_headers: &data_plane::ResumeHeaders,
    // Ever-granted DEDUP set (control-loop-local, insert-only): a path
    // the source manifests twice is granted at most once, and because it
    // is never removed, a concurrent data-plane claim can't re-open the
    // grant (fix-review F1).
    granted: &mut HashSet<String>,
    // Not-yet-delivered COMPLETION set (shared with the receive).
    outstanding: &data_plane::OutstandingNeeds,
    needed_paths: &mut Vec<String>,
    // otp-10b-2: w6-1 denominator — each NeedBatch sent reports a
    // ManifestBatch (files this DESTINATION requested), mirroring what
    // the push SOURCE reports per NeedBatch received.
    progress: Option<&RemoteTransferProgress>,
    phase_trace: Option<&BoundSessionPhaseTrace>,
    need_batch_seq: &mut u64,
) -> Result<()> {
    if chunk.is_empty() {
        return Ok(());
    }
    // ONE diff core, both carriers (codex otp-11a F1); plan D2: a need
    // is resume-flagged only when the session negotiated resume AND a
    // non-empty dest partial exists to diff against.
    let needed: Vec<(FileHeader, bool)> =
        diff_chunk_verdicts(chunk, dst_root, canonical_dst_root, compare_opts)
            .await?
            .into_iter()
            .map(|(header, resume_eligible)| (header, resume_enabled && resume_eligible))
            .collect();

    // Dedup on the ever-granted set (no lock — control-loop-local), then
    // insert the freshly granted paths into the shared `outstanding`
    // completion set BEFORE the NeedBatch goes out. The source can only
    // send a payload after receiving its need, so insert-before-send
    // orders the data-plane receive's `claim` strictly after this insert.
    let fresh: Vec<(FileHeader, bool)> = needed
        .into_iter()
        .filter(|(header, _)| granted.insert(header.relative_path.clone()))
        .collect();
    let entries: Vec<NeedEntry> = {
        let mut out = outstanding.lock().expect("outstanding-needs lock poisoned");
        fresh
            .iter()
            .map(|(header, resume)| {
                needed_paths.push(header.relative_path.clone());
                out.insert(header.relative_path.clone());
                NeedEntry {
                    relative_path: header.relative_path.clone(),
                    resume: *resume,
                }
            })
            .collect()
    };
    if entries.is_empty() {
        return Ok(());
    }
    if let Some(p) = progress {
        p.report_manifest_batch(entries.len());
    }
    let batch = *need_batch_seq;
    let count = entries.len() as u64;
    if let Some(trace) = phase_trace {
        trace.event(
            "need_batch_send_begin",
            SessionPhaseFields {
                batch: Some(batch),
                count: Some(count),
                ..Default::default()
            },
        );
    }
    transport
        .send(frame(Frame::NeedBatch(NeedBatch { entries })))
        .await?;
    if let Some(trace) = phase_trace {
        trace.event(
            "need_batch_sent",
            SessionPhaseFields {
                batch: Some(batch),
                count: Some(count),
                ..Default::default()
            },
        );
    }
    *need_batch_seq += 1;
    // otp-7a: each resume-flagged grant's hash list follows its batch on
    // the same ordered lane — the source HOLDS the need until the list
    // arrives, and ordered delivery guarantees every list precedes this
    // end's eventual NeedComplete.
    for (header, resume) in &fresh {
        if !*resume {
            continue;
        }
        let hashes = compute_resume_block_hashes(
            dst_root,
            canonical_dst_root,
            &header.relative_path,
            resume_block_size,
        )
        .await?;
        // Retain the grant BEFORE the hash list goes out (otp-7b): the
        // data-plane receive validates arriving block records against
        // this map on another task, so insert-before-send is what
        // orders its lookup strictly after the grant exists — the same
        // rule `outstanding` follows above.
        resume_headers
            .lock()
            .expect("resume-headers lock poisoned")
            .insert(header.relative_path.clone(), header.clone());
        transport
            .send(frame(Frame::BlockHashes(BlockHashList {
                relative_path: header.relative_path.clone(),
                block_size: resume_block_size as u32,
                hashes,
            })))
            .await?;
    }
    Ok(())
}

/// Stat-and-compare one manifest chunk on the blocking pool (2+
/// syscalls per entry — the daemon's w4-4 chunked-check rationale),
/// abortable when the session dies: under Checksum compare this chunk
/// hashes up to DEST_DIFF_CHUNK files (codex otp-10b-1 F3), so the
/// guard's Drop flips the flag, the loop checks it per entry, and the
/// hasher per 64 KiB chunk. The ONE diff core for both carriers
/// (codex otp-11a F1): `diff_chunk_and_send_needs` grants the result
/// to the source over the wire; `diff_chunk_and_apply_local` plans and
/// applies it in-process. Returns `(header, resume_eligible)` per
/// entry that must transfer.
async fn diff_chunk_verdicts(
    chunk: Vec<FileHeader>,
    dst_root: &Path,
    canonical_dst_root: Option<&Path>,
    compare_opts: &CompareOptions,
) -> Result<Vec<(FileHeader, bool)>> {
    let dst_root_owned = dst_root.to_path_buf();
    let canonical = canonical_dst_root.map(Path::to_path_buf);
    let opts = compare_opts.clone();
    let abort = Arc::new(AtomicBool::new(false));
    let _abort_guard = AbortFlagOnDrop(Arc::clone(&abort));
    tokio::task::spawn_blocking(move || -> Result<Vec<(FileHeader, bool)>> {
        let mut needed = Vec::new();
        for header in chunk {
            if abort.load(Ordering::Acquire) {
                eyre::bail!("destination diff aborted: session ended");
            }
            match destination_needs(
                &header,
                &dst_root_owned,
                canonical.as_deref(),
                &opts,
                &abort,
            )? {
                NeedVerdict::Skip => {}
                NeedVerdict::Transfer { resume_eligible } => {
                    needed.push((header, resume_eligible));
                }
            }
        }
        Ok(needed)
    })
    .await
    .map_err(|err| eyre::eyre!("destination diff task panicked: {err}"))?
}

/// The destination diff's per-entry verdict (otp-7a widened it from a
/// bool): does this entry transfer, and if so is it eligible for the
/// resume block phase — plan D2: the file exists at the dest as a
/// non-empty regular file (so there is a partial to hash) AND the
/// compare says it must transfer. Absent/empty/non-file targets are
/// plain full transfers. Session gating (ResumeSettings.enabled) is the
/// caller's, not this verdict's.
enum NeedVerdict {
    Skip,
    Transfer { resume_eligible: bool },
}

/// Does the destination need this manifest entry? Stats its own file
/// and delegates the verdict to `manifest::header_transfer_status` —
/// the one mode-aware compare owner - fed from a live stat instead
/// of a materialized target manifest.
fn destination_needs(
    header: &FileHeader,
    dst_root: &Path,
    canonical_dst_root: Option<&Path>,
    opts: &CompareOptions,
    abort: &AtomicBool,
) -> Result<NeedVerdict> {
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
    // otp-10b-1: a Checksum session hashes the local candidate so a
    // content-equal file SKIPS regardless of mtime (the old pull's
    // `--checksum` behavior, now role-agnostic). Only the same-size
    // case needs the hash — a size mismatch is already Modified — and
    // a hash failure degrades to the empty checksum, whose
    // `compare_file` arm conservatively transfers. This runs inside
    // the diff's blocking-pool chunk (same rationale as the resume
    // block hashing), so the hash never blocks the async loop; the
    // abort flag bounds it when the session dies (codex F3).
    let target_hash: Vec<u8> = match target {
        Some((size, _)) if opts.mode == CompareMode::Checksum && size == header.size => {
            match hash_file_abortable(&dst, abort) {
                Ok(hash) => hash,
                Err(_) if abort.load(Ordering::Acquire) => {
                    eyre::bail!("destination diff aborted: session ended")
                }
                Err(_) => Vec::new(),
            }
        }
        _ => Vec::new(),
    };
    let status = header_transfer_status(
        header,
        target.map(|(size, mtime)| (size, mtime, target_hash.as_slice())),
        opts,
    );
    Ok(match status {
        // Modified ⇒ a regular file exists at the dest (`target` was
        // Some); it is resume-eligible when non-empty (plan D2 — an
        // empty partial has nothing to hash, full transfer is strictly
        // simpler and byte-equivalent).
        FileStatus::Modified => NeedVerdict::Transfer {
            resume_eligible: target.is_some_and(|(size, _)| size > 0),
        },
        FileStatus::New => NeedVerdict::Transfer {
            resume_eligible: false,
        },
        _ => NeedVerdict::Skip,
    })
}

/// Blake3 of one whole local file, abortable between 64 KiB chunks —
/// the destination diff's Checksum-mode hasher (codex otp-10b-1 F3:
/// `checksum::hash_file` runs a whole file uninterruptibly; inside the
/// diff's blocking chunk that must yield to a dead session's abort
/// flag within one chunk).
fn hash_file_abortable(path: &Path, abort: &AtomicBool) -> Result<Vec<u8>> {
    use std::io::Read;
    let mut file = std::fs::File::open(path)?;
    let mut hasher = blake3::Hasher::new();
    let mut buf = vec![0u8; 64 * 1024];
    loop {
        if abort.load(Ordering::Acquire) {
            eyre::bail!("hash aborted: session ended");
        }
        let got = file.read(&mut buf)?;
        if got == 0 {
            break;
        }
        hasher.update(&buf[..got]);
    }
    Ok(hasher.finalize().as_bytes().to_vec())
}

/// otp-7a: hash the destination's existing partial for one
/// resume-flagged grant — Blake3 per `block_size` block, in order (the
/// wire shape of `BlockHashList.hashes`). Pure blocking FS work, so it
/// runs on the blocking pool (same rationale as the diff chunks). A file
/// that vanished (or emptied) between the diff and this read yields an
/// empty/short list — the implicit full-transfer fallback (plan D1): the
/// source sends every block a hash does not vouch for.
async fn compute_resume_block_hashes(
    dst_root: &Path,
    canonical_dst_root: Option<&Path>,
    relative_path: &str,
    block_size: usize,
) -> Result<Vec<Vec<u8>>> {
    let dst = match canonical_dst_root {
        Some(canonical) => {
            crate::path_safety::safe_join_contained(canonical, dst_root, relative_path)
        }
        None => crate::path_safety::safe_join(dst_root, relative_path),
    }
    .map_err(|err| {
        SessionFault::protocol_violation(format!(
            "resume path '{relative_path}' escapes the destination root: {err:#}"
        ))
    })?;
    tokio::task::spawn_blocking(move || -> Result<Vec<Vec<u8>>> {
        use std::io::Read;
        // Re-stat inside the claim: a partial that vanished, stopped
        // being a regular file, or grew past the hash-count cap since
        // the diff yields the empty list — the full-transfer fallback
        // (D1) — never an error and never an oversized frame (codex F1).
        match std::fs::metadata(&dst) {
            Ok(meta) if meta.is_file() && resume_hash_list_fits(meta.len(), block_size) => {}
            Ok(_) => return Ok(Vec::new()),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
            Err(e) => return Err(eyre::eyre!("stat {} for block hashes: {e}", dst.display())),
        }
        let mut file = match std::fs::File::open(&dst) {
            Ok(f) => f,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
            Err(e) => {
                return Err(eyre::eyre!(
                    "opening {} for block hashes: {e}",
                    dst.display()
                ))
            }
        };
        let mut hashes = Vec::new();
        let mut buf = vec![0u8; block_size];
        loop {
            let mut filled = 0usize;
            while filled < block_size {
                let got = file
                    .read(&mut buf[filled..])
                    .map_err(|e| eyre::eyre!("hashing {}: {e}", dst.display()))?;
                if got == 0 {
                    break;
                }
                filled += got;
            }
            if filled == 0 {
                break;
            }
            hashes.push(blake3::hash(&buf[..filled]).as_bytes().to_vec());
            // A file growing concurrently with this read could blow past
            // the stat-time cap check — degrade, don't overshoot.
            if hashes.len() as u64 > MAX_RESUME_BLOCK_HASHES {
                return Ok(Vec::new());
            }
            if filled < block_size {
                break;
            }
        }
        Ok(hashes)
    })
    .await
    .map_err(|err| eyre::eyre!("resume hash task panicked: {err}"))?
}

/// otp-7a: validate and claim the resume-flagged need an IN-STREAM
/// block record opens for. Fail-fast on every off-contract shape:
/// control-lane block records are valid only when no data plane is
/// active (with one, blocks ride the sockets and `NeedListSink` claims
/// them — otp-7b), only after ManifestComplete, only in a resume
/// session, and only for a path this end granted with `resume=true` —
/// exactly once (the header map and the outstanding set are both
/// claimed here).
fn claim_resume_record(
    relative_path: &str,
    resume_enabled: bool,
    data_plane_active: bool,
    manifest_complete: bool,
    resume_headers: &data_plane::ResumeHeaders,
    outstanding: &data_plane::OutstandingNeeds,
) -> Result<FileHeader> {
    if !resume_enabled {
        return Err(violation_for(
            relative_path,
            format!("block record for '{relative_path}' in a session opened without resume"),
        ));
    }
    if data_plane_active {
        return Err(violation_for(
            relative_path,
            format!(
                "block record for '{relative_path}' on the control lane while a TCP data plane is active"
            ),
        ));
    }
    if !manifest_complete {
        return Err(violation_for(
            relative_path,
            format!("block record for '{relative_path}' before ManifestComplete"),
        ));
    }
    let header = resume_headers
        .lock()
        .expect("resume-headers lock poisoned")
        .remove(relative_path)
        .ok_or_else(|| {
            violation_for(
                relative_path,
                format!(
                    "block record for '{relative_path}' which was not granted a resume-flagged need"
                ),
            )
        })?;
    if !outstanding
        .lock()
        .expect("outstanding-needs lock poisoned")
        .remove(relative_path)
    {
        return Err(violation_for(
            relative_path,
            format!("block record for '{relative_path}' which is not on the need list"),
        ));
    }
    Ok(header)
}

/// otp-7a: receive one strictly-serialized resume block record whose
/// first `BlockTransfer` is already in hand: apply each block in place
/// through the sink, until the record's `BlockTransferComplete`. Nothing
/// may interleave with the open record on the source lane — same rule as
/// file records.
async fn receive_block_record(
    transport: &mut FrameTransport,
    sink: &dyn TransferSink,
    header: &FileHeader,
    first: BlockTransfer,
) -> Result<crate::remote::transfer::SinkOutcome> {
    let mut bytes_written: u64 = 0;
    let mut block = first;
    loop {
        let len = block.content.len() as u64;
        if block.offset.saturating_add(len) > header.size {
            return Err(violation_for(
                &header.relative_path,
                format!(
                    "block record '{}' overran its size: offset {} + {} byte(s) > {}",
                    header.relative_path, block.offset, len, header.size
                ),
            ));
        }
        let outcome = sink
            .write_payload(PreparedPayload::FileBlock {
                relative_path: header.relative_path.clone(),
                offset: block.offset,
                bytes: block.content,
            })
            .await
            .map_err(|e| tag_path(e, &header.relative_path))?;
        bytes_written += outcome.bytes_written;
        // codex 7b-2 G3: a transport break inside the record names the
        // file the record already identified.
        let received = match transport
            .recv()
            .await
            .map_err(|e| tag_path(e, &header.relative_path))?
        {
            Some(f) => f,
            None => {
                return Err(eyre::Report::new(
                    SessionFault::internal(format!(
                        "peer closed inside block record '{}'",
                        header.relative_path
                    ))
                    .with_path(header.relative_path.as_str()),
                ))
            }
        };
        match received.frame {
            Some(Frame::Block(next)) if next.relative_path == header.relative_path => {
                block = next;
            }
            Some(Frame::BlockComplete(complete))
                if complete.relative_path == header.relative_path =>
            {
                let outcome = finish_block_record(sink, header, &complete).await?;
                return Ok(crate::remote::transfer::SinkOutcome {
                    files_written: outcome.files_written,
                    bytes_written: bytes_written + outcome.bytes_written,
                });
            }
            Some(Frame::Error(err)) => {
                // A mid-record abort (plan D4): the peer says why before
                // closing — surface ITS fault, not a violation about the
                // frame's position.
                return Err(eyre::Report::new(SessionFault::from_wire(err)));
            }
            other => {
                // Strict serialization: nothing may interleave with an
                // open record on the source lane — including a block for
                // a different path.
                return Err(violation_for(
                    &header.relative_path,
                    format!(
                        "{} inside block record '{}'",
                        frame_name(&other),
                        header.relative_path
                    ),
                ));
            }
        }
    }
}

/// otp-7a: finalize one resume block record — truncate to the manifest
/// size and stamp mtime/permissions from the retained manifest header
/// (the wire complete frame carries only `total_bytes`, which must match
/// the size the manifest promised, exactly as a file record must
/// complete at `header.size` bytes).
async fn finish_block_record(
    sink: &dyn TransferSink,
    header: &FileHeader,
    complete: &BlockTransferComplete,
) -> Result<crate::remote::transfer::SinkOutcome> {
    if complete.total_bytes != header.size {
        return Err(violation_for(
            &header.relative_path,
            format!(
                "block record '{}' completed at {} byte(s), manifest promised {}",
                header.relative_path, complete.total_bytes, header.size
            ),
        ));
    }
    sink.write_payload(PreparedPayload::FileBlockComplete {
        relative_path: header.relative_path.clone(),
        total_size: complete.total_bytes,
        mtime_seconds: header.mtime_seconds,
        permissions: header.permissions,
    })
    .await
    .map_err(|e| tag_path(e, &header.relative_path))
}

/// Receive one strictly-serialized file record (`file_begin` already
/// consumed) and stream its bytes into the sink through a bounded
/// in-memory pipe — record completion is exactly `header.size`
/// cumulative bytes (contract §Transport selection).
async fn receive_file_record(
    transport: &mut FrameTransport,
    sink: &dyn TransferSink,
    header: &FileHeader,
) -> Result<crate::remote::transfer::SinkOutcome> {
    let (mut pipe_wr, mut pipe_rd) = tokio::io::duplex(FILE_RECORD_PIPE_BYTES);
    let write = sink.write_file_stream(header, &mut pipe_rd);
    let feed = async {
        let mut remaining = header.size;
        while remaining > 0 {
            // codex 7b-2 G3: a transport break inside the record names
            // the file the record already identified.
            let received = match transport
                .recv()
                .await
                .map_err(|e| tag_path(e, &header.relative_path))?
            {
                Some(f) => f,
                None => {
                    return Err(eyre::Report::new(
                        SessionFault::internal(format!(
                            "peer closed inside file record '{}'",
                            header.relative_path
                        ))
                        .with_path(header.relative_path.as_str()),
                    ))
                }
            };
            match received.frame {
                Some(Frame::FileData(data)) => {
                    let len = data.content.len() as u64;
                    if len > remaining {
                        return Err(violation_for(
                            &header.relative_path,
                            format!(
                                "file record '{}' overran its size by {} byte(s)",
                                header.relative_path,
                                len - remaining
                            ),
                        ));
                    }
                    pipe_wr.write_all(&data.content).await?;
                    remaining -= len;
                }
                Some(Frame::Error(err)) => {
                    // A mid-record abort (plan D4): the peer says why
                    // before closing — surface ITS fault (a CANCELLED
                    // must stay CANCELLED), not a violation about the
                    // frame's position.
                    return Err(eyre::Report::new(SessionFault::from_wire(err)));
                }
                other => {
                    // Strict serialization: nothing else may interleave
                    // with an open record on the source lane.
                    return Err(violation_for(
                        &header.relative_path,
                        format!(
                            "{} inside file record '{}' ({} byte(s) short)",
                            frame_name(&other),
                            header.relative_path,
                            remaining
                        ),
                    ));
                }
            }
        }
        pipe_wr.shutdown().await?;
        Ok(())
    };
    let (outcome, ()) =
        tokio::try_join!(write, feed).map_err(|e| tag_path(e, &header.relative_path))?;
    Ok(outcome)
}

/// Receive one tar-shard record (`tar_shard_header` already consumed):
/// buffer to exactly `archive_size` (bounded by the shared tar cap)
/// and hand the archive to the sink's tar-safety unpack path.
async fn receive_tar_record(
    transport: &mut FrameTransport,
    sink: &dyn TransferSink,
    shard: TarShardHeader,
    small_file_probe: Option<&BoundSmallFileProbe>,
) -> Result<crate::remote::transfer::SinkOutcome> {
    if shard.archive_size > MAX_TAR_SHARD_BYTES {
        return Err(violation(format!(
            "tar shard of {} bytes exceeds the {} byte cap",
            shard.archive_size, MAX_TAR_SHARD_BYTES
        )));
    }
    let members = shard.files.len();
    let archive_bytes = shard.archive_size;
    let receive_started = small_file_probe.map(BoundSmallFileProbe::start);
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
                let decoded = receive_started.map(|_| std::time::Instant::now());
                let shard_id = small_file_probe.map(|probe| probe.shard_id(&shard.files));
                let correlated = receive_started.map(|_| std::time::Instant::now());
                let payload = PreparedPayload::TarShard {
                    headers: shard.files,
                    data,
                };
                let sink_started = receive_started.map(|_| std::time::Instant::now());
                let outcome = sink.write_payload(payload).await?;
                if let (
                    Some(probe),
                    Some(shard_id),
                    Some(started),
                    Some(decoded),
                    Some(correlated),
                    Some(sink_started),
                ) = (
                    small_file_probe,
                    shard_id,
                    receive_started,
                    decoded,
                    correlated,
                    sink_started,
                ) {
                    probe.note_shard_receive(
                        shard_id,
                        SmallFileCarrier::InStream,
                        members,
                        archive_bytes,
                        started,
                        decoded,
                        correlated,
                        sink_started,
                        std::time::Instant::now(),
                    );
                }
                return Ok(outcome);
            }
            Some(Frame::Error(err)) => {
                // Same mid-record abort contract (plan D4) as file and
                // block records: the peer's fault owns the outcome.
                return Err(eyre::Report::new(SessionFault::from_wire(err)));
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

    fn resize_request(
        op: DataPlaneResizeOp,
        epoch: u32,
        target_stream_count: u32,
    ) -> DataPlaneResize {
        DataPlaneResize {
            op: op as i32,
            epoch,
            target_stream_count,
            sub_token: if op == DataPlaneResizeOp::Add {
                vec![7; crate::remote::transfer::SUB_TOKEN_LEN]
            } else {
                Vec::new()
            },
        }
    }

    fn pending_resize(
        op: DataPlaneResizeOp,
        epoch: u32,
        target_streams: u32,
    ) -> data_plane::PendingResize {
        data_plane::PendingResize {
            epoch,
            target_streams,
            op,
            sub_token: (op == DataPlaneResizeOp::Add)
                .then(|| vec![9; crate::remote::transfer::SUB_TOKEN_LEN]),
        }
    }

    #[test]
    fn source_resize_ack_state_replays_exact_and_rejects_inconsistent_frames() {
        let settled = DataPlaneResizeAck {
            epoch: 1,
            effective_stream_count: 5,
            accepted: true,
        };
        let mut replay = SourceResizeState {
            proposals: None,
            pending: None,
            last_ack: Some(settled),
        };
        assert!(replay
            .take_pending_for_ack(&settled)
            .expect("exact duplicate replays")
            .is_none());
        let changed_duplicate = DataPlaneResizeAck {
            effective_stream_count: 4,
            ..settled
        };
        assert!(replay.take_pending_for_ack(&changed_duplicate).is_err());

        let mut pending = SourceResizeState {
            proposals: None,
            pending: Some(pending_resize(DataPlaneResizeOp::Add, 2, 6)),
            last_ack: None,
        };
        let future = DataPlaneResizeAck {
            epoch: 3,
            effective_stream_count: 6,
            accepted: true,
        };
        assert!(pending.take_pending_for_ack(&future).is_err());
        assert_eq!(pending.pending.as_ref().map(|resize| resize.epoch), Some(2));
        let accepted = DataPlaneResizeAck {
            epoch: 2,
            effective_stream_count: 6,
            accepted: true,
        };
        let taken = pending
            .take_pending_for_ack(&accepted)
            .expect("matching epoch")
            .expect("pending resize returned");
        assert_eq!(taken.epoch, 2);
        assert!(pending.pending.is_none());
    }

    #[test]
    fn source_resize_ack_effective_count_matches_acceptance() {
        let pending = pending_resize(DataPlaneResizeOp::Add, 1, 5);
        let accepted = DataPlaneResizeAck {
            epoch: 1,
            effective_stream_count: 5,
            accepted: true,
        };
        assert!(validate_source_resize_ack(&pending, 4, &accepted).is_ok());
        assert!(validate_source_resize_ack(
            &pending,
            4,
            &DataPlaneResizeAck {
                effective_stream_count: 4,
                ..accepted
            }
        )
        .is_err());

        let refused = DataPlaneResizeAck {
            epoch: 1,
            effective_stream_count: 4,
            accepted: false,
        };
        assert!(validate_source_resize_ack(&pending, 4, &refused).is_ok());
        assert!(validate_source_resize_ack(
            &pending,
            4,
            &DataPlaneResizeAck {
                effective_stream_count: 5,
                ..refused
            }
        )
        .is_err());
    }

    #[test]
    fn epoch0_grant_must_equal_receiver_bounded_floor() {
        assert_eq!(data_plane::validate_epoch0_streams(4, None).unwrap(), 4);
        assert!(data_plane::validate_epoch0_streams(1, None).is_err());

        let limited = CapacityProfile {
            max_streams: 2,
            ..Default::default()
        };
        assert_eq!(
            data_plane::validate_epoch0_streams(2, Some(&limited)).unwrap(),
            2
        );
        assert!(data_plane::validate_epoch0_streams(4, Some(&limited)).is_err());

        let unknown = CapacityProfile {
            max_streams: 0,
            ..Default::default()
        };
        assert_eq!(
            data_plane::validate_epoch0_streams(4, Some(&unknown)).unwrap(),
            4
        );
    }

    #[test]
    fn destination_resize_state_applies_add_remove_and_replays_exact_duplicate() {
        let mut state = DestinationResizeState::new(4, 17);
        let add = resize_request(DataPlaneResizeOp::Add, 1, 5);
        assert_eq!(
            state.classify(&add).expect("valid ADD"),
            DestinationResizeDecision::Apply(DataPlaneResizeOp::Add)
        );
        let add_ack = state.settle(add.clone(), true);
        assert_eq!(add_ack.effective_stream_count, 5);
        assert_eq!(state.live_streams, 5);

        assert_eq!(
            state.classify(&add).expect("exact duplicate replays"),
            DestinationResizeDecision::Replay(add_ack)
        );
        assert_eq!(state.live_streams, 5, "duplicate has no second effect");
        assert_eq!(state.settled_epoch, 1);

        let remove = resize_request(DataPlaneResizeOp::Remove, 2, 4);
        assert_eq!(
            state.classify(&remove).expect("valid REMOVE"),
            DestinationResizeDecision::Apply(DataPlaneResizeOp::Remove)
        );
        let remove_ack = state.settle(remove.clone(), true);
        assert_eq!(remove_ack.effective_stream_count, 4);
        assert_eq!(state.live_streams, 4);
        assert_eq!(
            state.classify(&remove).expect("REMOVE duplicate replays"),
            DestinationResizeDecision::Replay(remove_ack)
        );
        assert_eq!(state.live_streams, 4, "REMOVE duplicate retires nothing");
    }

    #[test]
    fn destination_resize_state_rejects_inconsistent_epochs_targets_ops_and_tokens() {
        let mut settled = DestinationResizeState::new(4, 17);
        let first = resize_request(DataPlaneResizeOp::Add, 1, 5);
        settled.settle(first.clone(), true);

        let mut changed_duplicate = first;
        changed_duplicate.sub_token[0] ^= 1;
        assert!(settled.classify(&changed_duplicate).is_err());
        assert!(settled
            .classify(&resize_request(DataPlaneResizeOp::Remove, 0, 4))
            .is_err());

        let fresh = DestinationResizeState::new(4, 17);
        assert!(fresh
            .classify(&resize_request(DataPlaneResizeOp::Add, 0, 5))
            .is_err());
        assert!(fresh
            .classify(&resize_request(DataPlaneResizeOp::Add, 2, 5))
            .is_err());
        assert!(fresh
            .classify(&resize_request(DataPlaneResizeOp::Add, 1, 4))
            .is_err());
        assert!(fresh
            .classify(&resize_request(DataPlaneResizeOp::Add, 1, 6))
            .is_err());
        assert!(fresh
            .classify(&resize_request(DataPlaneResizeOp::Remove, 1, 2))
            .is_err());
        assert!(fresh
            .classify(&resize_request(DataPlaneResizeOp::Remove, 1, 4))
            .is_err());

        let mut bad_add_token = resize_request(DataPlaneResizeOp::Add, 1, 5);
        bad_add_token.sub_token.pop();
        assert!(fresh.classify(&bad_add_token).is_err());
        let mut long_add_token = resize_request(DataPlaneResizeOp::Add, 1, 5);
        long_add_token.sub_token.push(1);
        assert!(fresh.classify(&long_add_token).is_err());
        let mut bad_remove_token = resize_request(DataPlaneResizeOp::Remove, 1, 3);
        bad_remove_token.sub_token.push(1);
        assert!(fresh.classify(&bad_remove_token).is_err());
        let mut unspecified = resize_request(DataPlaneResizeOp::Add, 1, 5);
        unspecified.op = DataPlaneResizeOp::Unspecified as i32;
        assert!(fresh.classify(&unspecified).is_err());
        let mut bad_op = resize_request(DataPlaneResizeOp::Add, 1, 5);
        bad_op.op = 99;
        assert!(fresh.classify(&bad_op).is_err());
    }

    #[test]
    fn destination_resize_refusal_consumes_epoch_and_is_terminal() {
        let mut ceiling = DestinationResizeState::new(4, 4);
        let above = resize_request(DataPlaneResizeOp::Add, 1, 5);
        assert_eq!(
            ceiling.classify(&above).expect("bound is a refusal"),
            DestinationResizeDecision::Refuse
        );
        let refused = ceiling.settle(above.clone(), false);
        assert_eq!(refused.effective_stream_count, 4);
        assert!(!refused.accepted);
        assert_eq!(
            ceiling.classify(&above).expect("refusal duplicate replays"),
            DestinationResizeDecision::Replay(refused)
        );
        assert!(ceiling
            .classify(&resize_request(DataPlaneResizeOp::Remove, 2, 3))
            .is_err());

        let floor = DestinationResizeState::new(1, 17);
        assert_eq!(
            floor
                .classify(&resize_request(DataPlaneResizeOp::Remove, 1, 0))
                .expect("floor is a refusal"),
            DestinationResizeDecision::Refuse
        );
    }

    struct PayloadGate {
        permits: tokio::sync::Semaphore,
        entered: std::sync::atomic::AtomicUsize,
        changed: tokio::sync::Notify,
    }

    impl PayloadGate {
        fn new() -> Arc<Self> {
            Arc::new(Self {
                permits: tokio::sync::Semaphore::new(0),
                entered: std::sync::atomic::AtomicUsize::new(0),
                changed: tokio::sync::Notify::new(),
            })
        }

        async fn wait_for_entered(&self, minimum: usize) {
            loop {
                let changed = self.changed.notified();
                tokio::pin!(changed);
                changed.as_mut().enable();
                if self.entered.load(Ordering::SeqCst) >= minimum {
                    return;
                }
                changed.await;
            }
        }

        fn release(&self, count: usize) {
            self.permits.add_permits(count);
        }
    }

    struct GatedTransferSource {
        inner: FsTransferSource,
        gate: Arc<PayloadGate>,
    }

    #[async_trait::async_trait]
    impl TransferSource for GatedTransferSource {
        fn scan(
            &self,
            filter: Option<crate::fs_enum::FileFilter>,
            unreadable_paths: Arc<StdMutex<Vec<String>>>,
        ) -> (mpsc::Receiver<FileHeader>, SourceScan) {
            self.inner.scan(filter, unreadable_paths)
        }

        async fn prepare_payload(&self, payload: TransferPayload) -> Result<PreparedPayload> {
            self.gate.entered.fetch_add(1, Ordering::SeqCst);
            self.gate.changed.notify_waiters();
            let permit = self
                .gate
                .permits
                .acquire()
                .await
                .map_err(|_| eyre::eyre!("payload test gate closed"))?;
            permit.forget();
            self.inner.prepare_payload(payload).await
        }

        async fn check_availability(
            &self,
            headers: Vec<FileHeader>,
            unreadable_paths: Arc<StdMutex<Vec<String>>>,
        ) -> Result<Vec<FileHeader>> {
            self.inner
                .check_availability(headers, unreadable_paths)
                .await
        }

        async fn open_file(
            &self,
            header: &FileHeader,
        ) -> Result<Box<dyn tokio::io::AsyncRead + Unpin + Send>> {
            self.inner.open_file(header).await
        }

        fn root(&self) -> &Path {
            self.inner.root()
        }
    }

    struct PrepareFaultSource {
        inner: FsTransferSource,
    }

    #[async_trait::async_trait]
    impl TransferSource for PrepareFaultSource {
        fn scan(
            &self,
            _filter: Option<crate::fs_enum::FileFilter>,
            _unreadable_paths: Arc<StdMutex<Vec<String>>>,
        ) -> (mpsc::Receiver<FileHeader>, SourceScan) {
            let (_tx, rx) = mpsc::channel(1);
            let task = tokio::spawn(async { Err(eyre::eyre!("injected TCP source fault")) });
            (rx, SourceScan::new(task))
        }

        async fn prepare_payload(&self, _payload: TransferPayload) -> Result<PreparedPayload> {
            Err(eyre::eyre!("injected TCP source fault"))
        }

        async fn check_availability(
            &self,
            headers: Vec<FileHeader>,
            unreadable_paths: Arc<StdMutex<Vec<String>>>,
        ) -> Result<Vec<FileHeader>> {
            self.inner
                .check_availability(headers, unreadable_paths)
                .await
        }

        async fn open_file(
            &self,
            header: &FileHeader,
        ) -> Result<Box<dyn tokio::io::AsyncRead + Unpin + Send>> {
            self.inner.open_file(header).await
        }

        fn root(&self) -> &Path {
            self.inner.root()
        }
    }

    struct ResizeCaptureTx {
        inner: Box<dyn FrameTx>,
        frames: Arc<StdMutex<Vec<DataPlaneResize>>>,
    }

    #[async_trait::async_trait]
    impl FrameTx for ResizeCaptureTx {
        async fn send(&mut self, frame: TransferFrame) -> Result<()> {
            if let Some(Frame::Resize(resize)) = frame.frame.as_ref() {
                self.frames
                    .lock()
                    .expect("resize capture lock poisoned")
                    .push(resize.clone());
            }
            self.inner.send(frame).await
        }
    }

    struct ResizeAckGate {
        entered: AtomicBool,
        changed: tokio::sync::Notify,
        release: tokio::sync::Semaphore,
        acks: StdMutex<Vec<DataPlaneResizeAck>>,
    }

    impl ResizeAckGate {
        fn new() -> Arc<Self> {
            Arc::new(Self {
                entered: AtomicBool::new(false),
                changed: tokio::sync::Notify::new(),
                release: tokio::sync::Semaphore::new(0),
                acks: StdMutex::new(Vec::new()),
            })
        }

        async fn wait_until_entered(&self) {
            loop {
                let changed = self.changed.notified();
                tokio::pin!(changed);
                changed.as_mut().enable();
                if self.entered.load(Ordering::Acquire) {
                    return;
                }
                changed.await;
            }
        }

        fn release(&self) {
            self.release.add_permits(1);
        }
    }

    struct ResizeAckGateTx {
        inner: Box<dyn FrameTx>,
        gate: Arc<ResizeAckGate>,
    }

    #[async_trait::async_trait]
    impl FrameTx for ResizeAckGateTx {
        async fn send(&mut self, frame: TransferFrame) -> Result<()> {
            if let Some(Frame::ResizeAck(ack)) = frame.frame.as_ref() {
                self.gate.acks.lock().unwrap().push(*ack);
                self.gate.entered.store(true, Ordering::Release);
                self.gate.changed.notify_waiters();
                self.gate
                    .release
                    .acquire()
                    .await
                    .map_err(|_| eyre::eyre!("resize ACK test gate closed"))?
                    .forget();
            }
            self.inner.send(frame).await
        }
    }

    struct CancelOnResizeAckTx {
        inner: Box<dyn FrameTx>,
        fired: Arc<AtomicBool>,
    }

    #[async_trait::async_trait]
    impl FrameTx for CancelOnResizeAckTx {
        async fn send(&mut self, frame: TransferFrame) -> Result<()> {
            if matches!(frame.frame.as_ref(), Some(Frame::ResizeAck(_))) {
                self.fired.store(true, Ordering::Release);
                return self
                    .inner
                    .send(super::frame(Frame::Error(SessionError {
                        code: session_error::Code::Cancelled as i32,
                        message: "injected cancellation during resize".into(),
                        relative_path: None,
                        ..Default::default()
                    })))
                    .await;
            }
            self.inner.send(frame).await
        }
    }

    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    struct WireResizeStep {
        epoch: u32,
        op: DataPlaneResizeOp,
        target: u32,
        token_len: usize,
    }

    struct DialTraceRun {
        steps: Vec<WireResizeStep>,
        add_tokens: Vec<Vec<u8>>,
        final_streams: usize,
        summary: TransferSummary,
        needed_paths: Vec<String>,
        events: Vec<crate::remote::transfer::session_phase::SessionPhaseEvent>,
    }

    async fn submit_dial_sample(
        samples: &mpsc::UnboundedSender<DialTestSample>,
        delta_bytes: u64,
        blocked_ratio: f64,
    ) -> DialTestObservation {
        let (reply, observation) = tokio::sync::oneshot::channel();
        samples
            .send(DialTestSample {
                delta_bytes,
                blocked_ratio,
                reply,
            })
            .expect("test tuner remains alive");
        tokio::time::timeout(std::time::Duration::from_secs(10), observation)
            .await
            .expect("dial sample timed out")
            .expect("test tuner dropped the sample reply")
    }

    async fn run_dial_trace(
        initiator_role: TransferRole,
        receiver_capacity: CapacityProfile,
        target_streams: usize,
        delta_bytes: u64,
        blocked_ratio: f64,
        hold_samples: usize,
    ) -> DialTraceRun {
        run_dial_trace_with_fixture(
            initiator_role,
            receiver_capacity,
            target_streams,
            delta_bytes,
            blocked_ratio,
            hold_samples,
            31,
            129 * 1024,
        )
        .await
    }

    #[allow(clippy::too_many_arguments)]
    async fn run_dial_trace_with_fixture(
        initiator_role: TransferRole,
        receiver_capacity: CapacityProfile,
        target_streams: usize,
        delta_bytes: u64,
        blocked_ratio: f64,
        hold_samples: usize,
        file_count: usize,
        file_bytes: usize,
    ) -> DialTraceRun {
        run_dial_trace_script_with_fixture(
            initiator_role,
            receiver_capacity,
            &[(target_streams, delta_bytes, blocked_ratio, hold_samples)],
            false,
            file_count,
            file_bytes,
        )
        .await
    }

    async fn run_dial_trace_script_with_fixture(
        initiator_role: TransferRole,
        receiver_capacity: CapacityProfile,
        phases: &[(usize, u64, f64, usize)],
        trace_enabled: bool,
        file_count: usize,
        file_bytes: usize,
    ) -> DialTraceRun {
        let tmp = tempfile::tempdir().expect("tempdir");
        let src_root = tmp.path().join("src");
        let dst_root = tmp.path().join("dst");
        std::fs::create_dir_all(&src_root).expect("source dir");
        std::fs::create_dir_all(&dst_root).expect("destination dir");
        let content = vec![0x5a; file_bytes];
        for i in 0..file_count {
            std::fs::write(src_root.join(format!("f{i:02}.bin")), &content).expect("write fixture");
        }

        let open = SessionOpen {
            initiator_role: initiator_role as i32,
            compare_mode: ComparisonMode::SizeMtime as i32,
            in_stream_bytes: false,
            ..Default::default()
        };
        let (source_endpoint, dest_endpoint, source_host, dest_host) = match initiator_role {
            TransferRole::Source => (
                SessionEndpoint::initiator(open),
                SessionEndpoint::Responder,
                Some("127.0.0.1".to_string()),
                None,
            ),
            TransferRole::Destination => (
                SessionEndpoint::Responder,
                SessionEndpoint::initiator(open),
                None,
                Some("127.0.0.1".to_string()),
            ),
            TransferRole::Unspecified => panic!("test must select an initiator role"),
        };

        let (sample_tx, sample_rx) = mpsc::unbounded_channel();
        let captured_events: Arc<
            StdMutex<Vec<crate::remote::transfer::session_phase::SessionPhaseEvent>>,
        > = Arc::default();
        let phase_trace = if trace_enabled {
            let captured = Arc::clone(&captured_events);
            SessionPhaseTrace::capture("dial-guard", move |event| {
                captured
                    .lock()
                    .expect("dial phase capture lock poisoned")
                    .push(event);
            })
        } else {
            SessionPhaseTrace::disabled()
        };
        let source_cfg = SourceSessionConfig {
            hello: HelloConfig::default(),
            endpoint: source_endpoint,
            plan_options: PlanOptions::default(),
            data_plane_host: source_host,
            instruments: SourceInstruments {
                dial_test_samples: Some(Arc::new(StdMutex::new(Some(sample_rx)))),
                session_phase_trace: phase_trace.clone(),
                ..Default::default()
            },
        };
        let dest_cfg = DestinationSessionConfig {
            hello: HelloConfig::default(),
            endpoint: dest_endpoint,
            data_plane_host: dest_host,
            receiver_capacity: Some(receiver_capacity),
            instruments: DestinationInstruments {
                session_phase_trace: phase_trace,
                ..Default::default()
            },
            local_apply: None,
        };

        let gate = PayloadGate::new();
        let source: Arc<dyn TransferSource> = Arc::new(GatedTransferSource {
            inner: FsTransferSource::new(src_root.clone()),
            gate: Arc::clone(&gate),
        });
        let captured: Arc<StdMutex<Vec<DataPlaneResize>>> = Arc::default();
        let (source_transport, dest_transport) = transport::in_process_pair();
        let (source_tx, source_rx) = source_transport.split();
        let source_transport = FrameTransport::new(
            Box::new(ResizeCaptureTx {
                inner: source_tx,
                frames: Arc::clone(&captured),
            }),
            source_rx,
        );

        let destination_root = dst_root.clone();
        let session = tokio::spawn(async move {
            tokio::join!(
                run_source(source_cfg, source_transport, source),
                run_destination(
                    dest_cfg,
                    dest_transport,
                    DestinationTarget::Fixed(destination_root)
                ),
            )
        });

        tokio::time::timeout(std::time::Duration::from_secs(10), gate.wait_for_entered(1))
            .await
            .expect("payload demand never reached the gated source");

        let mut live = crate::dial::receiver_initial_streams(Some(&receiver_capacity));
        for &(target_streams, delta_bytes, blocked_ratio, hold_samples) in phases {
            let mut attempts = 0usize;
            while live != target_streams {
                attempts += 1;
                assert!(attempts <= 256, "dial trace did not reach {target_streams}");
                let observed = submit_dial_sample(&sample_tx, delta_bytes, blocked_ratio).await;
                if let Some(proposal) = observed.proposal {
                    assert_eq!(proposal.epoch, observed.settled_epoch);
                    assert_eq!(proposal.target_streams, observed.live_streams);
                }
                live = observed.live_streams;
            }
            for _ in 0..hold_samples {
                let observed = submit_dial_sample(&sample_tx, delta_bytes, blocked_ratio).await;
                assert_eq!(observed.proposal, None, "bound/hold sample resized");
                assert_eq!(observed.live_streams, target_streams);
                assert_eq!(
                    observed.settled_epoch,
                    captured.lock().unwrap().len() as u32
                );
            }
        }

        gate.release(file_count + 8);
        let (source_result, destination_result) =
            tokio::time::timeout(std::time::Duration::from_secs(30), session)
                .await
                .expect("dial trace session timed out")
                .expect("dial trace session task panicked");
        let source_summary = source_result.expect("source session succeeds");
        let destination = destination_result.expect("destination session succeeds");
        assert_eq!(source_summary, destination.summary);
        assert_eq!(source_summary.files_transferred, file_count as u64);
        assert!(!source_summary.in_stream_carrier_used);
        let final_streams = destination
            .data_plane_streams
            .expect("data plane ran, final logical count recorded");
        assert_eq!(
            final_streams,
            phases
                .last()
                .expect("dial trace needs at least one phase")
                .0
        );
        let mut needed_paths = destination.needed_paths;
        needed_paths.sort();
        for i in 0..file_count {
            assert_eq!(
                std::fs::read(dst_root.join(format!("f{i:02}.bin"))).expect("read destination"),
                content
            );
        }

        let frames = captured.lock().expect("resize capture lock poisoned");
        let add_tokens = frames
            .iter()
            .filter(|resize| resize.op == DataPlaneResizeOp::Add as i32)
            .map(|resize| resize.sub_token.clone())
            .collect();
        let steps = frames
            .iter()
            .map(|resize| WireResizeStep {
                epoch: resize.epoch,
                op: DataPlaneResizeOp::try_from(resize.op).expect("known resize op"),
                target: resize.target_stream_count,
                token_len: resize.sub_token.len(),
            })
            .collect();
        drop(frames);
        let events = captured_events
            .lock()
            .expect("dial phase capture lock poisoned")
            .clone();
        DialTraceRun {
            steps,
            add_tokens,
            final_streams,
            summary: source_summary,
            needed_paths,
            events,
        }
    }

    fn constrained_profile(max_streams: u32) -> CapacityProfile {
        CapacityProfile {
            max_streams,
            max_chunk_bytes: crate::buffer::DATA_PLANE_BUFFER_FLOOR as u64,
            max_inflight_bytes: crate::buffer::DATA_PLANE_BUFFER_FLOOR as u64,
            ..Default::default()
        }
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    async fn live_dial_clean_trace_grows_both_layouts_through_seventeen() {
        let mut expected = Vec::new();
        for (offset, target) in (5u32..=17).enumerate() {
            expected.push(WireResizeStep {
                epoch: offset as u32 + 1,
                op: DataPlaneResizeOp::Add,
                target,
                token_len: crate::remote::transfer::SUB_TOKEN_LEN,
            });
        }
        let source_initiator = run_dial_trace(
            TransferRole::Source,
            constrained_profile(17),
            17,
            1024,
            0.0,
            8,
        )
        .await;
        let destination_initiator = run_dial_trace(
            TransferRole::Destination,
            constrained_profile(17),
            17,
            1024,
            0.0,
            8,
        )
        .await;
        assert_eq!(source_initiator.steps, expected);
        assert_eq!(destination_initiator.steps, expected);
        assert_eq!(source_initiator.summary, destination_initiator.summary);
        assert_eq!(
            source_initiator.needed_paths,
            destination_initiator.needed_paths
        );
        for run in [&source_initiator, &destination_initiator] {
            assert_eq!(run.add_tokens.len(), 13);
            assert!(run
                .add_tokens
                .iter()
                .all(|token| token.len() == crate::remote::transfer::SUB_TOKEN_LEN));
            let distinct: HashSet<_> = run.add_tokens.iter().collect();
            assert_eq!(
                distinct.len(),
                run.add_tokens.len(),
                "ADD tokens must be fresh"
            );
        }
        assert_eq!(source_initiator.final_streams, 17);
        assert_eq!(destination_initiator.final_streams, 17);
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    async fn live_dial_blocked_trace_shrinks_both_layouts_to_one() {
        let expected = vec![
            WireResizeStep {
                epoch: 1,
                op: DataPlaneResizeOp::Remove,
                target: 3,
                token_len: 0,
            },
            WireResizeStep {
                epoch: 2,
                op: DataPlaneResizeOp::Remove,
                target: 2,
                token_len: 0,
            },
            WireResizeStep {
                epoch: 3,
                op: DataPlaneResizeOp::Remove,
                target: 1,
                token_len: 0,
            },
        ];
        let mut runs = Vec::new();
        for role in [TransferRole::Source, TransferRole::Destination] {
            let run = run_dial_trace(role, constrained_profile(17), 1, 1024, 1.0, 8).await;
            assert_eq!(run.steps, expected, "initiator role {role:?}");
            assert_eq!(run.final_streams, 1);
            runs.push(run);
        }
        assert_eq!(runs[0].summary, runs[1].summary);
        assert_eq!(runs[0].needed_paths, runs[1].needed_paths);
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    async fn live_dial_idle_and_hysteresis_traces_hold_both_layouts() {
        for role in [TransferRole::Source, TransferRole::Destination] {
            let idle = run_dial_trace(role, constrained_profile(17), 4, 0, 0.0, 12).await;
            assert!(idle.steps.is_empty(), "idle resized for {role:?}");
            let hysteresis = run_dial_trace(role, constrained_profile(17), 4, 1024, 0.15, 12).await;
            assert!(
                hysteresis.steps.is_empty(),
                "hysteresis resized for {role:?}"
            );
        }
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    async fn workload_shape_has_no_resize_authority_in_either_layout() {
        for role in [TransferRole::Source, TransferRole::Destination] {
            let run =
                run_dial_trace_with_fixture(role, constrained_profile(17), 4, 0, 0.0, 8, 10_000, 1)
                    .await;
            assert!(run.steps.is_empty(), "workload shape resized for {role:?}");
            assert_eq!(run.final_streams, 4);
        }
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    async fn receiver_bounds_seed_both_layouts_identically() {
        for role in [TransferRole::Source, TransferRole::Destination] {
            let bounded = run_dial_trace(role, constrained_profile(2), 2, 1024, 0.0, 8).await;
            assert!(bounded.steps.is_empty());
            assert_eq!(bounded.final_streams, 2);

            let unknown = run_dial_trace(role, constrained_profile(0), 17, 1024, 0.0, 0).await;
            assert_eq!(unknown.steps.len(), 13);
            assert_eq!(unknown.final_streams, 17);
        }
    }

    async fn wait_for_captured_phase(
        events: &Arc<StdMutex<Vec<crate::remote::transfer::session_phase::SessionPhaseEvent>>>,
        endpoint_role: SessionPhaseRole,
        name: &'static str,
    ) {
        tokio::time::timeout(std::time::Duration::from_secs(10), async {
            loop {
                if events
                    .lock()
                    .expect("phase event capture lock poisoned")
                    .iter()
                    .any(|event| event.endpoint_role == endpoint_role && event.event == name)
                {
                    return;
                }
                tokio::task::yield_now().await;
            }
        })
        .await
        .unwrap_or_else(|_| panic!("timed out waiting for {endpoint_role:?}/{name}"));
    }

    async fn run_unsent_terminal_resize(initiator_role: TransferRole, blocked_ratio: f64) {
        let tmp = tempfile::tempdir().expect("tempdir");
        let src_root = tmp.path().join("src");
        let dst_root = tmp.path().join("dst");
        std::fs::create_dir_all(&src_root).expect("source dir");
        std::fs::create_dir_all(&dst_root).expect("destination dir");

        let open = SessionOpen {
            initiator_role: initiator_role as i32,
            compare_mode: ComparisonMode::SizeMtime as i32,
            in_stream_bytes: false,
            ..Default::default()
        };
        let (source_endpoint, dest_endpoint, source_host, dest_host) = match initiator_role {
            TransferRole::Source => (
                SessionEndpoint::initiator(open),
                SessionEndpoint::Responder,
                Some("127.0.0.1".to_string()),
                None,
            ),
            TransferRole::Destination => (
                SessionEndpoint::Responder,
                SessionEndpoint::initiator(open),
                None,
                Some("127.0.0.1".to_string()),
            ),
            TransferRole::Unspecified => unreachable!(),
        };

        let events: Arc<StdMutex<Vec<crate::remote::transfer::session_phase::SessionPhaseEvent>>> =
            Arc::default();
        let captured_events = Arc::clone(&events);
        let phase_trace = SessionPhaseTrace::capture("terminal-resize", move |event| {
            captured_events.lock().unwrap().push(event);
        });
        let gate = DialTerminalTestGate::new();
        let (sample_tx, sample_rx) = mpsc::unbounded_channel();
        let source_cfg = SourceSessionConfig {
            hello: HelloConfig::default(),
            endpoint: source_endpoint,
            plan_options: PlanOptions::default(),
            data_plane_host: source_host,
            instruments: SourceInstruments {
                session_phase_trace: phase_trace.clone(),
                dial_test_samples: Some(Arc::new(StdMutex::new(Some(sample_rx)))),
                dial_terminal_test_gate: Some(Arc::clone(&gate)),
                ..Default::default()
            },
        };
        let dest_cfg = DestinationSessionConfig {
            hello: HelloConfig::default(),
            endpoint: dest_endpoint,
            data_plane_host: dest_host,
            receiver_capacity: Some(constrained_profile(17)),
            instruments: DestinationInstruments {
                session_phase_trace: phase_trace,
                ..Default::default()
            },
            local_apply: None,
        };
        let resize_frames: Arc<StdMutex<Vec<DataPlaneResize>>> = Arc::default();
        let (source_transport, dest_transport) = transport::in_process_pair();
        let (source_tx, source_rx) = source_transport.split();
        let source_transport = FrameTransport::new(
            Box::new(ResizeCaptureTx {
                inner: source_tx,
                frames: Arc::clone(&resize_frames),
            }),
            source_rx,
        );
        let source: Arc<dyn TransferSource> = Arc::new(FsTransferSource::new(src_root));
        let session = tokio::spawn(async move {
            tokio::join!(
                run_source(source_cfg, source_transport, source),
                run_destination(dest_cfg, dest_transport, DestinationTarget::Fixed(dst_root)),
            )
        });

        gate.wait_until_entered().await;
        wait_for_captured_phase(&events, SessionPhaseRole::Source, "need_complete_received").await;
        for _ in 0..3 {
            let observed = submit_dial_sample(&sample_tx, 1024, blocked_ratio).await;
            assert_eq!(observed.proposal, None);
            assert_eq!(observed.live_streams, 4);
        }
        let (reply, pending_reply) = tokio::sync::oneshot::channel();
        sample_tx
            .send(DialTestSample {
                delta_bytes: 1024,
                blocked_ratio,
                reply,
            })
            .expect("test tuner remains alive");
        wait_for_captured_phase(&events, SessionPhaseRole::Source, "dial_pending").await;
        gate.release();

        let (source_result, destination_result) =
            tokio::time::timeout(std::time::Duration::from_secs(30), session)
                .await
                .expect("terminal resize session timed out")
                .expect("terminal resize session task panicked");
        let source_summary = source_result.expect("source succeeds");
        let destination = destination_result.expect("destination succeeds");
        assert_eq!(source_summary, destination.summary);
        assert_eq!(source_summary.files_transferred, 0);
        assert_eq!(destination.data_plane_streams, Some(4));
        assert!(resize_frames.lock().unwrap().is_empty());
        assert!(
            pending_reply.await.is_err(),
            "terminal shutdown stops the injected tuner instead of accepting its proposal"
        );
        let events = events.lock().unwrap();
        let settlement: Vec<_> = events
            .iter()
            .filter(|event| {
                event.endpoint_role == SessionPhaseRole::Source && event.event == "dial_settlement"
            })
            .collect();
        assert_eq!(settlement.len(), 1);
        assert_eq!(settlement[0].accepted, Some(false));
        assert_eq!(settlement[0].live_streams, Some(4));
        assert_eq!(settlement[0].peak_streams, Some(4));
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    async fn need_complete_refuses_ready_unsent_add_and_remove_in_both_layouts() {
        for role in [TransferRole::Source, TransferRole::Destination] {
            run_unsent_terminal_resize(role, 0.0).await;
            run_unsent_terminal_resize(role, 1.0).await;
        }
    }

    async fn run_accepted_terminal_resize(initiator_role: TransferRole, op: DataPlaneResizeOp) {
        let tmp = tempfile::tempdir().expect("tempdir");
        let src_root = tmp.path().join("src");
        let dst_root = tmp.path().join("dst");
        std::fs::create_dir_all(&src_root).expect("source dir");
        std::fs::create_dir_all(&dst_root).expect("destination dir");
        let content = vec![0x6b; 129 * 1024];
        for index in 0..16 {
            std::fs::write(src_root.join(format!("payload-{index:02}.bin")), &content)
                .expect("write fixture");
        }

        let open = SessionOpen {
            initiator_role: initiator_role as i32,
            compare_mode: ComparisonMode::SizeMtime as i32,
            in_stream_bytes: false,
            ..Default::default()
        };
        let (source_endpoint, dest_endpoint, source_host, dest_host) = match initiator_role {
            TransferRole::Source => (
                SessionEndpoint::initiator(open),
                SessionEndpoint::Responder,
                Some("127.0.0.1".to_string()),
                None,
            ),
            TransferRole::Destination => (
                SessionEndpoint::Responder,
                SessionEndpoint::initiator(open),
                None,
                Some("127.0.0.1".to_string()),
            ),
            TransferRole::Unspecified => unreachable!(),
        };
        let events: Arc<StdMutex<Vec<crate::remote::transfer::session_phase::SessionPhaseEvent>>> =
            Arc::default();
        let captured_events = Arc::clone(&events);
        let phase_trace = SessionPhaseTrace::capture("accepted-terminal", move |event| {
            captured_events.lock().unwrap().push(event);
        });
        let (sample_tx, sample_rx) = mpsc::unbounded_channel();
        let payload_gate = PayloadGate::new();
        let source: Arc<dyn TransferSource> = Arc::new(GatedTransferSource {
            inner: FsTransferSource::new(src_root),
            gate: Arc::clone(&payload_gate),
        });
        let source_cfg = SourceSessionConfig {
            hello: HelloConfig::default(),
            endpoint: source_endpoint,
            plan_options: PlanOptions::default(),
            data_plane_host: source_host,
            instruments: SourceInstruments {
                session_phase_trace: phase_trace.clone(),
                dial_test_samples: Some(Arc::new(StdMutex::new(Some(sample_rx)))),
                ..Default::default()
            },
        };
        let dest_cfg = DestinationSessionConfig {
            hello: HelloConfig::default(),
            endpoint: dest_endpoint,
            data_plane_host: dest_host,
            receiver_capacity: Some(constrained_profile(17)),
            instruments: DestinationInstruments {
                session_phase_trace: phase_trace,
                ..Default::default()
            },
            local_apply: None,
        };

        let source_resizes: Arc<StdMutex<Vec<DataPlaneResize>>> = Arc::default();
        let ack_gate = ResizeAckGate::new();
        let (source_transport, destination_transport) = transport::in_process_pair();
        let (source_tx, source_rx) = source_transport.split();
        let source_transport = FrameTransport::new(
            Box::new(ResizeCaptureTx {
                inner: source_tx,
                frames: Arc::clone(&source_resizes),
            }),
            source_rx,
        );
        let (destination_tx, destination_rx) = destination_transport.split();
        let destination_transport = FrameTransport::new(
            Box::new(ResizeAckGateTx {
                inner: destination_tx,
                gate: Arc::clone(&ack_gate),
            }),
            destination_rx,
        );
        let destination_root = dst_root.clone();
        let session = tokio::spawn(async move {
            tokio::join!(
                run_source(source_cfg, source_transport, source),
                run_destination(
                    dest_cfg,
                    destination_transport,
                    DestinationTarget::Fixed(destination_root)
                ),
            )
        });

        // Four workers are held in prepare and the one-slot queue is full,
        // so the SOURCE remains in its event-servicing queue loop while the
        // deterministic resize is proposed.
        payload_gate.wait_for_entered(4).await;
        let blocked_ratio = if op == DataPlaneResizeOp::Add {
            0.0
        } else {
            1.0
        };
        for _ in 0..3 {
            let observed = submit_dial_sample(&sample_tx, 1024, blocked_ratio).await;
            assert_eq!(observed.proposal, None);
        }
        let (reply, _pending_reply) = tokio::sync::oneshot::channel();
        sample_tx
            .send(DialTestSample {
                delta_bytes: 1024,
                blocked_ratio,
                reply,
            })
            .expect("test tuner remains alive");
        ack_gate.wait_until_entered().await;
        payload_gate.release(64);
        wait_for_captured_phase(&events, SessionPhaseRole::Source, "membership_sealed").await;
        ack_gate.release();
        wait_for_captured_phase(&events, SessionPhaseRole::Source, "dial_settlement").await;

        let (source_result, destination_result) =
            tokio::time::timeout(std::time::Duration::from_secs(30), session)
                .await
                .expect("accepted terminal resize session timed out")
                .expect("accepted terminal resize session task panicked");
        let source_summary = source_result.expect("source succeeds");
        let destination = destination_result.expect("destination succeeds");
        assert_eq!(source_summary, destination.summary);
        assert_eq!(source_summary.files_transferred, 16);
        let expected_final = if op == DataPlaneResizeOp::Add { 5 } else { 3 };
        let expected_peak = if op == DataPlaneResizeOp::Add { 5 } else { 4 };
        assert_eq!(destination.data_plane_streams, Some(expected_final));
        for index in 0..16 {
            assert_eq!(
                std::fs::read(dst_root.join(format!("payload-{index:02}.bin"))).unwrap(),
                content
            );
        }
        let resizes = source_resizes.lock().unwrap();
        assert_eq!(resizes.len(), 1);
        assert_eq!(DataPlaneResizeOp::try_from(resizes[0].op).unwrap(), op);
        assert_eq!(resizes[0].target_stream_count as usize, expected_final);
        let acks = ack_gate.acks.lock().unwrap();
        assert_eq!(acks.len(), 1);
        assert!(acks[0].accepted);
        assert_eq!(acks[0].effective_stream_count as usize, expected_final);
        let events = events.lock().unwrap();
        for endpoint_role in [SessionPhaseRole::Source, SessionPhaseRole::Destination] {
            let complete = events
                .iter()
                .find(|event| {
                    event.endpoint_role == endpoint_role && event.event == "data_plane_complete"
                })
                .expect("data-plane completion observed");
            assert_eq!(complete.live_streams, Some(expected_final as u32));
            assert_eq!(complete.peak_streams, Some(expected_peak as u32));
        }
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    async fn accepted_add_and_remove_settle_after_need_complete_in_both_layouts() {
        for role in [TransferRole::Source, TransferRole::Destination] {
            run_accepted_terminal_resize(role, DataPlaneResizeOp::Add).await;
            run_accepted_terminal_resize(role, DataPlaneResizeOp::Remove).await;
        }
    }

    fn assert_destination_receive_tasks_stopped(
        events: &[crate::remote::transfer::session_phase::SessionPhaseEvent],
    ) {
        let attached: HashSet<_> = events
            .iter()
            .filter(|event| {
                event.endpoint_role == SessionPhaseRole::Destination
                    && event.event == "socket_trace_attached"
            })
            .map(|event| (event.epoch, event.socket))
            .collect();
        let stopped: HashSet<_> = events
            .iter()
            .filter(|event| {
                event.endpoint_role == SessionPhaseRole::Destination
                    && event.event == "receive_task_stopped"
            })
            .map(|event| (event.epoch, event.socket))
            .collect();
        assert!(!attached.is_empty(), "TCP receive workers were started");
        assert_eq!(
            stopped, attached,
            "every receive task stopped before return"
        );
    }

    fn assert_data_plane_abort_accounting(
        events: &[crate::remote::transfer::session_phase::SessionPhaseEvent],
        source_streams: u32,
        destination_streams: u32,
    ) {
        for endpoint_role in [SessionPhaseRole::Source, SessionPhaseRole::Destination] {
            let aborted: Vec<_> = events
                .iter()
                .filter(|event| {
                    event.endpoint_role == endpoint_role && event.event == "data_plane_aborted"
                })
                .collect();
            assert_eq!(aborted.len(), 1, "one abort record for {endpoint_role:?}");
            let streams = match endpoint_role {
                SessionPhaseRole::Source => source_streams,
                SessionPhaseRole::Destination => destination_streams,
            };
            assert_eq!(aborted[0].live_streams, Some(streams));
            assert_eq!(aborted[0].peak_streams, Some(streams));
            assert_eq!(aborted[0].receiver_ceiling, Some(17));
        }
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    async fn tcp_source_fault_joins_data_plane_tasks_in_both_layouts() {
        for initiator_role in [TransferRole::Source, TransferRole::Destination] {
            let tmp = tempfile::tempdir().expect("tempdir");
            let src_root = tmp.path().join("src");
            let dst_root = tmp.path().join("dst");
            std::fs::create_dir_all(&src_root).unwrap();
            std::fs::create_dir_all(&dst_root).unwrap();
            std::fs::write(src_root.join("fault.bin"), vec![0x41; 129 * 1024]).unwrap();

            let open = SessionOpen {
                initiator_role: initiator_role as i32,
                compare_mode: ComparisonMode::SizeMtime as i32,
                in_stream_bytes: false,
                ..Default::default()
            };
            let (source_endpoint, dest_endpoint, source_host, dest_host) = match initiator_role {
                TransferRole::Source => (
                    SessionEndpoint::initiator(open),
                    SessionEndpoint::Responder,
                    Some("127.0.0.1".to_string()),
                    None,
                ),
                TransferRole::Destination => (
                    SessionEndpoint::Responder,
                    SessionEndpoint::initiator(open),
                    None,
                    Some("127.0.0.1".to_string()),
                ),
                TransferRole::Unspecified => unreachable!(),
            };
            let events: Arc<
                StdMutex<Vec<crate::remote::transfer::session_phase::SessionPhaseEvent>>,
            > = Arc::default();
            let captured = Arc::clone(&events);
            let phase_trace = SessionPhaseTrace::capture("source-fault", move |event| {
                captured.lock().unwrap().push(event);
            });
            let source_cfg = SourceSessionConfig {
                hello: HelloConfig::default(),
                endpoint: source_endpoint,
                plan_options: PlanOptions::default(),
                data_plane_host: source_host,
                instruments: SourceInstruments {
                    session_phase_trace: phase_trace.clone(),
                    ..Default::default()
                },
            };
            let dest_cfg = DestinationSessionConfig {
                hello: HelloConfig::default(),
                endpoint: dest_endpoint,
                data_plane_host: dest_host,
                receiver_capacity: Some(constrained_profile(17)),
                instruments: DestinationInstruments {
                    session_phase_trace: phase_trace,
                    ..Default::default()
                },
                local_apply: None,
            };
            let source: Arc<dyn TransferSource> = Arc::new(PrepareFaultSource {
                inner: FsTransferSource::new(src_root),
            });
            let (source_transport, destination_transport) = transport::in_process_pair();
            let session = tokio::spawn(async move {
                tokio::join!(
                    run_source(source_cfg, source_transport, source),
                    run_destination(
                        dest_cfg,
                        destination_transport,
                        DestinationTarget::Fixed(dst_root)
                    ),
                )
            });
            let (source_result, destination_result) =
                tokio::time::timeout(std::time::Duration::from_secs(30), session)
                    .await
                    .expect("source-fault session timed out")
                    .expect("source-fault session task panicked");
            let source_error = source_result.expect_err("source fault must fail SOURCE");
            let destination_error =
                destination_result.expect_err("source fault must fail DESTINATION");
            assert!(
                format!("{source_error:#}").contains("injected TCP source fault"),
                "source returned: {source_error:#}"
            );
            assert!(
                format!("{destination_error:#}").contains("injected TCP source fault"),
                "destination returned: {destination_error:#}"
            );
            let events = events.lock().unwrap();
            assert_data_plane_abort_accounting(&events, 4, 4);
            assert!(!events.iter().any(|event| event.event == "summary_sent"));
            assert_destination_receive_tasks_stopped(&events);
        }
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    async fn tcp_cancellation_during_add_joins_tasks_without_false_settlement() {
        for initiator_role in [TransferRole::Source, TransferRole::Destination] {
            let tmp = tempfile::tempdir().expect("tempdir");
            let src_root = tmp.path().join("src");
            let dst_root = tmp.path().join("dst");
            std::fs::create_dir_all(&src_root).unwrap();
            std::fs::create_dir_all(&dst_root).unwrap();
            let content = vec![0x33; 129 * 1024];
            for index in 0..16 {
                std::fs::write(src_root.join(format!("cancel-{index:02}.bin")), &content).unwrap();
            }

            let open = SessionOpen {
                initiator_role: initiator_role as i32,
                compare_mode: ComparisonMode::SizeMtime as i32,
                in_stream_bytes: false,
                ..Default::default()
            };
            let (source_endpoint, dest_endpoint, source_host, dest_host) = match initiator_role {
                TransferRole::Source => (
                    SessionEndpoint::initiator(open),
                    SessionEndpoint::Responder,
                    Some("127.0.0.1".to_string()),
                    None,
                ),
                TransferRole::Destination => (
                    SessionEndpoint::Responder,
                    SessionEndpoint::initiator(open),
                    None,
                    Some("127.0.0.1".to_string()),
                ),
                TransferRole::Unspecified => unreachable!(),
            };
            let events: Arc<
                StdMutex<Vec<crate::remote::transfer::session_phase::SessionPhaseEvent>>,
            > = Arc::default();
            let captured = Arc::clone(&events);
            let phase_trace = SessionPhaseTrace::capture("cancel-resize", move |event| {
                captured.lock().unwrap().push(event);
            });
            let payload_gate = PayloadGate::new();
            let source: Arc<dyn TransferSource> = Arc::new(GatedTransferSource {
                inner: FsTransferSource::new(src_root),
                gate: Arc::clone(&payload_gate),
            });
            let (sample_tx, sample_rx) = mpsc::unbounded_channel();
            let source_cfg = SourceSessionConfig {
                hello: HelloConfig::default(),
                endpoint: source_endpoint,
                plan_options: PlanOptions::default(),
                data_plane_host: source_host,
                instruments: SourceInstruments {
                    session_phase_trace: phase_trace.clone(),
                    dial_test_samples: Some(Arc::new(StdMutex::new(Some(sample_rx)))),
                    ..Default::default()
                },
            };
            let dest_cfg = DestinationSessionConfig {
                hello: HelloConfig::default(),
                endpoint: dest_endpoint,
                data_plane_host: dest_host,
                receiver_capacity: Some(constrained_profile(17)),
                instruments: DestinationInstruments {
                    session_phase_trace: phase_trace,
                    ..Default::default()
                },
                local_apply: None,
            };
            let resize_frames: Arc<StdMutex<Vec<DataPlaneResize>>> = Arc::default();
            let cancellation_fired = Arc::new(AtomicBool::new(false));
            let (source_transport, destination_transport) = transport::in_process_pair();
            let (source_tx, source_rx) = source_transport.split();
            let source_transport = FrameTransport::new(
                Box::new(ResizeCaptureTx {
                    inner: source_tx,
                    frames: Arc::clone(&resize_frames),
                }),
                source_rx,
            );
            let (destination_tx, destination_rx) = destination_transport.split();
            let destination_transport = FrameTransport::new(
                Box::new(CancelOnResizeAckTx {
                    inner: destination_tx,
                    fired: Arc::clone(&cancellation_fired),
                }),
                destination_rx,
            );
            let session = tokio::spawn(async move {
                tokio::join!(
                    run_source(source_cfg, source_transport, source),
                    run_destination(
                        dest_cfg,
                        destination_transport,
                        DestinationTarget::Fixed(dst_root)
                    ),
                )
            });

            payload_gate.wait_for_entered(4).await;
            for _ in 0..3 {
                assert_eq!(
                    submit_dial_sample(&sample_tx, 1024, 0.0).await.proposal,
                    None
                );
            }
            let (reply, _pending_reply) = tokio::sync::oneshot::channel();
            sample_tx
                .send(DialTestSample {
                    delta_bytes: 1024,
                    blocked_ratio: 0.0,
                    reply,
                })
                .expect("test tuner remains alive");

            let (source_result, destination_result) =
                tokio::time::timeout(std::time::Duration::from_secs(30), session)
                    .await
                    .expect("cancelled resize session timed out")
                    .expect("cancelled resize session task panicked");
            let source_error = source_result.expect_err("cancel must fail SOURCE");
            let _ = destination_result.expect_err("cancel must fail DESTINATION");
            assert!(cancellation_fired.load(Ordering::Acquire));
            assert!(
                format!("{source_error:#}").contains("injected cancellation during resize"),
                "source returned: {source_error:#}"
            );
            assert_eq!(resize_frames.lock().unwrap().len(), 1);
            let events = events.lock().unwrap();
            assert!(events.iter().any(|event| {
                event.endpoint_role == SessionPhaseRole::Source && event.event == "dial_pending"
            }));
            assert_data_plane_abort_accounting(&events, 4, 5);
            assert!(
                !events.iter().any(|event| {
                    event.endpoint_role == SessionPhaseRole::Source
                        && event.event == "dial_settlement"
                }),
                "faulted accepted transport must not be rewritten as a refusal"
            );
            assert!(!events.iter().any(|event| event.event == "summary_sent"));
            assert_destination_receive_tasks_stopped(&events);
        }
    }

    fn dial_event_semantics(
        events: &[crate::remote::transfer::session_phase::SessionPhaseEvent],
    ) -> Vec<(
        &'static str,
        Option<&'static str>,
        Option<&'static str>,
        Option<u32>,
        Option<u32>,
        Option<u32>,
        Option<bool>,
    )> {
        events
            .iter()
            .filter(|event| {
                event.endpoint_role == SessionPhaseRole::Source && event.event.starts_with("dial_")
            })
            .map(|event| {
                (
                    event.event,
                    event.action,
                    event.reason,
                    event.epoch,
                    event.target_streams,
                    event.live_streams,
                    event.accepted,
                )
            })
            .collect()
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    async fn dial_observer_is_inert_and_reports_peak_separately_from_final() {
        let phases = [(7, 1024, 0.0, 0), (1, 1024, 1.0, 0), (3, 1024, 0.0, 0)];
        let mut observed_runs = Vec::new();
        for role in [TransferRole::Source, TransferRole::Destination] {
            let off = run_dial_trace_script_with_fixture(
                role,
                constrained_profile(17),
                &phases,
                false,
                31,
                129 * 1024,
            )
            .await;
            let on = run_dial_trace_script_with_fixture(
                role,
                constrained_profile(17),
                &phases,
                true,
                31,
                129 * 1024,
            )
            .await;

            assert!(off.events.is_empty(), "observer OFF emitted for {role:?}");
            assert_eq!(off.steps, on.steps, "observer changed policy for {role:?}");
            assert_eq!(off.summary, on.summary);
            assert_eq!(off.needed_paths, on.needed_paths);
            assert_eq!(off.final_streams, 3);
            assert_eq!(on.final_streams, 3);
            assert_eq!(on.add_tokens.len(), 5, "4→7 then 1→3 opens five sockets");

            let samples: Vec<_> = on
                .events
                .iter()
                .filter(|event| {
                    event.endpoint_role == SessionPhaseRole::Source && event.event == "dial_sample"
                })
                .collect();
            assert!(!samples.is_empty());
            assert!(samples.iter().all(|event| {
                event.reason.is_some()
                    && event.sample_bytes.is_some()
                    && event.sample_blocked_ns.is_some()
                    && event.sample_elapsed_ns.is_some()
                    && event.sample_streams.is_some()
                    && event.sample_valid.is_some()
                    && event.blocked_ratio.is_some()
                    && event.chunk_bytes.is_some()
                    && event.prefetch_count.is_some()
                    && event.tcp_buffer_bytes.is_some()
                    && event.receiver_ceiling == Some(17)
                    && event.peak_streams.is_some()
                    && (crate::dial::blocked_ratio(
                        event.sample_blocked_ns.unwrap(),
                        std::time::Duration::from_nanos(event.sample_elapsed_ns.unwrap()),
                        event.sample_streams.unwrap() as usize,
                    ) - event.blocked_ratio.unwrap())
                    .abs()
                        <= 1e-12
            }));
            assert_eq!(
                on.events
                    .iter()
                    .filter(|event| {
                        event.endpoint_role == SessionPhaseRole::Source
                            && event.event == "dial_pending"
                    })
                    .count(),
                on.steps.len()
            );
            assert_eq!(
                on.events
                    .iter()
                    .filter(|event| {
                        event.endpoint_role == SessionPhaseRole::Source
                            && event.event == "dial_settlement"
                            && event.accepted == Some(true)
                    })
                    .count(),
                on.steps.len()
            );
            for endpoint_role in [SessionPhaseRole::Source, SessionPhaseRole::Destination] {
                let complete: Vec<_> = on
                    .events
                    .iter()
                    .filter(|event| {
                        event.endpoint_role == endpoint_role && event.event == "data_plane_complete"
                    })
                    .collect();
                assert_eq!(complete.len(), 1, "one completion for {endpoint_role:?}");
                assert_eq!(complete[0].live_streams, Some(3));
                assert_eq!(complete[0].peak_streams, Some(7));
                assert_eq!(complete[0].receiver_ceiling, Some(17));
            }

            let dial_json = serde_json::to_string(
                &on.events
                    .iter()
                    .filter(|event| event.event.starts_with("dial_"))
                    .collect::<Vec<_>>(),
            )
            .expect("dial events serialize");
            assert!(!dial_json.contains("f00.bin"));
            assert!(!dial_json.contains("token"));
            observed_runs.push(on);
        }

        assert_eq!(observed_runs[0].steps, observed_runs[1].steps);
        assert_eq!(observed_runs[0].summary, observed_runs[1].summary);
        assert_eq!(
            dial_event_semantics(&observed_runs[0].events),
            dial_event_semantics(&observed_runs[1].events),
            "semantic dial trace must not depend on who initiated"
        );
    }

    /// otp-10c-2 codex F4: the mirror delete pass containment-checks
    /// every planned target against the canonical destination root
    /// before any filesystem op. The wiring was unpinned (a mutation
    /// deleting the `contained(...)` call survived the suite): with a
    /// canonical root that does NOT contain the destination, the pass
    /// must refuse before deleting anything — and with the real root
    /// it deletes normally (the control arm, so this can't pass
    /// vacuously).
    #[test]
    fn mirror_delete_pass_containment_check_gates_every_deletion() {
        let tmp = tempfile::tempdir().unwrap();
        let dst = tmp.path().join("dst");
        std::fs::create_dir_all(&dst).unwrap();
        std::fs::write(dst.join("extraneous.txt"), b"x").unwrap();
        let elsewhere = tmp.path().join("elsewhere");
        std::fs::create_dir_all(&elsewhere).unwrap();
        let elsewhere = elsewhere.canonicalize().unwrap();

        let source_files: HashSet<String> = HashSet::new(); // everything is extraneous
        let filter = crate::fs_enum::FileFilter::default();
        let abort = AtomicBool::new(false);

        // Foreign canonical root → the containment check must refuse
        // the deletion and leave the file alone.
        let err = mirror_delete_pass(
            &dst,
            &source_files,
            &filter,
            false,
            Some(&elsewhere),
            &abort,
            true,
        )
        .expect_err("a target outside the canonical root must refuse");
        assert!(
            format!("{err:#}").contains("mirror delete containment"),
            "got: {err:#}"
        );
        assert!(
            dst.join("extraneous.txt").exists(),
            "nothing may be deleted once containment refuses"
        );

        // Control: the real canonical root deletes the extraneous file.
        let real_root = crate::path_safety::canonical_dest_root(&dst).unwrap();
        let deleted = mirror_delete_pass(
            &dst,
            &source_files,
            &filter,
            false,
            Some(&real_root),
            &abort,
            true,
        )
        .expect("in-root deletion proceeds");
        assert_eq!(deleted, (1, 0));
        assert!(!dst.join("extraneous.txt").exists());
    }

    /// otp-11b: plan-only mode (the local carrier's dry-run) counts
    /// the full plan without deleting anything.
    #[test]
    fn mirror_delete_pass_plan_only_counts_without_deleting() {
        let tmp = tempfile::tempdir().expect("tempdir");
        std::fs::create_dir_all(tmp.path().join("stale-dir")).expect("mkdir");
        std::fs::write(tmp.path().join("stale-dir/f.txt"), b"x").expect("write");
        std::fs::write(tmp.path().join("stale.txt"), b"x").expect("write");
        let source_files: HashSet<String> = HashSet::new();
        let filter = crate::fs_enum::FileFilter::default();
        let abort = AtomicBool::new(false);
        let counts = mirror_delete_pass(
            tmp.path(),
            &source_files,
            &filter,
            false,
            None,
            &abort,
            false,
        )
        .expect("plan-only pass");
        assert_eq!(counts, (2, 1));
        assert!(tmp.path().join("stale.txt").exists());
        assert!(tmp.path().join("stale-dir/f.txt").exists());
    }

    /// otp-11b: the split counters — files and dirs accounted
    /// separately across nesting (the local summary's split).
    #[test]
    fn mirror_delete_pass_splits_file_and_dir_counts() {
        let tmp = tempfile::tempdir().expect("tempdir");
        std::fs::create_dir_all(tmp.path().join("d1/d2")).expect("mkdir");
        std::fs::write(tmp.path().join("d1/a.txt"), b"x").expect("write");
        std::fs::write(tmp.path().join("d1/d2/b.txt"), b"x").expect("write");
        let source_files: HashSet<String> = HashSet::new();
        let filter = crate::fs_enum::FileFilter::default();
        let abort = AtomicBool::new(false);
        let counts = mirror_delete_pass(
            tmp.path(),
            &source_files,
            &filter,
            false,
            None,
            &abort,
            true,
        )
        .expect("pass");
        assert_eq!(counts, (2, 2));
        assert!(!tmp.path().join("d1").exists());
    }

    #[test]
    fn build_id_has_version_and_git_components() {
        let id = session_build_id();
        let (version, git) = id.split_once('+').expect("build id must be version+git");
        assert_eq!(version, env!("CARGO_PKG_VERSION"));
        assert!(!git.is_empty(), "git component must be non-empty");
    }

    /// codex otp-10a F5: converting a driver error into a
    /// `SessionFault` stringifies the chain, which would strip the
    /// `io::ErrorKind` the retry classifier keys on — the fault must
    /// carry it. A chain with no I/O source stays kind-less (fatal to
    /// the classifier, as before).
    #[test]
    fn fault_from_report_captures_the_underlying_io_kind() {
        let io_report = eyre::Report::new(std::io::Error::new(
            std::io::ErrorKind::ConnectionReset,
            "peer reset",
        ))
        .wrap_err("sending payload record");
        let fault = fault_from_report(io_report);
        assert_eq!(fault.io_kind, Some(std::io::ErrorKind::ConnectionReset));

        let plain = fault_from_report(eyre::eyre!("path escapes module root"));
        assert_eq!(plain.io_kind, None);

        // An already-typed fault passes through untouched.
        let typed = fault_from_report(eyre::Report::new(SessionFault::internal("x")));
        assert_eq!(typed.io_kind, None);
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
            relative_path: None,
            io_kind: None,
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
            relative_path: None,
            io_kind: None,
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

    /// otp-7b-2 (D-2026-07-09-1 Q2 rider): the end-of-operation summary
    /// names the affected file and suggests a re-run; a fault with no
    /// file identity yields no summary block (nothing to converge on).
    /// The path survives the wire round trip (`SessionError.relative_path`)
    /// so BOTH ends can report it, and a `FaultedPath` marker in an eyre
    /// chain is lifted into the fault by `fault_from_report`.
    #[test]
    fn fault_summary_names_the_file_and_survives_the_wire() {
        let fault = SessionFault::internal("'big.bin' hit EOF with 42 bytes still promised")
            .with_path("big.bin");
        let summary = fault
            .end_of_operation_summary()
            .expect("a fault naming a file yields a summary");
        assert!(summary.contains("big.bin"), "summary names the file");
        assert!(
            summary.contains("re-run"),
            "summary suggests a re-run to converge"
        );
        assert_eq!(
            SessionFault::internal("no file involved").end_of_operation_summary(),
            None,
            "no file identity, no summary block"
        );

        // Wire round trip: the path rides SessionError.relative_path.
        let restored = SessionFault::from_wire(fault.to_wire());
        assert_eq!(restored.relative_path.as_deref(), Some("big.bin"));
        let no_path = SessionFault::from_wire(SessionFault::internal("x").to_wire());
        assert_eq!(no_path.relative_path, None, "absent wire path is None");
        // codex 7b-2 G1: "" is the single-file-root identity — it must
        // survive the wire (explicit presence) and render non-blank.
        let root_file = SessionFault::from_wire(
            SessionFault::internal("root file fault")
                .with_path("")
                .to_wire(),
        );
        assert_eq!(
            root_file.relative_path.as_deref(),
            Some(""),
            "the empty single-file-root identity survives the wire"
        );
        assert!(
            root_file
                .end_of_operation_summary()
                .expect("a summary exists for the root-file identity")
                .contains("<the transfer root file>"),
            "the root-file identity renders non-blank"
        );

        // eyre-chain lift: a FaultedPath marker anywhere in a non-fault
        // report becomes the fault's structured identity.
        let report =
            eyre::eyre!("underlying io error").wrap_err(FaultedPath("dir/partial.bin".to_string()));
        let lifted = fault_from_report(report);
        assert_eq!(lifted.relative_path.as_deref(), Some("dir/partial.bin"));
        // ...and a report already carrying a SessionFault keeps that
        // fault verbatim (tag_path never wraps one).
        let fault_report = tag_path(
            eyre::Report::new(SessionFault::protocol_violation("v").with_path("kept.bin")),
            "other.bin",
        );
        let kept = fault_from_report(fault_report);
        assert_eq!(kept.relative_path.as_deref(), Some("kept.bin"));
    }

    /// otp-7a codex F1: the hash-count cap decision — a partial at
    /// exactly the cap hashes; one block past it degrades to the empty
    /// full-transfer fallback. Pure-function test because the boundary
    /// fixture would otherwise be MAX_RESUME_BLOCK_HASHES × 64 KiB = 4 GiB.
    #[test]
    fn resume_hash_list_cap_boundary() {
        let bs = MIN_RESUME_BLOCK_SIZE;
        let at_cap = MAX_RESUME_BLOCK_HASHES * bs as u64;
        assert!(resume_hash_list_fits(0, bs), "empty partial fits");
        assert!(resume_hash_list_fits(at_cap, bs), "exactly the cap fits");
        assert!(
            !resume_hash_list_fits(at_cap + 1, bs),
            "one byte past the cap degrades to the full-transfer fallback"
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
            relative_path: None,
            io_kind: None,
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

    /// codex otp-9b F2: the mirror pass runs on the blocking pool, where
    /// a dropped session future cannot reach it — the drop-guard's abort
    /// flag must stop it before the next filesystem op. With the flag
    /// pre-set the pass deletes NOTHING, even with a genuinely
    /// extraneous entry present.
    #[test]
    fn mirror_delete_pass_aborts_on_the_cancellation_flag() {
        let tmp = tempfile::tempdir().expect("tempdir");
        std::fs::write(tmp.path().join("extraneous.bin"), b"x").expect("write");
        let source_files: HashSet<String> = HashSet::new(); // everything extraneous
        let filter = crate::fs_enum::FileFilter::default();

        let abort = AtomicBool::new(true);
        let result = mirror_delete_pass(
            tmp.path(),
            &source_files,
            &filter,
            false,
            None,
            &abort,
            true,
        );
        assert!(result.is_err(), "an aborted pass reports the abort");
        assert!(
            tmp.path().join("extraneous.bin").exists(),
            "an aborted pass must not delete"
        );

        // Un-aborted control: the same fixture deletes the entry.
        let abort = AtomicBool::new(false);
        let deleted = mirror_delete_pass(
            tmp.path(),
            &source_files,
            &filter,
            false,
            None,
            &abort,
            true,
        )
        .expect("pass succeeds");
        assert_eq!(deleted, (1, 0));
        assert!(!tmp.path().join("extraneous.bin").exists());
    }

    fn tar_test_header(path: String) -> FileHeader {
        FileHeader {
            relative_path: path,
            size: 1,
            mtime_seconds: 1_600_000_000,
            permissions: 0o644,
            checksum: Vec::new(),
        }
    }

    /// codex otp-8 F2: an oversized shard splits into consecutive
    /// shards, each with its encoded member list under the bound; order
    /// and file set are preserved and non-shard payloads pass through.
    #[test]
    fn tar_shard_headers_split_under_the_in_stream_bound() {
        use prost::Message;
        let headers: Vec<FileHeader> = (0..40)
            .map(|i| tar_test_header(format!("{i:0>100}")))
            .collect();
        let expected: Vec<String> = headers.iter().map(|h| h.relative_path.clone()).collect();

        let out = bound_in_stream_tar_headers(
            vec![
                TransferPayload::TarShard {
                    headers: headers.clone(),
                },
                TransferPayload::File(tar_test_header("plain.bin".into())),
            ],
            512,
        );
        let shards: Vec<&Vec<FileHeader>> = out
            .iter()
            .filter_map(|p| match p {
                TransferPayload::TarShard { headers } => Some(headers),
                _ => None,
            })
            .collect();
        assert!(shards.len() > 1, "an oversized shard must split");
        let mut flat: Vec<String> = Vec::new();
        for shard in &shards {
            let encoded: usize = shard.iter().map(|h| h.encoded_len() + 5).sum();
            assert!(
                encoded <= 512,
                "each split shard's encoded members fit the bound (got {encoded})"
            );
            flat.extend(shard.iter().map(|h| h.relative_path.clone()));
        }
        assert_eq!(flat, expected, "order and file set preserved");
        assert!(
            matches!(out.last(), Some(TransferPayload::File(h)) if h.relative_path == "plain.bin"),
            "non-shard payloads pass through in order"
        );

        // Under the real bound the same shard stays whole.
        let out = bound_in_stream_tar_headers(
            vec![TransferPayload::TarShard { headers }],
            MAX_IN_STREAM_TAR_HEADER_BYTES,
        );
        assert_eq!(out.len(), 1, "a small shard passes through whole");

        // A single header over the bound is still emitted, alone —
        // there is nothing below one file to split to.
        let out = bound_in_stream_tar_headers(
            vec![TransferPayload::TarShard {
                headers: vec![tar_test_header("x".repeat(600))],
            }],
            512,
        );
        assert_eq!(out.len(), 1);
    }

    /// codex otp-8 F2, the wiring guard: `send_payload_records` itself
    /// must emit multiple `TarShardHeader` frames — each under the
    /// in-stream bound — when the planner hands it ONE shard whose
    /// header list would exceed it. 4096 one-byte files (the planner's
    /// per-shard count ceiling, forced into a single shard) with
    /// ~600-byte relative paths encode past the 2 MiB bound. Reverting
    /// the `bound_in_stream_tar_headers` call makes this fail on a
    /// single oversized frame.
    #[tokio::test]
    async fn in_stream_send_splits_oversized_tar_header_frames() {
        use prost::Message;
        use std::sync::Mutex as StdMutex2;

        struct CaptureTx(Arc<StdMutex2<Vec<TransferFrame>>>);
        #[async_trait::async_trait]
        impl FrameTx for CaptureTx {
            async fn send(&mut self, frame: TransferFrame) -> Result<()> {
                self.0.lock().expect("capture lock").push(frame);
                Ok(())
            }
        }

        let tmp = tempfile::tempdir().expect("tempdir");
        let deep = format!("{}/{}", "a".repeat(200), "b".repeat(200));
        std::fs::create_dir_all(tmp.path().join(&deep)).expect("deep dir");
        let mut batch: Vec<FileHeader> = Vec::with_capacity(4096);
        for i in 0..4096 {
            let rel = format!("{deep}/{:0>190}", i);
            std::fs::write(tmp.path().join(&rel), b"x").expect("write file");
            batch.push(tar_test_header(rel));
        }
        let encoded_total: usize = batch.iter().map(|h| h.encoded_len() + 5).sum();
        assert!(
            encoded_total > MAX_IN_STREAM_TAR_HEADER_BYTES,
            "fixture must exceed the bound to exercise the split (got {encoded_total})"
        );

        let frames: Arc<StdMutex2<Vec<TransferFrame>>> = Arc::default();
        let mut tx: Box<dyn FrameTx> = Box::new(CaptureTx(Arc::clone(&frames)));
        let source: Arc<dyn TransferSource> = Arc::new(
            crate::remote::transfer::source::FsTransferSource::new(tmp.path().to_path_buf()),
        );
        let plan_options = PlanOptions {
            force_tar: true,
            small_count_target: Some(4096),
            ..PlanOptions::default()
        };
        let mut read_buf = vec![0u8; IN_STREAM_CHUNK];
        send_payload_records(
            &mut tx,
            &source,
            plan_options,
            batch,
            &mut read_buf,
            None,
            None,
        )
        .await
        .expect("in-stream send succeeds");

        let frames = frames.lock().expect("capture lock");
        let shard_headers: Vec<&TarShardHeader> = frames
            .iter()
            .filter_map(|f| match &f.frame {
                Some(Frame::TarShardHeader(h)) => Some(h),
                _ => None,
            })
            .collect();
        assert!(
            shard_headers.len() > 1,
            "the oversized planner shard must split into multiple header frames"
        );
        let mut total_files = 0usize;
        for header in &shard_headers {
            assert!(
                header.encoded_len() <= MAX_IN_STREAM_TAR_HEADER_BYTES + 16,
                "every TarShardHeader frame stays under the in-stream bound (got {})",
                header.encoded_len()
            );
            total_files += header.files.len();
        }
        assert_eq!(total_files, 4096, "no file lost or duplicated by the split");
    }
}
