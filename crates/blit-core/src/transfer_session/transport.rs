//! Frame transports for the unified transfer session.
//!
//! The session drivers in this module's parent speak
//! [`TransferFrame`]s through the `FrameTx`/`FrameRx` halves and never
//! know what carries them. otp-3 ships the in-process pair below;
//! otp-4 adds a gRPC-backed implementation over the `Transfer` RPC
//! (transport substitution, not new choreography —
//! docs/TRANSFER_SESSION.md); otp-11 reuses the in-process pair for
//! local transfers.

use async_trait::async_trait;
use eyre::{eyre, Result};
use tokio::sync::mpsc;
use tonic::{Status, Streaming};

use crate::generated::TransferFrame;

/// Sending half of a frame transport. `send` applies the transport's
/// own backpressure (bounded channel here, HTTP/2 flow control on the
/// wire) — the session contract deliberately leans on it instead of
/// buffering (docs/TRANSFER_SESSION.md §Phase state machine).
#[async_trait]
pub trait FrameTx: Send {
    async fn send(&mut self, frame: TransferFrame) -> Result<()>;
}

/// Receiving half of a frame transport. `Ok(None)` means the peer
/// closed the stream cleanly; transport-level failures are `Err`.
#[async_trait]
pub trait FrameRx: Send {
    async fn recv(&mut self) -> Result<Option<TransferFrame>>;
}

/// One endpoint's bidirectional frame stream, splittable so a driver
/// can run its send and receive halves concurrently (the source
/// driver must keep draining need batches while it streams manifest
/// entries, or a full channel in each direction deadlocks the pair).
pub struct FrameTransport {
    tx: Box<dyn FrameTx>,
    rx: Box<dyn FrameRx>,
}

impl FrameTransport {
    pub fn new(tx: Box<dyn FrameTx>, rx: Box<dyn FrameRx>) -> Self {
        Self { tx, rx }
    }

    pub async fn send(&mut self, frame: TransferFrame) -> Result<()> {
        self.tx.send(frame).await
    }

    pub async fn recv(&mut self) -> Result<Option<TransferFrame>> {
        self.rx.recv().await
    }

    pub fn split(self) -> (Box<dyn FrameTx>, Box<dyn FrameRx>) {
        (self.tx, self.rx)
    }
}

/// Bounded per-direction capacity of the in-process pair. Small on
/// purpose: the session must stay live under transport backpressure
/// (both drivers are exercised against it in the role suite), and a
/// deep channel would only hide ordering bugs the wire will expose.
pub const IN_PROCESS_CHANNEL_FRAMES: usize = 64;

struct MpscFrameTx {
    tx: mpsc::Sender<TransferFrame>,
}

#[async_trait]
impl FrameTx for MpscFrameTx {
    async fn send(&mut self, frame: TransferFrame) -> Result<()> {
        self.tx
            .send(frame)
            .await
            .map_err(|_| eyre!("in-process transport peer closed"))
    }
}

struct MpscFrameRx {
    rx: mpsc::Receiver<TransferFrame>,
}

#[async_trait]
impl FrameRx for MpscFrameRx {
    async fn recv(&mut self) -> Result<Option<TransferFrame>> {
        Ok(self.rx.recv().await)
    }
}

/// Two connected in-process endpoints: what one sends, the other
/// receives. Both roles of a local transfer (otp-11) — and every
/// otp-3 test — run over this pair.
pub fn in_process_pair() -> (FrameTransport, FrameTransport) {
    let (a_tx, b_rx) = mpsc::channel(IN_PROCESS_CHANNEL_FRAMES);
    let (b_tx, a_rx) = mpsc::channel(IN_PROCESS_CHANNEL_FRAMES);
    (
        FrameTransport::new(
            Box::new(MpscFrameTx { tx: a_tx }),
            Box::new(MpscFrameRx { rx: a_rx }),
        ),
        FrameTransport::new(
            Box::new(MpscFrameTx { tx: b_tx }),
            Box::new(MpscFrameRx { rx: b_rx }),
        ),
    )
}

// ---------------------------------------------------------------------------
// gRPC-backed transport (otp-4)
// ---------------------------------------------------------------------------
//
// The unified `Transfer` RPC is bidi-streaming and carries `TransferFrame`
// both directions, so a session end reads its inbound `tonic::Streaming`
// as a `FrameRx` and writes its outbound frames through an mpsc whose
// receiver becomes the peer's stream. Two `FrameTx` impls exist only
// because the client's outbound stream item is a bare `TransferFrame`
// (what `BlitClient::transfer` wants) while the daemon's outbound item is
// `Result<TransferFrame, Status>` (what its `ReceiverStream` response
// yields). Session-level refusals travel as `SessionError` *frames*
// (`Ok`); the gRPC `Status` channel is reserved for terminal transport
// faults.

/// Inbound half over a tonic server/response stream — shared by both
/// ends (client reads the response stream, daemon reads the request
/// stream). `message()`'s `Ok(None)`/`Err(Status)` map directly onto the
/// `FrameRx` clean-close / transport-error contract.
struct GrpcFrameRx {
    stream: Streaming<TransferFrame>,
}

#[async_trait]
impl FrameRx for GrpcFrameRx {
    async fn recv(&mut self) -> Result<Option<TransferFrame>> {
        self.stream
            .message()
            .await
            // Preserve the gRPC code in the message — the FrameRx trait
            // carries only eyre, and the session's own SessionError
            // frames are the semantic error channel; this Err is a raw
            // transport failure the drivers surface as INTERNAL.
            .map_err(|status| {
                eyre!(
                    "gRPC transport error ({:?}): {}",
                    status.code(),
                    status.message()
                )
            })
    }
}

/// Client outbound half: frames go into the mpsc whose `ReceiverStream`
/// was handed to `BlitClient::transfer` as the request stream.
struct GrpcClientFrameTx {
    tx: mpsc::Sender<TransferFrame>,
}

#[async_trait]
impl FrameTx for GrpcClientFrameTx {
    async fn send(&mut self, frame: TransferFrame) -> Result<()> {
        self.tx
            .send(frame)
            .await
            .map_err(|_| eyre!("transfer transport peer closed"))
    }
}

/// Daemon outbound half: the session's frames ride the RPC response
/// stream, whose item type is `Result<TransferFrame, Status>`. Frames
/// are always `Ok` — the `Status` variant is used by the handler for a
/// terminal transport fault, not by the session.
struct GrpcDaemonFrameTx {
    tx: mpsc::Sender<Result<TransferFrame, Status>>,
}

#[async_trait]
impl FrameTx for GrpcDaemonFrameTx {
    async fn send(&mut self, frame: TransferFrame) -> Result<()> {
        self.tx
            .send(Ok(frame))
            .await
            .map_err(|_| eyre!("transfer transport peer closed"))
    }
}

/// Bounded capacity of the mpsc feeding a gRPC stream direction — the
/// session leans on transport backpressure (this channel plus HTTP/2
/// flow control), matching the push handler's `mpsc::channel(32)`.
pub const GRPC_CHANNEL_FRAMES: usize = 32;

/// Assemble the CLIENT end's transport: `out_tx` feeds the request
/// stream (build it with `ReceiverStream::new(out_rx)` and hand that to
/// `BlitClient::transfer`), `inbound` is the response stream.
pub fn grpc_client_transport(
    out_tx: mpsc::Sender<TransferFrame>,
    inbound: Streaming<TransferFrame>,
) -> FrameTransport {
    FrameTransport::new(
        Box::new(GrpcClientFrameTx { tx: out_tx }),
        Box::new(GrpcFrameRx { stream: inbound }),
    )
}

/// Assemble the DAEMON end's transport: `out_tx` feeds the RPC response
/// `ReceiverStream`, `inbound` is the request stream
/// (`request.into_inner()`).
pub fn grpc_daemon_transport(
    out_tx: mpsc::Sender<Result<TransferFrame, Status>>,
    inbound: Streaming<TransferFrame>,
) -> FrameTransport {
    FrameTransport::new(
        Box::new(GrpcDaemonFrameTx { tx: out_tx }),
        Box::new(GrpcFrameRx { stream: inbound }),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::generated::{transfer_frame, SourceDone};

    fn source_done_frame() -> TransferFrame {
        TransferFrame {
            frame: Some(transfer_frame::Frame::SourceDone(SourceDone {})),
        }
    }

    #[tokio::test]
    async fn pair_delivers_frames_both_directions() {
        let (mut a, mut b) = in_process_pair();
        a.send(source_done_frame()).await.unwrap();
        b.send(source_done_frame()).await.unwrap();
        assert!(matches!(
            b.recv().await.unwrap().unwrap().frame,
            Some(transfer_frame::Frame::SourceDone(_))
        ));
        assert!(matches!(
            a.recv().await.unwrap().unwrap().frame,
            Some(transfer_frame::Frame::SourceDone(_))
        ));
    }

    #[tokio::test]
    async fn dropped_peer_reads_as_clean_close_and_send_error() {
        let (mut a, b) = in_process_pair();
        drop(b);
        assert!(a.recv().await.unwrap().is_none(), "closed peer = Ok(None)");
        assert!(a.send(source_done_frame()).await.is_err());
    }
}
