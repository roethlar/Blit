use crate::generated::PushSummary;

#[derive(Debug, Clone)]
pub struct RemotePushReport {
    pub files_requested: Vec<String>,
    pub fallback_used: bool,
    pub data_port: Option<u32>,
    pub summary: PushSummary,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TransferMode {
    Undecided,
    DataPlane,
    Fallback,
}
