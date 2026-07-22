use crate::runtime::{ModuleConfig, RootExport};
use std::collections::HashMap;
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
                canonical_root: root.canonical_root.clone(),
                read_only: root.read_only,
                _comment: None,
                // Synthesized "default" module follows the daemon-wide
                // delegation policy without further narrowing.
                delegation_allowed: true,
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

/// Validate a wire-supplied directory-or-request relative path and
/// return its normalized form. Folds both empty and `"."` / `"./"`
/// inputs to `PathBuf::from(".")` (module root) — request paths for
/// list/find/du legitimately mean "the root" with these forms.
///
/// File-path contexts (per-file manifest entries, receive sink
/// targets) must NOT fold — the session's receive path validates
/// those itself, preserving empty as "the root is itself the file"
/// (R1-F3 of `docs/reviews/followup_review_2026-05-02.md` on why
/// conflating the two contexts leads to bugs).
#[allow(clippy::result_large_err)]
pub(crate) fn resolve_relative_path(rel: &str) -> Result<PathBuf, Status> {
    // Request-path-context fold: directory references (".", "./", "")
    // all map to "module root." Done before strict file-path validation
    // so the strict validator can keep rejecting these forms in
    // file-path contexts.
    let trimmed = rel.trim_end_matches('/');
    if trimmed.is_empty() || trimmed == "." {
        return Ok(PathBuf::from("."));
    }
    let normalized = blit_core::path_safety::validate_wire_path(rel)
        .map_err(|e| Status::invalid_argument(format!("path not allowed: {}: {e}", rel)))?;
    if normalized.as_os_str().is_empty() {
        Ok(PathBuf::from("."))
    } else {
        Ok(normalized)
    }
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

/// Join `rel` under `module.path` and verify the resolved location
/// stays inside the module's canonical root after symlink expansion.
/// Closes F2 of `docs/reviews/codebase_review_2026-05-01.md`.
///
/// Containment is checked against `module.canonical_root`, not
/// `module.path`. The push handler may rewrite `module.path` to
/// include a destination subpath (rsync "copy into here" semantics),
/// so `module.path` isn't reliably the canonical root anymore.
/// `module.canonical_root` is set once at runtime load time and
/// never mutated.
#[allow(clippy::result_large_err)]
pub(crate) fn resolve_contained_path(module: &ModuleConfig, rel: &Path) -> Result<PathBuf, Status> {
    let target = resolve_dest_path(&module.path, rel);
    blit_core::path_safety::verify_contained(&module.canonical_root, &target)
        .map_err(|e| Status::permission_denied(format!("path containment: {e:#}")))?;
    Ok(target)
}

/// Same as `resolve_contained_path` but takes a wire string and
/// runs the lexical `validate_wire_path` check too. Reserved for
/// future call sites; currently every daemon entry point either
/// goes through `resolve_relative_path` first (and ends up using
/// `resolve_contained_path` with a `&Path`) or constructs the path
/// directly from a manifest entry.
#[allow(dead_code)]
#[allow(clippy::result_large_err)]
pub(crate) fn resolve_contained_wire(module: &ModuleConfig, wire: &str) -> Result<PathBuf, Status> {
    blit_core::path_safety::contained_join(&module.canonical_root, wire)
        .map_err(|e| Status::permission_denied(format!("path containment: {e:#}")))
}
