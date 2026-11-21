//! local-copy-mode: a copy between two paths on this machine gives every
//! file its source's mode, however the file travels. A file copied on its
//! own — a large one, or a tree of one file — was left with the mode it
//! was created with (644 under the usual mask): a private file readable
//! by all, a program no longer one. Small files among others, and every
//! file to or from a daemon, kept theirs; `cp` and `rsync` keep them.

#![cfg(unix)]

use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::Path;
use std::process::Command;
use std::time::Duration;

mod common;
use common::{run_with_timeout, TestContext};

fn mode_of(path: &Path) -> u32 {
    fs::metadata(path)
        .unwrap_or_else(|error| panic!("{}: {error}", path.display()))
        .permissions()
        .mode()
        & 0o7777
}

fn copy(ctx: &TestContext, source: &Path, destination: &Path, options: &[&str]) {
    let mut cmd = Command::new(&ctx.cli_bin);
    cmd.arg("--config-dir")
        .arg(&ctx.config_dir)
        .arg("copy")
        .args(options)
        .arg(format!("{}/", source.display()))
        .arg(format!("{}/", destination.display()));
    let output = run_with_timeout(cmd, Duration::from_secs(120));
    assert!(
        output.status.success(),
        "{options:?}: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn a_local_copy_gives_every_file_its_sources_mode() {
    let ctx = TestContext::new();
    let tree = ctx.workspace.join("tree");
    let alone = ctx.workspace.join("alone");
    fs::create_dir_all(&tree).unwrap();
    fs::create_dir_all(&alone).unwrap();
    // Small files travel together; a large one is copied on its own, and
    // so is the only file of a tree.
    let files: [(&Path, &str, usize, u32); 5] = [
        (&tree, "small-private", 5, 0o600),
        (&tree, "small-group", 5, 0o640),
        (&tree, "large-private", 16 * 1024 * 1024, 0o600),
        (&tree, "large-program", 16 * 1024 * 1024, 0o750),
        (&alone, "private", 5, 0o600),
    ];
    for (root, name, bytes, mode) in files {
        fs::write(root.join(name), vec![b'x'; bytes]).unwrap();
        fs::set_permissions(root.join(name), fs::Permissions::from_mode(mode)).unwrap();
    }
    for options in [&[][..], &["--ignore-existing"], &["--checksum"]] {
        let landed = ctx.workspace.join(format!("landed{}", options.len()));
        let _ = fs::remove_dir_all(&landed);
        copy(&ctx, &tree, &landed.join("tree"), options);
        copy(&ctx, &alone, &landed.join("alone"), options);
        for (root, name, _, mode) in files {
            let at = landed.join(root.file_name().unwrap()).join(name);
            assert_eq!(
                mode_of(&at),
                mode,
                "{options:?}: {name} is {:o}, its source {mode:o}",
                mode_of(&at)
            );
        }
    }
}
