//! `du` and `df`, answered as the Unix commands answer them
//! (`docs/plan/UTILITY_VERBS_LOCAL_PARITY.md` slice 1, D-2026-11-20-2).
//!
//! These run on the machine that holds the files — a daemon, for
//! `blit du HOST:/module/path` — so a tree on a network share is measured
//! at local speed. The reference is GNU coreutils under `C.UTF-8`:
//!
//! - [`disk_usage`] walks one start path and reports, for each directory,
//!   the space ALLOCATED to it and to everything below it. Lines come an
//!   entry after everything below it, the start last, in the order the
//!   directories list their entries. A file with several hard links is
//!   counted the first time it is met and not again. A symbolic link is an
//!   entry of its own and is never followed. An entry that cannot be read
//!   is reported and the walk goes on.
//! - [`filesystem_space`] asks the operating system about the filesystem
//!   that holds one path — the path itself, not a list of mounts.
//! - [`blocks_1k`], [`human_size`] and [`use_percent`] are the numbers as
//!   `du` and `df` print them.

use std::collections::HashSet;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

/// What `du` was asked.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct UsageOptions {
    /// `-a`: a line for every file as well as every directory.
    pub all_files: bool,
    /// `-d N`: lines only for entries at most `N` levels below the start;
    /// `None` is every level, and `-s` is `Some(0)`. Totals always count
    /// everything, whatever is printed.
    pub max_depth: Option<u32>,
}

/// One line of `du`'s answer.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UsageRow {
    /// The entry's path below the start; empty for the start itself.
    pub relative_path: PathBuf,
    /// The space allocated to the entry and to everything below it.
    pub allocated_bytes: u64,
}

/// An entry the walk could not read. The walk goes on past it, and the
/// command that asked exits 1, as `du` does.
#[derive(Debug)]
pub struct UsageFailure {
    /// The entry's path below the start; empty for the start itself.
    pub relative_path: PathBuf,
    /// What could not be done, in `du`'s words: `cannot read directory`
    /// or `cannot access`.
    pub action: &'static str,
    pub error: io::Error,
}

/// What [`disk_usage`] hands its caller, in order.
#[derive(Debug)]
pub enum UsageEvent {
    Row(UsageRow),
    Failure(UsageFailure),
}

/// The caller no longer wants the answer (its peer went away).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Stopped;

/// `du` over `start`. `emit` gets each line and each failure as the walk
/// produces it; returning `Err(Stopped)` ends the walk. `Err` from here is
/// the start itself being unreadable (`cannot access`), with nothing
/// emitted, or the caller stopping.
pub fn disk_usage(
    start: &Path,
    options: UsageOptions,
    emit: &mut dyn FnMut(UsageEvent) -> Result<(), Stopped>,
) -> Result<(), UsageEnd> {
    let metadata = fs::symlink_metadata(start).map_err(UsageEnd::Start)?;
    let mut links = HardLinks::default();
    if !metadata.is_dir() {
        // A file, or a link, given as the start: its own line, always.
        let allocated_bytes = links.count(start, &metadata).unwrap_or(0);
        return emit(UsageEvent::Row(UsageRow {
            relative_path: PathBuf::new(),
            allocated_bytes,
        }))
        .map_err(UsageEnd::Stopped);
    }
    let within = |depth: u32| options.max_depth.is_none_or(|max| depth <= max);
    let mut frames: Vec<Frame> = Vec::new();
    match fs::read_dir(start) {
        Ok(entries) => frames.push(Frame {
            relative_path: PathBuf::new(),
            entries,
            total: allocated(&metadata),
        }),
        Err(error) => {
            emit(UsageEvent::Failure(UsageFailure {
                relative_path: PathBuf::new(),
                action: "cannot read directory",
                error,
            }))
            .map_err(UsageEnd::Stopped)?;
            return emit(UsageEvent::Row(UsageRow {
                relative_path: PathBuf::new(),
                allocated_bytes: allocated(&metadata),
            }))
            .map_err(UsageEnd::Stopped);
        }
    }
    while let Some(frame) = frames.last_mut() {
        // The depth of this directory's ENTRIES below the start.
        let depth = frames_depth(frame);
        let next = frame.entries.next();
        let Some(entry) = next else {
            // Everything below this directory is counted: its line, then
            // its total into the directory above.
            let done = frames.pop().expect("the frame just read");
            if within(depth - 1) {
                emit(UsageEvent::Row(UsageRow {
                    relative_path: done.relative_path,
                    allocated_bytes: done.total,
                }))
                .map_err(UsageEnd::Stopped)?;
            }
            if let Some(above) = frames.last_mut() {
                above.total += done.total;
            }
            continue;
        };
        let entry = match entry {
            Ok(entry) => entry,
            Err(error) => {
                let relative_path = frame.relative_path.clone();
                emit(UsageEvent::Failure(UsageFailure {
                    relative_path,
                    action: "cannot read directory",
                    error,
                }))
                .map_err(UsageEnd::Stopped)?;
                continue;
            }
        };
        let relative_path = frame.relative_path.join(entry.file_name());
        // `DirEntry::metadata` never follows a link.
        let metadata = match entry.metadata() {
            Ok(metadata) => metadata,
            Err(error) => {
                emit(UsageEvent::Failure(UsageFailure {
                    relative_path,
                    action: "cannot access",
                    error,
                }))
                .map_err(UsageEnd::Stopped)?;
                continue;
            }
        };
        if metadata.is_dir() {
            match fs::read_dir(entry.path()) {
                Ok(entries) => frames.push(Frame {
                    relative_path,
                    entries,
                    total: allocated(&metadata),
                }),
                Err(error) => {
                    // Its own space still counts and its line is still
                    // printed, as `du` does for a directory it cannot open.
                    let own = allocated(&metadata);
                    frame.total += own;
                    emit(UsageEvent::Failure(UsageFailure {
                        relative_path: relative_path.clone(),
                        action: "cannot read directory",
                        error,
                    }))
                    .map_err(UsageEnd::Stopped)?;
                    if within(depth) {
                        emit(UsageEvent::Row(UsageRow {
                            relative_path,
                            allocated_bytes: own,
                        }))
                        .map_err(UsageEnd::Stopped)?;
                    }
                }
            }
            continue;
        }
        // A file, a link or anything else that is not a directory. One
        // already counted under another name adds nothing and has no line.
        let Some(size) = links.count(&entry.path(), &metadata) else {
            continue;
        };
        frame.total += size;
        if options.all_files && within(depth) {
            emit(UsageEvent::Row(UsageRow {
                relative_path,
                allocated_bytes: size,
            }))
            .map_err(UsageEnd::Stopped)?;
        }
    }
    Ok(())
}

/// Why [`disk_usage`] ended without finishing.
#[derive(Debug)]
pub enum UsageEnd {
    /// The start itself could not be read: `cannot access`.
    Start(io::Error),
    /// The caller stopped the walk.
    Stopped(Stopped),
}

/// One directory being read.
struct Frame {
    relative_path: PathBuf,
    entries: fs::ReadDir,
    /// Its own space and everything counted below it so far.
    total: u64,
}

/// The depth below the start of the entries `frame` lists: 1 for the
/// start's own entries.
fn frames_depth(frame: &Frame) -> u32 {
    frame.relative_path.components().count() as u32 + 1
}

/// The space the operating system has allocated to one entry: its 512-byte
/// blocks on Unix (what `du` counts — a sparse file's holes are not there,
/// a directory's own blocks are); its length where the system does not
/// report allocation, a directory counting nothing of its own.
fn allocated(metadata: &fs::Metadata) -> u64 {
    #[cfg(unix)]
    {
        use std::os::unix::fs::MetadataExt as _;
        metadata.blocks() * 512
    }
    #[cfg(not(unix))]
    {
        if metadata.is_dir() {
            0
        } else {
            metadata.len()
        }
    }
}

/// The files with several names that have been counted already, by
/// identity. Only such files are remembered, as in `du`.
#[derive(Default)]
struct HardLinks {
    seen: HashSet<(u64, u64)>,
}

impl HardLinks {
    /// The space to count for the entry at `path`: its allocation, or
    /// `None` when it is a file with several names already counted under
    /// another of them.
    fn count(&mut self, path: &Path, metadata: &fs::Metadata) -> Option<u64> {
        if let Some(identity) = several_names(path, metadata) {
            if !self.seen.insert(identity) {
                return None;
            }
        }
        Some(allocated(metadata))
    }
}

/// The identity of a file that has more than one name; `None` for a file
/// with one name (the usual case, which costs nothing more to learn on
/// Unix) and for a directory.
fn several_names(path: &Path, metadata: &fs::Metadata) -> Option<(u64, u64)> {
    if metadata.is_dir() {
        return None;
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::MetadataExt as _;
        let _ = path;
        (metadata.nlink() > 1).then(|| (metadata.dev(), metadata.ino()))
    }
    #[cfg(not(unix))]
    {
        // The directory listing does not say how many names a file has;
        // the file has to be opened to learn it (and its identity).
        let (identity, names) = crate::transfers::filter::entry_identity(path)?;
        (names > 1).then_some(identity)
    }
}

/// The filesystem that holds one path, as `df` reports it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FilesystemSpace {
    /// Its size.
    pub total_bytes: u64,
    /// The space in use: its size less every free block, whoever may use it.
    pub used_bytes: u64,
    /// The space an ordinary user may still fill — less than the free
    /// space where the filesystem keeps a reserve.
    pub available_bytes: u64,
}

/// Ask the operating system about the filesystem that holds `path` — the
/// path given, whatever is mounted above or below it.
pub fn filesystem_space(path: &Path) -> io::Result<FilesystemSpace> {
    #[cfg(unix)]
    {
        use std::os::unix::ffi::OsStrExt as _;
        let c_path = std::ffi::CString::new(path.as_os_str().as_bytes())
            .map_err(|error| io::Error::new(io::ErrorKind::InvalidInput, error))?;
        let mut stats = std::mem::MaybeUninit::<libc::statvfs>::uninit();
        // SAFETY: `c_path` is a NUL-terminated string and `stats` is a
        // writable `statvfs`; the call writes it only on success.
        if unsafe { libc::statvfs(c_path.as_ptr(), stats.as_mut_ptr()) } != 0 {
            return Err(io::Error::last_os_error());
        }
        // SAFETY: the call returned 0, so the structure is initialized.
        let stats = unsafe { stats.assume_init() };
        // Counts are in units of the fragment size, as `df` reads them.
        // The fields' integer types differ by platform (64-bit here,
        // 32-bit on macOS), so each is widened where it is read.
        #[allow(clippy::unnecessary_cast)]
        let (unit, blocks, free, available) = (
            stats.f_frsize as u64,
            stats.f_blocks as u64,
            stats.f_bfree as u64,
            stats.f_bavail as u64,
        );
        Ok(FilesystemSpace {
            total_bytes: blocks.saturating_mul(unit),
            used_bytes: blocks.saturating_sub(free).saturating_mul(unit),
            available_bytes: available.saturating_mul(unit),
        })
    }
    #[cfg(windows)]
    {
        use std::os::windows::ffi::OsStrExt as _;
        use windows::core::PCWSTR;
        use windows::Win32::Storage::FileSystem::GetDiskFreeSpaceExW;
        // The call wants a directory; a file's filesystem is its folder's.
        // What the path IS comes from a lookup that can fail (cr-uv1-6):
        // `is_dir()` answers "no" for a path that is not there as it does
        // for a file, so the folder above a missing path was asked about
        // instead — its numbers, and success, where `df` says the path
        // does not exist.
        let directory = if std::fs::metadata(path)?.is_dir() {
            path
        } else {
            path.parent().unwrap_or(path)
        };
        let wide: Vec<u16> = directory
            .as_os_str()
            .encode_wide()
            .chain(std::iter::once(0))
            .collect();
        let (mut available, mut total, mut free) = (0u64, 0u64, 0u64);
        // SAFETY: `wide` is NUL-terminated and the three outputs are
        // writable for the length of the call.
        unsafe {
            GetDiskFreeSpaceExW(
                PCWSTR(wide.as_ptr()),
                Some(&mut available),
                Some(&mut total),
                Some(&mut free),
            )
        }
        .map_err(|error| io::Error::other(error.to_string()))?;
        Ok(FilesystemSpace {
            total_bytes: total,
            used_bytes: total.saturating_sub(free),
            available_bytes: available,
        })
    }
    #[cfg(not(any(unix, windows)))]
    {
        let _ = path;
        Err(io::Error::new(
            io::ErrorKind::Unsupported,
            "filesystem space is not reported on this platform",
        ))
    }
}

/// Why an operation failed, in the operating system's own words and no
/// more — `Permission denied`, `No such file or directory` — as the Unix
/// commands print it after the name (the standard library adds
/// ` (os error 13)`).
pub fn os_reason(error: &io::Error) -> String {
    let text = error.to_string();
    match text.rfind(" (os error ") {
        Some(at) if text.ends_with(')') => text[..at].to_string(),
        _ => text,
    }
}

/// A size in the 1K blocks `du` and `df -k` print: rounded up.
pub fn blocks_1k(bytes: u64) -> u64 {
    bytes.div_ceil(1024)
}

/// A size as `du -h` and `df -h` print it: powers of 1024, rounded UP,
/// one decimal below ten of a unit and none from ten on — `4.0K`, `12K`,
/// `1.5M`, `16G` — and a bare number below 1024.
pub fn human_size(bytes: u64) -> String {
    const UNITS: [char; 6] = ['K', 'M', 'G', 'T', 'P', 'E'];
    if bytes < 1024 {
        return bytes.to_string();
    }
    let mut unit = 0;
    let mut scale: u128 = 1024;
    while unit + 1 < UNITS.len() && u128::from(bytes) >= scale * 1024 {
        scale *= 1024;
        unit += 1;
    }
    let bytes = u128::from(bytes);
    let tenths = (bytes * 10).div_ceil(scale);
    if tenths < 100 {
        return format!("{}.{}{}", tenths / 10, tenths % 10, UNITS[unit]);
    }
    let whole = bytes.div_ceil(scale);
    if whole >= 1024 && unit + 1 < UNITS.len() {
        return format!("1.0{}", UNITS[unit + 1]);
    }
    format!("{whole}{}", UNITS[unit])
}

/// `df`'s `Use%`: the space in use as a share of what is in use plus what
/// an ordinary user may still fill, rounded up; `None` (printed `-`) for
/// a filesystem of no size.
pub fn use_percent(space: &FilesystemSpace) -> Option<u64> {
    let usable = u128::from(space.used_bytes) + u128::from(space.available_bytes);
    (usable > 0).then(|| (u128::from(space.used_bytes) * 100).div_ceil(usable) as u64)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    /// Every line `du` gives for `start`, and every failure, in order.
    fn walk(start: &Path, options: UsageOptions) -> (Vec<(String, u64)>, Vec<(String, String)>) {
        let (mut rows, mut failures) = (Vec::new(), Vec::new());
        let text = |path: &Path| path.to_string_lossy().replace('\\', "/");
        disk_usage(start, options, &mut |event| {
            match event {
                UsageEvent::Row(row) => rows.push((text(&row.relative_path), row.allocated_bytes)),
                UsageEvent::Failure(failure) => {
                    failures.push((text(&failure.relative_path), failure.action.to_string()))
                }
            }
            Ok(())
        })
        .expect("the walk ends");
        (rows, failures)
    }

    fn all_files() -> UsageOptions {
        UsageOptions {
            all_files: true,
            max_depth: None,
        }
    }

    fn file(path: &Path, bytes: usize) {
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, vec![7u8; bytes]).unwrap();
    }

    /// AC-2: one line per directory, an entry after everything below it,
    /// the start last and carrying everything; files only under `-a`; a
    /// depth bounds the lines and never a total; `-s` is the start alone.
    #[test]
    fn du_lists_directories_deepest_first_and_the_start_last() {
        let tmp = tempdir().unwrap();
        let root = tmp.path();
        file(&root.join("a/one.bin"), 5000);
        file(&root.join("a/deep/two.bin"), 70_000);
        file(&root.join("a/deep/er/three.bin"), 300_000);
        file(&root.join("b/four.bin"), 1);
        fs::create_dir_all(root.join("empty")).unwrap();

        let (rows, failures) = walk(root, UsageOptions::default());
        assert!(failures.is_empty());
        let paths: Vec<&str> = rows.iter().map(|(path, _)| path.as_str()).collect();
        let mut sorted = paths.clone();
        sorted.sort_unstable();
        assert_eq!(sorted, ["", "a", "a/deep", "a/deep/er", "b", "empty"]);
        assert_eq!(*paths.last().unwrap(), "", "the start is the last line");
        let at = |path: &str| paths.iter().position(|p| *p == path).unwrap();
        assert!(at("a/deep/er") < at("a/deep") && at("a/deep") < at("a"));
        let size = |path: &str| rows[at(path)].1;
        assert!(size("a/deep/er") >= 300_000, "what the file is allocated");
        assert!(size("a/deep") >= size("a/deep/er") + 70_000);
        assert!(size("a") >= size("a/deep") + 5000);
        assert!(size("") >= size("a") + size("b") + size("empty"));

        // `-a`: the four files too, each before the directory holding it.
        let (every, _) = walk(root, all_files());
        assert_eq!(every.len(), rows.len() + 4);
        let position = |path: &str| every.iter().position(|(p, _)| p == path).unwrap();
        assert!(position("a/deep/er/three.bin") < position("a/deep/er"));
        assert_eq!(
            every.last().unwrap(),
            rows.last().unwrap(),
            "the same total"
        );

        // A depth bounds the lines, counted from the start; no total moves.
        let depth = |max_depth| UsageOptions {
            all_files: false,
            max_depth: Some(max_depth),
        };
        let (one, _) = walk(root, depth(1));
        let mut one_paths: Vec<&str> = one.iter().map(|(p, _)| p.as_str()).collect();
        one_paths.sort_unstable();
        assert_eq!(one_paths, ["", "a", "b", "empty"]);
        assert_eq!(one.last().unwrap().1, size(""));
        assert_eq!(walk(root, depth(0)).0, [(String::new(), size(""))]);
        let (below, _) = walk(&root.join("a"), depth(1));
        let mut below: Vec<&str> = below.iter().map(|(p, _)| p.as_str()).collect();
        below.sort_unstable();
        assert_eq!(below, ["", "deep"], "depth counts from the start given");
        // With `-a`, a depth bounds the files' lines as well.
        let (shallow, _) = walk(
            root,
            UsageOptions {
                all_files: true,
                max_depth: Some(1),
            },
        );
        assert!(shallow.iter().all(|(path, _)| !path.contains('/')));
    }

    /// AC-2: a file given as the start is its own line, with or without
    /// `-a` and whatever the depth.
    #[test]
    fn du_of_a_file_is_that_files_line() {
        let tmp = tempdir().unwrap();
        let path = tmp.path().join("one.bin");
        file(&path, 5000);
        for options in [
            UsageOptions::default(),
            all_files(),
            UsageOptions {
                all_files: false,
                max_depth: Some(0),
            },
        ] {
            let (rows, failures) = walk(&path, options);
            assert!(failures.is_empty());
            assert_eq!(rows.len(), 1);
            assert_eq!(rows[0].0, "");
            assert!(rows[0].1 >= 5000);
        }
    }

    /// AC-2: a file with several hard links is counted ONCE — the first
    /// time it is met — and has no second line under `-a`. A tree of
    /// hard-linked snapshots is exactly where counting each name would
    /// mislead. (Unix here; the Windows identity is read another way and
    /// has not run.)
    #[cfg(unix)]
    #[test]
    fn du_counts_a_hard_linked_file_once() {
        let tmp = tempdir().unwrap();
        let root = tmp.path();
        file(&root.join("first/big.bin"), 300_000);
        fs::create_dir_all(root.join("second")).unwrap();
        fs::hard_link(root.join("first/big.bin"), root.join("second/again.bin")).unwrap();
        fs::hard_link(
            root.join("first/big.bin"),
            root.join("second/and-again.bin"),
        )
        .unwrap();
        file(&root.join("second/own.bin"), 5000);

        let (rows, _) = walk(root, all_files());
        let big = rows
            .iter()
            .filter(|(path, _)| path.ends_with("big.bin") || path.contains("again"))
            .collect::<Vec<_>>();
        assert_eq!(big.len(), 1, "one line for the three names: {rows:?}");
        let allocated = big[0].1;
        let total = rows.last().unwrap().1;
        assert!(
            total < 2 * allocated,
            "counted once: the whole tree is {total}, the file {allocated}"
        );
        // The two folders together hold the file once, whichever met it.
        let size = |path: &str| rows.iter().find(|(p, _)| p == path).unwrap().1;
        assert!(size("first") + size("second") < 2 * allocated + 20_000);
    }

    /// AC-12: a symbolic link is an entry of its own and is never
    /// followed — as the start, and inside the tree: nothing under its
    /// target is visited or counted through it.
    #[cfg(unix)]
    #[test]
    fn du_never_follows_a_symbolic_link() {
        let tmp = tempdir().unwrap();
        let root = tmp.path();
        file(&root.join("outside/huge.bin"), 400_000);
        file(&root.join("tree/small.bin"), 100);
        std::os::unix::fs::symlink(root.join("outside"), root.join("tree/link")).unwrap();

        let (rows, failures) = walk(&root.join("tree"), all_files());
        assert!(failures.is_empty());
        let paths: Vec<&str> = rows.iter().map(|(p, _)| p.as_str()).collect();
        assert!(paths.contains(&"link"), "the link is an entry: {paths:?}");
        assert!(
            !paths.iter().any(|path| path.starts_with("link/")),
            "nothing under its target: {paths:?}"
        );
        assert!(
            rows.last().unwrap().1 < 400_000,
            "the target is not counted"
        );

        let (as_start, _) = walk(&root.join("tree/link"), all_files());
        assert_eq!(as_start.len(), 1, "a link given as the start is one line");
        assert!(as_start[0].1 < 400_000);
    }

    /// AC-12: a directory that cannot be read is reported in `du`'s words,
    /// keeps its own line, and the walk goes on to everything else. A
    /// start that is not there ends the walk with the reason and no line.
    #[cfg(unix)]
    #[test]
    fn du_reports_what_it_cannot_read_and_goes_on() {
        use std::os::unix::fs::PermissionsExt as _;
        let tmp = tempdir().unwrap();
        let root = tmp.path();
        file(&root.join("open/one.bin"), 5000);
        file(&root.join("locked/inner/secret.bin"), 5000);
        let locked = root.join("locked");
        fs::set_permissions(&locked, fs::Permissions::from_mode(0o000)).unwrap();
        let readable_anyway = fs::read_dir(&locked).is_ok();
        let (rows, failures) = walk(root, UsageOptions::default());
        fs::set_permissions(&locked, fs::Permissions::from_mode(0o755)).unwrap();
        if readable_anyway {
            eprintln!("skipping: running privileged, mode 000 does not block reads");
            return;
        }
        assert_eq!(
            failures,
            [("locked".to_string(), "cannot read directory".to_string())]
        );
        let paths: Vec<&str> = rows.iter().map(|(p, _)| p.as_str()).collect();
        assert!(paths.contains(&"locked"), "its own line stays: {paths:?}");
        assert!(paths.contains(&"open") && paths.last().unwrap().is_empty());
        assert!(!paths.iter().any(|path| path.starts_with("locked/")));

        let missing = disk_usage(
            &root.join("no-such-entry"),
            UsageOptions::default(),
            &mut |_| panic!("nothing is emitted for a start that is not there"),
        );
        match missing {
            Err(UsageEnd::Start(error)) => {
                assert_eq!(error.kind(), io::ErrorKind::NotFound);
                assert_eq!(os_reason(&error), "No such file or directory");
            }
            other => panic!("the start cannot be accessed, got {other:?}"),
        }
    }

    /// A caller that stops listening ends the walk at once.
    #[test]
    fn du_stops_when_its_caller_does() {
        let tmp = tempdir().unwrap();
        for name in ["a", "b", "c"] {
            file(&tmp.path().join(name).join("f.bin"), 10);
        }
        let mut seen = 0;
        let ended = disk_usage(tmp.path(), all_files(), &mut |_| {
            seen += 1;
            Err(Stopped)
        });
        assert!(matches!(ended, Err(UsageEnd::Stopped(Stopped))));
        assert_eq!(seen, 1);
    }

    /// The numbers as `du` and `df` print them. Each `human_size` row is
    /// what GNU coreutils 9.12 printed for that many allocated bytes
    /// (`du -h`, `df -h`; run 2026-11-20): powers of 1024, rounded UP, one
    /// decimal below ten of a unit.
    #[test]
    fn sizes_are_printed_as_du_and_df_print_them() {
        for (bytes, blocks, human) in [
            (0u64, 0u64, "0"),
            (4096, 4, "4.0K"),
            (8192, 8, "8.0K"),
            (12_288, 12, "12K"),
            (73_728, 72, "72K"),
            (393_216, 384, "384K"),
            (1_048_576, 1024, "1.0M"),
            (1_572_864, 1536, "1.5M"),
            (10_485_760, 10_240, "10M"),
            (1_073_741_824, 1_048_576, "1.0G"),
            // `df -h` of a 16 GiB filesystem: 16118172 and 259788 1K blocks.
            (16_118_172 * 1024, 16_118_172, "16G"),
            (259_788 * 1024, 259_788, "254M"),
            (15_858_384 * 1024, 15_858_384, "16G"),
        ] {
            assert_eq!(blocks_1k(bytes), blocks, "{bytes} bytes in 1K blocks");
            assert_eq!(human_size(bytes), human, "{bytes} bytes");
        }
        // Rounded up, never down: one byte over is the next step.
        assert_eq!(blocks_1k(1), 1);
        assert_eq!(blocks_1k(1025), 2);
        assert_eq!(human_size(1024), "1.0K");
        assert_eq!(human_size(1025), "1.1K");
        assert_eq!(human_size(10 * 1024 + 1), "11K");
        assert_eq!(human_size(1024 * 1024 - 1), "1.0M");
        // One byte over 1023M is 1024M, which is written 1.0G.
        assert_eq!(human_size(1023 * 1024 * 1024 + 1), "1.0G");
        assert_eq!(human_size(1023 * 1024 * 1024 * 1024 + 1), "1.0T");
        assert_eq!(human_size(500), "500", "below 1K, the bytes themselves");
        assert_eq!(human_size(u64::MAX), "16E");
    }

    /// `df`'s numbers: used is the size less every free block, the
    /// percentage is of what an ordinary user can use, rounded up — 2% for
    /// the filesystem `df` itself reported so (run 2026-11-20).
    #[test]
    fn use_percent_is_dfs() {
        let space = |used: u64, available: u64| FilesystemSpace {
            total_bytes: used + available,
            used_bytes: used,
            available_bytes: available,
        };
        assert_eq!(use_percent(&space(259_788, 15_858_384)), Some(2));
        assert_eq!(use_percent(&space(0, 100)), Some(0));
        assert_eq!(use_percent(&space(1, 999)), Some(1), "rounded up");
        assert_eq!(use_percent(&space(100, 0)), Some(100));
        assert_eq!(use_percent(&space(0, 0)), None);
        // A reserve: used + available is less than the size.
        let reserved = FilesystemSpace {
            total_bytes: 1000,
            used_bytes: 500,
            available_bytes: 400,
        };
        assert_eq!(use_percent(&reserved), Some(56));
    }

    /// AC-1: the operating system is asked about the path itself. A path
    /// that is there has a filesystem of some size, a file answers for the
    /// filesystem it is on, and a path that is not there is the system's
    /// own "not found".
    #[test]
    fn filesystem_space_asks_about_the_path_given() {
        let tmp = tempdir().unwrap();
        let of_dir = filesystem_space(tmp.path()).expect("the temp folder's filesystem");
        assert!(of_dir.total_bytes > 0);
        assert!(of_dir.used_bytes <= of_dir.total_bytes);
        assert!(of_dir.available_bytes <= of_dir.total_bytes);
        let inside = tmp.path().join("a-file");
        fs::write(&inside, b"x").unwrap();
        assert_eq!(
            filesystem_space(&inside)
                .expect("a file's filesystem")
                .total_bytes,
            of_dir.total_bytes
        );
        let missing = filesystem_space(&tmp.path().join("no-such-entry")).unwrap_err();
        assert_eq!(missing.kind(), io::ErrorKind::NotFound);
    }

    /// The system's words, without the standard library's error number.
    #[test]
    fn os_reason_is_the_systems_words_alone() {
        let denied = io::Error::from(io::ErrorKind::PermissionDenied);
        assert!(!os_reason(&denied).contains("os error"));
        let custom = io::Error::other("a reason of our own");
        assert_eq!(os_reason(&custom), "a reason of our own");
        #[cfg(unix)]
        {
            assert_eq!(
                os_reason(&io::Error::from_raw_os_error(libc::EACCES)),
                "Permission denied"
            );
            assert_eq!(
                os_reason(&io::Error::from_raw_os_error(libc::ENOENT)),
                "No such file or directory"
            );
        }
    }
}
