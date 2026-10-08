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
    /// cr-rework-1: the exact subset of `failed_paths` whose failure left
    /// this run's own incomplete copy at the destination, and whether the
    /// sender could carry all of it.
    pub left_in_place: Vec<String>,
    pub left_in_place_truncated: bool,
    /// cr-rework-3: the failed paths whose incomplete copy the pass that
    /// produced this report positively removed, and whether all fit.
    pub removed_incomplete: Vec<String>,
    pub removed_incomplete_truncated: bool,
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
            left_in_place: summary.left_in_place.clone(),
            left_in_place_truncated: summary.left_in_place_truncated,
            removed_incomplete: summary.removed_incomplete.clone(),
            removed_incomplete_truncated: summary.removed_incomplete_truncated,
            unretried: 0,
        }
    }

    pub(crate) fn from_local(summary: &LocalMirrorSummary) -> Self {
        Self {
            files_failed: summary.files_failed,
            failures: summary.failures.clone(),
            failed_paths: summary.failed_paths.clone(),
            failed_paths_truncated: summary.failed_paths_truncated,
            left_in_place: summary.left_in_place.clone(),
            left_in_place_truncated: summary.left_in_place_truncated,
            removed_incomplete: summary.removed_incomplete.clone(),
            removed_incomplete_truncated: summary.removed_incomplete_truncated,
            unretried: 0,
        }
    }

    pub(crate) fn from_delegated(outcome: &DelegatedPullOutcome) -> Self {
        let (paths, truncated) = outcome.failed_paths();
        let (left, left_truncated) = outcome.left_in_place();
        let (removed, removed_truncated) = outcome.removed_incomplete();
        Self {
            files_failed: outcome.files_failed(),
            failures: outcome.contained_failures(),
            failed_paths: paths.to_vec(),
            failed_paths_truncated: truncated,
            left_in_place: left.to_vec(),
            left_in_place_truncated: left_truncated,
            removed_incomplete: removed.to_vec(),
            removed_incomplete_truncated: removed_truncated,
            unretried: 0,
        }
    }

    /// The set the next pass is limited to. The exact set when the
    /// sender could carry it; otherwise the named report's paths, with
    /// `truncated` telling the caller to say so.
    fn retry_set(&self) -> (HashSet<PathBuf>, bool) {
        // cr-fix2-2: the exact set is exact only when every counted
        // failure has a path in it (the unretried remainder is carried
        // separately). A peer that counted failures it could not name
        // (`scan_failures_dropped`) may still send `truncated = false`
        // when its named paths fit; the count exposes the gap, and the
        // set is then inexact — the difference must survive as unretried,
        // never be cleared by a later clean pass.
        let represented = (self.failed_paths.len() as u64).saturating_add(self.unretried);
        let all_represented = self.files_failed <= represented;
        if !self.failed_paths_truncated && !self.failed_paths.is_empty() && all_represented {
            return (self.failed_paths.iter().map(PathBuf::from).collect(), false);
        }
        if !self.failed_paths_truncated && !self.failed_paths.is_empty() {
            return (self.failed_paths.iter().map(PathBuf::from).collect(), true);
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

    /// cr-win-1 / cr-rework-1: the failed paths whose failure left this
    /// run's own incomplete copy at the destination — the exact set the
    /// destination sends beside `failed_paths`, never the capped report.
    fn left_in_place(&self) -> HashSet<PathBuf> {
        self.left_in_place.iter().map(PathBuf::from).collect()
    }

    /// Fold a second session of the same pass into this report.
    fn merge(&mut self, other: PassFailures) {
        self.files_failed = self.files_failed.saturating_add(other.files_failed);
        self.failures.extend(other.failures);
        self.failed_paths.extend(other.failed_paths);
        self.failed_paths_truncated |= other.failed_paths_truncated;
        self.left_in_place.extend(other.left_in_place);
        self.left_in_place_truncated |= other.left_in_place_truncated;
        self.removed_incomplete.extend(other.removed_incomplete);
        self.removed_incomplete_truncated |= other.removed_incomplete_truncated;
        self.unretried = self.unretried.saturating_add(other.unretried);
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
                    "{} file(s) were not retried: the retry set could not name them all; \
                     re-run to converge",
                    self.unretried
                ),
                raw_relative_path: None,
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
        // cr-rework-2: a leftover stays this run's own copy for as long as
        // its path keeps failing. A pass that failed it on the source side
        // never touched the copy, so its report — which names no leftover
        // for the path — cannot un-classify it; only a path that succeeded
        // leaves the set. cr-rework-3: nor does one whose copy the pass
        // positively removed — it is an ordinary failure from then on, and
        // a file that appears at the path later is protected by
        // --ignore-existing like any other. If the removed set did not
        // fit, a prior leftover it does not name may or may not still be
        // there: it is carried as neither, and classification is marked
        // incomplete, so the next pass retries only the leftovers it can
        // name and leaves the rest reported as not retried.
        let still_failing: HashSet<&str> = next.failed_paths.iter().map(String::as_str).collect();
        let removed: HashSet<&str> = next.removed_incomplete.iter().map(String::as_str).collect();
        let mut left: HashSet<String> = std::mem::take(&mut next.left_in_place)
            .into_iter()
            .collect();
        let carried: Vec<&String> = self
            .left_in_place
            .iter()
            .filter(|path| still_failing.contains(path.as_str()))
            .filter(|path| !removed.contains(path.as_str()))
            .filter(|path| !left.contains(*path))
            .collect();
        let ambiguous = next.removed_incomplete_truncated && !carried.is_empty();
        if !ambiguous {
            left.extend(carried.into_iter().cloned());
        }
        let mut left: Vec<String> = left.into_iter().collect();
        left.sort();
        next.left_in_place = left;
        next.left_in_place_truncated |= self.left_in_place_truncated || ambiguous;
        next
    }

    /// Write the final state back onto a wire-shaped summary.
    pub(crate) fn apply_to_summary(&self, summary: &mut TransferSummary) {
        summary.files_failed = self.files_failed;
        summary.failures = self.failures.iter().map(FileFailure::to_wire).collect();
        summary.failed_paths = self.failed_paths.clone();
        summary.failed_paths_truncated = self.failed_paths_truncated;
        summary.left_in_place = self.left_in_place.clone();
        summary.left_in_place_truncated = self.left_in_place_truncated;
        summary.removed_incomplete = self.removed_incomplete.clone();
        summary.removed_incomplete_truncated = self.removed_incomplete_truncated;
    }

    /// Write the final state back onto a local summary.
    pub(crate) fn apply_to_local(&self, summary: &mut LocalMirrorSummary) {
        summary.files_failed = self.files_failed;
        summary.failures = self.failures.clone();
        summary.failed_paths = self.failed_paths.clone();
        summary.failed_paths_truncated = self.failed_paths_truncated;
        summary.left_in_place = self.left_in_place.clone();
        summary.left_in_place_truncated = self.left_in_place_truncated;
        summary.removed_incomplete = self.removed_incomplete.clone();
        summary.removed_incomplete_truncated = self.removed_incomplete_truncated;
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

impl PassResult {
    /// Fold a second session of the same pass into this result.
    fn merge(&mut self, other: PassResult) {
        self.files_transferred = self
            .files_transferred
            .saturating_add(other.files_transferred);
        self.bytes_transferred = self
            .bytes_transferred
            .saturating_add(other.bytes_transferred);
        self.failures.merge(other.failures);
        self.in_stream_carrier_used |= other.in_stream_carrier_used;
        self.files_resumed = self.files_resumed.saturating_add(other.files_resumed);
    }
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
    /// 2026-10-07 defect (b): the user pressed Ctrl-C during the retry
    /// wait or a retry pass. The retries stopped; `final_failures` is the
    /// state before the pass that was cut short.
    pub interrupted: bool,
}

/// 2026-10-07 defect (b): what a run interrupted during its retries says
/// after its report, which names the files that did not land.
pub(crate) const INTERRUPTED_NOTE: &str =
    "blit: interrupted during the retries — the files listed \
     above did not land and were not retried further; re-run the same command to converge";

/// The exit code of a run interrupted during its retries (128 + SIGINT).
pub(crate) const INTERRUPTED_EXIT: u8 = 130;

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
        summary.left_in_place = self.final_failures.left_in_place.clone();
        summary.left_in_place_truncated = self.final_failures.left_in_place_truncated;
        summary.removed_incomplete = self.final_failures.removed_incomplete.clone();
        summary.removed_incomplete_truncated = self.final_failures.removed_incomplete_truncated;
    }
}

/// The pass loop, shared by every route. `run` executes one retry pass
/// with the prepared arguments (the retry-only set and the pass label
/// already threaded in) and returns that pass's result.
///
/// 2026-10-07 defect (b): Ctrl-C during the retry wait or a retry pass
/// stops the retries instead of killing the process, so the route still
/// prints the report of the pass that already ran — its deletions
/// included — and exits [`INTERRUPTED_EXIT`]. Once the handler has been
/// installed, a later Ctrl-C exits at once, as the default did.
pub(crate) async fn run_retry_passes<F, Fut>(
    args: &TransferArgs,
    main: PassFailures,
    run: F,
) -> Result<RetryOutcome>
where
    F: FnMut(TransferArgs) -> Fut,
    Fut: Future<Output = Result<PassResult>>,
{
    let armed = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
    let interrupt = {
        let armed = std::sync::Arc::clone(&armed);
        async move {
            armed.store(true, std::sync::atomic::Ordering::Relaxed);
            if tokio::signal::ctrl_c().await.is_err() {
                // No handler could be installed: never interrupt.
                std::future::pending::<()>().await;
            }
        }
    };
    let outcome = run_retry_passes_until(args, main, run, interrupt).await;
    if armed.load(std::sync::atomic::Ordering::Relaxed) {
        tokio::spawn(async {
            if tokio::signal::ctrl_c().await.is_ok() {
                std::process::exit(i32::from(INTERRUPTED_EXIT));
            }
        });
    }
    outcome
}

/// [`run_retry_passes`] with the interrupt as an input, so tests can
/// trigger it.
pub(crate) async fn run_retry_passes_until<F, Fut, I>(
    args: &TransferArgs,
    main: PassFailures,
    mut run: F,
    interrupt: I,
) -> Result<RetryOutcome>
where
    F: FnMut(TransferArgs) -> Fut,
    Fut: Future<Output = Result<PassResult>>,
    I: Future<Output = ()>,
{
    let mut interrupt = std::pin::pin!(interrupt);
    let mut interrupted = false;
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
        // cr-win-1: a retry pass compares exactly as the main pass did —
        // a failed write never leaves a target that looks finished (the
        // sink removes it or holds it one byte off the source's size), so
        // the user's compare is trusted here as on any later run. The one
        // exception is --ignore-existing: a path whose failure left this
        // run's own incomplete copy at the destination exists there now,
        // and that copy is not one the user asked to keep — those paths
        // retry with it off. cr-rework-1: they come from the exact set the
        // destination sends beside the failed paths, never the capped
        // report; if even that set did not fit, the paths it does not
        // name cannot be classified — retried with the flag a leftover is
        // skipped and its failure cleared, retried without it a file the
        // user asked to keep is overwritten — so none of them is retried:
        // they stay reported as not retried.
        let left: HashSet<PathBuf> = if args.ignore_existing {
            current
                .left_in_place()
                .intersection(&set)
                .cloned()
                .collect()
        } else {
            HashSet::new()
        };
        let (set, truncated) = if args.ignore_existing && current.left_in_place_truncated {
            (left.clone(), true)
        } else {
            (set, truncated)
        };
        if set.is_empty() {
            break;
        }
        let total = retries;
        let n = set.len();
        tokio::select! {
            biased;
            () = interrupt.as_mut() => {
                interrupted = true;
                break;
            }
            () = wait_before_pass(args, pass) => {}
        }
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
        pass_args.retry_pass = Some((pass, total, n));
        // JOB_LOGS jl-2: each pass is a phase of the run's log.
        let pass_phase = format!("retry pass {pass} of {total}");
        if let Some(run) = &args.run {
            run.record(blit_core::job_log::EventBody::Phase {
                name: pass_phase.clone(),
                state: blit_core::job_log::PhaseState::Start,
            })
            .await;
        }
        // win-1: one heap allocation per pass keeps the pass's session
        // out of this loop's state and its caller's frame.
        let pass_run = async {
            if left.is_empty() {
                pass_args.retry_only = Some(set);
                Box::pin(run(pass_args)).await
            } else {
                let rest: HashSet<PathBuf> = set.difference(&left).cloned().collect();
                let mut left_args = pass_args.clone();
                left_args.retry_only = Some(left);
                left_args.ignore_existing = false;
                let mut result = Box::pin(run(left_args)).await?;
                if !rest.is_empty() {
                    pass_args.retry_only = Some(rest);
                    result.merge(Box::pin(run(pass_args)).await?);
                }
                Ok(result)
            }
        };
        // Dropping a pass cut short ends its session; the sink's guards
        // settle any record it had open.
        let result = tokio::select! {
            biased;
            () = interrupt.as_mut() => {
                interrupted = true;
                break;
            }
            result = pass_run => result?,
        };
        if let Some(run) = &args.run {
            run.record(blit_core::job_log::EventBody::Phase {
                name: pass_phase,
                state: blit_core::job_log::PhaseState::End,
            })
            .await;
        }
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
        interrupted,
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
            raw_relative_path: None,
        }
    }

    fn failures(paths: &[&str]) -> PassFailures {
        PassFailures {
            files_failed: paths.len() as u64,
            failures: paths.iter().map(|p| failure(p)).collect(),
            failed_paths: paths.iter().map(|p| p.to_string()).collect(),
            failed_paths_truncated: false,
            left_in_place: Vec::new(),
            left_in_place_truncated: false,
            removed_incomplete: Vec::new(),
            removed_incomplete_truncated: false,
            unretried: 0,
        }
    }

    fn args(retries: u32) -> TransferArgs {
        let mut args = TransferArgs::for_tests("src", "dst");
        args.retries = retries;
        args.retry_wait = 0;
        args
    }

    /// cr-win-1: a retry pass compares like the main pass — the user's
    /// compare flags reach it unchanged.
    #[tokio::test]
    async fn a_retry_pass_keeps_the_users_compare() {
        let mut a = args(1);
        a.size_only = true;
        run_retry_passes(&a, failures(&["a"]), |pass_args| {
            assert!(pass_args.size_only && !pass_args.ignore_times && !pass_args.force);
            async move {
                Ok(PassResult {
                    files_transferred: 1,
                    bytes_transferred: 1,
                    failures: PassFailures::default(),
                    in_stream_carrier_used: false,
                    files_resumed: 0,
                })
            }
        })
        .await
        .expect("loop");
    }

    fn clean(files: u64) -> PassResult {
        PassResult {
            files_transferred: files,
            bytes_transferred: files * 5,
            failures: PassFailures::default(),
            in_stream_carrier_used: false,
            files_resumed: 0,
        }
    }

    /// cr-win-1 / cr-rework-1: under --ignore-existing, a path whose
    /// failure left this run's own incomplete copy at the destination
    /// retries with it off, in its own session; every other path keeps
    /// the user's flag, and the two results fold into one pass. The
    /// leftovers come from the exact set the destination sends — never
    /// the capped report, whose reasons here say nothing about them.
    #[tokio::test]
    async fn ignore_existing_retries_this_runs_own_leftovers_without_it() {
        let mut a = args(1);
        a.ignore_existing = true;
        let mut main = failures(&["left", "other"]);
        main.left_in_place = vec!["left".to_string()];
        let mut sessions: Vec<(Vec<PathBuf>, bool)> = Vec::new();
        let out = run_retry_passes(&a, main, |pass_args| {
            let mut set: Vec<PathBuf> = pass_args
                .retry_only
                .clone()
                .expect("retry set")
                .into_iter()
                .collect();
            set.sort();
            sessions.push((set, pass_args.ignore_existing));
            async move { Ok(clean(1)) }
        })
        .await
        .expect("loop");
        assert_eq!(
            sessions,
            vec![
                (vec![PathBuf::from("left")], false),
                (vec![PathBuf::from("other")], true),
            ]
        );
        assert_eq!(out.passes_run, 1);
        assert_eq!(out.added_files, 2);
        assert_eq!(out.final_failures.files_failed, 0);

        // Without --ignore-existing nothing is split.
        let mut main = failures(&["left", "other"]);
        main.left_in_place = vec!["left".to_string()];
        let mut calls = 0;
        run_retry_passes(&args(1), main, |_| {
            calls += 1;
            async move { Ok(clean(2)) }
        })
        .await
        .expect("loop");
        assert_eq!(calls, 1);
    }

    /// cr-rework-1: every route's summary hands the loop the exact
    /// left-in-place set and its truncation flag.
    #[test]
    fn every_route_carries_the_left_in_place_set() {
        let wire = TransferSummary {
            files_failed: 1,
            failed_paths: vec!["a".into()],
            left_in_place: vec!["a".into()],
            left_in_place_truncated: true,
            removed_incomplete: vec!["a".into()],
            removed_incomplete_truncated: true,
            ..TransferSummary::default()
        };
        let from_wire = PassFailures::from_summary(&wire);
        assert_eq!(from_wire.left_in_place, vec!["a".to_string()]);
        assert!(from_wire.left_in_place_truncated);
        assert_eq!(from_wire.removed_incomplete, vec!["a".to_string()]);
        assert!(from_wire.removed_incomplete_truncated);

        let local = LocalMirrorSummary {
            files_failed: 1,
            failed_paths: vec!["a".into()],
            left_in_place: vec!["a".into()],
            left_in_place_truncated: true,
            removed_incomplete: vec!["a".into()],
            removed_incomplete_truncated: true,
            ..LocalMirrorSummary::default()
        };
        let from_local = PassFailures::from_local(&local);
        assert_eq!(from_local.left_in_place, vec!["a".to_string()]);
        assert!(from_local.left_in_place_truncated);
        assert_eq!(from_local.removed_incomplete, vec!["a".to_string()]);
        assert!(from_local.removed_incomplete_truncated);

        let endpoint = blit_core::remote::RemoteEndpoint::parse("host:/m/").expect("endpoint");
        let delegated = DelegatedPullOutcome {
            summary: blit_core::remote::transfer::delegated_summary::delegated_summary_from_session(
                &wire,
                String::new(),
            ),
            src: endpoint.clone(),
            dst: endpoint,
        };
        let from_delegated = PassFailures::from_delegated(&delegated);
        assert_eq!(from_delegated.left_in_place, vec!["a".to_string()]);
        assert!(from_delegated.left_in_place_truncated);
        assert_eq!(from_delegated.removed_incomplete, vec!["a".to_string()]);
        assert!(from_delegated.removed_incomplete_truncated);
    }

    /// cr-rework-2: a leftover stays classified while its path keeps
    /// failing. The main pass leaves the copy; retry 1 fails on the source
    /// side, so its report names no leftover; retry 2 must still re-send
    /// the path with --ignore-existing off, or it would skip the copy still
    /// there and clear the failure.
    #[tokio::test]
    async fn a_leftover_stays_classified_through_a_source_side_retry_failure() {
        let mut a = args(2);
        a.ignore_existing = true;
        let mut main = failures(&["left"]);
        main.left_in_place = vec!["left".to_string()];
        let mut sessions: Vec<bool> = Vec::new();
        let out = run_retry_passes(&a, main, |pass_args| {
            sessions.push(pass_args.ignore_existing);
            let pass = sessions.len();
            async move {
                if pass == 1 {
                    Ok(PassResult {
                        files_transferred: 0,
                        bytes_transferred: 0,
                        failures: failures(&["left"]),
                        in_stream_carrier_used: false,
                        files_resumed: 0,
                    })
                } else {
                    Ok(clean(1))
                }
            }
        })
        .await
        .expect("loop");
        assert_eq!(
            sessions,
            vec![false, false],
            "both retries re-send the leftover without --ignore-existing"
        );
        assert_eq!(out.passes_run, 2);
        assert_eq!(out.final_failures.files_failed, 0);
    }

    /// cr-rework-3: once a retry positively removed this run's leftover,
    /// the path is an ordinary failure. Retry 1 touches the copy, fails on
    /// the source side and removes it; retry 2 runs the path WITH
    /// --ignore-existing, so a file another process created there during
    /// the wait is kept, not overwritten.
    #[tokio::test]
    async fn a_leftover_the_retry_removed_is_no_longer_classified() {
        let mut a = args(2);
        a.ignore_existing = true;
        let mut main = failures(&["left"]);
        main.left_in_place = vec!["left".to_string()];
        let mut sessions: Vec<bool> = Vec::new();
        run_retry_passes(&a, main, |pass_args| {
            sessions.push(pass_args.ignore_existing);
            let pass = sessions.len();
            async move {
                if pass == 1 {
                    let mut failed = failures(&["left"]);
                    failed.removed_incomplete = vec!["left".to_string()];
                    Ok(PassResult {
                        files_transferred: 0,
                        bytes_transferred: 0,
                        failures: failed,
                        in_stream_carrier_used: false,
                        files_resumed: 0,
                    })
                } else {
                    Ok(clean(0))
                }
            }
        })
        .await
        .expect("loop");
        assert_eq!(
            sessions,
            vec![false, true],
            "retry 2 keeps --ignore-existing once the copy is gone"
        );
    }

    /// cr-rework-3: if the removed set did not fit, a prior leftover it
    /// does not name may or may not still be there — retried under
    /// --ignore-existing it could be a skipped leftover cleared to exit 0,
    /// retried without it a newly created file overwritten — so it is not
    /// retried at all and stays failed.
    #[tokio::test]
    async fn an_ambiguous_leftover_is_not_retried_under_ignore_existing() {
        let mut a = args(2);
        a.ignore_existing = true;
        let mut main = failures(&["left"]);
        main.left_in_place = vec!["left".to_string()];
        let mut sessions: Vec<bool> = Vec::new();
        let out = run_retry_passes(&a, main, |pass_args| {
            sessions.push(pass_args.ignore_existing);
            async move {
                let mut failed = failures(&["left"]);
                failed.removed_incomplete_truncated = true;
                Ok(PassResult {
                    files_transferred: 0,
                    bytes_transferred: 0,
                    failures: failed,
                    in_stream_carrier_used: false,
                    files_resumed: 0,
                })
            }
        })
        .await
        .expect("loop");
        assert_eq!(
            sessions,
            vec![false],
            "no second pass for the ambiguous path"
        );
        assert_eq!(out.final_failures.files_failed, 1, "it stays failed");
    }

    /// cr-rework-1: more leftovers than the 64-entry named report holds —
    /// the 65th is still recognised, because the set is exact.
    #[tokio::test]
    async fn leftovers_past_the_named_report_cap_are_still_recognised() {
        let names: Vec<String> = (0..70).map(|i| format!("f{i:02}")).collect();
        let refs: Vec<&str> = names.iter().map(String::as_str).collect();
        let mut main = failures(&refs);
        main.failures.truncate(64);
        main.left_in_place = names.clone();
        let mut a = args(1);
        a.ignore_existing = true;
        let mut sessions: Vec<(usize, bool)> = Vec::new();
        run_retry_passes(&a, main, |pass_args| {
            let n = pass_args.retry_only.as_ref().expect("retry set").len();
            sessions.push((n, pass_args.ignore_existing));
            async move { Ok(clean(n as u64)) }
        })
        .await
        .expect("loop");
        assert_eq!(
            sessions,
            vec![(70, false)],
            "all 70 leftovers retry without --ignore-existing"
        );
    }

    /// cr-rework-1: when even the exact set did not fit, the paths it does
    /// not name cannot be classified, so none of them is retried under
    /// --ignore-existing: the known leftovers retry, the rest stay failed
    /// and reported as not retried — never cleared by a skip.
    #[tokio::test]
    async fn an_unclassifiable_remainder_is_not_retried_under_ignore_existing() {
        let mut main = failures(&["known", "u1", "u2"]);
        main.left_in_place = vec!["known".to_string()];
        main.left_in_place_truncated = true;
        let mut a = args(1);
        a.ignore_existing = true;
        let mut sessions: Vec<(Vec<PathBuf>, bool)> = Vec::new();
        let out = run_retry_passes(&a, main, |pass_args| {
            let mut set: Vec<PathBuf> = pass_args
                .retry_only
                .clone()
                .expect("retry set")
                .into_iter()
                .collect();
            set.sort();
            sessions.push((set, pass_args.ignore_existing));
            async move { Ok(clean(1)) }
        })
        .await
        .expect("loop");
        assert_eq!(sessions, vec![(vec![PathBuf::from("known")], false)]);
        assert_eq!(
            out.final_failures.files_failed, 2,
            "the unclassified two stay failed"
        );
        assert!(out
            .final_failures
            .failures
            .iter()
            .any(|f| f.reason.starts_with("2 file(s) were not retried")));
    }

    /// 2026-10-07 defect (b): Ctrl-C during the retry wait stops the
    /// retries before any pass; the first pass's failures stand. The wait
    /// is an hour, so only an interrupt that cuts the wait itself short
    /// returns in time.
    #[tokio::test]
    async fn an_interrupt_during_the_wait_stops_the_retries() {
        let mut a = args(2);
        a.retry_wait = 3600;
        let out = tokio::time::timeout(
            std::time::Duration::from_secs(10),
            run_retry_passes_until(
                &a,
                failures(&["a"]),
                |_| async { panic!("no retry pass may start once interrupted") },
                std::future::ready(()),
            ),
        )
        .await
        .expect("the interrupt cuts the hour-long wait short")
        .expect("loop");
        assert!(out.interrupted);
        assert_eq!(out.passes_run, 0);
        assert_eq!(out.final_failures.files_failed, 1);
        assert_eq!(out.final_failures.failures[0].relative_path, "a");
    }

    /// 2026-10-07 defect (b): Ctrl-C during a retry pass cuts it short;
    /// the report is the state before that pass.
    #[tokio::test]
    async fn an_interrupt_during_a_pass_stops_the_retries() {
        let out = run_retry_passes_until(
            &args(2),
            failures(&["a", "b"]),
            |_| async { std::future::pending::<Result<PassResult>>().await },
            tokio::time::sleep(std::time::Duration::from_millis(20)),
        )
        .await
        .expect("loop");
        assert!(out.interrupted);
        assert_eq!(out.passes_run, 0, "the cut-short pass does not count");
        assert_eq!(out.final_failures.files_failed, 2);
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

/// cr-fix2-2: a pass whose failure count exceeds the paths it names has
/// failures no retry can be given; they are carried as unretried and
/// survive a later clean pass, and the move gate refuses on them.
#[cfg(test)]
mod cr_fix2_2_tests {
    use super::*;

    #[tokio::test]
    async fn counted_but_unnamed_failures_survive_a_clean_retry_pass() {
        // The shape an older sender produced: one named path, three
        // counted failures, and the set NOT flagged truncated because
        // the one name fit the wire budget.
        let main = PassFailures {
            files_failed: 3,
            failures: vec![FileFailure {
                relative_path: "a".to_string(),
                reason: "source: missing at retry".to_string(),
                raw_relative_path: None,
            }],
            failed_paths: vec!["a".to_string()],
            failed_paths_truncated: false,
            left_in_place: Vec::new(),
            left_in_place_truncated: false,
            removed_incomplete: Vec::new(),
            removed_incomplete_truncated: false,
            unretried: 0,
        };
        let mut args = TransferArgs::for_tests("src", "dst");
        args.retries = 2;
        args.retry_wait = 0;
        let mut given: Vec<usize> = Vec::new();
        let out = run_retry_passes(&args, main, |pass_args| {
            given.push(pass_args.retry_only.as_ref().map_or(0, |s| s.len()));
            async move {
                // The named file converges; nothing else was given.
                Ok(PassResult {
                    files_transferred: 1,
                    bytes_transferred: 0,
                    failures: PassFailures::default(),
                    in_stream_carrier_used: false,
                    files_resumed: 0,
                })
            }
        })
        .await
        .expect("loop");
        assert_eq!(given, vec![1], "only the named path was retried, once");
        assert_eq!(
            out.final_failures.unretried, 2,
            "the two unnamed failures were never retried: {:?}",
            out.final_failures
        );
        assert_eq!(out.final_failures.files_failed, 2);
        assert!(
            out.final_failures
                .failures
                .iter()
                .any(|f| f.relative_path == UNRETRIED_PATH
                    && f.reason.starts_with("2 file(s) were not retried")),
            "{:?}",
            out.final_failures.failures
        );
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
}
