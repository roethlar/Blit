mod admin;
mod core;
pub(crate) mod delegated_pull;
#[cfg(test)]
mod delegated_session_e2e;
mod transfer;
#[cfg(test)]
mod transfer_session_e2e;
mod util;

pub use core::{spawn_progress_ticker, BlitServer, BlitService};

use blit_core::generated::{DiskUsageEntry, FindEntry};
use tokio::sync::mpsc;
use tonic::Status;

pub(crate) type FindSender = mpsc::Sender<Result<FindEntry, Status>>;
pub(crate) type DiskUsageSender = mpsc::Sender<Result<DiskUsageEntry, Status>>;
