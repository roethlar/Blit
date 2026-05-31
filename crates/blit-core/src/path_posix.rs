//! Canonical POSIX-form rendering of relative paths for the wire /
//! manifest layer.
//!
//! Every place blit converts a local `Path` (the OS filesystem walker's
//! native representation) into the wire/manifest's forward-slash string
//! form must route through [`relative_path_to_posix`].
//!
//! ## Why a helper, not a string `replace('\\', "/")`?
//!
//! Earlier code did `path.to_string_lossy().replace('\\', "/")` to
//! convert Windows-native `\` separators to `/` for the wire. That works
//! "by accident" on Windows but is **destructive on POSIX**, where `\`
//! is a legal filename character (e.g. macOS apps like Logic Pro ship
//! plug-in presets named `1\4 Single.pst`, with a literal backslash). A
//! blanket string-`replace` rewrites that filename character to `/`,
//! creating a phantom path component that doesn't exist on disk; the
//! manifest lookup then misses and the transfer fails (`tar shard
//! produced unexpected entry … (not in manifest)`).
//!
//! Walking [`Path::components`] is the right primitive: on Windows it
//! splits on the native `\` (or `/`), on POSIX it splits on `/` only.
//! Joining the resulting components with `/` produces the canonical
//! POSIX form on every platform, and any literal `\` (or `:`, or other
//! non-separator byte) inside a single component survives untouched.

use std::path::Path;

/// Render a relative `Path` as a forward-slash-joined POSIX-form string
/// for the wire / manifest. Components are joined via
/// [`Path::components`], so the result is correct on both POSIX (where
/// `\` is a legal filename character) and Windows (where `\` is the
/// native separator).
///
/// Empty paths and bare `.` produce an empty string — the convention
/// the daemon wire format uses for "the module root".
pub fn relative_path_to_posix(path: &Path) -> String {
    if path.as_os_str().is_empty() || path == Path::new(".") {
        return String::new();
    }
    path.components()
        .map(|c| c.as_os_str().to_string_lossy())
        .collect::<Vec<_>>()
        .join("/")
}

/// Render a relative path expressed as a string (e.g. user input or a
/// wire-supplied value of unknown provenance) as canonical POSIX form,
/// applying the same component-aware semantics as
/// [`relative_path_to_posix`]. On Windows, a user-typed `Folder\file` is
/// split on the native separator; on POSIX, `Folder\file` is a single
/// filename component and is preserved verbatim.
///
/// **Trailing-separator semantics are preserved** because for user
/// input the trailing separator is a meaningful UX signal — "this
/// prefix names a directory; complete *inside* it" (the rsync /
/// shell-completion convention). `Path::components()` strips trailing
/// separators, so we detect them on the raw input and re-attach a
/// canonical `/` after the canonicalization. On POSIX a trailing
/// backslash is **not** a separator (it's a literal filename byte) and
/// stays as part of the last component; on Windows a trailing native
/// `\` IS a separator and round-trips as `/`. Round-1 reopen
/// (GPT review) — `sub/` was canonicalizing to `sub`, which broke
/// `split_completion_prefix`'s "list inside sub/" path.
pub fn relative_str_to_posix(s: &str) -> String {
    let trailing_sep = s
        .chars()
        .next_back()
        .map(std::path::is_separator)
        .unwrap_or(false);
    let canonical = relative_path_to_posix(Path::new(s));
    if trailing_sep && !canonical.is_empty() && !canonical.ends_with('/') {
        let mut out = canonical;
        out.push('/');
        out
    } else {
        canonical
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn empty_and_dot_produce_empty_string() {
        assert_eq!(relative_path_to_posix(Path::new("")), "");
        assert_eq!(relative_path_to_posix(Path::new(".")), "");
    }

    #[test]
    fn simple_posix_path_is_unchanged() {
        assert_eq!(relative_path_to_posix(Path::new("a/b/c.txt")), "a/b/c.txt");
    }

    #[test]
    fn nested_relative_path_via_pathbuf() {
        let p: PathBuf = ["foo", "bar", "baz.txt"].iter().collect();
        assert_eq!(relative_path_to_posix(&p), "foo/bar/baz.txt");
    }

    /// **Regression**: the user-reported `blit mirror` failure on macOS
    /// where a Logic Pro plug-in preset named `1\4 Single.pst` (literal
    /// backslash) tripped the tar-shard safety check because a blanket
    /// `replace('\\', "/")` was destroying the filename character.
    ///
    /// On POSIX, `\` is a legal filename byte and a single-component
    /// path containing `\` must round-trip unchanged.
    #[cfg(not(windows))]
    #[test]
    fn posix_literal_backslash_in_filename_is_preserved() {
        let p = Path::new("Echo/1\\4 Single.pst");
        // One component is "Echo", the next is "1\4 Single.pst" — the
        // literal backslash stays as a filename character.
        assert_eq!(relative_path_to_posix(p), "Echo/1\\4 Single.pst");
    }

    /// On POSIX, a literal `:` is a legal filename byte too. (macOS HFS+
    /// / APFS expose `:` to userland under the same rules as any other
    /// byte; the `:` ↔ `/` Finder swap is presentation-layer only.)
    #[cfg(not(windows))]
    #[test]
    fn posix_literal_colon_in_filename_is_preserved() {
        let p = Path::new("Themes/Dark:Variant.toml");
        assert_eq!(relative_path_to_posix(p), "Themes/Dark:Variant.toml");
    }

    /// On Windows, `Path::components` splits on the native `\` separator
    /// (and also accepts `/`). The join with `/` converts the path to
    /// canonical POSIX form for the wire.
    #[cfg(windows)]
    #[test]
    fn windows_native_separators_become_forward_slashes() {
        let p = Path::new("Folder\\sub\\file.txt");
        assert_eq!(relative_path_to_posix(p), "Folder/sub/file.txt");
    }

    #[test]
    fn already_posix_input_via_relative_str() {
        assert_eq!(relative_str_to_posix("a/b/c"), "a/b/c");
    }

    /// **Round-1 reopen regression (GPT)**: shell completions like
    /// `sub/` MUST preserve the trailing `/` — it's the UX signal
    /// "complete inside `sub/`". Earlier the str helper went through
    /// `Path::components()` which drops trailing separators, turning
    /// `sub/` into `sub` and routing `split_completion_prefix` to look
    /// for module-root entries starting with `sub` instead of listing
    /// inside the directory.
    #[test]
    fn relative_str_preserves_trailing_slash() {
        assert_eq!(relative_str_to_posix("sub/"), "sub/");
        assert_eq!(relative_str_to_posix("Folder/sub/"), "Folder/sub/");
    }

    /// No trailing slash means no trailing slash — the helper must not
    /// invent one. This is the "complete things STARTING WITH `sub`"
    /// path (vs "complete inside `sub/`").
    #[test]
    fn relative_str_does_not_invent_trailing_slash() {
        assert_eq!(relative_str_to_posix("sub"), "sub");
        assert_eq!(relative_str_to_posix("Folder/file"), "Folder/file");
    }

    /// On POSIX, a literal `\` byte at the end of a filename is a
    /// legitimate filename character (NOT a separator) and must stay
    /// where it is — neither moved nor converted to `/`. (`is_separator`
    /// on POSIX returns false for `\`.)
    #[cfg(not(windows))]
    #[test]
    fn relative_str_preserves_trailing_literal_backslash_on_posix() {
        assert_eq!(relative_str_to_posix("sub\\"), "sub\\");
        assert_eq!(
            relative_str_to_posix("Echo/1\\4 Single.pst"),
            "Echo/1\\4 Single.pst"
        );
    }

    /// On Windows, a trailing native `\` IS a separator and must
    /// canonicalize to a trailing `/`, mirroring `sub/` semantics.
    #[cfg(windows)]
    #[test]
    fn relative_str_converts_trailing_native_separator_on_windows() {
        assert_eq!(relative_str_to_posix("Folder\\"), "Folder/");
        assert_eq!(relative_str_to_posix("Folder\\sub\\"), "Folder/sub/");
    }

    /// Idempotent: running the canonical helper on its own output is a
    /// no-op (no further normalization happens). This is the property
    /// that lets callers safely apply it at boundaries without worrying
    /// about double-conversion.
    #[test]
    fn idempotent() {
        let once = relative_path_to_posix(Path::new("a/b/c.txt"));
        let twice = relative_path_to_posix(Path::new(&once));
        assert_eq!(once, twice);
    }
}
