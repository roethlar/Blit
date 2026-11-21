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

use blit_core::generated::{DuReply, FindPathsReply};
use tokio::sync::mpsc;
use tonic::Status;

pub(crate) type FindSender = mpsc::Sender<Result<FindPathsReply, Status>>;
pub(crate) type DiskUsageSender = mpsc::Sender<Result<DuReply, Status>>;
pub(crate) type RmSender = mpsc::Sender<Result<blit_core::generated::RmReply, Status>>;
pub(crate) type LsSender = mpsc::Sender<Result<blit_core::generated::LsReply, Status>>;
