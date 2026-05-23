//! `ls` — list directory entries (local filesystem or remote
//! daemon via `List` RPC).
//!
//! Moved from `crates/blit-cli/src/ls.rs` in A.0. Smart-dispatch
//! (bare-host targets → `list-modules`) stays in the CLI because
//! it's a verb-routing decision; the same decision is made
//! UI-side by the TUI's F1/F3 split.

use blit_core::generated::ListRequest;
use blit_core::remote::endpoint::RemoteEndpoint;
use eyre::{Context, Result};
use serde::Serialize;
use std::fs;
use std::path::Path;
use std::time::UNIX_EPOCH;

/// One row of a directory listing. Same shape for local and
/// remote modes so the CLI's formatter doesn't care which side
/// produced it. JSON field names match the pre-A.0 `DirEntryJson`
/// shape exactly (A.0 is no-behavior-change).
#[derive(Debug, Clone, Serialize)]
pub struct DirEntry {
    pub name: String,
    pub is_dir: bool,
    pub size: u64,
    pub mtime_seconds: i64,
}

/// Outcome of `list_local`. Discriminates "target is a directory,
/// here are its contents" from "target is a single non-directory
/// (file, device, FIFO, socket, symlink); here is its stat
/// summary." Pre-A.0 the CLI used `!metadata.is_dir()` to decide
/// the second branch; doing the same in the CLI by re-checking
/// `Path::is_file()` would silently misclassify devices/FIFOs/
/// sockets as directories. Returning the discriminator lifts the
/// decision out of presentation.
#[derive(Debug, Clone)]
pub enum LocalListing {
    /// Target was a directory. Entries are sorted by path to
    /// match the pre-A.0 text layout.
    Directory { entries: Vec<DirEntry> },
    /// Target was a single non-directory. Single-entry summary,
    /// basename in `entry.name`. Pre-A.0 the CLI's text path
    /// rendered this as `FILE {size} {full path}`; that
    /// rendering is the CLI's responsibility.
    Target { entry: DirEntry },
}

impl LocalListing {
    /// Flatten into a single entry vec. Used for the CLI's `--json`
    /// output which has always emitted "one entry per target"
    /// regardless of whether the target was a directory.
    pub fn into_entries(self) -> Vec<DirEntry> {
        match self {
            LocalListing::Directory { entries } => entries,
            LocalListing::Target { entry } => vec![entry],
        }
    }
}

/// List entries under a remote `module:path`. Caller has already
/// resolved the smart-dispatch (bare host → list-modules) on its
/// own — this function assumes a real module/path target.
pub async fn list_remote(
    remote: &RemoteEndpoint,
    module: String,
    path: String,
) -> Result<Vec<DirEntry>> {
    let uri = remote.control_plane_uri();
    let mut client = crate::client::connect_with_timeout(uri.clone()).await?;
    let response = client
        .list(ListRequest { module, path })
        .await
        .map_err(|status| eyre::eyre!(status.message().to_string()))?
        .into_inner();

    Ok(response
        .entries
        .into_iter()
        .map(|info| DirEntry {
            name: info.name,
            is_dir: info.is_dir,
            size: info.size,
            mtime_seconds: info.mtime_seconds,
        })
        .collect())
}

/// Stat-or-list a local path. Mirrors pre-A.0 branching: a
/// directory expands into its entries (sorted by path); any
/// non-directory (regular file, device, FIFO, socket, symlink
/// to a non-directory) returns a single-entry `Target`.
/// Recursion is the caller's responsibility.
pub fn list_local(path: &Path) -> Result<LocalListing> {
    let metadata =
        fs::metadata(path).with_context(|| format!("reading metadata for {}", path.display()))?;

    if metadata.is_dir() {
        let mut entries: Vec<DirEntry> = Vec::new();
        let mut paths: Vec<_> = fs::read_dir(path)
            .with_context(|| format!("reading directory {}", path.display()))?
            .collect::<std::result::Result<Vec<_>, _>>()
            .with_context(|| format!("collecting entries for {}", path.display()))?;
        paths.sort_by_key(|e| e.path());
        for entry in paths {
            let meta = entry
                .metadata()
                .with_context(|| format!("reading metadata for {}", entry.path().display()))?;
            let is_dir = meta.is_dir();
            let size = if is_dir { 0 } else { meta.len() };
            entries.push(DirEntry {
                name: entry.file_name().to_string_lossy().into_owned(),
                is_dir,
                size,
                mtime_seconds: metadata_mtime_seconds(&meta).unwrap_or(0),
            });
        }
        Ok(LocalListing::Directory { entries })
    } else {
        let name = path
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_else(|| ".".to_string());
        Ok(LocalListing::Target {
            entry: DirEntry {
                name,
                is_dir: false,
                size: metadata.len(),
                mtime_seconds: metadata_mtime_seconds(&metadata).unwrap_or(0),
            },
        })
    }
}

/// Local helper duplicating `blit_cli::util::metadata_mtime_seconds`
/// so the CLI's util.rs split (paused per reviewer) doesn't block
/// this move. Consolidated when util.rs lands in blit-app.
fn metadata_mtime_seconds(meta: &fs::Metadata) -> Option<i64> {
    let modified = meta.modified().ok()?;
    match modified.duration_since(UNIX_EPOCH) {
        Ok(duration) => Some(duration.as_secs() as i64),
        Err(err) => {
            let dur = err.duration();
            Some(-(dur.as_secs() as i64))
        }
    }
}
