//! `du` — the space allocated under a path on a daemon, streamed line by
//! line (`docs/plan/UTILITY_VERBS_LOCAL_PARITY.md` slice 1).
//!
//! The daemon walks its own disk and sends only the lines that will be
//! printed, in `du`'s order: an entry after everything below it, the start
//! last. This module speaks gRPC; a front end formats.

use crate::generated::{du_reply, DuRequest};
use crate::remote::RemoteEndpoint;
use eyre::Result;

pub use crate::disk_usage::UsageOptions;

/// One thing the daemon reported, in order.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DuEvent {
    /// A line: the space allocated to an entry and everything below it.
    Row {
        path: EntryPath,
        allocated_bytes: u64,
    },
    /// An entry the daemon could not read. Its walk went on; the command
    /// that asked exits 1, as `du` does.
    Failure {
        path: EntryPath,
        /// `cannot read directory` or `cannot access`.
        action: String,
        /// The operating system's words: `Permission denied`.
        reason: String,
    },
}

/// An entry's path below the start (`/`-separated, empty for the start
/// itself), as text and — when the name is not valid UTF-8 — as the exact
/// bytes the daemon's disk holds, so what is printed is the real name.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EntryPath {
    pub text: String,
    pub raw: Option<Vec<u8>>,
}

impl EntryPath {
    /// The path's bytes: the exact ones when it has them, its text's
    /// otherwise.
    pub fn bytes(&self) -> &[u8] {
        self.raw.as_deref().unwrap_or(self.text.as_bytes())
    }

    /// Whether this is the start itself.
    pub fn is_start(&self) -> bool {
        self.bytes().is_empty()
    }
}

/// Run `du` on `remote`'s daemon over `start_path` below `module`,
/// handing each line and each failure to `on_event` as it arrives. The
/// daemon answers only an asker of its own contract
/// ([`super::same_build`]).
pub async fn stream<F>(
    remote: &RemoteEndpoint,
    module: String,
    start_path: String,
    options: UsageOptions,
    mut on_event: F,
) -> Result<()>
where
    F: FnMut(DuEvent) -> Result<()>,
{
    let uri = remote.control_plane_uri();
    let mut client = crate::client::connect_with_timeout(uri.clone()).await?;

    let mut stream = client
        .du(DuRequest {
            contract_version: crate::transfer_session::CONTRACT_VERSION,
            module,
            start_path,
            all_files: options.all_files,
            max_depth: options.max_depth,
        })
        .await
        .map_err(|status| super::same_build::refusal("du", status))?
        .into_inner();

    while let Some(reply) = stream
        .message()
        .await
        .map_err(|status| super::same_build::refusal("du", status))?
    {
        match reply.reply {
            Some(du_reply::Reply::Row(row)) => on_event(DuEvent::Row {
                path: EntryPath {
                    text: row.relative_path,
                    raw: row.raw_relative_path,
                },
                allocated_bytes: row.allocated_bytes,
            })?,
            Some(du_reply::Reply::Failure(failure)) => on_event(DuEvent::Failure {
                path: EntryPath {
                    text: failure.relative_path,
                    raw: failure.raw_relative_path,
                },
                action: failure.action,
                reason: failure.reason,
            })?,
            // A reply of a kind this build does not know: same contract,
            // so there is none; nothing to print.
            None => {}
        }
    }

    Ok(())
}
