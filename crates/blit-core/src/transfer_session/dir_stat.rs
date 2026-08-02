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
//! is a TRUSTED absent only where nothing but the listed names can
//! resolve. Two channels break that and both force the authoritative
//! stat instead: case-folded near-matches (a destination holding
//! `FOO.txt` never lets a manifest `foo.txt` be judged absent from the
//! sweep alone), and — cr-ls5-1 — Windows 8.3 short-name aliases, where
//! path lookup resolves `PROFES~1.XML` to a listed `Professional.xml`
//! the sweep knows by its long name only. A custom alias need not even
//! contain `~`, so on Windows EVERY miss in a listed directory is the
//! stat's to judge. The trusted absent survives where no alias can
//! exist — absent directories (everywhere) and listed directories on
//! non-Windows destinations — which still covers the fresh-copy
//! workload: its destination directories do not exist yet.
//!
//! cr-ls5-2 closes the other half of that boundary: the SESSION ITSELF
//! writes. Chunk N's payloads land while chunks N+1… are still being
//! diffed, so a directory that was absent or empty at sweep time can hold
//! this session's own freshly written files — and their 8.3 aliases and
//! case variants — by the time a later entry is judged against the same
//! snapshot. A [`DirStatCache::taint`] therefore rides every transfer
//! VERDICT: from then on, every ABSENCE answer for that directory (a
//! swept miss, an [`DirSnapshot::AbsentDir`] child, the empty-directory
//! carve-out) degrades to [`DirStatLookup::Fallback`], where the stat
//! resolves aliases and case exactly as the filesystem does. Listed-entry
//! HITS survive: a hit describes that file's own pre-write state, which is
//! the state the diff is entitled to compare against. Taint precedes the
//! writes it guards by construction — a chunk's verdicts are all recorded
//! before any of its payloads is queued, and the next chunk's diff starts
//! only after the previous chunk returned.

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
    /// cr-ls5-2: directories this session has decided to write into, and
    /// their ancestors. Deliberately NOT evicted with `dirs`: a re-swept
    /// snapshot is newer but still a snapshot, so the degradation has to
    /// outlive the listing it degrades. One `PathBuf` per destination
    /// directory the session touches is the same order as the tree it is
    /// writing.
    tainted: HashSet<PathBuf>,
}

/// Session-lifetime cache of destination directory sweeps. Shared by every
/// checker thread; the first thread to touch a directory sweeps it inside
/// the cell's `get_or_init` while later arrivals block on that cell only,
/// never on the whole cache.
///
/// Staleness window: a snapshot describes the directory as of its sweep.
/// Against EXTERNAL mutation that is the same check-then-act model the
/// per-file stat already had. Against the session's OWN apply it is not
/// (cr-ls5-2): applies run concurrently with the diff of later chunks, so
/// a snapshot outlives writes made into the directory it describes. Every
/// transfer verdict therefore [`DirStatCache::taint`]s its destination's
/// ancestors, and a tainted directory answers no absence from the
/// snapshot again.
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
        // cr-ls5-2: read the taint AFTER the snapshot, never before. A taint
        // recorded while this very sweep was running still precedes the write
        // it announces, so a pre-read would miss exactly the write whose
        // absence answer is about to go stale.
        let tainted = {
            let inner = self.inner.lock().expect("dir-stat cache poisoned");
            inner.tainted.contains(dir)
        };
        let looked = match snapshot {
            DirSnapshot::Swept { exact, folded } => match exact.get(name) {
                Some(SweptEntry::NeedsStat) => DirStatLookup::Fallback,
                Some(entry) => DirStatLookup::Entry(*entry),
                None if folded.contains(&fold_name(name)) => DirStatLookup::Fallback,
                // cr-ls5-2: this session has written into the directory
                // since the sweep, so a miss no longer proves absence —
                // the new file may be reachable under this very name as an
                // alias or a case variant. Only the stat can say.
                None if tainted => DirStatLookup::Fallback,
                // An alias can only resolve to a LISTED entry, so an
                // untainted empty directory keeps the trusted absent on
                // every platform — the copy-into-fresh-empty-directory case.
                None if exact.is_empty() => DirStatLookup::Absent,
                // cr-ls5-1: on Windows a name can miss the sweep yet still
                // resolve through an 8.3 short-name alias of a listed
                // entry; judging it absent would overwrite the aliased
                // file, including under --ignore-existing. Only the stat
                // resolves aliases.
                #[cfg(windows)]
                None => DirStatLookup::Fallback,
                #[cfg(not(windows))]
                None => DirStatLookup::Absent,
            },
            // cr-ls5-2: the write that made the directory exist is this
            // session's own, so "the directory was absent" no longer
            // settles anything about its children.
            DirSnapshot::AbsentDir if tainted => DirStatLookup::Fallback,
            DirSnapshot::AbsentDir => DirStatLookup::Absent,
            DirSnapshot::Unsweepable => DirStatLookup::Fallback,
        };
        match looked {
            DirStatLookup::Fallback => self.fallbacks.fetch_add(1, Ordering::Relaxed),
            _ => self.hits.fetch_add(1, Ordering::Relaxed),
        };
        looked
    }

    /// cr-ls5-2: record that this session has decided to WRITE `path`, so
    /// no snapshot of the directories holding it may answer absence again.
    ///
    /// Called at VERDICT time, not write time, and that ordering is the
    /// whole guarantee: a chunk's verdicts are all in before any of its
    /// payloads is queued, and the next chunk's diff begins only after the
    /// previous chunk returned, so every write is announced here before it
    /// happens. Ancestors are tainted too — landing a file creates the
    /// directories above it, which changes THEIR listings (and can mint
    /// short-name aliases for the directory names).
    ///
    /// Nothing here re-sweeps or invalidates: a listed entry keeps
    /// answering, because a hit describes that file's own pre-write state,
    /// which is exactly the state its diff compares against.
    pub fn taint(&self, path: &Path) {
        let mut inner = self.inner.lock().expect("dir-stat cache poisoned");
        // Ancestors are only ever inserted as a whole chain, so the first
        // one already present means every remaining one is too.
        for ancestor in path.ancestors().skip(1) {
            if !inner.tainted.insert(ancestor.to_path_buf()) {
                break;
            }
        }
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
    fn a_subdirectory_is_nonfile_and_a_miss_beside_it_follows_the_platform_rule() {
        let dir = tempfile::tempdir().expect("tempdir");
        std::fs::create_dir(dir.path().join("nested")).expect("mkdir");

        let cache = DirStatCache::default();
        assert_eq!(
            cache.lookup(dir.path(), OsStr::new("nested"), &probe()),
            DirStatLookup::Entry(SweptEntry::NonFile)
        );
        // cr-ls5-1: beside LISTED entries, a Windows miss could be an 8.3
        // alias of one of them — the stat judges it. Elsewhere no alias
        // channel exists and the sweep's absent is trusted.
        #[cfg(windows)]
        let expected_miss = DirStatLookup::Fallback;
        #[cfg(not(windows))]
        let expected_miss = DirStatLookup::Absent;
        assert_eq!(
            cache.lookup(dir.path(), OsStr::new("never-created"), &probe()),
            expected_miss
        );
        // Both answered from ONE sweep.
        assert_eq!(cache.sweeps(), 1);
    }

    #[cfg(windows)]
    #[test]
    fn a_windows_miss_beside_listed_entries_is_the_stats_to_judge() {
        // The reviewer's live repro behind cr-ls5-1: `PROFES~1.XML`
        // resolves to a listed `Professional.xml` through its 8.3 alias,
        // so judging the sweep-miss absent re-copies over the aliased
        // file — including under --ignore-existing. Auto 8.3 generation
        // is per-volume configuration this test cannot assume, so it pins
        // the DECISION (every miss beside listed entries falls back), not
        // the OS's alias table.
        let dir = tempfile::tempdir().expect("tempdir");
        std::fs::write(dir.path().join("Professional.xml"), b"keep me").expect("write");

        let cache = DirStatCache::default();
        assert_eq!(
            cache.lookup(dir.path(), OsStr::new("PROFES~1.XML"), &probe()),
            DirStatLookup::Fallback
        );
        assert_eq!(cache.fallbacks(), 1);
    }

    #[test]
    fn an_empty_directory_keeps_the_trusted_absent_on_every_platform() {
        let dir = tempfile::tempdir().expect("tempdir");

        let cache = DirStatCache::default();
        // Nothing is listed, so nothing can be aliased: the
        // copy-into-fresh-empty-directory case stays fallback-free.
        assert_eq!(
            cache.lookup(dir.path(), OsStr::new("anything.txt"), &probe()),
            DirStatLookup::Absent
        );
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

    #[test]
    fn a_tainted_directory_stops_trusting_every_absence_but_keeps_its_hits() {
        // cr-ls5-2: the session's own apply is what invalidates the
        // snapshot, so from the transfer verdict onward the listing may
        // only answer for what it LISTED — a hit is that file's own
        // pre-write state, which is the state its diff compares against.
        let dir = tempfile::tempdir().expect("tempdir");
        std::fs::write(dir.path().join("listed.txt"), b"12345").expect("write");

        let cache = DirStatCache::default();
        let before = cache.lookup(dir.path(), OsStr::new("listed.txt"), &probe());
        assert!(
            matches!(
                before,
                DirStatLookup::Entry(SweptEntry::File { size: 5, .. })
            ),
            "expected the swept file, got {before:?}"
        );

        cache.taint(&dir.path().join("just-written.bin"));

        let after = cache.lookup(dir.path(), OsStr::new("listed.txt"), &probe());
        assert_eq!(after, before, "a listed entry still answers from the sweep");
        // A miss, however, may now be this session's own freshly written
        // file reached under an alias or a case variant. Platform note: in
        // a LISTED directory cr-ls5-1 already forces this same Fallback on
        // Windows, so this line's taint-specific red is a non-Windows one.
        // The taint's platform-neutral reds are the two tests below, whose
        // directories list nothing for cr-ls5-1's rule to bite on.
        assert_eq!(
            cache.lookup(dir.path(), OsStr::new("never-listed.bin"), &probe()),
            DirStatLookup::Fallback
        );
        // Tainting judges nothing itself: still ONE sweep.
        assert_eq!(cache.sweeps(), 1);
    }

    #[test]
    fn a_tainted_empty_directory_loses_the_carve_out() {
        // The empty-directory carve-out survives only while the directory
        // is still empty. Once this session has planted a file in it,
        // "nothing is listed, so nothing can be aliased" is false.
        let dir = tempfile::tempdir().expect("tempdir");

        let cache = DirStatCache::default();
        assert_eq!(
            cache.lookup(dir.path(), OsStr::new("first.txt"), &probe()),
            DirStatLookup::Absent
        );
        cache.taint(&dir.path().join("first.txt"));
        assert_eq!(
            cache.lookup(dir.path(), OsStr::new("second.txt"), &probe()),
            DirStatLookup::Fallback
        );
    }

    #[test]
    fn a_tainted_absent_directory_falls_back_for_every_child_and_ancestor() {
        // Landing a file also creates the directories above it, so the
        // taint climbs: neither the absent directory nor the parent that
        // gains it may answer absence from its pre-write snapshot.
        let dir = tempfile::tempdir().expect("tempdir");
        let missing = dir.path().join("not-yet-created");

        let cache = DirStatCache::default();
        assert_eq!(
            cache.lookup(&missing, OsStr::new("a.txt"), &probe()),
            DirStatLookup::Absent
        );
        assert_eq!(
            cache.lookup(dir.path(), OsStr::new("not-yet-created"), &probe()),
            DirStatLookup::Absent
        );

        cache.taint(&missing.join("a.txt"));

        assert_eq!(
            cache.lookup(&missing, OsStr::new("b.txt"), &probe()),
            DirStatLookup::Fallback
        );
        assert_eq!(
            cache.lookup(dir.path(), OsStr::new("not-yet-created"), &probe()),
            DirStatLookup::Fallback
        );
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
