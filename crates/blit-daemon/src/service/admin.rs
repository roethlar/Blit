use super::{DiskUsageSender, FindSender};
use blit_core::enumeration::{EntryKind, FileEnumerator};
use blit_core::fs_enum::FileFilter;
use blit_core::generated::{DiskUsageEntry, FilesystemStatsResponse, FindEntry};
use std::collections::{HashMap, HashSet};
use std::fs;
use std::io::ErrorKind;
use std::path::{Path, PathBuf};
use sysinfo::Disks;
use tokio::task;
use tonic::Status;

use super::util::{metadata_mtime_seconds, pathbuf_to_display, resolve_relative_path};

#[derive(Debug, Default, Clone, Copy)]
pub(crate) struct DeletionStats {
    pub files: u64,
    pub dirs: u64,
}

impl DeletionStats {
    pub(crate) fn total(self) -> u64 {
        self.files + self.dirs
    }
}

pub(crate) fn sanitize_request_paths(paths: Vec<String>) -> Result<Vec<PathBuf>, Status> {
    let mut sanitized = Vec::new();
    for raw in paths {
        if raw.trim().is_empty() {
            return Err(Status::invalid_argument(
                "paths_to_delete cannot contain empty entries",
            ));
        }
        let rel = resolve_relative_path(&raw)?;
        if rel.as_os_str().is_empty() || rel == Path::new(".") {
            return Err(Status::invalid_argument(
                "refusing to delete module root; specify a sub-path",
            ));
        }
        sanitized.push(rel);
    }
    Ok(sanitized)
}

pub(crate) async fn delete_rel_paths(
    module_path: PathBuf,
    rel_paths: Vec<PathBuf>,
) -> Result<DeletionStats, Status> {
    task::spawn_blocking(move || delete_rel_paths_sync(&module_path, rel_paths))
        .await
        .map_err(|err| Status::internal(format!("purge task failed: {}", err)))?
}

pub(crate) async fn purge_extraneous_entries(
    module_path: PathBuf,
    expected_files: Vec<PathBuf>,
) -> Result<DeletionStats, Status> {
    task::spawn_blocking(move || {
        let extraneous = plan_extraneous_entries(&module_path, &expected_files)?;
        if extraneous.is_empty() {
            return Ok(DeletionStats::default());
        }
        delete_rel_paths_sync(&module_path, extraneous)
    })
    .await
    .map_err(|err| Status::internal(format!("purge task failed: {}", err)))?
}

fn plan_extraneous_entries(
    module_path: &Path,
    expected_files: &[PathBuf],
) -> Result<Vec<PathBuf>, Status> {
    let enumerator = FileEnumerator::new(FileFilter::default());
    let entries = enumerator.enumerate_local(module_path).map_err(|err| {
        Status::internal(format!(
            "enumerating target {}: {}",
            module_path.display(),
            err
        ))
    })?;

    let mut expected_file_set: HashSet<PathBuf> = HashSet::new();
    let mut expected_dirs: HashSet<PathBuf> = HashSet::new();
    expected_dirs.insert(PathBuf::from("."));

    for rel in expected_files {
        expected_file_set.insert(rel.clone());
        let mut current = rel.parent();
        while let Some(parent) = current {
            if parent.as_os_str().is_empty() {
                expected_dirs.insert(PathBuf::from("."));
                break;
            }
            expected_dirs.insert(parent.to_path_buf());
            current = parent.parent();
        }
    }

    let mut files_to_delete = Vec::new();
    let mut dirs_to_delete = Vec::new();

    for entry in entries {
        let rel = entry.relative_path;
        if rel.as_os_str().is_empty() || rel == Path::new(".") {
            continue;
        }
        match &entry.kind {
            EntryKind::Directory => {
                if !expected_dirs.contains(&rel) {
                    dirs_to_delete.push(rel);
                }
            }
            _ => {
                if !expected_file_set.contains(&rel) {
                    files_to_delete.push(rel);
                }
            }
        }
    }

    dirs_to_delete.sort_by_key(|p| p.components().count());
    dirs_to_delete.reverse();

    files_to_delete.extend(dirs_to_delete);
    Ok(files_to_delete)
}

fn delete_rel_paths_sync(
    module_path: &Path,
    rel_paths: Vec<PathBuf>,
) -> Result<DeletionStats, Status> {
    let mut files = Vec::new();
    let mut dirs = Vec::new();

    for rel in rel_paths {
        if rel.as_os_str().is_empty() || rel == Path::new(".") {
            continue;
        }

        let target = module_path.join(&rel);
        let metadata = match std::fs::symlink_metadata(&target) {
            Ok(meta) => meta,
            Err(err) if err.kind() == ErrorKind::NotFound => continue,
            Err(err) => {
                return Err(Status::internal(format!(
                    "stat {}: {}",
                    target.display(),
                    err
                )));
            }
        };

        if metadata.file_type().is_dir() {
            dirs.push(rel);
        } else {
            files.push(rel);
        }
    }

    let mut stats = DeletionStats::default();

    for rel in files {
        let target = module_path.join(&rel);
        #[cfg(windows)]
        {
            blit_core::win_fs::clear_readonly_recursive(&target);
        }
        match std::fs::remove_file(&target) {
            Ok(_) => {
                stats.files += 1;
            }
            Err(err) if err.kind() == ErrorKind::NotFound => {}
            Err(err) if err.kind() == ErrorKind::IsADirectory => {
                match std::fs::remove_dir_all(&target) {
                    Ok(_) => {
                        stats.dirs += 1;
                    }
                    Err(inner) if inner.kind() == ErrorKind::NotFound => {}
                    Err(inner) => {
                        return Err(Status::internal(format!(
                            "remove_dir_all {}: {}",
                            target.display(),
                            inner
                        )));
                    }
                }
            }
            Err(err) => {
                return Err(Status::internal(format!(
                    "remove_file {}: {}",
                    target.display(),
                    err
                )));
            }
        }
    }

    for rel in dirs {
        let target = module_path.join(&rel);
        match std::fs::remove_dir_all(&target) {
            Ok(_) => stats.dirs += 1,
            Err(err) if err.kind() == ErrorKind::NotFound => {}
            Err(err) => {
                return Err(Status::internal(format!(
                    "remove_dir_all {}: {}",
                    target.display(),
                    err
                )));
            }
        }
    }

    Ok(stats)
}

pub(crate) fn split_completion_prefix(raw: &str) -> Result<(PathBuf, String, String), Status> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return Ok((PathBuf::from("."), String::new(), String::new()));
    }

    let normalized = trimmed.replace('\\', "/");
    let (dir_part, leaf_part) = match normalized.rsplit_once('/') {
        Some((dir, leaf)) => (dir.to_string(), leaf.to_string()),
        None => (String::new(), normalized),
    };

    let display_prefix = dir_part.clone();
    let dir_rel = if dir_part.is_empty() {
        PathBuf::from(".")
    } else {
        resolve_relative_path(&dir_part)?
    };

    Ok((dir_rel, display_prefix, leaf_part))
}

pub(crate) fn list_completions(
    search_root: &Path,
    display_prefix: &str,
    leaf_prefix: &str,
    include_files: bool,
    include_dirs: bool,
) -> Result<Vec<String>, Status> {
    let mut results = Vec::new();
    let entries = fs::read_dir(search_root)
        .map_err(|err| Status::internal(format!("read_dir {}: {}", search_root.display(), err)))?;

    for entry in entries {
        let entry = entry.map_err(|err| {
            Status::internal(format!("read_dir entry {}: {}", search_root.display(), err))
        })?;
        let name = entry.file_name().to_string_lossy().into_owned();
        if !leaf_prefix.is_empty() && !name.starts_with(leaf_prefix) {
            continue;
        }

        let metadata = match entry.metadata() {
            Ok(meta) => meta,
            Err(err) => {
                eprintln!(
                    "[warn] failed to stat completion candidate {}: {}",
                    entry.path().display(),
                    err
                );
                continue;
            }
        };

        let is_dir = metadata.is_dir();
        if is_dir && !include_dirs {
            continue;
        }
        if !is_dir && !include_files {
            continue;
        }

        let mut completion = String::new();
        if !display_prefix.is_empty() {
            completion.push_str(display_prefix);
            if !display_prefix.ends_with('/') {
                completion.push('/');
            }
        }
        completion.push_str(&name);
        if is_dir {
            completion.push('/');
        }
        results.push(completion);
    }

    results.sort();
    results.dedup();
    Ok(results)
}

#[derive(Default)]
struct UsageAccum {
    bytes: u64,
    files: u64,
    dirs: u64,
}

pub(crate) fn stream_disk_usage(
    module_root: PathBuf,
    start_rel: PathBuf,
    max_depth: Option<usize>,
    sender: &DiskUsageSender,
) -> Result<(), Status> {
    let start_abs = module_root.join(&start_rel);
    if !start_abs.exists() {
        return Err(Status::not_found(format!(
            "start path not found for disk usage: {}",
            pathbuf_to_display(&start_rel)
        )));
    }

    let mut accum: HashMap<PathBuf, UsageAccum> = HashMap::new();
    accum.entry(PathBuf::from(".")).or_default();

    let add_file = |accum: &mut HashMap<PathBuf, UsageAccum>,
                    rel: &Path,
                    size: u64,
                    max_depth: Option<usize>| {
        for (depth, prefix) in prefix_paths(rel).into_iter().enumerate() {
            if let Some(max) = max_depth {
                if depth > max {
                    break;
                }
            }
            let entry = accum.entry(prefix).or_default();
            entry.bytes += size;
            entry.files += 1;
        }
    };

    let add_dir =
        |accum: &mut HashMap<PathBuf, UsageAccum>, rel: &Path, max_depth: Option<usize>| {
            for (depth, prefix) in prefix_paths(rel).into_iter().enumerate() {
                if let Some(max) = max_depth {
                    if depth > max {
                        break;
                    }
                }
                let entry = accum.entry(prefix).or_default();
                entry.dirs += 1;
            }
        };

    let metadata = start_abs
        .metadata()
        .map_err(|err| Status::internal(format!("stat {}: {}", start_abs.display(), err)))?;

    if start_abs.is_file() {
        add_file(&mut accum, &start_rel, metadata.len(), max_depth);
    } else {
        if start_rel != PathBuf::from(".") {
            add_dir(&mut accum, &start_rel, max_depth);
        }
        let enumerator = FileEnumerator::new(FileFilter::default());
        enumerator
            .enumerate_local_streaming(&start_abs, |entry| {
                let rel_from_root = if start_rel == PathBuf::from(".") {
                    entry.relative_path.clone()
                } else {
                    let mut combined = start_rel.clone();
                    if entry.relative_path != PathBuf::from(".") {
                        combined.push(&entry.relative_path);
                    }
                    combined
                };

                match entry.kind {
                    EntryKind::Directory => add_dir(&mut accum, &rel_from_root, max_depth),
                    EntryKind::File { size } => {
                        add_file(&mut accum, &rel_from_root, size, max_depth)
                    }
                    EntryKind::Symlink { .. } => {}
                }
                Ok(())
            })
            .map_err(|err| Status::internal(format!("disk usage enumeration failed: {err}")))?;
    }

    let mut entries: Vec<(usize, PathBuf, UsageAccum)> = accum
        .into_iter()
        .map(|(path, usage)| {
            let depth = if path == PathBuf::from(".") {
                0
            } else {
                path.components().count()
            };
            (depth, path, usage)
        })
        .collect();

    entries.sort_by(|a, b| {
        a.0.cmp(&b.0)
            .then_with(|| pathbuf_to_display(&a.1).cmp(&pathbuf_to_display(&b.1)))
    });

    for (depth, path, usage) in entries {
        if let Some(max) = max_depth {
            if depth > max {
                continue;
            }
        }
        let entry = DiskUsageEntry {
            relative_path: pathbuf_to_display(&path),
            byte_total: usage.bytes,
            file_count: usage.files,
            dir_count: usage.dirs,
        };
        sender
            .blocking_send(Ok(entry))
            .map_err(|_| Status::internal("client dropped disk usage stream"))?;
    }

    Ok(())
}

fn prefix_paths(rel: &Path) -> Vec<PathBuf> {
    if rel == Path::new(".") {
        return vec![PathBuf::from(".")];
    }
    let mut prefixes = vec![PathBuf::from(".")];
    let mut current = PathBuf::new();
    for component in rel.components() {
        current.push(component.as_os_str());
        prefixes.push(current.clone());
    }
    prefixes
}

pub(crate) fn stream_find_entries(
    module_root: PathBuf,
    start_rel: PathBuf,
    pattern: String,
    case_sensitive: bool,
    include_files: bool,
    include_dirs: bool,
    max_results: Option<usize>,
    sender: &FindSender,
) -> Result<(), Status> {
    let start_abs = module_root.join(&start_rel);
    if !start_abs.exists() {
        return Err(Status::not_found(format!(
            "start path not found for find: {}",
            pathbuf_to_display(&start_rel)
        )));
    }

    let matcher = if pattern.is_empty() {
        None
    } else if case_sensitive {
        Some(pattern)
    } else {
        Some(pattern.to_lowercase())
    };

    let mut sent = 0usize;
    let limit = max_results.filter(|&m| m > 0);

    let mut maybe_emit =
        |rel_path: PathBuf, metadata: std::fs::Metadata, is_dir: bool| -> Result<(), Status> {
            if let Some(limit) = limit {
                if sent >= limit {
                    return Ok(());
                }
            }
            if is_dir && !include_dirs {
                return Ok(());
            }
            if !is_dir && !include_files {
                return Ok(());
            }

            let rel_display = pathbuf_to_display(&rel_path);
            if let Some(ref pat) = matcher {
                let candidate = if case_sensitive {
                    rel_display.clone()
                } else {
                    rel_display.to_lowercase()
                };
                if !candidate.contains(pat) {
                    return Ok(());
                }
            }

            let entry = FindEntry {
                relative_path: rel_display,
                is_dir,
                size: if is_dir { 0 } else { metadata.len() },
                mtime_seconds: metadata_mtime_seconds(&metadata).unwrap_or(0),
            };
            sender
                .blocking_send(Ok(entry))
                .map_err(|_| Status::internal("client dropped find stream"))?;
            sent += 1;
            Ok(())
        };

    let metadata = start_abs
        .metadata()
        .map_err(|err| Status::internal(format!("stat {}: {}", start_abs.display(), err)))?;

    if start_abs.is_file() {
        maybe_emit(start_rel.clone(), metadata, false)?;
        return Ok(());
    }

    if include_dirs && start_rel != PathBuf::from(".") {
        maybe_emit(start_rel.clone(), metadata, true)?;
    }

    let enumerator = FileEnumerator::new(FileFilter::default());
    enumerator
        .enumerate_local_streaming(&start_abs, |entry| {
            let rel_from_root = if start_rel == PathBuf::from(".") {
                entry.relative_path.clone()
            } else {
                let mut combined = start_rel.clone();
                if entry.relative_path != PathBuf::from(".") {
                    combined.push(&entry.relative_path);
                }
                combined
            };
            let is_dir = matches!(entry.kind, EntryKind::Directory);
            maybe_emit(rel_from_root, entry.metadata, is_dir)?;
            Ok(())
        })
        .map_err(|err| Status::internal(format!("find enumeration failed: {err}")))?;

    Ok(())
}

pub(crate) fn filesystem_stats_for_path(path: &Path) -> Result<FilesystemStatsResponse, Status> {
    let canonical = fs::canonicalize(path).map_err(|err| {
        Status::internal(format!(
            "failed to resolve filesystem stats path {}: {}",
            path.display(),
            err
        ))
    })?;

    let mut disks = Disks::new_with_refreshed_list();
    disks.refresh_list();
    disks.refresh();

    let mut best_match = None;
    let mut best_len = 0usize;
    for disk in disks.iter() {
        let mount = disk.mount_point();
        if canonical.starts_with(mount) {
            let depth = mount.components().count();
            if depth >= best_len {
                best_len = depth;
                best_match = Some(disk);
            }
        }
    }

    let disk = best_match.ok_or_else(|| {
        Status::internal(format!(
            "no filesystem information available for {}",
            path.display()
        ))
    })?;

    Ok(FilesystemStatsResponse {
        module: pathbuf_to_display(path),
        total_bytes: disk.total_space(),
        used_bytes: disk.total_space().saturating_sub(disk.available_space()),
        free_bytes: disk.available_space(),
    })
}
