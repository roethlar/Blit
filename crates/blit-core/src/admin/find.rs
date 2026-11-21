//! `find` — a walk below a path on a daemon with the Unix command's tests,
//! streamed as it is found (`docs/plan/UTILITY_VERBS_LOCAL_PARITY.md`
//! slice 3).
//!
//! The daemon walks its own disk ([`crate::find`]) and sends the paths
//! found in batches, in the walk's order, with each entry it could not
//! read. This module speaks gRPC; a front end parses the expression and
//! prints.

use crate::admin::du::EntryPath;
use crate::generated::{
    find_paths_reply, FindAge, FindCompare, FindKind, FindName, FindPathsRequest, FindSize,
};
use crate::remote::RemoteEndpoint;
use eyre::Result;

pub use crate::find::{AgeTest, Compare, Kind, SizeTest};

/// One name test: `-name`, or `-iname` when `ignore_case`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NameTest {
    /// The pattern as typed, by its bytes.
    pub pattern: Vec<u8>,
    pub ignore_case: bool,
}

/// What `find` is asked: where to start, and the tests — every one of
/// which must hold.
#[derive(Debug, Clone, Default)]
pub struct FindQuery {
    pub module: String,
    /// The path below the module, as typed; empty is the module itself.
    pub start_path: super::du::EntryPath,
    /// The start was typed with a separator after it.
    pub follow_start: bool,
    pub names: Vec<NameTest>,
    pub kinds: Vec<Kind>,
    /// `-maxdepth N`; `None` is every level.
    pub max_depth: Option<u32>,
    /// `-size`, every one of which must hold (slice 4).
    pub sizes: Vec<SizeTest>,
    /// `-mtime`, every one of which must hold.
    pub ages: Vec<AgeTest>,
    /// `-delete`: what is found is removed, and nothing is listed.
    pub delete: bool,
}

/// One thing the daemon reported, in order.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FindEvent {
    /// An entry every test holds for.
    Found(EntryPath),
    /// An entry the daemon could not read. Its walk went on; the command
    /// that asked exits 1, as `find` does.
    Failure {
        path: EntryPath,
        /// The operating system's words: `Permission denied`.
        reason: String,
        /// The entry was found and could not be DELETED (`-delete`);
        /// otherwise it could not be read.
        deleting: bool,
    },
}

fn compare(compare: Compare) -> i32 {
    (match compare {
        Compare::Less => FindCompare::Less,
        Compare::Exactly => FindCompare::Exactly,
        Compare::More => FindCompare::More,
    }) as i32
}

/// Run `find` on `remote`'s daemon, handing each entry found and each
/// failure to `on_event` as it arrives. The daemon answers only an asker
/// of its own contract ([`super::same_build`]).
pub async fn stream<F>(remote: &RemoteEndpoint, query: FindQuery, mut on_event: F) -> Result<()>
where
    F: FnMut(FindEvent) -> Result<()>,
{
    let uri = remote.control_plane_uri();
    let mut client = crate::client::connect_with_timeout(uri.clone()).await?;

    let mut stream = client
        .find_paths(FindPathsRequest {
            contract_version: crate::transfer_session::CONTRACT_VERSION,
            module: query.module,
            start_path: query.start_path.text,
            raw_start_path: query.start_path.raw,
            follow_start: query.follow_start,
            names: query
                .names
                .into_iter()
                .map(|name| FindName {
                    pattern: name.pattern,
                    ignore_case: name.ignore_case,
                })
                .collect(),
            kinds: query
                .kinds
                .into_iter()
                .map(|kind| {
                    (match kind {
                        Kind::File => FindKind::File,
                        Kind::Directory => FindKind::Directory,
                        Kind::Link => FindKind::Link,
                    }) as i32
                })
                .collect(),
            max_depth: query.max_depth,
            sizes: query
                .sizes
                .into_iter()
                .map(|size| FindSize {
                    compare: compare(size.compare),
                    count: size.count,
                    unit_bytes: size.unit_bytes,
                })
                .collect(),
            mtimes: query
                .ages
                .into_iter()
                .map(|age| FindAge {
                    compare: compare(age.compare),
                    days: age.days,
                })
                .collect(),
            delete: query.delete,
        })
        .await
        .map_err(|status| super::same_build::refusal("find", status))?
        .into_inner();

    while let Some(reply) = stream
        .message()
        .await
        .map_err(|status| super::same_build::refusal("find", status))?
    {
        match reply.reply {
            Some(find_paths_reply::Reply::Found(batch)) => {
                for found in batch.paths {
                    on_event(FindEvent::Found(EntryPath {
                        text: found.relative_path,
                        raw: found.raw_relative_path,
                    }))?;
                }
            }
            Some(find_paths_reply::Reply::Failure(failure)) => on_event(FindEvent::Failure {
                path: EntryPath {
                    text: failure.relative_path,
                    raw: failure.raw_relative_path,
                },
                reason: failure.reason,
                deleting: failure.deleting,
            })?,
            // A reply of a kind this build does not know: same contract,
            // so there is none; nothing to print.
            None => {}
        }
    }

    Ok(())
}
