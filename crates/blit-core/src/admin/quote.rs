//! A name in a message, written so that it can be read back
//! (`docs/plan/UTILITY_VERBS_LOCAL_PARITY.md`, cr-uv2-5).
//!
//! A name need not be text, and a message is: `rm`, `du`, `df` and `find`
//! each put a name in what they say by a rule that loses nothing of it,
//! and these are those rules — measured from GNU coreutils 9.12 and
//! findutils 4.11.0 under `C.UTF-8`, name by name. Shown as text with a
//! replacement character, a name that is not text read as ANOTHER name:
//! `bad\xff.dat` and the file really called `bad\u{fffd}.dat` were told of
//! in the same words.
//!
//! What is printable is the reference locale's `print`
//! ([`crate::find_name::is_printable`]); a byte that is part of no
//! character is not.

use crate::find_name::is_printable;

/// One piece of a name: a character, or a byte that is part of none.
enum Piece<'a> {
    /// A character and the bytes that spell it.
    Character(char, &'a [u8]),
    /// A byte that is no part of any character.
    Stray(u8),
}

/// The name read as characters where it is text, byte by byte where not.
fn pieces(name: &[u8]) -> impl Iterator<Item = Piece<'_>> {
    name.utf8_chunks().flat_map(|chunk| {
        let valid = chunk.valid();
        valid
            .char_indices()
            .map(move |(at, character)| {
                Piece::Character(character, &valid.as_bytes()[at..at + character.len_utf8()])
            })
            .chain(chunk.invalid().iter().map(|&byte| Piece::Stray(byte)))
    })
}

/// The letter C gives a control character after a backslash, where it has
/// one: what both quoting rules write in place of three octal digits.
fn c_escape(byte: u8) -> Option<char> {
    Some(match byte {
        0x07 => 'a',
        0x08 => 'b',
        0x0c => 'f',
        b'\n' => 'n',
        b'\r' => 'r',
        b'\t' => 't',
        0x0b => 'v',
        _ => return None,
    })
}

/// `\` and what stands for `byte` after it.
fn escape_into(out: &mut String, byte: u8) {
    out.push('\\');
    match c_escape(byte) {
        Some(letter) => out.push(letter),
        None => out.push_str(&format!("{byte:03o}")),
    }
}

/// A name as `rm` and `du` write it in a message — always in quotes — and,
/// with `always` false, as `df` writes it: bare when a shell would read it
/// as the one word it is.
///
/// It is the form a shell reads back: `'plain'`; `"it's"` for a name with
/// an apostrophe and nothing else a shell minds; and what cannot be
/// printed spliced in as `$'\377'` — `'bad'$'\377''.dat'`.
pub fn for_shell(name: &[u8], always: bool) -> String {
    // What a shell reads specially, anywhere in a word or at its start.
    let special = |at: usize, byte: u8| {
        matches!(
            byte,
            b' ' | b'!'
                | b'"'
                | b'$'
                | b'&'
                | b'\''
                | b'('
                | b')'
                | b'*'
                | b';'
                | b'<'
                | b'='
                | b'>'
                | b'?'
                | b'['
                | b'\\'
                | b'^'
                | b'`'
                | b'|'
        ) || (at == 0 && matches!(byte, b'#' | b'~'))
            || (name.len() == 1 && matches!(byte, b'{' | b'}'))
    };
    // What reads the same between double quotes, in C and in a shell.
    let plain = |byte: u8| {
        byte.is_ascii_alphanumeric()
            || matches!(
                byte,
                b' ' | b'\'' | b'%' | b'+' | b',' | b'-' | b'.' | b'/' | b':' | b']' | b'_' | b'@'
            )
    };
    let printable = |piece: &Piece<'_>| match piece {
        Piece::Character(character, _) => is_printable(*character),
        Piece::Stray(_) => false,
    };
    let (mut bare, mut apostrophe, mut all_plain) = (!name.is_empty(), false, true);
    let mut at = 0;
    for piece in pieces(name) {
        match &piece {
            Piece::Character(character, bytes) if character.is_ascii() => {
                let byte = bytes[0];
                apostrophe |= byte == b'\'';
                bare &= printable(&piece) && !special(at, byte);
                all_plain &= printable(&piece) && plain(byte);
            }
            _ => {
                bare &= printable(&piece);
                all_plain &= printable(&piece);
            }
        }
        at += match &piece {
            Piece::Character(_, bytes) => bytes.len(),
            Piece::Stray(_) => 1,
        };
    }
    if bare && !always {
        return String::from_utf8_lossy(name).into_owned();
    }
    if apostrophe && all_plain {
        return format!("\"{}\"", String::from_utf8_lossy(name));
    }

    let mut out = String::from("'");
    // Inside `$'…'`, opened for what cannot be printed and closed before
    // the next thing that can.
    let mut escaping = false;
    for piece in pieces(name) {
        let unprintable = !printable(&piece);
        let bytes: &[u8] = match &piece {
            Piece::Character(_, bytes) => bytes,
            Piece::Stray(byte) => std::slice::from_ref(byte),
        };
        if unprintable {
            if !escaping {
                out.push_str("'$'");
                escaping = true;
            }
            for &byte in bytes {
                escape_into(&mut out, byte);
            }
            continue;
        }
        if bytes == b"'" {
            // Out of the quotes, an escaped apostrophe, and in again;
            // the first of the three closes `$'…'` when that is open.
            out.push_str("'\\''");
            escaping = false;
            continue;
        }
        if escaping {
            out.push_str("''");
            escaping = false;
        }
        out.push_str(std::str::from_utf8(bytes).expect("a character's own bytes"));
    }
    out.push('\'');
    out
}

/// A name as `find` writes it in a message: between `‘` and `’`, what
/// cannot be printed as a backslash and its octal digits — or C's letter
/// for it — and a backslash as two.
pub fn for_find(name: &[u8]) -> String {
    let mut out = String::from("\u{2018}");
    for piece in pieces(name) {
        match piece {
            Piece::Character('\\', _) => out.push_str("\\\\"),
            Piece::Character('\u{2019}', _) => out.push_str("\\\u{2019}"),
            Piece::Character(character, _) if is_printable(character) => out.push(character),
            Piece::Character(_, bytes) => {
                for &byte in bytes {
                    escape_into(&mut out, byte);
                }
            }
            Piece::Stray(byte) => escape_into(&mut out, byte),
        }
    }
    out.push('\u{2019}');
    out
}

/// cr-uv3-7: a name in an output that holds only text — `--json`. `entry`
/// gains `path`, the name as text, and — for a name that is NOT text —
/// `raw`, every byte of it: printable ASCII as itself, any other byte and
/// `\` as `\xNN` ([`crate::raw_name::escape_raw`], which
/// [`crate::raw_name::unescape_raw`] reads back). The text alone lost the
/// name: `bad\xff.dat` and the file really called `bad\u{fffd}.dat` were
/// the same string, twice.
pub fn in_json(name: &[u8], mut entry: serde_json::Value) -> serde_json::Value {
    entry["path"] = String::from_utf8_lossy(name).into_owned().into();
    if let Some(raw) = exact(name) {
        entry["raw"] = raw.into();
    }
    entry
}

/// The bytes of a name that is not text, as [`in_json`] writes them;
/// `None` for a name that is text, whose text is the name.
pub fn exact(name: &[u8]) -> Option<String> {
    std::str::from_utf8(name)
        .is_err()
        .then(|| crate::raw_name::escape_raw(name))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// cr-uv2-5: each line is what GNU `rm`, `df` and `find` wrote for a
    /// name that is not there (coreutils 9.12, findutils 4.11.0,
    /// `C.UTF-8`, 2026-11-21): `rm: cannot remove NAME`, `df: NAME`,
    /// `find: NAME`. `du` writes a name as `rm` does.
    #[test]
    fn a_name_is_written_as_the_reference_writes_it() {
        let cases: [(&[u8], &str, &str, &str); 24] = [
            (b"plain.txt", "'plain.txt'", "plain.txt", "‘plain.txt’"),
            (b"two words", "'two words'", "'two words'", "‘two words’"),
            (b"it's", "\"it's\"", "\"it's\"", "‘it's’"),
            (
                b"bad\xff.dat",
                "'bad'$'\\377''.dat'",
                "'bad'$'\\377''.dat'",
                "‘bad\\377.dat’",
            ),
            (b"\xff", "''$'\\377'", "''$'\\377'", "‘\\377’"),
            (
                b"a\xffb\xfec",
                "'a'$'\\377''b'$'\\376''c'",
                "'a'$'\\377''b'$'\\376''c'",
                "‘a\\377b\\376c’",
            ),
            (
                b"new\nline",
                "'new'$'\\n''line'",
                "'new'$'\\n''line'",
                "‘new\\nline’",
            ),
            (
                b"tab\there",
                "'tab'$'\\t''here'",
                "'tab'$'\\t''here'",
                "‘tab\\there’",
            ),
            (b"bell\x07", "'bell'$'\\a'", "'bell'$'\\a'", "‘bell\\a’"),
            (
                b"back\\slash",
                "'back\\slash'",
                "'back\\slash'",
                "‘back\\\\slash’",
            ),
            (
                b"quote\"double",
                "'quote\"double'",
                "'quote\"double'",
                "‘quote\"double’",
            ),
            (b"dollar$x", "'dollar$x'", "'dollar$x'", "‘dollar$x’"),
            ("café".as_bytes(), "'café'", "café", "‘café’"),
            (
                "em\u{2003}sp".as_bytes(),
                "'em\u{2003}sp'",
                "em\u{2003}sp",
                "‘em\u{2003}sp’",
            ),
            (
                b"ctl\x01\x02",
                "'ctl'$'\\001\\002'",
                "'ctl'$'\\001\\002'",
                "‘ctl\\001\\002’",
            ),
            (b"del\x7f", "'del'$'\\177'", "'del'$'\\177'", "‘del\\177’"),
            (b"-dash", "'-dash'", "-dash", "‘-dash’"),
            (b" lead", "' lead'", "' lead'", "‘ lead’"),
            (b"trail ", "'trail '", "'trail '", "‘trail ’"),
            (
                b"both\xff'q",
                "'both'$'\\377'\\''q'",
                "'both'$'\\377'\\''q'",
                "‘both\\377'q’",
            ),
            // The reviewer's: a Unix name with a backslash in it.
            (
                b"quarter\\notes.pst",
                "'quarter\\notes.pst'",
                "'quarter\\notes.pst'",
                "‘quarter\\\\notes.pst’",
            ),
            // An apostrophe beside what a shell minds stays in single quotes.
            (b"a'b$c", "'a'\\''b$c'", "'a'\\''b$c'", "‘a'b$c’"),
            // A control that is a character of its own, two bytes of it.
            (
                "nl\u{85}".as_bytes(),
                "'nl'$'\\302\\205'",
                "'nl'$'\\302\\205'",
                "‘nl\\302\\205’",
            ),
            (b"", "''", "''", "‘’"),
        ];
        for (name, rm, df, find) in cases {
            let shown = String::from_utf8_lossy(name);
            assert_eq!(for_shell(name, true), rm, "rm, du: {shown:?}");
            assert_eq!(for_shell(name, false), df, "df: {shown:?}");
            assert_eq!(for_find(name), find, "find: {shown:?}");
        }
    }

    /// Two names that differ are never written the same: the name that is
    /// not text, and the one holding the character a lossy reading puts in
    /// its place.
    #[test]
    fn two_names_are_never_written_alike() {
        let (stray, replaced) = (&b"bad\xff.dat"[..], "bad\u{fffd}.dat".as_bytes());
        assert_eq!(
            String::from_utf8_lossy(stray),
            String::from_utf8_lossy(replaced)
        );
        for always in [true, false] {
            assert_ne!(for_shell(stray, always), for_shell(replaced, always));
        }
        assert_ne!(for_find(stray), for_find(replaced));
        // A character no code point is assigned to cannot be printed.
        assert_eq!(
            for_shell("\u{378}".as_bytes(), true),
            "''$'\\315\\270'",
            "U+0378, by its two bytes"
        );
        // A word a shell reads specially only where it stands alone.
        assert_eq!(for_shell(b"{", false), "'{'");
        assert_eq!(for_shell(b"a{b}", false), "a{b}");
        assert_eq!(for_shell(b"#x", false), "'#x'");
        assert_eq!(for_shell(b"x#~", false), "x#~");
        assert_eq!(for_shell(b"a=b", false), "'a=b'");
    }
}

#[cfg(test)]
mod json_tests {
    use super::*;

    /// cr-uv3-7: a name that is text is its text and nothing more; one
    /// that is not has its bytes beside the text, and they read back.
    #[test]
    fn a_name_that_is_not_text_has_its_bytes_beside_it() {
        for text in [
            "plain.txt",
            "caf\u{e9}",
            "bad\u{fffd}.dat",
            "back\\slash",
            "",
        ] {
            assert_eq!(exact(text.as_bytes()), None, "{text:?}");
            assert_eq!(
                in_json(text.as_bytes(), serde_json::json!({ "n": 1 })),
                serde_json::json!({ "n": 1, "path": text })
            );
        }
        for (name, raw) in [
            (&b"bad\xff.dat"[..], r"bad\xff.dat"),
            (b"\xff", r"\xff"),
            // What is text in it is written byte by byte too: the whole
            // name is there, and one rule reads it back.
            (b"caf\xc3\xa9\xfe", r"caf\xc3\xa9\xfe"),
            (b"a\\b\xff", r"a\x5cb\xff"),
        ] {
            assert_eq!(exact(name).as_deref(), Some(raw));
            assert_eq!(
                crate::raw_name::unescape_raw(raw).as_deref(),
                Some(name),
                "{raw} reads back"
            );
            let entry = in_json(name, serde_json::json!({}));
            assert_eq!(entry["path"], String::from_utf8_lossy(name).as_ref());
            assert_eq!(entry["raw"], raw);
        }
        // The two names the text cannot tell apart.
        let (one, other) = (
            in_json(b"bad\xff.dat", serde_json::json!({})),
            in_json("bad\u{fffd}.dat".as_bytes(), serde_json::json!({})),
        );
        assert_eq!(one["path"], other["path"]);
        assert_ne!(one, other);
    }
}
