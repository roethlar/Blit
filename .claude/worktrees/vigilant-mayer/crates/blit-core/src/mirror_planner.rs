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

#[cfg(windows)]
#[derive(Hash, Eq, PartialEq, Clone, Debug)]
struct CasefoldKey(String);

#[cfg(windows)]
impl CasefoldKey {
    fn new(path: &Path) -> Self {
        let normalized = path
            .to_string_lossy()
            .replace('\\', "/")
            .to_ascii_lowercase();
        CasefoldKey(normalized)
    }
}

#[cfg(not(windows))]
#[derive(Hash, Eq, PartialEq, Clone, Debug)]
struct CasefoldKey(PathBuf);

#[cfg(not(windows))]
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

        Ok(plan_from_sets(destination, source_set, dest_set))
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

        Ok(plan_from_sets(destination, source_set, dest_set))
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

        plan_from_sets(dest_root, source_set, dest_set)
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

        Ok(plan_from_sets(dest_root, source_keys, dest_set))
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
