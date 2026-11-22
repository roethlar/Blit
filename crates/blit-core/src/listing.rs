//! `ls`, listed as the Unix command lists
//! (`docs/plan/UTILITY_VERBS_LOCAL_PARITY.md` slice 5, AC-6, AC-7).
//!
//! This runs on the machine that holds the files — a daemon, for `blit ls
//! HOST:/module/path` — so a directory on a network share is listed in
//! one request. The reference is GNU ls 9.12 under `C.UTF-8`, measured
//! 2026-11-21:
//!
//! - an operand that is a directory is listed INTO: its entries, those
//!   whose name begins with `.` left out unless all are asked for, when
//!   `.` and `..` are entries too;
//! - any other operand is one line, itself;
//! - an operand that is a link to a directory is listed into, as the
//!   directory — unless the long form was asked for, when it is the link.
//!   Typed with a separator after it, it is the directory either way;
//! - nothing is asked of the filesystem that will not be printed: without
//!   its facts an entry is its name, and no entry is looked up
//!   ([`lookups`] counts, for the guard).
//!
//! Entries come in the order the directory gives them. The asker sorts:
//! it holds them all anyway, to line the columns up.

use std::ffi::OsString;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

pub use crate::disk_usage::Stopped;

/// What an entry is: the letter `ls -l` begins its line with.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    File,
    Directory,
    /// A symbolic link, whatever it points to.
    Link,
    Fifo,
    Socket,
    CharDevice,
    BlockDevice,
    /// Something the system has no name for here.
    Other,
}

impl Kind {
    pub fn of(kind: fs::FileType) -> Self {
        if kind.is_symlink() {
            return Self::Link;
        }
        if kind.is_dir() {
            return Self::Directory;
        }
        if kind.is_file() {
            return Self::File;
        }
        #[cfg(unix)]
        {
            use std::os::unix::fs::FileTypeExt as _;
            if kind.is_fifo() {
                return Self::Fifo;
            }
            if kind.is_socket() {
                return Self::Socket;
            }
            if kind.is_char_device() {
                return Self::CharDevice;
            }
            if kind.is_block_device() {
                return Self::BlockDevice;
            }
        }
        Self::Other
    }

    /// The letter `ls -l` begins a line with.
    pub fn letter(self) -> char {
        match self {
            Self::File => '-',
            Self::Directory => 'd',
            Self::Link => 'l',
            Self::Fifo => 'p',
            Self::Socket => 's',
            Self::CharDevice => 'c',
            Self::BlockDevice => 'b',
            Self::Other => '?',
        }
    }

    /// The word `--json` gives for it.
    pub fn word(self) -> &'static str {
        match self {
            Self::File => "file",
            Self::Directory => "directory",
            Self::Link => "link",
            Self::Fifo => "fifo",
            Self::Socket => "socket",
            Self::CharDevice => "char_device",
            Self::BlockDevice => "block_device",
            Self::Other => "other",
        }
    }
}

/// What the long form prints of an entry besides its name. A link's are
/// the link's own, never those of what it points to.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Facts {
    pub kind: Kind,
    pub size: u64,
    pub mtime_seconds: i64,
    /// The space allocated to it, for the `total` line.
    pub allocated_bytes: u64,
    /// Where a link points, as the link holds it.
    pub link_target: Option<PathBuf>,
}

/// One entry: its name in its directory, and its facts when they were
/// asked for and could be read.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Entry {
    /// Empty for the operand itself, which the asker names as typed.
    pub name: OsString,
    pub facts: Option<Facts>,
}

/// What is asked of a listing.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct ListOptions {
    /// `-a`: names that begin with `.` too, and `.` and `..`.
    pub all: bool,
    /// Each entry's facts are wanted (the long form, `--json`). Without
    /// this no entry is looked up.
    pub facts: bool,
    /// `-l`: an operand that is a link is listed as the link, where
    /// otherwise a link to a directory is listed into.
    pub link_itself: bool,
}

/// What [`list`] hands its caller, in order: [`Itself`](Self::Itself) or
/// [`Directory`](Self::Directory) first, then the directory's entries.
#[derive(Debug)]
pub enum ListEvent {
    /// The operand is not listed into: this is its own line.
    Itself(Entry),
    /// The operand is a directory; its entries follow.
    Directory,
    Entry(Entry),
    /// An entry whose facts could not be read (`cannot access`). It
    /// follows as an [`Entry`](Self::Entry) without them.
    Failure {
        name: OsString,
        error: io::Error,
    },
}

/// Why a listing ended early.
#[derive(Debug)]
pub enum ListEnd {
    /// The operand is not there, or cannot be looked up: `cannot access`.
    Access(io::Error),
    /// The directory cannot be read: `cannot open directory`.
    Open(io::Error),
    /// The directory stopped giving its entries: `reading directory`.
    Read(io::Error),
    /// The caller stopped the listing.
    Stopped(Stopped),
}

impl ListEnd {
    /// What could not be done, in `ls`'s words, and why; `None` when the
    /// caller stopped.
    pub fn told(&self) -> Option<(&'static str, &io::Error)> {
        match self {
            Self::Access(error) => Some(("cannot access", error)),
            Self::Open(error) => Some(("cannot open directory", error)),
            Self::Read(error) => Some(("reading directory", error)),
            Self::Stopped(_) => None,
        }
    }
}

#[cfg(test)]
thread_local! {
    static LOOKUPS: std::cell::Cell<usize> = const { std::cell::Cell::new(0) };
}

/// How many times this thread's listings have asked the filesystem about
/// one entry — a `stat` each — since the count was last taken.
#[cfg(test)]
pub(crate) fn lookups() -> usize {
    LOOKUPS.with(|count| count.replace(0))
}

fn counted() {
    #[cfg(test)]
    LOOKUPS.with(|count| count.set(count.get() + 1));
}

/// What the long form prints of an entry looked at; `link_target` is
/// asked of a link alone.
fn facts(looked: &crate::held::Looked, link_target: impl FnOnce() -> io::Result<PathBuf>) -> Facts {
    Facts {
        kind: looked.kind,
        size: looked.len,
        mtime_seconds: looked.mtime_seconds,
        allocated_bytes: looked.allocated,
        link_target: if looked.kind == Kind::Link {
            link_target().ok()
        } else {
            None
        },
    }
}

/// Whether `ls` hides a name unless all are asked for.
fn hidden(name: &std::ffi::OsStr) -> bool {
    name.as_encoded_bytes().first() == Some(&b'.')
}

/// Whether `ls` shows a link it could not follow as the link itself. It
/// does for one that points to nothing, or leads round in a ring, and for
/// no other reason (cr-uv5-5; measured, GNU ls 9.12 — `ENOENT` and
/// `ELOOP`): a link to a file below a directory that cannot be searched,
/// a link through a file, a link by a name too long is `cannot access`,
/// exit 2 — and each was printed as if nothing were wrong.
fn shown_as_the_link(error: &io::Error) -> bool {
    #[cfg(unix)]
    {
        error.kind() == io::ErrorKind::NotFound || error.raw_os_error() == Some(libc::ELOOP)
    }
    // No `ls` to measure there: as it was.
    #[cfg(not(unix))]
    {
        let _ = error;
        true
    }
}

/// List `start` as `ls` does, handing `emit` what the operand is and then
/// each entry. `start` is written for the system as it was typed: with a
/// separator after it, the system looks through a link standing there and
/// refuses what is no directory. `root`, when given, is the directory
/// nothing above which is shown (a path with no link in it) — and
/// nothing out of which is read (cr-uv9-1): the start is reached from it
/// one name at a time, and the directory listed is read through what
/// holds it ([`crate::held`]), never by a path looked up again.
pub fn list(
    start: &Path,
    root: Option<&Path>,
    options: ListOptions,
    emit: &mut dyn FnMut(ListEvent) -> Result<(), Stopped>,
) -> Result<(), ListEnd> {
    let held = crate::held::Start::at(start, root).map_err(ListEnd::Access)?;
    counted();
    let own = held.look().map_err(ListEnd::Access)?;
    let mut directory = own.is_dir();
    let mut through = false;
    if own.kind == Kind::Link && !options.link_itself {
        // `ls link` lists the directory a link points to. A link to
        // anything else, or to nothing, is a line of its own — and one
        // that cannot be followed for another reason is an operand that
        // cannot be looked up (cr-uv5-5).
        counted();
        directory = match held.look_through() {
            Ok(target) => target.is_dir(),
            Err(error) if shown_as_the_link(&error) => false,
            Err(error) => return Err(ListEnd::Access(error)),
        };
        through = directory;
    }
    if !directory {
        let itself = Entry {
            name: OsString::new(),
            facts: options.facts.then(|| facts(&own, || held.link_target())),
        };
        return emit(ListEvent::Itself(itself)).map_err(ListEnd::Stopped);
    }
    let opened = if through {
        held.enter_through()
    } else {
        held.enter()
    };
    let mut entries = opened.map_err(ListEnd::Open)?;
    emit(ListEvent::Directory).map_err(ListEnd::Stopped)?;
    if options.all {
        for name in [".", ".."] {
            let name = OsString::from(name);
            let mut found = None;
            if options.facts {
                counted();
                let looked_up = if name == "." {
                    entries.itself()
                } else {
                    entries.above()
                };
                match looked_up {
                    Ok(looked) => {
                        found = Some(facts(&looked, || Err(io::ErrorKind::InvalidInput.into())))
                    }
                    Err(error) => emit(ListEvent::Failure {
                        name: name.clone(),
                        error,
                    })
                    .map_err(ListEnd::Stopped)?,
                }
            }
            emit(ListEvent::Entry(Entry { name, facts: found })).map_err(ListEnd::Stopped)?;
        }
    }
    while let Some(entry) = entries.next_entry() {
        let entry = entry.map_err(ListEnd::Read)?;
        let name = entry.name().to_os_string();
        if !options.all && hidden(&name) {
            continue;
        }
        let mut found = None;
        if options.facts {
            counted();
            match entries.look(&entry) {
                Ok(looked) => found = Some(facts(&looked, || entries.link_target(&entry))),
                Err(error) => emit(ListEvent::Failure {
                    name: name.clone(),
                    error,
                })
                .map_err(ListEnd::Stopped)?,
            }
        }
        emit(ListEvent::Entry(Entry { name, facts: found })).map_err(ListEnd::Stopped)?;
    }
    Ok(())
}

/// Half a year, as `ls` counts it: a time older than this, or still to
/// come, is printed with its year in place of its hour.
const HALF_A_YEAR_SECONDS: i64 = 31_556_952 / 2;

/// The time column of `ls -l`, in `zone`: `Nov 20 10:43` for a time in
/// the half year before `now_seconds`, `Jan  2  2025` for any other — an
/// older one, or one still to come.
pub fn time_column<Tz>(zone: &Tz, when_seconds: i64, now_seconds: i64) -> String
where
    Tz: chrono::TimeZone,
    Tz::Offset: std::fmt::Display,
{
    let Some(when) = zone.timestamp_opt(when_seconds, 0).earliest() else {
        // A time no calendar holds: the number, as `ls` prints it.
        return when_seconds.to_string();
    };
    let recent = now_seconds - HALF_A_YEAR_SECONDS < when_seconds && when_seconds <= now_seconds;
    if recent {
        when.format("%b %e %H:%M").to_string()
    } else {
        when.format("%b %e  %Y").to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// cr-uv9-1: what is listed, and how a listing ended.
    #[cfg(unix)]
    fn listed_in(start: &Path, root: &Path) -> (Vec<String>, Option<String>) {
        let mut names = Vec::new();
        let ended = list(start, Some(root), ListOptions::default(), &mut |event| {
            if let ListEvent::Entry(entry) = event {
                names.push(entry.name.to_string_lossy().into_owned());
            }
            Ok(())
        });
        names.sort();
        (names, ended.err().map(|end| format!("{end:?}")))
    }

    /// cr-uv10-1: and nothing a link put above its root leads to. The root
    /// is a path with no link in it, resolved when it was configured; a
    /// folder above it replaced by a link to another tree led the listing
    /// there — the root opened by its path. It is an error now, for the
    /// root itself and for a path below it.
    #[cfg(unix)]
    #[test]
    fn a_listing_is_not_led_away_by_a_link_above_its_root() {
        let tmp = tempfile::tempdir().unwrap();
        let top = tmp.path().canonicalize().unwrap();
        let root = top.join("above").join("module");
        fs::create_dir_all(root.join("dir")).unwrap();
        fs::write(root.join("dir/inside"), b"").unwrap();
        fs::create_dir_all(top.join("elsewhere/module/dir")).unwrap();
        fs::write(top.join("elsewhere/module/dir/leaked"), b"").unwrap();
        assert_eq!(listed_in(&root, &root).0, ["dir"]);
        assert_eq!(listed_in(&root.join("dir"), &root).0, ["inside"]);
        fs::rename(top.join("above"), top.join("above-was-here")).unwrap();
        std::os::unix::fs::symlink(top.join("elsewhere"), top.join("above")).unwrap();
        for start in [root.clone(), root.join("dir")] {
            let (names, ended) = listed_in(&start, &root);
            assert!(
                names.is_empty() && ended.is_some(),
                "{start:?}: {names:?} {ended:?}"
            );
        }
    }

    /// cr-uv9-1: a listing reads what its root holds and nothing outside
    /// it. A start replaced by a link as it is about to be read, or a
    /// directory on the way to what a start links to, is an error — by
    /// its path the listing went wherever the link pointed — and a start
    /// whose link leads out of the root is one it cannot look up.
    #[cfg(unix)]
    #[test]
    fn a_listing_reads_nothing_outside_its_root() {
        let tmp = tempfile::tempdir().unwrap();
        let top = tmp.path().canonicalize().unwrap();
        let root = top.join("module");
        let outside = top.join("outside");
        fs::create_dir_all(root.join("dir")).unwrap();
        fs::create_dir_all(root.join("real")).unwrap();
        fs::create_dir_all(&outside).unwrap();
        fs::write(root.join("dir/inside"), b"").unwrap();
        fs::write(root.join("real/inside"), b"").unwrap();
        fs::write(outside.join("leaked"), b"").unwrap();
        std::os::unix::fs::symlink("real", root.join("link")).unwrap();
        std::os::unix::fs::symlink(&outside, root.join("out")).unwrap();
        assert_eq!(listed_in(&root.join("dir"), &root).0, ["inside"]);
        assert_eq!(listed_in(&root.join("link/"), &root).0, ["inside"]);

        // The start, replaced as it is about to be read.
        let (dir, out) = (root.join("dir"), outside.clone());
        let moved = top.join("dir-was-here");
        let hook = crate::held::before_each_entering(move |path| {
            if path == dir.as_path() && !moved.exists() {
                fs::rename(&dir, &moved).unwrap();
                std::os::unix::fs::symlink(&out, &dir).unwrap();
            }
        });
        let (names, ended) = listed_in(&root.join("dir"), &root);
        drop(hook);
        assert!(names.is_empty() && ended.is_some(), "{names:?} {ended:?}");

        // A directory on the way to what a start links to, replaced once
        // the link has been followed and the listing goes there.
        let (real, out) = (root.join("real"), outside.clone());
        let moved = top.join("real-was-here");
        let mut seen = 0;
        let hook = crate::held::before_each_entering(move |path| {
            if path == real.as_path() {
                seen += 1;
                if seen == 2 {
                    fs::rename(&real, &moved).unwrap();
                    std::os::unix::fs::symlink(&out, &real).unwrap();
                }
            }
        });
        let (names, ended) = listed_in(&root.join("link/"), &root);
        drop(hook);
        assert!(names.is_empty() && ended.is_some(), "{names:?} {ended:?}");

        // A link out of the root.
        let (names, ended) = listed_in(&root.join("out/"), &root);
        assert!(names.is_empty() && ended.is_some(), "{names:?} {ended:?}");
    }

    /// A directory with every kind of entry the rules turn on.
    fn tree() -> tempfile::TempDir {
        let tmp = tempfile::tempdir().expect("tempdir");
        let t = tmp.path().join("t");
        fs::create_dir_all(t.join("sub")).unwrap();
        fs::create_dir(t.join("Empty")).unwrap();
        fs::write(t.join("a.txt"), b"hello\n").unwrap();
        fs::write(t.join("A.txt"), b"x").unwrap();
        fs::write(t.join(".dot"), b"x").unwrap();
        fs::write(t.join("sub/inner"), b"x").unwrap();
        #[cfg(unix)]
        {
            std::os::unix::fs::symlink("sub", t.join("link")).unwrap();
            std::os::unix::fs::symlink("a.txt", t.join("flink")).unwrap();
            std::os::unix::fs::symlink("nowhere", t.join("broken")).unwrap();
        }
        tmp
    }

    /// What a listing said: whether the operand was listed into, the
    /// operand's own entry when it was not, and the names, sorted.
    struct Said {
        directory: bool,
        itself: Option<Entry>,
        entries: Vec<Entry>,
        failures: Vec<String>,
    }

    impl Said {
        fn names(&self) -> Vec<String> {
            let mut names: Vec<String> = self
                .entries
                .iter()
                .map(|entry| entry.name.to_string_lossy().into_owned())
                .collect();
            names.sort();
            names
        }

        fn facts_of(&self, name: &str) -> &Facts {
            self.entries
                .iter()
                .find(|entry| entry.name == name)
                .and_then(|entry| entry.facts.as_ref())
                .unwrap_or_else(|| panic!("no facts for {name}"))
        }
    }

    fn listed(start: &Path, root: Option<&Path>, options: ListOptions) -> Result<Said, ListEnd> {
        let mut said = Said {
            directory: false,
            itself: None,
            entries: Vec::new(),
            failures: Vec::new(),
        };
        list(start, root, options, &mut |event| {
            match event {
                ListEvent::Itself(entry) => said.itself = Some(entry),
                ListEvent::Directory => said.directory = true,
                ListEvent::Entry(entry) => said.entries.push(entry),
                ListEvent::Failure { name, error } => said
                    .failures
                    .push(format!("{}: {error}", name.to_string_lossy())),
            }
            Ok(())
        })?;
        Ok(said)
    }

    const PLAIN: ListOptions = ListOptions {
        all: false,
        facts: false,
        link_itself: false,
    };
    const LONG: ListOptions = ListOptions {
        all: false,
        facts: true,
        link_itself: true,
    };

    /// AC-6: a directory is listed into; names that begin with `.` are
    /// left out unless all are asked for, when `.` and `..` are entries
    /// too. And (AC-7) a listing without facts looks up the operand and
    /// nothing else, however many entries it gives.
    #[test]
    fn a_directory_is_listed_into_and_its_entries_are_not_looked_up() {
        let tmp = tree();
        let t = tmp.path().join("t");
        let _ = lookups();
        let plain = listed(&t, None, PLAIN).unwrap();
        assert_eq!(lookups(), 1, "the operand, and no entry");
        assert!(plain.directory && plain.itself.is_none());
        let mut expected = vec!["A.txt", "Empty", "a.txt", "sub"];
        if cfg!(unix) {
            expected.extend(["broken", "flink", "link"]);
            expected.sort();
        }
        assert_eq!(plain.names(), expected);
        assert!(plain.entries.iter().all(|entry| entry.facts.is_none()));

        let all = listed(&t, None, ListOptions { all: true, ..PLAIN }).unwrap();
        assert_eq!(lookups(), 1, "nor with -a");
        let names = all.names();
        assert_eq!(&names[..3], [".", "..", ".dot"], "{names:?}");
        assert_eq!(names.len(), expected.len() + 3);

        // With its facts, each entry is asked once.
        let long = listed(&t, None, LONG).unwrap();
        assert_eq!(lookups(), 1 + expected.len());
        assert!(long.failures.is_empty(), "{:?}", long.failures);
        let file = long.facts_of("a.txt");
        assert_eq!((file.kind, file.size), (Kind::File, 6));
        assert_eq!(long.facts_of("sub").kind, Kind::Directory);
        assert!(file.mtime_seconds > 1_600_000_000);
        // An empty directory is listed into, and holds nothing.
        let empty = listed(&t.join("Empty"), None, PLAIN).unwrap();
        assert!(empty.directory && empty.entries.is_empty());
    }

    /// AC-6: an operand that is no directory is one line, itself — which
    /// tells a file from a directory holding one entry.
    #[test]
    fn an_operand_that_is_no_directory_is_itself() {
        let tmp = tree();
        let t = tmp.path().join("t");
        let file = listed(&t.join("a.txt"), None, PLAIN).unwrap();
        assert!(!file.directory && file.entries.is_empty());
        let itself = file.itself.expect("the file's own line");
        assert!(itself.name.is_empty() && itself.facts.is_none());
        let long = listed(&t.join("a.txt"), None, LONG).unwrap();
        let facts = long.itself.expect("its line").facts.expect("its facts");
        assert_eq!((facts.kind, facts.size), (Kind::File, 6));
        // A directory holding one entry lists that entry.
        let one = listed(&t.join("sub"), None, PLAIN).unwrap();
        assert!(one.directory && one.itself.is_none());
        assert_eq!(one.names(), ["inner"]);
        // What is not there, and a file typed as a directory.
        assert!(matches!(
            listed(&t.join("missing"), None, PLAIN),
            Err(ListEnd::Access(error)) if error.kind() == io::ErrorKind::NotFound
        ));
        let (told, _) = ListEnd::Access(io::Error::other("x")).told().unwrap();
        assert_eq!(told, "cannot access");
    }

    /// AC-6, AC-12: a link to a directory is listed into unless the long
    /// form was asked for, when it is the link, with where it points;
    /// typed with a separator after it, it is the directory either way. A
    /// link to a file, or to nothing, is a line of its own; and a link
    /// among a directory's entries is never looked through.
    #[cfg(unix)]
    #[test]
    fn a_link_is_listed_as_ls_lists_it() {
        let tmp = tree();
        let t = tmp.path().join("t");
        let plain = listed(&t.join("link"), None, PLAIN).unwrap();
        assert!(plain.directory, "ls link: the directory");
        assert_eq!(plain.names(), ["inner"]);
        let long = listed(&t.join("link"), None, LONG).unwrap();
        assert!(!long.directory, "ls -l link: the link");
        let facts = long.itself.expect("the link's line").facts.expect("facts");
        assert_eq!(facts.kind, Kind::Link);
        assert_eq!(facts.link_target.as_deref(), Some(Path::new("sub")));
        assert_eq!(facts.size, 3, "a link's size is its target's length");
        for options in [PLAIN, LONG] {
            let typed = listed(&t.join("link").join(""), None, options).unwrap();
            assert!(typed.directory, "ls link/: the directory, {options:?}");
            assert_eq!(typed.names(), ["inner"]);
        }
        for name in ["flink", "broken"] {
            let plain = listed(&t.join(name), None, PLAIN).unwrap();
            assert!(!plain.directory && plain.itself.is_some(), "{name}");
        }
        assert!(matches!(
            listed(&t.join("broken").join(""), None, PLAIN),
            Err(ListEnd::Access(error)) if error.kind() == io::ErrorKind::NotFound
        ));
        assert!(matches!(
            listed(&t.join("a.txt").join(""), None, PLAIN),
            Err(ListEnd::Access(_))
        ));
        // Among the entries: the link, and where it points.
        let entries = listed(&t, None, LONG).unwrap();
        let link = entries.facts_of("link");
        assert_eq!(link.kind, Kind::Link);
        assert_eq!(link.link_target.as_deref(), Some(Path::new("sub")));
        assert_eq!(
            entries.facts_of("broken").link_target.as_deref(),
            Some(Path::new("nowhere"))
        );
        assert_eq!(entries.facts_of("a.txt").link_target, None);
    }

    /// cr-uv5-5: `ls link` of a link that cannot be followed. It is the
    /// link's own line where the link points to nothing or leads round in
    /// a ring, and `cannot access` for any other reason (measured, GNU ls
    /// 9.12). With the long form it is the link, whatever it points to.
    #[cfg(unix)]
    #[test]
    fn a_link_that_cannot_be_followed_is_itself_only_where_ls_shows_it() {
        use std::os::unix::fs::symlink;

        let tmp = tree();
        let t = tmp.path().join("t");
        symlink("ring", t.join("ring")).unwrap();
        symlink("a.txt/x", t.join("through-a-file")).unwrap();
        symlink("n".repeat(300), t.join("too-long")).unwrap();
        for name in ["broken", "ring"] {
            let plain = listed(&t.join(name), None, PLAIN).unwrap();
            assert!(!plain.directory && plain.itself.is_some(), "{name}");
        }
        for (name, reason) in [
            ("through-a-file", libc::ENOTDIR),
            ("too-long", libc::ENAMETOOLONG),
        ] {
            match listed(&t.join(name), None, PLAIN) {
                Err(ListEnd::Access(error)) => {
                    assert_eq!(error.raw_os_error(), Some(reason), "{name}")
                }
                Err(other) => panic!("{name}: {other:?}"),
                Ok(_) => panic!("{name}: listed as if it could be followed"),
            }
            let long = listed(&t.join(name), None, LONG).unwrap();
            assert!(!long.directory, "ls -l {name}: the link");
            let facts = long.itself.expect("the link's line").facts.expect("facts");
            assert_eq!(facts.kind, Kind::Link, "{name}");
        }
    }

    /// With `-a` and its facts, `.` is the directory and `..` the one
    /// above it — and above the root given there is nothing: its `..` is
    /// itself, so a module's listing says nothing of where the module is.
    #[test]
    fn dot_dot_of_the_root_is_the_root() {
        let tmp = tree();
        let root = fs::canonicalize(tmp.path().join("t")).unwrap();
        fs::write(root.join("sub/second"), b"x").unwrap();
        let all = ListOptions { all: true, ..LONG };
        let at_root = listed(&root, Some(&root), all).unwrap();
        assert!(at_root.failures.is_empty(), "{:?}", at_root.failures);
        assert_eq!(at_root.facts_of("."), at_root.facts_of(".."));
        assert_eq!(at_root.facts_of(".").kind, Kind::Directory);
        // Below it, `..` is the directory above: the root, here.
        let below = listed(&root.join("sub"), Some(&root), all).unwrap();
        assert_eq!(below.facts_of(".."), at_root.facts_of("."));
        assert_eq!(below.facts_of(".").kind, Kind::Directory);
        // With no root given, above is above.
        let free = listed(&root, None, all).unwrap();
        let outside = fs::metadata(tmp.path()).unwrap();
        assert_eq!(free.facts_of("..").size, outside.len());
    }

    /// The letters `ls -l` begins a line with.
    #[test]
    fn each_kind_has_ls_s_letter() {
        let letters: String = [
            Kind::File,
            Kind::Directory,
            Kind::Link,
            Kind::Fifo,
            Kind::Socket,
            Kind::CharDevice,
            Kind::BlockDevice,
            Kind::Other,
        ]
        .iter()
        .map(|kind| kind.letter())
        .collect();
        assert_eq!(letters, "-dlpscb?");
    }

    /// The time column, as GNU ls 9.12 printed it at 2026-11-21T11:32Z
    /// under `TZ=UTC`: the hour for a time in the half year before now,
    /// the year for an older one and for one still to come.
    #[test]
    fn the_time_column_is_ls_s() {
        let at = |text: &str| {
            chrono::DateTime::parse_from_rfc3339(text)
                .unwrap()
                .timestamp()
        };
        let now = at("2026-11-21T11:32:00Z");
        let utc = chrono::Utc;
        for (when, printed) in [
            ("2026-11-20T10:43:00Z", "Nov 20 10:43"),
            ("2025-01-02T03:04:05Z", "Jan  2  2025"),
            ("2027-06-01T00:00:00Z", "Jun  1  2027"),
            ("2026-11-21T11:32:00Z", "Nov 21 11:32"),
            ("2026-11-21T11:32:01Z", "Nov 21  2026"),
            ("2026-06-01T09:05:00Z", "Jun  1 09:05"),
        ] {
            assert_eq!(time_column(&utc, at(when), now), printed, "{when}");
        }
        // The edge: half of 31,556,952 seconds before now.
        let edge = now - HALF_A_YEAR_SECONDS;
        assert!(
            time_column(&utc, edge, now).ends_with("2026"),
            "at the edge"
        );
        assert!(time_column(&utc, edge + 1, now).contains(':'), "inside it");
        // Another zone reads the same moment by its own clock.
        let east = chrono::FixedOffset::east_opt(2 * 3600).unwrap();
        assert_eq!(
            time_column(&east, at("2026-11-20T23:30:00Z"), now),
            "Nov 21 01:30"
        );
    }
}
