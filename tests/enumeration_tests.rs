use anyhow::Result;
use blit_core::enumeration::{EntryKind, FileEnumerator};
use blit_core::fs_enum::FileFilter;

#[test]
fn enumerator_returns_directories_and_files() -> Result<()> {
    let temp = tempfile::tempdir()?;
    let base = temp.path();
    std::fs::create_dir_all(base.join("dir"))?;
    std::fs::write(base.join("dir/file.txt"), b"hello")?;

    let filter = FileFilter::default();
    let entries = FileEnumerator::new(filter).enumerate_local(base)?;

    let mut saw_dir = false;
    let mut saw_file = false;

    for entry in entries {
        match entry.kind {
            EntryKind::Directory => {
                if entry.relative_path.as_os_str() != "." {
                    saw_dir = true;
                }
            }
            EntryKind::File { .. } => {
                saw_file = true;
                assert_eq!(entry.relative_path, std::path::Path::new("dir/file.txt"));
            }
            EntryKind::Symlink { .. } => {}
        }
    }

    assert!(saw_dir, "expected to encounter the directory entry");
    assert!(saw_file, "expected to encounter the file entry");

    Ok(())
}

#[test]
fn enumerator_respects_excludes() -> Result<()> {
    let temp = tempfile::tempdir()?;
    let base = temp.path();
    std::fs::create_dir_all(base.join("logs"))?;
    std::fs::write(base.join("logs/app.log"), b"log")?;
    std::fs::write(base.join("data.txt"), b"data")?;

    let mut filter = FileFilter::default();
    filter.exclude_dirs.push("logs".into());
    filter.exclude_files.push("*.tmp".into());

    let entries = FileEnumerator::new(filter).enumerate_local(base)?;
    let mut rels = entries
        .into_iter()
        .filter_map(|e| match e.kind {
            EntryKind::File { .. } => Some(e.relative_path),
            EntryKind::Directory => Some(e.relative_path),
            EntryKind::Symlink { .. } => None,
        })
        .collect::<Vec<_>>();
    rels.sort();

    assert!(rels.iter().any(|p| p == std::path::Path::new("data.txt")));
    assert!(rels.iter().all(|p| !p.starts_with("logs")));

    Ok(())
}

#[cfg(unix)]
#[test]
fn enumerator_reports_symlinks_when_requested() -> Result<()> {
    use std::os::unix::fs::symlink;

    let temp = tempfile::tempdir()?;
    let base = temp.path();
    std::fs::write(base.join("source.txt"), b"data")?;
    symlink(base.join("source.txt"), base.join("link.txt"))?;

    let filter = FileFilter::default();
    let entries = FileEnumerator::new(filter)
        .include_symlinks(true)
        .enumerate_local(base)?;

    assert!(entries
        .iter()
        .any(|e| matches!(e.kind, EntryKind::Symlink { .. })));

    Ok(())
}
