use std::fs;
use std::process::Command;
use std::time::Duration;

mod common;
use common::{run_with_timeout, TestContext};

#[test]
fn test_remote_move_local_to_remote() {
    let ctx = TestContext::new();
    let src_dir = ctx.workspace.join("src");
    fs::create_dir_all(&src_dir).expect("src dir");
    let src_file = src_dir.join("move_me.txt");
    fs::write(&src_file, "move content").expect("write file");

    // Source has trailing slash -> rsync "merge contents" semantics:
    // files under src/ land directly in the module root.
    let src_arg = format!("{}/", src_dir.display());
    let dest_remote = format!("127.0.0.1:{}:/test/", ctx.daemon_port);
    let mut cli_cmd = Command::new(&ctx.cli_bin);
    cli_cmd
        .arg("--config-dir")
        .arg(&ctx.config_dir)
        .arg("move")
        .arg("--yes")
        .arg(&src_arg)
        .arg(&dest_remote);

    let output = run_with_timeout(cli_cmd, Duration::from_secs(60));
    assert!(
        output.status.success(),
        "blit move failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    // Verify destination file exists at module root (merged).
    let dest_file = ctx.module_dir.join("move_me.txt");
    assert!(dest_file.exists(), "remote file missing");
    let bytes = fs::read(&dest_file).expect("read remote file");
    assert_eq!(bytes, b"move content");

    // Verify source is deleted after the move.
    assert!(!src_file.exists(), "source file should have been deleted");
    assert!(
        !src_dir.exists(),
        "source directory should have been deleted"
    );
}

/// review otp-10a F1: `blit move` to a remote destination must land the
/// source bytes even when the destination already holds a SAME-SIZE,
/// NEWER file — the copy-shaped SizeMtime compare would skip that cell,
/// and move's source-delete would then destroy the only copy of the
/// source content. The move verb pushes with IgnoreTimes (transfer
/// unconditionally), so the destination ends with the source bytes and
/// deleting the source is safe.
#[test]
fn move_lands_source_bytes_over_same_size_newer_destination() {
    let ctx = TestContext::new();
    let src_dir = ctx.workspace.join("src");
    fs::create_dir_all(&src_dir).expect("src dir");
    let src_file = src_dir.join("clash.txt");
    fs::write(&src_file, "source-bytes").expect("write source");

    // Same size, different content, newer mtime at the destination.
    let dest_file = ctx.module_dir.join("clash.txt");
    fs::write(&dest_file, "dest---bytes").expect("seed dest");
    let newer = filetime::FileTime::from_unix_time(
        filetime::FileTime::from_last_modification_time(
            &fs::metadata(&src_file).expect("src meta"),
        )
        .unix_seconds()
            + 60,
        0,
    );
    filetime::set_file_mtime(&dest_file, newer).expect("bump dest mtime");

    let src_arg = format!("{}/", src_dir.display());
    let dest_remote = format!("127.0.0.1:{}:/test/", ctx.daemon_port);
    let mut cli_cmd = Command::new(&ctx.cli_bin);
    cli_cmd
        .arg("--config-dir")
        .arg(&ctx.config_dir)
        .arg("move")
        .arg("--yes")
        .arg(&src_arg)
        .arg(&dest_remote);

    let output = run_with_timeout(cli_cmd, Duration::from_secs(60));
    assert!(
        output.status.success(),
        "blit move failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    assert_eq!(
        fs::read(&dest_file).expect("read dest"),
        b"source-bytes",
        "move must overwrite the same-size-newer destination with the source bytes"
    );
    assert!(
        !src_file.exists(),
        "source deleted only after its bytes landed"
    );
}

/// otp-10b-2 (review otp-10a F1 mirrored on pull): a remote→local move
/// through the actual binary must land the REMOTE source's bytes over
/// a same-size-newer LOCAL destination before deleting the remote
/// source — the pull twin of the push-move pin above. Pre-cutover the
/// old pull's SizeMtime compare would skip the file and the
/// source-delete would destroy the only copy of its content.
#[test]
fn pull_move_lands_source_bytes_over_same_size_newer_destination() {
    let ctx = TestContext::new();

    let remote_file = ctx.module_dir.join("clash.txt");
    fs::write(&remote_file, "source-bytes").expect("write remote source");

    // Same size, different content, newer mtime at the LOCAL destination.
    let dest_dir = ctx.workspace.join("dest");
    fs::create_dir_all(&dest_dir).expect("dest dir");
    let dest_file = dest_dir.join("clash.txt");
    fs::write(&dest_file, "dest---bytes").expect("seed dest");
    let newer = filetime::FileTime::from_unix_time(
        filetime::FileTime::from_last_modification_time(
            &fs::metadata(&remote_file).expect("remote meta"),
        )
        .unix_seconds()
            + 60,
        0,
    );
    filetime::set_file_mtime(&dest_file, newer).expect("bump dest mtime");

    // A single-file remote source (a module-root move cannot delete
    // the module root itself); the rsync rule drops it into dest_dir.
    let src_remote = format!("127.0.0.1:{}:/test/clash.txt", ctx.daemon_port);
    let mut cli_cmd = Command::new(&ctx.cli_bin);
    cli_cmd
        .arg("--config-dir")
        .arg(&ctx.config_dir)
        .arg("move")
        .arg("--yes")
        .arg(&src_remote)
        .arg(&dest_dir);

    let output = run_with_timeout(cli_cmd, Duration::from_secs(60));
    assert!(
        output.status.success(),
        "blit move failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    assert_eq!(
        fs::read(&dest_file).expect("read dest"),
        b"source-bytes",
        "pull-move must overwrite the same-size-newer destination with the remote bytes"
    );
    assert!(
        !remote_file.exists(),
        "remote source deleted only after its bytes landed"
    );
}

#[test]
fn test_remote_move_remote_to_local() {
    let ctx = TestContext::new();
    let dest_dir = ctx.workspace.join("dest");
    // Pre-create dest as a directory so the rsync resolver treats it as
    // a container (places `remote_move.txt` inside it) rather than an
    // exact rename target.
    fs::create_dir_all(&dest_dir).expect("create dest dir");

    // Setup remote file
    let remote_file = ctx.module_dir.join("remote_move.txt");
    fs::write(&remote_file, "remote move content").expect("write file");

    let src_remote = format!("127.0.0.1:{}:/test/remote_move.txt", ctx.daemon_port);
    let mut cli_cmd = Command::new(&ctx.cli_bin);
    cli_cmd
        .arg("--config-dir")
        .arg(&ctx.config_dir)
        .arg("move")
        .arg("--yes")
        .arg(&src_remote)
        .arg(&dest_dir);

    let output = run_with_timeout(cli_cmd, Duration::from_secs(60));
    assert!(output.status.success(), "blit move failed");

    // Verify local file exists
    let dest_file = dest_dir.join("remote_move.txt");
    assert!(dest_file.exists(), "local file missing");
    let bytes = fs::read(&dest_file).expect("read local file");
    assert_eq!(bytes, b"remote move content");

    // Verify remote file is deleted
    assert!(
        !remote_file.exists(),
        "remote file should have been deleted"
    );
}

// audit-6e: the single-file move tests above cover both remote directions,
// but a move of a multi-file *directory tree* exercises the recursive
// copy-then-delete-source path — the real partial-deletion data-loss
// surface. These assert every file lands at the destination AND the entire
// source side is removed (source deleted only after the verified copy).

// The former Windows ignore is deliberately gone. Retained CI stderr showed
// the old push protocol stalled on the native `nested\c.txt` need-list echo,
// before source deletion. Commit 48c5a11 canonicalized that echo; the unified
// session now preserves the source's POSIX wire path directly. Keep this exact
// nested push-move active on every supported OS so that Windows guards the
// original end-to-end failure rather than only lower-level nested pushes.
#[test]
fn test_remote_move_local_to_remote_directory_tree() {
    let ctx = TestContext::new();
    let src_dir = ctx.workspace.join("tree_src");
    fs::create_dir_all(src_dir.join("nested")).expect("src dirs");
    fs::write(src_dir.join("a.txt"), "alpha").expect("write a");
    fs::write(src_dir.join("b.txt"), "bravo").expect("write b");
    fs::write(src_dir.join("nested/c.txt"), "charlie").expect("write c");

    // Trailing slash -> merge contents into the module root.
    let src_arg = format!("{}/", src_dir.display());
    let dest_remote = format!("127.0.0.1:{}:/test/", ctx.daemon_port);
    let mut cli_cmd = Command::new(&ctx.cli_bin);
    cli_cmd
        .arg("--config-dir")
        .arg(&ctx.config_dir)
        .arg("move")
        .arg("--yes")
        .arg(&src_arg)
        .arg(&dest_remote);

    let output = run_with_timeout(cli_cmd, Duration::from_secs(60));
    assert!(
        output.status.success(),
        "blit move (push dir) failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    // Every file landed at the module root (merged), nested tree preserved.
    assert_eq!(fs::read(ctx.module_dir.join("a.txt")).expect("a"), b"alpha");
    assert_eq!(fs::read(ctx.module_dir.join("b.txt")).expect("b"), b"bravo");
    assert_eq!(
        fs::read(ctx.module_dir.join("nested/c.txt")).expect("c"),
        b"charlie"
    );

    // Entire source tree removed only after the verified copy.
    assert!(!src_dir.join("a.txt").exists(), "src a.txt not deleted");
    assert!(
        !src_dir.join("nested/c.txt").exists(),
        "src nested/c.txt not deleted"
    );
    assert!(!src_dir.exists(), "source tree should be fully removed");
}

#[test]
fn test_remote_move_remote_to_local_directory_tree() {
    let ctx = TestContext::new();
    // Remote subdir with a nested tree.
    let remote_sub = ctx.module_dir.join("tree");
    fs::create_dir_all(remote_sub.join("inner")).expect("remote dirs");
    fs::write(remote_sub.join("x.txt"), "x-data").expect("write x");
    fs::write(remote_sub.join("inner/y.txt"), "y-data").expect("write y");

    let dest_dir = ctx.workspace.join("pulled");
    fs::create_dir_all(&dest_dir).expect("dest dir");

    // Trailing slash -> merge contents into dest_dir.
    let src_remote = format!("127.0.0.1:{}:/test/tree/", ctx.daemon_port);
    let mut cli_cmd = Command::new(&ctx.cli_bin);
    cli_cmd
        .arg("--config-dir")
        .arg(&ctx.config_dir)
        .arg("move")
        .arg("--yes")
        .arg(&src_remote)
        .arg(&dest_dir);

    let output = run_with_timeout(cli_cmd, Duration::from_secs(60));
    assert!(
        output.status.success(),
        "blit move (pull dir) failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    // Files arrived locally, nested tree preserved.
    assert_eq!(fs::read(dest_dir.join("x.txt")).expect("x"), b"x-data");
    assert_eq!(
        fs::read(dest_dir.join("inner/y.txt")).expect("y"),
        b"y-data"
    );

    // Remote source files removed after the verified copy.
    assert!(
        !remote_sub.join("x.txt").exists(),
        "remote source x.txt should be deleted"
    );
    assert!(
        !remote_sub.join("inner/y.txt").exists(),
        "remote source inner/y.txt should be deleted"
    );
    // Symmetric to the push direction: the recursive move must remove the
    // source directory tree, not just unlink the files.
    assert!(
        !remote_sub.join("inner").exists(),
        "remote source nested dir inner/ should be removed"
    );
    assert!(
        !remote_sub.exists(),
        "remote source tree/ should be fully removed after the move"
    );
}

/// `blit <args>` against the context's daemon: its exit status and
/// everything it said.
fn blit(ctx: &TestContext, args: &[&str]) -> (Option<i32>, String) {
    let mut cmd = Command::new(&ctx.cli_bin);
    cmd.arg("--config-dir").arg(&ctx.config_dir).args(args);
    let output = run_with_timeout(cmd, Duration::from_secs(60));
    (
        output.status.code(),
        format!(
            "{}{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        ),
    )
}

/// cr-uv2-1: a source typed as a directory is one, or nothing is copied.
/// `single.txt/` was tidied to `single.txt` before it reached the daemon
/// and the file was copied — where `cp`, `mv` and `rsync` say `Not a
/// directory` — and a move then asked for the removal of `single.txt/`,
/// which under `-f` is no failure: exit 0, both files in place. The
/// daemon is now told what was typed and refuses before anything is
/// copied, for a copy, a move, and a move between two daemons.
#[test]
fn a_file_typed_as_a_directory_is_neither_copied_nor_moved() {
    let ctx = TestContext::builder().delegation(true).build();
    let source = ctx.module_dir.join("single.txt");
    fs::write(&source, "one file").expect("source");
    let host = format!("127.0.0.1:{}:/test", ctx.daemon_port);
    let local = ctx.workspace.join("landed.txt");
    let there = ctx.module_dir.join("landed.txt");
    for typed in ["single.txt/", "single.txt/.", "single.txt//"] {
        for (verb, to, lands) in [
            ("copy", local.display().to_string(), &local),
            ("move", local.display().to_string(), &local),
            ("move", format!("{host}/landed.txt"), &there),
        ] {
            let (code, said) = blit(&ctx, &[verb, "-y", &format!("{host}/{typed}"), &to]);
            let what = format!("{verb} {typed} -> {to}\nexit {code:?}\n{said}");
            assert_ne!(code, Some(0), "refused\n{what}");
            assert!(
                said.contains("source 'single.txt/': Not a directory"),
                "in the system's words\n{what}"
            );
            assert!(!lands.exists(), "nothing was copied\n{what}");
            assert_eq!(
                fs::read_to_string(&source).expect("the source stays"),
                "one file",
                "{what}"
            );
        }
    }
    // Typed as what it is, it is copied.
    let (code, said) = blit(
        &ctx,
        &[
            "copy",
            "-y",
            &format!("{host}/single.txt"),
            &local.display().to_string(),
        ],
    );
    assert_eq!(code, Some(0), "{said}");
    assert_eq!(fs::read_to_string(&local).expect("copied"), "one file");
}

/// cr-uv2-1: a move removes the path its session read. The two were read
/// off the endpoint separately: `dir/.` was copied as the directory's
/// contents, and the removal was asked for `dir/.`, which `rm` refuses —
/// everything copied, nothing removed. Both now go by one path.
#[test]
fn a_move_removes_the_path_its_session_read() {
    let ctx = TestContext::builder().delegation(true).build();
    let host = format!("127.0.0.1:{}:/test", ctx.daemon_port);
    for (case, typed) in ["dir/", "dir/.", "dir//", "dir"].into_iter().enumerate() {
        let source = ctx.module_dir.join("dir");
        fs::create_dir_all(source.join("sub")).expect("source");
        fs::write(source.join("inner.txt"), "inner").expect("file");
        fs::write(source.join("sub/deep.txt"), "deep").expect("file");
        let to = ctx.workspace.join(format!("to-{case}"));
        let (code, said) = blit(
            &ctx,
            &[
                "move",
                "-y",
                &format!("{host}/{typed}"),
                &format!("{}/", to.display()),
            ],
        );
        let what = format!("move {typed}\nexit {code:?}\n{said}");
        assert_eq!(code, Some(0), "{what}");
        // With a separator after it the contents land; without, the
        // directory does.
        let landed = if typed == "dir" {
            to.join("dir")
        } else {
            to.clone()
        };
        assert_eq!(
            fs::read_to_string(landed.join("sub/deep.txt")).expect("copied"),
            "deep",
            "{what}"
        );
        assert!(!source.exists(), "the source is gone\n{what}");
    }
}

/// cr-uv2-1: `link/` on a link to a directory is that directory to the
/// transfer, and to the removal it is a link that is never gone through.
/// A move of it copied everything, removed nothing and exited 0. `mv`
/// refuses the operand; so does a move, before anything is copied. A copy
/// removes nothing, and goes through the link as it did.
#[cfg(unix)]
#[test]
fn a_move_through_a_link_typed_as_a_directory_is_refused_before_copying() {
    let ctx = TestContext::builder().delegation(true).build();
    let host = format!("127.0.0.1:{}:/test", ctx.daemon_port);
    let real = ctx.module_dir.join("real");
    fs::create_dir_all(&real).expect("directory");
    fs::write(real.join("inner.txt"), "inner").expect("file");
    std::os::unix::fs::symlink("real", ctx.module_dir.join("link")).expect("link");

    let local = ctx.workspace.join("moved");
    let there = ctx.module_dir.join("landing");
    for (to, lands) in [
        (format!("{}/", local.display()), &local),
        (format!("{host}/landing/"), &there),
    ] {
        let (code, said) = blit(&ctx, &["move", "-y", &format!("{host}/link/"), &to]);
        let what = format!("move link/ -> {to}\nexit {code:?}\n{said}");
        assert_ne!(code, Some(0), "refused\n{what}");
        assert!(
            said.contains("source 'link/' is a symbolic link to a directory"),
            "and says why\n{what}"
        );
        assert!(
            !lands.join("inner.txt").exists(),
            "nothing was copied\n{what}"
        );
        assert!(
            real.join("inner.txt").exists() && ctx.module_dir.join("link").is_symlink(),
            "the source stays whole\n{what}"
        );
    }

    let copied = ctx.workspace.join("copied");
    let (code, said) = blit(
        &ctx,
        &[
            "copy",
            "-y",
            &format!("{host}/link/"),
            &format!("{}/", copied.display()),
        ],
    );
    assert_eq!(code, Some(0), "a copy goes through the link\n{said}");
    assert_eq!(
        fs::read_to_string(copied.join("inner.txt")).expect("copied"),
        "inner"
    );

    // The directory itself, typed as one, is moved whole.
    let (code, said) = blit(
        &ctx,
        &[
            "move",
            "-y",
            &format!("{host}/real/"),
            &format!("{}/", local.display()),
        ],
    );
    assert_eq!(code, Some(0), "{said}");
    assert!(local.join("inner.txt").exists() && !real.exists(), "{said}");
}

/// cr-uv3-1: the removal that ends a move takes what the move SENT. It was
/// a recursive removal of whatever stood at the source: `move incoming/.`,
/// holding a file and a link to it, exited 0 with the tree gone and the
/// file alone at the destination — the link, which no transfer carries,
/// removed with it. The daemon keeps an account of what each session sent
/// and removes that: the files, and the directories they leave empty. The
/// link stays where it was, the command says so, and it does not end
/// well. To this machine and between two daemons.
#[cfg(unix)]
#[test]
fn a_move_removes_what_it_sent_and_leaves_what_it_did_not() {
    let ctx = TestContext::builder().delegation(true).build();
    let host = format!("127.0.0.1:{}:/test", ctx.daemon_port);
    let local = ctx.workspace.join("moved");
    let there = ctx.module_dir.join("landing");
    for (typed, to, lands) in [
        ("incoming/.", format!("{}/", local.display()), &local),
        ("incoming/", format!("{host}/landing/"), &there),
    ] {
        let incoming = ctx.module_dir.join("incoming");
        fs::create_dir_all(incoming.join("sub/empty")).expect("tree");
        fs::write(incoming.join("file.txt"), "carried").expect("file");
        fs::write(incoming.join("sub/inner.txt"), "carried too").expect("file");
        std::os::unix::fs::symlink("file.txt", incoming.join("link")).expect("link");

        let (code, said) = blit(&ctx, &["move", "-y", &format!("{host}/{typed}"), &to]);
        let what = format!("move {typed} -> {to}\nexit {code:?}\n{said}");
        assert_eq!(code, Some(1), "the move is not complete\n{what}");
        assert!(
            said.contains("link': it is a symbolic link, which a move does not carry"),
            "and says what stayed, and why\n{what}"
        );
        // What was sent is at the destination, and gone from the source
        // with the directories it left empty.
        assert_eq!(
            fs::read_to_string(lands.join("file.txt")).expect("moved"),
            "carried",
            "{what}"
        );
        assert_eq!(
            fs::read_to_string(lands.join("sub/inner.txt")).expect("moved"),
            "carried too",
            "{what}"
        );
        assert!(
            !incoming.join("file.txt").exists() && !incoming.join("sub").exists(),
            "what was sent is gone\n{what}"
        );
        // What was not is where it was — and nowhere else.
        assert!(incoming.join("link").is_symlink(), "the link stays\n{what}");
        assert!(
            fs::symlink_metadata(lands.join("link")).is_err(),
            "no transfer carried it\n{what}"
        );
        fs::remove_dir_all(&incoming).expect("for the next");
    }

    // With nothing in it that a move does not carry, the source goes whole.
    let incoming = ctx.module_dir.join("incoming");
    fs::create_dir_all(incoming.join("sub/empty")).expect("tree");
    fs::write(incoming.join("sub/inner.txt"), "carried").expect("file");
    let whole = ctx.workspace.join("whole");
    let (code, said) = blit(
        &ctx,
        &[
            "move",
            "-y",
            &format!("{host}/incoming/"),
            &format!("{}/", whole.display()),
        ],
    );
    assert_eq!(code, Some(0), "{said}");
    assert!(
        whole.join("sub/inner.txt").exists() && !incoming.exists(),
        "{said}"
    );
}

/// A move from a daemon removes EVERY file it delivered, however many.
/// The daemon's account of what a move sent was written, when the
/// manifest was complete, from what was left of the map of the manifest —
/// and a need takes its entry out of that map, and needs arrive while the
/// manifest is still going. Past the first batch of needs (128 files) a
/// move delivered everything, left at its source whatever had been asked
/// for early, said `it is not a file the move sent` of each, and exited
/// 1. Found measuring a move of 20,000 files, 2026-11-21: 3,840 left.
#[test]
fn a_move_of_many_files_removes_every_one_it_delivered() {
    let ctx = TestContext::new();
    let tree = ctx.module_dir.join("tree");
    let (dirs, files) = (10, 100);
    for d in 0..dirs {
        let dir = tree.join(format!("d{d:02}"));
        fs::create_dir_all(&dir).unwrap();
        for f in 0..files {
            fs::write(dir.join(format!("f{f:03}")), format!("{d}/{f}")).unwrap();
        }
    }
    let landed = ctx.workspace.join("landed");
    let source = format!("127.0.0.1:{}:/test/tree/", ctx.daemon_port);
    let (code, said) = blit(
        &ctx,
        &["move", "-y", &source, &format!("{}/", landed.display())],
    );
    assert_eq!(code, Some(0), "{said}");
    let count = |root: &std::path::Path| -> usize {
        fn walk(dir: &std::path::Path, count: &mut usize) {
            for entry in fs::read_dir(dir).into_iter().flatten().flatten() {
                if entry.file_type().is_ok_and(|kind| kind.is_dir()) {
                    walk(&entry.path(), count);
                } else {
                    *count += 1;
                }
            }
        }
        let mut count = 0;
        walk(root, &mut count);
        count
    };
    assert_eq!(count(&landed), dirs * files, "everything landed\n{said}");
    assert_eq!(
        fs::read(landed.join("d09/f099")).unwrap(),
        b"9/99",
        "{said}"
    );
    assert_eq!(
        count(&tree),
        0,
        "and nothing delivered stays at the source\n{said}"
    );
    assert!(
        !tree.join("d00").exists(),
        "nor the directories it emptied\n{said}"
    );
}

/// cr-uv5-4: a move of two names of one text removes both once both have
/// landed. A source holding `dup` and the byte `FF`, and `dup` and
/// U+FFFD: the end that writes takes the first and refuses the second,
/// and the retry lands it. The first session's account struck what was
/// refused by its text AND its bytes — the file that had LANDED bears
/// that text as its name, was struck with it, and stayed at the source:
/// the reviewer saw `dup` and U+FFFD left behind, and exit 1.
#[cfg(unix)]
#[test]
fn a_move_of_two_names_of_one_text_removes_both() {
    use std::ffi::OsStr;
    use std::os::unix::ffi::OsStrExt;

    let ctx = TestContext::new();
    let tree = ctx.module_dir.join("tree");
    fs::create_dir_all(&tree).unwrap();
    let raw = OsStr::from_bytes(b"dup\xff");
    if fs::write(tree.join(raw), b"named by a byte").is_err() {
        eprintln!("this filesystem refuses a name that is not text: nothing to tell apart");
        return;
    }
    fs::write(tree.join("dup\u{fffd}"), b"named by the character").unwrap();
    fs::write(tree.join("plain.txt"), b"beside them").unwrap();
    let landed = ctx.workspace.join("landed");
    let source = format!("127.0.0.1:{}:/test/tree/", ctx.daemon_port);
    let (code, said) = blit(
        &ctx,
        &[
            "move",
            "-y",
            "--retries",
            "1",
            "--diagnostics-no-retry-wait",
            &source,
            &format!("{}/", landed.display()),
        ],
    );
    assert_eq!(code, Some(0), "{said}");
    assert_eq!(
        fs::read(landed.join(raw)).unwrap(),
        b"named by a byte",
        "{said}"
    );
    assert_eq!(
        fs::read(landed.join("dup\u{fffd}")).unwrap(),
        b"named by the character",
        "{said}"
    );
    let left: Vec<_> = fs::read_dir(&tree)
        .map(|entries| entries.flatten().map(|entry| entry.file_name()).collect())
        .unwrap_or_default();
    assert!(left.is_empty(), "left at the source: {left:?}\n{said}");
}

/// cr-uv5-3: a move takes every name of a file that has several. Both
/// names of one file arrived, and the removal — which takes a file only
/// while it is the file that was sent — refused the second: taking the
/// first name had changed the file, as taking a name does. The reviewer
/// saw one name left at the source, and exit 1.
#[cfg(unix)]
#[test]
fn a_move_takes_every_name_of_a_file_with_several() {
    let ctx = TestContext::new();
    let tree = ctx.module_dir.join("tree");
    fs::create_dir_all(tree.join("sub")).unwrap();
    fs::write(tree.join("first.txt"), b"one file, three names").unwrap();
    fs::hard_link(tree.join("first.txt"), tree.join("second.txt")).unwrap();
    fs::hard_link(tree.join("first.txt"), tree.join("sub/third.txt")).unwrap();
    fs::write(tree.join("alone.txt"), b"another").unwrap();
    let landed = ctx.workspace.join("landed");
    let source = format!("127.0.0.1:{}:/test/tree/", ctx.daemon_port);
    let (code, said) = blit(
        &ctx,
        &["move", "-y", &source, &format!("{}/", landed.display())],
    );
    assert_eq!(code, Some(0), "{said}");
    for name in ["first.txt", "second.txt", "sub/third.txt"] {
        assert_eq!(
            fs::read(landed.join(name)).unwrap(),
            b"one file, three names",
            "{name}\n{said}"
        );
    }
    assert_eq!(fs::read(landed.join("alone.txt")).unwrap(), b"another");
    let left: Vec<_> = fs::read_dir(&tree)
        .map(|entries| entries.flatten().map(|entry| entry.file_name()).collect())
        .unwrap_or_default();
    assert!(left.is_empty(), "left at the source: {left:?}\n{said}");
}

/// cr-uv5-2: a move never removes bytes nobody was sent. A file is
/// rewritten by another hand — as many bytes, its time put back — while
/// the move is still writing its manifest: after the file was read (here,
/// hashed: the destination already holds its first bytes under another
/// time, so by `--checksum` nothing of it is sent) and before the
/// manifest's end, which files that take a while to hash put off.
///
/// The account of what a move may remove was written at the manifest's
/// end, from a second look at each file, and that look took the rewritten
/// file for the one that had been hashed: it went from the source with
/// exit 0, and its bytes were nowhere. The account now holds each file as
/// the walk first saw it, so a file changed after that look stays.
///
/// What is asserted holds whenever the rewrite lands: the rewritten bytes
/// are at the source still, or at the destination. Against the code
/// before this fix it fails when the rewrite lands between the hash and
/// the manifest's end — which the sizes here make most of a second wide.
#[cfg(unix)]
#[test]
fn a_move_never_removes_bytes_nobody_was_sent() {
    let ctx = TestContext::new();
    let tree = ctx.module_dir.join("tree");
    let landed = ctx.workspace.join("landed");
    for at in [&tree, &landed] {
        fs::create_dir_all(at).unwrap();
        // A while to hash, and nothing to store: all holes.
        for n in 0..6 {
            fs::File::create(at.join(format!("big{n}.bin")))
                .and_then(|big| big.set_len(512 * 1024 * 1024))
                .unwrap();
        }
        // Made last, and first by name: a walk that reads a directory
        // newest first meets it first, and so does one that reads by name.
        fs::write(at.join("a-small.txt"), b"1234").unwrap();
    }
    // Another time at the destination: only the checksum says the two are
    // one file.
    filetime::set_file_mtime(
        landed.join("a-small.txt"),
        filetime::FileTime::from_unix_time(1_000_000_000, 0),
    )
    .unwrap();
    let small = tree.join("a-small.txt");
    let written = filetime::FileTime::from_last_modification_time(&fs::metadata(&small).unwrap());

    let mover = {
        let (cli, config) = (ctx.cli_bin.clone(), ctx.config_dir.clone());
        let source = format!("127.0.0.1:{}:/test/tree/", ctx.daemon_port);
        let destination = format!("{}/", landed.display());
        std::thread::spawn(move || {
            let mut cmd = Command::new(cli);
            cmd.arg("--config-dir")
                .arg(config)
                .args(["move", "-y", "--checksum"])
                .arg(source)
                .arg(destination);
            run_with_timeout(cmd, Duration::from_secs(180))
        })
    };
    std::thread::sleep(Duration::from_millis(250));
    // Another hand: as many bytes, and the time put back.
    fs::write(&small, b"4321").unwrap();
    filetime::set_file_mtime(&small, written).unwrap();
    let output = mover.join().expect("the move");
    let said = format!(
        "exit {:?}\n{}{}",
        output.status.code(),
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    let at_source = fs::read(&small).ok();
    let at_destination = fs::read(landed.join("a-small.txt")).unwrap();
    assert!(
        at_source.as_deref() == Some(&b"4321"[..]) || at_destination == b"4321",
        "the rewritten bytes are nowhere: gone from the source, and the destination holds \
         {:?}\n{said}",
        String::from_utf8_lossy(&at_destination)
    );
    match at_source {
        // Left where it was: said, and the move did not end well.
        Some(_) => {
            assert_eq!(output.status.code(), Some(1), "{said}");
            assert!(said.contains("a-small.txt"), "{said}");
        }
        None => assert_eq!(output.status.code(), Some(0), "{said}"),
    }
    // What was not touched went, either way.
    for n in 0..6 {
        assert!(!tree.join(format!("big{n}.bin")).exists(), "big{n}\n{said}");
    }
}
