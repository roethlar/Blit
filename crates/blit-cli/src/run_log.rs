//! This command's run (JOB_LOGS jl-2, jl-3): the run ID made where the
//! command is typed and sent with every session it opens, so every machine
//! involved logs the run under one ID; the numbering of those sessions; the
//! command's own log — the initiator's — in the per-user folder; and the
//! run's job record there: its spec (what to run again) and its record
//! (how this run went).

use crate::cli::TransferArgs;
use blit_core::config;
use blit_core::endpoints::{parse_transfer_endpoint, Endpoint};
use blit_core::job_log::{self, EventBody, Outcome, Role, RunInfo, RunTag};
use blit_core::job_record::{
    self, Failure, JobFile, JobSpec, LiveRun, RunRecord, RunState, RunStore, SavedJobs,
    SpecEndpoint, SpecOptions,
};
use blit_core::remote::transfer::RemoteTransferProgress;
use blit_core::remote::RemotePath;
use blit_core::run_log::{Disposition, End, LogPlace, RunLog, RunTotals};
use std::path::{Path, PathBuf};
use std::process::ExitCode;
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::sync::{Arc, Mutex};

/// One copy, mirror or move command's run, shared by every pass and every
/// `--retry` rerun of it (clones share it).
#[derive(Clone)]
pub struct CommandRun {
    inner: Arc<Inner>,
}

struct Inner {
    run_id: String,
    /// Sessions opened so far.
    sessions: AtomicU32,
    log: RunLog,
    /// How the run ended, when a route says so before it returns.
    ending: Mutex<Option<Ending>>,
    /// The run's job record, when one could be kept.
    record: Mutex<Option<Recorded>>,
    /// Why it could not, when it could not.
    unkept: Option<String>,
    /// The per-user folder and the run store, for `--save`/`--export`.
    config_dir: Option<PathBuf>,
    store: Option<RunStore>,
    /// The run's final account, for its record.
    totals: Mutex<Option<RunTotals>>,
    source_removed: AtomicBool,
    /// Whether the run writes what it moves (review cr-jl2-1).
    disposition: Disposition,
}

/// How a route said the run ended.
enum Ending {
    Interrupted(String),
    /// Went on on `daemon` as its job `job_id`.
    Detached {
        daemon: String,
        job_id: String,
    },
}

/// A run's record in the store, its command going.
struct Recorded {
    store: RunStore,
    spec: JobSpec,
    record: RunRecord,
    keep: usize,
    /// Held while the command goes.
    _live: LiveRun,
}

impl std::fmt::Debug for CommandRun {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "CommandRun({})", self.inner.run_id)
    }
}

/// This machine as the CLI keeps it: the per-user folder, its ID and
/// settings.
struct Machine {
    config_dir: PathBuf,
    id: String,
    keep: usize,
}

impl CommandRun {
    /// Start this command's run, its log and its job record in the per-user
    /// folder, after finishing any log an earlier, killed command left
    /// there. `None` only when the system cannot make a random ID; a log or
    /// record that cannot be kept is skipped with a warning — neither ever
    /// stops a command.
    ///
    /// `saved_job` names the saved job this run runs (`blit jobs run`).
    pub async fn start(verb: &str, args: &TransferArgs, saved_job: Option<String>) -> Option<Self> {
        let run_id = job_log::new_run_id().ok()?;
        let machine = tokio::task::spawn_blocking(this_machine)
            .await
            .ok()
            .flatten();
        let place = machine.as_ref().map(|machine| LogPlace {
            dir: logs_dir(&machine.config_dir),
            participant: machine.id.clone(),
            keep: machine.keep,
            finished: None,
        });
        let log = RunLog::new(place, &run_id);
        // Review cr-jl2-1: a dry run or a null-sink run writes nothing, and
        // its log must not say otherwise.
        let disposition = if args.dry_run {
            Disposition::DryRun
        } else if args.null {
            Disposition::Discarded
        } else {
            Disposition::Written
        };
        log.set_disposition(disposition);
        log.start(
            Role::Initiator,
            RunInfo {
                verb: verb.to_string(),
                source: args.source.clone(),
                destination: args.destination.clone(),
                options: option_words(args),
                job_id: None,
            },
            Some(RunTag {
                run_id: run_id.clone(),
                attempt: 1,
            }),
        );
        let config_dir = machine.as_ref().map(|machine| machine.config_dir.clone());
        let store = config_dir
            .as_deref()
            .map(|dir| RunStore::new(runs_dir(dir)));
        let (recorded, unkept) = match machine {
            Some(machine) => {
                let (verb, args, run_id) = (verb.to_string(), args.clone(), run_id.clone());
                tokio::task::spawn_blocking(move || {
                    begin_record(&machine, &verb, &args, &run_id, saved_job)
                })
                .await
                .unwrap_or_else(|error| (None, Some(format!("{error}"))))
            }
            None => (
                None,
                Some("this machine's folder or ID cannot be had".into()),
            ),
        };
        if args.verbose && !args.json {
            eprintln!("blit: job {run_id} (`blit jobs log {run_id}` shows its log)");
        }
        Some(Self {
            inner: Arc::new(Inner {
                run_id,
                sessions: AtomicU32::new(0),
                log,
                ending: Mutex::new(None),
                record: Mutex::new(recorded),
                unkept,
                config_dir,
                store,
                totals: Mutex::new(None),
                source_removed: AtomicBool::new(false),
                disposition,
            }),
        })
    }

    #[cfg(test)]
    pub fn run_id(&self) -> &str {
        &self.inner.run_id
    }

    /// A run that keeps no log, for tests.
    #[cfg(test)]
    pub fn unlogged() -> Self {
        let run_id = job_log::new_run_id().expect("a run ID");
        Self {
            inner: Arc::new(Inner {
                log: RunLog::new(None, &run_id),
                run_id,
                sessions: AtomicU32::new(0),
                ending: Mutex::new(None),
                record: Mutex::new(None),
                unkept: None,
                config_dir: None,
                store: None,
                totals: Mutex::new(None),
                source_removed: AtomicBool::new(false),
                disposition: Disposition::Written,
            }),
        }
    }

    /// `--save NAME` (JOB_LOGS jl-3b): keep this command's job as the
    /// saved job `name`, before it runs. An error when the job cannot be
    /// kept — the person asked for it.
    pub async fn save_as(&self, name: &str, quiet: bool) -> eyre::Result<()> {
        let spec = {
            let mut record = lock(&self.inner.record);
            let recorded = record.as_mut().ok_or_else(|| {
                eyre::eyre!(
                    "this command cannot be saved as a job: {}",
                    self.inner
                        .unkept
                        .as_deref()
                        .unwrap_or("its job is not kept")
                )
            })?;
            recorded.record.saved_job = Some(name.to_string());
            recorded.spec.clone()
        };
        let config_dir = self
            .inner
            .config_dir
            .clone()
            .ok_or_else(|| eyre::eyre!("this machine's folder cannot be had"))?;
        let name_owned = name.to_string();
        let replaced = tokio::task::spawn_blocking(move || {
            SavedJobs::new(saved_dir(&config_dir)).save(&name_owned, &spec)
        })
        .await??;
        if !quiet {
            let note = if replaced {
                " (replacing the job saved under that name before)"
            } else {
                ""
            };
            eprintln!("blit: saved job {name}{note}; `blit jobs run {name}` runs it again");
        }
        Ok(())
    }

    /// `--export FILE` (JOB_LOGS jl-3b): write this command's job, and how
    /// its run went, to `file` — after [`finish`](Self::finish).
    pub async fn export_to(&self, file: &Path, quiet: bool) -> eyre::Result<()> {
        let store = self.inner.store.clone().ok_or_else(|| {
            eyre::eyre!(
                "this command's job cannot be exported: {}",
                self.inner
                    .unkept
                    .as_deref()
                    .unwrap_or("its job is not kept")
            )
        })?;
        let (run_id, file_owned) = (self.inner.run_id.clone(), file.to_path_buf());
        tokio::task::spawn_blocking(move || -> eyre::Result<()> {
            let (spec, record) = store
                .load(&run_id)
                .map_err(|error| eyre::eyre!("this command's job cannot be exported: {error}"))?;
            let job = JobFile::new(record.saved_job.clone(), spec, Some(record));
            job_record::write_document(&file_owned, &job)
                .map_err(|error| eyre::eyre!("writing {}: {error}", file_owned.display()))
        })
        .await??;
        if !quiet {
            eprintln!("blit: wrote this job to {}", file.display());
        }
        Ok(())
    }

    /// The tag for the next session this run opens: its number counts every
    /// session of the run from 1 — the main pass, each retry session, each
    /// `--retry` rerun — so no two share a daemon's log key.
    pub fn next_session(&self) -> RunTag {
        RunTag {
            run_id: self.inner.run_id.clone(),
            attempt: self.inner.sessions.fetch_add(1, Ordering::Relaxed) + 1,
        }
    }

    /// A progress handle that also feeds this run's log from `end` of the
    /// transfer: `progress` with a fresh audit lane attached, or a log-only
    /// handle when there is no live display. `progress` unchanged when the
    /// run keeps no log.
    pub fn with_log(
        &self,
        progress: Option<RemoteTransferProgress>,
        end: End,
    ) -> Option<RemoteTransferProgress> {
        let Some(audit) = self.inner.log.audit_lane_at(end) else {
            return progress;
        };
        Some(match progress {
            Some(progress) => progress.with_audit(audit),
            None => RemoteTransferProgress::audit_only(audit),
        })
    }

    /// Record a line in the run's log now, in order with the transfer's.
    pub async fn record(&self, body: EventBody) {
        self.inner.log.record(body).await;
    }

    /// A diagnostic for the run's log, recorded at its close.
    pub fn note(&self, line: String) {
        self.inner.log.note(line);
    }

    /// The run's final account, across every pass.
    pub fn note_totals(&self, totals: RunTotals) {
        *lock(&self.inner.totals) = Some(totals.clone());
        self.inner.log.note_totals(totals);
    }

    /// A move removed its source.
    pub fn note_source_removed(&self, source: &str) {
        self.inner.source_removed.store(true, Ordering::Relaxed);
        self.note(format!("move: removed the source {source}"));
    }

    /// The run went on on `daemon` (`host:port`) as its job `job_id` after
    /// this command returns (`--detach`).
    pub fn note_detached(&self, daemon: &str, job_id: &str) {
        *lock(&self.inner.ending) = Some(Ending::Detached {
            daemon: daemon.to_string(),
            job_id: job_id.to_string(),
        });
    }

    /// The run stopped on Ctrl-C.
    pub fn note_interrupted(&self, detail: String) {
        *lock(&self.inner.ending) = Some(Ending::Interrupted(detail));
    }

    /// Close the run's log, and finish its record, with how the command
    /// ended.
    pub async fn finish(&self, result: &eyre::Result<ExitCode>) {
        let ending = lock(&self.inner.ending).take();
        let (outcome, mut detail) = match (&ending, result) {
            (Some(Ending::Interrupted(detail)), _) => (Outcome::Interrupted, Some(detail.clone())),
            (Some(Ending::Detached { daemon, job_id }), _) => (
                Outcome::Detached,
                Some(format!(
                    "runs on {daemon} as job {job_id} (`blit jobs log {daemon} {job_id}`)"
                )),
            ),
            (None, Err(error)) => (Outcome::Failed, Some(format!("{error:#}"))),
            (None, Ok(_)) => match self.inner.log.noted_files_failed() {
                0 => (Outcome::Ok, None),
                failed => (Outcome::Failed, Some(format!("{failed} file(s) failed"))),
            },
        };
        if let Some(report) = self.inner.log.close(outcome, detail.clone(), None).await {
            if !report.complete {
                eprintln!(
                    "blit: this run's log is incomplete: {}",
                    report.problem.unwrap_or_default()
                );
            }
        }
        let Some(recorded) = lock(&self.inner.record).take() else {
            return;
        };
        let mut totals = lock(&self.inner.totals).take();
        // Review cr-jl2-1: a run that wrote nothing copied nothing.
        if let Some(said) = self.inner.disposition.nothing_written() {
            if let Some(totals) = totals.as_mut() {
                totals.files_copied = 0;
                totals.files_deleted = 0;
                totals.bytes_copied = 0;
            }
            if detail.is_none() {
                detail = Some(said.to_string());
            }
        }
        let source_removed = self.inner.source_removed.load(Ordering::Relaxed);
        let raws = totals
            .as_ref()
            .map(|totals| failed_raw_names(&self.inner.log, totals))
            .unwrap_or_default();
        let _ = tokio::task::spawn_blocking(move || {
            finish_record(
                recorded,
                ending,
                outcome,
                detail,
                totals,
                &raws,
                source_removed,
            )
        })
        .await;
    }
}

/// Review cr-jl3a-3: each failed name that is not UTF-8, by its text, with
/// its exact bytes escaped, as the run's closed log knows it.
fn failed_raw_names(log: &RunLog, totals: &RunTotals) -> std::collections::HashMap<String, String> {
    totals
        .failed_paths
        .iter()
        .chain(totals.failures.iter().map(|failure| &failure.relative_path))
        .filter_map(|path| Some((path.clone(), log.raw_name(path)?)))
        .collect()
}

fn lock<T>(mutex: &Mutex<T>) -> std::sync::MutexGuard<'_, T> {
    mutex
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

/// The per-user folder, this machine's ID and its settings. `None`, with a
/// warning, when the folder or the ID cannot be had. Blocking.
fn this_machine() -> Option<Machine> {
    let warn = |what: String| eprintln!("blit: warning: this run is not logged: {what}");
    let config_dir = match config::config_dir() {
        Ok(dir) => dir,
        Err(error) => {
            warn(format!("{error:#}"));
            return None;
        }
    };
    let settings = config::load_settings(&config_dir).unwrap_or_else(|error| {
        eprintln!("blit: warning: {error:#}; using the default settings");
        config::Settings::default()
    });
    let id = match job_log::machine_id(&config_dir) {
        Ok(id) => id,
        Err(error) => {
            warn(format!("{error}"));
            return None;
        }
    };
    Some(Machine {
        config_dir,
        id,
        keep: settings.jobs_keep,
    })
}

/// This machine's run records: `<per-user folder>/jobs/runs`.
pub fn runs_dir(config_dir: &Path) -> PathBuf {
    config_dir.join("jobs").join("runs")
}

/// This machine's saved jobs: `<per-user folder>/jobs/saved`.
pub fn saved_dir(config_dir: &Path) -> PathBuf {
    config_dir.join("jobs").join("saved")
}

/// Write the run's spec and its `running` record. `None`, with a warning
/// and why, when either cannot be kept — the command runs regardless.
/// Blocking.
fn begin_record(
    machine: &Machine,
    verb: &str,
    args: &TransferArgs,
    run_id: &str,
    saved_job: Option<String>,
) -> (Option<Recorded>, Option<String>) {
    let unkept = |what: String| {
        eprintln!("blit: warning: this run's job is not kept: {what}");
        (None, Some(what))
    };
    let spec = match job_spec(verb, args, &machine.id) {
        Ok(spec) => spec,
        Err(what) => return unkept(what),
    };
    let store = RunStore::new(runs_dir(&machine.config_dir));
    let mut record = RunRecord::starting(run_id, &spec);
    record.saved_job = saved_job;
    match store.begin(&spec, &record) {
        Ok(live) => (
            Some(Recorded {
                store,
                spec,
                record,
                keep: machine.keep,
                _live: live,
            }),
            None,
        ),
        Err(error) => unkept(format!("{error}")),
    }
}

/// The run's record as the command ended, then pruning. Blocking.
fn finish_record(
    mut recorded: Recorded,
    ending: Option<Ending>,
    outcome: Outcome,
    detail: Option<String>,
    totals: Option<RunTotals>,
    raws: &std::collections::HashMap<String, String>,
    source_removed: bool,
) {
    let record = &mut recorded.record;
    record.ended_ms = Some(now_ms());
    record.state = match ending {
        Some(Ending::Detached { daemon, job_id }) => RunState::Waiting { daemon, job_id },
        Some(Ending::Interrupted(_)) => RunState::Interrupted,
        None => RunState::Finished,
    };
    record.outcome = match record.state {
        RunState::Waiting { .. } => None,
        _ => Some(outcome),
    };
    record.detail = detail;
    record.source_removed = source_removed;
    if let Some(totals) = totals {
        record.files_copied = totals.files_copied;
        record.files_deleted = totals.files_deleted;
        record.files_failed = totals.files_failed;
        record.bytes_copied = totals.bytes_copied;
        // Each named failure as its report made it — review cr-jl3afix1-1:
        // by its own bytes when it has them, else the bytes the log knows
        // for its text — then each failed path no failure names.
        let mut failures: Vec<Failure> = totals
            .failures
            .iter()
            .map(|failure| Failure {
                path: failure.relative_path.clone(),
                reason: failure.reason.clone(),
                raw: failure
                    .raw_relative_path
                    .as_deref()
                    .map(blit_core::raw_name::escape_raw)
                    .or_else(|| raws.get(&failure.relative_path).cloned()),
            })
            .collect();
        for path in &totals.failed_paths {
            if !failures.iter().any(|failure| &failure.path == path) {
                failures.push(Failure {
                    path: path.clone(),
                    reason: String::new(),
                    raw: raws.get(path).cloned(),
                });
            }
        }
        record.failures = failures;
        record.failures_truncated =
            totals.failed_paths_truncated || (record.failures.len() as u64) < totals.files_failed;
    }
    if let Err(error) = recorded.store.update(&recorded.record) {
        eprintln!("blit: warning: could not finish this run's job record: {error}");
    }
    let (store, keep) = (recorded.store.clone(), recorded.keep);
    // Let go of the run before pruning, so it counts as ended.
    drop(recorded);
    if let Err(error) = store.prune(keep) {
        log::warn!(
            "job records: pruning {} failed: {error}",
            store.dir().display()
        );
    }
}

/// The run's job: what to run again. An error, in words, when the command
/// line cannot be kept exactly (a folder or list name that is not UTF-8, an
/// unreadable `--files-from` list — the command reports those itself).
fn job_spec(verb: &str, args: &TransferArgs, machine: &str) -> Result<JobSpec, String> {
    let cwd = std::env::current_dir().map_err(|error| format!("the current folder: {error}"))?;
    let cwd_text = cwd
        .to_str()
        .ok_or("the current folder's name is not UTF-8")?
        .to_string();
    let endpoint = |text: &str| -> Result<SpecEndpoint, String> {
        match parse_transfer_endpoint(text).map_err(|error| format!("{error:#}"))? {
            Endpoint::Remote(remote) => {
                let (module, path) = match &remote.path {
                    RemotePath::Module { module, rel_path } => (
                        Some(module.clone()),
                        rel_path.to_string_lossy().into_owned(),
                    ),
                    RemotePath::Root { rel_path } => {
                        (None, rel_path.to_string_lossy().into_owned())
                    }
                    RemotePath::Discovery => (None, String::new()),
                };
                Ok(SpecEndpoint::Remote {
                    locator: text.to_string(),
                    host: remote.host.clone(),
                    port: remote.port,
                    module,
                    path,
                })
            }
            Endpoint::Local(_) => job_record::absolute_local(&cwd, text)
                .map(|path| SpecEndpoint::Local { path })
                .ok_or_else(|| format!("{text}: its full name is not UTF-8")),
        }
    };
    let files_from = match &args.files_from {
        Some(list) => {
            let text = std::fs::read_to_string(cwd.join(list))
                .map_err(|error| format!("{}: {error}", list.display()))?;
            Some(text.lines().map(str::to_string).collect())
        }
        None => None,
    };
    Ok(JobSpec::new(
        machine,
        verb,
        cwd_text,
        endpoint(&args.source)?,
        endpoint(&args.destination)?,
        spec_options(args),
        files_from,
    ))
}

/// Every option of `args` that changes what the transfer does.
fn spec_options(args: &TransferArgs) -> SpecOptions {
    SpecOptions {
        dry_run: args.dry_run,
        checksum: args.checksum,
        size_only: args.size_only,
        ignore_times: args.ignore_times,
        ignore_existing: args.ignore_existing,
        force: args.force,
        delete_scope: args.delete_scope.clone(),
        resume: args.resume,
        drop_windows_metadata: args.drop_windows_metadata,
        retries: args.retries,
        retry_wait: args.retry_wait,
        retry: args.retry,
        wait: args.wait,
        exclude: args.exclude.clone(),
        include: args.include.clone(),
        min_size: args.min_size.clone(),
        max_size: args.max_size.clone(),
        min_age: args.min_age.clone(),
        max_age: args.max_age.clone(),
        force_grpc: args.force_grpc,
        detach: args.detach,
        null: args.null,
        yes: args.yes,
    }
}

fn now_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |since| {
            u64::try_from(since.as_millis()).unwrap_or(u64::MAX)
        })
}

/// This machine's log folder in the per-user folder `config_dir`, after
/// finishing any log an earlier, killed command left there (a running
/// command's log is left alone). Blocking.
pub fn logs_dir(config_dir: &std::path::Path) -> std::path::PathBuf {
    let logs = config_dir.join("jobs").join("logs");
    for problem in job_log::recover(&logs).problems {
        log::warn!("job log recovery: {problem}");
    }
    logs
}

/// The command's transfer options, in words, for its log's `run-start`.
fn option_words(args: &TransferArgs) -> Vec<String> {
    let mut words = Vec::new();
    let flags = [
        (args.dry_run, "dry-run"),
        (args.checksum, "checksum"),
        (args.size_only, "size-only"),
        (args.ignore_times, "ignore-times"),
        (args.ignore_existing, "ignore-existing"),
        (args.force, "force"),
        (args.resume, "resume"),
        (args.detach, "detach"),
        (args.null, "null"),
    ];
    words.extend(
        flags
            .iter()
            .filter(|(on, _)| *on)
            .map(|(_, word)| word.to_string()),
    );
    words.extend(args.include.iter().map(|glob| format!("include={glob}")));
    words.extend(args.exclude.iter().map(|glob| format!("exclude={glob}")));
    if args.retries > 0 {
        words.push(format!("retries={}", args.retries));
    }
    if args.retry > 0 {
        words.push(format!("retry={}", args.retry));
    }
    words
}

/// A local run's final account, for its log.
pub fn totals_from_local(summary: &blit_core::transfer_session::LocalMirrorSummary) -> RunTotals {
    RunTotals {
        files_copied: summary.copied_files as u64,
        files_deleted: (summary.deleted_files + summary.deleted_dirs) as u64,
        files_failed: summary.files_failed,
        bytes_copied: summary.total_bytes,
        files_resumed: 0,
        in_stream_carrier: None,
        failed_paths: summary.failed_paths.clone(),
        failed_paths_truncated: summary.failed_paths_truncated,
        failures: summary.failures.clone(),
    }
}

/// What a local run's `-v` shows about how its work was shaped, plus what
/// its scan could not read and what it repaired in place, for its log.
pub fn local_details(summary: &blit_core::transfer_session::LocalMirrorSummary) -> Vec<String> {
    use blit_core::display::format_bytes;
    let mut lines = vec![
        format!(
            "planned {} file(s), total bytes {}",
            summary.planned_files,
            format_bytes(summary.total_bytes)
        ),
        format!(
            "planner mix: {} tar shard(s) [{} file(s), {}], {} bundle(s) [{} file(s), {}], {} large task(s) [{}]",
            summary.tar_shard_tasks,
            summary.tar_shard_files,
            format_bytes(summary.tar_shard_bytes),
            summary.raw_bundle_tasks,
            summary.raw_bundle_files,
            format_bytes(summary.raw_bundle_bytes),
            summary.large_tasks,
            format_bytes(summary.large_bytes),
        ),
        format!(
            "workers used: {}",
            blit_core::transfer_session::DEFAULT_SINK_WORKERS
        ),
    ];
    if summary.files_repaired > 0 {
        lines.push(format!(
            "metadata repaired, content already in place: {} file(s)",
            summary.files_repaired
        ));
    }
    lines.extend(
        summary
            .unreadable_paths
            .iter()
            .map(|entry| format!("could not read during the scan: {entry}")),
    );
    lines
}

/// A remote-to-remote run's final account, as its destination daemon
/// reported it, for the initiator's log.
pub fn totals_from_delegated(summary: &blit_core::generated::DelegatedPullSummary) -> RunTotals {
    RunTotals {
        files_copied: summary.files_transferred,
        files_deleted: summary.entries_deleted,
        files_failed: summary.files_failed,
        bytes_copied: summary.bytes_transferred,
        files_resumed: 0,
        in_stream_carrier: Some(summary.tcp_fallback_used),
        failed_paths: summary.failed_paths.clone(),
        failed_paths_truncated: summary.failed_paths_truncated,
        failures: summary
            .failures
            .iter()
            .map(blit_core::remote::transfer::sink::FileFailure::from_wire)
            .collect(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Review cr-jl3a-3: a run's record names a failed non-UTF-8 file by
    /// its exact bytes as well as its text.
    #[test]
    fn a_record_keeps_a_failed_names_exact_bytes() {
        let dir = tempfile::tempdir().unwrap();
        let store = RunStore::new(dir.path());
        let run_id = job_log::new_run_id().unwrap();
        let spec = JobSpec::new(
            "m1",
            "copy",
            "/".into(),
            SpecEndpoint::Local { path: "/a/".into() },
            SpecEndpoint::Local { path: "/b/".into() },
            SpecOptions::default(),
            None,
        );
        let record = RunRecord::starting(&run_id, &spec);
        let live = store.begin(&spec, &record).unwrap();
        let recorded = Recorded {
            store: store.clone(),
            spec,
            record,
            keep: 50,
            _live: live,
        };
        let shown = "bad\u{FFFD}.txt".to_string();
        // Review cr-jl3afix1-1: two entries whose names collapse to one
        // text — the first failed on its own, the second as its duplicate,
        // with its own bytes.
        let totals = RunTotals {
            files_failed: 3,
            failed_paths: vec![shown.clone(), "plain.txt".into()],
            failures: vec![
                blit_core::remote::transfer::sink::FileFailure {
                    relative_path: shown.clone(),
                    reason: "denied".into(),
                    raw_relative_path: None,
                },
                blit_core::remote::transfer::sink::FileFailure {
                    relative_path: shown.clone(),
                    reason: "duplicate".into(),
                    raw_relative_path: Some(b"bad\xfe.txt".to_vec()),
                },
            ],
            ..RunTotals::default()
        };
        let raws = std::collections::HashMap::from([(shown.clone(), "bad\\xff.txt".to_string())]);
        finish_record(
            recorded,
            None,
            Outcome::Failed,
            None,
            Some(totals),
            &raws,
            false,
        );
        let failures = store.load(&run_id).unwrap().1.failures;
        assert_eq!(
            failures,
            [
                Failure {
                    path: shown.clone(),
                    reason: "denied".into(),
                    raw: Some("bad\\xff.txt".into()),
                },
                Failure {
                    path: shown,
                    reason: "duplicate".into(),
                    raw: Some(blit_core::raw_name::escape_raw(b"bad\xfe.txt")),
                },
                Failure {
                    path: "plain.txt".into(),
                    reason: String::new(),
                    raw: None,
                },
            ]
        );
    }

    /// Review cr-jl3a-3: the names a run's record keeps exactly are the
    /// failed ones its log knows the bytes of.
    #[tokio::test]
    async fn a_failed_names_bytes_come_from_the_runs_log() {
        let dir = tempfile::tempdir().unwrap();
        let log = RunLog::new(
            Some(LogPlace {
                dir: dir.path().to_path_buf(),
                participant: "m1".into(),
                keep: 50,
                finished: None,
            }),
            "0123456789abcdef0123456789abcdef",
        );
        log.start(Role::Initiator, RunInfo::default(), None);
        let progress =
            RemoteTransferProgress::audit_only(log.audit_lane_at(End::Receiving).unwrap());
        for name in ["bad\u{FFFD}.txt", "fine\u{FFFD}.txt"] {
            progress
                .report_raw_name(name.into(), name.replace('\u{FFFD}', "\x00").into_bytes())
                .await;
        }
        drop(progress);
        log.close(Outcome::Failed, None, None).await;
        let totals = RunTotals {
            failed_paths: vec!["bad\u{FFFD}.txt".into(), "plain.txt".into()],
            ..RunTotals::default()
        };
        let raws = failed_raw_names(&log, &totals);
        assert_eq!(
            raws.keys().map(String::as_str).collect::<Vec<_>>(),
            ["bad\u{FFFD}.txt"],
            "only failed names, only those the log knows"
        );
    }

    #[test]
    fn every_session_of_a_run_takes_the_next_number() {
        let run = CommandRun::unlogged();
        let pass = run.clone();
        assert_eq!(run.next_session().attempt, 1);
        assert_eq!(pass.next_session().attempt, 2);
        let tag = run.next_session();
        assert_eq!((tag.run_id.as_str(), tag.attempt), (run.run_id(), 3));
        assert!(job_log::valid_id(run.run_id()));
    }
}
