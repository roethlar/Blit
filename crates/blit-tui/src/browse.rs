//! F3 Browse state — current view (module list or directory
//! contents) + cursor + path stack. Pure model; the F1
//! event loop owns the RPC fetcher tasks and `screens/f3`
//! owns the rendering.
//!
//! View model:
//!
//! - `BrowseView::Modules` — top level, listing the daemon's
//!   exported modules. Cursor-enter descends into the
//!   selected module's root.
//! - `BrowseView::Module { name, path }` — inside a module.
//!   `path` is the dir stack ("photos/2024" → ["photos",
//!   "2024"]). Empty path = module root.
//!
//! Navigation:
//!
//! - `enter` (or →): descend into the cursor's directory.
//!   No-op on a file entry today (transfer / detail panes
//!   are future slices).
//! - `esc` (or ←): pop the path. At module root, pops back
//!   to the module list. At the module list, no-op.

use blit_app::admin::list_modules::Module;
use blit_app::admin::ls::DirEntry;
use std::time::Instant;

/// Either the module list or the contents of a directory
/// within a module.
#[derive(Debug, Clone)]
pub enum BrowseView {
    /// Top-level module list. Entries are `Module` rows.
    Modules,
    /// Inside a module. `path` is the dir stack relative to
    /// the module root.
    Module { name: String, path: Vec<String> },
}

/// One row of the browse table. Unified shape so the
/// renderer doesn't care whether we're in the module list
/// or a directory listing.
#[derive(Debug, Clone)]
pub struct BrowseRow {
    pub name: String,
    pub kind: BrowseRowKind,
    pub size_bytes: u64,
    pub mtime_seconds: i64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BrowseRowKind {
    /// A module from `list_modules`. Cursor-enter descends
    /// into the module's root. `read_only` informs future
    /// transfer-action wiring (a1-7+).
    Module { read_only: bool },
    /// A subdirectory within a module. Cursor-enter pushes
    /// the name onto the path stack.
    Directory,
    /// A regular file. Cursor-enter is a no-op in this
    /// slice; future slices wire transfer triggers.
    File,
}

/// Fetch status for the current view's contents. Mirrors
/// `DaemonDetail`'s shape — Pending while in flight, Loaded
/// with rows + `fetched_at`, Error with a message.
#[derive(Debug, Clone)]
pub enum BrowseFetchStatus {
    /// No fetch attempted yet (the loop kicks one on the
    /// first iteration of a fresh view).
    Idle,
    /// Fetch in flight for the current view.
    Pending,
    /// Last fetch succeeded; rows are populated.
    Loaded { fetched_at: Instant },
    /// Last fetch failed; rows reflect whatever was loaded
    /// previously (could be empty), `message` describes the
    /// failure.
    Error { message: String },
}

#[derive(Debug, Clone)]
pub struct BrowseState {
    view: BrowseView,
    rows: Vec<BrowseRow>,
    selected: usize,
    status: BrowseFetchStatus,
    /// Per-view monotonically increasing request id. Used
    /// by the event loop to discard stale fetch replies
    /// (same generation pattern as `DaemonsState`).
    pending_request_id: u64,
}

impl Default for BrowseState {
    fn default() -> Self {
        Self::new()
    }
}

impl BrowseState {
    pub fn new() -> Self {
        Self {
            view: BrowseView::Modules,
            rows: Vec::new(),
            selected: 0,
            status: BrowseFetchStatus::Idle,
            pending_request_id: 0,
        }
    }

    pub fn view(&self) -> &BrowseView {
        &self.view
    }

    pub fn rows(&self) -> &[BrowseRow] {
        &self.rows
    }

    pub fn selected_index(&self) -> usize {
        self.selected
    }

    pub fn selected_row(&self) -> Option<&BrowseRow> {
        self.rows.get(self.selected)
    }

    pub fn status(&self) -> &BrowseFetchStatus {
        &self.status
    }

    /// Apply a fresh module-list result. The renderer
    /// translates `Module` rows into BrowseRowKind::Module
    /// rows; cursor resets to 0.
    pub fn apply_modules(&mut self, modules: Vec<Module>, fetched_at: Instant) {
        if !matches!(self.view, BrowseView::Modules) {
            // View moved away while the fetch was in flight
            // — drop the result.
            return;
        }
        self.rows = modules
            .into_iter()
            .map(|m| BrowseRow {
                name: m.name,
                kind: BrowseRowKind::Module {
                    read_only: m.read_only,
                },
                size_bytes: 0,
                mtime_seconds: 0,
            })
            .collect();
        self.selected = 0;
        self.status = BrowseFetchStatus::Loaded { fetched_at };
    }

    /// Apply a fresh directory listing result. Caller
    /// supplies the view context (the module name + path)
    /// the fetch was issued for so a stale reply doesn't
    /// land in a different view.
    pub fn apply_listing(
        &mut self,
        for_module: &str,
        for_path: &[String],
        entries: Vec<DirEntry>,
        fetched_at: Instant,
    ) {
        match &self.view {
            BrowseView::Module { name, path } if name == for_module && path == for_path => {}
            _ => return,
        }
        self.rows = entries
            .into_iter()
            .map(|e| BrowseRow {
                name: e.name,
                kind: if e.is_dir {
                    BrowseRowKind::Directory
                } else {
                    BrowseRowKind::File
                },
                size_bytes: e.size,
                mtime_seconds: e.mtime_seconds,
            })
            .collect();
        self.selected = 0;
        self.status = BrowseFetchStatus::Loaded { fetched_at };
    }

    /// Surface a fetch failure for the *current* view (the
    /// caller checked staleness via the request_id). Keeps
    /// the previous rows visible so the operator isn't
    /// dropped into a blank pane on a transient failure.
    pub fn note_fetch_error(&mut self, message: String) {
        self.status = BrowseFetchStatus::Error { message };
    }

    /// Move the cursor down one. No-op at the last row.
    pub fn select_next(&mut self) {
        if self.selected + 1 < self.rows.len() {
            self.selected += 1;
        }
    }

    /// Move the cursor up one. No-op at row 0.
    pub fn select_prev(&mut self) {
        if self.selected > 0 {
            self.selected -= 1;
        }
    }

    /// Descend into the selected row. Module → enter its
    /// root; Directory → push onto path. No-op on File.
    /// Returns the new view if navigation happened.
    pub fn descend(&mut self) -> Option<&BrowseView> {
        let row = self.rows.get(self.selected)?;
        match &row.kind {
            BrowseRowKind::Module { .. } => {
                let name = row.name.clone();
                self.view = BrowseView::Module {
                    name,
                    path: Vec::new(),
                };
            }
            BrowseRowKind::Directory => {
                let segment = row.name.clone();
                if let BrowseView::Module { path, .. } = &mut self.view {
                    path.push(segment);
                }
            }
            BrowseRowKind::File => return None,
        }
        self.rows.clear();
        self.selected = 0;
        self.status = BrowseFetchStatus::Idle;
        Some(&self.view)
    }

    /// Pop one level. At module root → back to module list.
    /// At module list → no-op. Returns the new view if
    /// navigation happened.
    pub fn ascend(&mut self) -> Option<&BrowseView> {
        match &mut self.view {
            BrowseView::Modules => return None,
            BrowseView::Module { path, .. } => {
                if path.is_empty() {
                    self.view = BrowseView::Modules;
                } else {
                    path.pop();
                }
            }
        }
        self.rows.clear();
        self.selected = 0;
        self.status = BrowseFetchStatus::Idle;
        Some(&self.view)
    }

    /// Bump the request id, set Pending, and return the
    /// new id. Caller embeds the id in the spawn so a
    /// stale reply (after navigation) can be discarded.
    pub fn begin_fetch(&mut self) -> u64 {
        self.pending_request_id += 1;
        self.status = BrowseFetchStatus::Pending;
        self.pending_request_id
    }

    /// Verify a reply matches the current generation. The
    /// caller should drop the result silently if this
    /// returns false.
    pub fn is_current_request(&self, request_id: u64) -> bool {
        request_id == self.pending_request_id
    }

    /// Render the current view as a short crumb string for
    /// the header — e.g. "modules", "home", "home/photos/2024".
    pub fn breadcrumb(&self) -> String {
        match &self.view {
            BrowseView::Modules => "modules".to_string(),
            BrowseView::Module { name, path } => {
                if path.is_empty() {
                    name.clone()
                } else {
                    format!("{}/{}", name, path.join("/"))
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use blit_app::admin::list_modules::Module;
    use blit_app::admin::ls::DirEntry;

    fn module(name: &str, read_only: bool) -> Module {
        Module {
            name: name.to_string(),
            path: format!("/srv/{name}"),
            read_only,
        }
    }

    fn dir_entry(name: &str, is_dir: bool) -> DirEntry {
        DirEntry {
            name: name.to_string(),
            is_dir,
            size: 100,
            mtime_seconds: 0,
        }
    }

    #[test]
    fn new_starts_in_modules_view() {
        let state = BrowseState::new();
        assert!(matches!(state.view(), BrowseView::Modules));
        assert!(state.rows().is_empty());
        assert!(matches!(state.status(), BrowseFetchStatus::Idle));
    }

    #[test]
    fn apply_modules_populates_rows_and_resets_cursor() {
        let mut state = BrowseState::new();
        state.apply_modules(
            vec![module("home", false), module("backups", true)],
            Instant::now(),
        );
        assert_eq!(state.rows().len(), 2);
        assert_eq!(state.rows()[0].name, "home");
        assert!(matches!(
            state.rows()[0].kind,
            BrowseRowKind::Module { read_only: false }
        ));
        assert!(matches!(
            state.rows()[1].kind,
            BrowseRowKind::Module { read_only: true }
        ));
        assert_eq!(state.selected_index(), 0);
    }

    #[test]
    fn descend_into_module_switches_view_and_clears_rows() {
        let mut state = BrowseState::new();
        state.apply_modules(vec![module("home", false)], Instant::now());
        let view = state.descend().cloned();
        match view {
            Some(BrowseView::Module { name, path }) => {
                assert_eq!(name, "home");
                assert!(path.is_empty());
            }
            other => panic!("expected Module view, got {other:?}"),
        }
        // Rows cleared; status reset to Idle ready for the
        // next fetch.
        assert!(state.rows().is_empty());
        assert!(matches!(state.status(), BrowseFetchStatus::Idle));
    }

    #[test]
    fn descend_into_directory_pushes_onto_path() {
        let mut state = BrowseState::new();
        state.apply_modules(vec![module("home", false)], Instant::now());
        state.descend(); // → BrowseView::Module { home, [] }
        state.apply_listing(
            "home",
            &[],
            vec![dir_entry("photos", true), dir_entry("readme.txt", false)],
            Instant::now(),
        );
        // photos @ 0, readme.txt @ 1.
        assert_eq!(state.rows().len(), 2);
        // Descend into photos.
        state.descend();
        match state.view() {
            BrowseView::Module { name, path } => {
                assert_eq!(name, "home");
                assert_eq!(path, &vec!["photos".to_string()]);
            }
            _ => panic!("expected Module view"),
        }
    }

    #[test]
    fn descend_on_file_is_no_op() {
        let mut state = BrowseState::new();
        state.apply_modules(vec![module("home", false)], Instant::now());
        state.descend();
        state.apply_listing(
            "home",
            &[],
            vec![dir_entry("readme.txt", false)],
            Instant::now(),
        );
        let prior_view = state.view().clone();
        let nav = state.descend();
        assert!(nav.is_none(), "descend on a file should return None");
        // View unchanged.
        match (state.view(), prior_view) {
            (
                BrowseView::Module { name: a, path: ap },
                BrowseView::Module { name: b, path: bp },
            ) => {
                assert_eq!(a, &b);
                assert_eq!(ap, &bp);
            }
            _ => panic!("expected Module view"),
        }
    }

    #[test]
    fn ascend_pops_path_then_returns_to_modules() {
        let mut state = BrowseState::new();
        state.apply_modules(vec![module("home", false)], Instant::now());
        state.descend(); // home
        state.apply_listing("home", &[], vec![dir_entry("photos", true)], Instant::now());
        state.descend(); // home/photos
        assert_eq!(
            match state.view() {
                BrowseView::Module { path, .. } => path.clone(),
                _ => panic!(),
            },
            vec!["photos".to_string()]
        );

        // Ascend → home (path empty).
        state.ascend();
        match state.view() {
            BrowseView::Module { name, path } => {
                assert_eq!(name, "home");
                assert!(path.is_empty());
            }
            _ => panic!("expected Module view"),
        }

        // Ascend again → Modules.
        state.ascend();
        assert!(matches!(state.view(), BrowseView::Modules));

        // Ascend at Modules → no-op.
        let nav = state.ascend();
        assert!(nav.is_none());
    }

    #[test]
    fn select_next_prev_bounded() {
        let mut state = BrowseState::new();
        state.apply_modules(
            vec![module("a", false), module("b", false), module("c", false)],
            Instant::now(),
        );
        state.select_prev();
        assert_eq!(state.selected_index(), 0);
        state.select_next();
        state.select_next();
        assert_eq!(state.selected_index(), 2);
        state.select_next(); // at last
        assert_eq!(state.selected_index(), 2);
    }

    #[test]
    fn apply_modules_dropped_when_view_changed() {
        // Stale module-list reply after the operator
        // already descended must NOT clobber the
        // directory rows.
        let mut state = BrowseState::new();
        state.apply_modules(vec![module("home", false)], Instant::now());
        state.descend(); // now BrowseView::Module
        state.apply_listing("home", &[], vec![dir_entry("a", false)], Instant::now());
        let prior_rows = state.rows().len();
        // Late module-list reply arrives.
        state.apply_modules(
            vec![module("home", false), module("backups", true)],
            Instant::now(),
        );
        assert_eq!(state.rows().len(), prior_rows);
    }

    #[test]
    fn apply_listing_dropped_when_path_no_longer_matches() {
        let mut state = BrowseState::new();
        state.apply_modules(vec![module("home", false)], Instant::now());
        state.descend(); // home
        state.apply_listing(
            "home",
            &[],
            vec![dir_entry("photos", true), dir_entry("docs", true)],
            Instant::now(),
        );
        state.descend(); // home/photos
                         // Stale reply for the home/ listing arrives:
        state.apply_listing(
            "home",
            &[],
            vec![
                dir_entry("photos", true),
                dir_entry("docs", true),
                dir_entry("extra", false),
            ],
            Instant::now(),
        );
        // Rows still belong to home/photos — empty so far
        // because we haven't applied that listing yet.
        assert!(state.rows().is_empty());
    }

    #[test]
    fn begin_fetch_and_is_current_request_track_generations() {
        let mut state = BrowseState::new();
        let id1 = state.begin_fetch();
        assert_eq!(id1, 1);
        assert!(state.is_current_request(1));
        let id2 = state.begin_fetch();
        assert_eq!(id2, 2);
        assert!(!state.is_current_request(1));
        assert!(state.is_current_request(2));
    }

    #[test]
    fn breadcrumb_reflects_current_view() {
        let mut state = BrowseState::new();
        assert_eq!(state.breadcrumb(), "modules");
        state.apply_modules(vec![module("home", false)], Instant::now());
        state.descend();
        assert_eq!(state.breadcrumb(), "home");
        state.apply_listing("home", &[], vec![dir_entry("photos", true)], Instant::now());
        state.descend();
        assert_eq!(state.breadcrumb(), "home/photos");
    }

    #[test]
    fn note_fetch_error_preserves_rows() {
        let mut state = BrowseState::new();
        state.apply_modules(vec![module("home", false)], Instant::now());
        state.note_fetch_error("connect refused".to_string());
        assert_eq!(state.rows().len(), 1);
        match state.status() {
            BrowseFetchStatus::Error { message } => assert_eq!(message, "connect refused"),
            other => panic!("expected Error, got {other:?}"),
        }
    }
}
