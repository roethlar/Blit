//! Transfer-shape dispatch primitives.
//!
//! Both the CLI's `run_transfer` and the future TUI's transfer
//! launcher need to answer the same question: "given a parsed
//! source endpoint, a parsed destination endpoint, and the
//! copy/mirror mode, which transport path do we take?" This
//! module owns the answer.
//!
//! The verb-entry functions (`run_transfer`, `run_move`) stay
//! in `blit-cli`; their bodies are dominated by CLI-shaped
//! error messages (specific flag names, recovery commands like
//! `blit rm`) and interactive prompts. The TUI will write its
//! own entry-points that consume [`TransferRoute`] and the
//! per-transport execution functions in
//! [`crate::transfers::local`] and
//! [`crate::transfers::remote`].

use crate::endpoints::Endpoint;
use blit_core::remote::RemoteEndpoint;
use std::path::PathBuf;

/// Copy vs mirror — the user-facing verb-tag the dispatcher
/// receives. Mirror means the destination's extraneous files
/// are removed after a successful transfer; copy leaves the
/// destination's surplus files alone.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum TransferKind {
    Copy,
    Mirror,
}

impl TransferKind {
    /// True iff the mode prunes destination-only entries on
    /// successful transfer. Used by both the local and remote
    /// dispatch arms to switch the per-transport function into
    /// "mirror" mode.
    pub fn is_mirror(self) -> bool {
        matches!(self, TransferKind::Mirror)
    }
}

/// Resolved transport choice for a transfer request. The
/// dispatcher returns one of these by inspecting the
/// `(source, destination)` endpoint pair.
///
/// Each variant carries the data the matching execution
/// function needs:
///
/// - [`TransferRoute::LocalToLocal`] → owned source + destination
///   paths.
/// - [`TransferRoute::LocalToRemote`] → owned source path +
///   parsed remote destination.
/// - [`TransferRoute::RemoteToLocal`] → parsed remote source +
///   owned destination path.
/// - [`TransferRoute::RemoteToRemoteDelegated`] → both
///   endpoints, used by the daemon-to-daemon
///   [`crate::transfers::remote::run_delegated_pull`] path.
///   Remote→remote is delegated-only: the CLI is never in the
///   byte path (D-2026-07-11-1 removed `--relay-via-cli`).
///
/// The `mirror` flag is reproduced on every variant so the
/// caller doesn't need a parallel `mirror_mode` parameter; the
/// route + the flag fully specify the transfer.
#[derive(Debug, Clone)]
pub enum TransferRoute {
    LocalToLocal {
        src: PathBuf,
        dst: PathBuf,
        mirror: bool,
    },
    LocalToRemote {
        src: PathBuf,
        dst: RemoteEndpoint,
        mirror: bool,
    },
    RemoteToLocal {
        src: RemoteEndpoint,
        dst: PathBuf,
        mirror: bool,
    },
    RemoteToRemoteDelegated {
        src: RemoteEndpoint,
        dst: RemoteEndpoint,
        mirror: bool,
    },
}

/// Pure function: pick the [`TransferRoute`] for the given
/// endpoint pair and verb mode.
///
/// No I/O, no presentation, no error cases — the dispatch is
/// total over `(Endpoint, Endpoint)`. CLI-side gates
/// (`--null`, source-exists check, mirror confirmation,
/// support gates) run before this function so they can produce
/// CLI-shaped error messages with appropriate recovery
/// guidance.
pub fn select_transfer_route(src: Endpoint, dst: Endpoint, kind: TransferKind) -> TransferRoute {
    let mirror = kind.is_mirror();
    match (src, dst) {
        (Endpoint::Local(src), Endpoint::Local(dst)) => {
            TransferRoute::LocalToLocal { src, dst, mirror }
        }
        (Endpoint::Local(src), Endpoint::Remote(dst)) => {
            TransferRoute::LocalToRemote { src, dst, mirror }
        }
        (Endpoint::Remote(src), Endpoint::Local(dst)) => {
            TransferRoute::RemoteToLocal { src, dst, mirror }
        }
        (Endpoint::Remote(src), Endpoint::Remote(dst)) => {
            TransferRoute::RemoteToRemoteDelegated { src, dst, mirror }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use blit_core::remote::RemotePath;
    use std::path::PathBuf;

    fn local(p: &str) -> Endpoint {
        Endpoint::Local(PathBuf::from(p))
    }

    fn remote(host: &str) -> Endpoint {
        Endpoint::Remote(RemoteEndpoint {
            host: host.to_string(),
            port: 9031,
            path: RemotePath::Module {
                module: "m".to_string(),
                rel_path: PathBuf::new(),
            },
        })
    }

    #[test]
    fn local_to_local_copy_routes_to_local_to_local_non_mirror() {
        let route = select_transfer_route(local("a"), local("b"), TransferKind::Copy);
        assert!(matches!(
            route,
            TransferRoute::LocalToLocal { mirror: false, .. }
        ));
    }

    #[test]
    fn local_to_local_mirror_carries_mirror_true() {
        let route = select_transfer_route(local("a"), local("b"), TransferKind::Mirror);
        assert!(matches!(
            route,
            TransferRoute::LocalToLocal { mirror: true, .. }
        ));
    }

    #[test]
    fn local_to_remote_routes_to_push() {
        let route = select_transfer_route(local("a"), remote("h"), TransferKind::Copy);
        assert!(matches!(route, TransferRoute::LocalToRemote { .. }));
    }

    #[test]
    fn remote_to_local_routes_to_pull() {
        let route = select_transfer_route(remote("h"), local("a"), TransferKind::Copy);
        assert!(matches!(route, TransferRoute::RemoteToLocal { .. }));
    }

    #[test]
    fn remote_to_remote_picks_delegated() {
        let route = select_transfer_route(remote("a"), remote("b"), TransferKind::Copy);
        assert!(matches!(
            route,
            TransferRoute::RemoteToRemoteDelegated { .. }
        ));
    }
}
