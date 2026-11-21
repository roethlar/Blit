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
use common::{cli_bin, run_with_timeout};

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
/// The settings folder is the test's own, under `scratch` (cr-ecm3-6): a
/// folder shared by every test is first used by several at once.
fn blit(scratch: &Path, args: &[&str], answer: Option<&str>) -> Ran {
    let mut cmd = std::process::Command::new(cli_bin());
    cmd.arg("--config-dir")
        .arg(scratch.join("settings"))
        .args(args);
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
///
/// UTILITY_VERBS_LOCAL_PARITY AC-10: "no" ends the command with exit
/// status 0; with nobody to answer it ends with 1 and names the option
/// that skips the question. It used to print "Aborted." and exit 0 there,
/// and an unattended job that did nothing looked like one that had run.
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
        assert!(untouched(&dst), "{answer:?}: nothing copied or deleted");
        if answer.is_some() {
            assert!(ran.stdout.contains("Aborted."), "{}", ran.all());
            assert_eq!(ran.code, Some(0), "{}", ran.all());
        } else {
            assert!(!ran.stdout.contains("Aborted."), "{}", ran.all());
            assert!(
                ran.stderr.contains("no answer could be read") && ran.stderr.contains("-y"),
                "{}",
                ran.all()
            );
            assert_eq!(ran.code, Some(1), "{}", ran.all());
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

/// cr-ecm3-5: the line follows what the matcher does. A pattern whose only
/// letters sit in a class (`*[a-z]`), and one whose letter has no single
/// upper form in the standard tables (`*ß`), are spoken of like any other —
/// as typed, this mirror deletes `TEST` and `CAPẞ`, which `--ignore-case`
/// would keep, and the line was silent about both.
#[test]
fn the_line_speaks_of_every_pattern_whose_reading_depends_on_case() {
    for (pattern, line, kept_only_ignoring_case) in [
        (
            "*[a-z]",
            "\"*[a-z]\" matches [a-z] only; capital letters matter (--ignore-case would also \
             match [A-Z]).",
            "TEST",
        ),
        (
            "*ß",
            "\"*ß\" matches ß only; capital letters matter (--ignore-case would also match ẞ).",
            "CAPẞ",
        ),
    ] {
        let tmp = tempdir().unwrap();
        let (src, dst) = (tmp.path().join("src"), tmp.path().join("dst"));
        write(&src, "keep.1");
        write(&dst, kept_only_ignoring_case);
        let (from, to) = (with_slash(&src), with_slash(&dst));
        let asked = blit(
            tmp.path(),
            &["mirror", "--exclude", pattern, &from, &to],
            Some("y\n"),
        );
        assert_eq!(asked.code, Some(0), "{}", asked.all());
        assert!(
            asked.stdout.contains(&format!("{line}\n{QUESTION}")),
            "{pattern}: {}",
            asked.all()
        );
        assert!(
            !dst.join(kept_only_ignoring_case).exists(),
            "{pattern}: as typed it is not named, as the line said"
        );

        write(&dst, kept_only_ignoring_case);
        let ignoring = blit(
            tmp.path(),
            &[
                "mirror",
                "-y",
                "--ignore-case",
                "--exclude",
                pattern,
                &from,
                &to,
            ],
            None,
        );
        assert_eq!(ignoring.code, Some(0), "{}", ignoring.all());
        assert!(
            dst.join(kept_only_ignoring_case).exists(),
            "{pattern}: with --ignore-case it is named and kept"
        );
    }
}

/// cr-ecm4-3: the line is said of the filter the run uses.
///
/// A negated class: `--ignore-case` makes `[!x]` take less, and the line
/// says so instead of showing `PHOTO[!x].JPG` as something the option
/// "would also match" — a glob admitting names the option does not match.
/// What it says is what happens: as typed `photoX.jpg` is named and kept;
/// with the option it is not, and is deleted.
///
/// An exclude the run has just reported as unable to match anything is not
/// then described as matching, nor counted; and an absolute exclude under
/// the source is described as what it names there, the source's own path
/// never shown in another case.
#[test]
fn the_line_is_said_of_the_filter_the_run_uses() {
    let tmp = tempdir().unwrap();
    let (src, dst) = (tmp.path().join("src"), tmp.path().join("dst"));
    write(&src, "keep.1");
    let (from, to) = (with_slash(&src), with_slash(&dst));
    let fill = || {
        write(&dst, "photoX.jpg");
        write(&dst, "PHOTOY.JPG");
    };

    fill();
    let asked = blit(
        tmp.path(),
        &["mirror", "--exclude", "photo[!x].jpg", &from, &to],
        Some("y\n"),
    );
    assert_eq!(asked.code, Some(0), "{}", asked.all());
    let line = "\"photo[!x].jpg\" matches exactly as typed; capital letters matter \
                (--ignore-case changes what it matches: [!x] would stop matching X).";
    assert!(
        asked.stdout.contains(&format!("{line}\n{QUESTION}")),
        "{}",
        asked.all()
    );
    assert!(!asked.all().contains("also match"), "{}", asked.all());
    assert!(dst.join("photoX.jpg").exists(), "as typed, [!x] takes X");
    assert!(
        !dst.join("PHOTOY.JPG").exists(),
        "as typed, capitals matter"
    );

    fill();
    let ignoring = blit(
        tmp.path(),
        &[
            "mirror",
            "-y",
            "--ignore-case",
            "--exclude",
            "photo[!x].jpg",
            &from,
            &to,
        ],
        None,
    );
    assert_eq!(ignoring.code, Some(0), "{}", ignoring.all());
    assert!(
        !dst.join("photoX.jpg").exists(),
        "with the option [!x] stops matching X, as the line said"
    );
    assert!(dst.join("PHOTOY.JPG").exists(), "and other capitals match");

    // An exclude that cannot match anything: said once, and no line about
    // capital letters is made from it.
    let outside = tmp.path().join("elsewhere").join("Thing");
    let outside = outside.to_str().unwrap();
    let alone = blit(
        tmp.path(),
        &["mirror", "--exclude", outside, &from, &to],
        Some("n\n"),
    );
    assert!(
        alone.stderr.contains(&format!(
            "--exclude '{outside}' cannot match anything under"
        )),
        "{}",
        alone.all()
    );
    assert!(!alone.all().contains(MARK), "{}", alone.all());
    assert!(alone.stdout.contains(QUESTION), "{}", alone.all());
    let beside = blit(
        tmp.path(),
        &[
            "mirror",
            "--exclude",
            outside,
            "--exclude",
            "*Cache",
            &from,
            &to,
        ],
        Some("n\n"),
    );
    assert!(
        beside.stdout.contains(&format!("{CACHE_LINE}\n{QUESTION}")),
        "the pattern that can match, and no count of the other:\n{}",
        beside.all()
    );

    // An absolute exclude under the source: echoed as typed, described as
    // what it names there.
    write(&src, "Data/in.txt");
    let inside = src.join("Data");
    let inside = inside.to_str().unwrap();
    let absolute = blit(
        tmp.path(),
        &["mirror", "--exclude", inside, &from, &to],
        Some("n\n"),
    );
    assert!(
        absolute.stdout.contains(&format!(
            "\"{inside}\" matches Data only; capital letters matter (--ignore-case would also \
             match data, DATA).\n{QUESTION}"
        )),
        "{}",
        absolute.all()
    );
}

/// cr-ecm5-6: under `--delete-scope all` a filter chooses what is copied
/// and protects nothing at the destination, and the question says so in
/// place of the line about capital letters. That line appeared as usual
/// with `--exclude '*Cache'` — "matches Cache only" — and the mirror then
/// deleted the destination's own `bCache`, which the pattern names.
#[test]
fn delete_scope_all_says_a_filter_protects_nothing_at_the_destination() {
    const SCOPE_LINE: &str = "--delete-scope all: the filter chooses what is copied, not what \
                              is kept; destination entries it leaves out are deleted too.";
    let tmp = tempdir().unwrap();
    let (src, dst) = trees(tmp.path());
    let (from, to) = (with_slash(&src), with_slash(&dst));
    let all = ["mirror", "--delete-scope", "all", "--exclude", "*Cache"];

    let asked = blit(tmp.path(), &[&all[..], &[&from, &to]].concat(), Some("n\n"));
    assert!(
        asked.stdout.contains(&format!("{SCOPE_LINE}\n{QUESTION}")),
        "the line, then the question:\n{}",
        asked.all()
    );
    assert!(!asked.all().contains(MARK), "{}", asked.all());
    assert!(untouched(&dst), "answered no: nothing copied or deleted");

    // And it is what happens: the entry the exclude names is deleted.
    let agreed = blit(tmp.path(), &[&all[..], &[&from, &to]].concat(), Some("y\n"));
    assert_eq!(agreed.code, Some(0), "{}", agreed.all());
    assert!(!dst.join("bCache").exists(), "{}", agreed.all());
    assert!(!dst.join("cCACHE").exists() && !dst.join("extra.txt").exists());
    assert!(dst.join("keep.txt").exists());

    // A filter of any kind brings the line; no filter, no line; and the
    // default scope keeps the line about capital letters.
    let sized = blit(
        tmp.path(),
        &[
            "mirror",
            "--delete-scope",
            "all",
            "--min-size",
            "1",
            &from,
            &to,
        ],
        Some("n\n"),
    );
    assert!(sized.stdout.contains(SCOPE_LINE), "{}", sized.all());
    let unfiltered = blit(
        tmp.path(),
        &["mirror", "--delete-scope", "all", &from, &to],
        Some("n\n"),
    );
    assert!(
        !unfiltered.all().contains("--delete-scope all:"),
        "{}",
        unfiltered.all()
    );
    assert!(unfiltered.stdout.contains(QUESTION), "{}", unfiltered.all());
    let subset = blit(
        tmp.path(),
        &["mirror", "--exclude", "*Cache", &from, &to],
        Some("n\n"),
    );
    assert!(subset.stdout.contains(CACHE_LINE), "{}", subset.all());
    assert!(
        !subset.all().contains("--delete-scope all:"),
        "{}",
        subset.all()
    );

    // With `-y` nothing is asked and nothing is said; a dry run asks
    // nothing and says it as a note; `copy` deletes nothing, so its note
    // is still the one about capital letters.
    let waived = blit(
        tmp.path(),
        &[&all[..], &["--dry-run", "-y", &from, &to]].concat(),
        None,
    );
    assert!(
        !waived.all().contains("--delete-scope all:"),
        "{}",
        waived.all()
    );
    let dry = blit(
        tmp.path(),
        &[&all[..], &["--dry-run", &from, &to]].concat(),
        None,
    );
    assert!(
        dry.stderr.contains(&format!("blit: {SCOPE_LINE}")),
        "{}",
        dry.all()
    );
    let copy = blit(
        tmp.path(),
        &[
            "copy",
            "--delete-scope",
            "all",
            "--exclude",
            "*Cache",
            "--dry-run",
            &from,
            &to,
        ],
        None,
    );
    assert!(copy.stderr.contains(CACHE_LINE), "{}", copy.all());
}

/// cr-ecm5-4: what the line shows keeps a directory the pattern requires.
/// `*/Cache` names `Cache` one level down, never at the top. The line used
/// to drop the `*/` and say `--ignore-case` would also match `cache`; the
/// mirror run that way deleted the top-level `cache` and kept `sub/cache`.
/// Now it shows `*/cache`, and that is what happens.
#[test]
fn the_line_keeps_a_directory_level_the_pattern_requires() {
    let tmp = tempdir().unwrap();
    let (src, dst) = (tmp.path().join("src"), tmp.path().join("dst"));
    write(&src, "keep.1");
    let (from, to) = (with_slash(&src), with_slash(&dst));
    let fill = || {
        write(&dst, "cache");
        write(&dst, "sub/cache");
    };

    fill();
    let asked = blit(
        tmp.path(),
        &["mirror", "--exclude", "*/Cache", &from, &to],
        Some("n\n"),
    );
    let line = "\"*/Cache\" matches */Cache only; capital letters matter \
                (--ignore-case would also match */cache, */CACHE).";
    assert!(
        asked.stdout.contains(&format!("{line}\n{QUESTION}")),
        "{}",
        asked.all()
    );

    let ignoring = blit(
        tmp.path(),
        &[
            "mirror",
            "-y",
            "--ignore-case",
            "--exclude",
            "*/Cache",
            &from,
            &to,
        ],
        None,
    );
    assert_eq!(ignoring.code, Some(0), "{}", ignoring.all());
    assert!(
        dst.join("sub/cache").exists(),
        "one level down it is named, as the line showed"
    );
    assert!(
        !dst.join("cache").exists(),
        "at the top it is not, as the line showed"
    );
}

/// cr-ecm6-2: `Cache/**` names what is UNDER a folder `Cache`, never a
/// file of that name, and the line shows it so. It used to trim the `/**`
/// and say `--ignore-case` would also match `cache`; the mirror run that
/// way deleted the destination's own FILE `cache`. Now it shows `cache/**`
/// — and with the option the file is deleted as before (the pattern never
/// named it) while what is under a folder `cache` is kept.
#[test]
fn the_line_keeps_what_is_under_a_folder_apart_from_the_folder() {
    let tmp = tempdir().unwrap();
    let (src, dst) = (tmp.path().join("src"), tmp.path().join("dst"));
    write(&src, "keep.1");
    let (from, to) = (with_slash(&src), with_slash(&dst));
    write(&dst, "cache");
    write(&dst, "sub/CACHE/inside.txt");

    let asked = blit(
        tmp.path(),
        &["mirror", "--exclude", "**/Cache/**", &from, &to],
        Some("n\n"),
    );
    let line = "\"**/Cache/**\" matches Cache/** only; capital letters matter \
                (--ignore-case would also match cache/**, CACHE/**).";
    assert!(
        asked.stdout.contains(&format!("{line}\n{QUESTION}")),
        "{}",
        asked.all()
    );

    let ignoring = blit(
        tmp.path(),
        &[
            "mirror",
            "-y",
            "--ignore-case",
            "--exclude",
            "**/Cache/**",
            &from,
            &to,
        ],
        None,
    );
    assert_eq!(ignoring.code, Some(0), "{}", ignoring.all());
    assert!(
        dst.join("sub/CACHE/inside.txt").exists(),
        "what is under a folder of that name is named, as the line showed"
    );
    assert!(
        !dst.join("cache").exists(),
        "a file of that name is not, as the line showed"
    );
}

/// cr-ecm7-2, cr-ecm7-3, cr-ecm7-4, cr-ecm8-2, cr-ecm9-2, cr-ecm10-2, cr-ecm10-1, cr-ecm10-5: commands whose line claimed
/// something the mirror then contradicted — or was absent when it had
/// something to say — each shown the line it gets now and run both ways.
///
/// - `**//Cache`: the line showed `Cache` and said the option would also
///   match `cache`; with the option the destination's own `cache` was
///   deleted. The second `/` is the pattern's own, and with it the pattern
///   names no entry at all: the line now offers no spelling.
/// - `Keep` and `[!a].dat`: the line gave `Keep`'s spellings and counted
///   the second pattern; as typed the mirror kept `A.dat`, with the option
///   it deleted it. The pattern the option narrows is the one shown.
/// - `{a,A}` and `*.Doc`: the line chose `{a,A}` and offered `{a,a}` and
///   `{A,A}`, both matched already; what changes is that `x.doc` is kept.
#[test]
fn the_line_claims_only_what_the_mirror_then_does() {
    struct Case {
        excludes: &'static [&'static str],
        includes: &'static [&'static str],
        line: &'static str,
        /// Destination-only entries, with whether each is kept as typed
        /// and whether it is kept with `--ignore-case`.
        entries: &'static [(&'static str, bool, bool)],
    }
    for case in [
        Case {
            excludes: &["**//Cache"],
            includes: &[],
            line: "\"**//Cache\" matches exactly as typed; capital letters matter \
                   (--ignore-case ignores them).",
            entries: &[("cache", false, false), ("Cache", false, false)],
        },
        Case {
            excludes: &["Keep", "[!a].dat"],
            includes: &[],
            line: "\"[!a].dat\" matches exactly as typed, and 1 more pattern matches only as \
                   typed; capital letters matter (--ignore-case changes what they match: [!a] \
                   would stop matching A).",
            entries: &[
                ("A.dat", true, false),
                ("b.dat", true, true),
                ("keep", false, true),
            ],
        },
        // cr-ecm8-2: a letter that is neither the lower nor the upper case
        // of the ones written. The line was absent; the mirror deleted `ς`,
        // which `--ignore-case` keeps.
        Case {
            excludes: &["{Σ,σ}"],
            includes: &[],
            line: "\"{Σ,σ}\" matches {Σ,σ} only; capital letters matter (--ignore-case would \
                   also match ς).",
            entries: &[("ς", false, true), ("σ", true, true)],
        },
        // cr-ecm8-2: a class beside the letters. The line was absent; the
        // mirror deleted `C`. Not every name can be asked about, so the
        // line states the rule and claims no more.
        Case {
            excludes: &["{[a-c],A,B}"],
            includes: &[],
            line: "\"{[a-c],A,B}\" matches exactly as typed; capital letters matter \
                   (--ignore-case ignores them).",
            entries: &[("C", false, true), ("B", true, true), ("c", true, true)],
        },
        // cr-ecm8-2: the narrowing was credited to `[!a]` and `A`; `A.1`
        // stays kept, and `B.2` is what the option stops keeping.
        Case {
            excludes: &["{[!a].1,A.1,[!b].2}"],
            includes: &[],
            line: "\"{[!a].1,A.1,[!b].2}\" matches exactly as typed; capital letters matter \
                   (--ignore-case changes what it matches: [!b] would stop matching B).",
            entries: &[
                ("A.1", true, true),
                ("B.2", true, false),
                ("x.1", true, true),
            ],
        },
        // cr-ecm9-2: one real loss is enough, and it is shown first. The
        // line gave `Keep`'s spellings; `A.txt` stays kept by the
        // alternative beside the class, and `B.txt` is what the option
        // stops keeping.
        Case {
            excludes: &["Keep", "{[!ab].txt,A.txt}"],
            includes: &[],
            line: "\"{[!ab].txt,A.txt}\" matches exactly as typed, and 1 more pattern matches \
                   only as typed; capital letters matter (--ignore-case changes what they \
                   match: [!ab] would stop matching B).",
            entries: &[
                ("A.txt", true, true),
                ("B.txt", true, false),
                ("keep", false, true),
            ],
        },
        // cr-ecm9-2: a gain is judged against the whole filter. For `a`
        // the line offered `A`, which the pattern beside it keeps already;
        // what the option changes is `x.doc`.
        Case {
            excludes: &["a", "A", "*.Doc"],
            includes: &[],
            line: "\"*.Doc\" matches .Doc only, and 2 more patterns match only as typed; \
                   capital letters matter (--ignore-case would also match .doc, .DOC).",
            entries: &[("A", true, true), ("a", true, true), ("x.doc", false, true)],
        },
        // cr-ecm10-1: beside `./A` there was no line at all — `./A` keeps
        // `A` at the root, where the name was tried — and the mirror
        // deleted `sub/A`, which the option keeps. The line stays, in
        // words that claim nothing about any one name.
        Case {
            excludes: &["a", "./A"],
            includes: &[],
            line: "\"a\" matches exactly as typed, and 1 more pattern matches only as typed; \
                   capital letters matter (--ignore-case ignores them).",
            entries: &[("sub/A", false, true), ("A", true, true), ("a", true, true)],
        },
        // cr-ecm10-1: the line gave `Keep`'s spellings; the one name tried
        // for `[!a]*.txt` was `Ax.txt`, which the third pattern keeps, and
        // the option gave up `Ay.txt`.
        Case {
            excludes: &["Keep", "[!a]*.txt", "Ax.txt"],
            includes: &[],
            line: "\"[!a]*.txt\" matches exactly as typed, and 2 more patterns match only as \
                   typed; capital letters matter (--ignore-case changes what they match: [!a] \
                   would stop matching A).",
            entries: &[
                ("Ay.txt", true, false),
                ("Ax.txt", true, true),
                ("keep", false, true),
            ],
        },
        // cr-ecm10-2: includes and excludes together. The line named the
        // loss of `A` at `[!a].bin`; no include names `A.bin`, and both
        // mirrors leave it alone. What the option gives up is `B.txt`.
        Case {
            excludes: &["[!a].bin", "[!b].txt"],
            includes: &["*.txt"],
            line: "\"[!b].txt\" matches exactly as typed, and 2 more patterns match only as \
                   typed; capital letters matter (--ignore-case changes what they match: [!b] \
                   would stop matching B).",
            entries: &[("A.bin", true, true), ("B.txt", true, false)],
        },
        // cr-ecm11-3: the line said `--ignore-case would also match A`.
        // Both mirrors keep `A`, which `[Aa]` leaves out either way; the
        // file the option's mirror deletes is `xA`, and that is the name
        // the line now prints — the one it checked.
        Case {
            excludes: &["[Aa]"],
            includes: &["*a"],
            line: "\"*a\" matches exactly as typed; capital letters matter (--ignore-case \
                   would also match xA).",
            entries: &[("xA", true, false), ("A", true, true), ("a", true, true)],
        },
        // cr-ecm13-7: and with alternatives of which one changes — the
        // line said `would also match {A,B}`.
        Case {
            excludes: &["[Aa]"],
            includes: &["{a,b}"],
            line: "\"{a,b}\" matches exactly as typed; capital letters matter (--ignore-case \
                   would also match B).",
            entries: &[("B", true, false), ("A", true, true), ("b", false, false)],
        },
        // cr-ecm14-5: and with a wildcard in the include, beside a pattern
        // that leaves some of its names out — the line said `would also
        // match A?B`, and `A1B` is one of those: both mirrors keep it.
        Case {
            excludes: &["?1?"],
            includes: &["a?b"],
            line: "\"a?b\" matches exactly as typed; capital letters matter (--ignore-case \
                   would also match AxB).",
            entries: &[
                ("AxB", true, false),
                ("A1B", true, true),
                ("axb", false, false),
            ],
        },
        // cr-ecm12-4: and with a class in the include — the line said
        // `would also match [A]`.
        Case {
            excludes: &["[Aa]"],
            includes: &["*[a]"],
            line: "\"*[a]\" matches exactly as typed; capital letters matter (--ignore-case \
                   would also match xA).",
            entries: &[("xA", true, false), ("A", true, true), ("a", true, true)],
        },
        // cr-ecm10-5: `Q[b-~]` was offered as a spelling the option would
        // also match, beside the pattern typed exactly so. What the
        // option adds is `QB`; no spelling of the pattern denotes it, and
        // none is offered.
        Case {
            excludes: &["q[b-~]", "Q[b-~]"],
            includes: &[],
            line: "\"q[b-~]\" matches exactly as typed, and 1 more pattern matches only as \
                   typed; capital letters matter (--ignore-case ignores them).",
            entries: &[("QB", false, true), ("Qb", true, true), ("qb", true, true)],
        },
        Case {
            // cr-ecm11-5: `{a,A}` reads the same both ways and is counted
            // all the same — by its text, like every other pattern.
            excludes: &["{a,A}", "*.Doc"],
            includes: &[],
            line: "\"*.Doc\" matches .Doc only, and 1 more pattern matches only as typed; \
                   capital letters matter (--ignore-case would also match .doc, .DOC).",
            entries: &[
                ("x.doc", false, true),
                ("x.Doc", true, true),
                ("a", true, true),
            ],
        },
    ] {
        let tmp = tempdir().unwrap();
        let (src, dst) = (tmp.path().join("src"), tmp.path().join("dst"));
        write(&src, "keep.1");
        let (from, to) = (with_slash(&src), with_slash(&dst));
        let mut args = vec!["mirror"];
        for exclude in case.excludes {
            args.extend(["--exclude", exclude]);
        }
        for include in case.includes {
            args.extend(["--include", include]);
        }
        let fill = || {
            for (entry, _, _) in case.entries {
                // On a filesystem that folds case, two spellings are one
                // entry; the first written stands for both.
                if !dst.join(entry).exists() {
                    write(&dst, entry);
                }
            }
        };

        fill();
        let asked = blit(
            tmp.path(),
            &[&args[..], &[&from, &to]].concat(),
            Some("y\n"),
        );
        assert_eq!(asked.code, Some(0), "{:?}: {}", case.excludes, asked.all());
        assert!(
            asked.stdout.contains(&format!("{}\n{QUESTION}", case.line)),
            "{:?}: {}",
            case.excludes,
            asked.all()
        );
        let folds = dst.join("KEEP.1").exists();
        for (entry, kept_as_typed, _) in case.entries {
            if !folds {
                assert_eq!(
                    dst.join(entry).exists(),
                    *kept_as_typed,
                    "{:?}, as typed: {entry}",
                    case.excludes
                );
            }
        }

        fill();
        let ignoring = blit(
            tmp.path(),
            &[&args[..], &["-y", "--ignore-case", &from, &to]].concat(),
            None,
        );
        assert_eq!(ignoring.code, Some(0), "{}", ignoring.all());
        for (entry, _, kept_ignoring_case) in case.entries {
            if !folds {
                assert_eq!(
                    dst.join(entry).exists(),
                    *kept_ignoring_case,
                    "{:?}, with --ignore-case: {entry}",
                    case.excludes
                );
            }
        }
    }
}

/// cr-ecm9-1: an exclude whose RULE cannot be compiled is refused before
/// the question, like any pattern that is no glob. `keep\/` is one as
/// typed; the rule read from it, without its trailing `/`, is `keep\`,
/// which the compiler refuses — and which was then compared as text, in
/// the typed case only. The question offered `KEEP\/` as a spelling
/// `--ignore-case` would also match; that mirror deleted `KEEP\`.
#[cfg(unix)]
#[test]
fn an_exclude_whose_rule_does_not_compile_is_refused_before_the_question() {
    for case_mode in [None, Some("--ignore-case"), Some("--match-case")] {
        let tmp = tempdir().unwrap();
        let (src, dst) = (tmp.path().join("src"), tmp.path().join("dst"));
        write(&src, "new.txt");
        for mine in [r"keep\", r"KEEP\", "extra.txt"] {
            write(&dst, mine);
        }
        let (from, to) = (with_slash(&src), with_slash(&dst));
        let mut args = vec!["mirror", "--exclude", r"keep\/"];
        args.extend(case_mode);
        args.extend([from.as_str(), to.as_str()]);
        let ran = blit(tmp.path(), &args, Some("y\n"));
        assert_ne!(ran.code, Some(0), "{case_mode:?}\n{}", ran.all());
        assert!(
            ran.stderr
                .contains(r#"--exclude pattern "keep\\/" cannot be used"#)
                && ran.stderr.contains("the rule read from it"),
            "{case_mode:?}\n{}",
            ran.all()
        );
        assert!(
            !ran.stdout.contains(QUESTION),
            "refused before it asks\n{}",
            ran.all()
        );
        assert!(
            dst.join(r"keep\").exists()
                && dst.join(r"KEEP\").exists()
                && dst.join("extra.txt").exists()
                && !dst.join("new.txt").exists(),
            "{case_mode:?}: nothing copied or deleted\n{}",
            ran.all()
        );
    }
}

/// `blit <args>` with a question in it: once the question is on the
/// screen, `meanwhile` runs (it is given everything shown so far); then
/// the answer is "y".
fn answered_yes_after(scratch: &Path, args: &[&str], meanwhile: impl FnOnce(&str)) -> Ran {
    use std::io::{Read as _, Write as _};
    use std::sync::{Arc, Mutex};

    let mut child = std::process::Command::new(cli_bin())
        .arg("--config-dir")
        .arg(scratch.join("settings"))
        .args(args)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn blit");
    let shown = Arc::new(Mutex::new(Vec::new()));
    let reader = {
        let (mut stdout, shown) = (child.stdout.take().unwrap(), Arc::clone(&shown));
        std::thread::spawn(move || {
            let mut chunk = [0u8; 4096];
            while let Ok(read) = stdout.read(&mut chunk) {
                if read == 0 {
                    break;
                }
                shown.lock().unwrap().extend_from_slice(&chunk[..read]);
            }
        })
    };
    let mut stderr = child.stderr.take().unwrap();
    let said = std::thread::spawn(move || {
        let mut text = String::new();
        let _ = stderr.read_to_string(&mut text);
        text
    });
    let text_shown = || String::from_utf8_lossy(&shown.lock().unwrap()).into_owned();

    let asked = std::time::Instant::now();
    while !text_shown().contains("Continue?") {
        assert!(
            asked.elapsed() < Duration::from_secs(60),
            "no question: {}",
            text_shown()
        );
        std::thread::sleep(Duration::from_millis(20));
    }
    meanwhile(&text_shown());
    child.stdin.take().unwrap().write_all(b"y\n").unwrap();
    let status = child.wait().expect("wait for blit");
    reader.join().unwrap();
    Ran {
        code: status.code(),
        stdout: text_shown(),
        stderr: said.join().unwrap(),
    }
}

/// cr-ecm6-4: the list the question was asked about is the list that
/// runs. `run_command` reads `--files-from` once, before anything is
/// asked, for the job's record; the filter used to open the file again
/// when it was built, after the answer. Changed from `first.txt` to
/// `second.txt` while the question was open, the mirror copied
/// `second.txt` — and its saved job replays `first.txt`. Now the run
/// copies what the record holds.
#[test]
fn the_list_read_before_the_question_is_the_list_that_runs() {
    let tmp = tempdir().unwrap();
    let (src, dst) = (tmp.path().join("src"), tmp.path().join("dst"));
    write(&src, "first.txt");
    write(&src, "second.txt");
    fs::create_dir_all(&dst).unwrap();
    let list = tmp.path().join("list.txt");
    fs::write(&list, "first.txt\n").unwrap();
    let (from, to) = (with_slash(&src), with_slash(&dst));

    let ran = answered_yes_after(
        tmp.path(),
        &[
            "mirror",
            "--save",
            "picked",
            "--files-from",
            list.to_str().unwrap(),
            &from,
            &to,
        ],
        |_shown| fs::write(&list, "second.txt\n").unwrap(),
    );
    assert_eq!(ran.code, Some(0), "{}", ran.all());
    assert!(dst.join("first.txt").exists(), "{}", ran.all());
    assert!(
        !dst.join("second.txt").exists(),
        "the list as it was when the command began\n{}",
        ran.all()
    );

    // The saved job is that same list: run again, it copies the same file.
    fs::remove_file(dst.join("first.txt")).unwrap();
    let again = blit(tmp.path(), &["jobs", "run", "picked"], Some("y\n"));
    assert_eq!(again.code, Some(0), "{}", again.all());
    assert!(dst.join("first.txt").exists() && !dst.join("second.txt").exists());
}

/// cr-ecm5-3: the filter the question showed is the filter that runs. An
/// absolute exclude through an alias of a folder in the source is shown as
/// what it names there, `first/Keep`. While the question waits, the alias
/// is pointed at another folder; the operator, who was shown `first/Keep`,
/// says yes — and `first/Keep` is what is left out. The excludes used to
/// be resolved again once the answer was in, to `second/Keep` by then, and
/// the confirmed mirror copied `first/Keep`.
///
/// Unix: an alias is a symbolic link, which Windows does not let every
/// account make. The rule itself — a filter is built from the resolution
/// it is handed — is guarded on every OS in
/// `blit_core::transfers::filter`.
#[cfg(unix)]
#[test]
fn the_filter_the_question_showed_is_the_filter_that_runs() {
    use std::io::{Read as _, Write as _};
    use std::sync::{Arc, Mutex};

    let tmp = tempdir().unwrap();
    let (src, dst) = (tmp.path().join("src"), tmp.path().join("dst"));
    write(&src, "first/Keep/f.txt");
    write(&src, "second/Keep/g.txt");
    write(&src, "other.txt");
    let alias = tmp.path().join("alias");
    std::os::unix::fs::symlink(src.join("first"), &alias).unwrap();
    let exclude = format!("{}/Keep", alias.display());

    let mut child = std::process::Command::new(cli_bin())
        .arg("--config-dir")
        .arg(tmp.path().join("settings"))
        .args(["mirror", "--exclude", &exclude])
        .arg(with_slash(&src))
        .arg(with_slash(&dst))
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn blit mirror");
    let shown = Arc::new(Mutex::new(Vec::new()));
    let reader = {
        let (mut stdout, shown) = (child.stdout.take().unwrap(), Arc::clone(&shown));
        std::thread::spawn(move || {
            let mut chunk = [0u8; 4096];
            while let Ok(read) = stdout.read(&mut chunk) {
                if read == 0 {
                    break;
                }
                shown.lock().unwrap().extend_from_slice(&chunk[..read]);
            }
        })
    };
    let mut stderr = child.stderr.take().unwrap();
    let said = std::thread::spawn(move || {
        let mut text = String::new();
        let _ = stderr.read_to_string(&mut text);
        text
    });
    let text_shown = || String::from_utf8_lossy(&shown.lock().unwrap()).into_owned();

    // The question is on the screen, and it describes `first/Keep`.
    let asked = std::time::Instant::now();
    while !text_shown().contains("Continue?") {
        assert!(
            asked.elapsed() < Duration::from_secs(60),
            "no question: {}",
            text_shown()
        );
        std::thread::sleep(Duration::from_millis(20));
    }
    assert!(
        text_shown().contains(&format!("\"{exclude}\" matches first/Keep only")),
        "{}",
        text_shown()
    );

    // The alias moves; then the operator answers what they were shown.
    fs::remove_file(&alias).unwrap();
    std::os::unix::fs::symlink(src.join("second"), &alias).unwrap();
    child.stdin.take().unwrap().write_all(b"y\n").unwrap();
    let status = child.wait().expect("wait for blit mirror");
    reader.join().unwrap();
    let said = said.join().unwrap();

    assert!(status.success(), "{}{said}", text_shown());
    assert!(dst.join("other.txt").exists(), "{said}");
    assert!(
        !dst.join("first/Keep").exists(),
        "what the question showed as excluded is left out\n{said}"
    );
    assert!(
        dst.join("second/Keep/g.txt").exists(),
        "and nothing else is\n{said}"
    );
}

/// cr-ecm3-5: no line when a list of names overrides the patterns. With
/// `--files-from`, what is copied is what is listed, whatever a pattern
/// says — the note described a `*.cfg` exclusion while the listed
/// `picked.cfg` was copied.
#[test]
fn no_line_when_a_list_of_names_overrides_the_patterns() {
    let tmp = tempdir().unwrap();
    let (src, dst) = (tmp.path().join("src"), tmp.path().join("dst"));
    write(&src, "picked.cfg");
    write(&src, "other.txt");
    fs::create_dir_all(&dst).unwrap();
    let list = tmp.path().join("list.txt");
    fs::write(&list, "picked.cfg\n").unwrap();
    let ran = blit(
        tmp.path(),
        &[
            "copy",
            "--exclude",
            "*.cfg",
            "--files-from",
            list.to_str().unwrap(),
            &with_slash(&src),
            &with_slash(&dst),
        ],
        None,
    );
    assert_eq!(ran.code, Some(0), "{}", ran.all());
    assert!(
        dst.join("picked.cfg").exists(),
        "the list decides: {}",
        ran.all()
    );
    assert!(!dst.join("other.txt").exists());
    assert!(!ran.all().contains(MARK), "{}", ran.all());
}

/// cr-ecm3-5: one line, whatever is typed. A pattern holding a line break
/// is echoed with the break written as its escape.
#[test]
fn a_pattern_holding_a_line_break_still_makes_one_line() {
    let tmp = tempdir().unwrap();
    let (src, dst) = (tmp.path().join("src"), tmp.path().join("dst"));
    write(&src, "keep.txt");
    fs::create_dir_all(&dst).unwrap();
    let ran = blit(
        tmp.path(),
        &[
            "copy",
            "--exclude",
            "first\nSecond",
            &with_slash(&src),
            &with_slash(&dst),
        ],
        None,
    );
    assert_eq!(ran.code, Some(0), "{}", ran.all());
    let noted: Vec<&str> = ran.stderr.lines().filter(|l| l.contains(MARK)).collect();
    assert_eq!(noted.len(), 1, "{}", ran.stderr);
    assert!(
        noted[0].starts_with(r#"blit: "first\nSecond" matches first\nSecond only;"#),
        "{}",
        noted[0]
    );
    assert!(
        !ran.stderr.lines().any(|l| l.starts_with("Second")),
        "nothing spills onto a second line: {}",
        ran.stderr
    );
}
