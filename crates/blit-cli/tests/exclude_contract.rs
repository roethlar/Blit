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

/// True when `dir`'s filesystem resolves `typed` to an entry made as
/// `stored` (probed with a file of that name, removed again).
fn filesystem_pairs(dir: &Path, stored: &str, typed: &str) -> bool {
    let probe = dir.join(stored);
    fs::write(&probe, b"").unwrap();
    let paired = fs::symlink_metadata(dir.join(typed)).is_ok();
    fs::remove_file(&probe).unwrap();
    paired
}

/// cr-g3fix3-1: an absolute exclude naming a symlink in a spelling only
/// the filesystem pairs — final sigma `ς` for a stored `σ` on APFS, which
/// per-character lowercase does not pair — keeps the destination's entry
/// of the stored name out of the mirror's reach. Before the fix the
/// exclude stayed `./ς`, matched nothing, warned about nothing, and the
/// mirror deleted `dst/σ/saved.txt` with its directory. Skipped where the
/// filesystem does not pair the two spellings.
#[cfg(unix)]
#[test]
fn mirror_exclude_symlink_in_a_filesystem_equivalent_spelling_keeps_the_destination_subtree() {
    let tmp = tempdir().unwrap();
    let (src, dst) = (tmp.path().join("src"), tmp.path().join("dst"));
    let outside = tmp.path().join("outside");
    fs::create_dir_all(&src).unwrap();
    fs::write(src.join("keep.txt"), b"k").unwrap();
    if !filesystem_pairs(&src, "σ", "ς") {
        eprintln!("skipping: {} does not pair ς with σ", src.display());
        return;
    }
    fs::create_dir_all(&outside).unwrap();
    fs::write(outside.join("o.txt"), b"o").unwrap();
    std::os::unix::fs::symlink(&outside, src.join("σ")).unwrap();
    fs::create_dir_all(dst.join("σ")).unwrap();
    fs::write(dst.join("σ").join("saved.txt"), b"s").unwrap();
    let pattern = format!("{}", src.join("ς").display());

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
        dst.join("σ").join("saved.txt").exists(),
        "the destination's σ/ is excluded and must survive the mirror:\nstdout: {stdout}"
    );
    assert!(dst.join("keep.txt").exists());
    assert!(
        !stderr.contains("cannot match anything"),
        "a pattern under the source must not warn: {stderr}"
    );
}

/// cr-g3fix4-1: the same symlink hard-linked under a second name
/// (`src/alias`, one entry with `src/σ`): no API names which of the two
/// the typed `ς` matched, so both are excluded, a note names them, and the
/// destination's `σ/` survives. Before the fix the hard link sent the
/// pairing to a case fold, which paired neither: the exclude matched
/// nothing, warned about nothing, and the mirror deleted `dst/σ/saved.txt`
/// with its directory. Skipped where the filesystem does not pair the
/// spellings or the link cannot be hard-linked as a link.
#[cfg(unix)]
#[test]
fn mirror_exclude_a_hard_linked_symlink_in_a_paired_spelling_keeps_the_destination_subtree() {
    let tmp = tempdir().unwrap();
    let (src, dst) = (tmp.path().join("src"), tmp.path().join("dst"));
    let outside = tmp.path().join("outside");
    fs::create_dir_all(&src).unwrap();
    fs::write(src.join("keep.txt"), b"k").unwrap();
    if !filesystem_pairs(&src, "σ", "ς") {
        eprintln!("skipping: {} does not pair ς with σ", src.display());
        return;
    }
    fs::create_dir_all(&outside).unwrap();
    fs::write(outside.join("o.txt"), b"o").unwrap();
    std::os::unix::fs::symlink(&outside, src.join("σ")).unwrap();
    let linked = fs::hard_link(src.join("σ"), src.join("alias")).is_ok()
        && fs::symlink_metadata(src.join("alias")).is_ok_and(|m| m.file_type().is_symlink());
    if !linked {
        eprintln!("skipping: {} cannot hard-link a symlink", src.display());
        return;
    }
    fs::create_dir_all(dst.join("σ")).unwrap();
    fs::write(dst.join("σ").join("saved.txt"), b"s").unwrap();
    let pattern = format!("{}", src.join("ς").display());

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
        dst.join("σ").join("saved.txt").exists(),
        "the destination's σ/ is excluded and must survive the mirror:\nstdout: {stdout}"
    );
    assert!(
        !dst.join("alias").exists(),
        "the second name is excluded too"
    );
    assert!(dst.join("keep.txt").exists());
    assert!(
        !stderr.contains("cannot match anything"),
        "a pattern under the source must not warn: {stderr}"
    );
    let note = format!(
        "blit: --exclude '{pattern}': 2 names share this entry here; excluding ./alias, ./σ"
    );
    assert_eq!(
        stderr.matches(&note).count(),
        1,
        "one note expected:\n{stderr}"
    );
}

/// cr-g3fix4-2: an excluded entry the destination stores in another case
/// than the source survives a mirror. The exclude is spelled as the source
/// stores it (or as typed); at the destination the deletion scope also
/// names the destination's own spelling, and an any-depth name matches
/// there case-insensitively when the destination ignores case. Before, each
/// case below deleted the destination's `saved.txt` with its directory.
/// Cases: a directory `src/cache` excluded as stored, against `dst/Cache`;
/// a symlink `src/cache` excluded as `CACHE` (Unix); a nested `src/a/b`
/// against `dst/A/B`; and the any-depth `--exclude cache` against
/// `dst/x/Cache`. Skipped where the filesystem is case-sensitive.
#[test]
fn mirror_exclude_keeps_a_destination_entry_stored_in_another_case() {
    let tmp = tempdir().unwrap();
    if !case_insensitive(tmp.path()) {
        eprintln!("skipping: {} is case-sensitive", tmp.path().display());
        return;
    }
    let mut cases: Vec<(&str, &[&str], &str, &str)> = vec![
        ("directory", &["cache/c.txt"], "/cache", "Cache/saved.txt"),
        (
            "nested",
            &["a/b/c.txt", "a/keep.txt"],
            "/a/b",
            "A/B/saved.txt",
        ),
        (
            "any depth",
            &["x/cache/c.txt", "x/keep.txt"],
            "cache",
            "x/Cache/saved.txt",
        ),
    ];
    if cfg!(unix) {
        cases.push(("symlink", &[], "/CACHE", "Cache/saved.txt"));
    }
    // Every case runs; the ones that lost the excluded entry are named
    // together at the end.
    let mut lost = Vec::new();
    for (case, files, exclude, saved) in cases {
        let root = tmp.path().join(case.replace(' ', "_"));
        let (src, dst) = (root.join("src"), root.join("dst"));
        for rel in files.iter().chain(&["keep.txt"]) {
            let path = src.join(rel);
            fs::create_dir_all(path.parent().unwrap()).unwrap();
            fs::write(path, b"x").unwrap();
        }
        #[cfg(unix)]
        if case == "symlink" {
            let outside = root.join("outside");
            fs::create_dir_all(&outside).unwrap();
            fs::write(outside.join("o.txt"), b"o").unwrap();
            std::os::unix::fs::symlink(&outside, src.join("cache")).unwrap();
        }
        let saved = dst.join(saved);
        fs::create_dir_all(saved.parent().unwrap()).unwrap();
        fs::write(&saved, b"s").unwrap();
        let pattern = match exclude.strip_prefix('/') {
            Some(rel) => format!("{}", src.join(rel).display()),
            None => exclude.to_string(),
        };

        let (ok, stdout, stderr) = blit(&[
            "mirror",
            "--yes",
            &with_slash(&src),
            &with_slash(&dst),
            "--exclude",
            &pattern,
        ]);
        assert!(
            ok,
            "{case}: mirror failed:\nstdout: {stdout}\nstderr: {stderr}"
        );
        if !saved.exists() {
            lost.push(format!("{case}: {}", stdout.trim().replace('\n', " | ")));
        }
        assert!(dst.join("keep.txt").exists(), "{case}: keep.txt");
        assert!(
            !stderr.contains("cannot match anything"),
            "{case}: a pattern under the source must not warn: {stderr}"
        );
    }
    assert!(
        lost.is_empty(),
        "the destination's excluded entry must survive the mirror:\n{}",
        lost.join("\n")
    );
}

/// cr-g3fix5-1: an excluded any-depth name keeps the destination's entry
/// stored in a spelling only the destination filesystem pairs with it, and
/// the directory holding it, out of the mirror's reach. The reported runs:
/// `--exclude σ` over `src/x/σ/` against a pre-existing `dst/x/ς/saved.txt`
/// printed "Deleted: 1 file(s), 2 dir(s)" and `dst/x` was gone; so with an
/// NFC `café` against an NFD `dst/x/café/` (and `é` against `É`). Every
/// case runs, and the ones that lost the entry are named together; each is
/// skipped with a message where the filesystem does not pair the spellings
/// (APFS pairs all three).
#[test]
fn mirror_exclude_keeps_a_destination_name_in_a_unicode_equivalent_spelling() {
    let tmp = tempdir().unwrap();
    let mut lost = Vec::new();
    for (case, typed, stored) in [
        ("final sigma", "σ", "ς"),
        ("nfd", "caf\u{e9}", "cafe\u{301}"),
        ("case", "\u{e9}", "\u{c9}"),
    ] {
        let root = tmp.path().join(case.replace(' ', "_"));
        let (src, dst) = (root.join("src"), root.join("dst"));
        fs::create_dir_all(&dst).unwrap();
        if !filesystem_pairs(&dst, stored, typed) {
            eprintln!(
                "skipping {case}: {} does not pair {typed:?} with {stored:?}",
                dst.display()
            );
            continue;
        }
        for rel in [format!("x/{typed}/private.txt"), "keep.txt".to_string()] {
            let path = src.join(rel);
            fs::create_dir_all(path.parent().unwrap()).unwrap();
            fs::write(path, b"x").unwrap();
        }
        let saved = dst.join("x").join(stored).join("saved.txt");
        fs::create_dir_all(saved.parent().unwrap()).unwrap();
        fs::write(&saved, b"s").unwrap();

        let (ok, stdout, stderr) = blit(&[
            "mirror",
            "--yes",
            &with_slash(&src),
            &with_slash(&dst),
            "--exclude",
            typed,
        ]);
        assert!(
            ok,
            "{case}: mirror failed:\nstdout: {stdout}\nstderr: {stderr}"
        );
        if !saved.exists() || !dst.join("x").is_dir() {
            lost.push(format!("{case}: {}", stdout.trim().replace('\n', " | ")));
        }
        assert!(dst.join("keep.txt").exists(), "{case}: keep.txt");
        assert!(
            !dst.join("x").join(stored).join("private.txt").exists(),
            "{case}: the excluded source entry is not copied"
        );
    }
    assert!(
        lost.is_empty(),
        "the destination's excluded entry and its directory must survive the mirror:\n{}",
        lost.join("\n")
    );
}

/// cr-g3fix5-1: the same on a destination daemon, whose deletion scope is
/// built by the same session code from the wire spec: a push mirror with
/// `--exclude σ` keeps the module's `x/ς/saved.txt` and deletes
/// `extra.txt`. Skipped where the filesystem does not pair `ς` with `σ`.
#[cfg(unix)]
#[test]
fn push_mirror_keeps_a_destination_name_in_a_unicode_equivalent_spelling() {
    let ctx = common::TestContext::new();
    let src = ctx.workspace.join("src");
    for rel in ["x/σ/private.txt", "keep.txt"] {
        let path = src.join(rel);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, b"x").unwrap();
    }
    if !filesystem_pairs(&ctx.module_dir, "ς", "σ") {
        eprintln!(
            "skipping: {} does not pair σ with ς",
            ctx.module_dir.display()
        );
        return;
    }
    fs::create_dir_all(ctx.module_dir.join("x/ς")).unwrap();
    fs::write(ctx.module_dir.join("x/ς/saved.txt"), b"s").unwrap();
    fs::write(ctx.module_dir.join("extra.txt"), b"extraneous").unwrap();

    let mut cmd = std::process::Command::new(&ctx.cli_bin);
    cmd.arg("--config-dir")
        .arg(&ctx.config_dir)
        .arg("mirror")
        .arg("--yes")
        .arg(with_slash(&src))
        .arg(format!("127.0.0.1:{}:/test/", ctx.daemon_port))
        .arg("--exclude")
        .arg("σ");
    let output = run_with_timeout(cmd, Duration::from_secs(60));
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        output.status.success(),
        "push mirror failed:\nstdout: {stdout}\nstderr: {stderr}"
    );
    assert!(
        ctx.module_dir.join("x/ς/saved.txt").exists(),
        "the daemon's deletion scope must name the destination's own spelling:\nstdout: {stdout}"
    );
    assert!(ctx.module_dir.join("keep.txt").exists());
    assert!(
        !ctx.module_dir.join("extra.txt").exists(),
        "a complete source scan lets the mirror delete the extraneous file"
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
/// on a pattern the user wrote correctly. cr-g3fix3-2: the escape the
/// name is written as (`./data[[]1[]]`) never names a directory spelled
/// like it — `--exclude <alias>` also left out the unrelated sibling
/// `data[[]1[]]/private.txt`, through the matcher's text comparison.
#[cfg(unix)]
#[test]
fn copy_exclude_through_an_alias_into_a_directory_named_like_a_glob() {
    let tmp = tempdir().unwrap();
    let src = tmp.path().join("src");
    for rel in [
        "data[1]/private.txt",
        "data[1]/keep.dat",
        "data1/private.txt",
        "data[[]1[]]/private.txt",
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
    assert!(
        dst.join("data[[]1[]]/private.txt").exists(),
        "a name the exclude never named must be copied"
    );
    assert!(dst.join("keep.txt").exists());
    assert!(
        !stderr.contains("cannot match anything"),
        "a pattern under the source must not warn: {stderr}"
    );

    let dst = tmp.path().join("dst_bare");
    let pattern = format!("{}", into_class.display());
    let (ok, stdout, stderr) = blit(&[
        "copy",
        &with_slash(&src),
        &with_slash(&dst),
        "--exclude",
        &pattern,
    ]);
    assert!(ok, "copy failed:\nstdout: {stdout}\nstderr: {stderr}");
    assert!(
        !dst.join("data[1]").exists(),
        "the aliased directory is left out"
    );
    for rel in ["data[[]1[]]/private.txt", "data1/private.txt", "keep.txt"] {
        assert!(
            dst.join(rel).exists(),
            "{rel}: a name the exclude never named must be copied"
        );
    }
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

/// cr-g3fix4-3: off Windows a typed `\` is globset's escape, so
/// `--exclude './a\b'` names the top-level `ab` and not `a/b`. The pattern
/// now leaves this host written `./ab` (a Windows peer read the bare `\`
/// as a separator and kept `ab` in its mirror's deletion scope); here it
/// still leaves out `ab` and copies `a/b`, with no warning.
#[cfg(unix)]
#[test]
fn copy_exclude_a_typed_unix_escape_names_the_escaped_character() {
    let tmp = tempdir().unwrap();
    let (src, dst) = (tmp.path().join("src"), tmp.path().join("dst"));
    for rel in ["ab/saved.txt", "a/b/saved.txt", "keep.txt"] {
        let path = src.join(rel);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, b"x").unwrap();
    }
    let (ok, stdout, stderr) = blit(&[
        "copy",
        &with_slash(&src),
        &with_slash(&dst),
        "--exclude",
        r"./a\b",
    ]);
    assert!(ok, "copy failed:\nstdout: {stdout}\nstderr: {stderr}");
    assert!(!dst.join("ab").exists(), "`a\\b` names `ab`");
    assert!(dst.join("a/b/saved.txt").exists(), "`a/b` was never named");
    assert!(dst.join("keep.txt").exists());
    assert!(
        stderr.is_empty() || !stderr.contains("--exclude"),
        "{stderr}"
    );
}

/// cr-g3fix5-3: off Windows `**\/cache` is two `*` and a literal `/cache`
/// — a component before `cache` is required — so it leaves out
/// `a/cache/x` and copies the top-level `cache/needed.txt`. Before the fix
/// the pattern was written `**/cache`, globset's recursive form, which
/// also names the top-level `cache`: the copy left out `needed.txt` and
/// exited 0.
#[cfg(unix)]
#[test]
fn copy_exclude_an_escaped_separator_after_a_recursive_wildcard_keeps_its_meaning() {
    let tmp = tempdir().unwrap();
    let (src, dst) = (tmp.path().join("src"), tmp.path().join("dst"));
    for rel in ["cache/needed.txt", "a/cache/x", "keep.txt"] {
        let path = src.join(rel);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, b"x").unwrap();
    }
    let (ok, stdout, stderr) = blit(&[
        "copy",
        &with_slash(&src),
        &with_slash(&dst),
        "--exclude",
        r"**\/cache",
    ]);
    assert!(ok, "copy failed:\nstdout: {stdout}\nstderr: {stderr}");
    assert!(
        dst.join("cache/needed.txt").exists(),
        "a top-level cache is not named"
    );
    assert!(!dst.join("a/cache").exists(), "a/cache is named");
    assert!(dst.join("keep.txt").exists());
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

/// cr-g3fix4-2: the same on a destination daemon — its deletion scope is
/// built by the same session code from the wire spec: a push of `src/`
/// with `--exclude <src>/cache` leaves the module's `Cache/saved.txt` in
/// place while the extraneous `extra.txt` is deleted. Skipped where the
/// filesystem is case-sensitive.
#[cfg(unix)]
#[test]
fn push_mirror_keeps_a_destination_entry_stored_in_another_case() {
    let ctx = common::TestContext::new();
    let src = ctx.workspace.join("src");
    for rel in ["cache/c.txt", "keep.txt"] {
        let path = src.join(rel);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, b"x").unwrap();
    }
    if !case_insensitive(&ctx.module_dir) {
        eprintln!("skipping: {} is case-sensitive", ctx.module_dir.display());
        return;
    }
    fs::create_dir_all(ctx.module_dir.join("Cache")).unwrap();
    fs::write(ctx.module_dir.join("Cache/saved.txt"), b"s").unwrap();
    fs::write(ctx.module_dir.join("extra.txt"), b"extraneous").unwrap();
    let pattern = format!("{}", src.join("cache").display());

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
    assert!(
        ctx.module_dir.join("Cache/saved.txt").exists(),
        "the daemon's deletion scope must name the destination's own spelling:\nstdout: {stdout}"
    );
    assert!(ctx.module_dir.join("keep.txt").exists());
    assert!(
        !ctx.module_dir.join("extra.txt").exists(),
        "a complete source scan lets the mirror delete the extraneous file"
    );
}
