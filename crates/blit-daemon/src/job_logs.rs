//! The daemon's job logs (JOB_LOGS jl-1b): one log per job and role this
//! daemon played, written through `blit_core::job_log`, kept in
//! `<state dir>/jobs/logs`, finished at startup when a stopped daemon left
//! one behind, and served by `GetJobLog`.

use blit_core::display::format_bytes;
use blit_core::generated::{
    job_log_chunk, ComparisonMode, FilterSpec, GetJobLogRequest, JobLogChunk, JobLogHeader,
    MirrorMode, ResumeSettings, TransferSummary,
};
use blit_core::job_log::{
    self, EventBody, FoundLog, LogKey, LogSender, LogWriter, Outcome, PhaseState, Role, RunInfo,
    Summary,
};
use blit_core::remote::transfer::progress::{
    audit_lane, AuditSender, PlannedTotals, AUDIT_LANE_DEPTH,
};
use blit_core::remote::transfer::ProgressEvent;
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::Instant;
use tokio::io::AsyncReadExt;
use tokio::sync::mpsc;
use tokio_stream::wrappers::ReceiverStream;
use tonic::{Response, Status};

/// The longest `GetJobLog` waits for a job's logs to finish.
const WAIT_FINISHED_LIMIT: std::time::Duration = std::time::Duration::from_secs(60);

/// Bytes per `data` chunk of a served log.
const CHUNK_BYTES: usize = 64 * 1024;

/// Where this daemon's logs live, and whose they are.
#[derive(Clone, Debug)]
pub(crate) struct JobLogs {
    dir: PathBuf,
    participant: String,
    keep: usize,
    /// Woken whenever one of this daemon's logs finishes (review
    /// cr-jl1c-1), for `GetJobLog`'s `wait_finished`.
    finished: Arc<tokio::sync::Notify>,
}

impl JobLogs {
    /// The logs under `state_dir`, kept to the newest `keep`, under this
    /// machine's ID (made on first use). First finishes every log a stopped
    /// daemon left behind. Blocking: call it before serving.
    pub(crate) fn open(state_dir: &Path, keep: usize) -> std::io::Result<Self> {
        let participant = job_log::machine_id(state_dir)?;
        let dir = state_dir.join("jobs").join("logs");
        let recovery = job_log::recover(&dir);
        if !recovery.finished.is_empty() {
            eprintln!(
                "blitd: finished {} job log(s) left by an earlier run as interrupted",
                recovery.finished.len()
            );
        }
        for problem in &recovery.problems {
            log::warn!("job log recovery: {problem}");
        }
        Ok(Self {
            dir,
            participant,
            keep,
            finished: Arc::new(tokio::sync::Notify::new()),
        })
    }

    #[cfg(test)]
    pub(crate) fn dir(&self) -> &Path {
        &self.dir
    }

    /// Start the log of job `run_id` in `role`. `None`, with a warning, only
    /// when the ID cannot name a log; a log never stops a job.
    pub(crate) fn start(&self, run_id: &str, role: Role, run: RunInfo) -> Option<LogWriter> {
        match LogKey::new(run_id, self.participant.clone(), role, 1) {
            Ok(key) => Some(LogWriter::start(&self.dir, key, run, self.keep)),
            Err(error) => {
                log::warn!("job {run_id}: not logged: {error:#}");
                None
            }
        }
    }
}

/// What `TransferSummary.failed_paths` names but a file's own failure was
/// never seen live gets this reason when the summary's capped list kept
/// none.
const NO_REASON_REPORTED: &str = "failed (the transfer's report kept no reason)";

/// How long closing a log waits for its audit lane to drain.
const RELAY_DRAIN_LIMIT: std::time::Duration = std::time::Duration::from_secs(30);

/// One job's log as the daemon's parts see it. Made when the job
/// registers and owned by the job's dispatcher task, which outlives the
/// raced session future; started once the job's role is known (a served
/// session learns it at open); fed from the progress relays; closed by the
/// dispatcher after the race settles. Cheap to clone.
#[derive(Clone)]
pub(crate) struct JobLog {
    inner: Arc<Mutex<LogState>>,
}

struct LogState {
    logs: Option<JobLogs>,
    run_id: String,
    active: Option<ActiveLog>,
    /// Diagnostics to record at close.
    notes: Vec<String>,
    summary: Option<TransferSummary>,
    /// The job's own failure message, when the dispatcher only has a
    /// marker (a delegated pull's phased error).
    error: Option<String>,
    /// The task draining the job's audit lane into the log (review
    /// cr-jl1b-2), and the lane's planned totals.
    relay: Option<tokio::task::JoinHandle<()>>,
    planned: Option<Arc<PlannedTotals>>,
}

struct ActiveLog {
    writer: LogWriter,
    sender: LogSender,
    role: Role,
    started: Instant,
    deleting: bool,
    /// Failed files already named, so the summary's list adds only the
    /// rest.
    failed: HashSet<String>,
    /// The exact bytes, escaped, of each name in this run that is not
    /// valid UTF-8, by its text (review cr-jl1a-1). The first wins, as at
    /// both ends of the transfer.
    raw_names: HashMap<String, String>,
}

impl JobLog {
    pub(crate) fn new(logs: Option<JobLogs>, run_id: &str) -> Self {
        Self {
            inner: Arc::new(Mutex::new(LogState {
                logs,
                run_id: run_id.to_string(),
                active: None,
                notes: Vec::new(),
                summary: None,
                error: None,
                relay: None,
                planned: None,
            })),
        }
    }

    fn state(&self) -> std::sync::MutexGuard<'_, LogState> {
        self.inner
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
    }

    /// Start the log in `role`; once only, and only when the daemon keeps
    /// logs. Safe from sync code inside a runtime: the only event it
    /// records goes into a fresh, empty queue, so it never waits.
    pub(crate) fn start(&self, role: Role, run: RunInfo) {
        let mut state = self.state();
        if state.active.is_some() {
            return;
        }
        let Some(writer) = state
            .logs
            .as_ref()
            .and_then(|logs| logs.start(&state.run_id, role, run))
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

    /// The bounded lane this job's transfer reports its log's facts on, and
    /// the task that drains it into the log (review cr-jl1b-2): while the
    /// log falls behind, the transfer waits. Made once per job; `None` when
    /// the daemon keeps no logs, or for a second call. The dispatcher's
    /// [`close`](Self::close) waits for the lane to drain, which it does
    /// once the transfer has dropped every sender.
    pub(crate) fn audit_lane(&self) -> Option<AuditSender> {
        let mut state = self.state();
        if state.logs.is_none() || state.relay.is_some() {
            return None;
        }
        let (sender, receiver) = audit_lane(AUDIT_LANE_DEPTH);
        state.planned = Some(receiver.planned());
        let log = self.clone();
        state.relay = Some(tokio::spawn(async move {
            while let Some(event) = receiver.recv().await {
                log.observe(&event).await;
            }
        }));
        Some(sender)
    }

    /// Record what a progress event names: each file copied (or, on a
    /// source, sent), failed or deleted, and the phase changes.
    pub(crate) async fn observe(&self, event: &ProgressEvent) {
        let (sender, bodies) = {
            let mut state = self.state();
            let (planned_files, planned_bytes) = state
                .planned
                .as_ref()
                .map_or((0, 0), |planned| planned.get());
            let Some(active) = state.active.as_mut() else {
                return;
            };
            let raw = |path: &str| active.raw_names.get(path).cloned();
            let bodies = match event {
                ProgressEvent::FileComplete { path } => vec![match active.role {
                    Role::Source => EventBody::FileSent {
                        path: path.clone(),
                        raw: raw(path),
                    },
                    _ => EventBody::FileCopied {
                        path: path.clone(),
                        bytes: None,
                        raw: raw(path),
                    },
                }],
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
                    raw: raw.as_deref().map(blit_core::raw_name::escape_raw),
                }],
                ProgressEvent::RawName { path, raw } => {
                    active
                        .raw_names
                        .entry(path.clone())
                        .or_insert_with(|| blit_core::raw_name::escape_raw(raw));
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

    /// A diagnostic line for the log, recorded at close.
    pub(crate) fn note(&self, line: String) {
        self.state().notes.push(line);
    }

    /// The job's summary, when the part that has it is not the dispatcher.
    pub(crate) fn note_summary(&self, summary: &TransferSummary) {
        self.state().summary = Some(summary.clone());
    }

    /// How many files failed on their own, by the noted summary.
    pub(crate) fn noted_files_failed(&self) -> u64 {
        self.state()
            .summary
            .as_ref()
            .map_or(0, |summary| summary.files_failed)
    }

    /// The job's own failure message, kept over the dispatcher's marker.
    pub(crate) fn note_error(&self, message: String) {
        self.state().error = Some(message);
    }

    /// Close the log: name the failed files the summary lists that were not
    /// seen live, end the open phase, record the diagnostics and the
    /// summary, then `run-end`, and finish the file.
    pub(crate) async fn close(
        &self,
        outcome: Outcome,
        detail: Option<String>,
        summary: Option<TransferSummary>,
    ) {
        // Every fact the transfer reported reaches the log before it
        // closes: the relay ends once the transfer has dropped its senders.
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
        let (active, notes, summary, error, run_id, planned) = {
            let mut state = self.state();
            let Some(active) = state.active.take() else {
                return;
            };
            (
                active,
                std::mem::take(&mut state.notes),
                summary.or_else(|| state.summary.take()),
                state.error.take(),
                state.run_id.clone(),
                state
                    .planned
                    .as_ref()
                    .map_or((0, 0), |planned| planned.get()),
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
        if let Some(summary) = &summary {
            for path in &summary.failed_paths {
                if active.failed.contains(path) {
                    continue;
                }
                let reason = summary
                    .failures
                    .iter()
                    .find(|failure| &failure.relative_path == path)
                    .map_or(NO_REASON_REPORTED, |failure| failure.reason.as_str());
                bodies.push(EventBody::FileFailed {
                    path: path.clone(),
                    reason: reason.to_string(),
                    raw: active.raw_names.get(path).cloned(),
                });
            }
            if summary.failed_paths_truncated {
                bodies.push(EventBody::Diagnostic {
                    message: format!(
                        "the list of failed files was cut short for size; {} failed in all",
                        summary.files_failed
                    ),
                });
            }
        }
        if relay_cut {
            bodies.push(EventBody::Diagnostic {
                message: "the transfer still held its log lane open when the job ended; \
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
        if let Some(summary) = &summary {
            bodies.push(EventBody::Diagnostic {
                message: if summary.in_stream_carrier_used {
                    "payload carrier: in-stream (gRPC)".into()
                } else {
                    "payload carrier: TCP data plane".into()
                },
            });
            if summary.files_resumed > 0 {
                bodies.push(EventBody::Diagnostic {
                    message: format!("resumed: {} file(s)", summary.files_resumed),
                });
            }
            let seconds = elapsed.as_secs_f64();
            if seconds > 0.0 && summary.bytes_transferred > 0 {
                bodies.push(EventBody::Diagnostic {
                    message: format!(
                        "average: {}/s over the whole job",
                        format_bytes((summary.bytes_transferred as f64 / seconds) as u64)
                    ),
                });
            }
            bodies.push(EventBody::Summary(Summary {
                files_copied: summary.files_transferred,
                files_deleted: summary.entries_deleted,
                files_failed: summary.files_failed,
                bytes_copied: summary.bytes_transferred,
                elapsed_ms: u64::try_from(elapsed.as_millis()).unwrap_or(u64::MAX),
            }));
        }
        for body in bodies {
            active.sender.record_async(body).await;
        }
        let detail = error.or(detail);
        let writer = active.writer;
        match tokio::task::spawn_blocking(move || writer.finish(outcome, detail)).await {
            Ok(report) if !report.complete => log::warn!(
                "job {run_id}: its log is incomplete: {}",
                report.problem.unwrap_or_default()
            ),
            Ok(_) => {}
            Err(error) => log::warn!("job {run_id}: closing its log failed: {error}"),
        }
        if let Some(logs) = self.state().logs.as_ref() {
            logs.finished.notify_waiters();
        }
    }
}

/// How a job ended, for its log: `ok` and `error` as the dispatcher scored
/// them, whether its `CancelJob` token fired, and how many files failed on
/// their own.
pub(crate) fn job_outcome(
    ok: bool,
    error: Option<&str>,
    cancelled: bool,
    files_failed: u64,
) -> (Outcome, Option<String>) {
    if ok {
        return if files_failed > 0 {
            (
                Outcome::Failed,
                Some(format!("{files_failed} file(s) failed")),
            )
        } else {
            (Outcome::Ok, None)
        };
    }
    if cancelled {
        return (Outcome::Cancelled, Some("cancelled via CancelJob".into()));
    }
    if error == Some(CLIENT_HUNG_UP) {
        return (
            Outcome::Cancelled,
            Some("the client hung up before the job finished".into()),
        );
    }
    (Outcome::Failed, error.map(str::to_string))
}

/// The dispatcher's error marker for a job whose client hung up mid-run.
pub(crate) const CLIENT_HUNG_UP: &str = "client cancelled";

/// The transfer options a log's `run-start` lists, in words.
pub(crate) fn option_words(
    compare_mode: ComparisonMode,
    mirror: Option<MirrorMode>,
    filter: Option<&FilterSpec>,
    resume: Option<&ResumeSettings>,
    ignore_existing: bool,
    require_complete_scan: bool,
) -> Vec<String> {
    let word = |name: &str, prefix: &str| {
        name.trim_start_matches(prefix)
            .to_lowercase()
            .replace('_', "-")
    };
    let mut words = Vec::new();
    if !matches!(
        compare_mode,
        ComparisonMode::Unspecified | ComparisonMode::SizeMtime
    ) {
        words.push(format!(
            "compare={}",
            word(compare_mode.as_str_name(), "COMPARISON_MODE_")
        ));
    }
    if let Some(mirror) =
        mirror.filter(|mirror| !matches!(mirror, MirrorMode::Unspecified | MirrorMode::Off))
    {
        words.push(format!(
            "mirror={}",
            word(mirror.as_str_name(), "MIRROR_MODE_")
        ));
    }
    if let Some(filter) = filter {
        words.extend(filter.include.iter().map(|glob| format!("include={glob}")));
        words.extend(filter.exclude.iter().map(|glob| format!("exclude={glob}")));
        if let Some(min) = filter.min_size {
            words.push(format!("min-size={min}"));
        }
        if let Some(max) = filter.max_size {
            words.push(format!("max-size={max}"));
        }
    }
    if resume.is_some_and(|resume| resume.enabled) {
        words.push("resume".into());
    }
    if ignore_existing {
        words.push("ignore-existing".into());
    }
    if require_complete_scan {
        words.push("require-complete-scan".into());
    }
    words
}

pub(crate) type JobLogStream = ReceiverStream<Result<JobLogChunk, Status>>;

/// `GetJobLog`: every log this daemon kept for the job (in one role, when
/// asked), each as a header and then its bytes as stored.
pub(crate) async fn serve(
    logs: Option<&JobLogs>,
    request: GetJobLogRequest,
) -> Result<Response<JobLogStream>, Status> {
    let id = request.transfer_id;
    if !job_log::valid_id(&id) {
        return Err(Status::invalid_argument(format!("{id:?} is not a job ID")));
    }
    let role = if request.role.is_empty() {
        None
    } else {
        Some(
            request
                .role
                .parse::<Role>()
                .map_err(|error| Status::invalid_argument(format!("{error:#}")))?,
        )
    };
    let Some(logs) = logs else {
        return Err(Status::not_found("this daemon keeps no job logs"));
    };
    let dir = logs.dir.clone();
    let list = || {
        let dir = dir.clone();
        let id = id.clone();
        async move {
            tokio::task::spawn_blocking(move || job_log::logs_for_run(&dir, &id, role))
                .await
                .map_err(|error| Status::internal(format!("listing job logs: {error}")))?
                .map_err(|error| Status::internal(format!("listing job logs: {error:#}")))
        }
    };
    let mut found = list().await?;
    if request.wait_finished {
        // Review cr-jl1c-1: wait here, woken as logs finish, rather than
        // have the caller re-download an unfinished log. A job with no log
        // at all gets none by waiting: its log starts at its open.
        let deadline = tokio::time::Instant::now() + WAIT_FINISHED_LIMIT;
        while !found.is_empty() && found.iter().any(|log| !log.finished) {
            let notified = logs.finished.notified();
            tokio::pin!(notified);
            notified.as_mut().enable();
            found = list().await?;
            if found.iter().all(|log| log.finished) {
                break;
            }
            let now = tokio::time::Instant::now();
            if now >= deadline {
                break;
            }
            // The re-check after a short sleep covers a log finished by
            // another path (startup recovery) that wakes no one.
            let _ = tokio::time::timeout(
                (deadline - now).min(std::time::Duration::from_secs(1)),
                notified,
            )
            .await;
            found = list().await?;
        }
    }
    if found.is_empty() {
        return Err(Status::not_found(match role {
            Some(role) => format!("no {} log for job {id}", role.as_str()),
            None => format!("no log for job {id}"),
        }));
    }
    let (tx, rx) = mpsc::channel(8);
    tokio::spawn(async move {
        for log in found {
            match send_log(&dir, log, &tx).await {
                Ok(true) => {}
                // The caller hung up.
                Ok(false) => return,
                Err(status) => {
                    let _ = tx.send(Err(status)).await;
                    return;
                }
            }
        }
    });
    Ok(Response::new(ReceiverStream::new(rx)))
}

/// Send one log; `Ok(false)` when the caller has gone.
async fn send_log(
    dir: &Path,
    log: FoundLog,
    tx: &mpsc::Sender<Result<JobLogChunk, Status>>,
) -> Result<bool, Status> {
    let unreadable =
        |error: std::io::Error| Status::internal(format!("reading a job log: {error}"));
    let (mut file, finished) = match tokio::fs::File::open(&log.path).await {
        Ok(file) => (file, log.finished),
        // The run finished between listing and opening: its partial is gone
        // and its finished log is in place.
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            let dir = dir.to_path_buf();
            let key = log.key.clone();
            let Some(now) = tokio::task::spawn_blocking(move || job_log::find_log(&dir, &key))
                .await
                .map_err(|error| Status::internal(format!("finding a job log: {error}")))?
            else {
                return Ok(true);
            };
            (
                tokio::fs::File::open(&now.path).await.map_err(unreadable)?,
                now.finished,
            )
        }
        Err(error) => return Err(unreadable(error)),
    };
    let header = JobLogHeader {
        role: log.key.role().as_str().into(),
        participant: log.key.participant().into(),
        attempt: log.key.attempt(),
        finished,
    };
    let chunk = |payload| {
        Ok(JobLogChunk {
            payload: Some(payload),
        })
    };
    if tx
        .send(chunk(job_log_chunk::Payload::Header(header)))
        .await
        .is_err()
    {
        return Ok(false);
    }
    let mut buf = vec![0u8; CHUNK_BYTES];
    loop {
        let read = file.read(&mut buf).await.map_err(unreadable)?;
        if read == 0 {
            return Ok(true);
        }
        if tx
            .send(chunk(job_log_chunk::Payload::Data(buf[..read].to_vec())))
            .await
            .is_err()
        {
            return Ok(false);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use blit_core::job_log::{EventBody, LogLine, LogLines, Outcome};
    use std::io::Read;
    use tokio_stream::StreamExt;

    fn run_info() -> RunInfo {
        RunInfo {
            verb: "push".into(),
            source: "client".into(),
            destination: "backup:/".into(),
            options: vec![],
        }
    }

    /// Every chunk the stream sends, as (header, bytes) per log.
    async fn fetch(
        logs: Option<&JobLogs>,
        id: &str,
        role: &str,
    ) -> Result<Vec<(JobLogHeader, Vec<u8>)>, Status> {
        let mut stream = serve(
            logs,
            GetJobLogRequest {
                transfer_id: id.into(),
                role: role.into(),
                wait_finished: false,
            },
        )
        .await?
        .into_inner();
        let mut fetched: Vec<(JobLogHeader, Vec<u8>)> = Vec::new();
        while let Some(chunk) = stream.next().await {
            match chunk?.payload.unwrap() {
                job_log_chunk::Payload::Header(header) => fetched.push((header, Vec::new())),
                job_log_chunk::Payload::Data(bytes) => {
                    fetched.last_mut().unwrap().1.extend_from_slice(&bytes)
                }
            }
        }
        Ok(fetched)
    }

    fn bodies(bytes: Vec<u8>) -> Vec<EventBody> {
        LogLines::new(job_log::decode(std::io::Cursor::new(bytes)).unwrap())
            .map(|line| match line.unwrap() {
                LogLine::Event(event) => event.body,
                other => panic!("unreadable: {other:?}"),
            })
            .collect()
    }

    #[tokio::test]
    async fn a_jobs_logs_are_served_by_role() {
        let state = tempfile::tempdir().unwrap();
        let logs = JobLogs::open(state.path(), job_log::DEFAULT_KEEP).unwrap();
        let destination = logs.start("t1-0", Role::Destination, run_info()).unwrap();
        destination.record(EventBody::FileCopied {
            path: "a".into(),
            bytes: Some(1),
            raw: None,
        });
        destination.finish(Outcome::Ok, None);
        // Still running: served as its partial. A phase change syncs it.
        let source = logs.start("t1-0", Role::Source, run_info()).unwrap();
        source.record(EventBody::Phase {
            name: "transfer".into(),
            state: job_log::PhaseState::Start,
        });
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(10);
        while !job_log::logs_for_run(logs.dir(), "t1-0", Some(Role::Source))
            .unwrap()
            .first()
            .is_some_and(|log| std::fs::metadata(&log.path).is_ok_and(|meta| meta.len() > 0))
        {
            assert!(
                std::time::Instant::now() < deadline,
                "the source partial never synced"
            );
            tokio::time::sleep(std::time::Duration::from_millis(20)).await;
        }

        let all = fetch(Some(&logs), "t1-0", "").await.unwrap();
        let summary: Vec<(String, bool)> = all
            .iter()
            .map(|(header, _)| (header.role.clone(), header.finished))
            .collect();
        assert_eq!(
            summary,
            [
                ("destination".to_string(), true),
                ("source".to_string(), false)
            ]
        );
        assert_eq!(all[0].0.participant, logs.participant);
        assert_eq!(all[0].0.attempt, 1);
        let destination = bodies(all[0].1.clone());
        assert_eq!(
            destination[1],
            EventBody::FileCopied {
                path: "a".into(),
                bytes: Some(1),
                raw: None,
            }
        );
        assert!(matches!(destination[0], EventBody::RunStart(_)));
        assert!(matches!(
            destination.last(),
            Some(EventBody::RunEnd {
                outcome: Outcome::Ok,
                ..
            })
        ));

        let only = fetch(Some(&logs), "t1-0", "destination").await.unwrap();
        assert_eq!(only.len(), 1);
        assert_eq!(only[0].0.role, "destination");
        // The compressed bytes travel as stored.
        let mut stored = Vec::new();
        std::fs::File::open(
            job_log::logs_for_run(logs.dir(), "t1-0", Some(Role::Destination)).unwrap()[0]
                .path
                .clone(),
        )
        .unwrap()
        .read_to_end(&mut stored)
        .unwrap();
        assert_eq!(only[0].1, stored);
        source.finish(Outcome::Ok, None);
    }

    #[tokio::test]
    async fn a_missing_or_malformed_job_is_refused_plainly() {
        let state = tempfile::tempdir().unwrap();
        let logs = JobLogs::open(state.path(), job_log::DEFAULT_KEEP).unwrap();
        let code =
            |result: Result<Vec<(JobLogHeader, Vec<u8>)>, Status>| result.unwrap_err().code();

        assert_eq!(
            code(fetch(Some(&logs), "t9-9", "").await),
            tonic::Code::NotFound
        );
        assert_eq!(
            code(fetch(Some(&logs), "../t1", "").await),
            tonic::Code::InvalidArgument
        );
        assert_eq!(
            code(fetch(Some(&logs), "t1-0", "owner").await),
            tonic::Code::InvalidArgument
        );
        assert_eq!(code(fetch(None, "t1-0", "").await), tonic::Code::NotFound);
    }

    /// Review cr-jl1b-2: closing a log waits for its audit lane to drain,
    /// so every fact the transfer handed over is in the log.
    #[tokio::test]
    async fn closing_a_log_waits_for_its_lane_to_drain() {
        let state = tempfile::tempdir().unwrap();
        let logs = JobLogs::open(state.path(), job_log::DEFAULT_KEEP).unwrap();
        let log = JobLog::new(Some(logs.clone()), "t1-0");
        log.start(Role::Destination, run_info());
        let (ui_tx, _ui_rx) = tokio::sync::mpsc::unbounded_channel();
        let progress = blit_core::remote::transfer::RemoteTransferProgress::new(ui_tx)
            .with_audit(log.audit_lane().expect("a lane"));
        assert!(log.audit_lane().is_none(), "one lane per job");
        for n in 0..1000 {
            progress.report_file_complete(format!("f{n}")).await;
        }
        drop(progress);
        log.close(Outcome::Ok, None, None).await;

        let found = job_log::logs_for_run(logs.dir(), "t1-0", None).unwrap();
        let copied = job_log::open_log(&found[0].path)
            .unwrap()
            .filter(|line| {
                matches!(
                    line,
                    Ok(LogLine::Event(job_log::Event {
                        body: EventBody::FileCopied { .. },
                        ..
                    }))
                )
            })
            .count();
        assert_eq!(copied, 1000);
    }

    /// Review cr-jl1a-1: a name that is not valid UTF-8 is logged with its
    /// exact bytes — from the manifest's raw name, the summary's fill-in,
    /// and the mirror pass's own path — so two such names never read the
    /// same.
    #[tokio::test]
    async fn a_raw_name_is_logged_with_its_exact_bytes() {
        use blit_core::remote::transfer::ProgressEvent;
        let state = tempfile::tempdir().unwrap();
        let logs = JobLogs::open(state.path(), job_log::DEFAULT_KEEP).unwrap();
        let log = JobLog::new(Some(logs.clone()), "t1-0");
        log.start(Role::Destination, run_info());
        let lossy = "caf\u{fffd}.txt".to_string();
        for event in [
            ProgressEvent::RawName {
                path: lossy.clone(),
                raw: b"caf\xe9.txt".to_vec(),
            },
            // A second name collapsing to the same text never replaces the
            // first, as at both ends.
            ProgressEvent::RawName {
                path: lossy.clone(),
                raw: b"caf\xe8.txt".to_vec(),
            },
            ProgressEvent::FileComplete {
                path: lossy.clone(),
            },
            ProgressEvent::Deleted {
                path: "old\u{fffd}/".into(),
                raw: Some(b"old\xff/".to_vec()),
            },
        ] {
            log.observe(&event).await;
        }
        let summary = TransferSummary {
            files_failed: 1,
            failed_paths: vec![lossy.clone()],
            ..Default::default()
        };
        log.close(Outcome::Failed, None, Some(summary)).await;

        let found = job_log::logs_for_run(logs.dir(), "t1-0", None).unwrap();
        let named: Vec<(String, Option<String>)> = job_log::open_log(&found[0].path)
            .unwrap()
            .filter_map(|line| match line.unwrap() {
                LogLine::Event(job_log::Event {
                    body:
                        EventBody::FileCopied { path, raw, .. }
                        | EventBody::FileDeleted { path, raw }
                        | EventBody::FileFailed { path, raw, .. },
                    ..
                }) => Some((path, raw)),
                _ => None,
            })
            .collect();
        assert_eq!(
            named,
            [
                (lossy.clone(), Some("caf\\xe9.txt".to_string())),
                ("old\u{fffd}/".to_string(), Some("old\\xff/".to_string())),
                (lossy, Some("caf\\xe9.txt".to_string())),
            ]
        );
    }

    /// Review cr-jl1c-1: with `wait_finished`, `GetJobLog` holds the reply
    /// until the job's log closes and then sends it whole, once.
    #[tokio::test]
    async fn wait_finished_sends_the_log_once_it_closes() {
        let state = tempfile::tempdir().unwrap();
        let logs = JobLogs::open(state.path(), job_log::DEFAULT_KEEP).unwrap();
        let log = JobLog::new(Some(logs.clone()), "t1-0");
        log.start(Role::Destination, run_info());
        // As in a real job, the log's file exists well before anyone asks
        // (the writer makes it on its own thread just after the start).
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(10);
        while job_log::logs_for_run(logs.dir(), "t1-0", None)
            .unwrap()
            .is_empty()
        {
            assert!(
                std::time::Instant::now() < deadline,
                "the log never appeared"
            );
            tokio::time::sleep(std::time::Duration::from_millis(10)).await;
        }
        let request = GetJobLogRequest {
            transfer_id: "t1-0".into(),
            role: String::new(),
            wait_finished: true,
        };
        let serving = {
            let logs = logs.clone();
            tokio::spawn(async move { serve(Some(&logs), request).await })
        };
        tokio::time::sleep(std::time::Duration::from_millis(300)).await;
        assert!(!serving.is_finished(), "the reply waited for the log");

        log.close(Outcome::Ok, None, None).await;
        // Woken by the close itself, well inside the one-second re-check.
        let mut stream = tokio::time::timeout(std::time::Duration::from_millis(400), serving)
            .await
            .expect("woken when the log closed")
            .unwrap()
            .unwrap()
            .into_inner();
        let mut headers = Vec::new();
        while let Some(chunk) = stream.next().await {
            if let job_log_chunk::Payload::Header(header) = chunk.unwrap().payload.unwrap() {
                headers.push(header.finished);
            }
        }
        assert_eq!(headers, [true]);
    }

    #[test]
    fn opening_finishes_logs_a_stopped_daemon_left() {
        let state = tempfile::tempdir().unwrap();
        let dir = state.path().join("jobs").join("logs");
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("t1-0.m1.destination.1.partial.jsonl"), b"").unwrap();

        let logs = JobLogs::open(state.path(), job_log::DEFAULT_KEEP).unwrap();

        let found = job_log::logs_for_run(logs.dir(), "t1-0", None).unwrap();
        assert_eq!(found.len(), 1);
        assert!(found[0].finished);
    }
}
