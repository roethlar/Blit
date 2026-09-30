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
}

impl PassFailures {
    pub(crate) fn from_summary(summary: &TransferSummary) -> Self {
        Self {
            files_failed: summary.files_failed,
            failures: super::failures::failures_from_wire(&summary.failures),
            failed_paths: summary.failed_paths.clone(),
            failed_paths_truncated: summary.failed_paths_truncated,
        }
    }

    pub(crate) fn from_local(summary: &LocalMirrorSummary) -> Self {
        Self {
            files_failed: summary.files_failed,
            failures: summary.failures.clone(),
            failed_paths: summary.failed_paths.clone(),
            failed_paths_truncated: summary.failed_paths_truncated,
        }
    }

    pub(crate) fn from_delegated(outcome: &DelegatedPullOutcome) -> Self {
        let (paths, truncated) = outcome.failed_paths();
        Self {
            files_failed: outcome.files_failed(),
            failures: outcome.contained_failures(),
            failed_paths: paths.to_vec(),
            failed_paths_truncated: truncated,
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

    /// Mark every remaining failure as one that survived a retry.
    fn mark_retried(&mut self) {
        for failure in &mut self.failures {
            if !failure.reason.ends_with(RETRIED_SUFFIX) {
                failure.reason.push_str(RETRIED_SUFFIX);
            }
        }
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

/// What one retry pass returns to the loop.
pub(crate) struct PassResult {
    pub files_transferred: u64,
    pub bytes_transferred: u64,
    pub failures: PassFailures,
}

/// What the loop hands back for the route to fold into its final
/// summary: the files and bytes the retries added, the final failure
/// state, and how many passes ran.
pub(crate) struct RetryOutcome {
    pub added_files: u64,
    pub added_bytes: u64,
    pub final_failures: PassFailures,
    pub passes_run: u32,
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
        current = result.failures;
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
                })
            }
        })
        .await
        .expect("loop");
        assert_eq!(out.passes_run, 1);
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
