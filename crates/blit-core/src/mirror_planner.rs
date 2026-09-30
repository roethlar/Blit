use eyre::Result;
use std::collections::HashSet;
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
// case-insensitive by default → fold (posix-normalize + ASCII-lowercase);
// Linux (ext4/xfs) is case-sensitive → exact. This matters most for the
// unified session, the first mirror path that diffs a WIRE source set (the
// source's on-disk case) against the DEST filesystem: on a case-insensitive
// dest the two cases can diverge, and an exact key would delete the
// just-written file (review otp-6b F1). Case-insensitive Linux mounts and
// case-sensitive macOS volumes are rare misconfigurations a compile-time cfg
// cannot detect; ASCII-only folding leaves Unicode case variants approximate
// (a known gap, same as the pre-existing Windows behavior).
#[cfg(any(windows, target_os = "macos"))]
#[derive(Hash, Eq, PartialEq, Clone, Debug)]
struct CasefoldKey(String);

#[cfg(any(windows, target_os = "macos"))]
impl CasefoldKey {
    fn new(path: &Path) -> Self {
        let normalized = crate::path_posix::relative_path_to_posix(path).to_ascii_lowercase();
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
    pub fn plan_session_deletions(
        &self,
        dest_root: &Path,
        source_files: &HashSet<String>,
        raw_entries: &[RawNamedEntry],
        shielded: &HashSet<String>,
        raw_names_storable: bool,
        filter: &FileFilter,
    ) -> Result<MirrorDeletionPlan> {
        let enumerator = FileEnumerator::new(filter.clone_without_cache());
        let dest_entries = enumerator.enumerate_local(dest_root)?;

        // Kept set = every source file plus each of its ancestor dirs, so a
        // directory holding a kept file is itself kept (never deleted).
        let mut source_set: HashSet<CasefoldKey> = HashSet::new();
        let mut keep = |path: &Path| {
            source_set.insert(CasefoldKey::new(path));
            let mut current = path.parent();
            while let Some(parent) = current {
                if parent.as_os_str().is_empty() {
                    break;
                }
                source_set.insert(CasefoldKey::new(parent));
                current = parent.parent();
            }
        };
        for rel in source_files {
            keep(Path::new(rel));
        }
        // Contract v7 (D-F): a source name that is not valid UTF-8 lives at
        // the destination under its exact bytes, which on a byte-keyed
        // filesystem never equal the lossy text — keep it by its bytes too.
        // cr-ssc5-3: received bytes are decoded only where this host can
        // store them; elsewhere the entry's text is its only identity.
        for entry in raw_entries {
            if let Some(path) =
                crate::raw_name::path_from_received_raw(&entry.raw, raw_names_storable)
            {
                keep(&path);
            }
        }

        let dest_set = dest_entries
            .into_iter()
            .map(|e| (e.relative_path, matches!(e.kind, EntryKind::Directory)))
            .collect::<Vec<_>>();
        let shield_set: HashSet<CasefoldKey> = shielded
            .iter()
            .map(|rel| CasefoldKey::new(Path::new(rel)))
            .collect();

        Ok(plan_from_sets(dest_root, source_set, shield_set, dest_set))
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
mod casefold_tests {
    use super::CasefoldKey;
    use std::path::Path;

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
