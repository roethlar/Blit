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

/// What a run of `blit <args>` against the test's daemons said, and
/// whether it succeeded.
fn daemon_run(ctx: &common::TestContext, args: &[&str]) -> (bool, String) {
    let mut cmd = std::process::Command::new(&ctx.cli_bin);
    cmd.arg("--config-dir")
        .arg(&ctx.config_dir)
        .args(args)
        .stdin(std::process::Stdio::null());
    let output = run_with_timeout(cmd, Duration::from_secs(90));
    (
        output.status.success(),
        format!(
            "{}{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        ),
    )
}

/// Empty `dir` and give it `rels`.
fn refill(dir: &Path, rels: &[&str]) {
    for entry in fs::read_dir(dir).unwrap() {
        let path = entry.unwrap().path();
        if path.is_dir() {
            fs::remove_dir_all(&path).unwrap();
        } else {
            fs::remove_file(&path).unwrap();
        }
    }
    for rel in rels {
        write(dir, rel);
    }
}

/// The source side of every case below: `aCache/` is named by `*cache`
/// only when case is ignored.
const AT_SOURCE: [&str; 2] = ["keep.txt", "aCache/from-source.txt"];
/// The destination side: `bCACHE/` likewise, and `extra.txt` by nothing.
const AT_DESTINATION: [&str; 2] = ["bCACHE/deep/saved.txt", "extra.txt"];

/// After a mirror with `--exclude '*cache'`: under `--ignore-case` the
/// source's `aCache/` was left out and the destination's `bCACHE/` kept;
/// as typed, neither. The unrelated extra is deleted either way.
fn assert_mirrored(dst: &Path, ignores: bool, route: &str, said: &str) {
    assert!(dst.join("keep.txt").exists(), "{route}: keep.txt\n{said}");
    assert!(
        !dst.join("extra.txt").exists(),
        "{route}: extra.txt\n{said}"
    );
    assert_eq!(
        !dst.join("aCache").exists(),
        ignores,
        "{route}, ignoring case {ignores}: the SOURCE end reads the pattern so\n{said}"
    );
    assert_eq!(
        dst.join("bCACHE/deep/saved.txt").exists(),
        ignores,
        "{route}, ignoring case {ignores}: the DESTINATION end reads the pattern so\n{said}"
    );
}

/// AC-6 (slice 3, contract 9): the case mode reaches a daemon at session
/// open, whichever end it is. Pushed to a daemon, the DESTINATION daemon
/// keeps its own `bCACHE/`; pulled from one, the SOURCE daemon leaves its
/// `aCache/` out of what it sends. As typed, and with `--match-case`, the
/// same pattern names neither there. Before contract 9 the wire had no
/// field for the mode and these routes were refused outright.
#[test]
fn mirror_ignore_case_reaches_a_daemon_at_either_end() {
    let ctx = common::TestContext::new();
    let remote = format!("127.0.0.1:{}:/test/", ctx.daemon_port);
    let local = ctx.workspace.join("local");
    fs::create_dir_all(&local).unwrap();

    for (mode, ignores) in [
        (None, false),
        (Some("--match-case"), false),
        (Some("--ignore-case"), true),
    ] {
        // Push: this machine is the source, the daemon the destination.
        refill(&local, &AT_SOURCE);
        refill(&ctx.module_dir, &AT_DESTINATION);
        let mut args = vec!["mirror", "--yes"];
        args.extend(mode);
        let from = with_slash(&local);
        args.extend(["--exclude", "*cache", from.as_str(), remote.as_str()]);
        let (ok, said) = daemon_run(&ctx, &args);
        assert!(ok, "push {mode:?}\n{said}");
        assert_mirrored(&ctx.module_dir, ignores, "push", &said);

        // Pull: the daemon is the source, this machine the destination.
        refill(&ctx.module_dir, &AT_SOURCE);
        refill(&local, &AT_DESTINATION);
        let mut args = vec!["mirror", "--yes"];
        args.extend(mode);
        let to = with_slash(&local);
        args.extend(["--exclude", "*cache", remote.as_str(), to.as_str()]);
        let (ok, said) = daemon_run(&ctx, &args);
        assert!(ok, "pull {mode:?}\n{said}");
        assert_mirrored(&local, ignores, "pull", &said);
    }
}

/// AC-6: daemon to daemon. The asking machine only hands the destination
/// daemon the request; that daemon opens the session to the source daemon
/// itself. The mode rides the whole way: the source daemon leaves
/// `aCache/` out and the destination daemon keeps `bCACHE/`.
#[test]
fn mirror_ignore_case_reaches_both_daemons_of_a_delegated_transfer() {
    let ctx = common::TestContext::new();
    let second = ctx.spawn_second_daemon(
        "daemon_dst",
        &common::DaemonOptions {
            delegation: true,
            ..Default::default()
        },
    );
    let from = format!("127.0.0.1:{}:/test/", ctx.daemon_port);
    let to = format!("127.0.0.1:{}:/test/", second.port);
    for (mode, ignores) in [(None, false), (Some("--ignore-case"), true)] {
        refill(&ctx.module_dir, &AT_SOURCE);
        fs::create_dir_all(&second.module_dir).unwrap();
        refill(&second.module_dir, &AT_DESTINATION);
        let mut args = vec!["mirror", "--yes"];
        args.extend(mode);
        args.extend(["--exclude", "*cache", from.as_str(), to.as_str()]);
        let (ok, said) = daemon_run(&ctx, &args);
        assert!(ok, "daemon to daemon {mode:?}\n{said}");
        assert_mirrored(&second.module_dir, ignores, "daemon to daemon", &said);
    }
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

/// cr-ecm1-3: `--include '[a].txt'` is a class, not a file name. A mirror's
/// deletion scope is what the includes name, so a destination-only file
/// merely SPELLED `[a].txt` was once in scope — named by a text comparison
/// of the pattern, in its typed case only — and deleted, while `[A].TXT`
/// beside it was not. Neither is named now, in either mode.
#[test]
fn an_include_class_does_not_name_a_file_spelled_like_the_pattern() {
    for mode in ["--match-case", "--ignore-case"] {
        let tmp = tempdir().unwrap();
        let (src, dst) = (tmp.path().join("src"), tmp.path().join("dst"));
        write(&src, "a.txt");
        for name in ["[a].txt", "[A].TXT", "B.txt"] {
            write(&dst, name);
        }
        let (code, out, err) = blit(&[
            "mirror",
            "--yes",
            mode,
            "--include",
            "[a].txt",
            &with_slash(&src),
            &with_slash(&dst),
        ]);
        assert_eq!(code, Some(0), "{mode}\n{out}{err}");
        for name in ["a.txt", "[a].txt", "[A].TXT", "B.txt"] {
            assert!(
                dst.join(name).exists(),
                "{mode}: {name} is at the destination"
            );
        }
    }
}

/// cr-ecm2-1: a bracket class matches one byte with `--ignore-case` as
/// without it. `ſ` (two bytes) and the Kelvin sign (three) are fold-mates
/// of `s` and `k`, and the class `[sk]` once named them when case was
/// ignored: `mirror --ignore-case --include '[sk].txt'` put a
/// destination-only `ſ.txt` in the deletion scope and deleted it, which
/// the same command as typed never did. The class gains its ASCII
/// partners and nothing more, for an include and for an exclude, at the
/// source and at the destination alike. (The entries sit in separate
/// folders so that a filesystem which itself pairs `ſ` with `S` holds
/// them all.)
#[test]
fn mirror_ignore_case_keeps_a_class_one_byte_wide() {
    let (long_s, kelvin) = ("ſ.txt", "\u{212a}.txt");
    for (mode, ignores) in [("--match-case", false), ("--ignore-case", true)] {
        // An include: what it names is the deletion scope.
        let tmp = tempdir().unwrap();
        let (src, dst) = (tmp.path().join("src"), tmp.path().join("dst"));
        write(&src, "other.dat");
        write(&dst, long_s);
        write(&dst, kelvin);
        write(&dst, "sub/S.txt");
        let (code, out, err) = blit(&[
            "mirror",
            "--yes",
            mode,
            "--include",
            "[sk].txt",
            &with_slash(&src),
            &with_slash(&dst),
        ]);
        assert_eq!(code, Some(0), "{mode}\n{out}{err}");
        assert!(dst.join(long_s).exists(), "{mode}: the class never names ſ");
        assert!(dst.join(kelvin).exists(), "{mode}: nor the Kelvin sign");
        assert_eq!(
            !dst.join("sub/S.txt").exists(),
            ignores,
            "{mode}: the ASCII partner is in scope only when case is ignored"
        );

        // An exclude: what it names is protected, here and there.
        let tmp = tempdir().unwrap();
        let (src, dst) = (tmp.path().join("src"), tmp.path().join("dst"));
        write(&src, "keep.txt");
        write(&src, "from/ſaved/in-source.txt");
        write(&dst, "one/ſaved/x.txt");
        write(&dst, "two/Saved/x.txt");
        write(&dst, "three/saved/x.txt");
        let (code, out, err) = blit(&[
            "mirror",
            "--yes",
            mode,
            "--exclude",
            "[s]aved",
            &with_slash(&src),
            &with_slash(&dst),
        ]);
        assert_eq!(code, Some(0), "{mode}\n{out}{err}");
        assert!(
            dst.join("from/ſaved/in-source.txt").exists(),
            "{mode}: the source walk does not leave ſaved out"
        );
        assert!(
            !dst.join("one/ſaved").exists(),
            "{mode}: nor does the destination keep it"
        );
        assert!(
            dst.join("three/saved/x.txt").exists(),
            "{mode}: named as typed"
        );
        assert_eq!(
            dst.join("two/Saved/x.txt").exists(),
            ignores,
            "{mode}: the ASCII partner is kept only when case is ignored"
        );
    }
}

/// cr-ecm2-5: help offers the case modes on the verbs that take them —
/// `copy`, `mirror`, `check` — and not on `move`, which refuses both on
/// every invocation. `move` still parses them: the refusal that says why
/// is the next test's.
#[test]
fn help_offers_the_case_modes_only_where_they_are_taken() {
    for (verb, offered) in [
        ("copy", true),
        ("mirror", true),
        ("check", true),
        ("move", false),
    ] {
        for flag in ["--help", "-h"] {
            let (code, out, err) = blit(&[verb, flag]);
            assert_eq!(code, Some(0), "{verb} {flag}\n{err}");
            for option in ["--match-case", "--ignore-case"] {
                assert_eq!(
                    out.contains(option),
                    offered,
                    "{verb} {flag} and {option}:\n{out}"
                );
            }
            // Slice 4 (AC-7, AC-10): the long help of a verb that takes
            // patterns says how one is read — exactly as typed, and `?` and
            // a class one byte — and `move`'s says nothing of patterns.
            let long = flag == "--help";
            for words in [
                "A pattern matches exactly as typed",
                "? and a [class] match one byte",
                "write {a,\u{e9}}.txt, not",
            ] {
                assert_eq!(
                    out.contains(words),
                    offered && long,
                    "{verb} {flag} and {words:?}:\n{out}"
                );
            }
        }
    }
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
