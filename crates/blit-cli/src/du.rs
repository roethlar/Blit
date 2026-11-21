//! `blit du` — `du`, run on a daemon's own disk
//! (`docs/plan/UTILITY_VERBS_LOCAL_PARITY.md` slice 1).
//!
//! One line per directory, an entry after everything below it and the path
//! given last, each `SIZE<TAB>PATH` with the path beginning with the
//! operand as typed. Sizes are allocated space in 1K blocks. An entry the
//! daemon could not read is reported on stderr in `du`'s words and the
//! exit status is 1, the rest of the answer printed all the same.

use std::io::Write as _;
use std::process::ExitCode;

use crate::cli::DuArgs;
use blit_core::admin::du::{self, DuEvent, UsageOptions};
use blit_core::admin::operand::Operand;
use blit_core::disk_usage::{blocks_1k, human_size};
use eyre::{bail, Result};

pub async fn run_du(args: DuArgs) -> Result<ExitCode> {
    // The operand as typed (cr-uv1-1, cr-uv1-4): `du link/` is the
    // directory the link points to, said by the separator; and a path
    // need not be text.
    let Some(operand) = Operand::read(&args.target)? else {
        bail!(
            "`blit du` requires a remote path (received local path: {})",
            args.target.to_string_lossy()
        );
    };
    let options = usage_options(&args)?;
    let (remote, module, start_path) = (
        operand.remote.clone(),
        operand.module.clone(),
        operand.wire_path(),
    );
    let size = |bytes: u64| {
        if args.human_readable {
            human_size(bytes)
        } else {
            blocks_1k(bytes).to_string()
        }
    };

    let mut failed = false;
    if args.json {
        let mut rows = Vec::new();
        let mut failures = Vec::new();
        du::stream(&remote, module, start_path, options, |event| {
            match event {
                DuEvent::Row {
                    path,
                    allocated_bytes,
                } => rows.push(serde_json::json!({
                    "path": String::from_utf8_lossy(&operand.below(&path)),
                    "allocated_bytes": allocated_bytes,
                })),
                DuEvent::Failure {
                    path,
                    action,
                    reason,
                } => failures.push(serde_json::json!({
                    "path": String::from_utf8_lossy(&operand.below(&path)),
                    "action": action,
                    "reason": reason,
                })),
            }
            Ok(())
        })
        .await?;
        failed = !failures.is_empty();
        println!(
            "{}",
            serde_json::to_string_pretty(
                &serde_json::json!({ "rows": rows, "failures": failures })
            )?
        );
    } else {
        du::stream(&remote, module, start_path, options, |event| {
            match event {
                DuEvent::Row {
                    path,
                    allocated_bytes,
                } => {
                    // The path's own bytes: a name that is not valid text
                    // is printed as the name it is.
                    let mut line = size(allocated_bytes).into_bytes();
                    line.push(b'\t');
                    line.extend_from_slice(&operand.below(&path));
                    line.push(b'\n');
                    std::io::stdout().lock().write_all(&line)?;
                }
                DuEvent::Failure {
                    path,
                    action,
                    reason,
                } => {
                    failed = true;
                    eprintln!(
                        "blit du: {action} '{}': {reason}",
                        String::from_utf8_lossy(&operand.below(&path))
                    );
                }
            }
            Ok(())
        })
        .await?;
    }

    Ok(if failed {
        ExitCode::from(1)
    } else {
        ExitCode::SUCCESS
    })
}

/// What `du` is asked, from the options — and `du`'s own two refusals.
fn usage_options(args: &DuArgs) -> Result<UsageOptions> {
    if args.summarize && args.all {
        bail!("cannot both summarize and show all entries");
    }
    match args.max_depth {
        Some(depth) if args.summarize && depth != 0 => {
            bail!("summarizing conflicts with --max-depth={depth}")
        }
        _ => {}
    }
    Ok(UsageOptions {
        all_files: args.all,
        max_depth: if args.summarize {
            Some(0)
        } else {
            args.max_depth
        },
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::Parser;

    fn parsed(args: &[&str]) -> std::result::Result<DuArgs, clap::Error> {
        let mut argv = vec!["blit", "du"];
        argv.extend(args);
        match crate::cli::Cli::try_parse_from(argv)?.command {
            crate::cli::Commands::Du(args) => Ok(args),
            _ => unreachable!("du parses as du"),
        }
    }

    /// AC-2: the options are `du`'s — `-a`, `-s`, `-d N` and
    /// `--max-depth=N`, `-h` — alone and bundled, each mapped to what is
    /// asked of the daemon; `-s` is a depth of 0.
    #[test]
    fn the_options_are_the_unix_commands() {
        let options = |args: &[&str]| usage_options(&parsed(args).unwrap()).unwrap();
        let every = UsageOptions::default();
        assert_eq!(options(&["h:/m/"]), every);
        assert_eq!(
            options(&["-a", "h:/m/"]),
            UsageOptions {
                all_files: true,
                ..every
            }
        );
        for depth in [
            &["-d", "2"][..],
            &["-d2"],
            &["--max-depth=2"],
            &["--max-depth", "2"],
        ] {
            let mut args = depth.to_vec();
            args.push("h:/m/");
            assert_eq!(options(&args).max_depth, Some(2), "{depth:?}");
        }
        for summary in [&["-s"][..], &["--summarize"], &["-sh"], &["-s", "-d", "0"]] {
            let mut args = summary.to_vec();
            args.push("h:/m/");
            assert_eq!(options(&args).max_depth, Some(0), "{summary:?}");
        }
        assert!(parsed(&["-ah", "h:/m/"]).unwrap().human_readable);
        assert!(
            parsed(&["--human-readable", "h:/m/"])
                .unwrap()
                .human_readable
        );
    }

    /// AC-8 and `du`'s own two refusals: an option `du` does not have is
    /// refused by name by the parser; `-s` with `-a`, and `-s` with a
    /// depth other than 0, are refused in `du`'s words. All before any
    /// request (none of this reaches a daemon).
    #[test]
    fn an_option_du_does_not_take_is_refused_by_name() {
        for unknown in ["--apparent-size", "-x", "--files", "-b"] {
            let error = parsed(&[unknown, "h:/m/"]).expect_err(unknown).to_string();
            assert!(error.contains(unknown), "{error}");
        }
        let refused = |args: &[&str]| {
            usage_options(&parsed(args).unwrap())
                .expect_err("refused")
                .to_string()
        };
        assert_eq!(
            refused(&["-s", "-a", "h:/m/"]),
            "cannot both summarize and show all entries"
        );
        assert_eq!(
            refused(&["-s", "-d", "1", "h:/m/"]),
            "summarizing conflicts with --max-depth=1"
        );
    }
}
