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
//! ─── Contract: two layers of safety ──────────────────────────────
//!
//! `safe_join(root, wire)` performs a *lexical* containment check.
//! It rejects path strings that escape via syntax (above), but does
//! NOT canonicalize the result or resolve symlinks.
//!
//! `contained_join(canonical_root, wire)` adds the canonicalize-
//! and-check pass on top of `safe_join`'s lexical layer. It walks
//! to the deepest existing ancestor of the target, canonicalizes,
//! and confirms the resolution stays under `canonical_root`. Use it
//! at every daemon site that touches a path under a module root
//! (F2 of `docs/reviews/codebase_review_2026-05-01.md`).
//!
//! Both layers are needed:
//!
//!   - `safe_join` alone fails on `module/link/file` where `link` is
//!     a symlink to `/etc`. Lexically the wire path is fine; the
//!     filesystem write follows the symlink outside the module.
//!     `contained_join` rejects this case via canonicalize.
//!
//!   - `contained_join` alone is more permissive about wire syntax
//!     than `safe_join` would be. Always run the wire input through
//!     `validate_wire_path` (which `contained_join` does internally
//!     via `safe_join`) so absolute / Windows-shaped / NUL inputs
//!     are rejected before any filesystem call.
//!
//! TOCTOU note: `contained_join` is check-then-use, so a symlink
//! could in principle be swapped between the canonicalize call and
//! the actual fs op. For the trust model (authenticated peers,
//! operator-controlled module roots) this matches rsync's chroot
//! module behavior. A fully race-proof alternative would use
//! `openat` + `O_NOFOLLOW` per-component descent.

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

/// Resolve a wire-supplied relative path under a daemon module root
/// AND verify the resolved location stays inside that root after
/// symlink resolution. Returns the lexical target path (not the
/// canonicalized one) so callers write to the path they expect.
///
/// This is the F2 chokepoint from
/// `docs/reviews/codebase_review_2026-05-01.md`. `safe_join` is
/// lexical — it rejects `../`, absolute paths, etc. — but does not
/// follow symlinks. A module that contains `module_root/link`
/// pointing at `/etc` would let a wire request for `link/passwd`
/// pass `safe_join` and then have the daemon read `/etc/passwd`.
/// `contained_join` closes that gap by canonicalizing the deepest
/// existing ancestor of the target and confirming it stays under
/// `canonical_module_root`.
///
/// `canonical_module_root` MUST already be the canonicalized form
/// (the daemon canonicalizes module paths at load time). The check
/// fails closed if either canonicalize call fails for a reason
/// other than NotFound.
///
/// Note: this is a check-then-use API with a TOCTOU window. Between
/// the canonicalize call and the actual filesystem operation, a
/// symlink within the parent could in principle be replaced. The
/// fully race-proof alternative would be openat(2) + O_NOFOLLOW
/// per-component descent, which is significantly more code. For
/// the threat model — authenticated peers and operator-trusted
/// module roots — the canonicalize-and-check approach matches
/// rsync's chroot module behavior and forecloses the practical
/// attack vector (a module containing pre-existing escape symlinks).
pub fn contained_join(canonical_module_root: &Path, wire_path: &str) -> Result<PathBuf> {
    let target = safe_join(canonical_module_root, wire_path)?;

    // Walk to the deepest existing ancestor and canonicalize. For a
    // read of an existing file, that's the file itself; for a write
    // creating a new file or directory tree, it's the deepest dir
    // that already exists.
    let mut probe: PathBuf = target.clone();
    let canonical_ancestor = loop {
        match std::fs::canonicalize(&probe) {
            Ok(c) => break c,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                if !probe.pop() {
                    bail!(
                        "path '{}' has no canonicalizable ancestor (root '{}' missing?)",
                        target.display(),
                        canonical_module_root.display()
                    );
                }
            }
            Err(e) => {
                bail!(
                    "canonicalize '{}' for containment check: {}",
                    probe.display(),
                    e
                );
            }
        }
    };

    if !canonical_ancestor.starts_with(canonical_module_root) {
        bail!(
            "path '{}' resolves to '{}' which escapes module root '{}'",
            target.display(),
            canonical_ancestor.display(),
            canonical_module_root.display()
        );
    }

    Ok(target)
}

/// Verify that an already-built absolute path stays inside
/// `canonical_module_root` after symlink resolution. Used by call
/// sites that received their path from `safe_join` upstream and
/// want the F2 containment check without redoing the wire-path
/// validation. Same semantics as `contained_join` but takes the
/// already-resolved `target` directly.
pub fn verify_contained(canonical_module_root: &Path, target: &Path) -> Result<()> {
    let mut probe: PathBuf = target.to_path_buf();
    let canonical_ancestor = loop {
        match std::fs::canonicalize(&probe) {
            Ok(c) => break c,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                if !probe.pop() {
                    bail!(
                        "path '{}' has no canonicalizable ancestor (root '{}' missing?)",
                        target.display(),
                        canonical_module_root.display()
                    );
                }
            }
            Err(e) => {
                bail!(
                    "canonicalize '{}' for containment check: {}",
                    probe.display(),
                    e
                );
            }
        }
    };
    if !canonical_ancestor.starts_with(canonical_module_root) {
        bail!(
            "path '{}' resolves to '{}' which escapes module root '{}'",
            target.display(),
            canonical_ancestor.display(),
            canonical_module_root.display()
        );
    }
    Ok(())
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

#[cfg(unix)]
#[cfg(test)]
mod containment_tests {
    //! F2 / `contained_join` regression tests. These exercise the
    //! canonicalize-and-check layer that sits on top of `safe_join`,
    //! using real symlinks. Unix-only — the tests use
    //! `std::os::unix::fs::symlink`, and the Windows daemon path
    //! semantics warrant their own test pass.

    use super::*;
    use std::os::unix::fs::symlink;
    use tempfile::tempdir;

    /// Helper: build a canonicalized module root inside a tempdir.
    fn module_root(tmp: &std::path::Path) -> PathBuf {
        let root = tmp.join("module");
        std::fs::create_dir_all(&root).unwrap();
        std::fs::canonicalize(&root).unwrap()
    }

    #[test]
    fn contained_join_accepts_simple_relative() {
        let tmp = tempdir().unwrap();
        let root = module_root(tmp.path());
        std::fs::create_dir_all(root.join("sub")).unwrap();
        std::fs::write(root.join("sub/file.txt"), b"x").unwrap();

        let target = contained_join(&root, "sub/file.txt").unwrap();
        assert_eq!(target, root.join("sub/file.txt"));
    }

    #[test]
    fn contained_join_accepts_nonexistent_target_inside_root() {
        // Writes need to work for paths that don't exist yet — the
        // helper walks up to the deepest existing ancestor (which is
        // `root` itself for a fresh module).
        let tmp = tempdir().unwrap();
        let root = module_root(tmp.path());
        let target = contained_join(&root, "newfile.txt").unwrap();
        assert_eq!(target, root.join("newfile.txt"));
    }

    #[test]
    fn contained_join_rejects_symlink_escaping_root() {
        // The classic F2 attack: a symlink inside the module points
        // outside, and a wire request for `link/passwd` would have
        // had the lexical safe_join layer happily return the join.
        let tmp = tempdir().unwrap();
        let root = module_root(tmp.path());
        let outside = tmp.path().join("outside");
        std::fs::create_dir_all(&outside).unwrap();
        std::fs::write(outside.join("victim.txt"), b"secret").unwrap();
        symlink(&outside, root.join("escape")).unwrap();

        let err = contained_join(&root, "escape/victim.txt").unwrap_err();
        let msg = err.to_string();
        assert!(
            msg.contains("escapes module root"),
            "expected escape rejection, got: {msg}"
        );
    }

    #[test]
    fn contained_join_rejects_top_level_symlink_to_outside() {
        // Direct symlink at the module root pointing outside.
        let tmp = tempdir().unwrap();
        let root = module_root(tmp.path());
        let outside = tmp.path().join("outside");
        std::fs::create_dir_all(&outside).unwrap();
        symlink(&outside, root.join("escape")).unwrap();

        let err = contained_join(&root, "escape").unwrap_err();
        assert!(err.to_string().contains("escapes module root"));
    }

    #[test]
    fn contained_join_accepts_intra_root_symlink() {
        // Symlinks INSIDE the module root that don't escape should
        // still work — operators legitimately use intra-module
        // symlinks (e.g., `latest -> v1.2.3`).
        let tmp = tempdir().unwrap();
        let root = module_root(tmp.path());
        std::fs::create_dir_all(root.join("v1")).unwrap();
        std::fs::write(root.join("v1/file.txt"), b"hi").unwrap();
        symlink(root.join("v1"), root.join("latest")).unwrap();

        let target = contained_join(&root, "latest/file.txt").unwrap();
        // Returns the lexical target so the caller writes/reads
        // through the symlink as expected.
        assert_eq!(target, root.join("latest/file.txt"));
    }

    #[test]
    fn contained_join_rejects_nonexistent_symlink_parent_escape() {
        // Write path: target file doesn't exist yet, but its parent
        // is a symlink pointing outside the root. The deepest
        // existing ancestor walk lands on the symlink, which
        // canonicalizes outside the root.
        let tmp = tempdir().unwrap();
        let root = module_root(tmp.path());
        let outside = tmp.path().join("outside");
        std::fs::create_dir_all(&outside).unwrap();
        symlink(&outside, root.join("escape")).unwrap();

        // Attempt to "create" escape/newfile.txt — the parent
        // (escape) canonicalizes outside.
        let err = contained_join(&root, "escape/newfile.txt").unwrap_err();
        assert!(err.to_string().contains("escapes module root"));
    }

    #[test]
    fn contained_join_rejects_lexical_traversal() {
        // The lexical layer (validate_wire_path) catches `..`
        // before we even reach the canonicalize step.
        let tmp = tempdir().unwrap();
        let root = module_root(tmp.path());
        assert!(contained_join(&root, "../escape").is_err());
    }

    #[test]
    fn contained_join_accepts_empty_wire_path() {
        // The empty wire path is the legitimate single-file source
        // case; safe_join returns root unchanged, and root is by
        // definition contained.
        let tmp = tempdir().unwrap();
        let root = module_root(tmp.path());
        let target = contained_join(&root, "").unwrap();
        assert_eq!(target, root);
    }

    #[test]
    fn verify_contained_passes_for_inside_path() {
        let tmp = tempdir().unwrap();
        let root = module_root(tmp.path());
        std::fs::write(root.join("ok.txt"), b"x").unwrap();
        verify_contained(&root, &root.join("ok.txt")).unwrap();
    }

    #[test]
    fn verify_contained_rejects_escape() {
        let tmp = tempdir().unwrap();
        let root = module_root(tmp.path());
        let outside = tmp.path().join("outside");
        std::fs::create_dir_all(&outside).unwrap();
        symlink(&outside, root.join("escape")).unwrap();
        let err = verify_contained(&root, &root.join("escape/anything")).unwrap_err();
        assert!(err.to_string().contains("escapes module root"));
    }
}
