//! Platform-specific filesystem capability abstraction
//!
//! Provides trait-based interface for metadata preservation, symlinks,
//! sparse files, and fast copy operations across macOS/Linux/Windows.
//!
//! Filesystem type is detected at runtime via `statfs` (Unix) or volume
//! queries (Windows), and capabilities are tailored to the actual FS.
//! Results are cached per device to avoid redundant probes.

use eyre::Result;
use std::collections::HashMap;
use std::path::Path;
use std::sync::{Mutex, OnceLock};

#[cfg(target_os = "macos")]
mod macos;
mod probe;
#[cfg(all(unix, not(target_os = "macos")))]
mod unix;
#[cfg(windows)]
mod windows;

#[cfg(windows)]
pub(crate) use windows::{mark_block_clone_unsupported, supports_block_clone_same_volume};

#[cfg(target_os = "macos")]
pub use macos::MacOSCapability as PlatformCapability;
#[cfg(all(unix, not(target_os = "macos")))]
pub use unix::UnixCapability as PlatformCapability;
#[cfg(windows)]
pub use windows::WindowsCapability as PlatformCapability;

pub use probe::{detect_filesystem_type, probe_capabilities};

/// Platform-specific filesystem operations
pub trait FilesystemCapability {
    /// Preserve metadata from src to dst
    fn preserve_metadata(&self, src: &Path, dst: &Path) -> Result<MetadataPreserved>;

    /// Get platform capabilities at runtime
    fn capabilities(&self) -> &Capabilities;

    /// Fast copy using OS-specific primitives
    fn fast_copy(&self, src: &Path, dst: &Path) -> Result<FastCopyResult>;
}

/// What metadata was actually preserved
#[derive(Debug, Clone, PartialEq)]
pub struct MetadataPreserved {
    pub mtime: bool,
    pub permissions: bool,
    pub xattrs: bool,
    pub acls: bool,
    pub owner_group: bool,
}

/// Platform capability flags
#[derive(Debug, Clone)]
pub struct Capabilities {
    pub sparse_files: bool,
    pub symlinks: bool,
    pub xattrs: bool,
    pub acls: bool,
    pub sendfile: bool,
    pub copy_file_range: bool,
    pub block_clone_same_volume: bool,
    /// Detected filesystem type (e.g. "apfs", "ext4", "btrfs"), if known.
    pub filesystem_type: Option<String>,
    /// Whether the filesystem supports reflink/clone (CoW copy).
    pub reflink: bool,
}

/// Result of fast copy attempt
#[derive(Debug)]
pub enum FastCopyResult {
    /// OS primitive used successfully
    Success { bytes: u64, method: &'static str },
    /// OS primitive not available or failed, fallback needed
    Fallback,
}

/// Get platform-specific filesystem capability handler
pub fn get_platform_capability() -> PlatformCapability {
    PlatformCapability::new()
}

/// Global cache of probed capabilities keyed by device ID.
fn probe_cache() -> &'static Mutex<HashMap<u64, Capabilities>> {
    static CACHE: OnceLock<Mutex<HashMap<u64, Capabilities>>> = OnceLock::new();
    CACHE.get_or_init(|| Mutex::new(HashMap::new()))
}

/// Probe capabilities for a path, using the cache if available.
///
/// Returns `None` if the probe fails (e.g. path does not exist).
pub fn cached_probe(path: &Path) -> Option<Capabilities> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::MetadataExt;
        let dev = std::fs::metadata(path).ok()?.dev();
        let cache = probe_cache();
        if let Ok(guard) = cache.lock() {
            if let Some(caps) = guard.get(&dev) {
                return Some(caps.clone());
            }
        }
        let caps = probe_capabilities(path)?;
        if let Ok(mut guard) = cache.lock() {
            guard.insert(dev, caps.clone());
        }
        Some(caps)
    }
    #[cfg(not(unix))]
    {
        probe_capabilities(path)
    }
}
