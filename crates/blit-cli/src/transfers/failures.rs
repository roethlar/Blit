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
//! `blit_core::transfers::failures` so any front end that deletes
//! move sources shares it; only rendering and the process exit
//! status are this crate's.

use std::process::ExitCode;

use blit_core::remote::transfer::FileFailure;
use serde_json::{json, Value};

use blit_core::transfers::failures::shown_path;
pub(crate) use blit_core::transfers::failures::{
    failures_from_wire, refuse_source_delete_on_failures,
};

use crate::style::{Palette, Role};

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
/// Production always goes through [`failure_block_styled`] — with a disabled
/// palette when colour is unwanted — so this plain form exists only as the
/// reference the clp-3 guards and the format pins compare against.
#[cfg(test)]
pub(crate) fn failure_block(files_failed: u64, failures: &[FileFailure]) -> Option<String> {
    failure_block_styled(files_failed, failures, Palette::disabled())
}

/// clp-3: the same block, coloured. A [`Palette::disabled`] palette returns
/// the input unchanged at every call, so `failure_block` above is the plain
/// form by construction rather than by a parallel implementation that could
/// drift from it.
///
/// Only the header, the per-file reason and the hint are painted. The failed
/// PATH stays default-foreground: it is the thing the operator copies out of
/// the terminal, and colouring it makes it harder to read against an
/// arbitrary background, not easier.
pub(crate) fn failure_block_styled(
    files_failed: u64,
    failures: &[FileFailure],
    palette: Palette,
) -> Option<String> {
    if files_failed == 0 {
        return None;
    }
    let mut block = palette.paint(
        Role::Failure,
        &format!(
            "{files_failed} file(s) could not be written and did not land at the destination:"
        ),
    );
    for failure in failures {
        block.push_str(&format!(
            "\n  {}: {}",
            sanitize(shown_path(&failure.relative_path)),
            palette.paint(Role::Failure, &sanitize(&failure.reason))
        ));
    }
    // The carried list is bounded (entry count and encoded bytes); the
    // count is exact. A shorter list therefore means "elided here", never
    // "forgotten" — and the destination logged every one of them.
    let elided = files_failed.saturating_sub(failures.len() as u64);
    if elided > 0 {
        block.push_str(&palette.paint(
            Role::Muted,
            &format!("\n  … and {elided} more — see destination log"),
        ));
    }
    block.push_str(&palette.paint(
        Role::Muted,
        "\nEverything else in the manifest landed; re-run the same command to converge.",
    ));
    Some(block)
}

/// Print the block after a human-readable summary. No-op on a clean run.
///
/// clp-3: the block goes to STDOUT, so the palette is resolved against
/// stdout's own terminal state — not the live row's, which owns stderr.
pub(crate) fn print_failure_block(files_failed: u64, failures: &[FileFailure]) {
    let palette = Palette::detect(console::Term::stdout().features().colors_supported());
    if let Some(block) = failure_block_styled(files_failed, failures, palette) {
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

    /// clp-3: colour is purely additive here too — strip the SGR sequences
    /// and the coloured block must equal the plain one byte for byte. This
    /// is what lets `failure_block` above stay the plain form and keeps
    /// every pre-clp-3 assertion in this file honest.
    #[test]
    fn colour_never_changes_the_block_text() {
        use crate::style::ColorDepth;

        let strip = |text: &str| -> String {
            let mut out = String::new();
            let mut chars = text.chars();
            while let Some(ch) = chars.next() {
                if ch != '\u{1b}' {
                    out.push(ch);
                    continue;
                }
                for next in chars.by_ref() {
                    if next == 'm' {
                        break;
                    }
                }
            }
            out
        };

        let cases: [(u64, Vec<FileFailure>); 3] = [
            (1, vec![failure("a.bin", "Access is denied. (os error 5)")]),
            (
                2,
                vec![
                    failure("sub/one.bin", "Is a directory (os error 21)"),
                    failure("two.bin", "Access is denied. (os error 5)"),
                ],
            ),
            // Elided case, so the muted "and N more" line is covered too.
            (70, vec![failure("one.bin", "boom")]),
        ];
        for depth in [ColorDepth::TrueColor, ColorDepth::Ansi256] {
            let palette = Palette::with_depth(depth);
            for (count, failures) in &cases {
                let plain = failure_block(*count, failures).expect("a block");
                let styled = failure_block_styled(*count, failures, palette).expect("a block");
                assert_ne!(plain, styled, "{depth:?}: nothing was coloured at all");
                assert_eq!(
                    strip(&styled),
                    plain,
                    "{depth:?}: colour changed the block's text"
                );
            }
        }
    }

    /// The header and the reason are RED, and the failed path is not.
    #[test]
    fn the_header_and_reason_are_red_and_the_path_is_not() {
        use crate::style::ColorDepth;

        let block = failure_block_styled(
            1,
            &[failure(
                "keep/this/path.bin",
                "Access is denied. (os error 5)",
            )],
            Palette::with_depth(ColorDepth::TrueColor),
        )
        .expect("a block");

        assert!(
            block.starts_with("\u{1b}[38;2;255;85;85m1 file(s) could not be written"),
            "the header must be Dracula red: {block:?}"
        );
        assert!(
            block.contains("\u{1b}[38;2;255;85;85mAccess is denied. (os error 5)\u{1b}[0m"),
            "the reason must be Dracula red: {block:?}"
        );
        // The path an operator copies out of the terminal stays plain.
        assert!(
            block.contains("\n  keep/this/path.bin: "),
            "the failed path must be unstyled: {block:?}"
        );
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
