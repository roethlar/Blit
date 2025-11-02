use crate::generated::PushSummary;
use std::time::Duration;
use tokio::sync::mpsc::UnboundedSender;

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

#[derive(Debug, Clone)]
pub enum ProgressEvent {
    ManifestBatch { files: usize },
    Payload { files: usize, bytes: u64 },
}

#[derive(Clone)]
pub struct RemotePushProgress {
    sender: UnboundedSender<ProgressEvent>,
}

impl RemotePushProgress {
    pub fn new(sender: UnboundedSender<ProgressEvent>) -> Self {
        Self { sender }
    }

    pub fn report_manifest_batch(&self, files: usize) {
        let _ = self.sender.send(ProgressEvent::ManifestBatch { files });
    }

    pub fn report_payload(&self, files: usize, bytes: u64) {
        let _ = self.sender.send(ProgressEvent::Payload { files, bytes });
    }
}
