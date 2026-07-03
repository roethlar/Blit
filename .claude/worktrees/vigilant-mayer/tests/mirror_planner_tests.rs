use eyre::Result;
use blit_core::checksum::ChecksumType;
use blit_core::enumeration::{EntryKind, FileEnumerator};
use blit_core::fs_enum::{self, CopyJob, FileEntry};
use blit_core::mirror_planner::{MirrorPlanner, RemoteEntryState};
use filetime::{set_file_mtime, FileTime};
use std::collections::{HashMap, HashSet};
use std::fs;
use std::thread;
use std::time::Duration;
use std::time::UNIX_EPOCH;

#[test]
fn planner_skips_identical_files() -> Result<()> {
    let temp = tempfile::tempdir()?;
    let src_root = temp.path().join("src");
    let dest_root = temp.path().join("dest");
    fs::create_dir_all(&src_root)?;
    fs::create_dir_all(&dest_root)?;

    let src_file = src_root.join("file.txt");
    fs::write(&src_file, b"hello")?;
    let dest_file = dest_root.join("file.txt");
    fs::write(&dest_file, b"hello")?;

    // Ensure timestamps allow skip (dest newer or equal).
    thread::sleep(Duration::from_millis(10));

    let entry = FileEntry {
        path: src_file.clone(),
        size: fs::metadata(&src_file)?.len(),
        is_directory: false,
    };
    let job = CopyJob { entry };

    let planner = MirrorPlanner::new(false);
    assert!(!planner.should_copy_entry(&job, &src_root, &dest_root));

    Ok(())
}

#[test]
fn planner_marks_changed_files() -> Result<()> {
    let temp = tempfile::tempdir()?;
    let src_root = temp.path().join("src");
    let dest_root = temp.path().join("dest");
    fs::create_dir_all(&src_root)?;
    fs::create_dir_all(&dest_root)?;

    let src_file = src_root.join("file.txt");
    fs::write(&src_file, b"hello world")?;
    let dest_file = dest_root.join("file.txt");
    fs::write(&dest_file, b"old")?;

    let entry = FileEntry {
        path: src_file.clone(),
        size: fs::metadata(&src_file)?.len(),
        is_directory: false,
    };
    let job = CopyJob { entry };

    let planner = MirrorPlanner::new(false);
    assert!(planner.should_copy_entry(&job, &src_root, &dest_root));

    Ok(())
}

#[test]
fn planner_keeps_directories() -> Result<()> {
    let temp = tempfile::tempdir()?;
    let src_root = temp.path().join("src");
    let dest_root = temp.path().join("dest");
    fs::create_dir_all(&src_root)?;
    fs::create_dir_all(&dest_root)?;

    let entry = FileEntry {
        path: src_root.join("dir"),
        size: 0,
        is_directory: true,
    };
    let job = CopyJob { entry };

    let planner = MirrorPlanner::new(false);
    assert!(planner.should_copy_entry(&job, &src_root, &dest_root));

    Ok(())
}

#[test]
fn planner_local_deletions_detects_extras() -> Result<()> {
    let temp = tempfile::tempdir()?;
    let src_root = temp.path().join("src");
    let dest_root = temp.path().join("dest");
    fs::create_dir_all(&src_root)?;
    fs::create_dir_all(&dest_root)?;

    fs::write(src_root.join("keep.txt"), b"data")?;
    fs::write(dest_root.join("keep.txt"), b"data")?;
    fs::write(dest_root.join("extra.txt"), b"old")?;

    let planner = MirrorPlanner::new(false);
    let filter = fs_enum::FileFilter::default();
    let plan = planner.plan_local_deletions(&src_root, &dest_root, &filter)?;

    assert!(plan.files.iter().any(|p| p.ends_with("extra.txt")));

    Ok(())
}

#[test]
fn planner_remote_deletions_detects_extras() -> Result<()> {
    let temp = tempfile::tempdir()?;
    let src_root = temp.path().join("src");
    let dest_root = temp.path().join("dest");
    fs::create_dir_all(&src_root)?;
    fs::create_dir_all(&dest_root)?;

    fs::write(src_root.join("keep.txt"), b"data")?;
    fs::write(dest_root.join("keep.txt"), b"data")?;
    fs::write(dest_root.join("extra.txt"), b"old")?;

    let enumerator = FileEnumerator::new(fs_enum::FileFilter::default());
    let source_entries = enumerator.enumerate_local(&src_root)?;
    let mut remote_entries = Vec::new();
    for entry in enumerator.enumerate_local(&dest_root)? {
        if let Some(file_entry) = entry.into_file_entry() {
            remote_entries.push(file_entry);
        }
    }

    let planner = MirrorPlanner::new(false);
    let plan = planner.plan_remote_deletions(&source_entries, &dest_root, &remote_entries);

    assert!(plan.files.iter().any(|p| p.ends_with("extra.txt")));

    Ok(())
}

#[test]
fn planner_expected_deletions_detects_extras() -> Result<()> {
    let temp = tempfile::tempdir()?;
    let dest_root = temp.path().join("dest");
    std::fs::create_dir_all(&dest_root)?;
    std::fs::write(dest_root.join("keep.txt"), b"data")?;
    std::fs::write(dest_root.join("extra.txt"), b"old")?;

    let mut expected = std::collections::HashSet::new();
    expected.insert(dest_root.join("keep.txt"));

    let planner = MirrorPlanner::new(false);
    let plan = planner.plan_expected_deletions(&dest_root, &expected)?;
    assert!(plan.files.iter().any(|p| p.ends_with("extra.txt")));
    Ok(())
}

fn first_file_entry(
    entries: Vec<blit_core::enumeration::EnumeratedEntry>,
) -> blit_core::enumeration::EnumeratedEntry {
    entries
        .into_iter()
        .find(|e| matches!(e.kind, EntryKind::File { .. }))
        .expect("expected at least one file entry")
}

#[test]
fn planner_remote_missing_requires_copy() -> Result<()> {
    let temp = tempfile::tempdir()?;
    let src_root = temp.path().join("src");
    fs::create_dir_all(&src_root)?;
    let file = src_root.join("file.txt");
    fs::write(&file, b"hello")?;

    let enumerator = FileEnumerator::new(fs_enum::FileFilter::default());
    let entry = first_file_entry(enumerator.enumerate_local(&src_root)?);

    let planner = MirrorPlanner::new(false);
    assert!(planner.should_copy_remote_entry(&entry, None));
    Ok(())
}

#[test]
fn planner_remote_identical_skips_copy() -> Result<()> {
    let temp = tempfile::tempdir()?;
    let src_root = temp.path().join("src");
    fs::create_dir_all(&src_root)?;
    let file = src_root.join("file.txt");
    fs::write(&file, b"hello world")?;

    let enumerator = FileEnumerator::new(fs_enum::FileFilter::default());
    let entry = first_file_entry(enumerator.enumerate_local(&src_root)?);

    let md = fs::metadata(&file)?;
    let secs = md.modified()?.duration_since(UNIX_EPOCH).unwrap().as_secs() as i64;
    let remote = RemoteEntryState {
        size: md.len(),
        mtime: secs,
        hash: None,
    };

    let planner = MirrorPlanner::new(false);
    assert!(!planner.should_copy_remote_entry(&entry, Some(&remote)));
    Ok(())
}

#[test]
fn planner_remote_mtime_delta_triggers_copy() -> Result<()> {
    let temp = tempfile::tempdir()?;
    let src_root = temp.path().join("src");
    fs::create_dir_all(&src_root)?;
    let file = src_root.join("file.txt");
    fs::write(&file, b"hello world")?;

    let enumerator = FileEnumerator::new(fs_enum::FileFilter::default());
    let entry = first_file_entry(enumerator.enumerate_local(&src_root)?);

    let md = fs::metadata(&file)?;
    let secs = md.modified()?.duration_since(UNIX_EPOCH).unwrap().as_secs() as i64;
    let remote = RemoteEntryState {
        size: md.len(),
        mtime: secs - 10,
        hash: None,
    };

    let planner = MirrorPlanner::new(false);
    assert!(planner.should_copy_remote_entry(&entry, Some(&remote)));
    Ok(())
}

#[test]
fn planner_remote_checksum_mismatch_triggers_copy() -> Result<()> {
    let temp = tempfile::tempdir()?;
    let src_root = temp.path().join("src");
    fs::create_dir_all(&src_root)?;
    let file = src_root.join("file.txt");
    fs::write(&file, b"checksum")?;

    let enumerator = FileEnumerator::new(fs_enum::FileFilter::default());
    let entry = first_file_entry(enumerator.enumerate_local(&src_root)?);

    let md = fs::metadata(&file)?;
    let secs = md.modified()?.duration_since(UNIX_EPOCH).unwrap().as_secs() as i64;
    let remote = RemoteEntryState {
        size: md.len(),
        mtime: secs,
        hash: Some(vec![0u8; 32]),
    };

    let planner = MirrorPlanner::new(true);
    assert!(planner.should_copy_remote_entry(&entry, Some(&remote)));

    let mut remote_same = remote;
    remote_same.hash = Some(blit_core::checksum::hash_file(&file, ChecksumType::Blake3)?);
    assert!(!planner.should_copy_remote_entry(&entry, Some(&remote_same)));
    Ok(())
}

#[test]
fn planner_fetch_remote_file_when_dest_missing() -> Result<()> {
    let temp = tempfile::tempdir()?;
    let dest_root = temp.path().join("dest");
    fs::create_dir_all(&dest_root)?;

    let dest_path = dest_root.join("file.txt");
    let remote = RemoteEntryState {
        size: 5,
        mtime: 0,
        hash: None,
    };
    let planner = MirrorPlanner::new(false);
    assert!(planner.should_fetch_remote_file(&dest_path, &remote));
    Ok(())
}

#[test]
fn planner_skips_fetch_when_dest_matches_remote() -> Result<()> {
    let temp = tempfile::tempdir()?;
    let dest_root = temp.path().join("dest");
    fs::create_dir_all(&dest_root)?;

    let dest_path = dest_root.join("file.txt");
    fs::write(&dest_path, b"hello world")?;
    let metadata = fs::metadata(&dest_path)?;
    let mtime = metadata
        .modified()?        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs() as i64;

    let remote = RemoteEntryState {
        size: metadata.len(),
        mtime,
        hash: None,
    };
    let planner = MirrorPlanner::new(false);
    assert!(!planner.should_fetch_remote_file(&dest_path, &remote));
    Ok(())
}

#[test]
fn planner_fetches_when_size_differs() -> Result<()> {
    let temp = tempfile::tempdir()?;
    let dest_root = temp.path().join("dest");
    fs::create_dir_all(&dest_root)?;

    let dest_path = dest_root.join("file.txt");
    fs::write(&dest_path, b"hello")?;
    let remote = RemoteEntryState {
        size: 999,
        mtime: 0,
        hash: None,
    };
    let planner = MirrorPlanner::new(false);
    assert!(planner.should_fetch_remote_file(&dest_path, &remote));
    Ok(())
}

#[test]
fn planner_checksum_fetch_logic() -> Result<()> {
    let temp = tempfile::tempdir()?;
    let dest_root = temp.path().join("dest");
    fs::create_dir_all(&dest_root)?;

    let dest_path = dest_root.join("file.txt");
    fs::write(&dest_path, b"checksum data")?;
    let metadata = fs::metadata(&dest_path)?;
    let mtime = metadata
        .modified()?        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs() as i64;

    let remote_mismatch = RemoteEntryState {
        size: metadata.len(),
        mtime,
        hash: Some(vec![0u8; 32]),
    };
    let planner = MirrorPlanner::new(true);
    assert!(planner.should_fetch_remote_file(&dest_path, &remote_mismatch));

    let mut remote_match = remote_mismatch;
    remote_match.hash = Some(blit_core::checksum::hash_file(&dest_path, ChecksumType::Blake3)?);
    assert!(!planner.should_fetch_remote_file(&dest_path, &remote_match));
    Ok(())
}

fn normalize_path(path: &std::path::Path) -> String {
    let replaced = path.to_string_lossy().replace('\\', "/");
    #[cfg(windows)]
    {
        replaced.to_ascii_lowercase()
    }
    #[cfg(not(windows))] 
    {
        replaced
    }
}

#[cfg(windows)]
#[test]
fn planner_remote_deletions_case_insensitive_windows() -> Result<()> {
    use blit_core::enumeration::FileEnumerator;
    use blit_core::fs_enum::FileFilter;

    let temp = tempfile::tempdir()?;
    let src_root = temp.path().join("src");
    let dest_root = temp.path().join("dest");
    fs::create_dir_all(&src_root)?;
    fs::create_dir_all(&dest_root)?;

    fs::write(src_root.join("file.txt"), b"data")?;
    fs::write(dest_root.join("FILE.TXT"), b"data")?;
    fs::write(dest_root.join("extra.txt"), b"old")?;

    let enumerator = FileEnumerator::new(FileFilter::default());
    let source_entries = enumerator.enumerate_local(&src_root)?;
    let dest_entries = enumerator.enumerate_local(&dest_root)?;
    let mut remote_entries = Vec::new();
    for entry in dest_entries {
        if let Some(file_entry) = entry.into_file_entry() {
            remote_entries.push(file_entry);
        }
    }

    let planner = MirrorPlanner::new(false);
    let plan = planner.plan_remote_deletions(&source_entries, &dest_root, &remote_entries);
    assert!(
        plan.files.iter().any(|p| p.ends_with("extra.txt")),
        "expected extra.txt to be scheduled for deletion"
    );
    assert!(
        !plan
            .files
            .iter()
            .any(|p| p.file_name().map(|n| n == "FILE.TXT").unwrap_or(false)),
        "FILE.TXT should not be deleted because source has file.txt"
    );
    Ok(())
}

#[test]
fn planner_mirror_parity_across_modes() -> Result<()> {
    let temp = tempfile::tempdir()?;
    let src_root = temp.path().join("src");
    let dest_root = temp.path().join("dest");
    fs::create_dir_all(&src_root)?;
    fs::create_dir_all(&dest_root)?;

    // Create identical file that should be skipped
    let keep_src = src_root.join("keep.txt");
    fs::write(&keep_src, b"same-content")?;
    let keep_dest = dest_root.join("keep.txt");
    fs::write(&keep_dest, b"same-content")?;
    let keep_time = FileTime::from_unix_time(1_700_000_000, 0);
    set_file_mtime(&keep_src, keep_time)?;
    set_file_mtime(&keep_dest, keep_time)?;

    // Create file that changed (size+mtime)
    let update_src = src_root.join("subdir").join("update.txt");
    fs::create_dir_all(update_src.parent().unwrap())?;
    fs::write(&update_src, b"new content that is longer")?;
    let update_dest = dest_root.join("subdir").join("update.txt");
    fs::create_dir_all(update_dest.parent().unwrap())?;
    fs::write(&update_dest, b"old")?;
    set_file_mtime(&update_src, FileTime::from_unix_time(1_700_000_020, 0))?;
    set_file_mtime(&update_dest, FileTime::from_unix_time(1_700_000_000, 0))?;

    // Create file only in source
    let new_src = src_root.join("only_in_source.bin");
    fs::write(&new_src, b"brand new")?;
    set_file_mtime(&new_src, FileTime::from_unix_time(1_700_000_030, 0))?;

    // Create extra file only in destination
    let extra_dest = dest_root.join("obsolete.log");
    fs::write(&extra_dest, b"remove me")?;

    let planner = MirrorPlanner::new(false);

    // Local mirror copy set
    let mut filter_local = fs_enum::FileFilter::default();
    let copy_entries = fs_enum::enumerate_directory_filtered(&src_root, &mut filter_local)?;
    let copy_jobs: Vec<CopyJob> = copy_entries
        .into_iter()
        .map(|entry| CopyJob { entry })
        .collect();
    let local_copy: HashSet<String> = copy_jobs
        .iter()
        .filter_map(|job| {
            if !job.entry.is_directory && planner.should_copy_entry(job, &src_root, &dest_root) {
                let rel = job.entry.path.strip_prefix(&src_root).unwrap();
                Some(normalize_path(rel))
            } else {
                None
            }
        })
        .collect();

    // Remote push copy set
    let enumerator = FileEnumerator::new(fs_enum::FileFilter::default());
    let source_entries = enumerator.enumerate_local(&src_root)?;

    let mut dest_filter = fs_enum::FileFilter::default();
    let dest_file_entries =
        fs_enum::enumerate_directory_filtered(&dest_root, &mut dest_filter)?;
    let mut remote_states: HashMap<String, RemoteEntryState> = HashMap::new();
    let mut dest_rel_paths: HashMap<String, std::path::PathBuf> = HashMap::new();
    for entry in &dest_file_entries {
        if entry.is_directory {
            continue;
        }
        let rel = entry.path.strip_prefix(&dest_root).unwrap().to_path_buf();
        let md = fs::metadata(&entry.path)?;
        let mtime = md
            .modified()?            .duration_since(UNIX_EPOCH)
            .ok()
            .map(|d| d.as_secs() as i64)
            .unwrap_or(0);
        let key = normalize_path(&rel);
        dest_rel_paths.insert(key.clone(), rel);
        remote_states.insert(
            key,
            RemoteEntryState {
                size: entry.size,
                mtime,
                hash: None,
            },
        );
    }

    let push_copy: HashSet<String> = source_entries
        .iter()
        .filter_map(|entry| {
            if let EntryKind::File { .. } = &entry.kind {
                let key = normalize_path(&entry.relative_path);
                let state = remote_states.get(&key);
                if planner.should_copy_remote_entry(entry, state) {
                    Some(key)
                } else {
                    None
                }
            } else {
                None
            }
        })
        .collect();

    // Remote pull fetch set
    let mut pull_states: HashMap<String, (std::path::PathBuf, RemoteEntryState)> = HashMap::new();
    for entry in &source_entries {
        if let EntryKind::File { size } = entry.kind {
            let key = normalize_path(&entry.relative_path);
            let mtime = entry
                .metadata
                .modified()
                .ok()
                .and_then(|m| m.duration_since(UNIX_EPOCH).ok())
                .map(|d| d.as_secs() as i64)
                .unwrap_or(0);
            pull_states.insert(
                key.clone(),
                (
                    entry.relative_path.clone(),
                    RemoteEntryState {
                        size,
                        mtime,
                        hash: None,
                    },
                ),
            );
        }
    }

    let pull_fetch: HashSet<String> = pull_states
        .iter()
        .filter_map(|(key, (rel_path, state))| {
            let rel = dest_rel_paths
                .get(key)
                .cloned()
                .unwrap_or_else(|| rel_path.clone());
            let dest_path = dest_root.join(rel);
            if planner.should_fetch_remote_file(&dest_path, state) {
                Some(key.clone())
            } else {
                None
            }
        })
        .collect();

    assert_eq!(local_copy, push_copy, "local vs push copy decisions differ");
    assert_eq!(
        local_copy, pull_fetch,
        "local vs pull fetch decisions differ"
    );

    // Compare deletion plans
    let filter_for_local_del = fs_enum::FileFilter::default();
    let local_del = planner.plan_local_deletions(&src_root, &dest_root, &filter_for_local_del)?;

    let remote_del = planner.plan_remote_deletions(&source_entries, &dest_root, &dest_file_entries);

    let local_del_files: HashSet<String> = local_del
        .files
        .iter()
        .map(|p| normalize_path(p.strip_prefix(&dest_root).unwrap()))
        .collect();
    let remote_del_files: HashSet<String> = remote_del
        .files
        .iter()
        .map(|p| normalize_path(p.strip_prefix(&dest_root).unwrap()))
        .collect();
    assert_eq!(
        local_del_files, remote_del_files,
        "deletion file sets differ"
    );

    let local_del_dirs: HashSet<String> = local_del
        .dirs
        .iter()
        .map(|p| normalize_path(p.strip_prefix(&dest_root).unwrap()))
        .collect();
    let remote_del_dirs: HashSet<String> = remote_del
        .dirs
        .iter()
        .map(|p| normalize_path(p.strip_prefix(&dest_root).unwrap()))
        .collect();
    assert_eq!(local_del_dirs, remote_del_dirs, "deletion dir sets differ");

    Ok(())
}
