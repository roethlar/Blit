//! Session-side TCP data-plane orchestration (otp-4b).
//!
//! The unified session reuses blit-core's data-plane byte plumbing —
//! [`DataPlaneSession`] record framing, [`execute_receive_pipeline`],
//! [`execute_sink_pipeline_streaming`], [`dial_data_plane`] — but owns
//! its OWN choreography here. The push-specific bind/arm/accept loop
//! (`blit-daemon` push service) and the multi-stream send driver
//! (`remote::push::client`) are per-direction drivers ONE_TRANSFER_PATH
//! deletes at cutover (otp-10), so nothing in this file calls into them.
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
//! via [`SinkControl::Add`]. The cheap-dial live tuner (chunk/prefetch) is
//! still future work — the resize moves only the stream count.

use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex as StdMutex};

use async_trait::async_trait;
use eyre::Result;
use tokio::io::AsyncReadExt;
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::mpsc;
use tokio::task::JoinSet;

use crate::buffer::BufferPool;
use crate::engine::{initial_stream_proposal, local_receiver_capacity, TransferDial};
use crate::generated::{session_error::Code, CapacityProfile, DataPlaneGrant, FileHeader};
use crate::remote::transfer::payload::{PreparedPayload, TransferPayload};
use crate::remote::transfer::pipeline::execute_receive_pipeline;
use crate::remote::transfer::sink::{DataPlaneSink, SinkOutcome, TransferSink};
use crate::remote::transfer::socket::{
    configure_data_socket, dial_data_plane, DATA_PLANE_ACCEPT_TIMEOUT, DATA_PLANE_TOKEN_TIMEOUT,
};
use crate::remote::transfer::source::TransferSource;
use crate::remote::transfer::stall_guard::{StallGuard, TRANSFER_STALL_TIMEOUT};
use crate::remote::transfer::{
    execute_sink_pipeline_elastic, generate_sub_token, AbortOnDrop, DataPlaneSession, SinkControl,
    SUB_TOKEN_LEN,
};

use super::SessionFault;

/// The set of granted-but-not-yet-received needs, shared between the
/// destination's control loop (which inserts each path before sending
/// its `NeedBatch`) and the data-plane receive (which claims each path
/// as its payload lands). Completion is an empty set — the same signal
/// the in-stream carrier uses via its inline `outstanding.remove`.
pub(super) type OutstandingNeeds = Arc<StdMutex<HashSet<String>>>;

fn dp_fault(msg: impl Into<String>) -> eyre::Report {
    eyre::Report::new(SessionFault::refusal(Code::DataPlaneFailed, msg))
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
    arm_tx: mpsc::UnboundedSender<Vec<u8>>,
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

impl ResponderDataPlane {
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
    pub(super) fn spawn(self, sink: Arc<dyn TransferSink>) -> ResponderDataPlaneRun {
        let ceiling = local_receiver_capacity().max_streams.max(1) as usize;
        let session_token = self.session_token.clone();
        let (arm_tx, arm_rx) = mpsc::unbounded_channel::<Vec<u8>>();
        let task = AbortOnDrop::new(tokio::spawn(self.accept_loop(sink, arm_rx)));
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
        arm_rx: mpsc::UnboundedReceiver<Vec<u8>>,
    ) -> Result<ReceiveTotals> {
        // Epoch-0 socket credential: session_token ‖ epoch0_sub_token.
        let mut epoch0 = self.session_token.clone();
        epoch0.extend_from_slice(&self.epoch0_sub_token);

        let mut receives: JoinSet<Result<SinkOutcome>> = JoinSet::new();
        let mut total = SinkOutcome::default();
        let mut streams = 0usize;

        // Accept the initial epoch-0 socket(s) first (the zero-knowledge
        // grant is always 1; the loop handles N for symmetry).
        for _ in 0..self.initial_streams {
            let socket = accept_authenticated(&self.listener, &epoch0).await?;
            streams += 1;
            spawn_receive(&mut receives, socket, &sink);
        }

        // Resize ADDs: each arms a `session_token ‖ sub_token` credential
        // whose socket the SOURCE dials right after its ack. `no_more` is
        // set when the control loop drops the arm sender at `SourceDone`;
        // the loop then drains the last armed sockets and workers. Because
        // the SOURCE only dials a credential it was acked for (and a dial
        // failure faults the whole session, aborting this task via
        // AbortOnDrop), an armed slot is always consumed — no orphan hang.
        let mut armed: Vec<Vec<u8>> = Vec::new();
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
                    Some(sub_token) => armed.push(sub_token),
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
                    let socket =
                        authenticate_resize(socket, &self.session_token, &mut armed).await?;
                    streams += 1;
                    spawn_receive(&mut receives, socket, &sink);
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
    pub(super) fn arm(&self, sub_token: Vec<u8>) -> bool {
        self.arm_tx.send(sub_token).is_ok()
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
) {
    let sink = Arc::clone(sink);
    receives.spawn(async move {
        let mut guarded = StallGuard::new(socket, TRANSFER_STALL_TIMEOUT);
        execute_receive_pipeline(&mut guarded, sink, None).await
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
    armed: &mut Vec<Vec<u8>>,
) -> Result<TcpStream> {
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
    match armed.iter().position(|t| t.as_slice() == sub) {
        Some(idx) => {
            armed.swap_remove(idx);
            Ok(socket)
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
) -> Result<InitiatorReceivePlaneRun> {
    let initial = grant.initial_streams.max(1) as usize;
    // Epoch-0 handshake: session_token ‖ epoch0_sub_token.
    let mut handshake = grant.session_token.clone();
    handshake.extend_from_slice(&grant.epoch0_sub_token);
    let addr = format!("{host}:{}", grant.tcp_port);

    let mut receives: JoinSet<Result<SinkOutcome>> = JoinSet::new();
    let mut streams = 0usize;
    for _ in 0..initial {
        // `dial_data_plane` connects, applies the data-socket policy, and
        // writes the handshake credential — the same bounded dial the
        // SOURCE initiator uses (design-3: one owner for every client-side
        // data-plane dial, both directions).
        let socket = dial_data_plane(&addr, &handshake, None)
            .await
            .map_err(|err| dp_fault(format!("dialing session data plane (receive): {err:#}")))?;
        streams += 1;
        spawn_receive(&mut receives, socket, &sink);
    }
    Ok(InitiatorReceivePlaneRun {
        receives,
        streams,
        host: host.to_string(),
        tcp_port: grant.tcp_port,
        session_token: grant.session_token.clone(),
        sink,
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
    pub(super) async fn add_dialed_stream(&mut self, sub_token: &[u8]) -> Result<()> {
        let mut handshake = self.session_token.clone();
        handshake.extend_from_slice(sub_token);
        let addr = format!("{}:{}", self.host, self.tcp_port);
        let socket = dial_data_plane(&addr, &handshake, None)
            .await
            .map_err(|err| dp_fault(format!("dialing resize data plane (receive): {err:#}")))?;
        self.streams += 1;
        spawn_receive(&mut self.receives, socket, &self.sink);
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
/// as an ELASTIC sink pipeline that `SinkControl::Add` grows mid-run (the
/// sf-2 shape correction). Planned payloads are fed via [`Self::queue`];
/// closing via [`Self::finish`] drains the pipeline, emits each socket's
/// END record, and returns the bytes this end sent.
pub(super) struct SourceDataPlane {
    payload_tx: Option<mpsc::Sender<TransferPayload>>,
    control_tx: mpsc::UnboundedSender<SinkControl>,
    // `AbortOnDrop<T>` wraps a `JoinHandle<T>`; the task's output is
    // `Result<SinkOutcome>`, so `T` is that (not the JoinHandle).
    pipeline: Option<AbortOnDrop<Result<SinkOutcome>>>,
    // The byte SENDER owns the live dial, bounded by the byte RECEIVER's
    // advertised capacity (contract §Invariants 5). The resize drives only
    // its shape-correction stream count; the cheap-dial tuner is future
    // work, so `chunk_bytes()`/`prefetch_count()` stay at the floor.
    dial: Arc<TransferDial>,
    source: Arc<dyn TransferSource>,
    session_token: Vec<u8>,
    pool: Arc<BufferPool>,
    /// How each epoch-N resize socket is acquired (dial for the SOURCE
    /// initiator, accept for the SOURCE responder). The data plane grows
    /// mid-transfer in both cases; the control-lane resize choreography is
    /// identical — only this transport action flips (otp-5b-2).
    sockets: SourceSockets,
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
    let mut sinks: Vec<Arc<dyn TransferSink>> = Vec::with_capacity(initial);
    for _ in 0..initial {
        let session = DataPlaneSession::connect(
            host,
            grant.tcp_port,
            &handshake,
            dial.chunk_bytes(),
            dial.prefetch_count(),
            false,
            dial.tcp_buffer_bytes(),
            Arc::clone(&pool),
        )
        .await
        .map_err(|err| dp_fault(format!("dialing session data plane: {err:#}")))?;
        // The source-side sink never reads its dst_root (it only sends);
        // `root()` is consulted by the relay/receive case, not here.
        sinks.push(Arc::new(DataPlaneSink::new(
            session,
            Arc::clone(&source),
            PathBuf::new(),
        )));
    }

    let prefetch = dial.prefetch_count().max(1);
    let (payload_tx, payload_rx) = mpsc::channel::<TransferPayload>(prefetch);
    let (control_tx, control_rx) = mpsc::unbounded_channel::<SinkControl>();
    let pipe_source = Arc::clone(&source);
    // Bounded by AbortOnDrop: a fault on the control lane that drops the
    // SourceDataPlane aborts the pipeline task instead of leaking it.
    let pipeline = AbortOnDrop::new(tokio::spawn(async move {
        execute_sink_pipeline_elastic(
            pipe_source,
            sinks,
            payload_rx,
            prefetch,
            None,
            Some(control_rx),
        )
        .await
    }));
    Ok(SourceDataPlane {
        payload_tx: Some(payload_tx),
        control_tx,
        pipeline: Some(pipeline),
        dial,
        source,
        session_token: grant.session_token.clone(),
        pool,
        // SOURCE initiator: each epoch-N resize socket is dialed to the
        // granted host:port.
        sockets: SourceSockets::Dial {
            host: host.to_string(),
            tcp_port: grant.tcp_port,
        },
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
    let mut sinks: Vec<Arc<dyn TransferSink>> = Vec::with_capacity(initial);
    for _ in 0..initial {
        let socket = accept_authenticated(&bound.listener, &epoch0).await?;
        let session = DataPlaneSession::from_stream(
            socket,
            false,
            dial.chunk_bytes(),
            dial.prefetch_count(),
            Arc::clone(&pool),
        )
        .await;
        sinks.push(Arc::new(DataPlaneSink::new(
            session,
            Arc::clone(&source),
            PathBuf::new(),
        )));
    }

    let prefetch = dial.prefetch_count().max(1);
    let (payload_tx, payload_rx) = mpsc::channel::<TransferPayload>(prefetch);
    let (control_tx, control_rx) = mpsc::unbounded_channel::<SinkControl>();
    let pipe_source = Arc::clone(&source);
    let pipeline = AbortOnDrop::new(tokio::spawn(async move {
        execute_sink_pipeline_elastic(
            pipe_source,
            sinks,
            payload_rx,
            prefetch,
            None,
            Some(control_rx),
        )
        .await
    }));
    Ok(SourceDataPlane {
        payload_tx: Some(payload_tx),
        control_tx,
        pipeline: Some(pipeline),
        dial,
        source,
        session_token: bound.session_token,
        pool,
        // SOURCE responder: each epoch-N resize socket is accepted off the
        // same listener epoch-0 came in on (otp-5b-2).
        sockets: SourceSockets::Accept {
            listener: bound.listener,
        },
    })
}

impl SourceDataPlane {
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
    /// to the running pipeline (`SinkControl::Add`). The SOURCE initiator
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
    pub(super) async fn add_stream(&self, sub_token: &[u8]) -> Result<()> {
        let session = match &self.sockets {
            SourceSockets::Dial { host, tcp_port } => {
                let mut handshake = self.session_token.clone();
                handshake.extend_from_slice(sub_token);
                DataPlaneSession::connect(
                    host,
                    *tcp_port,
                    &handshake,
                    self.dial.chunk_bytes(),
                    self.dial.prefetch_count(),
                    false,
                    self.dial.tcp_buffer_bytes(),
                    Arc::clone(&self.pool),
                )
                .await
                .map_err(|err| dp_fault(format!("dialing resize data socket: {err:#}")))?
            }
            SourceSockets::Accept { listener } => {
                let mut expected = self.session_token.clone();
                expected.extend_from_slice(sub_token);
                let socket = accept_authenticated(listener, &expected).await?;
                DataPlaneSession::from_stream(
                    socket,
                    false,
                    self.dial.chunk_bytes(),
                    self.dial.prefetch_count(),
                    Arc::clone(&self.pool),
                )
                .await
            }
        };
        let sink: Arc<dyn TransferSink> = Arc::new(DataPlaneSink::new(
            session,
            Arc::clone(&self.source),
            PathBuf::new(),
        ));
        if let Err(returned) = self.control_tx.send(SinkControl::Add(sink)) {
            if let SinkControl::Add(sink) = returned.0 {
                let _ = sink.finish().await;
            }
        }
        Ok(())
    }

    /// Feed one planned batch into the send pipeline. The pipeline
    /// prepares each payload (tar-shard/file) and writes it through the
    /// data-plane record framing across the live socket(s).
    pub(super) async fn queue(&mut self, payloads: Vec<TransferPayload>) -> Result<()> {
        let tx = self.payload_tx.as_ref().ok_or_else(|| {
            eyre::Report::new(SessionFault::internal("data plane already finished"))
        })?;
        for payload in payloads {
            tx.send(payload).await.map_err(|_| {
                dp_fault("data-plane send pipeline closed before all payloads sent")
            })?;
        }
        Ok(())
    }

    /// Signal end-of-stream, drain the pipeline (each worker emits its
    /// socket's END record on drain), and return the bytes sent. Must be
    /// awaited before `SourceDone` goes out so the destination's receive
    /// pipeline sees END and completes.
    pub(super) async fn finish(mut self) -> Result<SinkOutcome> {
        // Drop the sender: workers observe the closed queue, drain what
        // is left, then `finish()` (END record) and exit.
        self.payload_tx = None;
        let pipeline = self
            .pipeline
            .take()
            .expect("SourceDataPlane::finish called once");
        pipeline
            .join()
            .await
            .map_err(|err| dp_fault(format!("data-plane send pipeline panicked: {err}")))?
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
/// records the non-resume session never negotiated (codex otp-4b-1 F1).
/// Every written path must be a granted, not-yet-received need; resume
/// block records are rejected outright. The shared [`OutstandingNeeds`]
/// set makes completion `is_empty()` for both carriers.
pub(super) struct NeedListSink {
    inner: Arc<dyn TransferSink>,
    outstanding: OutstandingNeeds,
}

impl NeedListSink {
    pub(super) fn new(inner: Arc<dyn TransferSink>, outstanding: OutstandingNeeds) -> Self {
        Self { inner, outstanding }
    }

    /// Remove `path` from the outstanding set, or fault: a path that is
    /// not present is either off the need list or a duplicate delivery.
    fn claim(&self, path: &str) -> Result<()> {
        if self
            .outstanding
            .lock()
            .expect("outstanding-needs lock poisoned")
            .remove(path)
        {
            Ok(())
        } else {
            Err(eyre::Report::new(SessionFault::protocol_violation(
                format!(
                    "data-plane payload for '{path}' which is not an outstanding need \
                 (off the need list, or a duplicate delivery)"
                ),
            )))
        }
    }
}

#[async_trait]
impl TransferSink for NeedListSink {
    async fn write_payload(&self, payload: PreparedPayload) -> Result<SinkOutcome> {
        match &payload {
            PreparedPayload::File(header) => self.claim(&header.relative_path)?,
            PreparedPayload::TarShard { headers, .. } => {
                for header in headers {
                    self.claim(&header.relative_path)?;
                }
            }
            // The session did not negotiate resume (otp-7), so a block
            // record on the data plane is a protocol violation, not a
            // silently-applied patch.
            PreparedPayload::FileBlock { .. } | PreparedPayload::FileBlockComplete { .. } => {
                return Err(eyre::Report::new(SessionFault::protocol_violation(
                    "resume block record on the data plane of a non-resume session",
                )));
            }
        }
        self.inner.write_payload(payload).await
    }

    async fn write_file_stream(
        &self,
        header: &FileHeader,
        reader: &mut (dyn tokio::io::AsyncRead + Unpin + Send),
    ) -> Result<SinkOutcome> {
        self.claim(&header.relative_path)?;
        self.inner.write_file_stream(header, reader).await
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
        let sink = NeedListSink::new(Arc::new(NullSink::new()), Arc::clone(&outstanding));

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
}
