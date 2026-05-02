//! Wire-supplied path validation and safe joining.
//!
//! Every place blit accepts a path string from the wire (push manifest
//! entries, pull file headers, tar shard inner paths, resume block
//! identifiers, purge requests) must route through one of the helpers
//! here. The shared chokepoint replaces a half-dozen ad-hoc validators
//! that drifted apart and missed escape forms.
//!
//! A buggy or malicious peer should not be able to write outside the
//! destination root by sending:
//!   - absolute paths (`/etc/passwd`)
//!   - parent-dir traversal (`../../etc/passwd`, `foo/../../etc/passwd`)
//!   - Windows drive prefixes (`C:\evil`), UNC paths
//!     (`\\?\C:\evil`, `\\server\share\evil`), or single-leading-
//!     backslash Windows-root forms (`\Windows\System32`)
//!   - embedded NUL bytes
//!   - `"."` / `"./"` / `"./."` (these normalize to empty but are not
//!     the legitimate single-file empty-path case)
//!
//! Per F1 of `docs/reviews/codebase_review_2026-05-01.md`. Path safety is
//! sequenced first in `docs/plan/PIPELINE_UNIFICATION.md` because every
//! receive sink — current `pull_sync.rs` custom path AND the unified
//! pipeline — needs it.
//!
//! ─── Contract: lexical safety only ────────────────────────────────
//!
//! `safe_join(root, wire)` performs a *lexical* containment check.
//! It rejects path strings that escape via syntax (above), but does
//! NOT canonicalize the result or resolve symlinks. That means:
//!
//!   - If `dest_root/link` is a symlink that points outside
//!     `dest_root`, a wire path of `"link/file"` lexically passes and
//!     `safe_join` returns `dest_root/link/file`. The actual filesystem
//!     write follows the symlink and lands outside the root.
//!
//!   - Symlink-traversal containment is the responsibility of the
//!     `use_chroot` / canonical-containment work tracked as F2 in
//!     `docs/reviews/codebase_review_2026-05-01.md` and as the second
//!     post-pipeline-unification item in `TODO.md`. F2's canonicalize-
//!     and-check pass operates *after* `safe_join`'s lexical pass.
//!
//! For receive paths today (F1 only), this means an operator running
//! the daemon with `use_chroot = false` (the default) and a destination
//! root that contains attacker-controlled symlinks can have files
//! written through those links. The daemon docs describe this risk.
//! F2 is the resolution.

use std::path::{Component, Path, PathBuf};

use eyre::{bail, Result};

/// Validate a wire-supplied relative path. Returns the normalized
/// relative form (with `.` components stripped) or an error describing
/// why the path is unsafe.
///
/// Empty input returns an empty `PathBuf`. The caller decides what
/// empty means (e.g. "use the root unchanged" for single-file
/// transfers, or "treat as `.`" for directory enumeration). Most
/// receive-side callers want `safe_join` instead, which encodes the
/// "empty means use root unchanged" rule directly.
pub fn validate_wire_path(wire_path: &str) -> Result<PathBuf> {
    if wire_path.is_empty() {
        return Ok(PathBuf::new());
    }

    if wire_path.contains('\0') {
        bail!("path contains NUL byte: {:?}", wire_path);
    }

    // Catch Windows-shaped absolutes early. `Path::components` on Unix
    // does not classify `C:` as a `Prefix`, and a wire path that looks
    // like a Windows absolute should be rejected uniformly regardless
    // of which platform the daemon is running on — receive sinks may
    // serve cross-platform clients.
    if looks_like_windows_absolute(wire_path) {
        bail!("path uses Windows-absolute form: {:?}", wire_path);
    }

    let path = Path::new(wire_path);

    if path.is_absolute() {
        bail!("absolute path not allowed: {:?}", wire_path);
    }

    let mut normalized = PathBuf::new();
    for component in path.components() {
        match component {
            Component::Prefix(_) => {
                bail!(
                    "path has prefix component (Windows drive/UNC): {:?}",
                    wire_path
                );
            }
            Component::RootDir => {
                bail!("path has root component: {:?}", wire_path);
            }
            Component::ParentDir => {
                bail!("path has `..` component: {:?}", wire_path);
            }
            Component::CurDir => {
                // Strip `.` components silently. `./foo/bar` → `foo/bar`.
            }
            Component::Normal(part) => {
                normalized.push(part);
            }
        }
    }

    // R1-F3 (followup_review_2026-05-02): empty input is the legitimate
    // single-file destination case (`""` → use root verbatim). A non-
    // empty input that normalizes to nothing — `"."`, `"./"`, `"./."` —
    // is a directory hint, not a file path. Receivers expecting a file
    // path should reject these to avoid conflating them with the
    // single-file empty case.
    if normalized.as_os_str().is_empty() && !wire_path.is_empty() {
        bail!(
            "path normalizes to empty (only `.` components): {:?}",
            wire_path
        );
    }

    Ok(normalized)
}

/// Safely join a wire-supplied relative path under a destination root.
///
/// Validates the path with `validate_wire_path`, then joins. An empty
/// wire path returns `root` unchanged — this is the load-bearing
/// single-file-destination case where `root` is itself the final file
/// path; `PathBuf::join("")` would otherwise append a trailing
/// separator that `File::create` rejects with `ENOTDIR`.
pub fn safe_join(root: &Path, wire_path: &str) -> Result<PathBuf> {
    let validated = validate_wire_path(wire_path)?;
    if validated.as_os_str().is_empty() {
        Ok(root.to_path_buf())
    } else {
        Ok(root.join(validated))
    }
}

/// Detect strings that represent Windows-absolute paths regardless of
/// the host platform. This catches forms that `Path::components` on
/// Unix does not flag (because `C:` and `\` are normal characters
/// there, and `\foo` parses as a single path component).
///
/// Recognized:
///   - `\\?\...` and `\\.\...` (NT and DOS device paths)
///   - `\\server\share\...` (UNC paths)
///   - `\foo`, `\foo\bar`, bare `\` (Windows-root-shaped — caught
///     here so receivers running Unix don't accept them as relative
///     filenames; R1-F1 of `docs/reviews/followup_review_2026-05-02.md`)
///   - `//server/share/...` (UNC-shaped with forward slashes)
///   - `C:\...`, `C:/...`, or just `C:` for any letter (drive-relative)
fn looks_like_windows_absolute(s: &str) -> bool {
    // UNC and NT/DOS device paths.
    if s.starts_with("\\\\") || s.starts_with("//") {
        return true;
    }
    // Single leading backslash. On Unix this is a normal char as far as
    // Path::components is concerned, but in the protocol context this
    // is a Windows-shaped root path (e.g. `\Windows\System32`) and
    // should be rejected uniformly across hosts.
    if s.starts_with('\\') {
        return true;
    }
    // Drive-letter forms: at least 2 chars where chars[0] is ASCII alpha
    // and chars[1] is ':'.
    let bytes = s.as_bytes();
    if bytes.len() >= 2 && bytes[1] == b':' && bytes[0].is_ascii_alphabetic() {
        return true;
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_passes_as_empty() {
        assert_eq!(validate_wire_path("").unwrap(), PathBuf::new());
    }

    #[test]
    fn safe_join_empty_returns_root() {
        let root = Path::new("/dest/file.txt");
        assert_eq!(
            safe_join(root, "").unwrap(),
            PathBuf::from("/dest/file.txt")
        );
    }

    #[test]
    fn simple_relative_passes() {
        assert_eq!(
            validate_wire_path("foo/bar.txt").unwrap(),
            PathBuf::from("foo/bar.txt")
        );
    }

    #[test]
    fn safe_join_simple() {
        let root = Path::new("/dest");
        assert_eq!(
            safe_join(root, "foo/bar.txt").unwrap(),
            PathBuf::from("/dest/foo/bar.txt")
        );
    }

    #[test]
    fn dot_components_stripped() {
        assert_eq!(
            validate_wire_path("./foo/./bar").unwrap(),
            PathBuf::from("foo/bar")
        );
    }

    #[test]
    fn parent_dir_rejected() {
        assert!(validate_wire_path("../etc/passwd").is_err());
        assert!(validate_wire_path("foo/../etc/passwd").is_err());
        assert!(validate_wire_path("foo/bar/..").is_err());
    }

    #[test]
    fn parent_dir_inside_filename_allowed() {
        // `foo..bar.txt` is a legitimate filename; only a `..` *component*
        // is dangerous. The component split on `/` keeps these intact.
        assert_eq!(
            validate_wire_path("foo..bar.txt").unwrap(),
            PathBuf::from("foo..bar.txt")
        );
        assert_eq!(
            validate_wire_path("dir/foo..bar/baz").unwrap(),
            PathBuf::from("dir/foo..bar/baz")
        );
    }

    #[test]
    fn unix_absolute_rejected() {
        assert!(validate_wire_path("/etc/passwd").is_err());
        assert!(validate_wire_path("/").is_err());
    }

    #[test]
    fn windows_drive_letter_rejected() {
        assert!(validate_wire_path("C:\\evil").is_err());
        assert!(validate_wire_path("C:/evil").is_err());
        assert!(validate_wire_path("c:foo").is_err()); // drive-relative
        assert!(validate_wire_path("Z:\\").is_err());
    }

    #[test]
    fn windows_unc_rejected() {
        assert!(validate_wire_path("\\\\server\\share\\file").is_err());
        assert!(validate_wire_path("\\\\?\\C:\\evil").is_err());
        assert!(validate_wire_path("\\\\.\\pipe\\evil").is_err());
        // `//server/share` is also UNC-shaped on Windows.
        assert!(validate_wire_path("//server/share/file").is_err());
    }

    /// R1-F1 (followup_review_2026-05-02): single leading backslash is
    /// Windows-root-shaped and must be rejected on Unix hosts where
    /// `Path::components` would otherwise treat it as a normal
    /// component.
    #[test]
    fn single_leading_backslash_rejected() {
        assert!(validate_wire_path("\\Windows\\System32").is_err());
        assert!(validate_wire_path("\\tmp\\file").is_err());
        assert!(validate_wire_path("\\foo").is_err());
        assert!(validate_wire_path("\\").is_err());
    }

    #[test]
    fn nul_byte_rejected() {
        assert!(validate_wire_path("foo\0bar").is_err());
        assert!(validate_wire_path("\0").is_err());
    }

    #[test]
    fn safe_join_rejects_traversal() {
        let root = Path::new("/dest");
        assert!(safe_join(root, "../escape").is_err());
        assert!(safe_join(root, "/etc/passwd").is_err());
        assert!(safe_join(root, "C:\\evil").is_err());
    }

    #[test]
    fn deep_relative_passes() {
        let root = Path::new("/dest");
        assert_eq!(
            safe_join(root, "a/b/c/d/e.txt").unwrap(),
            PathBuf::from("/dest/a/b/c/d/e.txt")
        );
    }

    #[test]
    fn unicode_filename_passes() {
        let root = Path::new("/dest");
        assert_eq!(
            safe_join(root, "résumé/日本語/file.txt").unwrap(),
            PathBuf::from("/dest/résumé/日本語/file.txt")
        );
    }

    #[test]
    fn trailing_slash_normalizes() {
        // `foo/bar/` is a directory hint — Path::components yields
        // ["foo", "bar"] without a trailing CurDir, so this is just
        // foo/bar after normalization.
        assert_eq!(
            validate_wire_path("foo/bar/").unwrap(),
            PathBuf::from("foo/bar")
        );
    }

    /// R1-F3 (followup_review_2026-05-02): `"."` and `"./"` are
    /// directory-hint forms, not file paths. They must not be conflated
    /// with the legitimate empty single-file destination case (`""`),
    /// so a non-empty input that normalizes to zero normal components
    /// is rejected.
    #[test]
    fn just_dot_rejected() {
        assert!(validate_wire_path(".").is_err());
        assert!(validate_wire_path("./").is_err());
        assert!(validate_wire_path("./.").is_err());
    }
}
