use crate::cli::TransferArgs;
use crate::context::AppContext;
use blit_app::display::{format_bps, format_bytes};
use blit_core::remote::transfer::{
    ProgressEvent, ProgressTotals, RemoteTransferProgress, TransferLifecycleTrace,
};
use blit_core::transfer_session::{LocalMirrorOptions, LocalMirrorSummary, TransferOutcome};
use eyre::{bail, Result};
use indicatif::{ProgressBar, ProgressStyle};
use std::path::Path;
use std::time::{Duration, Instant};
use tokio::sync::mpsc;
use tokio::task::JoinHandle;
use tokio::time::{interval, MissedTickBehavior};

/// Convenience wrapper for callers that always want the summary
/// printed inline. Most CLI paths (copy / mirror) want this; move
/// uses [`run_local_transfer_deferred`] so it can suppress the
/// "success" output until after the source-delete decision is
/// made (R49-F3).
pub async fn run_local_transfer(
    ctx: &AppContext,
    args: &TransferArgs,
    src_path: &Path,
    dest_path: &Path,
    mirror: bool,
    lifecycle_trace: &TransferLifecycleTrace,
) -> Result<LocalMirrorSummary> {
    run_local_transfer_inner(
        ctx,
        args,
        src_path,
        dest_path,
        mirror,
        false,
        false,
        Some(lifecycle_trace),
    )
    .await
}

/// Same as [`run_local_transfer`] but for the MOVE verb: the caller
/// takes ownership of when (and whether) to print the final summary
/// (a failure during source-delete can then surface without first
/// emitting a successful-looking JSON document on stdout — R49-F3),
/// and the compare maps through the move rule (codex otp-10b-2 F3):
/// transfer unconditionally, or `--checksum` for the one skip that is
/// content-proven safe — a SizeMtime skip of a same-size same-mtime
/// changed file followed by the source-delete would destroy the only
/// copy, the same otp-10a F1 hazard the remote move verbs closed.
pub async fn run_local_transfer_deferred(
    ctx: &AppContext,
    args: &TransferArgs,
    src_path: &Path,
    dest_path: &Path,
    mirror: bool,
) -> Result<LocalMirrorSummary> {
    run_local_transfer_inner(ctx, args, src_path, dest_path, mirror, true, true, None).await
}

/// Print the standard summary block for a completed local
/// transfer. Exposed for `run_local_transfer_deferred` callers
/// (move) that need to emit output AFTER their own follow-up
/// (source-delete) succeeds. Mirrors the inline print in
/// `run_local_transfer_inner` so deferred + inline callers
/// produce byte-identical output.
pub fn print_local_transfer_summary(
    ctx: &AppContext,
    args: &TransferArgs,
    mirror: bool,
    summary: &LocalMirrorSummary,
    elapsed: Duration,
    src_path: &Path,
    dest_path: &Path,
) -> Result<()> {
    // Only presentation fields are read here; the compare mode (and
    // thus the move_verb flag) is irrelevant to printing.
    let options = build_local_options(ctx, args, mirror, false)?;
    if args.json {
        print_summary_json(mirror, summary, elapsed, src_path, dest_path);
    } else {
        print_summary(
            mirror,
            options.dry_run,
            options.null_sink,
            options.verbose,
            options.debug_mode,
            options.workers,
            summary,
            elapsed,
        );
    }
    Ok(())
}

async fn run_local_transfer_inner(
    ctx: &AppContext,
    args: &TransferArgs,
    src_path: &Path,
    dest_path: &Path,
    mirror: bool,
    defer_output: bool,
    move_verb: bool,
    lifecycle_trace: Option<&TransferLifecycleTrace>,
) -> Result<LocalMirrorSummary> {
    if !src_path.exists() {
        bail!("source path does not exist: {}", src_path.display());
    }

    let mut options = build_local_options(ctx, args, mirror, move_verb)?;
    let dry_run = options.dry_run;
    let null_sink = options.null_sink;
    let json_output = args.json;
    let verbose = options.verbose;
    let debug_mode = options.debug_mode;
    let workers = options.workers;
    if debug_mode {
        eprintln!(
            "blit: debug: worker limiter active – local apply pipeline capped to {workers} worker(s)."
        );
    }

    // `effective_progress` decides WHEN a row engages (unchanged); what
    // it shows is now the live lane instead of a fixed spinner message.
    // Per-file lines are a `-v` feature of the row; `--json` keeps its
    // stderr free of unstructured lines.
    let progress_row = LiveProgressRow::start(
        args.effective_progress(),
        verbose && !json_output,
        mirror,
        src_path,
        dest_path,
    )
    .map(|(sink, row)| {
        options.progress_events = Some(sink);
        row
    });

    let start = Instant::now();
    let result = blit_app::transfers::local::run(src_path, dest_path, options).await;

    // Clear the row BEFORE the result is propagated: a live steady-tick
    // row would otherwise redraw over the error the caller prints.
    if let Some(row) = progress_row {
        row.finish(result.is_ok()).await;
    }
    let summary = result?;

    let elapsed = start.elapsed();
    if !defer_output {
        super::render_result(
            lifecycle_trace.expect("inline local output has a lifecycle trace"),
            || {
                if json_output {
                    print_summary_json(mirror, &summary, elapsed, src_path, dest_path);
                } else {
                    print_summary(
                        mirror, dry_run, null_sink, verbose, debug_mode, workers, &summary, elapsed,
                    );
                }
                Ok(())
            },
        )?;
    }

    Ok(summary)
}

/// How often the row re-renders. Events are drained continuously (the
/// pipeline must never block on the sink) but folded into one repaint
/// per interval, so a hot small-file run costs one format per tick
/// instead of one per file.
const ROW_REFRESH: Duration = Duration::from_millis(125);

/// Upper bound on waiting for the consumer after the session returned.
/// A blocking enumeration task cannot be aborted (`spawn_blocking`), so
/// a failed session can still hold a sink clone; the exit path must not
/// hang on the row.
const ROW_DRAIN_GRACE: Duration = Duration::from_millis(500);

/// The row's own column budget. indicatif's `{wide_msg}` re-clips the
/// message against the live terminal width on every draw — that is what
/// guarantees the row never wraps — so this is only the width the row
/// lays itself out for. Chosen to fit the conventional 80-column
/// terminal: wider terminals simply leave the row short instead of
/// stretching a path across the screen, narrower ones are clipped by
/// indicatif with the counters (rendered first) intact.
const ROW_COLUMNS: usize = 80;

/// Below this the current-file segment is an ellipsis and a couple of
/// characters — noise rather than information, so the row drops it and
/// keeps the counters.
const MIN_FILE_SEGMENT: usize = 12;

/// Phase the event stream has reached. Ordered, and the row only ever
/// moves forward: a late enumeration event during the copy must not
/// relabel the row.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord)]
enum LivePhase {
    #[default]
    Enumerating,
    Comparing,
    Copying,
    Deleting,
}

/// Everything the live row renders. Counters ride blit-core's shared
/// fold ([`ProgressTotals`], w6-1 — consumers must not re-derive the
/// folding rules); the phase, the diff-finished fact, and the current
/// file are local.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
struct LiveRowState {
    phase: LivePhase,
    totals: ProgressTotals,
    /// The destination diffed the whole manifest: `manifest_files` is
    /// final. Not a phase — it can arrive while the copy already runs —
    /// but without it zero needed files is indistinguishable from a scan
    /// still running, and an up-to-date tree says "enumerating" for the
    /// whole run.
    diff_complete: bool,
    /// Most recent finished file, shown as the row's current-activity
    /// segment. The per-file lane is the only cheap signal the session
    /// already emits; there is no in-flight-file event to prefer.
    current_file: Option<String>,
}

impl LiveRowState {
    fn apply(&mut self, event: &ProgressEvent) {
        self.totals.apply(event);
        let phase = match event {
            // Source walk liveness.
            ProgressEvent::Enumerated { .. } => LivePhase::Enumerating,
            // The destination diff decided these files need transfer.
            ProgressEvent::ManifestBatch { .. } => LivePhase::Comparing,
            ProgressEvent::DiffComplete => {
                self.diff_complete = true;
                LivePhase::Comparing
            }
            // Bytes are landing.
            ProgressEvent::Payload { .. } => LivePhase::Copying,
            ProgressEvent::FileComplete { path } => {
                self.current_file = Some(sanitize_row_text(path));
                LivePhase::Copying
            }
            // The mirror's delete pass — no longer "copying".
            ProgressEvent::DeleteBegin => LivePhase::Deleting,
        };
        self.phase = self.phase.max(phase);
    }
}

/// Render the row's message, laid out for `width` columns. Pure state →
/// string (the row's only other content is the spinner), so the format
/// and the truncation are unit-testable.
///
/// Columns are counted as `char`s: a double-width name can still render
/// narrower than the budget, and indicatif's `{wide_msg}` makes the
/// final cut against the real terminal anyway.
fn render_live_row(state: &LiveRowState, width: usize) -> String {
    let totals = &state.totals;
    let copy_row = || {
        format!(
            "copying • {}/{} files • {}",
            totals.files,
            totals.manifest_files,
            format_bytes(totals.bytes)
        )
    };
    let (head, file) = match state.phase {
        // The pass deletes inside one blocking call, so there is no
        // count to move until it is over.
        LivePhase::Deleting => (
            "deleting • removing extraneous destination entries".to_string(),
            None,
        ),
        LivePhase::Copying => (copy_row(), state.current_file.as_deref()),
        LivePhase::Enumerating | LivePhase::Comparing => {
            if !state.diff_complete {
                match state.phase {
                    LivePhase::Enumerating => (
                        format!("enumerating • {} files found", totals.enumerated_files),
                        None,
                    ),
                    _ => (
                        format!(
                            "comparing • {} files found • {} to copy",
                            totals.enumerated_files, totals.manifest_files
                        ),
                        None,
                    ),
                }
            } else if totals.manifest_files == 0 {
                // The diff wanted nothing: the walk is the whole story.
                // (A mirror may still delete; that arrives as its own
                // phase.)
                (
                    format!("up to date • {} files checked", totals.enumerated_files),
                    None,
                )
            } else {
                // Need list final, first byte not yet observed — the
                // apply pipeline is already running, so the copy row is
                // the honest one.
                (copy_row(), state.current_file.as_deref())
            }
        }
    };
    fit_row(head, file, width)
}

/// Lay the counters and the current-file segment out inside `width`.
/// The counters come first and are never sacrificed for the file name.
fn fit_row(head: String, file: Option<&str>, width: usize) -> String {
    let mut row = truncate_columns(&head, width);
    let Some(file) = file else { return row };
    const SEPARATOR: &str = " • ";
    let remaining = width
        .saturating_sub(row.chars().count())
        .saturating_sub(SEPARATOR.chars().count());
    if remaining < MIN_FILE_SEGMENT {
        return row;
    }
    row.push_str(SEPARATOR);
    row.push_str(&truncate_path_head(file, remaining));
    row
}

/// Shorten a path from the LEFT: the tail names the file, the leading
/// directories are the disposable part.
fn truncate_path_head(path: &str, width: usize) -> String {
    let columns = path.chars().count();
    if columns <= width {
        return path.to_string();
    }
    let keep = width.saturating_sub(1);
    let mut out = String::from("…");
    out.extend(path.chars().skip(columns - keep));
    out
}

/// Hard bound for text that must not push the row past `width`.
fn truncate_columns(text: &str, width: usize) -> String {
    if text.chars().count() <= width {
        return text.to_string();
    }
    text.chars().take(width).collect()
}

/// Whether a live row attaches its sink to the transfer.
///
/// Two inputs, one rule. The caller must have asked for progress
/// (`-p`, or the interactive-TTY default), AND indicatif must be able
/// to draw: with stderr redirected the bar is hidden and draws nothing,
/// while attaching the sink would still gate blit-core's enumeration
/// heartbeat — the redirected log would lose its only liveness signal
/// and gain nothing. Not attaching there keeps the legacy
/// once-per-second lines flowing into the log (clp-2 residue c).
fn live_row_attaches(progress_requested: bool, bar_hidden: bool) -> bool {
    progress_requested && !bar_hidden
}

/// The row's output surface. `ProgressBar` in production; a test
/// substitutes a recorder so the drain loop runs without a terminal.
trait RowOutput: Send + Sync + 'static {
    fn set_message(&self, message: String);
    fn println(&self, line: &str);
}

/// A wire path may legally carry control bytes (a newline-bearing
/// filename is valid on unix) or ANSI escapes; rendered raw they break
/// the one-row invariant or smuggle sequences into the terminal.
/// Rendered text only — the transfer always uses the untouched path.
fn sanitize_row_text(text: &str) -> String {
    text.chars()
        .map(|c| if c.is_control() { '\u{FFFD}' } else { c })
        .collect()
}

/// The sink the `log` backend routes through while a row is live — a
/// named seam so the CLI half of the redirect wiring is provable
/// without a terminal (the backend half is guarded in `stderr_log`).
/// Backend lines interpolate wire filenames (an unreadable entry's
/// warn), so they get the same control-byte sanitizing as every other
/// rendered text path.
fn row_line_sink<O: RowOutput>(output: O) -> blit_core::stderr_log::LineSink {
    std::sync::Arc::new(move |line: &str| output.println(&sanitize_row_text(line)))
}

/// What the writer thread applies to the terminal. One ordered channel
/// carries both kinds so `-v` lines and repaints interleave exactly as
/// produced.
enum RowWrite {
    Message(String),
    Line(String),
}

/// Non-blocking handle the async side holds (cr-clp2-2): terminal I/O
/// — indicatif's internal lock plus the stderr write — lives on the
/// dedicated writer thread, so the drain task and the log-redirect
/// closure only ever perform an unbounded channel send.
#[derive(Clone)]
struct ThreadedRowOutput(std::sync::mpsc::Sender<RowWrite>);

impl RowOutput for ThreadedRowOutput {
    fn set_message(&self, message: String) {
        // A send after the writer exited has nowhere to draw; dropping
        // it is the degraded path, never a panic on shutdown races.
        let _ = self.0.send(RowWrite::Message(message));
    }

    fn println(&self, line: &str) {
        let _ = self.0.send(RowWrite::Line(line.to_string()));
    }
}

/// The writer thread's body: apply queued writes until every sender is
/// gone. Generic over the output so the ordering contract is testable
/// on a recorder.
fn row_writer_loop<O: RowOutput>(rx: std::sync::mpsc::Receiver<RowWrite>, output: O) {
    for write in rx {
        match write {
            RowWrite::Message(message) => output.set_message(message),
            RowWrite::Line(line) => output.println(&line),
        }
    }
}

impl RowOutput for ProgressBar {
    fn set_message(&self, message: String) {
        ProgressBar::set_message(self, message);
    }

    fn println(&self, line: &str) {
        ProgressBar::println(self, line);
    }
}

/// Consume the progress lane into one row until the channel closes.
///
/// Extracted from the spawn so the event → state → repaint decision is
/// testable end to end (feed a channel, assert the rendered row).
/// Events are drained continuously — the pipeline must never wait on
/// the renderer — and folded into one repaint per tick.
async fn drain_progress_lane(
    mut rx: mpsc::UnboundedReceiver<ProgressEvent>,
    output: impl RowOutput,
    verbose: bool,
) {
    let mut state = LiveRowState::default();
    let mut pending_repaint = false;
    let mut ticker = interval(ROW_REFRESH);
    ticker.set_missed_tick_behavior(MissedTickBehavior::Skip);
    loop {
        // Unbiased: the lane is unbounded so a producer never blocks on
        // the renderer regardless of poll order, and fair polling keeps
        // a hot event stream from starving the tick-arm repaint.
        tokio::select! {
            event = rx.recv() => match event {
                Some(event) => {
                    if verbose {
                        if let ProgressEvent::FileComplete { path } = &event {
                            // Through the row's handle, never raw
                            // stderr: the line scrolls above the row and
                            // the row is redrawn intact underneath it.
                            output.println(&sanitize_row_text(path));
                        }
                    }
                    state.apply(&event);
                    pending_repaint = true;
                }
                None => break,
            },
            _ = ticker.tick() => {
                if pending_repaint {
                    output.set_message(render_live_row(&state, ROW_COLUMNS));
                    pending_repaint = false;
                }
            }
        }
    }
    if pending_repaint {
        output.set_message(render_live_row(&state, ROW_COLUMNS));
    }
}

/// Wait for the consumer to finish draining, bounded by `grace`.
/// Returns false when the grace expired and the consumer was aborted: a
/// blocking enumeration task cannot be aborted (`spawn_blocking`), so a
/// failed session can still hold a sink clone and keep the lane open —
/// the exit path must not hang on the row.
async fn join_drained(consumer: JoinHandle<()>, grace: Duration) -> bool {
    let abort = consumer.abort_handle();
    if tokio::time::timeout(grace, consumer).await.is_err() {
        abort.abort();
        return false;
    }
    true
}

/// The live status row: one `ProgressBar` on stderr plus the task that
/// drains the transfer's progress lane into it. While this is alive it
/// is the only writer of transfer-time stderr — anything that must
/// print goes through [`ProgressBar::println`] / `suspend`, never raw
/// `eprintln!` (that is what scrolled the pre-clp spinner off-row).
/// That includes the `log` facade: `_log_redirect` routes every backend
/// line through the same handle for the row's lifetime.
struct LiveProgressRow {
    bar: ProgressBar,
    consumer: JoinHandle<()>,
    writer: std::thread::JoinHandle<()>,
    _log_redirect: blit_core::stderr_log::LineRedirect,
}

impl LiveProgressRow {
    /// Build the row and the sink the session reports into, or `None`
    /// when no row attaches (see [`live_row_attaches`]). A local
    /// session runs both roles in this process, so this ONE sink covers
    /// enumeration, diff, apply, and delete events, and this ONE task
    /// consumes them.
    fn start(
        progress_requested: bool,
        verbose: bool,
        mirror: bool,
        src_path: &Path,
        dest_path: &Path,
    ) -> Option<(RemoteTransferProgress, Self)> {
        if !progress_requested {
            // Probe nothing: constructing a bar on a terminal and
            // dropping it writes to stderr, and a run that asked for no
            // progress must touch stderr exactly as it did before.
            return None;
        }
        // Whether indicatif can draw is a property of the draw target it
        // picks for stderr, so the bar has to exist to answer it. A
        // hidden bar draws nothing on drop.
        let bar = ProgressBar::new_spinner();
        if !live_row_attaches(progress_requested, bar.is_hidden()) {
            return None;
        }
        bar.set_style(
            // `{wide_msg}` truncates the message to the live terminal
            // width on every draw — the row cannot wrap, whatever the
            // renderer produced or however the terminal is resized.
            ProgressStyle::with_template("{spinner} {wide_msg}")
                .unwrap()
                .tick_strings(&["-", "\\", "|", "/"]),
        );
        bar.enable_steady_tick(Duration::from_millis(120));

        // clp-2 residue (a): the `log` backend writes raw stderr, and a
        // library warn (an unreadable source entry, a contained write
        // failure) scrolls the row exactly as the old enumeration line
        // did. Route it through the row for as long as the row lives;
        // the guard restores the raw backend even on the error path.
        // cr-clp2-2: terminal I/O rides a dedicated writer thread; the
        // async side (drain task, log-redirect closure) only performs
        // unbounded channel sends. One channel keeps `-v` lines and
        // repaints in production order.
        let (write_tx, write_rx) = std::sync::mpsc::channel::<RowWrite>();
        let writer = std::thread::spawn({
            let bar = bar.clone();
            move || row_writer_loop(write_rx, bar)
        });
        let threaded = ThreadedRowOutput(write_tx);
        // Until the first event lands the row still says what is running.
        threaded.set_message(format!(
            "{} {} → {}",
            if mirror { "Mirroring" } else { "Copying" },
            src_path.display(),
            dest_path.display()
        ));
        let log_redirect = blit_core::stderr_log::redirect_lines(row_line_sink(threaded.clone()));

        let (tx, rx) = mpsc::unbounded_channel::<ProgressEvent>();
        let consumer = tokio::spawn(drain_progress_lane(rx, threaded, verbose));

        Some((
            RemoteTransferProgress::new(tx),
            Self {
                bar,
                consumer,
                writer,
                _log_redirect: log_redirect,
            },
        ))
    }

    /// Let the consumer drain, then clear the row so the summary owns
    /// the terminal (the pre-clp `finish_and_clear`, unchanged). The log
    /// redirect is restored when `self` drops here.
    ///
    /// cr-clp2-3: a SUCCESSFUL session has dropped every sink clone, so
    /// the lane provably closes and the consumer ends on its own —
    /// waiting unbounded loses nothing and never discards queued `-v`
    /// lines. The bounded grace + abort exists only for the FAILED
    /// path, where a blocking enumeration task can survive holding a
    /// sink clone that keeps the lane open.
    async fn finish(self, session_succeeded: bool) {
        let Self {
            bar,
            consumer,
            writer,
            _log_redirect: log_redirect,
        } = self;
        drain_for_outcome(consumer, session_succeeded, ROW_DRAIN_GRACE).await;
        // Restore the backend now: its sender must drop for the writer
        // thread to see the channel close.
        drop(log_redirect);
        // The writer applies what is queued and ends. Join off the
        // async thread; a wedged terminal degrades to detaching it.
        let _ = tokio::time::timeout(
            ROW_DRAIN_GRACE,
            tokio::task::spawn_blocking(move || {
                let _ = writer.join();
            }),
        )
        .await;
        bar.finish_and_clear();
    }
}

/// The finish-path drain decision (cr-clp2-3), extracted so the
/// success/failure split is testable: success ⇒ unbounded await of a
/// provably closing lane; failure ⇒ bounded grace + abort.
async fn drain_for_outcome(
    consumer: JoinHandle<()>,
    session_succeeded: bool,
    grace: Duration,
) -> bool {
    if session_succeeded {
        consumer.await.is_ok()
    } else {
        join_drained(consumer, grace).await
    }
}

fn build_local_options(
    ctx: &AppContext,
    args: &TransferArgs,
    mirror: bool,
    move_verb: bool,
) -> Result<LocalMirrorOptions> {
    use blit_core::transfer_session::{LocalCompareMode, LocalMirrorDeleteScope};

    // R58-F7: translate the per-flag CLI args into the unified
    // LocalCompareMode enum. The session then resolves it onto the
    // proper ComparisonMode for the diff_planner. Pre-fix only
    // --checksum was honored; --size-only / --ignore-times /
    // --force were silently dropped.
    //
    // Priority follows the pull-side ordering at
    // pull.rs:538-547: ignore_times > force > size_only >
    // checksum > default. This keeps local and pull behaviorally
    // identical when given the same flag combination.
    //
    // codex otp-10b-2 F3: a MOVE maps through the move rule instead
    // (IgnoreTimes, or Checksum when asked) — the local twin of
    // `blit_app::transfers::compare::move_comparison_mode`. Today the
    // non-mirror local path copies unconditionally regardless of the
    // compare mode (probed live at the F3 adjudication), so this is
    // defense-in-depth; it becomes load-bearing at otp-11, when local
    // transfers ride the session and its diff WOULD skip a same-size
    // same-mtime changed file — which move's source-delete then turns
    // into data loss. Pinned by
    // `local_move_lands_source_bytes_over_same_size_same_mtime_destination`.
    // The metadata flags are rejected on move upstream (R54-F2 gates).
    let compare_mode = if move_verb {
        if args.checksum {
            LocalCompareMode::Checksum
        } else {
            LocalCompareMode::IgnoreTimes
        }
    } else if args.ignore_times {
        LocalCompareMode::IgnoreTimes
    } else if args.force {
        LocalCompareMode::Force
    } else if args.size_only {
        LocalCompareMode::SizeOnly
    } else if args.checksum {
        LocalCompareMode::Checksum
    } else {
        LocalCompareMode::SizeMtime
    };

    // R58-F6: --delete-scope is now plumbed through to local
    // mirror. The CLI exposes `subset` (default — filter scope)
    // and `all`. Pre-fix LocalMirrorOptions had no field for
    // this and apply_mirror_deletions always operated through
    // the user's filter, then failed with ENOTEMPTY on dirs
    // containing excluded contents.
    let delete_scope = if args.delete_scope_all() {
        LocalMirrorDeleteScope::All
    } else {
        LocalMirrorDeleteScope::FilteredSubset
    };

    let mut options = LocalMirrorOptions {
        mirror,
        dry_run: args.dry_run,
        verbose: args.verbose,
        progress: args.effective_progress(),
        perf_history: ctx.perf_history_enabled,
        checksum: args.checksum,
        ignore_existing: args.ignore_existing,
        drop_windows_metadata: args.drop_windows_metadata,
        compare_mode,
        delete_scope,
        resume: args.resume,
        null_sink: args.null,
        filter: super::build_filter(args)?,
        ..LocalMirrorOptions::default()
    };
    if let Some(workers) = args.workers {
        options.workers = workers.max(1);
        options.debug_mode = true;
    }
    Ok(options)
}

/// Threshold below which the `• Throughput / Workers used` line is noise:
/// short transfers (startup-dominated) or single-file copies produce
/// misleading numbers (e.g. "184 B/s" on an NVMe). Keep it for bulk
/// transfers where it's meaningful.
const THROUGHPUT_LINE_MIN_BYTES: u64 = 1024 * 1024; // 1 MiB

fn print_summary(
    mirror: bool,
    dry_run: bool,
    null_sink: bool,
    verbose: bool,
    debug_mode: bool,
    workers: usize,
    summary: &LocalMirrorSummary,
    elapsed: Duration,
) {
    let operation = if mirror { "Mirror" } else { "Copy" };
    let suffix = if dry_run {
        " (dry run)"
    } else if null_sink {
        " (null sink — writes discarded)"
    } else {
        ""
    };
    let duration = if summary.duration.is_zero() {
        elapsed
    } else {
        summary.duration
    };

    // Distinguish the three legitimate zero-files cases from the normal
    // "transferred N files" case. Previously all four printed identically,
    // which masked two classes of bugs (rsync-semantics, single-file noop).
    match summary.outcome {
        TransferOutcome::UpToDate => {
            println!(
                "Up to date: {} files examined, 0 changed{} (in {:.2?})",
                summary.scanned_files, suffix, duration
            );
            return;
        }
        TransferOutcome::SourceEmpty => {
            println!(
                "Source is empty: 0 files copied{} (in {:.2?})",
                suffix, duration
            );
            return;
        }
        TransferOutcome::Transferred => {}
    }

    println!(
        "{}{} complete: {} files, {} in {:.2?}",
        operation,
        suffix,
        summary.copied_files,
        format_bytes(summary.total_bytes),
        duration
    );

    if summary.deleted_files > 0 || summary.deleted_dirs > 0 {
        println!(
            "• Deleted: {} file(s), {} dir(s)",
            summary.deleted_files, summary.deleted_dirs
        );
    }

    // Suppress throughput/workers noise on small transfers where startup
    // dominates wall time and the numbers are meaningless. Keep it for
    // bulk transfers where it's actually informative.
    let show_throughput =
        verbose || summary.total_bytes >= THROUGHPUT_LINE_MIN_BYTES || summary.copied_files > 1;
    if show_throughput {
        let throughput = if duration.as_secs_f64() > 0.0 {
            summary.total_bytes as f64 / duration.as_secs_f64()
        } else {
            0.0
        };
        // codex otp-11b B4: the session's apply pipeline runs one sink
        // worker unless the hidden debug limiter widened it — print
        // the EFFECTIVE count, not the options default (num_cpus).
        println!(
            "• Throughput: {} | Workers used: {}",
            format_bps(throughput as u64),
            if debug_mode { workers } else { 1 }
        );
    }
    if debug_mode {
        println!("• Debug limiter active – worker cap {} worker(s)", workers);
    }

    if verbose {
        println!(
            "• Planned {} file(s), total bytes {}",
            summary.planned_files,
            format_bytes(summary.total_bytes)
        );
        if summary.tar_shard_tasks > 0 || summary.raw_bundle_tasks > 0 || summary.large_tasks > 0 {
            println!(
                "• Planner mix: {} tar shard(s) [{} file(s), {}], {} bundle(s) [{} file(s), {}], {} large task(s) [{}]",
                summary.tar_shard_tasks,
                summary.tar_shard_files,
                format_bytes(summary.tar_shard_bytes),
                summary.raw_bundle_tasks,
                summary.raw_bundle_files,
                format_bytes(summary.raw_bundle_bytes),
                summary.large_tasks,
                format_bytes(summary.large_bytes),
            );
        }
    }
}

fn print_summary_json(
    mirror: bool,
    summary: &LocalMirrorSummary,
    elapsed: Duration,
    src: &Path,
    dst: &Path,
) {
    use serde_json::json;
    let duration = if summary.duration.is_zero() {
        elapsed
    } else {
        summary.duration
    };
    let outcome = match summary.outcome {
        TransferOutcome::Transferred => "transferred",
        TransferOutcome::UpToDate => "up_to_date",
        TransferOutcome::SourceEmpty => "source_empty",
    };
    let output = json!({
        "operation": if mirror { "mirror" } else { "copy" },
        "source": src.to_string_lossy(),
        "destination": dst.to_string_lossy(),
        "files_transferred": summary.copied_files,
        "files_examined": summary.scanned_files,
        "total_bytes": summary.total_bytes,
        "deleted_files": summary.deleted_files,
        "deleted_dirs": summary.deleted_dirs,
        "duration_ms": duration.as_millis() as u64,
        "dry_run": summary.dry_run,
        "outcome": outcome,
    });
    println!("{}", serde_json::to_string_pretty(&output).unwrap());
}

#[cfg(test)]
mod live_row_tests {
    use super::*;

    fn fold(events: &[ProgressEvent]) -> LiveRowState {
        let mut state = LiveRowState::default();
        for event in events {
            state.apply(event);
        }
        state
    }

    /// Before the diff has decided anything, the row reports the source
    /// walk — the count that used to arrive as a raw stderr line.
    #[test]
    fn enumerating_row_reports_the_walk_count() {
        let state = fold(&[
            ProgressEvent::Enumerated { files: 900 },
            ProgressEvent::Enumerated { files: 100 },
        ]);
        assert_eq!(state.phase, LivePhase::Enumerating);
        assert_eq!(
            render_live_row(&state, ROW_COLUMNS),
            "enumerating • 1000 files found"
        );
    }

    /// Once the destination diff reports needed files, the row shows the
    /// walk count and the transfer denominator side by side — the walk
    /// lane must not inflate the denominator.
    #[test]
    fn comparing_row_reports_walk_and_needed_counts() {
        let state = fold(&[
            ProgressEvent::Enumerated { files: 1000 },
            ProgressEvent::ManifestBatch {
                files: 12,
                bytes: 4096,
            },
        ]);
        assert_eq!(state.phase, LivePhase::Comparing);
        assert_eq!(
            render_live_row(&state, ROW_COLUMNS),
            "comparing • 1000 files found • 12 to copy"
        );
    }

    /// The copy row: files completed over files needed, plus bytes
    /// written. Bytes ride `Payload` only (the w6-1 contract), so the
    /// pair below counts one file and its bytes exactly once.
    /// (clp-2 adapted the expected string: the row now ends with the
    /// current-file segment, here the most recent completion.)
    #[test]
    fn copying_row_reports_completed_of_needed_and_bytes() {
        let state = fold(&[
            ProgressEvent::Enumerated { files: 3 },
            ProgressEvent::ManifestBatch {
                files: 2,
                bytes: 2048,
            },
            ProgressEvent::Payload {
                files: 0,
                bytes: 1024,
            },
            ProgressEvent::FileComplete {
                path: "a.txt".into(),
            },
            ProgressEvent::Payload {
                files: 0,
                bytes: 1024,
            },
            ProgressEvent::FileComplete {
                path: "b.txt".into(),
            },
        ]);
        assert_eq!(state.phase, LivePhase::Copying);
        assert_eq!(
            render_live_row(&state, ROW_COLUMNS),
            "copying • 2/2 files • 2.00 KiB • b.txt"
        );
    }

    /// pfc-2 (D-2026-07-30-1): a file whose write failed is contained
    /// and never reports a completion, while its bytes were already
    /// reported. The row shows the honest completed count — and, since
    /// clp-2, the last file that did complete (the expected string grew
    /// that segment; the counts are unchanged).
    #[test]
    fn contained_file_failure_is_not_counted_complete() {
        let state = fold(&[
            ProgressEvent::ManifestBatch {
                files: 2,
                bytes: 200,
            },
            ProgressEvent::Payload {
                files: 0,
                bytes: 100,
            },
            ProgressEvent::FileComplete {
                path: "ok.txt".into(),
            },
            ProgressEvent::Payload {
                files: 0,
                bytes: 100,
            },
        ]);
        assert_eq!(
            render_live_row(&state, ROW_COLUMNS),
            "copying • 1/2 files • 200 B • ok.txt"
        );
    }

    /// The phase never regresses: a local session interleaves the source
    /// walk with the destination's diff and applies, so enumeration
    /// events keep arriving after the first byte lands.
    #[test]
    fn phase_never_moves_backwards() {
        let state = fold(&[
            ProgressEvent::ManifestBatch {
                files: 1,
                bytes: 10,
            },
            ProgressEvent::Payload {
                files: 0,
                bytes: 10,
            },
            ProgressEvent::FileComplete {
                path: "a.txt".into(),
            },
            ProgressEvent::Enumerated { files: 5 },
            ProgressEvent::ManifestBatch {
                files: 1,
                bytes: 10,
            },
        ]);
        assert_eq!(state.phase, LivePhase::Copying);
        assert_eq!(
            render_live_row(&state, ROW_COLUMNS),
            "copying • 1/2 files • 10 B • a.txt"
        );
    }

    /// clp-2: an up-to-date tree used to say "enumerating" for the whole
    /// run — the diff needed nothing, so no event ever moved the phase.
    /// The diff-complete signal is what distinguishes "nothing to do"
    /// from "still scanning".
    #[test]
    fn diff_complete_with_nothing_needed_reports_up_to_date() {
        let state = fold(&[
            ProgressEvent::Enumerated { files: 1000 },
            ProgressEvent::DiffComplete,
        ]);
        assert_eq!(
            render_live_row(&state, ROW_COLUMNS),
            "up to date • 1000 files checked"
        );
    }

    /// With the need list final and files to move, the row is the copy
    /// row from the first tick — the apply pipeline is already running,
    /// so "comparing" would be stale.
    #[test]
    fn diff_complete_with_work_pending_shows_the_copy_row() {
        let state = fold(&[
            ProgressEvent::Enumerated { files: 10 },
            ProgressEvent::ManifestBatch {
                files: 4,
                bytes: 4096,
            },
            ProgressEvent::DiffComplete,
        ]);
        assert_eq!(
            render_live_row(&state, ROW_COLUMNS),
            "copying • 0/4 files • 0 B"
        );
    }

    /// clp-2: the mirror-delete pass used to render as "copying" — the
    /// copy was over and the row still claimed to be moving bytes.
    #[test]
    fn delete_pass_reports_deleting_and_outranks_the_copy_phase() {
        let state = fold(&[
            ProgressEvent::ManifestBatch {
                files: 1,
                bytes: 10,
            },
            ProgressEvent::Payload {
                files: 0,
                bytes: 10,
            },
            ProgressEvent::FileComplete {
                path: "a.txt".into(),
            },
            ProgressEvent::DiffComplete,
            ProgressEvent::DeleteBegin,
        ]);
        assert_eq!(state.phase, LivePhase::Deleting);
        assert_eq!(
            render_live_row(&state, ROW_COLUMNS),
            "deleting • removing extraneous destination entries"
        );
    }

    /// The current-file segment is the row's last field and the only one
    /// that may be shortened: the counters always survive, and the file
    /// name keeps its tail (the leading directories are disposable).
    #[test]
    fn a_long_current_file_is_truncated_from_the_left() {
        let state = fold(&[
            ProgressEvent::ManifestBatch {
                files: 1,
                bytes: 10,
            },
            ProgressEvent::Payload {
                files: 0,
                bytes: 10,
            },
            ProgressEvent::FileComplete {
                path: "deeply/nested/directory/tree/with/a/long/path/report.pdf".into(),
            },
        ]);
        let row = render_live_row(&state, ROW_COLUMNS);
        assert!(
            row.chars().count() <= ROW_COLUMNS,
            "the row must fit its budget: {row:?}"
        );
        assert!(
            row.starts_with("copying • 1/1 files • 10 B • "),
            "the counters survive intact: {row:?}"
        );
        assert!(
            row.ends_with("report.pdf"),
            "the name's tail identifies the file: {row:?}"
        );
        assert!(row.contains('…'), "the cut is marked: {row:?}");
    }

    /// No width, no wrap: even a pathological name on a narrow terminal
    /// stays inside the budget, and a budget too small for a useful
    /// segment drops it rather than rendering an ellipsis and a letter.
    #[test]
    fn no_width_lets_the_current_file_push_the_row_over() {
        let state = fold(&[
            ProgressEvent::ManifestBatch {
                files: 1,
                bytes: 10,
            },
            ProgressEvent::FileComplete {
                path: "x".repeat(500),
            },
        ]);
        for width in [0usize, 1, 8, 20, 40, 80, 200] {
            let row = render_live_row(&state, width);
            assert!(
                row.chars().count() <= width,
                "width {width} exceeded by {row:?}"
            );
        }
        assert_eq!(
            render_live_row(&state, 30),
            "copying • 1/1 files • 0 B",
            "no room for a useful segment: the counters stand alone"
        );
    }

    /// Multi-byte names are cut on character boundaries, never mid-byte
    /// (a byte slice here would panic).
    #[test]
    fn truncation_respects_character_boundaries() {
        let name = "报告/".repeat(40);
        let cut = truncate_path_head(&name, 10);
        assert_eq!(cut.chars().count(), 10);
        assert!(cut.starts_with('…'));
        assert!(name.ends_with(cut.trim_start_matches('…')));
    }
}

/// The drain loop and the row-attachment rule — the parts of the row
/// that are decisions rather than formatting.
#[cfg(test)]
mod live_row_loop_tests {
    use super::*;
    use std::sync::{Arc, Mutex};

    #[derive(Default)]
    struct Recorded {
        messages: Vec<String>,
        lines: Vec<String>,
    }

    /// Stands in for the `ProgressBar`, so the consumer task runs
    /// exactly as it does in production without a terminal.
    #[derive(Clone, Default)]
    struct Recorder(Arc<Mutex<Recorded>>);

    impl Recorder {
        fn last_message(&self) -> String {
            self.0
                .lock()
                .expect("recorder poisoned")
                .messages
                .last()
                .cloned()
                .unwrap_or_default()
        }

        fn lines(&self) -> Vec<String> {
            self.0.lock().expect("recorder poisoned").lines.clone()
        }

        fn messages(&self) -> Vec<String> {
            self.0.lock().expect("recorder poisoned").messages.clone()
        }
    }

    impl RowOutput for Recorder {
        fn set_message(&self, message: String) {
            self.0
                .lock()
                .expect("recorder poisoned")
                .messages
                .push(message);
        }

        fn println(&self, line: &str) {
            self.0
                .lock()
                .expect("recorder poisoned")
                .lines
                .push(line.to_string());
        }
    }

    /// clp-2 residue (a), CLI half: the named seam the row installs into
    /// the log backend delivers a formatted line to the row's handle.
    #[test]
    fn a_backend_line_routes_to_the_row_handle() {
        let recorder = Recorder::default();
        let sink = row_line_sink(recorder.clone());
        sink("blit: warn: scan skipping 'x' (denied)");
        assert_eq!(
            recorder.lines(),
            vec!["blit: warn: scan skipping 'x' (denied)".to_string()],
            "the redirect sink must hand backend lines to the row output"
        );
    }

    /// cr-clp2-2: the writer thread applies every queued write, both
    /// kinds, in production order, and ends when the senders drop.
    #[test]
    fn the_writer_loop_applies_queued_writes_in_order() {
        let (tx, rx) = std::sync::mpsc::channel();
        let recorder = Recorder::default();
        let out = ThreadedRowOutput(tx);
        out.set_message("m1".into());
        out.println("l1");
        out.set_message("m2".into());
        drop(out);
        row_writer_loop(rx, recorder.clone());
        assert_eq!(
            recorder.messages(),
            vec!["m1".to_string(), "m2".to_string()],
            "messages arrive in order"
        );
        assert_eq!(recorder.lines(), vec!["l1".to_string()], "lines arrive");
    }

    /// cr-clp2-1: a routed warn interpolating a control-byte filename
    /// must not break the row or smuggle escapes to the terminal.
    #[test]
    fn a_backend_line_with_control_bytes_is_sanitized() {
        let recorder = Recorder::default();
        let sink = row_line_sink(recorder.clone());
        sink("blit: warn: scan skipping 'evil\ndir/\x1b[31mx' (denied)");
        let lines = recorder.lines();
        assert!(
            lines.len() == 1 && !lines[0].contains('\n') && !lines[0].contains('\x1b'),
            "routed backend lines must be sanitized: {lines:?}"
        );
    }

    /// The mid-transfer repaint: while the lane stays open, the tick arm
    /// (not the post-loop flush) must render pending state.
    #[tokio::test(start_paused = true)]
    async fn the_ticker_repaints_while_the_lane_stays_open() {
        let (tx, rx) = mpsc::unbounded_channel();
        let recorder = Recorder::default();
        let consumer = tokio::spawn(drain_progress_lane(rx, recorder.clone(), false));
        tx.send(ProgressEvent::Enumerated { files: 7 })
            .expect("send");
        // Paused clock: sleeping past the refresh interval fires the
        // ticker while the sender is still alive.
        tokio::time::sleep(3 * ROW_REFRESH).await;
        assert!(
            recorder.last_message().contains("enumerating"),
            "the tick arm must repaint mid-lane, got {:?}",
            recorder.last_message()
        );
        drop(tx);
        consumer.await.expect("consumer");
    }

    /// A newline-bearing filename must not break the one-row invariant
    /// on the row message or the -v line.
    #[tokio::test]
    async fn control_bytes_in_a_path_cannot_break_the_row() {
        let (tx, rx) = mpsc::unbounded_channel();
        let recorder = Recorder::default();
        let consumer = tokio::spawn(drain_progress_lane(rx, recorder.clone(), true));
        tx.send(ProgressEvent::ManifestBatch { files: 1, bytes: 4 })
            .expect("send");
        tx.send(ProgressEvent::FileComplete {
            path: "evil\ndir/\x1b[31mname.txt".to_string(),
        })
        .expect("send");
        drop(tx);
        consumer.await.expect("consumer");
        assert!(
            !recorder.last_message().contains('\n') && !recorder.last_message().contains('\x1b'),
            "control bytes must be sanitized out of the row: {:?}",
            recorder.last_message()
        );
        let lines = recorder.lines();
        assert!(
            lines.len() == 1 && !lines[0].contains('\n') && !lines[0].contains('\x1b'),
            "control bytes must be sanitized out of -v lines: {lines:?}"
        );
    }

    /// clp-2 residue (b): drive the whole consumer through a channel and
    /// assert what the terminal would show. The closed lane ends the
    /// loop, which repaints the state it drained.
    #[tokio::test]
    async fn the_drain_loop_renders_the_final_state_of_the_lane() {
        let (tx, rx) = mpsc::unbounded_channel();
        let recorder = Recorder::default();
        for event in [
            ProgressEvent::Enumerated { files: 3 },
            ProgressEvent::ManifestBatch {
                files: 2,
                bytes: 2048,
            },
            ProgressEvent::DiffComplete,
            ProgressEvent::Payload {
                files: 0,
                bytes: 1024,
            },
            ProgressEvent::FileComplete {
                path: "one.txt".into(),
            },
            ProgressEvent::Payload {
                files: 0,
                bytes: 1024,
            },
            ProgressEvent::FileComplete {
                path: "two.txt".into(),
            },
        ] {
            tx.send(event).expect("the consumer is alive");
        }
        drop(tx);

        drain_progress_lane(rx, recorder.clone(), false).await;

        assert_eq!(
            recorder.last_message(),
            "copying • 2/2 files • 2.00 KiB • two.txt"
        );
        assert!(
            recorder.lines().is_empty(),
            "no per-file lines without -v: {:?}",
            recorder.lines()
        );
    }

    /// `-v` with a live row: each completion prints through the row's
    /// handle (never raw stderr) and the row keeps rendering underneath.
    #[tokio::test]
    async fn verbose_prints_each_completion_through_the_row() {
        let (tx, rx) = mpsc::unbounded_channel();
        let recorder = Recorder::default();
        for event in [
            ProgressEvent::ManifestBatch {
                files: 2,
                bytes: 20,
            },
            ProgressEvent::FileComplete {
                path: "one.txt".into(),
            },
            ProgressEvent::FileComplete {
                path: "dir/two.txt".into(),
            },
        ] {
            tx.send(event).expect("the consumer is alive");
        }
        drop(tx);

        drain_progress_lane(rx, recorder.clone(), true).await;

        assert_eq!(recorder.lines(), vec!["one.txt", "dir/two.txt"]);
        assert_eq!(
            recorder.last_message(),
            "copying • 2/2 files • 0 B • dir/two.txt"
        );
    }

    /// The mirror-delete phase survives the whole loop, not just the
    /// fold: the last thing the terminal shows before the summary is the
    /// delete pass, not a stale copy row.
    #[tokio::test]
    async fn the_drain_loop_ends_on_the_delete_phase() {
        let (tx, rx) = mpsc::unbounded_channel();
        let recorder = Recorder::default();
        for event in [
            ProgressEvent::Enumerated { files: 4 },
            ProgressEvent::DiffComplete,
            ProgressEvent::DeleteBegin,
        ] {
            tx.send(event).expect("the consumer is alive");
        }
        drop(tx);

        drain_progress_lane(rx, recorder.clone(), false).await;

        assert_eq!(
            recorder.last_message(),
            "deleting • removing extraneous destination entries"
        );
    }

    /// The finish path waits for the consumer when the lane closes.
    #[tokio::test]
    async fn finish_waits_for_a_consumer_that_ends() {
        let (tx, rx) = mpsc::unbounded_channel();
        let recorder = Recorder::default();
        tx.send(ProgressEvent::Enumerated { files: 1 })
            .expect("the consumer is alive");
        drop(tx);
        let consumer = tokio::spawn(drain_progress_lane(rx, recorder.clone(), false));

        assert!(join_drained(consumer, ROW_DRAIN_GRACE).await);
        assert_eq!(recorder.last_message(), "enumerating • 1 files found");
    }

    /// A slow output stands in for a backpressured terminal: `println`
    /// costs real time, so a queued backlog takes longer than any
    /// grace to drain.
    #[derive(Clone)]
    struct SlowRecorder(Recorder, Duration);

    impl RowOutput for SlowRecorder {
        fn set_message(&self, message: String) {
            self.0.set_message(message);
        }

        fn println(&self, line: &str) {
            std::thread::sleep(self.1);
            self.0.println(line);
        }
    }

    /// cr-clp2-3: a SUCCESSFUL session drains the whole closed lane —
    /// queued `-v` lines are never discarded by the failure-path grace,
    /// however long the terminal takes.
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn a_successful_session_drains_every_queued_line_past_the_grace() {
        let (tx, rx) = mpsc::unbounded_channel();
        let slow = SlowRecorder(Recorder::default(), Duration::from_millis(2));
        for i in 0..100 {
            tx.send(ProgressEvent::FileComplete {
                path: format!("f{i}"),
            })
            .expect("send");
        }
        drop(tx);
        let consumer = tokio::spawn(drain_progress_lane(rx, slow.clone(), true));
        // 100 × 2 ms of output far exceeds this grace; only the
        // success branch (unbounded await) can deliver every line.
        assert!(drain_for_outcome(consumer, true, Duration::from_millis(50)).await);
        assert_eq!(
            slow.0.lines().len(),
            100,
            "no queued -v line may be dropped on a successful session"
        );
    }

    /// A sink clone that outlived the session keeps the lane open
    /// forever. The bounded drain gives up and aborts instead of
    /// hanging the process exit. (The grace is a parameter so the test
    /// does not spend the production half-second waiting.)
    #[tokio::test]
    async fn finish_gives_up_on_a_lane_that_never_closes() {
        let (tx, rx) = mpsc::unbounded_channel();
        let recorder = Recorder::default();
        tx.send(ProgressEvent::Enumerated { files: 1 })
            .expect("the consumer is alive");
        let consumer = tokio::spawn(drain_progress_lane(rx, recorder.clone(), false));

        let started = Instant::now();
        let grace = Duration::from_millis(50);
        assert!(
            !join_drained(consumer, grace).await,
            "an open lane must hit the grace, not drain"
        );
        assert!(started.elapsed() >= grace);
        drop(tx);
    }

    /// clp-2 residue (c): the row attaches only when it can actually
    /// draw. With stderr redirected indicatif hides the bar, and a
    /// hidden row would gate blit-core's enumeration heartbeat while
    /// showing nothing — the log would lose its only liveness signal.
    #[test]
    fn the_row_attaches_only_when_requested_and_drawable() {
        assert!(live_row_attaches(true, false));
        assert!(
            !live_row_attaches(true, true),
            "redirected stderr keeps the legacy heartbeat lines"
        );
        assert!(!live_row_attaches(false, false));
        assert!(!live_row_attaches(false, true));
    }

    /// The gate the row is built on: with no row requested nothing is
    /// constructed and no sink is handed to the session.
    #[test]
    fn no_row_is_built_when_progress_was_not_requested() {
        assert!(
            LiveProgressRow::start(false, false, true, Path::new("src"), Path::new("dst"))
                .is_none()
        );
    }
}
