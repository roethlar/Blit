use std::fs;
use std::path::Path;

#[test]
fn abandoned_foundation_paths_stay_deleted() {
    let source_root = Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
    for relative in [
        "tar_stream.rs",
        "delete.rs",
        "copy/parallel.rs",
        "copy/stats.rs",
        "copy/file_copy/chunked.rs",
        "logger.rs",
    ] {
        assert!(
            !source_root.join(relative).exists(),
            "retired foundation path returned: {relative}"
        );
    }

    let public_surface = [
        "lib.rs",
        "copy/mod.rs",
        "copy/file_copy/mod.rs",
        "fs_enum.rs",
    ]
    .into_iter()
    .map(|relative| {
        fs::read_to_string(source_root.join(relative))
            .unwrap_or_else(|err| panic!("read {relative}: {err}"))
    })
    .collect::<Vec<_>>()
    .join("\n");

    for retired_name in [
        "pub mod delete;",
        "pub mod tar_stream;",
        "chunked_copy_file",
        "parallel_copy_files",
        "CopyStats",
        "SymlinkEntry",
        "enumerate_symlinks",
        "categorize_files",
        "enumerate_directory_deref_filtered",
        "pub mod logger;",
        "NoopLogger",
        "TextLogger",
        "&dyn Logger",
    ] {
        assert!(
            !public_surface.contains(retired_name),
            "retired foundation export returned: {retired_name}"
        );
    }
}
