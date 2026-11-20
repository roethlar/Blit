//! EXCLUDE_CASE_MATCHING slice 1 (D-2026-11-20-1, D-2026-11-20-4):
//! `--match-case` and `--ignore-case`, through the real binary.
//!
//! Every case here runs on every OS with no skip: how a pattern is read
//! does not depend on the disk (the plan's "Platform truth"). The controls
//! use a PURE glob (`*cache`), which no filesystem pairs with anything, so
//! "as typed, the entry is not named" holds on a case-insensitive disk too.

use std::fs;
use std::path::Path;
use std::time::Duration;

use tempfile::tempdir;

mod common;
use common::{cli_command, run_with_timeout};

/// Run `blit <args...>` with nobody at the terminal; (exit code, stdout,
/// stderr).
fn blit(args: &[&str]) -> (Option<i32>, String, String) {
    let mut cmd = cli_command();
    cmd.args(args).stdin(std::process::Stdio::null());
    let output = run_with_timeout(cmd, Duration::from_secs(60));
    (
        output.status.code(),
        String::from_utf8_lossy(&output.stdout).into_owned(),
        String::from_utf8_lossy(&output.stderr).into_owned(),
    )
}

fn with_slash(path: &Path) -> String {
    format!("{}/", path.display())
}

fn write(root: &Path, rel: &str) {
    let path = root.join(rel);
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, rel).unwrap();
}

/// A source holding a directory the pattern names only when case is
/// ignored, and a destination holding a destination-only directory of the
/// same kind beside an unrelated extra. Two different directory names, so
/// what was skipped at the source and what was kept at the destination are
/// told apart on a disk that ignores case as well.
fn trees(root: &Path) -> (std::path::PathBuf, std::path::PathBuf) {
    let (src, dst) = (root.join("src"), root.join("dst"));
    write(&src, "keep.txt");
    write(&src, "aCache/from-source.txt");
    write(&dst, "keep.txt");
    write(&dst, "bCACHE/deep/saved.txt");
    write(&dst, "extra.txt");
    (src, dst)
}

/// AC-2: `mirror --ignore-case --exclude '*cache'` leaves `aCache/` out of
/// the copy and keeps a destination-only `bCACHE/` with what it holds,
/// while an entry no pattern names is still deleted. AC-1: without the
/// option — and with `--match-case` typed — the same pattern names neither.
#[test]
fn mirror_ignore_case_reads_the_pattern_the_same_for_the_copy_and_the_delete() {
    let tmp = tempdir().unwrap();
    let (src, dst) = trees(tmp.path());
    let (code, out, err) = blit(&[
        "mirror",
        "--yes",
        "--ignore-case",
        "--exclude",
        "*cache",
        &with_slash(&src),
        &with_slash(&dst),
    ]);
    assert_eq!(code, Some(0), "stdout:\n{out}\nstderr:\n{err}");
    assert!(dst.join("keep.txt").exists());
    assert!(
        !dst.join("aCache").exists(),
        "aCache/ was named by *cache and must not be copied"
    );
    assert!(
        dst.join("bCACHE/deep/saved.txt").exists(),
        "the destination-only bCACHE/ was named by *cache and must be kept"
    );
    assert!(
        !dst.join("extra.txt").exists(),
        "an entry no pattern names is still deleted"
    );

    for exact in [&[][..], &["--match-case"][..]] {
        let tmp = tempdir().unwrap();
        let (src, dst) = trees(tmp.path());
        let mut args = vec!["mirror", "--yes"];
        args.extend_from_slice(exact);
        let (src_arg, dst_arg) = (with_slash(&src), with_slash(&dst));
        args.extend_from_slice(&["--exclude", "*cache", &src_arg, &dst_arg]);
        let (code, out, err) = blit(&args);
        assert_eq!(code, Some(0), "{exact:?}\nstdout:\n{out}\nstderr:\n{err}");
        assert!(
            dst.join("aCache/from-source.txt").exists(),
            "{exact:?}: as typed, *cache does not name aCache/"
        );
        assert!(
            !dst.join("bCACHE").exists(),
            "{exact:?}: as typed, *cache does not name bCACHE/"
        );
    }
}

/// AC-2, beyond ASCII and for a literal name: `--ignore-case` pairs a
/// letter with itself in another case in any alphabet, and never with
/// other letters.
#[test]
fn mirror_ignore_case_pairs_letters_of_any_alphabet_and_nothing_more() {
    let tmp = tempdir().unwrap();
    let (src, dst) = (tmp.path().join("src"), tmp.path().join("dst"));
    write(&src, "keep.txt");
    for rel in [
        "\u{fc}bung/saved.txt",    // übung, for the pattern Übung
        "x/\u{3c2}.txt",           // ς.txt, for the pattern σ*
        "STRA\u{1e9e}E/saved.txt", // STRAẞE, for the pattern straße
        "STRASSE/gone.txt",        // never named by straße
        "extra.txt",
    ] {
        write(&dst, rel);
    }
    let (code, out, err) = blit(&[
        "mirror",
        "--yes",
        "--ignore-case",
        "--exclude",
        "\u{dc}bung",
        "--exclude",
        "\u{3c3}*",
        "--exclude",
        "stra\u{df}e",
        &with_slash(&src),
        &with_slash(&dst),
    ]);
    assert_eq!(code, Some(0), "stdout:\n{out}\nstderr:\n{err}");
    for kept in [
        "\u{fc}bung/saved.txt",
        "x/\u{3c2}.txt",
        "STRA\u{1e9e}E/saved.txt",
    ] {
        assert!(dst.join(kept).exists(), "{kept} is named and kept");
    }
    for deleted in ["STRASSE", "extra.txt"] {
        assert!(!dst.join(deleted).exists(), "{deleted} is named by nothing");
    }
}

/// AC-12: `check` reads the patterns as the transfer did when it is given
/// the same options — no difference reported for what the mirror skipped
/// and kept — and reports both when it is not.
#[test]
fn check_ignore_case_reads_the_patterns_as_the_transfer_did() {
    let tmp = tempdir().unwrap();
    let (src, dst) = trees(tmp.path());
    let (src_arg, dst_arg) = (with_slash(&src), with_slash(&dst));
    let (code, out, err) = blit(&[
        "mirror",
        "--yes",
        "--ignore-case",
        "--exclude",
        "*cache",
        &src_arg,
        &dst_arg,
    ]);
    assert_eq!(code, Some(0), "stdout:\n{out}\nstderr:\n{err}");

    let (code, out, err) = blit(&[
        "check",
        "--ignore-case",
        "--exclude",
        "*cache",
        &src_arg,
        &dst_arg,
    ]);
    assert_eq!(
        code,
        Some(0),
        "the same options: nothing differs\nstdout:\n{out}\nstderr:\n{err}"
    );

    for exact in [&[][..], &["--match-case"][..]] {
        let mut args = vec!["check"];
        args.extend_from_slice(exact);
        args.extend_from_slice(&["--exclude", "*cache", &src_arg, &dst_arg]);
        let (code, out, err) = blit(&args);
        assert_eq!(
            code,
            Some(1),
            "{exact:?}: as typed, both directories count\nstdout:\n{out}\nstderr:\n{err}"
        );
        assert!(
            out.contains("from-source.txt") && out.contains("saved.txt"),
            "{exact:?}\n{out}"
        );
    }
}

/// Slice 1 carries no case mode on the wire (contract 9, slice 3), so a
/// route with a remote end is REFUSED — before any banner or destructive
/// prompt — never run with the option dropped. Fixture as in
/// `checker_pin_gate.rs`: a missing local source and a remote destination
/// select a remote route and cannot reach the network whatever the gate
/// does; "source path does not exist" would mean the run got past it.
#[test]
fn ignore_case_to_a_daemon_is_refused_before_any_banner_or_prompt() {
    for verb in ["copy", "mirror"] {
        let tmp = tempdir().unwrap();
        let missing = with_slash(&tmp.path().join("does-not-exist"));
        let (code, out, err) = blit(&[
            verb,
            "--ignore-case",
            "--exclude",
            "*cache",
            &missing,
            "blit-test-host:/dst/",
        ]);
        let combined = format!("{out}{err}");
        assert_ne!(code, Some(0), "{verb}\n{combined}");
        assert!(
            combined.contains("--ignore-case is not yet carried to a daemon"),
            "{verb}: the refusal names the option\n{combined}"
        );
        for never in [
            "source path does not exist",
            "Continue?",
            "will delete",
            &format!("starting {verb}"),
        ] {
            assert!(
                !combined.contains(never),
                "{verb}: refused before {never:?}\n{combined}"
            );
        }
    }
    // Exact matching needs nothing from the wire: `--match-case` passes the
    // gate on the same route and reaches the dispatch.
    let tmp = tempdir().unwrap();
    let missing = with_slash(&tmp.path().join("does-not-exist"));
    let (_, out, err) = blit(&[
        "copy",
        "--match-case",
        "--exclude",
        "*cache",
        &missing,
        "blit-test-host:/dst/",
    ]);
    assert!(
        format!("{out}{err}").contains("source path does not exist"),
        "{out}{err}"
    );
}

/// cr-ecm1-2: a pattern that compiles as typed and not once widened is
/// REFUSED under `--ignore-case`, with nothing copied or deleted. Before,
/// the exclude set could not be built and every pattern in it was compared
/// as text exactly as written, so `--ignore-case` named LESS than typing
/// nothing: this mirror deleted `SAVED.CFG` and `s`. The pattern is one
/// letter inside as many alternations as the compiler takes as typed
/// (found by asking the binary), which is one too many once the letter is
/// widened.
#[test]
fn a_pattern_that_cannot_be_compiled_widened_is_refused_before_anything_is_deleted() {
    let nested = |depth: usize| format!("{}s{}", "{".repeat(depth), "}".repeat(depth));
    let accepted_as_typed = |depth: usize| {
        let tmp = tempdir().unwrap();
        let (src, dst) = (tmp.path().join("src"), tmp.path().join("dst"));
        fs::create_dir_all(&src).unwrap();
        fs::create_dir_all(&dst).unwrap();
        let (code, _, _) = blit(&[
            "copy",
            "--exclude",
            &nested(depth),
            &with_slash(&src),
            &with_slash(&dst),
        ]);
        code == Some(0)
    };
    // The deepest nesting the binary takes as typed, by bisection.
    let (mut takes, mut refuses) = (1usize, 4096usize);
    assert!(accepted_as_typed(takes) && !accepted_as_typed(refuses));
    while refuses - takes > 1 {
        let middle = takes + (refuses - takes) / 2;
        if accepted_as_typed(middle) {
            takes = middle;
        } else {
            refuses = middle;
        }
    }
    let deep = nested(takes);

    let fixture = || {
        let tmp = tempdir().unwrap();
        let (src, dst) = (tmp.path().join("src"), tmp.path().join("dst"));
        write(&src, "shared");
        for name in ["s", "SAVED.CFG", "saved.cfg"] {
            write(&dst, name);
        }
        (tmp, src, dst)
    };
    let present = |dst: &Path| -> Vec<&'static str> {
        ["s", "SAVED.CFG", "saved.cfg"]
            .into_iter()
            .filter(|name| dst.join(name).exists())
            .collect()
    };

    // As typed, both patterns work: `SAVED.CFG` is named by neither.
    let (_tmp, src, dst) = fixture();
    let (code, out, err) = blit(&[
        "mirror",
        "--yes",
        "--match-case",
        "--exclude",
        "*.cfg",
        "--exclude",
        &deep,
        &with_slash(&src),
        &with_slash(&dst),
    ]);
    assert_eq!(code, Some(0), "{out}{err}");
    assert_eq!(present(&dst), ["s", "saved.cfg"]);

    let (_tmp, src, dst) = fixture();
    let (code, out, err) = blit(&[
        "mirror",
        "--yes",
        "--ignore-case",
        "--exclude",
        "*.cfg",
        "--exclude",
        &deep,
        &with_slash(&src),
        &with_slash(&dst),
    ]);
    let combined = format!("{out}{err}");
    assert_ne!(code, Some(0), "{combined}");
    assert!(
        combined.contains("cannot be compiled with --ignore-case"),
        "{combined}"
    );
    assert_eq!(
        present(&dst),
        ["s", "SAVED.CFG", "saved.cfg"],
        "refused before anything was deleted"
    );
    assert!(
        !dst.join("shared").exists(),
        "refused before anything was copied"
    );
}

/// `move` takes no `--exclude` or `--include` pattern (it refuses every
/// filter: the source-delete step would remove what a filter left behind),
/// so a case mode has nothing to act on there. It is refused by name, with
/// nothing moved — never accepted and ignored.
#[test]
fn move_refuses_a_case_mode_it_has_no_pattern_for() {
    for option in ["--ignore-case", "--match-case"] {
        let tmp = tempdir().unwrap();
        let (src, dst) = (tmp.path().join("src"), tmp.path().join("dst"));
        write(&src, "keep.txt");
        fs::create_dir_all(&dst).unwrap();
        let (code, out, err) = blit(&[
            "move",
            "--yes",
            option,
            &with_slash(&src),
            &with_slash(&dst),
        ]);
        let combined = format!("{out}{err}");
        assert_ne!(code, Some(0), "{option}\n{combined}");
        assert!(
            combined.contains(&format!("move does not support {option}")),
            "{option}\n{combined}"
        );
        assert!(src.join("keep.txt").exists(), "{option}: nothing was moved");
        assert!(
            !dst.join("keep.txt").exists(),
            "{option}: nothing was moved"
        );
    }
}

/// One mode per run: the two options cannot be given together, on a
/// transfer verb or on `check`, and both are documented where the other
/// filter options are.
#[test]
fn the_two_modes_exclude_each_other_and_are_documented() {
    for verb in ["copy", "mirror", "check"] {
        let (code, out, err) = blit(&[verb, "--match-case", "--ignore-case", "a", "b"]);
        assert_eq!(code, Some(2), "{verb}\n{out}{err}");
        assert!(
            err.contains("--match-case") && err.contains("--ignore-case"),
            "{verb}\n{err}"
        );
        let (code, help, _) = blit(&[verb, "--help"]);
        assert_eq!(code, Some(0));
        for option in ["--match-case", "--ignore-case"] {
            assert!(help.contains(option), "{verb} --help names {option}");
        }
    }
}
