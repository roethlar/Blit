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
//! - `enter` (or `→` / `l`): descend into the cursor's
//!   directory. No-op on a file entry today (transfer /
//!   detail panes are future slices).
//! - `←` (or `h`): pop the path. At module root, pops back
//!   to the module list. At the module list, no-op.
//!
//! `q` and `Esc` are reserved for Quit and are NOT
//! interpreted as ascend — the operator's muscle memory for
//! quitting wins over the file-manager Esc convention.
//!
//! d-26: substring filter. `/` enters filter-edit mode;
//! chars append, Backspace pops, Enter commits (filter
//! persists, normal navigation resumes), Esc cancels
//! (filter cleared). Filter is case-insensitive and
//! matches anywhere in the row name. Cursor invariant
//! during/after filtering: `self.selected` always points
//! at a row that matches the filter (or sits at index 0
//! if no row matches). Changing views (descend / ascend
//! / fresh fetch result) clears the filter so the
//! operator starts each new directory with full
//! visibility.
//!
//! d-27: rows are sorted by `(kind_priority, name)` —
//! directories before files, then alphabetical
//! (case-insensitive) within each kind. The daemon
//! holds modules in a `HashMap` and `ls.rs` sorts by raw
//! `PathBuf`, so without client-side sorting F3 could
//! shuffle modules between reconnects and mix dirs +
//! files in arbitrary `PathBuf` order. Sorting here
//! gives a stable, scannable display.

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
    /// d-26: case-insensitive substring filter applied to
    /// row names. Empty string = "match everything".
    filter: String,
    /// d-26: `true` while the operator is actively typing
    /// into the filter (input router captures chars +
    /// Backspace + Esc + Enter). `false` means either the
    /// filter is unused OR the operator has committed it
    /// (and is now navigating the filtered list normally).
    editing_filter: bool,
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
            filter: String::new(),
            editing_filter: false,
        }
    }

    pub fn view(&self) -> &BrowseView {
        &self.view
    }

    pub fn rows(&self) -> &[BrowseRow] {
        &self.rows
    }

    /// Raw cursor index into `self.rows()`. d-26 routed
    /// the renderer through `visible_selected_position`
    /// (filter-aware), so production code no longer
    /// reads this — only tests probing the model
    /// directly.
    #[cfg(test)]
    pub fn selected_index(&self) -> usize {
        self.selected
    }

    /// The row the cursor is on, IF it's currently visible
    /// under the active filter. Returns `None` when the
    /// filter matches no rows (cursor sits on hidden row 0
    /// as a defensive fallback) or — more generally — when
    /// `self.selected` somehow points at a non-matching
    /// row. d-26 round-2 fix: pre-fix this returned the raw
    /// row 0, which let the Stats block display a hidden
    /// row as "Selected" and let `descend` step into it on
    /// a zero-match filter.
    pub fn selected_row(&self) -> Option<&BrowseRow> {
        let row = self.rows.get(self.selected)?;
        if self.row_matches(row) {
            Some(row)
        } else {
            None
        }
    }

    pub fn status(&self) -> &BrowseFetchStatus {
        &self.status
    }

    /// Apply a fresh module-list result. The renderer
    /// translates `Module` rows into BrowseRowKind::Module
    /// rows; cursor resets to 0.
    ///
    /// d-26: a fresh fetch result invalidates the d-26
    /// filter — the operator was filtering against the
    /// previous row set, so the active filter no longer
    /// reflects what they typed for. Reset to empty +
    /// non-editing so the new view starts visible.
    ///
    /// d-27: rows are sorted case-insensitively by name.
    /// The daemon stores modules in a `HashMap`, so
    /// `list_modules` returns rows in non-deterministic
    /// hash order — pre-d-27 the F3 module list shuffled
    /// across reconnects. Sorting client-side gives the
    /// operator a stable visual scan order without any
    /// daemon-side change.
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
        sort_rows(&mut self.rows);
        self.selected = 0;
        self.status = BrowseFetchStatus::Loaded { fetched_at };
        self.reset_filter();
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
        // d-27: same sort as `apply_modules`. Dirs sort
        // before files via `sort_priority`, then
        // alphabetical within each group — matches
        // file-manager conventions and `ls --group-directories-first`.
        sort_rows(&mut self.rows);
        self.selected = 0;
        self.status = BrowseFetchStatus::Loaded { fetched_at };
        // d-26: fresh fetch → drop the filter, same
        // rationale as `apply_modules`.
        self.reset_filter();
    }

    /// Surface a fetch failure for the *current* view (the
    /// caller checked staleness via the request_id). Keeps
    /// the previous rows visible so the operator isn't
    /// dropped into a blank pane on a transient failure.
    pub fn note_fetch_error(&mut self, message: String) {
        self.status = BrowseFetchStatus::Error { message };
    }

    /// Move the cursor down one matching row. No-op at
    /// the last matching row.
    ///
    /// d-26: filter-aware. When `filter` is non-empty,
    /// skips non-matching rows so the operator only
    /// traverses what's visible in the table.
    pub fn select_next(&mut self) {
        let mut i = self.selected + 1;
        while i < self.rows.len() {
            if self.row_matches(&self.rows[i]) {
                self.selected = i;
                return;
            }
            i += 1;
        }
    }

    /// Move the cursor up one matching row. No-op at the
    /// first matching row. d-26: same filter-aware skip
    /// as `select_next`.
    pub fn select_prev(&mut self) {
        if self.selected == 0 {
            return;
        }
        let mut i = self.selected - 1;
        loop {
            if self.row_matches(&self.rows[i]) {
                self.selected = i;
                return;
            }
            if i == 0 {
                return;
            }
            i -= 1;
        }
    }

    /// Descend into the selected row. Module → enter its
    /// root; Directory → push onto path. No-op on File.
    /// Returns the new view if navigation happened.
    ///
    /// d-26 round-2 fix: no-op when the cursor is on a
    /// row that's hidden by the active filter — otherwise
    /// `/zz` + Enter would step into raw row 0 even
    /// though the table is empty from the operator's
    /// perspective.
    pub fn descend(&mut self) -> Option<&BrowseView> {
        let row = self.rows.get(self.selected)?;
        if !self.row_matches(row) {
            return None;
        }
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
        // d-26: changing view drops the filter — the new
        // view's rows haven't even been fetched yet.
        self.reset_filter();
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
        // d-26: same view-change rationale as `descend`.
        self.reset_filter();
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

    // ---- d-26: filter API ----

    /// Current filter text. Empty string = no filter.
    pub fn filter(&self) -> &str {
        &self.filter
    }

    /// `true` while the operator is actively typing into
    /// the filter — the input router routes chars,
    /// Backspace, Esc, Enter to the filter API rather
    /// than the normal F3 dispatch.
    pub fn is_editing_filter(&self) -> bool {
        self.editing_filter
    }

    /// Enter filter-edit mode. Existing filter text is
    /// preserved so an operator can resume editing
    /// without retyping.
    pub fn begin_edit_filter(&mut self) {
        self.editing_filter = true;
    }

    /// Exit edit mode but keep the filter applied. The
    /// cursor was already kept on a matching row by
    /// `push_filter_char` / `pop_filter_char`, so no
    /// further bookkeeping is needed here.
    pub fn commit_filter(&mut self) {
        self.editing_filter = false;
    }

    /// Exit edit mode AND clear the filter — equivalent
    /// to "give me back the full view". Cursor snaps to
    /// row 0 (the new first-matching row in the cleared
    /// filter).
    pub fn cancel_filter(&mut self) {
        self.reset_filter();
        self.selected = 0;
    }

    /// Append one char to the filter and snap the cursor
    /// to the first matching row (or 0 if the new filter
    /// matches no rows).
    pub fn push_filter_char(&mut self, c: char) {
        self.filter.push(c);
        self.selected = self.first_matching_row().unwrap_or(0);
    }

    /// Drop the last char from the filter. Cursor snaps
    /// to first matching row (the looser filter may
    /// reveal earlier rows that were hidden).
    /// Returns true if a char was actually popped.
    pub fn pop_filter_char(&mut self) -> bool {
        let popped = self.filter.pop().is_some();
        if popped {
            self.selected = self.first_matching_row().unwrap_or(0);
        }
        popped
    }

    /// Indices into `self.rows()` that match the current
    /// filter. With an empty filter this is `0..len()`.
    /// The renderer uses this to build the visible table.
    pub fn visible_indices(&self) -> Vec<usize> {
        if self.filter.is_empty() {
            return (0..self.rows.len()).collect();
        }
        (0..self.rows.len())
            .filter(|&i| self.row_matches(&self.rows[i]))
            .collect()
    }

    /// d-28: message the Stats block renders when nothing
    /// is "selected" from the filter-aware cursor's
    /// perspective. Distinguishes the two empty-cursor
    /// reasons so the operator knows whether to wait for
    /// data or relax the filter:
    ///
    /// - `(no rows match filter)` when rows are loaded
    ///   but the active filter excludes everything.
    /// - `(no entries)` for every other empty state
    ///   (fresh pre-fetch state, empty module list, etc.).
    pub fn empty_state_message(&self) -> &'static str {
        if !self.rows.is_empty() && !self.filter.is_empty() && self.visible_indices().is_empty() {
            "(no rows match filter)"
        } else {
            "(no entries)"
        }
    }

    /// Position of `self.selected` within `visible_indices()`.
    /// Renderer feeds this into `TableState::with_selected`.
    /// Returns `None` if the cursor isn't on a visible
    /// row (which the cursor invariants forbid, but the
    /// API is defensive).
    pub fn visible_selected_position(&self) -> Option<usize> {
        if self.rows.is_empty() {
            return None;
        }
        self.visible_indices()
            .iter()
            .position(|&i| i == self.selected)
    }

    /// Internal helper: does this row pass the current
    /// filter? Empty filter passes everything;
    /// non-empty filter does a case-insensitive substring
    /// match on `row.name`.
    fn row_matches(&self, row: &BrowseRow) -> bool {
        if self.filter.is_empty() {
            return true;
        }
        let needle = self.filter.to_lowercase();
        row.name.to_lowercase().contains(&needle)
    }

    /// First row index that matches the current filter,
    /// or `None` if no row matches. Used after a filter
    /// change to keep the cursor on something visible.
    fn first_matching_row(&self) -> Option<usize> {
        (0..self.rows.len()).find(|&i| self.row_matches(&self.rows[i]))
    }

    /// Reset filter + edit-mode to the cleared state.
    /// Called from `apply_modules` / `apply_listing` /
    /// `descend` / `ascend` whenever the row set changes
    /// underneath us.
    fn reset_filter(&mut self) {
        self.filter.clear();
        self.editing_filter = false;
    }
}

/// d-33: derive the canonical remote pull-source spec for
/// the F3 cursor — `<authority>:/<module>/<rel-path>` — or
/// `None` when there's nothing pullable (no authority, no
/// selected row, or a stale cursor on a non-matching
/// filtered row).
///
/// `authority` is the endpoint's display authority from
/// [`blit_core::remote::endpoint::RemoteEndpoint::host_port_display`]
/// — bracketed for IPv6 literals (`[::1]`) and carrying a
/// non-default port (`host:9999`). d-33 round 2 fix:
/// passing the raw `host` field produced `::1:/share/...`
/// for IPv6 remotes, which doesn't round-trip through
/// `RemoteEndpoint::parse`. The display authority is the
/// single source of truth for "how to render this host
/// back into a copyable spec".
///
/// This is the first slice of F3 transfer-from-cursor:
/// it surfaces the resolvable source path as read-only
/// info in the Stats block. Later slices add the
/// destination prompt and the actual pull execution,
/// which will reconstruct a `RemoteEndpoint` from the
/// operator's `--remote` (for host + port) plus the
/// module + rel_path this function's logic identifies.
/// The string here is the human-readable preview.
///
/// Cases:
/// - Modules view, cursor on a `Module` row →
///   `<authority>:/<module>/` (pull the module root).
/// - Module view, cursor on a `Directory` →
///   `<authority>:/<module>/<path>/<dir>/` (trailing slash).
/// - Module view, cursor on a `File` →
///   `<authority>:/<module>/<path>/<file>` (no trailing slash).
pub fn pull_source_spec(
    view: &BrowseView,
    selected: Option<&BrowseRow>,
    authority: &str,
) -> Option<String> {
    if authority.is_empty() {
        return None;
    }
    let row = selected?;
    match view {
        BrowseView::Modules => match &row.kind {
            BrowseRowKind::Module { .. } => Some(format!("{authority}:/{}/", row.name)),
            // A non-module row in the Modules view is a
            // contradiction the model never produces.
            _ => None,
        },
        BrowseView::Module { name, path } => {
            let mut rel = String::new();
            for seg in path {
                rel.push_str(seg);
                rel.push('/');
            }
            match &row.kind {
                BrowseRowKind::Directory => Some(format!("{authority}:/{name}/{rel}{}/", row.name)),
                BrowseRowKind::File => Some(format!("{authority}:/{name}/{rel}{}", row.name)),
                // A Module row inside a Module view is a
                // contradiction the model never produces.
                BrowseRowKind::Module { .. } => None,
            }
        }
    }
}

/// d-27: stable sort key for browse rows. Directories
/// sort before files; within each kind, alphabetical
/// by `name` (case-insensitive). Module rows (top-level
/// view) all hash to the same priority — only the name
/// matters there.
///
/// d-27 round 2: the key is `(priority, lowercase_name,
/// original_name)`. The lowercase form drives the
/// case-insensitive primary order, and the original
/// name breaks ties between case variants (`Foo` vs.
/// `foo` → both have lowercase `"foo"`, but the original
/// `"Foo"` < `"foo"` by raw bytes so the case-variant
/// pair lands deterministically). Without the
/// tiebreaker, stable-sort preserved upstream fetch
/// order — which for the daemon's `HashMap` is
/// non-deterministic across reconnects, defeating the
/// whole point of d-27.
///
/// Public-within-the-module so tests can probe the
/// helper directly. `sort_by_cached_key` builds the
/// composite key once per row, avoiding the
/// `to_lowercase()` allocation on every comparator
/// call.
fn sort_rows(rows: &mut [BrowseRow]) {
    rows.sort_by_cached_key(|row| {
        (
            sort_priority(&row.kind),
            row.name.to_lowercase(),
            row.name.clone(),
        )
    });
}

/// Sort-priority companion to [`sort_rows`]. Lower
/// number = sorted earlier:
///
/// - 0: modules (top-level view; all rows share this
///   priority so it's effectively a name-only sort).
/// - 1: directories.
/// - 2: files.
fn sort_priority(kind: &BrowseRowKind) -> u8 {
    match kind {
        BrowseRowKind::Module { .. } => 0,
        BrowseRowKind::Directory => 1,
        BrowseRowKind::File => 2,
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
        // d-27: rows are now sorted alphabetically — input
        // [home, backups] → display [backups, home].
        assert_eq!(state.rows()[0].name, "backups");
        assert_eq!(state.rows()[1].name, "home");
        assert!(matches!(
            state.rows()[0].kind,
            BrowseRowKind::Module { read_only: true }
        ));
        assert!(matches!(
            state.rows()[1].kind,
            BrowseRowKind::Module { read_only: false }
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

    // d-26: filter state machine + filter-aware nav.

    fn populated_state() -> BrowseState {
        let mut state = BrowseState::new();
        state.apply_modules(
            vec![
                module("home", false),
                module("backups", true),
                module("photos", false),
                module("scratch", false),
            ],
            Instant::now(),
        );
        state
    }

    #[test]
    fn new_state_has_empty_filter_and_not_editing() {
        let state = BrowseState::new();
        assert_eq!(state.filter(), "");
        assert!(!state.is_editing_filter());
    }

    #[test]
    fn begin_edit_filter_enters_edit_mode() {
        let mut state = populated_state();
        state.begin_edit_filter();
        assert!(state.is_editing_filter());
        // Filter text not changed by begin alone.
        assert_eq!(state.filter(), "");
    }

    #[test]
    fn push_filter_char_appends_and_snaps_cursor() {
        let mut state = populated_state();
        state.begin_edit_filter();
        // "p" matches "backups" (has a `p`) and "photos".
        // first_matching_row returns the lowest-index
        // match → "backups" (idx 1).
        state.push_filter_char('p');
        assert_eq!(state.filter(), "p");
        assert_eq!(state.rows()[state.selected_index()].name, "backups");
    }

    #[test]
    fn push_filter_char_is_case_insensitive() {
        let mut state = populated_state();
        state.begin_edit_filter();
        state.push_filter_char('B'); // matches "backups"
        assert_eq!(state.rows()[state.selected_index()].name, "backups");
    }

    #[test]
    fn pop_filter_char_widens_match_set() {
        let mut state = populated_state();
        state.begin_edit_filter();
        state.push_filter_char('p');
        state.push_filter_char('h'); // "ph" → photos only
        assert_eq!(state.filter(), "ph");
        assert_eq!(state.visible_indices(), vec![2]);
        let popped = state.pop_filter_char();
        assert!(popped);
        assert_eq!(state.filter(), "p");
    }

    #[test]
    fn pop_filter_char_returns_false_on_empty_filter() {
        let mut state = populated_state();
        state.begin_edit_filter();
        assert!(!state.pop_filter_char());
    }

    #[test]
    fn cancel_filter_clears_text_and_exits_mode() {
        let mut state = populated_state();
        state.begin_edit_filter();
        state.push_filter_char('p');
        state.cancel_filter();
        assert_eq!(state.filter(), "");
        assert!(!state.is_editing_filter());
        // Cursor goes back to row 0 (the new first match
        // in the cleared filter).
        assert_eq!(state.selected_index(), 0);
    }

    #[test]
    fn commit_filter_keeps_text_and_exits_mode() {
        let mut state = populated_state();
        state.begin_edit_filter();
        // Push "ph" → only "photos" matches ("backups"
        // has 'p' but not 'h' next-letter contiguously).
        state.push_filter_char('p');
        state.push_filter_char('h');
        state.commit_filter();
        assert_eq!(state.filter(), "ph");
        assert!(!state.is_editing_filter());
        // Cursor on the unique match.
        assert_eq!(state.rows()[state.selected_index()].name, "photos");
    }

    #[test]
    fn visible_indices_returns_all_rows_with_empty_filter() {
        let state = populated_state();
        assert_eq!(state.visible_indices(), vec![0, 1, 2, 3]);
    }

    #[test]
    fn visible_indices_filters_by_substring() {
        let mut state = populated_state();
        state.begin_edit_filter();
        state.push_filter_char('s');
        // d-27: populated_state() input is [home, backups,
        // photos, scratch] → sorted to [backups, home,
        // photos, scratch]. 's' matches backups (raw 0),
        // photos (raw 2), scratch (raw 3) — "home" has no
        // 's'. Indices reflect the sorted layout.
        let indices = state.visible_indices();
        assert_eq!(indices, vec![0, 2, 3]);
    }

    #[test]
    fn visible_indices_empty_when_no_match() {
        let mut state = populated_state();
        state.begin_edit_filter();
        state.push_filter_char('z'); // nothing matches
        assert!(state.visible_indices().is_empty());
        // Cursor snaps to 0 (defensive fallback).
        assert_eq!(state.selected_index(), 0);
    }

    #[test]
    fn select_next_skips_non_matching_rows_when_filter_active() {
        let mut state = populated_state();
        state.begin_edit_filter();
        state.push_filter_char('s'); // matches backups, photos, scratch
                                     // Cursor starts on "backups" (first match @ idx 1).
        assert_eq!(state.rows()[state.selected_index()].name, "backups");
        state.select_next();
        assert_eq!(state.rows()[state.selected_index()].name, "photos");
        state.select_next();
        assert_eq!(state.rows()[state.selected_index()].name, "scratch");
        // At last match — no-op.
        state.select_next();
        assert_eq!(state.rows()[state.selected_index()].name, "scratch");
    }

    #[test]
    fn select_prev_skips_non_matching_rows_when_filter_active() {
        let mut state = populated_state();
        state.begin_edit_filter();
        state.push_filter_char('s');
        // Position cursor on "scratch".
        state.select_next();
        state.select_next();
        assert_eq!(state.rows()[state.selected_index()].name, "scratch");
        state.select_prev();
        assert_eq!(state.rows()[state.selected_index()].name, "photos");
        state.select_prev();
        assert_eq!(state.rows()[state.selected_index()].name, "backups");
        state.select_prev();
        assert_eq!(state.rows()[state.selected_index()].name, "backups");
    }

    #[test]
    fn visible_selected_position_maps_into_filtered_ordinal() {
        let mut state = populated_state();
        state.begin_edit_filter();
        state.push_filter_char('s');
        // Filter visible: backups (raw 1), photos (raw 2),
        // scratch (raw 3). Cursor starts on raw 1, which
        // is visible-ordinal 0.
        assert_eq!(state.visible_selected_position(), Some(0));
        state.select_next();
        assert_eq!(state.visible_selected_position(), Some(1));
        state.select_next();
        assert_eq!(state.visible_selected_position(), Some(2));
    }

    #[test]
    fn descend_clears_filter() {
        let mut state = populated_state();
        state.begin_edit_filter();
        state.push_filter_char('p');
        state.commit_filter();
        // Cursor is on first 'p' match ("backups" — d-27
        // sort order). Descend into it.
        state.descend();
        assert_eq!(state.filter(), "");
        assert!(!state.is_editing_filter());
    }

    #[test]
    fn ascend_clears_filter() {
        let mut state = populated_state();
        // d-27 sort: cursor at row 0 = "backups". Descend
        // sets view to Module { backups, [] }; the
        // apply_listing args must match.
        state.descend();
        state.apply_listing(
            "backups",
            &[],
            vec![dir_entry("photos", true), dir_entry("readme.txt", false)],
            Instant::now(),
        );
        state.begin_edit_filter();
        state.push_filter_char('r');
        state.commit_filter();
        assert_eq!(state.filter(), "r");
        state.ascend();
        assert_eq!(state.filter(), "");
        assert!(!state.is_editing_filter());
    }

    #[test]
    fn apply_modules_clears_stale_filter() {
        let mut state = populated_state();
        state.begin_edit_filter();
        state.push_filter_char('s');
        // A new fetch lands — filter must drop.
        state.apply_modules(
            vec![module("home", false), module("other", false)],
            Instant::now(),
        );
        assert_eq!(state.filter(), "");
        assert!(!state.is_editing_filter());
    }

    #[test]
    fn apply_listing_clears_stale_filter() {
        let mut state = populated_state();
        // d-27 sort: row 0 = "backups". Descend into it.
        state.descend();
        state.apply_listing(
            "backups",
            &[],
            vec![dir_entry("photos", true)],
            Instant::now(),
        );
        state.begin_edit_filter();
        state.push_filter_char('p');
        // Another listing lands (e.g. operator pressed
        // `r` to refresh) — filter must reset.
        state.apply_listing(
            "backups",
            &[],
            vec![dir_entry("photos", true), dir_entry("readme.txt", false)],
            Instant::now(),
        );
        assert_eq!(state.filter(), "");
        assert!(!state.is_editing_filter());
    }

    #[test]
    fn select_next_with_no_match_keeps_cursor_at_zero() {
        let mut state = populated_state();
        state.begin_edit_filter();
        state.push_filter_char('z');
        state.select_next();
        // No matching rows → select_next walks to end
        // without finding a match → cursor unchanged.
        assert_eq!(state.selected_index(), 0);
    }

    // d-26 round 2: zero-match filter must not surface a
    // hidden row as selected / actionable.

    /// Reviewer-flagged regression: `/zz` matches nothing,
    /// `selected = 0` falls back to a hidden row. The
    /// renderer's "Selected: foo" line and the
    /// dispatcher's `descend` both used to read that
    /// hidden row. Post-fix: `selected_row()` returns
    /// `None` and `descend()` no-ops.
    #[test]
    fn selected_row_is_none_when_filter_matches_nothing() {
        let mut state = populated_state();
        state.begin_edit_filter();
        state.push_filter_char('z'); // matches none of home/backups/photos/scratch
        assert!(state.visible_indices().is_empty());
        assert!(
            state.selected_row().is_none(),
            "selected_row must hide row 0 when the filter matches no rows; \
             pre-fix this returned the raw row 0 and the Stats block lied \
             about what was selected"
        );
    }

    /// Reviewer-flagged regression: pressing Enter / → / l
    /// while the filter matches zero rows used to step
    /// into raw row 0. Post-fix: `descend` no-ops, view
    /// is unchanged.
    #[test]
    fn descend_no_ops_when_filter_matches_nothing() {
        let mut state = populated_state();
        state.begin_edit_filter();
        state.push_filter_char('z');
        let view_before = state.view().clone();
        let result = state.descend();
        assert!(result.is_none(), "descend on a hidden row must return None");
        // View unchanged — operator stayed in Modules.
        match (state.view(), &view_before) {
            (BrowseView::Modules, BrowseView::Modules) => {}
            other => panic!("descend leaked into {:?}", other.0),
        }
    }

    /// End-to-end scenario the reviewer described:
    /// `/zz` + commit + descend. Pre-fix the operator
    /// would silently step into a hidden module; post-fix
    /// everything is inert.
    #[test]
    fn zero_match_then_commit_then_enter_is_inert() {
        let mut state = populated_state();
        state.begin_edit_filter();
        state.push_filter_char('z');
        state.push_filter_char('z');
        state.commit_filter(); // Enter while editing
        assert_eq!(state.filter(), "zz");
        assert!(!state.is_editing_filter());
        // Enter again (now in nav mode, dispatcher calls descend).
        let view_before = state.view().clone();
        let result = state.descend();
        assert!(result.is_none());
        assert!(state.selected_row().is_none());
        assert!(matches!(state.view(), BrowseView::Modules));
        // And the original view fields are intact.
        assert!(matches!(view_before, BrowseView::Modules));
    }

    /// A filter that hides the previously-selected row but
    /// matches others must STILL move the cursor — push
    /// already snaps to first_matching_row. This pins the
    /// non-pathological case so the round-2 fix didn't
    /// regress it.
    #[test]
    fn filter_tightening_to_partial_match_still_advances_cursor() {
        let mut state = populated_state();
        // Cursor starts on "home" (idx 0).
        state.begin_edit_filter();
        // "ph" matches only "photos" — push_filter_char
        // snaps cursor to first_matching_row.
        state.push_filter_char('p');
        state.push_filter_char('h');
        assert!(
            state.selected_row().is_some(),
            "matching row must be visible"
        );
        assert_eq!(state.selected_row().unwrap().name, "photos");
    }

    // d-27: stable sort — alphabetical, dirs-first.

    /// Input is reverse-alphabetical; sort restores
    /// ascending order. Pins the lexicographic case.
    #[test]
    fn apply_modules_sorts_alphabetically() {
        let mut state = BrowseState::new();
        state.apply_modules(
            vec![
                module("zeta", false),
                module("alpha", false),
                module("mu", false),
            ],
            Instant::now(),
        );
        let names: Vec<&str> = state.rows().iter().map(|r| r.name.as_str()).collect();
        assert_eq!(names, vec!["alpha", "mu", "zeta"]);
    }

    /// Sort is case-insensitive — "Backups" and "alpha"
    /// land where their lowercased forms order them, not
    /// where ASCII-uppercase-first would put them
    /// ("Backups" < "alpha" by raw bytes).
    #[test]
    fn apply_modules_sort_is_case_insensitive() {
        let mut state = BrowseState::new();
        state.apply_modules(
            vec![
                module("Backups", false),
                module("alpha", false),
                module("Cache", false),
            ],
            Instant::now(),
        );
        let names: Vec<&str> = state.rows().iter().map(|r| r.name.as_str()).collect();
        assert_eq!(names, vec!["alpha", "Backups", "Cache"]);
    }

    /// Daemon-side `modules: HashMap` returns rows in
    /// hash order — non-deterministic across reconnects.
    /// d-27's client-side sort gives the operator a
    /// stable display regardless of fetch order.
    #[test]
    fn apply_modules_sort_is_deterministic_regardless_of_input_order() {
        let input_a = vec![
            module("home", false),
            module("backups", true),
            module("photos", false),
        ];
        let input_b = vec![
            module("photos", false),
            module("home", false),
            module("backups", true),
        ];
        let mut state_a = BrowseState::new();
        state_a.apply_modules(input_a, Instant::now());
        let mut state_b = BrowseState::new();
        state_b.apply_modules(input_b, Instant::now());
        let names_a: Vec<&str> = state_a.rows().iter().map(|r| r.name.as_str()).collect();
        let names_b: Vec<&str> = state_b.rows().iter().map(|r| r.name.as_str()).collect();
        assert_eq!(names_a, names_b);
        assert_eq!(names_a, vec!["backups", "home", "photos"]);
    }

    /// Directory listings: dirs sort before files, then
    /// alphabetical within each group. Matches `ls
    /// --group-directories-first` and file-manager
    /// conventions.
    #[test]
    fn apply_listing_sorts_dirs_before_files() {
        let mut state = BrowseState::new();
        state.apply_modules(vec![module("home", false)], Instant::now());
        state.descend();
        state.apply_listing(
            "home",
            &[],
            vec![
                dir_entry("zfile.txt", false),
                dir_entry("photos", true),
                dir_entry("afile.txt", false),
                dir_entry("docs", true),
            ],
            Instant::now(),
        );
        let names: Vec<&str> = state.rows().iter().map(|r| r.name.as_str()).collect();
        // docs, photos (dirs alphabetical), then afile.txt,
        // zfile.txt (files alphabetical).
        assert_eq!(names, vec!["docs", "photos", "afile.txt", "zfile.txt"]);
    }

    // d-28: differentiated empty-state message.

    /// Fresh state — no rows, no filter. Renderer falls
    /// through to the standard `(no entries)` line.
    #[test]
    fn empty_state_message_when_no_rows_returns_no_entries() {
        let state = BrowseState::new();
        assert_eq!(state.empty_state_message(), "(no entries)");
    }

    /// Populated rows, non-empty filter that matches
    /// everything — `empty_state_message` would be wrong
    /// to even ask in this case (selected_row returns
    /// Some), but the helper's contract should still be
    /// honest: it's not a "no rows match filter" state.
    #[test]
    fn empty_state_message_when_filter_has_matches_returns_no_entries() {
        let mut state = populated_state();
        state.begin_edit_filter();
        state.push_filter_char('p'); // matches backups + photos
        assert!(!state.visible_indices().is_empty());
        assert_eq!(state.empty_state_message(), "(no entries)");
    }

    /// d-28 headline: populated rows + filter that excludes
    /// everything → operator-facing hint that the filter,
    /// not the data, is what's hiding the view.
    #[test]
    fn empty_state_message_when_filter_matches_nothing_returns_filter_hint() {
        let mut state = populated_state();
        state.begin_edit_filter();
        state.push_filter_char('z'); // matches nothing in populated_state
        assert!(state.visible_indices().is_empty());
        assert_eq!(state.empty_state_message(), "(no rows match filter)");
    }

    /// Edge case: no rows AND non-empty filter. The
    /// filter can't be "the reason" the view is empty —
    /// there were no rows to filter in the first place.
    /// Fall through to `(no entries)`.
    #[test]
    fn empty_state_message_when_no_rows_with_filter_returns_no_entries() {
        let mut state = BrowseState::new();
        // No fetch — rows empty. Manually start the
        // filter to simulate the operator typing into an
        // unloaded view.
        state.begin_edit_filter();
        state.push_filter_char('x');
        assert!(state.rows().is_empty());
        assert!(!state.filter().is_empty());
        assert_eq!(state.empty_state_message(), "(no entries)");
    }

    /// `sort_priority` directly: pins the numeric ranks
    /// that drive the directory-first invariant.
    #[test]
    fn sort_priority_matrix() {
        assert_eq!(
            sort_priority(&BrowseRowKind::Module { read_only: false }),
            0
        );
        assert_eq!(sort_priority(&BrowseRowKind::Module { read_only: true }), 0);
        assert_eq!(sort_priority(&BrowseRowKind::Directory), 1);
        assert_eq!(sort_priority(&BrowseRowKind::File), 2);
    }

    // d-27 round 2: case-variant tiebreaker.

    /// Reviewer-flagged regression: case-variant names
    /// (`Foo` / `foo`) share the same lowercase sort key.
    /// Pre-R2 the stable sort preserved upstream fetch
    /// order — which for the daemon's `HashMap` is
    /// non-deterministic. R2 fix: tiebreak on the
    /// original name (raw bytes), so the pair lands the
    /// same way regardless of input order.
    #[test]
    fn case_variants_sort_deterministically_regardless_of_input_order() {
        let input_a = vec![module("Foo", false), module("foo", false)];
        let input_b = vec![module("foo", false), module("Foo", false)];
        let mut state_a = BrowseState::new();
        state_a.apply_modules(input_a, Instant::now());
        let mut state_b = BrowseState::new();
        state_b.apply_modules(input_b, Instant::now());
        let names_a: Vec<&str> = state_a.rows().iter().map(|r| r.name.as_str()).collect();
        let names_b: Vec<&str> = state_b.rows().iter().map(|r| r.name.as_str()).collect();
        assert_eq!(names_a, names_b);
        // Raw-byte tiebreak: 'F' (0x46) < 'f' (0x66), so
        // `Foo` precedes `foo` in both inputs.
        assert_eq!(names_a, vec!["Foo", "foo"]);
    }

    /// Mixed case-variants + non-variants in the same
    /// listing must remain deterministic. Sanity check
    /// the larger interaction.
    #[test]
    fn case_variants_mixed_with_other_names_stay_deterministic() {
        let input_a = vec![
            module("Foo", false),
            module("alpha", false),
            module("foo", false),
            module("zeta", false),
        ];
        let input_b = vec![
            module("zeta", false),
            module("foo", false),
            module("alpha", false),
            module("Foo", false),
        ];
        let mut state_a = BrowseState::new();
        state_a.apply_modules(input_a, Instant::now());
        let mut state_b = BrowseState::new();
        state_b.apply_modules(input_b, Instant::now());
        let names_a: Vec<&str> = state_a.rows().iter().map(|r| r.name.as_str()).collect();
        let names_b: Vec<&str> = state_b.rows().iter().map(|r| r.name.as_str()).collect();
        assert_eq!(names_a, names_b);
        // alpha < {Foo, foo} < zeta by lowercase; Foo
        // wins the case-variant tiebreak vs. foo.
        assert_eq!(names_a, vec!["alpha", "Foo", "foo", "zeta"]);
    }

    // d-33: pull-source spec derivation.

    fn module_row(name: &str) -> BrowseRow {
        BrowseRow {
            name: name.to_string(),
            kind: BrowseRowKind::Module { read_only: false },
            size_bytes: 0,
            mtime_seconds: 0,
        }
    }

    fn dir_row(name: &str) -> BrowseRow {
        BrowseRow {
            name: name.to_string(),
            kind: BrowseRowKind::Directory,
            size_bytes: 0,
            mtime_seconds: 0,
        }
    }

    fn file_row(name: &str) -> BrowseRow {
        BrowseRow {
            name: name.to_string(),
            kind: BrowseRowKind::File,
            size_bytes: 10,
            mtime_seconds: 0,
        }
    }

    #[test]
    fn pull_source_none_without_host() {
        let row = module_row("photos");
        assert!(pull_source_spec(&BrowseView::Modules, Some(&row), "").is_none());
    }

    #[test]
    fn pull_source_none_without_selection() {
        assert!(pull_source_spec(&BrowseView::Modules, None, "host").is_none());
    }

    #[test]
    fn pull_source_module_root_from_modules_view() {
        let row = module_row("photos");
        assert_eq!(
            pull_source_spec(&BrowseView::Modules, Some(&row), "nas"),
            Some("nas:/photos/".to_string())
        );
    }

    #[test]
    fn pull_source_directory_at_module_root() {
        let view = BrowseView::Module {
            name: "photos".to_string(),
            path: vec![],
        };
        let row = dir_row("2024");
        assert_eq!(
            pull_source_spec(&view, Some(&row), "nas"),
            Some("nas:/photos/2024/".to_string())
        );
    }

    #[test]
    fn pull_source_directory_nested() {
        let view = BrowseView::Module {
            name: "photos".to_string(),
            path: vec!["2024".to_string(), "summer".to_string()],
        };
        let row = dir_row("beach");
        assert_eq!(
            pull_source_spec(&view, Some(&row), "nas"),
            Some("nas:/photos/2024/summer/beach/".to_string())
        );
    }

    #[test]
    fn pull_source_file_has_no_trailing_slash() {
        let view = BrowseView::Module {
            name: "photos".to_string(),
            path: vec!["2024".to_string()],
        };
        let row = file_row("img001.jpg");
        assert_eq!(
            pull_source_spec(&view, Some(&row), "nas"),
            Some("nas:/photos/2024/img001.jpg".to_string())
        );
    }

    #[test]
    fn pull_source_file_at_module_root() {
        let view = BrowseView::Module {
            name: "docs".to_string(),
            path: vec![],
        };
        let row = file_row("readme.txt");
        assert_eq!(
            pull_source_spec(&view, Some(&row), "host"),
            Some("host:/docs/readme.txt".to_string())
        );
    }

    /// A module row inside a module view, or a non-module
    /// row in the modules view, are contradictions the
    /// model never produces — the helper returns None
    /// rather than emit a malformed spec.
    #[test]
    fn pull_source_rejects_contradictory_kind() {
        // Module row inside a Module view.
        let view = BrowseView::Module {
            name: "photos".to_string(),
            path: vec![],
        };
        let row = module_row("nested");
        assert!(pull_source_spec(&view, Some(&row), "nas").is_none());
        // Non-module row in the Modules view.
        let row = dir_row("strays");
        assert!(pull_source_spec(&BrowseView::Modules, Some(&row), "nas").is_none());
    }

    /// d-33 round 2 regression: an IPv6 authority is
    /// already bracketed (the caller passes
    /// `RemoteEndpoint::host_port_display()`, which
    /// brackets IPv6 literals). The resulting spec must
    /// round-trip through `RemoteEndpoint::parse`.
    /// Pre-fix, the raw `host` field `"::1"` produced the
    /// un-parseable `::1:/share/...`.
    #[test]
    fn pull_source_ipv6_authority_round_trips() {
        use blit_core::remote::endpoint::RemoteEndpoint;
        let view = BrowseView::Module {
            name: "share".to_string(),
            path: vec!["docs".to_string()],
        };
        let row = file_row("readme.txt");
        // The caller hands us the bracketed display
        // authority, not the raw host.
        let authority = "[::1]";
        let spec =
            pull_source_spec(&view, Some(&row), authority).expect("spec for an IPv6 authority");
        assert_eq!(spec, "[::1]:/share/docs/readme.txt");
        // The whole point: it parses back into an endpoint.
        let parsed = RemoteEndpoint::parse(&spec).expect("IPv6 spec must re-parse");
        assert_eq!(parsed.host, "::1");
    }

    /// d-33 round 2: a non-default port survives in the
    /// authority (also via `host_port_display`).
    #[test]
    fn pull_source_non_default_port_authority() {
        let view = BrowseView::Module {
            name: "share".to_string(),
            path: vec![],
        };
        let row = dir_row("logs");
        let spec =
            pull_source_spec(&view, Some(&row), "host:9999").expect("spec for a non-default port");
        assert_eq!(spec, "host:9999:/share/logs/");
    }
}
