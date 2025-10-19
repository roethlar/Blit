//! Platform-specific filesystem capability abstraction
//!
//! Provides trait-based interface for metadata preservation, symlinks,
//! sparse files, and fast copy operations across macOS/Linux/Windows.

use eyre::Result;
use std::path::Path;

#[cfg(target_os = "macos")]
mod macos;
#[cfg(all(unix, not(target_os = "macos")))]
mod unix;
#[cfg(windows)]
mod windows;

#[cfg(target_os = "macos")]
pub use macos::MacOSCapability as PlatformCapability;
#[cfg(all(unix, not(target_os = "macos")))]
pub use unix::UnixCapability as PlatformCapability;
#[cfg(windows)]
pub use windows::WindowsCapability as PlatformCapability;

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
