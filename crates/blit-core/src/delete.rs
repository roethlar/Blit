use anyhow::Result;
use std::collections::HashSet;
use std::path::{Path, PathBuf};

use crate::fs_enum::{enumerate_directory_filtered, FileEntry, FileFilter};

/// Planned deletions produced by diffing source and destination trees.
#[derive(Debug, Default, Clone)]
pub struct DeletePlan {
    pub files: Vec<PathBuf>,
    pub dirs: Vec<PathBuf>,
}

fn mark_parent_dirs(path: &Path, root: &Path, set: &mut HashSet<PathBuf>) {
    let mut current = path.parent();
    while let Some(parent) = current {
        if parent == root {
            break;
        }
        set.insert(parent.to_path_buf());
        current = parent.parent();
    }
}

/// Build a deletion plan using on-disk enumeration for both source and destination.
pub fn compute_delete_plan(source: &Path, dest: &Path, filter: &FileFilter) -> Result<DeletePlan> {
    let mut src_filter = filter.clone_without_cache();
    let mut dst_filter = filter.clone_without_cache();
    let source_entries = enumerate_directory_filtered(source, &mut src_filter)?;
    let dest_entries = enumerate_directory_filtered(dest, &mut dst_filter)?;
    Ok(generate_delete_plan(
        source,
        dest,
        &source_entries,
        &dest_entries,
    ))
}

/// Build a deletion plan from pre-enumerated file lists.
pub fn generate_delete_plan(
    source_root: &Path,
    dest_root: &Path,
    source_entries: &[FileEntry],
    dest_entries: &[FileEntry],
) -> DeletePlan {
    let mut expected_dirs: HashSet<PathBuf> = HashSet::new();
    let mut expected_files: HashSet<PathBuf> = HashSet::new();
    expected_dirs.insert(dest_root.to_path_buf());

    for entry in source_entries {
        let rel = match entry.path.strip_prefix(source_root) {
            Ok(r) if !r.as_os_str().is_empty() => r,
            _ => continue,
        };
        let dest_path = dest_root.join(rel);
        if entry.is_directory {
            expected_dirs.insert(dest_path.clone());
            mark_parent_dirs(&dest_path, dest_root, &mut expected_dirs);
        } else {
            expected_files.insert(dest_path.clone());
            mark_parent_dirs(&dest_path, dest_root, &mut expected_dirs);
        }
    }

    let mut files_to_delete = Vec::new();
    let mut dirs_to_delete = Vec::new();

    for entry in dest_entries {
        if entry.path == dest_root {
            continue;
        }
        if entry.is_directory {
            if !expected_dirs.contains(&entry.path) {
                dirs_to_delete.push(entry.path.clone());
            }
        } else if !expected_files.contains(&entry.path) {
            files_to_delete.push(entry.path.clone());
        }
    }

    dirs_to_delete.sort_by_key(|p| p.components().count());
    dirs_to_delete.reverse();

    DeletePlan {
        files: files_to_delete,
        dirs: dirs_to_delete,
    }
}

/// Convenience helper to summarise totals for logging.
pub fn plan_counts(plan: &DeletePlan) -> (usize, usize) {
    (plan.files.len(), plan.dirs.len())
}
