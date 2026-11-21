//! `blit rm` — `rm`, run on a daemon's own disk
//! (`docs/plan/UTILITY_VERBS_LOCAL_PARITY.md` slice 2).
//!
//! `blit rm [-r] [-f] TARGET...`, each operand treated as the Unix command
//! treats it: a directory needs `-r`, an operand that is not there is an
//! error unless `-f`, the other operands are processed all the same, and
//! the exit status is 0 only when every operand is gone. What it prints
//! is `rm`'s: nothing on success, `cannot remove 'OPERAND': REASON` for
//! each entry that could not be removed — every one, as the daemon
//! reports it.
//!
//! The one place it follows blit and not `rm`: it asks once before
//! anything is sent, as `mirror` and `move` do, unless `-y` — or `-f`,
//! which means "never ask" in `rm` itself, so `blit rm -rf` behaves as
//! `rm -rf` does.

use crate::cli::RmArgs;
use blit_core::admin::operand::{self, Operand};
use blit_core::admin::quote;
use blit_core::admin::rm::{self, RmError, RmOptions};
use eyre::{bail, Result};
use std::process::ExitCode;

// Re-export the helper used by `crate::transfers::mod::run_move`
// so existing call sites stay working without an import update.
pub use blit_core::admin::rm::delete_remote_path;

/// One entry that was not removed.
struct Failed {
    /// The operand it belongs to, by its place on the command line.
    operand: usize,
    /// The entry as these commands print it: by its bytes (cr-uv2-5).
    path: Vec<u8>,
    reason: String,
    /// `rm` itself refuses the operand: a path ending in `.` or `..`.
    refused: bool,
}

/// `rm`'s words for an operand whose last name is `.` or `..`.
const DOT_OR_DOT_DOT: &str = "refusing to remove '.' or '..' directory";

pub async fn run_rm(args: RmArgs) -> Result<ExitCode> {
    // Every operand is read before anything is asked or sent: one that
    // cannot be an operand at all refuses the command whole. Each is kept
    // as typed (cr-uv1-1) and by its bytes (cr-uv1-4): a separator after
    // it, a `.`, a space at its end, a name that is not text.
    let mut operands = Vec::with_capacity(args.targets.len());
    for typed in &args.targets {
        let Some(read) = Operand::read(typed)? else {
            bail!(
                "`blit rm` only supports remote paths (received local path: {})",
                typed.to_string_lossy()
            );
        };
        if operand::is_the_root(read.path()) {
            bail!(
                "refusing to delete entire module '{}'; specify a sub-path",
                read.module
            );
        }
        operands.push(read);
    }

    // `rm` refuses a path whose last name is `.` or `..` whatever its
    // options, and goes on to the others (cr-uv1-1: `work/.` was sent as
    // `work`, and `work` removed). Such an operand is neither asked about
    // nor sent.
    let mut refused: Vec<Failed> = Vec::new();
    let mut asked: Vec<usize> = Vec::new();
    for (at, each) in operands.iter().enumerate() {
        if operand::names_dot(each.path()) {
            refused.push(Failed {
                operand: at,
                path: each.typed().to_vec(),
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
    // What is said, as it is learned (cr-uv1-5): each entry the daemon
    // could not remove is printed when the daemon reports it, every one of
    // them, in the order the operands were typed.
    let mut said = Said {
        json: args.json,
        listed: Vec::new(),
        refused: refused.into(),
        any: false,
    };
    let mut entries_removed = 0u64;
    let mut unfinished: Vec<String> = Vec::new();
    for group in groups {
        let first = &operands[group[0]];
        let paths = group.iter().map(|&at| operands[at].wire_path()).collect();
        let answered = rm::remove(
            &first.remote,
            first.module.clone(),
            paths,
            options,
            |failure| {
                let Some(&at) = group.get(failure.operand) else {
                    return;
                };
                said.failed(Failed {
                    operand: at,
                    path: operands[at].below(&failure.path),
                    reason: failure.reason,
                    refused: false,
                });
            },
        )
        .await;
        match answered {
            Ok(removed) => entries_removed += removed,
            // The daemon refused the request whole (a read-only module,
            // another build, no answer): none of these operands went.
            Err(RmError::Refused(error)) => {
                for &at in &group {
                    said.failed(Failed {
                        operand: at,
                        path: operands[at].typed().to_vec(),
                        reason: format!("{error:#}"),
                        refused: false,
                    });
                }
            }
            // The answer stopped part way: what was reported stands, and
            // what else went is not known.
            Err(error @ RmError::Interrupted(_)) => {
                unfinished.push(format!("{}: {error}", first.remote.host_port_display()));
            }
        }
    }
    said.finish();

    if args.json {
        println!(
            "{}",
            serde_json::to_string_pretty(&serde_json::json!({
                "entries_removed": entries_removed,
                "failures": said.listed,
                "unfinished": unfinished,
            }))?
        );
    } else {
        for line in &unfinished {
            eprintln!("blit rm: {line}");
        }
    }
    Ok(if said.any || !unfinished.is_empty() {
        ExitCode::from(1)
    } else {
        ExitCode::SUCCESS
    })
}

/// What `rm` says of the entries it did not remove: printed as each is
/// learned, or collected for `--json`.
struct Said {
    json: bool,
    listed: Vec<serde_json::Value>,
    /// The operands `rm` itself refuses, waiting for their turn: each is
    /// said before anything about an operand typed after it.
    refused: std::collections::VecDeque<Failed>,
    any: bool,
}

impl Said {
    fn failed(&mut self, failed: Failed) {
        while self
            .refused
            .front()
            .is_some_and(|before| before.operand < failed.operand)
        {
            let before = self.refused.pop_front().expect("one is waiting");
            self.say(before);
        }
        self.say(failed);
    }

    /// The command is over: whatever still waits is said.
    fn finish(&mut self) {
        while let Some(waiting) = self.refused.pop_front() {
            self.say(waiting);
        }
    }

    /// A name is written as `rm` writes it (cr-uv2-5): quoted so that
    /// it reads back as the name it is — `'bad'$'\377''.dat'`. Shown as
    /// text, a name that is not text was told of in the words for
    /// another file, the one holding U+FFFD where this holds a byte.
    fn say(&mut self, failed: Failed) {
        self.any = true;
        if self.json {
            // cr-uv3-7: with its bytes, where it is not text.
            self.listed.push(quote::in_json(
                &failed.path,
                serde_json::json!({ "reason": failed.reason }),
            ));
            return;
        }
        let name = quote::for_shell(&failed.path, true);
        if failed.refused {
            eprintln!("blit rm: {}: skipping {name}", failed.reason);
        } else {
            eprintln!("blit rm: cannot remove {name}: {}", failed.reason);
        }
    }
}

/// What is asked before anything is removed: the operands, as typed.
fn question<'a>(operands: impl ExactSizeIterator<Item = &'a Operand>) -> String {
    const NAMED: usize = 3;
    let more = operands.len().saturating_sub(NAMED);
    let mut named: Vec<String> = operands
        .take(NAMED)
        .map(|operand| quote::for_shell(operand.typed(), true))
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
                Operand::read(std::ffi::OsStr::new(typed))
                    .expect(typed)
                    .expect("a remote path")
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
