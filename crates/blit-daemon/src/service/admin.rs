use super::{DiskUsageSender, FindSender};
use blit_core::enumeration::{EntryKind, FileEnumerator};
use blit_core::fs_enum::FileFilter;
use blit_core::generated::{DiskUsageEntry, FilesystemStatsResponse, FindEntry};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use sysinfo::Disks;
use tokio::task;
use tonic::Status;

use super::util::{internal_err, io_to_status, resolve_relative_path, response_channel_closed};
use blit_core::path_posix::request_path_to_posix;
use blit_core::wire_metadata::mtime_seconds;

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
) -> Result<blit_core::deletion::DeletionStats, Status> {
    task::spawn_blocking(move || delete_rel_paths_sync(&module_path, &canonical_root, rel_paths))
        .await
        .map_err(|err| internal_err("purge task failed", err))?
}

fn delete_rel_paths_sync(
    module_path: &Path,
    canonical_root: &Path,
    rel_paths: Vec<PathBuf>,
) -> Result<blit_core::deletion::DeletionStats, Status> {
    let (files, dirs) = blit_core::deletion::classify_explicit_targets(
        module_path,
        canonical_root,
        rel_paths,
        "purge",
    )
    .map_err(deletion_status)?;
    blit_core::deletion::execute_deletion_plan(
        &files,
        &dirs,
        blit_core::deletion::DeletionOptions {
            operation: "purge",
            canonical_root: Some(canonical_root),
            abort: None,
            execute: true,
            directory_mode: blit_core::deletion::DirectoryMode::Recursive,
        },
    )
    .map_err(deletion_status)
}

fn deletion_status(error: blit_core::deletion::DeletionError) -> Status {
    match error {
        error @ blit_core::deletion::DeletionError::Containment { .. } => {
            Status::permission_denied(error.to_string())
        }
        error => internal_err("purge deletion failed", error),
    }
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
        .map_err(|err| io_to_status(format!("read_dir {}", search_root.display()), err))?;

    for entry in entries {
        let entry = entry.map_err(|err| {
            io_to_status(format!("read_dir entry {}", search_root.display()), err)
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
            request_path_to_posix(&start_rel)
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
        .map_err(|err| io_to_status(format!("stat {}", start_abs.display()), err))?;

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
            .map_err(|err| internal_err("disk usage enumeration failed", err))?;
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
            .then_with(|| request_path_to_posix(&a.1).cmp(&request_path_to_posix(&b.1)))
    });

    for (depth, path, usage) in entries {
        if let Some(max) = max_depth {
            if depth > max {
                continue;
            }
        }
        let entry = DiskUsageEntry {
            relative_path: request_path_to_posix(&path),
            byte_total: usage.bytes,
            file_count: usage.files,
            dir_count: usage.dirs,
        };
        sender
            .blocking_send(Ok(entry))
            .map_err(|_| response_channel_closed("sending disk usage result"))?;
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
            request_path_to_posix(&start_rel)
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

            let rel_display = request_path_to_posix(&rel_path);
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
                mtime_seconds: mtime_seconds(&metadata).unwrap_or(0),
            };
            sender
                .blocking_send(Ok(entry))
                .map_err(|_| response_channel_closed("sending find result"))?;
            sent += 1;
            Ok(())
        };

    let metadata = start_abs
        .metadata()
        .map_err(|err| io_to_status(format!("stat {}", start_abs.display()), err))?;

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
        .map_err(|err| internal_err("find enumeration failed", err))?;

    Ok(())
}

pub(crate) fn filesystem_stats_for_path(path: &Path) -> Result<FilesystemStatsResponse, Status> {
    let canonical = fs::canonicalize(path).map_err(|err| {
        io_to_status(
            format!("failed to resolve filesystem stats path {}", path.display()),
            err,
        )
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
        module: request_path_to_posix(path),
        total_bytes: disk.total_space(),
        used_bytes: disk.total_space().saturating_sub(disk.available_space()),
        free_bytes: disk.available_space(),
    })
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

#[cfg(test)]
mod status_mapping_tests {
    use super::*;
    use tempfile::tempdir;
    use tonic::Code;

    #[test]
    fn missing_filesystem_paths_cross_as_not_found() {
        let temp = tempdir().expect("tempdir");
        let missing = temp.path().join("missing");

        let completion_error =
            list_completions(&missing, "", "", true, true).expect_err("missing completion root");
        assert_eq!(completion_error.code(), Code::NotFound);

        let stats_error =
            filesystem_stats_for_path(&missing).expect_err("missing filesystem stats root");
        assert_eq!(stats_error.code(), Code::NotFound);
    }

    #[test]
    fn closed_admin_streams_cross_as_cancelled_with_one_vocabulary() {
        let temp = tempdir().expect("tempdir");
        let file = temp.path().join("entry");
        std::fs::File::create(&file).expect("create empty fixture");
        let module_root = std::fs::canonicalize(temp.path()).expect("canonical tempdir");

        let (disk_sender, disk_receiver) = tokio::sync::mpsc::channel(1);
        drop(disk_receiver);
        let disk_error = stream_disk_usage(
            module_root.clone(),
            PathBuf::from("entry"),
            None,
            &disk_sender,
        )
        .expect_err("closed disk-usage stream");
        assert_eq!(disk_error.code(), Code::Cancelled);
        assert_eq!(
            disk_error.message(),
            "response channel closed (peer disconnected): sending disk usage result"
        );

        let (find_sender, find_receiver) = tokio::sync::mpsc::channel(1);
        drop(find_receiver);
        let find_error = stream_find_entries(
            module_root,
            PathBuf::from("entry"),
            String::new(),
            true,
            true,
            false,
            None,
            &find_sender,
        )
        .expect_err("closed find stream");
        assert_eq!(find_error.code(), Code::Cancelled);
        assert_eq!(
            find_error.message(),
            "response channel closed (peer disconnected): sending find result"
        );
    }
}
