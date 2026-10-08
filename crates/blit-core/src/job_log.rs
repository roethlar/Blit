//! Job logs — one participant's forensic record of one run (JOB_LOGS jl-1a).
//!
//! `docs/plan/JOB_LOGS.md` ("Log keys and event schema"; "Crash safety,
//! pruning and backpressure") is the contract this module implements:
//!
//! - **Schema.** One JSON [`Event`] per line, each with a wall-clock
//!   timestamp, a sequence number and a `kind`. The first event is
//!   `run-start`, carrying [`FORMAT`] and [`VERSION`]. Readers skip fields
//!   they do not know and read a kind they do not know as
//!   [`EventBody::Unknown`], so adding a field or a kind is not a break; a
//!   change an older reader would misread bumps [`VERSION`].
//! - **Key.** A log is keyed by run ID + participant + role + attempt
//!   ([`LogKey`]), so one daemon that plays two roles in one run writes two
//!   logs, never one interleaved file.
//! - **Writing.** [`LogWriter::start`] appends whole lines to
//!   `<key>.partial.jsonl` from a thread of its own, fed through a bounded
//!   queue: when the thread falls behind, the producer waits rather than
//!   drop an event, because a log with holes misleads. The partial is synced
//!   at every phase change and every few seconds. [`LogWriter::finish`]
//!   writes `run-end`, compresses the partial to `<key>.jsonl.gz` through a
//!   temp file and one rename, removes the partial, then prunes the folder.
//! - **Ownership.** A writer holds an exclusive lock on a sidecar
//!   `<key>.lock` for its whole life. The lock is on a separate file because
//!   a Windows lock on the partial itself would stop a reader from serving
//!   the active log.
//! - **Crash.** [`recover`] (run at startup) finishes every partial whose
//!   lock it can take — its writer is gone — with a `run-end` marked
//!   interrupted. [`open_log`] tolerates the torn last line a crash leaves.
//! - **Retention.** [`prune`] keeps the newest `keep` finished logs, under a
//!   folder lock; it never touches a partial or any file that is not a log.
//! - **Logging never fails a transfer** (R17). Nothing here returns an error
//!   to the transfer: if the log cannot be written, logging stops, a
//!   `log-incomplete` event is attempted, later events are dropped (so a
//!   producer never waits on a dead log), and the [`LogReport`] says the log
//!   is incomplete and why.

use flate2::{bufread::MultiGzDecoder, write::GzEncoder, Compression};
use serde::{Deserialize, Serialize};
use std::fs::{self, File, OpenOptions, TryLockError};
use std::io::{self, BufRead, BufReader, BufWriter, Read, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};
use std::thread::JoinHandle;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

/// The `format` every log's `run-start` event carries.
pub const FORMAT: &str = "blit-job-log";
/// The schema version this build writes.
pub const VERSION: u32 = 1;
/// How many finished logs a machine keeps when its settings do not say (R7).
pub const DEFAULT_KEEP: usize = 50;

const PARTIAL_SUFFIX: &str = ".partial.jsonl";
const FINISHED_SUFFIX: &str = ".jsonl.gz";
const TEMP_SUFFIX: &str = ".jsonl.gz.tmp";
const LOCK_SUFFIX: &str = ".lock";
/// The folder-wide lock [`prune`] holds. Not a log name: a key has four parts.
const PRUNE_LOCK: &str = "prune.lock";

/// How many events may wait for the writer before a producer waits too.
const QUEUE_DEPTH: usize = 4096;
/// The longest a written event stays unsynced.
const SYNC_INTERVAL: Duration = Duration::from_secs(2);
/// How long the writer sleeps when nothing is waiting to be synced.
const IDLE_WAIT: Duration = Duration::from_secs(3600);
const LONGEST_ID: usize = 64;

/// `run-end`'s detail when every handle to a log went away without `finish`.
const UNCLOSED_NOTE: &str = "the run ended without closing its log";
/// `run-end`'s detail when [`recover`] finishes a log its process left behind.
const RECOVERED_NOTE: &str =
    "the process ended before the log was finished; recovered at the next start";
/// Added to [`RECOVERED_NOTE`] when the log's start was lost too, so the
/// header recovery gives it names only what its file name says.
const LOST_START_NOTE: &str =
    "; its first line was lost, so its run-start was rebuilt from the log's name";

/// The part a participant played in a run.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Role {
    /// The machine where the command was typed.
    Initiator,
    /// A daemon serving the files being sent.
    Source,
    /// A daemon receiving them.
    Destination,
}

impl Role {
    pub fn as_str(self) -> &'static str {
        match self {
            Role::Initiator => "initiator",
            Role::Source => "source",
            Role::Destination => "destination",
        }
    }

    fn parse(text: &str) -> Option<Self> {
        [Role::Initiator, Role::Source, Role::Destination]
            .into_iter()
            .find(|role| role.as_str() == text)
    }
}

impl std::str::FromStr for Role {
    type Err = eyre::Report;

    fn from_str(text: &str) -> eyre::Result<Self> {
        Self::parse(text).ok_or_else(|| {
            eyre::eyre!("unknown role {text:?}: expected initiator, source or destination")
        })
    }
}

/// Whether `id` can be a run or participant ID in a [`LogKey`].
pub fn valid_id(id: &str) -> bool {
    check_id("ID", id).is_ok()
}

/// Which log: run ID + participant + role + attempt.
///
/// The parts become the log's file name, so the two IDs are limited to 1–64
/// lowercase ASCII letters, digits, `-` and `_` — nothing that could leave
/// the log folder, and no two keys a case-insensitive file system would take
/// for one.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct LogKey {
    run_id: String,
    participant: String,
    role: Role,
    attempt: u32,
}

impl LogKey {
    pub fn new(
        run_id: impl Into<String>,
        participant: impl Into<String>,
        role: Role,
        attempt: u32,
    ) -> eyre::Result<Self> {
        let run_id = run_id.into();
        let participant = participant.into();
        check_id("run ID", &run_id)?;
        check_id("participant ID", &participant)?;
        Ok(Self {
            run_id,
            participant,
            role,
            attempt,
        })
    }

    pub fn run_id(&self) -> &str {
        &self.run_id
    }

    pub fn participant(&self) -> &str {
        &self.participant
    }

    pub fn role(&self) -> Role {
        self.role
    }

    pub fn attempt(&self) -> u32 {
        self.attempt
    }

    /// The name every file of this log starts with:
    /// `<run>.<participant>.<role>.<attempt>`.
    pub fn file_stem(&self) -> String {
        format!(
            "{}.{}.{}.{}",
            self.run_id,
            self.participant,
            self.role.as_str(),
            self.attempt
        )
    }

    /// The key a [`file_stem`](Self::file_stem) was made from; `None` for
    /// any other name.
    pub fn from_file_stem(stem: &str) -> Option<Self> {
        let mut parts = stem.split('.');
        let (run_id, participant, role, attempt) =
            (parts.next()?, parts.next()?, parts.next()?, parts.next()?);
        if parts.next().is_some() {
            return None;
        }
        let number: u32 = attempt.parse().ok()?;
        // `parse` also takes "+1" and "01", which `file_stem` never writes.
        if number.to_string() != attempt {
            return None;
        }
        Self::new(run_id, participant, Role::parse(role)?, number).ok()
    }
}

fn check_id(what: &str, id: &str) -> eyre::Result<()> {
    let allowed = |byte: u8| {
        byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'-' || byte == b'_'
    };
    if id.is_empty() || id.len() > LONGEST_ID || !id.bytes().all(allowed) {
        eyre::bail!(
            "a job log's {what} must be 1–{LONGEST_ID} lowercase letters, digits, '-' or '_' (got {id:?})"
        );
    }
    Ok(())
}

/// One line of a log.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Event {
    /// When the event was recorded, in milliseconds since the Unix epoch.
    pub ts_ms: u64,
    /// The event's place in its log, from 0; a complete log has no gaps.
    pub seq: u64,
    #[serde(flatten)]
    pub body: EventBody,
}

/// What an event says; `kind` on the wire.
///
/// # Raw names
///
/// A file event's `path` is the name as text. When the name is not valid
/// UTF-8 (Blit transfers such names, contract 7 `raw_relative_path`), that
/// text shows it with replacement characters and `raw` carries its exact
/// bytes, escaped by [`crate::raw_name::escape_raw`]: printable ASCII as
/// itself, every other byte — and `\` — as `\xNN`, so the bytes can be
/// recovered and two such names never read the same (review cr-jl1a-1).
/// `raw` is absent for every name that is valid UTF-8.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "kebab-case")]
pub enum EventBody {
    /// Always the first event: the log's format, version and identity, and
    /// the run it records. The writer records it.
    RunStart(Box<RunStart>),
    /// A phase (`scan`, `transfer`, `delete`, …) starting or ending. The
    /// log is synced at each one.
    Phase { name: String, state: PhaseState },
    /// A file that landed at the destination, and its size when the
    /// recorder knew it.
    FileCopied {
        path: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        bytes: Option<u64>,
        /// The exact bytes of a name that is not valid UTF-8; see
        /// [`EventBody`]'s *Raw names*.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        raw: Option<String>,
    },
    /// A file the source finished sending. Whether it landed is the
    /// destination's to say; a file that did not follows as
    /// `file-failed`.
    FileSent {
        path: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        raw: Option<String>,
    },
    /// A file removed — from the destination by a mirror, from the source by
    /// a move.
    FileDeleted {
        path: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        raw: Option<String>,
    },
    /// A file that did not land, and why.
    FileFailed {
        path: String,
        reason: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        raw: Option<String>,
    },
    /// Work outstanding but nothing moving for `idle_ms`.
    Stall { idle_ms: u64, detail: String },
    /// A line of what `-v` shows — scan time, rate, workers, the plan, how
    /// files were batched — recorded whether or not `-v` was given.
    Diagnostic { message: String },
    /// The run's closing counts.
    Summary(Summary),
    /// The last event of a log that was closed: how the run ended. The
    /// writer records it.
    RunEnd {
        outcome: Outcome,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        detail: Option<String>,
    },
    /// Logging stopped here, for `reason`; nothing after it was kept.
    LogIncomplete { reason: String },
    /// A kind this build does not know, from a newer writer.
    #[serde(other)]
    Unknown,
}

/// `run-start`'s fields.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct RunStart {
    pub format: String,
    pub version: u32,
    pub run_id: String,
    pub participant: String,
    pub role: Role,
    pub attempt: u32,
    /// The machine's host name at the time — for people; the participant
    /// ID is the identity (hostnames change).
    #[serde(default)]
    pub host: String,
    /// The blit build that wrote the log.
    #[serde(default)]
    pub build: String,
    pub run: RunInfo,
}

/// What was run, as this participant saw it.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct RunInfo {
    pub verb: String,
    pub source: String,
    pub destination: String,
    pub options: Vec<String>,
}

/// `summary`'s fields.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct Summary {
    pub files_copied: u64,
    pub files_deleted: u64,
    pub files_failed: u64,
    pub bytes_copied: u64,
    pub elapsed_ms: u64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum PhaseState {
    Start,
    End,
}

/// How a run ended.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Outcome {
    /// Everything landed.
    Ok,
    /// The run finished with failures, or stopped on an error.
    Failed,
    /// Cancelled on request.
    Cancelled,
    /// The run ended without saying how: its process stopped, or it never
    /// closed its log.
    Interrupted,
}

/// What became of a log, for the run's report.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct LogReport {
    /// The log on disk: the finished `.jsonl.gz`, or — when finishing it
    /// failed — the partial [`recover`] finishes later. `None` when no log
    /// file could be made.
    pub path: Option<PathBuf>,
    /// Every event the run recorded is in the log, through its `run-end`.
    pub complete: bool,
    /// The first thing that went wrong, in words.
    pub problem: Option<String>,
}

enum Msg {
    Event {
        ts_ms: u64,
        body: EventBody,
    },
    Finish {
        outcome: Outcome,
        detail: Option<String>,
    },
}

/// Records events into one log; clone it for each part of a run that
/// reports.
///
/// Recording never fails. It waits while the writer's queue is full, and
/// drops the event once the log has stopped or been finished.
#[derive(Clone)]
pub struct LogSender {
    tx: flume::Sender<Msg>,
}

impl LogSender {
    /// Record `body` from blocking code.
    pub fn record(&self, body: EventBody) {
        let _ = self.tx.send(Msg::Event {
            ts_ms: now_ms(),
            body,
        });
    }

    /// Record `body` from async code.
    pub async fn record_async(&self, body: EventBody) {
        let _ = self
            .tx
            .send_async(Msg::Event {
                ts_ms: now_ms(),
                body,
            })
            .await;
    }
}

/// One log being written (see the module docs). Dropping it without
/// [`finish`](Self::finish) closes the log as interrupted once every
/// [`LogSender`] is gone too.
pub struct LogWriter {
    sender: LogSender,
    thread: Option<JoinHandle<LogReport>>,
    /// Why the writer thread could not be started.
    spawn_error: Option<String>,
}

impl LogWriter {
    /// Start the log `key` in `dir`, recording `run-start` with `run`.
    /// Finishing it later prunes `dir` to the newest `keep` finished logs.
    ///
    /// Returns at once: the folder and files are made on the writer's
    /// thread, and a failure there leaves an incomplete log, never an error.
    pub fn start(dir: &Path, key: LogKey, run: RunInfo, keep: usize) -> Self {
        Self::start_tuned(dir, key, run, Tuning::new(keep))
    }

    fn start_tuned(dir: &Path, key: LogKey, run: RunInfo, tuning: Tuning) -> Self {
        let (tx, rx) = flume::bounded(tuning.queue_depth);
        let started_ms = now_ms();
        let dir = dir.to_path_buf();
        let spawned = std::thread::Builder::new()
            .name("blit-job-log".into())
            .spawn(move || write_log(dir, key, run, started_ms, rx, tuning));
        let (thread, spawn_error) = match spawned {
            Ok(thread) => (Some(thread), None),
            Err(error) => (
                None,
                Some(format!("could not start the log writer: {error}")),
            ),
        };
        Self {
            sender: LogSender { tx },
            thread,
            spawn_error,
        }
    }

    pub fn sender(&self) -> LogSender {
        self.sender.clone()
    }

    pub fn record(&self, body: EventBody) {
        self.sender.record(body);
    }

    pub async fn record_async(&self, body: EventBody) {
        self.sender.record_async(body).await;
    }

    /// Close the log: record `run-end`, compress the log into place and
    /// prune the folder. Blocks until done — compressing a large log takes
    /// a moment — so async code calls it through `spawn_blocking`.
    pub fn finish(mut self, outcome: Outcome, detail: Option<String>) -> LogReport {
        let Some(thread) = self.thread.take() else {
            return LogReport {
                path: None,
                complete: false,
                problem: self.spawn_error.take(),
            };
        };
        let _ = self.sender.tx.send(Msg::Finish { outcome, detail });
        thread.join().unwrap_or_else(|_| LogReport {
            path: None,
            complete: false,
            problem: Some("the log writer stopped unexpectedly".into()),
        })
    }
}

struct Tuning {
    keep: usize,
    queue_depth: usize,
    sync_interval: Duration,
    /// The append that would write this `seq` fails once.
    #[cfg(test)]
    fail_at_seq: Option<u64>,
    /// The writer waits for this before it starts.
    #[cfg(test)]
    start_gate: Option<flume::Receiver<()>>,
}

impl Tuning {
    fn new(keep: usize) -> Self {
        Self {
            keep,
            queue_depth: QUEUE_DEPTH,
            sync_interval: SYNC_INTERVAL,
            #[cfg(test)]
            fail_at_seq: None,
            #[cfg(test)]
            start_gate: None,
        }
    }
}

/// The writer thread.
fn write_log(
    dir: PathBuf,
    key: LogKey,
    run: RunInfo,
    started_ms: u64,
    rx: flume::Receiver<Msg>,
    tuning: Tuning,
) -> LogReport {
    #[cfg(test)]
    if let Some(gate) = &tuning.start_gate {
        let _ = gate.recv();
    }
    let mut writer = Writer::open(dir, key.file_stem(), &tuning);
    writer.append(
        started_ms,
        EventBody::RunStart(Box::new(RunStart {
            format: FORMAT.into(),
            version: VERSION,
            run_id: key.run_id,
            participant: key.participant,
            role: key.role,
            attempt: key.attempt,
            host: hostname::get()
                .map(|name| name.to_string_lossy().into_owned())
                .unwrap_or_default(),
            build: crate::transfer_session::session_build_id().into(),
            run,
        })),
    );
    let (outcome, detail) = loop {
        match rx.recv_timeout(writer.until_sync(tuning.sync_interval)) {
            Ok(Msg::Event { ts_ms, body }) => {
                let phase = matches!(body, EventBody::Phase { .. });
                writer.append(ts_ms, body);
                if phase || writer.until_sync(tuning.sync_interval).is_zero() {
                    writer.sync();
                }
            }
            Ok(Msg::Finish { outcome, detail }) => break (outcome, detail),
            Err(flume::RecvTimeoutError::Timeout) => writer.sync(),
            Err(flume::RecvTimeoutError::Disconnected) => {
                break (Outcome::Interrupted, Some(UNCLOSED_NOTE.into()))
            }
        }
    };
    // A handle still recording now fails at once instead of waiting on a
    // queue no one drains.
    drop(rx);
    writer.append(now_ms(), EventBody::RunEnd { outcome, detail });
    writer.sync();
    writer.finish(tuning.keep)
}

/// The writer thread's state for one log.
struct Writer {
    dir: PathBuf,
    stem: String,
    /// The sidecar lock, held from opening the log until it is finished.
    lock: Option<File>,
    /// The partial being appended to; `None` once logging has stopped.
    file: Option<PartialFile>,
    report: LogReport,
}

impl Writer {
    fn open(dir: PathBuf, stem: String, tuning: &Tuning) -> Self {
        let mut writer = Writer {
            dir,
            stem,
            lock: None,
            file: None,
            report: LogReport::default(),
        };
        match open_partial(&writer.dir, &writer.stem) {
            Ok((lock, file)) => {
                writer.report.path = Some(path_of(&writer.dir, &writer.stem, PARTIAL_SUFFIX));
                writer.lock = Some(lock);
                writer.file = Some(PartialFile {
                    out: BufWriter::with_capacity(64 * 1024, file),
                    line: Vec::new(),
                    next_seq: 0,
                    unsynced: false,
                    last_sync: Instant::now(),
                    #[cfg(test)]
                    fail_at_seq: tuning.fail_at_seq,
                });
            }
            Err(error) => {
                writer.report.problem = Some(format!(
                    "could not start the log in {}: {error}",
                    writer.dir.display()
                ))
            }
        }
        #[cfg(not(test))]
        let _ = tuning;
        writer
    }

    fn append(&mut self, ts_ms: u64, body: EventBody) {
        let Some(file) = self.file.as_mut() else {
            return;
        };
        if let Err(error) = file.append(ts_ms, body) {
            self.stop(error);
        }
    }

    fn sync(&mut self) {
        let Some(file) = self.file.as_mut() else {
            return;
        };
        if !file.unsynced {
            return;
        }
        if let Err(error) = file.sync() {
            self.stop(error);
        }
    }

    /// How long until what was written is due to be synced.
    fn until_sync(&self, interval: Duration) -> Duration {
        match &self.file {
            Some(file) if file.unsynced => interval.saturating_sub(file.last_sync.elapsed()),
            _ => IDLE_WAIT,
        }
    }

    /// Logging stops at the first write that fails; the transfer goes on.
    fn stop(&mut self, error: io::Error) {
        let reason = format!("could not write the log: {error}");
        if let Some(mut file) = self.file.take() {
            // Best effort. The failed write may have left part of a line, so
            // end it first and the marker stands as a line of its own.
            let _ = file.out.write_all(b"\n").and_then(|()| {
                file.append(
                    now_ms(),
                    EventBody::LogIncomplete {
                        reason: reason.clone(),
                    },
                )?;
                file.sync()
            });
        }
        self.report.problem.get_or_insert(reason);
    }

    fn finish(mut self, keep: usize) -> LogReport {
        self.report.complete = self.file.is_some();
        // Close the partial before compressing it; it was synced already.
        drop(self.file.take());
        let Some(lock) = self.lock.take() else {
            return self.report;
        };
        match compress_into_place(&self.dir, &self.stem, None) {
            Ok(finished) => self.report.path = Some(finished),
            Err(error) => {
                self.report.problem.get_or_insert_with(|| {
                    format!("could not finish the log (the next start finishes it): {error}")
                });
                return self.report;
            }
        }
        drop(lock);
        let _ = fs::remove_file(path_of(&self.dir, &self.stem, LOCK_SUFFIX));
        if let Err(error) = prune(&self.dir, keep) {
            log::warn!("job logs: pruning {} failed: {error}", self.dir.display());
        }
        self.report
    }
}

struct PartialFile {
    out: BufWriter<File>,
    /// Each event is serialized here first, so the file only ever receives
    /// whole lines.
    line: Vec<u8>,
    next_seq: u64,
    unsynced: bool,
    last_sync: Instant,
    #[cfg(test)]
    fail_at_seq: Option<u64>,
}

impl PartialFile {
    fn append(&mut self, ts_ms: u64, body: EventBody) -> io::Result<()> {
        self.line.clear();
        serde_json::to_writer(
            &mut self.line,
            &Event {
                ts_ms,
                seq: self.next_seq,
                body,
            },
        )?;
        self.line.push(b'\n');
        #[cfg(test)]
        if self.fail_at_seq == Some(self.next_seq) {
            // Half the line reaches the file, as a write cut short by a full
            // disk leaves it.
            self.fail_at_seq = None;
            self.out.write_all(&self.line[..self.line.len() / 2])?;
            return Err(io::Error::other("injected write failure"));
        }
        self.out.write_all(&self.line)?;
        self.next_seq += 1;
        self.unsynced = true;
        Ok(())
    }

    fn sync(&mut self) -> io::Result<()> {
        self.out.flush()?;
        self.out.get_ref().sync_data()?;
        self.unsynced = false;
        self.last_sync = Instant::now();
        Ok(())
    }
}

/// Take the key's lock and create its partial. Fails rather than touch a
/// log another writer owns or one this key already has.
fn open_partial(dir: &Path, stem: &str) -> io::Result<(File, File)> {
    fs::create_dir_all(dir)?;
    let lock_path = path_of(dir, stem, LOCK_SUFFIX);
    let lock = open_lock_file(&lock_path)?;
    match lock.try_lock() {
        Ok(()) => {}
        Err(TryLockError::WouldBlock) => {
            return Err(io::Error::other("another writer has this log open"))
        }
        Err(TryLockError::Error(error)) => return Err(error),
    }
    let created = if path_of(dir, stem, FINISHED_SUFFIX).exists() {
        Err(io::Error::new(
            io::ErrorKind::AlreadyExists,
            "this run already has a finished log",
        ))
    } else {
        // A partial already here is an earlier writer's record of this key;
        // never append to it or truncate it.
        OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(path_of(dir, stem, PARTIAL_SUFFIX))
    };
    match created {
        Ok(file) => Ok((lock, file)),
        Err(error) => {
            drop(lock);
            let _ = fs::remove_file(&lock_path);
            Err(error)
        }
    }
}

fn open_lock_file(path: &Path) -> io::Result<File> {
    OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(false)
        .open(path)
}

fn path_of(dir: &Path, stem: &str, suffix: &str) -> PathBuf {
    dir.join(format!("{stem}{suffix}"))
}

/// Compress the partial into `<stem>.jsonl.gz` — through a temp file and one
/// rename, so a reader or a crash sees either no finished log or a whole
/// one — then remove the partial. The finished log keeps the partial's
/// modified time, so pruning orders a recovered log by when its run wrote.
fn compress_into_place(dir: &Path, stem: &str, header: Option<&[u8]>) -> io::Result<PathBuf> {
    let partial = path_of(dir, stem, PARTIAL_SUFFIX);
    let temp = path_of(dir, stem, TEMP_SUFFIX);
    let finished = path_of(dir, stem, FINISHED_SUFFIX);
    if let Err(error) = write_compressed(&partial, &temp, &finished, header) {
        let _ = fs::remove_file(&temp);
        return Err(error);
    }
    sync_dir(dir);
    // The finished log is whole now; a partial left by a failure here is
    // removed by `recover`.
    if let Err(error) = fs::remove_file(&partial) {
        log::warn!(
            "job logs: could not remove {} after finishing it: {error}",
            partial.display()
        );
    }
    Ok(finished)
}

/// `header`, when given, goes first: a rebuilt `run-start` for a log whose
/// own was lost.
fn write_compressed(
    partial: &Path,
    temp: &Path,
    finished: &Path,
    header: Option<&[u8]>,
) -> io::Result<()> {
    let mut input = File::open(partial)?;
    let modified = input.metadata()?.modified()?;
    let mut encoder = GzEncoder::new(BufWriter::new(File::create(temp)?), Compression::fast());
    if let Some(header) = header {
        encoder.write_all(header)?;
    }
    io::copy(&mut input, &mut encoder)?;
    let output = encoder
        .finish()?
        .into_inner()
        .map_err(|error| error.into_error())?;
    output.set_modified(modified)?;
    output.sync_all()?;
    drop(output);
    fs::rename(temp, finished)
}

/// Make a rename in `dir` durable (POSIX); Windows has no directory handle
/// to sync and needs none.
fn sync_dir(dir: &Path) {
    #[cfg(unix)]
    if let Ok(handle) = File::open(dir) {
        let _ = handle.sync_all();
    }
    #[cfg(not(unix))]
    let _ = dir;
}

/// What [`recover`] did.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Recovery {
    /// Logs this call finished, closed as interrupted unless they already
    /// held their `run-end`.
    pub finished: Vec<PathBuf>,
    /// Partials left alone because a running writer still owns them.
    pub live: usize,
    /// Logs that could not be recovered, and why; they stay as they are for
    /// the next attempt.
    pub problems: Vec<String>,
}

/// Finish every log in `dir` that a stopped process left behind (run at
/// startup). A partial whose lock is free has no writer: it gets a `run-end`
/// marked interrupted — unless it already ends with one, the crash having
/// come while it was being compressed — and is compressed into place.
/// Partials a running writer holds are left alone.
pub fn recover(dir: &Path) -> Recovery {
    let mut recovery = Recovery::default();
    let entries = match fs::read_dir(dir) {
        Ok(entries) => entries,
        Err(error) if error.kind() == io::ErrorKind::NotFound => return recovery,
        Err(error) => {
            recovery
                .problems
                .push(format!("could not read {}: {error}", dir.display()));
            return recovery;
        }
    };
    let mut stems: Vec<String> = Vec::new();
    for entry in entries.flatten() {
        let name = entry.file_name();
        let Some(name) = name.to_str() else { continue };
        let Some(stem) = name
            .strip_suffix(PARTIAL_SUFFIX)
            .or_else(|| name.strip_suffix(TEMP_SUFFIX))
        else {
            continue;
        };
        if LogKey::from_file_stem(stem).is_some() && !stems.iter().any(|seen| seen == stem) {
            stems.push(stem.to_string());
        }
    }
    stems.sort();
    for stem in stems {
        match recover_one(dir, &stem) {
            Ok(Recovered::Finished(path)) => recovery.finished.push(path),
            Ok(Recovered::Live) => recovery.live += 1,
            Ok(Recovered::Tidied) => {}
            Err(error) => recovery.problems.push(format!("{stem}: {error}")),
        }
    }
    recovery
}

enum Recovered {
    Finished(PathBuf),
    Live,
    /// Only leftovers of a log already finished were removed.
    Tidied,
}

fn recover_one(dir: &Path, stem: &str) -> io::Result<Recovered> {
    let lock_path = path_of(dir, stem, LOCK_SUFFIX);
    let lock = open_lock_file(&lock_path)?;
    match lock.try_lock() {
        Ok(()) => {}
        Err(TryLockError::WouldBlock) => return Ok(Recovered::Live),
        Err(TryLockError::Error(error)) => return Err(error),
    }
    let partial = path_of(dir, stem, PARTIAL_SUFFIX);
    let recovered = if !partial.exists() {
        // A temp file whose partial is gone.
        remove_if_present(&path_of(dir, stem, TEMP_SUFFIX))?;
        Recovered::Tidied
    } else if path_of(dir, stem, FINISHED_SUFFIX).exists() {
        // Finished before the crash; only removing the partial was lost.
        fs::remove_file(&partial)?;
        remove_if_present(&path_of(dir, stem, TEMP_SUFFIX))?;
        Recovered::Tidied
    } else {
        let lost_start = close_interrupted(&partial)?;
        // Review cr-jlfix1-1: typed reading needs the header, so a log whose
        // start was lost (a crash before its first sync) gets one rebuilt
        // from its name, at the front of the finished file.
        let header = match (lost_start, LogKey::from_file_stem(stem)) {
            (true, Some(key)) => Some(rebuilt_header(&key, &partial)?),
            _ => None,
        };
        Recovered::Finished(compress_into_place(dir, stem, header.as_deref())?)
    };
    drop(lock);
    let _ = fs::remove_file(&lock_path);
    Ok(recovered)
}

fn remove_if_present(path: &Path) -> io::Result<()> {
    match fs::remove_file(path) {
        Err(error) if error.kind() != io::ErrorKind::NotFound => Err(error),
        _ => Ok(()),
    }
}

/// Append `run-end` (interrupted) to a partial that lacks one, keeping the
/// partial's modified time. Returns whether the partial's start — its
/// `run-start` header — was lost. Reads the partial line by line without
/// the header check, since recovering a damaged log is the point.
fn close_interrupted(partial: &Path) -> io::Result<bool> {
    let mut last = None;
    let mut lost_start = true;
    let mut first = true;
    let mut lines = decode(File::open(partial)?)?;
    let mut buf = Vec::new();
    loop {
        buf.clear();
        if lines.read_until(b'\n', &mut buf)? == 0 {
            break;
        }
        let text = buf.trim_ascii();
        if text.is_empty() {
            continue;
        }
        if first {
            first = false;
            lost_start = check_header(text).is_err();
        }
        if let Ok(event) = serde_json::from_slice::<Event>(text) {
            last = Some(event);
        }
    }
    if let Some(Event {
        body: EventBody::RunEnd { .. },
        ..
    }) = last
    {
        return Ok(lost_start);
    }
    let mut file = OpenOptions::new().read(true).append(true).open(partial)?;
    let modified = file.metadata()?.modified()?;
    let mut line = Vec::new();
    // A torn last line gets its end, so the new event is a line of its own.
    if file.metadata()?.len() > 0 {
        let mut byte = [0u8; 1];
        file.seek(SeekFrom::End(-1))?;
        file.read_exact(&mut byte)?;
        if byte[0] != b'\n' {
            line.push(b'\n');
        }
    }
    serde_json::to_writer(
        &mut line,
        &Event {
            ts_ms: now_ms(),
            // Review cr-jlfix2-2: with nothing surviving, the start was
            // lost too, and the header rebuilt for it holds 0.
            seq: last.map_or(1, |event| event.seq + 1),
            body: EventBody::RunEnd {
                outcome: Outcome::Interrupted,
                detail: Some(if lost_start {
                    format!("{RECOVERED_NOTE}{LOST_START_NOTE}")
                } else {
                    RECOVERED_NOTE.into()
                }),
            },
        },
    )?;
    line.push(b'\n');
    file.write_all(&line)?;
    file.sync_data()?;
    drop(file);
    filetime::set_file_mtime(partial, filetime::FileTime::from_system_time(modified))?;
    Ok(lost_start)
}

/// A `run-start` line for a log whose own was lost, from what its file name
/// says, stamped with the partial's modified time.
fn rebuilt_header(key: &LogKey, partial: &Path) -> io::Result<Vec<u8>> {
    let ts_ms = fs::metadata(partial)?
        .modified()?
        .duration_since(UNIX_EPOCH)
        .map_or(0, |since| {
            u64::try_from(since.as_millis()).unwrap_or(u64::MAX)
        });
    let mut line = serde_json::to_vec(&Event {
        ts_ms,
        seq: 0,
        body: EventBody::RunStart(Box::new(RunStart {
            format: FORMAT.into(),
            version: VERSION,
            run_id: key.run_id.clone(),
            participant: key.participant.clone(),
            role: key.role,
            attempt: key.attempt,
            host: String::new(),
            build: String::new(),
            run: RunInfo {
                verb: "unknown".into(),
                ..RunInfo::default()
            },
        })),
    })?;
    line.push(b'\n');
    Ok(line)
}

/// Remove finished logs in `dir` beyond the newest `keep`, newest by when
/// each run last wrote to its log; returns the logs removed.
///
/// Holds `dir`'s prune lock throughout, so two runs finishing at once do not
/// race, and touches only finished logs: never a partial (a running log, or
/// one not yet recovered) and never a file that is not a log. Waits for the
/// lock, so call it from blocking code.
pub fn prune(dir: &Path, keep: usize) -> io::Result<Vec<PathBuf>> {
    let lock = open_lock_file(&dir.join(PRUNE_LOCK))?;
    lock.lock()?;
    let mut finished = Vec::new();
    for entry in fs::read_dir(dir)?.flatten() {
        let name = entry.file_name();
        let Some(stem) = name.to_str().and_then(|n| n.strip_suffix(FINISHED_SUFFIX)) else {
            continue;
        };
        if LogKey::from_file_stem(stem).is_none() {
            continue;
        }
        let Ok(meta) = entry.metadata() else { continue };
        let Ok(modified) = meta.modified() else {
            continue;
        };
        if meta.is_file() {
            finished.push((modified, entry.path()));
        }
    }
    // Newest first; names break ties so the order is the same every time.
    finished.sort_by(|a, b| b.cmp(a));
    let mut removed = Vec::new();
    for (_, path) in finished.into_iter().skip(keep) {
        match fs::remove_file(&path) {
            Ok(()) => removed.push(path),
            Err(error) => log::warn!("job logs: could not prune {}: {error}", path.display()),
        }
    }
    Ok(removed)
}

/// The log for `key` in `dir`: its finished file, or the partial of a run
/// still going (or not yet recovered); `None` if there is neither.
pub fn find_log(dir: &Path, key: &LogKey) -> Option<FoundLog> {
    let stem = key.file_stem();
    let finished = path_of(dir, &stem, FINISHED_SUFFIX);
    let partial = path_of(dir, &stem, PARTIAL_SUFFIX);
    let found = |path: PathBuf, finished: bool| FoundLog {
        key: key.clone(),
        path,
        finished,
    };
    // Finished, partial, then finished again: a run that finishes between the
    // first two looks is still found.
    if finished.is_file() {
        return Some(found(finished, true));
    }
    if partial.is_file() {
        return Some(found(partial, false));
    }
    finished.is_file().then(|| found(finished, true))
}

/// A log found in a folder.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FoundLog {
    pub key: LogKey,
    pub path: PathBuf,
    /// `false` for a partial: the run is going, or its log waits for
    /// [`recover`].
    pub finished: bool,
}

/// Every log in `dir` for the run `run_id` — in `role` only, when given —
/// ordered by role, participant and attempt. Errors only when `run_id`
/// cannot name a log or `dir` cannot be read; a missing `dir` holds no logs.
pub fn logs_for_run(dir: &Path, run_id: &str, role: Option<Role>) -> eyre::Result<Vec<FoundLog>> {
    check_id("run ID", run_id)?;
    let entries = match fs::read_dir(dir) {
        Ok(entries) => entries,
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(error) => return Err(eyre::Report::new(error)),
    };
    let mut found: Vec<FoundLog> = Vec::new();
    for entry in entries.flatten() {
        let name = entry.file_name();
        let Some(name) = name.to_str() else { continue };
        let (stem, finished) = if let Some(stem) = name.strip_suffix(FINISHED_SUFFIX) {
            (stem, true)
        } else if let Some(stem) = name.strip_suffix(PARTIAL_SUFFIX) {
            (stem, false)
        } else {
            continue;
        };
        let Some(key) = LogKey::from_file_stem(stem) else {
            continue;
        };
        if key.run_id != run_id || role.is_some_and(|role| role != key.role) {
            continue;
        }
        // A run caught between finishing and removing its partial shows
        // once, as finished.
        if let Some(seen) = found.iter_mut().find(|seen| seen.key == key) {
            if finished {
                *seen = FoundLog {
                    key,
                    path: entry.path(),
                    finished,
                };
            }
            continue;
        }
        found.push(FoundLog {
            key,
            path: entry.path(),
            finished,
        });
    }
    found.sort_by(|a, b| {
        (a.key.role.as_str(), &a.key.participant, a.key.attempt).cmp(&(
            b.key.role.as_str(),
            &b.key.participant,
            b.key.attempt,
        ))
    });
    Ok(found)
}

/// One event as a line of text, for `blit jobs log` without `--json`
/// (R1's "json to txt converter"). Times are shown in the reader's local
/// time zone.
pub fn text_line(event: &Event) -> String {
    let when = i64::try_from(event.ts_ms)
        .ok()
        .and_then(chrono::DateTime::from_timestamp_millis)
        .map(|time| {
            time.with_timezone(&chrono::Local)
                .format("%Y-%m-%d %H:%M:%S%.3f")
                .to_string()
        })
        .unwrap_or_else(|| event.ts_ms.to_string());
    format!("{when}  {}", describe(&event.body))
}

/// A name as text: its escaped exact bytes when it is not valid UTF-8.
/// A file event's name as text: a name that is not valid UTF-8 shows its
/// escaped exact bytes after `raw:`, so it never reads like a UTF-8 name
/// whose characters happen to look like an escape (review cr-jlfix1-2).
pub fn shown_name(path: &str, raw: Option<&str>) -> String {
    match raw {
        Some(raw) => format!("raw:{raw}"),
        None => path.to_string(),
    }
}

fn shown(path: &str, raw: &Option<String>) -> String {
    shown_name(path, raw.as_deref())
}

fn describe(body: &EventBody) -> String {
    let seconds = |ms: u64| format!("{:.1}s", ms as f64 / 1000.0);
    match body {
        EventBody::RunStart(start) => {
            let host = if start.host.is_empty() {
                &start.participant
            } else {
                &start.host
            };
            let mut text = format!(
                "start    {} {} -> {} (job {}, {} on {host}, attempt {}, blit {})",
                start.run.verb,
                start.run.source,
                start.run.destination,
                start.run_id,
                start.role.as_str(),
                start.attempt,
                start.build,
            );
            if !start.run.options.is_empty() {
                text.push_str(&format!(" options: {}", start.run.options.join(" ")));
            }
            text
        }
        EventBody::Phase { name, state } => {
            let state = match state {
                PhaseState::Start => "started",
                PhaseState::End => "ended",
            };
            format!("phase    {name} {state}")
        }
        EventBody::FileCopied { path, bytes, raw } => {
            let path = shown(path, raw);
            match bytes {
                Some(bytes) => {
                    format!("copied   {path} ({})", crate::display::format_bytes(*bytes))
                }
                None => format!("copied   {path}"),
            }
        }
        EventBody::FileSent { path, raw } => format!("sent     {}", shown(path, raw)),
        EventBody::FileDeleted { path, raw } => format!("deleted  {}", shown(path, raw)),
        EventBody::FileFailed { path, reason, raw } => {
            format!("FAILED   {}: {reason}", shown(path, raw))
        }
        EventBody::Stall { idle_ms, detail } => {
            format!("stall    nothing moved for {}: {detail}", seconds(*idle_ms))
        }
        EventBody::Diagnostic { message } => format!("info     {message}"),
        EventBody::Summary(summary) => format!(
            "summary  {} copied ({}), {} deleted, {} failed, in {}",
            summary.files_copied,
            crate::display::format_bytes(summary.bytes_copied),
            summary.files_deleted,
            summary.files_failed,
            seconds(summary.elapsed_ms),
        ),
        EventBody::RunEnd { outcome, detail } => {
            let outcome = match outcome {
                Outcome::Ok => "ok",
                Outcome::Failed => "failed",
                Outcome::Cancelled => "cancelled",
                Outcome::Interrupted => "interrupted",
            };
            match detail {
                Some(detail) => format!("end      {outcome}: {detail}"),
                None => format!("end      {outcome}"),
            }
        }
        EventBody::LogIncomplete { reason } => {
            format!("LOG INCOMPLETE  {reason}; nothing after this was kept")
        }
        EventBody::Unknown => "unknown  an event from a newer blit".into(),
    }
}

const MACHINE_ID_FILE: &str = "machine-id";

/// This machine's participant ID, kept in `dir`: made on first use (128
/// random bits as lowercase hex) and read back after. Host names change; this
/// does not.
pub fn machine_id(dir: &Path) -> io::Result<String> {
    let path = dir.join(MACHINE_ID_FILE);
    if !path.exists() {
        use rand::{rngs::SysRng, TryRng};
        let mut bits = [0u8; 16];
        SysRng
            .try_fill_bytes(&mut bits)
            .map_err(|error| io::Error::other(format!("system RNG unavailable: {error}")))?;
        let id: String = bits.iter().map(|byte| format!("{byte:02x}")).collect();
        fs::create_dir_all(dir)?;
        match OpenOptions::new().write(true).create_new(true).open(&path) {
            Ok(mut file) => {
                file.write_all(format!("{id}\n").as_bytes())?;
                file.sync_all()?;
            }
            // Another process made it first; read theirs.
            Err(error) if error.kind() == io::ErrorKind::AlreadyExists => {}
            Err(error) => return Err(error),
        }
    }
    let text = fs::read_to_string(&path)?;
    let id = text.trim();
    if check_id("machine ID", id).is_err() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!("{} does not hold a machine ID", path.display()),
        ));
    }
    Ok(id.to_string())
}

/// One line of a log, as read back.
#[derive(Clone, Debug, PartialEq)]
pub enum LogLine {
    Event(Event),
    /// A line that is not an event. `torn` marks a last line with no end —
    /// the damage a crash mid-write leaves.
    Unreadable {
        line: u64,
        torn: bool,
    },
}

/// Read a log line by line: a finished `.jsonl.gz`, a partial (running or
/// left by a crash) or an exported plain copy. Compression is detected from
/// the content, not the name. Blank lines are skipped.
pub fn open_log(path: &Path) -> io::Result<LogLines> {
    Ok(LogLines::new(decode(File::open(path)?)?))
}

/// A log's JSON lines from its bytes as stored — gzip-compressed or plain,
/// told apart by the content — for a caller that wants the lines as written
/// (`blit jobs log --json`) rather than parsed.
pub fn decode(reader: impl Read + Send + 'static) -> io::Result<Box<dyn BufRead + Send>> {
    let mut reader = BufReader::new(reader);
    let gzip = reader.fill_buf()?.starts_with(&[0x1f, 0x8b]);
    Ok(if gzip {
        Box::new(BufReader::new(MultiGzDecoder::new(reader)))
    } else {
        Box::new(reader)
    })
}

/// The lines of a log; see [`open_log`].
///
/// The first line is the log's `run-start` header; a log that is not a blit
/// job log, or is a newer version than this build reads, yields an
/// `InvalidData` error instead of being misread (review cr-jl1a-2).
/// [`decode`] stays the way to see any log's lines as stored.
pub struct LogLines {
    reader: Box<dyn BufRead + Send>,
    buf: Vec<u8>,
    line: u64,
    done: bool,
    /// Whether the first line has been checked as the header.
    checked: bool,
}

impl LogLines {
    /// The lines of a log already [`decode`]d.
    pub fn new(reader: Box<dyn BufRead + Send>) -> Self {
        Self {
            reader,
            buf: Vec::new(),
            line: 0,
            done: false,
            checked: false,
        }
    }
}

/// Check a log's first line: it must be the `run-start` header, of this
/// format and a version this build reads (reviews cr-jl1a-2, cr-jlfix1-1).
/// Read as untyped JSON, so a newer header's fields are never forced into
/// this version's shape. Anything else — another kind, a damaged or missing
/// first line, a file that is not a job log — is refused rather than read
/// as the current version; [`decode`] shows such a file's lines as stored.
/// (Recovery gives a log whose start was lost a header of its own.)
fn check_header(line: &[u8]) -> io::Result<()> {
    use serde_json::Value;
    let invalid = |message: String| Err(io::Error::new(io::ErrorKind::InvalidData, message));
    let not_a_log = || {
        invalid(
            "not a blit job log, or its first line is damaged \
             (`blit jobs log --json` shows its lines as stored)"
                .into(),
        )
    };
    let Ok(Value::Object(header)) = serde_json::from_slice::<Value>(line) else {
        return not_a_log();
    };
    if header.get("kind").and_then(Value::as_str) != Some("run-start") {
        return not_a_log();
    }
    let format = header.get("format").and_then(Value::as_str).unwrap_or("");
    if format != FORMAT {
        return invalid(format!("not a blit job log (format {format:?})"));
    }
    // Review cr-jlfix2-1: only versions this build knows, each parsed whole
    // as that version's `run-start`. Version 1 is the only one so far; when
    // VERSION moves past it, each older version gets its own arm (and its
    // migration).
    match header.get("version").and_then(Value::as_u64) {
        Some(1) => match serde_json::from_slice::<Event>(line) {
            Ok(Event {
                body: EventBody::RunStart(_),
                ..
            }) => Ok(()),
            _ => invalid(
                "the job log's run-start is incomplete or damaged \
                 (`blit jobs log --json` shows its lines as stored)"
                    .into(),
            ),
        },
        Some(version) if version > u64::from(VERSION) => invalid(format!(
            "this job log is version {version}; this blit reads versions up to {VERSION} \
             (`blit jobs log --json` shows it as stored)"
        )),
        Some(version) => invalid(format!(
            "this job log names version {version}, which no blit writes"
        )),
        None => invalid("the job log's run-start names no version".into()),
    }
}

impl Iterator for LogLines {
    type Item = io::Result<LogLine>;

    fn next(&mut self) -> Option<Self::Item> {
        while !self.done {
            self.buf.clear();
            match self.reader.read_until(b'\n', &mut self.buf) {
                Ok(0) => self.done = true,
                Ok(_) => {
                    self.line += 1;
                    let ended = self.buf.ends_with(b"\n");
                    let text = self.buf.trim_ascii();
                    if text.is_empty() {
                        continue;
                    }
                    if !self.checked {
                        self.checked = true;
                        if let Err(error) = check_header(text) {
                            self.done = true;
                            return Some(Err(error));
                        }
                    }
                    return Some(Ok(match serde_json::from_slice::<Event>(text) {
                        Ok(event) => LogLine::Event(event),
                        Err(_) => LogLine::Unreadable {
                            line: self.line,
                            torn: !ended,
                        },
                    }));
                }
                Err(error) => {
                    self.done = true;
                    return Some(Err(error));
                }
            }
        }
        None
    }
}

fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |since| {
            u64::try_from(since.as_millis()).unwrap_or(u64::MAX)
        })
}

#[cfg(test)]
mod tests {
    use super::*;
    use filetime::FileTime;
    use tempfile::TempDir;

    fn key(run: &str) -> LogKey {
        LogKey::new(run, "m1", Role::Destination, 1).unwrap()
    }

    fn info() -> RunInfo {
        RunInfo {
            verb: "copy".into(),
            source: "/src".into(),
            destination: "host:/dst".into(),
            options: vec!["--retry".into()],
        }
    }

    fn copied(path: &str) -> EventBody {
        EventBody::FileCopied {
            path: path.into(),
            bytes: Some(3),
            raw: None,
        }
    }

    fn lines(path: &Path) -> Vec<LogLine> {
        open_log(path).unwrap().map(Result::unwrap).collect()
    }

    fn events(path: &Path) -> Vec<Event> {
        lines(path)
            .into_iter()
            .map(|line| match line {
                LogLine::Event(event) => event,
                other => panic!("unreadable line in {}: {other:?}", path.display()),
            })
            .collect()
    }

    fn bodies(events: &[Event]) -> Vec<EventBody> {
        events.iter().map(|event| event.body.clone()).collect()
    }

    fn names(dir: &Path) -> Vec<String> {
        let mut names: Vec<String> = fs::read_dir(dir)
            .unwrap()
            .map(|entry| entry.unwrap().file_name().to_string_lossy().into_owned())
            .collect();
        names.sort();
        names
    }

    fn wait_for(what: &str, mut ready: impl FnMut() -> bool) {
        let deadline = Instant::now() + Duration::from_secs(10);
        while !ready() {
            assert!(Instant::now() < deadline, "timed out waiting for {what}");
            std::thread::sleep(Duration::from_millis(10));
        }
    }

    fn write_lines(path: &Path, events: &[Event], tail: &str) {
        let mut text = String::new();
        for event in events {
            text.push_str(&serde_json::to_string(event).unwrap());
            text.push('\n');
        }
        text.push_str(tail);
        fs::write(path, text).unwrap();
    }

    fn start_event() -> Event {
        event(
            0,
            EventBody::RunStart(Box::new(RunStart {
                format: FORMAT.into(),
                version: VERSION,
                run_id: "r1".into(),
                participant: "m1".into(),
                role: Role::Destination,
                attempt: 1,
                host: String::new(),
                build: String::new(),
                run: info(),
            })),
        )
    }

    fn event(seq: u64, body: EventBody) -> Event {
        Event {
            ts_ms: 1_000 + seq,
            seq,
            body,
        }
    }

    fn set_mtime(path: &Path, secs: i64) {
        filetime::set_file_mtime(path, FileTime::from_unix_time(secs, 0)).unwrap();
    }

    #[test]
    fn a_finished_log_holds_every_event_in_order_compressed() {
        let dir = TempDir::new().unwrap();
        let writer = LogWriter::start(dir.path(), key("r1"), info(), DEFAULT_KEEP);
        let recorded = vec![
            EventBody::Phase {
                name: "transfer".into(),
                state: PhaseState::Start,
            },
            copied("a"),
            EventBody::FileFailed {
                path: "b".into(),
                reason: "denied".into(),
                raw: None,
            },
            EventBody::FileDeleted {
                path: "c".into(),
                raw: None,
            },
            EventBody::Stall {
                idle_ms: 5_000,
                detail: "no bytes moved".into(),
            },
            EventBody::Diagnostic {
                message: "workers: 4".into(),
            },
            EventBody::Summary(Summary {
                files_copied: 1,
                files_deleted: 1,
                files_failed: 1,
                bytes_copied: 3,
                elapsed_ms: 10,
            }),
            EventBody::Phase {
                name: "transfer".into(),
                state: PhaseState::End,
            },
        ];
        for body in &recorded {
            writer.record(body.clone());
        }
        let report = writer.finish(Outcome::Failed, Some("1 file failed".into()));

        let finished = dir.path().join("r1.m1.destination.1.jsonl.gz");
        assert_eq!(
            report,
            LogReport {
                path: Some(finished.clone()),
                complete: true,
                problem: None,
            }
        );
        assert_eq!(
            names(dir.path()),
            ["prune.lock", "r1.m1.destination.1.jsonl.gz"]
        );
        assert!(fs::read(&finished).unwrap().starts_with(&[0x1f, 0x8b]));

        let events = events(&finished);
        let seqs: Vec<u64> = events.iter().map(|event| event.seq).collect();
        assert_eq!(seqs, (0..events.len() as u64).collect::<Vec<_>>());
        let EventBody::RunStart(start) = &events[0].body else {
            panic!("first event is {:?}", events[0].body);
        };
        assert_eq!(
            (
                start.format.as_str(),
                start.version,
                start.run_id.as_str(),
                start.participant.as_str(),
                start.role,
                start.attempt,
                &start.run
            ),
            (
                "blit-job-log",
                VERSION,
                "r1",
                "m1",
                Role::Destination,
                1,
                &info()
            )
        );
        assert_eq!(bodies(&events[1..events.len() - 1]), recorded);
        assert_eq!(
            events.last().unwrap().body,
            EventBody::RunEnd {
                outcome: Outcome::Failed,
                detail: Some("1 file failed".into()),
            }
        );
    }

    #[test]
    fn events_use_the_documented_wire_names() {
        let line = serde_json::to_string(&event(
            2,
            EventBody::FileFailed {
                path: "a".into(),
                reason: "r".into(),
                raw: None,
            },
        ))
        .unwrap();
        assert_eq!(
            line,
            r#"{"ts_ms":1002,"seq":2,"kind":"file-failed","path":"a","reason":"r"}"#
        );
        let end = serde_json::to_string(&event(
            3,
            EventBody::RunEnd {
                outcome: Outcome::Interrupted,
                detail: None,
            },
        ))
        .unwrap();
        assert_eq!(
            end,
            r#"{"ts_ms":1003,"seq":3,"kind":"run-end","outcome":"interrupted"}"#
        );

        // A newer writer's kind, and a field this build does not know.
        let future: Event =
            serde_json::from_str(r#"{"ts_ms":1,"seq":0,"kind":"from-the-future","x":1}"#).unwrap();
        assert_eq!(future.body, EventBody::Unknown);
        let extra: Event = serde_json::from_str(
            r#"{"ts_ms":1,"seq":0,"kind":"file-deleted","path":"p","why":"x"}"#,
        )
        .unwrap();
        assert_eq!(
            extra.body,
            EventBody::FileDeleted {
                path: "p".into(),
                raw: None
            }
        );
    }

    #[test]
    fn a_newer_or_foreign_log_is_refused_not_misread() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("log.jsonl");
        let read_all = |header: &str| {
            fs::write(
                &path,
                format!(
                    "{header}\n{}\n",
                    serde_json::to_string(&event(1, copied("a"))).unwrap()
                ),
            )
            .unwrap();
            open_log(&path)
                .unwrap()
                .collect::<io::Result<Vec<LogLine>>>()
        };
        let newer = read_all(r#"{"ts_ms":1,"seq":0,"kind":"run-start","format":"blit-job-log","version":2,"shape":"new"}"#)
            .unwrap_err();
        assert_eq!(newer.kind(), io::ErrorKind::InvalidData);
        assert!(newer.to_string().contains("version 2"), "{newer}");
        let foreign =
            read_all(r#"{"ts_ms":1,"seq":0,"kind":"run-start","format":"other","version":1}"#)
                .unwrap_err();
        assert!(
            foreign.to_string().contains("not a blit job log"),
            "{foreign}"
        );
        // The current version reads; a log with no header — damaged, or
        // not a job log at all — is refused too (review cr-jlfix1-1).
        let current = serde_json::to_string(&start_event()).unwrap();
        assert_eq!(read_all(&current).unwrap().len(), 2);
        for headless in [
            r#"{"ts_ms":1,"seq":0,"ki"#,
            r#"{"ts_ms":1,"seq":0,"kind":"file-deleted","path":"p"}"#,
            "plain text",
        ] {
            let error = read_all(headless).unwrap_err();
            assert!(
                error.to_string().contains("not a blit job log"),
                "{headless}: {error}"
            );
        }
        // Review cr-jlfix2-1: a current-version header must be whole, and
        // version 0 never existed.
        let incomplete = read_all(
            r#"{"ts_ms":1,"seq":0,"kind":"run-start","format":"blit-job-log","version":1}"#,
        )
        .unwrap_err();
        assert!(
            incomplete.to_string().contains("incomplete or damaged"),
            "{incomplete}"
        );
        let zero = read_all(&current.replace(r#""version":1"#, r#""version":0"#)).unwrap_err();
        assert!(zero.to_string().contains("version 0"), "{zero}");
    }

    #[test]
    fn a_torn_last_line_is_tolerated() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("r1.m1.destination.1.partial.jsonl");
        write_lines(
            &path,
            &[start_event(), event(1, copied("a")), event(2, copied("b"))],
            r#"{"ts_ms":1003,"seq":3,"ki"#,
        );
        assert_eq!(
            lines(&path),
            [
                LogLine::Event(start_event()),
                LogLine::Event(event(1, copied("a"))),
                LogLine::Event(event(2, copied("b"))),
                LogLine::Unreadable {
                    line: 4,
                    torn: true
                },
            ]
        );
    }

    #[test]
    fn startup_finishes_an_orphaned_log_as_interrupted() {
        let dir = TempDir::new().unwrap();
        let partial = dir.path().join("r1.m1.destination.1.partial.jsonl");
        write_lines(
            &partial,
            &[start_event(), event(1, copied("a")), event(2, copied("b"))],
            r#"{"ts_ms":1003,"se"#,
        );
        set_mtime(&partial, 1_000_000);

        let recovery = recover(dir.path());

        let finished = dir.path().join("r1.m1.destination.1.jsonl.gz");
        assert_eq!(
            recovery,
            Recovery {
                finished: vec![finished.clone()],
                live: 0,
                problems: vec![],
            }
        );
        assert_eq!(names(dir.path()), ["r1.m1.destination.1.jsonl.gz"]);
        let read = lines(&finished);
        assert_eq!(
            read[..3],
            [
                LogLine::Event(start_event()),
                LogLine::Event(event(1, copied("a"))),
                LogLine::Event(event(2, copied("b"))),
            ]
        );
        // The torn fragment was ended, so it is now an unreadable line in
        // the middle rather than glued to the closing event.
        assert_eq!(
            read[3],
            LogLine::Unreadable {
                line: 4,
                torn: false
            }
        );
        let LogLine::Event(end) = &read[4] else {
            panic!("expected run-end, got {:?}", read[4]);
        };
        assert_eq!(end.seq, 3);
        assert_eq!(
            end.body,
            EventBody::RunEnd {
                outcome: Outcome::Interrupted,
                detail: Some(RECOVERED_NOTE.into()),
            }
        );
        assert_eq!(read.len(), 5);
        // Ordered for pruning by when the run wrote, not by the recovery.
        assert_eq!(
            FileTime::from_last_modification_time(&fs::metadata(&finished).unwrap()),
            FileTime::from_unix_time(1_000_000, 0)
        );
    }

    #[test]
    fn startup_keeps_a_run_end_already_written() {
        // The crash came while the log was being compressed.
        let dir = TempDir::new().unwrap();
        let partial = dir.path().join("r1.m1.destination.1.partial.jsonl");
        let written = [
            start_event(),
            event(1, copied("a")),
            event(
                2,
                EventBody::RunEnd {
                    outcome: Outcome::Ok,
                    detail: None,
                },
            ),
        ];
        write_lines(&partial, &written, "");
        fs::write(dir.path().join("r1.m1.destination.1.jsonl.gz.tmp"), b"half").unwrap();

        let recovery = recover(dir.path());

        let finished = dir.path().join("r1.m1.destination.1.jsonl.gz");
        assert_eq!(recovery.finished, std::slice::from_ref(&finished));
        assert_eq!(names(dir.path()), ["r1.m1.destination.1.jsonl.gz"]);
        assert_eq!(events(&finished), written);
    }

    /// Review cr-jlfix1-1: a log whose start was lost (a crash before its
    /// first sync) is still finished, with a `run-start` rebuilt from its
    /// name at the front, so it reads like any other — and says so.
    #[test]
    fn startup_rebuilds_a_lost_start() {
        let dir = TempDir::new().unwrap();
        let partial = dir.path().join("r1.m1.destination.1.partial.jsonl");
        // The header itself was torn; two events made it to disk.
        write_lines(
            &partial,
            &[event(1, copied("a")), event(2, copied("b"))],
            "",
        );
        set_mtime(&partial, 2_000_000);

        let recovery = recover(dir.path());

        let finished = dir.path().join("r1.m1.destination.1.jsonl.gz");
        assert_eq!(recovery.finished, std::slice::from_ref(&finished));
        let read = events(&finished);
        let EventBody::RunStart(start) = &read[0].body else {
            panic!("first event: {:?}", read[0]);
        };
        assert_eq!(
            (
                start.run_id.as_str(),
                start.participant.as_str(),
                start.role,
                start.attempt,
                start.run.verb.as_str()
            ),
            ("r1", "m1", Role::Destination, 1, "unknown")
        );
        assert_eq!(read[0].ts_ms, 2_000_000_000);
        assert_eq!(bodies(&read[1..3]), [copied("a"), copied("b")]);
        assert_eq!(
            read[3].body,
            EventBody::RunEnd {
                outcome: Outcome::Interrupted,
                detail: Some(format!("{RECOVERED_NOTE}{LOST_START_NOTE}")),
            }
        );
        assert_eq!(read.len(), 4);
    }

    /// Review cr-jlfix2-2: an empty partial (killed before its first line
    /// reached disk) recovers as a rebuilt header and a run-end, numbered 0
    /// and 1.
    #[test]
    fn startup_numbers_an_empty_logs_lines_in_order() {
        let dir = TempDir::new().unwrap();
        fs::write(dir.path().join("r1.m1.destination.1.partial.jsonl"), b"").unwrap();

        recover(dir.path());

        let read = events(&dir.path().join("r1.m1.destination.1.jsonl.gz"));
        assert!(matches!(read[0].body, EventBody::RunStart(_)));
        assert!(matches!(read[1].body, EventBody::RunEnd { .. }));
        assert_eq!(
            read.iter().map(|event| event.seq).collect::<Vec<_>>(),
            [0, 1]
        );
    }

    #[test]
    fn startup_leaves_a_finished_log_whose_partial_was_not_removed() {
        let dir = TempDir::new().unwrap();
        let writer = LogWriter::start(dir.path(), key("r1"), info(), DEFAULT_KEEP);
        writer.record(copied("a"));
        let finished = writer.finish(Outcome::Ok, None).path.unwrap();
        let before = fs::read(&finished).unwrap();
        // The crash came after the rename, before the partial was removed.
        fs::write(
            dir.path().join("r1.m1.destination.1.partial.jsonl"),
            b"stale\n",
        )
        .unwrap();

        let recovery = recover(dir.path());

        assert_eq!(recovery, Recovery::default());
        assert_eq!(
            names(dir.path()),
            ["prune.lock", "r1.m1.destination.1.jsonl.gz"]
        );
        assert_eq!(fs::read(&finished).unwrap(), before);
    }

    #[test]
    fn startup_leaves_a_running_log_alone() {
        let dir = TempDir::new().unwrap();
        let writer = LogWriter::start(dir.path(), key("r1"), info(), DEFAULT_KEEP);
        let partial = dir.path().join("r1.m1.destination.1.partial.jsonl");
        wait_for("the partial", || partial.exists());

        let recovery = recover(dir.path());

        assert_eq!(
            recovery,
            Recovery {
                finished: vec![],
                live: 1,
                problems: vec![],
            }
        );
        assert!(partial.exists());
        writer.record(copied("a"));
        let report = writer.finish(Outcome::Ok, None);
        assert!(report.complete, "{report:?}");
        let events = events(&report.path.unwrap());
        assert_eq!(bodies(&events[1..2]), [copied("a")]);
        assert_eq!(
            events.last().unwrap().body,
            EventBody::RunEnd {
                outcome: Outcome::Ok,
                detail: None,
            }
        );
    }

    #[test]
    fn pruning_keeps_the_newest_finished_logs_and_nothing_else_is_touched() {
        let dir = TempDir::new().unwrap();
        for (run, secs) in [
            ("r0", 100),
            ("r1", 200),
            ("r2", 300),
            ("r3", 400),
            ("r4", 500),
        ] {
            let path = dir.path().join(format!("{run}.m1.source.1.jsonl.gz"));
            fs::write(&path, b"x").unwrap();
            set_mtime(&path, secs);
        }
        for other in [
            "old.m1.source.1.partial.jsonl",
            "old.m1.source.1.jsonl.gz.tmp",
            "old.m1.source.1.lock",
            "not-a-log.jsonl.gz",
            "notes.txt",
        ] {
            let path = dir.path().join(other);
            fs::write(&path, b"x").unwrap();
            set_mtime(&path, 1);
        }

        let mut removed = prune(dir.path(), 3).unwrap();
        removed.sort();

        assert_eq!(
            removed,
            [
                dir.path().join("r0.m1.source.1.jsonl.gz"),
                dir.path().join("r1.m1.source.1.jsonl.gz"),
            ]
        );
        assert_eq!(
            names(dir.path()),
            [
                "not-a-log.jsonl.gz",
                "notes.txt",
                "old.m1.source.1.jsonl.gz.tmp",
                "old.m1.source.1.lock",
                "old.m1.source.1.partial.jsonl",
                "prune.lock",
                "r2.m1.source.1.jsonl.gz",
                "r3.m1.source.1.jsonl.gz",
                "r4.m1.source.1.jsonl.gz",
            ]
        );
    }

    #[test]
    fn finishing_a_run_prunes_the_folder() {
        let dir = TempDir::new().unwrap();
        for (run, secs) in [("old0", 100), ("old1", 200), ("old2", 300)] {
            let path = dir.path().join(format!("{run}.m1.source.1.jsonl.gz"));
            fs::write(&path, b"x").unwrap();
            set_mtime(&path, secs);
        }

        let report = LogWriter::start(dir.path(), key("new"), info(), 2).finish(Outcome::Ok, None);

        assert!(report.complete, "{report:?}");
        assert_eq!(
            names(dir.path()),
            [
                "new.m1.destination.1.jsonl.gz",
                "old2.m1.source.1.jsonl.gz",
                "prune.lock",
            ]
        );
    }

    #[test]
    fn an_unusable_log_folder_does_not_hold_up_the_run() {
        let dir = TempDir::new().unwrap();
        let blocker = dir.path().join("a-file");
        fs::write(&blocker, b"x").unwrap();
        let (done_tx, done_rx) = std::sync::mpsc::channel();
        std::thread::spawn(move || {
            let writer = LogWriter::start(&blocker.join("logs"), key("r1"), info(), DEFAULT_KEEP);
            // More events than the queue holds: a dead log must not make the
            // run wait.
            for n in 0..(QUEUE_DEPTH * 3) {
                writer.record(copied(&n.to_string()));
            }
            done_tx.send(writer.finish(Outcome::Ok, None)).unwrap();
        });

        let report = done_rx
            .recv_timeout(Duration::from_secs(30))
            .expect("recording into an unusable folder must not block");

        assert_eq!(report.path, None);
        assert!(!report.complete);
        let problem = report.problem.unwrap();
        assert!(
            problem.starts_with("could not start the log in "),
            "{problem}"
        );
    }

    #[test]
    fn a_failed_write_stops_the_log_and_marks_it() {
        let dir = TempDir::new().unwrap();
        let tuning = Tuning {
            fail_at_seq: Some(3),
            ..Tuning::new(DEFAULT_KEEP)
        };
        let writer = LogWriter::start_tuned(dir.path(), key("r1"), info(), tuning);
        for n in 0..10 {
            writer.record(copied(&n.to_string()));
        }
        let report = writer.finish(Outcome::Ok, None);

        assert!(!report.complete);
        assert_eq!(
            report.problem.as_deref(),
            Some("could not write the log: injected write failure")
        );
        let read = lines(&report.path.unwrap());
        let event_at = |index: usize| match &read[index] {
            LogLine::Event(event) => event.clone(),
            other => panic!("line {index} is {other:?}"),
        };
        assert_eq!(read.len(), 5, "{read:?}");
        assert_eq!(
            [event_at(1).body, event_at(2).body],
            [copied("0"), copied("1")]
        );
        // The half-written line was ended, so it is one damaged line rather
        // than glued to the marker.
        assert_eq!(
            read[3],
            LogLine::Unreadable {
                line: 4,
                torn: false
            }
        );
        // The marker takes the failed write's place, and nothing follows it —
        // not even run-end, which would make the log look whole.
        let marker = event_at(4);
        assert_eq!(marker.seq, 3);
        assert_eq!(
            marker.body,
            EventBody::LogIncomplete {
                reason: "could not write the log: injected write failure".into(),
            }
        );
    }

    #[test]
    fn a_leftover_partial_is_never_appended_to_or_replaced() {
        let dir = TempDir::new().unwrap();
        let partial = dir.path().join("r1.m1.destination.1.partial.jsonl");
        fs::write(&partial, b"an earlier writer's record\n").unwrap();

        let report =
            LogWriter::start(dir.path(), key("r1"), info(), DEFAULT_KEEP).finish(Outcome::Ok, None);

        assert!(!report.complete);
        assert_eq!(report.path, None);
        assert_eq!(fs::read(&partial).unwrap(), b"an earlier writer's record\n");
    }

    #[test]
    fn a_full_queue_makes_the_producer_wait_instead_of_dropping() {
        let dir = TempDir::new().unwrap();
        let (gate_tx, gate_rx) = flume::bounded(1);
        let tuning = Tuning {
            queue_depth: 4,
            start_gate: Some(gate_rx),
            ..Tuning::new(DEFAULT_KEEP)
        };
        let writer = LogWriter::start_tuned(dir.path(), key("r1"), info(), tuning);
        let sender = writer.sender();
        let (done_tx, done_rx) = std::sync::mpsc::channel();
        std::thread::spawn(move || {
            for n in 0..20 {
                sender.record(copied(&n.to_string()));
            }
            done_tx.send(()).unwrap();
        });

        assert!(
            done_rx.recv_timeout(Duration::from_millis(300)).is_err(),
            "the producer finished while the writer was held: events were dropped"
        );
        gate_tx.send(()).unwrap();
        done_rx.recv_timeout(Duration::from_secs(10)).unwrap();
        let report = writer.finish(Outcome::Ok, None);

        let events = events(&report.path.unwrap());
        let recorded: Vec<EventBody> = (0..20).map(|n| copied(&n.to_string())).collect();
        assert_eq!(bodies(&events[1..21]), recorded);
        assert_eq!(events.len(), 22);
    }

    #[test]
    fn a_phase_change_reaches_the_partial_at_once() {
        let dir = TempDir::new().unwrap();
        let tuning = Tuning {
            sync_interval: IDLE_WAIT,
            ..Tuning::new(DEFAULT_KEEP)
        };
        let writer = LogWriter::start_tuned(dir.path(), key("r1"), info(), tuning);
        writer.record(EventBody::Phase {
            name: "scan".into(),
            state: PhaseState::Start,
        });
        let partial = dir.path().join("r1.m1.destination.1.partial.jsonl");
        wait_for("the phase line", || {
            fs::read_to_string(&partial).is_ok_and(|text| text.contains(r#""kind":"phase""#))
        });
        assert!(writer.finish(Outcome::Ok, None).complete);
    }

    #[test]
    fn a_quiet_event_reaches_the_partial_within_the_sync_interval() {
        let dir = TempDir::new().unwrap();
        let tuning = Tuning {
            sync_interval: Duration::from_millis(50),
            ..Tuning::new(DEFAULT_KEEP)
        };
        let writer = LogWriter::start_tuned(dir.path(), key("r1"), info(), tuning);
        writer.record(copied("quiet"));
        let partial = dir.path().join("r1.m1.destination.1.partial.jsonl");
        wait_for("the copied line", || {
            fs::read_to_string(&partial).is_ok_and(|text| text.contains("quiet"))
        });
        assert!(writer.finish(Outcome::Ok, None).complete);
    }

    #[test]
    fn a_log_dropped_without_finishing_closes_as_interrupted() {
        let dir = TempDir::new().unwrap();
        let writer = LogWriter::start(dir.path(), key("r1"), info(), DEFAULT_KEEP);
        writer.record(copied("a"));
        drop(writer);

        let finished = dir.path().join("r1.m1.destination.1.jsonl.gz");
        wait_for("the finished log", || finished.exists());
        let events = events(&finished);
        assert_eq!(bodies(&events[1..2]), [copied("a")]);
        assert_eq!(
            events.last().unwrap().body,
            EventBody::RunEnd {
                outcome: Outcome::Interrupted,
                detail: Some(UNCLOSED_NOTE.into()),
            }
        );
    }

    #[tokio::test]
    async fn async_producers_record_in_order() {
        let dir = TempDir::new().unwrap();
        let tuning = Tuning {
            queue_depth: 8,
            ..Tuning::new(DEFAULT_KEEP)
        };
        let writer = LogWriter::start_tuned(dir.path(), key("r1"), info(), tuning);
        for n in 0..100 {
            writer.record_async(copied(&n.to_string())).await;
        }
        let report = tokio::task::spawn_blocking(move || writer.finish(Outcome::Ok, None))
            .await
            .unwrap();

        let events = events(&report.path.unwrap());
        let recorded: Vec<EventBody> = (0..100).map(|n| copied(&n.to_string())).collect();
        assert_eq!(bodies(&events[1..101]), recorded);
    }

    #[test]
    fn a_second_writer_for_the_same_log_does_not_touch_the_first() {
        let dir = TempDir::new().unwrap();
        let first = LogWriter::start(dir.path(), key("r1"), info(), DEFAULT_KEEP);
        let partial = dir.path().join("r1.m1.destination.1.partial.jsonl");
        wait_for("the first partial", || partial.exists());
        first.record(copied("first"));

        let second = LogWriter::start(dir.path(), key("r1"), info(), DEFAULT_KEEP);
        second.record(copied("second"));
        let second = second.finish(Outcome::Ok, None);
        assert!(!second.complete);
        assert_eq!(second.path, None);
        assert!(
            second
                .problem
                .as_deref()
                .unwrap()
                .contains("another writer has this log open"),
            "{second:?}"
        );

        let first = first.finish(Outcome::Ok, None);
        assert!(first.complete, "{first:?}");
        let events = events(&first.path.unwrap());
        assert_eq!(bodies(&events[1..events.len() - 1]), [copied("first")]);
    }

    #[test]
    fn a_finished_log_is_never_overwritten() {
        let dir = TempDir::new().unwrap();
        let first = LogWriter::start(dir.path(), key("r1"), info(), DEFAULT_KEEP)
            .finish(Outcome::Ok, None)
            .path
            .unwrap();
        let before = fs::read(&first).unwrap();

        let again =
            LogWriter::start(dir.path(), key("r1"), info(), DEFAULT_KEEP).finish(Outcome::Ok, None);

        assert!(!again.complete);
        assert_eq!(fs::read(&first).unwrap(), before);
        assert_eq!(
            names(dir.path()),
            ["prune.lock", "r1.m1.destination.1.jsonl.gz"]
        );
    }

    #[test]
    fn log_keys_name_files_safely() {
        for bad in ["", "a/b", "..", "a.b", "A", "a b", &"x".repeat(65)] {
            assert!(LogKey::new(bad, "m1", Role::Source, 1).is_err(), "{bad:?}");
            assert!(LogKey::new("r1", bad, Role::Source, 1).is_err(), "{bad:?}");
        }
        let key = LogKey::new("0f3a-9c", "m_1", Role::Initiator, 12).unwrap();
        assert_eq!(key.file_stem(), "0f3a-9c.m_1.initiator.12");
        assert_eq!(LogKey::from_file_stem(&key.file_stem()), Some(key));
        for bad in [
            "r1.m1.source",
            "r1.m1.source.1.x",
            "r1.m1.source.01",
            "r1.m1.source.+1",
            "r1.m1.owner.1",
            "R1.m1.source.1",
        ] {
            assert_eq!(LogKey::from_file_stem(bad), None, "{bad:?}");
        }
    }

    #[test]
    fn a_runs_logs_are_found_by_role_and_shown_once() {
        let dir = TempDir::new().unwrap();
        for name in [
            "r1.m1.source.1.jsonl.gz",
            "r1.m1.destination.1.partial.jsonl",
            // Finished, but its partial was not yet removed.
            "r1.m1.destination.2.jsonl.gz",
            "r1.m1.destination.2.partial.jsonl",
            "r2.m1.source.1.jsonl.gz",
            "r1.m1.source.1.lock",
            "notes.txt",
        ] {
            fs::write(dir.path().join(name), b"").unwrap();
        }
        let found = |role| {
            logs_for_run(dir.path(), "r1", role)
                .unwrap()
                .into_iter()
                .map(|log| {
                    (
                        log.path.file_name().unwrap().to_string_lossy().into_owned(),
                        log.finished,
                    )
                })
                .collect::<Vec<_>>()
        };
        let owned = |list: &[(&str, bool)]| {
            list.iter()
                .map(|(name, finished)| (name.to_string(), *finished))
                .collect::<Vec<_>>()
        };

        assert_eq!(
            found(None),
            owned(&[
                ("r1.m1.destination.1.partial.jsonl", false),
                ("r1.m1.destination.2.jsonl.gz", true),
                ("r1.m1.source.1.jsonl.gz", true),
            ])
        );
        assert_eq!(
            found(Some(Role::Source)),
            owned(&[("r1.m1.source.1.jsonl.gz", true)])
        );
        assert_eq!(found(Some(Role::Initiator)), owned(&[]));
        assert!(logs_for_run(dir.path(), "../r1", None).is_err());
        assert_eq!(
            logs_for_run(&dir.path().join("missing"), "r1", None).unwrap(),
            []
        );
    }

    #[test]
    fn a_log_reads_the_same_compressed_or_plain() {
        let dir = TempDir::new().unwrap();
        let writer = LogWriter::start(dir.path(), key("r1"), info(), DEFAULT_KEEP);
        writer.record(copied("a"));
        let finished = writer.finish(Outcome::Ok, None).path.unwrap();
        let mut plain = String::new();
        decode(File::open(&finished).unwrap())
            .unwrap()
            .read_to_string(&mut plain)
            .unwrap();
        let exported = dir.path().join("exported.jsonl");
        fs::write(&exported, &plain).unwrap();

        assert!(plain
            .lines()
            .nth(1)
            .unwrap()
            .contains(r#""kind":"file-copied""#));
        assert_eq!(events(&exported), events(&finished));
    }

    #[test]
    fn events_read_as_text() {
        let text = |body| {
            let line = text_line(&event(0, body));
            // The time zone is the reader's; only the part after the time
            // is fixed.
            line.split_once("  ").unwrap().1.to_string()
        };
        assert_eq!(text(copied("a/b")), "copied   a/b (3 B)");
        assert_eq!(
            text(EventBody::FileCopied {
                path: "a/b".into(),
                bytes: None,
                raw: None,
            }),
            "copied   a/b"
        );
        assert_eq!(
            text(EventBody::FileSent {
                path: "s".into(),
                raw: None
            }),
            "sent     s"
        );
        // A name that is not valid UTF-8 shows its exact bytes, escaped.
        assert_eq!(
            text(EventBody::FileFailed {
                path: "caf\u{fffd}".into(),
                reason: "denied".into(),
                raw: Some("caf\\xe9".into()),
            }),
            "FAILED   raw:caf\\xe9: denied"
        );
        assert_eq!(
            text(EventBody::FileFailed {
                path: "c".into(),
                reason: "denied".into(),
                raw: None,
            }),
            "FAILED   c: denied"
        );
        assert_eq!(
            text(EventBody::Summary(Summary {
                files_copied: 2,
                files_deleted: 1,
                files_failed: 0,
                bytes_copied: 2048,
                elapsed_ms: 1500,
            })),
            "summary  2 copied (2.00 KiB), 1 deleted, 0 failed, in 1.5s"
        );
        assert_eq!(
            text(EventBody::RunEnd {
                outcome: Outcome::Interrupted,
                detail: Some("why".into())
            }),
            "end      interrupted: why"
        );
        assert!(text_line(&event(0, EventBody::Unknown)).contains("newer blit"));
    }

    #[test]
    fn the_machine_id_is_made_once_and_kept() {
        let dir = TempDir::new().unwrap();
        let first = machine_id(&dir.path().join("cfg")).unwrap();
        assert_eq!(first.len(), 32);
        assert!(LogKey::new("r1", first.clone(), Role::Source, 1).is_ok());
        assert_eq!(machine_id(&dir.path().join("cfg")).unwrap(), first);
        assert_ne!(machine_id(&dir.path().join("other")).unwrap(), first);

        fs::write(dir.path().join("cfg").join("machine-id"), "../evil\n").unwrap();
        let error = machine_id(&dir.path().join("cfg")).unwrap_err();
        assert_eq!(error.kind(), io::ErrorKind::InvalidData);
    }

    #[test]
    fn find_log_prefers_the_finished_log() {
        let dir = TempDir::new().unwrap();
        let key = key("r1");
        let found = |dir: &Path| find_log(dir, &key).map(|log| (log.path, log.finished));
        assert_eq!(found(dir.path()), None);
        let partial = dir.path().join("r1.m1.destination.1.partial.jsonl");
        fs::write(&partial, b"").unwrap();
        assert_eq!(found(dir.path()), Some((partial, false)));
        let finished = dir.path().join("r1.m1.destination.1.jsonl.gz");
        fs::write(&finished, b"").unwrap();
        assert_eq!(found(dir.path()), Some((finished, true)));
    }
}
