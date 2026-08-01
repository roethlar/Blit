//! ls-5: the destination directory-sweep stat cache.
//!
//! `docs/plan/LOCAL_SMALL_FILE_PATH.md` ls-5. The diff used to resolve every
//! manifest entry with its own `std::fs::metadata` — one destination round
//! trip per file, which ls-1 measured at ~49 s of a 46,041-file converged
//! SMB mirror. One `read_dir` sweep of the containing directory returns
//! every sibling's size, mtime and (on Windows) attribute DWORD in the
//! enumeration itself, so the per-file round trip buys nothing the sweep
//! did not already carry. Measured premise: a full sweep of the same field
//! tree costs 21.19 s against the ~49 s of stats it replaces.
//!
//! The design rule is SEMANTICS-FREE BY CONSTRUCTION: the cache answers
//! only when the sweep's answer is exact, and everything else —
//! symlinks/reparse points (the stat follows links, the sweep does not),
//! case-folded near-miss names, unreadable directories, mid-listing
//! errors — drops to [`DirStatLookup::Fallback`], where the caller's
//! per-file stat remains authoritative. Verdicts therefore cannot shift;
//! only round-trip counts can.
//!
//! The one trust boundary worth naming: a name MISSING from a clean sweep
//! with no case-folded near-match is a TRUSTED absent. Treating it as a
//! fallback instead would re-run the full per-file stat storm on exactly
//! the workload this cache exists to serve twice over — a fresh copy has
//! every file absent. The folded-name check is what keeps that trust
//! honest on case-insensitive filesystems: a destination holding
//! `FOO.txt` never lets a manifest `foo.txt` be judged absent from the
//! sweep alone, because the folded candidate forces the authoritative
//! stat, which resolves case the way the filesystem itself does.

use std::collections::{HashMap, HashSet, VecDeque};
use std::ffi::{OsStr, OsString};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex, OnceLock};
use std::time::Instant;

use super::phase_probe::{LocalPhase, LocalPhaseProbe};

/// Directories retained at once. Manifest order follows the source walk,
/// so directory locality is strong and a small window suffices; an
/// evicted directory is merely re-swept. Sized for deep trees whose
/// ancestors interleave with their descendants.
const MAX_CACHED_DIRS: usize = 128;

/// One judged entry from a directory sweep.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SweptEntry {
    /// A regular file: size, mtime seconds, and on Windows the attribute
    /// DWORD the enumeration already carried (`None` elsewhere).
    File {
        size: u64,
        mtime: i64,
        attributes: Option<u32>,
    },
    /// Present but not a regular file (a directory, say). A file write
    /// must replace it, so it diffs exactly as absent does — and unlike a
    /// link this needs no second look: the enumeration's own type bit is
    /// the same evidence the per-file stat would return.
    NonFile,
    /// A symlink or reparse point, or an entry whose metadata the sweep
    /// could not read. The per-file stat FOLLOWS links where the sweep
    /// does not, so only the stat can judge these.
    NeedsStat,
}

/// What the cache can say about one destination name.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DirStatLookup {
    /// The sweep holds an exact-name answer.
    Entry(SweptEntry),
    /// Not in the sweep and no case-folded near-match: trusted absent.
    Absent,
    /// The sweep cannot answer exactly — folded-name near-miss,
    /// unsweepable directory, or a [`SweptEntry::NeedsStat`] entry
    /// (surfaced through [`DirStatLookup::Entry`]). The caller's per-file
    /// stat is authoritative.
    Fallback,
}

/// One directory's sweep result.
enum DirSnapshot {
    /// `read_dir` succeeded and every entry was listed.
    Swept {
        exact: HashMap<OsString, SweptEntry>,
        /// Case-folded names, for the near-miss check. Two swept names
        /// folding together is harmless — exact matches still resolve
        /// each — so a set suffices.
        folded: HashSet<String>,
    },
    /// The directory itself does not exist (or its parent chain is not a
    /// directory): every child is absent, exactly as the per-file stat
    /// would report (`NotFound` / `NotADirectory` both diff as "target
    /// does not have it").
    AbsentDir,
    /// `read_dir` failed some other way, or errored mid-listing. A
    /// partial listing is never trusted: everything falls back.
    Unsweepable,
}

#[derive(Default)]
struct CacheInner {
    dirs: HashMap<PathBuf, Arc<OnceLock<DirSnapshot>>>,
    /// Insertion order, for FIFO eviction.
    order: VecDeque<PathBuf>,
}

/// Session-lifetime cache of destination directory sweeps. Shared by every
/// checker thread; the first thread to touch a directory sweeps it inside
/// the cell's `get_or_init` while later arrivals block on that cell only,
/// never on the whole cache.
///
/// Staleness window: a snapshot describes the directory as of its sweep.
/// That is the same check-then-act model the per-file stat already had —
/// the session's own apply only writes files AFTER their verdict, so the
/// widened window admits no self-inflicted misjudgement, only the
/// pre-existing external-mutation race.
#[derive(Default)]
pub struct DirStatCache {
    inner: Mutex<CacheInner>,
    sweeps: AtomicU64,
    hits: AtomicU64,
    fallbacks: AtomicU64,
}

impl std::fmt::Debug for DirStatCache {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("DirStatCache")
            .field("sweeps", &self.sweeps.load(Ordering::Relaxed))
            .field("hits", &self.hits.load(Ordering::Relaxed))
            .field("fallbacks", &self.fallbacks.load(Ordering::Relaxed))
            .finish_non_exhaustive()
    }
}

impl DirStatCache {
    /// Resolve one destination name against its directory's sweep,
    /// sweeping on first touch. `probe` times the sweep itself as
    /// [`LocalPhase::CompareSweep`].
    pub fn lookup(&self, dir: &Path, name: &OsStr, probe: &LocalPhaseProbe) -> DirStatLookup {
        let cell = {
            let mut inner = self.inner.lock().expect("dir-stat cache poisoned");
            match inner.dirs.get(dir) {
                Some(cell) => Arc::clone(cell),
                None => {
                    if inner.dirs.len() >= MAX_CACHED_DIRS {
                        if let Some(evicted) = inner.order.pop_front() {
                            inner.dirs.remove(&evicted);
                        }
                    }
                    let cell = Arc::new(OnceLock::new());
                    inner.dirs.insert(dir.to_path_buf(), Arc::clone(&cell));
                    inner.order.push_back(dir.to_path_buf());
                    cell
                }
            }
        };
        let snapshot = cell.get_or_init(|| {
            self.sweeps.fetch_add(1, Ordering::Relaxed);
            let span = probe.is_enabled().then(Instant::now);
            let swept = sweep_directory(dir);
            if let Some(started) = span {
                probe.record(LocalPhase::CompareSweep, started.elapsed());
            }
            swept
        });
        let looked = match snapshot {
            DirSnapshot::Swept { exact, folded } => match exact.get(name) {
                Some(SweptEntry::NeedsStat) => DirStatLookup::Fallback,
                Some(entry) => DirStatLookup::Entry(*entry),
                None if folded.contains(&fold_name(name)) => DirStatLookup::Fallback,
                None => DirStatLookup::Absent,
            },
            DirSnapshot::AbsentDir => DirStatLookup::Absent,
            DirSnapshot::Unsweepable => DirStatLookup::Fallback,
        };
        match looked {
            DirStatLookup::Fallback => self.fallbacks.fetch_add(1, Ordering::Relaxed),
            _ => self.hits.fetch_add(1, Ordering::Relaxed),
        };
        looked
    }

    /// Sweeps performed. Test observability for the injection seam — the
    /// converged-mirror guard asserts the session ANSWERED from sweeps,
    /// which no tree comparison can see (a per-file-stat session produces
    /// the identical tree, just slower).
    #[cfg(test)]
    pub fn sweeps(&self) -> u64 {
        self.sweeps.load(Ordering::Relaxed)
    }

    /// Lookups the cache answered (entries and trusted absents).
    #[cfg(test)]
    pub fn hits(&self) -> u64 {
        self.hits.load(Ordering::Relaxed)
    }

    /// Lookups that dropped to the authoritative per-file stat.
    #[cfg(test)]
    pub fn fallbacks(&self) -> u64 {
        self.fallbacks.load(Ordering::Relaxed)
    }
}

/// The mtime-seconds convention shared by the sweep and the per-file stat
/// fallback: seconds since the epoch, negated for pre-epoch times, zero
/// when the platform cannot say. Extracted verbatim from the diff's stat
/// path so the two resolutions cannot drift.
pub fn mtime_seconds(meta: &std::fs::Metadata) -> i64 {
    match meta.modified() {
        Ok(t) => match t.duration_since(std::time::UNIX_EPOCH) {
            Ok(d) => d.as_secs() as i64,
            Err(e) => -(e.duration().as_secs() as i64),
        },
        Err(_) => 0,
    }
}

/// Case-fold for the near-miss check. `to_lowercase` is an approximation
/// of NTFS's own upcase table; where they disagree the outcome is a
/// FALLBACK (never a trusted absent), so exotic case pairs cost a stat,
/// not a verdict. Non-UTF-8 names fold lossily, which can only widen the
/// fallback set the same way.
fn fold_name(name: &OsStr) -> String {
    name.to_string_lossy().to_lowercase()
}

fn sweep_directory(dir: &Path) -> DirSnapshot {
    let entries = match std::fs::read_dir(dir) {
        Ok(entries) => entries,
        Err(err) => {
            // The same two kinds the per-file stat folds into "target
            // does not have it" (audit-17 made `NotADirectory` a
            // first-class sibling of `NotFound` on this path).
            return match err.kind() {
                std::io::ErrorKind::NotFound | std::io::ErrorKind::NotADirectory => {
                    DirSnapshot::AbsentDir
                }
                _ => DirSnapshot::Unsweepable,
            };
        }
    };
    let mut exact: HashMap<OsString, SweptEntry> = HashMap::new();
    let mut folded: HashSet<String> = HashSet::new();
    for entry in entries {
        let Ok(entry) = entry else {
            // A partial listing cannot distinguish "absent" from
            // "unlisted"; trust none of it.
            return DirSnapshot::Unsweepable;
        };
        let name = entry.file_name();
        folded.insert(fold_name(&name));
        exact.insert(name, judge_entry(&entry));
    }
    DirSnapshot::Swept { exact, folded }
}

fn judge_entry(entry: &std::fs::DirEntry) -> SweptEntry {
    let Ok(file_type) = entry.file_type() else {
        return SweptEntry::NeedsStat;
    };
    if file_type.is_symlink() {
        return SweptEntry::NeedsStat;
    }
    let Ok(meta) = entry.metadata() else {
        return SweptEntry::NeedsStat;
    };
    #[cfg(windows)]
    {
        use std::os::windows::fs::MetadataExt;
        const FILE_ATTRIBUTE_REPARSE_POINT: u32 = 0x400;
        let attributes = meta.file_attributes();
        // Any reparse point — junction, placeholder, dedup — may resolve
        // differently under the following stat; only the stat judges it.
        if attributes & FILE_ATTRIBUTE_REPARSE_POINT != 0 {
            return SweptEntry::NeedsStat;
        }
        if meta.is_file() {
            return SweptEntry::File {
                size: meta.len(),
                mtime: mtime_seconds(&meta),
                attributes: Some(attributes),
            };
        }
        SweptEntry::NonFile
    }
    #[cfg(not(windows))]
    {
        if meta.is_file() {
            return SweptEntry::File {
                size: meta.len(),
                mtime: mtime_seconds(&meta),
                attributes: None,
            };
        }
        SweptEntry::NonFile
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn probe() -> LocalPhaseProbe {
        LocalPhaseProbe::disabled()
    }

    #[test]
    fn a_swept_file_reports_size_and_mtime_exactly_as_the_stat_would() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("payload.bin");
        std::fs::write(&path, b"12345").expect("write");
        let meta = std::fs::metadata(&path).expect("stat");

        let cache = DirStatCache::default();
        let looked = cache.lookup(dir.path(), OsStr::new("payload.bin"), &probe());
        match looked {
            DirStatLookup::Entry(SweptEntry::File { size, mtime, .. }) => {
                assert_eq!(size, 5);
                assert_eq!(mtime, mtime_seconds(&meta));
            }
            other => panic!("expected a swept file, got {other:?}"),
        }
        assert_eq!(cache.sweeps(), 1);
    }

    #[test]
    fn a_subdirectory_is_nonfile_and_a_missing_name_is_trusted_absent() {
        let dir = tempfile::tempdir().expect("tempdir");
        std::fs::create_dir(dir.path().join("nested")).expect("mkdir");

        let cache = DirStatCache::default();
        assert_eq!(
            cache.lookup(dir.path(), OsStr::new("nested"), &probe()),
            DirStatLookup::Entry(SweptEntry::NonFile)
        );
        assert_eq!(
            cache.lookup(dir.path(), OsStr::new("never-created"), &probe()),
            DirStatLookup::Absent
        );
        // Both answered from ONE sweep.
        assert_eq!(cache.sweeps(), 1);
        assert_eq!(cache.fallbacks(), 0);
    }

    #[test]
    fn a_case_folded_near_miss_falls_back_instead_of_trusting_absent() {
        let dir = tempfile::tempdir().expect("tempdir");
        std::fs::write(dir.path().join("README.txt"), b"x").expect("write");

        let cache = DirStatCache::default();
        // Exact name differs, folded name collides: the per-file stat must
        // decide, because only the filesystem knows its own case rules.
        assert_eq!(
            cache.lookup(dir.path(), OsStr::new("readme.TXT"), &probe()),
            DirStatLookup::Fallback
        );
        assert_eq!(cache.fallbacks(), 1);
    }

    #[test]
    fn an_absent_directory_answers_absent_for_every_child_without_fallback() {
        let dir = tempfile::tempdir().expect("tempdir");
        let missing = dir.path().join("not-yet-created");

        let cache = DirStatCache::default();
        for name in ["a.txt", "b.txt", "c.txt"] {
            assert_eq!(
                cache.lookup(&missing, OsStr::new(name), &probe()),
                DirStatLookup::Absent
            );
        }
        assert_eq!(cache.sweeps(), 1);
        assert_eq!(cache.fallbacks(), 0);
    }

    #[cfg(unix)]
    #[test]
    fn a_symlink_needs_the_following_stat() {
        let dir = tempfile::tempdir().expect("tempdir");
        let target = dir.path().join("real.txt");
        std::fs::write(&target, b"content").expect("write");
        std::os::unix::fs::symlink(&target, dir.path().join("link.txt")).expect("symlink");

        let cache = DirStatCache::default();
        assert_eq!(
            cache.lookup(dir.path(), OsStr::new("link.txt"), &probe()),
            DirStatLookup::Fallback
        );
    }

    #[test]
    fn eviction_is_bounded_and_an_evicted_directory_is_merely_reswept() {
        let root = tempfile::tempdir().expect("tempdir");
        let mut dirs = Vec::new();
        for i in 0..(MAX_CACHED_DIRS + 1) {
            let d = root.path().join(format!("d{i}"));
            std::fs::create_dir(&d).expect("mkdir");
            std::fs::write(d.join("f.txt"), b"x").expect("write");
            dirs.push(d);
        }
        let cache = DirStatCache::default();
        for d in &dirs {
            cache.lookup(d, OsStr::new("f.txt"), &probe());
        }
        // The first directory was evicted to admit the last; touching it
        // again re-sweeps rather than misanswering.
        cache.lookup(&dirs[0], OsStr::new("f.txt"), &probe());
        assert_eq!(cache.sweeps(), (MAX_CACHED_DIRS + 2) as u64);
        assert_eq!(
            cache.lookup(&dirs[0], OsStr::new("f.txt"), &probe()),
            match std::fs::metadata(dirs[0].join("f.txt")) {
                Ok(meta) => DirStatLookup::Entry(SweptEntry::File {
                    size: meta.len(),
                    mtime: mtime_seconds(&meta),
                    attributes: swept_attributes(&meta),
                }),
                Err(_) => panic!("fixture file vanished"),
            }
        );
    }

    #[cfg(windows)]
    fn swept_attributes(meta: &std::fs::Metadata) -> Option<u32> {
        use std::os::windows::fs::MetadataExt;
        Some(meta.file_attributes())
    }

    #[cfg(not(windows))]
    fn swept_attributes(_meta: &std::fs::Metadata) -> Option<u32> {
        None
    }
}
