//! Host glue for the Blit Console GUI: owns a [`Model`], executes
//! [`Effect`]s, and feeds completions back as [`Msg`]s. The eframe
//! view in the `blit-gui` binary is a thin renderer over this type —
//! no product logic lives there.
//!
//! Browse and discovery are async (they talk to the filesystem or
//! the LAN). Completions arrive on an internal channel; the face
//! calls [`Session::poll`] once per frame.

use blit_console_core::{browse, discover_daemons, update, Effect, Endpoint, Model, Msg};
use std::path::{Path, PathBuf};
use std::sync::mpsc::{self, Receiver, Sender};
use std::sync::Arc;

/// Walkable starting path for the local filesystem endpoint.
///
/// The core model seeds every fresh pane at `/`. That is the daemon
/// module-list root and a real Unix directory; on Windows it is not a
/// useful local folder, so the host opens the user profile instead
/// (see [`resolve_browse_path`]).
pub fn local_start_path() -> PathBuf {
    #[cfg(windows)]
    {
        std::env::var_os("USERPROFILE")
            .map(PathBuf::from)
            .or_else(|| std::env::current_dir().ok())
            .filter(|path| !is_placeholder_root(path))
            .unwrap_or_else(|| PathBuf::from("/"))
    }
    #[cfg(not(windows))]
    {
        PathBuf::from("/")
    }
}

/// True for the core's placeholder root (`/`, `\`, or empty) — not
/// a Windows drive root such as `C:\`.
pub fn is_placeholder_root(path: &Path) -> bool {
    path.as_os_str().is_empty() || path == Path::new("/") || path == Path::new("\\")
}

/// Path the host actually lists for an [`Effect::Browse`].
///
/// Daemon `/` stays `/` (module list). Local `/` becomes
/// [`local_start_path`] so a Windows operator is not dropped into
/// an unreadable placeholder.
pub fn resolve_browse_path(endpoint: &Endpoint, path: PathBuf) -> PathBuf {
    match endpoint {
        Endpoint::Local if is_placeholder_root(&path) => local_start_path(),
        _ => path,
    }
}

/// Parent directory for the Up control. `None` at a placeholder
/// root so the face can hide the button.
pub fn parent_path(path: &Path) -> Option<PathBuf> {
    let parent = path.parent()?;
    if parent.as_os_str().is_empty() {
        return None;
    }
    Some(parent.to_path_buf())
}

/// Child path when the operator opens a directory entry.
pub fn enter_path(current: &Path, name: &str) -> PathBuf {
    if is_placeholder_root(current) {
        return PathBuf::from("/").join(name);
    }
    current.join(name)
}

/// Listing rows the face may turn into click targets.
///
/// Core keeps the previous listing visible while a newer browse is
/// in flight (`loading` is true, `current_path` already moved).
/// Clicking those stale names would join them onto the new path
/// (`/photos` + leftover `photos/` → `/photos/photos`). The face
/// therefore treats an in-flight browse as having no interactive
/// rows.
pub fn interactive_listing(model: &Model) -> &[blit_console_core::DirEntry] {
    if model.is_loading() {
        &[]
    } else {
        model.listing()
    }
}

/// Console session: model plus the effect runtime the face drives.
pub struct Session {
    model: Model,
    tx: Sender<Msg>,
    rx: Receiver<Msg>,
    handle: tokio::runtime::Handle,
    /// Held when this session created the runtime, so spawned
    /// browse/discover tasks outlive the constructing stack frame.
    _runtime: Option<tokio::runtime::Runtime>,
    wake: Option<Arc<dyn Fn() + Send + Sync>>,
}

impl Session {
    /// Build a session on the current Tokio runtime, or create a
    /// multi-thread runtime when the caller is not already inside one
    /// (the eframe entry point).
    pub fn new() -> Self {
        match tokio::runtime::Handle::try_current() {
            Ok(handle) => Self::with_handle(handle, None),
            Err(_) => {
                let runtime = tokio::runtime::Builder::new_multi_thread()
                    .enable_all()
                    .build()
                    .expect("tokio runtime for blit-gui");
                let handle = runtime.handle().clone();
                Self::with_handle(handle, Some(runtime))
            }
        }
    }

    fn with_handle(
        handle: tokio::runtime::Handle,
        runtime: Option<tokio::runtime::Runtime>,
    ) -> Self {
        let (tx, rx) = mpsc::channel();
        Self {
            model: Model::new(),
            tx,
            rx,
            handle,
            _runtime: runtime,
            wake: None,
        }
    }

    /// Ask the face to redraw when a background effect completes.
    pub fn set_wake(&mut self, wake: Arc<dyn Fn() + Send + Sync>) {
        self.wake = Some(wake);
    }

    /// Open the local start path and scan the LAN for daemons.
    pub fn bootstrap(&mut self) {
        self.dispatch(Msg::NavigateTo(local_start_path()));
        self.dispatch(Msg::RefreshDiscovery);
    }

    pub fn model(&self) -> &Model {
        &self.model
    }

    /// Apply one message and spawn any effects it produces.
    pub fn dispatch(&mut self, msg: Msg) {
        let effects = update(&mut self.model, msg);
        for effect in effects {
            self.spawn(effect);
        }
    }

    /// Drain completed effects. Returns true if any message landed.
    pub fn poll(&mut self) -> bool {
        let mut progressed = false;
        while let Ok(msg) = self.rx.try_recv() {
            progressed = true;
            self.dispatch(msg);
        }
        progressed
    }

    fn spawn(&self, effect: Effect) {
        let tx = self.tx.clone();
        let wake = self.wake.clone();
        match effect {
            Effect::Browse {
                endpoint,
                path,
                generation,
            } => {
                let Some(ep) = self.model.endpoint(endpoint).cloned() else {
                    let _ = tx.send(Msg::ListingFailed {
                        generation,
                        path,
                        error: format!("unknown endpoint {}", endpoint.0),
                    });
                    if let Some(wake) = wake {
                        wake();
                    }
                    return;
                };
                let path = resolve_browse_path(&ep, path);
                self.handle.spawn(async move {
                    let msg = match browse(&ep, &path).await {
                        Ok(listing) => Msg::ListingLoaded {
                            generation,
                            path: listing.path,
                            entries: listing.entries,
                        },
                        Err(err) => Msg::ListingFailed {
                            generation,
                            path,
                            error: err.to_string(),
                        },
                    };
                    let _ = tx.send(msg);
                    if let Some(wake) = wake {
                        wake();
                    }
                });
            }
            Effect::Discover {
                timeout,
                generation,
            } => {
                self.handle.spawn(async move {
                    let msg = match discover_daemons(timeout).await {
                        Ok(endpoints) => Msg::DiscoveryLoaded {
                            generation,
                            endpoints,
                        },
                        Err(err) => Msg::DiscoveryFailed {
                            generation,
                            error: err.to_string(),
                        },
                    };
                    let _ = tx.send(msg);
                    if let Some(wake) = wake {
                        wake();
                    }
                });
            }
        }
    }
}

impl Default for Session {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use blit_console_core::{Endpoint, EndpointId};
    use std::time::{Duration, Instant};

    async fn wait_idle(session: &mut Session) {
        let start = Instant::now();
        loop {
            session.poll();
            if !session.model().is_loading() && !session.model().is_discovering() {
                return;
            }
            if start.elapsed() > Duration::from_secs(5) {
                panic!(
                    "session still busy after 5s (loading={} discovering={} err={:?})",
                    session.model().is_loading(),
                    session.model().is_discovering(),
                    session.model().last_error()
                );
            }
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    }

    #[test]
    fn placeholder_root_matches_core_seed_only() {
        assert!(is_placeholder_root(Path::new("/")));
        assert!(is_placeholder_root(Path::new("\\")));
        assert!(is_placeholder_root(Path::new("")));
        assert!(!is_placeholder_root(Path::new("/tmp")));
        assert!(!is_placeholder_root(Path::new("C:\\")));
        assert!(!is_placeholder_root(Path::new("C:\\Users")));
    }

    #[test]
    fn daemon_root_is_not_rewritten() {
        let daemon = Endpoint::Daemon(blit_console_core::DaemonEndpoint {
            address: "magneto:9833".to_string(),
            name: "magneto".to_string(),
        });
        assert_eq!(
            resolve_browse_path(&daemon, PathBuf::from("/")),
            PathBuf::from("/")
        );
    }

    #[test]
    fn local_placeholder_resolves_to_start_path() {
        let resolved = resolve_browse_path(&Endpoint::Local, PathBuf::from("/"));
        assert_eq!(resolved, local_start_path());
    }

    #[test]
    fn local_real_path_is_not_rewritten() {
        let path = PathBuf::from("/tmp");
        assert_eq!(resolve_browse_path(&Endpoint::Local, path.clone()), path);
    }

    #[test]
    fn parent_of_placeholder_is_none() {
        assert_eq!(parent_path(Path::new("/")), None);
        assert_eq!(parent_path(Path::new("")), None);
    }

    #[test]
    fn parent_of_child_is_root() {
        let parent = parent_path(Path::new("/tmp")).expect("tmp has a parent");
        assert!(is_placeholder_root(&parent));
    }

    #[test]
    fn enter_from_root_joins_name() {
        let child = enter_path(Path::new("/"), "tmp");
        assert_eq!(child.file_name().unwrap(), "tmp");
        assert!(parent_path(&child).is_some_and(|parent| is_placeholder_root(&parent)));
    }

    #[test]
    fn enter_from_directory_joins_name() {
        let child = enter_path(Path::new("/media"), "films");
        assert_eq!(child.file_name().unwrap(), "films");
        assert_eq!(
            child.parent().and_then(|parent| parent.file_name()),
            Some(std::ffi::OsStr::new("media"))
        );
    }

    #[tokio::test]
    async fn new_session_has_local_selected_and_is_idle() {
        let session = Session::new();
        assert_eq!(session.model().selected(), Some(EndpointId(0)));
        assert!(matches!(
            session.model().endpoint(EndpointId(0)),
            Some(Endpoint::Local)
        ));
        assert!(!session.model().is_loading());
        assert!(session.model().listing().is_empty());
    }

    #[tokio::test]
    async fn navigate_to_temp_dir_loads_entries() {
        let tmp = tempfile::tempdir().unwrap();
        std::fs::write(tmp.path().join("file.txt"), b"hello").unwrap();
        std::fs::create_dir(tmp.path().join("subdir")).unwrap();

        let mut session = Session::new();
        session.dispatch(Msg::NavigateTo(tmp.path().to_path_buf()));
        wait_idle(&mut session).await;

        assert!(!session.model().is_loading());
        assert_eq!(session.model().last_error(), None);
        assert_eq!(session.model().current_path(), tmp.path());
        let names: Vec<&str> = session
            .model()
            .listing()
            .iter()
            .map(|entry| entry.name.as_str())
            .collect();
        assert!(names.contains(&"file.txt"), "got {names:?}");
        assert!(names.contains(&"subdir"), "got {names:?}");
    }

    #[tokio::test]
    async fn later_navigate_drops_stale_listing() {
        let first = tempfile::tempdir().unwrap();
        std::fs::write(first.path().join("first.txt"), b"a").unwrap();
        let second = tempfile::tempdir().unwrap();
        std::fs::write(second.path().join("second.txt"), b"b").unwrap();

        let mut session = Session::new();
        session.dispatch(Msg::NavigateTo(first.path().to_path_buf()));
        session.dispatch(Msg::NavigateTo(second.path().to_path_buf()));
        wait_idle(&mut session).await;

        assert_eq!(session.model().current_path(), second.path());
        let names: Vec<&str> = session
            .model()
            .listing()
            .iter()
            .map(|entry| entry.name.as_str())
            .collect();
        assert_eq!(names, vec!["second.txt"]);
    }

    #[tokio::test]
    async fn in_flight_browse_has_no_interactive_rows() {
        let first = tempfile::tempdir().unwrap();
        std::fs::write(first.path().join("photos"), b"x").unwrap();
        std::fs::create_dir(first.path().join("keep")).unwrap();
        let second = tempfile::tempdir().unwrap();

        let mut session = Session::new();
        session.dispatch(Msg::NavigateTo(first.path().to_path_buf()));
        wait_idle(&mut session).await;
        assert!(
            !interactive_listing(session.model()).is_empty(),
            "settled listing should be clickable"
        );

        session.dispatch(Msg::NavigateTo(second.path().to_path_buf()));
        assert!(session.model().is_loading());
        assert!(
            !session.model().listing().is_empty(),
            "core keeps the stale listing while loading"
        );
        assert!(
            interactive_listing(session.model()).is_empty(),
            "stale rows must not stay clickable after current_path moves"
        );
    }

    #[tokio::test]
    async fn missing_path_surfaces_error() {
        let tmp = tempfile::tempdir().unwrap();
        let missing = tmp.path().join("nope");
        let mut session = Session::new();
        session.dispatch(Msg::NavigateTo(missing.clone()));
        wait_idle(&mut session).await;

        assert!(!session.model().is_loading());
        assert!(session.model().listing().is_empty());
        let err = session.model().last_error().expect("error surface");
        assert!(
            err.contains("nope") || err.contains("failed to list"),
            "unexpected error: {err}"
        );
    }
}
