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

use crate::display::format_bytes;
use crate::generated::TransferSummary;
use crate::job_log::{
    EventBody, LogKey, LogReport, LogSender, LogWriter, Outcome, PhaseState, Role, RunInfo, RunTag,
    Summary,
};
use crate::remote::transfer::progress::{audit_lane, AuditSender, PlannedTotals, AUDIT_LANE_DEPTH};
use crate::remote::transfer::ProgressEvent;
use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
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
    pub failed_paths_truncated: bool,
    /// `(path, reason)` for as many failed files as the report kept.
    pub failures: Vec<(String, String)>,
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
            failed_paths_truncated: summary.failed_paths_truncated,
            failures: summary
                .failures
                .iter()
                .map(|failure| (failure.relative_path.clone(), failure.reason.clone()))
                .collect(),
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
    role: Role,
    started: Instant,
    deleting: bool,
    /// Failed files already named, so the summary's list adds only the rest.
    failed: HashSet<String>,
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
        let Some(writer) = state
            .place
            .as_ref()
            .and_then(|place| place.start(&run_id, attempt, role, run))
        else {
            return;
        };
        let sender = writer.sender();
        sender.record(EventBody::Phase {
            name: "transfer".into(),
            state: PhaseState::Start,
        });
        state.active = Some(ActiveLog {
            writer,
            sender,
            role,
            started: Instant::now(),
            deleting: false,
            failed: HashSet::new(),
            raw_names: HashMap::new(),
        });
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
    /// place. Each call makes a lane of its own (a command runs one transfer
    /// per pass); [`close`](Self::close) waits for every one to drain, which
    /// each does once its transfer has dropped every sender.
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
        state.place.as_ref()?;
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
            while let Some(event) = receiver.recv().await {
                log.observe_from(&event, end).await;
            }
        }));
        Some(sender)
    }

    /// Record what a progress event names: each file copied — or, on the
    /// sending end, sent — failed or deleted, and the phase changes. The
    /// event is from the end the log's role names.
    pub async fn observe(&self, event: &ProgressEvent) {
        self.observe_from(event, None).await;
    }

    /// [`observe`](Self::observe) for an event from `end` of the transfer
    /// (`None`: the end the log's role names).
    async fn observe_from(&self, event: &ProgressEvent, end: Option<End>) {
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
            let raw = |path: &str| active.raw_names.get(path).cloned();
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
                ProgressEvent::FileFailed { path, reason } => {
                    let body = EventBody::FileFailed {
                        path: path.clone(),
                        reason: reason.clone(),
                        raw: raw(path),
                    };
                    active.failed.insert(path.clone());
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

    /// Close the log (see the module docs). `None` when it was never
    /// started.
    pub async fn close(
        &self,
        outcome: Outcome,
        detail: Option<String>,
        totals: Option<RunTotals>,
    ) -> Option<LogReport> {
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
        let (active, notes, totals, error, run_id, planned, finished, disposition) = {
            let mut state = self.state();
            let active = state.active.take()?;
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
                state
                    .place
                    .as_ref()
                    .and_then(|place| place.finished.clone()),
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
            for path in &totals.failed_paths {
                if active.failed.contains(path) {
                    continue;
                }
                let reason = totals
                    .failures
                    .iter()
                    .find(|(failed, _)| failed == path)
                    .map_or(NO_REASON_REPORTED, |(_, reason)| reason.as_str());
                bodies.push(EventBody::FileFailed {
                    path: path.clone(),
                    reason: reason.to_string(),
                    raw: active.raw_names.get(path).cloned(),
                });
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
        if relay_cut {
            bodies.push(EventBody::Diagnostic {
                message: "the transfer still held its log lane open when the run ended; \
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
        let writer = active.writer;
        let report = match tokio::task::spawn_blocking(move || writer.finish(outcome, detail)).await
        {
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
                    path: None,
                    complete: false,
                    problem: Some(format!("closing the log failed: {error}")),
                }
            }
        };
        if let Some(finished) = finished {
            finished.notify_waiters();
        }
        Some(report)
    }
}
