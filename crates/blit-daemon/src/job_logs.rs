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
use blit_core::remote::transfer::ProgressEvent;
use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::Instant;
use tokio::io::AsyncReadExt;
use tokio::sync::mpsc;
use tokio_stream::wrappers::ReceiverStream;
use tonic::{Response, Status};

/// Bytes per `data` chunk of a served log.
const CHUNK_BYTES: usize = 64 * 1024;

/// Where this daemon's logs live, and whose they are.
#[derive(Clone, Debug)]
pub(crate) struct JobLogs {
    dir: PathBuf,
    participant: String,
    keep: usize,
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
}

struct ActiveLog {
    writer: LogWriter,
    sender: LogSender,
    role: Role,
    started: Instant,
    planned_files: u64,
    planned_bytes: u64,
    deleting: bool,
    /// Failed files already named, so the summary's list adds only the
    /// rest.
    failed: HashSet<String>,
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
            planned_files: 0,
            planned_bytes: 0,
            deleting: false,
            failed: HashSet::new(),
        });
    }

    /// Record what a progress event names: each file copied (or, on a
    /// source, sent), failed or deleted, and the phase changes.
    pub(crate) async fn observe(&self, event: &ProgressEvent) {
        let (sender, bodies) = {
            let mut state = self.state();
            let Some(active) = state.active.as_mut() else {
                return;
            };
            let bodies = match event {
                ProgressEvent::FileComplete { path } => vec![match active.role {
                    Role::Source => EventBody::FileSent { path: path.clone() },
                    _ => EventBody::FileCopied {
                        path: path.clone(),
                        bytes: None,
                    },
                }],
                ProgressEvent::FileFailed { path, reason } => {
                    active.failed.insert(path.clone());
                    vec![EventBody::FileFailed {
                        path: path.clone(),
                        reason: reason.clone(),
                    }]
                }
                ProgressEvent::Deleted { path } => {
                    vec![EventBody::FileDeleted { path: path.clone() }]
                }
                ProgressEvent::ManifestBatch { files, bytes } => {
                    active.planned_files = active.planned_files.saturating_add(*files as u64);
                    active.planned_bytes = active.planned_bytes.saturating_add(*bytes);
                    vec![]
                }
                ProgressEvent::DiffComplete => vec![EventBody::Diagnostic {
                    message: format!(
                        "compared the whole source list: {} file(s), {} to send",
                        active.planned_files,
                        format_bytes(active.planned_bytes)
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
        let (active, notes, summary, error, run_id) = {
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
        let elapsed = active.started.elapsed();
        if active.planned_files > 0 {
            bodies.push(EventBody::Diagnostic {
                message: format!(
                    "planned: {} file(s), {}",
                    active.planned_files,
                    format_bytes(active.planned_bytes)
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
    let found = {
        let dir = dir.clone();
        let id = id.clone();
        tokio::task::spawn_blocking(move || job_log::logs_for_run(&dir, &id, role))
            .await
            .map_err(|error| Status::internal(format!("listing job logs: {error}")))?
            .map_err(|error| Status::internal(format!("listing job logs: {error:#}")))?
    };
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
                bytes: Some(1)
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
