//! UTILITY_VERBS_LOCAL_PARITY slice 2 (AC-3, AC-8, AC-10, AC-11, AC-12):
//! `blit rm` against a real daemon — the Unix command, run on the daemon's
//! own disk — and the question every deleting verb asks.
//!
//! Where the system has the reference (`rm` from GNU coreutils, Linux),
//! what is printed, the exit status and what is left on disk are compared
//! with it. The shape, the options, the question and the refusals are
//! checked on every OS.

use std::fs;
use std::path::Path;
use std::process::{Command, Stdio};
use std::time::Duration;

mod common;
use common::{run_with_timeout, TestContext};

struct Ran {
    code: Option<i32>,
    stdout: String,
    stderr: String,
}

impl Ran {
    fn all(&self) -> String {
        format!("exit {:?}\n{}{}", self.code, self.stdout, self.stderr)
    }
}

/// `blit <args>`; `answer` is what standard input holds — `None` for
/// nothing at all (closed), as under a scheduler.
fn blit(ctx: &TestContext, args: &[&str], answer: Option<&str>) -> Ran {
    let mut cmd = Command::new(&ctx.cli_bin);
    cmd.arg("--config-dir").arg(&ctx.config_dir).args(args);
    match answer {
        Some(text) => {
            let path = ctx.workspace.join("answer.txt");
            fs::write(&path, text).unwrap();
            cmd.stdin(fs::File::open(&path).unwrap());
        }
        None => {
            cmd.stdin(Stdio::null());
        }
    }
    let output = run_with_timeout(cmd, Duration::from_secs(60));
    Ran {
        code: output.status.code(),
        stdout: String::from_utf8_lossy(&output.stdout).into_owned(),
        stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
    }
}

/// The module as an operand, with `below` after it.
fn remote(ctx: &TestContext, below: &str) -> String {
    format!("127.0.0.1:{}:/test/{below}", ctx.daemon_port)
}

fn write(root: &Path, rel: &str) {
    let path = root.join(rel);
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, b"x").unwrap();
}

/// `t/` under `root`: a file, a folder holding files two deep, and an
/// empty folder.
fn tree(root: &Path) {
    write(root, "t/file.txt");
    write(root, "t/other.txt");
    write(root, "t/dir/inner.txt");
    write(root, "t/dir/deep/er.txt");
    fs::create_dir_all(root.join("t/empty")).unwrap();
}

/// AC-3, on every OS: each operand is treated as `rm` treats it. A
/// directory without `-r` is refused in `rm`'s words and nothing in it is
/// touched; an operand that is not there is an error; the other operands
/// are removed all the same; the exit status is 0 only when every operand
/// is gone; and nothing is printed for what was removed.
#[test]
fn rm_treats_each_operand_as_the_unix_command_does() {
    let ctx = TestContext::new();
    let t = ctx.module_dir.join("t");
    tree(&ctx.module_dir);
    let (file, missing, dir, empty) = (
        remote(&ctx, "t/file.txt"),
        remote(&ctx, "t/missing"),
        remote(&ctx, "t/dir"),
        remote(&ctx, "t/empty"),
    );

    let ran = blit(&ctx, &["rm", "-y", &file, &missing, &dir, &empty], None);
    assert_eq!(ran.code, Some(1), "{}", ran.all());
    assert_eq!(ran.stdout, "", "{}", ran.all());
    let said: Vec<&str> = ran.stderr.lines().collect();
    assert_eq!(
        said.len(),
        3,
        "one line for each operand that stayed\n{}",
        ran.all()
    );
    assert!(
        said[0].starts_with(&format!("blit rm: cannot remove '{missing}': ")),
        "{}",
        ran.all()
    );
    assert_eq!(
        said[1],
        format!("blit rm: cannot remove '{dir}': Is a directory")
    );
    assert_eq!(
        said[2],
        format!("blit rm: cannot remove '{empty}': Is a directory")
    );
    assert!(
        !t.join("file.txt").exists(),
        "the operand after it went all the same"
    );
    assert!(
        t.join("dir/inner.txt").exists()
            && t.join("dir/deep/er.txt").exists()
            && t.join("empty").is_dir(),
        "a directory refused is not touched"
    );

    // `-f`: an operand that is not there is no error, and nothing is said.
    // It also means "never ask": no `-y`, nobody to answer.
    let ran = blit(
        &ctx,
        &["rm", "-f", &missing, &remote(&ctx, "t/other.txt")],
        None,
    );
    assert_eq!(
        (ran.code, ran.stdout.as_str(), ran.stderr.as_str()),
        (Some(0), "", "")
    );
    assert!(!t.join("other.txt").exists());

    // `-r`, in each of its spellings: the directory and everything in it.
    for (spelling, operand) in [("-r", "t/dir"), ("-R", "t/empty"), ("--recursive", "t")] {
        let ran = blit(&ctx, &["rm", spelling, "-y", &remote(&ctx, operand)], None);
        assert_eq!(
            (ran.code, ran.stdout.as_str(), ran.stderr.as_str()),
            (Some(0), "", ""),
            "{spelling}"
        );
        assert!(!ctx.module_dir.join(operand).exists(), "{spelling}");
    }
    assert!(ctx.module_dir.is_dir(), "the module itself stands");

    // `--json`: what could not be removed, and how much was.
    tree(&ctx.module_dir);
    let ran = blit(&ctx, &["rm", "-y", "--json", &file, &dir], None);
    assert_eq!(ran.code, Some(1), "{}", ran.all());
    let report: serde_json::Value = serde_json::from_str(&ran.stdout).expect("JSON");
    assert_eq!(report["entries_removed"], 1);
    assert_eq!(report["failures"][0]["path"], dir.as_str());
    assert_eq!(report["failures"][0]["reason"], "Is a directory");
    // Every failure is listed (cr-uv1-5): there is no flag for "more".
    assert!(report.get("failures_truncated").is_none(), "{report}");
    assert_eq!(report["unfinished"], serde_json::json!([]));
}

/// AC-3 and AC-12, where the system command IS the reference: for the
/// same tree and the same operands, `blit rm` prints the lines `rm`
/// prints (its own name before them), ends with the same status and
/// leaves the same entries on disk — a directory with and without `-r`,
/// missing operands with and without `-f`, a link to a directory (removed
/// as the link, its target untouched), a tree holding a directory that
/// cannot be written, which both go on past, and an operand below a
/// directory that cannot be searched, which stops neither (cr-uv1-3).
#[cfg(target_os = "linux")]
#[test]
fn rm_equals_the_system_rm() {
    use std::os::unix::fs::PermissionsExt;

    let ctx = TestContext::new();
    let system_root = ctx.workspace.join("system");
    let is_root = Command::new("id")
        .arg("-u")
        .output()
        .map(|o| o.stdout == b"0\n")
        .unwrap_or(false);

    let build = |root: &Path| {
        let _ = fs::remove_dir_all(root.join("t"));
        tree(root);
        write(root, "t/target/kept.txt");
        std::os::unix::fs::symlink("target", root.join("t/link")).unwrap();
        std::os::unix::fs::symlink("../target", root.join("t/dir/inside-link")).unwrap();
        write(root, "t/locked/inner.txt");
        write(root, "t/locked/sub/deep.txt");
        write(root, "t/open/a.txt");
        fs::set_permissions(root.join("t/locked"), fs::Permissions::from_mode(0o555)).unwrap();
        // Two names that differ by a space at the end (cr-uv1-1).
        write(root, "t/report");
        write(root, "t/report ");
        // A directory that cannot even be searched, and an entry in it.
        write(root, "t/sealed/entry");
        fs::set_permissions(root.join("t/sealed"), fs::Permissions::from_mode(0o000)).unwrap();
    };
    let unlock = |root: &Path| {
        for dir in ["t/locked", "t/sealed"] {
            let dir = root.join(dir);
            if dir.exists() {
                fs::set_permissions(dir, fs::Permissions::from_mode(0o755)).unwrap();
            }
        }
    };
    let left = |root: &Path| -> Vec<String> {
        fn walk(dir: &Path, base: &Path, out: &mut Vec<String>) {
            for entry in fs::read_dir(dir).unwrap() {
                let path = entry.unwrap().path();
                out.push(path.strip_prefix(base).unwrap().display().to_string());
                if path.is_dir() && !path.is_symlink() {
                    walk(&path, base, out);
                }
            }
        }
        let mut out = Vec::new();
        if root.join("t").exists() {
            walk(&root.join("t"), root, &mut out);
        }
        out.sort();
        out
    };

    let mut cases: Vec<(&[&str], &[&str])> = vec![
        (
            &[],
            &["t/file.txt", "t/missing", "t/dir", "t/empty", "t/other.txt"],
        ),
        (&[], &["t/dir/", "t/empty/"]),
        (&["-f"], &["t/missing", "t/file.txt", "t/dir"]),
        (&["-r"], &["t/dir", "t/missing", "t/empty", "t/file.txt"]),
        (&["-r", "-f"], &["t/missing", "t/dir", "t/missing/too"]),
        // A link to a directory is a link: removed without `-r`, and with
        // it; what it points to is not entered.
        (&[], &["t/link"]),
        (&["-r"], &["t/link", "t/dir"]),
        // cr-uv1-1, the operand as typed. A last name of `.` or `..` is
        // refused, `-rf` or not, and the next operand goes.
        (&["-r", "-f"], &["t/dir/.", "t/empty"]),
        (&["-r"], &["t/dir/..", "t/./empty/", "t/dir//"]),
        (&["-r", "-f"], &["t/dir/./", "t/dir//.", "t/empty/.."]),
        // A separator after it: a directory, or nothing — `-f` passes
        // over a file typed so, and leaves it.
        (&[], &["t/file.txt/", "t/other.txt"]),
        (&["-f"], &["t/file.txt/", "t/missing/"]),
        (&["-r"], &["t/file.txt/", "t/dir/", "t/missing/"]),
        // A space at either end is part of a name.
        (&[], &["t/report "]),
        (&[], &[" t/report", "t/ report", "t/report"]),
        // A link typed as a directory, without `-r`.
        (&[], &["t/link/"]),
        (&["-f"], &["t/link/", "t/file.txt"]),
        (&["-r"], &["t"]),
    ];
    if is_root {
        eprintln!("running as root: a directory that cannot be written stops nothing; those cases are skipped");
        cases.retain(|(_, operands)| operands != &["t"]);
    } else {
        cases.push((&["-r"], &["t/locked", "t/open"]));
        cases.push((&["-r", "-f"], &["t/locked/sub", "t/open"]));
        // cr-uv1-3: an operand that cannot be reached is its own failure;
        // the operands before and after it go.
        cases.push((&[], &["t/file.txt", "t/sealed/entry", "t/other.txt"]));
        cases.push((&["-r", "-f"], &["t/sealed/entry", "t/dir", "t/sealed"]));
    }

    for (options, operands) in cases {
        build(&system_root);
        build(&ctx.module_dir);
        let system = Command::new("rm")
            .args(options)
            .args(operands)
            .current_dir(&system_root)
            .env("LC_ALL", "C.UTF-8")
            .stdin(Stdio::null())
            .output()
            .expect("the system rm");
        let typed: Vec<String> = operands
            .iter()
            .map(|operand| remote(&ctx, operand))
            .collect();
        let mut args = vec!["rm", "-y"];
        args.extend(options);
        args.extend(typed.iter().map(String::as_str));
        let ours = blit(&ctx, &args, None);
        unlock(&system_root);
        unlock(&ctx.module_dir);

        let mut theirs: Vec<String> = String::from_utf8_lossy(&system.stderr)
            .lines()
            .map(str::to_string)
            .collect();
        // What ours says of the same entries: its own name before the
        // line, and each operand as it was typed.
        let mut said: Vec<String> = ours
            .stderr
            .lines()
            .map(|line| {
                line.replacen("blit rm: ", "rm: ", 1)
                    .replace(&remote(&ctx, ""), "")
            })
            .collect();
        theirs.sort();
        said.sort();
        assert_eq!(said, theirs, "{options:?} {operands:?}\n{}", ours.all());
        assert_eq!(
            ours.code,
            system.status.code(),
            "{options:?} {operands:?}\n{}",
            ours.all()
        );
        assert_eq!(ours.stdout, "", "{options:?} {operands:?}");
        assert_eq!(
            left(&ctx.module_dir),
            left(&system_root),
            "{options:?} {operands:?}: what is left on disk"
        );
        // Whatever was removed, nothing a link points to was.
        if left(&system_root).iter().any(|entry| entry == "t/target") {
            assert!(
                ctx.module_dir.join("t/target/kept.txt").exists(),
                "{options:?} {operands:?}"
            );
        }
    }
}

/// cr-uv2-4, where the system command IS the reference: `blit rm` asks of
/// a directory what `rm` asks of it. A directory on the way to an operand
/// is searched, not read — in one of mode 0300 `rm` removes a file named
/// to it, and blit answered `Permission denied` and left it. A directory
/// that cannot be read is removed when it is empty, and reported when it
/// is not. For each tree: the same lines, the same status, the same
/// entries left.
#[cfg(target_os = "linux")]
#[test]
fn rm_asks_of_a_directory_what_the_system_rm_asks() {
    use std::os::unix::fs::PermissionsExt;

    let ctx = TestContext::new();
    let system_root = ctx.workspace.join("system");
    if Command::new("id")
        .arg("-u")
        .output()
        .map(|o| o.stdout == b"0\n")
        .unwrap_or(false)
    {
        eprintln!("running as root: no mode stops anything; nothing to compare");
        return;
    }
    /// Files to write, directories to make, modes to set — the deepest
    /// first — and what is typed.
    struct Case {
        files: &'static [&'static str],
        dirs: &'static [&'static str],
        modes: &'static [(&'static str, u32)],
        options: &'static [&'static str],
        operands: &'static [&'static str],
    }
    let cases = [
        // Write and search, no read: the reviewer's case, then a tree,
        // then two such levels.
        Case {
            files: &["t/w/known"],
            dirs: &[],
            modes: &[("t/w", 0o300)],
            options: &[],
            operands: &["t/w/known"],
        },
        Case {
            files: &["t/w/sub/file"],
            dirs: &[],
            modes: &[("t/w", 0o300)],
            options: &["-r"],
            operands: &["t/w/sub"],
        },
        Case {
            files: &["t/w/v/known"],
            dirs: &[],
            modes: &[("t/w/v", 0o300), ("t/w", 0o300)],
            options: &[],
            operands: &["t/w/v/known"],
        },
        // What does stop `rm`: no write, or no search.
        Case {
            files: &["t/x/known"],
            dirs: &[],
            modes: &[("t/x", 0o100)],
            options: &[],
            operands: &["t/x/known"],
        },
        Case {
            files: &["t/r/known"],
            dirs: &[],
            modes: &[("t/r", 0o500)],
            options: &[],
            operands: &["t/r/known"],
        },
        Case {
            files: &["t/n/known"],
            dirs: &[],
            modes: &[("t/n", 0o600)],
            options: &[],
            operands: &["t/n/known"],
        },
        // A directory that cannot be read: empty, it goes.
        Case {
            files: &[],
            dirs: &["t/u"],
            modes: &[("t/u", 0o300)],
            options: &["-r"],
            operands: &["t/u"],
        },
        Case {
            files: &[],
            dirs: &["t/z"],
            modes: &[("t/z", 0o000)],
            options: &["-r"],
            operands: &["t/z"],
        },
        Case {
            files: &["t/in/a"],
            dirs: &["t/in/z"],
            modes: &[("t/in/z", 0o000)],
            options: &["-r"],
            operands: &["t/in"],
        },
        // With something in it, it stays and is named.
        Case {
            files: &["t/v/inner"],
            dirs: &[],
            modes: &[("t/v", 0o300)],
            options: &["-r"],
            operands: &["t/v"],
        },
        Case {
            files: &["t/in/a", "t/in/v/inner"],
            dirs: &[],
            modes: &[("t/in/v", 0o300)],
            options: &["-r"],
            operands: &["t/in"],
        },
        Case {
            files: &["t/in/a", "t/in/v/inner"],
            dirs: &[],
            modes: &[("t/in/v", 0o300)],
            options: &["-r", "-f"],
            operands: &["t/in", "t/missing"],
        },
    ];
    fn unlock(dir: &Path) {
        let _ = fs::set_permissions(dir, fs::Permissions::from_mode(0o755));
        for entry in fs::read_dir(dir).into_iter().flatten().flatten() {
            if entry.file_type().is_ok_and(|kind| kind.is_dir()) {
                unlock(&entry.path());
            }
        }
    }
    fn left(dir: &Path, base: &Path, out: &mut Vec<String>) {
        for entry in fs::read_dir(dir).into_iter().flatten().flatten() {
            let path = entry.path();
            out.push(path.strip_prefix(base).unwrap().display().to_string());
            if path.is_dir() && !path.is_symlink() {
                left(&path, base, out);
            }
        }
    }
    for case in cases {
        let what = format!(
            "rm {:?} {:?} with {:?}",
            case.options, case.operands, case.modes
        );
        for root in [&system_root, &ctx.module_dir] {
            if root.join("t").exists() {
                unlock(&root.join("t"));
                fs::remove_dir_all(root.join("t")).unwrap();
            }
            for file in case.files {
                write(root, file);
            }
            for dir in case.dirs {
                fs::create_dir_all(root.join(dir)).unwrap();
            }
            for (dir, mode) in case.modes {
                fs::set_permissions(root.join(dir), fs::Permissions::from_mode(*mode)).unwrap();
            }
        }
        let system = Command::new("rm")
            .args(case.options)
            .args(case.operands)
            .current_dir(&system_root)
            .env("LC_ALL", "C.UTF-8")
            .stdin(Stdio::null())
            .output()
            .expect("the system rm");
        let typed: Vec<String> = case
            .operands
            .iter()
            .map(|operand| remote(&ctx, operand))
            .collect();
        let mut args = vec!["rm", "-y"];
        args.extend(case.options);
        args.extend(typed.iter().map(String::as_str));
        let ours = blit(&ctx, &args, None);
        let mut remain = [Vec::new(), Vec::new()];
        for (root, out) in [&system_root, &ctx.module_dir].into_iter().zip(&mut remain) {
            unlock(&root.join("t"));
            left(&root.join("t"), root, out);
            out.sort();
        }

        let mut theirs: Vec<String> = String::from_utf8_lossy(&system.stderr)
            .lines()
            .map(str::to_string)
            .collect();
        let mut said: Vec<String> = ours
            .stderr
            .lines()
            .map(|line| {
                line.replacen("blit rm: ", "rm: ", 1)
                    .replace(&remote(&ctx, ""), "")
            })
            .collect();
        theirs.sort();
        said.sort();
        assert_eq!(said, theirs, "{what}\n{}", ours.all());
        assert_eq!(ours.code, system.status.code(), "{what}\n{}", ours.all());
        assert_eq!(remain[1], remain[0], "{what}: what is left on disk");
    }
}

/// cr-uv2-5, where the system commands ARE the reference: a name in a
/// message is written as `rm`, `du`, `df` and `find` each write it, so
/// that it reads back as the name it is. Shown as text, a name that is
/// not text was told of in the words for ANOTHER file — `bad\xff.dat` as
/// the file really called `bad\u{fffd}.dat`. Twenty-two names that are not
/// there, each given to the four commands and to the system's four.
#[cfg(target_os = "linux")]
#[test]
fn a_name_in_a_message_is_written_as_the_system_commands_write_it() {
    use std::ffi::OsStr;
    use std::os::unix::ffi::OsStrExt;

    let ctx = TestContext::new();
    let elsewhere = ctx.workspace.join("system");
    fs::create_dir_all(&elsewhere).unwrap();
    let module = remote(&ctx, "");
    let names: [&[u8]; 22] = [
        b"plain.txt",
        b"two words",
        b"it's",
        b"bad\xff.dat",
        b"\xff",
        b"a\xffb\xfec",
        b"new\nline",
        b"tab\there",
        b"bell\x07",
        b"back\\slash",
        b"quarter\\notes.pst",
        b"quote\"double",
        b"dollar$x",
        "caf\u{e9}".as_bytes(),
        "em\u{2003}sp".as_bytes(),
        b"ctl\x01\x02",
        b"del\x7f",
        b"-dash",
        b" lead",
        b"trail ",
        b"both\xff'q",
        "bad\u{fffd}.dat".as_bytes(),
    ];
    let said = |output: std::process::Output| String::from_utf8_lossy(&output.stderr).into_owned();
    let mut written = std::collections::BTreeSet::new();
    for name in names {
        let shown = String::from_utf8_lossy(name).into_owned();
        let mut operand = module.clone().into_bytes();
        operand.extend_from_slice(name);
        let mut beside = b"./".to_vec();
        beside.extend_from_slice(name);
        for (verb, flags, start) in [
            ("rm", &["-y"][..], name),
            ("du", &[][..], name),
            ("df", &[][..], name),
            ("find", &[][..], &beside[..]),
        ] {
            let mut system = Command::new(verb);
            if verb != "find" {
                system.arg("--");
            }
            let theirs = said(
                system
                    .arg(OsStr::from_bytes(start))
                    .current_dir(&elsewhere)
                    .env("LC_ALL", "C.UTF-8")
                    .stdin(Stdio::null())
                    .output()
                    .expect("the system command"),
            );
            let mut cmd = Command::new(&ctx.cli_bin);
            cmd.arg("--config-dir")
                .arg(&ctx.config_dir)
                .arg(verb)
                .args(flags)
                .arg(OsStr::from_bytes(&operand))
                .stdin(Stdio::null());
            let ours = said(run_with_timeout(cmd, Duration::from_secs(60)));
            // Ours names itself and the daemon; `find` here started at `./`.
            let ours = ours
                .replacen(&format!("blit {verb}: "), &format!("{verb}: "), 1)
                .replace(&module, "");
            let theirs = theirs.replacen("\u{2018}./", "\u{2018}", 1);
            assert_eq!(ours, theirs, "{verb} {shown:?}");
            assert!(
                ours.contains("No such file or directory"),
                "{verb} {shown:?}: {ours}"
            );
            written.insert((verb, ours));
        }
    }
    // Every name is written in its own way by each command: none stands
    // for another.
    assert_eq!(written.len(), names.len() * 4);
}

/// What `find` prints for the one entry of `t` that `pattern` matches is
/// the remote path of `name`, byte for byte, and that line is an operand
/// `du`, `df` and `rm` take — `rm` removing that entry and no other.
#[cfg(unix)]
fn the_line_find_prints_is_taken(ctx: &TestContext, pattern: &str, name: &[u8]) {
    use std::ffi::OsStr;
    use std::os::unix::ffi::OsStrExt;

    let ours = |args: &[&OsStr]| {
        let mut cmd = Command::new(&ctx.cli_bin);
        cmd.arg("--config-dir")
            .arg(&ctx.config_dir)
            .args(args)
            .stdin(Stdio::null());
        run_with_timeout(cmd, Duration::from_secs(60))
    };
    let start = remote(ctx, "t");
    let found = ours(&[
        OsStr::new("find"),
        OsStr::new(&start),
        OsStr::new("-name"),
        OsStr::new(pattern),
    ]);
    let line = found.stdout.strip_suffix(b"\n").expect("one line").to_vec();
    let mut expected = remote(ctx, "t/").into_bytes();
    expected.extend_from_slice(name);
    assert_eq!(line, expected, "what `find` prints");
    let line = OsStr::from_bytes(&line);

    let usage = ours(&[OsStr::new("du"), line]);
    assert!(
        usage.status.success() && usage.stdout.ends_with(&[&expected[..], b"\n"].concat()),
        "du: {}{}",
        String::from_utf8_lossy(&usage.stdout),
        String::from_utf8_lossy(&usage.stderr)
    );
    let space = ours(&[OsStr::new("df"), line]);
    let row = space
        .stdout
        .split(|byte| *byte == b'\n')
        .nth(1)
        .unwrap_or_default();
    assert!(
        space.status.success() && row.starts_with(&[&expected[..], b" "].concat()),
        "df prints the operand by its bytes: {}{}",
        String::from_utf8_lossy(&space.stdout),
        String::from_utf8_lossy(&space.stderr)
    );
    let removed = ours(&[OsStr::new("rm"), OsStr::new("-y"), line]);
    assert!(
        removed.status.success() && removed.stderr.is_empty(),
        "rm: {}",
        String::from_utf8_lossy(&removed.stderr)
    );
    assert!(!ctx
        .module_dir
        .join("t")
        .join(OsStr::from_bytes(name))
        .exists());
    assert!(ctx.module_dir.join("t/plain.txt").exists());
}

/// cr-uv2-5: a name with a `\` in it — one Unix allows — is a path these
/// commands take. `find` printed `HOST:/module/t/quarter\notes.pst`, and
/// that line given back to `du`, `df` or `rm` was "a local path".
#[cfg(unix)]
#[test]
fn a_name_these_commands_print_is_one_they_take() {
    let ctx = TestContext::new();
    write(&ctx.module_dir, "t/quarter\\notes.pst");
    write(&ctx.module_dir, "t/plain.txt");
    the_line_find_prints_is_taken(&ctx, "quarter*", b"quarter\\notes.pst");
}

/// cr-uv2-5: and `df` prints the operand by its own bytes: a name that is
/// not text was printed as the text nearest it, which is another file's
/// name. cr-uv3-8: asked apart from the name above, and only where the
/// filesystem takes such a name — one that takes none (APFS) refused this
/// fixture, and the `\` case, which it does take, was never asked.
#[cfg(unix)]
#[test]
fn a_name_that_is_not_text_is_printed_as_one_these_commands_take() {
    use std::ffi::OsStr;
    use std::os::unix::ffi::OsStrExt;

    let ctx = TestContext::new();
    write(&ctx.module_dir, "t/plain.txt");
    let name = b"bad\xff.dat";
    if fs::write(ctx.module_dir.join("t").join(OsStr::from_bytes(name)), b"x").is_err() {
        eprintln!("this filesystem refuses a name that is not text: nothing to print");
        return;
    }
    the_line_find_prints_is_taken(&ctx, "bad*", name);
}

/// cr-uv3-7: `--json` holds text, and a name need not be text. `find
/// --json` gave one string twice for two names — one holding a byte that
/// is no text, the other the character shown in its place — and `rm
/// --json` two failures that read alike. A name that is not text has its
/// bytes beside the text, and they read back; a name that is text has
/// nothing added.
#[cfg(unix)]
#[test]
fn json_tells_a_name_that_is_not_text_from_the_text_shown_for_it() {
    use std::ffi::{OsStr, OsString};
    use std::os::unix::ffi::{OsStrExt, OsStringExt};

    let ctx = TestContext::new();
    write(&ctx.module_dir, "t/plain.txt");
    let json = |args: &[&OsStr]| -> (Option<i32>, serde_json::Value) {
        let mut cmd = Command::new(&ctx.cli_bin);
        cmd.arg("--config-dir")
            .arg(&ctx.config_dir)
            .args(args)
            .stdin(Stdio::null());
        let ran = run_with_timeout(cmd, Duration::from_secs(60));
        let said = serde_json::from_slice(&ran.stdout).unwrap_or_else(|error| {
            panic!(
                "{error}: {}{}",
                String::from_utf8_lossy(&ran.stdout),
                String::from_utf8_lossy(&ran.stderr)
            )
        });
        (ran.status.code(), said)
    };
    let below = |name: &[u8]| -> OsString {
        let mut typed = remote(&ctx, "t/").into_bytes();
        typed.extend_from_slice(name);
        OsString::from_vec(typed)
    };
    // The bytes an entry gives for its name, if it gives any.
    let bytes_of = |entry: &serde_json::Value| -> Option<Vec<u8>> {
        entry.get("raw").map(|raw| {
            blit_core::raw_name::unescape_raw(raw.as_str().expect("text")).expect("reads back")
        })
    };
    let (not_text, alike) = (&b"bad\xff.dat"[..], "bad\u{fffd}.dat".as_bytes());
    let shown_alike = |entry: &&serde_json::Value| {
        entry["path"]
            .as_str()
            .is_some_and(|path| path.ends_with("bad\u{fffd}.dat"))
    };

    // `rm`, of two operands that are not there: nothing need hold them.
    let (code, said) = json(&[
        OsStr::new("rm"),
        OsStr::new("-y"),
        OsStr::new("--json"),
        &below(not_text),
        &below(alike),
    ]);
    assert_eq!(code, Some(1), "{said}");
    let failures = said["failures"].as_array().expect("failures");
    assert_eq!(failures.len(), 2, "{said}");
    assert_eq!(failures[0]["path"], failures[1]["path"], "one text: {said}");
    assert!(
        bytes_of(&failures[0]).is_some_and(|bytes| bytes.ends_with(not_text)),
        "{said}"
    );
    assert!(
        failures[1].get("raw").is_none(),
        "a name that is text is its text: {said}"
    );

    // `find` and `du`, where the filesystem takes such a name.
    let t = ctx.module_dir.join("t");
    if fs::create_dir(t.join(OsStr::from_bytes(not_text))).is_err() {
        eprintln!("this filesystem refuses a name that is not text: `rm` alone was asked");
        return;
    }
    fs::create_dir(t.join(OsStr::from_bytes(alike))).unwrap();

    let (code, found) = json(&[
        OsStr::new("find"),
        OsStr::new(&remote(&ctx, "t")),
        OsStr::new("-name"),
        OsStr::new("bad*"),
        OsStr::new("--json"),
    ]);
    assert_eq!(code, Some(0), "{found}");
    let paths = found["paths"].as_array().expect("paths");
    assert_eq!(paths.len(), 2, "{found}");
    assert_eq!(paths[0], paths[1], "one text, twice: {found}");
    let raw_paths = found["raw_paths"].as_array().expect("raw_paths");
    assert_eq!(raw_paths.len(), 1, "one of them is not text: {found}");
    let at = raw_paths[0]["index"].as_u64().expect("a place in `paths`") as usize;
    assert!(at < 2, "{found}");
    assert!(
        bytes_of(&raw_paths[0]).is_some_and(|bytes| bytes.ends_with(not_text)),
        "{found}"
    );
    // The line `find` prints for it is those bytes.
    let mut printed = Command::new(&ctx.cli_bin);
    printed
        .arg("--config-dir")
        .arg(&ctx.config_dir)
        .args(["find", &remote(&ctx, "t"), "-name", "bad*"])
        .stdin(Stdio::null());
    let printed = run_with_timeout(printed, Duration::from_secs(60)).stdout;
    let line = printed.split(|byte| *byte == b'\n').nth(at).expect("line");
    assert_eq!(bytes_of(&raw_paths[0]).as_deref(), Some(line), "{found}");

    let (code, usage) = json(&[
        OsStr::new("du"),
        OsStr::new("--json"),
        OsStr::new(&remote(&ctx, "t")),
    ]);
    assert_eq!(code, Some(0), "{usage}");
    let rows: Vec<&serde_json::Value> = usage["rows"]
        .as_array()
        .expect("rows")
        .iter()
        .filter(shown_alike)
        .collect();
    assert_eq!(rows.len(), 2, "{usage}");
    let given: Vec<Vec<u8>> = rows.iter().filter_map(|row| bytes_of(row)).collect();
    assert_eq!(given.len(), 1, "one of them is not text: {usage}");
    assert!(given[0].ends_with(not_text), "{usage}");
}

/// cr-uv3-3: a name another system would read as a PATH is a name here.
/// `find` printed the module-root entries `\notes` and `C:receipt`, and
/// `du` refused both printed operands as "Windows paths": the daemon read
/// a typed path with a check made for transfers between unlike systems.
/// A path is its components between `/`, each one name — at the module's
/// root and below it.
#[cfg(unix)]
#[test]
fn a_name_another_system_reads_as_a_path_is_a_name_here() {
    use std::ffi::OsStr;

    let ctx = TestContext::new();
    for name in [
        "\\notes",
        "C:receipt",
        "a:b\\c",
        "t/\\notes",
        "t/D:\\x",
        "kept.txt",
    ] {
        write(&ctx.module_dir, name);
    }
    let ours = |args: &[&str]| {
        let mut cmd = Command::new(&ctx.cli_bin);
        cmd.arg("--config-dir")
            .arg(&ctx.config_dir)
            .args(args.iter().map(OsStr::new))
            .stdin(Stdio::null());
        run_with_timeout(cmd, Duration::from_secs(60))
    };
    for (start, pattern, name) in [
        ("", "?notes", "\\notes"),
        ("", "C:*", "C:receipt"),
        ("", "a:*", "a:b\\c"),
        ("t", "?notes", "t/\\notes"),
        ("t", "D:*", "t/D:\\x"),
    ] {
        let found = ours(&[
            "find",
            &remote(&ctx, start),
            "-maxdepth",
            "1",
            "-name",
            pattern,
        ]);
        let printed = String::from_utf8_lossy(&found.stdout)
            .trim_end()
            .to_string();
        assert_eq!(printed, remote(&ctx, name), "what `find` prints for {name}");

        let usage = ours(&["du", &printed]);
        assert!(
            usage.status.success()
                && String::from_utf8_lossy(&usage.stdout)
                    .trim_end()
                    .ends_with(&printed),
            "du {name}: {}{}",
            String::from_utf8_lossy(&usage.stdout),
            String::from_utf8_lossy(&usage.stderr)
        );
        let space = ours(&["df", &printed]);
        assert!(
            space.status.success(),
            "df {name}: {}",
            String::from_utf8_lossy(&space.stderr)
        );
        let removed = ours(&["rm", "-y", &printed]);
        assert!(
            removed.status.success() && removed.stderr.is_empty(),
            "rm {name}: {}",
            String::from_utf8_lossy(&removed.stderr)
        );
        assert!(!ctx.module_dir.join(name).exists(), "{name} is gone");
    }
    // That file, and no other: nothing was read as a path it is not.
    assert!(ctx.module_dir.join("kept.txt").exists() && ctx.module_dir.join("t").is_dir());
    assert_eq!(fs::read_dir(ctx.module_dir.join("t")).unwrap().count(), 0);
}

/// AC-12, on every Unix: a name that is not valid text is removed as the
/// name it is, with the tree that holds it.
#[cfg(unix)]
#[test]
fn rm_removes_a_name_that_is_not_text() {
    use std::os::unix::ffi::OsStrExt;
    let ctx = TestContext::new();
    let dir = ctx.module_dir.join("raw");
    fs::create_dir_all(&dir).unwrap();
    let name = std::ffi::OsStr::from_bytes(b"bad\xff.txt");
    if fs::write(dir.join(name), b"x").is_err() {
        eprintln!("this filesystem refuses a name that is not text: nothing to remove");
        return;
    }
    write(&dir, "plain.txt");
    let ran = blit(&ctx, &["rm", "-r", "-y", &remote(&ctx, "raw")], None);
    assert_eq!(
        (ran.code, ran.stdout.as_str(), ran.stderr.as_str()),
        (Some(0), "", "")
    );
    assert!(!dir.exists());
}

/// AC-3 and AC-10: `rm` asks once, before anything is sent, naming the
/// operands as typed. "No" stops it with nothing removed and exit 0; with
/// nobody to answer it stops with nothing removed, exit 1, and names the
/// options that skip the question; "yes" goes ahead. `-y` skips the
/// question and so does `-f`, as `rm -f` never asks.
#[test]
fn rm_asks_once_before_anything_is_sent() {
    let ctx = TestContext::new();
    let t = ctx.module_dir.join("t");
    tree(&ctx.module_dir);
    let (file, dir) = (remote(&ctx, "t/file.txt"), remote(&ctx, "t/dir"));
    let question = format!("Remove '{file}' and '{dir}'? [y/N]: ");
    let untouched = || t.join("file.txt").exists() && t.join("dir/inner.txt").exists();

    let no = blit(&ctx, &["rm", "-r", &file, &dir], Some("n\n"));
    assert_eq!(no.code, Some(0), "{}", no.all());
    assert_eq!(no.stdout, format!("{question}Aborted.\n"), "{}", no.all());
    assert!(untouched(), "no: nothing removed");

    let nobody = blit(&ctx, &["rm", "-r", &file, &dir], None);
    assert_eq!(nobody.code, Some(1), "{}", nobody.all());
    assert!(nobody.stdout.starts_with(&question), "{}", nobody.all());
    assert!(!nobody.stdout.contains("Aborted."), "{}", nobody.all());
    assert!(
        nobody.stderr.contains("no answer could be read") && nobody.stderr.contains("-y (or -f)"),
        "{}",
        nobody.all()
    );
    assert!(untouched(), "nobody to answer: nothing removed");

    let yes = blit(&ctx, &["rm", "-r", &file, &dir], Some("y\n"));
    assert_eq!(yes.code, Some(0), "{}", yes.all());
    assert_eq!(yes.stdout, question, "{}", yes.all());
    assert!(!t.join("file.txt").exists() && !t.join("dir").exists());

    // Neither option leaves a question to answer.
    for option in ["-y", "-f"] {
        tree(&ctx.module_dir);
        let ran = blit(&ctx, &["rm", "-r", option, &file, &dir], None);
        assert_eq!(
            (ran.code, ran.stdout.as_str(), ran.stderr.as_str()),
            (Some(0), "", ""),
            "{option}"
        );
        assert!(
            !t.join("file.txt").exists() && !t.join("dir").exists(),
            "{option}"
        );
    }
}

/// AC-10, the other verbs that ask: `mirror` and `move` between two local
/// folders. "No" stops each with nothing done and exit 0; with nobody to
/// answer each stops with nothing done, exit 1, and names `-y`. Both used
/// to print "Aborted." and exit 0 there, so an unattended job that did
/// nothing looked like one that had run.
#[test]
fn mirror_and_move_fail_when_nobody_can_answer() {
    let ctx = TestContext::new();
    let (src, dst) = (ctx.workspace.join("src"), ctx.workspace.join("dst"));
    let (from, to) = (format!("{}/", src.display()), format!("{}/", dst.display()));
    for verb in ["mirror", "move"] {
        for answer in [Some("n\n"), None] {
            let _ = fs::remove_dir_all(&src);
            let _ = fs::remove_dir_all(&dst);
            write(&src, "new.txt");
            write(&dst, "only-here.txt");
            let ran = blit(&ctx, &[verb, &from, &to], answer);
            assert!(
                src.join("new.txt").exists()
                    && dst.join("only-here.txt").exists()
                    && !dst.join("new.txt").exists(),
                "{verb}, {answer:?}: nothing copied, deleted or moved\n{}",
                ran.all()
            );
            match answer {
                Some(_) => {
                    assert_eq!(ran.code, Some(0), "{verb}\n{}", ran.all());
                    assert!(ran.stdout.contains("Aborted."), "{verb}\n{}", ran.all());
                }
                None => {
                    assert_eq!(ran.code, Some(1), "{verb}\n{}", ran.all());
                    assert!(!ran.stdout.contains("Aborted."), "{verb}\n{}", ran.all());
                    assert!(
                        ran.stderr.contains("no answer could be read")
                            && ran.stderr.contains("pass -y"),
                        "{verb}\n{}",
                        ran.all()
                    );
                }
            }
        }
    }
}

/// cr-uv1-1, the one place `blit rm` is deliberately NOT `rm`: a link
/// typed as a directory, with `-r`. `rm -r link/` empties the directory
/// the link points to and then says `Not a directory`; `rm -rf link/` does
/// the same and says nothing. `blit rm` says the same words and exits with
/// the same status, and removes NOTHING — it never removes through a link.
#[cfg(unix)]
#[test]
fn rm_never_removes_through_a_link_typed_as_a_directory() {
    let ctx = TestContext::new();
    tree(&ctx.module_dir);
    write(&ctx.module_dir, "t/target/kept.txt");
    write(&ctx.module_dir, "t/target/inner/deep.txt");
    std::os::unix::fs::symlink("target", ctx.module_dir.join("t/link")).unwrap();
    let link = remote(&ctx, "t/link/");
    let whole = || {
        ctx.module_dir.join("t/target/kept.txt").exists()
            && ctx.module_dir.join("t/target/inner/deep.txt").exists()
            && ctx.module_dir.join("t/link").symlink_metadata().is_ok()
    };

    let ran = blit(&ctx, &["rm", "-y", "-r", &link], None);
    assert_eq!(
        (ran.code, ran.stdout.as_str(), ran.stderr.as_str()),
        (
            Some(1),
            "",
            format!("blit rm: cannot remove '{link}': Not a directory\n").as_str()
        ),
        "{}",
        ran.all()
    );
    assert!(whole(), "nothing was removed through the link");

    let ran = blit(&ctx, &["rm", "-rf", &link], None);
    assert_eq!(
        (ran.code, ran.stdout.as_str(), ran.stderr.as_str()),
        (Some(0), "", ""),
        "{}",
        ran.all()
    );
    assert!(whole(), "nor with -f");
}

/// cr-uv1-1, the reviewer's three commands, on every OS (the system's
/// own `rm` is the reference on Linux, above). `rm -rf work/.` deleted
/// `work`; `rm doc/` unlinked a file; of two names differing by a space at
/// the end, the wrong one went.
#[test]
fn rm_removes_what_was_typed_and_nothing_tidier() {
    let ctx = TestContext::new();
    write(&ctx.module_dir, "work/a.txt");
    write(&ctx.module_dir, "work/sub/b.txt");
    write(&ctx.module_dir, "doc");
    write(&ctx.module_dir, "also");

    let dot = remote(&ctx, "work/.");
    let ran = blit(&ctx, &["rm", "-rf", &dot, &remote(&ctx, "also")], None);
    assert_eq!(ran.code, Some(1), "{}", ran.all());
    assert_eq!(
        ran.stderr,
        format!("blit rm: refusing to remove '.' or '..' directory: skipping '{dot}'\n"),
        "{}",
        ran.all()
    );
    assert!(ctx.module_dir.join("work/sub/b.txt").exists());
    assert!(
        !ctx.module_dir.join("also").exists(),
        "the next operand went"
    );

    // Nothing but such operands: nothing to ask about, nothing sent.
    let ran = blit(&ctx, &["rm", "-r", &remote(&ctx, "work/..")], None);
    assert_eq!(ran.code, Some(1), "{}", ran.all());
    assert!(
        ran.stderr.contains("skipping") && !ran.stdout.contains("Remove"),
        "{}",
        ran.all()
    );
    assert!(ctx.module_dir.join("work/a.txt").exists());

    let slash = remote(&ctx, "doc/");
    let ran = blit(&ctx, &["rm", "-y", &slash], None);
    assert_eq!(ran.code, Some(1), "{}", ran.all());
    assert_eq!(
        ran.stderr,
        format!("blit rm: cannot remove '{slash}': Not a directory\n")
    );
    assert!(ctx.module_dir.join("doc").exists());
    let ran = blit(&ctx, &["rm", "-f", &slash], None);
    assert_eq!(
        (ran.code, ran.stderr.as_str()),
        (Some(0), ""),
        "{}",
        ran.all()
    );
    assert!(ctx.module_dir.join("doc").exists(), "-f passes over it");

    // The question names what was typed, to the letter.
    let ran = blit(&ctx, &["rm", &slash], Some("n\n"));
    assert!(
        ran.stdout.contains(&format!("Remove '{slash}'?")),
        "{}",
        ran.all()
    );

    // A name ending in a space cannot be made on every filesystem.
    #[cfg(unix)]
    {
        write(&ctx.module_dir, "report");
        write(&ctx.module_dir, "report ");
        let ran = blit(&ctx, &["rm", "-y", &remote(&ctx, "report ")], None);
        assert_eq!(
            (ran.code, ran.stderr.as_str()),
            (Some(0), ""),
            "{}",
            ran.all()
        );
        assert!(
            ctx.module_dir.join("report").exists() && !ctx.module_dir.join("report ").exists(),
            "the name with the space, and only it"
        );
    }
}

/// AC-8 and AC-3's refusals: what `rm` cannot do is refused before
/// anything is asked or removed — an option it does not take (named), an
/// operand that is not on a daemon, a whole module — and a module that is
/// read-only refuses every operand.
#[test]
fn rm_refuses_what_it_cannot_do_before_removing_anything() {
    let ctx = TestContext::new();
    tree(&ctx.module_dir);
    let file = remote(&ctx, "t/file.txt");
    let kept = || ctx.module_dir.join("t/file.txt").exists();

    for option in ["-i", "-v", "--one-file-system", "--no-preserve-root", "-d"] {
        let ran = blit(&ctx, &["rm", "-y", option, &file], None);
        assert_eq!(ran.code, Some(2), "{option}\n{}", ran.all());
        assert!(
            ran.stderr.contains(option),
            "{option} is named\n{}",
            ran.all()
        );
        assert!(kept(), "{option}");
    }
    let none = blit(&ctx, &["rm", "-y"], None);
    assert_eq!(none.code, Some(2), "{}", none.all());

    // One operand that cannot be one refuses the command whole.
    let local = ctx.workspace.join("local.txt");
    fs::write(&local, b"x").unwrap();
    let ran = blit(&ctx, &["rm", "-y", &file, local.to_str().unwrap()], None);
    assert_ne!(ran.code, Some(0), "{}", ran.all());
    assert!(
        ran.stderr.contains("only supports remote paths"),
        "{}",
        ran.all()
    );
    assert!(kept() && local.exists());

    for module in [remote(&ctx, ""), remote(&ctx, ".")] {
        let ran = blit(&ctx, &["rm", "-r", "-f", &file, &module], None);
        assert_ne!(ran.code, Some(0), "{module}\n{}", ran.all());
        assert!(
            ran.stderr.contains("entire module"),
            "{module}\n{}",
            ran.all()
        );
        assert!(kept(), "{module}");
    }

    let read_only = TestContext::new_read_only();
    tree(&read_only.module_dir);
    let file = remote(&read_only, "t/file.txt");
    let ran = blit(&read_only, &["rm", "-y", &file], None);
    assert_eq!(ran.code, Some(1), "{}", ran.all());
    assert!(
        ran.stderr
            .contains(&format!("blit rm: cannot remove '{file}': "))
            && ran.stderr.contains("read-only"),
        "{}",
        ran.all()
    );
    assert!(read_only.module_dir.join("t/file.txt").exists());
}

/// AC-11, an asker of another contract against this daemon: the removal
/// is refused before anything is removed — an older asker and a newer one
/// alike — with both versions named; a whole module is refused by the
/// daemon itself, whoever asks, as a failure of that operand. (A daemon too old to know the request at
/// all is `remote_remote.rs`'s fake.)
#[test]
fn a_daemon_refuses_rm_from_another_contract() {
    use blit_core::generated::blit_client::BlitClient;
    use blit_core::generated::RmRequest;
    use blit_core::transfer_session::CONTRACT_VERSION;

    let ctx = TestContext::new();
    tree(&ctx.module_dir);
    let request = |contract_version: u32, path: &str| RmRequest {
        sent_tickets: Vec::new(),
        contract_version,
        module: "test".into(),
        paths: vec![path.to_string()],
        raw_paths: Vec::new(),
        recursive: true,
        force: true,
    };
    let runtime = tokio::runtime::Runtime::new().unwrap();
    runtime.block_on(async {
        let mut client = BlitClient::connect(format!("http://127.0.0.1:{}", ctx.daemon_port))
            .await
            .expect("connect");
        for version in [CONTRACT_VERSION - 1, CONTRACT_VERSION + 1] {
            let status = client
                .rm(request(version, "t"))
                .await
                .expect_err("rm from another contract");
            assert_eq!(status.code(), tonic::Code::FailedPrecondition);
            assert!(
                status
                    .message()
                    .contains(&format!("daemon v{CONTRACT_VERSION}"))
                    && status.message().contains(&format!("client v{version}")),
                "{}",
                status.message()
            );
            assert!(
                ctx.module_dir.join("t/dir/inner.txt").exists(),
                "nothing removed"
            );
        }
        // The module's root is refused by the daemon itself, whoever asks
        // — as that operand's failure (cr-uv1-3), with nothing removed.
        for root in ["", ".", "./"] {
            let (failures, removed) = answered(&mut client, request(CONTRACT_VERSION, root)).await;
            assert_eq!(
                removed,
                Some(0),
                "{root:?}: an answer that ends with `done`"
            );
            assert_eq!(failures.len(), 1, "{root:?} {failures:?}");
            assert!(
                failures[0].operand == 0
                    && failures[0].relative_path.is_empty()
                    && failures[0].reason.contains("the module's root"),
                "{root:?} {failures:?}"
            );
            assert!(ctx.module_dir.join("t/file.txt").exists(), "{root:?}");
        }
        // The same request from this contract is answered.
        let (failures, removed) = answered(&mut client, request(CONTRACT_VERSION, "t")).await;
        assert!(
            failures.is_empty() && removed.is_some_and(|removed| removed >= 8),
            "{failures:?} {removed:?}"
        );
        assert!(!ctx.module_dir.join("t").exists());
    });
}

/// A removal's whole answer, read to its end: the failures it reported,
/// and the count it finished with — `None` if it never said it was done.
async fn answered(
    client: &mut blit_core::generated::blit_client::BlitClient<tonic::transport::Channel>,
    request: blit_core::generated::RmRequest,
) -> (Vec<blit_core::generated::RmFailure>, Option<u64>) {
    use blit_core::generated::rm_reply::Reply;
    let mut answer = client
        .rm(request)
        .await
        .expect("an answer for the request")
        .into_inner();
    let (mut failures, mut removed) = (Vec::new(), None);
    while let Some(reply) = answer.message().await.expect("a message") {
        match reply.reply.expect("failures or done") {
            Reply::Failures(batch) => {
                assert!(removed.is_none(), "nothing follows `done`");
                failures.extend(batch.failures);
            }
            Reply::Done(done) => removed = Some(done.entries_removed),
        }
    }
    (failures, removed)
}

/// cr-uv1-5: every entry `rm` cannot remove is reported — 1,001 files in
/// a directory that cannot be written, where the answer stopped at 1,000
/// and said "more". The lines, the exit status and what is left equal the
/// system `rm`'s; and `--json` lists each, with no flag in place of any.
#[cfg(target_os = "linux")]
#[test]
fn rm_reports_every_entry_it_cannot_remove() {
    use std::os::unix::fs::PermissionsExt;
    let ctx = TestContext::new();
    let system_root = ctx.workspace.join("system");
    let build = |root: &Path| {
        write(root, "t/open/a.txt");
        fs::create_dir_all(root.join("t/many")).unwrap();
        for at in 0..1001 {
            fs::write(root.join(format!("t/many/f{at:04}")), b"").unwrap();
        }
        fs::set_permissions(root.join("t/many"), fs::Permissions::from_mode(0o555)).unwrap();
    };
    let unlock = |root: &Path| {
        fs::set_permissions(root.join("t/many"), fs::Permissions::from_mode(0o755)).unwrap();
    };
    build(&system_root);
    build(&ctx.module_dir);
    if fs::write(ctx.module_dir.join("t/many/probe"), b"x").is_ok() {
        let _ = fs::remove_file(ctx.module_dir.join("t/many/probe"));
        unlock(&system_root);
        unlock(&ctx.module_dir);
        eprintln!("skipping: running privileged, a directory that cannot be written stops nothing");
        return;
    }

    let system = Command::new("rm")
        .args(["-r", "t"])
        .current_dir(&system_root)
        .env("LC_ALL", "C.UTF-8")
        .stdin(Stdio::null())
        .output()
        .expect("the system rm");
    let ours = blit(&ctx, &["rm", "-y", "-r", &remote(&ctx, "t")], None);
    let mut theirs: Vec<String> = String::from_utf8_lossy(&system.stderr)
        .lines()
        .map(str::to_string)
        .collect();
    let mut said: Vec<String> = ours
        .stderr
        .lines()
        .map(|line| {
            line.replacen("blit rm: ", "rm: ", 1)
                .replace(&remote(&ctx, ""), "")
        })
        .collect();
    theirs.sort();
    said.sort();
    assert_eq!(said.len(), 1001, "each of them is named");
    assert_eq!(said, theirs);
    assert_eq!(ours.code, system.status.code());
    assert!(!ctx.module_dir.join("t/open").exists() && !system_root.join("t/open").exists());

    let json = blit(
        &ctx,
        &["rm", "-y", "-r", "--json", &remote(&ctx, "t")],
        None,
    );
    unlock(&system_root);
    unlock(&ctx.module_dir);
    assert_eq!(json.code, Some(1), "{}", json.stderr);
    let report: serde_json::Value = serde_json::from_str(&json.stdout).expect("JSON");
    assert_eq!(report["failures"].as_array().map(Vec::len), Some(1001));
    assert_eq!(report["entries_removed"], 0);
}

/// cr-uv3-1: a removal that names the sessions which copied — a move's —
/// is refused whole when the daemon does not hold the account of one of
/// them: there is then nothing to say what may be removed. A daemon
/// restarted since the transfer is such a daemon.
#[test]
fn a_removal_for_a_move_is_refused_without_the_account_of_what_was_sent() {
    use blit_core::generated::blit_client::BlitClient;
    use blit_core::generated::RmRequest;
    use blit_core::transfer_session::CONTRACT_VERSION;

    let ctx = TestContext::new();
    tree(&ctx.module_dir);
    let runtime = tokio::runtime::Runtime::new().unwrap();
    runtime.block_on(async {
        let mut client = BlitClient::connect(format!("http://127.0.0.1:{}", ctx.daemon_port))
            .await
            .expect("connect");
        for ticket in [vec![7u8; 16], vec![7u8; 3], Vec::new()] {
            let status = client
                .rm(RmRequest {
                    // An empty ticket beside a real one is still a ticket
                    // nothing answers to.
                    sent_tickets: vec![vec![9u8; 16], ticket],
                    contract_version: CONTRACT_VERSION,
                    module: "test".into(),
                    paths: vec!["t".to_string()],
                    raw_paths: Vec::new(),
                    recursive: true,
                    force: true,
                })
                .await
                .expect_err("no account of what was sent");
            assert_eq!(status.code(), tonic::Code::FailedPrecondition);
            assert!(
                status
                    .message()
                    .contains("holds no account of what that transfer sent that it may act on"),
                "{}",
                status.message()
            );
            assert!(
                ctx.module_dir.join("t/dir/inner.txt").exists()
                    && ctx.module_dir.join("t/file.txt").exists(),
                "nothing removed"
            );
        }
    });
}

/// AC-11, `move`'s source removal: a `move` from a daemon reports the
/// source removed only when every entry went. With a folder in the source
/// that cannot be written, everything is copied, the removal goes on past
/// what it cannot remove, and the command ends unsuccessfully naming an
/// entry that stayed.
#[cfg(unix)]
#[test]
fn a_move_from_a_daemon_fails_when_part_of_the_source_stays() {
    use std::os::unix::fs::PermissionsExt;
    let ctx = TestContext::new();
    if Command::new("id")
        .arg("-u")
        .output()
        .map(|o| o.stdout == b"0\n")
        .unwrap_or(false)
    {
        eprintln!("running as root: every folder can be written; nothing to leave behind");
        return;
    }
    let source = ctx.module_dir.join("from");
    write(&source, "a.txt");
    write(&source, "locked/inner.txt");
    write(&source, "open/b.txt");
    fs::set_permissions(source.join("locked"), fs::Permissions::from_mode(0o555)).unwrap();
    let dst = ctx.workspace.join("to");
    let ran = blit(
        &ctx,
        &[
            "move",
            "-y",
            &remote(&ctx, "from/"),
            &format!("{}/", dst.display()),
        ],
        None,
    );
    fs::set_permissions(source.join("locked"), fs::Permissions::from_mode(0o755)).unwrap();

    assert!(
        dst.join("a.txt").exists()
            && dst.join("locked/inner.txt").exists()
            && dst.join("open/b.txt").exists(),
        "everything was copied\n{}",
        ran.all()
    );
    assert_ne!(
        ran.code,
        Some(0),
        "the source was not removed whole\n{}",
        ran.all()
    );
    assert!(
        ran.stderr
            .contains("cannot remove 'from/locked/inner.txt': Permission denied"),
        "{}",
        ran.all()
    );
    assert!(
        source.join("locked/inner.txt").exists(),
        "what could not be removed stays"
    );
    assert!(
        !source.join("a.txt").exists() && !source.join("open").exists(),
        "the removal went on past it\n{}",
        ran.all()
    );
}
