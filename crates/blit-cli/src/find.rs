//! `blit find` — `find`, run on a daemon's own disk
//! (`docs/plan/UTILITY_VERBS_LOCAL_PARITY.md` slice 3).
//!
//! `blit find PATH [-name PATTERN] [-iname PATTERN] [-type f|d|l]
//! [-maxdepth N] [-size [+|-]N[ckMG]] [-mtime [+|-]N] [-delete]`: one path
//! per line, each beginning with PATH as typed, for every entry all the
//! tests given hold for. The walk and the tests are the Unix command's
//! (`blit_core::find`, `blit_core::find_name`). An entry the daemon could
//! not read is reported on stderr in `find`'s words and the exit status is
//! 1, the rest of the answer printed all the same.
//!
//! `-delete` (slice 4) removes what the tests hold for, contents before
//! their directory, and prints nothing but what could not be removed. It
//! is taken as the LAST word only: in `find` a test after it does not
//! limit it (`find d -delete -name '*.tmp'` removes all of `d`). It asks
//! once before anything is sent; `-y` skips the question.
//!
//! Anything else `find` takes — another test, an action, an operator — is
//! refused by name before anything is asked of the daemon: a test that
//! was silently dropped would find more than was asked for.

use std::ffi::OsString;
use std::io::Write as _;
use std::process::ExitCode;

use crate::cli::FindArgs;
use blit_core::admin::find::{
    self, AgeTest, Compare, FindEvent, FindQuery, Kind, NameTest, SizeTest,
};
use blit_core::admin::operand::{self, Operand};
use blit_core::find_name::NamePattern;
use eyre::{bail, Result};

/// What an expression asks, read from its words.
#[derive(Debug, Default, PartialEq, Eq)]
struct Expression {
    names: Vec<NameTest>,
    kinds: Vec<Kind>,
    max_depth: Option<u32>,
    sizes: Vec<SizeTest>,
    ages: Vec<AgeTest>,
    /// `-delete`, the last word.
    delete: bool,
    /// `-y`: the question `-delete` asks is skipped.
    yes: bool,
    json: bool,
}

/// The tests `blit find` takes, for every refusal.
const TAKES: &str = "blit find takes -name PATTERN, -iname PATTERN, -type f|d|l, -maxdepth N, \
     -size [+|-]N[c|k|M|G], -mtime [+|-]N and, last, -delete";

/// The sign before a number: `+N` is more than N, `-N` less, `N` exactly.
fn signed(typed: &str) -> (Compare, &str) {
    match typed.strip_prefix('+') {
        Some(rest) => (Compare::More, rest),
        None => match typed.strip_prefix('-') {
            Some(rest) => (Compare::Less, rest),
            None => (Compare::Exactly, typed),
        },
    }
}

/// `-size [+|-]N[c|k|M|G]`: N units, of 512 bytes where no letter says
/// otherwise — `find`'s own default, and its letters for bytes and for
/// kibi-, mebi- and gibibytes.
fn size_test(typed: &str) -> Result<SizeTest> {
    let (compare, rest) = signed(typed);
    let digits = rest.trim_end_matches(|letter: char| !letter.is_ascii_digit());
    let unit_bytes = match &rest[digits.len()..] {
        "" => 512,
        "c" => 1,
        "k" => 1024,
        "M" => 1024 * 1024,
        "G" => 1024 * 1024 * 1024,
        other => bail!(
            "-size {typed}: `{other}' is not a unit blit find takes: write N, Nc, Nk, NM or NG"
        ),
    };
    let count = digits.parse().map_err(|_| {
        eyre::eyre!("-size {typed}: expected [+|-]N, with c, k, M or G after it for a unit")
    })?;
    Ok(SizeTest {
        compare,
        count,
        unit_bytes,
    })
}

/// `-mtime [+|-]N`: N whole days.
fn age_test(typed: &str) -> Result<AgeTest> {
    let (compare, digits) = signed(typed);
    let days = digits
        .parse()
        .map_err(|_| eyre::eyre!("-mtime {typed}: expected [+|-]N, a whole number of days"))?;
    Ok(AgeTest { compare, days })
}

/// Read the words after the path. Every word is a test this command
/// takes, with its argument, or the command is refused: by the word's
/// name, with what replaces it when it is one of the options `blit find`
/// had before 1.0.
fn parse(words: &[OsString]) -> Result<Expression> {
    let mut expression = Expression::default();
    let mut words = words.iter();
    while let Some(word) = words.next() {
        // A test's own name is text; its argument need not be (cr-uv1-4:
        // a name `find` prints by its bytes can be a `-name` pattern).
        let word = word.to_string_lossy();
        let word = word.as_ref();
        let mut argument = |what: &str| -> Result<&OsString> {
            match words.next() {
                Some(argument) => Ok(argument),
                None => bail!("missing argument to `{word}': {what}"),
            }
        };
        // `-delete` is the last word, or it is refused: in `find` a test
        // written after it does not limit it — `find d -delete -name
        // '*.tmp'` removes all of `d`.
        if expression.delete && !matches!(word, "-y" | "--yes" | "--json") {
            bail!(
                "-delete must be the last word: `{word}' after it would not limit what is \
                 deleted (in find it does not either)"
            );
        }
        match word {
            "-name" | "-iname" => {
                let typed = argument("a pattern")?;
                let pattern = operand::word_bytes(typed)?;
                let ignore_case = word == "-iname";
                // Read here as the daemon will read it: a pattern it would
                // refuse is refused before anything is asked.
                NamePattern::new(pattern, ignore_case).map_err(|refusal| {
                    eyre::eyre!("{word} '{}': {refusal}", typed.to_string_lossy())
                })?;
                expression.names.push(NameTest {
                    pattern: pattern.to_vec(),
                    ignore_case,
                });
            }
            "-type" => {
                let kind = argument("f, d or l")?.to_string_lossy();
                expression.kinds.push(match kind.as_ref() {
                    "f" => Kind::File,
                    "d" => Kind::Directory,
                    "l" => Kind::Link,
                    other => bail!("-type {other} is not supported: -type takes f, d or l"),
                })
            }
            "-maxdepth" => {
                let depth = argument("a number of levels")?.to_string_lossy();
                expression.max_depth = Some(depth.parse().map_err(|_| {
                    eyre::eyre!("-maxdepth {depth}: expected a number of levels, 0 or more")
                })?);
            }
            "-size" => {
                let typed = argument("a size, as [+|-]N[c|k|M|G]")?.to_string_lossy();
                expression.sizes.push(size_test(&typed)?);
            }
            "-mtime" => {
                let typed = argument("a number of days, as [+|-]N")?.to_string_lossy();
                expression.ages.push(age_test(&typed)?);
            }
            "-delete" => expression.delete = true,
            "-y" | "--yes" => expression.yes = true,
            "--json" => expression.json = true,
            "--help" | "-help" => bail!("for help: blit find --help"),
            // The options this command had before it followed `find`.
            "--case-insensitive" => bail!("--case-insensitive is gone: write -iname PATTERN"),
            "--files" => bail!("--files is gone: write -type f"),
            "--dirs" => bail!("--dirs is gone: write -type d"),
            "--pattern" | "--limit" => {
                let value = words.next().map(|value| value.to_string_lossy());
                bail!("{}", gone(word, value.as_deref()))
            }
            other => match other.split_once('=') {
                Some((option @ ("--pattern" | "--limit"), value)) => {
                    bail!("{}", gone(option, Some(value)))
                }
                _ if other.starts_with('-') || matches!(other, "(" | ")" | "!" | ",") => {
                    bail!("{other} is not supported; {TAKES}")
                }
                _ => bail!("`{other}` is not a test, and blit find takes one path; {TAKES}"),
            },
        }
    }
    Ok(expression)
}

/// What replaces `--pattern` and `--limit`.
fn gone(option: &str, value: Option<&str>) -> String {
    match option {
        "--pattern" => format!(
            "--pattern is gone: write -name '{}' (the pattern is matched against each entry's \
             own name, as find matches it)",
            value.unwrap_or("PATTERN")
        ),
        _ => format!(
            "--limit is gone: find has no such option; pipe the output through `head -n {}`",
            value.unwrap_or("N")
        ),
    }
}

pub async fn run_find(args: FindArgs) -> Result<ExitCode> {
    // The whole expression is read before anything is asked of a daemon.
    let expression = parse(&args.expression)?;
    // The operand as typed (cr-uv1-1, cr-uv1-4).
    let Some(operand) = Operand::read(&args.target)? else {
        bail!(
            "`blit find` requires a remote path (received local path: {})",
            args.target.to_string_lossy()
        );
    };
    let remote = operand.remote.clone();
    // Slice 4 (AC-5, AC-10): `-delete` asks once, before anything is sent
    // — as `rm`, `mirror` and `move` ask. "No" stops it; nobody to answer
    // stops it with exit 1 and the option that skips the question.
    if expression.delete
        && !crate::confirm::confirm(
            None,
            &format!(
                "Delete every entry this finds under {}?",
                blit_core::admin::quote::for_shell(operand.typed(), true)
            ),
            args.yes || expression.yes,
            "-y",
        )?
    {
        println!("Aborted.");
        return Ok(ExitCode::SUCCESS);
    }
    let query = FindQuery {
        module: operand.module.clone(),
        start_path: operand.wire_path(),
        // `find dir/` names the directory a link called `dir` points to.
        follow_start: operand.path().ends_with(b"/"),
        names: expression.names,
        kinds: expression.kinds,
        max_depth: expression.max_depth,
        sizes: expression.sizes,
        ages: expression.ages,
        delete: expression.delete,
    };
    // As `find` prints an entry: every separator typed after the operand
    // kept (`dir//` prints `dir//sub`).
    let shown = |path: &blit_core::admin::du::EntryPath| operand.below_as_find(path);

    let mut failed = false;
    if args.json || expression.json {
        // cr-uv3-7: `paths` holds text, and a name need not be text: two
        // names came out as one string, twice. Each path that is not text
        // is listed in `raw_paths` by its place in `paths`, with its bytes.
        let (mut paths, mut raw_paths, mut failures) = (Vec::new(), Vec::new(), Vec::new());
        find::stream(&remote, query, |event| {
            match event {
                FindEvent::Found(path) => {
                    let printed = shown(&path);
                    if let Some(raw) = blit_core::admin::quote::exact(&printed) {
                        raw_paths.push(serde_json::json!({ "index": paths.len(), "raw": raw }));
                    }
                    paths.push(serde_json::Value::String(
                        String::from_utf8_lossy(&printed).into_owned(),
                    ));
                }
                FindEvent::Failure {
                    path,
                    reason,
                    deleting,
                } => {
                    failures.push(blit_core::admin::quote::in_json(
                        &shown(&path),
                        serde_json::json!({ "reason": reason, "deleting": deleting }),
                    ));
                }
            }
            Ok(())
        })
        .await?;
        failed = !failures.is_empty();
        println!(
            "{}",
            serde_json::to_string_pretty(&serde_json::json!({
                "paths": paths,
                "raw_paths": raw_paths,
                "failures": failures,
            }))?
        );
    } else {
        let mut out = std::io::BufWriter::new(std::io::stdout().lock());
        // Whoever was reading has gone (`blit find … | head`): there is
        // nobody to print for, and nothing to report.
        let mut reader_gone = false;
        let streamed = find::stream(&remote, query, |event| {
            let written = match event {
                FindEvent::Found(path) => {
                    // The path's own bytes: a name that is not valid text
                    // is printed as the name it is.
                    let mut line = shown(&path);
                    line.push(b'\n');
                    out.write_all(&line)
                }
                FindEvent::Failure {
                    path,
                    reason,
                    deleting,
                } => {
                    failed = true;
                    // What was found before it is printed before it.
                    let flushed = out.flush();
                    // The name as `find` writes it (cr-uv2-5), and what
                    // could not be done with it, in `find`'s words.
                    eprintln!(
                        "blit find: {}{}: {reason}",
                        if deleting { "cannot delete " } else { "" },
                        blit_core::admin::quote::for_find(&shown(&path))
                    );
                    flushed
                }
            };
            match written {
                Err(error) if error.kind() == std::io::ErrorKind::BrokenPipe => {
                    reader_gone = true;
                    Err(eyre::eyre!("the reader has gone"))
                }
                other => Ok(other?),
            }
        })
        .await;
        if reader_gone {
            return Ok(ExitCode::from(141));
        }
        streamed?;
        match out.flush() {
            Err(error) if error.kind() == std::io::ErrorKind::BrokenPipe => {
                return Ok(ExitCode::from(141))
            }
            flushed => flushed?,
        }
    }

    Ok(if failed {
        ExitCode::from(1)
    } else {
        ExitCode::SUCCESS
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn words(typed: &[&str]) -> Vec<OsString> {
        typed.iter().map(OsString::from).collect()
    }

    /// cr-uv1-4: a pattern need not be text — it is taken by its bytes,
    /// as the name it is to match is printed by its bytes. The tests' own
    /// names are text, and a word that is none is no test.
    #[cfg(unix)]
    #[test]
    fn a_pattern_that_is_not_text_is_taken_by_its_bytes() {
        use std::os::unix::ffi::OsStringExt as _;
        let pattern = OsString::from_vec(b"bad\xff*.dat".to_vec());
        let parsed = parse(&[OsString::from("-name"), pattern.clone()]).unwrap();
        assert_eq!(
            parsed.names,
            [NameTest {
                pattern: b"bad\xff*.dat".to_vec(),
                ignore_case: false,
            }]
        );
        let refused = parse(&[pattern]).expect_err("no test").to_string();
        assert!(refused.contains("is not a test"), "{refused}");
    }

    fn name(pattern: &str, ignore_case: bool) -> NameTest {
        NameTest {
            pattern: pattern.as_bytes().to_vec(),
            ignore_case,
        }
    }

    /// AC-4: the four tests, in any order and any number, each with its
    /// argument.
    #[test]
    fn an_expression_is_its_tests() {
        assert_eq!(parse(&[]).unwrap(), Expression::default());
        assert_eq!(
            parse(&words(&[
                "-maxdepth",
                "2",
                "-name",
                "*.dat",
                "-type",
                "f",
                "-iname",
                "A*",
                "-type",
                "l",
                "--json"
            ]))
            .unwrap(),
            Expression {
                names: vec![name("*.dat", false), name("A*", true)],
                kinds: vec![Kind::File, Kind::Link],
                max_depth: Some(2),
                json: true,
                ..Expression::default()
            }
        );
        assert_eq!(
            parse(&words(&["-type", "d", "-maxdepth", "0"])).unwrap(),
            Expression {
                kinds: vec![Kind::Directory],
                max_depth: Some(0),
                ..Expression::default()
            }
        );
    }

    /// AC-4, AC-8: every other word of `find` is refused by name — a test,
    /// an action, an operator — and so is a test given without its
    /// argument, or with one it does not take.
    #[test]
    fn what_find_takes_and_blit_find_does_not_is_refused_by_name() {
        for word in [
            "-newer",
            "-exec",
            "-print",
            "-print0",
            "-o",
            "-or",
            "-a",
            "-and",
            "-not",
            "!",
            "(",
            ")",
            ",",
            "-path",
            "-regex",
            "-empty",
            "-mindepth",
            "-L",
            "-xdev",
            "-prune",
            "-ls",
            "--frobnicate",
        ] {
            let refused = parse(&words(&["-name", "x", word, "y"])).expect_err(word);
            let said = refused.to_string();
            assert!(
                said.contains(&format!("{word} is not supported")) && said.contains(TAKES),
                "{word}: {said}"
            );
        }
        for (typed, said) in [
            (&["-name"][..], "missing argument to `-name'"),
            (&["-type"], "missing argument to `-type'"),
            (&["-maxdepth"], "missing argument to `-maxdepth'"),
            (&["-type", "b"], "-type b is not supported"),
            (&["-type", "f,d"], "-type f,d is not supported"),
            (&["-maxdepth", "-1"], "-maxdepth -1"),
            (&["-maxdepth", "two"], "-maxdepth two"),
            (
                &["-name", "[[=a=]]"],
                "-name '[[=a=]]': an equivalence class",
            ),
            (&["stray"], "`stray` is not a test"),
            (&["-help"], "blit find --help"),
        ] {
            let refused = parse(&words(typed)).expect_err(said).to_string();
            assert!(refused.contains(said), "{typed:?}: {refused}");
        }
    }

    /// AC-4: the options this command had before 1.0 are refused with the
    /// spelling that replaces each.
    #[test]
    fn the_old_options_are_refused_with_what_replaces_them() {
        for (typed, said) in [
            (
                &["--pattern", "*.csv"][..],
                "--pattern is gone: write -name '*.csv'",
            ),
            (
                &["--pattern=*.csv"],
                "--pattern is gone: write -name '*.csv'",
            ),
            (
                &["--case-insensitive"],
                "--case-insensitive is gone: write -iname PATTERN",
            ),
            (&["--files"], "--files is gone: write -type f"),
            (&["--dirs"], "--dirs is gone: write -type d"),
            (
                &["--limit", "10"],
                "--limit is gone: find has no such option; pipe the output through `head -n 10`",
            ),
            (&["--limit=10"], "`head -n 10`"),
        ] {
            let refused = parse(&words(typed)).expect_err(said).to_string();
            assert!(refused.contains(said), "{typed:?}: {refused}");
        }
    }

    /// Slice 4 (AC-5): `-size` and `-mtime` are read as `find` writes
    /// them, and anything else is refused by what was typed.
    #[test]
    fn size_and_age_are_read_as_find_writes_them() {
        let parsed = parse(&words(&[
            "-size", "+1k", "-size", "-2M", "-size", "3", "-size", "0c", "-size", "1G", "-mtime",
            "-1", "-mtime", "+0", "-mtime", "7",
        ]))
        .unwrap();
        let size = |compare, count, unit_bytes| SizeTest {
            compare,
            count,
            unit_bytes,
        };
        assert_eq!(
            parsed.sizes,
            [
                size(Compare::More, 1, 1024),
                size(Compare::Less, 2, 1024 * 1024),
                size(Compare::Exactly, 3, 512),
                size(Compare::Exactly, 0, 1),
                size(Compare::Exactly, 1, 1024 * 1024 * 1024),
            ]
        );
        let age = |compare, days| AgeTest { compare, days };
        assert_eq!(
            parsed.ages,
            [
                age(Compare::Less, 1),
                age(Compare::More, 0),
                age(Compare::Exactly, 7)
            ]
        );
        for (typed, says) in [
            (&["-size", "1b"][..], "`b' is not a unit blit find takes"),
            (&["-size", "1w"], "`w' is not a unit blit find takes"),
            (&["-size", "1K"], "`K' is not a unit blit find takes"),
            (&["-size", "x"], "`x' is not a unit blit find takes"),
            (&["-size", "1.5k"], "-size 1.5k: expected [+|-]N"),
            (&["-size", "+-1k"], "-size +-1k: expected [+|-]N"),
            (&["-size", "k"], "-size k: expected [+|-]N"),
            (&["-size", ""], "-size : expected [+|-]N"),
            (&["-size"], "missing argument to `-size'"),
            (&["-mtime", "x"], "-mtime x: expected [+|-]N"),
            (&["-mtime", "1.5"], "-mtime 1.5: expected [+|-]N"),
            (&["-mtime", "+-1"], "-mtime +-1: expected [+|-]N"),
            (&["-mtime"], "missing argument to `-mtime'"),
        ] {
            let refused = parse(&words(typed))
                .expect_err(&typed.join(" "))
                .to_string();
            assert!(refused.contains(says), "{typed:?}: {refused}");
        }
    }

    /// Slice 4 (AC-5): `-delete` is the last word, or the command is
    /// refused — in `find` a test after it does not limit it. `-y` and
    /// `--json` are no tests, and may stand anywhere.
    #[test]
    fn delete_is_taken_as_the_last_word_only() {
        for typed in [
            &["-name", "*.tmp", "-delete"][..],
            &["-delete"],
            &["-name", "*.tmp", "-delete", "-y"],
            &["-y", "-name", "*.tmp", "-delete", "--json"],
        ] {
            let parsed =
                parse(&words(typed)).unwrap_or_else(|refused| panic!("{typed:?}: {refused}"));
            assert!(parsed.delete, "{typed:?}");
            assert_eq!(parsed.yes, typed.contains(&"-y"), "{typed:?}");
        }
        assert!(!parse(&words(&["-name", "*.tmp"])).unwrap().delete);
        for (typed, after) in [
            (&["-delete", "-name", "*.tmp"][..], "-name"),
            (&["-delete", "-type", "f"], "-type"),
            (&["-name", "a", "-delete", "-maxdepth", "1"], "-maxdepth"),
            (&["-delete", "-delete"], "-delete"),
            (&["-delete", "-size", "+1k"], "-size"),
        ] {
            let refused = parse(&words(typed))
                .expect_err(&typed.join(" "))
                .to_string();
            assert!(
                refused.contains("-delete must be the last word")
                    && refused.contains(&format!("`{after}'")),
                "{typed:?}: {refused}"
            );
        }
    }
}
