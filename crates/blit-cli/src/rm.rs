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
use blit_core::admin::rm::{self, RmOptions};
use blit_core::endpoints::{parse_endpoint_or_local, Endpoint};
use blit_core::remote::RemoteEndpoint;
use eyre::{bail, Result};
use std::path::Path;
use std::process::ExitCode;

// Re-export the helper used by `crate::transfers::mod::run_move`
// so existing call sites stay working without an import update.
pub use blit_core::admin::rm::delete_remote_path;

/// One operand: as typed, and where it is.
struct Operand {
    typed: String,
    remote: RemoteEndpoint,
    module: String,
    /// Its path below the module, `/`-separated.
    rel: String,
}

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
        let rel = rel_path
            .iter()
            .map(|component| component.to_string_lossy().into_owned())
            .collect::<Vec<_>>()
            .join("/");
        if rel.is_empty() || rel_path == Path::new(".") {
            bail!(
                "refusing to delete entire module '{}'; specify a sub-path",
                module
            );
        }
        operands.push(Operand {
            typed: typed.clone(),
            remote,
            module,
            rel,
        });
    }

    // The question, once, for the whole command (Q1, D-2026-11-20-3).
    if !crate::confirm::confirm(
        None,
        &question(&operands),
        args.yes || args.force,
        "-y (or -f)",
    )? {
        println!("Aborted.");
        return Ok(ExitCode::SUCCESS);
    }

    // One request per daemon and module, in the order first named.
    let mut groups: Vec<Vec<usize>> = Vec::new();
    for (at, operand) in operands.iter().enumerate() {
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
    let mut failures: Vec<(String, String)> = Vec::new();
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
                    failures.push((String::from_utf8_lossy(&shown).into_owned(), failure.reason));
                }
            }
            // The daemon refused the request whole (a read-only module,
            // another build, no answer): none of these operands went.
            Err(error) => {
                for &at in &group {
                    failures.push((operands[at].typed.clone(), format!("{error:#}")));
                }
            }
        }
    }

    if args.json {
        let listed: Vec<serde_json::Value> = failures
            .iter()
            .map(|(path, reason)| serde_json::json!({ "path": path, "reason": reason }))
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
        for (path, reason) in &failures {
            eprintln!("blit rm: cannot remove '{path}': {reason}");
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
fn question(operands: &[Operand]) -> String {
    const NAMED: usize = 3;
    let mut named: Vec<String> = operands
        .iter()
        .take(NAMED)
        .map(|operand| format!("'{}'", operand.typed))
        .collect();
    if operands.len() > NAMED {
        named.push(format!("{} more", operands.len() - NAMED));
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
        assert_eq!(question(&operands(&["srv:/m/a"])), "Remove 'srv:/m/a'?");
        assert_eq!(
            question(&operands(&["srv:/m/a", "srv:/m/b"])),
            "Remove 'srv:/m/a' and 'srv:/m/b'?"
        );
        assert_eq!(
            question(&operands(&["srv:/m/a", "srv:/m/b", "srv:/m/c"])),
            "Remove 'srv:/m/a', 'srv:/m/b' and 'srv:/m/c'?"
        );
        assert_eq!(
            question(&operands(&[
                "srv:/m/a", "srv:/m/b", "srv:/m/c", "srv:/m/d", "srv:/m/e"
            ])),
            "Remove 'srv:/m/a', 'srv:/m/b', 'srv:/m/c' and 2 more?"
        );
    }
}
