//! Endpoint parsing for transfer sources / destinations.
//!
//! Moved from `crates/blit-cli/src/transfers/endpoints.rs` and
//! `crates/blit-cli/src/util.rs` as part of the Phase 5 A.0
//! extraction. The two pre-A.0 `Endpoint` enums (strict in
//! `transfers::endpoints`, loose in `util`) were structurally
//! identical; unified here behind two parsers that differ only
//! in their error stance — [`parse_transfer_endpoint`] is strict
//! (errors on remote-shaped input that fails to parse, errors on
//! forward-slash mishaps), [`parse_endpoint_or_local`] is loose
//! (falls back to `Local` for any input the strict parser
//! rejects).
//!
//! Two clap-coupled gate functions (`ensure_remote_pull_supported`
//! and `ensure_remote_push_supported`) stay in `blit-cli` for
//! now; they'll move once their inputs are reshaped to primitives
//! (final A.0 cleanup commit).

use blit_core::remote::{RemoteEndpoint, RemotePath};
use eyre::{bail, Result};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone)]
pub enum Endpoint {
    Local(PathBuf),
    Remote(RemoteEndpoint),
}

/// Parse a CLI / TUI source-or-destination input into an `Endpoint`.
/// Recognizes `host:/module/path` and `host://path` shapes as remote;
/// anything else is taken as a local filesystem path. Forward-slash
/// errors propagate so callers can show the user a clean diagnostic
/// instead of silently treating a misformatted remote as a local
/// path.
pub fn parse_transfer_endpoint(input: &str) -> Result<Endpoint> {
    match RemoteEndpoint::parse(input) {
        Ok(endpoint) => Ok(Endpoint::Remote(endpoint)),
        Err(err) => {
            let err_msg = err.to_string();
            if err_msg.contains("forward slashes") {
                return Err(err);
            }
            // Anything that looks like a remote URI (scheme or
            // `host:/path`) must parse as remote; treating a typo'd
            // remote as a local path silently was a long-standing
            // footgun.
            if input.contains("://") || input.contains(":/") {
                Err(err)
            } else {
                Ok(Endpoint::Local(PathBuf::from(input)))
            }
        }
    }
}

pub fn format_remote_endpoint(remote: &RemoteEndpoint) -> String {
    remote.display()
}

/// Reject a `RemoteEndpoint` whose `path` is `Discovery` (a bare
/// host without module / root). Used as the destination-side gate.
pub fn ensure_remote_destination_supported(remote: &RemoteEndpoint) -> Result<()> {
    match &remote.path {
        RemotePath::Module { .. } | RemotePath::Root { .. } => Ok(()),
        RemotePath::Discovery => bail!(
            "remote destination must include a module or root (e.g., server:/module/ or server://path)"
        ),
    }
}

/// Source-side counterpart of [`ensure_remote_destination_supported`].
pub fn ensure_remote_source_supported(remote: &RemoteEndpoint) -> Result<()> {
    match remote.path {
        RemotePath::Module { .. } | RemotePath::Root { .. } => Ok(()),
        RemotePath::Discovery => bail!(
            "remote source must include a module or root (e.g., server:/module/ or server://path)"
        ),
    }
}

/// Common transfer-flag gate shared by every remote-touching
/// path. Takes primitive booleans rather than `&TransferArgs`
/// so both the CLI and the future TUI can call it without a
/// clap dependency. CLI passes `args.dry_run` and
/// `args.workers.is_some()`.
///
/// Error messages reference the CLI flag names (`--dry-run`,
/// `--workers`) because those are the documented surface the
/// user knows; the TUI can map them to its own labels if it
/// surfaces the refusal verbatim.
pub fn ensure_remote_transfer_supported(dry_run: bool, workers_limit_set: bool) -> Result<()> {
    if dry_run {
        bail!("--dry-run is not supported for remote transfers");
    }
    if workers_limit_set {
        bail!("--workers limiter is not supported for remote transfers");
    }
    Ok(())
}

/// Gate for **remote-source / local-destination** pulls. Allows
/// `--checksum`: the pull-sync handshake negotiates checksum
/// support with the daemon and bails at the ack if the daemon has
/// `--no-server-checksums`. Closes R15-F1 of
/// `docs/reviews/followup_review_2026-05-02.md`: the previous
/// blanket `--checksum` rejection made the F11 ack-mismatch error
/// path unreachable from the CLI.
pub fn ensure_remote_pull_supported(dry_run: bool, workers_limit_set: bool) -> Result<()> {
    ensure_remote_transfer_supported(dry_run, workers_limit_set)
}

/// Gate for **local-source / remote-destination** pushes and
/// **remote-remote** relays. The push protocol has no per-transfer
/// capability negotiation yet, so `--checksum` is rejected here
/// rather than silently degrading. Symmetric pull-side support
/// arrived through the F11 ack negotiation; push needs its own
/// equivalent before this gate can lift.
pub fn ensure_remote_push_supported(
    dry_run: bool,
    workers_limit_set: bool,
    checksum: bool,
) -> Result<()> {
    ensure_remote_transfer_supported(dry_run, workers_limit_set)?;
    if checksum {
        bail!(
            "--checksum is not supported for remote-destination transfers \
             (push protocol has no checksum capability negotiation today)"
        );
    }
    Ok(())
}

/// Loose parser: returns `Endpoint::Remote` when the input parses
/// as a remote URI, falls back to `Endpoint::Local` for anything
/// else. Used by the admin verbs (`df`, `du`, `find`, `ls`,
/// `list-modules`, `rm`) where a malformed-looking input is
/// treated as a local path. For transfer commands prefer
/// [`parse_transfer_endpoint`] — the strict variant catches
/// remote-shaped typos rather than silently falling back.
pub fn parse_endpoint_or_local(input: &str) -> Endpoint {
    match RemoteEndpoint::parse(input) {
        Ok(endpoint) => Endpoint::Remote(endpoint),
        Err(_) => Endpoint::Local(PathBuf::from(input)),
    }
}

/// Pull the `(module, rel_path)` pair out of a `RemoteEndpoint`,
/// erroring with a generic message on `Discovery`. Different from
/// `admin::rm::extract_module_and_path` (rm-specific error wording);
/// kept separate so each verb can supply its own diagnostic.
pub fn module_and_rel_path(remote: &RemoteEndpoint) -> Result<(String, PathBuf)> {
    match &remote.path {
        RemotePath::Module { module, rel_path } => Ok((module.clone(), rel_path.clone())),
        RemotePath::Root { rel_path } => Ok((String::new(), rel_path.clone())),
        RemotePath::Discovery => {
            bail!("remote target must include a module path");
        }
    }
}

/// Render a relative `Path` as a forward-slashed string suitable
/// for the wire `path` / `start_path` fields. Empty or `.` paths
/// produce an empty string (the daemon-side convention for "the
/// module root"). Uses `to_string_lossy` for non-UTF8 components.
pub fn rel_path_to_string(path: &Path) -> String {
    if path.as_os_str().is_empty() || path == Path::new(".") {
        String::new()
    } else {
        path.components()
            .map(|c| c.as_os_str().to_string_lossy())
            .collect::<Vec<_>>()
            .join("/")
    }
}
