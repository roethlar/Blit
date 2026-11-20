//! EXCLUDE_CASE_MATCHING slice 3b (AC-8, AC-11, AC-12): pattern files —
//! `--exclude-from FILE` and `--include-from FILE` — through the real
//! binary.
//!
//! A pattern file is read into the command's pattern lists before
//! anything else looks at them, so each case here is the same command
//! with the patterns typed. Every case runs on every OS; the one that
//! needs a file name ending in a space is Unix-only (Windows trims it).

use std::fs;
use std::path::Path;
use std::process::Stdio;
use std::time::Duration;

use tempfile::tempdir;

mod common;
use common::{cli_command, run_with_timeout};

struct Ran {
    code: Option<i32>,
    stdout: String,
    stderr: String,
}

impl Ran {
    fn all(&self) -> String {
        format!("{}{}", self.stdout, self.stderr)
    }
}

/// `blit <args>` with nobody at the terminal.
fn blit(args: &[&str]) -> Ran {
    let mut cmd = cli_command();
    cmd.args(args).stdin(Stdio::null());
    let output = run_with_timeout(cmd, Duration::from_secs(60));
    Ran {
        code: output.status.code(),
        stdout: String::from_utf8_lossy(&output.stdout).into_owned(),
        stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
    }
}

fn with_slash(path: &Path) -> String {
    format!("{}/", path.display())
}

fn write(root: &Path, rel: &str) {
    let path = root.join(rel);
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, rel).unwrap();
}

/// What `dir` holds at its top, sorted.
fn names(dir: &Path) -> Vec<String> {
    let mut names: Vec<String> = fs::read_dir(dir)
        .unwrap()
        .map(|entry| entry.unwrap().file_name().to_string_lossy().into_owned())
        .collect();
    names.sort();
    names
}

/// A source with one file and a destination holding entries only it has.
fn trees(root: &Path, at_destination: &[&str]) -> (std::path::PathBuf, std::path::PathBuf) {
    let (src, dst) = (root.join("src"), root.join("dst"));
    write(&src, "keep.txt");
    fs::create_dir_all(&dst).unwrap();
    for rel in at_destination {
        write(&dst, rel);
    }
    (src, dst)
}

/// AC-8: a pattern file's lines are the command's patterns. Comments and
/// blank lines are skipped, both ends of a line and its CR LF are
/// stripped, a byte-order mark is not part of the first pattern — and the
/// mirror keeps and deletes exactly what the same patterns typed on the
/// command line do. The files are repeatable and add to typed patterns.
#[test]
fn a_pattern_files_lines_are_the_commands_patterns() {
    let at_destination = ["a.tmp", "b.bak", "c.old", "d.log", "extra.txt"];
    let after = ["a.tmp", "b.bak", "c.old", "d.log", "keep.txt"];

    let tmp = tempdir().unwrap();
    let (src, dst) = trees(tmp.path(), &at_destination);
    let (from, to) = (with_slash(&src), with_slash(&dst));
    let typed = blit(&[
        "mirror",
        "--yes",
        "--exclude",
        "*.tmp",
        "--exclude",
        "*.bak",
        "--exclude",
        "*.old",
        "--exclude",
        "*.log",
        &from,
        &to,
    ]);
    assert_eq!(typed.code, Some(0), "{}", typed.all());
    assert_eq!(names(&dst), after, "typed");

    let tmp = tempdir().unwrap();
    let (src, dst) = trees(tmp.path(), &at_destination);
    let (from, to) = (with_slash(&src), with_slash(&dst));
    let first = tmp.path().join("first.txt");
    let second = tmp.path().join("second.txt");
    fs::write(&first, "\u{feff}*.tmp\r\n# backups\r\n\r\n   *.bak   \r\n").unwrap();
    fs::write(&second, "\t*.old\t\n# nothing else\n").unwrap();
    let read = blit(&[
        "mirror",
        "--yes",
        "--exclude-from",
        first.to_str().unwrap(),
        "--exclude",
        "*.log",
        "--exclude-from",
        second.to_str().unwrap(),
        &from,
        &to,
    ]);
    assert_eq!(read.code, Some(0), "{}", read.all());
    assert_eq!(
        names(&dst),
        after,
        "read from two files and the command line"
    );

    // `--include-from`, under the run's case mode like any pattern.
    for (mode, kept) in [(None, false), (Some("--ignore-case"), true)] {
        let tmp = tempdir().unwrap();
        let (src, dst) = (tmp.path().join("src"), tmp.path().join("dst"));
        write(&src, "a.txt");
        write(&src, "B.TXT");
        write(&src, "c.dat");
        fs::create_dir_all(&dst).unwrap();
        let list = tmp.path().join("only.txt");
        fs::write(&list, "*.txt\n").unwrap();
        let mut args = vec!["copy"];
        args.extend(mode);
        let (from, to) = (with_slash(&src), with_slash(&dst));
        args.extend(["--include-from", list.to_str().unwrap(), &from, &to]);
        let ran = blit(&args);
        assert_eq!(ran.code, Some(0), "{}", ran.all());
        assert!(dst.join("a.txt").exists() && !dst.join("c.dat").exists());
        assert_eq!(dst.join("B.TXT").exists(), kept, "{mode:?}");
    }
}

/// AC-8 (Q5): a space at the end of a line is not part of the pattern —
/// `*.jpg ` in a file protects `a.jpg` — and a space that IS wanted at an
/// edge is written `[ ]`: `*.jpg[ ]` protects only the name that ends in
/// one. Unix only: Windows cannot hold a name that ends in a space.
#[cfg(unix)]
#[test]
fn a_space_at_a_lines_edge_is_stripped_and_a_wanted_one_is_written_as_a_class() {
    for (line, kept) in [("*.jpg \n", "a.jpg"), ("*.jpg[ ]\n", "a.jpg ")] {
        let tmp = tempdir().unwrap();
        let (src, dst) = trees(tmp.path(), &["a.jpg", "a.jpg "]);
        let list = tmp.path().join("skip.txt");
        fs::write(&list, line).unwrap();
        let ran = blit(&[
            "mirror",
            "--yes",
            "--exclude-from",
            list.to_str().unwrap(),
            &with_slash(&src),
            &with_slash(&dst),
        ]);
        assert_eq!(ran.code, Some(0), "{}", ran.all());
        assert_eq!(names(&dst), [kept, "keep.txt"], "{line:?}");
    }
}

/// AC-8: a pattern file that is missing, or cannot be read as text,
/// refuses the run before any transfer and before `mirror`'s question —
/// nobody answers here, and nothing changes.
#[test]
fn a_pattern_file_that_cannot_be_read_refuses_the_run_before_the_question() {
    let tmp = tempdir().unwrap();
    let (src, dst) = trees(tmp.path(), &["extra.txt"]);
    let (from, to) = (with_slash(&src), with_slash(&dst));
    let missing = tmp.path().join("no-such-file.txt");
    let not_text = tmp.path().join("not-text.txt");
    fs::write(&not_text, b"*.tmp\n\xff\xfe\n").unwrap();
    for (file, words) in [
        (&missing, "opening pattern file"),
        (&not_text, "must be UTF-8 text"),
    ] {
        for option in ["--exclude-from", "--include-from"] {
            for verb in ["mirror", "copy"] {
                let ran = blit(&[verb, option, file.to_str().unwrap(), &from, &to]);
                assert_ne!(ran.code, Some(0), "{verb} {option}: {}", ran.all());
                assert!(
                    ran.all().contains(words) && ran.all().contains(file.to_str().unwrap()),
                    "{verb} {option}: {}",
                    ran.all()
                );
                assert!(
                    !ran.all().contains("Continue?") && !ran.all().contains("starting"),
                    "{verb} {option}: refused before anything else\n{}",
                    ran.all()
                );
                assert_eq!(names(&dst), ["extra.txt"], "{verb} {option}");
            }
        }
    }
}

/// AC-11 (Q8): a line that is an rsync filter rule — `- pattern`,
/// `+ pattern`, `!` — refuses the run, naming the file and the line, with
/// nothing transferred, asked or noted. `[-] name` is how a real entry
/// that begins dash-space is named. And the `--files-from` list is no
/// pattern file: a name there that begins `- ` stays a name.
#[test]
fn an_rsync_rule_line_refuses_the_run_and_a_class_names_the_real_entry() {
    for (text, line, rule) in [
        ("*.tmp\n- *.jpg\n", 2, "- *.jpg"),
        ("+ *.jpg\n", 1, "+ *.jpg"),
        ("*.tmp\n\n!\n", 3, "!"),
    ] {
        let tmp = tempdir().unwrap();
        let (src, dst) = trees(tmp.path(), &["a.tmp", "extra.txt"]);
        let list = tmp.path().join("rules.txt");
        fs::write(&list, text).unwrap();
        for verb in ["mirror", "copy", "check"] {
            let ran = blit(&[
                verb,
                "--exclude-from",
                list.to_str().unwrap(),
                &with_slash(&src),
                &with_slash(&dst),
            ]);
            let said = ran.all();
            assert_ne!(ran.code, Some(0), "{verb}: {said}");
            assert!(
                said.contains(&format!("pattern file {}, line {line}", list.display()))
                    && said.contains(&format!("{rule:?} is an rsync filter rule")),
                "{verb}: {said}"
            );
            for never in ["Continue?", "starting", "capital letters matter"] {
                assert!(!said.contains(never), "{verb}: before {never:?}\n{said}");
            }
            assert_eq!(names(&dst), ["a.tmp", "extra.txt"], "{verb}");
        }
    }

    // The entry really called `- old.jpg` is named with a class.
    let tmp = tempdir().unwrap();
    let (src, dst) = trees(tmp.path(), &["- old.jpg", "old.jpg"]);
    let list = tmp.path().join("skip.txt");
    fs::write(&list, "[-] old.jpg\n").unwrap();
    let ran = blit(&[
        "mirror",
        "--yes",
        "--exclude-from",
        list.to_str().unwrap(),
        &with_slash(&src),
        &with_slash(&dst),
    ]);
    assert_eq!(ran.code, Some(0), "{}", ran.all());
    assert_eq!(names(&dst), ["- old.jpg", "keep.txt"]);

    // `--files-from` holds names: `- name` there is a name.
    let tmp = tempdir().unwrap();
    let (src, dst) = (tmp.path().join("src"), tmp.path().join("dst"));
    write(&src, "- name.txt");
    write(&src, "other.txt");
    fs::create_dir_all(&dst).unwrap();
    let list = tmp.path().join("list.txt");
    fs::write(&list, "- name.txt\n").unwrap();
    let ran = blit(&[
        "copy",
        "--files-from",
        list.to_str().unwrap(),
        &with_slash(&src),
        &with_slash(&dst),
    ]);
    assert_eq!(ran.code, Some(0), "{}", ran.all());
    assert_eq!(names(&dst), ["- name.txt"]);
}

/// AC-8: the line about capital letters says which patterns the operator
/// did not type — the count names the file when the pattern shown was
/// typed and the others were read, and the pattern shown names its file
/// when it was read itself.
#[test]
fn the_line_about_capital_letters_names_the_pattern_file() {
    let tmp = tempdir().unwrap();
    let (src, dst) = trees(tmp.path(), &["extra.txt"]);
    let (from, to) = (with_slash(&src), with_slash(&dst));
    let list = tmp.path().join("skip.txt");
    fs::write(&list, "*.tmp\n??\n*.bak\n").unwrap();
    let file = list.to_str().unwrap();

    let ran = blit(&[
        "mirror",
        "--exclude",
        "Thumbs.db",
        "--exclude-from",
        file,
        &from,
        &to,
    ]);
    let line = format!(
        "\"Thumbs.db\" matches Thumbs.db only, and 2 more patterns (from {file}) match only as \
         typed; capital letters matter (--ignore-case would also match thumbs.db, THUMBS.DB)."
    );
    assert!(
        ran.stdout.contains(&format!("{line}\nMirror will delete")),
        "{}",
        ran.all()
    );
    assert_eq!(names(&dst), ["extra.txt"], "nobody answered");

    let ran = blit(&["copy", "--exclude-from", file, &from, &to]);
    let note = format!(
        "blit: \"*.tmp\" (from {file}) matches .tmp only, and 1 more pattern (from {file}) \
         matches only as typed; capital letters matter (--ignore-case would also match .TMP)."
    );
    assert_eq!(ran.code, Some(0), "{}", ran.all());
    assert_eq!(ran.stderr.matches(&note).count(), 1, "{}", ran.all());
}

/// AC-12: `check` takes both options and reads the files as the transfer
/// did, so what the mirror left alone is no difference to it.
#[test]
fn check_reads_pattern_files_as_the_transfer_did() {
    let tmp = tempdir().unwrap();
    let (src, dst) = trees(tmp.path(), &["a.tmp", "sub/b.bak"]);
    let (from, to) = (with_slash(&src), with_slash(&dst));
    let list = tmp.path().join("skip.txt");
    fs::write(&list, "# left alone\n*.tmp\n*.bak\n").unwrap();
    let file = list.to_str().unwrap();

    let mirrored = blit(&["mirror", "--yes", "--exclude-from", file, &from, &to]);
    assert_eq!(mirrored.code, Some(0), "{}", mirrored.all());
    assert!(dst.join("a.tmp").exists() && dst.join("sub/b.bak").exists());

    let same = blit(&["check", "--exclude-from", file, &from, &to]);
    assert_eq!(same.code, Some(0), "no difference:\n{}", same.all());
    let without = blit(&["check", &from, &to]);
    assert_eq!(
        without.code,
        Some(1),
        "the two extras differ:\n{}",
        without.all()
    );
    assert!(
        !same.all().contains("capital letters"),
        "check notes nothing"
    );
}

/// AC-7, Q6: a job keeps the PATTERNS a pattern file held, not the file.
/// The exported job names no file, and runs again — leaving the same
/// entries out — with the file deleted.
#[test]
fn a_job_keeps_the_patterns_read_and_runs_again_without_the_file() {
    let tmp = tempdir().unwrap();
    let (src, dst) = (tmp.path().join("src"), tmp.path().join("dst"));
    for rel in ["keep.txt", "a.tmp", "b.bak"] {
        write(&src, rel);
    }
    fs::create_dir_all(&dst).unwrap();
    let (from, to) = (with_slash(&src), with_slash(&dst));
    let list = tmp.path().join("skip.txt");
    fs::write(&list, "*.tmp\n# and\n*.bak\n").unwrap();
    let job = tmp.path().join("job.json");

    let first = blit(&[
        "copy",
        "--exclude",
        "*.old",
        "--exclude-from",
        list.to_str().unwrap(),
        "--export",
        job.to_str().unwrap(),
        &from,
        &to,
    ]);
    assert_eq!(first.code, Some(0), "{}", first.all());
    assert_eq!(names(&dst), ["keep.txt"]);

    let text = fs::read_to_string(&job).unwrap();
    let exported: serde_json::Value = serde_json::from_str(&text).unwrap();
    assert_eq!(
        exported["spec"]["options"]["exclude"],
        serde_json::json!(["*.old", "*.tmp", "*.bak"])
    );
    assert!(!text.contains("skip.txt"), "the job names no file:\n{text}");

    fs::remove_file(&list).unwrap();
    fs::remove_file(dst.join("keep.txt")).unwrap();
    let again = blit(&["jobs", "run", job.to_str().unwrap()]);
    assert_eq!(again.code, Some(0), "{}", again.all());
    assert_eq!(names(&dst), ["keep.txt"], "the same entries left out");
}

/// `move` takes no pattern, so it takes no pattern file: both options are
/// refused by name with the file unread (it does not exist here), and its
/// help does not offer them. `copy`, `mirror` and `check` offer both.
#[test]
fn move_refuses_pattern_files_and_help_offers_them_where_they_are_taken() {
    let tmp = tempdir().unwrap();
    let (src, dst) = trees(tmp.path(), &[]);
    let missing = tmp.path().join("never-read.txt");
    for option in ["--exclude-from", "--include-from"] {
        let ran = blit(&[
            "move",
            "--yes",
            option,
            missing.to_str().unwrap(),
            &with_slash(&src),
            &with_slash(&dst),
        ]);
        assert_ne!(ran.code, Some(0), "{}", ran.all());
        assert!(
            ran.all().contains("move does not support filters") && ran.all().contains(option),
            "{}",
            ran.all()
        );
        assert!(
            !ran.all().contains("opening pattern file"),
            "refused before the file is read:\n{}",
            ran.all()
        );
        assert!(src.join("keep.txt").exists(), "nothing moved");
    }
    for (verb, offered) in [
        ("copy", true),
        ("mirror", true),
        ("check", true),
        ("move", false),
    ] {
        let ran = blit(&[verb, "--help"]);
        assert_eq!(ran.code, Some(0));
        for option in ["--exclude-from", "--include-from"] {
            assert_eq!(
                ran.stdout.contains(option),
                offered,
                "{verb} --help and {option}"
            );
        }
    }
}
