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
    canonical_root: PathBuf,
    rel_paths: Vec<PathBuf>,
) -> Result<DeletionStats, Status> {
    task::spawn_blocking(move || delete_rel_paths_sync(&module_path, &canonical_root, rel_paths))
        .await
        .map_err(|err| Status::internal(format!("purge task failed: {}", err)))?
}

pub(crate) async fn purge_extraneous_entries(
    module_path: PathBuf,
    canonical_root: PathBuf,
    expected_files: Vec<PathBuf>,
    // R59 #1 F2: filter to apply when enumerating the destination
    // for purge candidates. Default = unfiltered (legacy /
    // mirror_kind=ALL behavior). When the caller is a
    // FilteredSubset mirror, this carries the user's source-side
    // filter so out-of-scope destination entries aren't classified
    // as extraneous and deleted.
    purge_filter: FileFilter,
) -> Result<DeletionStats, Status> {
    task::spawn_blocking(move || {
        // R13-F1: verify the purge root is contained before any
        // enumeration. The delete phase below has its own per-entry
        // verify_contained, but plan_extraneous_entries enumerates
        // module_path itself — that read-side filesystem op needs the
        // same protection. The push handler's destination_path
        // containment check should already reject escape paths at
        // handshake; this is defense-in-depth in case a future
        // caller bypasses the handshake check.
        blit_core::path_safety::verify_contained(&canonical_root, &module_path)
            .map_err(|e| Status::permission_denied(format!("purge root containment: {e:#}")))?;

        let extraneous = plan_extraneous_entries(&module_path, &expected_files, &purge_filter)?;
        if extraneous.is_empty() {
            return Ok(DeletionStats::default());
        }
        delete_rel_paths_sync(&module_path, &canonical_root, extraneous)
    })
    .await
    .map_err(|err| Status::internal(format!("purge task failed: {}", err)))?
}

fn plan_extraneous_entries(
    module_path: &Path,
    expected_files: &[PathBuf],
    purge_filter: &FileFilter,
) -> Result<Vec<PathBuf>, Status> {
    // R59 #1 F2: enumerate the destination through the user's
    // filter so we never classify out-of-scope entries as
    // "extraneous". When the caller is mirror_kind=ALL the filter
    // is FileFilter::default() and behavior matches the historical
    // unfiltered purge.
    let enumerator = FileEnumerator::new(purge_filter.clone_without_cache());
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
    canonical_root: &Path,
    rel_paths: Vec<PathBuf>,
) -> Result<DeletionStats, Status> {
    let mut files = Vec::new();
    let mut dirs = Vec::new();

    for rel in rel_paths {
        if rel.as_os_str().is_empty() || rel == Path::new(".") {
            continue;
        }

        let target = module_path.join(&rel);
        // F2: containment check before any filesystem operation.
        // Verified against canonical_root so post-push-mutated
        // module_path doesn't bypass the original boundary.
        blit_core::path_safety::verify_contained(canonical_root, &target)
            .map_err(|e| Status::permission_denied(format!("path containment: {e:#}")))?;
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

    // Route user-typed input through the canonical helper so a literal
    // `\` in a POSIX filename (legal on macOS/Linux) is preserved, while
    // a Windows-native `Folder\file` is correctly split on the native
    // separator. Plain string `replace` would conflate the two.
    let normalized = blit_core::path_posix::relative_str_to_posix(trimmed);
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
                log::warn!(
                    "failed to stat completion candidate {}: {}",
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
    // F2: containment check before any filesystem operation. The
    // enumerator below has follow_symlinks=false so it won't escape
    // during traversal, but the start point itself can be a symlink
    // that points outside the module root. `module_root` here is the
    // canonical root — du callers don't mutate it.
    blit_core::path_safety::verify_contained(&module_root, &start_abs)
        .map_err(|e| Status::permission_denied(format!("path containment: {e:#}")))?;
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
        if start_rel != Path::new(".") {
            add_dir(&mut accum, &start_rel, max_depth);
        }
        let enumerator = FileEnumerator::new(FileFilter::default());
        enumerator
            .enumerate_local_streaming(&start_abs, |entry| {
                let rel_from_root = if start_rel == Path::new(".") {
                    entry.relative_path.clone()
                } else {
                    let mut combined = start_rel.clone();
                    if entry.relative_path != Path::new(".") {
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
            let depth = if path == Path::new(".") {
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
    // F2: containment check before any filesystem operation.
    blit_core::path_safety::verify_contained(&module_root, &start_abs)
        .map_err(|e| Status::permission_denied(format!("path containment: {e:#}")))?;
    if !start_abs.exists() {
        return Err(Status::not_found(format!(
            "start path not found for find: {}",
            pathbuf_to_display(&start_rel)
        )));
    }

    // Pattern matching is glob-based, matching `BLIT_UTILS_PLAN.md`.
    // Pre-0.1.0 behavior was substring containment; the move to
    // glob is a deliberate API change before 0.1.0 ships (no
    // backwards-compat constraint).
    //
    // R41-F3: `literal_separator(true)` matches POSIX shell-glob
    // semantics — `*` does NOT cross `/`, so `foo*.csv` matches
    // `foo-bar.csv` but NOT `foo/bar.csv`. The basename fallback
    // below covers the common "find files with this extension at
    // any depth" use case (`*.csv` matches both `top.csv` and
    // `nested/x.csv` via the basename), without making `*` greedy
    // across the whole path. Users wanting a path-component-
    // crossing match write `**/`.
    let matcher = if pattern.is_empty() {
        None
    } else {
        let glob = globset::GlobBuilder::new(&pattern)
            .case_insensitive(!case_sensitive)
            .literal_separator(true)
            .build()
            .map_err(|e| {
                Status::invalid_argument(format!("invalid find --pattern glob '{pattern}': {e}"))
            })?;
        Some(glob.compile_matcher())
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
            if let Some(ref glob_matcher) = matcher {
                // Match against the relative path AND its file-name
                // tail. Users typically write `--pattern '*.txt'`
                // expecting it to match any file with that
                // extension regardless of depth. With
                // `literal_separator(false)` `*.txt` matches across
                // directory components, but matching against the
                // basename too is the intuitive fallback for
                // patterns that don't use `**`.
                let basename = std::path::Path::new(&rel_display)
                    .file_name()
                    .and_then(|s| s.to_str())
                    .unwrap_or(&rel_display);
                if !glob_matcher.is_match(&rel_display) && !glob_matcher.is_match(basename) {
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

    if include_dirs && start_rel != Path::new(".") {
        maybe_emit(start_rel.clone(), metadata, true)?;
    }

    let enumerator = FileEnumerator::new(FileFilter::default());
    enumerator
        .enumerate_local_streaming(&start_abs, |entry| {
            let rel_from_root = if start_rel == Path::new(".") {
                entry.relative_path.clone()
            } else {
                let mut combined = start_rel.clone();
                if entry.relative_path != Path::new(".") {
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

    // On Windows, fs::canonicalize returns extended-length paths with \\?\ prefix,
    // but sysinfo::Disks returns mount points without this prefix. Strip it for comparison.
    #[cfg(windows)]
    let canonical = {
        let s = canonical.to_string_lossy();
        if let Some(stripped) = s.strip_prefix(r"\\?\") {
            PathBuf::from(stripped)
        } else {
            canonical
        }
    };

    let disks = Disks::new_with_refreshed_list();

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

#[cfg(test)]
mod purge_filter_tests {
    //! R59 #1 F2: the daemon's purge enumerator must honor the
    //! source-side filter when mirror_kind=FilteredSubset, so
    //! destination entries excluded by the user's filter aren't
    //! deleted just because they're absent from the (filtered)
    //! source manifest. Pre-fix the enumerator used
    //! FileFilter::default() and treated every out-of-scope file
    //! as extraneous.

    use super::*;
    use std::fs;
    use tempfile::tempdir;

    #[test]
    fn filtered_subset_keeps_excluded_destination_entries() {
        let tmp = tempdir().unwrap();
        let module = tmp.path();
        fs::write(module.join("kept.txt"), b"a").unwrap();
        fs::write(module.join("kept.log"), b"b").unwrap();
        fs::write(module.join("extra.txt"), b"c").unwrap();

        // Source filter is `--include '*.txt'`. Source manifest
        // would only carry `*.txt` files. Expected = the one txt
        // file the client knows about. Without the filter scope
        // applied during enumeration, the daemon would classify
        // kept.log as extraneous (it's not in expected) and
        // delete it.
        let mut filter = FileFilter::default();
        filter.include_files = vec!["*.txt".to_string()];

        let expected = vec![PathBuf::from("kept.txt")];
        let extras = plan_extraneous_entries(module, &expected, &filter).unwrap();

        assert!(
            extras.contains(&PathBuf::from("extra.txt")),
            "extra.txt is an in-scope file absent from source — must be purged"
        );
        assert!(
            !extras.iter().any(|p| p == Path::new("kept.log")),
            "kept.log is out of scope (matches --exclude semantics implicit \
             in --include '*.txt') — must NOT be purged"
        );
    }

    #[test]
    fn unfiltered_purge_treats_all_unexpected_as_extras() {
        let tmp = tempdir().unwrap();
        let module = tmp.path();
        fs::write(module.join("kept.txt"), b"a").unwrap();
        fs::write(module.join("kept.log"), b"b").unwrap();

        // mirror_kind=ALL → empty filter → matches pre-R59 behavior.
        let filter = FileFilter::default();
        let expected = vec![PathBuf::from("kept.txt")];
        let extras = plan_extraneous_entries(module, &expected, &filter).unwrap();

        assert!(extras.contains(&PathBuf::from("kept.log")));
    }
}

#[cfg(test)]
mod disk_usage_depth_tests {
    //! d-41 R2: pins the `stream_disk_usage` depth contract the
    //! TUI's F3 `u` hotkey relies on. The TUI requests a bounded
    //! depth (`main::F3_DU_MAX_DEPTH = 1`) so a single Stats-line
    //! aggregate doesn't pull the full descendant stream over
    //! gRPC. This test fails if `max_depth = Some(1)` ever stops
    //! bounding the emitted rows, or if the root entry stops
    //! carrying the complete subtree total.

    use super::*;
    use blit_core::generated::DiskUsageEntry;
    use std::fs;
    use tempfile::tempdir;
    use tokio::sync::mpsc;
    use tonic::Status;

    #[test]
    fn depth_one_bounds_stream_yet_root_total_is_complete() {
        let tmp = tempdir().unwrap();
        // Canonicalize: on macOS the temp dir is under a /var →
        // /private/var symlink, and `stream_disk_usage`'s
        // containment check compares the canonical start path
        // against `module_root`. The real daemon always passes a
        // canonical module root.
        let root = fs::canonicalize(tmp.path()).unwrap();

        // 3 immediate child dirs, each holding files nested up to
        // 3 levels deep. ~180 files / many descendant paths — an
        // unbounded (depth 0 → None) query would stream a row for
        // each. A depth-1 query must stream only root + the 3
        // immediate children.
        let mut total_files = 0u64;
        for child_name in ["alpha", "beta", "gamma"] {
            let child = root.join(child_name);
            fs::create_dir_all(child.join("deep/deeper")).unwrap();
            for i in 0..20 {
                fs::write(child.join(format!("f{i}.bin")), [0u8; 100]).unwrap();
                fs::write(child.join("deep").join(format!("g{i}.bin")), [0u8; 100]).unwrap();
                fs::write(
                    child.join("deep/deeper").join(format!("h{i}.bin")),
                    [0u8; 100],
                )
                .unwrap();
                total_files += 3;
            }
        }

        // Drain on a worker thread so `blocking_send` inside
        // `stream_disk_usage` always has a receiver (no runtime
        // needed — both ends are blocking).
        let (tx, mut rx) = mpsc::channel::<Result<DiskUsageEntry, Status>>(4096);
        let module_root = root.clone();
        let handle = std::thread::spawn(move || {
            stream_disk_usage(module_root, PathBuf::from("."), Some(1), &tx)
        });
        let mut entries = Vec::new();
        while let Some(item) = rx.blocking_recv() {
            entries.push(item.expect("du entry"));
        }
        handle.join().unwrap().expect("stream_disk_usage ok");

        // Bounded: root + 3 immediate children = 4 rows, NOT one
        // per descendant path (~180+).
        assert_eq!(
            entries.len(),
            4,
            "depth=1 must bound the stream to root + immediate children; got {} rows: {:?}",
            entries.len(),
            entries
                .iter()
                .map(|e| e.relative_path.clone())
                .collect::<Vec<_>>(),
        );

        // The root aggregate (largest byte_total) still reflects
        // the COMPLETE subtree despite the depth cap.
        let root_entry = entries
            .iter()
            .max_by_key(|e| e.byte_total)
            .expect("at least one entry");
        assert_eq!(
            root_entry.file_count, total_files,
            "root must count every descendant file"
        );
        assert_eq!(
            root_entry.byte_total,
            total_files * 100,
            "root must sum every descendant byte"
        );
    }

    #[test]
    fn depth_zero_is_unbounded_streaming_every_descendant() {
        // Pins WHY the TUI must not request depth 0: the daemon
        // (core.rs) maps request max_depth==0 to `None`, and this
        // helper treats `None` as unbounded — every prefix path
        // is emitted. This is the behavior d-41 R1 accidentally
        // triggered.
        let tmp = tempdir().unwrap();
        // Canonicalize: on macOS the temp dir is under a /var →
        // /private/var symlink, and `stream_disk_usage`'s
        // containment check compares the canonical start path
        // against `module_root`. The real daemon always passes a
        // canonical module root.
        let root = fs::canonicalize(tmp.path()).unwrap();
        let child = root.join("alpha");
        fs::create_dir_all(child.join("deep")).unwrap();
        fs::write(child.join("a.bin"), [0u8; 10]).unwrap();
        fs::write(child.join("deep/b.bin"), [0u8; 10]).unwrap();

        let (tx, mut rx) = mpsc::channel::<Result<DiskUsageEntry, Status>>(4096);
        let module_root = root.clone();
        let handle = std::thread::spawn(move || {
            stream_disk_usage(module_root, PathBuf::from("."), None, &tx)
        });
        let mut paths = Vec::new();
        while let Some(item) = rx.blocking_recv() {
            paths.push(item.expect("entry").relative_path);
        }
        handle.join().unwrap().expect("ok");

        // Unbounded emits root + alpha + alpha/deep (every prefix),
        // i.e. strictly more than the depth-1 root+children bound.
        assert!(
            paths.len() > 2,
            "unbounded (None) must stream nested descendant paths; got {paths:?}"
        );
    }
}
