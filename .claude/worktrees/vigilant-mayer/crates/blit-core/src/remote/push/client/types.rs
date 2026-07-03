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
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TransferMode {
    Undecided,
    DataPlane,
    Fallback,
}

pub type RemotePushProgress = RemoteTransferProgress;
