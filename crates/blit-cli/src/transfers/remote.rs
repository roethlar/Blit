use crate::cli::TransferArgs;
use eyre::Result;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};
use tokio::sync::mpsc;
use tokio::task::JoinHandle;
use tokio::time::{interval, MissedTickBehavior};

use blit_core::remote::transfer::{
    ProgressEvent, ProgressTotals, RemoteTransferProgress, TransferLifecycleTrace,
};
use blit_core::remote::RemoteEndpoint;
use blit_core::transfers::compare::{comparison_mode, move_comparison_mode, CompareFlags};
use blit_core::transfers::remote::{
    run_remote_pull, run_remote_push, PullExecution, PullVerbOutcome, PushExecution,
};

use blit_core::endpoints::format_remote_endpoint;

/// CLI-facing alias for the library's pull-outcome struct — since
/// otp-10b-2 the session verb outcome (`summary` + `dest_root`); the
/// public name `DeferredPullState` that `transfers::mod` imports is
/// preserved across the retype.
pub type DeferredPullState = PullVerbOutcome;

/// True for the events that actually move payload bytes. Per the
/// `ProgressEvent` contract, bytes ride `Payload` only — `FileComplete`
/// carries none, `ManifestBatch`/`Enumerated` are denominators, the
/// phase signals carry nothing, and `SummaryReconciled` is a
/// close-boundary correction rather than payload in flight. A
/// zero-byte `Payload` (an empty file on the aggregate lane) moves the
/// file count, not the byte stream, so it does not extend the rate
/// window.
fn carries_payload(event: &ProgressEvent) -> bool {
    matches!(event, ProgressEvent::Payload { bytes, .. } if *bytes > 0)
}

/// One tick's rates, in bytes per second.
struct RateSample {
    avg_bps: f64,
    current_bps: f64,
    /// The window moved nothing while payload was still in flight.
    /// `current_bps` is a true 0 — the human line says so in words
    /// ("stalled") rather than printing a rate that reads like a
    /// measurement, and the JSON keeps its frozen numeric shape.
    stalled: bool,
}

/// The window the CLI progress monitor measures rates against.
///
/// A transfer's wall clock keeps running through the post-payload
/// phase — an in-session mirror purge starts only after the last
/// payload byte, and a push then waits for the destination's summary —
/// but no bytes move there. Measuring against that tail printed
/// trailing "0.00 MiB/s current" lines and divided the final average by
/// non-transfer time. So the averaging window ends at the last
/// byte-carrying event, and an idle tick is read against the phase it
/// lands in (owner ruling "revised b", 2026-08-19):
///
/// 1. **Before the first payload byte** — nothing to say about a rate;
///    the caller's manifest-liveness branch owns this phase and the
///    window stays silent.
/// 2. **Mid-payload** (a byte has moved, no `DeleteBegin` yet, and the
///    file counter is still short of its manifest denominator) — an
///    idle second is a stall the user needs to see, so the line still
///    prints with the current-rate segment replaced by the word
///    "stalled". Silence here looked like a hung monitor.
/// 3. **Post-payload** — a purge, or the wait for the destination's
///    summary, is *supposed* to move no bytes, so "stalled" would be a
///    lie and idle ticks print nothing. Two independent signals put us
///    here: `DeleteBegin` (a mirror purge is starting), and the file
///    counter reaching the manifest total (every file is placed).
///    Neither alone is sufficient — a non-mirror push never emits
///    `DeleteBegin`, and a run that skips files never reaches its
///    denominator — so both are checked.
///
/// State only, no clock of its own — every method takes `now`, so the
/// decisions are unit-testable with constructed instants.
struct RateWindow {
    start: Instant,
    /// Instant of the last byte-carrying event; `None` until the
    /// payload stream produces its first byte. Doubles as the phase-1
    /// discriminator: `None` means payload has not started.
    last_payload: Option<Instant>,
    /// Set once `DeleteBegin` passes through the event stream: the
    /// payload is over and idleness is expected from here on (phase 3).
    post_payload: bool,
    /// Start of the current per-second window.
    prev_instant: Instant,
    prev_bytes: u64,
    prev_files: u64,
}

impl RateWindow {
    fn new(start: Instant) -> Self {
        Self {
            start,
            last_payload: None,
            post_payload: false,
            prev_instant: start,
            prev_bytes: 0,
            prev_files: 0,
        }
    }

    /// Record that payload bytes arrived at `now` (call only when
    /// [`carries_payload`] says so).
    fn mark_payload(&mut self, now: Instant) {
        self.last_payload = Some(now);
    }

    /// Enter phase 3: the payload stream is done and the session is in
    /// its purge/bookkeeping tail. Call on `ProgressEvent::DeleteBegin`.
    fn begin_post_payload(&mut self) {
        self.post_payload = true;
    }

    /// Average over the payload window: start → the last byte-carrying
    /// event, falling back to `now` while no bytes have moved yet (the
    /// numerator is then 0 anyway).
    fn avg_bps(&self, bytes: u64, now: Instant) -> f64 {
        let end = self.last_payload.unwrap_or(now);
        let elapsed = end
            .saturating_duration_since(self.start)
            .as_secs_f64()
            .max(1e-6);
        (bytes as f64) / elapsed
    }

    /// The per-second decision. `None` means "print nothing for this
    /// tick"; a sample with `stalled` set means "print the line, but
    /// say stalled instead of a rate".
    ///
    /// The window always rolls forward, emitted or not: an idle second
    /// must not dilute the *next* window's current rate.
    fn tick(&mut self, totals: &ProgressTotals, now: Instant) -> Option<RateSample> {
        // Files as well as bytes: a run of empty files makes real
        // progress the user should still see.
        let moved = totals.bytes != self.prev_bytes || totals.files != self.prev_files;
        let window_elapsed = now
            .saturating_duration_since(self.prev_instant)
            .as_secs_f64()
            .max(1e-6);
        let window_bytes = totals.bytes.saturating_sub(self.prev_bytes);
        self.prev_instant = now;
        self.prev_bytes = totals.bytes;
        self.prev_files = totals.files;
        if !moved {
            // A stall is only a stall while work is still outstanding.
            // Phase 1 (no byte has moved yet) and phase 3 (`DeleteBegin`
            // seen) both expect idleness; so does a run whose file
            // counter has reached its manifest denominator — every file
            // is placed and the session is just waiting on the
            // destination's summary. That last check is what covers a
            // non-mirror push, which never emits `DeleteBegin` at all.
            let outstanding = totals.files < totals.manifest_files;
            if self.last_payload.is_none() || self.post_payload || !outstanding {
                return None;
            }
            return Some(RateSample {
                avg_bps: self.avg_bps(totals.bytes, now),
                current_bps: 0.0,
                stalled: true,
            });
        }
        Some(RateSample {
            avg_bps: self.avg_bps(totals.bytes, now),
            current_bps: (window_bytes as f64) / window_elapsed,
            stalled: false,
        })
    }
}

const MIB: f64 = 1024.0 * 1024.0;

/// Only the trailing segment moves: a measured second reports its rate,
/// a mid-payload idle second reports the word "stalled". Printing
/// "0.00 MiB/s current" there would read as a measurement of a moving
/// stream rather than as an absence of one.
fn progress_line(totals: &ProgressTotals, sample: &RateSample) -> String {
    let current = if sample.stalled {
        "stalled".to_string()
    } else {
        format!("{:.2} MiB/s current", sample.current_bps / MIB)
    };
    format!(
        "[progress] {}/{} files \u{2022} {:.2} MiB copied \u{2022} {:.2} MiB/s avg \u{2022} {}",
        totals.files,
        totals.manifest_files,
        totals.bytes as f64 / MIB,
        sample.avg_bps / MIB,
        current,
    )
}

/// JSON shape is frozen: same event name, same five fields, always
/// present — a stall is expressed as `current_bytes_sec: 0`, never as a
/// new field or a missing one. `avg_bytes_sec` divides by the payload
/// window (start → last byte-carrying event) rather than by total wall
/// time, so a run with a post-payload tail reports the rate the payload
/// actually moved at.
fn progress_json_line(totals: &ProgressTotals, sample: &RateSample) -> String {
    format!(
        "{{\"event\":\"progress\",\"files\":{},\"total_files\":{},\"bytes_copied\":{},\"avg_bytes_sec\":{:.0},\"current_bytes_sec\":{:.0}}}",
        totals.files, totals.manifest_files, totals.bytes, sample.avg_bps, sample.current_bps
    )
}

fn final_line(totals: &ProgressTotals, avg_bps: f64) -> String {
    format!(
        "[progress] final: {} file(s) transferred \u{2022} {:.2} MiB total \u{2022} {:.2} MiB/s avg",
        totals.files,
        totals.bytes as f64 / MIB,
        avg_bps / MIB,
    )
}

/// Same three fields as before. `avg_bytes_sec` is the payload-window
/// average (see [`progress_json_line`]).
fn final_json_line(totals: &ProgressTotals, avg_bps: f64) -> String {
    format!(
        "{{\"event\":\"final\",\"files_transferred\":{},\"total_bytes\":{},\"avg_bytes_sec\":{:.0}}}",
        totals.files, totals.bytes, avg_bps
    )
}

/// Spawn the per-transfer progress monitor. `suppress_final_line=true`
/// lets move callers gate the post-transfer "[progress] final: …"
/// line so a transfer-looking success summary doesn't appear on
/// stdout before source-delete runs (and possibly fails). The
/// per-file / per-second progress lines still emit because the
/// user wants liveness signal during the transfer; only the
/// post-transfer "final:" line is gated (R53-F1).
pub(crate) fn spawn_progress_monitor_with_options(
    enabled: bool,
    verbose: bool,
    json: bool,
    suppress_final_line: bool,
) -> (Option<RemoteTransferProgress>, Option<JoinHandle<()>>) {
    if !enabled {
        return (None, None);
    }

    let (tx, mut rx) = mpsc::unbounded_channel::<ProgressEvent>();
    let progress = RemoteTransferProgress::new(tx);
    let join = tokio::spawn(async move {
        let start = Instant::now();
        // w6-1: fold through the shared accumulator in blit-core — the
        // per-direction folding rules (and the CLI's byte double-count
        // on TCP pulls, design-1) are gone with the contract.
        let mut totals = ProgressTotals::default();
        let mut window = RateWindow::new(start);
        let mut ticker = interval(Duration::from_secs(1));
        ticker.set_missed_tick_behavior(MissedTickBehavior::Skip);

        loop {
            tokio::select! {
                biased;
                event = rx.recv() => {
                    match event {
                        Some(event) => {
                            totals.apply(&event);
                            if carries_payload(&event) {
                                window.mark_payload(Instant::now());
                            }
                            if matches!(event, ProgressEvent::DeleteBegin) {
                                // Last payload byte is behind us: idle
                                // ticks are normal from here (phase 3).
                                window.begin_post_payload();
                            }
                            if let ProgressEvent::FileComplete { path } = &event {
                                if json {
                                    // `bytes` stays in the JSON shape for
                                    // stream compatibility; per-event bytes
                                    // no longer exist under the contract
                                    // (they ride Payload events), so it is
                                    // always 0.
                                    eprintln!(
                                        "{{\"event\":\"file_complete\",\"path\":\"{}\",\"bytes\":0}}",
                                        path.replace('\\', "\\\\").replace('"', "\\\""),
                                    );
                                } else if verbose {
                                    println!("{}", path);
                                }
                            }
                        }
                        None => break,
                    }
                }
                _ = ticker.tick() => {
                    if totals.started() {
                        // `None` = say nothing this second: either no
                        // payload byte has moved yet, or the payload is
                        // over (a mirror purge or the wait for the
                        // destination's summary) and idleness is normal.
                        // A mid-payload idle tick returns a `stalled`
                        // sample instead, so the user sees the hang.
                        if let Some(sample) = window.tick(&totals, Instant::now()) {
                            if json {
                                eprintln!("{}", progress_json_line(&totals, &sample));
                            } else {
                                println!("{}", progress_line(&totals, &sample));
                            }
                        }
                    } else if totals.manifest_files > 0 {
                        if json {
                            eprintln!(
                                "{{\"event\":\"manifest\",\"total_files\":{}}}",
                                totals.manifest_files
                            );
                        } else {
                            println!(
                                "[progress] manifest enumerated {} file(s)\u{2026}",
                                totals.manifest_files
                            );
                        }
                    }
                }
            }
        }

        if totals.started() && !suppress_final_line {
            // Payload window, not total wall: a post-payload purge is
            // not transfer time and must not dilute the average.
            let avg_bps = window.avg_bps(totals.bytes, Instant::now());
            if json {
                eprintln!("{}", final_json_line(&totals, avg_bps));
            } else {
                println!("{}", final_line(&totals, avg_bps));
            }
        } else if !totals.started() && totals.manifest_files > 0 {
            if json {
                eprintln!(
                    "{{\"event\":\"manifest\",\"total_files\":{}}}",
                    totals.manifest_files
                );
            } else {
                println!(
                    "[progress] manifest enumerated {} file(s)",
                    totals.manifest_files
                );
            }
        }
    });

    (Some(progress), Some(join))
}

/// The user's compare flags, lifted off clap once for both verbs —
/// the inputs to the one `transfers::compare` mapping (otp-10b-2).
fn verb_compare_flags(args: &TransferArgs) -> CompareFlags {
    CompareFlags {
        checksum: args.checksum,
        size_only: args.size_only,
        ignore_times: args.ignore_times,
        force: args.force,
    }
}

/// Returns the push state so the caller can read the destination's
/// per-file failure report for the exit status (pfc-5); the summary is
/// already printed inline.
pub async fn run_remote_push_transfer(
    args: &TransferArgs,
    source: PathBuf,
    remote: RemoteEndpoint,
    mirror_mode: bool,
    lifecycle_trace: &TransferLifecycleTrace,
) -> Result<DeferredPushState> {
    run_remote_push_transfer_inner(
        args,
        source,
        remote,
        mirror_mode,
        false,
        false,
        lifecycle_trace,
    )
    .await
}

/// R51-F4: move's variant of [`run_remote_push_transfer`]. Returns
/// the push summary instead of printing inline so the caller can
/// defer output until after source-delete.
///
/// review otp-10a F1: move maps through `move_comparison_mode` —
/// `IgnoreTimes` (transfer every file unconditionally), or `Checksum`
/// when the user asked for it (a content-proven skip is safe). Move
/// deletes the source on success, so a metadata-shaped skip of a
/// same-size file whose content differs would destroy the only copy;
/// the mapping makes the delete safe by construction. Copy/mirror map
/// through the shared copy mapping (SizeMtime default, whose
/// same-size dest-newer skip is the standing owner question).
pub async fn run_remote_push_transfer_deferred(
    args: &TransferArgs,
    source: PathBuf,
    remote: RemoteEndpoint,
    mirror_mode: bool,
    lifecycle_trace: &TransferLifecycleTrace,
) -> Result<DeferredPushState> {
    run_remote_push_transfer_inner(
        args,
        source,
        remote,
        mirror_mode,
        true,
        true,
        lifecycle_trace,
    )
    .await
}

pub struct DeferredPushState {
    pub summary: blit_core::generated::TransferSummary,
    pub destination: String,
}

pub fn print_deferred_push_result(args: &TransferArgs, state: &DeferredPushState) {
    if args.json {
        print_push_json(&state.summary, &state.destination);
    } else {
        describe_push_result(&state.summary, &state.destination);
        // pfc-5: the end-of-operation block follows the summary, never
        // replaces it — a partial run still reports what landed.
        super::failures::print_failure_block(
            state.summary.files_failed,
            &super::failures::failures_from_wire(&state.summary.failures),
        );
    }
}

/// otp-10a: a failed session names the file a fault touched
/// (D-2026-07-09-1) — extract that end-of-operation summary from the
/// error chain, so the operator sees which file to re-run for without
/// digging through it. Applies to both fault shapes: a `SessionFault`
/// raised by a running session and a `TransferOpenRefusal` from a
/// session that never opened (whose inner fault never names a file —
/// `end_of_operation_summary` then returns `None`). Extraction is
/// split from the printing so the chain-walking is unit-pinned
/// (review otp-10a F7).
fn session_fault_summary(err: &eyre::Report) -> Option<String> {
    use blit_core::remote::transfer::session_client::TransferOpenRefusal;
    use blit_core::transfer_session::SessionFault;
    err.chain()
        .find_map(|cause| {
            cause
                .downcast_ref::<SessionFault>()
                .or_else(|| cause.downcast_ref::<TransferOpenRefusal>().map(|r| &r.0))
        })
        .and_then(|fault| fault.end_of_operation_summary())
}

fn emit_session_fault_summary(err: &eyre::Report) {
    if let Some(line) = session_fault_summary(err) {
        eprintln!("{line}");
    }
}

async fn run_remote_push_transfer_inner(
    args: &TransferArgs,
    source: PathBuf,
    remote: RemoteEndpoint,
    mirror_mode: bool,
    move_verb: bool,
    defer_output: bool,
    lifecycle_trace: &TransferLifecycleTrace,
) -> Result<DeferredPushState> {
    let show_progress = args.effective_progress() || args.verbose;
    let (progress_handle, progress_task) = spawn_progress_monitor_with_options(
        show_progress,
        args.verbose,
        args.json,
        defer_output, // R53-F1: suppress the final progress line on move
    );

    // Filter parity: the wire FilterSpec rides `SessionOpen.filter`
    // (otp-10a); the session's SOURCE end applies it through the
    // universal `FilteredSource` chokepoint and the daemon DESTINATION
    // scopes mirror deletions with it — identical rules to what
    // `--exclude/--include/--min-size/...` produce on pull.
    let filter_spec = super::build_filter_spec(args)?;

    // R59 #1 F2: translate the user's --delete-scope flag to the wire
    // MirrorMode enum. Default to FilteredSubset so `push --include …
    // --mirror` deletes only files in scope. R59 #1 F1: require a
    // complete source scan for any mirror operation — a partial scan
    // could cause silent dest-side data loss when the daemon purges
    // entries it (wrongly) thinks are absent from the source.
    let mirror_kind = if mirror_mode {
        if args.delete_scope_all() {
            blit_core::generated::MirrorMode::All
        } else {
            blit_core::generated::MirrorMode::FilteredSubset
        }
    } else {
        blit_core::generated::MirrorMode::Off
    };

    // otp-10b-2: the ONE args→compare mapping, shared with the pull
    // verb (the old push driver ignored every compare flag).
    let compare_mode = if move_verb {
        move_comparison_mode(verb_compare_flags(args))
    } else {
        comparison_mode(verb_compare_flags(args))
    };

    let execution = PushExecution {
        source,
        remote: remote.clone(),
        filter: Some(filter_spec),
        mirror_mode,
        mirror_kind,
        force_grpc: args.force_grpc,
        trace_data_plane: args.trace_data_plane,
        // Mirror needs a complete source scan (R59 #1 F1). Move-push
        // keeps otp-10a's posture instead: the readable subset lands,
        // the unreadable accumulator fails the call, and the deferred
        // print + source-delete gate never fire.
        require_complete_scan: mirror_mode,
        drop_windows_metadata: args.drop_windows_metadata,
        resume: args.resume,
        resume_block_size: 0, // destination default (1 MiB)
        compare_mode,
        ignore_existing: args.ignore_existing,
        remote_label: format_remote_endpoint(&remote),
        lifecycle_trace: lifecycle_trace.clone(),
        verbose: args.verbose,
        // cr-a16-1: `-p` is the other half of audit-16's gate, so the
        // remote route reads the same effective decision the local one
        // does (`LocalMirrorOptions.progress`) rather than losing the
        // documented "(or `-p`)" liveness fallback.
        progress: args.effective_progress(),
    };

    // Push has no caller-side destructive step (mirror-delete is
    // daemon-side and surfaces via the summary), so unlike the pull
    // lifecycle there is no need to drop the progress handle
    // *before* a follow-up library call — the monitor's lifetime
    // already matches the RPC.
    let outcome = run_remote_push(execution, progress_handle.as_ref()).await;

    drop(progress_handle);
    if let Some(task) = progress_task {
        let _ = task.await;
    }

    let outcome = match outcome {
        Ok(outcome) => outcome,
        Err(err) => {
            emit_session_fault_summary(&err);
            return Err(err);
        }
    };
    let state = DeferredPushState {
        summary: outcome.summary,
        destination: outcome.destination,
    };
    if !defer_output {
        super::render_result(lifecycle_trace, || {
            print_deferred_push_result(args, &state);
            Ok(())
        })?;
    }
    Ok(state)
}

/// Returns the pull state so the caller can read the destination's
/// per-file failure report for the exit status (pfc-5); the summary is
/// already printed inline.
pub async fn run_remote_pull_transfer(
    args: &TransferArgs,
    remote: RemoteEndpoint,
    dest_root: &Path,
    mirror_mode: bool,
    move_verb: bool,
    lifecycle_trace: &TransferLifecycleTrace,
) -> Result<DeferredPullState> {
    run_remote_pull_transfer_inner(
        args,
        remote,
        dest_root,
        mirror_mode,
        move_verb,
        false, // emit success summary inline (copy/mirror default)
        lifecycle_trace,
    )
    .await
}

/// R51-F4: move's variant of `run_remote_pull_transfer` — runs the
/// transfer but does NOT emit the success summary. Caller is
/// responsible for printing after source-delete completes (or
/// refusing to print on source-delete failure).
pub async fn run_remote_pull_transfer_deferred(
    args: &TransferArgs,
    remote: RemoteEndpoint,
    dest_root: &Path,
    mirror_mode: bool,
    move_verb: bool,
    lifecycle_trace: &TransferLifecycleTrace,
) -> Result<DeferredPullState> {
    run_remote_pull_transfer_inner(
        args,
        remote,
        dest_root,
        mirror_mode,
        move_verb,
        true,
        lifecycle_trace,
    )
    .await
}

pub fn print_deferred_pull_result(args: &TransferArgs, state: &DeferredPullState) {
    if args.json {
        print_pull_json(&state.summary, &state.dest_root);
    } else {
        describe_pull_result(&state.summary, &state.dest_root);
        // pfc-5: the end-of-operation block follows the summary, never
        // replaces it — a partial run still reports what landed.
        super::failures::print_failure_block(
            state.summary.files_failed,
            &super::failures::failures_from_wire(&state.summary.failures),
        );
    }
}

async fn run_remote_pull_transfer_inner(
    args: &TransferArgs,
    remote: RemoteEndpoint,
    dest_root: &Path,
    mirror_mode: bool,
    move_verb: bool,
    defer_output: bool,
    lifecycle_trace: &TransferLifecycleTrace,
) -> Result<DeferredPullState> {
    // Filter parity: the wire FilterSpec rides `SessionOpen.filter`
    // (otp-10b-2); the daemon SOURCE applies it through the universal
    // `FilteredSource` chokepoint and this DESTINATION scopes mirror
    // deletions with it — identical rules to push, by construction.
    let filter_spec = super::build_filter_spec(args)?;

    let show_progress = args.effective_progress() || args.verbose;
    let (progress_handle, progress_task) = spawn_progress_monitor_with_options(
        show_progress,
        args.verbose,
        args.json,
        defer_output, // R53-F1: suppress final progress line on move
    );

    // R59 #1 F2: --delete-scope → wire MirrorMode, same mapping as the
    // push verb (FilteredSubset default so `--include … --mirror`
    // deletes only in-scope entries).
    let mirror_kind = if mirror_mode {
        if args.delete_scope_all() {
            blit_core::generated::MirrorMode::All
        } else {
            blit_core::generated::MirrorMode::FilteredSubset
        }
    } else {
        blit_core::generated::MirrorMode::Off
    };

    // otp-10b-2: the ONE args→compare mapping, shared with push.
    let compare_mode = if move_verb {
        move_comparison_mode(verb_compare_flags(args))
    } else {
        comparison_mode(verb_compare_flags(args))
    };

    let execution = PullExecution {
        remote: remote.clone(),
        dest_root: dest_root.to_path_buf(),
        filter: Some(filter_spec),
        mirror_mode,
        mirror_kind,
        force_grpc: args.force_grpc,
        trace_data_plane: args.trace_data_plane,
        // R49-F2 / otp-9b F1: move refuses a partial source scan
        // (ScanIncomplete, before any deletion decision) — the remote
        // source is deleted after this returns. Mirror needs no flag:
        // the session refuses an incomplete-scan mirror on its own.
        require_complete_scan: move_verb,
        drop_windows_metadata: args.drop_windows_metadata,
        resume: args.resume,
        resume_block_size: 0, // destination default (1 MiB)
        compare_mode,
        ignore_existing: args.ignore_existing,
        remote_label: format_remote_endpoint(&remote),
        lifecycle_trace: lifecycle_trace.clone(),
    };

    // Mirror deletions run in-session at SourceDone (the one delete
    // rule, otp-6b) — there is no post-RPC destructive step, so the
    // monitor's lifetime matches the one library call, exactly like
    // the push verb. (The old pull's two-phase split existed to tear
    // the monitor down before a client-side purge; both the split and
    // the purge died with the driver at otp-10c-2.)
    let outcome = run_remote_pull(execution, progress_handle.as_ref()).await;

    drop(progress_handle);
    if let Some(task) = progress_task {
        let _ = task.await;
    }

    let state = match outcome {
        Ok(state) => state,
        Err(err) => {
            // otp-10a Q2 parity: a failed session names the file the
            // fault touched before the error propagates.
            emit_session_fault_summary(&err);
            return Err(err);
        }
    };

    // R51-F4: when deferred, skip the inline print. The caller
    // (move) prints via `print_deferred_pull_result` after the
    // source-delete step succeeds — so a post-transfer failure
    // never leaves a success-looking JSON document on stdout.
    if !defer_output {
        super::render_result(lifecycle_trace, || {
            print_deferred_pull_result(args, &state);
            Ok(())
        })?;
    }

    Ok(state)
}

fn print_pull_json(summary: &blit_core::generated::TransferSummary, dest_root: &Path) {
    use serde_json::json;
    // otp-10b-2: the pull verb reports the session's
    // destination-computed summary — the same keys as the push verb's
    // JSON. Keys only the deleted driver could fill (bytes_zero_copy —
    // always 0 on the session; the R46-F6 mirror_purge object — the
    // wire carries one entries_deleted count) are gone; files_resumed
    // is new.
    // pfc-5: `files_failed` / `failures` are the machine half of the
    // human failure block, in the one shape every route emits. The
    // process still exits with the partial-failure status, so a consumer
    // that only checks the status is not misled by a document that
    // otherwise looks like a clean pull.
    let failures = super::failures::failures_from_wire(&summary.failures);
    let summary = json!({
        "operation": "pull",
        "destination": dest_root.to_string_lossy(),
        "files_transferred": summary.files_transferred,
        "bytes_transferred": summary.bytes_transferred,
        "files_resumed": summary.files_resumed,
        "entries_deleted": summary.entries_deleted,
        "tcp_fallback": summary.in_stream_carrier_used,
        "files_failed": summary.files_failed,
        "failures": super::failures::failures_json(&failures),
    });
    println!("{}", serde_json::to_string_pretty(&summary).unwrap());
}

fn print_push_json(summary: &blit_core::generated::TransferSummary, destination: &str) {
    use serde_json::json;
    // otp-10a: the push verb reports the session's destination-computed
    // summary. Keys that only the deleted driver could fill
    // (files_requested, bytes_zero_copy, first_payload_ms) are gone;
    // files_resumed is new with push-side --resume.
    // pfc-5: same two fields, same shape, as every other route's document.
    let failures = super::failures::failures_from_wire(&summary.failures);
    let summary = json!({
        "operation": "push",
        "destination": destination,
        "files_transferred": summary.files_transferred,
        "bytes_transferred": summary.bytes_transferred,
        "files_resumed": summary.files_resumed,
        "entries_deleted": summary.entries_deleted,
        "tcp_fallback": summary.in_stream_carrier_used,
        "files_failed": summary.files_failed,
        "failures": super::failures::failures_json(&failures),
    });
    println!("{}", serde_json::to_string_pretty(&summary).unwrap());
}

pub fn describe_pull_result(summary: &blit_core::generated::TransferSummary, dest_root: &Path) {
    // otp-10b-2: the session's DESTINATION (this end) is the scorer.
    // The pinned `Pull complete:` prefix and `[gRPC fallback]` marker
    // keep their exact wording; the old driver-only zero-copy clause
    // is gone (always 0 on the session — zero-copy returns as a
    // post-cutover write strategy, D-2026-07-05-3).
    let resumed = if summary.files_resumed > 0 {
        format!(" ({} resumed block-wise)", summary.files_resumed)
    } else {
        String::new()
    };
    println!(
        "Pull complete: {} file(s), {} bytes{}{} -> {}.",
        summary.files_transferred,
        summary.bytes_transferred,
        resumed,
        if summary.in_stream_carrier_used {
            " [gRPC fallback]"
        } else {
            ""
        },
        dest_root.display()
    );
    if summary.entries_deleted > 0 {
        let plural = if summary.entries_deleted == 1 {
            "y"
        } else {
            "ies"
        };
        println!(
            "Mirror purge removed {} entr{}.",
            summary.entries_deleted, plural
        );
    }
}

pub fn describe_push_result(summary: &blit_core::generated::TransferSummary, destination: &str) {
    // otp-10a: the session's DESTINATION is the scorer; the old
    // negotiation-phase lines (file counts scheduled, data port) died
    // with the per-direction driver. `[gRPC fallback]` keeps its exact
    // wording — it marks the session's in-stream byte carrier now.
    if summary.files_transferred == 0 && summary.files_resumed == 0 {
        println!(
            "Remote already up to date; nothing to upload ({}).",
            destination
        );
    }
    let resumed = if summary.files_resumed > 0 {
        format!(" ({} resumed block-wise)", summary.files_resumed)
    } else {
        String::new()
    };
    println!(
        "Transfer complete: {} file(s), {} bytes{}{}.",
        summary.files_transferred,
        summary.bytes_transferred,
        resumed,
        if summary.in_stream_carrier_used {
            " [gRPC fallback]"
        } else {
            ""
        }
    );
    if summary.entries_deleted > 0 {
        // otp-10b-2: "entr"/"entrs" typo fixed; matches the pull
        // printer's entry/entries.
        let plural = if summary.entries_deleted == 1 {
            "y"
        } else {
            "ies"
        };
        println!(
            "Remote purge removed {} entr{}.",
            summary.entries_deleted, plural
        );
    }
    println!("Destination: {}", destination);
}

// This module's test surface is reserved for CLI-entry-point
// behavior; library behavior is pinned in blit_core.

#[cfg(test)]
mod rate_window_tests {
    use super::{carries_payload, final_line, progress_json_line, progress_line, RateWindow, MIB};
    use blit_core::remote::transfer::{ProgressEvent, ProgressTotals};
    use std::time::{Duration, Instant};

    /// One scripted moment on the monitor's timeline.
    enum Step {
        /// An event arrived on the progress channel.
        Event(ProgressEvent),
        /// The one-second ticker fired.
        Tick,
    }

    /// What a replayed timeline would have printed.
    struct Replayed {
        /// The human per-second lines, in order.
        lines: Vec<String>,
        /// The `--json` rendering of the very same ticks, so the two
        /// surfaces cannot silently disagree about a tick.
        json: Vec<String>,
        /// The final line rendered at the script's end instant.
        final_text: String,
    }

    /// Replay a timeline through the monitor's decision path with
    /// injected instants — the same calls the ticker loop makes, so the
    /// test pins behaviour rather than a private formula.
    fn replay(script: &[(u64, Step)], end_ms: u64) -> Replayed {
        let t0 = Instant::now();
        let at = |ms: u64| t0 + Duration::from_millis(ms);
        let mut totals = ProgressTotals::default();
        let mut window = RateWindow::new(t0);
        let mut lines = Vec::new();
        let mut json = Vec::new();
        for (ms, step) in script {
            match step {
                Step::Event(event) => {
                    totals.apply(event);
                    if carries_payload(event) {
                        window.mark_payload(at(*ms));
                    }
                    if matches!(event, ProgressEvent::DeleteBegin) {
                        window.begin_post_payload();
                    }
                }
                Step::Tick => {
                    if totals.started() {
                        if let Some(sample) = window.tick(&totals, at(*ms)) {
                            lines.push(progress_line(&totals, &sample));
                            json.push(progress_json_line(&totals, &sample));
                        }
                    }
                }
            }
        }
        let avg = window.avg_bps(totals.bytes, at(end_ms));
        Replayed {
            lines,
            json,
            final_text: final_line(&totals, avg),
        }
    }

    fn payload(mib: u64) -> ProgressEvent {
        ProgressEvent::Payload {
            files: 1,
            bytes: mib * MIB as u64,
        }
    }

    /// Phase 3, reached by `DeleteBegin`. The residue this fix closes:
    /// 9 MiB moved over the first three seconds, then an in-session
    /// mirror purge holds the session open for five more. Past
    /// `DeleteBegin` the ticker must go fully quiet — no trailing
    /// "0.00 MiB/s current", and no "stalled" either, because a purge
    /// moving no payload is not a stall — and the final average must
    /// divide by the 3 s payload window (3.00 MiB/s), not by the 8 s
    /// wall clock (1.12 MiB/s).
    ///
    /// The manifest promises 4 files and only 3 transfer (the fourth is
    /// skipped as already-current, the ordinary mirror case). So the
    /// file counter never reaches its denominator and the
    /// files-complete signal cannot fire: `DeleteBegin` is the only
    /// thing holding the purge quiet, which is exactly what this test
    /// is for.
    #[test]
    fn post_payload_phase_emits_no_zero_rate_and_does_not_dilute_the_average() {
        let script = vec![
            (
                0,
                Step::Event(ProgressEvent::ManifestBatch {
                    files: 4,
                    bytes: 12 * MIB as u64,
                }),
            ),
            (1000, Step::Event(payload(3))),
            (1500, Step::Tick),
            (2000, Step::Event(payload(3))),
            (2500, Step::Tick),
            (3000, Step::Event(payload(3))),
            (3500, Step::Tick),
            // Last payload byte is behind us; the purge runs on.
            (3600, Step::Event(ProgressEvent::DeleteBegin)),
            (4500, Step::Tick),
            (5500, Step::Tick),
            (6500, Step::Tick),
            (7500, Step::Tick),
        ];
        let Replayed {
            lines, final_text, ..
        } = replay(&script, 8000);

        assert_eq!(
            lines.len(),
            3,
            "one line per payload-carrying second, none for the purge: {lines:#?}"
        );
        assert!(
            lines.iter().all(|l| !l.contains("0.00 MiB/s current")),
            "no tick may report a fabricated zero current rate: {lines:#?}"
        );
        assert!(
            lines.iter().all(|l| !l.contains("stalled")),
            "a post-payload purge is not a stall: {lines:#?}"
        );
        assert!(
            final_text.contains("3.00 MiB/s avg"),
            "the average divides by the payload window: {final_text}"
        );
        assert!(
            !final_text.contains("1.12 MiB/s avg"),
            "wall-clock dilution regressed: {final_text}"
        );
        assert!(
            final_text.contains("9.00 MiB total"),
            "byte total is untouched by the window change: {final_text}"
        );
    }

    /// Movement, not bytes alone: a run of empty files moves the file
    /// count with zero bytes and must still report liveness. The
    /// trailing idle tick lands in phase 1 — no payload byte has ever
    /// moved, so there is no stall to report and nothing prints.
    #[test]
    fn file_only_progress_still_emits_a_line() {
        let script = vec![
            (
                500,
                Step::Event(ProgressEvent::FileComplete { path: "a".into() }),
            ),
            (1000, Step::Tick),
            (
                1500,
                Step::Event(ProgressEvent::FileComplete { path: "b".into() }),
            ),
            (2000, Step::Tick),
            // Nothing at all moves in this window.
            (3000, Step::Tick),
        ];
        let Replayed { lines, .. } = replay(&script, 3000);
        assert_eq!(
            lines.len(),
            2,
            "empty-file progress is still progress, and pre-payload idle stays silent: {lines:#?}"
        );
        assert!(lines[1].contains("2/0 files"), "got: {}", lines[1]);
    }

    /// Phase 2, the behaviour "revised b" adds. Payload starts, then the
    /// stream hangs for two seconds. Those seconds must still print —
    /// silence there reads as a dead monitor — but say "stalled" rather
    /// than a fabricated rate. The average is frozen at the last
    /// payload byte, so it does not decay while nothing moves, and the
    /// JSON keeps its five frozen fields with `current_bytes_sec: 0`.
    #[test]
    fn mid_payload_idle_ticks_report_a_stall() {
        let script = vec![
            (
                0,
                Step::Event(ProgressEvent::ManifestBatch {
                    files: 2,
                    bytes: 6 * MIB as u64,
                }),
            ),
            (1000, Step::Event(payload(3))),
            (1500, Step::Tick),
            // The stream hangs — no bytes, no files, no DeleteBegin.
            (2500, Step::Tick),
            (3500, Step::Tick),
        ];
        let Replayed { lines, json, .. } = replay(&script, 3500);

        assert_eq!(
            lines.len(),
            3,
            "a mid-payload stall is reported, not swallowed: {lines:#?}"
        );
        assert!(
            lines[0].contains("MiB/s current") && !lines[0].contains("stalled"),
            "the moving second is unchanged: {}",
            lines[0]
        );
        for line in &lines[1..] {
            assert_eq!(
                line, "[progress] 1/2 files \u{2022} 3.00 MiB copied \u{2022} 3.00 MiB/s avg \u{2022} stalled",
                "exact stalled shape: only the rate segment changes"
            );
        }
        assert!(
            json[1].contains("\"current_bytes_sec\":0"),
            "JSON expresses a stall as a zero rate, not a new field: {}",
            json[1]
        );
        assert!(
            json[1].starts_with("{\"event\":\"progress\",\"files\":1,\"total_files\":2,\"bytes_copied\":3145728,\"avg_bytes_sec\":3145728,"),
            "the machine shape is frozen: {}",
            json[1]
        );
        assert!(
            !json[1].contains("stalled"),
            "the word never leaks into the machine surface: {}",
            json[1]
        );
    }

    /// Phase 3 reached WITHOUT `DeleteBegin` — the case the first cut
    /// of this change got wrong. A non-mirror push runs no purge and so
    /// never emits `DeleteBegin`, but it still holds the session open
    /// after the last file while it waits for the destination's
    /// summary. Every manifest file is placed by then, so there is no
    /// outstanding work to stall on and the tail must stay silent
    /// rather than accusing a healthy transfer of hanging.
    #[test]
    fn the_summary_wait_tail_is_silent_without_a_delete_begin() {
        let script = vec![
            (
                0,
                Step::Event(ProgressEvent::ManifestBatch {
                    files: 2,
                    bytes: 6 * MIB as u64,
                }),
            ),
            (1000, Step::Event(payload(3))),
            (1500, Step::Tick),
            (2000, Step::Event(payload(3))),
            (2500, Step::Tick),
            // Both files are placed. No DeleteBegin will ever arrive;
            // the session is just waiting on the peer's summary.
            (3500, Step::Tick),
            (4500, Step::Tick),
            (5500, Step::Tick),
        ];
        let Replayed {
            lines, final_text, ..
        } = replay(&script, 6000);

        assert_eq!(
            lines.len(),
            2,
            "the summary wait is not a stall: {lines:#?}"
        );
        assert!(
            lines.iter().all(|l| !l.contains("stalled")),
            "no stall may be reported once every manifest file is placed: {lines:#?}"
        );
        assert!(
            final_text.contains("3.00 MiB/s avg"),
            "the average still divides by the 2 s payload window: {final_text}"
        );
    }

    /// An idle stretch must not bleed into the next window's current
    /// rate: the window rolls forward on a stalled tick too, so the
    /// second that resumes payload reports the rate it actually moved
    /// at (4 MiB in the 1 s since the last tick), not 4 MiB spread over the idle
    /// time. The two idle seconds are mid-payload with a file still
    /// outstanding (1 of 3 placed), so they print as stalls rather than
    /// vanishing.
    #[test]
    fn a_stalled_tick_still_rolls_the_window_forward() {
        let script = vec![
            (
                0,
                Step::Event(ProgressEvent::ManifestBatch {
                    files: 3,
                    bytes: 5 * MIB as u64,
                }),
            ),
            (500, Step::Event(payload(1))),
            (1000, Step::Tick),
            (2000, Step::Tick),
            (3000, Step::Tick),
            (3500, Step::Event(payload(4))),
            (4000, Step::Tick),
        ];
        let Replayed { lines, .. } = replay(&script, 4000);
        assert_eq!(
            lines.len(),
            4,
            "two live seconds bracketing two stalled ones: {lines:#?}"
        );
        assert!(
            lines[1].ends_with("stalled") && lines[2].ends_with("stalled"),
            "the idle stretch is reported as stalled: {lines:#?}"
        );
        assert!(
            lines[3].contains("4.00 MiB/s current"),
            "the resumed window is measured from the last tick, not from the last live one: {}",
            lines[3]
        );
    }

    /// Only a byte-bearing `Payload` extends the rate window — the
    /// classification the whole fix rests on.
    #[test]
    fn only_byte_bearing_payload_events_extend_the_window() {
        assert!(carries_payload(&ProgressEvent::Payload {
            files: 0,
            bytes: 1
        }));
        assert!(!carries_payload(&ProgressEvent::Payload {
            files: 1,
            bytes: 0
        }));
        assert!(!carries_payload(&ProgressEvent::FileComplete {
            path: "a".into()
        }));
        assert!(!carries_payload(&ProgressEvent::ManifestBatch {
            files: 1,
            bytes: 99
        }));
        assert!(!carries_payload(&ProgressEvent::Enumerated { files: 5 }));
        assert!(!carries_payload(&ProgressEvent::DiffComplete));
        assert!(!carries_payload(&ProgressEvent::DeleteBegin));
        assert!(!carries_payload(&ProgressEvent::SummaryReconciled {
            files_failed: 1,
            bytes_landed: 7,
        }));
    }
}

#[cfg(test)]
mod session_fault_summary_tests {
    use super::session_fault_summary;
    use blit_core::generated::session_error::Code;
    use blit_core::remote::transfer::session_client::TransferOpenRefusal;
    use blit_core::transfer_session::SessionFault;

    fn fault_with_path(path: &str) -> SessionFault {
        SessionFault {
            code: Code::Internal,
            message: "'big.bin' hit EOF with 42 bytes still promised".into(),
            local_build_id: String::new(),
            peer_build_id: String::new(),
            peer_notified: true,
            relative_path: Some(path.into()),
            io_kind: None,
        }
    }

    /// The verb-level print's contract (D-2026-07-09-1 Q2): the
    /// summary extracted from a real, context-wrapped verb error names
    /// the affected file and suggests a re-run.
    #[test]
    fn names_the_file_and_suggests_a_rerun_through_context_layers() {
        let err = eyre::Report::new(fault_with_path("big.bin"))
            .wrap_err("pushing to 127.0.0.1:9031:/test/");
        let line = session_fault_summary(&err).expect("fault with a path yields a summary");
        assert!(line.contains("affected file: big.bin"), "got: {line}");
        assert!(line.contains("re-run"), "got: {line}");
    }

    /// An open-time refusal wraps its fault in `TransferOpenRefusal`;
    /// the extraction must reach through it. Open faults carry no file
    /// (nothing transferred yet) — no summary, nothing printed.
    #[test]
    fn open_refusals_without_a_file_yield_no_summary() {
        let mut fault = fault_with_path("x");
        fault.relative_path = None;
        let err = eyre::Report::new(TransferOpenRefusal(fault)).wrap_err("pushing to host:/mod/");
        assert!(session_fault_summary(&err).is_none());
    }

    /// A refusal whose inner fault DOES name a file still summarizes —
    /// the downcast reaches the inner fault through the wrapper.
    #[test]
    fn open_refusal_with_a_file_summarizes_through_the_wrapper() {
        let err = eyre::Report::new(TransferOpenRefusal(fault_with_path("nested/f.txt")))
            .wrap_err("pushing");
        let line = session_fault_summary(&err).expect("inner fault names a file");
        assert!(line.contains("affected file: nested/f.txt"), "got: {line}");
    }

    /// Non-session errors (connect failures, arg errors) never print a
    /// transfer-abort block.
    #[test]
    fn plain_errors_yield_no_summary() {
        assert!(session_fault_summary(&eyre::eyre!("connection refused")).is_none());
    }
}
