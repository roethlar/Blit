//! ssc-6 (D-2026-09-28-1, D-2026-09-28-3): end-of-run retry passes over
//! the files that failed.
//!
//! Owner's rule: "collect all errors, then … retry at the end of the
//! transfer that will rescan and retry." After a route's main pass, if
//! it recorded per-file failures and `--retries` is non-zero, the
//! orchestrator waits `--retry-wait` seconds and runs one further
//! session limited to exactly those paths (`FileFilter.files_from`),
//! re-scanned fresh through the same machinery, pass after pass, until
//! a pass ends with no failures or `--retries` passes have run. Files
//! that land on a retry count once as transferred and leave the
//! report; files that fail again are reported once, their reason
//! marked `(retried)`. The final state is what the exit status, the
//! JSON fields and the move source-delete gate read.
//!
//! Retry passes never mirror-delete: deletions are planned by the main
//! pass from its complete source set (with the failed paths shielding
//! their destination subtrees, A19); a retry pass only transfers.
//!
//! No prompt in any mode — unattended runs must not hang — and no other
//! switch: `--retries 0` is how a caller opts out.

use std::collections::HashSet;
use std::future::Future;
use std::path::PathBuf;
use std::time::Duration;

use blit_core::generated::TransferSummary;
use blit_core::remote::transfer::FileFailure;
use blit_core::transfer_session::local::LocalMirrorSummary;
use blit_core::transfers::remote::DelegatedPullOutcome;
use eyre::Result;

use crate::cli::TransferArgs;

/// The failure report of one pass, in the one shape every route
/// produces: the exact count, the named (capped) report, and the exact
/// path set the next pass retries.
#[derive(Clone, Debug, Default)]
pub(crate) struct PassFailures {
    pub files_failed: u64,
    pub failures: Vec<FileFailure>,
    pub failed_paths: Vec<String>,
    pub failed_paths_truncated: bool,
    /// cr-ssc6-1: failures no retry pass was ever given — the unnamed
    /// remainder of a truncated retry set. They stay failed (they were
    /// never observed to converge), are counted in `files_failed`, and
    /// are reported once as a synthesized entry.
    pub unretried: u64,
}

impl PassFailures {
    pub(crate) fn from_summary(summary: &TransferSummary) -> Self {
        Self {
            files_failed: summary.files_failed,
            failures: super::failures::failures_from_wire(&summary.failures),
            failed_paths: summary.failed_paths.clone(),
            failed_paths_truncated: summary.failed_paths_truncated,
            unretried: 0,
        }
    }

    pub(crate) fn from_local(summary: &LocalMirrorSummary) -> Self {
        Self {
            files_failed: summary.files_failed,
            failures: summary.failures.clone(),
            failed_paths: summary.failed_paths.clone(),
            failed_paths_truncated: summary.failed_paths_truncated,
            unretried: 0,
        }
    }

    pub(crate) fn from_delegated(outcome: &DelegatedPullOutcome) -> Self {
        let (paths, truncated) = outcome.failed_paths();
        Self {
            files_failed: outcome.files_failed(),
            failures: outcome.contained_failures(),
            failed_paths: paths.to_vec(),
            failed_paths_truncated: truncated,
            unretried: 0,
        }
    }

    /// The set the next pass is limited to. The exact set when the
    /// sender could carry it; otherwise the named report's paths, with
    /// `truncated` telling the caller to say so.
    fn retry_set(&self) -> (HashSet<PathBuf>, bool) {
        if !self.failed_paths_truncated && !self.failed_paths.is_empty() {
            return (self.failed_paths.iter().map(PathBuf::from).collect(), false);
        }
        let truncated =
            self.failed_paths_truncated || (self.files_failed as usize) > self.failures.len();
        (
            self.failures
                .iter()
                .map(|f| PathBuf::from(&f.relative_path))
                .collect(),
            truncated,
        )
    }

    /// Mark every remaining failure as one that survived a retry, and
    /// name the ones no pass could reach.
    fn mark_retried(&mut self) {
        for failure in &mut self.failures {
            if !failure.reason.ends_with(RETRIED_SUFFIX) {
                failure.reason.push_str(RETRIED_SUFFIX);
            }
        }
        if self.unretried > 0 {
            self.failures.push(FileFailure {
                relative_path: UNRETRIED_PATH.to_string(),
                reason: format!(
                    "{} file(s) were not retried: the retry set was truncated to the \
                     named report; re-run to converge",
                    self.unretried
                ),
            });
        }
    }

    /// cr-ssc6-1: fold one pass's result over the set it was given. A
    /// pass positively accounts for every path it was asked to retry — a
    /// scoped scan reports what it could not enumerate as that file's
    /// failure (`ManifestComplete.scan_failures`), and the diff/sink
    /// report the rest — so its failure report IS the pending set of the
    /// paths it retried. What the pass was never given (the unnamed
    /// remainder of a truncated set) is carried forward unchanged.
    fn after_pass(&self, retried: usize, truncated: bool, mut next: PassFailures) -> PassFailures {
        let previously_named = self.files_failed.saturating_sub(self.unretried);
        let not_given = if truncated {
            previously_named.saturating_sub(retried as u64)
        } else {
            0
        };
        next.unretried = self.unretried.saturating_add(not_given);
        next.files_failed = next.files_failed.saturating_add(next.unretried);
        next
    }

    /// Write the final state back onto a wire-shaped summary.
    pub(crate) fn apply_to_summary(&self, summary: &mut TransferSummary) {
        summary.files_failed = self.files_failed;
        summary.failures = self.failures.iter().map(FileFailure::to_wire).collect();
        summary.failed_paths = self.failed_paths.clone();
        summary.failed_paths_truncated = self.failed_paths_truncated;
    }

    /// Write the final state back onto a local summary.
    pub(crate) fn apply_to_local(&self, summary: &mut LocalMirrorSummary) {
        summary.files_failed = self.files_failed;
        summary.failures = self.failures.clone();
        summary.failed_paths = self.failed_paths.clone();
        summary.failed_paths_truncated = self.failed_paths_truncated;
    }
}

/// Appended to the reason of a file that failed again on a retry pass.
pub(crate) const RETRIED_SUFFIX: &str = " (retried)";

/// The path column of the synthesized report entry for failures no retry
/// pass was given (cr-ssc6-1).
pub(crate) const UNRETRIED_PATH: &str = "(not retried)";

/// cr-ssc6-3: `--detach` hands the transfer to the destination daemon
/// and exits before any summary exists, so no retry pass can run on this
/// side; a daemon-owned retry would need the pass loop and its switches
/// on the wire (recorded as a known gap). Rather than accept `--retries`
/// silently, a detached run says so once — unless the caller already
/// opted out with `--retries 0`. Returned as text so the notice is
/// testable; the caller prints it to stderr.
pub(crate) fn detach_retry_notice(args: &TransferArgs) -> Option<String> {
    if !args.detach || args.retries == 0 {
        return None;
    }
    Some(format!(
        "retry passes are not applied to detached jobs (--retries {} ignored); \
         pass --retries 0 to silence this notice",
        args.retries
    ))
}

/// What one retry pass returns to the loop.
pub(crate) struct PassResult {
    pub files_transferred: u64,
    pub bytes_transferred: u64,
    pub failures: PassFailures,
    /// cr-ssc6-4: carrier and resume facts of the pass, folded into the
    /// final summary (OR / sum) so a retry that fell back to the
    /// in-stream carrier or resumed block-wise is reported as such.
    pub in_stream_carrier_used: bool,
    pub files_resumed: u64,
}

/// What the loop hands back for the route to fold into its final
/// summary: the files and bytes the retries added, the final failure
/// state, and how many passes ran.
pub(crate) struct RetryOutcome {
    pub added_files: u64,
    pub added_bytes: u64,
    pub final_failures: PassFailures,
    pub passes_run: u32,
    /// cr-ssc6-4: true when any retry pass used the in-stream carrier.
    pub in_stream_carrier_used: bool,
    /// cr-ssc6-4: files the retry passes resumed block-wise, summed.
    pub files_resumed: u64,
}

impl RetryOutcome {
    /// Fold the retries into a wire-shaped summary: files and bytes the
    /// retries landed are added once, and the failure report becomes the
    /// final state.
    pub(crate) fn fold_into_summary(&self, summary: &mut TransferSummary) {
        if self.passes_run == 0 {
            return;
        }
        summary.files_transferred = summary.files_transferred.saturating_add(self.added_files);
        summary.bytes_transferred = summary.bytes_transferred.saturating_add(self.added_bytes);
        summary.in_stream_carrier_used |= self.in_stream_carrier_used;
        summary.files_resumed = summary.files_resumed.saturating_add(self.files_resumed);
        self.final_failures.apply_to_summary(summary);
    }

    /// The delegated re-encode's shape of the same fold.
    pub(crate) fn fold_into_delegated(
        &self,
        summary: &mut blit_core::generated::DelegatedPullSummary,
    ) {
        if self.passes_run == 0 {
            return;
        }
        summary.files_transferred = summary.files_transferred.saturating_add(self.added_files);
        summary.bytes_transferred = summary.bytes_transferred.saturating_add(self.added_bytes);
        // The delegated summary names the carrier fact `tcp_fallback_used`
        // and carries no resume count (cr-ssc6-4 known gap: a delegated
        // retry's block-wise resumes are not reported).
        summary.tcp_fallback_used |= self.in_stream_carrier_used;
        summary.files_failed = self.final_failures.files_failed;
        summary.failures = self
            .final_failures
            .failures
            .iter()
            .map(FileFailure::to_wire)
            .collect();
        summary.failed_paths = self.final_failures.failed_paths.clone();
        summary.failed_paths_truncated = self.final_failures.failed_paths_truncated;
    }
}

/// The pass loop, shared by every route. `run` executes one retry pass
/// with the prepared arguments (the retry-only set and the pass label
/// already threaded in) and returns that pass's result.
pub(crate) async fn run_retry_passes<F, Fut>(
    args: &TransferArgs,
    main: PassFailures,
    mut run: F,
) -> Result<RetryOutcome>
where
    F: FnMut(TransferArgs) -> Fut,
    Fut: Future<Output = Result<PassResult>>,
{
    let mut current = main;
    let mut added_files = 0u64;
    let mut added_bytes = 0u64;
    let mut passes_run = 0u32;
    let mut in_stream_carrier_used = false;
    let mut files_resumed = 0u64;
    // A dry run writes nothing, so nothing it reported can converge by
    // retrying; the report stands as the first pass produced it.
    let retries = if args.dry_run { 0 } else { args.retries };
    for pass in 1..=retries {
        if current.files_failed == 0 {
            break;
        }
        let (set, truncated) = current.retry_set();
        if set.is_empty() {
            break;
        }
        let total = retries;
        let n = set.len();
        wait_before_pass(args, pass).await;
        if !args.json {
            if truncated {
                eprintln!(
                    "retrying {n} of {} file(s) (pass {pass} of {total}) — the retry set was \
                     truncated to the named report; re-run to converge the rest",
                    current.files_failed
                );
            } else {
                eprintln!("retrying {n} file(s) (pass {pass} of {total})");
            }
        }
        blit_core::remote::instrumentation::record_retry_pass(u64::from(pass));
        let mut pass_args = args.clone();
        pass_args.retry_only = Some(set);
        pass_args.retry_pass = Some((pass, total, n));
        let result = run(pass_args).await?;
        added_files = added_files.saturating_add(result.files_transferred);
        added_bytes = added_bytes.saturating_add(result.bytes_transferred);
        in_stream_carrier_used |= result.in_stream_carrier_used;
        files_resumed = files_resumed.saturating_add(result.files_resumed);
        current = current.after_pass(n, truncated, result.failures);
        passes_run = pass;
    }
    if passes_run > 0 {
        current.mark_retried();
    }
    Ok(RetryOutcome {
        added_files,
        added_bytes,
        final_failures: current,
        passes_run,
        in_stream_carrier_used,
        files_resumed,
    })
}

/// Honour `--retry-wait` before a pass. The requested wait is always
/// recorded to the diagnostics counter file (when one is installed);
/// the hidden diagnostics switch replaces the sleep itself so tests
/// never wait for real.
async fn wait_before_pass(args: &TransferArgs, _pass: u32) {
    blit_core::remote::instrumentation::record_retry_wait_seconds(args.retry_wait);
    if args.retry_wait == 0 || args.diagnostics_no_retry_wait {
        return;
    }
    tokio::time::sleep(Duration::from_secs(args.retry_wait)).await;
}

#[cfg(test)]
mod tests {
    use super::*;

    fn failure(path: &str) -> FileFailure {
        FileFailure {
            relative_path: path.to_string(),
            reason: "source: cannot open: boom".to_string(),
        }
    }

    fn failures(paths: &[&str]) -> PassFailures {
        PassFailures {
            files_failed: paths.len() as u64,
            failures: paths.iter().map(|p| failure(p)).collect(),
            failed_paths: paths.iter().map(|p| p.to_string()).collect(),
            failed_paths_truncated: false,
            unretried: 0,
        }
    }

    fn args(retries: u32) -> TransferArgs {
        let mut args = TransferArgs::for_tests("src", "dst");
        args.retries = retries;
        args.retry_wait = 0;
        args
    }

    #[tokio::test]
    async fn retries_stop_early_when_a_pass_ends_clean() {
        let mut calls = 0u32;
        let out = run_retry_passes(&args(3), failures(&["a", "b"]), |pass_args| {
            calls += 1;
            let set = pass_args.retry_only.clone().expect("retry set");
            async move {
                assert_eq!(set.len(), 2);
                Ok(PassResult {
                    files_transferred: 2,
                    bytes_transferred: 10,
                    failures: PassFailures::default(),
                    in_stream_carrier_used: false,
                    files_resumed: 0,
                })
            }
        })
        .await
        .expect("loop");
        assert_eq!(calls, 1, "a clean pass ends the loop");
        assert_eq!(out.passes_run, 1);
        assert_eq!(out.added_files, 2);
        assert_eq!(out.final_failures.files_failed, 0);
    }

    #[tokio::test]
    async fn retries_are_bounded_and_survivors_are_marked() {
        let mut calls = 0u32;
        let out = run_retry_passes(&args(2), failures(&["a"]), |pass_args| {
            calls += 1;
            assert_eq!(pass_args.retry_pass, Some((calls, 2, 1)));
            async move {
                Ok(PassResult {
                    files_transferred: 0,
                    bytes_transferred: 0,
                    failures: failures(&["a"]),
                    in_stream_carrier_used: false,
                    files_resumed: 0,
                })
            }
        })
        .await
        .expect("loop");
        assert_eq!(calls, 2, "--retries bounds the passes");
        assert_eq!(out.passes_run, 2);
        assert_eq!(out.final_failures.files_failed, 1);
        assert!(out.final_failures.failures[0]
            .reason
            .ends_with(RETRIED_SUFFIX));
    }

    #[tokio::test]
    async fn zero_retries_or_zero_failures_run_nothing() {
        let mut calls = 0u32;
        let out = run_retry_passes(&args(0), failures(&["a"]), |_| {
            calls += 1;
            async { unreachable!("no pass with --retries 0") }
        })
        .await
        .expect("loop");
        assert_eq!((calls, out.passes_run), (0, 0));
        assert_eq!(out.final_failures.files_failed, 1);
        assert!(
            !out.final_failures.failures[0]
                .reason
                .ends_with(RETRIED_SUFFIX),
            "nothing was retried, so nothing is marked"
        );
        let out = run_retry_passes(&args(3), PassFailures::default(), |_| {
            calls += 1;
            async { unreachable!("no pass without failures") }
        })
        .await
        .expect("loop");
        assert_eq!((calls, out.passes_run), (0, 0));
    }

    #[tokio::test]
    async fn a_truncated_set_retries_the_named_report_only() {
        let mut main = failures(&["a", "b"]);
        main.failed_paths.clear();
        main.failed_paths_truncated = true;
        main.files_failed = 5; // three more than the report names
        let out = run_retry_passes(&args(1), main, |pass_args| {
            let set = pass_args.retry_only.clone().expect("retry set");
            async move {
                assert_eq!(set.len(), 2, "only the named report is retried");
                Ok(PassResult {
                    files_transferred: 2,
                    bytes_transferred: 0,
                    failures: PassFailures::default(),
                    in_stream_carrier_used: false,
                    files_resumed: 0,
                })
            }
        })
        .await
        .expect("loop");
        assert_eq!(out.passes_run, 1);
        // cr-ssc6-1: the three failures the report could not name were
        // never retried, so they are still failed — counted, named once
        // as a synthesized entry, and enough for the move gate to refuse.
        assert_eq!(out.final_failures.files_failed, 3);
        assert_eq!(out.final_failures.unretried, 3);
        let synth: Vec<_> = out
            .final_failures
            .failures
            .iter()
            .filter(|f| f.relative_path == UNRETRIED_PATH)
            .collect();
        assert_eq!(
            synth.len(),
            1,
            "one synthesized entry: {:?}",
            out.final_failures.failures
        );
        assert!(synth[0].reason.starts_with("3 file(s) were not retried"));
        assert!(
            blit_core::transfers::failures::refuse_source_delete_on_failures(
                "src",
                out.final_failures.files_failed,
                &out.final_failures.failures,
            )
            .is_err(),
            "a move must refuse while unretried failures remain"
        );
    }

    /// cr-ssc6-1: a pass that reports fewer failures than it was given
    /// has positively accounted for the rest (the scoped scan names what
    /// it could not enumerate); nothing unnamed is invented for a
    /// complete set, and a truncated remainder survives a clean pass.
    #[tokio::test]
    async fn unretried_remainder_survives_a_clean_pass_only_when_truncated() {
        let out = run_retry_passes(&args(1), failures(&["a", "b"]), |_| async {
            Ok(PassResult {
                files_transferred: 1,
                bytes_transferred: 0,
                failures: failures(&["b"]),
                in_stream_carrier_used: false,
                files_resumed: 0,
            })
        })
        .await
        .expect("loop");
        assert_eq!(out.final_failures.files_failed, 1);
        assert_eq!(out.final_failures.unretried, 0);
        assert_eq!(out.final_failures.failures.len(), 1);
    }

    /// cr-ssc6-3: a detached run with retries requested says once that
    /// none will be applied; `--retries 0` (the opt-out) and attached
    /// runs say nothing.
    #[test]
    fn a_detached_run_with_retries_is_told_none_apply() {
        let mut a = args(2);
        a.detach = true;
        let notice = detach_retry_notice(&a).expect("notice");
        assert!(notice.contains("not applied to detached jobs"), "{notice}");
        assert!(notice.contains("--retries 2 ignored"), "{notice}");
        a.retries = 0;
        assert!(
            detach_retry_notice(&a).is_none(),
            "--retries 0 is the opt-out"
        );
        let mut attached = args(2);
        attached.detach = false;
        assert!(detach_retry_notice(&attached).is_none());
    }

    /// cr-ssc6-4: a retry pass that rode the in-stream carrier or resumed
    /// block-wise is reported in the final summary (OR / sum), not lost
    /// with the pass.
    #[tokio::test]
    async fn retry_passes_carry_carrier_and_resume_facts_into_the_summary() {
        let out = run_retry_passes(&args(2), failures(&["a", "b"]), |pass_args| {
            let pass = pass_args.retry_pass.map(|(k, _, _)| k).unwrap_or(0);
            async move {
                Ok(PassResult {
                    files_transferred: 1,
                    bytes_transferred: 0,
                    failures: if pass == 1 {
                        failures(&["b"])
                    } else {
                        PassFailures::default()
                    },
                    in_stream_carrier_used: pass == 2,
                    files_resumed: 1,
                })
            }
        })
        .await
        .expect("loop");
        assert_eq!(out.passes_run, 2);
        let mut summary = TransferSummary::default();
        out.fold_into_summary(&mut summary);
        assert!(
            summary.in_stream_carrier_used,
            "one pass used the in-stream carrier"
        );
        assert_eq!(
            summary.files_resumed, 2,
            "resumed files are summed across passes"
        );
        let mut delegated = blit_core::generated::DelegatedPullSummary::default();
        out.fold_into_delegated(&mut delegated);
        assert!(
            delegated.tcp_fallback_used,
            "the delegated summary's carrier fact"
        );
    }

    #[tokio::test]
    async fn a_dry_run_never_retries() {
        let mut a = args(3);
        a.dry_run = true;
        let out = run_retry_passes(&a, failures(&["a"]), |_| async {
            unreachable!("dry run retries nothing")
        })
        .await
        .expect("loop");
        assert_eq!(out.passes_run, 0);
    }
}
