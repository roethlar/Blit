mod admin;
mod core;
mod pull;
mod pull_sync;
mod push;
mod util;

pub use core::{BlitServer, BlitService};

use blit_core::generated::{
    pull_chunk::Payload as PullPayload, DiskUsageEntry, FindEntry, PullChunk,
    ServerPullMessage, ServerPushResponse,
};
use tokio::sync::mpsc;
use tonic::Status;

pub(crate) type PushSender = mpsc::Sender<Result<ServerPushResponse, Status>>;
pub(crate) type PullSender = mpsc::Sender<Result<PullChunk, Status>>;
pub(crate) type PullSyncSender = mpsc::Sender<Result<ServerPullMessage, Status>>;
pub(crate) type FindSender = mpsc::Sender<Result<FindEntry, Status>>;
pub(crate) type DiskUsageSender = mpsc::Sender<Result<DiskUsageEntry, Status>>;
