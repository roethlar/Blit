//! UTILITY_VERBS_LOCAL_PARITY slice 5 (AC-6, AC-8, AC-11, AC-12): `blit ls`
//! against a real daemon — the Unix command, run on the daemon's own disk
//! — and on a local path.
//!
//! Where the system has the reference (`ls` from GNU coreutils, Linux),
//! what is printed, what is reported and the exit status are compared
//! with it, operand by operand and option by option. The long form is
//! compared less the columns a listing does not carry (permissions, link
//! count, owner, group). The shape and the refusals are checked on every
//! Unix.

use std::fs;
use std::path::Path;
use std::process::{Command, Stdio};
use std::time::Duration;

mod common;
use common::{run_with_timeout, TestContext};

struct Ran {
    code: Option<i32>,
    stdout: Vec<u8>,
    stderr: String,
}

impl Ran {
    fn all(&self) -> String {
        format!(
            "exit {:?}\n{}{}",
            self.code,
            String::from_utf8_lossy(&self.stdout),
            self.stderr
        )
    }
}

/// `blit` with `args`, run from `cwd` when given; times are read in UTC.
fn blit_in(ctx: &TestContext, cwd: Option<&Path>, args: &[&str]) -> Ran {
    let mut cmd = Command::new(&ctx.cli_bin);
    cmd.arg("--config-dir")
        .arg(&ctx.config_dir)
        .args(args)
        .env("TZ", "UTC")
        .stdin(Stdio::null());
    if let Some(cwd) = cwd {
        cmd.current_dir(cwd);
    }
    let output = run_with_timeout(cmd, Duration::from_secs(60));
    Ran {
        code: output.status.code(),
        stdout: output.stdout,
        stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
    }
}

fn blit(ctx: &TestContext, args: &[&str]) -> Ran {
    blit_in(ctx, None, args)
}

/// The module as an operand, with `below` after it.
#[cfg(unix)]
fn remote(ctx: &TestContext, below: &str) -> String {
    format!("127.0.0.1:{}:/test/{below}", ctx.daemon_port)
}

/// `t/` under `root`: names that sort apart by their bytes, a hidden one,
/// one that is not text, sizes either side of a unit, a directory holding
/// one entry and an empty one, links to a directory, to a file and to
/// nothing, links that cannot be followed (cr-uv5-5: round in a ring,
/// through a file, by a name too long), and a pipe — every entry's time
/// set, so that two such trees print alike: most an hour ago, one long
/// ago, one still to come.
#[cfg(unix)]
fn tree(root: &Path) {
    use filetime::FileTime;
    use std::os::unix::ffi::OsStrExt;

    let t = root.join("t");
    for dir in ["sub", "one", "Empty"] {
        fs::create_dir_all(t.join(dir)).unwrap();
    }
    for (name, bytes) in [
        ("a.txt", 6usize),
        ("A.txt", 1),
        ("b", 0),
        (".dot", 1),
        ("sub/inner", 1),
        ("one/only", 1),
        ("sp ace", 1),
        ("é.txt", 1),
        ("Z", 1),
        ("_u", 1),
        ("~t", 1),
        ("0n", 1),
        ("-dash", 1),
        ("!bang", 1),
        ("k15", 1500),
        ("m1", 1024 * 1024),
        ("k10", 10240),
        ("b1023", 1023),
        ("k1", 1024),
    ] {
        fs::write(t.join(name), vec![b'x'; bytes]).unwrap();
    }
    fs::write(t.join(std::ffi::OsStr::from_bytes(b"bad\xffname")), b"x").unwrap();
    std::os::unix::fs::symlink("sub", t.join("link")).unwrap();
    std::os::unix::fs::symlink("a.txt", t.join("flink")).unwrap();
    std::os::unix::fs::symlink("nowhere", t.join("broken")).unwrap();
    std::os::unix::fs::symlink("ring", t.join("ring")).unwrap();
    std::os::unix::fs::symlink("a.txt/x", t.join("through-a-file")).unwrap();
    std::os::unix::fs::symlink("n".repeat(300), t.join("too-long")).unwrap();
    assert!(Command::new("mkfifo")
        .arg(t.join("pipe"))
        .status()
        .unwrap()
        .success());

    let hour = 60 * 60;
    let now = FileTime::now().unix_seconds();
    let recent = FileTime::from_unix_time(now - hour, 0);
    let times = |path: &Path| match path.file_name().and_then(|name| name.to_str()) {
        // 2025-01-02T03:04:05Z
        Some("A.txt") => FileTime::from_unix_time(1_735_787_045, 0),
        Some("b") => FileTime::from_unix_time(now + 200 * 24 * hour, 0),
        _ => recent,
    };
    // Entries before the directories that hold them: writing an entry's
    // time does not touch its directory's, creating one did.
    let mut dirs = vec![root.to_path_buf(), t.clone()];
    for entry in fs::read_dir(&t).unwrap().flatten() {
        let path = entry.path();
        if entry.file_type().unwrap().is_dir() {
            for inner in fs::read_dir(&path).unwrap().flatten() {
                filetime::set_symlink_file_times(inner.path(), recent, recent).unwrap();
            }
            dirs.push(path);
        } else {
            let at = times(&path);
            filetime::set_symlink_file_times(&path, at, at).unwrap();
        }
    }
    for dir in dirs {
        filetime::set_symlink_file_times(&dir, recent, recent).unwrap();
    }
}

/// The system `ls`, in the reference locale and UTC, from `root`.
#[cfg(target_os = "linux")]
fn system_ls(root: &Path, args: &[&str]) -> std::process::Output {
    Command::new("ls")
        .args(args)
        .current_dir(root)
        .env("LC_ALL", "C.UTF-8")
        .env("TZ", "UTC")
        .stdin(Stdio::null())
        .output()
        .expect("the system ls")
}

/// What the system `ls` printed, as `blit ls` is to print it: each line
/// of the long form less the columns a listing does not carry — the nine
/// permission letters, the link count, the owner and the group — with the
/// sizes lined up on the right again; any other line as it is.
#[cfg(target_os = "linux")]
fn as_a_listing_carries_it(theirs: &[u8], long: bool) -> Vec<Vec<u8>> {
    let lines = theirs
        .split(|&byte| byte == b'\n')
        .filter(|line| !line.is_empty());
    if !long {
        return lines.map(<[u8]>::to_vec).collect();
    }
    enum Line {
        Total(Vec<u8>),
        Entry(u8, Vec<u8>, Vec<u8>, Vec<u8>),
    }
    let mut parsed = Vec::new();
    for line in lines {
        if line.starts_with(b"total ") {
            parsed.push(Line::Total(line.to_vec()));
            continue;
        }
        // Five words — permissions, links, owner, group, size — then one
        // space, the twelve characters of the time, one space, the name.
        let mut at = 0;
        let mut words = Vec::new();
        for _ in 0..5 {
            while line[at] == b' ' {
                at += 1;
            }
            let from = at;
            while line[at] != b' ' {
                at += 1;
            }
            words.push(&line[from..at]);
        }
        parsed.push(Line::Entry(
            words[0][0],
            words[4].to_vec(),
            line[at + 1..at + 13].to_vec(),
            line[at + 14..].to_vec(),
        ));
    }
    let width = parsed
        .iter()
        .map(|line| match line {
            Line::Entry(_, size, _, _) => size.len(),
            Line::Total(_) => 0,
        })
        .max()
        .unwrap_or(0);
    parsed
        .into_iter()
        .map(|line| match line {
            Line::Total(line) => line,
            Line::Entry(letter, size, time, name) => {
                let mut line = vec![letter, b' '];
                line.resize(2 + width - size.len(), b' ');
                line.extend(size);
                line.push(b' ');
                line.extend(time);
                line.push(b' ');
                line.extend(name);
                line
            }
        })
        .collect()
}

/// What `said` on stderr, as the system `ls` would have said it of the
/// same path typed from the tree's root.
#[cfg(target_os = "linux")]
fn as_ls_says(said: &str, prefix: &str) -> String {
    said.lines()
        .map(|line| line.replacen("blit ls: ", "ls: ", 1).replace(prefix, "") + "\n")
        .collect()
}

#[cfg(target_os = "linux")]
fn is_root() -> bool {
    Command::new("id")
        .arg("-u")
        .output()
        .map(|o| o.stdout == b"0\n")
        .unwrap_or(false)
}

/// Slice 5 (AC-6, AC-12): `blit ls` prints what the system `ls` prints —
/// names one per line in `ls`'s order, hidden names hidden, `-a` with `.`
/// and `..`, the long form less what a listing does not carry, `-h`; an
/// operand that is a file, a directory holding one entry, a link to a
/// directory with and without the long form and with a separator after
/// it, a link to a file and to nothing, a pipe; and for what cannot be
/// listed, `ls`'s words and `ls`'s exit status.
#[cfg(target_os = "linux")]
#[test]
fn ls_equals_the_system_ls() {
    use std::os::unix::fs::PermissionsExt;

    let ctx = TestContext::new();
    let system_root = ctx.workspace.join("system");
    for root in [system_root.as_path(), ctx.module_dir.as_path()] {
        tree(root);
    }
    // What was written is on disk before either is asked what it takes.
    assert!(Command::new("sync").status().unwrap().success());
    let mut operands = vec![
        "t",
        "t/",
        "t/sub",
        "t/sub/.",
        "t/one",
        "t/Empty",
        "t/a.txt",
        "t/a.txt/",
        "t/sp ace",
        "t/link",
        "t/link/",
        "t/flink",
        "t/broken",
        "t/broken/",
        "t/pipe",
        "t/missing",
        "t/missing/",
        "t//sub",
        // cr-uv5-5: links `ls` cannot follow. One round in a ring is the
        // link, as one to nothing is; the others cannot be accessed.
        "t/ring",
        "t/ring/x",
        "t/through-a-file",
        "t/too-long",
    ];
    if !is_root() {
        for root in [system_root.as_path(), ctx.module_dir.as_path()] {
            fs::create_dir(root.join("t/sub/locked")).unwrap();
            // cr-uv5-5: a file below what cannot be searched, and a link
            // to it.
            fs::write(root.join("t/sub/locked/below"), b"x").unwrap();
            std::os::unix::fs::symlink("locked/below", root.join("t/sub/to-below")).unwrap();
            fs::set_permissions(root.join("t/sub/locked"), fs::Permissions::from_mode(0o000))
                .unwrap();
            let recent = fs::metadata(root.join("t/sub/inner"))
                .unwrap()
                .modified()
                .unwrap();
            let recent = filetime::FileTime::from_system_time(recent);
            for path in ["t/sub/to-below", "t/sub/locked", "t/sub"] {
                filetime::set_symlink_file_times(root.join(path), recent, recent).unwrap();
            }
        }
        operands.extend([
            "t/sub/locked",
            "t/sub/locked/below",
            "t/sub/to-below",
            "t/sub/to-below/",
        ]);
    }
    let prefix = remote(&ctx, "");
    let mut compared = 0;
    for operand in &operands {
        for options in [
            &[][..],
            &["-a"],
            &["--all"],
            &["-l"],
            &["-la"],
            &["-lh"],
            &["-l", "-a", "-h"],
            &["-h"],
            &["--human-readable", "-l"],
        ] {
            let long = options
                .iter()
                .any(|option| !option.starts_with("--") && option.contains('l'));
            let mut theirs_args = options.to_vec();
            theirs_args.push(operand);
            let theirs = system_ls(&system_root, &theirs_args);
            let target = remote(&ctx, operand);
            let mut ours_args = vec!["ls"];
            ours_args.extend(options);
            ours_args.push(&target);
            let ours = blit(&ctx, &ours_args);
            let what = format!("ls {options:?} {operand}\n{}", ours.all());

            let expected = as_a_listing_carries_it(&theirs.stdout, long);
            let printed: Vec<Vec<u8>> = ours
                .stdout
                .split(|&byte| byte == b'\n')
                .filter(|line| !line.is_empty())
                // An operand listed as itself is printed as typed: with
                // the daemon and the module before it, here.
                .map(|line| {
                    String::from_utf8(line.to_vec())
                        .map(|text| text.replace(&prefix, "").into_bytes())
                        .unwrap_or_else(|_| line.to_vec())
                })
                .collect();
            let shown = |lines: &[Vec<u8>]| -> Vec<String> {
                lines
                    .iter()
                    .map(|line| String::from_utf8_lossy(line).into_owned())
                    .collect()
            };
            assert_eq!(
                printed,
                expected,
                "{what}\nexpected {:#?}",
                shown(&expected)
            );
            assert_eq!(
                as_ls_says(&ours.stderr, &prefix),
                String::from_utf8_lossy(&theirs.stderr),
                "{what}"
            );
            assert_eq!(ours.code, theirs.status.code(), "{what}");
            compared += 1;
        }
    }
    assert!(compared >= 22 * 9, "{compared} comparisons");
    for root in [system_root.as_path(), ctx.module_dir.as_path()] {
        let _ = fs::set_permissions(root.join("t/sub/locked"), fs::Permissions::from_mode(0o755));
    }
}

/// Slice 5 (AC-6): a path on this machine is listed by the same rules —
/// `blit ls` and the system `ls`, run from the same directory with the
/// same words, print the same bytes.
#[cfg(target_os = "linux")]
#[test]
fn ls_of_a_local_path_equals_the_system_ls() {
    let ctx = TestContext::new();
    let root = ctx.workspace.join("here");
    tree(&root);
    assert!(Command::new("sync").status().unwrap().success());
    // A bare word is a host to `blit ls`, whose modules it lists; a path
    // on this machine is written as one.
    for operand in [
        "./t",
        "t/",
        "t/sub",
        "t/a.txt",
        "./t/a.txt",
        "t/link",
        "t/link/",
        "t/broken",
        "t/missing",
        "t/a.txt/",
        "t/Empty",
        "t/ring",
        "t/ring/x",
        "t/through-a-file",
        "t/too-long",
    ] {
        for options in [&[][..], &["-a"], &["-l"], &["-lah"]] {
            let long = options
                .iter()
                .any(|option| !option.starts_with("--") && option.contains('l'));
            let mut args = options.to_vec();
            args.push(operand);
            let theirs = system_ls(&root, &args);
            let mut ours_args = vec!["ls"];
            ours_args.extend(&args);
            let ours = blit_in(&ctx, Some(&root), &ours_args);
            let what = format!("ls {options:?} {operand}\n{}", ours.all());
            let printed: Vec<Vec<u8>> = ours
                .stdout
                .split(|&byte| byte == b'\n')
                .filter(|line| !line.is_empty())
                .map(<[u8]>::to_vec)
                .collect();
            assert_eq!(
                printed,
                as_a_listing_carries_it(&theirs.stdout, long),
                "{what}"
            );
            assert_eq!(
                as_ls_says(&ours.stderr, "\0"),
                String::from_utf8_lossy(&theirs.stderr),
                "{what}"
            );
            assert_eq!(ours.code, theirs.status.code(), "{what}");
        }
    }
}

/// Slice 5 (AC-6): the shape, on every Unix. Names one per line and
/// nothing else; a file operand is the operand as typed, where a
/// directory holding one entry is that entry; the long form's line.
#[cfg(unix)]
#[test]
fn ls_prints_names_and_tells_a_file_from_a_directory_holding_one() {
    let ctx = TestContext::new();
    tree(&ctx.module_dir);
    let lines = |ran: &Ran| -> Vec<String> {
        String::from_utf8_lossy(&ran.stdout)
            .lines()
            .map(str::to_string)
            .collect()
    };
    let plain = blit(&ctx, &["ls", &remote(&ctx, "t")]);
    assert_eq!(plain.code, Some(0), "{}", plain.all());
    let names = lines(&plain);
    assert_eq!(&names[..4], ["!bang", "-dash", "0n", "A.txt"], "{names:?}");
    assert!(!names.iter().any(|name| name.starts_with('.')), "{names:?}");
    assert_eq!(names.last().map(String::as_str), Some("é.txt"));
    assert!(plain.stderr.is_empty(), "{}", plain.all());

    let all = lines(&blit(&ctx, &["ls", "-a", &remote(&ctx, "t")]));
    assert_eq!(all.len(), names.len() + 3);
    for dotted in [".", "..", ".dot"] {
        assert!(all.contains(&dotted.to_string()), "{all:?}");
    }

    // A file is itself, as typed; a directory holding one entry is that
    // entry. The reply this request replaces could not tell them apart.
    let file = remote(&ctx, "t/one/only");
    assert_eq!(
        lines(&blit(&ctx, &["ls", &file])),
        std::slice::from_ref(&file)
    );
    assert_eq!(
        lines(&blit(&ctx, &["ls", &remote(&ctx, "t/one")])),
        ["only"]
    );
    let long = lines(&blit(&ctx, &["ls", "-l", &file]));
    assert_eq!(long.len(), 1, "no total for a file: {long:?}");
    assert!(
        long[0].starts_with("- 1 ") && long[0].ends_with(&file),
        "{long:?}"
    );
    let long = lines(&blit(&ctx, &["ls", "-l", &remote(&ctx, "t/one")]));
    assert!(
        long.len() == 2 && long[0].starts_with("total ") && long[1].ends_with(" only"),
        "{long:?}"
    );

    // A link among the entries is the link, with where it points; and a
    // module's own `..` is the module: nothing above it is shown.
    let long = lines(&blit(&ctx, &["ls", "-l", &remote(&ctx, "t")]));
    assert!(
        long.iter()
            .any(|line| line.starts_with("l ") && line.ends_with(" link -> sub")),
        "{long:?}"
    );
    assert!(
        long.iter()
            .any(|line| line.starts_with("l ") && line.ends_with(" broken -> nowhere")),
        "{long:?}"
    );
    let top = lines(&blit(&ctx, &["ls", "-la", &remote(&ctx, "")]));
    let (dot, dot_dot) = (
        top.iter().find(|line| line.ends_with(" .")).expect("."),
        top.iter().find(|line| line.ends_with(" ..")).expect(".."),
    );
    assert_eq!(dot.trim_end_matches('.'), dot_dot.trim_end_matches('.'));
}

/// Slice 5: `--json` says whether the path was listed into, and gives
/// each entry its facts — sorted as the lines are — and a name that is
/// not text its bytes; a failure is told there too, with the same exit
/// status.
#[cfg(unix)]
#[test]
fn ls_json_gives_the_entries_and_what_the_path_was() {
    let ctx = TestContext::new();
    tree(&ctx.module_dir);
    let json = |args: &[&str]| -> (Option<i32>, serde_json::Value) {
        let ran = blit(&ctx, args);
        let value = serde_json::from_slice(&ran.stdout)
            .unwrap_or_else(|error| panic!("{error}: {}", ran.all()));
        (ran.code, value)
    };
    let (code, listed) = json(&["ls", "--json", &remote(&ctx, "t")]);
    assert_eq!(code, Some(0));
    assert_eq!(listed["listed_into"], true);
    let entries = listed["entries"].as_array().unwrap();
    let names: Vec<&str> = entries
        .iter()
        .map(|entry| entry["name"].as_str().unwrap())
        .collect();
    assert_eq!(&names[..3], ["!bang", "-dash", "0n"]);
    assert!(!names.contains(&".dot"));
    let named = |name: &str| {
        entries
            .iter()
            .find(|entry| entry["name"] == name)
            .unwrap_or_else(|| panic!("no {name}"))
    };
    assert_eq!(named("sub")["kind"], "directory");
    assert_eq!(named("sub")["is_dir"], true);
    assert_eq!(named("k15")["size"], 1500);
    assert_eq!(named("link")["kind"], "link");
    assert_eq!(named("link")["target"], "sub");
    assert_eq!(named("pipe")["kind"], "fifo");
    assert_eq!(named("A.txt")["mtime_seconds"], 1_735_787_045);
    assert_eq!(named("bad\u{fffd}name")["raw"], "bad\\xffname");

    let file = remote(&ctx, "t/a.txt");
    let (code, listed) = json(&["ls", "--json", &file]);
    assert_eq!(code, Some(0));
    assert_eq!(listed["listed_into"], false);
    assert_eq!(listed["entries"][0]["name"], file.as_str());
    assert_eq!(listed["entries"][0]["size"], 6);

    let (code, listed) = json(&["ls", "--json", &remote(&ctx, "t/missing")]);
    assert_eq!(code, Some(2));
    assert_eq!(listed["listed_into"], false);
    assert!(listed["entries"].as_array().unwrap().is_empty());
    assert_eq!(listed["failures"][0]["action"], "cannot access");
    assert_eq!(listed["failures"][0]["reason"], "No such file or directory");
}

/// Slice 5 (AC-12): a path that is a link out of the module is refused,
/// whether or not it would be listed into, and nothing outside the module
/// is listed. Among a directory's entries the same link is a line of its
/// own, with where it points — which is the module's to tell.
#[cfg(unix)]
#[test]
fn ls_lists_nothing_through_a_link_out_of_the_module() {
    let ctx = TestContext::new();
    let outside = ctx.workspace.join("outside");
    fs::create_dir_all(&outside).unwrap();
    fs::write(outside.join("secret.txt"), b"x").unwrap();
    fs::create_dir_all(ctx.module_dir.join("t")).unwrap();
    std::os::unix::fs::symlink(&outside, ctx.module_dir.join("t/out")).unwrap();
    for (options, operand) in [
        (&[][..], "t/out"),
        (&[], "t/out/"),
        (&["-l"], "t/out/"),
        (&["-la"], "t/out"),
        (&["--json"], "t/out"),
    ] {
        let target = remote(&ctx, operand);
        let mut args = vec!["ls"];
        args.extend(options);
        args.push(&target);
        let ran = blit(&ctx, &args);
        let what = format!("ls {options:?} {operand}\n{}", ran.all());
        assert_ne!(ran.code, Some(0), "{what}");
        assert!(ran.stderr.contains("path containment"), "{what}");
        assert!(!ran.all().contains("secret.txt"), "{what}");
    }
    let ran = blit(&ctx, &["ls", "-l", &remote(&ctx, "t")]);
    assert_eq!(ran.code, Some(0), "{}", ran.all());
    let printed = String::from_utf8_lossy(&ran.stdout).into_owned();
    let link = format!(" out -> {}", outside.display());
    assert!(
        printed
            .lines()
            .any(|line| line.starts_with("l ") && line.ends_with(&link)),
        "{printed}"
    );
    assert!(!printed.contains("secret.txt"), "{printed}");
}

/// cr-uv5-5: a module may be one file, and is then read as the file it
/// is — `ls` names it, `du` measures it, `find` finds it. The module was
/// asked of the system as `root/.`, which the system refuses for a file,
/// and the refusal was taken for a path out of the module. Typed as a
/// directory it is none, in `ls`'s own words and with its exit status.
#[cfg(unix)]
#[test]
fn a_module_that_is_one_file_is_read_as_the_file_it_is() {
    let ctx = TestContext::new();
    fs::write(ctx.workspace.join("module_one"), b"the whole module\n").unwrap();
    let one = ctx.spawn_second_daemon("one", &common::DaemonOptions::default());
    let module = format!("127.0.0.1:{}:/test/", one.port);
    let printed = |ran: &Ran| String::from_utf8_lossy(&ran.stdout).into_owned();

    let ran = blit(&ctx, &["ls", &module]);
    assert_eq!(
        (ran.code, printed(&ran)),
        (Some(0), format!("{module}\n")),
        "{}",
        ran.all()
    );
    let ran = blit(&ctx, &["ls", "-l", &module]);
    assert_eq!(ran.code, Some(0), "{}", ran.all());
    assert!(
        printed(&ran).starts_with("- 17 ") && printed(&ran).ends_with(&format!(" {module}\n")),
        "{}",
        ran.all()
    );
    let ran = blit(&ctx, &["ls", "--json", &module]);
    assert_eq!(ran.code, Some(0), "{}", ran.all());
    let listed: serde_json::Value = serde_json::from_slice(&ran.stdout)
        .unwrap_or_else(|error| panic!("{error}: {}", ran.all()));
    assert_eq!(listed["listed_into"], false);
    assert_eq!(listed["entries"][0]["kind"], "file");
    assert_eq!(listed["entries"][0]["size"], 17);
    assert_eq!(listed["entries"][0]["name"], module.as_str());

    // `du` and `find` start where `ls` does.
    let ran = blit(&ctx, &["du", &module]);
    assert_eq!(ran.code, Some(0), "{}", ran.all());
    assert!(
        printed(&ran).lines().count() == 1 && printed(&ran).ends_with(&format!("\t{module}\n")),
        "{}",
        ran.all()
    );
    let ran = blit(&ctx, &["find", &module]);
    assert_eq!(
        (ran.code, printed(&ran)),
        (Some(0), format!("{module}\n")),
        "{}",
        ran.all()
    );

    // `ls file/.` — `cannot access 'file/.': Not a directory`, exit 2.
    let dotted = format!("{module}.");
    let ran = blit(&ctx, &["ls", &dotted]);
    assert_eq!(ran.code, Some(2), "{}", ran.all());
    assert_eq!(
        ran.stderr,
        format!("blit ls: cannot access '{dotted}': Not a directory\n")
    );
}

/// cr-uv5-5: a start the daemon's system will not look up — a file below
/// a directory that cannot be searched — is told as the command tells
/// it, with the command's exit status. It ended the request as a path
/// out of the module does: exit 1, and nothing in `--json`. And a path
/// that IS out of the module is still refused, whatever cannot be looked
/// up beyond the link that leaves it.
#[cfg(target_os = "linux")]
#[test]
fn a_start_that_cannot_be_looked_up_is_told_and_one_out_of_the_module_refused() {
    use std::os::unix::fs::PermissionsExt;

    if is_root() {
        eprintln!("root searches every directory: nothing here cannot be looked up");
        return;
    }
    let ctx = TestContext::new();
    let outside = ctx.workspace.join("outside");
    for root in [ctx.module_dir.as_path(), outside.as_path()] {
        fs::create_dir_all(root.join("locked")).unwrap();
        fs::write(root.join("locked/below"), b"x").unwrap();
        fs::set_permissions(root.join("locked"), fs::Permissions::from_mode(0o000)).unwrap();
    }
    std::os::unix::fs::symlink(&outside, ctx.module_dir.join("out")).unwrap();

    let below = remote(&ctx, "locked/below");
    let ran = blit(&ctx, &["ls", &below]);
    assert_eq!(ran.code, Some(2), "{}", ran.all());
    assert_eq!(
        ran.stderr,
        format!("blit ls: cannot access '{below}': Permission denied\n")
    );
    let ran = blit(&ctx, &["ls", "--json", &below]);
    assert_eq!(ran.code, Some(2), "{}", ran.all());
    let listed: serde_json::Value = serde_json::from_slice(&ran.stdout)
        .unwrap_or_else(|error| panic!("{error}: {}", ran.all()));
    assert_eq!(listed["listed_into"], false);
    assert_eq!(listed["failures"][0]["action"], "cannot access");
    assert_eq!(listed["failures"][0]["path"], below.as_str());
    assert_eq!(listed["failures"][0]["reason"], "Permission denied");
    // `du` and `find` start at the same place, and say it their own way.
    let ran = blit(&ctx, &["du", &below]);
    assert_eq!(ran.code, Some(1), "{}", ran.all());
    assert_eq!(
        ran.stderr,
        format!("blit du: cannot access '{below}': Permission denied\n")
    );
    let ran = blit(&ctx, &["find", &below]);
    assert_eq!(ran.code, Some(1), "{}", ran.all());
    assert_eq!(
        ran.stderr,
        format!("blit find: \u{2018}{below}\u{2019}: Permission denied\n")
    );

    // Out of the module: refused whole, and nothing said of what is there.
    let out = remote(&ctx, "out/locked/below");
    for verb in [&["ls"][..], &["ls", "--json"], &["du"], &["find"]] {
        let mut args = verb.to_vec();
        args.push(&out);
        let ran = blit(&ctx, &args);
        let what = format!("{verb:?}\n{}", ran.all());
        assert_eq!(ran.code, Some(1), "{what}");
        assert!(ran.stderr.contains("path containment"), "{what}");
        assert!(ran.stdout.is_empty(), "{what}");
    }
    for root in [ctx.module_dir.as_path(), outside.as_path()] {
        let _ = fs::set_permissions(root.join("locked"), fs::Permissions::from_mode(0o755));
    }
}

/// AC-8: an option `ls` has and `blit ls` does not take is refused by
/// name before anything is asked of a daemon — nothing listens where
/// these are sent.
#[test]
fn ls_refuses_what_it_does_not_take_before_asking_anything() {
    let ctx = TestContext::new();
    for option in ["-R", "-t", "-r", "-S", "-d", "-1", "--recursive", "-F"] {
        let ran = blit(&ctx, &["ls", option, "127.0.0.1:9:/test/t"]);
        assert_ne!(ran.code, Some(0), "{option}: {}", ran.all());
        assert!(
            ran.stderr.contains(option) && ran.stderr.contains("unexpected argument"),
            "{option}: {}",
            ran.all()
        );
        assert!(
            !ran.stderr.to_lowercase().contains("connect"),
            "{option}: nothing was asked: {}",
            ran.all()
        );
    }
    // `-h` is `ls`'s `-h`; help is `--help`.
    let help = blit(&ctx, &["ls", "--help"]);
    assert_eq!(help.code, Some(0), "{}", help.all());
    assert!(
        String::from_utf8_lossy(&help.stdout).contains("--human-readable"),
        "{}",
        help.all()
    );
}

/// AC-11: the daemon answers `ls` only from an asker of its own contract,
/// older or newer, before anything is read; and it no longer serves the
/// request `ls` replaced.
#[test]
fn a_daemon_refuses_ls_from_another_contract() {
    use blit_core::generated::blit_client::BlitClient;
    use blit_core::generated::LsRequest;
    use blit_core::transfer_session::CONTRACT_VERSION;

    let ctx = TestContext::new();
    fs::create_dir_all(ctx.module_dir.join("t")).unwrap();
    fs::write(ctx.module_dir.join("t/a.txt"), b"x").unwrap();
    let request = |contract_version: u32| LsRequest {
        contract_version,
        module: "test".into(),
        path: "t".into(),
        ..Default::default()
    };
    let runtime = tokio::runtime::Runtime::new().unwrap();
    runtime.block_on(async {
        let mut client = BlitClient::connect(format!("http://127.0.0.1:{}", ctx.daemon_port))
            .await
            .expect("connect");
        for version in [CONTRACT_VERSION - 1, CONTRACT_VERSION + 1] {
            let status = client
                .ls(request(version))
                .await
                .expect_err("ls from another contract");
            assert_eq!(status.code(), tonic::Code::FailedPrecondition);
            assert!(
                status
                    .message()
                    .contains(&format!("daemon v{CONTRACT_VERSION}"))
                    && status.message().contains(&format!("client v{version}")),
                "{}",
                status.message()
            );
        }
        // The same request from this contract is answered: a directory,
        // then its entry.
        let mut replies = client
            .ls(request(CONTRACT_VERSION))
            .await
            .expect("ls from this contract")
            .into_inner();
        let mut names = Vec::new();
        while let Some(reply) = replies.message().await.unwrap() {
            if let Some(blit_core::generated::ls_reply::Reply::Entries(batch)) = reply.reply {
                names.extend(batch.entries.into_iter().map(|entry| entry.name));
            }
        }
        assert_eq!(names, ["a.txt"]);
    });
}
