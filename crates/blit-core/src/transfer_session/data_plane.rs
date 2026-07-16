//! Session-side TCP data-plane orchestration (otp-4b).
//!
//! The unified session reuses blit-core's data-plane byte plumbing —
//! [`DataPlaneSession`] record framing, [`execute_receive_pipeline`],
//! [`execute_sink_pipeline_streaming`], [`dial_data_plane`] — but owns
//! its OWN choreography here. The per-direction drivers (the old
//! daemon push service loop and `remote::push::client` send driver)
//! were deleted at cutover (otp-10c-2); this file is the one
//! choreography.
//!
//! Two orthogonal axes (otp-5b): the **connection role** — the RESPONDER
//! binds+accepts, the INITIATOR dials (NAT reality) — and the **byte
//! role** — the SOURCE sends, the DESTINATION receives. otp-4b wired the
//! push pair (DESTINATION responder accepts+receives; SOURCE initiator
//! dials+sends); otp-5b adds the pull pair (SOURCE responder accepts+
//! sends via [`accept_source_data_plane`]; DESTINATION initiator dials+
//! receives via [`dial_destination_data_plane`]). The byte machinery is
//! shared — send is `DataPlaneSession`/`DataPlaneSink`/the elastic
//! pipeline, receive is `execute_receive_pipeline` — only socket
//! acquisition differs per byte role. Because the grant is issued before
//! any manifest is seen, the zero-knowledge `initial_stream_proposal` is
//! 1 — the session data plane always starts single-stream (otp-4b-1) and
//! grows via resize in BOTH directions (push otp-4b-2, pull otp-5b-2).
//!
//! Mid-transfer growth (otp-4b-2 push, otp-5b-2 pull): the SOURCE owns a
//! [`TransferDial`] (bounded by the receiver's advertised capacity) and
//! drives the sf-2 shape correction — as the need list accumulates it
//! re-runs the shape table and proposes `DataPlaneResize{ADD}` (one stream
//! per epoch) on the control lane; the DESTINATION replies
//! `DataPlaneResizeAck` and grows its receive set. The control-lane frames
//! are identical in both directions — only the transport action flips
//! (the connection-initiating end always dials, the responder always
//! accepts): in push the SOURCE **initiator** dials the epoch-N socket and
//! the DESTINATION **responder** arms+accepts it; in pull the DESTINATION
//! **initiator** dials and the SOURCE **responder** accepts. Either way
//! the SOURCE hands its new send socket to the running elastic pipeline
//! through acknowledged member admission. The cheap-dial live tuner (chunk/prefetch) is
//! still future work — the resize moves only the stream count.

use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicU32, AtomicU64, Ordering};
use std::sync::{Arc, Mutex as StdMutex};

use async_trait::async_trait;
use eyre::Result;
use tokio::io::AsyncReadExt;
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::mpsc;
use tokio::task::JoinSet;

use crate::buffer::BufferPool;
use crate::dial::{initial_stream_proposal, local_receiver_capacity, TransferDial};
use crate::generated::{session_error::Code, CapacityProfile, DataPlaneGrant, FileHeader};
use crate::remote::transfer::payload::{PreparedPayload, TransferPayload};
use crate::remote::transfer::pipeline::execute_receive_pipeline_with_phase;
use crate::remote::transfer::session_phase::{BoundSessionPhaseTrace, SessionPhaseFields};
use crate::remote::transfer::sink::{DataPlaneSink, SinkOutcome, TransferSink};
use crate::remote::transfer::small_file_probe::{BoundSmallFileProbe, SmallFileCarrier};
use crate::remote::transfer::socket::{
    configure_data_socket, dial_data_plane, DATA_PLANE_ACCEPT_TIMEOUT, DATA_PLANE_TOKEN_TIMEOUT,
};
use crate::remote::transfer::source::TransferSource;
use crate::remote::transfer::stall_guard::{StallGuard, TRANSFER_STALL_TIMEOUT};
use crate::remote::transfer::{
    execute_sink_pipeline_elastic, generate_sub_token, AbortOnDrop, DataPlaneSession,
    ElasticPipelineControl, ElasticPipelineOutcome, MembershipOutcome, RemoteTransferProgress,
    SinkMember, StreamId, StreamProbeRegistry, SUB_TOKEN_LEN,
};

use super::{SessionFault, SourceInstruments};

/// The set of granted-but-not-yet-received needs, shared between the
/// destination's control loop (which inserts each path before sending
/// its `NeedBatch`) and the data-plane receive (which claims each path
/// as its payload lands). Completion is an empty set — the same signal
/// the in-stream carrier uses via its inline `outstanding.remove`.
pub(super) type OutstandingNeeds = Arc<StdMutex<HashSet<String>>>;

/// Headers of resume-granted needs (otp-7a/7b), keyed by relative path
/// and retained until the grant's block record completes. Shared
/// between the destination's control loop (which inserts each header
/// before sending that file's `BlockHashList`, and claims it inline on
/// the in-stream carrier) and the data-plane receive (which validates
/// and claims it as block records land on the sockets) — the same
/// sharing shape as [`OutstandingNeeds`].
pub(super) type ResumeHeaders = Arc<StdMutex<HashMap<String, FileHeader>>>;

/// otp-7b: the resume half of the data-plane receive contract — present
/// only when the session negotiated resume. `headers` is the shared
/// grant map above; `resumed` is the destination's `files_resumed`
/// counter, incremented here because the control loop never sees
/// data-plane block records.
pub(super) struct ResumeRecv {
    pub(super) headers: ResumeHeaders,
    pub(super) resumed: Arc<AtomicU64>,
}

fn dp_fault(msg: impl Into<String>) -> eyre::Report {
    eyre::Report::new(SessionFault::refusal(Code::DataPlaneFailed, msg))
}

/// [`dp_fault`] for failures that stringify an underlying I/O-bearing
/// report (socket dials): carry the `io::ErrorKind` so the retry
/// classifier still sees a transient transport condition (codex
/// otp-10a F5).
fn dp_fault_io(err: &eyre::Report, msg: impl Into<String>) -> eyre::Report {
    eyre::Report::new(SessionFault::refusal(Code::DataPlaneFailed, msg).with_io_kind_from(err))
}

// ---------------------------------------------------------------------------
// Responder (DESTINATION) — bind, grant, accept, receive
// ---------------------------------------------------------------------------

/// A bound data-plane listener plus the credentials the responder
/// advertises in its `SessionAccept`. Held by the responder driver
/// across the handshake so the accept loop can run after establish.
pub(super) struct ResponderDataPlane {
    listener: TcpListener,
    session_token: Vec<u8>,
    epoch0_sub_token: Vec<u8>,
    initial_streams: u32,
    port: u16,
}

/// Bind a data-plane listener and mint credentials for the grant. Any
/// failure (bind, addr, RNG) logs and returns `None` — the caller then
/// issues a grant-less `SessionAccept` and the session falls back to the
/// in-stream carrier (contract §Transport selection: a responder that
/// cannot bind grants no data plane).
pub(super) async fn prepare_responder_data_plane() -> Option<ResponderDataPlane> {
    let listener = match TcpListener::bind(("0.0.0.0", 0)).await {
        Ok(listener) => listener,
        Err(err) => {
            log::warn!("session data-plane bind failed, using in-stream carrier: {err:#}");
            return None;
        }
    };
    let port = match listener.local_addr() {
        Ok(addr) => addr.port(),
        Err(err) => {
            log::warn!("session data-plane local_addr failed, using in-stream carrier: {err:#}");
            return None;
        }
    };
    // Two independent 16-byte credentials (contract §Transport: a socket
    // opens with session_token ‖ epoch0_sub_token). `generate_sub_token`
    // is the fallible-RNG minter — a missing system RNG is an error, not
    // a weaker credential.
    let session_token = match generate_sub_token() {
        Ok(token) => token,
        Err(err) => {
            log::warn!("session data-plane token RNG failed, using in-stream carrier: {err:#}");
            return None;
        }
    };
    let epoch0_sub_token = match generate_sub_token() {
        Ok(token) => token,
        Err(err) => {
            log::warn!("session data-plane sub-token RNG failed, using in-stream carrier: {err:#}");
            return None;
        }
    };
    // The grant is issued before any manifest is seen, so the proposal
    // has zero knowledge: initial_streams == 1. All growth is via resize
    // (otp-4b-2). The ceiling is this end's own advertised max_streams.
    let ceiling = local_receiver_capacity().max_streams.max(1) as usize;
    let initial_streams = initial_stream_proposal(0, 0, ceiling).max(1);
    Some(ResponderDataPlane {
        listener,
        session_token,
        epoch0_sub_token,
        initial_streams,
        port,
    })
}

/// Aggregated destination-side receive result: the write outcome plus
/// the number of data sockets accepted (epoch-0 + accepted resizes),
/// which IS the settled live stream count this end observed. The sf-2
/// pin reads it through [`super::DestinationOutcome::data_plane_streams`].
pub(super) struct ReceiveTotals {
    pub(super) outcome: SinkOutcome,
    pub(super) streams: usize,
}

/// Live handle to a running responder data plane. The control loop arms
/// resize credentials through [`Self::arm`] and joins the accept loop at
/// `SourceDone` via [`Self::finish`].
pub(super) struct ResponderDataPlaneRun {
    arm_tx: mpsc::UnboundedSender<ResizeArm>,
    task: AbortOnDrop<Result<ReceiveTotals>>,
    /// The `session_token` half of every socket credential (the control
    /// loop does not need it, but keeping it here documents the shape).
    #[allow(dead_code)]
    session_token: Vec<u8>,
    /// The receiver's advertised `max_streams` — the control loop refuses
    /// a resize that would grow past it (defense in depth; the source's
    /// dial already clamps to the same ceiling).
    pub(super) ceiling: usize,
}

struct ResizeArm {
    epoch: u32,
    sub_token: Vec<u8>,
}

impl ResponderDataPlane {
    pub(super) fn session_token(&self) -> &[u8] {
        &self.session_token
    }

    /// The `DataPlaneGrant` this responder advertises in `SessionAccept`.
    pub(super) fn grant(&self) -> DataPlaneGrant {
        DataPlaneGrant {
            tcp_port: self.port as u32,
            session_token: self.session_token.clone(),
            initial_streams: self.initial_streams,
            epoch0_sub_token: self.epoch0_sub_token.clone(),
        }
    }

    /// The epoch-0 stream count this responder granted (always 1 — the
    /// zero-knowledge proposal). The control loop seeds its `resize_live`
    /// counter from it.
    pub(super) fn initial_streams(&self) -> u32 {
        self.initial_streams
    }

    /// Spawn the accept+receive loop and return a live handle. The loop
    /// accepts the epoch-0 socket(s) immediately, then accepts one more
    /// socket per armed resize credential until the control loop signals
    /// `SourceDone` (drops the arm sender) and every receive worker has
    /// drained its END. Runs concurrently with the control-stream diff
    /// loop; the DESTINATION is the scorer, so it returns the totals.
    pub(super) fn spawn(
        self,
        sink: Arc<dyn TransferSink>,
        progress: Option<RemoteTransferProgress>,
        phase_trace: Option<BoundSessionPhaseTrace>,
        small_file_probe: Option<BoundSmallFileProbe>,
    ) -> ResponderDataPlaneRun {
        let ceiling = local_receiver_capacity().max_streams.max(1) as usize;
        let session_token = self.session_token.clone();
        let (arm_tx, arm_rx) = mpsc::unbounded_channel::<ResizeArm>();
        let task = AbortOnDrop::new(tokio::spawn(self.accept_loop(
            sink,
            progress,
            phase_trace,
            small_file_probe,
            arm_rx,
        )));
        ResponderDataPlaneRun {
            arm_tx,
            task,
            session_token,
            ceiling,
        }
    }

    async fn accept_loop(
        self,
        sink: Arc<dyn TransferSink>,
        progress: Option<RemoteTransferProgress>,
        phase_trace: Option<BoundSessionPhaseTrace>,
        small_file_probe: Option<BoundSmallFileProbe>,
        arm_rx: mpsc::UnboundedReceiver<ResizeArm>,
    ) -> Result<ReceiveTotals> {
        // Epoch-0 socket credential: session_token ‖ epoch0_sub_token.
        let mut epoch0 = self.session_token.clone();
        epoch0.extend_from_slice(&self.epoch0_sub_token);

        let mut receives: JoinSet<Result<SinkOutcome>> = JoinSet::new();
        let mut total = SinkOutcome::default();
        let mut streams = 0usize;

        // Accept the initial epoch-0 socket(s) first (the zero-knowledge
        // grant is always 1; the loop handles N for symmetry).
        for socket_id in 0..self.initial_streams {
            if let Some(trace) = &phase_trace {
                trace.event(
                    "socket_accept_begin",
                    SessionPhaseFields {
                        epoch: Some(0),
                        socket: Some(socket_id),
                        ..Default::default()
                    },
                );
            }
            let socket = accept_authenticated(&self.listener, &epoch0).await?;
            if let Some(trace) = &phase_trace {
                trace.event(
                    "socket_accept_end",
                    SessionPhaseFields {
                        epoch: Some(0),
                        socket: Some(socket_id),
                        ..Default::default()
                    },
                );
            }
            streams += 1;
            spawn_receive(
                &mut receives,
                socket,
                &sink,
                progress.clone(),
                phase_trace.clone(),
                small_file_probe.clone(),
                0,
                socket_id,
            );
        }

        // Resize ADDs: each arms a `session_token ‖ sub_token` credential
        // whose socket the SOURCE dials right after its ack. `no_more` is
        // set when the control loop drops the arm sender at `SourceDone`;
        // the loop then drains the last armed sockets and workers. Because
        // the SOURCE only dials a credential it was acked for (and a dial
        // failure faults the whole session, aborting this task via
        // AbortOnDrop), an armed slot is always consumed — no orphan hang.
        let mut armed: Vec<ResizeArm> = Vec::new();
        let mut arm_rx = Some(arm_rx);
        let mut no_more = false;
        loop {
            if no_more && armed.is_empty() && receives.is_empty() {
                break;
            }
            // A closed arm channel resolves `recv()` instantly to `None`
            // every poll; parking it on `pending()` once closed keeps the
            // biased select from starving the accept/join arms (otherwise
            // the None arm wins every race and the loop spins without ever
            // collecting a finished worker).
            let arm_recv = async {
                match arm_rx.as_mut() {
                    Some(rx) => rx.recv().await,
                    None => std::future::pending().await,
                }
            };
            tokio::select! {
                biased;
                // Control FIRST: an arm must register before its socket
                // (which the SOURCE dials only after the ack the control
                // loop sends right after arming), so the accept arm below
                // always sees a populated `armed` set.
                arm = arm_recv => match arm {
                    Some(arm) => {
                        if let Some(trace) = &phase_trace {
                            trace.event(
                                "resize_arm_ready",
                                SessionPhaseFields {
                                    epoch: Some(arm.epoch),
                                    ..Default::default()
                                },
                            );
                            trace.event(
                                "socket_accept_begin",
                                SessionPhaseFields {
                                    epoch: Some(arm.epoch),
                                    socket: Some(0),
                                    ..Default::default()
                                },
                            );
                        }
                        armed.push(arm);
                    }
                    // Arm sender dropped at SourceDone: no more resizes.
                    None => {
                        arm_rx = None;
                        no_more = true;
                    }
                },
                // Accept only when a resize credential is armed. `accept`
                // is cancel-safe, so losing this arm to another (its
                // pending connection stays queued) drops no socket. The
                // credential read happens OUTSIDE the select (below) so a
                // select cancel can never truncate a half-read socket.
                accepted = accept_raw(&self.listener), if !armed.is_empty() => {
                    let socket = accepted?;
                    let (socket, epoch) =
                        authenticate_resize(socket, &self.session_token, &mut armed).await?;
                    if let Some(trace) = &phase_trace {
                        trace.event(
                            "socket_accept_end",
                            SessionPhaseFields {
                                epoch: Some(epoch),
                                socket: Some(0),
                                ..Default::default()
                            },
                        );
                    }
                    streams += 1;
                    spawn_receive(
                        &mut receives,
                        socket,
                        &sink,
                        progress.clone(),
                        phase_trace.clone(),
                        small_file_probe.clone(),
                        epoch,
                        0,
                    );
                }
                joined = receives.join_next(), if !receives.is_empty() => {
                    let outcome = joined
                        .expect("join_next is None only when empty, guarded above")
                        .map_err(|err| dp_fault(format!("receive task panicked: {err}")))??;
                    total.files_written += outcome.files_written;
                    total.bytes_written += outcome.bytes_written;
                }
            }
        }
        Ok(ReceiveTotals {
            outcome: total,
            streams,
        })
    }
}

impl ResponderDataPlaneRun {
    /// Arm a resize credential so the next socket presenting
    /// `session_token ‖ sub_token` is accepted. Returns false if the
    /// accept loop is gone (its receiver dropped) — the control loop then
    /// acks the resize as refused.
    pub(super) fn arm(&self, epoch: u32, sub_token: Vec<u8>) -> bool {
        self.arm_tx.send(ResizeArm { epoch, sub_token }).is_ok()
    }

    /// Signal `SourceDone` (no more resizes) and join the accept loop for
    /// the aggregated receive totals.
    pub(super) async fn finish(self) -> Result<ReceiveTotals> {
        let ResponderDataPlaneRun { arm_tx, task, .. } = self;
        // Dropping the arm sender is the "no more resizes" signal.
        drop(arm_tx);
        task.join()
            .await
            .map_err(|err| dp_fault(format!("data-plane receive task panicked: {err}")))?
    }
}

/// Spawn one receive worker draining `socket` into `sink` via the shared
/// receive pipeline, guarded by the transfer stall timeout (carried REV4
/// RELIABLE invariant, matching the old push receive: a peer that
/// authenticates then stalls mid-record trips the stall timeout rather
/// than pinning the task until TCP keepalive).
fn spawn_receive(
    receives: &mut JoinSet<Result<SinkOutcome>>,
    socket: TcpStream,
    sink: &Arc<dyn TransferSink>,
    progress: Option<RemoteTransferProgress>,
    phase_trace: Option<BoundSessionPhaseTrace>,
    small_file_probe: Option<BoundSmallFileProbe>,
    epoch: u32,
    socket_id: u32,
) {
    if let Some(trace) = &phase_trace {
        trace.event(
            "socket_trace_attached",
            SessionPhaseFields {
                epoch: Some(epoch),
                socket: Some(socket_id),
                ..Default::default()
            },
        );
    }
    let sink = Arc::clone(sink);
    receives.spawn(async move {
        let mut guarded = StallGuard::new(socket, TRANSFER_STALL_TIMEOUT);
        execute_receive_pipeline_with_phase(
            &mut guarded,
            sink,
            progress.as_ref(),
            phase_trace,
            small_file_probe,
            epoch,
            socket_id,
        )
        .await
    });
}

/// Accept one data socket under the shared bounded-accept timeout and
/// apply the data-plane socket policy. Cancel-safe (the accept itself is;
/// no bytes are read here).
async fn accept_raw(listener: &TcpListener) -> Result<TcpStream> {
    let accept = tokio::time::timeout(DATA_PLANE_ACCEPT_TIMEOUT, listener.accept()).await;
    let socket = match accept {
        Ok(Ok((socket, _peer))) => socket,
        Ok(Err(err)) => return Err(dp_fault(format!("data-plane accept failed: {err}"))),
        Err(_) => {
            return Err(dp_fault(format!(
            "data-plane accept timed out after {DATA_PLANE_ACCEPT_TIMEOUT:?} (source never dialed)"
        )))
        }
    };
    configure_data_socket(&socket, None)
        .map_err(|err| dp_fault(format!("configuring accepted data socket: {err}")))?;
    Ok(socket)
}

/// Read the fixed-length epoch-0 credential and verify it whole. A socket
/// presenting anything else is a `DATA_PLANE_FAILED` fault (the session
/// arms exactly the sockets it dials, so a mismatch is fatal here).
async fn accept_authenticated(listener: &TcpListener, expected: &[u8]) -> Result<TcpStream> {
    let mut socket = accept_raw(listener).await?;
    let mut buf = vec![0u8; expected.len()];
    let read = tokio::time::timeout(DATA_PLANE_TOKEN_TIMEOUT, socket.read_exact(&mut buf)).await;
    match read {
        Ok(Ok(_)) => {}
        Ok(Err(err)) => return Err(dp_fault(format!("reading data-plane credential: {err}"))),
        Err(_) => {
            return Err(dp_fault(format!(
                "data-plane credential read timed out after {DATA_PLANE_TOKEN_TIMEOUT:?}"
            )))
        }
    }
    // Constant-time comparison is not required: the tokens are 16 random
    // bytes read once per socket, single-session; a timing oracle buys
    // nothing against per-transfer secrets (same posture as the old push
    // acceptor's `token == expected_token`).
    if buf != expected {
        return Err(dp_fault(
            "data-plane socket presented an invalid credential",
        ));
    }
    Ok(socket)
}

/// Read a resize socket's `session_token ‖ sub_token(16)` credential
/// (bounded), verify the session token, and match the sub-token against
/// an armed credential — removing it so each arm is consumed once. Runs
/// in the accept loop body (never a select arm), so a select cancel can
/// never truncate a half-read socket.
async fn authenticate_resize(
    socket: TcpStream,
    session_token: &[u8],
    armed: &mut Vec<ResizeArm>,
) -> Result<(TcpStream, u32)> {
    let mut socket = socket;
    let mut buf = vec![0u8; session_token.len() + SUB_TOKEN_LEN];
    let read = tokio::time::timeout(DATA_PLANE_TOKEN_TIMEOUT, socket.read_exact(&mut buf)).await;
    match read {
        Ok(Ok(_)) => {}
        Ok(Err(err)) => {
            return Err(dp_fault(format!(
                "reading resize data-plane credential: {err}"
            )))
        }
        Err(_) => {
            return Err(dp_fault(format!(
                "resize data-plane credential read timed out after {DATA_PLANE_TOKEN_TIMEOUT:?}"
            )))
        }
    }
    if buf[..session_token.len()] != *session_token {
        return Err(dp_fault(
            "resize data socket presented a wrong session token",
        ));
    }
    let sub = &buf[session_token.len()..];
    match armed.iter().position(|arm| arm.sub_token.as_slice() == sub) {
        Some(idx) => {
            let epoch = armed.swap_remove(idx).epoch;
            Ok((socket, epoch))
        }
        None => Err(dp_fault(
            "resize data socket presented an unarmed credential",
        )),
    }
}

// ---------------------------------------------------------------------------
// Initiator (DESTINATION) — dial, receive (otp-5b-1)
// ---------------------------------------------------------------------------

/// Live handle to a DESTINATION **initiator** receive data plane (the
/// pull direction): the initiator dials the granted epoch-0 socket(s) and
/// drains each into the sink through the shared receive pipeline — the
/// same byte machinery the DESTINATION responder uses, only the socket is
/// dialed instead of accepted. Resize (otp-5b-2): on a `DataPlaneResize`
/// the control loop dials one more epoch-N socket via
/// [`Self::add_dialed_stream`] (the pull mirror of the SOURCE responder's
/// accept). [`Self::finish`] joins the workers for the aggregated write
/// outcome + settled stream count.
pub(super) struct InitiatorReceivePlaneRun {
    receives: JoinSet<Result<SinkOutcome>>,
    streams: usize,
    /// The responder host+port and session token, retained so a resize can
    /// dial another receive socket to the same listener (otp-5b-2). The
    /// DESTINATION initiator always dials; the SOURCE responder accepts.
    host: String,
    tcp_port: u32,
    session_token: Vec<u8>,
    /// The shared need-list receive sink each dialed worker drains into.
    sink: Arc<dyn TransferSink>,
    /// w6-1 progress lane each receive worker reports into (otp-10b-2);
    /// cloned per worker, including resize-added ones.
    progress: Option<RemoteTransferProgress>,
    /// `[data-plane-client]` connect traces (`--trace-data-plane`,
    /// otp-10b-2). Applied to the epoch-0 dials at construction and to
    /// each epoch-N resize dial in [`Self::add_dialed_stream`].
    trace: bool,
    phase_trace: Option<BoundSessionPhaseTrace>,
    small_file_probe: Option<BoundSmallFileProbe>,
}

/// Dial the granted epoch-0 socket(s) and spawn one receive worker per
/// socket. `host` is the responder's host (the initiator reached the
/// control plane there; the data plane rides the same host on the granted
/// port — contract §Transport: the initiator always dials). Each worker
/// drains its socket into `sink` (a [`NeedListSink`], same strictness the
/// in-stream carrier applies inline).
pub(super) async fn dial_destination_data_plane(
    host: &str,
    grant: &DataPlaneGrant,
    sink: Arc<dyn TransferSink>,
    progress: Option<RemoteTransferProgress>,
    trace: bool,
    phase_trace: Option<BoundSessionPhaseTrace>,
    small_file_probe: Option<BoundSmallFileProbe>,
) -> Result<InitiatorReceivePlaneRun> {
    let initial = grant.initial_streams.max(1) as usize;
    // Epoch-0 handshake: session_token ‖ epoch0_sub_token.
    let mut handshake = grant.session_token.clone();
    handshake.extend_from_slice(&grant.epoch0_sub_token);
    let addr = format!("{host}:{}", grant.tcp_port);

    let mut receives: JoinSet<Result<SinkOutcome>> = JoinSet::new();
    let mut streams = 0usize;
    for socket_id in 0..initial {
        // `dial_data_plane` connects, applies the data-socket policy, and
        // writes the handshake credential — the same bounded dial the
        // SOURCE initiator uses (design-3: one owner for every client-side
        // data-plane dial, both directions).
        if trace {
            eprintln!("[data-plane-client] connecting to {addr} (receive)");
        }
        if let Some(phase) = &phase_trace {
            phase.event(
                "socket_dial_begin",
                SessionPhaseFields {
                    epoch: Some(0),
                    socket: Some(socket_id as u32),
                    ..Default::default()
                },
            );
        }
        let socket = dial_data_plane(&addr, &handshake, None)
            .await
            .map_err(|err| {
                dp_fault_io(
                    &err,
                    format!("dialing session data plane (receive): {err:#}"),
                )
            })?;
        if let Some(phase) = &phase_trace {
            phase.event(
                "socket_dial_end",
                SessionPhaseFields {
                    epoch: Some(0),
                    socket: Some(socket_id as u32),
                    ..Default::default()
                },
            );
        }
        streams += 1;
        spawn_receive(
            &mut receives,
            socket,
            &sink,
            progress.clone(),
            phase_trace.clone(),
            small_file_probe.clone(),
            0,
            socket_id as u32,
        );
    }
    Ok(InitiatorReceivePlaneRun {
        receives,
        streams,
        host: host.to_string(),
        tcp_port: grant.tcp_port,
        session_token: grant.session_token.clone(),
        sink,
        progress,
        trace,
        phase_trace,
        small_file_probe,
    })
}

impl InitiatorReceivePlaneRun {
    /// Dial one epoch-N resize socket to the responder and spawn its
    /// receive worker (otp-5b-2 — the pull mirror of the SOURCE
    /// responder's accept). Credential `session_token ‖ sub_token`. A dial
    /// failure is FATAL, matching the SOURCE initiator's `add_stream`: a
    /// same-build peer that granted+bound epoch-0 failing an epoch-N dial
    /// is a transport fault worth surfacing (the DESTINATION dials before
    /// it acks, so a failure faults the session before the SOURCE
    /// responder commits to accepting the socket).
    pub(super) async fn add_dialed_stream(&mut self, epoch: u32, sub_token: &[u8]) -> Result<()> {
        let mut handshake = self.session_token.clone();
        handshake.extend_from_slice(sub_token);
        let addr = format!("{}:{}", self.host, self.tcp_port);
        if self.trace {
            eprintln!("[data-plane-client] connecting to {addr} (receive resize)");
        }
        if let Some(phase) = &self.phase_trace {
            phase.event(
                "socket_dial_begin",
                SessionPhaseFields {
                    epoch: Some(epoch),
                    socket: Some(0),
                    ..Default::default()
                },
            );
        }
        let socket = dial_data_plane(&addr, &handshake, None)
            .await
            .map_err(|err| {
                dp_fault_io(
                    &err,
                    format!("dialing resize data plane (receive): {err:#}"),
                )
            })?;
        if let Some(phase) = &self.phase_trace {
            phase.event(
                "socket_dial_end",
                SessionPhaseFields {
                    epoch: Some(epoch),
                    socket: Some(0),
                    ..Default::default()
                },
            );
        }
        self.streams += 1;
        spawn_receive(
            &mut self.receives,
            socket,
            &self.sink,
            self.progress.clone(),
            self.phase_trace.clone(),
            self.small_file_probe.clone(),
            epoch,
            0,
        );
        Ok(())
    }

    /// Join every receive worker for the aggregated write totals. A worker
    /// error (receive failure / stall) surfaces here; each drains to its
    /// socket's END record on a clean transfer.
    async fn finish(mut self) -> Result<ReceiveTotals> {
        let mut total = SinkOutcome::default();
        while let Some(joined) = self.receives.join_next().await {
            let outcome =
                joined.map_err(|err| dp_fault(format!("receive task panicked: {err}")))??;
            total.files_written += outcome.files_written;
            total.bytes_written += outcome.bytes_written;
        }
        Ok(ReceiveTotals {
            outcome: total,
            streams: self.streams,
        })
    }
}

/// The DESTINATION end's receive data plane, tagged by connection role.
/// Both drain socket bytes into the sink through the same receive
/// pipeline; they differ only in how sockets are obtained (accept vs dial)
/// and whether resize is armable (push only, otp-4b-2).
pub(super) enum DestRecvPlane {
    /// DESTINATION **responder** (push, otp-4b): accepts sockets; resize
    /// grows the set by arming a credential its accept loop then accepts.
    Responder(ResponderDataPlaneRun),
    /// DESTINATION **initiator** (pull, otp-5b): dials sockets; resize grows
    /// the set by dialing one more epoch-N socket (otp-5b-2).
    Initiator(InitiatorReceivePlaneRun),
}

impl DestRecvPlane {
    /// Drain the data plane to completion and report the settled stream
    /// count + write outcome (the DESTINATION is the scorer).
    pub(super) async fn finish(self) -> Result<ReceiveTotals> {
        match self {
            DestRecvPlane::Responder(run) => run.finish().await,
            DestRecvPlane::Initiator(run) => run.finish().await,
        }
    }
}

// ---------------------------------------------------------------------------
// Initiator (SOURCE) — dial, authenticate, send, resize
// ---------------------------------------------------------------------------

/// A resize the SOURCE has proposed and minted a credential for but not
/// yet completed: the driver has sent (or will send) the matching
/// `DataPlaneResize{ADD}` on the control lane and, on the peer's
/// `DataPlaneResizeAck`, dials the epoch-N socket. At most one is in
/// flight (the dial's `pending_epoch` enforces it; this is the
/// driver-side record the ack is matched against).
pub(super) struct PendingResize {
    pub(super) epoch: u32,
    pub(super) target_streams: u32,
    pub(super) sub_token: Vec<u8>,
}

/// How the SOURCE acquires each epoch-N data socket for a shape resize —
/// the two connection roles of otp-5b. Byte direction is identical (the
/// SOURCE sends), and `propose_resize` is the same either way; only socket
/// acquisition flips.
enum SourceSockets {
    /// SOURCE **initiator** (push, otp-4b-2): dials each epoch-N socket to
    /// the granted host:port.
    Dial { host: String, tcp_port: u32 },
    /// SOURCE **responder** (pull, otp-5b-2): accepts each epoch-N socket
    /// off the listener it already bound for epoch-0, credential
    /// `session_token ‖ sub_token`.
    Accept { listener: TcpListener },
}

/// A running source-side data plane: the dialed/accepted socket(s) wrapped
/// as an ELASTIC sink pipeline whose acknowledged membership control grows
/// it mid-run (the sf-2 shape correction). Planned payloads are fed via [`Self::queue`];
/// closing via [`Self::finish`] drains the pipeline, emits each socket's
/// END record, and returns the bytes this end sent.
pub(super) struct SourceDataPlane {
    payload_tx: Option<mpsc::Sender<TransferPayload>>,
    control: Option<ElasticPipelineControl>,
    // `AbortOnDrop<T>` wraps a `JoinHandle<T>`; the task's output is
    // `Result<ElasticPipelineOutcome>`, so `T` is that (not the JoinHandle).
    pipeline: Option<AbortOnDrop<Result<ElasticPipelineOutcome>>>,
    // The byte SENDER owns the live dial, bounded by the byte RECEIVER's
    // advertised capacity (contract §Invariants 5). The resize drives only
    // its shape-correction stream count; the cheap-dial tuner is future
    // work, so `chunk_bytes()`/`prefetch_count()` stay at the floor.
    dial: Arc<TransferDial>,
    next_member_id: AtomicU32,
    source: Arc<dyn TransferSource>,
    session_token: Vec<u8>,
    pool: Arc<BufferPool>,
    /// `[data-plane-client]` connect traces (`--trace-data-plane`,
    /// otp-10a). Applied to the epoch-0 sockets at construction and to
    /// each epoch-N resize socket in [`Self::add_stream`].
    trace: bool,
    /// How each epoch-N resize socket is acquired (dial for the SOURCE
    /// initiator, accept for the SOURCE responder). The data plane grows
    /// mid-transfer in both cases; the control-lane resize choreography is
    /// identical — only this transport action flips (otp-5b-2).
    sockets: SourceSockets,
    phase_trace: Option<BoundSessionPhaseTrace>,
    small_file_probe: Option<BoundSmallFileProbe>,
    queue_trace_armed: AtomicBool,
}

/// Dial the granted data plane and start the elastic send pipeline.
/// `host` is the responder's host (the initiator connected the control
/// plane to it; the data plane rides the same host on the granted port —
/// contract §Transport: the initiator always dials). `receiver_capacity`
/// is the DESTINATION's advertised profile from `SessionAccept`; it
/// bounds the sender's dial ceiling (0/absent fields ⇒ conservative,
/// never unlimited).
pub(super) async fn dial_source_data_plane(
    host: &str,
    grant: &DataPlaneGrant,
    receiver_capacity: Option<&CapacityProfile>,
    source: Arc<dyn TransferSource>,
    instruments: &SourceInstruments,
    phase_trace: Option<BoundSessionPhaseTrace>,
    small_file_probe: Option<BoundSmallFileProbe>,
) -> Result<SourceDataPlane> {
    let initial = grant.initial_streams.max(1) as usize;
    // The byte sender's dial, bounded by the receiver's advertised
    // capacity. Seed the settled live count to the granted epoch-0
    // streams — every shape-resize proposal steps from here.
    let dial = TransferDial::conservative_within(receiver_capacity).shared();
    dial.set_negotiated_streams(initial);

    // Epoch-0 handshake: session_token ‖ epoch0_sub_token.
    let mut handshake = grant.session_token.clone();
    handshake.extend_from_slice(&grant.epoch0_sub_token);

    // Provision the pool for the dial ceiling so resize-added sockets
    // draw buffers from the same pool without re-pooling (as old push
    // does — a shared pool sized for the maximum stream count).
    let pool = Arc::new(BufferPool::for_data_plane(
        dial.chunk_bytes(),
        dial.ceiling_max_streams().max(1),
    ));
    let trace = instruments.trace_data_plane;
    let mut sinks = Vec::with_capacity(initial);
    for socket_id in 0..initial {
        if let Some(phase) = &phase_trace {
            phase.event(
                "socket_dial_begin",
                SessionPhaseFields {
                    epoch: Some(0),
                    socket: Some(socket_id as u32),
                    ..Default::default()
                },
            );
        }
        let session = DataPlaneSession::connect(
            host,
            grant.tcp_port,
            &handshake,
            dial.chunk_bytes(),
            dial.prefetch_count(),
            trace,
            dial.tcp_buffer_bytes(),
            Arc::clone(&pool),
        )
        .await
        .map_err(|err| dp_fault_io(&err, format!("dialing session data plane: {err:#}")))?;
        if let Some(phase) = &phase_trace {
            phase.event(
                "socket_dial_end",
                SessionPhaseFields {
                    epoch: Some(0),
                    socket: Some(socket_id as u32),
                    ..Default::default()
                },
            );
        }
        let session = session.with_session_phase_trace(phase_trace.clone(), 0, socket_id as u32);
        // The source-side sink never reads its dst_root (it only sends);
        // `root()` is consulted by the relay/receive case, not here.
        let sink: Arc<dyn TransferSink> = Arc::new(DataPlaneSink::new(
            session,
            Arc::clone(&source),
            PathBuf::new(),
        ));
        sinks.push(SinkMember::new(StreamId(socket_id as u32), sink));
    }

    let prefetch = dial.prefetch_count().max(1);
    let (payload_tx, payload_rx) = mpsc::channel::<TransferPayload>(prefetch);
    let (control, commands) = ElasticPipelineControl::channel();
    let probes = Arc::new(StdMutex::new(StreamProbeRegistry::default()));
    let pipe_source = Arc::clone(&source);
    let pipe_progress = instruments.progress.clone();
    // Bounded by AbortOnDrop: a fault on the control lane that drops the
    // SourceDataPlane aborts the pipeline task instead of leaking it.
    let pipeline = AbortOnDrop::new(tokio::spawn(async move {
        execute_sink_pipeline_elastic(
            pipe_source,
            sinks,
            payload_rx,
            prefetch,
            pipe_progress.as_ref(),
            Some(commands),
            probes,
        )
        .await
    }));
    let queue_trace_armed = phase_trace.is_some();
    Ok(SourceDataPlane {
        payload_tx: Some(payload_tx),
        control: Some(control),
        pipeline: Some(pipeline),
        dial,
        next_member_id: AtomicU32::new(initial as u32),
        source,
        session_token: grant.session_token.clone(),
        pool,
        trace,
        // SOURCE initiator: each epoch-N resize socket is dialed to the
        // granted host:port.
        sockets: SourceSockets::Dial {
            host: host.to_string(),
            tcp_port: grant.tcp_port,
        },
        phase_trace,
        small_file_probe,
        queue_trace_armed: AtomicBool::new(queue_trace_armed),
    })
}

/// Accept the granted epoch-0 socket(s) off a bound responder listener and
/// start the elastic SEND pipeline over them — the SOURCE **responder**
/// half of the pull data plane (otp-5b-1). Symmetric with
/// [`dial_source_data_plane`] (the SOURCE **initiator** half): both return
/// a [`SourceDataPlane`] the send half drives via `queue`/`finish`; only
/// socket acquisition differs (accept here, dial there).
/// `DataPlaneSession::from_stream` builds a send session from an already-
/// accepted socket — the same primitive the old `pull_sync` daemon-send
/// path uses. `receiver_capacity` is the DESTINATION initiator's advertised
/// profile from its `SessionOpen` (the byte RECEIVER advertises capacity,
/// wherever it initiates). The bound listener is retained so each epoch-N
/// resize socket is accepted off it (otp-5b-2): the DESTINATION initiator
/// dials, this end accepts, the control-lane frames identical to push.
pub(super) async fn accept_source_data_plane(
    bound: ResponderDataPlane,
    receiver_capacity: Option<&CapacityProfile>,
    source: Arc<dyn TransferSource>,
    instruments: &SourceInstruments,
    phase_trace: Option<BoundSessionPhaseTrace>,
    small_file_probe: Option<BoundSmallFileProbe>,
) -> Result<SourceDataPlane> {
    let initial = bound.initial_streams.max(1) as usize;
    // The byte sender's dial, bounded by the receiver's advertised
    // capacity; seed the live count to the granted epoch-0 streams. Growth
    // is via resize (otp-5b-2): the accept-based epoch-N socket steps from
    // here, one stream per epoch, same as the SOURCE initiator.
    let dial = TransferDial::conservative_within(receiver_capacity).shared();
    dial.set_negotiated_streams(initial);

    // Epoch-0 credential the dialing DESTINATION presents:
    // session_token ‖ epoch0_sub_token (contract §Transport).
    let mut epoch0 = bound.session_token.clone();
    epoch0.extend_from_slice(&bound.epoch0_sub_token);

    let pool = Arc::new(BufferPool::for_data_plane(
        dial.chunk_bytes(),
        dial.ceiling_max_streams().max(1),
    ));
    let trace = instruments.trace_data_plane;
    let mut sinks = Vec::with_capacity(initial);
    for socket_id in 0..initial {
        if let Some(phase) = &phase_trace {
            phase.event(
                "socket_accept_begin",
                SessionPhaseFields {
                    epoch: Some(0),
                    socket: Some(socket_id as u32),
                    ..Default::default()
                },
            );
        }
        let socket = accept_authenticated(&bound.listener, &epoch0).await?;
        if let Some(phase) = &phase_trace {
            phase.event(
                "socket_accept_end",
                SessionPhaseFields {
                    epoch: Some(0),
                    socket: Some(socket_id as u32),
                    ..Default::default()
                },
            );
        }
        let session = DataPlaneSession::from_stream(
            socket,
            trace,
            dial.chunk_bytes(),
            dial.prefetch_count(),
            Arc::clone(&pool),
        )
        .await
        .with_session_phase_trace(phase_trace.clone(), 0, socket_id as u32);
        let sink: Arc<dyn TransferSink> = Arc::new(DataPlaneSink::new(
            session,
            Arc::clone(&source),
            PathBuf::new(),
        ));
        sinks.push(SinkMember::new(StreamId(socket_id as u32), sink));
    }

    let prefetch = dial.prefetch_count().max(1);
    let (payload_tx, payload_rx) = mpsc::channel::<TransferPayload>(prefetch);
    let (control, commands) = ElasticPipelineControl::channel();
    let probes = Arc::new(StdMutex::new(StreamProbeRegistry::default()));
    let pipe_source = Arc::clone(&source);
    let pipe_progress = instruments.progress.clone();
    let pipeline = AbortOnDrop::new(tokio::spawn(async move {
        execute_sink_pipeline_elastic(
            pipe_source,
            sinks,
            payload_rx,
            prefetch,
            pipe_progress.as_ref(),
            Some(commands),
            probes,
        )
        .await
    }));
    let queue_trace_armed = phase_trace.is_some();
    Ok(SourceDataPlane {
        payload_tx: Some(payload_tx),
        control: Some(control),
        pipeline: Some(pipeline),
        dial,
        next_member_id: AtomicU32::new(initial as u32),
        source,
        session_token: bound.session_token,
        pool,
        trace,
        // SOURCE responder: each epoch-N resize socket is accepted off the
        // same listener epoch-0 came in on (otp-5b-2).
        sockets: SourceSockets::Accept {
            listener: bound.listener,
        },
        phase_trace,
        small_file_probe,
        queue_trace_armed: AtomicBool::new(queue_trace_armed),
    })
}

impl SourceDataPlane {
    pub(super) fn phase_trace(&self) -> Option<&BoundSessionPhaseTrace> {
        self.phase_trace.as_ref()
    }

    /// The live dial (the byte sender owns it). The driver reads
    /// `live_streams()` for observability and calls `resize_settled` as
    /// each proposal completes.
    pub(super) fn dial(&self) -> &Arc<TransferDial> {
        &self.dial
    }

    /// sf-2 shape correction: propose one ADD toward the stream count the
    /// accumulated need list implies, if none is in flight and the shape
    /// wants more than the current live count. Mints the resize
    /// credential; the driver sends the `DataPlaneResize{ADD}` and hands
    /// the record back on the matching ack.
    pub(super) fn propose_resize(
        &self,
        needed_bytes: u64,
        needed_count: usize,
    ) -> Result<Option<PendingResize>> {
        let desired =
            initial_stream_proposal(needed_bytes, needed_count, self.dial.ceiling_max_streams())
                as usize;
        let Some(proposal) = self.dial.propose_shape_resize(desired) else {
            return Ok(None);
        };
        let sub_token = generate_sub_token()
            .map_err(|err| dp_fault(format!("minting resize sub-token: {err:#}")))?;
        Ok(Some(PendingResize {
            epoch: proposal.epoch,
            target_streams: proposal.target_streams as u32,
            sub_token,
        }))
    }

    /// Acquire the epoch-N data socket for an accepted resize and hand it
    /// to the running pipeline through acknowledged membership. The SOURCE initiator
    /// (push) DIALS it; the SOURCE responder (pull, otp-5b-2) ACCEPTS the
    /// socket the DESTINATION initiator dials after its ack, off the same
    /// listener epoch-0 came in on. A dial/accept failure is FATAL
    /// (fail-fast): a same-build peer that established epoch-0 failing an
    /// epoch-N socket is a transport fault worth surfacing — and faulting
    /// the session aborts the peer's counterpart via AbortOnDrop, so no
    /// slot orphans. (Old push recovers non-fatally via an arm TTL; the
    /// session trades that for simplicity — noted in the finding doc.) If
    /// the pipeline is already gone (transfer completing under the ADD),
    /// the just-acquired socket is closed cleanly so the peer's worker sees
    /// its END, not a reset.
    ///
    /// The accept is bounded and unambiguous: at most one resize is in
    /// flight (the driver's `pending_resize`) and epoch-0 is already
    /// accepted, so the next connection off the listener is exactly this
    /// resize's socket — verified against `session_token ‖ sub_token`.
    pub(super) async fn add_stream(
        &self,
        epoch: u32,
        sub_token: &[u8],
    ) -> Result<MembershipOutcome> {
        // Allocate before transport acquisition. A failed socket burns the
        // identity rather than reusing it for a different authenticated
        // connection later in the session.
        let member_id = StreamId(
            self.next_member_id
                .fetch_update(Ordering::Relaxed, Ordering::Relaxed, |next| {
                    next.checked_add(1)
                })
                .map_err(|_| dp_fault("data-plane stream member id exhausted"))?,
        );
        let session = match &self.sockets {
            SourceSockets::Dial { host, tcp_port } => {
                let mut handshake = self.session_token.clone();
                handshake.extend_from_slice(sub_token);
                if let Some(phase) = &self.phase_trace {
                    phase.event(
                        "socket_dial_begin",
                        SessionPhaseFields {
                            epoch: Some(epoch),
                            socket: Some(0),
                            ..Default::default()
                        },
                    );
                }
                let session = DataPlaneSession::connect(
                    host,
                    *tcp_port,
                    &handshake,
                    self.dial.chunk_bytes(),
                    self.dial.prefetch_count(),
                    self.trace,
                    self.dial.tcp_buffer_bytes(),
                    Arc::clone(&self.pool),
                )
                .await
                .map_err(|err| dp_fault_io(&err, format!("dialing resize data socket: {err:#}")))?;
                if let Some(phase) = &self.phase_trace {
                    phase.event(
                        "socket_dial_end",
                        SessionPhaseFields {
                            epoch: Some(epoch),
                            socket: Some(0),
                            ..Default::default()
                        },
                    );
                }
                session.with_session_phase_trace(self.phase_trace.clone(), epoch, 0)
            }
            SourceSockets::Accept { listener } => {
                let mut expected = self.session_token.clone();
                expected.extend_from_slice(sub_token);
                if let Some(phase) = &self.phase_trace {
                    phase.event(
                        "socket_accept_begin",
                        SessionPhaseFields {
                            epoch: Some(epoch),
                            socket: Some(0),
                            ..Default::default()
                        },
                    );
                }
                let socket = accept_authenticated(listener, &expected).await?;
                if let Some(phase) = &self.phase_trace {
                    phase.event(
                        "socket_accept_end",
                        SessionPhaseFields {
                            epoch: Some(epoch),
                            socket: Some(0),
                            ..Default::default()
                        },
                    );
                }
                DataPlaneSession::from_stream(
                    socket,
                    self.trace,
                    self.dial.chunk_bytes(),
                    self.dial.prefetch_count(),
                    Arc::clone(&self.pool),
                )
                .await
                .with_session_phase_trace(self.phase_trace.clone(), epoch, 0)
            }
        };
        let sink: Arc<dyn TransferSink> = Arc::new(DataPlaneSink::new(
            session,
            Arc::clone(&self.source),
            PathBuf::new(),
        ));
        self.control
            .as_ref()
            .ok_or_else(|| dp_fault("data-plane membership control already closed"))?
            .add(SinkMember::new(member_id, sink))
            .await
            .map_err(|err| dp_fault(format!("admitting data-plane member: {err:#}")))
    }

    /// Feed one planned payload into the bounded send pipeline. The caller
    /// selects this send against control events so resize acknowledgements
    /// keep growing the same work-stealing queue under backpressure.
    pub(super) async fn queue(&self, payload: TransferPayload) -> Result<()> {
        let tar_probe = self.small_file_probe.as_ref().and_then(|probe| {
            if let TransferPayload::TarShard { headers } = &payload {
                Some((probe, headers.len(), probe.start()))
            } else {
                None
            }
        });
        let tx = self.payload_tx.as_ref().ok_or_else(|| {
            eyre::Report::new(SessionFault::internal("data plane already finished"))
        })?;
        let result =
            if self.phase_trace.is_none() || !self.queue_trace_armed.load(Ordering::Relaxed) {
                tx.send(payload).await.map_err(|_| {
                    dp_fault("data-plane send pipeline closed before all payloads sent")
                })
            } else {
                let permit = tx.reserve().await.map_err(|_| {
                    dp_fault("data-plane send pipeline closed before all payloads sent")
                })?;
                let queued_at = if self.queue_trace_armed.load(Ordering::Relaxed)
                    && self
                        .queue_trace_armed
                        .compare_exchange(true, false, Ordering::AcqRel, Ordering::Relaxed)
                        .is_ok()
                {
                    self.phase_trace.as_ref().map(BoundSessionPhaseTrace::stamp)
                } else {
                    None
                };
                permit.send(payload);
                if let (Some(trace), Some(queued_at)) = (&self.phase_trace, queued_at) {
                    trace.first_payload_queued_at(queued_at);
                }
                Ok(())
            };
        if result.is_ok() {
            if let Some((probe, members, started)) = tar_probe {
                probe.note_tar_queue(started.elapsed(), members);
            }
        }
        result
    }

    /// Declare that every payload has been queued without waiting for the
    /// elastic workers to finish. Closing the input lets idle workers emit
    /// END promptly while the control lane settles any remaining resize
    /// epochs; a late ADD likewise sees a closed queue and emits END rather
    /// than waiting under the receiver's StallGuard.
    pub(super) fn close_payloads(&mut self) -> Result<()> {
        // The control command is sent first. Because this handle is the
        // sole command producer, every accepted late membership operation
        // is ordered after Seal even if its worker races the payload sender
        // being dropped below.
        let sealed = self
            .control
            .as_ref()
            .ok_or_else(|| dp_fault("data-plane membership control already closed"))?
            .seal();
        self.payload_tx = None;
        sealed.map_err(|err| dp_fault(format!("sealing data-plane membership: {err:#}")))
    }

    /// Signal end-of-stream, drain the pipeline (each worker emits its
    /// socket's END record on drain), and return the bytes sent. Must be
    /// awaited before `SourceDone` goes out so the destination's receive
    /// pipeline sees END and completes.
    pub(super) async fn finish(mut self) -> Result<SinkOutcome> {
        // Seal before dropping the payload sender, then drop the sole
        // command endpoint before joining. Retaining it here would keep a
        // zero-worker terminal supervisor alive forever.
        let close_result = self.close_payloads();
        let resize_pending = self.dial.resize_pending();
        drop(self.control.take());
        let pipeline = self
            .pipeline
            .take()
            .expect("SourceDataPlane::finish called once");
        let elastic = pipeline
            .join()
            .await
            .map_err(|err| dp_fault(format!("data-plane send pipeline panicked: {err}")))??;
        close_result?;
        if resize_pending {
            return Err(dp_fault(
                "data-plane membership sealed with a resize epoch still pending",
            ));
        }
        if elastic.logical_count != self.dial.live_streams() {
            return Err(dp_fault(format!(
                "data-plane membership ended at {} but dial settled {}",
                elastic.logical_count,
                self.dial.live_streams()
            )));
        }
        if let Some(trace) = &self.phase_trace {
            trace.event("data_plane_complete", SessionPhaseFields::default());
        }
        Ok(elastic.outcome)
    }
}

// ---------------------------------------------------------------------------
// Need-list enforcement for the data-plane receive
// ---------------------------------------------------------------------------

/// Sink decorator that enforces the session's need-list contract on the
/// data-plane receive, giving it the SAME strictness the in-stream
/// carrier applies inline in the control loop (`outstanding.remove`).
/// `execute_receive_pipeline` writes socket-provided paths directly, so
/// without this a peer could substitute an off-need-list path for a
/// needed one (count-preserving), duplicate one, or send resume block
/// records the session never negotiated (codex otp-4b-1 F1). Every
/// written path must be a granted, not-yet-received need. Resume
/// sessions (otp-7b) additionally validate + claim block records
/// against the shared [`ResumeHeaders`] grant map — with the identical
/// strictness the in-stream `claim_resume_record` applies — and count
/// completions into the shared resumed counter; in a non-resume session
/// block records are rejected outright. The shared [`OutstandingNeeds`]
/// set makes completion `is_empty()` for both carriers.
pub(super) struct NeedListSink {
    inner: Arc<dyn TransferSink>,
    outstanding: OutstandingNeeds,
    small_file_probe: Option<BoundSmallFileProbe>,
    /// `Some` iff the session negotiated resume (otp-7b): the shared
    /// grant map + resumed counter block records are validated and
    /// claimed against. `None` ⇒ any block record is a violation.
    resume: Option<ResumeRecv>,
}

impl NeedListSink {
    pub(super) fn new(
        inner: Arc<dyn TransferSink>,
        outstanding: OutstandingNeeds,
        resume: Option<ResumeRecv>,
        small_file_probe: Option<BoundSmallFileProbe>,
    ) -> Self {
        Self {
            inner,
            outstanding,
            resume,
            small_file_probe,
        }
    }

    /// Remove `path` from the outstanding set, or fault: a path that is
    /// not present is either off the need list or a duplicate delivery.
    fn claim(&self, path: &str) -> Result<()> {
        let removed = if let Some(probe) = &self.small_file_probe {
            let wait_started = probe.start();
            let mut outstanding = self
                .outstanding
                .lock()
                .expect("outstanding-needs lock poisoned");
            let wait = wait_started.elapsed();
            let hold_started = probe.start();
            let removed = outstanding.remove(path);
            drop(outstanding);
            let hold = hold_started.elapsed();
            probe.note_claim(
                SmallFileCarrier::Tcp,
                1,
                1,
                usize::from(removed),
                wait,
                hold,
            );
            removed
        } else {
            self.outstanding
                .lock()
                .expect("outstanding-needs lock poisoned")
                .remove(path)
        };
        if removed {
            Ok(())
        } else {
            Err(eyre::Report::new(
                SessionFault::protocol_violation(format!(
                    "data-plane payload for '{path}' which is not an outstanding need \
                 (off the need list, or a duplicate delivery)"
                ))
                .with_path(path),
            ))
        }
    }

    /// codex otp-7a F3, data-plane parity: a resume-flagged grant may
    /// be satisfied ONLY by its block record — a whole-file or tar-shard
    /// delivery for it bypasses the hash choreography this end committed
    /// to.
    fn reject_resume_flagged(&self, path: &str) -> Result<()> {
        if let Some(resume) = &self.resume {
            if resume
                .headers
                .lock()
                .expect("resume-headers lock poisoned")
                .contains_key(path)
            {
                return Err(eyre::Report::new(
                    SessionFault::protocol_violation(format!(
                        "data-plane file payload for resume-flagged '{path}' — the \
                         contract requires its block record"
                    ))
                    .with_path(path),
                ));
            }
        }
        Ok(())
    }

    /// otp-7b: validate one mid-record `FileBlock` against its grant —
    /// the path must hold a live resume grant, still be an outstanding
    /// need (its completion has not claimed it), and the block must stay
    /// inside the manifested size. The grant is NOT claimed here;
    /// [`Self::claim_block_complete`] does that exactly once.
    fn check_block(&self, path: &str, offset: u64, len: u64) -> Result<()> {
        let Some(resume) = &self.resume else {
            return Err(eyre::Report::new(SessionFault::protocol_violation(
                "resume block record on the data plane of a non-resume session",
            )));
        };
        let size = {
            let held = resume.headers.lock().expect("resume-headers lock poisoned");
            match held.get(path) {
                Some(header) => header.size,
                None => {
                    return Err(eyre::Report::new(
                        SessionFault::protocol_violation(format!(
                            "data-plane block record for '{path}' which was not granted \
                             a resume-flagged need"
                        ))
                        .with_path(path),
                    ))
                }
            }
        };
        if !self
            .outstanding
            .lock()
            .expect("outstanding-needs lock poisoned")
            .contains(path)
        {
            return Err(eyre::Report::new(
                SessionFault::protocol_violation(format!(
                    "data-plane block record for '{path}' which is not an outstanding need"
                ))
                .with_path(path),
            ));
        }
        if offset.saturating_add(len) > size {
            return Err(eyre::Report::new(
                SessionFault::protocol_violation(format!(
                    "block record '{path}' overran its size: offset {offset} + {len} \
                     byte(s) > {size}"
                ))
                .with_path(path),
            ));
        }
        Ok(())
    }

    /// otp-7b: claim one `FileBlockComplete` — remove the grant, verify
    /// the completed size against the manifest promise, and claim the
    /// outstanding need. Mirrors the in-stream `claim_resume_record` +
    /// `finish_block_record` checks. The resumed COUNT happens in
    /// `write_payload` only after the finalization write lands, matching
    /// the in-stream ordering.
    fn claim_block_complete(&self, path: &str, total_size: u64) -> Result<()> {
        let Some(resume) = &self.resume else {
            return Err(eyre::Report::new(SessionFault::protocol_violation(
                "resume block record on the data plane of a non-resume session",
            )));
        };
        let header = resume
            .headers
            .lock()
            .expect("resume-headers lock poisoned")
            .remove(path)
            .ok_or_else(|| {
                eyre::Report::new(
                    SessionFault::protocol_violation(format!(
                        "data-plane block complete for '{path}' which was not granted \
                         a resume-flagged need"
                    ))
                    .with_path(path),
                )
            })?;
        if total_size != header.size {
            return Err(eyre::Report::new(
                SessionFault::protocol_violation(format!(
                    "block record '{path}' completed at {total_size} byte(s), manifest \
                     promised {}",
                    header.size
                ))
                .with_path(path),
            ));
        }
        self.claim(path)
    }
}

#[async_trait]
impl TransferSink for NeedListSink {
    async fn write_payload(&self, payload: PreparedPayload) -> Result<SinkOutcome> {
        match &payload {
            PreparedPayload::File(header) => {
                self.reject_resume_flagged(&header.relative_path)?;
                self.claim(&header.relative_path)?;
            }
            PreparedPayload::TarShard { headers, .. } => {
                for header in headers {
                    self.reject_resume_flagged(&header.relative_path)?;
                }
                for header in headers {
                    self.claim(&header.relative_path)?;
                }
            }
            // otp-7b: resume block records ride the data plane. A
            // mid-record block validates against its live grant (claimed
            // only at completion); the completion claims the grant, the
            // outstanding need, and the resumed count — all against the
            // same shared state the in-stream arms use inline. In a
            // non-resume session both are violations, never a
            // silently-applied patch.
            PreparedPayload::FileBlock {
                relative_path,
                offset,
                bytes,
            } => {
                self.check_block(relative_path, *offset, bytes.len() as u64)?;
            }
            PreparedPayload::FileBlockComplete {
                relative_path,
                total_size,
                ..
            } => {
                self.claim_block_complete(relative_path, *total_size)?;
                let path = relative_path.clone();
                let outcome = self
                    .inner
                    .write_payload(payload)
                    .await
                    .map_err(|e| super::tag_path(e, &path))?;
                // Count only after the finalization write landed —
                // the same ordering the in-stream arms follow.
                self.resume
                    .as_ref()
                    .expect("claim_block_complete verified resume is negotiated")
                    .resumed
                    .fetch_add(1, Ordering::Relaxed);
                return Ok(outcome);
            }
            // Send-side composite (otp-7b) — the wire never carries it,
            // so the receive pipeline can never produce one here.
            PreparedPayload::ResumeFile { .. } => {
                return Err(eyre::Report::new(SessionFault::protocol_violation(
                    "composite ResumeFile payload on the data-plane receive",
                )));
            }
        }
        // Tag the inner write's failure with the file it concerned
        // (otp-7b-2) where the payload names exactly one file.
        let tag: Option<String> = match &payload {
            PreparedPayload::File(h) => Some(h.relative_path.clone()),
            PreparedPayload::FileBlock { relative_path, .. } => Some(relative_path.clone()),
            _ => None,
        };
        match tag {
            Some(path) => self
                .inner
                .write_payload(payload)
                .await
                .map_err(|e| super::tag_path(e, &path)),
            None => self.inner.write_payload(payload).await,
        }
    }

    async fn write_file_stream(
        &self,
        header: &FileHeader,
        reader: &mut (dyn tokio::io::AsyncRead + Unpin + Send),
    ) -> Result<SinkOutcome> {
        self.reject_resume_flagged(&header.relative_path)?;
        self.claim(&header.relative_path)?;
        self.inner
            .write_file_stream(header, reader)
            .await
            .map_err(|e| super::tag_path(e, &header.relative_path))
    }

    async fn finish(&self) -> Result<()> {
        self.inner.finish().await
    }

    fn root(&self) -> &Path {
        self.inner.root()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The otp-4b-1 grant invariant: the responder always grants a
    /// single epoch-0 stream (the zero-knowledge proposal — no manifest
    /// has been seen when SessionAccept goes out) with two independent
    /// 16-byte credentials on a real port. Multi-stream is resize-only
    /// (otp-4b-2).
    #[tokio::test]
    async fn responder_grant_is_single_stream_with_16_byte_tokens() {
        let rdp = prepare_responder_data_plane()
            .await
            .expect("bind loopback data plane");
        let grant = rdp.grant();
        assert_eq!(
            grant.initial_streams, 1,
            "zero-knowledge grant starts single-stream (otp-4b-1)"
        );
        assert_eq!(grant.session_token.len(), SUB_TOKEN_LEN);
        assert_eq!(grant.epoch0_sub_token.len(), SUB_TOKEN_LEN);
        assert_ne!(
            grant.session_token, grant.epoch0_sub_token,
            "session token and epoch-0 sub-token are independent credentials"
        );
        assert_ne!(grant.tcp_port, 0, "a real ephemeral port is granted");
    }

    /// codex otp-4b-1 F1: the data-plane receive must enforce the same
    /// need-list contract the in-stream carrier does inline. A path not
    /// on the outstanding set, a duplicate delivery, and a resume block
    /// record (non-resume session) all fault; a granted path claims once.
    #[tokio::test]
    async fn need_list_sink_enforces_membership_and_rejects_blocks() {
        use crate::remote::transfer::sink::NullSink;

        let outstanding: OutstandingNeeds =
            Arc::new(StdMutex::new(HashSet::from(["a.txt".to_string()])));
        let sink = NeedListSink::new(
            Arc::new(NullSink::new()),
            Arc::clone(&outstanding),
            None,
            None,
        );

        let file = |path: &str| {
            PreparedPayload::File(FileHeader {
                relative_path: path.to_string(),
                ..Default::default()
            })
        };

        // Off-need-list path faults with a SessionFault.
        let err = sink
            .write_payload(file("evil.txt"))
            .await
            .expect_err("off-need-list path must fault");
        assert!(
            err.downcast_ref::<SessionFault>().is_some(),
            "off-list rejection is a SessionFault: {err:#}"
        );

        // Granted need claims exactly once; a duplicate then faults.
        sink.write_payload(file("a.txt"))
            .await
            .expect("granted need writes");
        assert!(
            outstanding.lock().expect("lock").is_empty(),
            "claimed need is removed from the outstanding set"
        );
        let _ = sink
            .write_payload(file("a.txt"))
            .await
            .expect_err("duplicate delivery must fault");

        // Resume block records are rejected in a non-resume session.
        let _ = sink
            .write_payload(PreparedPayload::FileBlockComplete {
                relative_path: "a.txt".to_string(),
                total_size: 0,
                mtime_seconds: 0,
                permissions: 0,
            })
            .await
            .expect_err("resume block on a non-resume session must fault");
    }

    /// otp-7b: the data-plane receive enforces the resume grant
    /// contract with the same strictness the in-stream
    /// `claim_resume_record` applies — blocks validate against a live
    /// grant and the manifested size, completion claims exactly once
    /// and counts, ungranted paths and wrong sizes fault, and a
    /// whole-file delivery for a resume-flagged grant is rejected.
    #[tokio::test]
    async fn need_list_sink_enforces_the_resume_grant_contract() {
        use crate::remote::transfer::sink::NullSink;

        let outstanding: OutstandingNeeds = Arc::new(StdMutex::new(HashSet::from([
            "part.bin".to_string(),
            "plain.txt".to_string(),
        ])));
        let headers: ResumeHeaders = Arc::new(StdMutex::new(HashMap::from([(
            "part.bin".to_string(),
            FileHeader {
                relative_path: "part.bin".to_string(),
                size: 100,
                ..Default::default()
            },
        )])));
        let resumed = Arc::new(AtomicU64::new(0));
        let sink = NeedListSink::new(
            Arc::new(NullSink::new()),
            Arc::clone(&outstanding),
            Some(ResumeRecv {
                headers: Arc::clone(&headers),
                resumed: Arc::clone(&resumed),
            }),
            None,
        );

        // A whole-file record for the resume-flagged grant bypasses the
        // hash choreography — rejected (codex otp-7a F3 parity).
        let _ = sink
            .write_payload(PreparedPayload::File(FileHeader {
                relative_path: "part.bin".to_string(),
                ..Default::default()
            }))
            .await
            .expect_err("file record for a resume-flagged grant must fault");

        // A block for an ungranted path faults.
        let _ = sink
            .write_payload(PreparedPayload::FileBlock {
                relative_path: "plain.txt".to_string(),
                offset: 0,
                bytes: vec![0u8; 4],
            })
            .await
            .expect_err("block for a non-resume-granted path must fault");

        // A block overrunning the manifested size faults.
        let _ = sink
            .write_payload(PreparedPayload::FileBlock {
                relative_path: "part.bin".to_string(),
                offset: 90,
                bytes: vec![0u8; 20],
            })
            .await
            .expect_err("block overrunning the manifest size must fault");

        // In-bounds blocks pass and do NOT claim the need.
        sink.write_payload(PreparedPayload::FileBlock {
            relative_path: "part.bin".to_string(),
            offset: 0,
            bytes: vec![0u8; 50],
        })
        .await
        .expect("in-bounds block writes");
        assert!(
            outstanding.lock().expect("lock").contains("part.bin"),
            "a mid-record block must not claim the outstanding need"
        );
        assert_eq!(resumed.load(Ordering::Relaxed), 0);

        // A completion at the wrong size faults.
        let _ = sink
            .write_payload(PreparedPayload::FileBlockComplete {
                relative_path: "part.bin".to_string(),
                total_size: 99,
                mtime_seconds: 0,
                permissions: 0,
            })
            .await
            .expect_err("completion at the wrong size must fault");

        // The grant was consumed by the failed completion attempt above
        // (the session would abort there); re-arm it to exercise the
        // happy-path completion claim.
        headers.lock().expect("lock").insert(
            "part.bin".to_string(),
            FileHeader {
                relative_path: "part.bin".to_string(),
                size: 100,
                ..Default::default()
            },
        );
        sink.write_payload(PreparedPayload::FileBlockComplete {
            relative_path: "part.bin".to_string(),
            total_size: 100,
            mtime_seconds: 0,
            permissions: 0,
        })
        .await
        .expect("correct completion claims");
        assert!(
            !outstanding.lock().expect("lock").contains("part.bin"),
            "completion claims the outstanding need"
        );
        assert!(
            headers.lock().expect("lock").is_empty(),
            "completion consumes the grant"
        );
        assert_eq!(resumed.load(Ordering::Relaxed), 1, "completion counts");

        // A duplicate completion (no grant left) faults.
        let _ = sink
            .write_payload(PreparedPayload::FileBlockComplete {
                relative_path: "part.bin".to_string(),
                total_size: 100,
                mtime_seconds: 0,
                permissions: 0,
            })
            .await
            .expect_err("duplicate completion must fault");
    }
}
