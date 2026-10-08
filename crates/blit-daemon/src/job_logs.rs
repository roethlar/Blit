//! The daemon's job logs (JOB_LOGS jl-1b): one log per job and role this
//! daemon played, written through `blit_core::job_log`, kept in
//! `<state dir>/jobs/logs`, finished at startup when a stopped daemon left
//! one behind, and served by `GetJobLog`.

use blit_core::generated::{
    job_log_chunk, ComparisonMode, FilterSpec, GetJobLogRequest, JobLogChunk, JobLogHeader,
    MirrorMode, ResumeSettings, TransferSummary,
};
use blit_core::job_log::{self, FoundLog, Outcome, Role, RunInfo, RunTag};
use blit_core::remote::transfer::progress::AuditSender;
use blit_core::run_log::{LogPlace, RunLog, RunTotals};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
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

    /// Where a job's [`RunLog`] keeps its file.
    pub(crate) fn place(&self) -> LogPlace {
        LogPlace {
            dir: self.dir.clone(),
            participant: self.participant.clone(),
            keep: self.keep,
            finished: Some(Arc::clone(&self.finished)),
        }
    }

    #[cfg(test)]
    pub(crate) fn start(
        &self,
        run_id: &str,
        attempt: u32,
        role: Role,
        run: RunInfo,
    ) -> Option<blit_core::job_log::LogWriter> {
        self.place().start(run_id, attempt, role, run)
    }
}

/// One job's log as the daemon's parts see it: a [`RunLog`] made when the
/// job registers and owned by the job's dispatcher task, which outlives the
/// raced session future; started once the job's role is known (a served
/// session learns it at open); fed from the job's audit lane; closed by the
/// dispatcher after the race settles. Cheap to clone.
#[derive(Clone)]
pub(crate) struct JobLog {
    log: RunLog,
    /// This daemon's own ID for the job (`t…`).
    job_id: String,
    /// One audit lane per job: a daemon job is one session.
    lane_made: Arc<AtomicBool>,
}

impl JobLog {
    pub(crate) fn new(logs: Option<JobLogs>, job_id: &str) -> Self {
        Self {
            log: RunLog::new(logs.map(|logs| logs.place()), job_id),
            job_id: job_id.to_string(),
            lane_made: Arc::new(AtomicBool::new(false)),
        }
    }

    /// Start the log in `role`, recording this daemon's job ID in
    /// `run-start`; see [`RunLog::start`].
    pub(crate) fn start(&self, role: Role, mut run: RunInfo, tag: Option<RunTag>) {
        run.job_id = Some(self.job_id.clone());
        self.log.start(role, run, tag);
    }

    /// The run ID the log is kept under (the job ID when the initiator sent
    /// none).
    pub(crate) fn run_id(&self) -> String {
        self.log.run_id()
    }

    /// The job's one audit lane (review cr-jl1b-2); `None` when the daemon
    /// keeps no logs, or for a second call.
    pub(crate) fn audit_lane(&self) -> Option<AuditSender> {
        if self.lane_made.swap(true, Ordering::Relaxed) {
            return None;
        }
        self.log.audit_lane()
    }

    #[cfg(test)]
    pub(crate) async fn observe(&self, event: &blit_core::remote::transfer::ProgressEvent) {
        self.log.observe(event).await;
    }

    /// A diagnostic line for the log, recorded at close.
    pub(crate) fn note(&self, line: String) {
        self.log.note(line);
    }

    /// The job's summary, when the part that has it is not the dispatcher.
    pub(crate) fn note_summary(&self, summary: &TransferSummary) {
        self.log.note_totals(RunTotals::from(summary));
    }

    /// How many files failed on their own, by the noted summary.
    pub(crate) fn noted_files_failed(&self) -> u64 {
        self.log.noted_files_failed()
    }

    /// The job's own failure message, kept over the dispatcher's marker.
    pub(crate) fn note_error(&self, message: String) {
        self.log.note_error(message);
    }

    /// Close the log; see [`RunLog::close`].
    pub(crate) async fn close(
        &self,
        outcome: Outcome,
        detail: Option<String>,
        summary: Option<TransferSummary>,
    ) {
        self.log
            .close(outcome, detail, summary.as_ref().map(RunTotals::from))
            .await;
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
    let list = |run: String| {
        let dir = dir.clone();
        async move {
            tokio::task::spawn_blocking(move || job_log::logs_for_run(&dir, &run, role))
                .await
                .map_err(|error| Status::internal(format!("listing job logs: {error}")))?
                .map_err(|error| Status::internal(format!("listing job logs: {error:#}")))
        }
    };
    // The run whose logs are sent: the ID asked for, or — JOB_LOGS jl-2 — the
    // run this daemon's own job ID belongs to (what `blit jobs list` and
    // `--detach` show), since a daemon keeps its log under the run's ID.
    let mut run = id.clone();
    let mut found = list(run.clone()).await?;
    if found.is_empty() {
        let scan_dir = dir.clone();
        let job = id.clone();
        let owner = tokio::task::spawn_blocking(move || job_log::run_for_job(&scan_dir, &job))
            .await
            .map_err(|error| Status::internal(format!("finding a job's run: {error}")))?;
        if let Some(owner) = owner {
            run = owner;
            found = list(run.clone()).await?;
        }
    }
    if request.wait_finished {
        // Review cr-jl1c-1: wait here, woken as logs finish, rather than
        // have the caller re-download an unfinished log. A job with no log
        // at all gets none by waiting: its log starts at its open.
        let deadline = tokio::time::Instant::now() + WAIT_FINISHED_LIMIT;
        while !found.is_empty() && found.iter().any(|log| !log.finished) {
            let notified = logs.finished.notified();
            tokio::pin!(notified);
            notified.as_mut().enable();
            found = list(run.clone()).await?;
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
            found = list(run.clone()).await?;
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
            job_id: None,
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
        let destination = logs
            .start("t1-0", 1, Role::Destination, run_info())
            .unwrap();
        destination.record(EventBody::FileCopied {
            path: "a".into(),
            bytes: Some(1),
            raw: None,
        });
        destination.finish(Outcome::Ok, None);
        // Still running: served as its partial. A phase change syncs it.
        let source = logs.start("t1-0", 1, Role::Source, run_info()).unwrap();
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
        log.start(Role::Destination, run_info(), None);
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
        log.start(Role::Destination, run_info(), None);
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
        wait_finished_case(None, "t1-0").await;
    }

    /// JOB_LOGS jl-2: a log kept under the initiator's run ID, asked for by
    /// this daemon's own job ID, is waited for under the run's ID.
    #[tokio::test]
    async fn wait_finished_follows_a_job_id_to_its_runs_log() {
        let tag = RunTag {
            run_id: "0123456789abcdef0123456789abcdef".into(),
            attempt: 1,
        };
        wait_finished_case(Some(tag), "t1-0").await;
    }

    /// Start job `t1-0`'s log (under `tag`'s run when given), ask for it by
    /// `ask` with `wait_finished`, and check the reply waits for the close
    /// and then sends the finished log.
    async fn wait_finished_case(tag: Option<RunTag>, ask: &str) {
        let state = tempfile::tempdir().unwrap();
        let logs = JobLogs::open(state.path(), job_log::DEFAULT_KEEP).unwrap();
        let log = JobLog::new(Some(logs.clone()), "t1-0");
        log.start(Role::Destination, run_info(), tag);
        // As in a real job, the log's file — header first — exists well
        // before anyone asks (the writer makes it on its own thread just
        // after the start).
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(10);
        while job_log::run_for_job(logs.dir(), "t1-0").is_none() {
            assert!(
                std::time::Instant::now() < deadline,
                "the log never appeared"
            );
            tokio::time::sleep(std::time::Duration::from_millis(10)).await;
        }
        let request = GetJobLogRequest {
            transfer_id: ask.into(),
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
