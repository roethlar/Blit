use crate::runtime::{ModuleConfig, RootExport};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::sync::Mutex;
use tonic::Status;

pub(crate) async fn resolve_module(
    modules: &Arc<Mutex<HashMap<String, ModuleConfig>>>,
    default_root: Option<&RootExport>,
    name: &str,
) -> Result<ModuleConfig, Status> {
    if name.trim().is_empty() {
        if let Some(root) = default_root {
            return Ok(ModuleConfig {
                name: "default".to_string(),
                path: root.path.clone(),
                read_only: root.read_only,
                _comment: None,
                _use_chroot: root.use_chroot,
            });
        } else {
            return Err(Status::not_found(
                "default root is not configured on the remote daemon",
            ));
        }
    }

    let guard = modules.lock().await;
    guard
        .get(name)
        .cloned()
        .ok_or_else(|| Status::not_found(format!("module '{}' not found", name)))
}

/// Validate a wire-supplied relative path and return its normalized
/// form. Folds an empty input to `.` (legacy behavior — see
/// `resolve_manifest_relative_path` for the empty-preserving variant).
///
/// Thin wrapper over `blit_core::path_safety::validate_wire_path` —
/// the actual policy (reject `..`, absolute paths, Windows drive
/// prefixes, UNC, NUL, etc.) lives in the shared module.
#[allow(clippy::result_large_err)]
pub(crate) fn resolve_relative_path(rel: &str) -> Result<PathBuf, Status> {
    let normalized = blit_core::path_safety::validate_wire_path(rel)
        .map_err(|e| Status::invalid_argument(format!("path not allowed: {}: {e}", rel)))?;
    if normalized.as_os_str().is_empty() {
        Ok(PathBuf::from("."))
    } else {
        Ok(normalized)
    }
}

/// Same validation as `resolve_relative_path` but preserves an empty
/// input as `PathBuf::new()` instead of folding it to ".".
///
/// Used for per-file manifest entries during push: a single-file source
/// legitimately emits `relative_path = ""` to mean "the root is itself
/// the file". Folding to "." here breaks that — `module.path/file.txt`
/// vs. `module.path/file.txt/.` are not the same thing when the caller
/// opens the path (File::create on `.../file.txt/.` fails ENOTDIR).
#[allow(clippy::result_large_err)]
pub(crate) fn resolve_manifest_relative_path(rel: &str) -> Result<PathBuf, Status> {
    blit_core::path_safety::validate_wire_path(rel)
        .map_err(|e| Status::invalid_argument(format!("path not allowed: {}: {e}", rel)))
}

/// Resolve a destination file path as `base.join(rel)`, but preserving
/// `base` verbatim when `rel` is empty. `PathBuf::join("")` appends a
/// trailing separator on Unix (e.g. `/a/b` + `""` → `/a/b/`), which
/// `File::create` then rejects with `ENOTDIR` when `base` is itself
/// the intended file path (single-file push flow).
pub(crate) fn resolve_dest_path(base: &Path, rel: &Path) -> PathBuf {
    if rel.as_os_str().is_empty() {
        base.to_path_buf()
    } else {
        base.join(rel)
    }
}

pub(crate) fn metadata_mtime_seconds(meta: &fs::Metadata) -> Option<i64> {
    use std::time::UNIX_EPOCH;

    let modified = meta.modified().ok()?;
    match modified.duration_since(UNIX_EPOCH) {
        Ok(duration) => Some(duration.as_secs() as i64),
        Err(err) => {
            let dur = err.duration();
            Some(-(dur.as_secs() as i64))
        }
    }
}

pub(crate) fn permissions_mode(meta: &fs::Metadata) -> u32 {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        meta.permissions().mode()
    }
    #[cfg(not(unix))]
    {
        let _ = meta;
        0
    }
}

pub(crate) fn normalize_relative_path(path: &Path) -> String {
    let raw = path.to_string_lossy();
    #[cfg(windows)]
    {
        raw.replace('\\', "/")
    }
    #[cfg(not(windows))]
    {
        raw.into_owned()
    }
}

pub(crate) fn pathbuf_to_display(path: &Path) -> String {
    if path == Path::new(".") {
        return ".".to_string();
    }
    path.components()
        .map(|comp| comp.as_os_str().to_string_lossy())
        .collect::<Vec<_>>()
        .join("/")
}
