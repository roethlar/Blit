//! UTILITY_VERBS_LOCAL_PARITY slice 3 (AC-4, AC-8, AC-11, AC-12): `blit
//! find` against a real daemon — the Unix command, walked on the daemon's
//! own disk.
//!
//! Where the system has the reference (`find` from GNU findutils, Linux),
//! the lines, what is reported and the exit status are compared with it,
//! expression by expression. The shape, the tests and the refusals are
//! checked on every OS.

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
    fn lines(&self) -> Vec<String> {
        let mut lines: Vec<String> = String::from_utf8_lossy(&self.stdout)
            .lines()
            .map(str::to_string)
            .collect();
        lines.sort();
        lines
    }

    fn all(&self) -> String {
        format!(
            "exit {:?}\n{}{}",
            self.code,
            String::from_utf8_lossy(&self.stdout),
            self.stderr
        )
    }
}

fn blit(ctx: &TestContext, args: &[&str]) -> Ran {
    let mut cmd = Command::new(&ctx.cli_bin);
    cmd.arg("--config-dir")
        .arg(&ctx.config_dir)
        .args(args)
        .stdin(Stdio::null());
    let output = run_with_timeout(cmd, Duration::from_secs(60));
    Ran {
        code: output.status.code(),
        stdout: output.stdout,
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

/// `t/` under `root`: files in two cases, a hidden one, folders two deep
/// and an empty one.
fn tree(root: &Path) {
    for rel in [
        "t/a.dat",
        "t/B.DAT",
        "t/.hidden",
        "t/sub/inner.dat",
        "t/sub/deep/er.dat",
    ] {
        write(root, rel);
    }
    fs::create_dir_all(root.join("t/empty")).unwrap();
}

/// AC-4, on every OS: one path per line and nothing else, each beginning
/// with the operand as typed; the operand itself is an entry and comes
/// first; every test given must hold; `-maxdepth` counts from the operand;
/// a separator after the operand is not doubled.
#[test]
fn find_prints_one_path_per_line_from_the_operand_as_typed() {
    let ctx = TestContext::new();
    tree(&ctx.module_dir);
    let t = remote(&ctx, "t");
    let under = |names: &[&str]| -> Vec<String> {
        let mut lines: Vec<String> = names
            .iter()
            .map(|name| {
                if name.is_empty() {
                    t.clone()
                } else {
                    format!("{t}/{name}")
                }
            })
            .collect();
        lines.sort();
        lines
    };
    let found = |expression: &[&str]| {
        let mut args = vec!["find", t.as_str()];
        args.extend(expression);
        let ran = blit(&ctx, &args);
        assert_eq!(
            (ran.code, ran.stderr.as_str()),
            (Some(0), ""),
            "{expression:?}"
        );
        ran
    };

    let everything = found(&[]);
    assert_eq!(
        everything.lines(),
        under(&[
            "",
            ".hidden",
            "B.DAT",
            "a.dat",
            "empty",
            "sub",
            "sub/deep",
            "sub/deep/er.dat",
            "sub/inner.dat"
        ])
    );
    let in_order = String::from_utf8_lossy(&everything.stdout).into_owned();
    assert_eq!(
        in_order.lines().next(),
        Some(t.as_str()),
        "the operand comes first"
    );

    assert_eq!(
        found(&["-name", "*.dat"]).lines(),
        under(&["a.dat", "sub/deep/er.dat", "sub/inner.dat"])
    );
    assert_eq!(
        found(&["-iname", "*.dat"]).lines(),
        under(&["B.DAT", "a.dat", "sub/deep/er.dat", "sub/inner.dat"])
    );
    assert_eq!(
        found(&["-name", "*"]).lines().len(),
        9,
        "a leading dot is matched too"
    );
    assert_eq!(
        found(&["-type", "d"]).lines(),
        under(&["", "empty", "sub", "sub/deep"])
    );
    assert_eq!(
        found(&["-type", "d", "-maxdepth", "1"]).lines(),
        under(&["", "empty", "sub"])
    );
    assert_eq!(found(&["-maxdepth", "0"]).lines(), under(&[""]));
    assert_eq!(
        found(&["-maxdepth", "1", "-type", "f", "-name", "*.dat"]).lines(),
        under(&["a.dat"])
    );
    assert_eq!(
        found(&["-name", "t"]).lines(),
        under(&[""]),
        "the operand's own name"
    );
    assert_eq!(
        found(&["-name", "nothing-is-called-this"]).lines(),
        Vec::<String>::new()
    );

    // A separator after the operand is kept as typed, and not doubled.
    let slashed = blit(
        &ctx,
        &["find", &format!("{t}/"), "-name", "*.dat", "-maxdepth", "1"],
    );
    assert_eq!(slashed.lines(), [format!("{t}/a.dat")], "{}", slashed.all());
    // A file as the operand is the one entry.
    let file = blit(&ctx, &["find", &remote(&ctx, "t/a.dat")]);
    assert_eq!(file.lines(), [remote(&ctx, "t/a.dat")]);
    // The module itself: its own line, and its name for the name tests.
    let module = blit(&ctx, &["find", &remote(&ctx, ""), "-maxdepth", "1"]);
    assert_eq!(
        module.lines(),
        [remote(&ctx, ""), remote(&ctx, "t")],
        "{}",
        module.all()
    );
    let named = blit(&ctx, &["find", &remote(&ctx, ""), "-name", "test"]);
    assert_eq!(named.lines(), [remote(&ctx, "")], "{}", named.all());

    // `--json`, before the path or after the tests.
    for args in [
        vec!["find", "--json", t.as_str(), "-name", "a.*"],
        vec!["find", t.as_str(), "-name", "a.*", "--json"],
    ] {
        let ran = blit(&ctx, &args);
        assert_eq!(ran.code, Some(0), "{}", ran.all());
        let answer: serde_json::Value = serde_json::from_slice(&ran.stdout).expect("JSON");
        assert_eq!(answer["paths"], serde_json::json!([format!("{t}/a.dat")]));
        assert_eq!(answer["failures"], serde_json::json!([]));
    }
}

/// AC-4 and AC-12, where the system command IS the reference: for the
/// same tree and the same expression, `blit find` prints the lines `find`
/// prints (as sets, each beginning with the operand as typed), reports
/// what `find` reports and ends with the same status. The names are the
/// ones the two readings of a pattern and the lowercase rule of `-iname`
/// turn on, a name that is not text, a link, and a start that is a file,
/// a link, a link with a separator after it, and not there at all.
#[cfg(target_os = "linux")]
#[test]
fn find_equals_the_system_find() {
    use std::os::unix::ffi::OsStrExt;
    use std::os::unix::fs::PermissionsExt;

    let ctx = TestContext::new();
    let system_root = ctx.workspace.join("system");
    let is_root = Command::new("id")
        .arg("-u")
        .output()
        .map(|o| o.stdout == b"0\n")
        .unwrap_or(false);
    let names = [
        "a.dat",
        "ab.dat",
        "abc.dat",
        "é.dat",
        "éa.dat",
        "€.dat",
        ".hidden",
        "A.DAT",
        "x]y",
        "x-y",
        "x^y",
        "x!y",
        "x*y",
        "x?y",
        "x\\y",
        "[lit]",
        "ÜBUNG.txt",
        "übung.txt",
        "Σ.sig",
        "σ.sig",
        "ς.sig",
        "ǅ.x",
        "ǆ.x",
        "Ǆ.x",
        "İ.x",
        "i.x",
        "ı.x",
        "I.x",
        "ß.x",
        "ẞ.x",
        "K.x",
        "k.x",
        "\u{212a}.x",
        "0",
        "9",
        "b",
        "B",
        "sub.txt",
        "a.dat\\",
        "[a",
        // A name that ends in a space (cr-uv1-1).
        "sp ",
    ];
    for root in [system_root.as_path(), ctx.module_dir.as_path()] {
        for name in names {
            write(root, &format!("t/{name}"));
        }
        fs::write(
            root.join("t")
                .join(std::ffi::OsStr::from_bytes(b"bad\xff.dat")),
            b"x",
        )
        .unwrap();
        write(root, "t/sub/inner.dat");
        write(root, "t/sub/deep/er.dat");
        std::os::unix::fs::symlink("sub", root.join("t/link")).unwrap();
        std::os::unix::fs::symlink("nowhere", root.join("t/broken")).unwrap();
        write(root, "t/locked/inside.dat");
        fs::set_permissions(root.join("t/locked"), fs::Permissions::from_mode(0o000)).unwrap();
    }

    let expressions: Vec<Vec<&str>> = vec![
        vec![],
        vec!["-name", "?.dat"],
        vec!["-name", "??.dat"],
        vec!["-name", "???.dat"],
        vec!["-name", "*"],
        vec!["-name", ".*"],
        vec!["-name", "*.dat"],
        vec!["-name", "[ab]"],
        vec!["-name", "[!ab]"],
        vec!["-name", "[^ab]"],
        vec!["-name", "[a-c]"],
        vec!["-name", "[0-9]"],
        vec!["-name", "x[]]y"],
        vec!["-name", "x[!]]y"],
        vec!["-name", "x[*]y"],
        vec!["-name", "x\\*y"],
        vec!["-name", "x\\\\y"],
        vec!["-name", "x[\\\\]y"],
        vec!["-name", "[[]lit]"],
        vec!["-name", "\\[lit\\]"],
        vec!["-name", "[lit]"],
        vec!["-name", "x[-!^]y"],
        vec!["-name", "x[!-]y"],
        vec!["-name", "[[:upper:]]"],
        vec!["-name", "[[:digit:]]"],
        vec!["-name", "[é€].dat"],
        vec!["-name", "[a"],
        vec!["-name", "a.dat\\"],
        vec!["-name", "sub/inner.dat"],
        vec!["-name", "t"],
        vec!["-name", "sub"],
        vec!["-name", "bad*.dat"],
        vec!["-name", "bad?.dat"],
        vec!["-name", "bad??.dat"],
        vec!["-iname", "übung*"],
        vec!["-iname", "ÜBUNG*"],
        vec!["-iname", "σ.sig"],
        vec!["-iname", "Σ.sig"],
        vec!["-iname", "ς.sig"],
        vec!["-iname", "ǆ.x"],
        vec!["-iname", "ǅ.x"],
        vec!["-iname", "i.x"],
        vec!["-iname", "I.x"],
        vec!["-iname", "İ.x"],
        vec!["-iname", "ı.x"],
        vec!["-iname", "ß.x"],
        vec!["-iname", "ẞ.x"],
        vec!["-iname", "k.x"],
        vec!["-iname", "\u{212a}.x"],
        vec!["-iname", "[a-c]"],
        vec!["-iname", "[A-C]"],
        vec!["-iname", "a.DAT"],
        vec!["-iname", "[[:upper:]]"],
        vec!["-iname", "[[:lower:]]"],
        vec!["-type", "f"],
        vec!["-type", "d"],
        vec!["-type", "l"],
        vec!["-maxdepth", "0"],
        vec!["-maxdepth", "1", "-type", "d"],
        vec!["-type", "f", "-name", "*.dat", "-maxdepth", "1"],
        vec!["-name", "*.dat", "-name", "*a*"],
        vec!["-type", "f", "-type", "d"],
    ];
    let starts = [
        "t",
        "t/",
        "t//sub",
        "t/sub/",
        "t/link",
        "t/link/",
        "t/broken",
        "t/a.dat",
        "t/missing",
        // cr-uv1-1, the operand as typed: a `.` after a link names the
        // directory it points to, as a separator does; a `.` on the way
        // says nothing; a space at the end is part of the name; a file
        // or a broken link typed as a directory is none.
        "t/link/.",
        "t/sub/.",
        "t/./sub",
        "t/sp ",
        "t/a.dat/",
        "t/broken/",
        "t/missing/",
    ];
    let mut compared = 0;
    for (start, expression) in starts.iter().flat_map(|start| {
        let few: Vec<Vec<&str>> = vec![vec![], vec!["-name", "*i*"], vec!["-type", "d"]];
        let these = if *start == "t" {
            expressions.clone()
        } else {
            few
        };
        these
            .into_iter()
            .map(move |expression| (*start, expression))
    }) {
        let system = Command::new("find")
            .arg(start)
            .args(&expression)
            .current_dir(&system_root)
            .env("LC_ALL", "C.UTF-8")
            .stdin(Stdio::null())
            .output()
            .expect("the system find");
        let operand = remote(&ctx, start);
        let mut args = vec!["find", operand.as_str()];
        args.extend(expression.iter().copied());
        let ours = blit(&ctx, &args);

        // Lines as bytes: a name need not be text.
        let lines = |out: &[u8], prefix: &[u8]| -> Vec<Vec<u8>> {
            let mut lines: Vec<Vec<u8>> = out
                .split(|byte| *byte == b'\n')
                .filter(|line| !line.is_empty())
                .map(|line| line.strip_prefix(prefix).unwrap_or(line).to_vec())
                .collect();
            lines.sort();
            lines
        };
        let module = remote(&ctx, "");
        assert_eq!(
            lines(&ours.stdout, module.as_bytes())
                .iter()
                .map(|line| String::from_utf8_lossy(line).into_owned())
                .collect::<Vec<_>>(),
            lines(&system.stdout, b"")
                .iter()
                .map(|line| String::from_utf8_lossy(line).into_owned())
                .collect::<Vec<_>>(),
            "find {start} {expression:?}\n{}",
            ours.all()
        );
        assert_eq!(
            lines(&ours.stdout, module.as_bytes()),
            lines(&system.stdout, b""),
            "find {start} {expression:?}: the same bytes"
        );
        if is_root {
            continue;
        }
        let mut theirs: Vec<String> = String::from_utf8_lossy(&system.stderr)
            .lines()
            .map(str::to_string)
            .collect();
        let mut said: Vec<String> = ours
            .stderr
            .lines()
            .map(|line| {
                line.replacen("blit find: ", "find: ", 1)
                    .replace(&module, "")
            })
            .collect();
        theirs.sort();
        said.sort();
        assert_eq!(said, theirs, "find {start} {expression:?}");
        assert_eq!(
            ours.code,
            system.status.code(),
            "find {start} {expression:?}"
        );
        compared += 1;
    }
    for root in [system_root.as_path(), ctx.module_dir.as_path()] {
        fs::set_permissions(root.join("t/locked"), fs::Permissions::from_mode(0o755)).unwrap();
    }
    if !is_root {
        assert!(compared > 100, "{compared} expressions were compared");
    }
}

/// AC-12, on every Unix: a directory that cannot be read is listed like
/// any entry, reported in `find`'s words with the path as typed, and the
/// rest is still printed — with exit status 1. A link is listed and not
/// entered. A name that is not text is printed as the bytes it is.
#[cfg(unix)]
#[test]
fn find_reports_what_it_cannot_read_and_goes_on() {
    use std::os::unix::ffi::OsStrExt;
    use std::os::unix::fs::PermissionsExt;
    let ctx = TestContext::new();
    tree(&ctx.module_dir);
    let t = ctx.module_dir.join("t");
    std::os::unix::fs::symlink("sub", t.join("link")).unwrap();
    let raw_kept = fs::write(t.join(std::ffi::OsStr::from_bytes(b"bad\xff.dat")), b"x").is_ok();
    write(&t, "locked/inside.dat");
    fs::set_permissions(t.join("locked"), fs::Permissions::from_mode(0o000)).unwrap();
    let readable = fs::read_dir(t.join("locked")).is_ok();
    let operand = remote(&ctx, "t");
    let ran = blit(&ctx, &["find", &operand]);
    fs::set_permissions(t.join("locked"), fs::Permissions::from_mode(0o755)).unwrap();

    let lines = ran.lines();
    assert!(lines.contains(&format!("{operand}/link")), "{}", ran.all());
    assert!(
        !lines
            .iter()
            .any(|line| line.starts_with(&format!("{operand}/link/"))),
        "{}",
        ran.all()
    );
    assert!(
        lines.contains(&format!("{operand}/sub/deep/er.dat")),
        "{}",
        ran.all()
    );
    if raw_kept {
        let mut wanted = format!("{operand}/").into_bytes();
        wanted.extend_from_slice(b"bad\xff.dat\n");
        assert!(
            ran.stdout
                .windows(wanted.len())
                .any(|window| window == wanted),
            "the name's own bytes are printed\n{}",
            ran.all()
        );
    }
    if readable {
        eprintln!("running as root: every directory can be read; nothing to report");
        return;
    }
    assert_eq!(ran.code, Some(1), "{}", ran.all());
    assert_eq!(
        ran.stderr.trim_end(),
        format!("blit find: \u{2018}{operand}/locked\u{2019}: Permission denied"),
        "{}",
        ran.all()
    );
    assert!(
        lines.contains(&format!("{operand}/locked")),
        "the directory itself is an entry"
    );
    assert!(!lines.contains(&format!("{operand}/locked/inside.dat")));

    // A path that is not there: reported the same way, nothing found.
    let missing = remote(&ctx, "t/missing");
    let ran = blit(&ctx, &["find", &missing, "-name", "*"]);
    assert_eq!(ran.code, Some(1), "{}", ran.all());
    assert!(ran.stdout.is_empty(), "{}", ran.all());
    assert!(
        ran.stderr
            .starts_with(&format!("blit find: \u{2018}{missing}\u{2019}: ")),
        "{}",
        ran.all()
    );
}

/// AC-8 and AC-4's refusals: a word of `find` that `blit find` does not
/// take, a test with no argument, an option it had before 1.0 — each is
/// refused by name before anything is asked of a daemon. The path here is
/// on a port nothing listens on: a command that asked would say it could
/// not connect.
#[test]
fn find_refuses_what_it_does_not_take_before_asking_anything() {
    let ctx = TestContext::new();
    let nowhere = "127.0.0.1:9:/test/t";
    for (words, said) in [
        (&["-newer", "x"][..], "-newer is not supported"),
        (&["-name", "x", "-o", "-name", "y"], "-o is not supported"),
        (&["!", "-name", "x"], "! is not supported"),
        (&["-name", "x", "-delete"], "-delete is not supported"),
        (&["-exec", "rm", "{}", ";"], "-exec is not supported"),
        (&["-print"], "-print is not supported"),
        (&["-size", "+1M"], "-size is not supported"),
        (&["-type", "b"], "-type b is not supported"),
        (&["-name"], "missing argument to `-name'"),
        (&["-maxdepth", "x"], "-maxdepth x"),
        (
            &["--pattern", "*.csv"],
            "--pattern is gone: write -name '*.csv'",
        ),
        (
            &["--case-insensitive"],
            "--case-insensitive is gone: write -iname PATTERN",
        ),
        (&["--files"], "--files is gone: write -type f"),
        (&["--dirs"], "--dirs is gone: write -type d"),
        (&["--limit", "3"], "--limit is gone"),
        (&["other/path"], "`other/path` is not a test"),
    ] {
        let mut args = vec!["find", nowhere];
        args.extend(words.iter().copied());
        let ran = blit(&ctx, &args);
        assert_ne!(ran.code, Some(0), "{words:?}\n{}", ran.all());
        assert!(ran.stdout.is_empty(), "{words:?}\n{}", ran.all());
        assert!(ran.stderr.contains(said), "{words:?}\n{}", ran.all());
        assert!(
            !ran.stderr.to_lowercase().contains("connect"),
            "{words:?}: refused before anything was asked\n{}",
            ran.all()
        );
    }
    // And a path that is not on a daemon.
    let ran = blit(
        &ctx,
        &["find", ctx.workspace.to_str().unwrap(), "-name", "x"],
    );
    assert_ne!(ran.code, Some(0));
    assert!(
        ran.stderr.contains("requires a remote path"),
        "{}",
        ran.all()
    );
}

/// AC-11, an asker of another contract against this daemon: the walk is
/// refused before anything is read — an older asker and a newer one alike
/// — with both versions named. (A daemon too old to know the request at
/// all is `remote_remote.rs`'s fake.)
#[test]
fn a_daemon_refuses_find_from_another_contract() {
    use blit_core::generated::blit_client::BlitClient;
    use blit_core::generated::FindPathsRequest;
    use blit_core::transfer_session::CONTRACT_VERSION;

    let ctx = TestContext::new();
    tree(&ctx.module_dir);
    let request = |contract_version: u32| FindPathsRequest {
        contract_version,
        module: "test".into(),
        start_path: "t".into(),
        ..Default::default()
    };
    let runtime = tokio::runtime::Runtime::new().unwrap();
    runtime.block_on(async {
        let mut client = BlitClient::connect(format!("http://127.0.0.1:{}", ctx.daemon_port))
            .await
            .expect("connect");
        for version in [CONTRACT_VERSION - 1, CONTRACT_VERSION + 1] {
            let status = client
                .find_paths(request(version))
                .await
                .expect_err("find from another contract");
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
        // The same request from this contract is answered.
        let mut replies = client
            .find_paths(request(CONTRACT_VERSION))
            .await
            .expect("find from this contract")
            .into_inner();
        assert!(replies.message().await.unwrap().is_some());
    });
}

/// `blit find … | head -1`: when whoever was reading has gone, the command
/// stops without a word, as a command the shell pipes does.
#[cfg(unix)]
#[test]
fn find_stops_quietly_when_its_reader_goes() {
    use std::io::{BufRead as _, BufReader, Read as _};
    let ctx = TestContext::new();
    for folder in 0..40 {
        for file in 0..250 {
            write(
                &ctx.module_dir,
                &format!("big/d{folder:02}/a-rather-long-file-name-{file:04}"),
            );
        }
    }
    let mut child = Command::new(&ctx.cli_bin)
        .arg("--config-dir")
        .arg(&ctx.config_dir)
        .args(["find", &remote(&ctx, "big")])
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn blit find");
    let mut first = String::new();
    {
        let mut reader = BufReader::new(child.stdout.take().unwrap());
        reader.read_line(&mut first).unwrap();
        // The reader goes, with most of the listing still to come.
    }
    let mut said = String::new();
    child
        .stderr
        .take()
        .unwrap()
        .read_to_string(&mut said)
        .unwrap();
    let status = child.wait().unwrap();
    assert_eq!(first.trim_end(), remote(&ctx, "big"));
    assert_eq!(said, "", "nothing is reported");
    assert_eq!(
        status.code(),
        Some(141),
        "as a command stopped by a closed pipe"
    );
}
