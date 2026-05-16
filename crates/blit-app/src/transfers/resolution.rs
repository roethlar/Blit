//! Rsync-style source / destination resolution.
//!
//! Moved from `crates/blit-cli/src/transfers/mod.rs` in A.0.
//! Three pure helpers consumed by every transfer dispatch site
//! (copy / mirror / move) and by `diagnostics dump` to surface
//! the resolution result in its snapshot output:
//!
//! - [`source_is_contents`] — does the raw source argument mean
//!   "copy the contents of this directory" (trailing `/`, `/.`,
//!   or bare `.`)? Matches rsync `flist.c:send_file_list`.
//! - [`dest_is_container`] — should the destination be treated
//!   as "put it INTO this dir" (trailing slash, or pre-existing
//!   local directory)? Matches rsync `main.c:get_local_name`.
//! - [`resolve_destination`] — applies both above to produce the
//!   final target path.
//!
//! These were `pub(crate)` in the CLI; widened to `pub` for the
//! cross-crate consumers. `source_basename` (private helper) is
//! also `pub` so future callers (TUI transfer-options modal)
//! can preview the final target without going through the full
//! resolve path.

use crate::endpoints::Endpoint;
use blit_core::remote::RemotePath;
use std::ffi::OsString;

/// Returns true if the raw CLI source string specifies "copy
/// contents" mode, matching rsync's DOTDIR_NAME classification
/// from `flist.c:send_file_list`.
///
/// Rsync treats these source forms as "copy the contents of
/// this directory":
///   - ends with `/`     (e.g. `src/`)
///   - ends with `/.`    (e.g. `src/.`)
///   - is exactly `.`
///
/// On Windows, `\` and `\.` are also recognized as trailing
/// separators. On Unix, `\` is a literal filename character and
/// is NOT recognized.
pub fn source_is_contents(raw_source: &str) -> bool {
    let s = raw_source.trim_end_matches([' ', '\t']);
    if s.is_empty() {
        return false;
    }
    let bytes = s.as_bytes();
    let last = bytes[bytes.len() - 1];

    // Trailing directory separator
    if last == b'/' {
        return true;
    }
    #[cfg(windows)]
    if last == b'\\' {
        return true;
    }

    // Trailing "/." or just "." — rsync flist.c:2296
    if last == b'.' {
        if bytes.len() == 1 {
            return true; // source is exactly "."
        }
        let second_last = bytes[bytes.len() - 2];
        if second_last == b'/' {
            return true;
        }
        #[cfg(windows)]
        if second_last == b'\\' {
            return true;
        }
    }

    false
}

/// Extract the source's final path component (basename) as an
/// `OsString`, regardless of whether the source is local or
/// remote. Returns `None` for empty, root, or `"."` basenames
/// (which would be meaningless to append).
pub fn source_basename(src: &Endpoint) -> Option<OsString> {
    let basename = match src {
        Endpoint::Local(p) => p.file_name().map(|s| s.to_os_string()),
        Endpoint::Remote(r) => match &r.path {
            RemotePath::Module { rel_path, .. } | RemotePath::Root { rel_path } => {
                rel_path.file_name().map(|s| s.to_os_string())
            }
            RemotePath::Discovery => None,
        },
    };
    match basename {
        Some(b) if !b.is_empty() && b != "." => Some(b),
        _ => None,
    }
}

/// Returns true if the destination should be treated as a
/// container (i.e. the source should be placed INSIDE it, not
/// AS it).
///
/// Matches rsync's `main.c:get_local_name`: a dest is a
/// container if it has a trailing slash, or if it already
/// exists as a directory. The existing-directory probe is
/// local-only — remote requires an RPC we don't want here.
pub fn dest_is_container(raw_dest: &str, dst: &Endpoint) -> bool {
    let s = raw_dest.trim_end_matches([' ', '\t']);
    // Trailing "/" or "/." counts as container ("put into here").
    if s.ends_with('/') || s.ends_with("/.") {
        return true;
    }
    #[cfg(windows)]
    if s.ends_with('\\') || s.ends_with("\\.") {
        return true;
    }
    // Existing directory (local only — remote requires an RPC we don't want here).
    if let Endpoint::Local(p) = dst {
        if p.is_dir() {
            return true;
        }
    }
    false
}

/// Apply rsync-style destination semantics to compute the final
/// target path.
///
/// Matches the union of rsync `flist.c:send_file_list` (source
/// slash) and `main.c:get_local_name` (dest container detection):
///
///   - If source is "contents" form (`src/`, `src/.`, `.`) →
///     dest used as-is.
///   - Else if dest is a container (trailing slash, or existing
///     local dir) → append source's basename to dest.
///   - Else → dest used as-is (the user named an exact target
///     path / rename).
///
/// Examples:
///   blit copy /a/src  /b/dst/    ->  /b/dst/src/...     (nest under dst)
///   blit copy /a/src/ /b/dst/    ->  /b/dst/<contents>  (merge)
///   blit copy /a/src  /b/newdst  ->  /b/newdst/...      (dst becomes src copy)
///   blit copy /a/f.txt /b/dst/   ->  /b/dst/f.txt       (into dir)
///   blit copy /a/f.txt /b/renamed.txt -> /b/renamed.txt (rename)
pub fn resolve_destination(
    raw_source: &str,
    raw_dest: &str,
    src: &Endpoint,
    dst: Endpoint,
) -> Endpoint {
    if source_is_contents(raw_source) {
        return dst;
    }
    if !dest_is_container(raw_dest, &dst) {
        return dst;
    }
    let Some(basename) = source_basename(src) else {
        return dst;
    };
    match dst {
        Endpoint::Local(p) => Endpoint::Local(p.join(&basename)),
        Endpoint::Remote(mut r) => {
            r.path = match r.path {
                RemotePath::Module { module, rel_path } => RemotePath::Module {
                    module,
                    rel_path: rel_path.join(&basename),
                },
                RemotePath::Root { rel_path } => RemotePath::Root {
                    rel_path: rel_path.join(&basename),
                },
                other => other,
            };
            Endpoint::Remote(r)
        }
    }
}

#[cfg(test)]
mod tests {
    //! Rsync-compat unit tests. Moved from
    //! `crates/blit-cli/src/transfers/mod.rs` in A.0 to live with
    //! the implementation they exercise. The CLI's `transfers/mod`
    //! test module retains its end-to-end dispatcher tests but no
    //! longer duplicates the resolution-helper unit coverage.

    use super::*;
    use blit_core::remote::{RemoteEndpoint, RemotePath};
    use std::path::PathBuf;
    use tempfile::tempdir;

    // rsync-compat: "copy contents" detection matches flist.c:send_file_list
    #[test]
    fn source_is_contents_trailing_slash() {
        assert!(source_is_contents("src/"));
        assert!(source_is_contents("/a/b/src/"));
        assert!(source_is_contents("src///"));
    }

    #[test]
    fn source_is_contents_trailing_dot_slash() {
        assert!(source_is_contents("src/."));
        assert!(source_is_contents("/a/b/src/."));
    }

    #[test]
    fn source_is_contents_just_dot() {
        assert!(source_is_contents("."));
    }

    #[test]
    fn source_is_contents_no_trailing() {
        assert!(!source_is_contents("src"));
        assert!(!source_is_contents("/a/b/src"));
        assert!(!source_is_contents(""));
        assert!(!source_is_contents("src.txt"));
        // second-to-last char is not a slash, so trailing '.' is part of filename
        assert!(!source_is_contents("foo.bar"));
    }

    #[test]
    fn source_is_contents_trims_whitespace() {
        assert!(source_is_contents("src/  "));
        assert!(source_is_contents("src/.\t"));
    }

    #[cfg(windows)]
    #[test]
    fn source_is_contents_windows_backslash() {
        assert!(source_is_contents("src\\"));
        assert!(source_is_contents("src\\."));
        assert!(source_is_contents("C:\\path\\"));
    }

    // resolve_destination: core rsync-style semantics
    //
    // Rules (applied identically to local→local, local→remote, remote→local,
    // remote→remote, since resolution happens at the top-level dispatch):
    //   source contents form (SRC/, SRC/., .) → dest as-is (merge)
    //   dest trailing slash OR existing local dir → append source basename (nest)
    //   else → dest as-is (rename / exact target)

    #[test]
    fn resolve_destination_existing_dir_appends_basename() {
        let tmp = tempdir().unwrap();
        let src = Endpoint::Local(PathBuf::from("/a/GameDir"));
        let dst = Endpoint::Local(tmp.path().to_path_buf());
        let dst_raw = tmp.path().to_string_lossy().into_owned();
        let resolved = resolve_destination("/a/GameDir", &dst_raw, &src, dst);
        match resolved {
            Endpoint::Local(p) => assert_eq!(p, tmp.path().join("GameDir")),
            _ => panic!("expected local endpoint"),
        }
    }

    #[test]
    fn resolve_destination_trailing_slash_on_dest_appends_basename() {
        // Dest doesn't exist, but has trailing slash → still a container.
        let src = Endpoint::Local(PathBuf::from("/a/GameDir"));
        let dst = Endpoint::Local(PathBuf::from("/b/new_dst"));
        let resolved = resolve_destination("/a/GameDir", "/b/new_dst/", &src, dst);
        match resolved {
            Endpoint::Local(p) => assert_eq!(p, PathBuf::from("/b/new_dst/GameDir")),
            _ => panic!("expected local endpoint"),
        }
    }

    #[test]
    fn resolve_destination_nonexistent_no_slash_uses_dest_as_target() {
        // rsync: `rsync src newdst` where newdst doesn't exist → newdst becomes src copy
        let src = Endpoint::Local(PathBuf::from("/a/GameDir"));
        let dst = Endpoint::Local(PathBuf::from("/definitely/does/not/exist/newdst"));
        let resolved =
            resolve_destination("/a/GameDir", "/definitely/does/not/exist/newdst", &src, dst);
        match resolved {
            Endpoint::Local(p) => assert_eq!(p, PathBuf::from("/definitely/does/not/exist/newdst")),
            _ => panic!("expected local endpoint"),
        }
    }

    #[test]
    fn resolve_destination_source_trailing_slash_keeps_dest() {
        let tmp = tempdir().unwrap();
        let src = Endpoint::Local(PathBuf::from("/a/GameDir"));
        let dst = Endpoint::Local(tmp.path().to_path_buf());
        let dst_before = tmp.path().to_path_buf();
        let dst_raw = tmp.path().to_string_lossy().into_owned();
        let resolved = resolve_destination("/a/GameDir/", &dst_raw, &src, dst);
        match resolved {
            Endpoint::Local(p) => assert_eq!(p, dst_before),
            _ => panic!("expected local endpoint"),
        }
    }

    #[test]
    fn resolve_destination_source_dot_slash_keeps_dest() {
        let tmp = tempdir().unwrap();
        let src = Endpoint::Local(PathBuf::from("/a/GameDir"));
        let dst = Endpoint::Local(tmp.path().to_path_buf());
        let dst_before = tmp.path().to_path_buf();
        let dst_raw = tmp.path().to_string_lossy().into_owned();
        let resolved = resolve_destination("/a/GameDir/.", &dst_raw, &src, dst);
        match resolved {
            Endpoint::Local(p) => assert_eq!(p, dst_before),
            _ => panic!("expected local endpoint"),
        }
    }

    #[test]
    fn resolve_destination_file_to_existing_dir_appends_filename() {
        let tmp = tempdir().unwrap();
        let src = Endpoint::Local(PathBuf::from("/a/file.txt"));
        let dst = Endpoint::Local(tmp.path().to_path_buf());
        let dst_raw = tmp.path().to_string_lossy().into_owned();
        let resolved = resolve_destination("/a/file.txt", &dst_raw, &src, dst);
        match resolved {
            Endpoint::Local(p) => assert_eq!(p, tmp.path().join("file.txt")),
            _ => panic!("expected local endpoint"),
        }
    }

    #[test]
    fn resolve_destination_file_to_exact_path_is_rename() {
        // Dest doesn't exist, no trailing slash → exact rename.
        let src = Endpoint::Local(PathBuf::from("/a/file.txt"));
        let dst = Endpoint::Local(PathBuf::from("/b/renamed.txt"));
        let resolved = resolve_destination("/a/file.txt", "/b/renamed.txt", &src, dst);
        match resolved {
            Endpoint::Local(p) => assert_eq!(p, PathBuf::from("/b/renamed.txt")),
            _ => panic!("expected local endpoint"),
        }
    }

    #[test]
    fn resolve_destination_remote_dest_with_trailing_slash_appends() {
        // Remote dest with trailing slash is a container (same rule as local).
        let src = Endpoint::Local(PathBuf::from("/a/GameDir"));
        let dst = Endpoint::Remote(RemoteEndpoint {
            host: "h".into(),
            port: 9031,
            path: RemotePath::Module {
                module: "m".into(),
                rel_path: PathBuf::from("common"),
            },
        });
        let resolved = resolve_destination("/a/GameDir", "h:/m/common/", &src, dst);
        match resolved {
            Endpoint::Remote(r) => match r.path {
                RemotePath::Module { rel_path, .. } => {
                    assert_eq!(rel_path, PathBuf::from("common/GameDir"))
                }
                _ => panic!("expected module path"),
            },
            _ => panic!("expected remote endpoint"),
        }
    }

    #[test]
    fn resolve_destination_remote_dest_no_slash_preserves_target() {
        // No trailing slash + remote dest (can't stat) → treat as exact target.
        let src = Endpoint::Local(PathBuf::from("/a/GameDir"));
        let dst = Endpoint::Remote(RemoteEndpoint {
            host: "h".into(),
            port: 9031,
            path: RemotePath::Module {
                module: "m".into(),
                rel_path: PathBuf::from("common/target"),
            },
        });
        let resolved = resolve_destination("/a/GameDir", "h:/m/common/target", &src, dst);
        match resolved {
            Endpoint::Remote(r) => match r.path {
                RemotePath::Module { rel_path, .. } => {
                    // No trailing slash on dest, can't stat remote → preserve target
                    assert_eq!(rel_path, PathBuf::from("common/target"))
                }
                _ => panic!("expected module path"),
            },
            _ => panic!("expected remote endpoint"),
        }
    }

    #[test]
    fn resolve_destination_remote_source_appends_basename_on_container() {
        let tmp = tempdir().unwrap();
        let src = Endpoint::Remote(RemoteEndpoint {
            host: "h".into(),
            port: 9031,
            path: RemotePath::Module {
                module: "m".into(),
                rel_path: PathBuf::from("Games/DOOM"),
            },
        });
        let dst = Endpoint::Local(tmp.path().to_path_buf());
        let dst_raw = tmp.path().to_string_lossy().into_owned();
        let resolved = resolve_destination("h:/m/Games/DOOM", &dst_raw, &src, dst);
        match resolved {
            Endpoint::Local(p) => assert_eq!(p, tmp.path().join("DOOM")),
            _ => panic!("expected local endpoint"),
        }
    }
}
