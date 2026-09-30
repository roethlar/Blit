//! Non-UTF-8 relative names on the wire (contract v7,
//! `FileHeader.raw_relative_path`; `docs/plan/SOURCE_SIDE_CONTAINMENT.md`
//! D-F, owner ruling D-2026-09-29-3 — rsync parity).
//!
//! The manifest identity of every file is the lossy UTF-8 text in
//! `FileHeader.relative_path`. When a source name has a component that is
//! not valid UTF-8, the scan ALSO carries the exact source bytes (with `/`
//! separators) in `raw_relative_path`, and:
//!
//! * the source opens the file by those bytes, never by the lossy text
//!   (which names nothing on disk);
//! * a destination that can hold the bytes creates the real name;
//! * a destination that cannot reports the file at manifest intake with
//!   [`DESTINATION_CANNOT_STORE_REASON`] and never grants it.
//!
//! The bytes are never a key: every map, need, record, skip and terminator
//! stays keyed by the text.

use std::ffi::OsStr;
use std::path::{Path, PathBuf};

use crate::generated::FileHeader;

/// Reason recorded at manifest intake for a name the destination cannot
/// store (A13). Exact wording is pinned.
pub const DESTINATION_CANNOT_STORE_REASON: &str =
    "source: filename is not valid UTF-8; the destination cannot store it (rename it to transfer)";

/// Reason recorded for the second of two manifest entries that collapse
/// to the same text path (A13). Exact prefix is pinned; the raw bytes of
/// the rejected entry follow it when known.
pub const DUPLICATE_MANIFEST_PATH_REASON: &str =
    "source: duplicate manifest path (lossy name collision)";

/// The exact bytes of `rel` (a relative path from the scan) joined with
/// `/`, iff at least one component is not valid UTF-8. `None` for every
/// representable name, so the common case costs nothing on the wire.
pub fn raw_relative_bytes(rel: &Path) -> Option<Vec<u8>> {
    let mut lossy = false;
    let mut bytes: Vec<u8> = Vec::new();
    for (i, component) in rel.components().enumerate() {
        let part = component.as_os_str();
        if part.to_str().is_none() {
            lossy = true;
        }
        if i > 0 {
            bytes.push(b'/');
        }
        bytes.extend_from_slice(part.as_encoded_bytes());
    }
    lossy.then_some(bytes)
}

/// Whether THIS process, as a destination, can create a name from raw
/// bytes: Linux and the BSDs accept any bytes; macOS (APFS/HFS+) rejects
/// invalid UTF-8 with `EILSEQ`; Windows names are UTF-16 and cannot hold
/// arbitrary bytes at all. The create-time errno remains the per-file
/// backstop ([`is_unrepresentable_name_error`]).
pub const fn destination_can_store_raw_names() -> bool {
    cfg!(all(unix, not(target_os = "macos")))
}

/// Whether `error` from a create/open is the filesystem refusing the
/// bytes of the name itself (`EILSEQ`, or `EINVAL` from a filesystem that
/// validates names) rather than an ordinary I/O failure.
pub fn is_unrepresentable_name_error(error: &std::io::Error) -> bool {
    #[cfg(unix)]
    {
        matches!(
            error.raw_os_error(),
            Some(code) if code == libc::EILSEQ || code == libc::EINVAL
        )
    }
    #[cfg(not(unix))]
    {
        let _ = error;
        false
    }
}

/// Rebuild the path a raw-name header names on THIS host's filesystem.
/// Unix: the bytes verbatim. Windows: the bytes are the WTF-8 this same
/// process's scan produced, so they round-trip exactly.
pub fn path_from_raw(raw: &[u8]) -> PathBuf {
    #[cfg(unix)]
    {
        use std::os::unix::ffi::OsStrExt as _;
        PathBuf::from(OsStr::from_bytes(raw))
    }
    #[cfg(windows)]
    {
        // SAFETY: the only producer of `raw_relative_path` on a Windows
        // source is `raw_relative_bytes` in this same process, which
        // took the bytes from `OsStr::as_encoded_bytes`; the Windows
        // destination never creates from raw bytes
        // (`destination_can_store_raw_names` is false), so every byte
        // string that reaches here is this host's own WTF-8 encoding.
        // Split on the wire separator and rebuild with the native one.
        let mut out = PathBuf::new();
        for piece in raw.split(|b| *b == b'/') {
            let os_piece: &OsStr = unsafe { OsStr::from_encoded_bytes_unchecked(piece) };
            out.push(os_piece);
        }
        out
    }
    #[cfg(not(any(unix, windows)))]
    {
        PathBuf::from(String::from_utf8_lossy(raw).into_owned())
    }
}

/// The on-disk path of `header` under `root`: by raw bytes when the name
/// is not representable as text, by the text otherwise. An empty
/// relative path means "`root` is itself the file".
pub fn source_path(root: &Path, header: &FileHeader) -> PathBuf {
    if let Some(raw) = header.raw_relative_path.as_deref() {
        return root.join(path_from_raw(raw));
    }
    if header.relative_path.is_empty() {
        root.to_path_buf()
    } else {
        root.join(&header.relative_path)
    }
}

/// Render raw name bytes for a report: printable ASCII as-is, every other
/// byte as `\xNN`, so two names that collapse to one lossy text can still
/// be told apart in the failure block.
pub fn escape_raw(raw: &[u8]) -> String {
    let mut out = String::with_capacity(raw.len());
    for &b in raw {
        if (0x20..0x7f).contains(&b) && b != b'\\' {
            out.push(b as char);
        } else {
            out.push_str(&format!("\\x{b:02x}"));
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn representable_names_carry_no_raw_bytes() {
        assert_eq!(raw_relative_bytes(Path::new("sub/café.txt")), None);
        assert_eq!(raw_relative_bytes(Path::new("")), None);
    }

    #[cfg(unix)]
    #[test]
    fn a_non_utf8_component_carries_the_exact_bytes_with_slash_separators() {
        use std::os::unix::ffi::OsStrExt as _;
        let rel = Path::new(OsStr::from_bytes(b"sub/caf\xe9.txt"));
        assert_eq!(raw_relative_bytes(rel), Some(b"sub/caf\xe9.txt".to_vec()));
        assert_eq!(
            path_from_raw(b"sub/caf\xe9.txt"),
            PathBuf::from(OsStr::from_bytes(b"sub/caf\xe9.txt"))
        );
    }

    #[cfg(unix)]
    #[test]
    fn source_path_prefers_the_raw_bytes_over_the_lossy_text() {
        use std::os::unix::ffi::OsStrExt as _;
        let header = FileHeader {
            relative_path: "sub/caf\u{fffd}.txt".into(),
            raw_relative_path: Some(b"sub/caf\xe9.txt".to_vec()),
            ..Default::default()
        };
        assert_eq!(
            source_path(Path::new("/root"), &header),
            PathBuf::from(OsStr::from_bytes(b"/root/sub/caf\xe9.txt"))
        );
        let plain = FileHeader {
            relative_path: "sub/plain.txt".into(),
            ..Default::default()
        };
        assert_eq!(
            source_path(Path::new("/root"), &plain),
            PathBuf::from("/root/sub/plain.txt")
        );
        let root_file = FileHeader::default();
        assert_eq!(
            source_path(Path::new("/root/f"), &root_file),
            PathBuf::from("/root/f")
        );
    }

    #[test]
    fn escape_raw_makes_collapsed_names_distinguishable() {
        assert_eq!(escape_raw(b"caf\xe9.txt"), "caf\\xe9.txt");
        assert_eq!(
            escape_raw("caf\u{fffd}.txt".as_bytes()),
            "caf\\xef\\xbf\\xbd.txt"
        );
    }
}
