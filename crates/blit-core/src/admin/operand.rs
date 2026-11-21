//! A utility verb's operand below its module, read as it was typed
//! (cr-uv1-1).
//!
//! `rm`, `du`, `df` and `find` are the Unix commands run on a daemon's own
//! disk, and to those commands HOW a path is written says something: `x/`
//! names `x` only if it is a directory, and through a link; `x/.` likewise;
//! `rm` refuses a path whose last name is `.` or `..`; a space at either
//! end is part of a name. A path that was tidied on the way — its last
//! separator dropped, a `.` folded away, its ends trimmed — is another
//! path: `rm -rf HOST:/m/work/.` deleted `work`, `rm HOST:/m/doc/` unlinked
//! a file, and of `report` and `report ` (a space after it) the wrong one
//! went.
//!
//! So the path goes from the command line to the daemon as typed, and what
//! its writing says is read here, by both ends, from that one text.
//! Separators are `/`: a remote path is written with forward slashes on
//! every machine.
//!
//! The text is BYTES (cr-uv1-4). A name on a Unix disk need not be UTF-8,
//! and these commands print such a name by its own bytes; a path they
//! print has to be one they take. [`Operand`] reads an operand from the
//! command line so: the daemon and the module as text, the path below the
//! module as the bytes typed.

use std::ffi::OsStr;

use eyre::{bail, Result};

use super::du::EntryPath;
use crate::remote::endpoint::{RemoteEndpoint, RemotePath};

/// The last name of a typed path: the text after its last separator, the
/// separators after that name set aside. Empty for a path that holds no
/// name at all.
pub fn final_name(typed: &[u8]) -> &[u8] {
    let end = typed
        .iter()
        .rposition(|&byte| byte != b'/')
        .map_or(0, |last| last + 1);
    let named = &typed[..end];
    match named.iter().rposition(|&byte| byte == b'/') {
        Some(separator) => &named[separator + 1..],
        None => named,
    }
}

/// The path's last name is `.` or `..` — what `rm` refuses to remove,
/// whatever its options (`refusing to remove '.' or '..' directory`).
pub fn names_dot(typed: &[u8]) -> bool {
    matches!(final_name(typed), b"." | b"..")
}

/// The path is written so that it names a directory and nothing else: with
/// a separator after it, or with `.` as its last name. The system then
/// resolves a link standing there and refuses anything that is no
/// directory (`Not a directory`).
pub fn names_a_directory(typed: &[u8]) -> bool {
    typed.ends_with(b"/") || final_name(typed) == b"."
}

/// The path holds no name: only separators and `.`. It is the module
/// itself.
pub fn is_the_root(typed: &[u8]) -> bool {
    typed
        .split(|&byte| byte == b'/')
        .all(|part| part.is_empty() || part == b".")
}

/// The bytes of a command-line word. On Unix a word is bytes, and they
/// are these; elsewhere a word is text or it is refused.
pub fn word_bytes(word: &OsStr) -> Result<&[u8]> {
    #[cfg(unix)]
    {
        use std::os::unix::ffi::OsStrExt as _;
        Ok(word.as_bytes())
    }
    #[cfg(not(unix))]
    {
        match word.to_str() {
            Some(text) => Ok(text.as_bytes()),
            None => bail!("`{}` is not valid text", word.to_string_lossy()),
        }
    }
}

/// A utility verb's operand as typed: `HOST:/MODULE/PATH` or
/// `HOST://PATH`, the path by its bytes.
#[derive(Debug, Clone)]
pub struct Operand {
    /// The daemon it names.
    pub remote: RemoteEndpoint,
    /// The module; empty for the default root (`HOST://`).
    pub module: String,
    /// The whole operand, as typed.
    typed: Vec<u8>,
    /// Where in it the path below the module begins.
    path_at: usize,
}

impl Operand {
    /// Read one operand. `Ok(None)` is a word that names no daemon — a
    /// local path. An operand that names a daemon and no module, or whose
    /// host or module is not text, is an error.
    pub fn read(word: &OsStr) -> Result<Option<Self>> {
        let typed = word_bytes(word)?;
        // The daemon and the module are text, and stand before the path:
        // they are read from the text the operand begins with. Whatever
        // in the PATH is not text stays as the bytes it is.
        let (text, all_text) = match std::str::from_utf8(typed) {
            Ok(text) => (text, true),
            Err(error) => (
                std::str::from_utf8(&typed[..error.valid_up_to()]).expect("valid up to there"),
                false,
            ),
        };
        let holds_a_path = |remote: &RemoteEndpoint| !matches!(remote.path, RemotePath::Discovery);
        let read = RemoteEndpoint::parse(text)
            .ok()
            .filter(holds_a_path)
            .and_then(|remote| Some((remote, RemoteEndpoint::path_offset(text)?)));
        let Some((remote, path_at)) = read else {
            // Does the whole word, read leniently, name a daemon at all?
            let lenient = String::from_utf8_lossy(typed);
            return match RemoteEndpoint::parse(&lenient) {
                Err(_) => Ok(None),
                Ok(_) if !all_text => {
                    bail!("the daemon and the module in `{lenient}` must be valid text")
                }
                Ok(_) => {
                    bail!("remote target must include a module path (e.g., server:/module/path)")
                }
            };
        };
        let module = match &remote.path {
            RemotePath::Module { module, .. } => module.clone(),
            _ => String::new(),
        };
        Ok(Some(Self {
            remote,
            module,
            typed: typed.to_vec(),
            path_at,
        }))
    }

    /// The path below the module exactly as typed: no separator dropped,
    /// no `.` folded away, no space trimmed, no byte replaced. (Separators
    /// between the module and the first name belong to neither.)
    pub fn path(&self) -> &[u8] {
        let path = &self.typed[self.path_at..];
        let lead = path.iter().take_while(|&&byte| byte == b'/').count();
        &path[lead..]
    }

    /// That path as a request carries it: its text, and its own bytes
    /// when they are not text.
    pub fn wire_path(&self) -> EntryPath {
        EntryPath::typed(self.path())
    }

    /// The operand as typed, by its bytes — what the commands print.
    pub fn typed(&self) -> &[u8] {
        &self.typed
    }

    /// The operand as typed, as text for a message.
    pub fn shown(&self) -> String {
        String::from_utf8_lossy(&self.typed).into_owned()
    }

    /// An entry below this operand as `du` and `rm` print it: the operand
    /// as typed for the operand itself; below it the operand, ONE
    /// separator, and the entry's path — `dir/` and `dir` both print
    /// `dir/sub`. The separators that END THE DAEMON'S PART are never
    /// taken for the operand's own: below `HOST://` an entry is
    /// `HOST://sub`. (cr-uv1-4: it was printed `HOST:/sub`, which names a
    /// module called `sub`.)
    pub fn below(&self, entry: &EntryPath) -> Vec<u8> {
        if entry.is_start() {
            return self.typed.clone();
        }
        let (daemon, path) = self.typed.split_at(self.path_at);
        let end = path
            .iter()
            .rposition(|&byte| byte != b'/')
            .map_or(0, |last| last + 1);
        let mut shown = daemon.to_vec();
        shown.extend_from_slice(&path[..end]);
        if end > 0 {
            shown.push(b'/');
        }
        shown.extend_from_slice(entry.bytes());
        shown
    }

    /// An entry below this operand as `find` prints it: the operand as
    /// typed, a separator unless it ends in one, and the entry's path —
    /// `dir//` prints `dir//sub`, every separator typed kept.
    pub fn below_as_find(&self, entry: &EntryPath) -> Vec<u8> {
        let mut shown = self.typed.clone();
        if entry.is_start() {
            return shown;
        }
        if !shown.ends_with(b"/") {
            shown.push(b'/');
        }
        shown.extend_from_slice(entry.bytes());
        shown
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// cr-uv1-1: what the writing of a path says, read from the text as
    /// typed — each row a path, its last name, and whether it ends in `.`
    /// or `..`, is written as a directory, is the module itself.
    #[test]
    fn a_typed_path_says_how_it_was_written() {
        for (typed, last, dot, directory, root) in [
            ("work", "work", false, false, false),
            ("work/", "work", false, true, false),
            ("work//", "work", false, true, false),
            ("work/.", ".", true, true, false),
            ("work/./", ".", true, true, false),
            ("work//.", ".", true, true, false),
            ("work/..", "..", true, false, false),
            ("work/sub/../", "..", true, true, false),
            ("a/b/c", "c", false, false, false),
            ("a/./b", "b", false, false, false),
            // A space is part of a name, at either end.
            ("report ", "report ", false, false, false),
            (" report", " report", false, false, false),
            ("dir/ ", " ", false, false, false),
            // A name that only looks like `.`.
            ("...", "...", false, false, false),
            (".hidden", ".hidden", false, false, false),
            ("x/. ", ". ", false, false, false),
            // No name at all: the module.
            ("", "", false, false, true),
            (".", ".", true, true, true),
            ("./", ".", true, true, true),
            ("/", "", false, true, true),
            ("././/.", ".", true, true, true),
        ] {
            let typed = typed.as_bytes();
            assert_eq!(final_name(typed), last.as_bytes(), "{typed:?}");
            assert_eq!(names_dot(typed), dot, "{typed:?}");
            assert_eq!(names_a_directory(typed), directory, "{typed:?}");
            assert_eq!(is_the_root(typed), root, "{typed:?}");
        }
        // Bytes that are no text are a name like any other.
        assert_eq!(final_name(b"dir/bad\xff.txt/"), b"bad\xff.txt");
        assert!(!names_dot(b"\xff") && !is_the_root(b"\xff"));
    }

    fn read(typed: &str) -> Operand {
        Operand::read(OsStr::new(typed))
            .expect(typed)
            .expect("names a daemon")
    }

    fn entry(path: &str) -> EntryPath {
        EntryPath::typed(path.as_bytes())
    }

    /// cr-uv1-4: what is printed for an entry begins with the operand as
    /// typed and can be given back as an operand. The separators that end
    /// the daemon's part are not the operand's to lose: `HOST://` printed
    /// its entries as `HOST:/child` — a module called `child`.
    #[test]
    fn what_is_printed_below_an_operand_can_be_given_back() {
        for (typed, module, path, as_du, as_find) in [
            ("srv://", "", "", "srv://child", "srv://child"),
            ("srv://dir", "", "dir", "srv://dir/child", "srv://dir/child"),
            (
                "srv://dir/",
                "",
                "dir/",
                "srv://dir/child",
                "srv://dir/child",
            ),
            ("srv:/m/", "m", "", "srv:/m/child", "srv:/m/child"),
            (
                "srv:/m/dir",
                "m",
                "dir",
                "srv:/m/dir/child",
                "srv:/m/dir/child",
            ),
            (
                "srv:/m/dir/",
                "m",
                "dir/",
                "srv:/m/dir/child",
                "srv:/m/dir/child",
            ),
            (
                "srv:/m/dir//",
                "m",
                "dir//",
                "srv:/m/dir/child",
                "srv:/m/dir//child",
            ),
            (
                "srv:/m/dir/.",
                "m",
                "dir/.",
                "srv:/m/dir/./child",
                "srv:/m/dir/./child",
            ),
            (
                "srv:/m/a b ",
                "m",
                "a b ",
                "srv:/m/a b /child",
                "srv:/m/a b /child",
            ),
            (
                "srv:9031:/m/x",
                "m",
                "x",
                "srv:9031:/m/x/child",
                "srv:9031:/m/x/child",
            ),
            (
                "[::1]:/m/x",
                "m",
                "x",
                "[::1]:/m/x/child",
                "[::1]:/m/x/child",
            ),
            (
                "  srv:/m/x",
                "m",
                "x",
                "  srv:/m/x/child",
                "  srv:/m/x/child",
            ),
        ] {
            let operand = read(typed);
            assert_eq!(operand.module, module, "{typed}");
            assert_eq!(operand.path(), path.as_bytes(), "{typed}");
            assert_eq!(
                operand.below(&entry("")),
                typed.as_bytes(),
                "{typed}: itself"
            );
            assert_eq!(
                operand.below_as_find(&entry("")),
                typed.as_bytes(),
                "{typed}"
            );
            assert_eq!(
                String::from_utf8_lossy(&operand.below(&entry("child"))),
                as_du,
                "{typed}: as du and rm print it"
            );
            assert_eq!(
                String::from_utf8_lossy(&operand.below_as_find(&entry("child"))),
                as_find,
                "{typed}: as find prints it"
            );
            // And given back, it names the entry, in the same module.
            for printed in [as_du, as_find] {
                let again = read(printed);
                assert_eq!(again.module, module, "{printed}");
                assert!(
                    again.path().ends_with(b"child") && again.remote.host == operand.remote.host,
                    "{printed}"
                );
            }
        }
        // Not a daemon's path: a local one. A daemon with no module: refused.
        assert!(Operand::read(OsStr::new("./local/path")).unwrap().is_none());
        assert!(Operand::read(OsStr::new("/abs")).unwrap().is_none());
        assert!(Operand::read(OsStr::new("srv")).is_err());
    }

    /// cr-uv1-4: a path that is not text is taken by its bytes — the name
    /// `find` printed can be given to `rm`. The daemon and the module are
    /// text; a byte that is none in them names no daemon.
    #[cfg(unix)]
    #[test]
    fn a_path_that_is_not_text_is_taken_by_its_bytes() {
        use std::os::unix::ffi::OsStrExt as _;
        let operand = Operand::read(OsStr::from_bytes(b"srv:/m/dir/bad\xff.txt"))
            .unwrap()
            .expect("names a daemon");
        assert_eq!(
            (operand.module.as_str(), operand.path()),
            ("m", &b"dir/bad\xff.txt"[..])
        );
        let wire = operand.wire_path();
        assert_eq!(wire.raw.as_deref(), Some(&b"dir/bad\xff.txt"[..]));
        assert_eq!(wire.text, "dir/bad\u{fffd}.txt");
        assert_eq!(operand.typed(), b"srv:/m/dir/bad\xff.txt");
        assert_eq!(
            operand.below(&EntryPath::typed(b"in\xfe")),
            b"srv:/m/dir/bad\xff.txt/in\xfe"
        );
        // Text all through carries no second copy.
        assert_eq!(read("srv:/m/dir/ok.txt").wire_path().raw, None);
        // A byte that is no text in the daemon's name or the module's is
        // refused: neither can be asked for by it.
        assert!(Operand::read(OsStr::from_bytes(b"sr\xffv:/m/x")).is_err());
        assert!(Operand::read(OsStr::from_bytes(b"srv:/m\xff/x")).is_err());
        // And a local path that is not text is a local path.
        assert!(Operand::read(OsStr::from_bytes(b"./loc\xffal"))
            .unwrap()
            .is_none());
    }
}
