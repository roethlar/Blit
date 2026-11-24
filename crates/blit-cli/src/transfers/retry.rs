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

/// What a pass is limited to (cr-ecm13-3): paths named by their text,
/// and — for names that are not text — by their exact bytes, `/` between
/// names. A name that is not text was asked for by the text nearest it,
/// which is another name: the file that failed was never retried, the one
/// that bears the other name was sent again, and the run ended well.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct RetrySet {
    pub text: HashSet<PathBuf>,
    pub raw: HashSet<Vec<u8>>,
}

impl RetrySet {
    fn len(&self) -> usize {
        self.text.len() + self.raw.len()
    }

    fn is_empty(&self) -> bool {
        self.text.is_empty() && self.raw.is_empty()
    }

    fn intersection(&self, other: &RetrySet) -> RetrySet {
        RetrySet {
            text: self.text.intersection(&other.text).cloned().collect(),
            raw: self.raw.intersection(&other.raw).cloned().collect(),
        }
    }

    fn difference(&self, other: &RetrySet) -> RetrySet {
        RetrySet {
            text: self.text.difference(&other.text).cloned().collect(),
            raw: self.raw.difference(&other.raw).cloned().collect(),
        }
    }

    /// Limit a pass to this set.
    fn limit(self, args: &mut TransferArgs) {
        args.retry_only = Some(self.text);
        args.retry_only_raw = (!self.raw.is_empty()).then_some(self.raw);
    }
}

/// `raw` at the places of `paths`: itself when it has one entry for every
/// path, and nothing at every place otherwise.
fn at_their_places(paths: &[String], raw: &[Vec<u8>]) -> Vec<Vec<u8>> {
    if raw.len() == paths.len() {
        raw.to_vec()
    } else {
        vec![Vec::new(); paths.len()]
    }
}

/// The list as it goes back into a summary: none when no path has bytes.
fn where_any(raw: &[Vec<u8>]) -> Vec<Vec<u8>> {
    if raw.iter().all(Vec::is_empty) {
        Vec::new()
    } else {
        raw.to_vec()
    }
}

/// The failure report of one pass, in the one shape every route
/// produces: the exact count, the named (capped) report, and the exact
/// path set the next pass retries.
#[derive(Clone, Debug, Default)]
pub(crate) struct PassFailures {
    pub files_failed: u64,
    pub failures: Vec<FileFailure>,
    pub failed_paths: Vec<String>,
    pub failed_paths_truncated: bool,
    /// cr-ecm13-3: the exact bytes of each of `failed_paths` whose name is
    /// not text, at its place, and nothing at the place of one that is.
    /// Always one entry for every path.
    pub failed_paths_raw: Vec<Vec<u8>>,
    /// The same for `left_in_place`: the names that are not text among
    /// those, by their bytes (a list of its own, as the wire has it).
    pub left_in_place_raw: Vec<Vec<u8>>,
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
    /// D-2026-11-24-1: the report's failures were marked as ones that
    /// survived a retry — said here, never read back from their reasons,
    /// whose words may be anything, a file's name among them.
    pub retried: bool,
}

impl PassFailures {
    pub(crate) fn from_summary(summary: &TransferSummary) -> Self {
        Self {
            files_failed: summary.files_failed,
            failures: super::failures::failures_of(summary),
            failed_paths: summary.failed_paths.clone(),
            failed_paths_truncated: summary.failed_paths_truncated,
            failed_paths_raw: at_their_places(&summary.failed_paths, &summary.failed_paths_raw),
            left_in_place_raw: summary.left_in_place_raw.clone(),
            left_in_place: summary.left_in_place.clone(),
            left_in_place_truncated: summary.left_in_place_truncated,
            removed_incomplete: summary.removed_incomplete.clone(),
            removed_incomplete_truncated: summary.removed_incomplete_truncated,
            unretried: 0,
            retried: false,
        }
    }

    pub(crate) fn from_local(summary: &LocalMirrorSummary) -> Self {
        Self {
            files_failed: summary.files_failed,
            failures: summary.failures.clone(),
            failed_paths: summary.failed_paths.clone(),
            failed_paths_truncated: summary.failed_paths_truncated,
            failed_paths_raw: at_their_places(&summary.failed_paths, &summary.failed_paths_raw),
            left_in_place_raw: summary.left_in_place_raw.clone(),
            left_in_place: summary.left_in_place.clone(),
            left_in_place_truncated: summary.left_in_place_truncated,
            removed_incomplete: summary.removed_incomplete.clone(),
            removed_incomplete_truncated: summary.removed_incomplete_truncated,
            unretried: 0,
            retried: false,
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
            failed_paths_raw: at_their_places(paths, outcome.failed_paths_raw()),
            left_in_place_raw: outcome.left_in_place_raw().to_vec(),
            left_in_place: left.to_vec(),
            left_in_place_truncated: left_truncated,
            removed_incomplete: removed.to_vec(),
            removed_incomplete_truncated: removed_truncated,
            unretried: 0,
            retried: false,
        }
    }

    /// The set the next pass is limited to. The exact set when the
    /// sender could carry it; otherwise the named report's paths, with
    /// `truncated` telling the caller to say so.
    fn retry_set(&self) -> (RetrySet, bool) {
        // cr-ecm13-3: each path by its text, or — where its name is not
        // text — by its bytes and by nothing else.
        let exact = || {
            let mut set = RetrySet::default();
            for (at, path) in self.failed_paths.iter().enumerate() {
                match self.failed_paths_raw.get(at) {
                    Some(raw) if !raw.is_empty() => {
                        set.raw.insert(raw.clone());
                    }
                    _ => {
                        set.text.insert(PathBuf::from(path));
                    }
                }
            }
            set
        };
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
            return (exact(), false);
        }
        if !self.failed_paths_truncated && !self.failed_paths.is_empty() {
            return (exact(), true);
        }
        let truncated =
            self.failed_paths_truncated || (self.files_failed as usize) > self.failures.len();
        // cr-ecm37-5: a list cut for size still names exactly every file it
        // carries — with the report's names, that is what the pass is given.
        // Given the report's alone, 260 long names, each report entry's path
        // shortened, retried none of the 225 the list carried.
        let mut named = exact();
        // cr-ecm36-3: a path the report shortened names no file to retry.
        for failure in self
            .failures
            .iter()
            .filter(|failure| !failure.path_shortened)
        {
            match &failure.raw_relative_path {
                Some(raw) => {
                    named.raw.insert(raw.clone());
                }
                None => {
                    named.text.insert(PathBuf::from(&failure.relative_path));
                }
            }
        }
        (named, truncated)
    }

    /// cr-win-1 / cr-rework-1: the failed paths whose failure left this
    /// run's own incomplete copy at the destination — the exact set the
    /// destination sends beside `failed_paths`, never the capped report.
    fn left_in_place(&self) -> RetrySet {
        RetrySet {
            text: self.left_in_place.iter().map(PathBuf::from).collect(),
            raw: self.left_in_place_raw.iter().cloned().collect(),
        }
    }

    /// cr-ecm37-3: the failed files whose failure may or may not have left
    /// this run's copy — the destination could not be looked at — by the
    /// report's flag. One whose path the report shortened names no file;
    /// the destination marks the account inexact for it.
    fn left_unknown(&self) -> RetrySet {
        let mut set = RetrySet::default();
        for failure in self
            .failures
            .iter()
            .filter(|failure| failure.left_unknown && !failure.path_shortened)
        {
            match &failure.raw_relative_path {
                Some(raw) => {
                    set.raw.insert(raw.clone());
                }
                None => {
                    set.text.insert(PathBuf::from(&failure.relative_path));
                }
            }
        }
        set
    }

    /// The set the next pass is given, the leftovers among it, and whether
    /// it is short of the failures (see the retry loop). Under
    /// `--ignore-existing`, a failure whose leftover cannot be classified is
    /// not given: every one past a cut leftover list, and (cr-ecm37-3) each
    /// one the destination could not tell — retried with the flag its
    /// partial would be kept as a file the user put there, retried as a
    /// leftover a file put there since would be overwritten. They stay
    /// reported as not retried.
    fn pass_set(&self, ignore_existing: bool) -> (RetrySet, RetrySet, bool) {
        let (set, truncated) = self.retry_set();
        if !ignore_existing {
            return (set, RetrySet::default(), truncated);
        }
        let left = self.left_in_place().intersection(&set);
        if self.left_in_place_truncated {
            return (left.clone(), left, true);
        }
        let unknown = self.left_unknown().intersection(&set);
        if unknown.is_empty() {
            (set, left, truncated)
        } else {
            (set.difference(&unknown), left, true)
        }
    }

    /// Fold a second session of the same pass into this report.
    fn merge(&mut self, other: PassFailures) {
        self.files_failed = self.files_failed.saturating_add(other.files_failed);
        self.failures.extend(other.failures);
        self.failed_paths.extend(other.failed_paths);
        self.failed_paths_raw.extend(other.failed_paths_raw);
        self.left_in_place_raw.extend(other.left_in_place_raw);
        self.failed_paths_truncated |= other.failed_paths_truncated;
        self.left_in_place.extend(other.left_in_place);
        self.left_in_place_truncated |= other.left_in_place_truncated;
        self.removed_incomplete.extend(other.removed_incomplete);
        self.removed_incomplete_truncated |= other.removed_incomplete_truncated;
        self.unretried = self.unretried.saturating_add(other.unretried);
    }

    /// Mark every remaining failure as one that survived a retry — once:
    /// the report says whether it was marked (D-2026-11-24-1), not its
    /// reasons' words. The ones no pass could reach stay a count,
    /// `unretried`, which the report renders as its own line: never a
    /// failure of a made-up name, which a real file can have (cr-ecm34-7).
    fn mark_retried(&mut self) {
        if std::mem::replace(&mut self.retried, true) {
            return;
        }
        for failure in &mut self.failures {
            failure.reason.push_str(RETRIED_SUFFIX);
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
        // cr-ecm13-3: the same for the names that are not text, each by
        // its bytes. (What a pass removed is listed by text alone.)
        let still_failing_raw: HashSet<&[u8]> = next
            .failed_paths_raw
            .iter()
            .filter(|raw| !raw.is_empty())
            .map(Vec::as_slice)
            .collect();
        let mut left_raw: HashSet<Vec<u8>> = std::mem::take(&mut next.left_in_place_raw)
            .into_iter()
            .collect();
        let carried_raw: Vec<&Vec<u8>> = self
            .left_in_place_raw
            .iter()
            .filter(|raw| still_failing_raw.contains(raw.as_slice()))
            .filter(|raw| !removed.contains(String::from_utf8_lossy(raw).as_ref()))
            .filter(|raw| !left_raw.contains(*raw))
            .collect();
        let ambiguous =
            next.removed_incomplete_truncated && (!carried.is_empty() || !carried_raw.is_empty());
        if !ambiguous {
            left.extend(carried.into_iter().cloned());
            left_raw.extend(carried_raw.into_iter().cloned());
        }
        let mut left_raw: Vec<Vec<u8>> = left_raw.into_iter().collect();
        left_raw.sort();
        next.left_in_place_raw = left_raw;
        let mut left: Vec<String> = left.into_iter().collect();
        left.sort();
        next.left_in_place = left;
        next.left_in_place_truncated |= self.left_in_place_truncated || ambiguous;
        next
    }

    /// Write the final state back onto a wire-shaped summary.
    pub(crate) fn apply_to_summary(&self, summary: &mut TransferSummary) {
        summary.files_failed = self.files_failed;
        // cr-ecm37-5: each entry linked to its exact name in the list.
        summary.failures = blit_core::remote::transfer::sink::failures_to_report(
            &self.failures,
            &self.failed_paths,
            &where_any(&self.failed_paths_raw),
        );
        summary.failed_paths = self.failed_paths.clone();
        summary.failed_paths_truncated = self.failed_paths_truncated;
        summary.failed_paths_raw = where_any(&self.failed_paths_raw);
        summary.left_in_place_raw = self.left_in_place_raw.clone();
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
        summary.failed_paths_raw = where_any(&self.failed_paths_raw);
        summary.left_in_place_raw = self.left_in_place_raw.clone();
        summary.left_in_place = self.left_in_place.clone();
        summary.left_in_place_truncated = self.left_in_place_truncated;
        summary.removed_incomplete = self.removed_incomplete.clone();
        summary.removed_incomplete_truncated = self.removed_incomplete_truncated;
    }
}

/// Appended to the reason of a file that failed again on a retry pass.
pub(crate) const RETRIED_SUFFIX: &str = " (retried)";

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
        // cr-ecm38-6: linked, as every report is (cr-ecm37-5). Unlinked, a
        // long name's reason was lost on reading the summary back.
        summary.failures = blit_core::remote::transfer::sink::failures_to_report(
            &self.final_failures.failures,
            &self.final_failures.failed_paths,
            &where_any(&self.final_failures.failed_paths_raw),
        );
        summary.failed_paths = self.final_failures.failed_paths.clone();
        summary.failed_paths_truncated = self.final_failures.failed_paths_truncated;
        summary.failed_paths_raw = where_any(&self.final_failures.failed_paths_raw);
        summary.left_in_place_raw = self.final_failures.left_in_place_raw.clone();
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
        // cr-win-1: a retry pass compares exactly as the main pass did —
        // a failed write never leaves a target that looks finished (the
        // sink removes it or holds it one byte off the source's size), so
        // the user's compare is trusted here as on any later run. The one
        // exception is --ignore-existing: a path whose failure left this
        // run's own incomplete copy at the destination exists there now,
        // and that copy is not one the user asked to keep — those paths
        // retry apart, as such (cr-ecm13-1: the end that writes copies to
        // one only if nothing stands there by then, and keeps and
        // reports a file that does). cr-rework-1: they come from the exact set the
        // destination sends beside the failed paths, never the capped
        // report; if even that set did not fit, the paths it does not
        // name cannot be classified — retried with the flag a leftover is
        // skipped and its failure cleared, retried without it a file the
        // user asked to keep is overwritten — so none of them is retried:
        // they stay reported as not retried.
        let (set, left, truncated) = current.pass_set(args.ignore_existing);
        if set.is_empty() {
            // cr-ecm35-7: no pass can start — no failed file could be named
            // for it (a name over the wire's budget, say) — so none of them
            // is retried, and the report says so by its count. Left, the
            // run counted the failures and said nothing of the retry it
            // could not make.
            if passes_run == 0 {
                current.unretried = current.files_failed;
            }
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
                set.limit(&mut pass_args);
                Box::pin(run(pass_args)).await
            } else {
                let rest = set.difference(&left);
                let mut left_args = pass_args.clone();
                left.limit(&mut left_args);
                // cr-ecm12-2: said to be this run's own copies, with the
                // option still on. Run with it off, the pass replaced
                // whatever stood at these paths by then.
                left_args.retry_left = true;
                let mut result = Box::pin(run(left_args)).await?;
                if !rest.is_empty() {
                    rest.limit(&mut pass_args);
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

    /// cr-ecm38-6: a delegated retry's report is linked to its exact names
    /// like any other, so a long name read back is that name, with its
    /// reason. Unlinked, the reason was lost on decoding.
    #[test]
    fn a_delegated_retry_links_its_report() {
        let long = format!("{}/name.bin", "d".repeat(4500));
        let out = RetryOutcome {
            added_files: 0,
            added_bytes: 0,
            final_failures: PassFailures {
                files_failed: 1,
                failures: vec![FileFailure {
                    relative_path: long.clone(),
                    reason: "source: missing".into(),
                    ..FileFailure::default()
                }],
                failed_paths: vec![long.clone()],
                failed_paths_raw: vec![Vec::new()],
                ..PassFailures::default()
            },
            passes_run: 1,
            in_stream_carrier_used: false,
            files_resumed: 0,
            interrupted: false,
        };
        let mut delegated = blit_core::generated::DelegatedPullSummary::default();
        out.fold_into_delegated(&mut delegated);
        assert_eq!(delegated.failures[0].failed_path_at, 1);
        let read =
            blit_core::remote::transfer::delegated_summary::delegated_summary_failures(&delegated);
        assert_eq!(read[0].relative_path, long);
        assert_eq!(read[0].reason, "source: missing");
    }

    /// cr-ecm37-5: a failed-path list cut for size still names exactly every
    /// file it carries, and the pass is given them all; given the report's
    /// names alone, whose paths were shortened, it was given none.
    #[test]
    fn a_cut_list_gives_the_pass_every_name_it_carries() {
        let report = PassFailures {
            files_failed: 3,
            failures: vec![FileFailure {
                relative_path: "[truncated]…c".into(),
                reason: "r".into(),
                path_shortened: true,
                ..FileFailure::default()
            }],
            failed_paths: vec!["a".into(), "b".into()],
            failed_paths_raw: vec![Vec::new(), Vec::new()],
            failed_paths_truncated: true,
            ..PassFailures::default()
        };
        let (set, truncated) = report.retry_set();
        assert_eq!(set.text, [PathBuf::from("a"), PathBuf::from("b")].into());
        assert!(truncated);
    }

    /// cr-ecm37-5: the report written back onto the run's summary links each
    /// entry to its exact name in the list, so a reader of that summary has
    /// the name a shortened path stands for.
    #[test]
    fn the_final_summary_links_its_report() {
        let long = format!("{}/name.bin", "d".repeat(4600));
        let report = PassFailures {
            files_failed: 1,
            failures: vec![FileFailure {
                relative_path: long.clone(),
                reason: "r".into(),
                ..FileFailure::default()
            }],
            failed_paths: vec![long],
            failed_paths_raw: vec![Vec::new()],
            ..PassFailures::default()
        };
        let mut summary = TransferSummary::default();
        report.apply_to_summary(&mut summary);
        assert!(summary.failures[0].path_shortened);
        assert_eq!(summary.failures[0].failed_path_at, 1);
    }

    /// cr-ecm37-3: under --ignore-existing a failure that may or may not
    /// have left this run's copy is not retried — kept, the copy would stand
    /// as a file the user put there; overwritten, a file put there since
    /// would be lost — and the pass says it was not given all. Without the
    /// flag it is retried like any other.
    #[test]
    fn an_unknown_leftover_is_not_retried_under_ignore_existing() {
        let failure = |path: &str, unknown: bool| FileFailure {
            relative_path: path.into(),
            reason: "r".into(),
            left_unknown: unknown,
            ..FileFailure::default()
        };
        let report = PassFailures {
            files_failed: 2,
            failures: vec![failure("u.bin", true), failure("k.bin", false)],
            failed_paths: vec!["u.bin".into(), "k.bin".into()],
            failed_paths_raw: vec![Vec::new(), Vec::new()],
            ..PassFailures::default()
        };
        let (set, _, truncated) = report.pass_set(true);
        assert_eq!(set.text, [PathBuf::from("k.bin")].into());
        assert!(truncated);
        let (set, _, truncated) = report.pass_set(false);
        assert_eq!(set.len(), 2);
        assert!(!truncated);
    }

    /// cr-ecm12-2: whether a pass is one of plain `--ignore-existing` —
    /// `false` for the pass of this run's own incomplete copies, which
    /// says what they are (`retry_left`) so that a file standing at one
    /// of them is reported as well as kept (cr-ecm13-1). The option
    /// itself is never turned off: these passes ran without it, and
    /// replaced whatever stood at their paths.
    fn keeps_every_file(pass: &TransferArgs) -> bool {
        assert!(pass.ignore_existing, "the option stays on in every pass");
        !pass.retry_left
    }

    fn failure(path: &str) -> FileFailure {
        FileFailure {
            relative_path: path.to_string(),
            reason: "source: cannot open: boom".to_string(),
            raw_relative_path: None,
            left_in_place: false,
            path_shortened: false,
            left_unknown: false,
        }
    }

    fn failures(paths: &[&str]) -> PassFailures {
        PassFailures {
            files_failed: paths.len() as u64,
            failures: paths.iter().map(|p| failure(p)).collect(),
            failed_paths: paths.iter().map(|p| p.to_string()).collect(),
            failed_paths_truncated: false,
            failed_paths_raw: vec![Vec::new(); paths.len()],
            left_in_place_raw: Vec::new(),
            left_in_place: Vec::new(),
            left_in_place_truncated: false,
            removed_incomplete: Vec::new(),
            removed_incomplete_truncated: false,
            unretried: 0,
            retried: false,
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

    /// cr-ecm13-3: a failed name that is not text is asked for by its
    /// bytes, and by nothing else — the text it is held under is another
    /// name, and asking for that sent another file. From the exact set,
    /// from the named report where the set did not fit, and under
    /// `--ignore-existing` in the pass its own incomplete copy belongs to.
    #[tokio::test]
    async fn a_name_that_is_not_text_is_retried_by_its_bytes() {
        type Asked = Vec<(Vec<PathBuf>, Vec<Vec<u8>>, bool)>;
        async fn passes(a: &TransferArgs, main: PassFailures) -> Asked {
            let mut asked: Asked = Vec::new();
            run_retry_passes(a, main, |pass_args| {
                let mut text: Vec<PathBuf> = pass_args
                    .retry_only
                    .clone()
                    .expect("retry set")
                    .into_iter()
                    .collect();
                text.sort();
                let raw: Vec<Vec<u8>> = pass_args
                    .retry_only_raw
                    .clone()
                    .unwrap_or_default()
                    .into_iter()
                    .collect();
                asked.push((text, raw, pass_args.retry_left));
                async move { Ok(clean(1)) }
            })
            .await
            .expect("loop");
            asked
        }
        let twins = || {
            let mut main = failures(&["plain", "dup\u{fffd}"]);
            main.failed_paths_raw = vec![Vec::new(), b"dup\xff".to_vec()];
            main.failures[1].raw_relative_path = Some(b"dup\xff".to_vec());
            main
        };
        let both = vec![(
            vec![PathBuf::from("plain")],
            vec![b"dup\xff".to_vec()],
            false,
        )];

        // The exact set.
        assert_eq!(passes(&args(1), twins()).await, both);
        // The named report, where the set did not fit.
        let mut capped = twins();
        capped.failed_paths.clear();
        capped.failed_paths_raw.clear();
        capped.failed_paths_truncated = true;
        assert_eq!(passes(&args(1), capped).await, both);
        // Its own incomplete copy, under --ignore-existing: apart, by bytes.
        let mut keeping = args(1);
        keeping.ignore_existing = true;
        let mut left = twins();
        left.left_in_place_raw = vec![b"dup\xff".to_vec()];
        assert_eq!(
            passes(&keeping, left).await,
            vec![
                (Vec::new(), vec![b"dup\xff".to_vec()], true),
                (vec![PathBuf::from("plain")], Vec::new(), false),
            ]
        );

        // And what a pass leaves goes back with its bytes.
        let mut again = failures(&["dup\u{fffd}"]);
        again.failed_paths_raw = vec![b"dup\xff".to_vec()];
        let out = run_retry_passes(&args(1), twins(), |_| {
            let again = again.clone();
            async move {
                Ok(PassResult {
                    files_transferred: 1,
                    bytes_transferred: 1,
                    failures: again,
                    in_stream_carrier_used: false,
                    files_resumed: 0,
                })
            }
        })
        .await
        .expect("loop");
        let mut summary = TransferSummary::default();
        out.final_failures.apply_to_summary(&mut summary);
        assert_eq!(summary.failed_paths, ["dup\u{fffd}"]);
        assert_eq!(summary.failed_paths_raw, [b"dup\xff".to_vec()]);
    }

    /// cr-win-1 / cr-rework-1: under --ignore-existing, a path whose
    /// failure left this run's own incomplete copy at the destination
    /// retries with it off, in its own session; every other path keeps
    /// the user's flag, and the two results fold into one pass. The
    /// leftovers come from the exact set the destination sends — never
    /// the capped report, whose reasons here say nothing about them.
    #[tokio::test]
    async fn ignore_existing_retries_this_runs_own_leftovers_as_such() {
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
            sessions.push((set, keeps_every_file(&pass_args)));
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
            sessions.push(keeps_every_file(&pass_args));
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
            "both retries re-send the leftover as this run's own copy"
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
            sessions.push(keeps_every_file(&pass_args));
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
            "retry 2 no longer lists it as the run's own once the copy is gone"
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
            sessions.push(keeps_every_file(&pass_args));
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
            sessions.push((n, keeps_every_file(&pass_args)));
            async move { Ok(clean(n as u64)) }
        })
        .await
        .expect("loop");
        assert_eq!(
            sessions,
            vec![(70, false)],
            "all 70 leftovers retry as the run's own copies"
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
            sessions.push((set, keeps_every_file(&pass_args)));
            async move { Ok(clean(1)) }
        })
        .await
        .expect("loop");
        assert_eq!(sessions, vec![(vec![PathBuf::from("known")], false)]);
        assert_eq!(
            out.final_failures.files_failed, 2,
            "the unclassified two stay failed"
        );
        assert_eq!(
            out.final_failures.unretried, 2,
            "reported as not retried, by count (cr-ecm34-7)"
        );
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

    /// D-2026-11-24-1: a failure that survived a retry is marked so by the
    /// report's flag, never by its words: one whose reason already ends as
    /// a marked one does — its file's name ends so — is marked all the
    /// same, and a report is marked once however often it is asked.
    #[tokio::test]
    async fn a_retried_failure_is_marked_by_the_report_not_its_words() {
        let name = format!("a{RETRIED_SUFFIX}");
        let out = run_retry_passes(&args(1), failures(&[name.as_str()]), |_| {
            let name = name.clone();
            async move {
                let mut failed = failures(&[name.as_str()]);
                failed.failures[0].reason = format!("source: cannot open {name}");
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
        let mut report = out.final_failures;
        assert!(report.retried);
        let marked = format!("source: cannot open {name}{RETRIED_SUFFIX}");
        assert_eq!(report.failures[0].reason, marked);
        report.mark_retried();
        assert_eq!(report.failures[0].reason, marked, "marked once");
    }

    /// cr-ecm35-7: a retry that cannot start — its one failed file named
    /// nowhere the retry can read — runs no pass, and counts that file as
    /// not retried. It was counted failed, and not retried by nothing.
    #[tokio::test]
    async fn a_retry_that_cannot_start_counts_its_failures_as_not_retried() {
        let mut main = PassFailures {
            files_failed: 1,
            failed_paths_truncated: true,
            ..PassFailures::default()
        };
        main.unretried = 0;
        let out = run_retry_passes(&args(1), main, |_| async {
            unreachable!("no pass can start without a name")
        })
        .await
        .expect("loop");
        assert_eq!(out.passes_run, 0);
        assert_eq!(out.final_failures.files_failed, 1);
        assert_eq!(out.final_failures.unretried, 1, "{:?}", out.final_failures);
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
        assert!(!out.final_failures.retried);
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
        // never retried, so they are still failed — counted, and enough for
        // the move gate to refuse. cr-ecm34-7: a count, not a failure of a
        // made-up name.
        assert_eq!(out.final_failures.files_failed, 3);
        assert_eq!(out.final_failures.unretried, 3);
        assert!(
            out.final_failures.failures.is_empty(),
            "no synthesized entry: {:?}",
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
            left_in_place_raw: Vec::new(),
            failed_paths_raw: Vec::new(),
            files_failed: 3,
            failures: vec![FileFailure {
                relative_path: "a".to_string(),
                reason: "source: missing at retry".to_string(),
                raw_relative_path: None,
                left_in_place: false,
                path_shortened: false,
                left_unknown: false,
            }],
            failed_paths: vec!["a".to_string()],
            failed_paths_truncated: false,
            left_in_place: Vec::new(),
            left_in_place_truncated: false,
            removed_incomplete: Vec::new(),
            removed_incomplete_truncated: false,
            unretried: 0,
            retried: false,
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
            out.final_failures.failures.is_empty(),
            "a count, not a failure of a made-up name (cr-ecm34-7): {:?}",
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
