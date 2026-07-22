//! Phase 6 dual-pane browser state.
//!
//! This module is intentionally presenter-agnostic: it models the
//! two-pane shell, locations, row selection, marks, action labels, and
//! browse-provider request/response mapping. Transfer execution lands
//! in later Phase 6 slices.

use blit_app::admin::list_modules::Module;
use blit_app::admin::ls::DirEntry;
use blit_core::remote::{RemoteEndpoint, RemotePath};
use eyre::{Context, Result};
use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};

pub type EntryId = String;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PaneId {
    Left,
    Right,
}

impl PaneId {
    pub fn other(self) -> Self {
        match self {
            Self::Left => Self::Right,
            Self::Right => Self::Left,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::Left => "Left",
            Self::Right => "Right",
        }
    }
}

#[derive(Debug, Clone)]
pub enum Location {
    Places,
    Local(PathBuf),
    Remote(RemoteEndpoint),
}

impl Location {
    #[cfg(test)]
    pub fn local(path: impl Into<PathBuf>) -> Self {
        Self::Local(path.into())
    }

    #[cfg(test)]
    pub fn local_path(&self) -> Option<&Path> {
        match self {
            Self::Places => None,
            Self::Local(path) => Some(path),
            Self::Remote(_) => None,
        }
    }

    #[cfg(test)]
    pub fn remote_endpoint(&self) -> Option<&RemoteEndpoint> {
        match self {
            Self::Places => None,
            Self::Local(_) => None,
            Self::Remote(endpoint) => Some(endpoint),
        }
    }

    pub fn display(&self) -> String {
        match self {
            Self::Places => "Places".to_string(),
            Self::Local(path) => format!("Local {}", display_path(path)),
            Self::Remote(endpoint) => endpoint.display(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EntryKind {
    Place,
    Parent,
    Module,
    Directory,
    File,
    Symlink,
    Other,
}

impl EntryKind {
    pub fn label(self) -> &'static str {
        match self {
            Self::Place => "place",
            Self::Parent => "up",
            Self::Module => "module",
            Self::Directory => "dir",
            Self::File => "file",
            Self::Symlink => "link",
            Self::Other => "other",
        }
    }
}

#[derive(Debug, Clone)]
pub struct BrowserEntry {
    pub id: EntryId,
    pub name: String,
    pub kind: EntryKind,
    pub size: Option<u64>,
    pub mtime_seconds: Option<i64>,
    pub read_only: bool,
    pub target_location: Option<Location>,
}

impl BrowserEntry {
    pub fn new(id: impl Into<EntryId>, name: impl Into<String>, kind: EntryKind) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            kind,
            size: None,
            mtime_seconds: None,
            read_only: false,
            target_location: None,
        }
    }

    pub fn with_target(mut self, target: Location) -> Self {
        self.target_location = Some(target);
        self
    }

    #[cfg(test)]
    pub fn with_size(mut self, size: u64) -> Self {
        self.size = Some(size);
        self
    }
}

#[derive(Debug, Clone)]
pub enum PaneFetchStatus {
    Idle,
    Pending { request_id: u64 },
    Loaded,
    Error { message: String },
}

#[derive(Debug, Clone)]
pub enum PaneFetchRequest {
    Local {
        pane_id: PaneId,
        request_id: u64,
        path: PathBuf,
    },
    RemoteModules {
        pane_id: PaneId,
        request_id: u64,
        endpoint: RemoteEndpoint,
    },
    RemoteListing {
        pane_id: PaneId,
        request_id: u64,
        endpoint: RemoteEndpoint,
        module: String,
        path: String,
    },
}

#[derive(Debug, Clone)]
pub struct PaneState {
    id: PaneId,
    location: Location,
    entries: Vec<BrowserEntry>,
    cursor: usize,
    marked: BTreeSet<EntryId>,
    path_editor: String,
    filter: String,
    status: PaneFetchStatus,
    pending_request_id: u64,
}

impl PaneState {
    pub fn new(id: PaneId, location: Location) -> Self {
        let path_editor = location.display();
        Self {
            id,
            location,
            entries: Vec::new(),
            cursor: 0,
            marked: BTreeSet::new(),
            path_editor,
            filter: String::new(),
            status: PaneFetchStatus::Idle,
            pending_request_id: 0,
        }
    }

    pub fn id(&self) -> PaneId {
        self.id
    }

    pub fn location(&self) -> &Location {
        &self.location
    }

    pub fn path_editor(&self) -> &str {
        &self.path_editor
    }

    pub fn filter(&self) -> &str {
        &self.filter
    }

    pub fn status(&self) -> &PaneFetchStatus {
        &self.status
    }

    pub fn entries(&self) -> &[BrowserEntry] {
        &self.entries
    }

    pub fn cursor(&self) -> usize {
        self.cursor
    }

    pub fn marked_count(&self) -> usize {
        self.marked.len()
    }

    pub fn is_marked(&self, id: &str) -> bool {
        self.marked.contains(id)
    }

    pub fn set_entries(&mut self, entries: Vec<BrowserEntry>) {
        self.entries = entries;
        self.cursor = self.cursor.min(self.entries.len().saturating_sub(1));
        self.marked
            .retain(|id| self.entries.iter().any(|entry| &entry.id == id));
        self.status = PaneFetchStatus::Loaded;
    }

    pub fn select_next(&mut self) {
        if !self.entries.is_empty() {
            self.cursor = (self.cursor + 1).min(self.entries.len() - 1);
        }
    }

    pub fn select_prev(&mut self) {
        self.cursor = self.cursor.saturating_sub(1);
    }

    pub fn select_first(&mut self) {
        self.cursor = 0;
    }

    pub fn select_last(&mut self) {
        if !self.entries.is_empty() {
            self.cursor = self.entries.len() - 1;
        }
    }

    pub fn selected_entry(&self) -> Option<&BrowserEntry> {
        self.entries.get(self.cursor)
    }

    pub fn toggle_mark(&mut self) {
        let Some(id) = self.selected_entry().map(|entry| entry.id.clone()) else {
            return;
        };
        if !self.marked.remove(&id) {
            self.marked.insert(id);
        }
    }

    pub fn begin_fetch(&mut self) -> Option<PaneFetchRequest> {
        if !matches!(self.status, PaneFetchStatus::Idle) {
            return None;
        }
        if matches!(self.location, Location::Places) {
            return None;
        }
        self.pending_request_id += 1;
        let request_id = self.pending_request_id;
        self.status = PaneFetchStatus::Pending { request_id };
        match &self.location {
            Location::Local(path) => Some(PaneFetchRequest::Local {
                pane_id: self.id,
                request_id,
                path: path.clone(),
            }),
            Location::Places => None,
            Location::Remote(endpoint) => match &endpoint.path {
                RemotePath::Discovery => Some(PaneFetchRequest::RemoteModules {
                    pane_id: self.id,
                    request_id,
                    endpoint: endpoint.clone(),
                }),
                RemotePath::Module { module, rel_path } => Some(PaneFetchRequest::RemoteListing {
                    pane_id: self.id,
                    request_id,
                    endpoint: endpoint.clone(),
                    module: module.clone(),
                    path: blit_core::path_posix::relative_path_to_posix(rel_path),
                }),
                RemotePath::Root { rel_path } => Some(PaneFetchRequest::RemoteListing {
                    pane_id: self.id,
                    request_id,
                    endpoint: endpoint.clone(),
                    module: String::new(),
                    path: blit_core::path_posix::relative_path_to_posix(rel_path),
                }),
            },
        }
    }

    pub fn apply_fetch_result(
        &mut self,
        request_id: u64,
        result: Result<Vec<BrowserEntry>, String>,
    ) {
        match self.status {
            PaneFetchStatus::Pending {
                request_id: current,
            } if current == request_id => {}
            _ => return,
        }
        match result {
            Ok(entries) => self.set_entries(entries),
            Err(message) => self.status = PaneFetchStatus::Error { message },
        }
    }

    pub fn refresh(&mut self) {
        self.entries.clear();
        self.cursor = 0;
        self.marked.clear();
        self.status = PaneFetchStatus::Idle;
    }

    pub fn open_selected(&mut self) -> bool {
        let Some(entry) = self.selected_entry() else {
            return false;
        };
        if let Some(target) = entry.target_location.clone() {
            self.set_location(target);
            return true;
        }
        let next_location = match (&self.location, entry.kind) {
            (Location::Local(current_path), EntryKind::Parent) => current_path
                .parent()
                .map(Path::to_path_buf)
                .map(Location::Local),
            (Location::Local(current_path), EntryKind::Directory) => {
                Some(Location::Local(current_path.join(&entry.name)))
            }
            (Location::Remote(endpoint), EntryKind::Parent) => remote_parent_location(endpoint),
            (Location::Remote(endpoint), EntryKind::Module) => {
                Some(Location::Remote(remote_module_root(endpoint, &entry.name)))
            }
            (Location::Remote(endpoint), EntryKind::Directory) => {
                Some(Location::Remote(remote_child(endpoint, &entry.name)))
            }
            _ => None,
        };
        let Some(next_location) = next_location else {
            return false;
        };
        self.set_location(next_location);
        true
    }

    pub fn ascend(&mut self) -> bool {
        let next = match &self.location {
            Location::Local(current_path) => current_path
                .parent()
                .map(Path::to_path_buf)
                .map(Location::Local),
            Location::Places => None,
            Location::Remote(endpoint) => remote_parent_location(endpoint),
        };
        let Some(next) = next else {
            return false;
        };
        self.set_location(next);
        true
    }

    fn set_location(&mut self, location: Location) {
        self.path_editor = location.display();
        self.location = location;
        self.entries.clear();
        self.cursor = 0;
        self.marked.clear();
        self.status = PaneFetchStatus::Idle;
    }
}

#[derive(Debug, Clone)]
pub struct DualPaneState {
    left: PaneState,
    right: PaneState,
    active: PaneId,
}

impl DualPaneState {
    pub fn new(left: Location, right: Location) -> Self {
        Self {
            left: PaneState::new(PaneId::Left, left),
            right: PaneState::new(PaneId::Right, right),
            active: PaneId::Left,
        }
    }

    pub fn for_launch(launch_dir: PathBuf, remote: Option<RemoteEndpoint>) -> Self {
        let right = remote
            .map(Location::Remote)
            .unwrap_or_else(|| Location::Local(default_secondary_local(&launch_dir)));
        Self::new(Location::Local(launch_dir), right)
    }

    pub fn active(&self) -> PaneId {
        self.active
    }

    pub fn inactive(&self) -> PaneId {
        self.active.other()
    }

    pub fn switch_active(&mut self) {
        self.active = self.active.other();
    }

    pub fn pane(&self, id: PaneId) -> &PaneState {
        match id {
            PaneId::Left => &self.left,
            PaneId::Right => &self.right,
        }
    }

    pub fn pane_mut(&mut self, id: PaneId) -> &mut PaneState {
        match id {
            PaneId::Left => &mut self.left,
            PaneId::Right => &mut self.right,
        }
    }

    pub fn active_pane(&self) -> &PaneState {
        self.pane(self.active)
    }

    pub fn active_pane_mut(&mut self) -> &mut PaneState {
        self.pane_mut(self.active)
    }

    pub fn inactive_pane(&self) -> &PaneState {
        self.pane(self.inactive())
    }

    pub fn action_labels(&self) -> Vec<String> {
        let dest = self.inactive().label();
        vec![
            format!("Copy -> {dest}"),
            format!("Mirror -> {dest}"),
            format!("Move -> {dest}"),
            "Delete".to_string(),
            "Verify".to_string(),
            "More".to_string(),
        ]
    }

    pub fn populate_places_if_needed(&mut self, id: PaneId, places: Vec<Location>) -> bool {
        let pane = self.pane_mut(id);
        if !matches!(pane.location(), Location::Places)
            || !matches!(pane.status(), PaneFetchStatus::Idle)
        {
            return false;
        }
        pane.set_entries(entries_from_places(places));
        true
    }
}

pub fn list_local_entries(path: &Path) -> Result<Vec<BrowserEntry>> {
    let metadata =
        fs::metadata(path).with_context(|| format!("reading metadata for {}", path.display()))?;
    if !metadata.is_dir() {
        return Ok(vec![entry_from_path(path, &metadata)?]);
    }

    let mut entries = Vec::new();
    entries.push(places_entry());
    if path.parent().is_some() {
        entries.push(BrowserEntry::new(
            parent_entry_id(path),
            "..",
            EntryKind::Parent,
        ));
    }

    let mut children: Vec<_> = fs::read_dir(path)
        .with_context(|| format!("reading directory {}", path.display()))?
        .collect::<std::result::Result<Vec<_>, _>>()
        .with_context(|| format!("collecting entries for {}", path.display()))?;
    children.sort_by_cached_key(|entry| entry.file_name());

    for child in children {
        let child_path = child.path();
        let symlink_metadata = fs::symlink_metadata(&child_path)
            .with_context(|| format!("reading metadata for {}", child_path.display()))?;
        entries.push(entry_from_dir_child(&child_path, symlink_metadata));
    }

    Ok(entries)
}

pub fn entries_from_remote_modules(modules: Vec<Module>) -> Vec<BrowserEntry> {
    let mut entries: Vec<_> = vec![places_entry()];
    entries.extend(modules.into_iter().map(|module| {
        let mut entry = BrowserEntry::new(
            format!("module:{}", module.name),
            module.name,
            EntryKind::Module,
        );
        entry.read_only = module.read_only;
        entry
    }));
    sort_browser_entries(&mut entries);
    entries
}

pub fn entries_from_remote_listing(
    endpoint: &RemoteEndpoint,
    entries: Vec<DirEntry>,
) -> Vec<BrowserEntry> {
    let mut out = Vec::new();
    out.push(places_entry());
    if remote_parent_location(endpoint).is_some() {
        out.push(BrowserEntry::new(
            remote_parent_id(endpoint),
            "..",
            EntryKind::Parent,
        ));
    }
    out.extend(entries.into_iter().map(|entry| {
        let kind = if entry.is_dir {
            EntryKind::Directory
        } else {
            EntryKind::File
        };
        let mut row = BrowserEntry::new(format!("remote:{}", entry.name), entry.name, kind);
        row.size = size_for_remote_entry(entry.size, kind);
        row.mtime_seconds = Some(entry.mtime_seconds);
        row
    }));
    sort_browser_entries(&mut out);
    out
}

pub fn entries_from_places(places: Vec<Location>) -> Vec<BrowserEntry> {
    let mut entries: Vec<_> = places
        .into_iter()
        .map(|location| {
            let label = location.display();
            BrowserEntry::new(format!("place:{label}"), label, EntryKind::Place)
                .with_target(location)
        })
        .collect();
    sort_browser_entries(&mut entries);
    entries
}

fn places_entry() -> BrowserEntry {
    BrowserEntry::new("places", "Places", EntryKind::Place).with_target(Location::Places)
}

fn entry_from_path(path: &Path, metadata: &fs::Metadata) -> Result<BrowserEntry> {
    let name = path
        .file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_else(|| display_path(path));
    let kind = kind_from_metadata(metadata);
    let mut entry = BrowserEntry::new(path.to_string_lossy().into_owned(), name, kind);
    entry.size = size_for_kind(metadata, kind);
    entry.mtime_seconds = blit_core::wire_metadata::mtime_seconds(metadata);
    entry.read_only = metadata.permissions().readonly();
    Ok(entry)
}

fn entry_from_dir_child(path: &Path, metadata: fs::Metadata) -> BrowserEntry {
    let name = path
        .file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_else(|| display_path(path));
    let kind = kind_from_metadata(&metadata);
    let mut entry = BrowserEntry::new(path.to_string_lossy().into_owned(), name, kind);
    entry.size = size_for_kind(&metadata, kind);
    entry.mtime_seconds = blit_core::wire_metadata::mtime_seconds(&metadata);
    entry.read_only = metadata.permissions().readonly();
    entry
}

fn sort_browser_entries(entries: &mut [BrowserEntry]) {
    entries.sort_by_cached_key(|entry| {
        (
            sort_priority(entry.kind),
            entry.name.to_lowercase(),
            entry.name.clone(),
        )
    });
}

fn sort_priority(kind: EntryKind) -> u8 {
    match kind {
        EntryKind::Place => 0,
        EntryKind::Parent => 1,
        EntryKind::Module | EntryKind::Directory => 2,
        EntryKind::File | EntryKind::Symlink | EntryKind::Other => 3,
    }
}

fn kind_from_metadata(metadata: &fs::Metadata) -> EntryKind {
    let file_type = metadata.file_type();
    if file_type.is_symlink() {
        EntryKind::Symlink
    } else if file_type.is_dir() {
        EntryKind::Directory
    } else if file_type.is_file() {
        EntryKind::File
    } else {
        EntryKind::Other
    }
}

fn size_for_kind(metadata: &fs::Metadata, kind: EntryKind) -> Option<u64> {
    match kind {
        EntryKind::File | EntryKind::Symlink | EntryKind::Other => Some(metadata.len()),
        EntryKind::Place | EntryKind::Parent | EntryKind::Module | EntryKind::Directory => None,
    }
}

fn size_for_remote_entry(size: u64, kind: EntryKind) -> Option<u64> {
    match kind {
        EntryKind::File | EntryKind::Symlink | EntryKind::Other => Some(size),
        EntryKind::Place | EntryKind::Parent | EntryKind::Module | EntryKind::Directory => None,
    }
}

fn remote_module_root(base: &RemoteEndpoint, module: &str) -> RemoteEndpoint {
    RemoteEndpoint {
        host: base.host.clone(),
        port: base.port,
        path: RemotePath::Module {
            module: module.to_string(),
            rel_path: PathBuf::new(),
        },
    }
}

fn remote_child(base: &RemoteEndpoint, segment: &str) -> RemoteEndpoint {
    let path = match &base.path {
        RemotePath::Discovery => RemotePath::Discovery,
        RemotePath::Module { module, rel_path } => RemotePath::Module {
            module: module.clone(),
            rel_path: rel_path.join(segment),
        },
        RemotePath::Root { rel_path } => RemotePath::Root {
            rel_path: rel_path.join(segment),
        },
    };
    RemoteEndpoint {
        host: base.host.clone(),
        port: base.port,
        path,
    }
}

fn remote_parent_location(endpoint: &RemoteEndpoint) -> Option<Location> {
    let path = match &endpoint.path {
        RemotePath::Discovery => return None,
        RemotePath::Module { module, rel_path } if rel_path.as_os_str().is_empty() => {
            RemotePath::Discovery
        }
        RemotePath::Module { module, rel_path } => {
            let mut parent = rel_path.clone();
            parent.pop();
            RemotePath::Module {
                module: module.clone(),
                rel_path: parent,
            }
        }
        RemotePath::Root { rel_path } if rel_path.as_os_str().is_empty() => RemotePath::Discovery,
        RemotePath::Root { rel_path } => {
            let mut parent = rel_path.clone();
            parent.pop();
            RemotePath::Root { rel_path: parent }
        }
    };
    Some(Location::Remote(RemoteEndpoint {
        host: endpoint.host.clone(),
        port: endpoint.port,
        path,
    }))
}

fn remote_parent_id(endpoint: &RemoteEndpoint) -> String {
    remote_parent_location(endpoint)
        .map(|location| location.display())
        .unwrap_or_else(|| "..".to_string())
}

fn parent_entry_id(path: &Path) -> String {
    path.parent()
        .map(|parent| parent.to_string_lossy().into_owned())
        .unwrap_or_else(|| "..".to_string())
}

fn default_secondary_local(launch_dir: &Path) -> PathBuf {
    let home = std::env::var_os("HOME").map(PathBuf::from);
    if let Some(home) = home {
        if home != launch_dir {
            return home;
        }
    }
    launch_dir
        .parent()
        .map(Path::to_path_buf)
        .unwrap_or_else(|| PathBuf::from(std::path::MAIN_SEPARATOR.to_string()))
}

fn display_path(path: &Path) -> String {
    if path.as_os_str().is_empty() {
        ".".to_string()
    } else {
        path.display().to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn entry(name: &str) -> BrowserEntry {
        BrowserEntry::new(name, name, EntryKind::Directory)
    }

    fn remote(raw: &str) -> RemoteEndpoint {
        RemoteEndpoint::parse(raw).expect("remote")
    }

    #[test]
    fn active_and_inactive_panes_are_directional() {
        let mut state = DualPaneState::new(Location::local("/src"), Location::local("/dst"));
        assert_eq!(state.active(), PaneId::Left);
        assert_eq!(state.inactive(), PaneId::Right);
        assert_eq!(state.action_labels()[0], "Copy -> Right");

        state.switch_active();

        assert_eq!(state.active(), PaneId::Right);
        assert_eq!(state.inactive(), PaneId::Left);
        assert_eq!(state.action_labels()[0], "Copy -> Left");
    }

    #[test]
    fn pane_cursor_and_marks_follow_current_entries() {
        let mut pane = PaneState::new(PaneId::Left, Location::local("/src"));
        pane.set_entries(vec![entry("a"), entry("b")]);

        pane.select_next();
        pane.toggle_mark();

        assert_eq!(pane.cursor(), 1);
        assert!(pane.is_marked("b"));

        pane.set_entries(vec![entry("a")]);

        assert_eq!(pane.cursor(), 0);
        assert_eq!(pane.marked_count(), 0);
    }

    #[test]
    fn local_listing_includes_parent_dirs_and_files() {
        let temp = tempfile::tempdir().expect("tempdir");
        fs::create_dir(temp.path().join("folder")).expect("mkdir");
        fs::write(temp.path().join("note.txt"), b"hello").expect("write");

        let entries = list_local_entries(temp.path()).expect("list");

        assert_eq!(entries[0].name, "Places");
        assert_eq!(entries[0].kind, EntryKind::Place);
        assert!(entries
            .iter()
            .any(|entry| entry.name == ".." && entry.kind == EntryKind::Parent));
        assert!(entries
            .iter()
            .any(|entry| entry.name == "folder" && entry.kind == EntryKind::Directory));
        assert!(entries.iter().any(|entry| {
            entry.name == "note.txt" && entry.kind == EntryKind::File && entry.size == Some(5)
        }));
    }

    #[test]
    fn pane_navigation_opens_directory_and_ascends() {
        let temp = tempfile::tempdir().expect("tempdir");
        let child = temp.path().join("child");
        fs::create_dir(&child).expect("mkdir");
        let mut pane = PaneState::new(PaneId::Left, Location::local(temp.path()));
        pane.set_entries(list_local_entries(temp.path()).expect("list"));

        while pane
            .selected_entry()
            .is_some_and(|entry| entry.name != "child")
        {
            pane.select_next();
        }

        assert!(pane.open_selected());
        assert_eq!(pane.location().local_path(), Some(child.as_path()));

        assert!(pane.ascend());
        assert_eq!(pane.location().local_path(), Some(temp.path()));
    }

    #[test]
    fn places_row_opens_places_location() {
        let temp = tempfile::tempdir().expect("tempdir");
        let mut pane = PaneState::new(PaneId::Left, Location::local(temp.path()));
        pane.set_entries(list_local_entries(temp.path()).expect("list"));

        assert_eq!(
            pane.selected_entry().map(|entry| entry.name.as_str()),
            Some("Places")
        );
        assert!(pane.open_selected());
        assert!(matches!(pane.location(), Location::Places));
    }

    #[test]
    fn local_fetch_generation_drops_stale_replies() {
        let temp = tempfile::tempdir().expect("tempdir");
        let mut pane = PaneState::new(PaneId::Left, Location::local(temp.path()));

        let PaneFetchRequest::Local { request_id, .. } = pane.begin_fetch().expect("begin") else {
            panic!("expected local request");
        };
        pane.apply_fetch_result(request_id + 1, Ok(vec![entry("stale")]));
        assert!(matches!(pane.status(), PaneFetchStatus::Pending { .. }));

        pane.apply_fetch_result(request_id, Ok(vec![entry("current")]));
        assert!(matches!(pane.status(), PaneFetchStatus::Loaded));
        assert_eq!(pane.entries()[0].name, "current");
    }

    #[test]
    fn remote_discovery_lists_modules_and_enters_module_root() {
        let mut pane = PaneState::new(PaneId::Right, Location::Remote(remote("nas")));

        let request = pane.begin_fetch().expect("request");
        assert!(matches!(
            request,
            PaneFetchRequest::RemoteModules {
                pane_id: PaneId::Right,
                ..
            }
        ));

        pane.apply_fetch_result(
            1,
            Ok(entries_from_remote_modules(vec![Module {
                name: "backup".to_string(),
                path: "/srv/backup".to_string(),
                read_only: false,
            }])),
        );

        pane.select_next();
        assert_eq!(
            pane.selected_entry().map(|entry| entry.kind),
            Some(EntryKind::Module)
        );
        assert!(pane.open_selected());
        assert_eq!(pane.location().display(), "nas:/backup/");
    }

    #[test]
    fn remote_module_listing_descends_and_ascends() {
        let mut pane = PaneState::new(PaneId::Right, Location::Remote(remote("nas:/backup/")));
        pane.set_entries(entries_from_remote_listing(
            pane.location().remote_endpoint().expect("endpoint"),
            vec![DirEntry {
                name: "photos".to_string(),
                is_dir: true,
                size: 0,
                mtime_seconds: 10,
            }],
        ));

        while pane
            .selected_entry()
            .is_some_and(|entry| entry.name != "photos")
        {
            pane.select_next();
        }
        assert_eq!(
            pane.selected_entry().map(|entry| entry.name.as_str()),
            Some("photos")
        );

        assert!(pane.open_selected());
        assert_eq!(pane.location().display(), "nas:/backup/photos");

        let PaneFetchRequest::RemoteListing { module, path, .. } =
            pane.begin_fetch().expect("request")
        else {
            panic!("expected remote listing request");
        };
        assert_eq!(module, "backup");
        assert_eq!(path, "photos");

        assert!(pane.ascend());
        assert_eq!(pane.location().display(), "nas:/backup/");
    }

    #[test]
    fn places_location_populates_jump_targets() {
        let mut state = DualPaneState::new(Location::Places, Location::local("/dst"));
        assert!(state.populate_places_if_needed(
            PaneId::Left,
            vec![Location::local("/tmp"), Location::Remote(remote("nas"))],
        ));

        let names: Vec<_> = state
            .pane(PaneId::Left)
            .entries()
            .iter()
            .map(|entry| entry.name.as_str())
            .collect();
        assert!(names.contains(&"Local /tmp"));
        assert!(names.contains(&"nas"));
    }
}
