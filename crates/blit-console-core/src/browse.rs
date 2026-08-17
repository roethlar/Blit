//! Browse model: list one directory on one endpoint through the
//! typed blit-app admin APIs — the same call paths `blit ls` and
//! `blit list-modules` use (`crates/blit-cli/src/ls.rs`). No stdout
//! parsing, no spawning.
//!
//! Local browsing goes through `admin::ls::list_local`. Daemon
//! browsing mirrors the CLI's smart-dispatch: the daemon root (`/`)
//! lists the daemon's exported modules (`admin::list_modules::query`,
//! the bare-host resolution), and anything below root lists within
//! one module (`admin::ls::list_remote`) — see
//! [`resolve_daemon_target`].

use crate::endpoint::{DaemonEndpoint, Endpoint};
use blit_core::admin::list_modules::{self, Module};
use blit_core::admin::ls::{self, DirEntry};
use blit_core::remote::endpoint::RemoteEndpoint;
use std::fmt;
use std::path::{Component, Path, PathBuf};

/// A directory listing for one path on one endpoint. `entries`
/// reuses blit-app's `DirEntry` (name, is_dir, size, mtime_seconds)
/// so every face sees exactly what `blit ls --json` sees.
#[derive(Debug, Clone)]
pub struct Listing {
    pub path: PathBuf,
    pub entries: Vec<DirEntry>,
}

/// Why a browse failed.
#[derive(Debug)]
pub enum BrowseError {
    /// The daemon's stored address does not parse as a remote
    /// endpoint — a model bug or corrupt input, never a daemon fault.
    InvalidDaemonAddress { address: String, reason: String },
    /// The path cannot be resolved against a daemon endpoint (see
    /// [`resolve_daemon_target`] for the accepted shapes).
    UnresolvablePath { path: PathBuf, reason: String },
    /// The listing call itself failed (missing path, permission,
    /// unreachable daemon, …).
    Failed { path: PathBuf, reason: String },
}

impl fmt::Display for BrowseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            BrowseError::InvalidDaemonAddress { address, reason } => {
                write!(f, "invalid daemon address {address}: {reason}")
            }
            BrowseError::UnresolvablePath { path, reason } => {
                write!(f, "cannot browse {} on a daemon: {reason}", path.display())
            }
            BrowseError::Failed { path, reason } => {
                write!(f, "failed to list {}: {reason}", path.display())
            }
        }
    }
}

impl std::error::Error for BrowseError {}

/// Where a daemon-side path points, mirroring the CLI's
/// smart-dispatch (`blit ls <bare-host>` lists modules; a
/// `module/path` target lists entries).
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum DaemonTarget {
    /// The daemon root: list its exported modules.
    Modules,
    /// Inside one exported module; `path` is the module-relative
    /// path (`""` for the module root, matching the CLI's
    /// `server:/module/` form).
    Listing { module: String, path: String },
}

/// Resolve a console path against a daemon endpoint. Daemon paths
/// are virtual and slash-rooted: `/` (or an empty path) is the
/// daemon's module list, `/module` is a module's root, and
/// `/module/sub/dir` is a directory inside that module — the first
/// component is always the module name, everything after it the
/// module-relative path handed to the `List` RPC.
pub(crate) fn resolve_daemon_target(path: &Path) -> Result<DaemonTarget, BrowseError> {
    let mut components = path.components().filter_map(|component| match component {
        Component::Normal(part) => Some(part.to_string_lossy().into_owned()),
        // RootDir / CurDir carry no path information; a Windows
        // prefix cannot appear in a daemon path.
        _ => None,
    });
    let Some(module) = components.next() else {
        return Ok(DaemonTarget::Modules);
    };
    let rel_path = components.collect::<Vec<_>>().join("/");
    if module.is_empty() {
        return Err(BrowseError::UnresolvablePath {
            path: path.to_path_buf(),
            reason: "the first path component must be a module name".to_string(),
        });
    }
    Ok(DaemonTarget::Listing {
        module,
        path: rel_path,
    })
}

/// Render a daemon's module list as browse entries: every module is
/// a directory the operator can descend into. Sizes and mtimes are
/// module-level unknowns, so they are zero — the same shape the
/// CLI's `list-modules` text output implies (`name (ro|rw) path`).
pub(crate) fn modules_to_entries(modules: Vec<Module>) -> Vec<DirEntry> {
    modules
        .into_iter()
        .map(|module| DirEntry {
            name: module.name,
            is_dir: true,
            size: 0,
            mtime_seconds: 0,
        })
        .collect()
}

/// List `path` on `endpoint`. Local listings reuse
/// `blit_core::admin::ls::list_local` on a blocking thread (matching
/// the CLI's local branch without blocking the caller's runtime);
/// daemon listings go over gRPC per [`resolve_daemon_target`].
pub async fn browse(endpoint: &Endpoint, path: &Path) -> Result<Listing, BrowseError> {
    match endpoint {
        Endpoint::Local => browse_local(path).await,
        Endpoint::Daemon(daemon) => browse_daemon(daemon, path).await,
    }
}

async fn browse_local(path: &Path) -> Result<Listing, BrowseError> {
    let owned = path.to_path_buf();
    let listed = tokio::task::spawn_blocking(move || ls::list_local(&owned))
        .await
        .map_err(|join| BrowseError::Failed {
            path: path.to_path_buf(),
            reason: format!("local listing task failed: {join}"),
        })?
        .map_err(|err| BrowseError::Failed {
            path: path.to_path_buf(),
            reason: err.to_string(),
        })?;
    Ok(Listing {
        path: path.to_path_buf(),
        entries: listed.into_entries(),
    })
}

async fn browse_daemon(daemon: &DaemonEndpoint, path: &Path) -> Result<Listing, BrowseError> {
    let remote = RemoteEndpoint::parse(&daemon.address).map_err(|err| {
        BrowseError::InvalidDaemonAddress {
            address: daemon.address.clone(),
            reason: err.to_string(),
        }
    })?;
    match resolve_daemon_target(path)? {
        DaemonTarget::Modules => {
            let modules =
                list_modules::query(&remote)
                    .await
                    .map_err(|err| BrowseError::Failed {
                        path: path.to_path_buf(),
                        reason: err.to_string(),
                    })?;
            Ok(Listing {
                path: path.to_path_buf(),
                entries: modules_to_entries(modules),
            })
        }
        DaemonTarget::Listing { module, path: rel } => {
            let entries =
                ls::list_remote(&remote, module, rel)
                    .await
                    .map_err(|err| BrowseError::Failed {
                        path: path.to_path_buf(),
                        reason: err.to_string(),
                    })?;
            Ok(Listing {
                path: path.to_path_buf(),
                entries,
            })
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn local_directory_lists_entries() {
        let tmp = tempfile::tempdir().unwrap();
        std::fs::write(tmp.path().join("file.txt"), b"hello").unwrap();
        std::fs::create_dir(tmp.path().join("subdir")).unwrap();

        let listing = browse(&Endpoint::Local, tmp.path()).await.unwrap();
        assert_eq!(listing.path, tmp.path());
        assert_eq!(listing.entries.len(), 2);
        // blit-app sorts by path: file.txt before subdir.
        let file = &listing.entries[0];
        assert_eq!(file.name, "file.txt");
        assert!(!file.is_dir);
        assert_eq!(file.size, 5);
        let dir = &listing.entries[1];
        assert_eq!(dir.name, "subdir");
        assert!(dir.is_dir);
    }

    #[tokio::test]
    async fn local_single_file_returns_target_entry() {
        let tmp = tempfile::tempdir().unwrap();
        let file_path = tmp.path().join("one.bin");
        std::fs::write(&file_path, b"xyz").unwrap();

        let listing = browse(&Endpoint::Local, &file_path).await.unwrap();
        assert_eq!(listing.entries.len(), 1);
        assert_eq!(listing.entries[0].name, "one.bin");
        assert!(!listing.entries[0].is_dir);
        assert_eq!(listing.entries[0].size, 3);
    }

    #[tokio::test]
    async fn local_missing_path_fails() {
        let tmp = tempfile::tempdir().unwrap();
        let err = browse(&Endpoint::Local, &tmp.path().join("nope"))
            .await
            .unwrap_err();
        match err {
            BrowseError::Failed { path, .. } => assert!(path.ends_with("nope")),
            other => panic!("expected Failed, got {other:?}"),
        }
    }

    #[test]
    fn daemon_root_resolves_to_modules() {
        assert_eq!(
            resolve_daemon_target(Path::new("/")).unwrap(),
            DaemonTarget::Modules
        );
        assert_eq!(
            resolve_daemon_target(Path::new("")).unwrap(),
            DaemonTarget::Modules
        );
    }

    #[test]
    fn daemon_first_component_is_the_module() {
        assert_eq!(
            resolve_daemon_target(Path::new("/media")).unwrap(),
            DaemonTarget::Listing {
                module: "media".to_string(),
                path: String::new(),
            }
        );
        assert_eq!(
            resolve_daemon_target(Path::new("/media/films/classics")).unwrap(),
            DaemonTarget::Listing {
                module: "media".to_string(),
                path: "films/classics".to_string(),
            }
        );
    }

    #[test]
    fn daemon_resolution_tolerates_relative_paths() {
        // The model always navigates from "/", but a face handing a
        // relative path gets the same first-component-is-module rule
        // rather than a panic.
        assert_eq!(
            resolve_daemon_target(Path::new("media/films")).unwrap(),
            DaemonTarget::Listing {
                module: "media".to_string(),
                path: "films".to_string(),
            }
        );
    }

    #[test]
    fn modules_become_directory_entries() {
        let entries = modules_to_entries(vec![
            Module {
                name: "home".to_string(),
                path: "/srv/home".to_string(),
                read_only: true,
            },
            Module {
                name: "media".to_string(),
                path: "/srv/media".to_string(),
                read_only: false,
            },
        ]);
        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].name, "home");
        assert!(entries[0].is_dir);
        assert_eq!(entries[0].size, 0);
        assert_eq!(entries[0].mtime_seconds, 0);
        assert_eq!(entries[1].name, "media");
        assert!(entries[1].is_dir);
    }

    #[tokio::test]
    async fn daemon_with_unparseable_address_fails_before_any_network() {
        // Deterministic: `":9031"` has no host, so `RemoteEndpoint::parse`
        // bails before any DNS or socket is touched (a merely odd
        // hostname would parse and reach the resolver — cr-ls1-14's
        // lesson). The error variant proves which stage rejected it.
        let daemon = Endpoint::Daemon(DaemonEndpoint {
            address: ":9031".to_string(),
            name: "broken".to_string(),
        });
        let err = browse(&daemon, Path::new("/")).await.unwrap_err();
        match err {
            BrowseError::InvalidDaemonAddress { address, .. } => {
                assert_eq!(address, ":9031");
            }
            other => panic!("expected InvalidDaemonAddress, got {other:?}"),
        }
    }
}
