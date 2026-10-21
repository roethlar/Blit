use eyre::Result;
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::time::UNIX_EPOCH;

use crate::checksum::{self, ChecksumType};
use crate::copy::file_needs_copy_with_checksum_type;
use crate::enumeration::{EntryKind, EnumeratedEntry, FileEnumerator};
use crate::fs_enum::{CopyJob, FileEntry, FileFilter};

/// Planner responsible for deciding whether files should be transferred and for
/// computing mirror deletion plans.
#[derive(Clone, Copy, Debug)]
pub struct MirrorPlanner {
    checksum: Option<ChecksumType>,
}

/// Captures the remote file state used when comparing local enumerated entries
/// against remote metadata provided by the daemon.
#[derive(Clone, Debug)]
pub struct RemoteEntryState {
    pub size: u64,
    pub mtime: i64,
    pub hash: Option<Vec<u8>>,
}

#[derive(Debug, Default, Clone)]
pub struct MirrorDeletionPlan {
    pub files: Vec<PathBuf>,
    pub dirs: Vec<PathBuf>,
}

/// A manifest entry whose name is not valid UTF-8 (contract v7): the
/// lossy text every map is keyed by, and the exact source bytes.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RawNamedEntry {
    pub text: String,
    pub raw: Vec<u8>,
}

// Case-sensitivity is a property of the platform's default filesystem, and
// this key exists to model it so a mirror never deletes a file the source
// kept under a different case. Windows (NTFS) and macOS (APFS) are
// case-insensitive by default → fold (posix-normalize + Unicode lowercase);
// Linux (ext4/xfs) is case-sensitive → exact. This matters most for the
// unified session, the first mirror path that diffs a WIRE source set (the
// source's on-disk case) against the DEST filesystem: on a case-insensitive
// dest the two cases can diverge, and an exact key would delete the
// just-written file (review otp-6b F1). Case-insensitive Linux mounts and
// case-sensitive macOS volumes are rare misconfigurations a compile-time cfg
// cannot detect. af-3: the fold is Unicode lowercase (`Übung`/`übung`), and
// what a filesystem pairs beyond that — APFS NFC with NFD, `ς` with `σ`,
// `ß` with `SS`; NTFS both sigmas — is asked of the filesystem itself in
// `pair_unmatched_by_identity`, never approximated by a text rule.
#[cfg(any(windows, target_os = "macos"))]
#[derive(Hash, Eq, PartialEq, Clone, Debug)]
struct CasefoldKey(String);

#[cfg(any(windows, target_os = "macos"))]
impl CasefoldKey {
    fn new(path: &Path) -> Self {
        let normalized = crate::path_posix::relative_path_to_posix(path).to_lowercase();
        CasefoldKey(normalized)
    }
}

#[cfg(not(any(windows, target_os = "macos")))]
#[derive(Hash, Eq, PartialEq, Clone, Debug)]
struct CasefoldKey(PathBuf);

#[cfg(not(any(windows, target_os = "macos")))]
impl CasefoldKey {
    fn new(path: &Path) -> Self {
        CasefoldKey(path.to_path_buf())
    }
}

impl MirrorPlanner {
    pub fn new(enable_checksum: bool) -> Self {
        Self {
            checksum: if enable_checksum {
                Some(ChecksumType::Blake3)
            } else {
                None
            },
        }
    }

    pub fn plan_local_deletions_from_entries(
        &self,
        source_entries: &[EnumeratedEntry],
        destination: &Path,
        filter: &FileFilter,
    ) -> Result<MirrorDeletionPlan> {
        let enumerator = FileEnumerator::new(filter.clone_without_cache());
        let dest_entries = enumerator.enumerate_local(destination)?;

        let source_set = source_entries
            .iter()
            .map(|e| CasefoldKey::new(&e.relative_path))
            .collect::<HashSet<_>>();
        let dest_set = dest_entries
            .into_iter()
            .map(|e| (e.relative_path, matches!(e.kind, EntryKind::Directory)))
            .collect::<Vec<_>>();

        Ok(plan_from_sets(
            destination,
            source_set,
            HashSet::new(),
            dest_set,
        ))
    }

    pub fn checksum_enabled(&self) -> bool {
        self.checksum.is_some()
    }

    fn should_copy(&self, src: &Path, dest: &Path) -> bool {
        if !dest.exists() {
            return true;
        }
        file_needs_copy_with_checksum_type(src, dest, self.checksum).unwrap_or(true)
    }

    pub fn should_copy_entry(&self, job: &CopyJob, src_root: &Path, dest_root: &Path) -> bool {
        if job.entry.is_directory {
            return true;
        }
        let rel = job
            .entry
            .path
            .strip_prefix(src_root)
            .unwrap_or(job.entry.path.as_path());
        let dest = dest_root.join(rel);
        self.should_copy(&job.entry.path, &dest)
    }

    pub fn plan_local_deletions(
        &self,
        source: &Path,
        destination: &Path,
        filter: &FileFilter,
    ) -> Result<MirrorDeletionPlan> {
        let enumerator = FileEnumerator::new(filter.clone_without_cache());
        let source_entries = enumerator.enumerate_local(source)?;
        let dest_entries = enumerator.enumerate_local(destination)?;

        let source_set = source_entries
            .iter()
            .map(|e| CasefoldKey::new(&e.relative_path))
            .collect::<HashSet<_>>();
        let dest_set = dest_entries
            .into_iter()
            .map(|e| (e.relative_path, matches!(e.kind, EntryKind::Directory)))
            .collect::<Vec<_>>();

        Ok(plan_from_sets(
            destination,
            source_set,
            HashSet::new(),
            dest_set,
        ))
    }

    pub fn plan_remote_deletions(
        &self,
        source_entries: &[EnumeratedEntry],
        dest_root: &Path,
        remote_entries: &[FileEntry],
    ) -> MirrorDeletionPlan {
        let source_set = source_entries
            .iter()
            .map(|e| CasefoldKey::new(&e.relative_path))
            .collect::<HashSet<_>>();

        let dest_set = remote_entries
            .iter()
            .map(|entry| {
                let rel = entry
                    .path
                    .strip_prefix(dest_root)
                    .unwrap_or(entry.path.as_path())
                    .to_path_buf();
                (rel, entry.is_directory)
            })
            .collect::<Vec<_>>();

        plan_from_sets(dest_root, source_set, HashSet::new(), dest_set)
    }
    pub fn plan_expected_deletions(
        &self,
        dest_root: &Path,
        expected: &std::collections::HashSet<PathBuf>,
    ) -> Result<MirrorDeletionPlan> {
        let enumerator = FileEnumerator::new(FileFilter::default());
        let dest_entries = enumerator.enumerate_local(dest_root)?;

        let mut source_keys = HashSet::new();
        for entry in &dest_entries {
            if expected.contains(&entry.absolute_path) {
                source_keys.insert(CasefoldKey::new(&entry.relative_path));
            }
        }

        let dest_set = dest_entries
            .into_iter()
            .map(|e| (e.relative_path, matches!(e.kind, EntryKind::Directory)))
            .collect::<Vec<_>>();

        Ok(plan_from_sets(
            dest_root,
            source_keys,
            HashSet::new(),
            dest_set,
        ))
    }

    /// otp-6: the unified session's single mirror-delete rule. Given the
    /// COMPLETE set of source-side relative file paths (posix, exactly as
    /// they arrived on the wire) and the enumeration `filter`, enumerate
    /// `dest_root` and return the extraneous entries — dest paths absent
    /// from the source set — dirs deepest-first.
    ///
    /// The session manifest lists files only (directories are implicit, see
    /// `spawn_manifest_task`), so a directory's survival is derived here from
    /// the parent chains of the kept files: a dir is extraneous iff no kept
    /// source file lives under it. This is the same derivation the daemon's
    /// `plan_extraneous_entries` does; centralizing it here is the session's
    /// "one delete rule" that replaces the three per-direction purges at
    /// cutover.
    ///
    /// `filter` scopes the dest enumeration: for `MirrorMode::FilteredSubset`
    /// it is the user's filter (so out-of-scope dest entries are never
    /// candidates); for `MirrorMode::All` it is `FileFilter::default()` (the
    /// whole dest tree is in scope).
    ///
    /// `shielded` (cr-ssc1-1, SOURCE_SIDE_CONTAINMENT A19) is the exact set
    /// of source paths that FAILED to land in this session: each one and
    /// every component-wise descendant of it at the destination is left
    /// alone — a skipped source *file* whose destination is a populated
    /// *directory* must not have that directory's contents deleted when
    /// the replacement never arrived. Unrelated extraneous entries are
    /// still deleted.
    /// `source_files` holds the text names of the REPRESENTABLE source
    /// entries; every raw-named entry comes through `raw_entries` and is
    /// kept under exactly one identity (cr-ssc5-5).
    pub fn plan_session_deletions(
        &self,
        dest_root: &Path,
        source_files: &HashSet<String>,
        raw_entries: &[RawNamedEntry],
        shielded: &HashSet<String>,
        raw_names_storable: bool,
        filter: &FileFilter,
    ) -> Result<MirrorDeletionPlan> {
        self.plan_session_deletions_with_probe(
            dest_root,
            source_files,
            raw_entries,
            shielded,
            raw_names_storable,
            filter,
            &mut |path| {
                crate::transfers::filter::entry_identity(path).map(|(identity, _)| identity)
            },
        )
    }

    /// `plan_session_deletions` with the destination filesystem's identity
    /// lookup (`pair_unmatched_by_identity`'s `probe`) injected, so tests
    /// count the probes and model a filesystem's pairing on any platform.
    fn plan_session_deletions_with_probe<F>(
        &self,
        dest_root: &Path,
        source_files: &HashSet<String>,
        raw_entries: &[RawNamedEntry],
        shielded: &HashSet<String>,
        raw_names_storable: bool,
        filter: &FileFilter,
        probe: &mut F,
    ) -> Result<MirrorDeletionPlan>
    where
        F: FnMut(&Path) -> Option<(u64, u64)>,
    {
        let enumerator = FileEnumerator::new(filter.clone_without_cache());
        let dest_entries = enumerator.enumerate_local(dest_root)?;

        // Kept set = every source file plus each of its ancestor dirs, so a
        // directory holding a kept file is itself kept (never deleted).
        // `kept_paths` records each path as the source spells it, for the
        // identity pairing below.
        let mut source_set: HashSet<CasefoldKey> = HashSet::new();
        let mut kept_paths: Vec<PathBuf> = Vec::new();
        let mut keep = |path: &Path| {
            keep_with_ancestors(&mut source_set, path, |kept| {
                kept_paths.push(kept.to_path_buf())
            });
        };
        for rel in source_files {
            keep(Path::new(rel));
        }
        // Contract v7 (D-F): a source name that is not valid UTF-8 lives at
        // the destination under ONE physical identity (cr-ssc5-5): its
        // exact bytes where this host can store them, its lossy text
        // otherwise. cr-ssc5-3: received bytes are decoded only where
        // storable. The text of a raw-named entry is kept on a storable
        // host only when a representable source entry has that very name
        // (`source_files` holds representable names only) — otherwise a
        // distinct file under the lossy text is extraneous.
        for entry in raw_entries {
            match crate::raw_name::path_from_received_raw(&entry.raw, raw_names_storable) {
                Some(path) => keep(&path),
                None => keep(Path::new(&entry.text)),
            }
        }

        let dest_set = dest_entries
            .into_iter()
            .map(|e| (e.relative_path, matches!(e.kind, EntryKind::Directory)))
            .collect::<Vec<_>>();
        let walked: HashSet<PathBuf> = if filter.is_empty() {
            HashSet::new()
        } else {
            dest_set.iter().map(|(rel, _)| rel.clone()).collect()
        };
        // cr-ssc5-4: a failed entry shields its destination subtree under the
        // identity it actually has here — a raw-named entry's exact bytes
        // where storable (its text names nothing of its own there), its text
        // otherwise.
        // cr-fix2-3: a failed entry is known here by its TEXT only, and
        // several distinct raw names can collapse to one text (the
        // duplicate-collision case). Shield conservatively: EVERY raw entry
        // that collapses to it, decoded where storable — never a
        // first-match guess between them.
        // cr-fix3-1: and the text path itself only where it is a real
        // identity of the failure — always where raw names are unstorable
        // (the text is then the raw entry's identity), and on a storable
        // host only when no raw entry claims the text or a representable
        // source entry carries that very name. Otherwise the text names a
        // distinct, unrelated destination path (see the keep set above),
        // and shielding it would pin stale data forever.
        let mut shield_set: HashSet<CasefoldKey> = HashSet::new();
        let mut shield_paths: Vec<PathBuf> = Vec::new();
        let mut add_shield = |path: &Path| {
            if shield_set.insert(CasefoldKey::new(path)) {
                shield_paths.push(path.to_path_buf());
            }
        };
        for rel in shielded {
            let mut claimed_by_raw = false;
            for entry in raw_entries.iter().filter(|entry| &entry.text == rel) {
                claimed_by_raw = true;
                if let Some(path) =
                    crate::raw_name::path_from_received_raw(&entry.raw, raw_names_storable)
                {
                    add_shield(&path);
                }
            }
            if !raw_names_storable || !claimed_by_raw || source_files.contains(rel) {
                add_shield(Path::new(rel));
            }
        }

        pair_unmatched_by_identity(
            dest_root,
            &mut source_set,
            &mut shield_set,
            &kept_paths,
            &shield_paths,
            &dest_set,
            probe,
        );

        let mut plan = plan_from_sets(dest_root, source_set, shield_set, dest_set);
        if !filter.is_empty() {
            keep_dirs_holding_left_out_entries(&mut plan.dirs, dest_root, &walked, filter);
        }
        Ok(plan)
    }

    pub fn should_copy_remote_entry(
        &self,
        entry: &EnumeratedEntry,
        remote_state: Option<&RemoteEntryState>,
    ) -> bool {
        match &entry.kind {
            EntryKind::Directory => false,
            EntryKind::Symlink { .. } => remote_state.is_none(),
            EntryKind::File { size } => {
                let Some(remote) = remote_state else {
                    return true;
                };

                if remote.size != *size {
                    return true;
                }

                if let Some(ChecksumType::Blake3) = self.checksum {
                    match (
                        remote.hash.as_ref(),
                        checksum::hash_file(&entry.absolute_path, ChecksumType::Blake3).ok(),
                    ) {
                        (Some(remote_hash), Some(local_hash)) => {
                            remote_hash.as_slice() != local_hash.as_slice()
                        }
                        _ => true,
                    }
                } else {
                    match entry.metadata.modified() {
                        Ok(modified) => match modified.duration_since(UNIX_EPOCH) {
                            Ok(duration) => {
                                let local_secs = duration.as_secs() as i64;
                                let diff = local_secs - remote.mtime;
                                !(-2..=2).contains(&diff)
                            }
                            Err(_) => true,
                        },
                        Err(_) => true,
                    }
                }
            }
        }
    }

    pub fn should_fetch_remote_file(
        &self,
        dest_path: &Path,
        remote_state: &RemoteEntryState,
    ) -> bool {
        if !dest_path.exists() {
            return true;
        }

        let metadata = match dest_path.metadata() {
            Ok(md) => md,
            Err(_) => return true,
        };

        if metadata.len() != remote_state.size {
            return true;
        }

        if let Some(ChecksumType::Blake3) = self.checksum {
            match (
                remote_state.hash.as_ref(),
                checksum::hash_file(dest_path, ChecksumType::Blake3).ok(),
            ) {
                (Some(remote_hash), Some(local_hash)) => {
                    remote_hash.as_slice() != local_hash.as_slice()
                }
                _ => true,
            }
        } else {
            let local_secs = metadata
                .modified()
                .ok()
                .and_then(|m| m.duration_since(UNIX_EPOCH).ok())
                .map(|d| d.as_secs() as i64)
                .unwrap_or(0);
            let diff = local_secs - remote_state.mtime;
            !(-2..=2).contains(&diff)
        }
    }
}

fn plan_from_sets(
    dest_root: &Path,
    source_set: HashSet<CasefoldKey>,
    shield_set: HashSet<CasefoldKey>,
    dest_entries: Vec<(PathBuf, bool)>,
) -> MirrorDeletionPlan {
    let mut files = Vec::new();
    let mut dirs = Vec::new();

    for (rel, is_dir) in dest_entries {
        if rel.as_os_str().is_empty() {
            continue;
        }
        if source_set.contains(&CasefoldKey::new(&rel)) {
            continue;
        }
        // cr-ssc1-1: the entry, or any ancestor of it, is a path that
        // failed to land this session — shielded, whatever it holds.
        if !shield_set.is_empty() && is_shielded(&rel, &shield_set) {
            continue;
        }
        let abs = dest_root.join(&rel);
        if is_dir {
            dirs.push(abs);
        } else {
            files.push(abs);
        }
    }

    dirs.sort_by_key(|p| p.components().count());
    dirs.reverse();

    MirrorDeletionPlan { files, dirs }
}

/// cr-g3fix5-1: drop from `dirs` (a deletion plan's directories, under
/// `dest_root`) every directory that holds an entry `filter` — the
/// deletion scope — leaves out, and every directory above one: an
/// excluded entry, in any spelling the destination pairs with it, is never
/// removed, so neither is the directory it lives in, whether or not the
/// source still has that directory. The source walk prunes an excluded
/// entry, so a source directory that held only excluded entries sends no
/// file and its destination counterpart is planned as extraneous (the
/// session manifest lists files only); the walk of the destination prunes
/// the excluded entry too, so the plan never saw it. Before, such a
/// directory was planned and survived only because removing it failed
/// with "not empty" (tolerated), while a dry run counted it as deleted;
/// with the destination's own spelling out of scope as well
/// (cr-g3fix5-1), the plan must not count on that. A child the walk
/// visited is in the plan or in the source, and a child the filter lets
/// through but the walk does not list (a symlink) leaves the directory
/// as it was. One listing per planned directory, never per entry.
fn keep_dirs_holding_left_out_entries(
    dirs: &mut Vec<PathBuf>,
    dest_root: &Path,
    walked: &HashSet<PathBuf>,
    filter: &FileFilter,
) {
    let mut kept: HashSet<PathBuf> = HashSet::new();
    for dir in dirs.iter() {
        if kept.contains(dir) {
            continue;
        }
        let (Ok(rel), Ok(listing)) = (dir.strip_prefix(dest_root), std::fs::read_dir(dir)) else {
            continue;
        };
        let holds = listing.flatten().any(|entry| {
            let child = rel.join(entry.file_name());
            if walked.contains(&child) {
                return false;
            }
            let path = entry.path();
            if entry.file_type().is_ok_and(|kind| kind.is_dir()) {
                return !filter.allows_dir(&child, &path);
            }
            std::fs::symlink_metadata(&path).is_ok_and(|metadata| {
                !filter.allows_entry(
                    Some(&child),
                    &path,
                    metadata.len(),
                    metadata.modified().ok(),
                )
            })
        });
        if holds {
            let mut current = Some(dir.as_path());
            while let Some(path) = current {
                if path == dest_root || !path.starts_with(dest_root) {
                    break;
                }
                if !kept.insert(path.to_path_buf()) {
                    break;
                }
                current = path.parent();
            }
        }
    }
    dirs.retain(|dir| !kept.contains(dir));
}

/// Insert `path` and each of its non-empty ancestors into `set`, calling
/// `on_new` for every key that was not there before.
fn keep_with_ancestors(set: &mut HashSet<CasefoldKey>, path: &Path, mut on_new: impl FnMut(&Path)) {
    if set.insert(CasefoldKey::new(path)) {
        on_new(path);
    }
    let mut current = path.parent();
    while let Some(parent) = current {
        if parent.as_os_str().is_empty() {
            break;
        }
        if set.insert(CasefoldKey::new(parent)) {
            on_new(parent);
        }
        current = parent.parent();
    }
}

/// af-3: pair, through the destination filesystem itself, the destination
/// entries no key matched with the kept and shielded paths no destination
/// key matched. `CasefoldKey` folds what Unicode lowercase folds; a
/// filesystem may pair more (APFS: NFC with NFD, `ς` with `σ`, `ß` with
/// `SS`; NTFS: both sigmas), and a mirror trusting the key alone would
/// delete the destination's spelling of a file the source kept, or of a
/// file that failed to land. `probe` is the identity of the entry a path
/// names (`entry_identity`, the link itself and never its target), `None`
/// when it names nothing. No text rule reproduces every filesystem's
/// table, so none is attempted.
///
/// FAST: probes are confined to candidate directories — a directory
/// holding, at one depth, both an unmatched destination entry and an
/// unmatched kept or shielded path, at least one of them spelled with a
/// non-ASCII byte (two ASCII paths pair only by case, which the key
/// already folds where the platform does). A pure-ASCII tree, a synced tree and an empty
/// destination (a first sync, dry run or not) probe nothing; within a
/// candidate directory the destination side is probed only once a source
/// path there resolved. Depth by depth, top down: a destination directory
/// paired with a kept directory aliases its children's grouping to the
/// kept directory's, so `bSS/a.txt` meets `bß/a.txt`.
///
/// A destination entry whose identity is a kept path's is kept with its
/// ancestors; one whose identity is a shielded path's is shielded with
/// its descendants. A hard link to a kept file counts as kept — the safe
/// direction, as `listed_name`'s over-exclusion is.
fn pair_unmatched_by_identity<F>(
    dest_root: &Path,
    source_set: &mut HashSet<CasefoldKey>,
    shield_set: &mut HashSet<CasefoldKey>,
    kept_paths: &[PathBuf],
    shield_paths: &[PathBuf],
    dest_entries: &[(PathBuf, bool)],
    probe: &mut F,
) where
    F: FnMut(&Path) -> Option<(u64, u64)>,
{
    let depth = |path: &Path| path.components().count();
    let dest_keys: HashSet<CasefoldKey> = dest_entries
        .iter()
        .map(|(rel, _)| CasefoldKey::new(rel))
        .collect();
    // Unmatched kept (`false`) and shielded (`true`) paths, bucketed by depth.
    let mut src_by_depth: Vec<Vec<(&Path, bool)>> = Vec::new();
    let unmatched_src = kept_paths
        .iter()
        .map(|path| (path.as_path(), false))
        .chain(shield_paths.iter().map(|path| (path.as_path(), true)))
        .filter(|(path, _)| {
            !path.as_os_str().is_empty() && !dest_keys.contains(&CasefoldKey::new(path))
        });
    for (path, is_shield) in unmatched_src {
        let level = depth(path);
        if src_by_depth.len() <= level {
            src_by_depth.resize_with(level + 1, Vec::new);
        }
        src_by_depth[level].push((path, is_shield));
    }
    if src_by_depth.iter().all(Vec::is_empty) {
        return;
    }
    // Unmatched destination entries (deletion candidates), bucketed by depth.
    let mut dest_by_depth: Vec<Vec<(&Path, bool)>> = vec![Vec::new(); src_by_depth.len()];
    for (rel, is_dir) in dest_entries {
        let level = depth(rel);
        if rel.as_os_str().is_empty()
            || level >= dest_by_depth.len()
            || source_set.contains(&CasefoldKey::new(rel))
            || (!shield_set.is_empty() && is_shielded(rel, shield_set))
        {
            continue;
        }
        dest_by_depth[level].push((rel.as_path(), *is_dir));
    }

    let parent_key = |path: &Path| CasefoldKey::new(path.parent().unwrap_or(Path::new("")));
    // The whole relative path, not the name alone: a child of a paired
    // directory (`bSS/a.txt` for `bß/a.txt`) has an ASCII name of its own.
    let non_ascii = |path: &Path| !path.as_os_str().as_encoded_bytes().is_ascii();
    #[derive(Default)]
    struct Group<'a> {
        src: Vec<(&'a Path, bool)>,
        dest: Vec<(&'a Path, bool)>,
        non_ascii: bool,
    }
    // A paired destination directory's key → its kept directory's key, so
    // the children of the one are grouped with the children of the other.
    let mut alias: HashMap<CasefoldKey, CasefoldKey> = HashMap::new();
    for (src_level, dest_level) in src_by_depth.iter().zip(&dest_by_depth) {
        if src_level.is_empty() || dest_level.is_empty() {
            continue;
        }
        let mut groups: HashMap<CasefoldKey, Group> = HashMap::new();
        for &(path, is_shield) in src_level {
            let group = groups.entry(parent_key(path)).or_default();
            group.non_ascii |= non_ascii(path);
            group.src.push((path, is_shield));
        }
        for &(rel, is_dir) in dest_level {
            let key = parent_key(rel);
            let key = alias.get(&key).cloned().unwrap_or(key);
            let Some(group) = groups.get_mut(&key) else {
                continue;
            };
            group.non_ascii |= non_ascii(rel);
            group.dest.push((rel, is_dir));
        }
        for group in groups.into_values() {
            if group.dest.is_empty() || !group.non_ascii {
                continue;
            }
            let mut kept: HashMap<(u64, u64), &Path> = HashMap::new();
            let mut shielded: HashSet<(u64, u64)> = HashSet::new();
            for (path, is_shield) in group.src {
                let Some(identity) = probe(&dest_root.join(path)) else {
                    continue;
                };
                if is_shield {
                    shielded.insert(identity);
                } else {
                    kept.entry(identity).or_insert(path);
                }
            }
            if kept.is_empty() && shielded.is_empty() {
                continue;
            }
            for (rel, is_dir) in group.dest {
                let Some(identity) = probe(&dest_root.join(rel)) else {
                    continue;
                };
                if shielded.contains(&identity) {
                    shield_set.insert(CasefoldKey::new(rel));
                }
                if let Some(paired) = kept.get(&identity) {
                    keep_with_ancestors(source_set, rel, |_| {});
                    if is_dir {
                        alias.insert(CasefoldKey::new(rel), CasefoldKey::new(paired));
                    }
                }
            }
        }
    }
}

/// Whether `rel` or any of its ancestors is in `shield_set`.
fn is_shielded(rel: &Path, shield_set: &HashSet<CasefoldKey>) -> bool {
    let mut current = Some(rel);
    while let Some(path) = current {
        // cr-fix1-1: test the path BEFORE stopping at the empty ancestor —
        // the empty relative path is a real identity (a single-file source
        // root), and a failed single-file mirror shields the whole
        // destination directory beneath it.
        if shield_set.contains(&CasefoldKey::new(path)) {
            return true;
        }
        if path.as_os_str().is_empty() {
            break;
        }
        current = path.parent();
    }
    false
}

#[cfg(test)]
mod shield_tests {
    use super::*;
    use std::collections::HashSet;

    /// cr-ssc5-3: foreign raw bytes (a Linux name reaching a Windows or
    /// macOS destination) are never decoded during mirror planning on a
    /// host that cannot store them — the entry counts by its text only,
    /// the plan is computed, and an unrelated extraneous entry is still
    /// planned.
    /// cr-ssc5-4: a failed raw-named entry shields the destination subtree
    /// under its RAW path (where storable): a populated directory there is
    /// untouched, while an unrelated extraneous entry is still planned.
    #[test]
    fn a_failed_raw_named_entry_shields_its_raw_destination_subtree() {
        let tmp = tempfile::tempdir().unwrap();
        let dst = tmp.path().join("dst");
        std::fs::create_dir_all(dst.join("realdir")).unwrap();
        std::fs::write(dst.join("realdir/child.bin"), b"keep me").unwrap();
        std::fs::write(dst.join("extraneous.bin"), b"x").unwrap();
        let raw = vec![RawNamedEntry {
            text: "caf\u{fffd}dir".to_string(),
            raw: b"realdir".to_vec(),
        }];
        let shielded: HashSet<String> = HashSet::from(["caf\u{fffd}dir".to_string()]);
        let filter = crate::fs_enum::FileFilter::default();
        let plan = MirrorPlanner::new(false)
            .plan_session_deletions(&dst, &HashSet::new(), &raw, &shielded, true, &filter)
            .unwrap();
        assert_eq!(
            plan.files,
            vec![dst.join("extraneous.bin")],
            "the failed raw entry's subtree is untouched, the unrelated entry is planned: {plan:?}"
        );
        assert!(plan.dirs.is_empty(), "{plan:?}");
    }

    /// cr-fix2-3: two raw names collapse to one lossy text; the second is
    /// the rejected duplicate and its destination path is a populated
    /// directory. The shield for that text must cover EVERY raw entry
    /// sharing it — the directory is untouched, the first file is kept,
    /// and an unrelated extraneous entry is still planned.
    #[test]
    fn a_lossy_collision_shields_every_raw_entry_sharing_the_text() {
        let tmp = tempfile::tempdir().unwrap();
        let dst = tmp.path().join("dst");
        std::fs::create_dir_all(dst.join("second")).unwrap();
        std::fs::write(dst.join("second/child.bin"), b"keep me").unwrap();
        std::fs::write(dst.join("first.bin"), b"landed").unwrap();
        std::fs::write(dst.join("extraneous.bin"), b"x").unwrap();
        let text = "caf\u{fffd}.txt".to_string();
        let raw = vec![
            RawNamedEntry {
                text: text.clone(),
                raw: b"first.bin".to_vec(),
            },
            RawNamedEntry {
                text: text.clone(),
                raw: b"second".to_vec(),
            },
        ];
        let shielded: HashSet<String> = HashSet::from([text]);
        let filter = crate::fs_enum::FileFilter::default();
        let plan = MirrorPlanner::new(false)
            .plan_session_deletions(&dst, &HashSet::new(), &raw, &shielded, true, &filter)
            .unwrap();
        assert_eq!(
            plan.files,
            vec![dst.join("extraneous.bin")],
            "both raw identities are shielded; only the unrelated entry is planned: {plan:?}"
        );
        assert!(plan.dirs.is_empty(), "{plan:?}");
    }

    /// cr-fix3-1: a failed raw-named entry's lossy TEXT is a destination
    /// identity of its own only where raw names cannot be stored, or where
    /// a representable source entry carries that very name. On a
    /// byte-capable destination with neither, a valid-UTF-8 directory
    /// equal to the lossy rendering is unrelated and extraneous: it and
    /// its contents are planned, while the failed entry's decoded raw
    /// path stays shielded. (ASCII "raw" bytes keep this portable.)
    #[test]
    fn a_failed_raw_entry_shields_its_lossy_text_only_as_a_real_identity() {
        let tmp = tempfile::tempdir().unwrap();
        let dst = tmp.path().join("dst");
        let text = "caf\u{fffd}dir".to_string();
        std::fs::create_dir_all(dst.join(&text)).unwrap();
        std::fs::write(dst.join(&text).join("child.bin"), b"unrelated").unwrap();
        std::fs::create_dir_all(dst.join("realdir")).unwrap();
        std::fs::write(dst.join("realdir/child.bin"), b"keep me").unwrap();
        std::fs::write(dst.join("extraneous.bin"), b"x").unwrap();
        let raw = vec![RawNamedEntry {
            text: text.clone(),
            raw: b"realdir".to_vec(),
        }];
        let shielded: HashSet<String> = HashSet::from([text.clone()]);
        let filter = crate::fs_enum::FileFilter::default();
        let sorted = |mut paths: Vec<PathBuf>| {
            paths.sort();
            paths
        };

        // Storable, no representable entry of that name: the lossy-text
        // subtree is unrelated and planned; the raw subtree is shielded.
        let plan = MirrorPlanner::new(false)
            .plan_session_deletions(&dst, &HashSet::new(), &raw, &shielded, true, &filter)
            .unwrap();
        assert_eq!(
            sorted(plan.files.clone()),
            sorted(vec![
                dst.join("extraneous.bin"),
                dst.join(&text).join("child.bin")
            ]),
            "storable: the unrelated lossy-text subtree is planned: {plan:?}"
        );
        assert_eq!(
            plan.dirs,
            vec![dst.join(&text)],
            "storable: the unrelated lossy-text dir is planned: {plan:?}"
        );

        // Storable, and a representable source entry carries the text: the
        // failure may be that entry's, so its subtree is shielded too.
        let source_files: HashSet<String> = HashSet::from([text.clone()]);
        let plan = MirrorPlanner::new(false)
            .plan_session_deletions(&dst, &source_files, &raw, &shielded, true, &filter)
            .unwrap();
        assert_eq!(
            plan.files,
            vec![dst.join("extraneous.bin")],
            "storable + representable: the text subtree is kept: {plan:?}"
        );
        assert!(plan.dirs.is_empty(), "{plan:?}");

        // Unstorable: the text IS the entry's identity and stays shielded;
        // the raw bytes name nothing here, so `realdir` is extraneous.
        let plan = MirrorPlanner::new(false)
            .plan_session_deletions(&dst, &HashSet::new(), &raw, &shielded, false, &filter)
            .unwrap();
        assert_eq!(
            sorted(plan.files.clone()),
            sorted(vec![
                dst.join("extraneous.bin"),
                dst.join("realdir/child.bin")
            ]),
            "unstorable: the text subtree is kept: {plan:?}"
        );
        assert_eq!(plan.dirs, vec![dst.join("realdir")], "{plan:?}");

        // Storable, and NO raw entry claims the failed text: it is a
        // failure of its own (cr-ssc1-1) and keeps its shield, listed in
        // the manifest or not.
        let plan = MirrorPlanner::new(false)
            .plan_session_deletions(&dst, &HashSet::new(), &[], &shielded, true, &filter)
            .unwrap();
        assert_eq!(
            sorted(plan.files.clone()),
            sorted(vec![
                dst.join("extraneous.bin"),
                dst.join("realdir/child.bin")
            ]),
            "unclaimed: the failed text path keeps its shield: {plan:?}"
        );
        assert_eq!(plan.dirs, vec![dst.join("realdir")], "{plan:?}");
    }

    /// cr-ssc5-5: a raw-named source entry has ONE destination identity.
    /// Where raw names are storable, a distinct file under its lossy text
    /// is extraneous and is planned; where they are not, the text is the
    /// identity and is kept. (ASCII "raw" bytes keep this portable.)
    #[test]
    fn a_raw_named_entry_keeps_exactly_one_destination_identity() {
        let tmp = tempfile::tempdir().unwrap();
        let dst = tmp.path().join("dst");
        std::fs::create_dir_all(&dst).unwrap();
        std::fs::write(dst.join("real.bin"), b"raw-named file").unwrap();
        std::fs::write(dst.join("caf\u{fffd}.txt"), b"distinct lossy-text file").unwrap();
        let raw = vec![RawNamedEntry {
            text: "caf\u{fffd}.txt".to_string(),
            raw: b"real.bin".to_vec(),
        }];
        let filter = crate::fs_enum::FileFilter::default();
        let plan = MirrorPlanner::new(false)
            .plan_session_deletions(&dst, &HashSet::new(), &raw, &HashSet::new(), true, &filter)
            .unwrap();
        assert_eq!(
            plan.files,
            vec![dst.join("caf\u{fffd}.txt")],
            "storable: the lossy-text neighbour is extraneous, the raw file is kept: {plan:?}"
        );
        let plan = MirrorPlanner::new(false)
            .plan_session_deletions(&dst, &HashSet::new(), &raw, &HashSet::new(), false, &filter)
            .unwrap();
        assert_eq!(
            plan.files,
            vec![dst.join("real.bin")],
            "unstorable: the text is the identity and is kept: {plan:?}"
        );
    }

    #[test]
    fn foreign_raw_bytes_are_not_decoded_where_unstorable() {
        let tmp = tempfile::tempdir().unwrap();
        let dst = tmp.path().join("dst");
        std::fs::create_dir_all(&dst).unwrap();
        std::fs::write(dst.join("extraneous.bin"), b"x").unwrap();
        std::fs::write(dst.join("caf\u{fffd}.txt"), b"text counterpart").unwrap();
        let source_files: HashSet<String> = HashSet::from(["caf\u{fffd}.txt".to_string()]);
        let raw = vec![RawNamedEntry {
            text: "caf\u{fffd}.txt".to_string(),
            raw: b"caf\xe9.txt".to_vec(),
        }];
        let filter = crate::fs_enum::FileFilter::default();
        let plan = MirrorPlanner::new(false)
            .plan_session_deletions(&dst, &source_files, &raw, &HashSet::new(), false, &filter)
            .unwrap();
        assert_eq!(
            plan.files,
            vec![dst.join("extraneous.bin")],
            "only the unrelated entry is planned: {plan:?}"
        );
    }

    /// cr-fix1-1: a single-file source whose one file (relative path "")
    /// failed leaves a populated destination directory untouched — every
    /// entry descends from the empty failed path.
    #[test]
    fn an_empty_failed_path_shields_every_destination_descendant() {
        let tmp = tempfile::tempdir().unwrap();
        let dst = tmp.path().join("dst");
        std::fs::create_dir_all(dst.join("b")).unwrap();
        std::fs::write(dst.join("a"), b"a").unwrap();
        std::fs::write(dst.join("b/c"), b"c").unwrap();
        let source_files: HashSet<String> = HashSet::from([String::new()]);
        let filter = crate::fs_enum::FileFilter::default();

        let shielded: HashSet<String> = HashSet::from([String::new()]);
        let plan = MirrorPlanner::new(false)
            .plan_session_deletions(&dst, &source_files, &[], &shielded, true, &filter)
            .unwrap();
        assert!(
            plan.files.is_empty() && plan.dirs.is_empty(),
            "nothing under a failed single-file root is deleted: {plan:?}"
        );

        // Control: with nothing shielded the same tree IS extraneous.
        let plan = MirrorPlanner::new(false)
            .plan_session_deletions(&dst, &source_files, &[], &HashSet::new(), true, &filter)
            .unwrap();
        assert_eq!(plan.files.len(), 2, "control: extraneous files are planned");
    }
}

#[cfg(test)]
mod identity_pairing_tests {
    use super::*;
    use std::collections::HashSet;
    use std::ffi::OsString;
    use std::hash::{Hash, Hasher};

    /// af-3: a destination filesystem whose pairing rule is a table — each
    /// `(spelling, on_disk)` row says the filesystem resolves `spelling`
    /// to the entry stored as `on_disk` — so the planner's identity pass
    /// is exercised on every platform, with the probes counted. A path
    /// exists when its on-disk spelling does; its identity is that
    /// spelling's hash.
    struct PairingFs {
        root: PathBuf,
        pairs: Vec<(&'static str, &'static str)>,
    }

    impl PairingFs {
        fn on_disk(&self, rel: &Path) -> PathBuf {
            rel.components()
                .map(|component| {
                    let typed = component.as_os_str().to_string_lossy();
                    self.pairs
                        .iter()
                        .find(|(spelling, _)| *spelling == typed)
                        .map(|(_, on_disk)| OsString::from(on_disk))
                        .unwrap_or_else(|| component.as_os_str().to_os_string())
                })
                .collect()
        }

        fn probe(&self, path: &Path) -> Option<(u64, u64)> {
            let rel = path.strip_prefix(&self.root).ok()?;
            let on_disk = self.on_disk(rel);
            std::fs::symlink_metadata(self.root.join(&on_disk)).ok()?;
            let mut hasher = std::collections::hash_map::DefaultHasher::new();
            on_disk.hash(&mut hasher);
            Some((1, hasher.finish()))
        }
    }

    fn plan_with(
        fs: &PairingFs,
        source_files: &[&str],
        shielded: &[&str],
    ) -> (MirrorDeletionPlan, usize) {
        let source_files: HashSet<String> = source_files.iter().map(|s| s.to_string()).collect();
        let shielded: HashSet<String> = shielded.iter().map(|s| s.to_string()).collect();
        let filter = crate::fs_enum::FileFilter::default();
        let mut probes = 0usize;
        let plan = MirrorPlanner::new(false)
            .plan_session_deletions_with_probe(
                &fs.root,
                &source_files,
                &[],
                &shielded,
                true,
                &filter,
                &mut |path| {
                    probes += 1;
                    fs.probe(path)
                },
            )
            .unwrap();
        (plan, probes)
    }

    fn sorted(mut paths: Vec<PathBuf>) -> Vec<PathBuf> {
        paths.sort();
        paths
    }

    /// FAST: a pure-ASCII tree with stale destination entries and new
    /// source entries in the same directories, an empty destination (a
    /// first sync) and a synced non-ASCII tree all plan without one
    /// identity probe.
    #[test]
    fn ascii_first_sync_and_synced_trees_probe_no_identities() {
        let tmp = tempfile::tempdir().unwrap();
        let dst = tmp.path().join("dst");
        std::fs::create_dir_all(dst.join("dir")).unwrap();
        std::fs::write(dst.join("keep.txt"), b"k").unwrap();
        std::fs::write(dst.join("stale.txt"), b"s").unwrap();
        std::fs::write(dst.join("dir/stale.txt"), b"s").unwrap();
        let fs = PairingFs {
            root: dst.clone(),
            pairs: vec![],
        };
        let (plan, probes) = plan_with(&fs, &["keep.txt", "dir/new.txt", "other/new.txt"], &[]);
        assert_eq!(probes, 0, "a pure-ASCII tree is never probed");
        assert_eq!(
            sorted(plan.files.clone()),
            sorted(vec![dst.join("stale.txt"), dst.join("dir/stale.txt")]),
        );
        assert!(plan.dirs.is_empty(), "{plan:?}");

        let empty = tmp.path().join("empty");
        std::fs::create_dir_all(&empty).unwrap();
        let fs = PairingFs {
            root: empty,
            pairs: vec![],
        };
        let (plan, probes) = plan_with(&fs, &["\u{fc}bung/notes.txt", "caf\u{e9}.txt"], &[]);
        assert_eq!(
            probes, 0,
            "a first sync has no destination candidate to pair"
        );
        assert!(plan.files.is_empty() && plan.dirs.is_empty(), "{plan:?}");

        let synced = tmp.path().join("synced");
        std::fs::create_dir_all(synced.join("\u{fc}bung")).unwrap();
        std::fs::write(synced.join("\u{fc}bung/notes.txt"), b"n").unwrap();
        let fs = PairingFs {
            root: synced,
            pairs: vec![],
        };
        let (plan, probes) = plan_with(&fs, &["\u{fc}bung/notes.txt"], &[]);
        assert_eq!(
            probes, 0,
            "a synced tree has nothing unmatched on either side"
        );
        assert!(plan.files.is_empty() && plan.dirs.is_empty(), "{plan:?}");
    }

    /// AC-3: the destination stores the kept source directory `bß` as the
    /// ASCII spelling `bSS` (APFS folds `ß` to `SS`). The directory and
    /// the kept file inside it are kept — the child meets its source
    /// spelling through the paired directory — while a stale child of the
    /// same directory and an unrelated entry are still planned.
    #[test]
    fn an_ascii_destination_spelling_of_a_kept_non_ascii_directory_is_kept_with_its_children() {
        let tmp = tempfile::tempdir().unwrap();
        let dst = tmp.path().join("dst");
        std::fs::create_dir_all(dst.join("bSS")).unwrap();
        std::fs::write(dst.join("bSS/a.txt"), b"kept").unwrap();
        std::fs::write(dst.join("bSS/stale.txt"), b"stale").unwrap();
        std::fs::write(dst.join("other.txt"), b"x").unwrap();
        let fs = PairingFs {
            root: dst.clone(),
            pairs: vec![("b\u{df}", "bSS")],
        };
        let (plan, probes) = plan_with(&fs, &["b\u{df}/a.txt"], &[]);
        assert!(probes > 0, "the candidate directory is probed");
        assert_eq!(
            sorted(plan.files.clone()),
            sorted(vec![dst.join("bSS/stale.txt"), dst.join("other.txt")]),
        );
        assert!(
            plan.dirs.is_empty(),
            "the paired directory is kept: {plan:?}"
        );
    }

    /// AC-3: a failed source path `ς` is stored at the destination as
    /// `σ` (APFS and NTFS pair the sigmas). The directory under that
    /// spelling is shielded with everything in it; an unrelated entry is
    /// still planned. Control: with nothing failed and nothing kept, the
    /// same directory is extraneous.
    #[test]
    fn a_destination_spelling_the_filesystem_pairs_with_a_failed_path_is_shielded() {
        let tmp = tempfile::tempdir().unwrap();
        let dst = tmp.path().join("dst");
        std::fs::create_dir_all(dst.join("\u{3c3}")).unwrap();
        std::fs::write(dst.join("\u{3c3}/child.bin"), b"keep me").unwrap();
        std::fs::write(dst.join("extraneous.bin"), b"x").unwrap();
        let fs = PairingFs {
            root: dst.clone(),
            pairs: vec![("\u{3c2}", "\u{3c3}")],
        };
        let (plan, probes) = plan_with(&fs, &[], &["\u{3c2}"]);
        assert!(probes > 0);
        assert_eq!(plan.files, vec![dst.join("extraneous.bin")], "{plan:?}");
        assert!(plan.dirs.is_empty(), "{plan:?}");

        let (plan, _) = plan_with(&fs, &[], &[]);
        assert_eq!(
            sorted(plan.files.clone()),
            sorted(vec![
                dst.join("extraneous.bin"),
                dst.join("\u{3c3}/child.bin")
            ]),
            "control: unshielded, the directory is extraneous: {plan:?}"
        );
        assert_eq!(plan.dirs, vec![dst.join("\u{3c3}")]);
    }

    /// AC-3: the source's NFC `café.txt` is stored at the destination in
    /// NFD (APFS pairs the two); the file is kept and a stale neighbour
    /// is planned.
    #[test]
    fn nfc_and_nfd_spellings_pair_through_the_filesystem() {
        let tmp = tempfile::tempdir().unwrap();
        let dst = tmp.path().join("dst");
        std::fs::create_dir_all(&dst).unwrap();
        std::fs::write(dst.join("cafe\u{301}.txt"), b"kept").unwrap();
        std::fs::write(dst.join("stale.txt"), b"stale").unwrap();
        let fs = PairingFs {
            root: dst.clone(),
            pairs: vec![("caf\u{e9}.txt", "cafe\u{301}.txt")],
        };
        let (plan, _) = plan_with(&fs, &["caf\u{e9}.txt"], &[]);
        assert_eq!(plan.files, vec![dst.join("stale.txt")], "{plan:?}");
        assert!(plan.dirs.is_empty(), "{plan:?}");
    }

    /// AC-3 on the real filesystem: NTFS and APFS pair `Übung` with
    /// `übung`, which the key folds in memory. Runs on the
    /// case-insensitive-default platforms only. (NTFS pairs simple case
    /// mappings only — measured 2026-10-21 on this repo's Windows host, it
    /// resolves neither `ς` to `σ`, NFD to NFC, nor `ẞ` to `ß` — so no
    /// plain name reaches the identity pass there; see the APFS test.)
    #[cfg(any(windows, target_os = "macos"))]
    #[test]
    fn unicode_case_spellings_are_kept_on_case_insensitive_platforms() {
        let tmp = tempfile::tempdir().unwrap();
        let dst = tmp.path().join("dst");
        std::fs::create_dir_all(dst.join("\u{dc}bung")).unwrap();
        std::fs::write(dst.join("\u{dc}bung/notes.txt"), b"kept").unwrap();
        std::fs::write(dst.join("stale.txt"), b"stale").unwrap();
        let source_files: HashSet<String> = HashSet::from(["\u{fc}bung/notes.txt".to_string()]);
        let filter = crate::fs_enum::FileFilter::default();
        let plan = MirrorPlanner::new(false)
            .plan_session_deletions(&dst, &source_files, &[], &HashSet::new(), true, &filter)
            .unwrap();
        assert_eq!(plan.files, vec![dst.join("stale.txt")], "{plan:?}");
        assert!(plan.dirs.is_empty(), "{plan:?}");
    }

    /// AC-3 on APFS, through the filesystem's own pairing rather than the
    /// key: the source's final sigma `ς` is kept under the destination's
    /// `σ`, and its NFC `café` under the destination's NFD spelling.
    #[cfg(target_os = "macos")]
    #[test]
    fn sigma_and_nfd_spellings_are_kept_on_apfs() {
        let tmp = tempfile::tempdir().unwrap();
        let dst = tmp.path().join("dst");
        std::fs::create_dir_all(&dst).unwrap();
        std::fs::write(dst.join("\u{3c3}.txt"), b"kept").unwrap();
        std::fs::write(dst.join("cafe\u{301}.txt"), b"kept").unwrap();
        std::fs::write(dst.join("stale.txt"), b"stale").unwrap();
        let source_files: HashSet<String> =
            HashSet::from(["\u{3c2}.txt".to_string(), "caf\u{e9}.txt".to_string()]);
        let filter = crate::fs_enum::FileFilter::default();
        let plan = MirrorPlanner::new(false)
            .plan_session_deletions(&dst, &source_files, &[], &HashSet::new(), true, &filter)
            .unwrap();
        assert_eq!(plan.files, vec![dst.join("stale.txt")], "{plan:?}");
        assert!(plan.dirs.is_empty(), "{plan:?}");
    }
}

#[cfg(test)]
mod casefold_tests {
    use super::CasefoldKey;
    use std::path::Path;

    // af-3: the fold is Unicode lowercase, not ASCII — `Übung` and `übung`
    // are one entry on NTFS and APFS.
    #[cfg(any(windows, target_os = "macos"))]
    #[test]
    fn casefold_folds_unicode_case_on_case_insensitive_platforms() {
        assert_eq!(
            CasefoldKey::new(Path::new("\u{dc}bung/\u{3a3}.TXT")),
            CasefoldKey::new(Path::new("\u{fc}bung/\u{3c3}.txt"))
        );
    }

    // On NTFS/APFS the mirror keep-set must treat `Foo.txt` and `foo.txt` as
    // the same file, or the session mirror deletes a just-written file whose
    // wire case differs from the dest-FS case (review otp-6b F1). Runs on
    // Windows/macOS CI (the case-insensitive-default platforms); a Linux dev
    // box cannot exercise it — see the exact-match test below.
    #[cfg(any(windows, target_os = "macos"))]
    #[test]
    fn casefold_folds_case_on_case_insensitive_platforms() {
        assert_eq!(
            CasefoldKey::new(Path::new("Dir/Foo.TXT")),
            CasefoldKey::new(Path::new("dir/foo.txt"))
        );
    }

    // On ext4/xfs the wire case and dest-FS case always agree, so the key
    // stays exact: `Foo.txt` and `foo.txt` are distinct files and a mirror
    // deletes the one absent from the source. Pins that the macOS fold above
    // did not leak into the case-sensitive default.
    #[cfg(not(any(windows, target_os = "macos")))]
    #[test]
    fn casefold_is_exact_on_case_sensitive_platforms() {
        assert_ne!(
            CasefoldKey::new(Path::new("Foo.txt")),
            CasefoldKey::new(Path::new("foo.txt"))
        );
    }
}
