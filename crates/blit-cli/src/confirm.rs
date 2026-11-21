//! The question a command asks before it deletes.
//!
//! One rule for every verb that asks — `mirror`, `move`, `rm`
//! (D-2026-11-20-3, D-2026-11-20-4; `docs/plan/UTILITY_VERBS_LOCAL_PARITY.md`
//! AC-10): "yes" goes ahead; anything else typed is "no", which stops the
//! command with nothing done and exit status 0; and when NO answer can be
//! read — standard input is closed, as under a scheduler or in a script
//! that forgot the option — the command stops with nothing done and exit
//! status 1, naming the option that skips the question. That last case
//! used to read as "no": the command printed "Aborted." and exited 0, and
//! an unattended job that deleted nothing looked like one that had run.

use eyre::{bail, Result};
use std::io::{self, Write};

/// Ask `message` and read the answer. `Ok(true)` to go ahead, `Ok(false)`
/// when the answer is no; an error when there is nobody to answer.
/// Always `Ok(true)` when `skip` (the operator passed the option that
/// waives the question). `note` is printed on its own line just before
/// the question; `how_to_skip` names that option for the error.
pub(crate) fn confirm(
    note: Option<&str>,
    message: &str,
    skip: bool,
    how_to_skip: &str,
) -> Result<bool> {
    if skip {
        return Ok(true);
    }
    if let Some(note) = note {
        println!("{note}");
    }
    print!("{message} [y/N]: ");
    io::stdout().flush()?;
    let mut input = String::new();
    if io::stdin().read_line(&mut input)? == 0 {
        // The question is left without its line end; give it one.
        println!();
        bail!(
            "no answer could be read (standard input is closed), so nothing was done; \
             pass {how_to_skip} to go ahead without being asked"
        );
    }
    let decision = input.trim().to_ascii_lowercase();
    Ok(decision == "y" || decision == "yes")
}
