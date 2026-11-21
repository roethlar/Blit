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
    assert_eq!(report["failures_truncated"], false);
}

/// AC-3 and AC-12, where the system command IS the reference: for the
/// same tree and the same operands, `blit rm` prints the lines `rm`
/// prints (its own name before them), ends with the same status and
/// leaves the same entries on disk — a directory with and without `-r`,
/// missing operands with and without `-f`, a link to a directory (removed
/// as the link, its target untouched), and a tree holding a directory
/// that cannot be written, which both go on past.
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
    };
    let unlock = |root: &Path| {
        let locked = root.join("t/locked");
        if locked.exists() {
            fs::set_permissions(locked, fs::Permissions::from_mode(0o755)).unwrap();
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
        (&["-r"], &["t"]),
    ];
    if is_root {
        eprintln!("running as root: a directory that cannot be written stops nothing; those cases are skipped");
        cases.retain(|(_, operands)| operands != &["t"]);
    } else {
        cases.push((&["-r"], &["t/locked", "t/open"]));
        cases.push((&["-r", "-f"], &["t/locked/sub", "t/open"]));
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
/// daemon itself, whoever asks. (A daemon too old to know the request at
/// all is `remote_remote.rs`'s fake.)
#[test]
fn a_daemon_refuses_rm_from_another_contract() {
    use blit_core::generated::blit_client::BlitClient;
    use blit_core::generated::RmRequest;
    use blit_core::transfer_session::CONTRACT_VERSION;

    let ctx = TestContext::new();
    tree(&ctx.module_dir);
    let request = |contract_version: u32, path: &str| RmRequest {
        contract_version,
        module: "test".into(),
        paths: vec![path.to_string()],
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
        for root in ["", ".", "./"] {
            let status = client
                .rm(request(CONTRACT_VERSION, root))
                .await
                .expect_err("the module's root");
            assert_eq!(status.code(), tonic::Code::InvalidArgument, "{root:?}");
            assert!(ctx.module_dir.join("t/file.txt").exists(), "{root:?}");
        }
        // The same request from this contract is answered.
        let reply = client
            .rm(request(CONTRACT_VERSION, "t"))
            .await
            .expect("rm from this contract")
            .into_inner();
        assert!(
            reply.failures.is_empty() && reply.entries_removed >= 8,
            "{reply:?}"
        );
        assert!(!ctx.module_dir.join("t").exists());
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
