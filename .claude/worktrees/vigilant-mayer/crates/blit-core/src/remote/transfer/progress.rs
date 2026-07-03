use tokio::sync::mpsc::UnboundedSender;

#[derive(Debug, Clone)]
pub enum ProgressEvent {
    ManifestBatch { files: usize },
    Payload { files: usize, bytes: u64 },
    FileComplete { path: String, bytes: u64 },
}

#[derive(Clone)]
pub struct RemoteTransferProgress {
    sender: UnboundedSender<ProgressEvent>,
}

impl RemoteTransferProgress {
    pub fn new(sender: UnboundedSender<ProgressEvent>) -> Self {
        Self { sender }
    }

    pub fn report_manifest_batch(&self, files: usize) {
        let _ = self.sender.send(ProgressEvent::ManifestBatch { files });
    }

    pub fn report_payload(&self, files: usize, bytes: u64) {
        let _ = self.sender.send(ProgressEvent::Payload { files, bytes });
    }

    pub fn report_file_complete(&self, path: String, bytes: u64) {
        let _ = self
            .sender
            .send(ProgressEvent::FileComplete { path, bytes });
    }
}
