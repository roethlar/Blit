//! Windows filesystem capability implementation

use super::{Capabilities, FastCopyResult, FilesystemCapability, MetadataPreserved};
use anyhow::{Context, Result};
use std::path::Path;

pub struct WindowsCapability {
    capabilities: Capabilities,
}

impl WindowsCapability {
    pub fn new() -> Self {
        Self {
            capabilities: Capabilities {
                sparse_files: true, // NTFS supports sparse files
                symlinks: true,     // Requires Developer Mode or elevation
                xattrs: false,      // NTFS has alternate data streams, not standard xattrs
                acls: true,         // Windows ACLs
                sendfile: false,
                copy_file_range: false,
            },
        }
    }
}

impl FilesystemCapability for WindowsCapability {
    fn preserve_metadata(&self, src: &Path, dst: &Path) -> Result<MetadataPreserved> {
        use filetime::{set_file_mtime, FileTime};

        let mut preserved = MetadataPreserved {
            mtime: false,
            permissions: false,
            xattrs: false,
            acls: false,
            owner_group: false,
        };

        // Preserve mtime
        if let Ok(md) = std::fs::metadata(src) {
            if let Ok(modified) = md.modified() {
                let ft = FileTime::from_system_time(modified);
                if set_file_mtime(dst, ft).is_ok() {
                    preserved.mtime = true;
                }
            }

            // Preserve read-only attribute
            #[cfg(windows)]
            {
                let src_readonly = md.permissions().readonly();
                if let Ok(mut dst_perms) = std::fs::metadata(dst).map(|m| m.permissions()) {
                    dst_perms.set_readonly(src_readonly);
                    if std::fs::set_permissions(dst, dst_perms).is_ok() {
                        preserved.permissions = true;
                    }
                }
            }
        }

        // ACLs would require Windows API (GetSecurityInfo/SetSecurityInfo)
        // Not implemented for now
        preserved.acls = false;

        Ok(preserved)
    }

    fn capabilities(&self) -> &Capabilities {
        &self.capabilities
    }

    fn fast_copy(&self, src: &Path, dst: &Path) -> Result<FastCopyResult> {
        #[cfg(windows)]
        {
            if let Ok(bytes) = try_copyfileex(src, dst) {
                return Ok(FastCopyResult::Success {
                    bytes,
                    method: "CopyFileEx",
                });
            }
        }

        Ok(FastCopyResult::Fallback)
    }
}

#[cfg(windows)]
fn try_copyfileex(src: &Path, dst: &Path) -> Result<u64> {
    use std::os::windows::ffi::OsStrExt;
    use windows::Win32::Storage::FileSystem::CopyFileExW;

    let src_wide: Vec<u16> = src.as_os_str().encode_wide().chain(Some(0)).collect();
    let dst_wide: Vec<u16> = dst.as_os_str().encode_wide().chain(Some(0)).collect();

    unsafe {
        CopyFileExW(
            windows::core::PCWSTR(src_wide.as_ptr()),
            windows::core::PCWSTR(dst_wide.as_ptr()),
            None,
            None,
            None,
            0,
        )
    }
    .context("CopyFileExW failed")?;

    let bytes = std::fs::metadata(src)?.len();
    Ok(bytes)
}

#[cfg(not(windows))]
fn try_copyfileex(_src: &Path, _dst: &Path) -> Result<u64> {
    anyhow::bail!("CopyFileEx not available")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_windows_capabilities() {
        let cap = WindowsCapability::new();
        let caps = cap.capabilities();

        #[cfg(windows)]
        {
            assert!(caps.symlinks, "Windows supports symlinks");
            assert!(caps.sparse_files, "NTFS supports sparse files");
            assert!(!caps.sendfile, "Windows doesn't have sendfile");
        }
    }
}
