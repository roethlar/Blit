//! Mirror-deletion pass for the engine's streaming strategy.
//! Moved verbatim from `orchestrator/orchestrator.rs` at ue-r2-1c.

use std::collections::HashSet;
use std::path::Path;

use eyre::{bail, Result};

use crate::fs_enum::FileFilter;

/// Delete destination files/dirs not present in the source header set.
///
/// R58-F6: `delete_scope` controls which destination entries are
/// even considered for deletion:
///   - `FilteredSubset` (default): enumerate the destination
///     *through the user's filter*, then delete entries not in
///     the source set. Excluded files (e.g. `*.log` when
///     `--exclude '*.log'`) are out of scope — they're not
///     candidates for deletion, and their parent directories are
///     therefore non-empty from the user's perspective. When
///     `remove_dir` fails with ENOTEMPTY on a parent whose only
///     remaining contents are out-of-scope, we treat it as
///     expected, not as an error.
///   - `All`: enumerate the destination *without* the filter so
///     every entry is in scope. ENOTEMPTY is a genuine error
///     here (we did walk everything, so something other than
///     filter-excluded content must be in the way).
pub(super) fn apply_mirror_deletions(
    source_paths: &HashSet<String>,
    dest_root: &Path,
    filter: &FileFilter,
    delete_scope: crate::orchestrator::LocalMirrorDeleteScope,
    perform: bool,
    verbose: bool,
) -> Result<(usize, usize)> {
    use crate::enumeration::{EntryKind, FileEnumerator};
    use crate::orchestrator::LocalMirrorDeleteScope;

    // R58-F6: FilteredSubset uses the user's filter for the
    // enumeration (only in-scope entries become deletion
    // candidates). All bypasses the filter so every destination
    // entry is considered.
    let enum_filter = match delete_scope {
        LocalMirrorDeleteScope::FilteredSubset => filter.clone_without_cache(),
        LocalMirrorDeleteScope::All => FileFilter::default(),
    };
    let enumerator = FileEnumerator::new(enum_filter);
    let dest_entries = enumerator.enumerate_local(dest_root)?;

    // R48-F1: source.scan() only emits file headers, so
    // `source_paths` is a set of *files*. Pre-fix this meant every
    // destination directory was "not in source_paths" and got
    // queued for deletion. Combined with R46-F5's hard-error
    // policy on remove_* failures, a normal mirror containing
    // `sub/file.txt` would keep `sub/file.txt`, then try
    // `remove_dir("sub")` and fail the whole operation with
    // ENOTEMPTY. Derive `source_dirs` from each file's parent
    // chain so dest dirs that exist implicitly on the source
    // side (because they contain a source file) get preserved.
    let mut source_dirs: HashSet<String> = HashSet::new();
    for path in source_paths {
        let p = std::path::Path::new(path);
        let mut cur = p.parent();
        while let Some(parent) = cur {
            if parent.as_os_str().is_empty() {
                break;
            }
            let parent_str = crate::path_posix::relative_path_to_posix(parent);
            // Insert and keep walking up; if already present every
            // shallower ancestor is too, so we could break — but
            // the walk is cheap and the eager form is simpler to
            // reason about.
            source_dirs.insert(parent_str);
            cur = parent.parent();
        }
    }

    let mut files_to_delete = Vec::new();
    let mut dirs_to_delete = Vec::new();

    for entry in &dest_entries {
        let rel = crate::path_posix::relative_path_to_posix(&entry.relative_path);
        let absent_at_source = match entry.kind {
            EntryKind::Directory => !source_dirs.contains(&rel),
            _ => !source_paths.contains(&rel),
        };
        if absent_at_source {
            let abs = dest_root.join(&entry.relative_path);
            match entry.kind {
                EntryKind::Directory => dirs_to_delete.push(abs),
                _ => files_to_delete.push(abs),
            }
        }
    }

    // Sort dirs deepest-first so children are deleted before parents.
    dirs_to_delete.sort_by_key(|b| std::cmp::Reverse(b.components().count()));

    let mut deleted_files = 0usize;
    let mut deleted_dirs = 0usize;
    // R46-F5: collect deletion failures and bail at the end. Pre-fix
    // each `remove_file` / `remove_dir` error was printed as a
    // warning and the function returned Ok, so a mirror could
    // succeed-on-paper while leaving stale destination content
    // behind. Now we still attempt every deletion (better partial
    // progress than abort-on-first-failure), but we bail with an
    // aggregated error if any failed — the caller's mirror operation
    // returns Err, the user sees the failed entries, and the summary
    // line doesn't claim "complete".
    let mut failures: Vec<String> = Vec::new();

    for path in files_to_delete {
        #[cfg(windows)]
        crate::win_fs::clear_readonly_recursive(&path);

        if perform {
            match std::fs::remove_file(&path) {
                Ok(_) => {
                    deleted_files += 1;
                    if verbose {
                        eprintln!("Deleted file: {}", path.display());
                    }
                }
                Err(err) => {
                    eprintln!("Failed to delete file {}: {}", path.display(), err);
                    failures.push(format!("{}: {}", path.display(), err));
                }
            }
        } else {
            deleted_files += 1;
        }
    }

    for path in dirs_to_delete {
        #[cfg(windows)]
        crate::win_fs::clear_readonly_recursive(&path);

        if perform {
            match std::fs::remove_dir(&path) {
                Ok(_) => {
                    deleted_dirs += 1;
                    if verbose {
                        eprintln!("Deleted directory: {}", path.display());
                    }
                }
                Err(err) => {
                    // R58-F6: in FilteredSubset mode, ENOTEMPTY on
                    // a destination dir means the dir contains
                    // out-of-scope content (files matching the
                    // user's exclude rules). Those files
                    // intentionally aren't candidates for
                    // deletion, so the dir genuinely can't be
                    // empty — that's not a failure, it's the
                    // expected behavior of the scope contract.
                    // Skip silently in that case; surface the
                    // error in `All` mode where the dir really
                    // should have been empty.
                    let is_not_empty = err.kind() == std::io::ErrorKind::DirectoryNotEmpty
                        || err.raw_os_error() == Some(66); // ENOTEMPTY on macOS/BSD
                    if matches!(delete_scope, LocalMirrorDeleteScope::FilteredSubset)
                        && is_not_empty
                    {
                        if verbose {
                            eprintln!(
                                "Kept directory {} (contains out-of-scope contents)",
                                path.display()
                            );
                        }
                    } else {
                        eprintln!("Failed to delete directory {}: {}", path.display(), err);
                        failures.push(format!("{}: {}", path.display(), err));
                    }
                }
            }
        } else {
            deleted_dirs += 1;
        }
    }

    if !failures.is_empty() {
        let preview = failures
            .iter()
            .take(5)
            .cloned()
            .collect::<Vec<_>>()
            .join("; ");
        bail!(
            "mirror-delete left {} entr{} in place at {} ({} succeeded): {}",
            failures.len(),
            if failures.len() == 1 { "y" } else { "ies" },
            dest_root.display(),
            deleted_files + deleted_dirs,
            preview
        );
    }

    Ok((deleted_files, deleted_dirs))
}
