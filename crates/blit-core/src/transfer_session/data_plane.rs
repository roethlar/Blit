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
//! acquisition differs per byte role.
//!
//! Every TCP SOURCE starts at the same conservative floor clamped by the
//! DESTINATION's advertised receiver ceiling. One [`TransferDial`], one
//! probe registry, and one telemetry tuner then propose one-step ADD or
//! REMOVE epochs. The control records and membership settlement are
//! identical in both layouts; only the transport action flips (the
//! connection initiator dials and the responder accepts). Workload shape
//! selects payload strategy, never a worker target.

use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicU32, AtomicU64, Ordering};
use std::sync::{Arc, Mutex as StdMutex};

use async_trait::async_trait;
use eyre::Result;
use tokio::io::AsyncReadExt;
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::{mpsc, watch};
use tokio::task::JoinSet;

use crate::buffer::BufferPool;
#[cfg(test)]
use crate::dial::{blocked_ratio, DialSampleInput};
use crate::dial::{
    receiver_initial_streams, receiver_stream_ceiling, spawn_dial_tuner_with_resize,
    DialObservationEvent, DialObserver, ResizeProposal, TransferDial,
};
use crate::generated::{
    session_error::Code, CapacityProfile, DataPlaneGrant, DataPlaneResizeOp, FileHeader,
};
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
    ElasticPipelineControl, ElasticPipelineOutcome, FaultedPath, LiveProbe, MembershipOutcome,
    RemoteTransferProgress, SharedStreamProbes, SinkMember, StreamId, StreamProbe,
    StreamProbeRegistry, SUB_TOKEN_LEN,
};

use super::{SessionFault, SourceInstruments};

/// The set of granted-but-not-yet-received needs, shared between the
/// destination's control loop (which inserts each path before sending
/// its `NeedBatch`) and the data-plane receive (which claims each path
/// as its payload lands). Completion is an empty set — the same signal
/// the in-stream carrier uses via its inline `outstanding.remove`.
pub(super) type OutstandingNeeds = Arc<StdMutex<HashSet<String>>>;

/// Manifest headers for ordinary (non-resume) granted needs. Payload metadata
/// is validated against this retained descriptor before the need is claimed.
pub(super) type GrantedHeaders = Arc<StdMutex<HashMap<String, FileHeader>>>;

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
    let fault = SessionFault::refusal(Code::DataPlaneFailed, msg).with_io_kind_from(err);
    let fault = match err.downcast_ref::<FaultedPath>() {
        Some(FaultedPath(path)) => fault.with_path(path.clone()),
        None => fault,
    };
    eyre::Report::new(fault)
}

pub(super) fn validate_epoch0_streams(
    granted: u32,
    receiver_capacity: Option<&CapacityProfile>,
) -> Result<usize> {
    let expected = receiver_initial_streams(receiver_capacity);
    if granted as usize != expected {
        return Err(eyre::Report::new(SessionFault::protocol_violation(
            format!(
                "data-plane grant initial_streams {granted}, expected receiver-bounded floor {expected}"
            ),
        )));
    }
    Ok(expected)
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
    ceiling: usize,
    port: u16,
}

/// Bind a data-plane listener and mint credentials for the grant. Any
/// failure (bind, addr, RNG) logs and returns `None` — the caller then
/// issues a grant-less `SessionAccept` and the session falls back to the
/// in-stream carrier (contract §Transport selection: a responder that
/// cannot bind grants no data plane).
pub(super) async fn prepare_responder_data_plane(
    receiver_capacity: Option<&CapacityProfile>,
) -> Option<ResponderDataPlane> {
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
    // Epoch 0 is the one conservative floor, bounded by the actual byte
    // receiver's advertised limit. It never depends on manifest shape.
    let ceiling = receiver_stream_ceiling(receiver_capacity);
    let initial_streams = receiver_initial_streams(receiver_capacity) as u32;
    Some(ResponderDataPlane {
        listener,
        session_token,
        epoch0_sub_token,
        initial_streams,
        ceiling,
        port,
    })
}

/// Aggregated destination-side receive result. Logical membership belongs to
/// the control-lane resize state; counting sockets opened is wrong after a
/// REMOVE.
pub(super) struct ReceiveTotals {
    pub(super) outcome: SinkOutcome,
}

/// Live handle to a running responder data plane. The control loop arms
/// resize credentials through [`Self::arm`] and joins the accept loop at
/// `SourceDone` via [`Self::finish`].
pub(super) struct ResponderDataPlaneRun {
    arm_tx: Option<mpsc::UnboundedSender<ResizeArm>>,
    shutdown: watch::Sender<bool>,
    task: Option<AbortOnDrop<Result<ReceiveTotals>>>,
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

    /// The receiver-bounded epoch-0 floor this responder granted. The
    /// control loop seeds its logical resize count from this exact value.
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
        let ceiling = self.ceiling;
        let (arm_tx, arm_rx) = mpsc::unbounded_channel::<ResizeArm>();
        let (shutdown, shutdown_rx) = watch::channel(false);
        let task = AbortOnDrop::new(tokio::spawn(self.accept_loop(
            sink,
            progress,
            phase_trace,
            small_file_probe,
            arm_rx,
            shutdown_rx,
        )));
        ResponderDataPlaneRun {
            arm_tx: Some(arm_tx),
            shutdown,
            task: Some(task),
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
        mut shutdown: watch::Receiver<bool>,
    ) -> Result<ReceiveTotals> {
        // Epoch-0 socket credential: session_token ‖ epoch0_sub_token.
        let mut epoch0 = self.session_token.clone();
        epoch0.extend_from_slice(&self.epoch0_sub_token);

        let mut receives: JoinSet<Result<SinkOutcome>> = JoinSet::new();
        let mut total = SinkOutcome::default();
        // Accept the receiver-bounded epoch-0 floor first.
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
            let socket = tokio::select! {
                biased;
                _ = wait_for_shutdown(&mut shutdown) => {
                    abort_receive_workers(&mut receives).await;
                    return Err(dp_fault("data-plane receive aborted"));
                }
                socket = accept_authenticated(&self.listener, &epoch0) => match socket {
                    Ok(socket) => socket,
                    Err(error) => {
                        abort_receive_workers(&mut receives).await;
                        return Err(error);
                    }
                },
            };
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
                _ = wait_for_shutdown(&mut shutdown) => {
                    abort_receive_workers(&mut receives).await;
                    return Err(dp_fault("data-plane receive aborted"));
                }
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
                    let socket = match accepted {
                        Ok(socket) => socket,
                        Err(error) => {
                            abort_receive_workers(&mut receives).await;
                            return Err(error);
                        }
                    };
                    let (socket, epoch) = tokio::select! {
                        biased;
                        _ = wait_for_shutdown(&mut shutdown) => {
                            abort_receive_workers(&mut receives).await;
                            return Err(dp_fault("data-plane receive aborted"));
                        }
                        authenticated = authenticate_resize(
                            socket,
                            &self.session_token,
                            &mut armed,
                        ) => match authenticated {
                            Ok(authenticated) => authenticated,
                            Err(error) => {
                                abort_receive_workers(&mut receives).await;
                                return Err(error);
                            }
                        },
                    };
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
                    let outcome = match joined
                        .expect("join_next is None only when empty, guarded above")
                        .map_err(|err| dp_fault(format!("receive task panicked: {err}")))
                        .and_then(|outcome| outcome)
                    {
                        Ok(outcome) => outcome,
                        Err(error) => {
                            abort_receive_workers(&mut receives).await;
                            return Err(error);
                        }
                    };
                    total.files_written += outcome.files_written;
                    total.bytes_written += outcome.bytes_written;
                }
            }
        }
        Ok(ReceiveTotals { outcome: total })
    }
}

impl ResponderDataPlaneRun {
    /// Arm a resize credential so the next socket presenting
    /// `session_token ‖ sub_token` is accepted. Returns false if the
    /// accept loop is gone (its receiver dropped) — the control loop then
    /// acks the resize as refused.
    pub(super) fn arm(&self, epoch: u32, sub_token: Vec<u8>) -> bool {
        self.arm_tx
            .as_ref()
            .is_some_and(|tx| tx.send(ResizeArm { epoch, sub_token }).is_ok())
    }

    /// Signal `SourceDone` (no more resizes) and join the accept loop for
    /// the aggregated receive totals.
    pub(super) async fn finish(&mut self) -> Result<ReceiveTotals> {
        // Dropping the arm sender is the "no more resizes" signal.
        drop(self.arm_tx.take());
        let task = self
            .task
            .as_mut()
            .expect("ResponderDataPlaneRun::finish called once");
        let joined = task
            .join_mut()
            .await
            .map_err(|err| dp_fault(format!("data-plane receive task panicked: {err}")));
        self.task = None;
        joined?
    }

    pub(super) async fn abort_and_join(&mut self) {
        drop(self.arm_tx.take());
        let _ = self.shutdown.send(true);
        if let Some(task) = self.task.as_mut() {
            let _ = task.join_mut().await;
            self.task = None;
        }
    }
}

async fn wait_for_shutdown(shutdown: &mut watch::Receiver<bool>) {
    loop {
        if *shutdown.borrow() {
            return;
        }
        if shutdown.changed().await.is_err() {
            std::future::pending::<()>().await;
        }
    }
}

async fn abort_receive_workers(receives: &mut JoinSet<Result<SinkOutcome>>) {
    receives.abort_all();
    while receives.join_next().await.is_some() {}
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
    struct ReceiveTaskStopped {
        trace: Option<BoundSessionPhaseTrace>,
        epoch: u32,
        socket_id: u32,
    }

    impl Drop for ReceiveTaskStopped {
        fn drop(&mut self) {
            if let Some(trace) = &self.trace {
                trace.event(
                    "receive_task_stopped",
                    SessionPhaseFields {
                        epoch: Some(self.epoch),
                        socket: Some(self.socket_id),
                        ..Default::default()
                    },
                );
            }
        }
    }

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
    // Construct the sentinel before spawning. A JoinSet task can be aborted
    // before its first poll; keeping the sentinel in the future's captured
    // state makes that path observable too.
    let stopped = ReceiveTaskStopped {
        trace: phase_trace.clone(),
        epoch,
        socket_id,
    };
    receives.spawn(async move {
        let _stopped = stopped;
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
/// outcome.
pub(super) struct InitiatorReceivePlaneRun {
    receives: JoinSet<Result<SinkOutcome>>,
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
    receiver_capacity: Option<&CapacityProfile>,
    sink: Arc<dyn TransferSink>,
    progress: Option<RemoteTransferProgress>,
    trace: bool,
    phase_trace: Option<BoundSessionPhaseTrace>,
    small_file_probe: Option<BoundSmallFileProbe>,
) -> Result<InitiatorReceivePlaneRun> {
    let initial = validate_epoch0_streams(grant.initial_streams, receiver_capacity)?;
    // Epoch-0 handshake: session_token ‖ epoch0_sub_token.
    let mut handshake = grant.session_token.clone();
    handshake.extend_from_slice(&grant.epoch0_sub_token);
    let addr = format!("{host}:{}", grant.tcp_port);

    let mut receives: JoinSet<Result<SinkOutcome>> = JoinSet::new();
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
        let socket = match dial_data_plane(&addr, &handshake, None).await {
            Ok(socket) => socket,
            Err(err) => {
                abort_receive_workers(&mut receives).await;
                return Err(dp_fault_io(
                    &err,
                    format!("dialing session data plane (receive): {err:#}"),
                ));
            }
        };
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
    async fn finish(&mut self) -> Result<ReceiveTotals> {
        let mut total = SinkOutcome::default();
        while let Some(joined) = self.receives.join_next().await {
            let outcome = match joined
                .map_err(|err| dp_fault(format!("receive task panicked: {err}")))
                .and_then(|outcome| outcome)
            {
                Ok(outcome) => outcome,
                Err(error) => {
                    abort_receive_workers(&mut self.receives).await;
                    return Err(error);
                }
            };
            total.files_written += outcome.files_written;
            total.bytes_written += outcome.bytes_written;
        }
        Ok(ReceiveTotals { outcome: total })
    }

    async fn abort_and_join(&mut self) {
        abort_receive_workers(&mut self.receives).await;
    }
}

/// The DESTINATION end's receive data plane, tagged by connection role.
/// Both drain socket bytes into the sink through the same receive
/// pipeline; they differ only in how sockets are obtained (accept vs dial)
/// for epoch 0 and ADD. REMOVE is a shared logical membership transition.
pub(super) enum DestRecvPlane {
    /// DESTINATION responder: accepts epoch-0 sockets; ADD arms a credential
    /// for one more accept, while REMOVE changes only logical membership.
    Responder(ResponderDataPlaneRun),
    /// DESTINATION initiator: dials epoch-0 sockets; ADD dials one more
    /// epoch-N socket, while REMOVE changes only logical membership.
    Initiator(InitiatorReceivePlaneRun),
}

impl DestRecvPlane {
    /// Drain the data plane to completion and report its write outcome. The
    /// shared control-lane resize state owns final logical membership.
    pub(super) async fn finish(&mut self) -> Result<ReceiveTotals> {
        match self {
            DestRecvPlane::Responder(run) => run.finish().await,
            DestRecvPlane::Initiator(run) => run.finish().await,
        }
    }

    pub(super) async fn abort_and_join(&mut self) {
        match self {
            DestRecvPlane::Responder(run) => run.abort_and_join().await,
            DestRecvPlane::Initiator(run) => run.abort_and_join().await,
        }
    }
}

// ---------------------------------------------------------------------------
// Initiator (SOURCE) — dial, authenticate, send, resize
// ---------------------------------------------------------------------------

/// One SOURCE proposal awaiting the peer's ACK and local membership
/// settlement. ADD alone carries a fresh socket credential; REMOVE carries
/// none. At most one epoch is in flight.
pub(super) struct PendingResize {
    pub(super) epoch: u32,
    pub(super) target_streams: u32,
    pub(super) op: DataPlaneResizeOp,
    pub(super) sub_token: Option<Vec<u8>>,
}

/// How the SOURCE acquires epoch-0 and ADD data sockets. Byte direction,
/// tuning policy, pipeline, and settlement are identical; only socket
/// acquisition follows connection topology.
enum SourceSockets {
    /// SOURCE **initiator** (push, otp-4b-2): dials each epoch-N socket to
    /// the granted host:port.
    Dial { host: String, tcp_port: u32 },
    /// SOURCE **responder** (pull, otp-5b-2): accepts each epoch-N socket
    /// off the listener it already bound for epoch-0, credential
    /// `session_token ‖ sub_token`.
    Accept { listener: TcpListener },
}

fn dial_u32(value: usize) -> u32 {
    u32::try_from(value).unwrap_or(u32::MAX)
}

fn dial_action(add: bool) -> &'static str {
    if add {
        "ADD"
    } else {
        "REMOVE"
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct TerminalProbeTotals {
    payload_bytes: u64,
    blocked_ns: u64,
    streams: u32,
}

fn terminal_probe_totals(probes: &[StreamProbe]) -> TerminalProbeTotals {
    let (payload_bytes, blocked_ns) =
        probes
            .iter()
            .fold((0_u64, 0_u64), |(payload_bytes, blocked_ns), probe| {
                let snapshot = probe.snapshot();
                (
                    payload_bytes.saturating_add(snapshot.bytes_sent),
                    blocked_ns.saturating_add(snapshot.write_blocked_nanos),
                )
            });
    TerminalProbeTotals {
        payload_bytes,
        blocked_ns,
        streams: dial_u32(probes.len()),
    }
}

fn retain_terminal_probe(
    terminal_probes: &Option<StdMutex<Vec<StreamProbe>>>,
    probe: &StreamProbe,
) {
    if let Some(probes) = terminal_probes {
        probes
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .push(probe.clone());
    }
}

fn phase_dial_observer(phase: &BoundSessionPhaseTrace) -> DialObserver {
    let phase = phase.clone();
    DialObserver::new(move |event| match event {
        DialObservationEvent::Sample(sample) => {
            let proposal = sample.proposal;
            phase.event(
                "dial_sample",
                SessionPhaseFields {
                    action: proposal.map(|proposal| dial_action(proposal.add)),
                    reason: Some(sample.reason.as_str()),
                    epoch: Some(
                        proposal
                            .map(|proposal| proposal.epoch)
                            .unwrap_or(sample.settled_epoch),
                    ),
                    target_streams: proposal.map(|proposal| dial_u32(proposal.target_streams)),
                    live_streams: Some(dial_u32(sample.live_streams)),
                    sample_bytes: Some(sample.input.delta_bytes),
                    sample_blocked_ns: Some(sample.input.delta_blocked_nanos),
                    sample_elapsed_ns: Some(sample.input.elapsed_nanos),
                    sample_streams: Some(dial_u32(sample.input.sampled_streams)),
                    sample_valid: Some(sample.input.valid),
                    blocked_ratio: Some(sample.input.blocked_ratio),
                    chunk_bytes: Some(sample.chunk_bytes as u64),
                    prefetch_count: Some(dial_u32(sample.prefetch_count)),
                    tcp_buffer_bytes: Some(sample.tcp_buffer_bytes as u64),
                    receiver_ceiling: Some(dial_u32(sample.receiver_ceiling)),
                    peak_streams: Some(dial_u32(sample.peak_streams)),
                    ..Default::default()
                },
            );
        }
        DialObservationEvent::Pending {
            proposal,
            reason,
            live_streams,
            peak_streams,
            receiver_ceiling,
        } => phase.event(
            "dial_pending",
            SessionPhaseFields {
                action: Some(dial_action(proposal.add)),
                reason: Some(reason.as_str()),
                epoch: Some(proposal.epoch),
                target_streams: Some(dial_u32(proposal.target_streams)),
                live_streams: Some(dial_u32(live_streams)),
                receiver_ceiling: Some(dial_u32(receiver_ceiling)),
                peak_streams: Some(dial_u32(peak_streams)),
                ..Default::default()
            },
        ),
        DialObservationEvent::Settlement {
            proposal,
            reason,
            accepted,
            live_streams,
            peak_streams,
            receiver_ceiling,
        } => phase.event(
            "dial_settlement",
            SessionPhaseFields {
                action: Some(dial_action(proposal.add)),
                reason: Some(reason.as_str()),
                epoch: Some(proposal.epoch),
                target_streams: Some(dial_u32(proposal.target_streams)),
                live_streams: Some(dial_u32(live_streams)),
                accepted: Some(accepted),
                receiver_ceiling: Some(dial_u32(receiver_ceiling)),
                peak_streams: Some(dial_u32(peak_streams)),
                ..Default::default()
            },
        ),
    })
}

/// A running source-side data plane: dialed or accepted sockets wrapped in
/// one elastic sink pipeline whose acknowledged membership grows or shrinks
/// from SOURCE telemetry. Planned payloads are fed via [`Self::queue`];
/// closing via [`Self::finish`] drains the pipeline, emits each socket's END,
/// and returns the bytes this end sent.
pub(super) struct SourceDataPlane {
    payload_tx: Option<mpsc::Sender<TransferPayload>>,
    control: Option<ElasticPipelineControl>,
    // The queue path must be able to take and drain a failed pipeline through
    // `&self`, because its send future is selected alongside resize/control
    // work. The mutex is ownership coordination, not hot-path data: it is
    // touched only after the payload receiver closes or during teardown.
    pipeline: tokio::sync::Mutex<Option<AbortOnDrop<Result<ElasticPipelineOutcome>>>>,
    tuner: Option<AbortOnDrop<()>>,
    resize_rx: Option<mpsc::UnboundedReceiver<ResizeProposal>>,
    // The byte SENDER owns the live dial, bounded by the byte RECEIVER's
    // advertised capacity (contract §Invariants 5). Cheap values are read
    // when queues/sockets are built; live stream membership is settled here.
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
    /// Diagnostic-only clones of every initial and accepted ADD probe.
    /// Unlike the live registry, retirement never removes these handles.
    /// The collection exists only when session-phase tracing is active.
    terminal_probes: Option<StdMutex<Vec<StreamProbe>>>,
    small_file_probe: Option<BoundSmallFileProbe>,
    queue_trace_armed: AtomicBool,
}

/// Preserve the first worker failure instead of replacing it with the
/// producer-side symptom that the payload receiver closed. This restores the
/// pipeline error contract the pre-session sender already enforced.
async fn drain_send_pipeline_outcome(
    pipeline: AbortOnDrop<Result<ElasticPipelineOutcome>>,
) -> Result<ElasticPipelineOutcome> {
    match pipeline.join().await {
        Ok(Ok(outcome)) => Ok(outcome),
        Ok(Err(error)) => Err(dp_fault_io(
            &error,
            format!("data-plane send pipeline failed: {error:#}"),
        )),
        Err(join) => Err(dp_fault(format!(
            "data-plane send pipeline panicked: {join}"
        ))),
    }
}

async fn drain_send_pipeline_error(
    pipeline: AbortOnDrop<Result<ElasticPipelineOutcome>>,
) -> eyre::Report {
    match drain_send_pipeline_outcome(pipeline).await {
        Ok(_) => {
            dp_fault("data-plane send pipeline closed cleanly before all payloads were admitted")
        }
        Err(error) => error,
    }
}

fn start_source_tuner(
    _instruments: &SourceInstruments,
    dial: &Arc<TransferDial>,
    probes: SharedStreamProbes,
    resize_tx: mpsc::UnboundedSender<ResizeProposal>,
) -> Result<AbortOnDrop<()>> {
    #[cfg(test)]
    if let Some(samples) = &_instruments.dial_test_samples {
        let mut samples = samples
            .lock()
            .expect("dial test sample source lock poisoned")
            .take()
            .ok_or_else(|| dp_fault("dial test sample source already consumed"))?;
        let dial = Arc::clone(dial);
        let proposal_gate = _instruments.dial_proposal_test_gate.clone();
        let handle = tokio::spawn(async move {
            while let Some(sample) = samples.recv().await {
                // The injected values replace only the clock/kernel read. The
                // real registry still has to match settled membership, so a
                // constructor or ADD accidentally changed back to NoProbe
                // makes the role guard stop producing decisions.
                let observed_streams = probes.lock().expect("probe registry poisoned").len();
                let membership_aligned = observed_streams == dial.live_streams();
                let decision = if membership_aligned {
                    let elapsed_nanos = 1_000_000_000_u64;
                    let blocked_capacity =
                        (elapsed_nanos as u128).saturating_mul(observed_streams as u128);
                    let delta_blocked_nanos = (sample.blocked_ratio.clamp(0.0, 1.0)
                        * blocked_capacity as f64)
                        .round()
                        .min(u64::MAX as f64) as u64;
                    dial.apply_sample_input(DialSampleInput {
                        delta_bytes: sample.delta_bytes,
                        delta_blocked_nanos,
                        elapsed_nanos,
                        sampled_streams: observed_streams,
                        blocked_ratio: blocked_ratio(
                            delta_blocked_nanos,
                            std::time::Duration::from_nanos(elapsed_nanos),
                            observed_streams,
                        ),
                        valid: true,
                    })
                } else {
                    dial.apply_sample_input(DialSampleInput {
                        delta_bytes: 0,
                        delta_blocked_nanos: 0,
                        elapsed_nanos: 0,
                        sampled_streams: observed_streams,
                        blocked_ratio: 0.0,
                        valid: false,
                    })
                };
                let proposal = decision.and_then(|decision| decision.proposal);
                if let Some(proposal) = proposal {
                    if let Some(gate) = &proposal_gate {
                        gate.hold().await;
                    }
                    if resize_tx.send(proposal).is_err() {
                        dial.resize_settled(proposal.epoch, dial.live_streams(), false);
                    } else {
                        dial.wait_for_resize_settlement(proposal.epoch).await;
                    }
                }
                let refused = dial.resize_refused();
                let _ = sample.reply.send(super::DialTestObservation {
                    proposal,
                    live_streams: dial.live_streams(),
                    settled_epoch: dial.resize_epoch(),
                });
                if refused {
                    return;
                }
            }
        });
        return Ok(AbortOnDrop::new(handle));
    }

    let handle = spawn_dial_tuner_with_resize(dial, probes, Some(resize_tx));
    Ok(AbortOnDrop::new(handle))
}

#[allow(clippy::too_many_arguments)]
async fn start_source_data_plane(
    sockets: SourceSockets,
    session_token: Vec<u8>,
    epoch0_sub_token: Vec<u8>,
    granted_initial: u32,
    receiver_capacity: Option<&CapacityProfile>,
    source: Arc<dyn TransferSource>,
    instruments: &SourceInstruments,
    phase_trace: Option<BoundSessionPhaseTrace>,
    small_file_probe: Option<BoundSmallFileProbe>,
) -> Result<SourceDataPlane> {
    let initial = validate_epoch0_streams(granted_initial, receiver_capacity)?;
    let observer = phase_trace.as_ref().map(phase_dial_observer);
    let dial = TransferDial::conservative_within(receiver_capacity)
        .with_observer(observer)
        .shared();
    dial.set_negotiated_streams(initial);

    let mut epoch0_handshake = session_token.clone();
    epoch0_handshake.extend_from_slice(&epoch0_sub_token);
    let pool = Arc::new(BufferPool::for_data_plane(
        dial.chunk_bytes(),
        dial.ceiling_max_streams().max(1),
    ));
    let trace = instruments.trace_data_plane;
    let probes: SharedStreamProbes = Arc::new(StdMutex::new(StreamProbeRegistry::default()));
    let terminal_probes = phase_trace
        .as_ref()
        .map(|_| StdMutex::new(Vec::with_capacity(initial)));
    let mut sinks = Vec::with_capacity(initial);

    for socket_id in 0..initial {
        let member_id = StreamId(socket_id as u32);
        let probe = StreamProbe::new(member_id);
        retain_terminal_probe(&terminal_probes, &probe);
        let (begin_event, end_event) = match &sockets {
            SourceSockets::Dial { .. } => ("socket_dial_begin", "socket_dial_end"),
            SourceSockets::Accept { .. } => ("socket_accept_begin", "socket_accept_end"),
        };
        if let Some(phase) = &phase_trace {
            phase.event(
                begin_event,
                SessionPhaseFields {
                    epoch: Some(0),
                    socket: Some(socket_id as u32),
                    ..Default::default()
                },
            );
        }

        // The explicit type is a structural guard: every TransferSession
        // SOURCE socket must carry real telemetry. Replacing any dial or
        // accept branch with the probe-free constructor must not compile.
        let session: DataPlaneSession<LiveProbe> = match &sockets {
            SourceSockets::Dial { host, tcp_port } => DataPlaneSession::connect_with_probe(
                host,
                *tcp_port,
                &epoch0_handshake,
                dial.chunk_bytes(),
                dial.prefetch_count(),
                trace,
                dial.tcp_buffer_bytes(),
                Arc::clone(&pool),
                LiveProbe(probe.clone()),
            )
            .await
            .map_err(|err| dp_fault_io(&err, format!("dialing session data plane: {err:#}")))?,
            SourceSockets::Accept { listener } => {
                let socket = accept_authenticated(listener, &epoch0_handshake).await?;
                configure_data_socket(&socket, dial.tcp_buffer_bytes()).map_err(|err| {
                    dp_fault(format!("configuring accepted source socket: {err}"))
                })?;
                DataPlaneSession::from_stream_with_probe(
                    socket,
                    trace,
                    dial.chunk_bytes(),
                    dial.prefetch_count(),
                    Arc::clone(&pool),
                    LiveProbe(probe.clone()),
                )
                .await
            }
        };
        debug_assert!(
            session.uses_probe(&probe),
            "epoch-0 session and member registry must share one probe"
        );

        if let Some(phase) = &phase_trace {
            phase.event(
                end_event,
                SessionPhaseFields {
                    epoch: Some(0),
                    socket: Some(socket_id as u32),
                    ..Default::default()
                },
            );
        }
        let session = session.with_session_phase_trace(phase_trace.clone(), 0, socket_id as u32);
        let sink: Arc<dyn TransferSink> = Arc::new(DataPlaneSink::new(
            session,
            Arc::clone(&source),
            PathBuf::new(),
        ));
        sinks.push(SinkMember::with_probe(member_id, sink, probe));
    }

    let prefetch = dial.prefetch_count().max(1);
    let (payload_tx, payload_rx) = mpsc::channel::<TransferPayload>(prefetch);
    let (control, commands) = ElasticPipelineControl::channel();
    let pipe_source = Arc::clone(&source);
    let pipe_progress = instruments.progress.clone();
    let pipeline_probes = Arc::clone(&probes);
    let pipeline = AbortOnDrop::new(tokio::spawn(async move {
        execute_sink_pipeline_elastic(
            pipe_source,
            sinks,
            payload_rx,
            prefetch,
            pipe_progress.as_ref(),
            Some(commands),
            pipeline_probes,
        )
        .await
    }));
    let admitted = control
        .logical_count()
        .await
        .map_err(|err| dp_fault(format!("awaiting epoch-0 member admission: {err:#}")))?;
    if admitted != initial {
        return Err(dp_fault(format!(
            "epoch-0 pipeline admitted {admitted} members, expected {initial}"
        )));
    }

    let (resize_tx, resize_rx) = mpsc::unbounded_channel();
    let tuner = start_source_tuner(instruments, &dial, Arc::clone(&probes), resize_tx)?;
    let queue_trace_armed = phase_trace.is_some();
    Ok(SourceDataPlane {
        payload_tx: Some(payload_tx),
        control: Some(control),
        pipeline: tokio::sync::Mutex::new(Some(pipeline)),
        tuner: Some(tuner),
        resize_rx: Some(resize_rx),
        dial,
        next_member_id: AtomicU32::new(initial as u32),
        source,
        session_token,
        pool,
        trace,
        sockets,
        phase_trace,
        terminal_probes,
        small_file_probe,
        queue_trace_armed: AtomicBool::new(queue_trace_armed),
    })
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
    start_source_data_plane(
        SourceSockets::Dial {
            host: host.to_string(),
            tcp_port: grant.tcp_port,
        },
        grant.session_token.clone(),
        grant.epoch0_sub_token.clone(),
        grant.initial_streams,
        receiver_capacity,
        source,
        instruments,
        phase_trace,
        small_file_probe,
    )
    .await
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
    start_source_data_plane(
        SourceSockets::Accept {
            listener: bound.listener,
        },
        bound.session_token,
        bound.epoch0_sub_token,
        bound.initial_streams,
        receiver_capacity,
        source,
        instruments,
        phase_trace,
        small_file_probe,
    )
    .await
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

    pub(super) fn take_resize_proposals(
        &mut self,
    ) -> Result<mpsc::UnboundedReceiver<ResizeProposal>> {
        self.resize_rx
            .take()
            .ok_or_else(|| dp_fault("data-plane resize proposal receiver already consumed"))
    }

    pub(super) async fn stop_tuner(&mut self) -> Result<()> {
        let Some(tuner) = self.tuner.as_mut() else {
            return Ok(());
        };
        let joined = tuner.abort_and_join_mut().await;
        self.tuner = None;
        match joined {
            Ok(()) => Ok(()),
            Err(err) if err.is_cancelled() => Ok(()),
            Err(err) => Err(dp_fault(format!("live dial tuner panicked: {err}"))),
        }
    }

    /// Turn one claimed dial proposal into its wire record. ADD alone mints
    /// a socket credential. If credential generation fails before anything
    /// reaches the peer, settle unchanged and stop resizing while the
    /// already-live workers continue the transfer.
    pub(super) fn prepare_resize(&self, proposal: ResizeProposal) -> Result<Option<PendingResize>> {
        let live = self.dial.live_streams();
        let expected_target = if proposal.add {
            live.checked_add(1)
        } else {
            live.checked_sub(1)
        };
        if expected_target != Some(proposal.target_streams) || proposal.epoch == 0 {
            self.dial
                .resize_settled(proposal.epoch, self.dial.live_streams(), false);
            return Err(dp_fault(format!(
                "dial proposed inconsistent resize epoch={} live={live} target={} add={}",
                proposal.epoch, proposal.target_streams, proposal.add
            )));
        }
        let target_streams = u32::try_from(proposal.target_streams)
            .map_err(|_| dp_fault("data-plane resize target exceeds the wire range"))?;
        let op = if proposal.add {
            DataPlaneResizeOp::Add
        } else {
            DataPlaneResizeOp::Remove
        };
        let sub_token = if proposal.add {
            match generate_sub_token() {
                Ok(token) => Some(token),
                Err(err) => {
                    log::warn!(
                        "data-plane resize disabled after sub-token generation failed: {err:#}"
                    );
                    self.dial.resize_settled(proposal.epoch, live, false);
                    return Ok(None);
                }
            }
        } else {
            None
        };
        Ok(Some(PendingResize {
            epoch: proposal.epoch,
            target_streams,
            op,
            sub_token,
        }))
    }

    pub(super) fn refuse_unsent_resize(&self, proposal: ResizeProposal) {
        self.dial
            .resize_settled(proposal.epoch, self.dial.live_streams(), false);
    }

    pub(super) async fn retire_stream(&self) -> Result<MembershipOutcome> {
        self.control
            .as_ref()
            .ok_or_else(|| dp_fault("data-plane membership control already closed"))?
            .retire_one()
            .await
            .map_err(|err| dp_fault(format!("retiring data-plane member: {err:#}")))
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
        let probe = StreamProbe::new(member_id);
        let session: DataPlaneSession<LiveProbe> = match &self.sockets {
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
                let session = DataPlaneSession::connect_with_probe(
                    host,
                    *tcp_port,
                    &handshake,
                    self.dial.chunk_bytes(),
                    self.dial.prefetch_count(),
                    self.trace,
                    self.dial.tcp_buffer_bytes(),
                    Arc::clone(&self.pool),
                    LiveProbe(probe.clone()),
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
                configure_data_socket(&socket, self.dial.tcp_buffer_bytes()).map_err(|err| {
                    dp_fault(format!("configuring accepted resize source socket: {err}"))
                })?;
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
                DataPlaneSession::from_stream_with_probe(
                    socket,
                    self.trace,
                    self.dial.chunk_bytes(),
                    self.dial.prefetch_count(),
                    Arc::clone(&self.pool),
                    LiveProbe(probe.clone()),
                )
                .await
                .with_session_phase_trace(self.phase_trace.clone(), epoch, 0)
            }
        };
        debug_assert!(
            session.uses_probe(&probe),
            "ADD session and member registry must share one probe"
        );
        retain_terminal_probe(&self.terminal_probes, &probe);
        let sink: Arc<dyn TransferSink> = Arc::new(DataPlaneSink::new(
            session,
            Arc::clone(&self.source),
            PathBuf::new(),
        ));
        self.control
            .as_ref()
            .ok_or_else(|| dp_fault("data-plane membership control already closed"))?
            .add(SinkMember::with_probe(member_id, sink, probe))
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
                tx.send(payload).await.map_err(|_| ())
            } else {
                match tx.reserve().await {
                    Ok(permit) => {
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
                    }
                    Err(_) => Err(()),
                }
            };
        if result.is_err() {
            let pipeline = self.pipeline.lock().await.take().ok_or_else(|| {
                dp_fault("data-plane send pipeline handle missing after payload queue closed")
            })?;
            return Err(drain_send_pipeline_error(pipeline).await);
        }
        if let Some((probe, members, started)) = tar_probe {
            probe.note_tar_queue(started.elapsed(), members);
        }
        Ok(())
    }

    /// Declare that every payload has been queued without waiting for the
    /// elastic workers to finish. Closing the input lets idle workers emit
    /// END promptly while the control lane settles any remaining resize
    /// epochs; a late ADD likewise sees a closed queue and emits END rather
    /// than waiting under the receiver's StallGuard.
    pub(super) fn close_payloads(&mut self) -> Result<()> {
        let first_close = self.payload_tx.is_some();
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
        sealed.map_err(|err| dp_fault(format!("sealing data-plane membership: {err:#}")))?;
        if first_close {
            if let Some(trace) = &self.phase_trace {
                trace.event(
                    "membership_sealed",
                    SessionPhaseFields {
                        live_streams: Some(dial_u32(self.dial.live_streams())),
                        receiver_ceiling: Some(dial_u32(self.dial.ceiling_max_streams())),
                        peak_streams: Some(dial_u32(self.dial.peak_streams())),
                        ..Default::default()
                    },
                );
            }
        }
        Ok(())
    }

    /// Seal payload admission without replacing a worker failure that already
    /// closed the membership command lane. The supervisor owns the structured
    /// file/IO cause; a failed `Seal` send is only the producer-side symptom.
    pub(super) async fn close_payloads_preserving_pipeline_error(&mut self) -> Result<()> {
        let close_error = match self.close_payloads() {
            Ok(()) => return Ok(()),
            Err(error) => error,
        };
        let pipeline = self.pipeline.lock().await.take();
        match pipeline {
            Some(pipeline) => match drain_send_pipeline_outcome(pipeline).await {
                Err(worker_error) => Err(worker_error),
                Ok(_) => Err(close_error),
            },
            None => Err(close_error),
        }
    }

    /// Signal end-of-stream, drain the pipeline (each worker emits its
    /// socket's END record on drain), and return the bytes sent. Must be
    /// awaited before `SourceDone` goes out so the destination's receive
    /// pipeline sees END and completes.
    pub(super) async fn finish(&mut self) -> Result<SinkOutcome> {
        self.stop_tuner().await?;
        if let Some(mut proposals) = self.resize_rx.take() {
            proposals.close();
            while let Ok(proposal) = proposals.try_recv() {
                self.refuse_unsent_resize(proposal);
            }
        }
        // Seal before dropping the payload sender, then explicitly mark the
        // command stream finished. The endpoint stays owned until the join
        // completes so a peer fault that cancels this future can still send a
        // cooperative Abort and wait for every nested worker to be reaped.
        let close_result = self.close_payloads();
        let resize_pending = self.dial.resize_pending();
        let finish_result = self
            .control
            .as_ref()
            .ok_or_else(|| dp_fault("data-plane membership control already closed"))?
            .finish()
            .map_err(|err| dp_fault(format!("finishing data-plane membership: {err:#}")));
        let pipeline = self
            .pipeline
            .lock()
            .await
            .take()
            .expect("SourceDataPlane::finish called once");
        let joined = drain_send_pipeline_outcome(pipeline).await;
        self.control = None;
        let elastic = joined?;
        close_result?;
        finish_result?;
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
            let totals = self
                .terminal_probes
                .as_ref()
                .map(|probes| {
                    let probes = probes
                        .lock()
                        .unwrap_or_else(std::sync::PoisonError::into_inner);
                    terminal_probe_totals(&probes)
                })
                .unwrap_or(TerminalProbeTotals {
                    payload_bytes: 0,
                    blocked_ns: 0,
                    streams: 0,
                });
            trace.event(
                "dial_terminal_sample",
                SessionPhaseFields {
                    live_streams: Some(dial_u32(self.dial.live_streams())),
                    terminal_payload_bytes: Some(totals.payload_bytes),
                    terminal_blocked_ns: Some(totals.blocked_ns),
                    terminal_streams: Some(totals.streams),
                    receiver_ceiling: Some(dial_u32(self.dial.ceiling_max_streams())),
                    peak_streams: Some(dial_u32(self.dial.peak_streams())),
                    ..Default::default()
                },
            );
            trace.event(
                "data_plane_complete",
                SessionPhaseFields {
                    live_streams: Some(dial_u32(self.dial.live_streams())),
                    receiver_ceiling: Some(dial_u32(self.dial.ceiling_max_streams())),
                    peak_streams: Some(dial_u32(self.dial.peak_streams())),
                    ..Default::default()
                },
            );
        }
        Ok(elastic.outcome)
    }

    /// Error/cancellation teardown: stop the sampler, ask the elastic
    /// supervisor to abort and reap every nested worker, then join the outer
    /// task. A pending accepted epoch is intentionally not rewritten as a
    /// refusal; the session fault owns that outcome.
    pub(super) async fn abort_and_join(&mut self) {
        let _ = self.stop_tuner().await;
        self.resize_rx = None;
        self.payload_tx = None;
        let supervisor_drained = match self.control.as_ref() {
            Some(control) => control.abort_and_drain().await.is_ok(),
            None => false,
        };
        self.control = None;
        let pipeline = self.pipeline.lock().await.take();
        if let Some(pipeline) = pipeline {
            let joined = if supervisor_drained {
                pipeline.join().await
            } else {
                pipeline.abort_and_join().await
            };
            if let Err(err) = joined {
                if !err.is_cancelled() {
                    log::debug!("data-plane pipeline teardown join failed: {err}");
                }
            }
        }
        if let Some(trace) = &self.phase_trace {
            trace.event(
                "data_plane_aborted",
                SessionPhaseFields {
                    live_streams: Some(dial_u32(self.dial.live_streams())),
                    receiver_ceiling: Some(dial_u32(self.dial.ceiling_max_streams())),
                    peak_streams: Some(dial_u32(self.dial.peak_streams())),
                    ..Default::default()
                },
            );
        }
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
    granted_headers: GrantedHeaders,
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
        granted_headers: GrantedHeaders,
        resume: Option<ResumeRecv>,
        small_file_probe: Option<BoundSmallFileProbe>,
    ) -> Self {
        Self {
            inner,
            outstanding,
            granted_headers,
            resume,
            small_file_probe,
        }
    }

    fn validate_and_claim_header(&self, payload: &FileHeader) -> Result<()> {
        let manifest = self
            .granted_headers
            .lock()
            .expect("granted-headers lock poisoned")
            .get(&payload.relative_path)
            .cloned()
            .ok_or_else(|| {
                eyre::Report::new(
                    SessionFault::protocol_violation(format!(
                        "data-plane payload for '{}' has no retained manifest grant",
                        payload.relative_path
                    ))
                    .with_path(payload.relative_path.as_str()),
                )
            })?;
        crate::windows_metadata::validate_payload_against_manifest(
            payload.windows_metadata.as_ref(),
            manifest.windows_metadata.as_ref(),
        )
        .map_err(|error| {
            eyre::Report::new(
                SessionFault::protocol_violation(format!(
                    "invalid Windows metadata for '{}': {error:#}",
                    payload.relative_path
                ))
                .with_path(payload.relative_path.as_str()),
            )
        })?;
        if payload.size != manifest.size
            || payload.mtime_seconds != manifest.mtime_seconds
            || payload.permissions != manifest.permissions
        {
            return Err(eyre::Report::new(
                SessionFault::protocol_violation(format!(
                    "data-plane payload header for '{}' changed after the manifest",
                    payload.relative_path
                ))
                .with_path(payload.relative_path.as_str()),
            ));
        }
        self.granted_headers
            .lock()
            .expect("granted-headers lock poisoned")
            .remove(&payload.relative_path);
        self.claim(&payload.relative_path)
    }

    fn validate_and_claim_shard_headers(&self, payloads: &[FileHeader]) -> Result<()> {
        {
            let manifests = self
                .granted_headers
                .lock()
                .expect("granted-headers lock poisoned");
            for payload in payloads {
                let manifest = manifests.get(&payload.relative_path).ok_or_else(|| {
                    eyre::Report::new(
                        SessionFault::protocol_violation(format!(
                            "data-plane tar member '{}' has no retained manifest grant",
                            payload.relative_path
                        ))
                        .with_path(payload.relative_path.as_str()),
                    )
                })?;
                crate::windows_metadata::validate_payload_against_manifest(
                    payload.windows_metadata.as_ref(),
                    manifest.windows_metadata.as_ref(),
                )
                .map_err(|error| {
                    eyre::Report::new(
                        SessionFault::protocol_violation(format!(
                            "invalid Windows metadata for '{}': {error:#}",
                            payload.relative_path
                        ))
                        .with_path(payload.relative_path.as_str()),
                    )
                })?;
                if payload.size != manifest.size
                    || payload.mtime_seconds != manifest.mtime_seconds
                    || payload.permissions != manifest.permissions
                {
                    return Err(eyre::Report::new(
                        SessionFault::protocol_violation(format!(
                            "data-plane tar header for '{}' changed after the manifest",
                            payload.relative_path
                        ))
                        .with_path(payload.relative_path.as_str()),
                    ));
                }
            }
        }
        {
            let mut manifests = self
                .granted_headers
                .lock()
                .expect("granted-headers lock poisoned");
            for payload in payloads {
                manifests.remove(&payload.relative_path);
            }
        }
        self.claim_shard(payloads)
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

    /// Claim one tar shard while holding the outstanding-set mutex once.
    /// This preserves the single-file claim's ordered failure behavior (an
    /// invalid later member faults after earlier members were claimed) without
    /// paying one lock/unlock pair per small file.
    fn claim_shard(&self, headers: &[FileHeader]) -> Result<()> {
        let rejected = if let Some(probe) = &self.small_file_probe {
            let wait_started = probe.start();
            let mut outstanding = self
                .outstanding
                .lock()
                .expect("outstanding-needs lock poisoned");
            let wait = wait_started.elapsed();
            let hold_started = probe.start();
            let mut rejected = None;
            let mut removed = 0usize;
            for header in headers {
                if outstanding.remove(&header.relative_path) {
                    removed += 1;
                } else {
                    rejected = Some(header.relative_path.clone());
                    break;
                }
            }
            drop(outstanding);
            let hold = hold_started.elapsed();
            probe.note_claim(SmallFileCarrier::Tcp, headers.len(), 1, removed, wait, hold);
            rejected
        } else {
            let mut outstanding = self
                .outstanding
                .lock()
                .expect("outstanding-needs lock poisoned");
            let mut rejected = None;
            for header in headers {
                if !outstanding.remove(&header.relative_path) {
                    rejected = Some(header.relative_path.clone());
                    break;
                }
            }
            rejected
        };
        match rejected {
            None => Ok(()),
            Some(path) => Err(eyre::Report::new(
                SessionFault::protocol_violation(format!(
                    "data-plane payload for '{path}' which is not an outstanding need \
                     (off the need list, or a duplicate delivery)"
                ))
                .with_path(path),
            )),
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
    fn claim_block_complete(
        &self,
        path: &str,
        total_size: u64,
        mtime_seconds: i64,
        permissions: u32,
        windows_metadata: Option<&crate::generated::WindowsFileMetadata>,
    ) -> Result<()> {
        let Some(resume) = &self.resume else {
            return Err(eyre::Report::new(SessionFault::protocol_violation(
                "resume block record on the data plane of a non-resume session",
            )));
        };
        let header = resume
            .headers
            .lock()
            .expect("resume-headers lock poisoned")
            .get(path)
            .cloned()
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
        if mtime_seconds != header.mtime_seconds || permissions != header.permissions {
            return Err(eyre::Report::new(
                SessionFault::protocol_violation(format!(
                    "block-complete metadata for '{path}' changed after the manifest"
                ))
                .with_path(path),
            ));
        }
        crate::windows_metadata::validate_payload_against_manifest(
            windows_metadata,
            header.windows_metadata.as_ref(),
        )
        .map_err(|error| {
            eyre::Report::new(
                SessionFault::protocol_violation(format!(
                    "invalid Windows metadata for '{path}': {error:#}"
                ))
                .with_path(path),
            )
        })?;
        resume
            .headers
            .lock()
            .expect("resume-headers lock poisoned")
            .remove(path);
        self.claim(path)
    }
}

#[async_trait]
impl TransferSink for NeedListSink {
    async fn write_payload(&self, payload: PreparedPayload) -> Result<SinkOutcome> {
        match &payload {
            PreparedPayload::File(header) => {
                self.reject_resume_flagged(&header.relative_path)?;
                self.validate_and_claim_header(header)?;
            }
            PreparedPayload::TarShard { headers, .. } => {
                for header in headers {
                    self.reject_resume_flagged(&header.relative_path)?;
                }
                self.validate_and_claim_shard_headers(headers)?;
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
                mtime_seconds,
                permissions,
                windows_metadata,
            } => {
                self.claim_block_complete(
                    relative_path,
                    *total_size,
                    *mtime_seconds,
                    *permissions,
                    windows_metadata.as_ref(),
                )?;
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
        self.validate_and_claim_header(header)?;
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
    use crate::remote::transfer::source::FsTransferSource;
    use tempfile::tempdir;

    struct DropFlag(Arc<AtomicBool>);

    impl Drop for DropFlag {
        fn drop(&mut self) {
            self.0.store(true, Ordering::SeqCst);
        }
    }

    #[test]
    fn terminal_probe_totals_saturate_and_include_every_retained_probe() {
        let first = StreamProbe::new(StreamId(1));
        first.record_bytes(u64::MAX - 5);
        first.add_write_blocked(u64::MAX - 7);
        let removed = StreamProbe::new(StreamId(2));
        removed.record_bytes(10);
        removed.add_write_blocked(20);
        let terminal_add = StreamProbe::new(StreamId(3));

        assert_eq!(
            terminal_probe_totals(&[first, removed, terminal_add]),
            TerminalProbeTotals {
                payload_bytes: u64::MAX,
                blocked_ns: u64::MAX,
                streams: 3,
            }
        );
    }

    /// The grant starts at the canonical receiver-bounded floor without
    /// consulting manifest shape and carries two independent credentials.
    #[tokio::test]
    async fn responder_grant_uses_receiver_bounded_floor_with_16_byte_tokens() {
        let rdp = prepare_responder_data_plane(None)
            .await
            .expect("bind loopback data plane");
        let grant = rdp.grant();
        assert_eq!(
            grant.initial_streams as usize,
            receiver_initial_streams(None),
            "epoch 0 is the conservative default floor"
        );
        assert_eq!(grant.session_token.len(), SUB_TOKEN_LEN);
        assert_eq!(grant.epoch0_sub_token.len(), SUB_TOKEN_LEN);
        assert_ne!(
            grant.session_token, grant.epoch0_sub_token,
            "session token and epoch-0 sub-token are independent credentials"
        );
        assert_ne!(grant.tcp_port, 0, "a real ephemeral port is granted");
    }

    #[tokio::test(flavor = "current_thread")]
    async fn receive_finish_reaps_siblings_before_returning_first_error() {
        use crate::remote::transfer::sink::NullSink;

        let mut receives = JoinSet::new();
        let entered = Arc::new(tokio::sync::Notify::new());
        let entered_task = Arc::clone(&entered);
        let dropped = Arc::new(AtomicBool::new(false));
        let dropped_task = Arc::clone(&dropped);
        receives.spawn(async move {
            let _drop_flag = DropFlag(dropped_task);
            entered_task.notify_one();
            std::future::pending::<()>().await;
            Ok(SinkOutcome::default())
        });
        tokio::time::timeout(std::time::Duration::from_secs(5), entered.notified())
            .await
            .expect("pending receive task entered");
        receives.spawn(async { Err(eyre::eyre!("injected receive failure")) });

        let mut run = InitiatorReceivePlaneRun {
            receives,
            host: String::new(),
            tcp_port: 0,
            session_token: Vec::new(),
            sink: Arc::new(NullSink::new()),
            progress: None,
            trace: false,
            phase_trace: None,
            small_file_probe: None,
        };
        let error = match run.finish().await {
            Err(error) => error,
            Ok(_) => panic!("first receive failure must surface"),
        };
        assert!(format!("{error:#}").contains("injected receive failure"));
        assert!(
            dropped.load(Ordering::SeqCst),
            "sibling receive task must be destroyed before finish returns"
        );
    }

    #[tokio::test]
    async fn source_queue_surfaces_the_pipeline_worker_error() {
        let tmp = tempdir().unwrap();
        let source: Arc<dyn TransferSource> =
            Arc::new(FsTransferSource::new(tmp.path().to_path_buf()));
        let (payload_tx, payload_rx) = mpsc::channel(1);
        drop(payload_rx);
        let pipeline = AbortOnDrop::new(tokio::spawn(async {
            Err::<ElasticPipelineOutcome, _>(eyre::eyre!("injected source worker failure"))
        }));
        let (control, _commands) = ElasticPipelineControl::channel();
        let plane = SourceDataPlane {
            payload_tx: Some(payload_tx),
            control: Some(control),
            pipeline: tokio::sync::Mutex::new(Some(pipeline)),
            tuner: None,
            resize_rx: None,
            dial: TransferDial::conservative().shared(),
            next_member_id: AtomicU32::new(1),
            source,
            session_token: Vec::new(),
            pool: Arc::new(BufferPool::for_data_plane(
                crate::buffer::DATA_PLANE_BUFFER_FLOOR,
                1,
            )),
            trace: false,
            sockets: SourceSockets::Dial {
                host: "127.0.0.1".into(),
                tcp_port: 0,
            },
            phase_trace: None,
            terminal_probes: None,
            small_file_probe: None,
            queue_trace_armed: AtomicBool::new(false),
        };

        let error = plane
            .queue(TransferPayload::File(FileHeader {
                relative_path: "unused.bin".into(),
                ..Default::default()
            }))
            .await
            .expect_err("closed payload queue must surface its worker failure");
        let rendered = format!("{error:#}");
        assert!(
            rendered.contains("injected source worker failure"),
            "worker cause was replaced by a queue symptom: {rendered}"
        );
    }

    #[tokio::test]
    async fn source_close_surfaces_a_worker_error_that_won_the_seal_race() {
        let tmp = tempdir().unwrap();
        let source: Arc<dyn TransferSource> =
            Arc::new(FsTransferSource::new(tmp.path().to_path_buf()));
        let (payload_tx, _payload_rx) = mpsc::channel(1);
        let pipeline = AbortOnDrop::new(tokio::spawn(async {
            Err::<ElasticPipelineOutcome, _>(
                eyre::Report::new(FaultedPath("big.bin".into()))
                    .wrap_err("injected early worker failure"),
            )
        }));
        let (control, commands) = ElasticPipelineControl::channel();
        drop(commands);
        let mut plane = SourceDataPlane {
            payload_tx: Some(payload_tx),
            control: Some(control),
            pipeline: tokio::sync::Mutex::new(Some(pipeline)),
            tuner: None,
            resize_rx: None,
            dial: TransferDial::conservative().shared(),
            next_member_id: AtomicU32::new(1),
            source,
            session_token: Vec::new(),
            pool: Arc::new(BufferPool::for_data_plane(
                crate::buffer::DATA_PLANE_BUFFER_FLOOR,
                1,
            )),
            trace: false,
            sockets: SourceSockets::Dial {
                host: "127.0.0.1".into(),
                tcp_port: 0,
            },
            phase_trace: None,
            terminal_probes: None,
            small_file_probe: None,
            queue_trace_armed: AtomicBool::new(false),
        };

        let error = plane
            .close_payloads_preserving_pipeline_error()
            .await
            .expect_err("the worker cause must replace the failed Seal symptom");
        let fault = error
            .downcast_ref::<SessionFault>()
            .expect("the worker cause becomes a structured data-plane fault");
        assert_eq!(fault.relative_path.as_deref(), Some("big.bin"));
        assert!(fault.message.contains("injected early worker failure"));
        assert!(!fault.message.contains("before it was sealed"));
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
        let granted_headers: GrantedHeaders = Arc::new(StdMutex::new(HashMap::from([(
            "a.txt".to_string(),
            FileHeader {
                relative_path: "a.txt".to_string(),
                ..Default::default()
            },
        )])));
        let sink = NeedListSink::new(
            Arc::new(NullSink::new()),
            Arc::clone(&outstanding),
            granted_headers,
            None,
            None,
        );

        let file = |path: &str| {
            PreparedPayload::File(FileHeader {
                relative_path: path.to_string(),
                ..Default::default()
            })
        };

        // Metadata that was absent from the manifest is rejected before the
        // need or retained header is consumed, so no sink can publish success.
        let err = sink
            .write_payload(PreparedPayload::File(FileHeader {
                relative_path: "a.txt".to_string(),
                windows_metadata: Some(crate::generated::WindowsFileMetadata {
                    file_attributes: 0,
                    named_streams: vec![],
                }),
                ..Default::default()
            }))
            .await
            .expect_err("metadata mismatch must fault before claim");
        assert!(format!("{err:#}").contains("Windows metadata"));
        assert!(outstanding.lock().expect("lock").contains("a.txt"));

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
                windows_metadata: None,
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
            GrantedHeaders::default(),
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
                windows_metadata: None,
            })
            .await
            .expect_err("completion at the wrong size must fault");

        // Replacing the same retained grant is harmless and keeps this setup
        // explicit for the happy-path completion claim.
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
            windows_metadata: None,
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
                windows_metadata: None,
            })
            .await
            .expect_err("duplicate completion must fault");
    }
}
