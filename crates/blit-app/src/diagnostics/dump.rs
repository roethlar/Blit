//! Diagnostics dump — endpoint introspection helpers.
//!
//! Moved from `crates/blit-cli/src/diagnostics.rs` (dump path)
//! in A.0. The CLI's `diagnostics dump` verb assembles the
//! top-level JSON snapshot (including rsync-resolution flags
//! sourced from `transfers/mod.rs`, which hasn't moved yet);
//! this module owns the per-endpoint snapshot builders, the
//! display-string helper, the same-device check, and the
//! disk-space lookup. All three are also useful to the TUI's
//! future F1 daemon-detail and F3 path-inspect affordances.

use crate::endpoints::{format_remote_endpoint, Endpoint};
use blit_core::fs_capability::cached_probe;
use blit_core::remote::RemotePath;
use serde_json::{json, Value};
use std::path::Path;

/// Build a JSON snapshot for a single endpoint. Local endpoints
/// get filesystem caps + free/total disk; remote endpoints get
/// the parsed host/port/module/path breakdown.
pub fn endpoint_snapshot(raw: &str, endpoint: &Endpoint) -> Value {
    match endpoint {
        Endpoint::Local(path) => local_path_snapshot(raw, path),
        Endpoint::Remote(remote) => {
            let (kind, module, rel_path) = match &remote.path {
                RemotePath::Module { module, rel_path } => (
                    "module",
                    Some(module.as_str().to_string()),
                    Some(rel_path.display().to_string()),
                ),
                RemotePath::Root { rel_path } => {
                    ("root", None, Some(rel_path.display().to_string()))
                }
                RemotePath::Discovery => ("discovery", None, None),
            };
            json!({
                "raw": raw,
                "kind": "remote",
                "host": remote.host.to_string(),
                "port": remote.port,
                "path_kind": kind,
                "module": module,
                "rel_path": rel_path,
                "display": format_remote_endpoint(remote),
            })
        }
    }
}

/// Render an endpoint as a single user-readable string. Local
/// endpoints use `Path::display()`; remote endpoints use
/// `endpoints::format_remote_endpoint`.
pub fn endpoint_display(endpoint: &Endpoint) -> String {
    match endpoint {
        Endpoint::Local(p) => p.display().to_string(),
        Endpoint::Remote(r) => format_remote_endpoint(r),
    }
}

/// Same-device check between two local endpoints. The biggest
/// single predictor of reflink eligibility and general zero-copy
/// viability on Linux. Remote endpoints (in either slot)
/// short-circuit to `Some(false)` — no shared-device semantics
/// across the wire. Returns `None` on non-Unix platforms where
/// `dev()` isn't available.
pub fn same_device(src: &Endpoint, dst: &Endpoint) -> Option<bool> {
    let (Endpoint::Local(s), Endpoint::Local(d)) = (src, dst) else {
        return Some(false);
    };

    #[cfg(unix)]
    {
        use std::os::unix::fs::MetadataExt;
        // Fall back to parent directory for dest because the
        // resolved path may not exist yet on a fresh target.
        let src_meta = std::fs::metadata(s).ok()?;
        let dst_meta = std::fs::metadata(d)
            .or_else(|_| {
                d.parent().map_or_else(
                    || Err(std::io::Error::from(std::io::ErrorKind::NotFound)),
                    std::fs::metadata,
                )
            })
            .ok()?;
        Some(src_meta.dev() == dst_meta.dev())
    }
    #[cfg(not(unix))]
    {
        let _ = (s, d);
        None
    }
}

// ── internal ─────────────────────────────────────────────────────

fn local_path_snapshot(raw: &str, path: &Path) -> Value {
    let abs_path = std::fs::canonicalize(path)
        .ok()
        .map(|p| p.display().to_string());
    let metadata = std::fs::metadata(path).ok();
    let exists = metadata.is_some();
    let is_file = metadata.as_ref().map(|m| m.is_file()).unwrap_or(false);
    let is_dir = metadata.as_ref().map(|m| m.is_dir()).unwrap_or(false);
    let size = metadata.as_ref().filter(|m| m.is_file()).map(|m| m.len());

    let caps = cached_probe(path);
    let (fs_type, reflink, block_clone) = caps
        .as_ref()
        .map(|c| {
            (
                c.filesystem_type.clone(),
                Some(c.reflink),
                Some(c.block_clone_same_volume),
            )
        })
        .unwrap_or((None, None, None));

    let (free_bytes, total_bytes) = disk_free_total(path);

    json!({
        "raw": raw,
        "kind": "local",
        "input_path": path.display().to_string(),
        "absolute_path": abs_path,
        "exists": exists,
        "is_file": is_file,
        "is_dir": is_dir,
        "size_bytes": size,
        "filesystem_type": fs_type,
        "reflink": reflink,
        "block_clone_same_volume": block_clone,
        "free_bytes": free_bytes,
        "total_bytes": total_bytes,
    })
}

/// Returns `(free_bytes, total_bytes)` for the disk containing
/// `path`, if we can match the path against one of sysinfo's
/// mount points. Returns `(None, None)` if no match — better
/// than a guess.
fn disk_free_total(path: &Path) -> (Option<u64>, Option<u64>) {
    // Walk up `path` until we find a prefix that matches a
    // mount_point. sysinfo's list of disks is not sorted, so
    // find the longest match.
    let disks = sysinfo::Disks::new_with_refreshed_list();
    let canonical = std::fs::canonicalize(path).unwrap_or_else(|_| path.to_path_buf());
    // Windows canonicalize returns extended-length paths
    // (\\?\C:\...) but sysinfo's mount_point is the bare drive
    // (C:\). Strip the prefix so starts_with matches.
    let canonical = strip_windows_extended_prefix(&canonical);
    let mut best: Option<(&sysinfo::Disk, usize)> = None;
    for disk in disks.iter() {
        let mp = disk.mount_point();
        if canonical.starts_with(mp) {
            let len = mp.as_os_str().len();
            if best.is_none_or(|(_, prev_len)| len > prev_len) {
                best = Some((disk, len));
            }
        }
    }
    match best {
        Some((disk, _)) => (Some(disk.available_space()), Some(disk.total_space())),
        None => (None, None),
    }
}

#[cfg(windows)]
fn strip_windows_extended_prefix(path: &Path) -> std::path::PathBuf {
    let s = path.to_string_lossy();
    if let Some(stripped) = s.strip_prefix(r"\\?\") {
        std::path::PathBuf::from(stripped)
    } else {
        path.to_path_buf()
    }
}

#[cfg(not(windows))]
fn strip_windows_extended_prefix(path: &Path) -> std::path::PathBuf {
    path.to_path_buf()
}
