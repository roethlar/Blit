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
