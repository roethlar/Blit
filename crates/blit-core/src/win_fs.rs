use normpath::PathExt;
use std::ffi::OsStr;
use std::fs;
use std::path::{Path, PathBuf};
use windows::{
    core::PCWSTR,
    Win32::{
        Foundation::{CloseHandle, GetLastError, HANDLE, LUID, WIN32_ERROR},
        Security::{
            AdjustTokenPrivileges, LookupPrivilegeValueW, PrivilegeCheck, LUID_AND_ATTRIBUTES,
            PRIVILEGE_SET, SE_PRIVILEGE_ENABLED, TOKEN_ADJUST_PRIVILEGES, TOKEN_PRIVILEGES,
            TOKEN_QUERY,
        },
        System::Threading::{GetCurrentProcess, OpenProcessToken},
    },
};

/// Normalizes a Windows path, resolving `.` and `..` components.
///
/// This function uses the `normpath` crate to provide a robust normalization
/// that works even for paths that do not exist on the filesystem.
///
/// # Arguments
///
/// * `path` - The path to normalize.
///
/// # Returns
///
/// A `PathBuf` containing the normalized path.
pub fn normalize_path(path: &Path) -> PathBuf {
    // The `normalize` method on the `PathExt` trait will handle the
    // normalization of the path, including handling `.` and `..`.
    match path.normalize() {
        Ok(normalized_path) => normalized_path.into_path_buf(),
        Err(_) => path.to_path_buf(), // Fallback to original path on error
    }
}

fn to_wide(path: &Path) -> Vec<u16> {
    use std::os::windows::ffi::OsStrExt;
    path.as_os_str()
        .encode_wide()
        .chain(std::iter::once(0))
        .collect()
}

pub fn create_symlink(target: &Path, link: &Path, is_dir: bool) -> std::io::Result<()> {
    let target_wide = to_wide(target);
    let link_wide = to_wide(link);

    // SAFETY: The wide-string buffers are NUL-terminated vectors that live for the duration of the
    // call, and we pass the documented flag for directory vs file symlinks.
    unsafe {
        use windows::Win32::Storage::FileSystem::{
            CreateSymbolicLinkW, SYMBOLIC_LINK_FLAGS, SYMBOLIC_LINK_FLAG_DIRECTORY,
        };
        let flags: SYMBOLIC_LINK_FLAGS = if is_dir {
            SYMBOLIC_LINK_FLAG_DIRECTORY
        } else {
            SYMBOLIC_LINK_FLAGS(0)
        };
        let result = CreateSymbolicLinkW(
            PCWSTR(link_wide.as_ptr()),
            PCWSTR(target_wide.as_ptr()),
            flags,
        );
        if result.as_bool() {
            Ok(())
        } else {
            Err(std::io::Error::last_os_error())
        }
    }
}

/// Compares two relative paths case-insensitively, which is important on Windows.
///
/// # Arguments
///
/// * `path1` - The first path to compare.
/// * `path2` - The second path to compare.
///
/// # Returns
///
/// `true` if the paths are equal, `false` otherwise.
pub fn compare_paths_case_insensitive(path1: &Path, path2: &Path) -> bool {
    let a = normalize_path(path1).to_string_lossy().to_lowercase();
    let b = normalize_path(path2).to_string_lossy().to_lowercase();
    a == b
}

/// Checks if the current process has the privilege to create symbolic links.
///
/// On Windows, creating symbolic links requires the `SeCreateSymbolicLinkPrivilege`.
/// This function checks if this privilege is enabled for the current process token.
///
/// # Returns
///
/// `true` if the privilege is held, `false` otherwise.
pub fn has_symlink_privilege() -> bool {
    // SAFETY: We request a token for the current process and only call Win32 APIs with pointers to
    // stack-allocated structs whose lifetimes outlive the calls; handles are closed on all paths.
    unsafe {
        let mut token = HANDLE::default();
        if OpenProcessToken(GetCurrentProcess(), TOKEN_QUERY, &mut token).is_err() {
            return false;
        }

        let privilege_name: PCWSTR = windows::core::w!("SeCreateSymbolicLinkPrivilege");
        let mut luid = LUID::default();
        if LookupPrivilegeValueW(None, privilege_name, &mut luid).is_err() {
            let _ = CloseHandle(token);
            return false;
        }

        let mut privilege_set = PRIVILEGE_SET {
            PrivilegeCount: 1,
            Control: 0,
            Privilege: [LUID_AND_ATTRIBUTES {
                Luid: luid,
                Attributes: SE_PRIVILEGE_ENABLED,
            }],
        };

        use windows::Win32::Foundation::BOOL;
        let mut has_privilege = BOOL(0);
        let ok = PrivilegeCheck(token, &mut privilege_set, &mut has_privilege).is_ok();

        let _ = CloseHandle(token);

        ok && has_privilege.as_bool()
    }
}

/// Attempt to enable the `SeManageVolumePrivilege` required for block clone operations.
///
/// Returns `true` if the privilege is enabled or already present.
pub fn enable_manage_volume_privilege() -> bool {
    unsafe {
        let mut token = HANDLE::default();
        let desired_access = TOKEN_QUERY | TOKEN_ADJUST_PRIVILEGES;
        if OpenProcessToken(GetCurrentProcess(), desired_access, &mut token).is_err() {
            return false;
        }

        let privilege_name: PCWSTR = windows::core::w!("SeManageVolumePrivilege");
        let mut luid = LUID::default();
        if LookupPrivilegeValueW(None, privilege_name, &mut luid).is_err() {
            let _ = CloseHandle(token);
            return false;
        }

        let mut privileges = TOKEN_PRIVILEGES {
            PrivilegeCount: 1,
            Privileges: [LUID_AND_ATTRIBUTES {
                Luid: luid,
                Attributes: SE_PRIVILEGE_ENABLED,
            }],
        };

        if AdjustTokenPrivileges(token, false, Some(&mut privileges), 0, None, None).is_err() {
            let _ = CloseHandle(token);
            return false;
        }

        let last_error = GetLastError();
        let _ = CloseHandle(token);

        last_error == WIN32_ERROR(0)
    }
}

/// Recursively clears the read-only attribute from a path and all its contents.
///
/// This is essential for Windows mirror deletions where files may have the
/// read-only attribute set, preventing normal deletion operations.
///
/// # Arguments
///
/// * `path` - The path to clear read-only attributes from
///
/// # Note
///
/// This function will silently continue on errors to ensure best-effort clearing.
/// Optimized to only call metadata once per file.
#[cfg_attr(windows, allow(clippy::permissions_set_readonly_false))]
pub fn clear_readonly_recursive(path: &Path) {
    let mut stack = vec![path.to_path_buf()];
    while let Some(p) = stack.pop() {
        if let Ok(metadata) = fs::symlink_metadata(&p) {
            if metadata.permissions().readonly() {
                let mut perms = metadata.permissions();
                perms.set_readonly(false);
                let _ = fs::set_permissions(&p, perms);
            }
            if metadata.is_dir() {
                if let Ok(entries) = fs::read_dir(&p) {
                    for entry in entries.flatten() {
                        stack.push(entry.path());
                    }
                }
            }
        }
    }
}

/// Returns true if the basename (without extension) is a Windows reserved device name.
/// The check is case-insensitive and applies whether or not an extension is present.
pub fn is_reserved_name(basename: &OsStr) -> bool {
    let s = basename.to_string_lossy();
    let lower = s.trim_end_matches(&[' ', '.'][..]).to_ascii_lowercase();
    // Extract stem before first dot
    let stem = lower.split('.').next().unwrap_or("");
    match stem {
        "con" | "prn" | "aux" | "nul" => true,
        _ => {
            if let Some(num) = stem.strip_prefix("com").and_then(|n| n.parse::<u8>().ok()) {
                return (1..=9).contains(&num);
            }
            if let Some(num) = stem.strip_prefix("lpt").and_then(|n| n.parse::<u8>().ok()) {
                return (1..=9).contains(&num);
            }
            false
        }
    }
}

/// Ensure a path is suitable for long-path operations by adding the `\\?\` prefix when needed.
/// UNC paths will be rewritten as `\\?\UNC\server\share\...`.
pub fn ensure_long_path(p: &Path) -> PathBuf {
    let s = p.as_os_str().to_string_lossy();
    // Already verbatim
    if s.starts_with(r"\\?\") {
        return p.to_path_buf();
    }
    // UNC path (\\server\share\...)
    if s.starts_with(r"\\") {
        // Strip leading \\
        let rest = &s[2..];
        let mut buf = String::from(r"\\?\UNC\");
        buf.push_str(rest);
        return PathBuf::from(buf);
    }
    // Drive path like C:\...
    let norm = match p.normalize() {
        Ok(n) => n.into_path_buf(),
        Err(_) => p.to_path_buf(),
    };
    if norm.is_absolute() {
        let mut buf = String::from(r"\\?\");
        buf.push_str(&norm.as_os_str().to_string_lossy());
        PathBuf::from(buf)
    } else {
        // Relative paths are left as-is
        norm
    }
}
