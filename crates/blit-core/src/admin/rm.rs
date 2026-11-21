//! `rm` — remove paths on a remote daemon, as the Unix command does
//! (`docs/plan/UTILITY_VERBS_LOCAL_PARITY.md` slice 2).
//!
//! One request per daemon and module carries every operand; the daemon —
//! the side that knows what each path is — answers with each entry it
//! could not remove. The question before deleting, the messages and the
//! exit status are the front end's. The CLI's `move` verb uses
//! [`delete_remote_path`] for its source-side removal step.

use crate::admin::du::EntryPath;
use crate::generated::RmRequest;
use crate::remote::endpoint::{RemoteEndpoint, RemotePath};
use eyre::{bail, Result};
use std::path::PathBuf;

pub use crate::deletion::RmOptions;

/// One entry the daemon could not remove.
#[derive(Debug, Clone)]
pub struct RmFailure {
    /// Which operand it belongs to: its place in the list sent.
    pub operand: usize,
    /// The entry below that operand; the operand itself when
    /// [`EntryPath::is_start`].
    pub path: EntryPath,
    /// Why, in the operating system's own words.
    pub reason: String,
}

/// What a removal did.
#[derive(Debug, Clone, Default)]
pub struct RmReport {
    /// Every entry that could not be removed, operand by operand.
    pub failures: Vec<RmFailure>,
    /// Entries removed, the operands and everything below them.
    pub entries_removed: u64,
    /// More entries failed than `failures` lists.
    pub failures_truncated: bool,
}

/// Ask `remote` to remove `paths` (each relative to `module`), as `rm`
/// treats its operands under `options`. The daemon answers only an asker
/// of its own contract ([`super::same_build`]).
pub async fn remove(
    remote: &RemoteEndpoint,
    module: String,
    paths: Vec<String>,
    options: RmOptions,
) -> Result<RmReport> {
    let uri = remote.control_plane_uri();
    let mut client = crate::client::connect_with_timeout(uri.clone()).await?;
    let reply = client
        .rm(RmRequest {
            contract_version: crate::transfer_session::CONTRACT_VERSION,
            module,
            paths,
            recursive: options.recursive,
            force: options.force,
        })
        .await
        .map_err(|status| super::same_build::refusal("rm", status))?
        .into_inner();
    Ok(RmReport {
        failures: reply
            .failures
            .into_iter()
            .map(|failure| RmFailure {
                operand: failure.operand as usize,
                path: EntryPath {
                    text: failure.relative_path,
                    raw: failure.raw_relative_path,
                },
                reason: failure.reason,
            })
            .collect(),
        entries_removed: reply.entries_removed,
        failures_truncated: reply.failures_truncated,
    })
}

/// Remove one path and everything under it, for `blit move`'s source-side
/// removal step: asked for explicitly as a recursive removal (AC-11 — the
/// old request removed a tree without being told to), and a source that
/// is already gone is no failure. `Ok` with the count removed ONLY when
/// the daemon reports no entry left behind; otherwise the error names the
/// first one, and the source is not reported removed.
pub async fn delete_remote_path(remote: &RemoteEndpoint, rel_path: &str) -> Result<u64> {
    let (module, _) = extract_module_and_path(remote)?;
    let report = remove(
        remote,
        module,
        vec![rel_path.to_string()],
        RmOptions {
            recursive: true,
            force: true,
        },
    )
    .await?;
    if let Some(first) = report.failures.first() {
        let below = String::from_utf8_lossy(first.path.bytes());
        let entry = if first.path.is_start() {
            rel_path.to_string()
        } else {
            format!("{}/{below}", rel_path.trim_end_matches('/'))
        };
        let more = match report.failures.len() - 1 {
            0 if !report.failures_truncated => String::new(),
            0 => " (and more)".to_string(),
            n => format!(" (and {n} more)"),
        };
        bail!(
            "cannot remove '{entry}': {}{more}; {} entries were removed",
            first.reason,
            report.entries_removed
        );
    }
    Ok(report.entries_removed)
}

/// Pull the (module, rel_path) pair out of a `RemoteEndpoint`,
/// rejecting bare-host (`Discovery`) variants with an rm-specific
/// error message. Pub so the CLI's interactive `rm` handler can
/// share the same validation.
pub fn extract_module_and_path(remote: &RemoteEndpoint) -> Result<(String, PathBuf)> {
    match &remote.path {
        RemotePath::Module { module, rel_path } => Ok((module.clone(), rel_path.clone())),
        RemotePath::Root { rel_path } => Ok((String::new(), rel_path.clone())),
        RemotePath::Discovery => {
            bail!("remote removal requires module syntax (e.g., server:/module/path)")
        }
    }
}
