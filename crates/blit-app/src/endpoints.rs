//! Endpoint parsing + remote-transfer support gates.
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
//! The three remote-transfer support gates
//! ([`ensure_remote_transfer_supported`],
//! [`ensure_remote_pull_supported`], and
//! [`ensure_remote_push_supported`]) take primitive booleans —
//! the CLI's `crates/blit-cli/src/transfers/endpoints.rs` keeps
//! two paper-thin wrappers that map `&TransferArgs` →
//! primitives, and the future TUI will call the library
//! functions directly. Error messages reference the CLI flag
//! names (`--dry-run`, `--workers`) because those are the
//! documented user surface; if the TUI ever surfaces the refusal
//! verbatim it can remap to its own labels at the catch point.
//!
//! Endpoint-shape gates ([`ensure_remote_destination_supported`],
//! [`ensure_remote_source_supported`]) reject
//! `RemotePath::Discovery` inputs on transfer paths — the
//! parser produces `Discovery` for bare-host shapes like
//! `host:` with no module / root, which the wire protocol
//! can't route.

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
            // `RemoteEndpoint::parse` bails "input appears to be a
            // local path" when `check_local_path` recognizes a local
            // path — including Windows drive paths (`C:/path`,
            // `C:\path`) whose `:/` would otherwise trip the
            // remote-shaped-typo guard below. Honor that lower-level
            // classification: it's local, not a typo'd remote.
            if err_msg.contains("appears to be a local path") {
                return Ok(Endpoint::Local(PathBuf::from(input)));
            }
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
            "remote destination must include a module or root (e.g., server:/module/ or server://path){}",
            local_path_hint(&remote.host)
        ),
    }
}

/// Source-side counterpart of [`ensure_remote_destination_supported`].
pub fn ensure_remote_source_supported(remote: &RemoteEndpoint) -> Result<()> {
    match remote.path {
        RemotePath::Module { .. } | RemotePath::Root { .. } => Ok(()),
        RemotePath::Discovery => bail!(
            "remote source must include a module or root (e.g., server:/module/ or server://path){}",
            local_path_hint(&remote.host)
        ),
    }
}

/// Owner decision 2026-07-12 (STATE.md, CLI foot-gun): a bare local
/// dir name with no `./` parses as a Discovery endpoint on purpose —
/// parsing is deliberately unchanged (no local-wins ambiguity). But
/// when the transfer gates refuse it AND a file or directory of that
/// exact name exists relative to `dir`, the refusal must point at the
/// `./` escape hatch instead of leaving a pure network-shaped error.
fn local_path_hint_in(dir: &Path, host: &str) -> Option<String> {
    let meta = std::fs::metadata(dir.join(host)).ok()?;
    let kind = if meta.is_dir() { "folder" } else { "file" };
    Some(format!(
        "; '{host}' exists here as a {kind} — did you mean ./{host}?"
    ))
}

/// CWD-anchored wrapper for the gates. Returns an empty string when
/// there is nothing to suggest so the gates can append unconditionally.
pub(crate) fn local_path_hint(host: &str) -> String {
    std::env::current_dir()
        .ok()
        .and_then(|dir| local_path_hint_in(&dir, host))
        .unwrap_or_default()
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
/// `--checksum`: the session's Checksum compare (contract v3,
/// otp-10b-1) is refused at OPEN with `CHECKSUM_DISABLED` when the
/// daemon runs `--no-server-checksums` — the successor to the old
/// pull's F11 ack negotiation (R15-F1 made that path reachable).
pub fn ensure_remote_pull_supported(dry_run: bool, workers_limit_set: bool) -> Result<()> {
    ensure_remote_transfer_supported(dry_run, workers_limit_set)
}

/// Gate for **local-source / remote-destination** pushes and
/// **remote-remote** relays. otp-10b-2 lifted the historical
/// `--checksum` rejection: the unified session's Checksum compare is
/// role-agnostic (whichever end holds SOURCE hashes its manifest, the
/// DESTINATION hashes its diff candidates), and a
/// `--no-server-checksums` daemon refuses at OPEN either direction —
/// push `--checksum` now behaves exactly like pull `--checksum`.
pub fn ensure_remote_push_supported(dry_run: bool, workers_limit_set: bool) -> Result<()> {
    ensure_remote_transfer_supported(dry_run, workers_limit_set)
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
/// module root"). Delegates to the single canonical helper in
/// `blit_core::path_posix` so the conversion is consistent everywhere.
pub fn rel_path_to_string(path: &Path) -> String {
    blit_core::path_posix::relative_path_to_posix(path)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// d-68 R4: a Windows drive path (`C:/...` or `C:\...`) is a
    /// local destination even though it contains `:/`. The classifier
    /// must honor `RemoteEndpoint::parse`'s lower-level "local path"
    /// verdict rather than treating the `:/` as a remote-shaped typo.
    #[test]
    fn windows_drive_paths_are_local() {
        for input in ["C:/tmp/out", "C:\\tmp\\out", "D:/data"] {
            match parse_transfer_endpoint(input) {
                Ok(Endpoint::Local(p)) => assert_eq!(p, PathBuf::from(input)),
                other => panic!("{input:?}: expected Local, got {other:?}"),
            }
        }
    }

    /// The remote-shaped-typo guard still rejects a module path that's
    /// missing its trailing slash — must NOT be swallowed as local.
    #[test]
    fn remote_shaped_typo_still_errors() {
        assert!(parse_transfer_endpoint("skippy:/backup").is_err());
    }

    /// A genuine remote module dest still parses as remote.
    #[test]
    fn module_dest_is_remote() {
        assert!(matches!(
            parse_transfer_endpoint("skippy:/backup/"),
            Ok(Endpoint::Remote(_))
        ));
    }

    /// Owner decision 2026-07-12: when the Discovery refusal fires and
    /// a directory of that name exists, the error suggests `./NAME`.
    #[test]
    fn discovery_hint_suggests_dot_slash_for_existing_dir() {
        let tmp = tempfile::tempdir().unwrap();
        std::fs::create_dir(tmp.path().join("myfolder")).unwrap();
        let hint = local_path_hint_in(tmp.path(), "myfolder").unwrap();
        assert!(hint.contains("'myfolder' exists here as a folder"), "{hint}");
        assert!(hint.contains("./myfolder"), "{hint}");
    }

    /// A plain file gets the same suggestion, worded as a file.
    #[test]
    fn discovery_hint_suggests_dot_slash_for_existing_file() {
        let tmp = tempfile::tempdir().unwrap();
        std::fs::write(tmp.path().join("notes.txt"), b"x").unwrap();
        let hint = local_path_hint_in(tmp.path(), "notes.txt").unwrap();
        assert!(hint.contains("'notes.txt' exists here as a file"), "{hint}");
        assert!(hint.contains("./notes.txt"), "{hint}");
    }

    /// No local path of that name -> no suggestion; the gate message
    /// stays exactly the network-shaped refusal.
    #[test]
    fn discovery_hint_absent_when_no_local_path() {
        let tmp = tempfile::tempdir().unwrap();
        assert!(local_path_hint_in(tmp.path(), "no-such-host").is_none());
    }
}
