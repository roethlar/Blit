//! Windows filesystem capability implementation

use super::{Capabilities, FastCopyResult, FilesystemCapability, MetadataPreserved};
use crate::win_fs::ensure_long_path;
use eyre::{eyre, Context, Result};
use once_cell::sync::Lazy;
use parking_lot::RwLock;
use std::collections::HashMap;
use std::ffi::OsStr;
use std::os::windows::ffi::OsStrExt;
use std::path::{Path, PathBuf};
use windows::core::PCWSTR;
use windows::Win32::Storage::FileSystem::{GetVolumeInformationW, GetVolumePathNameW};

pub struct WindowsCapability {
    capabilities: Capabilities,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum BlockCloneStatus {
    Supported,
    Unsupported,
}

#[derive(Debug, Clone)]
struct VolumeInfo {
    serial_number: u32,
    filesystem: String,
}

static BLOCK_CLONE_CACHE: Lazy<RwLock<HashMap<u32, BlockCloneStatus>>> =
    Lazy::new(|| RwLock::new(HashMap::new()));

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
                block_clone_same_volume: false,
            },
        }
    }

    /// Probe whether the source/destination pair supports same-volume block cloning.
    ///
    /// The result is cached per volume serial so subsequent calls avoid redundant probes.
    pub fn probe_block_clone_support(&mut self, src: &Path, dst: &Path) -> Result<bool> {
        let supported = supports_block_clone_same_volume_internal(src, dst)?;
        if supported {
            self.capabilities.block_clone_same_volume = true;
        }
        Ok(supported)
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

/// Public helper for other modules to check block-clone fast-path eligibility.
pub(crate) fn supports_block_clone_same_volume(src: &Path, dst: &Path) -> Result<bool> {
    supports_block_clone_same_volume_internal(src, dst)
}

pub(crate) fn mark_block_clone_unsupported(src: &Path, dst: &Path) {
    if let (Ok(src_info), Ok(dst_info)) = (volume_info_for_path(src), volume_info_for_path(dst)) {
        if src_info.serial_number == dst_info.serial_number {
            let mut cache = BLOCK_CLONE_CACHE.write();
            cache.insert(src_info.serial_number, BlockCloneStatus::Unsupported);
        }
    }
}

fn supports_block_clone_same_volume_internal(src: &Path, dst: &Path) -> Result<bool> {
    let src_info = volume_info_for_path(src)?;
    let dst_info = volume_info_for_path(dst)?;

    if src_info.serial_number != dst_info.serial_number {
        return Ok(false);
    }

    let serial = src_info.serial_number;

    if let Some(status) = BLOCK_CLONE_CACHE.read().get(&serial) {
        return Ok(matches!(status, BlockCloneStatus::Supported));
    }

    let supported = should_enable_block_clone(&src_info, &dst_info);
    let mut cache = BLOCK_CLONE_CACHE.write();
    cache.insert(
        serial,
        if supported {
            BlockCloneStatus::Supported
        } else {
            BlockCloneStatus::Unsupported
        },
    );
    Ok(supported)
}

fn should_enable_block_clone(src: &VolumeInfo, dst: &VolumeInfo) -> bool {
    src.serial_number == dst.serial_number && is_refs_filesystem(&dst.filesystem)
}

fn volume_info_for_path(path: &Path) -> Result<VolumeInfo> {
    let query_path = path_with_existing_ancestor(path)
        .with_context(|| format!("no existing ancestor found for {}", path.display()))?;
    let query_display = query_path.display().to_string();
    let long_path = ensure_long_path(&query_path);
    let wide_path = os_str_to_wide(long_path.as_os_str());

    let mut volume_path_buf = vec![0u16; 512];
    unsafe {
        GetVolumePathNameW(PCWSTR(wide_path.as_ptr()), volume_path_buf.as_mut_slice())
            .context(format!("GetVolumePathNameW failed for {query_display}"))?;
    }

    let mut fs_name_buf = vec![0u16; 64];
    let mut serial_number = 0u32;

    unsafe {
        GetVolumeInformationW(
            PCWSTR(volume_path_buf.as_ptr()),
            None,
            Some((&mut serial_number) as *mut u32),
            None,
            None,
            Some(fs_name_buf.as_mut_slice()),
        )
        .context(format!("GetVolumeInformationW failed for {query_display}"))?;
    }

    Ok(VolumeInfo {
        serial_number,
        filesystem: utf16_to_string(&fs_name_buf),
    })
}

fn path_with_existing_ancestor(path: &Path) -> Result<PathBuf> {
    let mut current = Some(path);
    while let Some(p) = current {
        if p.exists() {
            return Ok(p.to_path_buf());
        }
        current = p.parent();
    }
    Err(eyre!(
        "could not find existing ancestor for {}",
        path.display()
    ))
}

fn os_str_to_wide(s: &OsStr) -> Vec<u16> {
    s.encode_wide().chain(std::iter::once(0)).collect()
}

fn utf16_to_string(buf: &[u16]) -> String {
    let end = buf.iter().position(|&c| c == 0).unwrap_or(buf.len());
    String::from_utf16_lossy(&buf[..end])
}

fn is_refs_filesystem(fs_name: &str) -> bool {
    fs_name.eq_ignore_ascii_case("ReFS")
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
    bail!("CopyFileEx not available")
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

    #[test]
    fn block_clone_requires_same_volume() {
        let src = VolumeInfo {
            serial_number: 1,
            filesystem: "ReFS".to_string(),
        };
        let dst = VolumeInfo {
            serial_number: 2,
            filesystem: "ReFS".to_string(),
        };
        assert!(!should_enable_block_clone(&src, &dst));
    }

    #[test]
    fn block_clone_requires_refs_filesystem() {
        let src = VolumeInfo {
            serial_number: 42,
            filesystem: "NTFS".to_string(),
        };
        let dst = VolumeInfo {
            serial_number: 42,
            filesystem: "NTFS".to_string(),
        };
        assert!(!should_enable_block_clone(&src, &dst));
    }

    #[test]
    fn block_clone_supported_for_refs_same_volume() {
        let src = VolumeInfo {
            serial_number: 99,
            filesystem: "ReFS".to_string(),
        };
        let dst = VolumeInfo {
            serial_number: 99,
            filesystem: "ReFS".to_string(),
        };
        assert!(should_enable_block_clone(&src, &dst));
    }
}
