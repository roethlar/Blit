//! UTILITY_VERBS_LOCAL_PARITY slice 1 (AC-1, AC-2, AC-8, AC-11, AC-12):
//! `blit du` and `blit df` against a real daemon — the Unix commands, run
//! on the daemon's own disk.
//!
//! Where the system has the reference (`du` and `df` from GNU coreutils,
//! Linux), the output is compared with it line for line. The shape, the
//! options and the refusals are checked on every OS.

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
    fn out(&self) -> String {
        String::from_utf8_lossy(&self.stdout).into_owned()
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

fn file(path: &Path, bytes: usize) {
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, vec![7u8; bytes]).unwrap();
}

/// `t/` in the module: folders three deep, files of several sizes, an
/// empty folder and a name with a space.
fn tree(ctx: &TestContext) -> std::path::PathBuf {
    let t = ctx.module_dir.join("t");
    file(&t.join("a/five-k.bin"), 5000);
    file(&t.join("a/one-byte.bin"), 1);
    file(&t.join("a/deep/big.bin"), 300_000);
    file(&t.join("a/deep/er/mid.bin"), 70_000);
    file(&t.join("b/sp ace.txt"), 1);
    fs::write(t.join("b/empty-file"), b"").unwrap();
    fs::create_dir_all(t.join("empty")).unwrap();
    t
}

/// The lines of a `du` answer as (size, path).
fn lines(out: &str) -> Vec<(String, String)> {
    out.lines()
        .map(|line| {
            let (size, path) = line.split_once('\t').expect("SIZE<TAB>PATH");
            (size.to_string(), path.to_string())
        })
        .collect()
}

/// AC-2, on every OS: one line per directory as `SIZE<TAB>PATH`, each path
/// beginning with the operand as typed, an entry after everything below
/// it and the operand LAST and once; `-a` adds the files; `-s` is the
/// operand alone; `-d N` counts from the operand; a file operand is its
/// own line; a trailing slash on the operand is not doubled.
#[test]
fn du_prints_the_unix_commands_lines() {
    let ctx = TestContext::new();
    tree(&ctx);
    let operand = remote(&ctx, "t");

    let ran = blit(&ctx, &["du", &operand]);
    assert_eq!(ran.code, Some(0), "{}", ran.stderr);
    let rows = lines(&ran.out());
    let paths: Vec<&str> = rows.iter().map(|(_, path)| path.as_str()).collect();
    let mut sorted = paths.clone();
    sorted.sort_unstable();
    let under = |below: &str| format!("{operand}/{below}");
    assert_eq!(
        sorted,
        [
            operand.clone(),
            under("a"),
            under("a/deep"),
            under("a/deep/er"),
            under("b"),
            under("empty"),
        ]
    );
    assert_eq!(
        *paths.last().unwrap(),
        operand,
        "the operand last, and once"
    );
    let at = |path: &str| paths.iter().position(|p| *p == path).unwrap();
    assert!(at(&under("a/deep/er")) < at(&under("a/deep")));
    assert!(at(&under("a/deep")) < at(&under("a")));
    let blocks = |path: &str| rows[at(path)].0.parse::<u64>().expect("1K blocks");
    // 300,000 and 70,000 bytes: at least 292 and 68 blocks of 1K.
    assert!(
        blocks(&under("a/deep")) >= 292 + 68,
        "what the files hold, in 1K"
    );
    // ci-windows-du-rounding: each line is rounded up to whole 1K blocks on
    // its own. Off Unix a line counts the bytes its files hold (`Looked::
    // allocated`), so two children rounded up can show one block more
    // between them than their parent; Unix counts allocated blocks.
    let rounding = if cfg!(unix) { 0 } else { 1 };
    assert!(blocks(&operand) + rounding >= blocks(&under("a")) + blocks(&under("b")));

    let all = lines(&blit(&ctx, &["du", "-a", &operand]).out());
    assert_eq!(all.len(), rows.len() + 6, "the six files too");
    assert!(all.iter().any(|(_, path)| *path == under("b/sp ace.txt")));
    assert_eq!(all.last().unwrap(), rows.last().unwrap());

    let total = rows.last().unwrap().clone();
    for summary in [
        &["-s"][..],
        &["--max-depth=0"],
        &["-d", "0"],
        &["--summarize"],
    ] {
        let mut args = vec!["du"];
        args.extend(summary);
        args.push(&operand);
        assert_eq!(
            lines(&blit(&ctx, &args).out()),
            std::slice::from_ref(&total),
            "{summary:?}"
        );
    }

    let one = lines(&blit(&ctx, &["du", "-d", "1", &operand]).out());
    let mut one_paths: Vec<&str> = one.iter().map(|(_, path)| path.as_str()).collect();
    one_paths.sort_unstable();
    assert_eq!(
        one_paths,
        [operand.clone(), under("a"), under("b"), under("empty")]
    );
    // Depth is counted from the operand, wherever it sits in the module.
    let deep_operand = remote(&ctx, "t/a");
    let below = lines(&blit(&ctx, &["du", "-d", "1", &deep_operand]).out());
    let mut below: Vec<&str> = below.iter().map(|(_, path)| path.as_str()).collect();
    below.sort_unstable();
    assert_eq!(
        below,
        [deep_operand.clone(), format!("{deep_operand}/deep")]
    );

    let slashed = remote(&ctx, "t/");
    let with_slash = lines(&blit(&ctx, &["du", "-s", &slashed]).out());
    assert_eq!(with_slash[0].1, slashed);
    let children = lines(&blit(&ctx, &["du", "-d", "1", &slashed]).out());
    assert!(
        children.iter().any(|(_, path)| *path == under("a")),
        "{children:?}"
    );

    let a_file = remote(&ctx, "t/a/five-k.bin");
    let of_file = lines(&blit(&ctx, &["du", &a_file]).out());
    assert_eq!(of_file.len(), 1);
    assert_eq!(of_file[0].1, a_file);

    let human = lines(&blit(&ctx, &["du", "-sh", &operand]).out());
    assert!(
        human[0].0.ends_with(['K', 'M']) && human[0].0.parse::<u64>().is_err(),
        "a human-readable size: {human:?}"
    );
    let json = blit(&ctx, &["du", "-s", "--json", &operand]);
    let parsed: serde_json::Value = serde_json::from_slice(&json.stdout).expect("json");
    assert_eq!(parsed["rows"][0]["path"], operand.as_str());
    assert!(parsed["rows"][0]["allocated_bytes"].as_u64().unwrap() >= 375_000);
    assert_eq!(parsed["failures"].as_array().map(Vec::len), Some(0));
}

/// AC-2, where the system has the reference: `blit du` prints what `du`
/// prints for the same tree and options, line for line — sizes, order and
/// paths — hard links, a symbolic link and a name that is not valid text
/// included. GNU coreutils under `C.UTF-8`; Linux only (macOS ships the
/// BSD command, Windows none).
#[cfg(target_os = "linux")]
#[test]
fn du_equals_the_system_du() {
    use std::os::unix::ffi::OsStrExt as _;
    let ctx = TestContext::new();
    let t = tree(&ctx);
    fs::hard_link(t.join("a/deep/big.bin"), t.join("b/hard-link-to-big")).unwrap();
    std::os::unix::fs::symlink("../a/deep", t.join("b/link-to-dir")).unwrap();
    // A name that ends in a space (cr-uv1-1).
    file(&t.join("sp "), 3000);
    let odd = t
        .join("b")
        .join(std::ffi::OsStr::from_bytes(b"not-\xff-text.bin"));
    let has_odd = fs::write(&odd, vec![7u8; 9000]).is_ok();

    let system = |args: &[&str], operand: &str| {
        let output = Command::new("du")
            .args(args)
            .arg(operand)
            .current_dir(&ctx.module_dir)
            .env("LC_ALL", "C.UTF-8")
            .output()
            .expect("the system du");
        (output.status.code(), output.stdout, output.stderr)
    };
    // `blit du`'s bytes with its operand written as the system's.
    let as_local = |bytes: &[u8], operand: &str, local: &str| -> Vec<u8> {
        let (from, to) = (operand.as_bytes(), local.as_bytes());
        let mut out = Vec::with_capacity(bytes.len());
        let mut rest = bytes;
        while let Some(at) = rest.windows(from.len()).position(|window| window == from) {
            out.extend_from_slice(&rest[..at]);
            out.extend_from_slice(to);
            rest = &rest[at + from.len()..];
        }
        out.extend_from_slice(rest);
        out
    };

    for (options, local) in [
        (&[][..], "t"),
        (&["-a"], "t"),
        (&["-s"], "t"),
        (&["-d", "1"], "t"),
        (&["--max-depth=0"], "t"),
        (&["-h"], "t"),
        (&["-sh"], "t"),
        (&["-ah"], "t"),
        (&["-a", "-d", "2"], "t"),
        (&["-a"], "t/"),
        (&["-a"], "t/b"),
        (&[], "t/a/five-k.bin"),
        (&["-s"], "t/b/link-to-dir"),
        // cr-uv1-1, the operand as typed: a separator or a `.` after a
        // link names the directory it points to (`-s` of the link alone
        // is the link, 0); a `.` on the way says nothing.
        (&["-s"], "t/b/link-to-dir/"),
        (&["-s"], "t/b/link-to-dir/."),
        (&["-a"], "t/b/link-to-dir/"),
        (&[], "t/a/."),
        (&["-a"], "t/./a"),
        (&["-s"], "t/sp "),
        // Typed as a directory, and none: `cannot access`, exit 1.
        (&["-s"], "t/a/five-k.bin/"),
        (&[], "t/missing/"),
    ] {
        let operand = remote(&ctx, local);
        let mut args = vec!["du"];
        args.extend(options);
        args.push(&operand);
        let ours = blit(&ctx, &args);
        let (code, theirs, their_err) = system(options, local);
        assert_eq!(
            String::from_utf8_lossy(&as_local(&ours.stdout, &operand, local)),
            String::from_utf8_lossy(&theirs),
            "du {options:?} {local}\nblit said: {}\ndu said: {}",
            ours.stderr,
            String::from_utf8_lossy(&their_err)
        );
        assert_eq!(
            as_local(&ours.stdout, &operand, local),
            theirs,
            "{options:?} {local}: bytes"
        );
        assert_eq!(ours.code, code, "du {options:?} {local}: exit status");
        // What it says of an operand it cannot measure, in `du`'s words.
        assert_eq!(
            String::from_utf8_lossy(&as_local(ours.stderr.as_bytes(), &operand, local)).replacen(
                "blit du: ",
                "du: ",
                1
            ),
            String::from_utf8_lossy(&their_err),
            "du {options:?} {local}: what it says"
        );
    }
    assert!(has_odd, "this filesystem stores a name that is not UTF-8");
    // cr-uv1-4: and that name, as printed, is an operand `du` and `df`
    // take — the bytes typed are the bytes asked about.
    let mut typed = remote(&ctx, "t/b/").into_bytes();
    typed.extend_from_slice(b"not-\xff-text.bin");
    let typed = std::ffi::OsStr::from_bytes(&typed);
    let ours = run_with_timeout(
        {
            let mut cmd = Command::new(&ctx.cli_bin);
            cmd.arg("--config-dir")
                .arg(&ctx.config_dir)
                .arg("du")
                .arg(typed);
            cmd
        },
        Duration::from_secs(60),
    );
    let theirs = Command::new("du")
        .arg(std::ffi::OsStr::from_bytes(b"t/b/not-\xff-text.bin"))
        .current_dir(&ctx.module_dir)
        .env("LC_ALL", "C.UTF-8")
        .output()
        .expect("the system du");
    assert_eq!(
        ours.status.code(),
        Some(0),
        "{}",
        String::from_utf8_lossy(&ours.stderr)
    );
    assert_eq!(
        as_local(&ours.stdout, &remote(&ctx, ""), ""),
        theirs.stdout,
        "du of a name that is not text: {}",
        String::from_utf8_lossy(&ours.stdout)
    );
    let ours = run_with_timeout(
        {
            let mut cmd = Command::new(&ctx.cli_bin);
            cmd.arg("--config-dir")
                .arg(&ctx.config_dir)
                .arg("df")
                .arg(typed);
            cmd
        },
        Duration::from_secs(60),
    );
    assert_eq!(
        ours.status.code(),
        Some(0),
        "{}",
        String::from_utf8_lossy(&ours.stderr)
    );
    let all = blit(&ctx, &["du", "-a", &remote(&ctx, "t/b")]);
    assert!(
        all.stdout.windows(14).any(|w| w == b"not-\xff-text.bin"),
        "the name's own bytes are printed"
    );
}

/// AC-12: a directory the daemon cannot read is reported in `du`'s words,
/// the rest is printed, and the exit status is 1 — on Linux exactly as the
/// system `du` does it. A path that is not there is `cannot access`, exit
/// 1, and nothing on stdout.
#[cfg(unix)]
#[test]
fn du_reports_what_the_daemon_cannot_read_and_exits_1() {
    use std::os::unix::fs::PermissionsExt as _;
    let ctx = TestContext::new();
    let t = tree(&ctx);
    file(&t.join("locked/inner/secret.bin"), 5000);
    let locked = t.join("locked");
    fs::set_permissions(&locked, fs::Permissions::from_mode(0o000)).unwrap();
    let privileged = fs::read_dir(&locked).is_ok();
    let operand = remote(&ctx, "t");
    let ours = blit(&ctx, &["du", &operand]);
    #[cfg(target_os = "linux")]
    let theirs = Command::new("du")
        .arg("t")
        .current_dir(&ctx.module_dir)
        .env("LC_ALL", "C.UTF-8")
        .output()
        .expect("the system du");
    fs::set_permissions(&locked, fs::Permissions::from_mode(0o755)).unwrap();
    if privileged {
        eprintln!("skipping: running privileged, mode 000 does not block reads");
        return;
    }
    assert_eq!(ours.code, Some(1), "{}", ours.stderr);
    assert_eq!(
        ours.stderr.trim_end(),
        format!("blit du: cannot read directory '{operand}/locked': Permission denied")
    );
    let rows = lines(&ours.out());
    let paths: Vec<&str> = rows.iter().map(|(_, path)| path.as_str()).collect();
    assert!(
        paths.contains(&format!("{operand}/locked").as_str()),
        "its own line stays"
    );
    assert!(
        paths.contains(&format!("{operand}/a/deep/er").as_str()),
        "the rest is printed"
    );
    assert_eq!(*paths.last().unwrap(), operand);
    #[cfg(target_os = "linux")]
    {
        assert_eq!(theirs.status.code(), Some(1));
        assert_eq!(
            ours.out().replace(&operand, "t"),
            String::from_utf8_lossy(&theirs.stdout)
        );
        assert_eq!(
            ours.stderr
                .replace(&operand, "t")
                .replace("blit du:", "du:"),
            String::from_utf8_lossy(&theirs.stderr)
        );
    }

    let missing = remote(&ctx, "t/no-such-entry");
    let ran = blit(&ctx, &["du", &missing]);
    assert_eq!(ran.code, Some(1));
    assert!(ran.stdout.is_empty(), "{}", ran.out());
    assert_eq!(
        ran.stderr.trim_end(),
        format!("blit du: cannot access '{missing}': No such file or directory")
    );
}

/// AC-8 and `du`'s own refusals: an option `du` does not have is refused
/// by name, and so are `-s` with `-a` and `-s` with a depth — each before
/// anything is asked of a daemon (none listens where these point).
#[test]
fn du_and_df_refuse_an_option_they_do_not_take_before_asking_anything() {
    let ctx = TestContext::new();
    let nowhere = "127.0.0.1:9:/test/";
    for (verb, option) in [
        ("du", "--apparent-size"),
        ("du", "-x"),
        ("du", "--files"),
        ("df", "--output=size"),
        ("df", "-i"),
        ("df", "-T"),
    ] {
        let ran = blit(&ctx, &[verb, option, nowhere]);
        assert_ne!(ran.code, Some(0), "{verb} {option}");
        let flag = option.split('=').next().unwrap();
        assert!(
            ran.stderr.contains(flag),
            "{verb} {option} is named: {}",
            ran.stderr
        );
        assert!(
            !ran.stderr.to_lowercase().contains("connect"),
            "{verb} {option}: refused before any request\n{}",
            ran.stderr
        );
    }
    for (args, words) in [
        (
            &["du", "-s", "-a", nowhere][..],
            "cannot both summarize and show all entries",
        ),
        (
            &["du", "-s", "-d", "1", nowhere],
            "summarizing conflicts with --max-depth=1",
        ),
    ] {
        let ran = blit(&ctx, args);
        assert_ne!(ran.code, Some(0), "{args:?}");
        assert!(ran.stderr.contains(words), "{args:?}: {}", ran.stderr);
        assert!(
            !ran.stderr.to_lowercase().contains("connect"),
            "{}",
            ran.stderr
        );
    }
    // `-h` is the commands' own `-h`; help is `--help`.
    for verb in ["du", "df"] {
        let help = blit(&ctx, &[verb, "--help"]);
        assert_eq!(help.code, Some(0));
        assert!(help.out().contains("--human-readable"), "{}", help.out());
    }
}

/// AC-1: `blit df PATH` reports the filesystem that holds THAT path, in
/// `df -k`'s shape with the operand first and no "Mounted on" column;
/// `-h` prints human-readable sizes; a path that is not there is `df`'s
/// error and exit 1.
#[test]
fn df_reports_the_filesystem_that_holds_the_path() {
    let ctx = TestContext::new();
    tree(&ctx);
    for below in ["", "t", "t/a/deep", "t/a/five-k.bin"] {
        let operand = remote(&ctx, below);
        let ran = blit(&ctx, &["df", &operand]);
        assert_eq!(ran.code, Some(0), "{below}: {}", ran.stderr);
        let out = ran.out();
        let rows: Vec<&str> = out.lines().collect();
        assert_eq!(rows.len(), 2, "{out}");
        assert_eq!(
            rows[0].split_whitespace().collect::<Vec<_>>(),
            ["Filesystem", "1K-blocks", "Used", "Available", "Use%"]
        );
        let fields: Vec<&str> = rows[1].split_whitespace().collect();
        assert_eq!(fields.len(), 5, "{out}");
        assert_eq!(fields[0], operand, "the operand in the first column");
        let number = |at: usize| fields[at].parse::<u64>().expect("1K blocks");
        assert!(number(1) > 0 && number(2) <= number(1) && number(3) <= number(1));
        assert!(fields[4].ends_with('%'), "{out}");
    }

    let operand = remote(&ctx, "t");
    let human = blit(&ctx, &["df", "-h", &operand]).out();
    assert_eq!(
        human
            .lines()
            .next()
            .unwrap()
            .split_whitespace()
            .collect::<Vec<_>>(),
        ["Filesystem", "Size", "Used", "Avail", "Use%"]
    );
    let json = blit(&ctx, &["df", "--json", &operand]);
    let parsed: serde_json::Value = serde_json::from_slice(&json.stdout).expect("json");
    assert!(parsed["total_bytes"].as_u64().unwrap() > 0);
    assert!(parsed["available_bytes"].as_u64().is_some());

    let missing = remote(&ctx, "t/no-such-entry");
    let ran = blit(&ctx, &["df", &missing]);
    assert_eq!(ran.code, Some(1));
    assert!(ran.stdout.is_empty());
    // ci-windows-df-words: in the system's own words for a path that is not
    // there — Unix's were expected, and Windows says "The system cannot find
    // the file specified." cr-ecm43-7: written here, as each system writes
    // them; asked of `os_reason`, the formatter this checks, a formatter
    // that gave any words at all gave the ones expected.
    let words = if cfg!(windows) {
        "The system cannot find the file specified."
    } else {
        "No such file or directory"
    };
    assert_eq!(
        ran.stderr.trim_end(),
        format!("blit df: {missing}: {words}")
    );
}

/// AC-1, where the system has the reference: the size `blit df` prints is
/// the size `df -k` prints for that path, and so — give or take what other
/// processes write meanwhile — are the used and available figures. The
/// module here is on its own filesystem wherever the temp folder is one
/// (the run that started this plan reported another disk's numbers for
/// such a module). Linux only.
#[cfg(target_os = "linux")]
#[test]
fn df_equals_the_system_df() {
    let ctx = TestContext::new();
    tree(&ctx);
    let system = |path: &Path| -> Vec<u64> {
        let output = Command::new("df")
            .args(["-k", "--output=size,used,avail"])
            .arg(path)
            .env("LC_ALL", "C.UTF-8")
            .output()
            .expect("the system df");
        String::from_utf8_lossy(&output.stdout)
            .lines()
            .nth(1)
            .expect("a line of numbers")
            .split_whitespace()
            .map(|field| field.parse().expect("a number"))
            .collect()
    };
    for below in ["", "t/a/deep"] {
        let ran = blit(&ctx, &["df", &remote(&ctx, below)]);
        assert_eq!(ran.code, Some(0), "{}", ran.stderr);
        let out = ran.out();
        let ours: Vec<u64> = out
            .lines()
            .nth(1)
            .unwrap()
            .split_whitespace()
            .skip(1)
            .take(3)
            .map(|field| field.parse().unwrap())
            .collect();
        let theirs = system(&ctx.module_dir.join(below));
        assert_eq!(ours[0], theirs[0], "the filesystem's size, exactly\n{out}");
        let slack = theirs[0] / 50;
        for at in [1, 2] {
            assert!(
                ours[at].abs_diff(theirs[at]) <= slack,
                "column {at}: blit {} against df {}\n{out}",
                ours[at],
                theirs[at]
            );
        }
    }
    // cr-uv1-1, the operand as typed: a file typed as a directory, and a
    // path that is not there, in `df`'s words and with its status.
    for below in ["t/a/five-k.bin/", "t/missing", "t/a/deep/missing/"] {
        let operand = remote(&ctx, below);
        let ran = blit(&ctx, &["df", &operand]);
        let theirs = Command::new("df")
            .arg(below)
            .current_dir(&ctx.module_dir)
            .env("LC_ALL", "C.UTF-8")
            .output()
            .expect("the system df");
        assert_eq!(ran.code, theirs.status.code(), "df {below}\n{}", ran.stderr);
        assert_eq!(
            ran.stderr
                .replacen("blit df: ", "df: ", 1)
                .replace(&operand, below),
            String::from_utf8_lossy(&theirs.stderr),
            "df {below}"
        );
    }
    // And one typed as a directory that is one answers like the same
    // path without the separator.
    let plain = blit(&ctx, &["df", &remote(&ctx, "t/a/deep")]).out();
    let slashed = blit(&ctx, &["df", &remote(&ctx, "t/a/deep/")]).out();
    assert_eq!(
        plain.lines().nth(1).unwrap().split_whitespace().nth(1),
        slashed.lines().nth(1).unwrap().split_whitespace().nth(1)
    );

    // The module's own filesystem, not the root one, where they differ.
    let (module, root) = (system(&ctx.module_dir), system(Path::new("/")));
    if module[0] == root[0] {
        eprintln!("the temp folder is on the root filesystem here: nothing to tell apart");
    } else {
        let out = blit(&ctx, &["df", &remote(&ctx, "")]).out();
        let size: u64 = out
            .lines()
            .nth(1)
            .unwrap()
            .split_whitespace()
            .nth(1)
            .unwrap()
            .parse()
            .unwrap();
        assert_eq!(size, module[0]);
        assert_ne!(size, root[0], "not another disk's numbers");
    }
}

/// AC-11, an asker of another contract against this daemon: the request
/// is refused before anything is read — an older asker and a newer one
/// alike — with both versions named. (A daemon too old to know the
/// request at all is `remote_remote.rs`'s fake.)
#[test]
fn a_daemon_refuses_du_and_df_from_another_contract() {
    use blit_core::generated::blit_client::BlitClient;
    use blit_core::generated::{DfRequest, DuRequest};
    use blit_core::transfer_session::CONTRACT_VERSION;

    let ctx = TestContext::new();
    tree(&ctx);
    let runtime = tokio::runtime::Runtime::new().unwrap();
    runtime.block_on(async {
        let mut client = BlitClient::connect(format!("http://127.0.0.1:{}", ctx.daemon_port))
            .await
            .expect("connect");
        for version in [CONTRACT_VERSION - 1, CONTRACT_VERSION + 1] {
            let du = client
                .du(DuRequest {
                    contract_version: version,
                    module: "test".into(),
                    start_path: "t".into(),
                    raw_start_path: None,
                    all_files: false,
                    max_depth: None,
                })
                .await
                .expect_err("du from another contract");
            let df = client
                .df(DfRequest {
                    contract_version: version,
                    module: "test".into(),
                    path: "t".into(),
                    raw_path: None,
                })
                .await
                .expect_err("df from another contract");
            for status in [du, df] {
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
        }
        // The same requests from this contract are answered.
        let mut rows = client
            .du(DuRequest {
                contract_version: CONTRACT_VERSION,
                module: "test".into(),
                start_path: "t".into(),
                raw_start_path: None,
                all_files: false,
                max_depth: Some(0),
            })
            .await
            .expect("du from this contract")
            .into_inner();
        assert!(rows.message().await.unwrap().is_some());
    });
}
