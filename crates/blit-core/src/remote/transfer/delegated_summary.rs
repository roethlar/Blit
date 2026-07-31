//! The delegated topology's summary re-encode (cr-pfc4-1).
//!
//! Every transfer ends in exactly one authoritative summary: the
//! session's [`TransferSummary`], computed by the end that wrote the
//! bytes (`transfer_session::run_session`'s single construction site).
//! The delegated remote→remote route is the one topology where that
//! summary does NOT reach the initiator directly — the destination
//! daemon terminates the session and re-encodes its outcome into the
//! `DelegatedPull` RPC's own [`DelegatedPullSummary`] progress message.
//!
//! That second encoding is why pfc-4's per-file failure report went
//! missing on this route: extending the session summary is not enough
//! when a second message re-states it. The re-encode therefore lives
//! HERE, beside the contract it copies, rather than inline at the
//! daemon's send site — one function to extend when the summary grows
//! again, and one place to read to see what the delegated initiator can
//! observe.
//!
//! The failure report is copied, never recomputed: the sender-side
//! 64-entry cap ([`crate::remote::transfer::MAX_REPORTED_FILE_FAILURES`],
//! applied by `SinkOutcome::wire_failures` at the summary's construction
//! site) has already been applied, and `files_failed` is the exact total
//! behind that bounded list. A second capping path here could only
//! disagree with the authoritative one.

use crate::generated::{DelegatedPullSummary, TransferSummary};
use crate::remote::transfer::sink::FileFailure;

/// Re-encode the session summary this delegated destination computed
/// into the `DelegatedPull` RPC's summary message.
///
/// `source_peer_observed` is the delegation-specific diagnostic the
/// session summary has no field for (the data-plane peer this
/// destination observed while pulling from the source).
///
/// Field mapping notes:
///   * `bytes_zero_copy` has no session-summary counterpart and stays 0
///     — it predates the unified session and no lane reports it.
///   * `tcp_fallback_used` is wire-compat for the old "the gRPC byte
///     fallback carried the payload" bit, which on the session is
///     `in_stream_carrier_used`.
///   * `files_failed` / `failures` are the destination's contained
///     per-file failure report, carried through verbatim.
pub fn delegated_summary_from_session(
    summary: &TransferSummary,
    source_peer_observed: String,
) -> DelegatedPullSummary {
    DelegatedPullSummary {
        files_transferred: summary.files_transferred,
        bytes_transferred: summary.bytes_transferred,
        bytes_zero_copy: 0,
        tcp_fallback_used: summary.in_stream_carrier_used,
        entries_deleted: summary.entries_deleted,
        source_peer_observed,
        files_failed: summary.files_failed,
        failures: summary.failures.clone(),
    }
}

/// The delegated summary's failure report in the in-memory shape the
/// local surface carries (`LocalMirrorSummary::failures`), so an
/// initiator renders one shape whatever topology produced the run.
///
/// The count stays on the message as the exact
/// [`DelegatedPullSummary::files_failed`]: a returned list shorter than
/// that count means the sender capped its detail, not that failures
/// were lost.
pub fn delegated_summary_failures(summary: &DelegatedPullSummary) -> Vec<FileFailure> {
    summary
        .failures
        .iter()
        .map(FileFailure::from_wire)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::remote::transfer::sink::{SinkOutcome, MAX_REPORTED_FILE_FAILURES};

    /// Build the authoritative summary exactly as the session's single
    /// construction site does: the count is `SinkOutcome`'s exact total,
    /// the list is its already-capped wire form.
    fn session_summary(contained: &SinkOutcome) -> TransferSummary {
        TransferSummary {
            files_transferred: 7,
            bytes_transferred: 4096,
            entries_deleted: 2,
            in_stream_carrier_used: true,
            files_resumed: 1,
            files_failed: contained.files_failed_total,
            failures: contained.wire_failures(),
        }
    }

    /// cr-pfc4-1: a destination that contained more failures than the
    /// report cap must reach the delegated initiator with the EXACT
    /// count and every carried entry intact — the count is what tells an
    /// operator files are missing, and the entries are what name them.
    /// Before the copy-through the re-encode dropped both, so a
    /// delegated remote→remote mirror reported clean success.
    #[test]
    fn delegated_summary_round_trips_the_capped_failure_report() {
        let mut contained = SinkOutcome::written(7, 4096);
        for index in 0..(MAX_REPORTED_FILE_FAILURES + 6) {
            contained.record_failure(format!("dir/f{index}.bin"), format!("synthetic {index}"));
        }
        let summary = session_summary(&contained);
        assert_eq!(summary.files_failed, 70);
        assert_eq!(summary.failures.len(), MAX_REPORTED_FILE_FAILURES);

        let delegated = delegated_summary_from_session(&summary, "127.0.0.1:9031".to_string());

        // The rest of the re-encode still holds (a copy-through must not
        // cost the fields that already worked).
        assert_eq!(delegated.files_transferred, 7);
        assert_eq!(delegated.bytes_transferred, 4096);
        assert_eq!(delegated.entries_deleted, 2);
        assert!(delegated.tcp_fallback_used);
        assert_eq!(delegated.source_peer_observed, "127.0.0.1:9031");

        // Exact count behind a bounded list — never the list's length.
        assert_eq!(delegated.files_failed, 70);
        assert_eq!(delegated.failures.len(), MAX_REPORTED_FILE_FAILURES);

        // …and back onto the consumer surface, entries intact and in
        // order.
        let report = delegated_summary_failures(&delegated);
        assert_eq!(report.len(), MAX_REPORTED_FILE_FAILURES);
        assert_eq!(report[0].relative_path, "dir/f0.bin");
        assert_eq!(report[0].reason, "synthetic 0");
        assert_eq!(
            report[MAX_REPORTED_FILE_FAILURES - 1].relative_path,
            format!("dir/f{}.bin", MAX_REPORTED_FILE_FAILURES - 1)
        );
        assert_eq!(
            report[MAX_REPORTED_FILE_FAILURES - 1].reason,
            format!("synthetic {}", MAX_REPORTED_FILE_FAILURES - 1)
        );
        assert_eq!(
            report,
            contained.failures[..MAX_REPORTED_FILE_FAILURES].to_vec(),
            "the initiator's report is the destination's own records"
        );
    }

    /// A clean run carries no report at all — the delegated initiator
    /// must be able to tell "nothing failed" from "the report was
    /// dropped in transit".
    #[test]
    fn a_clean_session_summary_re_encodes_with_an_empty_report() {
        let summary = session_summary(&SinkOutcome::written(3, 30));
        let delegated = delegated_summary_from_session(&summary, String::new());

        assert_eq!(delegated.files_failed, 0);
        assert!(delegated.failures.is_empty());
        assert!(delegated_summary_failures(&delegated).is_empty());
    }
}
