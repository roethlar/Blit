//! `df` — the filesystem that holds a path on a daemon
//! (`docs/plan/UTILITY_VERBS_LOCAL_PARITY.md` slice 1).
//!
//! This module speaks gRPC and returns the structured answer; a front
//! end formats it ([`crate::disk_usage`] has the numbers as `df` prints
//! them).

use crate::generated::DfRequest;
use crate::remote::RemoteEndpoint;
use eyre::Result;
use serde::Serialize;

/// The filesystem that holds a path on a daemon, as `df` reports it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct FilesystemSpace {
    pub total_bytes: u64,
    /// The size less every free block, whoever may use it.
    pub used_bytes: u64,
    /// What an ordinary user may still fill.
    pub available_bytes: u64,
}

impl From<FilesystemSpace> for crate::disk_usage::FilesystemSpace {
    fn from(space: FilesystemSpace) -> Self {
        Self {
            total_bytes: space.total_bytes,
            used_bytes: space.used_bytes,
            available_bytes: space.available_bytes,
        }
    }
}

/// Ask `remote`'s daemon about the filesystem that holds `path` below
/// `module` (empty: the module itself). The daemon answers only an asker
/// of its own contract ([`super::same_build`]).
pub async fn query(
    remote: &RemoteEndpoint,
    module: String,
    path: super::du::EntryPath,
) -> Result<FilesystemSpace> {
    let uri = remote.control_plane_uri();
    let mut client = crate::client::connect_with_timeout(uri.clone()).await?;

    let reply = client
        .df(DfRequest {
            contract_version: crate::transfer_session::CONTRACT_VERSION,
            module,
            path: path.text,
            raw_path: path.raw,
        })
        .await
        .map_err(|status| super::same_build::refusal("df", status))?
        .into_inner();

    Ok(FilesystemSpace {
        total_bytes: reply.total_bytes,
        used_bytes: reply.used_bytes,
        available_bytes: reply.available_bytes,
    })
}
