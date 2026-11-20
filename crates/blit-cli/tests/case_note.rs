//! EXCLUDE_CASE_MATCHING slice 2 (AC-3, AC-4, AC-5, AC-9): the line about
//! capital letters, through the real binary — in `mirror`'s question, and
//! as a note on stderr from a run that asks nothing.
//!
//! Every case runs on every OS with no skip: the line is built from the
//! command's words alone, never from a disk.

use std::fs;
use std::path::Path;
use std::process::Stdio;
use std::time::Duration;

use tempfile::tempdir;

mod common;
use common::{cli_command, run_with_timeout};

/// AC-3's line, to the letter, for `--exclude '*Cache'`.
const CACHE_LINE: &str = "\"*Cache\" matches Cache only; capital letters matter \
     (--ignore-case would also match cache, CACHE).";
const QUESTION: &str = "Mirror will delete extraneous files at destination '";
const MARK: &str = "capital letters matter";

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

/// `blit <args>` with `answer` on standard input; `None` is nobody there.
fn blit(scratch: &Path, args: &[&str], answer: Option<&str>) -> Ran {
    let mut cmd = cli_command();
    cmd.args(args);
    match answer {
        Some(text) => {
            let path = scratch.join("answer.txt");
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

fn with_slash(path: &Path) -> String {
    format!("{}/", path.display())
}

fn write(root: &Path, rel: &str) {
    let path = root.join(rel);
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, rel).unwrap();
}

/// A source with one file, and a destination holding three things the
/// source lacks: `bCache/` (named by `*Cache` as typed), `cCACHE/` (named
/// only when case is ignored) and `extra.txt` (named by nothing).
fn trees(root: &Path) -> (std::path::PathBuf, std::path::PathBuf) {
    let (src, dst) = (root.join("src"), root.join("dst"));
    write(&src, "keep.txt");
    write(&dst, "bCache/saved.txt");
    write(&dst, "cCACHE/saved.txt");
    write(&dst, "extra.txt");
    (src, dst)
}

fn untouched(dst: &Path) -> bool {
    dst.join("bCache/saved.txt").exists()
        && dst.join("cCACHE/saved.txt").exists()
        && dst.join("extra.txt").exists()
        && !dst.join("keep.txt").exists()
}

/// AC-3: `mirror --exclude '*Cache'` without `-y` shows the line once, as
/// part of its question and just before it; "no" — and nobody answering —
/// deletes nothing; "yes" goes on, and deletes exactly what the line said
/// the pattern does not name.
#[test]
fn mirror_shows_the_line_in_its_question_before_any_delete() {
    for answer in [Some("n\n"), None] {
        let tmp = tempdir().unwrap();
        let (src, dst) = trees(tmp.path());
        let ran = blit(
            tmp.path(),
            &[
                "mirror",
                "--exclude",
                "*Cache",
                &with_slash(&src),
                &with_slash(&dst),
            ],
            answer,
        );
        assert!(
            ran.stdout.contains(&format!("{CACHE_LINE}\n{QUESTION}")),
            "the line, then the question:\n{}",
            ran.all()
        );
        assert_eq!(ran.all().matches(MARK).count(), 1, "{}", ran.all());
        assert!(ran.stdout.contains("Aborted."), "{}", ran.all());
        assert!(untouched(&dst), "{answer:?}: nothing copied or deleted");
        if answer.is_some() {
            assert_eq!(ran.code, Some(0), "{}", ran.all());
        }
    }

    let tmp = tempdir().unwrap();
    let (src, dst) = trees(tmp.path());
    let ran = blit(
        tmp.path(),
        &[
            "mirror",
            "--exclude",
            "*Cache",
            &with_slash(&src),
            &with_slash(&dst),
        ],
        Some("y\n"),
    );
    assert_eq!(ran.code, Some(0), "{}", ran.all());
    assert!(ran.stdout.contains(&format!("{CACHE_LINE}\n{QUESTION}")));
    assert!(dst.join("keep.txt").exists());
    assert!(dst.join("bCache/saved.txt").exists(), "named as typed");
    assert!(
        !dst.join("cCACHE").exists(),
        "not named as typed, as the line said"
    );
    assert!(!dst.join("extra.txt").exists());
}

/// AC-4 (Q9): with the mode stated — `--ignore-case`, or a typed
/// `--match-case` — the question is the normal delete warning alone.
#[test]
fn mirror_with_a_case_mode_typed_asks_its_question_alone() {
    for mode in ["--ignore-case", "--match-case"] {
        let tmp = tempdir().unwrap();
        let (src, dst) = trees(tmp.path());
        let ran = blit(
            tmp.path(),
            &[
                "mirror",
                mode,
                "--exclude",
                "*Cache",
                &with_slash(&src),
                &with_slash(&dst),
            ],
            Some("n\n"),
        );
        assert_eq!(ran.code, Some(0), "{mode}: {}", ran.all());
        assert!(ran.stdout.contains(QUESTION), "{mode}: {}", ran.all());
        assert!(!ran.all().contains(MARK), "{mode}: {}", ran.all());
        assert!(untouched(&dst), "{mode}");
    }
}

/// AC-5: one line however many patterns (the first that holds a letter,
/// excludes before includes, and a count of the others that do); no line
/// when no pattern holds a letter; no question and no line with `-y`.
#[test]
fn the_line_is_one_line_or_none() {
    let several = "\"Thumbs.db\" matches Thumbs.db only, and 2 more patterns match only as \
                   typed; capital letters matter (--ignore-case would also match thumbs.db, \
                   THUMBS.DB).";
    let tmp = tempdir().unwrap();
    let (src, dst) = trees(tmp.path());
    let ran = blit(
        tmp.path(),
        &[
            "mirror",
            "--include",
            "*.TXT",
            "--exclude",
            "??",
            "--exclude",
            "*.1",
            "--exclude",
            "Thumbs.db",
            "--exclude",
            "*Cache",
            &with_slash(&src),
            &with_slash(&dst),
        ],
        Some("n\n"),
    );
    assert!(
        ran.stdout.contains(&format!("{several}\n{QUESTION}")),
        "{}",
        ran.all()
    );
    assert_eq!(ran.all().matches(MARK).count(), 1, "{}", ran.all());
    assert!(untouched(&dst));

    // No pattern holds a letter: the question alone.
    let ran = blit(
        tmp.path(),
        &[
            "mirror",
            "--exclude",
            "??",
            "--exclude",
            "*.1",
            &with_slash(&src),
            &with_slash(&dst),
        ],
        Some("n\n"),
    );
    assert!(ran.stdout.contains(QUESTION), "{}", ran.all());
    assert!(!ran.all().contains(MARK), "{}", ran.all());
    assert!(untouched(&dst));

    // `-y`: no question, no line, and the mirror runs.
    let ran = blit(
        tmp.path(),
        &[
            "mirror",
            "-y",
            "--exclude",
            "*Cache",
            &with_slash(&src),
            &with_slash(&dst),
        ],
        None,
    );
    assert_eq!(ran.code, Some(0), "{}", ran.all());
    assert!(
        !ran.all().contains(MARK) && !ran.all().contains("Continue?"),
        "{}",
        ran.all()
    );
    assert!(dst.join("keep.txt").exists() && !dst.join("extra.txt").exists());
}

/// AC-9 (Q3): `copy` asks nothing, so it prints the line as a note on
/// stderr, once, before the transfer — and copies as it always has, with
/// nobody at the terminal. No note with a case mode typed or with `-y`.
/// A `mirror --dry-run` asks nothing either and notes the same way.
#[test]
fn copy_prints_the_line_as_a_note_and_asks_nothing() {
    let note = "blit: \"*.cfg\" matches .cfg only; capital letters matter (--ignore-case would \
                also match .CFG).";
    let fixture = || {
        let tmp = tempdir().unwrap();
        let (src, dst) = (tmp.path().join("src"), tmp.path().join("dst"));
        for name in ["keep.txt", "a.cfg", "B.CFG"] {
            write(&src, name);
        }
        fs::create_dir_all(&dst).unwrap();
        (tmp, src, dst)
    };

    let (tmp, src, dst) = fixture();
    let noted = blit(
        tmp.path(),
        &[
            "copy",
            "--exclude",
            "*.cfg",
            &with_slash(&src),
            &with_slash(&dst),
        ],
        None,
    );
    assert_eq!(noted.code, Some(0), "{}", noted.all());
    assert_eq!(noted.stderr.matches(note).count(), 1, "{}", noted.all());
    assert_eq!(noted.all().matches(MARK).count(), 1, "{}", noted.all());
    assert!(!noted.stdout.contains(MARK), "stdout is the summary alone");
    assert!(!noted.all().contains("Continue?"), "{}", noted.all());
    assert!(dst.join("keep.txt").exists() && !dst.join("a.cfg").exists());
    assert!(
        dst.join("B.CFG").exists(),
        "not named as typed, as the note said"
    );
    let before_start = noted.stderr.find(note) < noted.stderr.find("starting copy");
    assert!(
        before_start,
        "the note comes before the transfer: {}",
        noted.stderr
    );

    for quiet in ["--match-case", "--ignore-case", "-y"] {
        let (tmp, src, dst) = fixture();
        let ran = blit(
            tmp.path(),
            &[
                "copy",
                quiet,
                "--exclude",
                "*.cfg",
                &with_slash(&src),
                &with_slash(&dst),
            ],
            None,
        );
        assert_eq!(ran.code, noted.code, "{quiet}: {}", ran.all());
        assert!(!ran.all().contains(MARK), "{quiet}: {}", ran.all());
        // The note changes nothing else: a run that reads the pattern the
        // same way (every mode here but `--ignore-case`, which also leaves
        // `B.CFG` out) copies the same files and prints the same summary.
        if quiet != "--ignore-case" {
            assert_eq!(
                ran.stdout.lines().count(),
                noted.stdout.lines().count(),
                "{quiet}: the same summary on stdout"
            );
            assert!(dst.join("B.CFG").exists(), "{quiet}");
        }
        assert!(dst.join("keep.txt").exists(), "{quiet}");
    }

    // No letter, no note.
    let (tmp, src, dst) = fixture();
    let ran = blit(
        tmp.path(),
        &[
            "copy",
            "--exclude",
            "*.1",
            &with_slash(&src),
            &with_slash(&dst),
        ],
        None,
    );
    assert_eq!(ran.code, Some(0));
    assert!(!ran.all().contains(MARK), "{}", ran.all());

    // A dry-run mirror asks nothing: the same note, no question.
    let tmp = tempdir().unwrap();
    let (src, dst) = trees(tmp.path());
    let ran = blit(
        tmp.path(),
        &[
            "mirror",
            "--dry-run",
            "--exclude",
            "*Cache",
            &with_slash(&src),
            &with_slash(&dst),
        ],
        None,
    );
    assert_eq!(ran.code, Some(0), "{}", ran.all());
    assert_eq!(
        ran.stderr.matches(&format!("blit: {CACHE_LINE}")).count(),
        1,
        "{}",
        ran.all()
    );
    assert!(!ran.all().contains("Continue?"), "{}", ran.all());
    assert!(untouched(&dst));
}

/// Records (Q9): a job run again shows or omits the line exactly as the
/// command it was made from did — the record keeps the mode as typed.
#[test]
fn a_job_run_again_shows_the_line_as_its_first_run_did() {
    let note = "blit: \"*.cfg\" matches .cfg only";
    for (mode, noted) in [(None, true), (Some("--match-case"), false)] {
        let tmp = tempdir().unwrap();
        let (src, dst) = (tmp.path().join("src"), tmp.path().join("dst"));
        write(&src, "keep.txt");
        fs::create_dir_all(&dst).unwrap();
        let job = tmp.path().join("job.json");
        let mut args = vec!["copy"];
        args.extend(mode);
        args.extend(["--exclude", "*.cfg", "--export", job.to_str().unwrap()]);
        let (from, to) = (with_slash(&src), with_slash(&dst));
        args.extend([from.as_str(), to.as_str()]);
        let first = blit(tmp.path(), &args, None);
        assert_eq!(first.code, Some(0), "{}", first.all());
        assert_eq!(
            first.stderr.contains(note),
            noted,
            "{mode:?}: {}",
            first.all()
        );

        let again = blit(tmp.path(), &["jobs", "run", job.to_str().unwrap()], None);
        assert_eq!(again.code, Some(0), "{}", again.all());
        assert_eq!(
            again.stderr.contains(note),
            noted,
            "{mode:?}: {}",
            again.all()
        );
    }
}
