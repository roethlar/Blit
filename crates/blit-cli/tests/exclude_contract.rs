//! audit-19 (RELEASE_1_0 G3): `--exclude` matches the forms people type.
//!
//! Before the fix `--exclude .java` dropped only an entry literally named
//! `.java` (its contents still copied) and `--exclude /home/me/.java`
//! matched nothing at all — both silently. The contract now: a pattern
//! with no separator names a component at any depth (a directory with its
//! contents); a pattern with one is source-relative and names a subtree;
//! an absolute pattern under a local source is made relative to it; one
//! that cannot match anything is named on stderr and the run goes on;
//! an excluded directory is pruned, never opened.

use std::fs;
use std::path::Path;
use std::time::Duration;

use tempfile::tempdir;

mod common;
use common::{cli_command, run_with_timeout};

/// `.java/fonts/x`, `a/.java/y`, `keep.txt`, `a/keep.txt` under `src`.
fn java_tree(src: &Path) {
    for (rel, body) in [
        (".java/fonts/x", "x"),
        ("a/.java/y", "y"),
        ("keep.txt", "k"),
        ("a/keep.txt", "k"),
    ] {
        let path = src.join(rel);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, body).unwrap();
    }
}

/// Run `blit <verb> <args...>` and return (success, stdout, stderr).
fn blit(args: &[&str]) -> (bool, String, String) {
    let mut cmd = cli_command();
    cmd.args(args);
    let output = run_with_timeout(cmd, Duration::from_secs(60));
    (
        output.status.success(),
        String::from_utf8_lossy(&output.stdout).into_owned(),
        String::from_utf8_lossy(&output.stderr).into_owned(),
    )
}

fn with_slash(path: &Path) -> String {
    format!("{}/", path.display())
}

/// Restores a directory's mode so the tempdir can be cleaned up.
#[cfg(unix)]
struct Restore(std::path::PathBuf);

#[cfg(unix)]
impl Drop for Restore {
    fn drop(&mut self) {
        use std::os::unix::fs::PermissionsExt;
        let _ = fs::set_permissions(&self.0, fs::Permissions::from_mode(0o755));
    }
}

/// Make `dir` unreadable; `None` when the run is privileged and reads
/// through mode 000 anyway (the never-opened proof then cannot bite).
#[cfg(unix)]
fn make_unreadable(dir: &Path) -> Option<Restore> {
    use std::os::unix::fs::PermissionsExt;
    fs::set_permissions(dir, fs::Permissions::from_mode(0o000)).unwrap();
    let restore = Restore(dir.to_path_buf());
    if fs::read_dir(dir).is_ok() {
        eprintln!("skipping: running privileged, mode 000 does not block reads");
        return None;
    }
    Some(restore)
}

#[test]
fn mirror_exclude_bare_name_leaves_the_directory_out_at_any_depth() {
    let tmp = tempdir().unwrap();
    let (src, dst) = (tmp.path().join("src"), tmp.path().join("dst"));
    java_tree(&src);

    let (ok, stdout, stderr) = blit(&[
        "mirror",
        "--yes",
        &with_slash(&src),
        &with_slash(&dst),
        "--exclude",
        ".java",
    ]);
    assert!(ok, "mirror failed:\nstdout: {stdout}\nstderr: {stderr}");
    assert!(
        !dst.join(".java").exists(),
        "top-level .java must be left out"
    );
    assert!(
        !dst.join("a/.java").exists(),
        "nested .java must be left out"
    );
    assert!(dst.join("keep.txt").exists());
    assert!(dst.join("a/keep.txt").exists());
}

#[test]
fn mirror_exclude_absolute_path_under_the_source_is_made_relative() {
    let tmp = tempdir().unwrap();
    let (src, dst) = (tmp.path().join("src"), tmp.path().join("dst"));
    java_tree(&src);
    let pattern = format!("{}", src.join(".java").display());

    let (ok, stdout, stderr) = blit(&[
        "mirror",
        "--yes",
        &with_slash(&src),
        &with_slash(&dst),
        "--exclude",
        &pattern,
    ]);
    assert!(ok, "mirror failed:\nstdout: {stdout}\nstderr: {stderr}");
    assert!(
        !dst.join(".java").exists(),
        "the named path must be left out"
    );
    // An absolute path names that one path: `a/.java` is another.
    assert!(
        dst.join("a/.java/y").exists(),
        "only the named path is excluded"
    );
    assert!(dst.join("keep.txt").exists());
    assert!(
        !stderr.contains("cannot match anything"),
        "a pattern under the source must not warn: {stderr}"
    );
}

#[test]
fn mirror_exclude_absolute_path_outside_the_source_warns_and_copies_everything() {
    let tmp = tempdir().unwrap();
    let (src, dst) = (tmp.path().join("src"), tmp.path().join("dst"));
    java_tree(&src);

    let (ok, stdout, stderr) = blit(&[
        "mirror",
        "--yes",
        &with_slash(&src),
        &with_slash(&dst),
        "--exclude",
        "/nowhere/.java",
    ]);
    assert!(ok, "mirror failed:\nstdout: {stdout}\nstderr: {stderr}");
    assert!(dst.join(".java/fonts/x").exists(), "nothing was excluded");
    assert!(dst.join("a/.java/y").exists(), "nothing was excluded");
    let warning = "blit: --exclude '/nowhere/.java' cannot match anything under";
    assert_eq!(
        stderr.matches(warning).count(),
        1,
        "exactly one warning line expected:\n{stderr}"
    );
}

#[test]
fn check_exclude_absolute_path_under_the_source_is_made_relative() {
    let tmp = tempdir().unwrap();
    let (src, dst) = (tmp.path().join("src"), tmp.path().join("dst"));
    java_tree(&src);
    // The destination holds everything but the top-level `.java`.
    java_tree(&dst);
    fs::remove_dir_all(dst.join(".java")).unwrap();
    let pattern = format!("{}", src.join(".java").display());

    let (ok, stdout, stderr) = blit(&[
        "check",
        &src.display().to_string(),
        &dst.display().to_string(),
        "--exclude",
        &pattern,
    ]);
    assert!(
        ok,
        "the excluded path must not count as a difference:\nstdout: {stdout}\nstderr: {stderr}"
    );
}

/// True when `dir`'s filesystem ignores case: a file made as `case-probe`
/// opens as `CASE-PROBE`.
fn case_insensitive(dir: &Path) -> bool {
    let probe = dir.join("case-probe");
    fs::write(&probe, b"").unwrap();
    let insensitive = fs::File::open(dir.join("CASE-PROBE")).is_ok();
    fs::remove_file(&probe).unwrap();
    insensitive
}

/// cr-g3fix-1: on a case-insensitive filesystem (macOS and Windows by
/// default) an absolute exclude in another case leaves out the entry on
/// disk. Before the fix it was rewritten as typed (`./CACHE`), matched
/// nothing — every rule compares case-sensitively — and warned about
/// nothing. Skipped where the filesystem is case-sensitive.
#[test]
fn mirror_exclude_absolute_path_in_another_case_leaves_the_entry_out() {
    let tmp = tempdir().unwrap();
    let (src, dst) = (tmp.path().join("src"), tmp.path().join("dst"));
    java_tree(&src);
    fs::create_dir_all(src.join("cache")).unwrap();
    fs::write(src.join("cache").join("private.txt"), b"p").unwrap();
    if !case_insensitive(&src) {
        eprintln!("skipping: {} is case-sensitive", src.display());
        return;
    }
    let pattern = format!("{}", src.join("CACHE").display());

    let (ok, stdout, stderr) = blit(&[
        "mirror",
        "--yes",
        &with_slash(&src),
        &with_slash(&dst),
        "--exclude",
        &pattern,
    ]);
    assert!(ok, "mirror failed:\nstdout: {stdout}\nstderr: {stderr}");
    assert!(
        !dst.join("cache").exists(),
        "the entry named in another case must be left out"
    );
    assert!(dst.join("keep.txt").exists());
    assert!(
        !stderr.contains("cannot match anything"),
        "a pattern under the source must not warn: {stderr}"
    );
}

/// cr-g3fix2-1: on a case-insensitive filesystem an absolute exclude in
/// another case that names a symlink in the source keeps the destination's
/// entry of that name out of the mirror's reach. Before the fix the link's
/// stored name was read through `canonicalize`, which cannot name a link
/// without following it, so the exclude stayed `./CACHE`, matched nothing,
/// warned about nothing, and the mirror deleted the destination's
/// `cache/` subtree. Skipped where the filesystem is case-sensitive.
#[cfg(unix)]
#[test]
fn mirror_exclude_symlink_in_another_case_keeps_the_destination_subtree() {
    let tmp = tempdir().unwrap();
    let (src, dst) = (tmp.path().join("src"), tmp.path().join("dst"));
    let outside = tmp.path().join("outside");
    fs::create_dir_all(&src).unwrap();
    fs::write(src.join("keep.txt"), b"k").unwrap();
    fs::create_dir_all(&outside).unwrap();
    fs::write(outside.join("o.txt"), b"o").unwrap();
    std::os::unix::fs::symlink(&outside, src.join("cache")).unwrap();
    fs::create_dir_all(dst.join("cache")).unwrap();
    fs::write(dst.join("cache").join("saved.txt"), b"s").unwrap();
    if !case_insensitive(&src) {
        eprintln!("skipping: {} is case-sensitive", src.display());
        return;
    }
    let pattern = format!("{}", src.join("CACHE").display());

    let (ok, stdout, stderr) = blit(&[
        "mirror",
        "--yes",
        &with_slash(&src),
        &with_slash(&dst),
        "--exclude",
        &pattern,
    ]);
    assert!(ok, "mirror failed:\nstdout: {stdout}\nstderr: {stderr}");
    assert!(
        dst.join("cache").join("saved.txt").exists(),
        "the destination's cache/ is excluded and must survive the mirror:\nstdout: {stdout}"
    );
    assert!(dst.join("keep.txt").exists());
    assert!(
        !stderr.contains("cannot match anything"),
        "a pattern under the source must not warn: {stderr}"
    );
}

/// cr-g3fix-2: an absolute glob written through a symlink to the source
/// is made relative. The glob path itself never exists, so only its
/// literal part is resolved; before the fix the pattern was reported as
/// unable to match and every `.txt` file copied.
#[cfg(unix)]
#[test]
fn mirror_exclude_absolute_glob_through_a_symlinked_source() {
    let tmp = tempdir().unwrap();
    let (src, dst) = (tmp.path().join("src"), tmp.path().join("dst"));
    java_tree(&src);
    fs::write(src.join("other.dat"), b"o").unwrap();
    let alias = tmp.path().join("alias");
    std::os::unix::fs::symlink(&src, &alias).unwrap();
    let pattern = format!("{}", alias.join("*.txt").display());

    let (ok, stdout, stderr) = blit(&[
        "mirror",
        "--yes",
        &with_slash(&src),
        &with_slash(&dst),
        "--exclude",
        &pattern,
    ]);
    assert!(ok, "mirror failed:\nstdout: {stdout}\nstderr: {stderr}");
    assert!(
        !dst.join("keep.txt").exists(),
        "the glob through the alias must exclude keep.txt"
    );
    assert!(dst.join("other.dat").exists());
    assert!(
        !stderr.contains("cannot match anything"),
        "a glob under the source must not warn: {stderr}"
    );
}

/// cr-g3fix2-2: a name read from the disk is matched as itself. Through
/// an alias into `data[1]` the resolved name reached the matcher as glob
/// text, where `[1]` is a class: `data[1]/private.txt` — the file the
/// exclude names — was copied and the unrelated `data1/private.txt` left
/// out. Through an alias into `open[x` (an unmatched `[`) the run failed
/// on a pattern the user wrote correctly.
#[cfg(unix)]
#[test]
fn copy_exclude_through_an_alias_into_a_directory_named_like_a_glob() {
    let tmp = tempdir().unwrap();
    let src = tmp.path().join("src");
    for rel in [
        "data[1]/private.txt",
        "data[1]/keep.dat",
        "data1/private.txt",
        "open[x/private.txt",
        "keep.txt",
    ] {
        let path = src.join(rel);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, b"x").unwrap();
    }
    let into_class = tmp.path().join("into_class");
    std::os::unix::fs::symlink(src.join("data[1]"), &into_class).unwrap();
    let into_open = tmp.path().join("into_open");
    std::os::unix::fs::symlink(src.join("open[x"), &into_open).unwrap();

    let dst = tmp.path().join("dst_class");
    let pattern = format!("{}", into_class.join("*.txt").display());
    let (ok, stdout, stderr) = blit(&[
        "copy",
        &with_slash(&src),
        &with_slash(&dst),
        "--exclude",
        &pattern,
    ]);
    assert!(ok, "copy failed:\nstdout: {stdout}\nstderr: {stderr}");
    assert!(
        !dst.join("data[1]/private.txt").exists(),
        "the file the exclude names must be left out"
    );
    assert!(dst.join("data[1]/keep.dat").exists());
    assert!(
        dst.join("data1/private.txt").exists(),
        "a name the exclude never named must be copied"
    );
    assert!(dst.join("keep.txt").exists());
    assert!(
        !stderr.contains("cannot match anything"),
        "a pattern under the source must not warn: {stderr}"
    );

    let dst = tmp.path().join("dst_open");
    let pattern = format!("{}", into_open.display());
    let (ok, stdout, stderr) = blit(&[
        "copy",
        &with_slash(&src),
        &with_slash(&dst),
        "--exclude",
        &pattern,
    ]);
    assert!(ok, "copy failed:\nstdout: {stdout}\nstderr: {stderr}");
    assert!(
        !dst.join("open[x").exists(),
        "the aliased directory is left out"
    );
    assert!(dst.join("data1/private.txt").exists());
    assert!(
        !stderr.contains("cannot match anything"),
        "a pattern under the source must not warn: {stderr}"
    );
}

/// cr-g3fix-3: on Windows `\` is a separator in a relative pattern too —
/// `.\.java` names the top-level `.java` only, `a\.java\` the `a/.java`
/// subtree, and `.java\` the name at any depth. Before the fix none of the
/// three excluded anything.
#[cfg(windows)]
#[test]
fn mirror_exclude_windows_backslash_forms() {
    for (pattern, gone, kept) in [
        (r".\.java", &[".java"][..], &["a/.java/y"][..]),
        (r"a\.java\", &["a/.java"][..], &[".java/fonts/x"][..]),
        (r".java\", &[".java", "a/.java"][..], &["a/keep.txt"][..]),
    ] {
        let tmp = tempdir().unwrap();
        let (src, dst) = (tmp.path().join("src"), tmp.path().join("dst"));
        java_tree(&src);
        let (ok, stdout, stderr) = blit(&[
            "mirror",
            "--yes",
            &with_slash(&src),
            &with_slash(&dst),
            "--exclude",
            pattern,
        ]);
        assert!(
            ok,
            "{pattern}: mirror failed:\nstdout: {stdout}\nstderr: {stderr}"
        );
        for rel in gone {
            assert!(!dst.join(rel).exists(), "{pattern}: {rel} must be left out");
        }
        for rel in kept {
            assert!(dst.join(rel).exists(), "{pattern}: {rel} must be copied");
        }
        assert!(dst.join("keep.txt").exists(), "{pattern}: keep.txt");
    }
}

/// The excluded directory is never opened: unreadable, it would make the
/// source scan incomplete, and an incomplete scan refuses a mirror's
/// deletions (here: the run fails and `stale.txt` survives).
#[cfg(unix)]
#[test]
fn mirror_never_opens_an_excluded_unreadable_directory() {
    let tmp = tempdir().unwrap();
    let (src, dst) = (tmp.path().join("src"), tmp.path().join("dst"));
    java_tree(&src);
    fs::create_dir_all(&dst).unwrap();
    fs::write(dst.join("stale.txt"), b"extraneous").unwrap();
    let Some(_restore) = make_unreadable(&src.join(".java")) else {
        return;
    };

    let (ok, stdout, stderr) = blit(&[
        "mirror",
        "--yes",
        &with_slash(&src),
        &with_slash(&dst),
        "--exclude",
        ".java",
    ]);
    assert!(ok, "mirror failed:\nstdout: {stdout}\nstderr: {stderr}");
    assert!(dst.join("keep.txt").exists());
    assert!(
        !dst.join("stale.txt").exists(),
        "a complete scan lets the mirror delete the extraneous file"
    );
}

/// Push: the absolute pattern is relativized before the wire spec is
/// built, so the client's source scan prunes the (unreadable) directory
/// and the daemon's mirror scope leaves the destination's copy of it
/// alone, while the extraneous file is still deleted.
#[cfg(unix)]
#[test]
fn push_mirror_relativizes_an_absolute_exclude_and_prunes_it() {
    let ctx = common::TestContext::new();
    let src = ctx.workspace.join("src");
    java_tree(&src);
    fs::write(ctx.module_dir.join("extra.txt"), b"extraneous").unwrap();
    fs::create_dir_all(ctx.module_dir.join(".java")).unwrap();
    fs::write(ctx.module_dir.join(".java/old"), b"out of scope").unwrap();
    let Some(_restore) = make_unreadable(&src.join(".java")) else {
        return;
    };
    let pattern = format!("{}", src.join(".java").display());

    let mut cmd = std::process::Command::new(&ctx.cli_bin);
    cmd.arg("--config-dir")
        .arg(&ctx.config_dir)
        .arg("mirror")
        .arg("--yes")
        .arg(with_slash(&src))
        .arg(format!("127.0.0.1:{}:/test/", ctx.daemon_port))
        .arg("--exclude")
        .arg(&pattern);
    let output = run_with_timeout(cmd, Duration::from_secs(60));
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        output.status.success(),
        "push mirror failed:\nstdout: {stdout}\nstderr: {stderr}"
    );
    assert!(ctx.module_dir.join("keep.txt").exists());
    assert!(
        ctx.module_dir.join("a/.java/y").exists(),
        "only the named path is excluded"
    );
    assert!(
        !ctx.module_dir.join("extra.txt").exists(),
        "a complete source scan lets the mirror delete the extraneous file"
    );
    assert!(
        ctx.module_dir.join(".java/old").exists(),
        "the daemon's mirror scope must exclude the relativized path"
    );
}
