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
use crate::remote::RemoteEndpoint;
use crate::remote::RemotePath;
use eyre::{bail, Result};
use std::ffi::OsString;
use std::path::{Component, Path, PathBuf};

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

#[cfg(any(windows, target_os = "macos"))]
fn components_eq(a: Component<'_>, b: Component<'_>) -> bool {
    if a == b {
        return true;
    }
    a.as_os_str().to_string_lossy().to_lowercase() == b.as_os_str().to_string_lossy().to_lowercase()
}

#[cfg(not(any(windows, target_os = "macos")))]
fn components_eq(a: Component<'_>, b: Component<'_>) -> bool {
    a == b
}

/// Validate that `src_root` and `dst_root` do not resolve to the same path or
/// an invalid overlapping pair.
///
/// - Identical canonical roots always bail (`copy`, `mirror`, and `move`).
/// - `dst_root` containing `src_root` (`canon_src` is a strict descendant of
///   `canon_dst`) always bails.
/// - `src_root` containing `dst_root` (`canon_dst` is a strict descendant of
///   `canon_src`) bails when `is_move` is `true` (`blit move /a/dir /a/dir/sub`
///   would delete `/a/dir` and the newly moved destination with it), and
///   returns `Ok(Some(rel))` when `is_move` is `false` so `copy` can exclude
///   the nested destination subtree from source enumeration.
pub fn validate_local_transfer_roots(
    src_root: &Path,
    dst_root: &Path,
    is_move: bool,
) -> Result<Option<PathBuf>> {
    let canon_src = crate::path_safety::canonical_target_path(src_root)?;
    let canon_dst = crate::path_safety::canonical_target_path(dst_root)?;

    let src_comps: Vec<Component<'_>> = canon_src.components().collect();
    let dst_comps: Vec<Component<'_>> = canon_dst.components().collect();
    let common = src_comps
        .iter()
        .zip(dst_comps.iter())
        .take_while(|(a, b)| components_eq(**a, **b))
        .count();

    if common == src_comps.len() && common == dst_comps.len() {
        bail!(
            "source and destination resolve to the same path: '{}'",
            src_root.display()
        );
    }
    if common == dst_comps.len() && src_comps.len() > dst_comps.len() {
        bail!(
            "destination '{}' contains source '{}'",
            dst_root.display(),
            src_root.display()
        );
    }
    if common == src_comps.len() && dst_comps.len() > src_comps.len() {
        if is_move {
            bail!(
                "cannot move '{}' into a subdirectory of itself, '{}'",
                src_root.display(),
                dst_root.display()
            );
        }
        let rel: PathBuf = dst_comps[common..].iter().map(|c| c.as_os_str()).collect();
        return Ok(Some(rel));
    }
    Ok(None)
}

fn normalize_remote_rel_components(rel: &Path) -> Option<Vec<OsString>> {
    let mut comps = Vec::new();
    for comp in rel.components() {
        match comp {
            Component::CurDir | Component::RootDir | Component::Prefix(_) => {}
            Component::Normal(name) => comps.push(name.to_os_string()),
            Component::ParentDir => {
                comps.pop()?;
            }
        }
    }
    Some(comps)
}

/// Reject remote-to-remote delegated `move` when source and destination
/// target the same host, port, and module with equal or overlapping relative
/// paths.
pub fn validate_remote_move_endpoints(src: &RemoteEndpoint, dst: &RemoteEndpoint) -> Result<()> {
    if !src.host.eq_ignore_ascii_case(&dst.host) || src.port != dst.port {
        return Ok(());
    }
    let (src_rel, dst_rel) = match (&src.path, &dst.path) {
        (
            RemotePath::Module {
                module: src_mod,
                rel_path: src_rel,
            },
            RemotePath::Module {
                module: dst_mod,
                rel_path: dst_rel,
            },
        ) if src_mod == dst_mod => (src_rel.as_path(), dst_rel.as_path()),
        (RemotePath::Root { rel_path: src_rel }, RemotePath::Root { rel_path: dst_rel }) => {
            (src_rel.as_path(), dst_rel.as_path())
        }
        _ => return Ok(()),
    };
    let (Some(src_comps), Some(dst_comps)) = (
        normalize_remote_rel_components(src_rel),
        normalize_remote_rel_components(dst_rel),
    ) else {
        return Ok(());
    };
    let common = src_comps
        .iter()
        .zip(dst_comps.iter())
        .take_while(|(a, b)| a == b)
        .count();
    if common == src_comps.len() && common == dst_comps.len() {
        bail!(
            "source and destination resolve to the same remote path: '{}'",
            crate::endpoints::format_remote_endpoint(src)
        );
    }
    if common == dst_comps.len() && src_comps.len() > dst_comps.len() {
        bail!(
            "destination '{}' contains source '{}'",
            crate::endpoints::format_remote_endpoint(dst),
            crate::endpoints::format_remote_endpoint(src)
        );
    }
    if common == src_comps.len() && dst_comps.len() > src_comps.len() {
        bail!(
            "cannot move '{}' into a subdirectory of itself, '{}'",
            crate::endpoints::format_remote_endpoint(src),
            crate::endpoints::format_remote_endpoint(dst)
        );
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    //! Rsync-compat unit tests. Moved from
    //! `crates/blit-cli/src/transfers/mod.rs` in A.0 to live with
    //! the implementation they exercise. The CLI's `transfers/mod`
    //! test module retains its end-to-end dispatcher tests but no
    //! longer duplicates the resolution-helper unit coverage.

    use super::*;
    use crate::remote::{RemoteEndpoint, RemotePath};
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

    #[test]
    fn validate_local_transfer_roots_rejects_same_ancestor_and_nested_move() {
        let tmp = tempdir().unwrap();
        let dir = tmp.path().join("dir");
        let sub = dir.join("sub");
        std::fs::create_dir_all(&sub).unwrap();

        // Identical path (including via ./.) rejected for copy and move.
        for is_move in [false, true] {
            let err = validate_local_transfer_roots(&dir, &dir.join("."), is_move)
                .expect_err("same path must be rejected");
            assert!(
                err.to_string().contains("same path"),
                "unexpected error: {err:#}"
            );

            // Destination containing source (dst is ancestor of src) rejected.
            let err = validate_local_transfer_roots(&sub, &dir, is_move)
                .expect_err("ancestor destination must be rejected");
            assert!(
                err.to_string().contains("contains source"),
                "unexpected error: {err:#}"
            );
        }

        // Source containing destination (dst is descendant of src):
        // allowed for copy (returning relative tail), rejected for move.
        let nested_new = dir.join("backup").join("2026");
        let rel = validate_local_transfer_roots(&dir, &nested_new, false)
            .expect("copy into nested subdir is allowed")
            .expect("must return relative subtree path");
        assert_eq!(rel, PathBuf::from("backup").join("2026"));

        let err = validate_local_transfer_roots(&dir, &nested_new, true)
            .expect_err("move into nested subdir must be rejected");
        assert!(
            err.to_string().contains("into a subdirectory of itself"),
            "unexpected error: {err:#}"
        );
    }

    #[test]
    fn validate_remote_move_endpoints_rejects_same_and_overlapping_paths() {
        let ep = |host: &str, module: &str, rel: &str| RemoteEndpoint {
            host: host.into(),
            port: 9031,
            path: RemotePath::Module {
                module: module.into(),
                rel_path: PathBuf::from(rel),
            },
        };

        // Same path (case-insensitive host, `./` normalization)
        let err = validate_remote_move_endpoints(
            &ep("Host-A", "mod", "dir/sub"),
            &ep("host-a", "mod", "./dir/sub"),
        )
        .expect_err("same remote path must fail");
        assert!(err.to_string().contains("same remote path"));

        // Nested destination
        let err = validate_remote_move_endpoints(
            &ep("host-a", "mod", "dir"),
            &ep("host-a", "mod", "dir/sub"),
        )
        .expect_err("nested remote move must fail");
        assert!(err.to_string().contains("into a subdirectory of itself"));

        // Ancestor destination
        let err = validate_remote_move_endpoints(
            &ep("host-a", "mod", "dir/sub"),
            &ep("host-a", "mod", "dir"),
        )
        .expect_err("ancestor remote move must fail");
        assert!(err.to_string().contains("contains source"));

        // Disjoint path or different module succeeds
        validate_remote_move_endpoints(
            &ep("host-a", "mod", "dir/a"),
            &ep("host-a", "mod", "dir/b"),
        )
        .expect("sibling remote paths are allowed");
        validate_remote_move_endpoints(
            &ep("host-a", "mod-a", "dir"),
            &ep("host-a", "mod-b", "dir"),
        )
        .expect("different modules are allowed");
    }
}
