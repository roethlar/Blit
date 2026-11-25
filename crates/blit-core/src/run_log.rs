//! One run's log as the parts of a run see it (JOB_LOGS jl-1b, jl-2).
//!
//! The adapter between a transfer and its [`crate::job_log`] file, shared by
//! the daemon (one log per job and role) and the CLI (the initiator's log of
//! a whole command). It is made when the run begins and owned by whatever
//! outlives the transfer itself; started once the run's role is known; fed
//! by the transfer's bounded audit lane (review cr-jl1b-2) and by direct
//! notes; and closed with the run's summary. Closing names the failed files
//! the summary lists that were never seen live, ends the open phase, records
//! the diagnostics and the summary, then `run-end`, and finishes the file.
//!
//! JOB_LOGS jl-5 (FAST): a log can start before its place is known
//! ([`RunLog::start_with`]) — the place is found on the log's own thread
//! while the run goes — and closing comes in two halves
//! ([`RunLog::prepare_close`], then [`ClosingLog::finish`]), so a caller can
//! finish the file alongside other closing work.

use crate::display::format_bytes;
use crate::generated::TransferSummary;
use crate::job_log::{
    EventBody, LogHome, LogKey, LogReport, LogSender, LogWriter, Outcome, PhaseState, Role,
    RunInfo, RunTag, Summary,
};
use crate::remote::transfer::progress::{audit_lane, AuditSender, PlannedTotals, AUDIT_LANE_DEPTH};
use crate::remote::transfer::sink::FileFailure;
use crate::remote::transfer::ProgressEvent;
use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::sync::{Arc, Mutex, OnceLock};
use std::time::{Duration, Instant};

/// The reason a failed file the summary names gets when the summary's capped
/// list kept none for it and it was never seen live.
pub const NO_REASON_REPORTED: &str = "failed (the transfer's report kept no reason)";

/// How long closing a log waits for its audit lane to drain.
const RELAY_DRAIN_LIMIT: Duration = Duration::from_secs(30);

/// Where a machine keeps its logs, and whose they are.
#[derive(Clone, Debug)]
pub struct LogPlace {
    pub dir: PathBuf,
    /// This machine's ID ([`crate::job_log::machine_id`]).
    pub participant: String,
    /// Prune to the newest this many finished logs.
    pub keep: usize,
    /// Woken whenever a log here finishes (the daemon's `GetJobLog`
    /// `wait_finished`).
    pub finished: Option<Arc<tokio::sync::Notify>>,
}

impl LogPlace {
    /// Start the log of run `run_id`'s session `attempt` in `role`. `None`,
    /// with a warning, only when the ID cannot name a log; a log never stops
    /// a run.
    pub fn start(&self, run_id: &str, attempt: u32, role: Role, run: RunInfo) -> Option<LogWriter> {
        match LogKey::new(run_id, self.participant.clone(), role, attempt) {
            Ok(key) => Some(LogWriter::start(&self.dir, key, run, self.keep)),
            Err(error) => {
                log::warn!("run {run_id}: not logged: {error:#}");
                None
            }
        }
    }
}

/// A run's closing account, whatever route it took.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct RunTotals {
    pub files_copied: u64,
    pub files_deleted: u64,
    pub files_failed: u64,
    pub bytes_copied: u64,
    pub files_resumed: u64,
    /// Whether the payload rode the in-stream carrier; `None` where the
    /// question does not arise (a local run).
    pub in_stream_carrier: Option<bool>,
    /// The failed files by name; exact unless `failed_paths_truncated`.
    pub failed_paths: Vec<String>,
    /// cr-ecm34-5: the exact bytes of each of `failed_paths` whose name is
    /// not text, at its place in that list, and nothing at the place of one
    /// that is — one entry for every path, so a failure past the summary's
    /// reason cap is still named by its own bytes.
    pub failed_paths_raw: Vec<Vec<u8>>,
    pub failed_paths_truncated: bool,
    /// As many failed files as the report kept, each with its reason and
    /// — when its name is not its text — its own bytes (review
    /// cr-jl3afix1-1).
    pub failures: Vec<FileFailure>,
    /// The failed files whose write left this run's own incomplete copy at
    /// the destination, and whether that list is whole (review cr-jl4-1:
    /// an `--ignore-existing` retry must not skip them as existing).
    pub left_in_place: Vec<String>,
    pub left_in_place_truncated: bool,
    /// The same for names that are not UTF-8, by their exact bytes (review
    /// cr-jl4fix2-1).
    pub left_in_place_raw: Vec<Vec<u8>>,
}

impl RunTotals {
    /// Add `other`'s account to this one: the parts of one run.
    pub fn merge(&mut self, other: RunTotals) {
        self.files_copied += other.files_copied;
        self.files_deleted += other.files_deleted;
        self.files_failed += other.files_failed;
        self.bytes_copied += other.bytes_copied;
        self.files_resumed += other.files_resumed;
        self.in_stream_carrier = match (self.in_stream_carrier, other.in_stream_carrier) {
            (Some(a), Some(b)) => Some(a || b),
            (a, b) => a.or(b),
        };
        let mut other_raw = other.failed_paths_raw;
        other_raw.resize(other.failed_paths.len(), Vec::new());
        self.failed_paths_raw
            .resize(self.failed_paths.len(), Vec::new());
        self.failed_paths.extend(other.failed_paths);
        self.failed_paths_raw.extend(other_raw);
        self.failed_paths_truncated |= other.failed_paths_truncated;
        self.failures.extend(other.failures);
        self.left_in_place.extend(other.left_in_place);
        self.left_in_place_truncated |= other.left_in_place_truncated;
        self.left_in_place_raw.extend(other.left_in_place_raw);
    }
}

/// One failed file of a run's account, by its identity — its text, and its
/// bytes when its name is not text — with the reason the report kept for it,
/// when it kept one, and whether its failure left this run's copy
/// (cr-ecm35-3). One list, from [`RunTotals::failed_files`], for the log and
/// the saved record alike: each joined its failures to its paths by the
/// text alone, and two files of one text, the reason cap falling between
/// them, were named once — or the second without its bytes.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FailedFile {
    pub path: String,
    pub raw: Option<Vec<u8>>,
    pub reason: Option<String>,
    /// Whether its failure left this run's copy — `None` where the account
    /// does not say (cr-ecm35-5): a file past the reason cap missing from a
    /// left-in-place list that was cut short.
    pub left_in_place: Option<bool>,
}

impl RunTotals {
    /// Every failed file the account names, each once: the failures the
    /// report kept, then each failed path none of them is. A path is one of
    /// them by its identity when the account carries the paths' bytes, and
    /// by its text when it carries none (a peer that sent none).
    pub fn failed_files(&self) -> Vec<FailedFile> {
        let carried = self.failed_paths_raw.iter().any(|bytes| !bytes.is_empty());
        // cr-ecm37-7: the leftover lists by hash too, built once — searched
        // for each failed file, 24,000 files that all left a copy took
        // seconds.
        let left_text: std::collections::HashSet<&str> =
            self.left_in_place.iter().map(String::as_str).collect();
        let left_raw: std::collections::HashSet<&[u8]> =
            self.left_in_place_raw.iter().map(Vec::as_slice).collect();
        let left = |path: &str, raw: Option<&[u8]>| match raw {
            Some(bytes) => left_raw.contains(bytes),
            None => left_text.contains(path),
        };
        let mut files: Vec<FailedFile> = Vec::new();
        // cr-ecm36-4: by hash, not by searching the list for each — a run of
        // tens of thousands of failures took seconds.
        let mut named: std::collections::HashSet<(String, Option<Vec<u8>>)> =
            std::collections::HashSet::new();
        let mut named_texts: std::collections::HashSet<String> = std::collections::HashSet::new();
        for failure in &self.failures {
            // cr-ecm36-3: a path the report shortened is a rendering, not a
            // name: the exact name is among the failed paths.
            if failure.path_shortened {
                continue;
            }
            let raw = failure.raw_relative_path.clone();
            if !named.insert((failure.relative_path.clone(), raw.clone())) {
                continue;
            }
            named_texts.insert(failure.relative_path.clone());
            // A failure the report kept says it itself (contract 30) — and
            // cr-ecm37-3: one whose disposition the destination could not
            // tell is not known.
            let left_here = failure.left_in_place || left(&failure.relative_path, raw.as_deref());
            files.push(FailedFile {
                left_in_place: (left_here || !failure.left_unknown).then_some(left_here),
                path: failure.relative_path.clone(),
                raw,
                reason: Some(failure.reason.clone()),
            });
        }
        for (index, path) in self.failed_paths.iter().enumerate() {
            let raw = self
                .failed_paths_raw
                .get(index)
                .filter(|bytes| !bytes.is_empty())
                .cloned();
            let is_named = if carried {
                !named.insert((path.clone(), raw.clone()))
            } else {
                named_texts.contains(path) || !named.insert((path.clone(), raw.clone()))
            };
            if is_named {
                continue;
            }
            // cr-ecm35-5: one past the cap is told by the lists alone — and
            // missing from a list cut short, it is not known.
            let left_in_place = if left(path, raw.as_deref()) {
                Some(true)
            } else if self.left_in_place_truncated {
                None
            } else {
                Some(false)
            };
            files.push(FailedFile {
                left_in_place,
                path: path.clone(),
                raw,
                reason: None,
            });
        }
        files
    }
}

/// A summary's `failed_paths_raw` at their places: one entry for every
/// path (the wire sends either that, or none when no name needs bytes).
pub fn raw_at_their_places(paths: &[String], raw: &[Vec<u8>]) -> Vec<Vec<u8>> {
    if raw.len() == paths.len() {
        raw.to_vec()
    } else {
        vec![Vec::new(); paths.len()]
    }
}

impl From<&TransferSummary> for RunTotals {
    fn from(summary: &TransferSummary) -> Self {
        Self {
            files_copied: summary.files_transferred,
            files_deleted: summary.entries_deleted,
            files_failed: summary.files_failed,
            bytes_copied: summary.bytes_transferred,
            files_resumed: summary.files_resumed,
            in_stream_carrier: Some(summary.in_stream_carrier_used),
            failed_paths: summary.failed_paths.clone(),
            failed_paths_raw: raw_at_their_places(&summary.failed_paths, &summary.failed_paths_raw),
            failed_paths_truncated: summary.failed_paths_truncated,
            failures: crate::transfers::failures::failures_of(summary),
            left_in_place: summary.left_in_place.clone(),
            left_in_place_truncated: summary.left_in_place_truncated,
            left_in_place_raw: summary.left_in_place_raw.clone(),
        }
    }
}

/// One run's log; see the module docs. Cheap to clone.
#[derive(Clone)]
pub struct RunLog {
    inner: Arc<Mutex<State>>,
}

struct State {
    place: Option<LogPlace>,
    /// The ID the log is kept under when no run was given: the daemon's own
    /// job ID, or the CLI's run ID.
    own_id: String,
    /// The run the log is kept under, once started.
    run_id: String,
    active: Option<ActiveLog>,
    /// Diagnostics to record at close.
    notes: Vec<String>,
    totals: Option<RunTotals>,
    /// The run's own failure message, kept over a caller's marker.
    error: Option<String>,
    /// The task draining the audit lane into the log, and the lane's
    /// planned totals.
    relay: Option<tokio::task::JoinHandle<()>>,
    planned: Option<Arc<PlannedTotals>>,
    disposition: Disposition,
    /// The closed log's non-UTF-8 names, kept for the run's record (review
    /// cr-jl3a-3).
    closed_raw_names: HashMap<String, String>,
}

/// What became of what a run moved (review cr-jl2-1): written, or —
/// a dry run, a null-sink run — nothing. A run that wrote nothing names no
/// file copied and counts none; its log says so, and what it would have
/// copied (a dry run) or read and discarded (`--null`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Disposition {
    #[default]
    Written,
    /// `--dry-run`: planned, nothing changed.
    DryRun,
    /// `--null`: read, then discarded.
    Discarded,
}

impl Disposition {
    /// What a log says of a run that wrote nothing; `None` for one that
    /// wrote.
    pub fn nothing_written(self) -> Option<&'static str> {
        match self {
            Disposition::Written => None,
            Disposition::DryRun => Some("dry run: nothing was written"),
            Disposition::Discarded => {
                Some("--null: what was read was discarded; nothing was written")
            }
        }
    }
}

/// Which end of a transfer an audit lane watches. A file finished on the
/// sending end was sent — whether it was written is the receiving end's to
/// say — so its log says `sent`; on the receiving end, `copied`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum End {
    Sending,
    Receiving,
}

struct ActiveLog {
    writer: LogWriter,
    sender: LogSender,
    /// Its place's `finished`, once the place is known.
    finished: Arc<OnceLock<Arc<tokio::sync::Notify>>>,
    role: Role,
    started: Instant,
    deleting: bool,
    /// Failures already named, by text and escaped bytes (review
    /// cr-jl3afix2-1: two entries of one text are two files), so the
    /// summary's list adds only the rest.
    failed: HashSet<(String, Option<String>)>,
    /// The exact bytes, escaped, of each name in this run that is not valid
    /// UTF-8, by its text (review cr-jl1a-1). The first wins, as at both
    /// ends of a transfer.
    raw_names: HashMap<String, String>,
}

impl RunLog {
    /// A log not yet started, to be kept in `place` (nowhere when `None`)
    /// under `own_id` unless a run is given at [`start`](Self::start).
    pub fn new(place: Option<LogPlace>, own_id: &str) -> Self {
        Self {
            inner: Arc::new(Mutex::new(State {
                place,
                own_id: own_id.to_string(),
                run_id: own_id.to_string(),
                active: None,
                notes: Vec::new(),
                totals: None,
                error: None,
                relay: None,
                planned: None,
                disposition: Disposition::Written,
                closed_raw_names: HashMap::new(),
            })),
        }
    }

    fn state(&self) -> std::sync::MutexGuard<'_, State> {
        self.inner
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
    }

    /// Start the log in `role`; once only, and only when it has a place.
    /// Safe from sync code inside a runtime: the only event it records goes
    /// into a fresh, empty queue, so it never waits.
    ///
    /// JOB_LOGS jl-2: with a run `tag`, the log is kept under that run's ID
    /// and session number, so every machine involved logs the run under one
    /// ID; otherwise under the own ID, as session 1.
    pub fn start(&self, role: Role, run: RunInfo, tag: Option<RunTag>) {
        let place = self.state().place.clone();
        self.begin(role, run, tag, place.map(|place| move || Some(place)));
    }

    /// [`start`](Self::start) a log whose place is not known yet (JOB_LOGS
    /// jl-5): `resolve` finds it on the log's own thread — it may read
    /// files — while what the run records waits in the log's queue. A
    /// `None` from it means no log is kept: the log takes in and drops what
    /// it is given, and closes as `None`, as a log with no place does.
    pub fn start_with(
        &self,
        role: Role,
        run: RunInfo,
        tag: Option<RunTag>,
        resolve: impl FnOnce() -> Option<LogPlace> + Send + 'static,
    ) {
        self.begin(role, run, tag, Some(resolve));
    }

    fn begin(
        &self,
        role: Role,
        run: RunInfo,
        tag: Option<RunTag>,
        resolve: Option<impl FnOnce() -> Option<LogPlace> + Send + 'static>,
    ) {
        let mut state = self.state();
        if state.active.is_some() {
            return;
        }
        let (run_id, attempt) = match tag {
            Some(tag) => (tag.run_id, tag.attempt),
            None => (state.own_id.clone(), 1),
        };
        // A record naming the run gets it whether or not a log is kept.
        state.run_id = run_id.clone();
        let Some(resolve) = resolve else {
            return;
        };
        let finished = Arc::new(OnceLock::new());
        let found = Arc::clone(&finished);
        let writer = LogWriter::start_deferred(run, move || {
            let place = resolve()?;
            let key = match LogKey::new(run_id.as_str(), place.participant, role, attempt) {
                Ok(key) => key,
                Err(error) => {
                    log::warn!("run {run_id}: not logged: {error:#}");
                    return None;
                }
            };
            if let Some(notify) = place.finished {
                let _ = found.set(notify);
            }
            Some(LogHome {
                dir: place.dir,
                key,
                keep: place.keep,
            })
        });
        let sender = writer.sender();
        sender.record(EventBody::Phase {
            name: "transfer".into(),
            state: PhaseState::Start,
        });
        state.active = Some(ActiveLog {
            writer,
            sender,
            finished,
            role,
            started: Instant::now(),
            deleting: false,
            failed: HashSet::new(),
            raw_names: HashMap::new(),
        });
    }

    /// The exact bytes, escaped, of the name `path` stands for when it is
    /// not valid UTF-8, as the run's log knows it — after the log closes,
    /// every name its transfer reported (review cr-jl3a-3). `None` for a
    /// UTF-8 name.
    pub fn raw_name(&self, path: &str) -> Option<String> {
        let state = self.state();
        match &state.active {
            Some(active) => active.raw_names.get(path).cloned(),
            None => state.closed_raw_names.get(path).cloned(),
        }
    }

    /// What became of what the run moves: a dry run or a null-sink run
    /// writes nothing, and its log says so (review cr-jl2-1).
    pub fn set_disposition(&self, disposition: Disposition) {
        self.state().disposition = disposition;
    }

    /// The run ID the log is kept under (the own ID until a run is adopted
    /// at start).
    pub fn run_id(&self) -> String {
        self.state().run_id.clone()
    }

    /// The bounded lane the run's transfer reports its log's facts on, and
    /// the task that drains it into the log (review cr-jl1b-2): while the
    /// log falls behind, the transfer waits. `None` when the log has no
    /// place and was not started with one to find. Each call makes a lane
    /// of its own (a command runs one transfer per pass);
    /// [`close`](Self::close) waits for every one to drain, which each does
    /// once its transfer has dropped every sender.
    ///
    /// The lane watches the end the log's role names: a source sends, a
    /// destination receives. An initiator, which is either, says which with
    /// [`audit_lane_at`](Self::audit_lane_at).
    pub fn audit_lane(&self) -> Option<AuditSender> {
        self.lane(None)
    }

    /// [`audit_lane`](Self::audit_lane) for a lane watching `end` of the
    /// transfer, whatever the log's role.
    pub fn audit_lane_at(&self, end: End) -> Option<AuditSender> {
        self.lane(Some(end))
    }

    fn lane(&self, end: Option<End>) -> Option<AuditSender> {
        let mut state = self.state();
        if state.place.is_none() && state.active.is_none() {
            return None;
        }
        let (sender, receiver) = audit_lane(AUDIT_LANE_DEPTH);
        let planned = receiver.planned();
        let log = self.clone();
        let earlier = state.relay.take();
        state.planned = Some(planned);
        state.relay = Some(tokio::spawn(async move {
            // One lane at a time, in order: an earlier pass's events land
            // before this one's.
            if let Some(earlier) = earlier {
                let _ = earlier.await;
            }
            // cr-ecm39-4: the names this lane's pass heard, its own — one
            // entry for each text, as its manifest grants — so a file is
            // named by the bytes of its own pass, never another's.
            let mut names: HashMap<String, String> = HashMap::new();
            while let Some(event) = receiver.recv().await {
                log.observe_from(&event, end, Some(&mut names)).await;
            }
        }));
        Some(sender)
    }

    /// Record what a progress event names: each file copied — or, on the
    /// sending end, sent — failed or deleted, and the phase changes. The
    /// event is from the end the log's role names.
    pub async fn observe(&self, event: &ProgressEvent) {
        self.observe_from(event, None, None).await;
    }

    /// [`observe`](Self::observe) for an event from `end` of the transfer
    /// (`None`: the end the log's role names).
    ///
    /// cr-ecm39-4: `names`, the names heard on the lane `event` came by — one
    /// pass's — name its files; without a lane, the log's own (one
    /// session's). Named by every pass's, a text-named file's failure was
    /// logged as an earlier pass's byte-named sibling of its spelling.
    async fn observe_from(
        &self,
        event: &ProgressEvent,
        end: Option<End>,
        mut names: Option<&mut HashMap<String, String>>,
    ) {
        if let (ProgressEvent::RawName { path, raw }, Some(names)) = (event, names.as_deref_mut()) {
            names
                .entry(path.clone())
                .or_insert_with(|| crate::raw_name::escape_raw(raw));
        }
        let (sender, bodies) = {
            let mut state = self.state();
            let (planned_files, planned_bytes) = state
                .planned
                .as_ref()
                .map_or((0, 0), |planned| planned.get());
            let disposition = state.disposition;
            let Some(active) = state.active.as_mut() else {
                return;
            };
            let raw = |path: &str| match names.as_deref() {
                Some(names) => names.get(path).cloned(),
                None => active.raw_names.get(path).cloned(),
            };
            let bodies = match event {
                // A run that writes nothing copies nothing.
                ProgressEvent::FileComplete { .. } if disposition != Disposition::Written => {
                    vec![]
                }
                ProgressEvent::FileComplete { path } => {
                    vec![match end.unwrap_or(match active.role {
                        Role::Source => End::Sending,
                        _ => End::Receiving,
                    }) {
                        End::Sending => EventBody::FileSent {
                            path: path.clone(),
                            raw: raw(path),
                        },
                        End::Receiving => EventBody::FileCopied {
                            path: path.clone(),
                            bytes: None,
                            raw: raw(path),
                        },
                    }]
                }
                ProgressEvent::FileFailed {
                    path,
                    reason,
                    left_in_place,
                } => {
                    let body = EventBody::FileFailed {
                        path: path.clone(),
                        reason: reason.clone(),
                        raw: raw(path),
                        left_in_place: *left_in_place,
                    };
                    active.failed.insert((path.clone(), raw(path)));
                    vec![body]
                }
                ProgressEvent::Deleted { path, raw } => vec![EventBody::FileDeleted {
                    path: path.clone(),
                    raw: raw.as_deref().map(crate::raw_name::escape_raw),
                }],
                ProgressEvent::RawName { path, raw } => {
                    active
                        .raw_names
                        .entry(path.clone())
                        .or_insert_with(|| crate::raw_name::escape_raw(raw));
                    vec![]
                }
                ProgressEvent::DiffComplete => vec![EventBody::Diagnostic {
                    message: format!(
                        "compared the whole source list: {planned_files} file(s), {} to send",
                        format_bytes(planned_bytes)
                    ),
                }],
                ProgressEvent::DeleteBegin => {
                    active.deleting = true;
                    vec![
                        EventBody::Phase {
                            name: "transfer".into(),
                            state: PhaseState::End,
                        },
                        EventBody::Phase {
                            name: "delete".into(),
                            state: PhaseState::Start,
                        },
                    ]
                }
                _ => vec![],
            };
            (active.sender.clone(), bodies)
        };
        for body in bodies {
            sender.record_async(body).await;
        }
    }

    /// Record `body` now, in order with what the transfer reports (the
    /// CLI's own notes: a retry pass starting, a move's source removal).
    pub async fn record(&self, body: EventBody) {
        let sender = {
            let state = self.state();
            let Some(active) = state.active.as_ref() else {
                return;
            };
            active.sender.clone()
        };
        sender.record_async(body).await;
    }

    /// A diagnostic line for the log, recorded at close.
    pub fn note(&self, line: String) {
        self.state().notes.push(line);
    }

    /// The run's summary, when the part that has it is not the closer.
    pub fn note_totals(&self, totals: RunTotals) {
        self.state().totals = Some(totals);
    }

    /// How many files failed on their own, by the noted summary.
    pub fn noted_files_failed(&self) -> u64 {
        self.state()
            .totals
            .as_ref()
            .map_or(0, |totals| totals.files_failed)
    }

    /// The run's own failure message, kept over the closer's.
    pub fn note_error(&self, message: String) {
        self.state().error = Some(message);
    }

    /// Close the log (see the module docs):
    /// [`prepare_close`](Self::prepare_close), then [`ClosingLog::finish`].
    /// `None` when it was never started, or had no place.
    pub async fn close(
        &self,
        outcome: Outcome,
        detail: Option<String>,
        totals: Option<RunTotals>,
    ) -> Option<LogReport> {
        self.prepare_close(outcome, detail, totals)
            .await?
            .finish()
            .await
    }

    /// The first half of [`close`](Self::close): wait for the audit lanes
    /// to drain; record the end of the open phase, the failures only the
    /// summary names, the diagnostics and the summary; keep the log's
    /// non-UTF-8 names for [`raw_name`](Self::raw_name); and hand back the
    /// log, to be finished with how the run ended. `None` when it was never
    /// started.
    pub async fn prepare_close(
        &self,
        outcome: Outcome,
        detail: Option<String>,
        totals: Option<RunTotals>,
    ) -> Option<ClosingLog> {
        // Every fact the transfer reported reaches the log before it closes:
        // the relay ends once the transfer has dropped its senders.
        let relay = self.state().relay.take();
        let mut relay_cut = false;
        if let Some(mut relay) = relay {
            if tokio::time::timeout(RELAY_DRAIN_LIMIT, &mut relay)
                .await
                .is_err()
            {
                relay.abort();
                relay_cut = true;
            }
        }
        let (mut active, notes, totals, error, run_id, planned, disposition) = {
            let mut state = self.state();
            let active = state.active.take()?;
            state.closed_raw_names = active.raw_names.clone();
            (
                active,
                std::mem::take(&mut state.notes),
                totals.or_else(|| state.totals.take()),
                state.error.take(),
                state.run_id.clone(),
                state
                    .planned
                    .as_ref()
                    .map_or((0, 0), |planned| planned.get()),
                state.disposition,
            )
        };
        // The open phase ends first: what the summary adds below was learned
        // at the end, while every failure seen live sits inside the phase.
        let mut bodies = vec![EventBody::Phase {
            name: if active.deleting {
                "delete"
            } else {
                "transfer"
            }
            .into(),
            state: PhaseState::End,
        }];
        if let Some(totals) = &totals {
            // Reviews cr-jl3afix1-1, cr-jl3afix2-1: each failure named for a
            // text, by its own bytes when it has them — two entries whose
            // names collapse to one text are two files — and each named
            // once: one already named live, with those bytes, is not again.
            let mut failed = std::mem::take(&mut active.failed);
            // cr-ecm35-3: each failed file once, by its identity, from the
            // one join the saved record uses too. log-named-text: by the
            // identity the account gives it, and no other — a failure the
            // account names by its text alone IS a text name (peers of one
            // contract carry the bytes of every name that is not text), and
            // the bytes heard live for its spelling are another file's.
            for file in totals.failed_files() {
                let raw = file.raw.as_deref().map(crate::raw_name::escape_raw);
                if failed.insert((file.path.clone(), raw.clone())) {
                    let left_in_place = file.left_in_place;
                    bodies.push(EventBody::FileFailed {
                        reason: file
                            .reason
                            .unwrap_or_else(|| NO_REASON_REPORTED.to_string()),
                        path: file.path,
                        raw,
                        left_in_place,
                    });
                }
            }
            if totals.failed_paths_truncated {
                bodies.push(EventBody::Diagnostic {
                    message: format!(
                        "the list of failed files was cut short for size; {} failed in all",
                        totals.files_failed
                    ),
                });
            }
        }
        // log-relay-cut: marked as `log-incomplete`, the fact every reader
        // of a log acts on — events were lost. Said only in a diagnostic's
        // words, the log was read as whole, and a retry completed a record
        // from it.
        if relay_cut {
            bodies.push(EventBody::LogIncomplete {
                reason: "the transfer still held its log lane open when the run ended; \
                         events it had not handed over are missing"
                    .into(),
            });
        }
        let elapsed = active.started.elapsed();
        let (planned_files, planned_bytes) = planned;
        if planned_files > 0 {
            bodies.push(EventBody::Diagnostic {
                message: format!(
                    "planned: {planned_files} file(s), {}",
                    format_bytes(planned_bytes)
                ),
            });
        }
        for message in notes {
            bodies.push(EventBody::Diagnostic { message });
        }
        if let Some(totals) = &totals {
            if let Some(in_stream) = totals.in_stream_carrier {
                bodies.push(EventBody::Diagnostic {
                    message: if in_stream {
                        "payload carrier: in-stream (gRPC)".into()
                    } else {
                        "payload carrier: TCP data plane".into()
                    },
                });
            }
            if totals.files_resumed > 0 {
                bodies.push(EventBody::Diagnostic {
                    message: format!("resumed: {} file(s)", totals.files_resumed),
                });
            }
            match disposition {
                Disposition::Written => {}
                // Review cr-jl2fix1-1: what it would have copied is what it
                // planned; a dry run's written totals are nothing.
                Disposition::DryRun => bodies.push(EventBody::Diagnostic {
                    message: format!(
                        "dry run: nothing was written; it would have copied {planned_files} \
                         file(s), {}, and deleted {}",
                        format_bytes(planned_bytes),
                        totals.files_deleted
                    ),
                }),
                Disposition::Discarded => bodies.push(EventBody::Diagnostic {
                    message: format!(
                        "--null: nothing was written; {} file(s), {} read and discarded",
                        totals.files_copied,
                        format_bytes(totals.bytes_copied)
                    ),
                }),
            }
            let seconds = elapsed.as_secs_f64();
            if seconds > 0.0 && totals.bytes_copied > 0 && disposition != Disposition::DryRun {
                bodies.push(EventBody::Diagnostic {
                    message: format!(
                        "average: {}/s over the whole run",
                        format_bytes((totals.bytes_copied as f64 / seconds) as u64)
                    ),
                });
            }
            let written = disposition == Disposition::Written;
            bodies.push(EventBody::Summary(Summary {
                files_copied: if written { totals.files_copied } else { 0 },
                files_deleted: if written { totals.files_deleted } else { 0 },
                files_failed: totals.files_failed,
                bytes_copied: if written { totals.bytes_copied } else { 0 },
                elapsed_ms: u64::try_from(elapsed.as_millis()).unwrap_or(u64::MAX),
            }));
        }
        for body in bodies {
            active.sender.record_async(body).await;
        }
        let detail = error
            .or(detail)
            .or_else(|| disposition.nothing_written().map(str::to_string));
        Some(ClosingLog {
            writer: active.writer,
            finished: active.finished,
            outcome,
            detail,
            run_id,
        })
    }
}

/// A run's log with every event recorded, to be finished
/// ([`RunLog::prepare_close`]).
pub struct ClosingLog {
    writer: LogWriter,
    finished: Arc<OnceLock<Arc<tokio::sync::Notify>>>,
    outcome: Outcome,
    detail: Option<String>,
    run_id: String,
}

impl ClosingLog {
    /// Record `run-end`, compress the log into place and prune its folder.
    /// `None` when the log had no place to be kept.
    pub async fn finish(self) -> Option<LogReport> {
        let Self {
            writer,
            finished,
            outcome,
            detail,
            run_id,
        } = self;
        let report = match tokio::task::spawn_blocking(move || writer.finish(outcome, detail)).await
        {
            Ok(report) if report.no_place => return None,
            Ok(report) => {
                if !report.complete {
                    log::warn!(
                        "run {run_id}: its log is incomplete: {}",
                        report.problem.as_deref().unwrap_or_default()
                    );
                }
                report
            }
            Err(error) => {
                log::warn!("run {run_id}: closing its log failed: {error}");
                LogReport {
                    problem: Some(format!("closing the log failed: {error}")),
                    ..LogReport::default()
                }
            }
        };
        if let Some(finished) = finished.get() {
            finished.notify_waiters();
        }
        Some(report)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::job_log::{RunInfo, DEFAULT_KEEP};

    /// JOB_LOGS jl-5: a log started before its place is known hands out its
    /// lane at once and keeps what the run reports through it; one whose
    /// place cannot be found closes as a log with no place does — `None`,
    /// nothing kept, nothing said.
    #[tokio::test]
    async fn a_log_started_with_a_resolver_keeps_the_run_or_closes_as_unplaced() {
        use crate::job_log::{logs_in, open_log, LogLine};
        let dir = tempfile::tempdir().unwrap();
        for found in [false, true] {
            let log = RunLog::new(None, "0123456789abcdef0123456789abcdef");
            let place = LogPlace {
                dir: dir.path().to_path_buf(),
                participant: "m1".into(),
                keep: DEFAULT_KEEP,
                finished: None,
            };
            log.start_with(Role::Initiator, RunInfo::default(), None, move || {
                found.then_some(place)
            });
            let progress = crate::remote::transfer::RemoteTransferProgress::audit_only(
                log.audit_lane_at(End::Receiving)
                    .expect("a lane before the place is known"),
            );
            progress.report_file_complete("a.txt".into()).await;
            drop(progress);
            let report = log.close(Outcome::Ok, None, None).await;
            if !found {
                assert_eq!(report, None, "a log with no place closes as none");
                assert!(logs_in(dir.path()).unwrap().is_empty());
                continue;
            }
            let report = report.expect("a report");
            assert!(report.complete, "{report:?}");
            let copied: Vec<String> = open_log(&report.path.expect("a log"))
                .unwrap()
                .filter_map(|line| match line.unwrap() {
                    LogLine::Event(event) => match event.body {
                        EventBody::FileCopied { path, .. } => Some(path),
                        _ => None,
                    },
                    _ => None,
                })
                .collect();
            assert_eq!(copied, ["a.txt"]);
        }
    }

    /// Review cr-jl3a-3: a run's record can name a failed file exactly —
    /// the log hands out a non-UTF-8 name's bytes, after it closes too.
    #[tokio::test]
    async fn a_closed_log_still_names_a_raw_name_exactly() {
        let dir = tempfile::tempdir().unwrap();
        let log = RunLog::new(
            Some(LogPlace {
                dir: dir.path().to_path_buf(),
                participant: "m1".into(),
                keep: DEFAULT_KEEP,
                finished: None,
            }),
            "0123456789abcdef0123456789abcdef",
        );
        log.start(
            Role::Initiator,
            RunInfo {
                verb: "copy".into(),
                ..RunInfo::default()
            },
            None,
        );
        let lane = log.audit_lane_at(End::Receiving).expect("a lane");
        let progress = crate::remote::transfer::RemoteTransferProgress::audit_only(lane);
        progress
            .report_raw_name("bad\u{FFFD}.txt".into(), b"bad\xff.txt".to_vec())
            .await;
        drop(progress);
        log.close(Outcome::Ok, None, None).await.expect("a report");
        assert_eq!(
            log.raw_name("bad\u{FFFD}.txt"),
            Some(crate::raw_name::escape_raw(b"bad\xff.txt"))
        );
        assert_eq!(log.raw_name("plain.txt"), None);
    }
    /// Review cr-jl3afix1-1: two entries whose names collapse to one text
    /// are two files — the log names each failure by its own bytes, not
    /// by the first entry's that its text maps to.
    #[tokio::test]
    async fn each_failure_is_logged_by_its_own_bytes() {
        use crate::job_log::{open_log, LogLine};
        let dir = tempfile::tempdir().unwrap();
        let log = RunLog::new(
            Some(LogPlace {
                dir: dir.path().to_path_buf(),
                participant: "m1".into(),
                keep: DEFAULT_KEEP,
                finished: None,
            }),
            "0123456789abcdef0123456789abcdef",
        );
        log.start(Role::Initiator, RunInfo::default(), None);
        let progress = crate::remote::transfer::RemoteTransferProgress::audit_only(
            log.audit_lane_at(End::Receiving).expect("a lane"),
        );
        // The first entry of the text, as the manifest reported it.
        progress
            .report_raw_name("dup\u{FFFD}.bin".into(), b"dup\xfe.bin".to_vec())
            .await;
        drop(progress);
        let totals = RunTotals {
            files_failed: 1,
            failed_paths: vec!["dup\u{FFFD}.bin".into()],
            failures: vec![FileFailure {
                relative_path: "dup\u{FFFD}.bin".into(),
                reason: "duplicate".into(),
                raw_relative_path: Some(b"dup\xff.bin".to_vec()),
                left_in_place: false,
                path_shortened: false,
                left_unknown: false,
                identity_known: false,
            }],
            ..RunTotals::default()
        };
        let report = log
            .close(Outcome::Failed, None, Some(totals))
            .await
            .expect("a report");
        let raws: Vec<Option<String>> = open_log(&report.path.expect("a log"))
            .unwrap()
            .filter_map(|line| match line.unwrap() {
                LogLine::Event(event) => match event.body {
                    EventBody::FileFailed { raw, .. } => Some(raw),
                    _ => None,
                },
                _ => None,
            })
            .collect();
        assert_eq!(raws, [Some(crate::raw_name::escape_raw(b"dup\xff.bin"))]);
    }

    /// cr-ecm34-5: a run whose log heard no name live — its account only
    /// the summary — names a failure past the reason cap by the bytes the
    /// summary carries for its path, and says it left its copy by them.
    #[tokio::test]
    async fn a_summary_only_failure_is_named_by_its_own_bytes() {
        use crate::job_log::{open_log, LogLine};
        let dir = tempfile::tempdir().unwrap();
        let log = RunLog::new(
            Some(LogPlace {
                dir: dir.path().to_path_buf(),
                participant: "m1".into(),
                keep: DEFAULT_KEEP,
                finished: None,
            }),
            "0123456789abcdef0123456789abcdef",
        );
        log.start(Role::Initiator, RunInfo::default(), None);
        let summary = TransferSummary {
            files_failed: 2,
            failed_paths: vec!["plain.bin".into(), "dup\u{FFFD}.bin".into()],
            failed_paths_raw: vec![Vec::new(), b"dup\xfe.bin".to_vec()],
            left_in_place_raw: vec![b"dup\xfe.bin".to_vec()],
            ..TransferSummary::default()
        };
        let report = log
            .close(Outcome::Failed, None, Some(RunTotals::from(&summary)))
            .await
            .expect("a report");
        let failed: Vec<(String, Option<String>, Option<bool>)> =
            open_log(&report.path.expect("a log"))
                .unwrap()
                .filter_map(|line| match line.unwrap() {
                    LogLine::Event(event) => match event.body {
                        EventBody::FileFailed {
                            path,
                            raw,
                            left_in_place,
                            ..
                        } => Some((path, raw, left_in_place)),
                        _ => None,
                    },
                    _ => None,
                })
                .collect();
        assert_eq!(
            failed,
            [
                ("plain.bin".to_string(), None, Some(false)),
                (
                    "dup\u{FFFD}.bin".to_string(),
                    Some(crate::raw_name::escape_raw(b"dup\xfe.bin")),
                    Some(true)
                ),
            ]
        );
        // Merged with a part that carried no bytes, each path keeps its own.
        let mut merged = RunTotals {
            failed_paths: vec!["a".into()],
            ..RunTotals::default()
        };
        merged.merge(RunTotals::from(&summary));
        assert_eq!(merged.failed_paths_raw.len(), merged.failed_paths.len());
        assert_eq!(merged.failed_paths_raw[2], b"dup\xfe.bin".to_vec());
    }

    /// cr-ecm35-5: a failed file past the reason cap is told left or not by
    /// the lists alone, and missing from a list cut short it is not known —
    /// logged so, never as known nothing. Logged as nothing left, a recovered
    /// `--ignore-existing` retry could keep its own partial as a file.
    #[test]
    fn a_failure_missing_from_a_cut_list_is_not_known() {
        let totals = RunTotals {
            files_failed: 2,
            failed_paths: vec!["in.bin".into(), "out.bin".into()],
            left_in_place: vec!["in.bin".into()],
            left_in_place_truncated: true,
            ..RunTotals::default()
        };
        let said: Vec<(String, Option<bool>)> = totals
            .failed_files()
            .into_iter()
            .map(|file| (file.path, file.left_in_place))
            .collect();
        assert_eq!(
            said,
            [
                ("in.bin".to_string(), Some(true)),
                ("out.bin".to_string(), None)
            ]
        );
        let whole = RunTotals {
            left_in_place_truncated: false,
            ..totals
        };
        assert_eq!(whole.failed_files()[1].left_in_place, Some(false));
    }

    /// cr-ecm37-5: a failure whose path the wire shortened keeps its reason
    /// in the account, joined to its exact name. Its shortened path no name,
    /// the record lost the reason.
    #[test]
    fn a_shortened_failure_keeps_its_reason() {
        let long = format!("{}/name.bin", "d".repeat(4600));
        let mut outcome = crate::remote::transfer::sink::SinkOutcome::default();
        outcome.record_failure(long.clone(), "source: missing");
        let (paths, raw, truncated) = outcome.wire_failed_identities();
        let summary = crate::generated::TransferSummary {
            files_failed: 1,
            failures: outcome.wire_failures_linked(&paths, &raw),
            failed_paths: paths,
            failed_paths_raw: raw,
            failed_paths_truncated: truncated,
            ..Default::default()
        };
        let files = RunTotals::from(&summary).failed_files();
        assert_eq!(files.len(), 1, "{files:?}");
        assert_eq!(files[0].path, long);
        assert_eq!(files[0].reason.as_deref(), Some("source: missing"));
    }

    /// cr-ecm37-3: a failure the report kept as not known to have left its
    /// copy is not known in the account either — never "nothing left".
    #[test]
    fn a_kept_unknown_is_not_known() {
        let totals = RunTotals {
            files_failed: 1,
            failed_paths: vec!["u.bin".into()],
            failures: vec![FileFailure {
                relative_path: "u.bin".into(),
                reason: "unseen".into(),
                left_unknown: true,
                ..FileFailure::default()
            }],
            ..RunTotals::default()
        };
        assert_eq!(totals.failed_files()[0].left_in_place, None);
    }

    /// cr-ecm37-7: the join is linear in the leftover lists as well as in the
    /// failures. A hundred thousand failed files that all left a copy, half
    /// named by text and half by bytes, are joined well inside the bound:
    /// searching either list for each file, half of them took several
    /// seconds in a debug build. (The bound is far from both: the join by
    /// hash takes a small fraction of it.)
    #[test]
    fn the_join_is_linear_in_the_leftovers() {
        const EACH: usize = 50_000;
        let text: Vec<String> = (0..EACH).map(|n| format!("t/{n:06}")).collect();
        let raw: Vec<Vec<u8>> = (0..EACH)
            .map(|n| {
                let mut bytes = format!("r/{n:06}").into_bytes();
                bytes.push(0xFF);
                bytes
            })
            .collect();
        let mut failed_paths = text.clone();
        failed_paths.extend(
            raw.iter()
                .map(|bytes| String::from_utf8_lossy(bytes).into_owned()),
        );
        let mut failed_paths_raw = vec![Vec::new(); EACH];
        failed_paths_raw.extend(raw.iter().cloned());
        let totals = RunTotals {
            files_failed: 2 * EACH as u64,
            failed_paths,
            failed_paths_raw,
            left_in_place: text,
            left_in_place_raw: raw,
            ..RunTotals::default()
        };
        let started = std::time::Instant::now();
        let files = totals.failed_files();
        let took = started.elapsed();
        assert_eq!(files.len(), 2 * EACH);
        assert!(files.iter().all(|file| file.left_in_place == Some(true)));
        assert!(
            took < std::time::Duration::from_secs(3),
            "the join took {took:?}"
        );
    }

    /// cr-ecm36-3: a failure whose path the report shortened is no file: the
    /// long name, from the failed-path set, is the one file failed. Taken
    /// for a name, the shortened rendering was a second file, and the list
    /// was called whole.
    #[test]
    fn a_shortened_path_is_no_file() {
        let long = format!("{}/name.bin", "d".repeat(4600));
        let totals = RunTotals {
            files_failed: 1,
            failed_paths: vec![long.clone()],
            failures: vec![FileFailure {
                relative_path: "[truncated]…name.bin".into(),
                reason: "source: missing".into(),
                path_shortened: true,
                ..FileFailure::default()
            }],
            ..RunTotals::default()
        };
        let files = totals.failed_files();
        assert_eq!(files.len(), 1, "{files:?}");
        assert_eq!(files[0].path, long);
    }

    /// cr-ecm35-3: two failed files of one text, the reason cap falling
    /// between them, are two files — each logged by its own bytes, the one
    /// past the cap with no reason. Joined by the text, the second was not
    /// logged at all.
    #[tokio::test]
    async fn two_failures_of_one_text_past_the_cap_are_two_files() {
        use crate::job_log::{open_log, LogLine};
        let dir = tempfile::tempdir().unwrap();
        let log = RunLog::new(
            Some(LogPlace {
                dir: dir.path().to_path_buf(),
                participant: "m1".into(),
                keep: DEFAULT_KEEP,
                finished: None,
            }),
            "0123456789abcdef0123456789abcdef",
        );
        log.start(Role::Initiator, RunInfo::default(), None);
        let text = "dup\u{FFFD}".to_string();
        let totals = RunTotals {
            files_failed: 2,
            failed_paths: vec![text.clone(), text.clone()],
            failed_paths_raw: vec![b"dup\xff".to_vec(), b"dup\xfe".to_vec()],
            failures: vec![FileFailure {
                relative_path: text.clone(),
                reason: "denied".into(),
                raw_relative_path: Some(b"dup\xff".to_vec()),
                left_in_place: false,
                path_shortened: false,
                left_unknown: false,
                identity_known: false,
            }],
            ..RunTotals::default()
        };
        assert_eq!(totals.failed_files().len(), 2);
        let report = log
            .close(Outcome::Failed, None, Some(totals))
            .await
            .expect("a report");
        let failed: Vec<(String, Option<String>)> = open_log(&report.path.expect("a log"))
            .unwrap()
            .filter_map(|line| match line.unwrap() {
                LogLine::Event(event) => match event.body {
                    EventBody::FileFailed { reason, raw, .. } => Some((reason, raw)),
                    _ => None,
                },
                _ => None,
            })
            .collect();
        assert_eq!(
            failed,
            [
                (
                    "denied".to_string(),
                    Some(crate::raw_name::escape_raw(b"dup\xff"))
                ),
                (
                    NO_REASON_REPORTED.to_string(),
                    Some(crate::raw_name::escape_raw(b"dup\xfe"))
                ),
            ]
        );
    }

    /// D-2026-11-24-1 (cr-ecm33-5): a failure the summary names without a
    /// reason is logged left in place by the entry's own bytes when it has
    /// them — the raw set, never the text set, which may be another entry's.
    #[tokio::test]
    async fn an_unnamed_failure_is_left_in_place_by_its_own_bytes() {
        use crate::job_log::{open_log, LogLine};
        for (raw_left, text_left, logged_left) in [(true, false, true), (false, true, false)] {
            let dir = tempfile::tempdir().unwrap();
            let log = RunLog::new(
                Some(LogPlace {
                    dir: dir.path().to_path_buf(),
                    participant: "m1".into(),
                    keep: DEFAULT_KEEP,
                    finished: None,
                }),
                "0123456789abcdef0123456789abcdef",
            );
            log.start(Role::Initiator, RunInfo::default(), None);
            let progress = crate::remote::transfer::RemoteTransferProgress::audit_only(
                log.audit_lane_at(End::Receiving).expect("a lane"),
            );
            let text = "dup\u{FFFD}.bin".to_string();
            progress
                .report_raw_name(text.clone(), b"dup\xfe.bin".to_vec())
                .await;
            drop(progress);
            let totals = RunTotals {
                files_failed: 1,
                failed_paths: vec![text.clone()],
                // log-named-text: by its own bytes, as a peer of one
                // contract carries them.
                failed_paths_raw: vec![b"dup\xfe.bin".to_vec()],
                left_in_place: if text_left {
                    vec![text.clone()]
                } else {
                    Vec::new()
                },
                left_in_place_raw: if raw_left {
                    vec![b"dup\xfe.bin".to_vec()]
                } else {
                    Vec::new()
                },
                ..RunTotals::default()
            };
            let report = log
                .close(Outcome::Failed, None, Some(totals))
                .await
                .expect("a report");
            let left: Vec<bool> = open_log(&report.path.expect("a log"))
                .unwrap()
                .filter_map(|line| match line.unwrap() {
                    LogLine::Event(event) => match event.body {
                        EventBody::FileFailed { left_in_place, .. } => left_in_place,
                        _ => None,
                    },
                    _ => None,
                })
                .collect();
            assert_eq!(left, [logged_left], "raw {raw_left}, text {text_left}");
        }
    }

    /// cr-ecm39-4: each pass's live events are named by the bytes heard on
    /// that pass's own lane. A byte-named file copied by one pass, and its
    /// text-named sibling of the same spelling failing on the next, are
    /// logged each as itself. Named by every pass's, the sibling's failure
    /// was logged as the byte-named file.
    #[tokio::test]
    async fn each_pass_names_its_own_files() {
        use crate::job_log::{open_log, LogLine};
        let dir = tempfile::tempdir().unwrap();
        let log = RunLog::new(
            Some(LogPlace {
                dir: dir.path().to_path_buf(),
                participant: "m1".into(),
                keep: DEFAULT_KEEP,
                finished: None,
            }),
            "0123456789abcdef0123456789abcdef",
        );
        log.start(Role::Initiator, RunInfo::default(), None);
        let text = "dup\u{FFFD}".to_string();
        let first = crate::remote::transfer::RemoteTransferProgress::audit_only(
            log.audit_lane_at(End::Receiving).expect("a lane"),
        );
        first
            .report_raw_name(text.clone(), b"dup\xff".to_vec())
            .await;
        first.report_file_complete(text.clone()).await;
        drop(first);
        let second = crate::remote::transfer::RemoteTransferProgress::audit_only(
            log.audit_lane_at(End::Receiving).expect("a lane"),
        );
        second
            .report_file_failed(text.clone(), "denied".into())
            .await;
        drop(second);
        let report = log
            .close(Outcome::Failed, None, None)
            .await
            .expect("a report");
        let named: Vec<(&'static str, Option<String>)> = open_log(&report.path.expect("a log"))
            .unwrap()
            .filter_map(|line| match line.unwrap() {
                LogLine::Event(event) => match event.body {
                    EventBody::FileCopied { raw, .. } => Some(("copied", raw)),
                    EventBody::FileFailed { raw, .. } => Some(("failed", raw)),
                    _ => None,
                },
                _ => None,
            })
            .collect();
        assert_eq!(
            named,
            [
                ("copied", Some(crate::raw_name::escape_raw(b"dup\xff"))),
                ("failed", None),
            ]
        );
    }

    /// log-relay-cut: a lane the transfer still holds when the run closes is
    /// cut, and the log says it lost events as `log-incomplete`, which a
    /// retry reads — not only in a diagnostic's words.
    #[tokio::test(start_paused = true)]
    async fn a_lane_held_open_at_the_close_leaves_the_log_incomplete() {
        use crate::job_log::{open_log, LogLine};
        let dir = tempfile::tempdir().unwrap();
        let log = RunLog::new(
            Some(LogPlace {
                dir: dir.path().to_path_buf(),
                participant: "m1".into(),
                keep: DEFAULT_KEEP,
                finished: None,
            }),
            "0123456789abcdef0123456789abcdef",
        );
        log.start(Role::Initiator, RunInfo::default(), None);
        let held = crate::remote::transfer::RemoteTransferProgress::audit_only(
            log.audit_lane_at(End::Receiving).expect("a lane"),
        );
        let report = log
            .close(Outcome::Failed, None, None)
            .await
            .expect("a report");
        drop(held);
        let incomplete = open_log(&report.path.expect("a log")).unwrap().any(|line| {
            matches!(line, Ok(LogLine::Event(event))
                    if matches!(event.body, EventBody::LogIncomplete { .. }))
        });
        assert!(incomplete, "the cut marked");
    }

    /// log-named-text: a failure the account names by its text alone is a
    /// text name, logged as one — not by the bytes heard live for its
    /// spelling, which are another file's (a sibling the run sent). Named by
    /// them, a vanished `dup` + U+FFFD was logged as `dup` + byte `FF`, and a
    /// retry from the log asked for the file that had landed.
    #[tokio::test]
    async fn a_text_failure_is_not_named_by_another_files_bytes() {
        use crate::job_log::{open_log, LogLine};
        let dir = tempfile::tempdir().unwrap();
        let log = RunLog::new(
            Some(LogPlace {
                dir: dir.path().to_path_buf(),
                participant: "m1".into(),
                keep: DEFAULT_KEEP,
                finished: None,
            }),
            "0123456789abcdef0123456789abcdef",
        );
        log.start(Role::Initiator, RunInfo::default(), None);
        let progress = crate::remote::transfer::RemoteTransferProgress::audit_only(
            log.audit_lane_at(End::Receiving).expect("a lane"),
        );
        let text = "dup\u{FFFD}".to_string();
        progress
            .report_raw_name(text.clone(), b"dup\xff".to_vec())
            .await;
        drop(progress);
        let totals = RunTotals {
            files_failed: 1,
            failed_paths: vec![text.clone()],
            failures: vec![FileFailure {
                relative_path: text.clone(),
                reason: "source: missing at retry".into(),
                ..FileFailure::default()
            }],
            ..RunTotals::default()
        };
        let report = log
            .close(Outcome::Failed, None, Some(totals))
            .await
            .expect("a report");
        let failed: Vec<(String, Option<String>)> = open_log(&report.path.expect("a log"))
            .unwrap()
            .filter_map(|line| match line.unwrap() {
                LogLine::Event(event) => match event.body {
                    EventBody::FileFailed { path, raw, .. } => Some((path, raw)),
                    _ => None,
                },
                _ => None,
            })
            .collect();
        assert_eq!(failed, [(text, None)]);
    }

    /// Review cr-jl3afix2-1: a failure named live hides no other failure
    /// of its text — the second entry the summary names, with its own
    /// bytes, is logged too — and is not logged twice.
    #[tokio::test]
    async fn a_live_failure_hides_no_other_failure_of_its_text() {
        use crate::job_log::{open_log, LogLine};
        let dir = tempfile::tempdir().unwrap();
        let log = RunLog::new(
            Some(LogPlace {
                dir: dir.path().to_path_buf(),
                participant: "m1".into(),
                keep: DEFAULT_KEEP,
                finished: None,
            }),
            "0123456789abcdef0123456789abcdef",
        );
        log.start(Role::Initiator, RunInfo::default(), None);
        let progress = crate::remote::transfer::RemoteTransferProgress::audit_only(
            log.audit_lane_at(End::Receiving).expect("a lane"),
        );
        let text = "dup\u{FFFD}.bin".to_string();
        progress
            .report_raw_name(text.clone(), b"dup\xfe.bin".to_vec())
            .await;
        // The first entry fails during the transfer.
        progress
            .report_file_failed(text.clone(), "denied".into())
            .await;
        drop(progress);
        let totals = RunTotals {
            files_failed: 2,
            failed_paths: vec![text.clone()],
            failures: vec![
                // log-named-text: each by its own bytes, as a peer of one
                // contract carries them.
                FileFailure {
                    relative_path: text.clone(),
                    reason: "denied".into(),
                    raw_relative_path: Some(b"dup\xfe.bin".to_vec()),
                    left_in_place: false,
                    path_shortened: false,
                    left_unknown: false,
                    identity_known: false,
                },
                FileFailure {
                    relative_path: text.clone(),
                    reason: "duplicate".into(),
                    raw_relative_path: Some(b"dup\xff.bin".to_vec()),
                    left_in_place: false,
                    path_shortened: false,
                    left_unknown: false,
                    identity_known: false,
                },
            ],
            ..RunTotals::default()
        };
        let report = log
            .close(Outcome::Failed, None, Some(totals))
            .await
            .expect("a report");
        let failed: Vec<(String, Option<String>)> = open_log(&report.path.expect("a log"))
            .unwrap()
            .filter_map(|line| match line.unwrap() {
                LogLine::Event(event) => match event.body {
                    EventBody::FileFailed { reason, raw, .. } => Some((reason, raw)),
                    _ => None,
                },
                _ => None,
            })
            .collect();
        assert_eq!(
            failed,
            [
                (
                    "denied".to_string(),
                    Some(crate::raw_name::escape_raw(b"dup\xfe.bin"))
                ),
                (
                    "duplicate".to_string(),
                    Some(crate::raw_name::escape_raw(b"dup\xff.bin"))
                ),
            ]
        );
    }
}
