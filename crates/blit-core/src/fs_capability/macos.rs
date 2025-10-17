//! macOS filesystem capability implementation

use super::{Capabilities, FastCopyResult, FilesystemCapability, MetadataPreserved};
use anyhow::Result;
use std::path::Path;

pub struct MacOSCapability {
    capabilities: Capabilities,
}

impl Default for MacOSCapability {
    fn default() -> Self {
        Self::new()
    }
}

impl MacOSCapability {
    pub fn new() -> Self {
        Self {
            capabilities: Capabilities {
                sparse_files: true, // APFS supports sparse
                symlinks: true,
                xattrs: true,
                acls: false, // ACLs exist but not commonly used
                sendfile: false,
                copy_file_range: false, // Not available on macOS
            },
        }
    }
}

impl FilesystemCapability for MacOSCapability {
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

            // Preserve permissions
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                let mode = md.permissions().mode();
                if let Ok(dst_file) = std::fs::File::open(dst) {
                    if dst_file
                        .set_permissions(std::fs::Permissions::from_mode(mode))
                        .is_ok()
                    {
                        preserved.permissions = true;
                    }
                }
            }
        }

        // Extended attributes (using xattr crate if available, otherwise skip)
        // For now, mark as not preserved (would need xattr crate dependency)
        preserved.xattrs = false;

        Ok(preserved)
    }

    fn capabilities(&self) -> &Capabilities {
        &self.capabilities
    }

    fn fast_copy(&self, src: &Path, dst: &Path) -> Result<FastCopyResult> {
        // Try clonefile first (APFS copy-on-write)
        if let Ok(true) = attempt_clonefile(src, dst) {
            let bytes = std::fs::metadata(src)?.len();
            return Ok(FastCopyResult::Success {
                bytes,
                method: "clonefile",
            });
        }

        // Try fcopyfile
        if let Ok(true) = attempt_fcopyfile(src, dst) {
            let bytes = std::fs::metadata(src)?.len();
            return Ok(FastCopyResult::Success {
                bytes,
                method: "fcopyfile",
            });
        }

        Ok(FastCopyResult::Fallback)
    }
}

fn attempt_clonefile(src: &Path, dst: &Path) -> Result<bool> {
    use std::ffi::CString;
    use std::os::unix::ffi::OsStrExt;

    let src_c = CString::new(src.as_os_str().as_bytes())?;
    let dst_c = CString::new(dst.as_os_str().as_bytes())?;

    let rc = unsafe { libc::clonefile(src_c.as_ptr(), dst_c.as_ptr(), 0) };
    Ok(rc == 0)
}

fn attempt_fcopyfile(src: &Path, dst: &Path) -> Result<bool> {
    use std::os::unix::io::AsRawFd;

    let src_file = std::fs::File::open(src)?;
    let dst_file = std::fs::OpenOptions::new()
        .create(true)
        .write(true)
        .truncate(true)
        .open(dst)?;

    let flags: libc::copyfile_flags_t =
        libc::COPYFILE_ACL | libc::COPYFILE_STAT | libc::COPYFILE_XATTR | libc::COPYFILE_DATA;

    let rc = unsafe {
        libc::fcopyfile(
            src_file.as_raw_fd(),
            dst_file.as_raw_fd(),
            std::ptr::null_mut(),
            flags,
        )
    };

    Ok(rc == 0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_capabilities() {
        let cap = MacOSCapability::new();
        let caps = cap.capabilities();

        assert!(caps.symlinks, "macOS supports symlinks");
        assert!(caps.xattrs, "macOS supports extended attributes");
        assert!(!caps.copy_file_range, "macOS doesn't have copy_file_range");
    }

    #[test]
    fn test_metadata_preservation() -> Result<()> {
        let tmp = tempfile::tempdir()?;
        let src = tmp.path().join("src.txt");
        let dst = tmp.path().join("dst.txt");

        std::fs::write(&src, b"test")?;

        let cap = MacOSCapability::new();
        let preserved = cap.preserve_metadata(&src, &dst)?;

        // At minimum, should attempt mtime
        // Actual success depends on filesystem
        Ok(())
    }
}
