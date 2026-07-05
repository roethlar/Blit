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
