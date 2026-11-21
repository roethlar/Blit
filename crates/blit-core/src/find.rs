//! `find`, walked and tested as the Unix command does it
//! (`docs/plan/UTILITY_VERBS_LOCAL_PARITY.md` slice 3, AC-4, AC-7, AC-12).
//!
//! This runs on the machine that holds the files — a daemon, for
//! `blit find HOST:/module/path` — so a tree on a network share is
//! searched at local speed. The reference is GNU find 4.11 under
//! `C.UTF-8`:
//!
//! - the start is an entry like any other and comes first; a directory
//!   comes before what is in it, entries in the order the directory lists
//!   them;
//! - a symbolic link is an entry of its own and is never followed — except
//!   the start itself when it was typed with a separator after it, which
//!   names the directory the link points to ([`FindOptions::follow_start`]);
//! - a directory that cannot be read is still an entry (it is tested and
//!   may be found), is reported, and the walk goes on;
//! - `-maxdepth N` enters nothing more than `N` levels below the start;
//! - every test given must hold ([`crate::find_name`] for the name tests).
//!
//! Nothing is asked of the filesystem that no test needs: an entry's kind
//! comes with its directory listing, and only the start is looked up by
//! itself ([`lookups`] counts, for the guard).

use crate::find_name::NamePattern;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

pub use crate::disk_usage::Stopped;

/// What `-type` asks for.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    /// `f`: a regular file.
    File,
    /// `d`
    Directory,
    /// `l`: a symbolic link, whatever it points to.
    Link,
}

impl Kind {
    fn of(kind: fs::FileType) -> Option<Self> {
        if kind.is_symlink() {
            Some(Self::Link)
        } else if kind.is_dir() {
            Some(Self::Directory)
        } else if kind.is_file() {
            Some(Self::File)
        } else {
            // A socket, a device, a pipe: an entry, and none of the three.
            None
        }
    }
}

/// What `find` was asked: the tests, every one of which must hold, and how
/// far to walk.
#[derive(Debug, Clone, Default)]
pub struct FindOptions {
    /// `-name` and `-iname`, against the entry's base name.
    pub names: Vec<NamePattern>,
    /// `-type`.
    pub kinds: Vec<Kind>,
    /// `-maxdepth N`; `None` is every level.
    pub max_depth: Option<u32>,
    /// The start was typed with a separator after it: a start that is a
    /// link to a directory names that directory, and is entered.
    pub follow_start: bool,
}

impl FindOptions {
    fn holds(&self, name: &[u8], kind: Option<Kind>) -> bool {
        self.kinds.iter().all(|wanted| Some(*wanted) == kind)
            && self.names.iter().all(|pattern| pattern.matches(name))
    }
}

/// An entry the walk could not read. The walk goes on past it, and the
/// command that asked exits 1, as `find` does.
#[derive(Debug)]
pub struct FindFailure {
    /// The entry's path below the start; empty for the start itself.
    pub relative_path: PathBuf,
    pub error: io::Error,
}

/// What [`find`] hands its caller, in order.
#[derive(Debug)]
pub enum FindEvent {
    /// An entry every test holds for: its path below the start, empty for
    /// the start itself.
    Found(PathBuf),
    Failure(FindFailure),
}

/// Why [`find`] ended without finishing.
#[derive(Debug)]
pub enum FindEnd {
    /// The start itself is not there, or cannot be looked up.
    Start(io::Error),
    /// The caller stopped the walk.
    Stopped(Stopped),
}

#[cfg(test)]
thread_local! {
    static LOOKUPS: std::cell::Cell<usize> = const { std::cell::Cell::new(0) };
}

/// How many entries this thread's walks have looked up by themselves — a
/// `stat` each — since the count was last taken.
#[cfg(test)]
pub(crate) fn lookups() -> usize {
    LOOKUPS.with(|count| count.replace(0))
}

/// A name's bytes as the name tests read them: its own on Unix, where a
/// name need not be text, and its text's elsewhere.
pub fn name_bytes(name: &std::ffi::OsStr) -> std::borrow::Cow<'_, [u8]> {
    crate::fs_enum::match_bytes(name)
}

/// The one place the walk asks the filesystem about a single entry.
fn look_up(path: &Path, follow: bool) -> io::Result<fs::Metadata> {
    #[cfg(test)]
    LOOKUPS.with(|count| count.set(count.get() + 1));
    if follow {
        fs::metadata(path)
    } else {
        fs::symlink_metadata(path)
    }
}

/// `find` from `start`, whose base name for the name tests is
/// `start_name` (the last name of the path as it was typed). `emit` gets
/// each entry found and each failure as the walk meets it; returning
/// `Err(Stopped)` ends the walk.
pub fn find(
    start: &Path,
    start_name: &[u8],
    options: &FindOptions,
    emit: &mut dyn FnMut(FindEvent) -> Result<(), Stopped>,
) -> Result<(), FindEnd> {
    let mut metadata = look_up(start, false).map_err(FindEnd::Start)?;
    if options.follow_start && metadata.file_type().is_symlink() {
        metadata = look_up(start, true).map_err(FindEnd::Start)?;
    }
    let kind = Kind::of(metadata.file_type());
    if options.holds(start_name, kind) {
        emit(FindEvent::Found(PathBuf::new())).map_err(FindEnd::Stopped)?;
    }
    if kind != Some(Kind::Directory) || options.max_depth == Some(0) {
        return Ok(());
    }

    /// One directory being read.
    struct Frame {
        relative_path: PathBuf,
        entries: fs::ReadDir,
    }
    let mut frames: Vec<Frame> = Vec::new();
    match fs::read_dir(start) {
        Ok(entries) => frames.push(Frame {
            relative_path: PathBuf::new(),
            entries,
        }),
        Err(error) => {
            return emit(FindEvent::Failure(FindFailure {
                relative_path: PathBuf::new(),
                error,
            }))
            .map_err(FindEnd::Stopped);
        }
    }
    while let Some(frame) = frames.last_mut() {
        let Some(entry) = frame.entries.next() else {
            frames.pop();
            continue;
        };
        let entry = match entry {
            Ok(entry) => entry,
            Err(error) => {
                let relative_path = frame.relative_path.clone();
                emit(FindEvent::Failure(FindFailure {
                    relative_path,
                    error,
                }))
                .map_err(FindEnd::Stopped)?;
                continue;
            }
        };
        let name = entry.file_name();
        let relative_path = frame.relative_path.join(&name);
        // The listing's own word for the entry: no lookup, and never
        // through a link.
        let kind = match entry.file_type() {
            Ok(kind) => Kind::of(kind),
            Err(error) => {
                emit(FindEvent::Failure(FindFailure {
                    relative_path,
                    error,
                }))
                .map_err(FindEnd::Stopped)?;
                continue;
            }
        };
        if options.holds(&name_bytes(&name), kind) {
            emit(FindEvent::Found(relative_path.clone())).map_err(FindEnd::Stopped)?;
        }
        // This entry is `frames.len()` levels below the start.
        let enter = kind == Some(Kind::Directory)
            && options
                .max_depth
                .is_none_or(|max| (frames.len() as u32) < max);
        if !enter {
            continue;
        }
        match fs::read_dir(entry.path()) {
            Ok(entries) => frames.push(Frame {
                relative_path,
                entries,
            }),
            Err(error) => {
                emit(FindEvent::Failure(FindFailure {
                    relative_path,
                    error,
                }))
                .map_err(FindEnd::Stopped)?;
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn write(root: &Path, rel: &str) {
        let path = root.join(rel);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, b"x").unwrap();
    }

    /// What a walk found, sorted, and what it could not read.
    fn walk(start: &Path, name: &str, options: &FindOptions) -> (Vec<String>, Vec<String>) {
        let (mut found, mut failed) = (Vec::new(), Vec::new());
        find(start, name.as_bytes(), options, &mut |event| {
            match event {
                FindEvent::Found(path) => found.push(path.to_string_lossy().replace('\\', "/")),
                FindEvent::Failure(failure) => {
                    failed.push(failure.relative_path.to_string_lossy().replace('\\', "/"))
                }
            }
            Ok(())
        })
        .expect("the walk");
        found.sort();
        (found, failed)
    }

    fn named(pattern: &str, ignore_case: bool) -> NamePattern {
        NamePattern::new(pattern.as_bytes(), ignore_case).unwrap()
    }

    fn tree() -> tempfile::TempDir {
        let dir = tempfile::tempdir().unwrap();
        for rel in [
            "t/a.dat",
            "t/B.DAT",
            "t/.hidden",
            "t/sub/inner.dat",
            "t/sub/deep/er.dat",
        ] {
            write(dir.path(), rel);
        }
        fs::create_dir_all(dir.path().join("t/empty")).unwrap();
        dir
    }

    /// AC-4: the start is an entry and comes first, a directory before
    /// what is in it; every test given must hold; `-maxdepth` counts from
    /// the start.
    #[test]
    fn the_walk_finds_what_every_test_holds_for() {
        let dir = tree();
        let start = dir.path().join("t");
        let all = FindOptions::default();
        assert_eq!(
            walk(&start, "t", &all).0,
            [
                "",
                ".hidden",
                "B.DAT",
                "a.dat",
                "empty",
                "sub",
                "sub/deep",
                "sub/deep/er.dat",
                "sub/inner.dat"
            ]
        );
        // In walk order the start is first and a directory precedes its
        // contents.
        let mut order = Vec::new();
        find(&start, b"t", &all, &mut |event| {
            if let FindEvent::Found(path) = event {
                order.push(path.to_string_lossy().replace('\\', "/"));
            }
            Ok(())
        })
        .unwrap();
        assert_eq!(order[0], "");
        let at = |rel: &str| order.iter().position(|found| found == rel).unwrap();
        assert!(at("sub") < at("sub/inner.dat") && at("sub/deep") < at("sub/deep/er.dat"));

        let with =
            |names: Vec<NamePattern>, kinds: Vec<Kind>, max_depth: Option<u32>| FindOptions {
                names,
                kinds,
                max_depth,
                follow_start: false,
            };
        let found = |options: FindOptions| walk(&start, "t", &options).0;
        assert_eq!(
            found(with(vec![named("*.dat", false)], vec![], None)),
            ["a.dat", "sub/deep/er.dat", "sub/inner.dat"]
        );
        assert_eq!(
            found(with(vec![named("*.dat", true)], vec![], None)),
            ["B.DAT", "a.dat", "sub/deep/er.dat", "sub/inner.dat"]
        );
        assert_eq!(
            found(with(vec![], vec![Kind::Directory], None)),
            ["", "empty", "sub", "sub/deep"]
        );
        assert_eq!(
            found(with(vec![], vec![Kind::Directory], Some(1))),
            ["", "empty", "sub"]
        );
        assert_eq!(found(with(vec![], vec![], Some(0))), [""]);
        assert_eq!(
            found(with(vec![named("*.dat", false)], vec![Kind::File], Some(1))),
            ["a.dat"]
        );
        // The start is tested by the name it was typed with.
        assert_eq!(found(with(vec![named("t", false)], vec![], None)), [""]);
        // Two name tests: both must hold. Two kinds: nothing is both.
        assert_eq!(
            found(with(
                vec![named("*.dat", false), named("i*", false)],
                vec![],
                None
            )),
            ["sub/inner.dat"]
        );
        assert_eq!(
            found(with(vec![], vec![Kind::File, Kind::Directory], None)),
            Vec::<String>::new()
        );
        // A file given as the start is the one entry.
        assert_eq!(walk(&start.join("a.dat"), "a.dat", &all).0, [""]);
        // A start that is not there: nothing found, and said so.
        let mut nothing = |_| Ok(());
        assert!(matches!(
            find(&start.join("missing"), b"missing", &all, &mut nothing),
            Err(FindEnd::Start(error)) if error.kind() == io::ErrorKind::NotFound
        ));
    }

    /// AC-7: nothing is asked of the filesystem that no test needs. The
    /// start is looked up; no other entry is, however many there are — an
    /// entry's kind comes with its directory's listing.
    #[test]
    fn only_the_start_is_looked_up() {
        let dir = tempfile::tempdir().unwrap();
        for folder in 0..20 {
            for file in 0..20 {
                write(dir.path(), &format!("t/d{folder}/f{file}"));
            }
        }
        let start = dir.path().join("t");
        lookups();
        let (found, failed) = walk(
            &start,
            "t",
            &FindOptions {
                names: vec![named("f1*", false)],
                kinds: vec![Kind::File],
                max_depth: None,
                follow_start: false,
            },
        );
        assert_eq!((found.len(), failed.len()), (20 * 11, 0));
        assert_eq!(lookups(), 1, "421 entries walked, one lookup: the start");
    }

    /// AC-12: a symbolic link is an entry of its own and nothing under
    /// what it points to is visited — unless it is the start and was
    /// typed with a separator after it, which names the directory.
    #[cfg(unix)]
    #[test]
    fn a_link_is_an_entry_and_is_not_followed() {
        let dir = tree();
        let start = dir.path().join("t");
        std::os::unix::fs::symlink("sub", start.join("link")).unwrap();
        std::os::unix::fs::symlink("nowhere", start.join("broken")).unwrap();
        let all = FindOptions::default();
        let found = walk(&start, "t", &all).0;
        assert!(found.contains(&"link".to_string()) && found.contains(&"broken".to_string()));
        assert!(
            !found.iter().any(|path| path.starts_with("link/")),
            "{found:?}"
        );
        let links = FindOptions {
            kinds: vec![Kind::Link],
            ..FindOptions::default()
        };
        assert_eq!(walk(&start, "t", &links).0, ["broken", "link"]);

        // The link as the start: one entry, a link.
        assert_eq!(walk(&start.join("link"), "link", &all).0, [""]);
        assert_eq!(walk(&start.join("link"), "link", &links).0, [""]);
        // With a separator after it: the directory it points to.
        let followed = FindOptions {
            follow_start: true,
            ..FindOptions::default()
        };
        assert_eq!(
            walk(&start.join("link"), "link", &followed).0,
            ["", "deep", "deep/er.dat", "inner.dat"]
        );
        let directories = FindOptions {
            kinds: vec![Kind::Directory],
            follow_start: true,
            ..FindOptions::default()
        };
        assert_eq!(
            walk(&start.join("link"), "link", &directories).0,
            ["", "deep"]
        );
    }

    /// AC-12: a directory that cannot be read is found like any entry, is
    /// reported, and the walk goes on past it.
    #[cfg(unix)]
    #[test]
    fn a_directory_that_cannot_be_read_is_reported_and_passed() {
        use std::os::unix::fs::PermissionsExt;
        let dir = tree();
        let start = dir.path().join("t");
        write(&start, "locked/inside.dat");
        fs::set_permissions(start.join("locked"), fs::Permissions::from_mode(0o000)).unwrap();
        let readable = fs::read_dir(start.join("locked")).is_ok();
        let (found, failed) = walk(&start, "t", &FindOptions::default());
        fs::set_permissions(start.join("locked"), fs::Permissions::from_mode(0o755)).unwrap();
        if readable {
            eprintln!("running as root: every directory can be read; nothing to report");
            return;
        }
        assert_eq!(failed, ["locked"]);
        assert!(
            found.contains(&"locked".to_string()),
            "the directory itself is an entry"
        );
        assert!(!found.contains(&"locked/inside.dat".to_string()));
        assert!(
            found.contains(&"sub/deep/er.dat".to_string()),
            "and the walk went on"
        );
    }

    /// AC-12: a name that is not valid text is found, and matched, as the
    /// name it is.
    #[cfg(unix)]
    #[test]
    fn a_name_that_is_not_text_is_found_by_its_bytes() {
        use std::os::unix::ffi::{OsStrExt, OsStringExt};
        let dir = tempfile::tempdir().unwrap();
        let start = dir.path().join("t");
        fs::create_dir_all(&start).unwrap();
        let raw = std::ffi::OsStr::from_bytes(b"bad\xff.dat");
        if fs::write(start.join(raw), b"x").is_err() {
            eprintln!("this filesystem refuses a name that is not text: nothing to find");
            return;
        }
        let mut found = Vec::new();
        let options = FindOptions {
            names: vec![named("bad?.dat", false)],
            ..FindOptions::default()
        };
        find(&start, b"t", &options, &mut |event| {
            if let FindEvent::Found(path) = event {
                found.push(path.into_os_string().into_vec());
            }
            Ok(())
        })
        .unwrap();
        assert_eq!(found, [b"bad\xff.dat".to_vec()]);
    }

    /// A caller that no longer wants the answer ends the walk.
    #[test]
    fn the_walk_stops_when_its_caller_does() {
        let dir = tree();
        let mut seen = 0;
        let end = find(
            &dir.path().join("t"),
            b"t",
            &FindOptions::default(),
            &mut |_| {
                seen += 1;
                if seen == 3 {
                    Err(Stopped)
                } else {
                    Ok(())
                }
            },
        );
        assert!(matches!(end, Err(FindEnd::Stopped(Stopped))));
        assert_eq!(seen, 3);
    }
}
