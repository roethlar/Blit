//! Filesystem type detection and capability probing.
//!
//! Uses `statfs` on Unix and volume queries on Windows to detect
//! the filesystem type at a given path, then maps it to accurate
//! capability flags.

use super::Capabilities;
use std::path::Path;

/// Detect the filesystem type for a given path.
///
/// Returns a lowercase string like `"apfs"`, `"ext4"`, `"btrfs"`, `"ntfs"`, etc.
/// Returns `None` if detection fails.
pub fn detect_filesystem_type(path: &Path) -> Option<String> {
    detect_filesystem_type_impl(path)
}

/// Probe capabilities for a path based on the detected filesystem type.
///
/// Returns `None` if the path doesn't exist or detection fails.
pub fn probe_capabilities(path: &Path) -> Option<Capabilities> {
    let fs_type = detect_filesystem_type(path);
    Some(capabilities_for_filesystem(fs_type.as_deref()))
}

/// Map a filesystem type string to accurate capability flags.
fn capabilities_for_filesystem(fs_type: Option<&str>) -> Capabilities {
    match fs_type {
        // macOS filesystems
        Some("apfs") => Capabilities {
            sparse_files: true,
            symlinks: true,
            xattrs: true,
            acls: false,
            sendfile: false,
            copy_file_range: false,
            block_clone_same_volume: false,
            filesystem_type: Some("apfs".into()),
            reflink: true, // APFS supports clonefile
        },
        Some("hfs") | Some("hfs+") => Capabilities {
            sparse_files: true,
            symlinks: true,
            xattrs: true,
            acls: false,
            sendfile: false,
            copy_file_range: false,
            block_clone_same_volume: false,
            filesystem_type: Some("hfs+".into()),
            reflink: false,
        },
        // Linux filesystems
        Some("btrfs") => Capabilities {
            sparse_files: true,
            symlinks: true,
            xattrs: true,
            acls: true,
            sendfile: true,
            copy_file_range: true,
            block_clone_same_volume: false,
            filesystem_type: Some("btrfs".into()),
            reflink: true, // btrfs supports FICLONE
        },
        Some("xfs") => Capabilities {
            sparse_files: true,
            symlinks: true,
            xattrs: true,
            acls: true,
            sendfile: true,
            copy_file_range: true,
            block_clone_same_volume: false,
            filesystem_type: Some("xfs".into()),
            reflink: true, // XFS v5 with reflink enabled
        },
        Some("ext4") | Some("ext3") | Some("ext2") => Capabilities {
            sparse_files: true,
            symlinks: true,
            xattrs: true,
            acls: true,
            sendfile: true,
            copy_file_range: true,
            block_clone_same_volume: false,
            filesystem_type: fs_type.map(Into::into),
            reflink: false,
        },
        Some("zfs") => Capabilities {
            sparse_files: true,
            symlinks: true,
            xattrs: true,
            acls: true,
            sendfile: true,
            copy_file_range: true,
            block_clone_same_volume: false,
            filesystem_type: Some("zfs".into()),
            reflink: false, // ZFS block cloning exists but not via standard interfaces
        },
        Some("tmpfs") => Capabilities {
            sparse_files: true,
            symlinks: true,
            xattrs: true,
            acls: false,
            sendfile: true,
            copy_file_range: true,
            block_clone_same_volume: false,
            filesystem_type: Some("tmpfs".into()),
            reflink: false,
        },
        Some("nfs") | Some("nfs4") | Some("cifs") | Some("smbfs") => Capabilities {
            sparse_files: false,
            symlinks: true,
            xattrs: false,
            acls: false,
            sendfile: false,
            copy_file_range: false,
            block_clone_same_volume: false,
            filesystem_type: fs_type.map(Into::into),
            reflink: false,
        },
        // Windows filesystems (detected via volume queries, not statfs)
        Some("ntfs") => Capabilities {
            sparse_files: true,
            symlinks: true,
            xattrs: false,
            acls: true,
            sendfile: false,
            copy_file_range: false,
            block_clone_same_volume: false,
            filesystem_type: Some("ntfs".into()),
            reflink: false,
        },
        Some("refs") => Capabilities {
            sparse_files: true,
            symlinks: true,
            xattrs: false,
            acls: true,
            sendfile: false,
            copy_file_range: false,
            block_clone_same_volume: true,
            filesystem_type: Some("refs".into()),
            reflink: true,
        },
        // Unknown or unsupported — conservative defaults
        _ => {
            let mut caps = platform_defaults();
            caps.filesystem_type = fs_type.map(Into::into);
            caps
        }
    }
}

/// Conservative platform defaults when the filesystem type is unknown.
fn platform_defaults() -> Capabilities {
    #[cfg(target_os = "macos")]
    {
        Capabilities {
            sparse_files: true,
            symlinks: true,
            xattrs: true,
            acls: false,
            sendfile: false,
            copy_file_range: false,
            block_clone_same_volume: false,
            filesystem_type: None,
            reflink: false,
        }
    }
    #[cfg(all(unix, not(target_os = "macos")))]
    {
        Capabilities {
            sparse_files: true,
            symlinks: true,
            xattrs: true,
            acls: false,
            sendfile: true,
            copy_file_range: true,
            block_clone_same_volume: false,
            filesystem_type: None,
            reflink: false,
        }
    }
    #[cfg(windows)]
    {
        Capabilities {
            sparse_files: true,
            symlinks: true,
            xattrs: false,
            acls: true,
            sendfile: false,
            copy_file_range: false,
            block_clone_same_volume: false,
            filesystem_type: None,
            reflink: false,
        }
    }
}

// --- Platform-specific detection ---

#[cfg(target_os = "macos")]
fn detect_filesystem_type_impl(path: &Path) -> Option<String> {
    use std::ffi::CString;
    use std::os::unix::ffi::OsStrExt;

    let c_path = CString::new(path.as_os_str().as_bytes()).ok()?;
    let mut stat: libc::statfs = unsafe { std::mem::zeroed() };
    let rc = unsafe { libc::statfs(c_path.as_ptr(), &mut stat) };
    if rc != 0 {
        return None;
    }

    // f_fstypename is a [c_char; 16] on macOS
    let name_bytes: Vec<u8> = stat
        .f_fstypename
        .iter()
        .take_while(|&&c| c != 0)
        .map(|&c| c as u8)
        .collect();
    let name = String::from_utf8(name_bytes).ok()?;
    Some(name.to_lowercase())
}

#[cfg(all(unix, not(target_os = "macos")))]
fn detect_filesystem_type_impl(path: &Path) -> Option<String> {
    use std::ffi::CString;
    use std::os::unix::ffi::OsStrExt;

    let c_path = CString::new(path.as_os_str().as_bytes()).ok()?;
    let mut stat: libc::statfs = unsafe { std::mem::zeroed() };
    let rc = unsafe { libc::statfs(c_path.as_ptr(), &mut stat) };
    if rc != 0 {
        return None;
    }

    // Map f_type magic numbers to filesystem names
    #[allow(clippy::unnecessary_cast)]
    let fs_name = match stat.f_type as u64 {
        0x9123683E => "btrfs",
        0x58465342 => "xfs",
        0xEF53 => "ext4", // ext2/3/4 share the same magic
        0x2FC12FC1 => "zfs",
        0x01021994 => "tmpfs",
        0x6969 => "nfs",
        0xFF534D42 => "cifs",
        0x5346544E => "ntfs", // NTFS-3G
        0x61756673 => "aufs",
        0x794C7630 => "overlayfs",
        0xF15F => "ecryptfs",
        0x137D => "ext2",
        0xEF51 => "ext2",
        _ => return Some(format!("unknown(0x{:X})", stat.f_type)),
    };
    Some(fs_name.to_string())
}

#[cfg(windows)]
fn detect_filesystem_type_impl(_path: &Path) -> Option<String> {
    // Windows detection is handled by the existing windows.rs volume_info_for_path
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn probe_on_current_directory() {
        let caps = probe_capabilities(Path::new("."));
        assert!(caps.is_some(), "should detect capabilities for cwd");
        let caps = caps.unwrap();
        assert!(caps.symlinks, "symlinks should be supported");
        if let Some(ref fs_type) = caps.filesystem_type {
            assert!(!fs_type.is_empty(), "filesystem type should be non-empty");
        }
    }

    #[test]
    fn detect_fs_type_on_current_directory() {
        let fs_type = detect_filesystem_type(Path::new("."));
        // Should succeed on any real filesystem
        assert!(fs_type.is_some(), "should detect filesystem type for cwd");
    }

    #[test]
    fn detect_fs_type_returns_none_for_missing_path() {
        let fs_type = detect_filesystem_type(Path::new("/nonexistent/path/abc123"));
        assert!(fs_type.is_none());
    }

    #[test]
    fn capabilities_for_known_filesystems() {
        let btrfs = capabilities_for_filesystem(Some("btrfs"));
        assert!(btrfs.reflink, "btrfs supports reflink");
        assert!(btrfs.copy_file_range, "btrfs supports copy_file_range");

        let ext4 = capabilities_for_filesystem(Some("ext4"));
        assert!(!ext4.reflink, "ext4 does not support reflink");

        let nfs = capabilities_for_filesystem(Some("nfs"));
        assert!(!nfs.sparse_files, "nfs does not reliably support sparse");
        assert!(!nfs.sendfile, "nfs does not support sendfile");
    }

    #[test]
    fn capabilities_for_unknown_returns_defaults() {
        let caps = capabilities_for_filesystem(Some("foofs"));
        assert_eq!(caps.filesystem_type.as_deref(), Some("foofs"));
        // Should get platform defaults, not panic
    }

    #[test]
    fn capabilities_for_none_returns_defaults() {
        let caps = capabilities_for_filesystem(None);
        assert!(caps.filesystem_type.is_none());
    }

    #[test]
    fn cached_probe_returns_consistent_results() {
        let first = super::super::cached_probe(Path::new("."));
        let second = super::super::cached_probe(Path::new("."));
        assert!(first.is_some());
        assert!(second.is_some());
        let f = first.unwrap();
        let s = second.unwrap();
        assert_eq!(f.filesystem_type, s.filesystem_type);
        assert_eq!(f.reflink, s.reflink);
    }
}
