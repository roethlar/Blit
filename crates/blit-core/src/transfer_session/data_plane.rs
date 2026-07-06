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
//! otp-4b-1 scope: a single epoch-0 stream, no resize. The RESPONDER
//! (whichever end is DESTINATION for otp-4/-5) binds a listener, mints
//! the tokens, grants them in `SessionAccept`, and accepts + receives;
//! the INITIATOR (SOURCE here) dials + authenticates + sends. Because
//! the grant is issued before any manifest is seen,
//! [`initial_stream_proposal`] with zero knowledge is 1 — the session
//! data plane always starts single-stream and grows only via
//! SOURCE-driven resize, which lands at otp-4b-2.

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
use crate::engine::{
    initial_stream_proposal, local_receiver_capacity, DIAL_FLOOR_CHUNK_BYTES, DIAL_FLOOR_PREFETCH,
};
use crate::generated::{session_error::Code, DataPlaneGrant, FileHeader};
use crate::remote::transfer::payload::{PreparedPayload, TransferPayload};
use crate::remote::transfer::pipeline::execute_receive_pipeline;
use crate::remote::transfer::sink::{DataPlaneSink, SinkOutcome, TransferSink};
use crate::remote::transfer::socket::{
    configure_data_socket, DATA_PLANE_ACCEPT_TIMEOUT, DATA_PLANE_TOKEN_TIMEOUT,
};
use crate::remote::transfer::source::TransferSource;
use crate::remote::transfer::stall_guard::{StallGuard, TRANSFER_STALL_TIMEOUT};
use crate::remote::transfer::{
    execute_sink_pipeline_streaming, generate_sub_token, AbortOnDrop, DataPlaneSession,
};

use super::SessionFault;

/// The set of granted-but-not-yet-received needs, shared between the
/// destination's control loop (which inserts each path before sending
/// its `NeedBatch`) and the data-plane receive (which claims each path
/// as its payload lands). Completion is an empty set — the same signal
/// the in-stream carrier uses via its inline `outstanding.remove`.
pub(super) type OutstandingNeeds = Arc<StdMutex<HashSet<String>>>;

/// Dial values for the session data plane. otp-4b-1 has no live dial
/// tuner, so it runs at the engine floor — the conservative start the
/// dial contract mandates (absent/0 capacity fields ⇒ conservative,
/// never unlimited). A live dial + tuner is future work, not this slice.
const SESSION_DP_CHUNK_BYTES: usize = DIAL_FLOOR_CHUNK_BYTES;
const SESSION_DP_PREFETCH: usize = DIAL_FLOOR_PREFETCH;

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

    /// Accept exactly `initial_streams` authenticated data sockets and
    /// drain each into `sink` via the shared receive pipeline, returning
    /// the aggregated write outcome (the DESTINATION is the scorer). The
    /// caller runs this concurrently with the control-stream diff loop
    /// and joins it on `SourceDone`.
    pub(super) async fn accept_and_receive(
        self,
        sink: Arc<dyn TransferSink>,
    ) -> Result<SinkOutcome> {
        // Epoch-0 socket credential: session_token ‖ epoch0_sub_token.
        let mut expected = self.session_token.clone();
        expected.extend_from_slice(&self.epoch0_sub_token);

        let mut receives: JoinSet<Result<SinkOutcome>> = JoinSet::new();
        for _ in 0..self.initial_streams {
            let socket = accept_authenticated(&self.listener, &expected).await?;
            let sink = Arc::clone(&sink);
            receives.spawn(async move {
                // Read-side StallGuard (carried REV4 RELIABLE invariant,
                // matching the old push receive): a peer that authenticates
                // then stalls mid-record trips the transfer stall timeout
                // instead of pinning this task until TCP keepalive.
                let mut guarded = StallGuard::new(socket, TRANSFER_STALL_TIMEOUT);
                execute_receive_pipeline(&mut guarded, sink, None).await
            });
        }

        let mut total = SinkOutcome::default();
        while let Some(joined) = receives.join_next().await {
            let outcome =
                joined.map_err(|err| dp_fault(format!("receive task panicked: {err}")))??;
            total.files_written += outcome.files_written;
            total.bytes_written += outcome.bytes_written;
        }
        Ok(total)
    }
}

/// Accept one data socket under the shared bounded-accept timeout, apply
/// the data-plane socket policy, read the fixed-length credential under
/// the shared bounded-read timeout, and verify it. A socket presenting
/// anything else is a `DATA_PLANE_FAILED` fault (contract §Transport: a
/// mismatched socket is closed without response — here the whole session
/// faults, since otp-4b-1 arms exactly the sockets it dials).
async fn accept_authenticated(listener: &TcpListener, expected: &[u8]) -> Result<TcpStream> {
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

    let mut socket = socket;
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

// ---------------------------------------------------------------------------
// Initiator (SOURCE) — dial, authenticate, send
// ---------------------------------------------------------------------------

/// A running source-side data plane: the dialed socket(s) wrapped as a
/// sink pipeline. Planned payloads are fed via [`Self::queue`]; closing
/// via [`Self::finish`] drains the pipeline, emits each socket's END
/// record, and returns the bytes this end sent.
pub(super) struct SourceDataPlane {
    payload_tx: Option<mpsc::Sender<TransferPayload>>,
    // `AbortOnDrop<T>` wraps a `JoinHandle<T>`; the task's output is
    // `Result<SinkOutcome>`, so `T` is that (not the JoinHandle).
    pipeline: Option<AbortOnDrop<Result<SinkOutcome>>>,
}

/// Dial the granted data plane and start the send pipeline. `host` is
/// the responder's host (the initiator connected the control plane to
/// it; the data plane rides the same host on the granted port —
/// contract §Transport: the initiator always dials).
pub(super) async fn dial_source_data_plane(
    host: &str,
    grant: &DataPlaneGrant,
    source: Arc<dyn TransferSource>,
) -> Result<SourceDataPlane> {
    let streams = grant.initial_streams.max(1) as usize;
    // Epoch-0 handshake: session_token ‖ epoch0_sub_token.
    let mut handshake = grant.session_token.clone();
    handshake.extend_from_slice(&grant.epoch0_sub_token);

    let pool = Arc::new(BufferPool::for_data_plane(SESSION_DP_CHUNK_BYTES, streams));
    let mut sinks: Vec<Arc<dyn TransferSink>> = Vec::with_capacity(streams);
    for _ in 0..streams {
        let session = DataPlaneSession::connect(
            host,
            grant.tcp_port,
            &handshake,
            SESSION_DP_CHUNK_BYTES,
            SESSION_DP_PREFETCH,
            false,
            None,
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

    let (payload_tx, payload_rx) = mpsc::channel::<TransferPayload>(SESSION_DP_PREFETCH.max(1));
    // Bounded by AbortOnDrop: a fault on the control lane that drops the
    // SourceDataPlane aborts the pipeline task instead of leaking it.
    let pipeline = AbortOnDrop::new(tokio::spawn(async move {
        execute_sink_pipeline_streaming(source, sinks, payload_rx, SESSION_DP_PREFETCH, None).await
    }));
    Ok(SourceDataPlane {
        payload_tx: Some(payload_tx),
        pipeline: Some(pipeline),
    })
}

impl SourceDataPlane {
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
    use crate::remote::transfer::SUB_TOKEN_LEN;

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
