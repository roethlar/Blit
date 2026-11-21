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

/// How a number an entry has is held against the number typed: `-N`,
/// `N`, `+N`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Compare {
    Less,
    Exactly,
    More,
}

impl Compare {
    fn holds(self, has: i128, typed: i128) -> bool {
        match self {
            Self::Less => has < typed,
            Self::Exactly => has == typed,
            Self::More => has > typed,
        }
    }
}

/// `-size [+|-]N[ckMG]` (slice 4, AC-5), as the reference counts it: an
/// entry's size in UNITS, rounded UP — so `-size 1k` holds for every file
/// of 1 to 1024 bytes, and `-size -1k` for an empty one alone (GNU find
/// 4.11.0, run 2026-11-21).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SizeTest {
    pub compare: Compare,
    pub count: u64,
    /// Bytes to a unit: 512 with no letter, 1 for `c`, 1024 for `k`, and
    /// so on. Never 0.
    pub unit_bytes: u64,
}

impl SizeTest {
    pub fn holds(&self, bytes: u64) -> bool {
        let units = bytes.div_ceil(self.unit_bytes.max(1));
        self.compare
            .holds(i128::from(units), i128::from(self.count))
    }
}

/// `-mtime [+|-]N`, as the reference counts it: whole 24-hour periods
/// since the entry was last written, any part of one dropped — so
/// `-mtime 0` is "in the last 24 hours" and `-mtime +0` "more than one
/// day ago". A time still to come is less than none (`-mtime -1` holds,
/// `-mtime 0` does not; same run).
///
/// The two moments are subtracted as they are, to the nanosecond, and
/// what has ELAPSED is cut to whole periods (cr-uv4-4). Each cut to a
/// whole second first, a file 86,399.12 seconds old could count as a day
/// old — and go, with `-delete`, before its day was up.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AgeTest {
    pub compare: Compare,
    pub days: u64,
}

/// A moment as nanoseconds since 1970 began; before it, less than none.
pub fn nanos_of(moment: std::time::SystemTime) -> i128 {
    match moment.duration_since(std::time::UNIX_EPOCH) {
        Ok(since) => since.as_nanos() as i128,
        Err(before) => -(before.duration().as_nanos() as i128),
    }
}

impl AgeTest {
    /// Whether an entry written at `written_nanos` passes at `now_nanos`
    /// ([`nanos_of`]).
    pub fn holds(&self, now_nanos: i128, written_nanos: i128) -> bool {
        const A_DAY_NANOS: i128 = 24 * 60 * 60 * 1_000_000_000;
        let periods = (now_nanos - written_nanos).div_euclid(A_DAY_NANOS);
        self.compare.holds(periods, i128::from(self.days))
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
    /// `-size`, every one of which must hold.
    pub sizes: Vec<SizeTest>,
    /// `-mtime`, every one of which must hold.
    pub ages: Vec<AgeTest>,
    /// The moment ages are counted from ([`nanos_of`]): when the walk was
    /// asked for, on the clock of the machine that walks.
    pub now_nanos: i128,
    /// An entry is given after everything below it — `find`'s `-depth`,
    /// which `-delete` turns on: what is in a directory goes before the
    /// directory can.
    pub contents_first: bool,
}

impl FindOptions {
    /// The tests an entry's name and kind answer, which its directory's
    /// listing gives for nothing.
    fn holds(&self, name: &[u8], kind: Option<Kind>) -> bool {
        self.kinds.iter().all(|wanted| Some(*wanted) == kind)
            && self.names.iter().all(|pattern| pattern.matches(name))
    }

    /// Whether any test asks what only the entry itself can say.
    fn needs_facts(&self) -> bool {
        !self.sizes.is_empty() || !self.ages.is_empty()
    }

    /// The tests the entry itself answers — a link as the link it is.
    fn facts_hold(&self, metadata: &fs::Metadata) -> bool {
        self.sizes.iter().all(|test| test.holds(metadata.len()))
            && (self.ages.is_empty() || {
                // The time as the filesystem keeps it, every digit.
                let written = metadata.modified().map_or(0, nanos_of);
                self.ages
                    .iter()
                    .all(|test| test.holds(self.now_nanos, written))
            })
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
    /// The walk goes on: it is about to read a directory, or has read
    /// another [`ENTRIES_BETWEEN_WORDS`] entries of one. Nothing was
    /// found and nothing failed — it is the caller's moment to stop a
    /// walk nobody is waiting for (cr-uv4-2), which a walk that finds
    /// nothing, or removes what it finds and says nothing of it, would
    /// otherwise never give.
    Walking,
    /// An entry every test holds for: its path below the start, empty for
    /// the start itself — and what kind of entry it is, where it is one of
    /// the three.
    Found(PathBuf, Option<Kind>),
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

/// How many entries of one directory the walk reads between two
/// [`FindEvent::Walking`].
pub const ENTRIES_BETWEEN_WORDS: usize = 1024;

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
    let start_holds = options.holds(start_name, kind) && options.facts_hold(&metadata);
    let entered = kind == Some(Kind::Directory) && options.max_depth != Some(0);
    // Contents first: a start that is entered is given last.
    let start_last = start_holds && options.contents_first && entered;
    if start_holds && !start_last {
        emit(FindEvent::Found(PathBuf::new(), kind)).map_err(FindEnd::Stopped)?;
    }
    if !entered {
        return Ok(());
    }

    /// One directory being read.
    struct Frame {
        relative_path: PathBuf,
        entries: fs::ReadDir,
        /// The directory is itself found, and is given once everything
        /// in it has been (`contents_first`).
        found_last: bool,
        /// Entries read since the caller last had a word.
        read: usize,
    }
    let mut frames: Vec<Frame> = Vec::new();
    emit(FindEvent::Walking).map_err(FindEnd::Stopped)?;
    match fs::read_dir(start) {
        Ok(entries) => frames.push(Frame {
            relative_path: PathBuf::new(),
            entries,
            found_last: start_last,
            read: 0,
        }),
        Err(error) => {
            emit(FindEvent::Failure(FindFailure {
                relative_path: PathBuf::new(),
                error,
            }))
            .map_err(FindEnd::Stopped)?;
            if start_last {
                emit(FindEvent::Found(PathBuf::new(), kind)).map_err(FindEnd::Stopped)?;
            }
            return Ok(());
        }
    }
    while let Some(frame) = frames.last_mut() {
        let Some(entry) = frame.entries.next() else {
            let read = frames.pop().expect("the frame just read");
            if read.found_last {
                emit(FindEvent::Found(read.relative_path, Some(Kind::Directory)))
                    .map_err(FindEnd::Stopped)?;
            }
            continue;
        };
        frame.read += 1;
        if frame.read >= ENTRIES_BETWEEN_WORDS {
            frame.read = 0;
            emit(FindEvent::Walking).map_err(FindEnd::Stopped)?;
        }
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
        let mut holds = options.holds(&name_bytes(&name), kind);
        // The entry itself is asked only when a test needs what only it
        // can say, and only once its name and kind have held (AC-7).
        if holds && options.needs_facts() {
            match facts_of(&entry) {
                Ok(metadata) => holds = options.facts_hold(&metadata),
                Err(error) => {
                    holds = false;
                    emit(FindEvent::Failure(FindFailure {
                        relative_path: relative_path.clone(),
                        error,
                    }))
                    .map_err(FindEnd::Stopped)?;
                }
            }
        }
        // This entry is `frames.len()` levels below the start.
        let enter = kind == Some(Kind::Directory)
            && options
                .max_depth
                .is_none_or(|max| (frames.len() as u32) < max);
        let last = holds && options.contents_first && enter;
        if holds && !last {
            emit(FindEvent::Found(relative_path.clone(), kind)).map_err(FindEnd::Stopped)?;
        }
        if !enter {
            continue;
        }
        emit(FindEvent::Walking).map_err(FindEnd::Stopped)?;
        match fs::read_dir(entry.path()) {
            Ok(entries) => frames.push(Frame {
                relative_path,
                entries,
                found_last: last,
                read: 0,
            }),
            Err(error) => {
                emit(FindEvent::Failure(FindFailure {
                    relative_path: relative_path.clone(),
                    error,
                }))
                .map_err(FindEnd::Stopped)?;
                if last {
                    emit(FindEvent::Found(relative_path, kind)).map_err(FindEnd::Stopped)?;
                }
            }
        }
    }
    Ok(())
}

/// What the entry itself says — its size, when it was written — a link as
/// the link it is. One lookup, counted like the start's.
fn facts_of(entry: &fs::DirEntry) -> io::Result<fs::Metadata> {
    #[cfg(test)]
    LOOKUPS.with(|count| count.set(count.get() + 1));
    entry.metadata()
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
                FindEvent::Walking => {}
                FindEvent::Found(path, _) => found.push(path.to_string_lossy().replace('\\', "/")),
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
            if let FindEvent::Found(path, _) = event {
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
                ..FindOptions::default()
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
    /// Slice 4 (AC-5): `-size` and `-mtime` count as the reference does.
    /// Each row is what GNU find 4.11.0 printed (run 2026-11-21) for files
    /// of these sizes, and of these ages against one moment.
    #[test]
    fn size_and_age_are_counted_as_find_counts_them() {
        let sizes: [(&str, u64); 12] = [
            ("a.tmp", 0),
            ("b511", 511),
            ("b512", 512),
            ("b513", 513),
            ("empty", 0),
            ("k1", 1024),
            ("k1p", 1025),
            ("k2", 2048),
            ("m1", 1024 * 1024),
            ("m1m", 1024 * 1024 - 1),
            ("m1p", 1024 * 1024 + 1),
            ("one", 1),
        ];
        let (b, c, k, m, g) = (512, 1, 1024, 1024 * 1024, 1024 * 1024 * 1024);
        use Compare::{Exactly, Less, More};
        let every = "b511 b512 b513 k1 k1p k2 m1 m1m m1p one";
        for (compare, count, unit_bytes, found) in [
            (Exactly, 0, b, "a.tmp empty"),
            (Exactly, 1, b, "b511 b512 one"),
            (Exactly, 2, b, "b513 k1"),
            (More, 1, b, "b513 k1 k1p k2 m1 m1m m1p"),
            (Less, 1, b, "a.tmp empty"),
            (Less, 2, b, "a.tmp b511 b512 empty one"),
            (Exactly, 1, c, "one"),
            (Exactly, 512, c, "b512"),
            (More, 512, c, "b513 k1 k1p k2 m1 m1m m1p"),
            (Less, 512, c, "a.tmp b511 empty one"),
            (Exactly, 1, k, "b511 b512 b513 k1 one"),
            (Exactly, 2, k, "k1p k2"),
            (More, 1, k, "k1p k2 m1 m1m m1p"),
            (Less, 1, k, "a.tmp empty"),
            (Less, 2, k, "a.tmp b511 b512 b513 empty k1 one"),
            (Exactly, 1, m, "b511 b512 b513 k1 k1p k2 m1 m1m one"),
            (More, 1, m, "m1p"),
            (Less, 1, m, "a.tmp empty"),
            (Exactly, 2, m, "m1p"),
            (Exactly, 1, g, every),
            (Less, 1, g, "a.tmp empty"),
            (Exactly, 0, c, "a.tmp empty"),
            (More, 0, b, every),
        ] {
            let test = SizeTest {
                compare,
                count,
                unit_bytes,
            };
            let holds: Vec<&str> = sizes
                .iter()
                .filter(|(_, bytes)| test.holds(*bytes))
                .map(|(name, _)| *name)
                .collect();
            assert_eq!(holds.join(" "), found, "{test:?}");
        }

        let hour = 60 * 60;
        let now = 1_800_000_000i64;
        let ages: [(&str, i64); 7] = [
            ("d10", 10 * 24 * hour + hour),
            ("future", -hour),
            ("h23", 23 * hour),
            ("h25", 25 * hour),
            ("h47", 47 * hour),
            ("h49", 49 * hour),
            ("now", 0),
        ];
        for (compare, days, found) in [
            (Exactly, 0, "h23 now"),
            (Exactly, 1, "h25 h47"),
            (Exactly, 2, "h49"),
            (More, 0, "d10 h25 h47 h49"),
            (More, 1, "d10 h49"),
            (Less, 1, "future h23 now"),
            (Less, 2, "future h23 h25 h47 now"),
            (More, 9, "d10"),
            (Exactly, 10, "d10"),
        ] {
            let test = AgeTest { compare, days };
            let second = 1_000_000_000i128;
            let holds: Vec<&str> = ages
                .iter()
                .filter(|(_, ago)| {
                    test.holds(i128::from(now) * second, i128::from(now - ago) * second)
                })
                .map(|(name, _)| *name)
                .collect();
            assert_eq!(holds.join(" "), found, "{test:?}");
        }
    }

    /// cr-uv4-2: the walk gives its caller a word before each directory
    /// it reads and every [`ENTRIES_BETWEEN_WORDS`] entries of one, found
    /// or not — and stops there when told to, before anything more is
    /// read or given.
    #[test]
    fn the_walk_can_be_stopped_though_it_finds_nothing() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let start = tmp.path().join("t");
        fs::create_dir_all(start.join("a/b")).unwrap();
        fs::create_dir(start.join("many")).unwrap();
        for n in 0..(2 * ENTRIES_BETWEEN_WORDS + 5) {
            fs::write(start.join("many").join(format!("f{n}")), b"").unwrap();
        }
        let nothing = FindOptions {
            names: vec![named("no such name", false)],
            ..FindOptions::default()
        };
        let mut words = 0;
        find(&start, b"t", &nothing, &mut |event| {
            match event {
                FindEvent::Walking => words += 1,
                other => panic!("nothing is found: {other:?}"),
            }
            Ok(())
        })
        .expect("the walk");
        // Four directories read, and two more words inside the long one.
        assert_eq!(words, 4 + 2);
        // Stopped at the first word: nothing was read, nothing given.
        let mut given = Vec::new();
        let ended = find(
            &start,
            b"t",
            &FindOptions::default(),
            &mut |event| match event {
                FindEvent::Walking => Err(Stopped),
                FindEvent::Found(path, _) => {
                    given.push(path);
                    Ok(())
                }
                FindEvent::Failure(failed) => panic!("{failed:?}"),
            },
        );
        assert!(matches!(ended, Err(FindEnd::Stopped(Stopped))));
        assert_eq!(given, [PathBuf::new()], "the start, and no more");
        // Contents first, the start is not given at all: it is given
        // last, and the walk never came to it.
        let mut given = 0;
        let ended = find(
            &start,
            b"t",
            &FindOptions {
                contents_first: true,
                ..FindOptions::default()
            },
            &mut |event| match event {
                FindEvent::Walking => Err(Stopped),
                _ => {
                    given += 1;
                    Ok(())
                }
            },
        );
        assert!(matches!(ended, Err(FindEnd::Stopped(Stopped))));
        assert_eq!(given, 0);
    }

    /// cr-uv4-4: an age is what has ELAPSED, cut to whole days — not the
    /// difference of two times each cut to a whole second. The reviewer's
    /// case: a file 86,399.12 seconds old is not yet a day old, whatever
    /// part of a second the clock stands at; GNU find leaves it out of
    /// `-mtime +0`, and so `-delete` leaves it.
    #[test]
    fn an_age_is_counted_from_the_whole_of_both_times() {
        let second = 1_000_000_000i128;
        let day = 24 * 60 * 60 * second;
        let over_a_day = AgeTest {
            compare: Compare::More,
            days: 0,
        };
        let today = AgeTest {
            compare: Compare::Exactly,
            days: 0,
        };
        // Written at .88 of a second, looked at 86,399.12 seconds later —
        // on the second: the two whole seconds are 86,400 apart.
        let written = 1_800_000_000 * second + 880_000_000;
        let now = written + 86_399 * second + 120_000_000;
        assert_eq!(now % second, 0, "the clock stands on the second");
        assert_eq!((now / second) - (written / second), 86_400);
        assert!(!over_a_day.holds(now, written), "not a day old yet");
        assert!(today.holds(now, written));
        // One nanosecond short of a day, and the day itself.
        assert!(!over_a_day.holds(written + day - 1, written));
        assert!(over_a_day.holds(written + day, written));
        assert!(!today.holds(written + day, written));
        // A time to come, by less than a second, is less than no age.
        let ahead = AgeTest {
            compare: Compare::Less,
            days: 0,
        };
        assert!(ahead.holds(written, written + 1));
        assert!(!today.holds(written, written + 1));
        // Before 1970 a moment is less than none, and still a moment.
        let before = std::time::UNIX_EPOCH - std::time::Duration::from_nanos(1_500_000_000);
        assert_eq!(nanos_of(before), -1_500_000_000);
        assert_eq!(nanos_of(std::time::UNIX_EPOCH), 0);
    }

    /// cr-uv4-4, at the walk: the time is the one the filesystem keeps,
    /// to the part of a second it keeps.
    #[test]
    fn the_walk_reads_an_entrys_time_whole() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let start = tmp.path().join("t");
        fs::create_dir(&start).unwrap();
        let file = start.join("f");
        fs::write(&file, b"x").unwrap();
        let second = 1_000_000_000i128;
        // Written at .88 of a second.
        let written = filetime::FileTime::from_unix_time(1_800_000_000, 880_000_000);
        filetime::set_file_mtime(&file, written).unwrap();
        let kept = fs::metadata(&file)
            .unwrap()
            .modified()
            .map(nanos_of)
            .unwrap();
        if kept % second == 0 {
            // A filesystem that keeps whole seconds cannot show this.
            return;
        }
        let found = |now_nanos: i128| {
            let options = FindOptions {
                kinds: vec![Kind::File],
                ages: vec![AgeTest {
                    compare: Compare::More,
                    days: 0,
                }],
                now_nanos,
                ..FindOptions::default()
            };
            walk(&start, "t", &options).0
        };
        let on_the_second = 1_800_000_000 * second + 86_400 * second;
        assert!(found(on_the_second).is_empty(), "86,399.12 seconds old");
        assert_eq!(found(on_the_second + 880_000_000), ["f"], "a day old");
    }

    /// Slice 4: with `contents_first` an entry is given after everything
    /// below it — what `-delete` needs — and the entries are the same
    /// ones. And an entry is asked about itself only when a test needs
    /// it, and only once its name and kind have held (AC-7).
    #[test]
    fn contents_come_first_when_asked_and_an_entry_is_asked_only_when_needed() {
        let tmp = tree();
        let start = tmp.path().join("t");
        // In the order given: the helper the other guards use sorts.
        let order = |options: &FindOptions| {
            let mut given = Vec::new();
            find(&start, b"t", options, &mut |event| {
                if let FindEvent::Found(path, _) = event {
                    given.push(path.to_string_lossy().replace('\\', "/"));
                }
                Ok(())
            })
            .expect("the walk");
            given
        };
        let plain = order(&FindOptions::default());
        let first = order(&FindOptions {
            contents_first: true,
            ..FindOptions::default()
        });
        assert_eq!(
            plain.first().map(String::as_str),
            Some(""),
            "the start first"
        );
        assert_eq!(first.last().map(String::as_str), Some(""), "the start last");
        let mut sorted = (plain.clone(), first.clone());
        sorted.0.sort();
        sorted.1.sort();
        assert_eq!(sorted.0, sorted.1, "the same entries");
        for (at, path) in first.iter().enumerate() {
            let below = format!("{path}/");
            assert!(
                first[at..]
                    .iter()
                    .all(|later| path.is_empty() || !later.starts_with(&below)),
                "{path} is given after what is in it: {first:?}"
            );
        }
        // No test of the entry itself: one lookup, the start's.
        let _ = lookups();
        let names_only = FindOptions {
            names: vec![named("*.dat", false)],
            ..FindOptions::default()
        };
        let named_ones = order(&names_only).len();
        assert_eq!(lookups(), 1);
        // With one, each entry whose name held is asked, and no other.
        let with_size = FindOptions {
            sizes: vec![SizeTest {
                compare: Compare::More,
                count: 0,
                unit_bytes: 1,
            }],
            ..names_only
        };
        let _ = order(&with_size);
        assert_eq!(lookups(), 1 + named_ones);
    }

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
                ..FindOptions::default()
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
            if let FindEvent::Found(path, _) = event {
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
