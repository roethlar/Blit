//! local-copy-mode: a copy between two paths on this machine gives every
//! file its source's mode, however the file travels. A file copied on its
//! own — a large one, or a tree of one file — was left with the mode it
//! was created with (644 under the usual mask): a private file readable
//! by all, a program no longer one. Small files among others, and every
//! file to or from a daemon, kept theirs; `cp` and `rsync` keep them.
//! And with `--resume` (cr-ecm16-1), whose copy had a path of its own.

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
    // cr-ecm16-1: `--resume` too, which left a file copied on its own
    // 644.
    for (at, options) in [
        &[][..],
        &["--ignore-existing"],
        &["--checksum"],
        &["--resume"],
    ]
    .into_iter()
    .enumerate()
    {
        let landed = ctx.workspace.join(format!("landed{at}"));
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

/// cr-ecm29-7: a resume where there is no `/proc` — as for a daemon in a
/// namespace without one — still replaces a destination it may not reuse.
/// What the exchange swapped out was looked at through `/proc/self/fd`;
/// with nothing there it was taken for something else, put back, and the
/// copy failed. Run in a user and mount namespace with an empty `/proc`;
/// where the system grants none (`unshare` refused), there is nothing to
/// run, and the test says so.
#[cfg(target_os = "linux")]
#[test]
fn a_resume_without_proc_replaces_what_it_may_not_reuse() {
    let probe = Command::new("unshare")
        .args(["--user", "--map-root-user", "--mount", "sh", "-c"])
        .arg("mount -t tmpfs none /proc && test ! -e /proc/self")
        .output();
    if !probe.as_ref().is_ok_and(|probe| probe.status.success()) {
        eprintln!("SKIPPED: no user and mount namespace here to hide /proc in: {probe:?}");
        return;
    }
    let ctx = TestContext::new();
    let source = ctx.workspace.join("source");
    let target = ctx.workspace.join("target");
    fs::create_dir_all(&source).unwrap();
    fs::create_dir_all(&target).unwrap();
    let data: Vec<u8> = (0..300 * 1024).map(|at| (at % 251) as u8).collect();
    fs::write(source.join("big.bin"), &data).unwrap();
    fs::set_permissions(source.join("big.bin"), fs::Permissions::from_mode(0o600)).unwrap();
    // Wider than its source: not reused, a fresh partial replaces it.
    fs::write(target.join("big.bin"), &data[..1000]).unwrap();
    fs::set_permissions(target.join("big.bin"), fs::Permissions::from_mode(0o644)).unwrap();
    let mut cmd = Command::new("unshare");
    cmd.args(["--user", "--map-root-user", "--mount", "sh", "-c"])
        .arg("mount -t tmpfs none /proc && exec \"$@\"")
        .arg("sh")
        .arg(&ctx.cli_bin)
        .arg("--config-dir")
        .arg(&ctx.config_dir)
        .arg("copy")
        .arg("--resume")
        .arg(format!("{}/", source.display()))
        .arg(format!("{}/", target.display()));
    let output = run_with_timeout(cmd, Duration::from_secs(120));
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(fs::read(target.join("big.bin")).unwrap(), data);
    let left: Vec<_> = fs::read_dir(&target).unwrap().collect();
    assert_eq!(left.len(), 1, "nothing left beside it");
}

/// cr-ecm29-5, cr-ecm30-3: a resume under a narrow mask — 0200, and 0400,
/// which takes away the owner's own reading — still makes its fresh
/// partial, in place, and the copy completes. Made in a staging directory
/// of its own, the mask left that directory unusable and the copy failed
/// (exit 2), leaving the directory behind.
#[test]
fn a_resume_under_a_narrow_mask_makes_its_fresh_partial() {
    use std::os::unix::process::CommandExt as _;
    for mask in [0o200u32, 0o400] {
        let ctx = TestContext::new();
        let source = ctx.workspace.join("source");
        let target = ctx.workspace.join("target");
        fs::create_dir_all(&source).unwrap();
        fs::create_dir_all(&target).unwrap();
        let data: Vec<u8> = (0..300 * 1024).map(|at| (at % 251) as u8).collect();
        fs::write(source.join("big.bin"), &data).unwrap();
        fs::set_permissions(source.join("big.bin"), fs::Permissions::from_mode(0o600)).unwrap();
        // Wider than its source: not reused, a fresh partial is made for it.
        fs::write(target.join("big.bin"), &data[..1000]).unwrap();
        fs::set_permissions(target.join("big.bin"), fs::Permissions::from_mode(0o644)).unwrap();
        let mut cmd = Command::new(&ctx.cli_bin);
        cmd.arg("--config-dir")
            .arg(&ctx.config_dir)
            .arg("copy")
            .arg("--resume")
            .arg(format!("{}/", source.display()))
            .arg(format!("{}/", target.display()));
        // SAFETY: the closure runs in the child before it executes blit, and
        // calls only `umask`, which is safe to call there.
        unsafe {
            cmd.pre_exec(move || {
                libc::umask(mask as libc::mode_t);
                Ok(())
            });
        }
        let output = run_with_timeout(cmd, Duration::from_secs(120));
        assert!(
            output.status.success(),
            "mask {mask:o}: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        assert_eq!(
            fs::read(target.join("big.bin")).unwrap(),
            data,
            "mask {mask:o}"
        );
        assert_eq!(mode_of(&target.join("big.bin")), 0o600, "mask {mask:o}");
        let left: Vec<_> = fs::read_dir(&target).unwrap().collect();
        assert_eq!(left.len(), 1, "mask {mask:o}: nothing left beside it");
    }
}

/// cr-ecm22-1, D-2026-11-23-1: a resume whose fresh partial cannot be made
/// ready — here a file-size limit refuses its length — fails, and leaves
/// nothing at the name that looks finished: the destination it could not
/// reuse is removed to make the partial in place, and the partial is
/// removed in turn, for a retry to land. Left, an empty file stood there.
#[test]
fn a_resume_that_cannot_ready_its_partial_leaves_nothing_that_looks_finished() {
    use std::os::unix::process::CommandExt as _;
    let ctx = TestContext::new();
    let source = ctx.workspace.join("source");
    let target = ctx.workspace.join("target");
    fs::create_dir_all(&source).unwrap();
    fs::create_dir_all(&target).unwrap();
    fs::write(source.join("big.bin"), vec![b'x'; 300 * 1024]).unwrap();
    fs::set_permissions(source.join("big.bin"), fs::Permissions::from_mode(0o600)).unwrap();
    fs::write(target.join("big.bin"), b"eighteen bytes ok!").unwrap();
    fs::set_permissions(target.join("big.bin"), fs::Permissions::from_mode(0o644)).unwrap();
    let mut cmd = Command::new(&ctx.cli_bin);
    cmd.arg("--config-dir")
        .arg(&ctx.config_dir)
        .arg("copy")
        .arg("--resume")
        .arg(format!("{}/", source.display()))
        .arg(format!("{}/", target.display()));
    // SAFETY: the closure runs in the child before it executes blit, and
    // calls only `setrlimit` and `signal`, which are safe to call there.
    unsafe {
        cmd.pre_exec(|| {
            let limit = libc::rlimit {
                rlim_cur: 256 * 1024,
                rlim_max: 256 * 1024,
            };
            if libc::setrlimit(libc::RLIMIT_FSIZE, &limit) != 0 {
                return Err(std::io::Error::last_os_error());
            }
            libc::signal(libc::SIGXFSZ, libc::SIG_IGN);
            Ok(())
        });
    }
    let output = run_with_timeout(cmd, Duration::from_secs(120));
    assert!(!output.status.success(), "the limit refuses the copy");
    assert!(
        !target.join("big.bin").exists(),
        "nothing at the name: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(
        fs::read_dir(&target).unwrap().count(),
        0,
        "nothing beside it"
    );
}
