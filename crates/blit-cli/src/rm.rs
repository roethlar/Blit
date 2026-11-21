//! `blit rm` — `rm`, run on a daemon's own disk
//! (`docs/plan/UTILITY_VERBS_LOCAL_PARITY.md` slice 2).
//!
//! `blit rm [-r] [-f] TARGET...`, each operand treated as the Unix command
//! treats it: a directory needs `-r`, an operand that is not there is an
//! error unless `-f`, the other operands are processed all the same, and
//! the exit status is 0 only when every operand is gone. What it prints
//! is `rm`'s: nothing on success, `cannot remove 'OPERAND': REASON` for
//! each entry that could not be removed.
//!
//! The one place it follows blit and not `rm`: it asks once before
//! anything is sent, as `mirror` and `move` do, unless `-y` — or `-f`,
//! which means "never ask" in `rm` itself, so `blit rm -rf` behaves as
//! `rm -rf` does.

use crate::cli::RmArgs;
use blit_core::admin::operand;
use blit_core::admin::rm::{self, RmOptions};
use blit_core::endpoints::{parse_endpoint_or_local, Endpoint};
use blit_core::remote::RemoteEndpoint;
use eyre::{bail, Result};
use std::process::ExitCode;

// Re-export the helper used by `crate::transfers::mod::run_move`
// so existing call sites stay working without an import update.
pub use blit_core::admin::rm::delete_remote_path;

/// One operand: as typed, and where it is.
struct Operand {
    typed: String,
    remote: RemoteEndpoint,
    module: String,
    /// Its path below the module, exactly as typed (cr-uv1-1): what is
    /// sent.
    rel: String,
}

/// One entry that was not removed.
struct Failed {
    /// The operand it belongs to, by its place on the command line.
    operand: usize,
    path: String,
    reason: String,
    /// `rm` itself refuses the operand: a path ending in `.` or `..`.
    refused: bool,
}

/// `rm`'s words for an operand whose last name is `.` or `..`.
const DOT_OR_DOT_DOT: &str = "refusing to remove '.' or '..' directory";

pub async fn run_rm(args: RmArgs) -> Result<ExitCode> {
    // Every operand is read before anything is asked or sent: one that
    // cannot be an operand at all refuses the command whole.
    let mut operands = Vec::with_capacity(args.targets.len());
    for typed in &args.targets {
        let remote = match parse_endpoint_or_local(typed) {
            Endpoint::Local(path) => {
                bail!(
                    "`blit rm` only supports remote paths (received local path: {})",
                    path.display()
                );
            }
            Endpoint::Remote(remote) => remote,
        };
        let (module, rel_path) = rm::extract_module_and_path(&remote)?;
        // As typed: the endpoint was read from text, and its path is kept
        // as that text — a separator after it, a `.`, a space at its end.
        let Some(rel) = rel_path.to_str() else {
            bail!("remote path is not valid text: {}", rel_path.display());
        };
        if operand::is_the_root(rel) {
            bail!(
                "refusing to delete entire module '{}'; specify a sub-path",
                module
            );
        }
        operands.push(Operand {
            typed: typed.clone(),
            remote,
            module,
            rel: rel.to_string(),
        });
    }

    // `rm` refuses a path whose last name is `.` or `..` whatever its
    // options, and goes on to the others (cr-uv1-1: `work/.` was sent as
    // `work`, and `work` removed). Such an operand is neither asked about
    // nor sent.
    let mut failures: Vec<Failed> = Vec::new();
    let mut asked: Vec<usize> = Vec::new();
    for (at, each) in operands.iter().enumerate() {
        if operand::names_dot(&each.rel) {
            failures.push(Failed {
                operand: at,
                path: each.typed.clone(),
                reason: DOT_OR_DOT_DOT.to_string(),
                refused: true,
            });
        } else {
            asked.push(at);
        }
    }

    // The question, once, for the whole command (Q1, D-2026-11-20-3).
    if !asked.is_empty()
        && !crate::confirm::confirm(
            None,
            &question(asked.iter().map(|&at| &operands[at])),
            args.yes || args.force,
            "-y (or -f)",
        )?
    {
        println!("Aborted.");
        return Ok(ExitCode::SUCCESS);
    }

    // One request per daemon and module, in the order first named.
    let mut groups: Vec<Vec<usize>> = Vec::new();
    for &at in &asked {
        let operand = &operands[at];
        let same = |other: &Operand| {
            other.remote.host == operand.remote.host
                && other.remote.port == operand.remote.port
                && other.module == operand.module
        };
        match groups.iter_mut().find(|group| same(&operands[group[0]])) {
            Some(group) => group.push(at),
            None => groups.push(vec![at]),
        }
    }

    let options = RmOptions {
        recursive: args.recursive,
        force: args.force,
    };
    let mut entries_removed = 0u64;
    let mut truncated = false;
    for group in groups {
        let first = &operands[group[0]];
        let paths = group.iter().map(|&at| operands[at].rel.clone()).collect();
        match rm::remove(&first.remote, first.module.clone(), paths, options).await {
            Ok(report) => {
                entries_removed += report.entries_removed;
                truncated |= report.failures_truncated;
                for failure in report.failures {
                    let Some(&at) = group.get(failure.operand) else {
                        continue;
                    };
                    let shown = crate::du::shown(&operands[at].typed, &failure.path);
                    failures.push(Failed {
                        operand: at,
                        path: String::from_utf8_lossy(&shown).into_owned(),
                        reason: failure.reason,
                        refused: false,
                    });
                }
            }
            // The daemon refused the request whole (a read-only module,
            // another build, no answer): none of these operands went.
            Err(error) => {
                for &at in &group {
                    failures.push(Failed {
                        operand: at,
                        path: operands[at].typed.clone(),
                        reason: format!("{error:#}"),
                        refused: false,
                    });
                }
            }
        }
    }
    // In the order the operands were typed, as `rm` reports them.
    failures.sort_by_key(|failed| failed.operand);

    if args.json {
        let listed: Vec<serde_json::Value> = failures
            .iter()
            .map(|failed| serde_json::json!({ "path": failed.path, "reason": failed.reason }))
            .collect();
        println!(
            "{}",
            serde_json::to_string_pretty(&serde_json::json!({
                "entries_removed": entries_removed,
                "failures": listed,
                "failures_truncated": truncated,
            }))?
        );
    } else {
        for failed in &failures {
            if failed.refused {
                eprintln!("blit rm: {}: skipping '{}'", failed.reason, failed.path);
            } else {
                eprintln!(
                    "blit rm: cannot remove '{}': {}",
                    failed.path, failed.reason
                );
            }
        }
        if truncated {
            eprintln!("blit rm: more entries could not be removed than are listed");
        }
    }
    Ok(if failures.is_empty() && !truncated {
        ExitCode::SUCCESS
    } else {
        ExitCode::from(1)
    })
}

/// What is asked before anything is removed: the operands, as typed.
fn question<'a>(operands: impl ExactSizeIterator<Item = &'a Operand>) -> String {
    const NAMED: usize = 3;
    let more = operands.len().saturating_sub(NAMED);
    let mut named: Vec<String> = operands
        .take(NAMED)
        .map(|operand| format!("'{}'", operand.typed))
        .collect();
    if more > 0 {
        named.push(format!("{more} more"));
    }
    let list = match named.len() {
        1 => named.remove(0),
        _ => {
            let last = named.pop().expect("more than one");
            format!("{} and {last}", named.join(", "))
        }
    };
    format!("Remove {list}?")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn operands(typed: &[&str]) -> Vec<Operand> {
        typed
            .iter()
            .map(|typed| {
                let Endpoint::Remote(remote) = parse_endpoint_or_local(typed) else {
                    panic!("{typed} is a remote path");
                };
                Operand {
                    typed: typed.to_string(),
                    remote,
                    module: "m".into(),
                    rel: "x".into(),
                }
            })
            .collect()
    }

    /// The question names what will be removed, as typed: every operand
    /// up to three, then a count.
    #[test]
    fn the_question_names_the_operands() {
        let asked = |typed: &[&str]| question(operands(typed).iter());
        assert_eq!(asked(&["srv:/m/a"]), "Remove 'srv:/m/a'?");
        assert_eq!(
            asked(&["srv:/m/a", "srv:/m/b"]),
            "Remove 'srv:/m/a' and 'srv:/m/b'?"
        );
        assert_eq!(
            asked(&["srv:/m/a", "srv:/m/b", "srv:/m/c"]),
            "Remove 'srv:/m/a', 'srv:/m/b' and 'srv:/m/c'?"
        );
        assert_eq!(
            asked(&["srv:/m/a", "srv:/m/b", "srv:/m/c", "srv:/m/d", "srv:/m/e"]),
            "Remove 'srv:/m/a', 'srv:/m/b', 'srv:/m/c' and 2 more?"
        );
        // As typed, to the letter: a separator after it, a space.
        assert_eq!(
            asked(&["srv:/m/a/", "srv:/m/b "]),
            "Remove 'srv:/m/a/' and 'srv:/m/b '?"
        );
    }
}
