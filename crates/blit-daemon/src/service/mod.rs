mod admin;
mod core;
pub(crate) mod delegated_pull;
#[cfg(test)]
mod delegated_session_e2e;
mod pull_sync;
mod push;
mod transfer;
#[cfg(test)]
mod transfer_session_e2e;
mod util;

pub use core::{spawn_progress_ticker, BlitServer, BlitService};

use blit_core::generated::{DiskUsageEntry, FindEntry, ServerPullMessage, ServerPushResponse};
use tokio::sync::mpsc;
use tonic::Status;

pub(crate) type PushSender = mpsc::Sender<Result<ServerPushResponse, Status>>;
pub(crate) type PullSyncSender = mpsc::Sender<Result<ServerPullMessage, Status>>;
pub(crate) type FindSender = mpsc::Sender<Result<FindEntry, Status>>;
pub(crate) type DiskUsageSender = mpsc::Sender<Result<DiskUsageEntry, Status>>;
