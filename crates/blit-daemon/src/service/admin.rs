use super::{DiskUsageSender, FindSender};
use blit_core::disk_usage::{disk_usage, os_reason, Stopped, UsageEnd, UsageEvent, UsageOptions};
use blit_core::enumeration::{EntryKind, FileEnumerator};
use blit_core::fs_enum::FileFilter;
use blit_core::generated::{du_reply, DfReply, DuFailure, DuReply, DuRow, FindEntry};
use std::fs;
use std::path::{Path, PathBuf};
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

/// `du` over `start_rel` below the module
/// (`docs/plan/UTILITY_VERBS_LOCAL_PARITY.md` slice 1): the walk is
/// `blit_core::disk_usage`, on this daemon's own disk; every line and
/// every unreadable entry goes to the asker as it is produced.
///
/// A start that cannot be read at all is one failure line for the start
/// itself, not a failed request: the asker prints it in `du`'s words
/// (`cannot access 'PATH': No such file or directory`) and exits 1, as
/// for any other entry.
pub(crate) fn stream_disk_usage(
    module_root: PathBuf,
    start_rel: PathBuf,
    options: UsageOptions,
    sender: &DiskUsageSender,
) -> Result<(), Status> {
    let start_abs = module_root.join(&start_rel);
    // F2: containment check before any filesystem operation. The walk
    // never follows a link, but the start point itself can be a symlink
    // that points outside the module root. `module_root` here is the
    // canonical root — du callers don't mutate it.
    blit_core::path_safety::verify_contained(&module_root, &start_abs)
        .map_err(|e| Status::permission_denied(format!("path containment: {e:#}")))?;

    let send = |reply: du_reply::Reply| {
        sender
            .blocking_send(Ok(DuReply { reply: Some(reply) }))
            .map_err(|_| Stopped)
    };
    let failure = |relative: &Path, action: &str, error: &std::io::Error| {
        du_reply::Reply::Failure(DuFailure {
            relative_path: wire_path(relative),
            raw_relative_path: blit_core::raw_name::raw_relative_bytes(relative),
            action: action.to_string(),
            reason: os_reason(error),
        })
    };
    let mut emit = |event: UsageEvent| match event {
        UsageEvent::Row(row) => send(du_reply::Reply::Row(DuRow {
            relative_path: wire_path(&row.relative_path),
            raw_relative_path: blit_core::raw_name::raw_relative_bytes(&row.relative_path),
            allocated_bytes: row.allocated_bytes,
        })),
        UsageEvent::Failure(failed) => {
            send(failure(&failed.relative_path, failed.action, &failed.error))
        }
    };
    let closed = || response_channel_closed("sending disk usage result");
    match disk_usage(&start_abs, options, &mut emit) {
        Ok(()) => Ok(()),
        Err(UsageEnd::Stopped(Stopped)) => Err(closed()),
        Err(UsageEnd::Start(error)) => {
            send(failure(Path::new(""), "cannot access", &error)).map_err(|Stopped| closed())
        }
    }
}

/// A path below the start as it goes to the asker: `/`-separated text,
/// empty for the start itself.
fn wire_path(relative: &Path) -> String {
    if relative.as_os_str().is_empty() {
        String::new()
    } else {
        request_path_to_posix(relative)
    }
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

/// `df` for `path` (`docs/plan/UTILITY_VERBS_LOCAL_PARITY.md` slice 1):
/// the operating system is asked about the filesystem that holds THIS
/// path. It once matched the path against a list of mounts and took the
/// longest match — a filesystem that list did not hold fell through to
/// the mount above it, and a module on a 16 GiB filesystem was reported
/// as another disk's 1.8 TiB.
///
/// A failure crosses with the operating system's words alone
/// (`No such file or directory`): the asker prints them after the path
/// it was given, as `df` does, and this daemon's own path stays here.
pub(crate) fn filesystem_space_for_path(path: &Path) -> Result<DfReply, Status> {
    match blit_core::disk_usage::filesystem_space(path) {
        Ok(space) => Ok(DfReply {
            total_bytes: space.total_bytes,
            used_bytes: space.used_bytes,
            available_bytes: space.available_bytes,
        }),
        Err(error) => {
            let reason = os_reason(&error);
            Err(match error.kind() {
                std::io::ErrorKind::NotFound => Status::not_found(reason),
                std::io::ErrorKind::PermissionDenied => Status::permission_denied(reason),
                _ => Status::internal(reason),
            })
        }
    }
}

#[cfg(test)]
mod disk_usage_tests {
    //! The daemon's half of `du` (UTILITY_VERBS_LOCAL_PARITY slice 1): what
    //! it SENDS. The walk itself is guarded in `blit_core::disk_usage`.

    use super::*;
    use std::fs;
    use tempfile::tempdir;
    use tokio::sync::mpsc;

    /// Everything `stream_disk_usage` sends for `start` under `root`.
    fn sent(root: &Path, start: &str, options: UsageOptions) -> Vec<du_reply::Reply> {
        // Drain on this thread while a worker sends: both ends block.
        let (tx, mut rx) = mpsc::channel::<Result<DuReply, Status>>(4096);
        let (module_root, start_rel) = (root.to_path_buf(), PathBuf::from(start));
        let worker =
            std::thread::spawn(move || stream_disk_usage(module_root, start_rel, options, &tx));
        let mut replies = Vec::new();
        while let Some(item) = rx.blocking_recv() {
            replies.push(item.expect("a reply").reply.expect("a row or a failure"));
        }
        worker.join().unwrap().expect("the stream ends well");
        replies
    }

    fn rows(replies: &[du_reply::Reply]) -> Vec<(String, u64)> {
        replies
            .iter()
            .filter_map(|reply| match reply {
                du_reply::Reply::Row(row) => Some((row.relative_path.clone(), row.allocated_bytes)),
                du_reply::Reply::Failure(_) => None,
            })
            .collect()
    }

    /// 3 folders, each holding files 3 levels deep: 180 files.
    fn tree() -> (tempfile::TempDir, PathBuf) {
        let tmp = tempdir().unwrap();
        // Canonicalize: on macOS the temp dir is under a /var →
        // /private/var symlink, and the containment check compares the
        // canonical start path against the module root. The real daemon
        // always passes a canonical module root.
        let root = fs::canonicalize(tmp.path()).unwrap();
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
            }
        }
        (tmp, root)
    }

    /// AC-7: `du` without `-a` sends NO file rows — a row is a line that
    /// will be printed, and nothing else crosses. 180 files, 10 rows: the
    /// module, its 3 folders and their 6 sub-folders, the start LAST and
    /// carrying everything. With `-a`, the 180 files are rows too.
    #[test]
    fn du_sends_a_row_for_each_line_it_prints_and_no_other() {
        let (_tmp, root) = tree();
        let every_level = UsageOptions::default();
        let sent_rows = rows(&sent(&root, ".", every_level));
        assert_eq!(sent_rows.len(), 10, "{sent_rows:?}");
        assert!(
            sent_rows.iter().all(|(path, _)| !path.ends_with(".bin")),
            "no file row without -a: {sent_rows:?}"
        );
        let (start, total) = sent_rows.last().unwrap();
        assert_eq!(
            start, "",
            "the start is the last row, with no path of its own"
        );
        assert!(
            sent_rows.iter().all(|(_, bytes)| bytes <= total),
            "the start carries every entry: {sent_rows:?}"
        );
        // An entry after everything below it.
        let at = |path: &str| sent_rows.iter().position(|(p, _)| p == path).unwrap();
        assert!(at("alpha/deep/deeper") < at("alpha/deep") && at("alpha/deep") < at("alpha"));

        let all = rows(&sent(
            &root,
            ".",
            UsageOptions {
                all_files: true,
                max_depth: None,
            },
        ));
        assert_eq!(all.len(), 190, "10 directories and 180 files");
    }

    /// AC-2: `-d N` counts from the START path, not from the module, and
    /// bounds the rows without changing a total. `-d 1` under `alpha` is
    /// `alpha/deep` and `alpha` itself — the old request counted from the
    /// module's root, so a depth of 1 on a sub-folder printed only that
    /// folder — and `-d 0` (`-s`) is the start alone.
    #[test]
    fn du_depth_counts_from_the_start_and_bounds_rows_not_totals() {
        let (_tmp, root) = tree();
        let unbounded = rows(&sent(&root, "alpha", UsageOptions::default()));
        let total = unbounded.last().unwrap().1;
        let paths = |replies: &[(String, u64)]| -> Vec<String> {
            replies.iter().map(|(path, _)| path.clone()).collect()
        };
        assert_eq!(paths(&unbounded), ["deep/deeper", "deep", ""]);

        let depth = |max| UsageOptions {
            all_files: false,
            max_depth: Some(max),
        };
        let one = rows(&sent(&root, "alpha", depth(1)));
        assert_eq!(paths(&one), ["deep", ""]);
        assert_eq!(one.last().unwrap().1, total, "a bound changes no total");
        let summary = rows(&sent(&root, "alpha", depth(0)));
        assert_eq!(summary, [(String::new(), total)]);
        let module = rows(&sent(&root, ".", depth(1)));
        assert_eq!(module.len(), 4, "the module and its 3 folders: {module:?}");
    }

    /// AC-2, AC-12: a file given as the start is its own row; a start that
    /// does not exist is one failure for the start, in `du`'s words, and
    /// the stream still ends well.
    #[test]
    fn du_answers_for_a_file_and_for_a_start_that_is_not_there() {
        let (_tmp, root) = tree();
        let file = sent(&root, "alpha/f0.bin", UsageOptions::default());
        assert_eq!(rows(&file).len(), 1);
        assert_eq!(rows(&file)[0].0, "");

        let missing = sent(&root, "no-such-entry", UsageOptions::default());
        match missing.as_slice() {
            [du_reply::Reply::Failure(failure)] => {
                assert_eq!(failure.relative_path, "");
                assert_eq!(failure.action, "cannot access");
                assert!(
                    !failure.reason.contains("os error") && !failure.reason.is_empty(),
                    "the system's words alone: {}",
                    failure.reason
                );
            }
            other => panic!("one failure for the start, got {other:?}"),
        }
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

        let space_error =
            filesystem_space_for_path(&missing).expect_err("a path that is not there");
        assert_eq!(space_error.code(), Code::NotFound);
        assert!(
            !space_error.message().contains(&*missing.to_string_lossy())
                && !space_error.message().contains("os error"),
            "the system's words alone, and no path of this daemon's: {}",
            space_error.message()
        );
        // The filesystem that holds a path that IS there has a size.
        let space = filesystem_space_for_path(temp.path()).expect("the temp folder's filesystem");
        assert!(space.total_bytes > 0 && space.used_bytes <= space.total_bytes);
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
            UsageOptions::default(),
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
