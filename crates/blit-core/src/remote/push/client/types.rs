use crate::generated::PushSummary;
use crate::remote::transfer::progress::RemoteTransferProgress;
use std::time::Duration;

#[derive(Debug, Clone)]
pub struct RemotePushReport {
    pub files_requested: Vec<String>,
    pub fallback_used: bool,
    pub data_port: Option<u32>,
    pub summary: PushSummary,
    pub first_payload_elapsed: Option<Duration>,
    /// sf-2: the dial's settled live stream count when the transfer
    /// finished (`None` on the gRPC fallback path — no data plane).
    /// Observable pin for the shape-correction resize: a many-tiny-file
    /// push must end above the 1-stream partial-manifest proposal.
    pub data_plane_streams: Option<usize>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TransferMode {
    Undecided,
    DataPlane,
    Fallback,
}

pub type RemotePushProgress = RemoteTransferProgress;
