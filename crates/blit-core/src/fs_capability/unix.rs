//! Unix/Linux filesystem capability implementation

use super::{Capabilities, FastCopyResult, FilesystemCapability, MetadataPreserved};
use anyhow::{Context, Result};
use std::path::Path;

pub struct UnixCapability {
    capabilities: Capabilities,
}

impl UnixCapability {
    pub fn new() -> Self {
        Self {
            capabilities: Capabilities {
                sparse_files: true, // Most Linux filesystems support sparse
                symlinks: true,
                xattrs: true, // ext4, xfs, btrfs support xattrs
                acls: false,  // ACLs exist but complex to preserve correctly
                sendfile: true,
                copy_file_range: true, // Linux 4.5+
            },
        }
    }
}

impl FilesystemCapability for UnixCapability {
    fn preserve_metadata(&self, src: &Path, dst: &Path) -> Result<MetadataPreserved> {
        use filetime::{set_file_mtime, FileTime};
        use std::os::unix::fs::PermissionsExt;

        let mut preserved = MetadataPreserved {
            mtime: false,
            permissions: false,
            xattrs: false,
            acls: false,
            owner_group: false,
        };

        let md = std::fs::metadata(src).with_context(|| format!("metadata {}", src.display()))?;

        // Preserve mtime
        if let Ok(modified) = md.modified() {
            let ft = FileTime::from_system_time(modified);
            if set_file_mtime(dst, ft).is_ok() {
                preserved.mtime = true;
            }
        }

        // Preserve permissions
        let mode = md.permissions().mode();
        if std::fs::set_permissions(dst, std::fs::Permissions::from_mode(mode)).is_ok() {
            preserved.permissions = true;
        }

        // Owner/group (requires privileges, likely to fail)
        #[cfg(unix)]
        {
            use std::os::unix::fs::MetadataExt;
            let uid = md.uid();
            let gid = md.gid();

            if let Ok(dst_file) = std::fs::File::open(dst) {
                use std::os::unix::io::AsRawFd;
                let fd = dst_file.as_raw_fd();
                let rc = unsafe { libc::fchown(fd, uid, gid) };
                if rc == 0 {
                    preserved.owner_group = true;
                }
            }
        }

        Ok(preserved)
    }

    fn capabilities(&self) -> &Capabilities {
        &self.capabilities
    }

    fn fast_copy(&self, src: &Path, dst: &Path) -> Result<FastCopyResult> {
        // Try copy_file_range first (Linux 4.5+)
        #[cfg(target_os = "linux")]
        if let Ok(bytes) = try_copy_file_range(src, dst) {
            return Ok(FastCopyResult::Success {
                bytes,
                method: "copy_file_range",
            });
        }

        // Try sendfile
        if let Ok(bytes) = try_sendfile(src, dst) {
            return Ok(FastCopyResult::Success {
                bytes,
                method: "sendfile",
            });
        }

        Ok(FastCopyResult::Fallback)
    }
}

#[cfg(target_os = "linux")]
fn try_copy_file_range(src: &Path, dst: &Path) -> Result<u64> {
    use std::os::unix::io::AsRawFd;

    let src_file = std::fs::File::open(src)?;
    let dst_file = std::fs::OpenOptions::new()
        .create(true)
        .write(true)
        .truncate(true)
        .open(dst)?;

    let src_fd = src_file.as_raw_fd();
    let dst_fd = dst_file.as_raw_fd();
    let len = src_file.metadata()?.len();

    let mut offset = 0u64;
    let mut remaining = len;

    while remaining > 0 {
        let to_copy = remaining.min(i32::MAX as u64) as usize;
        let copied = unsafe {
            libc::syscall(
                libc::SYS_copy_file_range,
                src_fd,
                &mut offset as *mut u64,
                dst_fd,
                std::ptr::null_mut::<u64>(),
                to_copy,
                0,
            )
        };

        if copied < 0 {
            anyhow::bail!("copy_file_range failed");
        }

        if copied == 0 {
            break;
        }

        remaining -= copied as u64;
    }

    Ok(len)
}

#[cfg(not(target_os = "linux"))]
fn try_copy_file_range(_src: &Path, _dst: &Path) -> Result<u64> {
    anyhow::bail!("copy_file_range not available")
}

fn try_sendfile(src: &Path, dst: &Path) -> Result<u64> {
    use std::os::unix::io::AsRawFd;

    let src_file = std::fs::File::open(src)?;
    let dst_file = std::fs::OpenOptions::new()
        .create(true)
        .write(true)
        .truncate(true)
        .open(dst)?;

    let src_fd = src_file.as_raw_fd();
    let dst_fd = dst_file.as_raw_fd();
    let len = src_file.metadata()?.len();

    let mut offset = 0i64;
    let mut remaining = len;

    while remaining > 0 {
        let to_send = remaining.min(i32::MAX as u64) as usize;
        let sent = unsafe { libc::sendfile(dst_fd, src_fd, &mut offset, to_send) };

        if sent < 0 {
            anyhow::bail!("sendfile failed");
        }

        if sent == 0 {
            break;
        }

        remaining -= sent as u64;
    }

    Ok(len)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_unix_capabilities() {
        let cap = UnixCapability::new();
        let caps = cap.capabilities();

        assert!(caps.symlinks, "Unix supports symlinks");
        assert!(caps.sendfile, "Unix has sendfile");
    }
}
