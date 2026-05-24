//! audit-7d7: pure F3-delete request-building helpers extracted from
//! `main.rs` (behavior-preserving — verbatim move, no logic change). These
//! classify and assemble the module-relative `Purge` wire request from
//! resolved cursor/marked endpoints. All pure (no async, no AppState); the
//! dispatcher and the spawn tasks call them.

use blit_core::remote::endpoint::RemoteEndpoint;

/// d-45 R2: the module-relative Purge wire path for a cursor
/// endpoint — forward-slash joined regardless of client OS.
/// Thin wrapper over `rel_path_to_string` so the conversion
/// boundary is named + unit-testable.
pub(crate) fn del_wire_path(rel_path: &std::path::Path) -> String {
    blit_app::endpoints::rel_path_to_string(rel_path)
}

/// d-50: assemble a delete request from resolved cursor/marked
/// endpoints. Filters out non-deletable targets (module roots),
/// converts each to a canonical wire rel-path, and returns
/// `(module_endpoint, rel_paths, label, gate_path)` or `None`
/// when nothing is deletable.
///
/// - `batch` (a multi-select was active) → label "N item(s)",
///   `gate_path = None` (outcome shows until the next action).
/// - single cursor row → label is the path spec, `gate_path =
///   Some(spec)` (outcome hides once the cursor leaves it, the
///   d-45 behavior).
///
/// All targets share one module (they come from one F3 view), so
/// the first endpoint carries the module for the single `Purge`.
pub(crate) fn build_delete_request(
    endpoints: Vec<RemoteEndpoint>,
    batch: bool,
) -> Option<(RemoteEndpoint, Vec<String>, String, Option<String>)> {
    use blit_app::admin::rm;
    let deletable: Vec<RemoteEndpoint> = endpoints
        .into_iter()
        .filter(is_deletable_remote_path)
        .collect();
    let module_endpoint = deletable.first()?.clone();
    let mut rel_paths = Vec::with_capacity(deletable.len());
    for ep in &deletable {
        if let Ok((_module, rel_path)) = rm::extract_module_and_path(ep) {
            rel_paths.push(del_wire_path(&rel_path));
        }
    }
    if rel_paths.is_empty() {
        return None;
    }
    let (label, gate_path) = if batch {
        (format!("{} item(s)", rel_paths.len()), None)
    } else {
        let spec = module_endpoint.display();
        (spec.clone(), Some(spec))
    };
    Some((module_endpoint, rel_paths, label, gate_path))
}

/// d-45: may this cursor endpoint be deleted from the TUI?
///
/// Refuses a module root or empty rel-path — you can't nuke a
/// whole module via `D` (mirrors `blit rm`'s guard). Also refuses
/// `Discovery` (bare-host) endpoints, which carry no path.
/// Pure — the dispatcher gates the confirm prompt on this.
pub(crate) fn is_deletable_remote_path(endpoint: &RemoteEndpoint) -> bool {
    use blit_app::admin::rm;
    match rm::extract_module_and_path(endpoint) {
        Ok((_module, rel_path)) => {
            let rel = rel_path.to_string_lossy();
            !rel.is_empty() && rel != "."
        }
        Err(_) => false,
    }
}
