//! Browse model: list one directory on one endpoint through the
//! typed blit-app admin API — the same call path `blit ls` uses
//! (`crates/blit-cli/src/ls.rs`). No stdout parsing, no spawning.
//!
//! Local browsing is implemented in this slice; daemon browsing is
//! stubbed with a typed [`BrowseError::Unsupported`] so the seam is
//! visible to the later discovery/daemon slice.

use crate::endpoint::Endpoint;
use blit_app::admin::ls::{self, DirEntry};
use std::fmt;
use std::path::{Path, PathBuf};

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
    /// The endpoint kind cannot be browsed yet. Daemon browse lands
    /// with the discovery slice; this variant is the visible seam.
    Unsupported { endpoint: String },
    /// The local listing call itself failed (missing path,
    /// permission, …).
    Failed { path: PathBuf, reason: String },
}

impl fmt::Display for BrowseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            BrowseError::Unsupported { endpoint } => {
                write!(f, "browsing {endpoint} is not supported yet")
            }
            BrowseError::Failed { path, reason } => {
                write!(f, "failed to list {}: {reason}", path.display())
            }
        }
    }
}

impl std::error::Error for BrowseError {}

/// List `path` on `endpoint`. For the local endpoint this is a
/// synchronous call into `blit_app::admin::ls::list_local`, matching
/// the CLI's local branch; daemon endpoints return
/// [`BrowseError::Unsupported`].
pub fn browse(endpoint: &Endpoint, path: &Path) -> Result<Listing, BrowseError> {
    match endpoint {
        Endpoint::Local => {
            let listing = ls::list_local(path).map_err(|err| BrowseError::Failed {
                path: path.to_path_buf(),
                reason: err.to_string(),
            })?;
            Ok(Listing {
                path: path.to_path_buf(),
                entries: listing.into_entries(),
            })
        }
        Endpoint::Daemon(daemon) => Err(BrowseError::Unsupported {
            endpoint: format!("{} ({})", daemon.name, daemon.address),
        }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::endpoint::DaemonEndpoint;

    #[test]
    fn local_directory_lists_entries() {
        let tmp = tempfile::tempdir().unwrap();
        std::fs::write(tmp.path().join("file.txt"), b"hello").unwrap();
        std::fs::create_dir(tmp.path().join("subdir")).unwrap();

        let listing = browse(&Endpoint::Local, tmp.path()).unwrap();
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

    #[test]
    fn local_single_file_returns_target_entry() {
        let tmp = tempfile::tempdir().unwrap();
        let file_path = tmp.path().join("one.bin");
        std::fs::write(&file_path, b"xyz").unwrap();

        let listing = browse(&Endpoint::Local, &file_path).unwrap();
        assert_eq!(listing.entries.len(), 1);
        assert_eq!(listing.entries[0].name, "one.bin");
        assert!(!listing.entries[0].is_dir);
        assert_eq!(listing.entries[0].size, 3);
    }

    #[test]
    fn local_missing_path_fails() {
        let tmp = tempfile::tempdir().unwrap();
        let err = browse(&Endpoint::Local, &tmp.path().join("nope")).unwrap_err();
        match err {
            BrowseError::Failed { path, .. } => assert!(path.ends_with("nope")),
            other => panic!("expected Failed, got {other:?}"),
        }
    }

    #[test]
    fn daemon_browse_is_typed_unsupported() {
        let daemon = Endpoint::Daemon(DaemonEndpoint {
            address: "magneto:9833".to_string(),
            name: "magneto".to_string(),
        });
        let err = browse(&daemon, Path::new("/")).unwrap_err();
        match err {
            BrowseError::Unsupported { endpoint } => {
                assert!(endpoint.contains("magneto"), "{endpoint}");
            }
            other => panic!("expected Unsupported, got {other:?}"),
        }
    }
}
