//! The per-file failure report's surface-neutral half: how a failed
//! file is named, how a peer's carried report is read back, and the
//! source-deletion gate every move route shares (pfc-5,
//! D-2026-07-30-1).
//!
//! This lives in `blit-app`, not in a single front end, because BOTH
//! shipped surfaces delete a move's source: `blit-cli`'s four
//! `run_move` routes and `blit-tui`'s four move routes. One gate, one
//! refusal wording, no second copy to keep in step. Rendering (the
//! operator block, the JSON shape, the process exit status) stays in
//! the consuming crate, per this crate's no-presentation charter.

use blit_core::remote::transfer::FileFailure;
use eyre::{bail, Result};

/// How one failed file is named. `""` is the single-file destination
/// convention (the destination root IS the file), named as such rather
/// than left blank — the same rule `SessionFault`'s end-of-operation
/// summary applies. Shared so the refusal and the operator block never
/// disagree about what a file is called.
pub fn shown_path(relative_path: &str) -> &str {
    if relative_path.is_empty() {
        "<the transfer root file>"
    } else {
        relative_path
    }
}

/// A peer's carried report read back into the in-memory shape the
/// gate and the renderers take, so push / pull / delegated share the
/// local carrier's code path instead of each handling the wire type.
pub fn failures_from_wire(wire: &[blit_core::generated::FileFailure]) -> Vec<FileFailure> {
    wire.iter().map(FileFailure::from_wire).collect()
}

/// Number of failed files a refusal names inline before deferring to
/// the report/log. The refusal is one error line, not the report.
const REFUSAL_NAMED_FAILURES: usize = 3;

/// D-2026-07-30-1 Q1(b): every source-deleting route refuses to delete
/// while any file failed to land. A failed file's source copy is its
/// only copy, so the destination has to converge first — re-run, then
/// move.
///
/// This is the real gate that replaced pfc-2's interim in-session
/// `!mirror_enabled` refusal: the session now contains per-file
/// failures under every declaration and reports them, and the decision
/// that used to be guessed from `mirror_enabled` is taken here, where
/// the caller knows whether it is about to delete the source. Because
/// the interlock is gone, a route that skips this call deletes the
/// source of a file that never landed.
///
/// Complements — never replaces — the R47-F4 scan gate beside it: that
/// one covers SOURCE-side read failures, this one the destination's
/// write failures.
pub fn refuse_source_delete_on_failures(
    source: &str,
    files_failed: u64,
    failures: &[FileFailure],
) -> Result<()> {
    if files_failed == 0 {
        return Ok(());
    }
    let named = failures
        .iter()
        .take(REFUSAL_NAMED_FAILURES)
        .map(|failure| format!("{}: {}", shown_path(&failure.relative_path), failure.reason))
        .collect::<Vec<_>>()
        .join("; ");
    bail!(
        "refusing to remove source {source}: {files_failed} file(s) could not \
         be written and remain un-landed at the destination — deleting the \
         source now would destroy their only copy; first {}: {named}. Re-run \
         to converge, then move.",
        failures.len().min(REFUSAL_NAMED_FAILURES),
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    fn failure(relative_path: &str, reason: &str) -> FileFailure {
        FileFailure {
            relative_path: relative_path.to_string(),
            reason: reason.to_string(),
        }
    }

    /// Q1(b): the gate passes a clean run through and refuses a run
    /// with any failure, naming what did not land.
    #[test]
    fn the_source_delete_gate_refuses_only_when_a_file_failed() {
        refuse_source_delete_on_failures("/src", 0, &[])
            .expect("a clean run may delete its source");

        let err = refuse_source_delete_on_failures(
            "/src",
            1,
            &[failure("blocked.bin", "Is a directory (os error 21)")],
        )
        .expect_err("a failed file must block the source delete");
        let message = format!("{err:#}");
        assert!(
            message.contains("refusing to remove source /src"),
            "{message}"
        );
        assert!(message.contains("blocked.bin"), "{message}");
        assert!(
            message.contains("Re-run to converge, then move."),
            "{message}"
        );
    }

    /// The count is exact even when the carried detail is capped, and
    /// the refusal names only its inline budget — it must not print 64
    /// paths into one error line.
    #[test]
    fn a_capped_report_still_refuses_and_names_only_its_budget() {
        let failures = (0..10)
            .map(|i| failure(&format!("f{i}.bin"), "boom"))
            .collect::<Vec<_>>();
        let err = refuse_source_delete_on_failures("/src", 70, &failures)
            .expect_err("a capped report still refuses");
        let message = format!("{err:#}");
        assert!(message.contains("70 file(s) could not"), "{message}");
        assert!(message.contains("f0.bin"), "{message}");
        assert!(
            !message.contains("f3.bin"),
            "only the first {REFUSAL_NAMED_FAILURES} are named inline: {message}"
        );
    }

    /// The single-file destination convention: an empty relative path
    /// is the transfer root, not an unnamed file.
    #[test]
    fn the_transfer_root_file_is_named_not_blank() {
        assert_eq!(shown_path(""), "<the transfer root file>");
        assert_eq!(shown_path("sub/x.bin"), "sub/x.bin");
    }

    /// A peer's wire entries arrive as the same in-memory records the
    /// local carrier produces.
    #[test]
    fn wire_entries_read_back_as_in_memory_records() {
        let wire = vec![blit_core::generated::FileFailure {
            relative_path: "sub/blocked.bin".to_string(),
            reason: "Access is denied. (os error 5)".to_string(),
        }];
        let read_back = failures_from_wire(&wire);
        assert_eq!(read_back.len(), 1);
        assert_eq!(read_back[0].relative_path, "sub/blocked.bin");
        assert_eq!(read_back[0].reason, "Access is denied. (os error 5)");
        assert!(failures_from_wire(&[]).is_empty());
    }
}
