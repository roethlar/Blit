//! Jobs on the machine where a command is typed (JOB_LOGS jl-3, plan
//! "Three artifacts"): each run's **JobSpec** — what to run again, written
//! once at submission — and its **RunRecord** — how that run went, updated
//! as it goes. Both are versioned JSON documents, read only when this build
//! knows their format and version (each older version gets its own
//! migration when the version moves on). A run's logs are `job_log`'s;
//! these are the index of a machine's jobs and what `blit jobs` acts on.
//!
//! The store is one folder, `<per-user folder>/jobs/runs/`, holding per run
//! `<run>.spec.json`, `<run>.run.json` and, while the run's command lives,
//! `<run>.lock`, which it holds: a `running` record whose lock can be taken
//! belongs to a command that died, and reads as `interrupted`. Records are
//! replaced whole (temp file, then rename), so a reader never sees half of
//! one. Pruning keeps the newest records, never a running or waiting one.

use crate::job_log::Outcome;
use serde::{Deserialize, Serialize};
use std::fs::{self, File, OpenOptions};
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

pub const SPEC_FORMAT: &str = "blit-job-spec";
pub const RECORD_FORMAT: &str = "blit-run-record";
pub const FILE_FORMAT: &str = "blit-job";
/// The version this build writes, and the newest it reads.
pub const VERSION: u32 = 1;

const SPEC_SUFFIX: &str = ".spec.json";
const RECORD_SUFFIX: &str = ".run.json";
const LOCK_SUFFIX: &str = ".lock";
const TEMP_SUFFIX: &str = ".tmp";
/// The folder-wide lock pruning holds. Not a run's name: a run ID has no
/// dot.
const PRUNE_LOCK: &str = "prune.lock";

/// What to run again: everything that decides what a transfer does, and
/// nothing about how it is shown. Local paths are absolute, made from the
/// folder the command was typed in, so the job means the same thing from
/// anywhere; a `--files-from` list is kept by its contents.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct JobSpec {
    pub format: String,
    pub version: u32,
    /// The machine the job belongs to; only it runs the job again (R4).
    pub machine: String,
    /// That machine's host name when the job was made — for people; the
    /// machine ID is the identity (host names change).
    #[serde(default)]
    pub host: String,
    pub created_ms: u64,
    /// `copy`, `mirror` or `move`.
    pub verb: String,
    /// The folder the command was typed in.
    pub cwd: String,
    pub source: SpecEndpoint,
    pub destination: SpecEndpoint,
    pub options: SpecOptions,
    /// The `--files-from` list's lines, as read when the job was made.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub files_from: Option<Vec<String>>,
}

/// One end of a job's transfer.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "kebab-case", deny_unknown_fields)]
pub enum SpecEndpoint {
    /// An absolute local path, its trailing separator kept (it means "the
    /// contents of" a source, "into" a destination).
    Local { path: String },
    /// A daemon's path, as typed (`host:/module/path`), with its parts for
    /// readers.
    Remote {
        locator: String,
        host: String,
        port: u16,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        module: Option<String>,
        path: String,
    },
}

impl SpecEndpoint {
    /// The endpoint as a command line names it.
    pub fn as_arg(&self) -> &str {
        match self {
            SpecEndpoint::Local { path } => path,
            SpecEndpoint::Remote { locator, .. } => locator,
        }
    }
}

/// Every option that changes what a transfer does. A spec naming an option
/// this build does not know is refused rather than run without it.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct SpecOptions {
    pub dry_run: bool,
    pub checksum: bool,
    pub size_only: bool,
    pub ignore_times: bool,
    pub ignore_existing: bool,
    pub force: bool,
    /// `subset` or `all`.
    pub delete_scope: String,
    pub resume: bool,
    pub drop_windows_metadata: bool,
    pub retries: u32,
    pub retry_wait: u64,
    pub retry: u32,
    pub wait: u64,
    pub exclude: Vec<String>,
    pub include: Vec<String>,
    pub min_size: Option<String>,
    pub max_size: Option<String>,
    pub min_age: Option<String>,
    pub max_age: Option<String>,
    pub force_grpc: bool,
    pub detach: bool,
    pub null: bool,
    /// The person agreed, when making the job, to what it deletes.
    pub yes: bool,
}

/// One run of a job.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RunRecord {
    pub format: String,
    pub version: u32,
    pub run_id: String,
    pub machine: String,
    /// 1 for a new run; a retry of a run is its next attempt (jl-4).
    pub attempt: u32,
    /// The run a retry finishes (jl-4).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub parent: Option<String>,
    /// The saved job this run ran, when it ran one.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub saved_job: Option<String>,
    /// What the run did, in words, for a listing (the spec is the job).
    pub verb: String,
    pub source: String,
    pub destination: String,
    pub started_ms: u64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ended_ms: Option<u64>,
    pub state: RunState,
    /// How the run ended, once it has.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub outcome: Option<Outcome>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub detail: Option<String>,
    #[serde(default)]
    pub files_copied: u64,
    #[serde(default)]
    pub files_deleted: u64,
    #[serde(default)]
    pub files_failed: u64,
    #[serde(default)]
    pub bytes_copied: u64,
    /// Each file that failed, exactly as named, with its reason.
    #[serde(default)]
    pub failures: Vec<Failure>,
    /// More files failed than `failures` names (a daemon's summary caps
    /// its list); the run's log names them all.
    #[serde(default)]
    pub failures_truncated: bool,
    /// The failed files whose write left this run's own incomplete copy at
    /// the destination, and whether that list is whole (review cr-jl4-1:
    /// an `--ignore-existing` retry sends those with the flag off).
    #[serde(default)]
    pub left_in_place: Vec<String>,
    #[serde(default)]
    pub left_in_place_truncated: bool,
    /// A move removed its source.
    #[serde(default)]
    pub source_removed: bool,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Failure {
    pub path: String,
    pub reason: String,
    /// The name's exact bytes, escaped, when it is not valid UTF-8 (as a
    /// log's `raw`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub raw: Option<String>,
}

/// Where a run is.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub enum RunState {
    /// Its command is going.
    Running,
    /// It ended, and the record says how.
    Finished,
    /// Its command stopped before it ended (Ctrl-C, or the process died).
    Interrupted,
    /// It goes on on a daemon (`--detach`); how it ends is the daemon's to
    /// say, and the record is filled in from it once it has.
    Waiting {
        /// The daemon, as `host:port`.
        daemon: String,
        /// The daemon's own ID for the job.
        job_id: String,
    },
}

impl RunState {
    pub fn as_str(&self) -> &'static str {
        match self {
            RunState::Running => "running",
            RunState::Finished => "finished",
            RunState::Interrupted => "interrupted",
            RunState::Waiting { .. } => "waiting",
        }
    }
}

impl RunRecord {
    /// A record for a run of `spec` starting now.
    pub fn starting(run_id: &str, spec: &JobSpec) -> Self {
        Self {
            format: RECORD_FORMAT.into(),
            version: VERSION,
            run_id: run_id.into(),
            machine: spec.machine.clone(),
            attempt: 1,
            parent: None,
            saved_job: None,
            verb: spec.verb.clone(),
            source: spec.source.as_arg().to_string(),
            destination: spec.destination.as_arg().to_string(),
            started_ms: now_ms(),
            ended_ms: None,
            state: RunState::Running,
            outcome: None,
            detail: None,
            files_copied: 0,
            files_deleted: 0,
            files_failed: 0,
            bytes_copied: 0,
            failures: Vec::new(),
            failures_truncated: false,
            left_in_place: Vec::new(),
            left_in_place_truncated: false,
            source_removed: false,
        }
    }
}

impl JobSpec {
    pub fn new(
        machine: &str,
        verb: &str,
        cwd: String,
        source: SpecEndpoint,
        destination: SpecEndpoint,
        options: SpecOptions,
        files_from: Option<Vec<String>>,
    ) -> Self {
        Self {
            format: SPEC_FORMAT.into(),
            version: VERSION,
            machine: machine.into(),
            host: host_name(),
            created_ms: now_ms(),
            verb: verb.into(),
            cwd,
            source,
            destination,
            options,
            files_from,
        }
    }
}

/// This machine's host name, or empty when it has none to give.
pub fn host_name() -> String {
    hostname::get()
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_default()
}

/// A job as a file of its own (jl-3b): a saved job (`--save`, `blit jobs
/// save`) or an exported one (`--export`, `blit jobs export`) — what to
/// run, under a name when it has one, and the run it was taken from when
/// it was a run's (which `blit jobs retry` needs).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct JobFile {
    pub format: String,
    pub version: u32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    pub spec: JobSpec,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub run: Option<RunRecord>,
}

impl JobFile {
    pub fn new(name: Option<String>, spec: JobSpec, run: Option<RunRecord>) -> Self {
        Self {
            format: FILE_FORMAT.into(),
            version: VERSION,
            name,
            spec,
            run,
        }
    }
}

/// A job file from a document's bytes: refused unless it and the documents
/// inside it are formats and versions this build reads.
pub fn read_job_file(bytes: &[u8]) -> io::Result<JobFile> {
    let file: JobFile = read_versioned(bytes, FILE_FORMAT)?;
    let inner = |format: &str, version: u32, expected: &str| {
        if format != expected {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!("its {expected} is not one (format {format:?})"),
            ));
        }
        if version != VERSION {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!("its {expected} is version {version}; this blit reads {VERSION}"),
            ));
        }
        Ok(())
    };
    inner(&file.spec.format, file.spec.version, SPEC_FORMAT)?;
    if let Some(run) = &file.run {
        inner(&run.format, run.version, RECORD_FORMAT)?;
    }
    Ok(file)
}

/// Whether `name` can name a saved job; see [`job_name_problem`].
pub fn valid_job_name(name: &str) -> bool {
    job_name_problem(name).is_none()
}

/// Why `name` cannot name a saved job, in words, or `None` when it can: 1
/// to 64 letters, digits, `-`, `_` or `.`, not starting with `.`, and not
/// shaped like a run ID (32 lowercase hex digits), so a bare word names
/// one job only (reviews cr-jl3b-1, cr-jl3bfix1-1: every refusal says its
/// own reason).
pub fn job_name_problem(name: &str) -> Option<&'static str> {
    if !(1..=64).contains(&name.len()) {
        return Some("a job name is 1 to 64 characters long");
    }
    if name.starts_with('.') {
        return Some("a job name may not start with `.`");
    }
    if !name
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '.'))
    {
        return Some("a job name is made of letters, digits, `-`, `_` and `.`");
    }
    if name.len() == 32 && name.bytes().all(|b| matches!(b, b'0'..=b'9' | b'a'..=b'f')) {
        return Some(
            "32 lowercase hex digits are a run ID's shape, kept for run IDs; choose \
             another name",
        );
    }
    None
}

/// Whether a `blit jobs` target names a file: it is a path — it holds a
/// separator (`./job.json`, `/tmp/x.jsonl.gz`). A bare word is a saved
/// job's name or a run ID, whatever files the current folder holds
/// (review cr-jl3b-1).
pub fn names_a_path(target: &str) -> bool {
    target.contains('/') || (cfg!(windows) && target.contains('\\'))
}

/// A machine's saved jobs: `<per-user folder>/jobs/saved/<name>.json`, kept
/// until deleted (never pruned).
#[derive(Clone, Debug)]
pub struct SavedJobs {
    dir: PathBuf,
}

impl SavedJobs {
    pub fn new(dir: impl Into<PathBuf>) -> Self {
        Self { dir: dir.into() }
    }

    pub fn dir(&self) -> &Path {
        &self.dir
    }

    fn path(&self, name: &str) -> io::Result<PathBuf> {
        if let Some(problem) = job_name_problem(name) {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                format!("{name:?} cannot name a saved job: {problem}"),
            ));
        }
        Ok(self.dir.join(format!("{name}.json")))
    }

    /// Keep `spec` as the saved job `name`, replacing one of that name;
    /// returns whether one was replaced.
    pub fn save(&self, name: &str, spec: &JobSpec) -> io::Result<bool> {
        let path = self.path(name)?;
        fs::create_dir_all(&self.dir)?;
        let replaced = path.exists();
        write_document(&path, &JobFile::new(Some(name.into()), spec.clone(), None))?;
        Ok(replaced)
    }

    /// The saved job `name`.
    pub fn load(&self, name: &str) -> io::Result<JobFile> {
        let path = self.path(name)?;
        match fs::read(&path) {
            Ok(bytes) => read_job_file(&bytes),
            Err(error) if error.kind() == io::ErrorKind::NotFound => Err(io::Error::new(
                io::ErrorKind::NotFound,
                format!("no saved job named {name}"),
            )),
            Err(error) => Err(error),
        }
    }

    /// Remove the saved job `name`.
    pub fn delete(&self, name: &str) -> io::Result<()> {
        match fs::remove_file(self.path(name)?) {
            Err(error) if error.kind() == io::ErrorKind::NotFound => Err(io::Error::new(
                io::ErrorKind::NotFound,
                format!("no saved job named {name}"),
            )),
            other => other,
        }
    }

    /// Every saved job, by name; one this build cannot read is listed with
    /// why. A missing folder holds none.
    pub fn list(&self) -> io::Result<Vec<(String, io::Result<JobFile>)>> {
        let entries = match fs::read_dir(&self.dir) {
            Ok(entries) => entries,
            Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(Vec::new()),
            Err(error) => return Err(error),
        };
        let mut jobs: Vec<(String, io::Result<JobFile>)> = entries
            .flatten()
            .filter_map(|entry| {
                let name = entry
                    .file_name()
                    .to_str()?
                    .strip_suffix(".json")?
                    .to_string();
                valid_job_name(&name).then(|| {
                    let job = fs::read(entry.path()).and_then(|bytes| read_job_file(&bytes));
                    (name, job)
                })
            })
            .collect();
        jobs.sort_by(|a, b| a.0.cmp(&b.0));
        Ok(jobs)
    }
}

/// A local endpoint as the spec keeps it: `path` made absolute against
/// `cwd` the way the command read it, its trailing separator kept. `None`
/// when the result is not valid UTF-8 (a spec must name it exactly).
pub fn absolute_local(cwd: &Path, path: &str) -> Option<String> {
    let joined = cwd.join(path);
    let mut text = joined.to_str()?.to_string();
    let trailing = path.ends_with('/') || (cfg!(windows) && path.ends_with('\\'));
    let sep = if cfg!(windows) { '\\' } else { '/' };
    if trailing && !text.ends_with(['/', '\\']) {
        text.push(sep);
    }
    Some(text)
}

/// Read a document `kind` (`format`) from `bytes`: refused unless it names
/// that format and a version this build reads, then parsed whole as that
/// version. The version is read first, untyped, so a newer document's
/// fields are never forced into this version's shape.
fn read_versioned<T: for<'de> Deserialize<'de>>(bytes: &[u8], format: &str) -> io::Result<T> {
    use serde_json::Value;
    let invalid = |message: String| io::Error::new(io::ErrorKind::InvalidData, message);
    let Ok(Value::Object(head)) = serde_json::from_slice::<Value>(bytes) else {
        return Err(invalid(format!("not a {format} document")));
    };
    let named = head.get("format").and_then(Value::as_str).unwrap_or("");
    if named != format {
        return Err(invalid(format!(
            "not a {format} document (format {named:?})"
        )));
    }
    match head.get("version").and_then(Value::as_u64) {
        Some(1) => serde_json::from_slice(bytes)
            .map_err(|error| invalid(format!("a damaged {format} document: {error}"))),
        Some(v) if v > u64::from(VERSION) => Err(invalid(format!(
            "written by a newer blit ({format} version {v}; this one reads up to {VERSION})"
        ))),
        Some(v) => Err(invalid(format!(
            "{format} version {v}, which no blit writes"
        ))),
        None => Err(invalid(format!("a {format} document without a version"))),
    }
}

/// A job spec from a document's bytes.
pub fn read_spec(bytes: &[u8]) -> io::Result<JobSpec> {
    read_versioned(bytes, SPEC_FORMAT)
}

/// A run record from a document's bytes.
pub fn read_record(bytes: &[u8]) -> io::Result<RunRecord> {
    read_versioned(bytes, RECORD_FORMAT)
}

/// Write `value` to `path` whole: a temp file beside it, synced, then
/// renamed over it.
///
/// Review cr-jl3a-2: each write stages in a file of its own — two writers
/// of one document (two `blit jobs` commands settling the same run) never
/// share one — and the folder is synced after the rename, so the new
/// document survives a crash.
pub fn write_document(path: &Path, value: &impl Serialize) -> io::Result<()> {
    let name = path.file_name().ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::InvalidInput,
            format!("{} names no file", path.display()),
        )
    })?;
    let dir = path
        .parent()
        .filter(|dir| !dir.as_os_str().is_empty())
        .unwrap_or(Path::new("."));
    let mut staged = std::ffi::OsString::from(".");
    staged.push(name);
    staged.push(format!(".{}{TEMP_SUFFIX}", crate::job_log::new_run_id()?));
    let temp = dir.join(staged);
    let result = (|| {
        let mut file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&temp)?;
        serde_json::to_writer_pretty(&mut file, value)?;
        file.write_all(b"\n")?;
        file.sync_all()?;
        drop(file);
        fs::rename(&temp, path)?;
        sync_dir(dir);
        Ok(())
    })();
    if result.is_err() {
        let _ = fs::remove_file(&temp);
    }
    result
}

/// Make a folder's entries durable (Unix; elsewhere the rename is).
fn sync_dir(dir: &Path) {
    #[cfg(unix)]
    if let Ok(handle) = File::open(dir) {
        let _ = handle.sync_all();
    }
    #[cfg(not(unix))]
    let _ = dir;
}

/// A machine's run records: `<per-user folder>/jobs/runs/`.
#[derive(Clone, Debug)]
pub struct RunStore {
    dir: PathBuf,
}

/// A run in the store whose command is going: holds the run's lock, so the
/// record reads `running` until it is replaced, or `interrupted` if the
/// command dies first. Dropping it lets go.
#[derive(Debug)]
pub struct LiveRun {
    _lock: File,
    lock_path: PathBuf,
}

impl Drop for LiveRun {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.lock_path);
    }
}

/// A run as the store lists it.
#[derive(Clone, Debug, PartialEq)]
pub struct StoredRun {
    pub record: RunRecord,
    pub spec_path: PathBuf,
    pub record_path: PathBuf,
}

impl RunStore {
    pub fn new(dir: impl Into<PathBuf>) -> Self {
        Self { dir: dir.into() }
    }

    pub fn dir(&self) -> &Path {
        &self.dir
    }

    fn path(&self, run_id: &str, suffix: &str) -> PathBuf {
        self.dir.join(format!("{run_id}{suffix}"))
    }

    /// Record a run starting: its spec, written once, and its record,
    /// `running`, under the run's lock, held until the returned
    /// [`LiveRun`] drops.
    pub fn begin(&self, spec: &JobSpec, record: &RunRecord) -> io::Result<LiveRun> {
        if !crate::job_log::valid_id(&record.run_id) {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                format!("{:?} cannot name a run", record.run_id),
            ));
        }
        fs::create_dir_all(&self.dir)?;
        let lock_path = self.path(&record.run_id, LOCK_SUFFIX);
        let lock = open_lock(&lock_path)?;
        lock.try_lock().map_err(|error| match error {
            fs::TryLockError::WouldBlock => io::Error::new(
                io::ErrorKind::AlreadyExists,
                format!("run {} is already going", record.run_id),
            ),
            fs::TryLockError::Error(error) => error,
        })?;
        let live = LiveRun {
            _lock: lock,
            lock_path,
        };
        write_document(&self.path(&record.run_id, SPEC_SUFFIX), spec)?;
        write_document(&self.path(&record.run_id, RECORD_SUFFIX), record)?;
        Ok(live)
    }

    /// Replace a run's record.
    pub fn update(&self, record: &RunRecord) -> io::Result<()> {
        write_document(&self.path(&record.run_id, RECORD_SUFFIX), record)
    }

    /// A run's spec and record.
    pub fn load(&self, run_id: &str) -> io::Result<(JobSpec, RunRecord)> {
        if !crate::job_log::valid_id(run_id) {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                format!("{run_id:?} is not a job ID"),
            ));
        }
        let spec = read_spec(&fs::read(self.path(run_id, SPEC_SUFFIX))?)?;
        let record = self.settled(read_record(&fs::read(self.path(run_id, RECORD_SUFFIX))?)?);
        Ok((spec, record))
    }

    /// `record` as it stands: a `running` record whose command has died
    /// (its lock is free) becomes `interrupted`, on disk too.
    fn settled(&self, mut record: RunRecord) -> RunRecord {
        if record.state != RunState::Running {
            return record;
        }
        let lock_path = self.path(&record.run_id, LOCK_SUFFIX);
        let Ok(lock) = open_lock(&lock_path) else {
            return record;
        };
        if lock.try_lock().is_err() {
            // Its command is going.
            return record;
        }
        // Re-read under the lock: the command may have finished between.
        if let Ok(now) =
            fs::read(self.path(&record.run_id, RECORD_SUFFIX)).and_then(|bytes| read_record(&bytes))
        {
            record = now;
        }
        if record.state == RunState::Running {
            record.state = RunState::Interrupted;
            record.outcome.get_or_insert(Outcome::Interrupted);
            record
                .detail
                .get_or_insert_with(|| "its command stopped before the run ended".into());
            if let Err(error) = self.update(&record) {
                log::warn!(
                    "job records: could not mark run {} interrupted: {error}",
                    record.run_id
                );
            }
        }
        drop(lock);
        let _ = fs::remove_file(&lock_path);
        record
    }

    /// Every run in the store, newest first, each settled (see
    /// [`load`](Self::load)). A record this build cannot read is skipped
    /// with a warning; a missing folder holds none.
    pub fn list(&self) -> io::Result<Vec<StoredRun>> {
        let entries = match fs::read_dir(&self.dir) {
            Ok(entries) => entries,
            Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(Vec::new()),
            Err(error) => return Err(error),
        };
        let mut runs = Vec::new();
        for entry in entries.flatten() {
            let name = entry.file_name();
            let Some(run_id) = name.to_str().and_then(|n| n.strip_suffix(RECORD_SUFFIX)) else {
                continue;
            };
            if !crate::job_log::valid_id(run_id) {
                continue;
            }
            let record = match fs::read(entry.path()).and_then(|bytes| read_record(&bytes)) {
                Ok(record) => record,
                Err(error) => {
                    log::warn!("job records: skipping {}: {error}", entry.path().display());
                    continue;
                }
            };
            runs.push(StoredRun {
                spec_path: self.path(run_id, SPEC_SUFFIX),
                record_path: entry.path(),
                record: self.settled(record),
            });
        }
        runs.sort_by(|a, b| {
            b.record
                .started_ms
                .cmp(&a.record.started_ms)
                .then_with(|| b.record.run_id.cmp(&a.record.run_id))
        });
        Ok(runs)
    }

    /// Remove the oldest runs beyond the newest `keep`, never one running
    /// or waiting on a daemon; returns the runs removed. Holds the folder's
    /// prune lock throughout, so two runs finishing at once do not race.
    pub fn prune(&self, keep: usize) -> io::Result<Vec<String>> {
        let lock = match open_lock(&self.dir.join(PRUNE_LOCK)) {
            Ok(lock) => lock,
            Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(Vec::new()),
            Err(error) => return Err(error),
        };
        lock.lock()?;
        let mut removed = Vec::new();
        for run in self.list()?.into_iter().skip(keep) {
            if matches!(
                run.record.state,
                RunState::Running | RunState::Waiting { .. }
            ) {
                continue;
            }
            let run_id = &run.record.run_id;
            match fs::remove_file(&run.record_path) {
                Ok(()) => {
                    let _ = fs::remove_file(&run.spec_path);
                    removed.push(run_id.clone());
                }
                Err(error) => {
                    log::warn!("job records: could not prune run {run_id}: {error}")
                }
            }
        }
        Ok(removed)
    }
}

/// What a daemon says of a run that went on on it (`--detach`).
#[derive(Clone, Debug, PartialEq)]
pub enum DaemonAnswer {
    /// The run ended: its record, filled in from the daemon.
    Ended(Box<RunRecord>),
    /// The run goes on.
    Going,
}

/// What a daemon's log of a run says, as read for the run's record.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct LogRead {
    /// Logs read (one per session of the run on that daemon).
    pub logs: usize,
    /// One of them is still being written, or waits for recovery.
    pub unfinished: bool,
    /// One says events were lost (`log-incomplete`).
    pub incomplete: bool,
    /// Its `run-end`: when, how, and why.
    pub ended: Option<(u64, Outcome, Option<String>)>,
    pub summary: Option<crate::job_log::Summary>,
    pub failures: Vec<Failure>,
    /// The failures whose reason says the write left its own incomplete
    /// copy (a log keeps each reason whole).
    pub left_in_place: Vec<String>,
}

impl LogRead {
    /// Review cr-jl3a-1: the log alone says how the run ended only when it
    /// is finished, ends with its `run-end` and `summary`, and lost nothing.
    pub fn complete(&self) -> bool {
        self.logs > 0
            && !self.unfinished
            && !self.incomplete
            && self.ended.is_some()
            && self.summary.is_some()
    }

    /// Take in one line of the log: its failures (each once), summary and
    /// end, and whether anything was lost — a `log-incomplete` event or a
    /// line that cannot be read.
    pub fn read_line(&mut self, line: crate::job_log::LogLine) {
        use crate::job_log::{EventBody, LogLine};
        let LogLine::Event(event) = line else {
            self.incomplete = true;
            return;
        };
        match event.body {
            EventBody::FileFailed { path, reason, raw } => {
                if reason.contains(crate::remote::transfer::sink::INCOMPLETE_LEFT_IN_PLACE)
                    && !self.left_in_place.contains(&path)
                {
                    self.left_in_place.push(path.clone());
                }
                if !self
                    .failures
                    .iter()
                    .any(|seen| seen.path == path && seen.raw == raw)
                {
                    self.failures.push(Failure { path, reason, raw });
                }
            }
            // Review cr-jl4fix1-1: the run's terminal state — a file a
            // later event landed, by the same identity, failed no more.
            EventBody::FileCopied { path, raw, .. } | EventBody::FileSent { path, raw } => {
                self.failures
                    .retain(|failed| !(failed.path == path && failed.raw == raw));
                if !self.failures.iter().any(|failed| failed.path == path) {
                    self.left_in_place.retain(|left| left != &path);
                }
            }
            EventBody::Summary(summary) => self.summary = Some(summary),
            EventBody::RunEnd { outcome, detail } => {
                self.ended = Some((event.ts_ms, outcome, detail))
            }
            EventBody::LogIncomplete { .. } => self.incomplete = true,
            _ => {}
        }
    }

    /// `record` ended as this log says, its failures `truncated` or not.
    fn ended_record(&self, record: &RunRecord, truncated: bool) -> RunRecord {
        let mut ended = record.clone();
        let (ended_ms, outcome, detail) = self.ended.clone().unwrap_or((
            now_ms(),
            Outcome::Interrupted,
            Some("its log has no end".into()),
        ));
        ended.ended_ms = Some(ended_ms);
        ended.state = match outcome {
            Outcome::Interrupted => RunState::Interrupted,
            _ => RunState::Finished,
        };
        ended.outcome = Some(outcome);
        ended.detail = detail;
        if let Some(summary) = &self.summary {
            ended.files_copied = summary.files_copied;
            ended.files_deleted = summary.files_deleted;
            ended.files_failed = summary.files_failed;
            ended.bytes_copied = summary.bytes_copied;
        }
        ended.failures = self.failures.clone();
        ended.failures_truncated = truncated;
        ended.left_in_place = self.left_in_place.clone();
        ended.left_in_place_truncated = truncated;
        ended
    }
}

/// How a waiting run ended, from what its daemon said (plan "Detached
/// jobs", review cr-jl3a-1): its log of the run when that is
/// [complete](LogRead::complete); otherwise the daemon's own job state —
/// `job`, `None` when it was not asked — decides: still active, it goes on;
/// finished, it ended as the job record says, keeping the failures the log
/// names (marked truncated, as the log is not whole); unknown to the
/// daemon, a finished log with its `run-end` is the best account left
/// (truncated), and anything less an error.
pub fn settle(
    record: &RunRecord,
    read: &LogRead,
    job: Option<&crate::admin::jobs::WatchSnapshot>,
) -> eyre::Result<DaemonAnswer> {
    use crate::admin::jobs::WatchSnapshot;
    if read.complete() {
        // A list shorter than the summary's count is not the whole list.
        let named_all = read
            .summary
            .is_some_and(|summary| read.failures.len() as u64 >= summary.files_failed);
        return Ok(DaemonAnswer::Ended(Box::new(
            read.ended_record(record, !named_all),
        )));
    }
    let (daemon, job_id) = match &record.state {
        RunState::Waiting { daemon, job_id } => (daemon.as_str(), job_id.as_str()),
        _ => return Ok(DaemonAnswer::Ended(Box::new(record.clone()))),
    };
    match job {
        Some(WatchSnapshot::Active(_)) => Ok(DaemonAnswer::Going),
        Some(WatchSnapshot::Finished(job)) => {
            let mut ended = record.clone();
            ended.ended_ms = Some(job.start_unix_ms.saturating_add(job.duration_ms));
            ended.state = RunState::Finished;
            let (outcome, detail) = match (job.ok, job.files_failed) {
                (true, 0) => (Outcome::Ok, None),
                (true, failed) => (Outcome::Failed, Some(format!("{failed} file(s) failed"))),
                (false, _) => (Outcome::Failed, Some(job.error_message.clone())),
            };
            ended.outcome = Some(outcome);
            ended.detail = detail;
            ended.files_copied = job.files;
            ended.bytes_copied = job.bytes;
            ended.files_failed = job.files_failed;
            ended.failures = read.failures.clone();
            ended.failures_truncated = job.files_failed > 0;
            ended.left_in_place = read.left_in_place.clone();
            ended.left_in_place_truncated = job.files_failed > 0;
            Ok(DaemonAnswer::Ended(Box::new(ended)))
        }
        Some(WatchSnapshot::NotFound) | None => {
            if read.logs > 0 && !read.unfinished && read.ended.is_some() {
                return Ok(DaemonAnswer::Ended(Box::new(
                    read.ended_record(record, true),
                )));
            }
            let why = if read.logs == 0 {
                "and no log of it"
            } else {
                "and its log there is unfinished"
            };
            Err(eyre::eyre!(
                "{daemon} has no record of job {job_id} (run {}) any more, {why}",
                record.run_id
            ))
        }
    }
}

/// Ask the daemon a waiting run went on on how it ended — its log of the
/// run, by run ID, and, unless that is complete, its job state — and
/// [`settle`] it. An error when the daemon cannot be reached or knows
/// nothing of the run. A record not waiting is returned as it is.
pub async fn ask_daemon(record: &RunRecord) -> eyre::Result<DaemonAnswer> {
    use crate::admin::jobs;
    use crate::job_log::{LogLines, Role};
    use crate::remote::endpoint::RemoteEndpoint;
    use eyre::WrapErr;
    use std::sync::{Arc, Mutex};

    let RunState::Waiting { daemon, job_id } = &record.state else {
        return Ok(DaemonAnswer::Ended(Box::new(record.clone())));
    };
    let remote = RemoteEndpoint::parse(daemon)
        .wrap_err_with(|| format!("parsing the daemon address {daemon:?}"))?;

    let read = Arc::new(Mutex::new(LogRead::default()));
    let sink = Arc::clone(&read);
    let fetched = jobs::read_job_logs(
        &remote,
        &record.run_id,
        Some(Role::Destination),
        false,
        move |header, lines| {
            let mut read = sink.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
            read.logs += 1;
            read.unfinished |= !header.finished;
            for line in LogLines::new(lines) {
                read.read_line(line?);
            }
            Ok(())
        },
    )
    .await;
    let mut read =
        std::mem::take(&mut *read.lock().unwrap_or_else(|poisoned| poisoned.into_inner()));
    if fetched.is_err() {
        // Whatever arrived before the failure is not the whole log.
        read.incomplete = true;
    }
    if read.complete() {
        return settle(record, &read, None);
    }
    let state = jobs::query(&remote, 0)
        .await
        .wrap_err_with(|| format!("asking {daemon} how job {job_id} ended"))?;
    settle(record, &read, Some(&jobs::watch_snapshot(&state, job_id)))
}

fn open_lock(path: &Path) -> io::Result<File> {
    OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(false)
        .open(path)
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

    const RUN: &str = "0123456789abcdef0123456789abcdef";

    fn spec() -> JobSpec {
        JobSpec::new(
            "m1",
            "copy",
            "/home/me".into(),
            SpecEndpoint::Local {
                path: "/home/me/src/".into(),
            },
            SpecEndpoint::Remote {
                locator: "server:/backup/me".into(),
                host: "server".into(),
                port: 9031,
                module: Some("backup".into()),
                path: "me".into(),
            },
            SpecOptions {
                checksum: true,
                delete_scope: "subset".into(),
                exclude: vec!["*.tmp".into()],
                ..SpecOptions::default()
            },
            Some(vec!["a.txt".into()]),
        )
    }

    #[test]
    fn a_run_reads_back_as_written_and_settles_when_its_command_ends() {
        let dir = tempfile::tempdir().unwrap();
        let store = RunStore::new(dir.path().join("runs"));
        let job = spec();
        let record = RunRecord::starting(RUN, &job);
        let live = store.begin(&job, &record).unwrap();

        let (read_spec, read_record) = store.load(RUN).unwrap();
        assert_eq!(read_spec, job);
        // Its command holds the lock: still running.
        assert_eq!(read_record.state, RunState::Running);
        assert!(
            store.begin(&spec(), &record).is_err(),
            "one command per run"
        );

        let mut done = read_record;
        done.state = RunState::Finished;
        done.outcome = Some(Outcome::Ok);
        done.files_copied = 3;
        store.update(&done).unwrap();
        drop(live);
        assert_eq!(store.load(RUN).unwrap().1, done);
        assert!(!dir.path().join("runs").join(format!("{RUN}.lock")).exists());
    }

    #[test]
    fn a_record_left_running_without_its_lock_settles_as_interrupted() {
        let dir = tempfile::tempdir().unwrap();
        let store = RunStore::new(dir.path());
        drop(
            store
                .begin(&spec(), &RunRecord::starting(RUN, &spec()))
                .unwrap(),
        );
        // As a killed command leaves it: the record says running, and no
        // one holds its lock.
        let record = store.load(RUN).unwrap().1;
        assert_eq!(record.state, RunState::Interrupted);
        assert_eq!(record.outcome, Some(Outcome::Interrupted));
        // On disk too.
        let on_disk =
            read_record(&fs::read(dir.path().join(format!("{RUN}.run.json"))).unwrap()).unwrap();
        assert_eq!(on_disk.state, RunState::Interrupted);
    }

    /// Review cr-jl3a-2: writers of one document at once never share a
    /// staging file — every write lands whole, and none fails.
    #[test]
    fn concurrent_writes_of_one_record_each_land_whole() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join(format!("{RUN}.run.json"));
        let record = RunRecord::starting(RUN, &spec());
        let writers: Vec<_> = (0..8u64)
            .map(|n| {
                let (path, mut record) = (path.clone(), record.clone());
                std::thread::spawn(move || {
                    for round in 0..40 {
                        record.files_copied = n * 1000 + round;
                        write_document(&path, &record).expect("every write lands");
                    }
                })
            })
            .collect();
        for writer in writers {
            writer.join().unwrap();
        }
        let landed = read_record(&fs::read(&path).unwrap()).unwrap();
        assert_eq!(landed.files_copied % 1000, 39, "a whole last write");
        let left: Vec<_> = fs::read_dir(dir.path())
            .unwrap()
            .flatten()
            .map(|entry| entry.file_name())
            .collect();
        assert_eq!(left.len(), 1, "no staging file left: {left:?}");
    }

    #[test]
    fn documents_of_another_format_or_a_newer_version_are_refused() {
        let mut value = serde_json::to_value(spec()).unwrap();
        assert!(read_spec(&serde_json::to_vec(&value).unwrap()).is_ok());
        value["version"] = 2.into();
        let error = read_spec(&serde_json::to_vec(&value).unwrap()).unwrap_err();
        assert!(error.to_string().contains("newer blit"), "{error}");
        value["version"] = 0.into();
        assert!(read_spec(&serde_json::to_vec(&value).unwrap()).is_err());
        value["version"] = 1.into();
        value["format"] = "blit-run-record".into();
        assert!(read_spec(&serde_json::to_vec(&value).unwrap()).is_err());
        // An option this build does not know: refused, not run without it.
        value["format"] = SPEC_FORMAT.into();
        value["options"]["compress"] = true.into();
        let error = read_spec(&serde_json::to_vec(&value).unwrap()).unwrap_err();
        assert!(error.to_string().contains("compress"), "{error}");
    }

    #[test]
    fn pruning_keeps_the_newest_and_never_a_live_or_waiting_run() {
        let dir = tempfile::tempdir().unwrap();
        let store = RunStore::new(dir.path());
        let ids: Vec<String> = (0..5).map(|n| format!("{n:032x}")).collect();
        let mut live = Vec::new();
        for (n, id) in ids.iter().enumerate() {
            let mut record = RunRecord::starting(id, &spec());
            record.started_ms = 1000 + n as u64;
            let guard = store.begin(&spec(), &record).unwrap();
            record.state = match n {
                // The oldest is waiting on a daemon; the next is running.
                0 => RunState::Waiting {
                    daemon: "server:9031".into(),
                    job_id: "t1-0".into(),
                },
                1 => RunState::Running,
                _ => RunState::Finished,
            };
            store.update(&record).unwrap();
            if n == 1 {
                live.push(guard);
            }
        }
        let removed = store.prune(2).unwrap();
        // Newest two kept (4, 3); 2 removed; 1 is running, 0 waiting.
        assert_eq!(removed, vec![ids[2].clone()]);
        let left: Vec<String> = store
            .list()
            .unwrap()
            .into_iter()
            .map(|run| run.record.run_id)
            .collect();
        assert_eq!(
            left,
            vec![
                ids[4].clone(),
                ids[3].clone(),
                ids[1].clone(),
                ids[0].clone()
            ]
        );
        assert!(!dir.path().join(format!("{}.spec.json", ids[2])).exists());
    }

    /// Review cr-jl3a-1: a detached run is settled from its daemon's log
    /// only when that log is whole; otherwise the daemon's job state
    /// decides, and the log's failures are kept but marked truncated.
    #[test]
    fn a_detached_run_trusts_only_a_complete_log() {
        use crate::admin::jobs::WatchSnapshot;
        use crate::generated::{ActiveTransfer, TransferRecord};
        use crate::job_log::Summary;
        let mut waiting = RunRecord::starting(RUN, &spec());
        waiting.state = RunState::Waiting {
            daemon: "server:9031".into(),
            job_id: "t1-0".into(),
        };
        let failure = Failure {
            path: "a.txt".into(),
            reason: "denied".into(),
            raw: None,
        };
        let whole = LogRead {
            logs: 1,
            ended: Some((7, Outcome::Failed, Some("1 file(s) failed".into()))),
            summary: Some(Summary {
                files_copied: 2,
                files_failed: 1,
                ..Summary::default()
            }),
            failures: vec![failure.clone()],
            ..LogRead::default()
        };
        let ended = |answer: eyre::Result<DaemonAnswer>| match answer.unwrap() {
            DaemonAnswer::Ended(record) => *record,
            DaemonAnswer::Going => panic!("expected an end"),
        };
        let done = TransferRecord {
            transfer_id: "t1-0".into(),
            ok: true,
            files: 5,
            files_failed: 1,
            ..TransferRecord::default()
        };

        // Whole, but naming fewer failures than its summary counts: the
        // list is marked incomplete (review cr-jl4fix1-1).
        let short = LogRead {
            summary: Some(Summary {
                files_copied: 2,
                files_failed: 3,
                ..Summary::default()
            }),
            ..whole.clone()
        };
        assert!(ended(settle(&waiting, &short, None)).failures_truncated);

        // Whole: the log decides, its failures exact.
        let from_log = ended(settle(&waiting, &whole, None));
        assert_eq!(
            (
                from_log.state.clone(),
                from_log.outcome,
                from_log.files_copied
            ),
            (RunState::Finished, Some(Outcome::Failed), 2)
        );
        assert_eq!(
            (from_log.failures.clone(), from_log.failures_truncated),
            (vec![failure.clone()], false)
        );

        // Unfinished, or missing its summary, or with a gap: the daemon's
        // job state decides.
        for partial in [
            LogRead {
                unfinished: true,
                ..whole.clone()
            },
            LogRead {
                summary: None,
                ..whole.clone()
            },
            LogRead {
                incomplete: true,
                ..whole.clone()
            },
        ] {
            assert!(!partial.complete());
            assert_eq!(
                settle(
                    &waiting,
                    &partial,
                    Some(&WatchSnapshot::Active(ActiveTransfer::default()))
                )
                .unwrap(),
                DaemonAnswer::Going
            );
            let from_job = ended(settle(
                &waiting,
                &partial,
                Some(&WatchSnapshot::Finished(done.clone())),
            ));
            assert_eq!(
                (
                    from_job.outcome,
                    from_job.files_copied,
                    from_job.files_failed
                ),
                (Some(Outcome::Failed), 5, 1)
            );
            assert_eq!(
                (from_job.failures, from_job.failures_truncated),
                (vec![failure.clone()], true)
            );
        }

        // Unknown to the daemon: a finished log with its end is the best
        // account (truncated); an unfinished one, or none, is an error.
        let recovered = LogRead {
            summary: None,
            ended: Some((9, Outcome::Interrupted, None)),
            ..whole.clone()
        };
        let best = ended(settle(&waiting, &recovered, Some(&WatchSnapshot::NotFound)));
        assert_eq!(
            (best.state, best.failures_truncated),
            (RunState::Interrupted, true)
        );
        for lost in [
            LogRead {
                unfinished: true,
                ..whole.clone()
            },
            LogRead::default(),
        ] {
            assert!(settle(&waiting, &lost, Some(&WatchSnapshot::NotFound)).is_err());
        }
    }

    /// Review cr-jl3a-1: reading a log notes what makes it less than whole.
    #[test]
    fn reading_a_log_notes_what_it_lost() {
        use crate::job_log::{Event, EventBody, LogLine, Summary};
        let event = |seq, body| {
            LogLine::Event(Event {
                ts_ms: 5,
                seq,
                body,
            })
        };
        let failed = || EventBody::FileFailed {
            path: "a.txt".into(),
            reason: "denied".into(),
            raw: None,
        };
        let mut read = LogRead {
            logs: 1,
            ..LogRead::default()
        };
        read.read_line(event(1, failed()));
        read.read_line(event(2, failed()));
        read.read_line(event(3, EventBody::Summary(Summary::default())));
        read.read_line(event(
            4,
            EventBody::RunEnd {
                outcome: Outcome::Ok,
                detail: None,
            },
        ));
        assert_eq!(read.failures.len(), 1, "each failure once");
        assert!(read.complete());
        // Review cr-jl4-1: a whole reason says when the write left its own
        // incomplete copy.
        assert!(read.left_in_place.is_empty());
        let mut own = read.clone();
        own.read_line(event(
            5,
            EventBody::FileFailed {
                path: "b.txt".into(),
                reason: format!(
                    "write failed: {}: it could not be removed",
                    crate::remote::transfer::sink::INCOMPLETE_LEFT_IN_PLACE
                ),
                raw: None,
            },
        ));
        assert_eq!(own.left_in_place, ["b.txt"]);
        // Review cr-jl4fix1-1: a later copy of the same identity lands it —
        // failed no more, and no longer left in place.
        own.read_line(event(
            6,
            EventBody::FileCopied {
                path: "b.txt".into(),
                bytes: None,
                raw: None,
            },
        ));
        assert!(own.failures.iter().all(|failed| failed.path != "b.txt"));
        assert!(own.left_in_place.is_empty());
        // ...but a copy of another identity of that text does not.
        let mut other = read.clone();
        other.read_line(event(
            7,
            EventBody::FileCopied {
                path: "a.txt".into(),
                bytes: None,
                raw: Some("a\\xff.txt".into()),
            },
        ));
        assert_eq!(other.failures.len(), 1);
        let mut gap = read.clone();
        gap.read_line(event(
            5,
            EventBody::LogIncomplete {
                reason: "disk full".into(),
            },
        ));
        assert!(!gap.complete());
        let mut torn = read.clone();
        torn.read_line(LogLine::Unreadable {
            line: 6,
            torn: true,
        });
        assert!(!torn.complete());
    }

    #[test]
    fn saved_jobs_keep_a_spec_by_name_until_deleted() {
        let dir = tempfile::tempdir().unwrap();
        let saved = SavedJobs::new(dir.path().join("saved"));
        let job = spec();
        assert!(!saved.save("nightly", &job).unwrap());
        assert!(saved.save("nightly", &job).unwrap(), "replaced");
        let file = saved.load("nightly").unwrap();
        assert_eq!((file.name.as_deref(), &file.spec), (Some("nightly"), &job));
        let listed: Vec<String> = saved.list().unwrap().into_iter().map(|(n, _)| n).collect();
        assert_eq!(listed, ["nightly"]);
        saved.delete("nightly").unwrap();
        let error = saved.load("nightly").unwrap_err();
        assert!(
            error.to_string().contains("no saved job named nightly"),
            "{error}"
        );
        assert!(saved.delete("nightly").is_err());
        // Each refusal names its own reason (review cr-jl3bfix1-1).
        let long = "x".repeat(65);
        for (bad, reason) in [
            ("", "1 to 64 characters"),
            (long.as_str(), "1 to 64 characters"),
            (".hidden", "may not start with `.`"),
            ("a/b", "letters, digits"),
            ("a b", "letters, digits"),
            ("0123456789abcdef0123456789abcdef", "kept for run IDs"),
        ] {
            let error = saved.save(bad, &job).unwrap_err().to_string();
            assert!(error.contains(reason), "{bad:?}: {error}");
        }
        // Upper-case hex, or another length, is a name like any other.
        assert!(valid_job_name("0123456789ABCDEF0123456789ABCDEF"));
        assert!(valid_job_name("0123456789abcdef"));
    }

    #[test]
    fn a_job_file_refuses_documents_inside_it_it_cannot_read() {
        let file = JobFile::new(None, spec(), None);
        let bytes = serde_json::to_vec(&file).unwrap();
        assert_eq!(read_job_file(&bytes).unwrap(), file);
        let mut value = serde_json::to_value(&file).unwrap();
        value["spec"]["version"] = 2.into();
        let error = read_job_file(&serde_json::to_vec(&value).unwrap()).unwrap_err();
        assert!(error.to_string().contains("version 2"), "{error}");
        let mut value = serde_json::to_value(&file).unwrap();
        value["format"] = "blit-job-spec".into();
        assert!(read_job_file(&serde_json::to_vec(&value).unwrap()).is_err());
    }

    #[test]
    fn local_paths_are_made_absolute_with_their_trailing_separator() {
        let cwd = Path::new(if cfg!(windows) { r"C:\work" } else { "/work" });
        let sep = std::path::MAIN_SEPARATOR;
        assert_eq!(
            absolute_local(cwd, "src/").unwrap(),
            format!("{}{sep}src{sep}", cwd.display())
        );
        assert_eq!(
            absolute_local(cwd, "src").unwrap(),
            format!("{}{sep}src", cwd.display())
        );
        let absolute = if cfg!(windows) { r"D:\data\" } else { "/data/" };
        assert_eq!(absolute_local(cwd, absolute).unwrap(), absolute);
    }
}
