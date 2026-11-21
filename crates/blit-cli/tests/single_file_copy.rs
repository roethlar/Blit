//! Regression tests for the "single-file source silent no-op" bug.
//!
//! See `docs/bugs/single-file-source-silent-noop.md`. Any local-to-local
//! copy with a file source must actually copy the file, not silently
//! report "0 files" with success.

use std::fs;
use std::time::Duration;

mod common;
use common::{cli_command, run_with_timeout};

fn run_copy(args: &[&str]) -> std::process::Output {
    let mut cmd = cli_command();
    cmd.arg("copy").arg("--yes");
    for a in args {
        cmd.arg(a);
    }
    run_with_timeout(cmd, Duration::from_secs(30))
}

#[test]
fn single_file_to_dir_with_trailing_slash() {
    let tmp = tempfile::tempdir().unwrap();
    let src = tmp.path().join("file.txt");
    let dst = tmp.path().join("dst");
    fs::write(&src, b"hello world").unwrap();
    fs::create_dir_all(&dst).unwrap();

    let dst_arg = format!("{}/", dst.display());
    let output = run_copy(&[&src.to_string_lossy(), &dst_arg]);
    assert!(
        output.status.success(),
        "blit copy failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(fs::read(dst.join("file.txt")).unwrap(), b"hello world");
}

#[test]
fn single_file_to_existing_dir_no_slash() {
    let tmp = tempfile::tempdir().unwrap();
    let src = tmp.path().join("file.txt");
    let dst = tmp.path().join("dst");
    fs::write(&src, b"hello world").unwrap();
    fs::create_dir_all(&dst).unwrap();

    let output = run_copy(&[&src.to_string_lossy(), &dst.to_string_lossy()]);
    assert!(output.status.success());
    assert_eq!(fs::read(dst.join("file.txt")).unwrap(), b"hello world");
}

#[test]
fn single_file_rename_to_exact_path() {
    let tmp = tempfile::tempdir().unwrap();
    let src = tmp.path().join("file.txt");
    let dst_dir = tmp.path().join("dst");
    let renamed = dst_dir.join("renamed.txt");
    fs::write(&src, b"hello world").unwrap();
    fs::create_dir_all(&dst_dir).unwrap();

    let output = run_copy(&[&src.to_string_lossy(), &renamed.to_string_lossy()]);
    assert!(output.status.success());
    assert_eq!(fs::read(&renamed).unwrap(), b"hello world");
}

#[test]
fn single_file_copy_reports_nonzero_files() {
    let tmp = tempfile::tempdir().unwrap();
    let src = tmp.path().join("file.txt");
    let dst = tmp.path().join("dst");
    fs::write(&src, b"hello world").unwrap();
    fs::create_dir_all(&dst).unwrap();

    let dst_arg = format!("{}/", dst.display());
    let output = run_copy(&[&src.to_string_lossy(), &dst_arg]);
    assert!(output.status.success());

    let stdout = String::from_utf8_lossy(&output.stdout);
    // Must report one copied file, not zero — the original bug silently
    // reported 0 while not copying anything. ls-0 moved the count onto its
    // own labelled line; the property under guard is unchanged.
    assert!(
        stdout.contains("• Copied: 1 file(s)"),
        "expected one copied file in summary, got:\n{}",
        stdout
    );
    assert!(
        !stdout.contains("• Copied: 0 file(s)"),
        "the zero-file regression must not return, got:\n{}",
        stdout
    );
}

#[test]
fn single_file_copy_idempotent() {
    let tmp = tempfile::tempdir().unwrap();
    let src = tmp.path().join("file.txt");
    let dst = tmp.path().join("dst");
    fs::write(&src, b"hello world").unwrap();
    fs::create_dir_all(&dst).unwrap();

    let dst_arg = format!("{}/", dst.display());
    // First copy: transfers. ls-0 moved the count off the header onto its
    // own "• Copied:" line, so the Transferred outcome now shows as a
    // header plus a copied line rather than "Copy complete: 1 files".
    let out1 = run_copy(&[&src.to_string_lossy(), &dst_arg]);
    assert!(out1.status.success());
    assert_eq!(fs::read(dst.join("file.txt")).unwrap(), b"hello world");
    let stdout1 = String::from_utf8_lossy(&out1.stdout);
    assert!(
        stdout1.contains("Copy complete in"),
        "first run should report the Transferred header, got:\n{}",
        stdout1
    );
    assert!(
        stdout1.contains("• Copied: 1 file(s)"),
        "first run should report one copied file, got:\n{}",
        stdout1
    );

    // Second copy: skip_unchanged should detect the match and emit a
    // distinct "Up to date" summary — NOT a zero-file copy report, which
    // is the regression this test guards against.
    let out2 = run_copy(&[&src.to_string_lossy(), &dst_arg]);
    assert!(out2.status.success());
    assert_eq!(fs::read(dst.join("file.txt")).unwrap(), b"hello world");
    let stdout2 = String::from_utf8_lossy(&out2.stdout);
    assert!(
        stdout2.contains("Up to date"),
        "second run should report 'Up to date', got:\n{}",
        stdout2
    );
    assert!(
        !stdout2.contains("• Copied: 0 file(s)"),
        "second run must not print a misleading zero-file copy line, got:\n{}",
        stdout2
    );
}

/// ls-0: the end-of-run summary must not read as self-contradictory.
///
/// The owner's field run printed `Mirror complete: 9578 files, 393.01 MiB in
/// 355.23s` immediately above `Repaired metadata on 5445 file(s) — no bytes
/// re-sent` and read it as incoherent: a byte total next to "no bytes
/// re-sent". The two counts describe DISJOINT sets and always did, but
/// nothing on screen said so. This pins the shape that fixes it, against the
/// real binary's real stdout.
#[test]
fn the_summary_separates_copied_files_from_the_byte_rate() {
    let tmp = tempfile::tempdir().unwrap();
    let src = tmp.path().join("src");
    let dst = tmp.path().join("dst");
    fs::create_dir_all(&src).unwrap();
    fs::create_dir_all(&dst).unwrap();
    // Two files so the throughput/workers block is not suppressed as
    // small-transfer noise.
    fs::write(src.join("a.bin"), vec![b'a'; 4096]).unwrap();
    fs::write(src.join("b.bin"), vec![b'b'; 4096]).unwrap();

    let out = run_copy(&[
        &format!("{}/", src.display()),
        &format!("{}/", dst.display()),
    ]);
    assert!(out.status.success());
    let stdout = String::from_utf8_lossy(&out.stdout);

    // The header commits to no count, so it can never sit in tension with a
    // separate repaired-file line beneath it.
    assert!(
        stdout.contains("Copy complete in"),
        "header should carry the duration only, got:\n{stdout}"
    );
    assert!(
        !stdout.contains("complete: 2 files"),
        "the header must not carry a file count again, got:\n{stdout}"
    );
    // Copied files are their own labelled line.
    assert!(
        stdout.contains("• Copied: 2 file(s)"),
        "copied files need their own line, got:\n{stdout}"
    );
    // The rate says what it averages over. Unlabelled, it reads as a copy
    // rate, which is what made 1.11 MiB/s look like a transfer speed when
    // most of the wall clock was scanning.
    assert!(
        stdout.contains("• Average:") && stdout.contains("includes scan and compare"),
        "the rate must state its interval, got:\n{stdout}"
    );
    assert!(
        !stdout.contains("• Throughput:"),
        "the unqualified 'Throughput' label is what misled, got:\n{stdout}"
    );
    // The worker count stays visible. ls-0 pinned the literal `1` because
    // that WAS the effective count and the defect worth seeing; ls-4 raised
    // the default after measuring 2.7× on a local destination, so the
    // assertion moves to the property — the line is present and reports a
    // concurrent count — rather than to a number that encoded the bug.
    let workers_line = stdout
        .lines()
        .find(|line| line.starts_with("• Workers used: "))
        .unwrap_or_else(|| panic!("the effective worker count must stay on screen:\n{stdout}"));
    let reported: usize = workers_line
        .trim_start_matches("• Workers used: ")
        .trim()
        .parse()
        .unwrap_or_else(|err| panic!("unparseable worker count {workers_line:?}: {err}"));
    assert!(
        reported > 1,
        "a normal run must apply concurrently; got {workers_line:?}"
    );
}

#[test]
fn single_file_missing_source_fails() {
    let tmp = tempfile::tempdir().unwrap();
    let missing = tmp.path().join("does-not-exist.txt");
    let dst = tmp.path().join("dst");
    fs::create_dir_all(&dst).unwrap();

    let output = run_copy(&[&missing.to_string_lossy(), &dst.to_string_lossy()]);
    assert!(
        !output.status.success(),
        "blit copy should fail when source is missing"
    );
}

/// cr-ecm9-3: a source that is ONE FILE is filtered under one name — the
/// file's own — by `copy`, by `check`, and by the account of what a list
/// asked for. It was asked about under the empty path in one place and
/// under its name in another:
/// - `--ignore-case --exclude './report.txt'` over `Report.txt`: `copy`
///   left the file out, and `check` with the same options compared it
///   and reported a difference;
/// - `Report.txt` in a `--files-from` list: `check` compared no file and
///   reported success over an unequal pair, and `copy` wrote the file and
///   then ended with exit 2, reporting it missing.
#[test]
fn a_single_file_source_is_one_name_to_copy_and_to_check() {
    let run = |verb: &str, options: &[&str], src: &std::path::Path, dst: &std::path::Path| {
        let mut cmd = cli_command();
        cmd.arg(verb)
            .args(options)
            .arg(src)
            .arg(dst)
            .stdin(std::process::Stdio::null());
        let output = run_with_timeout(cmd, Duration::from_secs(30));
        (
            output.status.code(),
            format!(
                "{}{}",
                String::from_utf8_lossy(&output.stdout),
                String::from_utf8_lossy(&output.stderr)
            ),
        )
    };
    let tmp = tempfile::tempdir().unwrap();
    let (src, dst) = (
        tmp.path().join("src/Report.txt"),
        tmp.path().join("out/Report.txt"),
    );
    let list = tmp.path().join("list.txt");
    fs::write(&list, "Report.txt\n").unwrap();
    let other = tmp.path().join("other.txt");
    fs::write(&other, "other.txt\n").unwrap();
    let reset = || {
        for (path, body) in [(&src, "the new one, longer"), (&dst, "old")] {
            fs::create_dir_all(path.parent().unwrap()).unwrap();
            fs::write(path, body).unwrap();
        }
    };
    let (differs, same) = ("1 difference(s) found", "no differences found");

    // An exclude that names the file only when case is ignored.
    let skipping = ["--ignore-case", "--exclude", "./report.txt"];
    reset();
    let (code, said) = run("check", &skipping, &src, &dst);
    assert_eq!(
        code,
        Some(0),
        "the one file is left out: nothing to compare\n{said}"
    );
    assert!(said.contains(same), "{said}");
    let (code, said) = run("copy", &skipping, &src, &dst);
    assert_eq!(code, Some(0), "{said}");
    assert_eq!(
        fs::read_to_string(&dst).unwrap(),
        "old",
        "and nothing to copy\n{said}"
    );
    // As typed it does not name the file: both see it.
    let exact = ["--exclude", "./report.txt"];
    let (code, said) = run("check", &exact, &src, &dst);
    assert_eq!(code, Some(1), "{said}");
    assert!(said.contains(differs), "{said}");
    let (code, said) = run("copy", &exact, &src, &dst);
    assert_eq!(code, Some(0), "{said}");
    assert_eq!(fs::read_to_string(&dst).unwrap(), "the new one, longer");

    // A list that names the file: both see it, and it is not missing.
    let listed = ["--files-from", list.to_str().unwrap()];
    reset();
    let (code, said) = run("check", &listed, &src, &dst);
    assert_eq!(code, Some(1), "an unequal pair is a difference\n{said}");
    assert!(said.contains(differs), "{said}");
    let (code, said) = run("copy", &listed, &src, &dst);
    assert_eq!(code, Some(0), "copied, and not reported missing\n{said}");
    assert!(!said.contains("did not land"), "{said}");
    assert_eq!(fs::read_to_string(&dst).unwrap(), "the new one, longer");
    // A list that names another file: neither does.
    let unlisted = ["--retries", "0", "--files-from", other.to_str().unwrap()];
    reset();
    let (code, said) = run("check", &unlisted[2..], &src, &dst);
    assert_eq!(code, Some(0), "{said}");
    assert!(said.contains(same), "{said}");
    let (code, said) = run("copy", &unlisted, &src, &dst);
    assert_eq!(fs::read_to_string(&dst).unwrap(), "old", "{said}");
    assert_eq!(code, Some(2), "the name it lists was not found\n{said}");
    assert!(said.contains("other.txt: source: missing"), "{said}");
}

/// cr-ecm10-4: a file copied under another name is still the one entry
/// the source's name stands for. A listed copy `Report.txt` → `Saved.bin`
/// exits 0; `check --checksum` with the same list reported "Missing on
/// destination", exit 1, having looked for `Saved.bin` in a list that
/// names `Report.txt`. With no list it found one match. `check` now asks
/// about both files under the source's name: what a copy with those
/// options writes, a check with them passes.
#[test]
fn a_file_copied_under_another_name_checks_under_the_sources() {
    let run = |verb: &str, options: &[&str], src: &std::path::Path, dst: &std::path::Path| {
        let mut cmd = cli_command();
        cmd.arg(verb)
            .args(options)
            .arg(src)
            .arg(dst)
            .stdin(std::process::Stdio::null());
        let output = run_with_timeout(cmd, Duration::from_secs(30));
        (
            output.status.code(),
            format!(
                "{}{}",
                String::from_utf8_lossy(&output.stdout),
                String::from_utf8_lossy(&output.stderr)
            ),
        )
    };
    let tmp = tempfile::tempdir().unwrap();
    let (src, dst) = (tmp.path().join("Report.txt"), tmp.path().join("Saved.bin"));
    fs::write(&src, "what the report says").unwrap();
    let list = tmp.path().join("list.txt");
    fs::write(&list, "Report.txt\n").unwrap();
    let listed = ["--files-from", list.to_str().unwrap()];

    let (code, said) = run("copy", &listed, &src, &dst);
    assert_eq!(code, Some(0), "{said}");
    assert_eq!(fs::read_to_string(&dst).unwrap(), "what the report says");

    for options in [&["--checksum"][..], &["--checksum", listed[0], listed[1]]] {
        let (code, said) = run("check", options, &src, &dst);
        assert_eq!(code, Some(0), "check {options:?}\n{said}");
        assert!(
            said.contains("1 files match") && !said.contains("Missing"),
            "check {options:?}\n{said}"
        );
    }
    // And a difference is a difference, not a missing file.
    fs::write(&dst, "what it said before!").unwrap();
    let (code, said) = run("check", &["--checksum", listed[0], listed[1]], &src, &dst);
    assert_eq!(code, Some(1), "{said}");
    assert!(
        said.contains("1 difference(s) found") && !said.contains("Missing"),
        "{said}"
    );
    // A pattern is read against the source's name on both sides too: one
    // that names the destination's own name names nothing.
    let (code, said) = run("check", &["--checksum", "--exclude", "*.bin"], &src, &dst);
    assert_eq!(code, Some(1), "{said}");
    assert!(
        said.contains("Differing (1)") && !said.contains("Missing"),
        "the two are compared, and differ\n{said}"
    );
    let (code, said) = run("check", &["--checksum", "--exclude", "*.txt"], &src, &dst);
    assert_eq!(code, Some(0), "left out, as a copy leaves it out\n{said}");
}
