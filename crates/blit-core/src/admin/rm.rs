//! `rm` — remove paths on a remote daemon, as the Unix command does
//! (`docs/plan/UTILITY_VERBS_LOCAL_PARITY.md` slice 2).
//!
//! One request per daemon and module carries every operand; the daemon —
//! the side that knows what each path is — answers with each entry it
//! could not remove, as it meets it, and says when it is done. The
//! question before deleting, the messages and the exit status are the
//! front end's. The CLI's `move` verb uses
//! [`delete_remote_path`] for its source-side removal step.

use crate::admin::du::EntryPath;
use crate::generated::{rm_reply, RmRequest};
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

/// Why a removal has no complete answer.
#[derive(Debug)]
pub enum RmError {
    /// The daemon could not be asked, or refused the request, before
    /// anything was read or removed (a read-only module, another build):
    /// none of the operands went.
    Refused(eyre::Report),
    /// The answer ended before the daemon said it was done. What it had
    /// reported until then stands; what else was removed is not known.
    Interrupted(eyre::Report),
}

impl std::fmt::Display for RmError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Refused(error) => write!(formatter, "{error:#}"),
            Self::Interrupted(error) => {
                write!(formatter, "the removal did not finish: {error:#}")
            }
        }
    }
}

impl std::error::Error for RmError {}

/// Ask `remote` to remove `paths` (each below `module`, exactly as typed),
/// as `rm` treats its operands under `options`. `on_failure` is told of
/// each entry the daemon could not remove as the daemon reports it
/// (cr-uv1-5): every one, in the order met. `Ok` is the count of entries
/// removed, and is returned only when the daemon said it had finished.
/// The daemon answers only an asker of its own contract
/// ([`super::same_build`]).
pub async fn remove<F>(
    remote: &RemoteEndpoint,
    module: String,
    paths: Vec<EntryPath>,
    options: RmOptions,
    mut on_failure: F,
) -> std::result::Result<u64, RmError>
where
    F: FnMut(RmFailure),
{
    let uri = remote.control_plane_uri();
    let mut client = crate::client::connect_with_timeout(uri.clone())
        .await
        .map_err(RmError::Refused)?;
    let mut answer = client
        .rm(RmRequest {
            contract_version: crate::transfer_session::CONTRACT_VERSION,
            module,
            // Beside the text, the bytes of each path that is not text
            // (cr-uv1-4) — or none at all when every one is.
            raw_paths: if paths.iter().any(|path| path.raw.is_some()) {
                paths
                    .iter()
                    .map(|path| path.raw.clone().unwrap_or_default())
                    .collect()
            } else {
                Vec::new()
            },
            paths: paths.into_iter().map(|path| path.text).collect(),
            recursive: options.recursive,
            force: options.force,
        })
        .await
        .map_err(|status| RmError::Refused(super::same_build::refusal("rm", status)))?
        .into_inner();
    loop {
        let reply = answer
            .message()
            .await
            .map_err(|status| RmError::Interrupted(eyre::eyre!("{}", status.message())))?;
        match reply.and_then(|reply| reply.reply) {
            Some(rm_reply::Reply::Failures(batch)) => {
                for failure in batch.failures {
                    on_failure(RmFailure {
                        operand: failure.operand as usize,
                        path: EntryPath {
                            text: failure.relative_path,
                            raw: failure.raw_relative_path,
                        },
                        reason: failure.reason,
                    });
                }
            }
            Some(rm_reply::Reply::Done(done)) => return Ok(done.entries_removed),
            None => {
                return Err(RmError::Interrupted(eyre::eyre!(
                    "the daemon's answer ended before it said it was done"
                )))
            }
        }
    }
}

/// Remove one path and everything under it, for `blit move`'s source-side
/// removal step: asked for explicitly as a recursive removal (AC-11 — the
/// old request removed a tree without being told to), and a source that
/// is already gone is no failure. `Ok` with the count removed ONLY when
/// the daemon reports no entry left behind; otherwise the error names the
/// first one, and the source is not reported removed.
pub async fn delete_remote_path(remote: &RemoteEndpoint, rel_path: &str) -> Result<u64> {
    let (module, _) = extract_module_and_path(remote)?;
    // The first entry left behind, and how many more: a move names one.
    let (mut first, mut more): (Option<RmFailure>, u64) = (None, 0);
    let removed = remove(
        remote,
        module,
        vec![EntryPath::typed(rel_path.as_bytes())],
        RmOptions {
            recursive: true,
            force: true,
        },
        |failure| match first {
            None => first = Some(failure),
            Some(_) => more += 1,
        },
    )
    .await;
    if let Some(first) = first {
        let below = String::from_utf8_lossy(first.path.bytes());
        let entry = if first.path.is_start() {
            rel_path.to_string()
        } else {
            format!("{}/{below}", rel_path.trim_end_matches('/'))
        };
        let more = match more {
            0 => String::new(),
            n => format!(" (and {n} more)"),
        };
        let removed = match removed {
            Ok(removed) => format!("{removed} entries were removed"),
            Err(error) => format!("{error}"),
        };
        bail!("cannot remove '{entry}': {}{more}; {removed}", first.reason);
    }
    Ok(removed?)
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
