//! The end-of-operation per-file failure report, shared by every
//! transfer route (pfc-5, D-2026-07-30-1).
//!
//! One renderer, one exit status, one JSON shape. All four topologies
//! reach the same destination-attested report — `LocalMirrorSummary` for
//! the local carrier, `TransferSummary` for push/pull,
//! `DelegatedPullSummary` for the delegated re-encode — so an operator
//! reading the block cannot tell which route produced it.
//!
//! The surface-neutral half — the failed-file naming rule, the wire
//! read-back, and the source-deletion gate — lives in
//! `blit_app::transfers::failures` because `blit-tui` deletes move
//! sources too; only rendering and the process exit status are this
//! crate's.

use std::process::ExitCode;

use blit_core::remote::transfer::FileFailure;
use serde_json::{json, Value};

use blit_app::transfers::failures::shown_path;
pub(crate) use blit_app::transfers::failures::{
    failures_from_wire, refuse_source_delete_on_failures,
};

/// Exit status for an operation that COMPLETED while some files did not
/// land. `0` still means every file landed, and a session that aborted
/// keeps whatever non-zero status its error already produced. In `--json`
/// mode a machine consumer gets both signals: this status and the
/// summary's `files_failed` field.
pub(crate) const EXIT_PARTIAL_FAILURE: u8 = 2;

/// Process exit status for a completed operation, from the
/// destination-attested count of files that did not land.
pub(crate) fn exit_for_failures(files_failed: u64) -> ExitCode {
    if files_failed == 0 {
        ExitCode::SUCCESS
    } else {
        ExitCode::from(EXIT_PARTIAL_FAILURE)
    }
}

/// A wire path may legally carry control bytes (a newline-bearing
/// filename is valid on unix), which would break the block's
/// one-entry-per-line shape. Rendered text only — the transfer always
/// uses the untouched path.
fn sanitize(text: &str) -> String {
    text.chars()
        .map(|c| if c.is_control() { '\u{FFFD}' } else { c })
        .collect()
}

/// The end-of-operation failure block (otp-7 D4 rider pattern,
/// D-2026-07-09-1): the count, one line per named file, the elision note
/// when the carried list is shorter than the count, and the
/// re-run-to-converge hint. `None` when nothing failed.
///
/// Returns the text rather than printing it so the format is pinned
/// without a terminal. Callers print it after the summary, which already
/// runs after the live row finished.
///
/// Entries render exactly as carried: an over-long path or reason was
/// already bounded and marked by the wire bound (cr-pfc4-2), so marking
/// it again here would double the annotation.
pub(crate) fn failure_block(files_failed: u64, failures: &[FileFailure]) -> Option<String> {
    if files_failed == 0 {
        return None;
    }
    let mut block =
        format!("{files_failed} file(s) could not be written and did not land at the destination:");
    for failure in failures {
        block.push_str(&format!(
            "\n  {}: {}",
            sanitize(shown_path(&failure.relative_path)),
            sanitize(&failure.reason)
        ));
    }
    // The carried list is bounded (entry count and encoded bytes); the
    // count is exact. A shorter list therefore means "elided here", never
    // "forgotten" — and the destination logged every one of them.
    let elided = files_failed.saturating_sub(failures.len() as u64);
    if elided > 0 {
        block.push_str(&format!("\n  … and {elided} more — see destination log"));
    }
    block
        .push_str("\nEverything else in the manifest landed; re-run the same command to converge.");
    Some(block)
}

/// Print the block after a human-readable summary. No-op on a clean run.
pub(crate) fn print_failure_block(files_failed: u64, failures: &[FileFailure]) {
    if let Some(block) = failure_block(files_failed, failures) {
        println!("{block}");
    }
}

/// The `failures` array every route's summary JSON carries, so one shape
/// serves every topology. Paired with the exit status, never instead of
/// it.
pub(crate) fn failures_json(failures: &[FileFailure]) -> Value {
    Value::Array(
        failures
            .iter()
            .map(|failure| {
                json!({
                    "relative_path": failure.relative_path,
                    "reason": failure.reason,
                })
            })
            .collect(),
    )
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

    /// A clean run has no block and exits 0 — the whole feature is
    /// invisible when every file landed.
    #[test]
    fn a_clean_run_renders_no_block_and_exits_zero() {
        assert!(failure_block(0, &[]).is_none());
        assert_eq!(
            format!("{:?}", exit_for_failures(0)),
            format!("{:?}", ExitCode::SUCCESS)
        );
    }

    /// The block's contract: the count, one `path: reason` line per named
    /// file, and the re-run hint.
    #[test]
    fn the_block_names_every_carried_file_with_its_reason() {
        let block = failure_block(
            2,
            &[
                failure("sub/blocked.bin", "Is a directory (os error 21)"),
                failure("other.bin", "Access is denied. (os error 5)"),
            ],
        )
        .expect("failures render a block");
        assert!(
            block.starts_with("2 file(s) could not be written"),
            "{block}"
        );
        assert!(
            block.contains("\n  sub/blocked.bin: Is a directory (os error 21)"),
            "{block}"
        );
        assert!(
            block.contains("\n  other.bin: Access is denied. (os error 5)"),
            "{block}"
        );
        assert!(
            block.contains("re-run the same command to converge"),
            "{block}"
        );
        assert!(
            !block.contains("more — see destination log"),
            "an uncapped list has nothing to elide: {block}"
        );
    }

    /// Past the carried cap the count stays exact and the block says so,
    /// pointing at the log that has every entry.
    #[test]
    fn a_capped_list_reports_the_elided_remainder() {
        let block = failure_block(70, &[failure("named.bin", "boom")]).expect("block");
        assert!(
            block.contains("  … and 69 more — see destination log"),
            "{block}"
        );
    }

    /// The single-file destination convention: an empty relative path is
    /// the transfer root, not an unnamed file.
    #[test]
    fn the_transfer_root_file_is_named_not_blank() {
        let block = failure_block(1, &[failure("", "boom")]).expect("block");
        assert!(block.contains("<the transfer root file>: boom"), "{block}");
    }

    /// A control byte in a path must not break the one-entry-per-line
    /// shape (a newline-bearing filename is valid on unix).
    #[test]
    fn control_bytes_never_add_lines_to_the_block() {
        let block = failure_block(1, &[failure("two\nlines.bin", "b\u{1b}[2Joom")]).expect("block");
        assert_eq!(
            block.lines().count(),
            3,
            "header + one entry + hint: {block}"
        );
        assert!(block.contains("two\u{FFFD}lines.bin"), "{block}");
    }

    /// A completed run with failures exits with the named partial-failure
    /// code, not success.
    #[test]
    fn a_contained_failure_exits_with_the_partial_failure_code() {
        assert_eq!(EXIT_PARTIAL_FAILURE, 2);
        assert_eq!(
            format!("{:?}", exit_for_failures(1)),
            format!("{:?}", ExitCode::from(EXIT_PARTIAL_FAILURE))
        );
    }

    /// The JSON shape every route carries: one object per named failure,
    /// same field names as the wire message.
    #[test]
    fn the_json_array_carries_path_and_reason_per_entry() {
        let value = failures_json(&[failure("blocked.bin", "Is a directory")]);
        assert_eq!(value[0]["relative_path"], "blocked.bin");
        assert_eq!(value[0]["reason"], "Is a directory");
        assert_eq!(failures_json(&[]), json!([]));
    }
}
