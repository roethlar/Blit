mod admin;
mod core;
mod pull;
mod push;
mod util;

pub use core::{BlitServer, BlitService};

use blit_core::generated::{
    pull_chunk::Payload as PullPayload, DiskUsageEntry, FindEntry, PullChunk, ServerPushResponse,
};
use tokio::sync::mpsc;
use tonic::Status;

pub(crate) type PushSender = mpsc::Sender<Result<ServerPushResponse, Status>>;
pub(crate) type PullSender = mpsc::Sender<Result<PullChunk, Status>>;
pub(crate) type FindSender = mpsc::Sender<Result<FindEntry, Status>>;
pub(crate) type DiskUsageSender = mpsc::Sender<Result<DiskUsageEntry, Status>>;
