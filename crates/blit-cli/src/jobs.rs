use crate::cli::{
    JobsCancelArgs, JobsCommand, JobsDeleteArgs, JobsExportArgs, JobsListArgs, JobsLogArgs,
    JobsSaveArgs, JobsWatchArgs, LogRole,
};
use crate::run_log::RunOrigin;
use blit_core::admin::jobs;
use blit_core::admin::jobs::{CancelJobOutcome, WatchSnapshot};
use blit_core::generated::{daemon_event, DaemonState, JobLogHeader};
use blit_core::job_log::{self, LogLine, LogLines, Outcome, Role};
use blit_core::job_record::{
    self, DaemonAnswer, JobFile, JobSpec, RunRecord, RunState, RunStore, SavedJobs, StoredRun,
};
use blit_core::remote::endpoint::RemoteEndpoint;
use eyre::{Context, Result};
use std::collections::HashSet;
use std::io::{BufRead, Write};
use std::path::{Path, PathBuf};
use std::process::ExitCode;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

/// Return shape from [`run_jobs`]. `list` always exits with
/// success once the RPC returned cleanly; `cancel` carries
/// the per-outcome exit code mandated by the CLI contract
/// (`docs/plan/TUI_DESIGN.md` §6.5):
///
///   Cancelled  → 0
///   NotFound   → 1
///   Unsupported → 2
///
/// Same pattern as `run_check`: the verb owns the
/// `ExitCode`, `main` returns it.
pub async fn run_jobs(command: JobsCommand) -> Result<ExitCode> {
    match command {
        JobsCommand::List(args) => {
            run_jobs_list(args).await?;
            Ok(ExitCode::SUCCESS)
        }
        JobsCommand::Cancel(args) => run_jobs_cancel(args).await,
        JobsCommand::Watch(args) => run_jobs_watch(args).await,
        JobsCommand::Log(args) => {
            run_jobs_log(args).await?;
            Ok(ExitCode::SUCCESS)
        }
        JobsCommand::Save(args) => {
            run_jobs_save(args).await?;
            Ok(ExitCode::SUCCESS)
        }
        JobsCommand::Export(args) => {
            run_jobs_export(args).await?;
            Ok(ExitCode::SUCCESS)
        }
        JobsCommand::Delete(args) => {
            run_jobs_delete(args).await?;
            Ok(ExitCode::SUCCESS)
        }
        // A saved job, or a retry, runs as the transfer command it is
        // (`main`).
        JobsCommand::Run(_) | JobsCommand::Retry(_) => {
            eyre::bail!("`blit jobs run` and `retry` are dispatched with the transfer verbs")
        }
    }
}

/// This machine's run records and saved jobs.
fn local_stores() -> Result<(PathBuf, RunStore, SavedJobs)> {
    let config_dir = blit_core::config::config_dir()?;
    let runs = RunStore::new(crate::run_log::runs_dir(&config_dir));
    let saved = SavedJobs::new(crate::run_log::saved_dir(&config_dir));
    Ok((config_dir, runs, saved))
}

/// A run on this machine, its record settled — a `--detach` run asked of
/// its daemon first (plan "Detached jobs"). `None` when no run has that
/// ID here.
async fn local_run(runs: &RunStore, run_id: &str) -> Result<Option<(JobSpec, RunRecord)>> {
    if !job_log::valid_id(run_id) {
        return Ok(None);
    }
    let loaded = {
        let (runs, run_id) = (runs.clone(), run_id.to_string());
        tokio::task::spawn_blocking(move || runs.load(&run_id))
            .await
            .context("reading the job")?
    };
    let (spec, record) = match loaded {
        Ok(found) => found,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(error) => return Err(eyre::eyre!("reading job {run_id}: {error}")),
    };
    let (record, note) = settle_detached(runs, record).await;
    if let Some(note) = note {
        eprintln!("blit: job {run_id}: {note}");
    }
    Ok(Some((spec, record)))
}

/// `blit jobs save <job-id> <name>` (JOB_LOGS jl-3b): keep a run's job as
/// a saved job.
async fn run_jobs_save(args: JobsSaveArgs) -> Result<()> {
    let (_, runs, saved) = local_stores()?;
    let (spec, _) = local_run(&runs, &args.job_id)
        .await?
        .ok_or_else(|| no_such_run(&args.job_id))?;
    let name = args.name.clone();
    let replaced = tokio::task::spawn_blocking(move || saved.save(&name, &spec))
        .await
        .context("saving the job")??;
    let note = if replaced {
        " (replacing the job saved under that name before)"
    } else {
        ""
    };
    println!(
        "Saved job {} from run {}{note}; `blit jobs run {}` runs it again.",
        args.name, args.job_id, args.name
    );
    Ok(())
}

fn no_such_run(run_id: &str) -> eyre::Report {
    eyre::eyre!("no job {run_id} on this machine (`blit jobs list` shows the jobs kept here)")
}

/// `blit jobs export <job-id|name> <file>` (JOB_LOGS jl-3b): a run's job
/// and how it went, or a saved job, as a file of its own.
async fn run_jobs_export(args: JobsExportArgs) -> Result<()> {
    let (_, runs, saved) = local_stores()?;
    let job = match local_run(&runs, &args.job).await? {
        Some((spec, record)) => JobFile::new(record.saved_job.clone(), spec, Some(record)),
        None if job_record::valid_job_name(&args.job) => {
            let name = args.job.clone();
            tokio::task::spawn_blocking(move || saved.load(&name))
                .await
                .context("reading the saved job")?
                .map_err(|error| match error.kind() {
                    std::io::ErrorKind::NotFound => eyre::eyre!(
                        "no job or saved job named {} on this machine \
                         (`blit jobs list` shows both)",
                        args.job
                    ),
                    _ => eyre::eyre!("{error}"),
                })?
        }
        None => return Err(no_such_run(&args.job)),
    };
    let file = args.file.clone();
    tokio::task::spawn_blocking(move || job_record::write_document(&file, &job))
        .await
        .context("writing the job")?
        .with_context(|| format!("writing {}", args.file.display()))?;
    println!("Wrote job {} to {}.", args.job, args.file.display());
    Ok(())
}

/// `blit jobs delete <name>` (JOB_LOGS jl-3b).
async fn run_jobs_delete(args: JobsDeleteArgs) -> Result<()> {
    let (_, _, saved) = local_stores()?;
    let name = args.name.clone();
    tokio::task::spawn_blocking(move || saved.delete(&name))
        .await
        .context("deleting the job")??;
    println!("Deleted saved job {}.", args.name);
    Ok(())
}

/// A job to run again (`blit jobs run`): the transfer command it is, the
/// saved job's name, and its `--files-from` list written out for the run.
pub(crate) struct JobToRun {
    pub verb: String,
    pub args: crate::cli::TransferArgs,
    pub origin: RunOrigin,
    /// Removes the written list when the run is done.
    pub _list: Option<ListFile>,
}

/// A job's `--files-from` lines, written out for its run; removed on drop.
pub(crate) struct ListFile(PathBuf);

impl Drop for ListFile {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.0);
    }
}

/// `blit jobs run <name|file>` (JOB_LOGS jl-3b): a saved job, or a job file,
/// as the command to run — refused unless it belongs to this machine (R4).
/// A name that is a file is read as one.
pub(crate) async fn job_to_run(target: &str) -> Result<JobToRun> {
    let target = target.to_string();
    tokio::task::spawn_blocking(move || job_to_run_blocking(&target))
        .await
        .context("reading the job")?
}

fn job_to_run_blocking(target: &str) -> Result<JobToRun> {
    let (config_dir, _, saved) = local_stores()?;
    // Review cr-jl3b-1: a path is a job file; a bare word is a saved job,
    // whatever files the current folder holds.
    let job = if job_record::names_a_path(target) {
        let bytes = std::fs::read(target).with_context(|| format!("reading {target}"))?;
        job_record::read_job_file(&bytes).with_context(|| format!("reading {target}"))?
    } else if job_record::valid_job_name(target) {
        saved.load(target).map_err(|error| {
            let hint = if Path::new(target).is_file() {
                format!(" (to run the file {target}, give it as a path: ./{target})")
            } else {
                String::new()
            };
            eyre::eyre!("{error}{hint}")
        })?
    } else {
        eyre::bail!("{target} is not a saved job's name; give a job file as a path (./{target})");
    };
    check_machine(&config_dir, &job.spec)?;
    let (args, list) = spec_args(&config_dir, &job.spec)?;
    Ok(JobToRun {
        verb: job.spec.verb.clone(),
        args,
        origin: RunOrigin {
            saved_job: job.name.clone(),
            parent: None,
        },
        _list: list,
    })
}

/// `blit jobs retry <job-id|file>` (JOB_LOGS jl-4): a run's failed files
/// sent again, as a new run — that run's next attempt, its parent
/// recorded — on the machine the job belongs to. `None`, said, when the
/// run failed nothing. Refused while the run goes on (here, or on the
/// daemon a `--detach` run went on), and when its failures are not all
/// known by name.
///
/// A copy's or a mirror's retry is a copy of exactly the failed files,
/// through the retry passes' own `retry_only` set: a mirror's deletions
/// ran in its first run, and a mirror limited to a few files could delete
/// the rest under `--delete-scope all`. A move's retry is the move again:
/// its compare re-sends only what did not land, and its source is removed
/// only once everything has — sending only the failed files and then
/// removing the source would lose whatever changed there since.
pub(crate) async fn job_to_retry(target: &str) -> Result<Option<JobToRun>> {
    let (config_dir, runs, _) = local_stores()?;
    // Review cr-jl3b-1: a path is a job file, a bare word a run ID.
    let (spec, record) = if job_record::names_a_path(target) {
        let bytes = std::fs::read(target).with_context(|| format!("reading {target}"))?;
        let file =
            job_record::read_job_file(&bytes).with_context(|| format!("reading {target}"))?;
        let run = file.run.ok_or_else(|| {
            eyre::eyre!("{target} holds a job but no run of it to retry; `blit jobs run` runs it")
        })?;
        // A run in a file that waited on its daemon: ask it now.
        let run = match job_record::ask_daemon(&run).await {
            Ok(DaemonAnswer::Ended(ended)) => *ended,
            _ => run,
        };
        (file.spec, run)
    } else {
        local_run(&runs, target)
            .await?
            .ok_or_else(|| no_such_run(target))?
    };
    check_machine(&config_dir, &spec)?;
    match &record.state {
        RunState::Running => eyre::bail!(
            "job {} is still running; retry it once it has ended",
            record.run_id
        ),
        RunState::Waiting { daemon, job_id } => eyre::bail!(
            "job {} went on on {daemon} as job {job_id}, and how it ended is not known \
             yet; retry it once `blit jobs list` shows it ended",
            record.run_id
        ),
        RunState::Finished | RunState::Interrupted => {}
    }
    if record.files_failed == 0 && record.failures.is_empty() {
        println!("Job {} failed no files; nothing to retry.", record.run_id);
        return Ok(None);
    }
    let (mut args, list) = spec_args(&config_dir, &spec)?;
    let verb = if spec.verb == "move" {
        "move"
    } else {
        args.retry_only = Some(failed_paths(&config_dir, &record)?);
        "copy"
    };
    Ok(Some(JobToRun {
        verb: verb.to_string(),
        args,
        origin: RunOrigin {
            saved_job: record.saved_job.clone(),
            parent: Some((record.run_id.clone(), record.attempt)),
        },
        _list: list,
    }))
}

/// Every file a run failed, by name: its record's, completed from this
/// machine's log of the run when the record's list was cut short. An
/// error when they are still not all known — a retry would leave the rest.
fn failed_paths(config_dir: &Path, record: &RunRecord) -> Result<HashSet<PathBuf>> {
    use crate::transfers::retry::UNRETRIED_PATH;
    let mut known: HashSet<(String, Option<String>)> = record
        .failures
        .iter()
        .filter(|failure| failure.path != UNRETRIED_PATH)
        .map(|failure| (failure.path.clone(), failure.raw.clone()))
        .collect();
    let short = |known: &HashSet<(String, Option<String>)>| {
        record.failures_truncated
            || record
                .failures
                .iter()
                .any(|failure| failure.path == UNRETRIED_PATH)
            || (known.len() as u64) < record.files_failed
    };
    if short(&known) {
        // Review cr-jl4-3: the run's terminal state, read from its log in
        // order — a file that failed, and was not landed by a later pass —
        // not every failure the run ever met.
        let logs = config_dir.join("jobs").join("logs");
        for log in job_log::logs_for_run(&logs, &record.run_id, None).unwrap_or_default() {
            let Ok(lines) = job_log::open_log(&log.path) else {
                continue;
            };
            let mut still_failed: HashSet<(String, Option<String>)> = HashSet::new();
            for line in lines.flatten() {
                let LogLine::Event(event) = line else {
                    continue;
                };
                match event.body {
                    job_log::EventBody::FileFailed { path, raw, .. } => {
                        still_failed.insert((path, raw));
                    }
                    job_log::EventBody::FileCopied { path, raw, .. }
                    | job_log::EventBody::FileSent { path, raw } => {
                        still_failed.remove(&(path, raw));
                    }
                    _ => {}
                }
            }
            known.extend(still_failed);
        }
        if (known.len() as u64) < record.files_failed {
            eyre::bail!(
                "job {} names only {} of the {} files it failed, so a retry would leave the \
                 rest; run the whole job again instead (`blit jobs save {} <name>`, then \
                 `blit jobs run <name>`)",
                record.run_id,
                known.len(),
                record.files_failed,
                record.run_id
            );
        }
    }
    Ok(known
        .into_iter()
        .map(|(path, _)| PathBuf::from(path))
        .collect())
}

/// R4: a job runs only on the machine it was made on; refused, naming
/// that machine (by ID, and by host name when it has one), anywhere else.
fn check_machine(config_dir: &Path, spec: &JobSpec) -> Result<()> {
    let machine = job_log::machine_id(config_dir)
        .map_err(|error| eyre::eyre!("this machine's ID: {error}"))?;
    if spec.machine != machine {
        let host = if spec.host.is_empty() {
            String::new()
        } else {
            format!(" ({})", spec.host)
        };
        eyre::bail!(
            "this job belongs to machine {}{host}; a job runs only on the machine it was made on",
            spec.machine
        );
    }
    Ok(())
}

/// The command line a job's spec stands for, and its `--files-from` list
/// written out for the run (removed when the returned guard drops).
fn spec_args(
    config_dir: &Path,
    spec: &JobSpec,
) -> Result<(crate::cli::TransferArgs, Option<ListFile>)> {
    let list = match &spec.files_from {
        Some(lines) => {
            let dir = config_dir.join("jobs");
            std::fs::create_dir_all(&dir)?;
            let name = job_log::new_run_id().map_err(|error| eyre::eyre!("{error}"))?;
            let path = dir.join(format!("files-from-{name}.txt"));
            let mut text = lines.join("\n");
            text.push('\n');
            std::fs::write(&path, text).with_context(|| format!("writing {}", path.display()))?;
            Some(ListFile(path))
        }
        None => None,
    };
    let o = &spec.options;
    let args = crate::cli::TransferArgs {
        source: spec.source.as_arg().to_string(),
        destination: spec.destination.as_arg().to_string(),
        dry_run: o.dry_run,
        checksum: o.checksum,
        size_only: o.size_only,
        ignore_times: o.ignore_times,
        ignore_existing: o.ignore_existing,
        force: o.force,
        delete_scope: o.delete_scope.clone(),
        resume: o.resume,
        drop_windows_metadata: o.drop_windows_metadata,
        retries: o.retries,
        retry_wait: o.retry_wait,
        retry: o.retry,
        wait: o.wait,
        exclude: o.exclude.clone(),
        include: o.include.clone(),
        files_from: list.as_ref().map(|list| list.0.clone()),
        min_size: o.min_size.clone(),
        max_size: o.max_size.clone(),
        min_age: o.min_age.clone(),
        max_age: o.max_age.clone(),
        force_grpc: o.force_grpc,
        detach: o.detach,
        null: o.null,
        yes: o.yes,
        ..Default::default()
    };
    Ok((args, list))
}

/// `blit jobs log` (JOB_LOGS jl-1b, jl-2): each log kept for the job — by
/// the daemon named first, or on this machine — or the one log file named,
/// as text or, with `--json`, as its JSON lines.
async fn run_jobs_log(args: JobsLogArgs) -> Result<()> {
    let role = args.role.map(|role| match role {
        LogRole::Initiator => Role::Initiator,
        LogRole::Source => Role::Source,
        LogRole::Destination => Role::Destination,
    });
    let json = args.json;
    let Some(job_id) = args.job_id else {
        let target = args.target;
        let went_on = detached_ending(&target).await;
        let shown = {
            let target = target.clone();
            tokio::task::spawn_blocking(move || {
                let stdout = std::io::stdout();
                let mut out = stdout.lock();
                write_local_log(&mut out, &target, role, json)
            })
            .await
            .context("reading the log")?
        };
        if let Some(went_on) = went_on {
            eprintln!("blit: {went_on}");
        }
        return shown;
    };
    let remote = RemoteEndpoint::parse(&args.target)
        .with_context(|| format!("parsing remote endpoint '{}'", args.target))?;
    let mut first = true;
    jobs::read_job_logs(&remote, &job_id, role, false, move |header, lines| {
        let stdout = std::io::stdout();
        let mut out = stdout.lock();
        if json {
            write_json_log(&mut out, lines)?;
        } else {
            if !first {
                writeln!(out)?;
            }
            write_text_log(&mut out, &Heading::from_daemon(&header), lines)?;
        }
        first = false;
        out.flush()?;
        Ok(())
    })
    .await
}

/// For a job run here that went on on a daemon (`--detach`): how it ended
/// there, once the daemon is asked (its record is updated), or why that is
/// not known yet. `None` for any other job, or a file.
async fn detached_ending(target: &str) -> Option<String> {
    if job_record::names_a_path(target) || !job_log::valid_id(target) {
        return None;
    }
    let store = RunStore::new(crate::run_log::runs_dir(
        &blit_core::config::config_dir().ok()?,
    ));
    let loaded = {
        let (store, run_id) = (store.clone(), target.to_string());
        tokio::task::spawn_blocking(move || store.load(&run_id))
            .await
            .ok()?
            .ok()?
    };
    let RunState::Waiting { daemon, job_id } = loaded.1.state.clone() else {
        return None;
    };
    let (record, note) = settle_detached(&store, loaded.1).await;
    let there = format!("`blit jobs log {daemon} {job_id}` shows its log there");
    Some(match (note, record.outcome) {
        (Some(note), _) => format!("job {target} went on on {daemon} as job {job_id}: {note}"),
        (None, Some(outcome)) => {
            let detail = record
                .detail
                .map(|detail| format!(": {detail}"))
                .unwrap_or_default();
            format!(
                "job {target} went on on {daemon} as job {job_id} and ended {}{detail}; {there}",
                outcome.as_str()
            )
        }
        (None, None) => format!("job {target} went on on {daemon} as job {job_id}; {there}"),
    })
}

/// What `blit jobs log` prints above a log's text.
struct Heading {
    role: String,
    participant: String,
    attempt: u32,
    /// Why the log may not be finished, when it is not.
    unfinished: Option<&'static str>,
}

impl Heading {
    fn from_daemon(header: &JobLogHeader) -> Self {
        Self {
            role: header.role.clone(),
            participant: header.participant.clone(),
            attempt: header.attempt,
            unfinished: (!header.finished)
                .then_some("the job is running, or the daemon stopped during it"),
        }
    }

    fn from_disk(role: Role, participant: &str, attempt: u32, finished: bool) -> Self {
        Self {
            role: role.as_str().to_string(),
            participant: participant.to_string(),
            attempt,
            unfinished: (!finished).then_some("the job is running"),
        }
    }
}

/// `blit jobs log <job-id|file>` (jl-2): a job run on this machine — every
/// log kept for it here — or the one log file named. A name that is a file
/// is read as one; any other is a job ID.
fn write_local_log(
    out: &mut impl Write,
    target: &str,
    role: Option<Role>,
    json: bool,
) -> Result<()> {
    let file = Path::new(target);
    // Review cr-jl3b-1: a path is a file, a bare word a job ID.
    let logs: Vec<(PathBuf, Heading)> = if job_record::names_a_path(target) {
        if role.is_some() {
            eyre::bail!("--role picks among a job's logs; {target} is one log file");
        }
        if json {
            // Any file's lines as stored, a damaged log's too.
            write_json_log(out, open_stored(file)?)?;
            out.flush()?;
            return Ok(());
        }
        let start =
            job_log::read_start(file).with_context(|| format!("reading {}", file.display()))?;
        let heading = Heading::from_disk(
            start.role,
            &start.participant,
            start.attempt,
            !job_log::is_partial(file),
        );
        vec![(file.to_path_buf(), heading)]
    } else if job_log::valid_id(target) {
        let dir = crate::run_log::logs_dir(&blit_core::config::config_dir()?);
        let found = job_log::logs_for_run(&dir, target, role)?;
        if found.is_empty() {
            eyre::bail!(
                "no log for job {target} on this machine (`blit jobs list` shows the \
                 jobs kept here; for a daemon's job, name its host first: \
                 `blit jobs log <host> <job-id>`)"
            );
        }
        found
            .into_iter()
            .map(|log| {
                let heading = Heading::from_disk(
                    log.key.role(),
                    log.key.participant(),
                    log.key.attempt(),
                    log.finished,
                );
                (log.path, heading)
            })
            .collect()
    } else {
        eyre::bail!("{target} is not a log file or a job ID");
    };
    for (index, (path, heading)) in logs.iter().enumerate() {
        let lines = open_stored(path)?;
        if json {
            write_json_log(out, lines)?;
        } else {
            if index > 0 {
                writeln!(out)?;
            }
            write_text_log(out, heading, lines)?;
        }
    }
    out.flush()?;
    Ok(())
}

/// A log file's lines as stored.
fn open_stored(path: &Path) -> Result<Box<dyn BufRead + Send>> {
    std::fs::File::open(path)
        .and_then(job_log::decode)
        .with_context(|| format!("reading {}", path.display()))
}

fn write_text_log(
    out: &mut impl Write,
    heading: &Heading,
    lines: Box<dyn BufRead + Send>,
) -> Result<()> {
    let unfinished = heading
        .unfinished
        .map(|why| format!(" — not finished: {why}"))
        .unwrap_or_default();
    writeln!(
        out,
        "== {} log from machine {} (attempt {}){unfinished} ==",
        heading.role, heading.participant, heading.attempt
    )?;
    for line in LogLines::new(lines) {
        match line? {
            LogLine::Event(event) => writeln!(out, "{}", job_log::text_line(&event))?,
            LogLine::Unreadable { line, torn: true } => {
                writeln!(out, "(line {line} was cut short)")?
            }
            LogLine::Unreadable { line, torn: false } => {
                writeln!(out, "(line {line} is not a log event)")?
            }
        }
    }
    Ok(())
}

/// The log's lines as stored. A log cut short mid-line still ends with a
/// newline, so the next log starts on a line of its own.
fn write_json_log(out: &mut impl Write, mut lines: Box<dyn BufRead + Send>) -> Result<()> {
    let mut line = Vec::new();
    loop {
        line.clear();
        if lines.read_until(b'\n', &mut line)? == 0 {
            return Ok(());
        }
        out.write_all(&line)?;
        if !line.ends_with(b"\n") {
            out.write_all(b"\n")?;
        }
    }
}

async fn run_jobs_list(args: JobsListArgs) -> Result<()> {
    let Some(remote) = args.remote else {
        return local_jobs_list(args.recent_limit, args.json).await;
    };
    let remote = RemoteEndpoint::parse(&remote)
        .with_context(|| format!("parsing remote endpoint '{remote}'"))?;
    let state = jobs::query(&remote, args.recent_limit).await?;

    if args.json {
        print_json(&state)?;
    } else {
        print_human(&remote, &state);
    }
    Ok(())
}

/// `blit jobs list` with no host (jl-2, jl-3): the jobs run on this
/// machine, newest first — `limit` of them, or all when 0 — from their run
/// records, each run still going on a daemon (`--detach`) asked how it
/// ended first.
async fn local_jobs_list(limit: u32, json: bool) -> Result<()> {
    let config_dir = blit_core::config::config_dir()?;
    let store = RunStore::new(crate::run_log::runs_dir(&config_dir));
    let logs = config_dir.join("jobs").join("logs");
    let listed = {
        let store = store.clone();
        tokio::task::spawn_blocking(move || store.list())
            .await
            .context("listing the jobs")?
    }
    .with_context(|| format!("reading {}", store.dir().display()))?;
    let mut runs: Vec<(StoredRun, Option<String>)> = Vec::new();
    for run in listed.into_iter().take(if limit == 0 {
        usize::MAX
    } else {
        limit as usize
    }) {
        let (record, note) = settle_detached(&store, run.record.clone()).await;
        runs.push((StoredRun { record, ..run }, note));
    }
    let saved = SavedJobs::new(crate::run_log::saved_dir(&config_dir));
    tokio::task::spawn_blocking(move || {
        let saved_jobs = saved
            .list()
            .with_context(|| format!("reading {}", saved.dir().display()))?;
        let stdout = std::io::stdout();
        let mut out = stdout.lock();
        write_local_list(&mut out, &store, &logs, &runs, &saved, &saved_jobs, json)
    })
    .await
    .context("listing the jobs")?
}

/// A run's record once the daemon a `--detach` run went on has said how it
/// ended (plan "Detached jobs"): updated here, so the daemon is not asked
/// again. While it goes on, or when the daemon cannot be asked, the record
/// as it was, with a note saying so.
pub(crate) async fn settle_detached(
    store: &RunStore,
    record: RunRecord,
) -> (RunRecord, Option<String>) {
    let RunState::Waiting { daemon, .. } = &record.state else {
        return (record, None);
    };
    let daemon = daemon.clone();
    match job_record::ask_daemon(&record).await {
        Ok(DaemonAnswer::Ended(ended)) => {
            let ended = *ended;
            let saved = {
                let (store, ended) = (store.clone(), ended.clone());
                tokio::task::spawn_blocking(move || store.update(&ended)).await
            };
            match saved {
                Ok(Ok(())) => (ended, None),
                Ok(Err(error)) => (ended, Some(format!("could not update its record: {error}"))),
                Err(error) => (ended, Some(format!("could not update its record: {error}"))),
            }
        }
        Ok(DaemonAnswer::Going) => (record, Some(format!("still going on {daemon}"))),
        Err(error) => (record, Some(format!("could not ask {daemon}: {error:#}"))),
    }
}

fn write_local_list(
    out: &mut impl Write,
    store: &RunStore,
    logs: &Path,
    runs: &[(StoredRun, Option<String>)],
    saved: &SavedJobs,
    saved_jobs: &[(String, std::io::Result<JobFile>)],
    json: bool,
) -> Result<()> {
    let saved_rows = saved_jobs.iter().map(|(name, job)| match job {
        Ok(job) => serde_json::json!({
            "name": name,
            "verb": job.spec.verb,
            "source": job.spec.source.as_arg(),
            "destination": job.spec.destination.as_arg(),
            "file": saved.dir().join(format!("{name}.json")).display().to_string(),
        }),
        Err(error) => serde_json::json!({ "name": name, "error": error.to_string() }),
    });
    if json {
        let jobs: Vec<serde_json::Value> = runs
            .iter()
            .map(|(run, note)| {
                let mut row = serde_json::to_value(&run.record)?;
                row["spec_file"] = run.spec_path.display().to_string().into();
                row["record_file"] = run.record_path.display().to_string().into();
                if let Some(log) = initiator_log(logs, &run.record.run_id) {
                    row["log"] = log.display().to_string().into();
                }
                if let Some(note) = note {
                    row["note"] = note.clone().into();
                }
                Ok(row)
            })
            .collect::<Result<_, serde_json::Error>>()?;
        let listing = serde_json::json!({
            "runs_dir": store.dir().display().to_string(),
            "jobs": jobs,
            "saved_dir": saved.dir().display().to_string(),
            "saved": saved_rows.collect::<Vec<_>>(),
        });
        writeln!(out, "{}", serde_json::to_string_pretty(&listing)?)?;
    } else {
        if runs.is_empty() {
            writeln!(out, "Jobs on this machine: (none)")?;
        } else {
            writeln!(out, "Jobs on this machine ({}), newest first:", runs.len())?;
            for (run, note) in runs {
                write!(out, "  {}", local_row(&run.record))?;
                match note {
                    Some(note) => writeln!(out, " — {note}")?,
                    None => writeln!(out)?,
                }
            }
        }
        if !saved_jobs.is_empty() {
            writeln!(out)?;
            writeln!(out, "Saved jobs ({}):", saved_jobs.len())?;
            for (name, job) in saved_jobs {
                match job {
                    Ok(job) => writeln!(
                        out,
                        "  {name}  {}  {} -> {}",
                        job.spec.verb,
                        job.spec.source.as_arg(),
                        job.spec.destination.as_arg()
                    )?,
                    Err(error) => writeln!(out, "  {name}  (unreadable: {error})")?,
                }
            }
        }
    }
    out.flush()?;
    Ok(())
}

/// This machine's own log of the run, if it kept one.
fn initiator_log(logs: &Path, run_id: &str) -> Option<PathBuf> {
    job_log::logs_for_run(logs, run_id, Some(Role::Initiator))
        .ok()?
        .into_iter()
        .next()
        .map(|log| log.path)
}

/// One job's line in `blit jobs list` (no host): its ID, when it started,
/// what it did, and where it is or how it ended.
fn local_row(record: &RunRecord) -> String {
    let started = i64::try_from(record.started_ms)
        .ok()
        .and_then(chrono::DateTime::from_timestamp_millis)
        .map(|time| {
            time.with_timezone(&chrono::Local)
                .format("%Y-%m-%d %H:%M:%S")
                .to_string()
        })
        .unwrap_or_else(|| "-".into());
    let mut status = match &record.state {
        RunState::Running => "running".to_string(),
        RunState::Waiting { daemon, job_id } => format!("waiting on {daemon} (job {job_id})"),
        RunState::Interrupted | RunState::Finished => record
            .outcome
            .map_or(record.state.as_str(), Outcome::as_str)
            .to_string(),
    };
    if !matches!(record.state, RunState::Running | RunState::Waiting { .. }) {
        if let Some(detail) = &record.detail {
            status.push_str(&format!(": {detail}"));
        }
    }
    if record.state == RunState::Finished {
        status.push_str(&format!(
            " ({} copied, {} deleted, {} failed)",
            record.files_copied, record.files_deleted, record.files_failed
        ));
    }
    format!(
        "{}  {started}  {}  {} -> {}  {status}",
        record.run_id, record.verb, record.source, record.destination
    )
}

async fn run_jobs_cancel(args: JobsCancelArgs) -> Result<ExitCode> {
    let remote = RemoteEndpoint::parse(&args.remote)
        .with_context(|| format!("parsing remote endpoint '{}'", args.remote))?;
    let outcome = jobs::cancel(&remote, &args.transfer_id).await?;
    if args.json {
        print_cancel_json(&outcome);
    } else {
        print_cancel_human(&remote, &outcome);
    }
    Ok(cancel_exit_code(&outcome))
}

/// Map [`CancelJobOutcome`] to the contract's exit codes.
/// Pulled out as a sync helper so unit tests can pin the
/// mapping without spinning up a tonic server.
pub(crate) fn cancel_exit_code(outcome: &CancelJobOutcome) -> ExitCode {
    match outcome {
        CancelJobOutcome::Cancelled { .. } => ExitCode::SUCCESS,
        CancelJobOutcome::NotFound { .. } => ExitCode::from(1),
        CancelJobOutcome::Unsupported { .. } => ExitCode::from(2),
    }
}

/// Snapshot of the active row's metadata, captured by the
/// initial `GetState` before the streaming loop. Used to merge
/// the wire `TransferComplete` / `TransferError` event's
/// (sparse) fields back into the pre-existing
/// `WatchSnapshot::Finished` JSON schema so JSON-Lines
/// consumers see a stable terminal shape on both the
/// snapshot-finished and stream-finished paths.
struct ActiveSnapshot {
    kind: i32,
    peer: String,
    module: String,
    path: String,
    start_unix_ms: u64,
    bytes_completed: u64,
    files_completed: u64,
}

impl ActiveSnapshot {
    /// Merge with a `TransferComplete` to produce a
    /// `TransferRecord`-shaped value that matches the JSON
    /// schema emitted by `print_watch_json(Finished(...))`.
    fn to_finished_complete(
        &self,
        c: &blit_core::generated::TransferComplete,
    ) -> blit_core::generated::TransferRecord {
        blit_core::generated::TransferRecord {
            transfer_id: c.transfer_id.clone(),
            kind: self.kind,
            peer: self.peer.clone(),
            module: self.module.clone(),
            path: self.path.clone(),
            start_unix_ms: self.start_unix_ms,
            duration_ms: c.duration_ms,
            bytes: c.bytes,
            files: c.files,
            tcp_fallback_used: c.tcp_fallback_used,
            ok: true,
            error_message: String::new(),
            files_failed: c.files_failed,
            run_id: String::new(),
        }
    }

    /// Merge with a `TransferError` to produce the same shape.
    /// `duration_ms` derives from `start_unix_ms` since the
    /// event itself doesn't carry it; partial bytes/files come from the
    /// initial active snapshot.
    fn to_finished_error(
        &self,
        e: &blit_core::generated::TransferError,
    ) -> blit_core::generated::TransferRecord {
        let now_ms = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_millis() as u64)
            .unwrap_or(0);
        let duration_ms = now_ms.saturating_sub(self.start_unix_ms);
        blit_core::generated::TransferRecord {
            transfer_id: e.transfer_id.clone(),
            kind: self.kind,
            peer: self.peer.clone(),
            module: self.module.clone(),
            path: self.path.clone(),
            start_unix_ms: self.start_unix_ms,
            duration_ms,
            bytes: self.bytes_completed,
            files: self.files_completed,
            tcp_fallback_used: false,
            ok: false,
            error_message: e.message.clone(),
            files_failed: 0,
            run_id: String::new(),
        }
    }
}

/// Stream live progress for a single transfer until it
/// terminates or the optional timeout fires. Uses the c-2
/// `Subscribe` RPC scoped by c-5a's `transfer_id_filter` so
/// the CLI only receives events for the watched transfer.
///
/// Exit codes:
///
///   Finished + ok=true, no file failed → 0
///   Finished + ok=false, or files failed → 1 (the failed files are
///                              named from the job's log)
///   NotFound             → 2 (id never seen, or completed
///                              before subscribe + rotated out
///                              of the recent ring)
///   Timeout while active → 3 (deadline fired before any
///                              terminal event arrived)
///
/// Flow:
/// 1. Open the Subscribe stream FIRST — registers our
///    per-subscriber forwarder with the daemon so any terminal
///    event fired after this point lands in the mpsc and is
///    observable on the loop's first `message().await`.
/// 2. Query GetState. Three branches:
///    - Already in recent[] → drop stream, emit terminal,
///      return appropriate exit code.
///    - In active[]        → emit initial line, cache active
///      metadata, fall through to stream loop.
///    - NotFound           → emit not-found, return 2.
/// 3. Consume Subscribe stream events for the transfer:
///    - TransferProgress → update progress line / JSON.
///    - TransferComplete → emit terminal line, return 0 — or 1, naming
///      the failed files, when any failed.
///    - TransferError    → emit failed line, return 1.
///    - TransferStarted  → ignored (initial GetState already
///      reported state).
/// 4. Stream errors fall back to a final GetState query so a
///    Subscribe Lagged or daemon disconnect doesn't leave the
///    operator without a terminal answer.
async fn run_jobs_watch(args: JobsWatchArgs) -> Result<ExitCode> {
    let remote = RemoteEndpoint::parse(&args.remote)
        .with_context(|| format!("parsing remote endpoint '{}'", args.remote))?;
    if args.transfer_id.trim().is_empty() {
        eyre::bail!("transfer_id must not be empty");
    }
    let deadline = if args.timeout_secs > 0 {
        Some(Instant::now() + Duration::from_secs(args.timeout_secs))
    } else {
        None
    };

    if !args.json {
        eprintln!(
            "Watching transfer {} on {} (streaming)...",
            args.transfer_id,
            remote.display(),
        );
    }

    // c-6 round 2: subscribe FIRST so terminal events that
    // fire between the snapshot and our stream registration
    // land in the per-subscriber mpsc and are observable on
    // the loop's first `message().await`. The original
    // ordering (GetState first, Subscribe second) allowed a
    // race: transfer was Active at snapshot time, then drained
    // before Subscribe registered, terminal events broadcast
    // before our receiver existed, no replay (c-5b deferred),
    // and the stream hung forever waiting for a transfer_id
    // that's never going to fire again.
    // c-7: ask for replay_recent so any TransferProgress
    // events that fired between our snapshot and the next
    // tick land in the stream immediately instead of waiting
    // up to ~100ms. The replayed TransferStarted that comes
    // through is harmless — the loop's TransferStarted arm is
    // a no-op since the initial GetState already rendered the
    // active line.
    let mut stream = jobs::subscribe(&remote, &args.transfer_id, true).await?;

    // Step 1: GetState snapshot so we handle the already-
    // completed and never-existed cases (and the in-flight
    // case where we want to render an initial line + cache
    // metadata for terminal JSON merge).
    let state = jobs::query(&remote, 0).await?;
    let snap = jobs::watch_snapshot(&state, &args.transfer_id);
    let mut active_snapshot = match &snap {
        WatchSnapshot::Finished(r) => {
            if args.json {
                print_watch_json(&snap);
            } else {
                emit_human_finished(r);
                if r.ok && r.files_failed > 0 {
                    print_failed_files(&remote, &args.remote, &r.transfer_id, r.files_failed).await;
                }
            }
            return Ok(finished_exit(r.ok, r.files_failed));
        }
        WatchSnapshot::NotFound => {
            if args.json {
                print_watch_json(&snap);
            } else {
                eprintln!(
                    "[not-found] transfer '{}' is not on {} (already completed \
                     and rotated out of the recent ring, or never existed)",
                    args.transfer_id,
                    remote.display()
                );
            }
            return Ok(ExitCode::from(2));
        }
        WatchSnapshot::Active(a) => {
            if args.json {
                print_watch_json(&snap);
            } else {
                emit_human_active(a, None);
            }
            // c-6 round 2: cache the active row's metadata so
            // when a terminal event arrives over the stream we
            // can synthesize a `WatchSnapshot::Finished`-shaped
            // JSON object — same schema (kind, peer, module,
            // path, start_unix_ms, duration_ms, ok,
            // error_message) that the snapshot-finished path
            // emits. Subscribers iterating JSON-Lines see one
            // stable terminal shape regardless of which path
            // produced it.
            ActiveSnapshot {
                kind: a.kind,
                peer: a.peer.clone(),
                module: a.module.clone(),
                path: a.path.clone(),
                start_unix_ms: a.start_unix_ms,
                bytes_completed: a.bytes_completed,
                files_completed: a.files_completed,
            }
        }
    };

    loop {
        // tonic's `Streaming::message()` returns
        // `Result<Option<T>, Status>`:
        //   Ok(Some(msg))  → forward frame
        //   Ok(None)        → stream ended cleanly
        //   Err(status)     → stream error (Aborted = Lagged)
        let next_message = match deadline {
            Some(d) => {
                let remaining = d.saturating_duration_since(Instant::now());
                if remaining.is_zero() {
                    if args.json {
                        print_watch_timeout_json(&args.transfer_id, args.timeout_secs);
                    } else {
                        eprintln!(
                            "[timeout] transfer '{}' still active after {}s",
                            args.transfer_id, args.timeout_secs
                        );
                    }
                    return Ok(ExitCode::from(3));
                }
                match tokio::time::timeout(remaining, stream.message()).await {
                    Ok(item) => item,
                    Err(_) => {
                        if args.json {
                            print_watch_timeout_json(&args.transfer_id, args.timeout_secs);
                        } else {
                            eprintln!(
                                "[timeout] transfer '{}' still active after {}s",
                                args.transfer_id, args.timeout_secs
                            );
                        }
                        return Ok(ExitCode::from(3));
                    }
                }
            }
            None => stream.message().await,
        };
        match next_message {
            Ok(Some(event)) => match event.payload {
                Some(daemon_event::Payload::TransferProgress(p)) => {
                    active_snapshot.bytes_completed = p.bytes_completed;
                    active_snapshot.files_completed = p.files_completed;
                    if args.json {
                        print_watch_progress_json(&p);
                    } else {
                        emit_human_progress(&args.transfer_id, &p);
                    }
                }
                Some(daemon_event::Payload::TransferComplete(c)) => {
                    if args.json {
                        // Synthesize a Finished-shaped JSON
                        // object by merging the event's fields
                        // with the cached active snapshot —
                        // schema matches the GetState-finished
                        // path so JSON-Lines consumers see one
                        // stable terminal shape.
                        let merged = active_snapshot.to_finished_complete(&c);
                        print_watch_json(&WatchSnapshot::Finished(merged));
                    } else {
                        emit_human_complete(&c);
                        if c.files_failed > 0 {
                            print_failed_files(
                                &remote,
                                &args.remote,
                                &c.transfer_id,
                                c.files_failed,
                            )
                            .await;
                        }
                    }
                    return Ok(finished_exit(true, c.files_failed));
                }
                Some(daemon_event::Payload::TransferError(e)) => {
                    if args.json {
                        let merged = active_snapshot.to_finished_error(&e);
                        print_watch_json(&WatchSnapshot::Finished(merged));
                    } else {
                        eprintln!("blit: transfer '{}' failed: {}", e.transfer_id, e.message);
                    }
                    return Ok(ExitCode::from(1));
                }
                Some(daemon_event::Payload::TransferStarted(_)) | None => {
                    // Started already covered by the initial
                    // GetState. None happens for a future
                    // wire variant we don't recognize — drop.
                }
            },
            Err(status) => {
                // Stream error (typically Lagged → Aborted).
                // Fall back to a final GetState so the operator
                // gets a terminal answer rather than a stream
                // failure.
                eprintln!(
                    "blit: subscribe stream failed ({}); reconciling via GetState...",
                    status.message()
                );
                return reconcile_via_get_state(&args, &remote).await;
            }
            Ok(None) => {
                // Daemon closed the stream — likely shutting
                // down. Same fallback.
                eprintln!(
                    "blit: daemon closed the subscribe stream; \
                     reconciling via GetState..."
                );
                return reconcile_via_get_state(&args, &remote).await;
            }
        }
    }
}

/// On Subscribe stream error / end, query GetState once more
/// to decide the terminal exit. Mirrors the initial-snapshot
/// branches so the operator always gets a coherent answer.
async fn reconcile_via_get_state(
    args: &JobsWatchArgs,
    remote: &RemoteEndpoint,
) -> Result<ExitCode> {
    let state = jobs::query(remote, 0).await?;
    let snap = jobs::watch_snapshot(&state, &args.transfer_id);
    if args.json {
        print_watch_json(&snap);
    }
    match snap {
        WatchSnapshot::Finished(r) => {
            if !args.json {
                emit_human_finished(&r);
                if r.ok && r.files_failed > 0 {
                    print_failed_files(remote, &args.remote, &r.transfer_id, r.files_failed).await;
                }
            }
            Ok(finished_exit(r.ok, r.files_failed))
        }
        WatchSnapshot::Active(a) => {
            if !args.json {
                emit_human_active(&a, Some("still active after stream loss"));
            }
            // Stream is gone and the transfer is still active.
            // Without polling we can't follow it further; exit
            // 3 (timeout-equivalent: "we gave up watching").
            Ok(ExitCode::from(3))
        }
        WatchSnapshot::NotFound => {
            if !args.json {
                eprintln!(
                    "[not-found] transfer '{}' is no longer on {}",
                    args.transfer_id,
                    remote.display()
                );
            }
            Ok(ExitCode::from(2))
        }
    }
}

fn emit_human_active(a: &blit_core::generated::ActiveTransfer, note: Option<&str>) {
    let age_ms = age_ms_since(a.start_unix_ms);
    let progress = format!(
        "bytes={} files={}",
        format_progress_pair(a.bytes_completed, a.bytes_total),
        format_progress_pair(a.files_completed, a.files_total),
    );
    if let Some(note) = note {
        eprintln!(
            "[active] {} {} peer={} {} age={} ({})",
            jobs::kind_label(a.kind),
            module_path(&a.module, &a.path),
            a.peer,
            progress,
            format_ms(age_ms),
            note,
        );
    } else {
        eprintln!(
            "[active] {} {} peer={} {} age={}",
            jobs::kind_label(a.kind),
            module_path(&a.module, &a.path),
            a.peer,
            progress,
            format_ms(age_ms),
        );
    }
}

/// How a finished job ended, in words: a job whose files failed on their
/// own ran to its end but did not succeed (jl-1c).
fn finished_status(r: &blit_core::generated::TransferRecord) -> String {
    if !r.ok {
        format!("FAILED: {}", r.error_message)
    } else if r.files_failed > 0 {
        format!("FAILED: {} file(s) did not land", r.files_failed)
    } else {
        "ok".to_string()
    }
}

/// `jobs watch`'s exit for a finished job: 0 only when it ran to its end
/// with no file failed; 1 otherwise (jl-1c — before, a job whose files
/// failed one by one exited 0, defect (c) of 2026-10-07).
fn finished_exit(ok: bool, files_failed: u64) -> ExitCode {
    if ok && files_failed == 0 {
        ExitCode::SUCCESS
    } else {
        ExitCode::from(1)
    }
}

/// Name the files a finished job failed, from the job's log on the daemon
/// (jl-1c). The daemon closes the log a moment after the job's record, so
/// this waits briefly for the log to finish; at most [`FAILED_SHOWN`] are
/// listed, with a pointer to `blit jobs log` for the rest.
async fn print_failed_files(
    remote: &RemoteEndpoint,
    remote_arg: &str,
    transfer_id: &str,
    files_failed: u64,
) {
    eprintln!("blit: {files_failed} file(s) failed in transfer {transfer_id}:");
    // Review cr-jl1c-1: the daemon waits for the job's log to finish and
    // sends it once; nothing here re-reads it.
    let named = Arc::new(Mutex::new(FailedNames::default()));
    let sink = Arc::clone(&named);
    let read = jobs::read_job_logs(remote, transfer_id, None, true, move |header, lines| {
        let mut named = sink.lock().unwrap_or_else(|e| e.into_inner());
        named.unfinished |= !header.finished;
        for line in LogLines::new(lines) {
            if let LogLine::Event(job_log::Event {
                body: job_log::EventBody::FileFailed { path, reason, raw },
                ..
            }) = line?
            {
                named.add(path, raw, reason);
            }
        }
        Ok(())
    })
    .await;
    let named = std::mem::take(&mut *named.lock().unwrap_or_else(|e| e.into_inner()));
    for (path, reason) in &named.shown {
        eprintln!("  {path}: {reason}");
    }
    if let Err(error) = read {
        eprintln!("  (could not read the job's log: {error:#})");
        return;
    }
    let distinct = named.distinct.len() as u64;
    if distinct > named.shown.len() as u64 {
        eprintln!(
            "  ... and {} more; `blit jobs log {remote_arg} {transfer_id}` lists them all",
            distinct - named.shown.len() as u64
        );
    }
    if distinct != files_failed {
        let why = if named.unfinished {
            "; the log was still being written"
        } else {
            ""
        };
        eprintln!("  (the job's log names {distinct} of the {files_failed} failed file(s){why})");
    }
}

/// The failed files `jobs watch` names, de-duplicated by the file's identity
/// — its text and, for a name that is not valid UTF-8, its exact bytes —
/// never by how it reads (review cr-jlfix1-2).
#[derive(Default)]
struct FailedNames {
    shown: Vec<(String, String)>,
    distinct: std::collections::HashSet<(String, Option<String>)>,
    unfinished: bool,
}

impl FailedNames {
    fn add(&mut self, path: String, raw: Option<String>, reason: String) {
        let name = job_log::shown_name(&path, raw.as_deref());
        if self.distinct.insert((path, raw)) && self.shown.len() < FAILED_SHOWN {
            self.shown.push((name, reason));
        }
    }
}

/// How many failed files `jobs watch` names before pointing at `jobs log`.
const FAILED_SHOWN: usize = 20;

fn emit_human_finished(r: &blit_core::generated::TransferRecord) {
    let status = finished_status(r);
    eprintln!(
        "[done] {} {} bytes={} files={} carrier={} duration={} {}",
        jobs::kind_label(r.kind),
        module_path(&r.module, &r.path),
        r.bytes,
        r.files,
        if !r.ok {
            "-"
        } else if r.tcp_fallback_used {
            "gRPC"
        } else {
            "TCP"
        },
        format_ms(r.duration_ms),
        status,
    );
}

fn emit_human_progress(transfer_id: &str, p: &blit_core::generated::TransferProgress) {
    eprintln!("{}", human_progress_line(transfer_id, p));
}

fn human_progress_line(transfer_id: &str, p: &blit_core::generated::TransferProgress) -> String {
    let bps = p.throughput_bps;
    format!(
        "[progress] {} bytes={} files={} throughput={}",
        transfer_id,
        format_progress_pair(p.bytes_completed, p.bytes_total),
        format_progress_pair(p.files_completed, p.files_total),
        blit_core::display::format_bps(bps),
    )
}

fn format_progress_pair(completed: u64, total: u64) -> String {
    if total == 0 {
        format!("{completed}/?")
    } else {
        format!("{completed}/{total}")
    }
}

fn emit_human_complete(c: &blit_core::generated::TransferComplete) {
    let status = if c.files_failed > 0 {
        format!("FAILED: {} file(s) did not land", c.files_failed)
    } else {
        "ok".to_string()
    };
    eprintln!(
        "[done] transfer {} bytes={} files={} carrier={} duration={} {status}",
        c.transfer_id,
        c.bytes,
        c.files,
        if c.tcp_fallback_used { "gRPC" } else { "TCP" },
        format_ms(c.duration_ms),
    );
}

fn print_watch_progress_json(p: &blit_core::generated::TransferProgress) {
    let body = watch_progress_json(p);
    if let Ok(line) = serde_json::to_string(&body) {
        println!("{}", line);
    }
}

fn watch_progress_json(p: &blit_core::generated::TransferProgress) -> serde_json::Value {
    serde_json::json!({
        "state": "progress",
        "transfer_id": p.transfer_id,
        "bytes_completed": p.bytes_completed,
        "bytes_total": p.bytes_total,
        "files_completed": p.files_completed,
        "files_total": p.files_total,
        "throughput_bps": p.throughput_bps,
    })
}

// c-6 round 2: the standalone `print_watch_complete_json` /
// `print_watch_error_json` emitters were replaced by merging
// the event into a `WatchSnapshot::Finished` via
// `ActiveSnapshot::into_finished_*`, then routing through the
// existing `print_watch_json`. That keeps the terminal JSON
// schema identical regardless of which path produced it.

fn print_watch_json(snap: &WatchSnapshot) {
    let body = watch_json(snap);
    // JSON-Lines: one object per poll, no trailing newline
    // from to_string (println! adds it).
    if let Ok(line) = serde_json::to_string(&body) {
        println!("{}", line);
    }
}

fn watch_json(snap: &WatchSnapshot) -> serde_json::Value {
    use serde_json::json;
    match snap {
        WatchSnapshot::Active(a) => json!({
            "state": "active",
            "transfer_id": a.transfer_id,
            "kind": jobs::kind_label(a.kind),
            "peer": a.peer,
            "module": a.module,
            "path": a.path,
            "start_unix_ms": a.start_unix_ms,
            "bytes_completed": a.bytes_completed,
            "bytes_total": a.bytes_total,
            "files_completed": a.files_completed,
            "files_total": a.files_total,
        }),
        WatchSnapshot::Finished(r) => json!({
            "state": "finished",
            "transfer_id": r.transfer_id,
            "kind": jobs::kind_label(r.kind),
            "peer": r.peer,
            "module": r.module,
            "path": r.path,
            "start_unix_ms": r.start_unix_ms,
            "duration_ms": r.duration_ms,
            "bytes": r.bytes,
            "files": r.files,
            "tcp_fallback_used": r.tcp_fallback_used,
            "ok": r.ok,
            "error_message": r.error_message,
            "files_failed": r.files_failed,
        }),
        WatchSnapshot::NotFound => json!({
            "state": "not_found",
        }),
    }
}

/// Emit the terminal `state: "timeout"` line when --timeout-secs
/// fires while the transfer is still in active[]. JSON consumers
/// rely on the stream having a terminal state line — exit code 3
/// is for shells; the JSON object is for the same stream that's
/// been seeing `state: "active"` rows.
fn print_watch_timeout_json(transfer_id: &str, timeout_secs: u64) {
    use serde_json::json;
    let body = json!({
        "state": "timeout",
        "transfer_id": transfer_id,
        "timeout_secs": timeout_secs,
    });
    if let Ok(line) = serde_json::to_string(&body) {
        println!("{}", line);
    }
}

fn print_cancel_json(outcome: &CancelJobOutcome) {
    use serde_json::json;
    let body = match outcome {
        CancelJobOutcome::Cancelled { transfer_id } => json!({
            "outcome": "cancelled",
            "transfer_id": transfer_id,
        }),
        CancelJobOutcome::NotFound { transfer_id } => json!({
            "outcome": "not_found",
            "transfer_id": transfer_id,
        }),
        CancelJobOutcome::Unsupported {
            transfer_id,
            message,
        } => json!({
            "outcome": "unsupported",
            "transfer_id": transfer_id,
            "message": message,
        }),
    };
    println!(
        "{}",
        serde_json::to_string_pretty(&body).unwrap_or_default()
    );
}

fn print_cancel_human(remote: &RemoteEndpoint, outcome: &CancelJobOutcome) {
    match outcome {
        CancelJobOutcome::Cancelled { transfer_id } => {
            println!("Cancelled transfer {transfer_id} on {}", remote.display());
        }
        CancelJobOutcome::NotFound { transfer_id } => {
            eprintln!(
                "No active transfer with id '{transfer_id}' on {}",
                remote.display()
            );
        }
        CancelJobOutcome::Unsupported {
            transfer_id,
            message,
        } => {
            eprintln!("blit: cannot cancel transfer '{transfer_id}': {message}");
        }
    }
}

fn print_json(state: &DaemonState) -> Result<()> {
    use serde_json::json;
    let active: Vec<_> = state
        .active
        .iter()
        .map(|a| {
            json!({
                "transfer_id": a.transfer_id,
                "kind": jobs::kind_label(a.kind),
                "peer": a.peer,
                "module": a.module,
                "path": a.path,
                "start_unix_ms": a.start_unix_ms,
                "bytes_completed": a.bytes_completed,
                "bytes_total": a.bytes_total,
                "files_completed": a.files_completed,
                "files_total": a.files_total,
            })
        })
        .collect();
    let recent: Vec<_> = state
        .recent
        .iter()
        .map(|r| {
            json!({
                "transfer_id": r.transfer_id,
                "kind": jobs::kind_label(r.kind),
                "peer": r.peer,
                "module": r.module,
                "path": r.path,
                "start_unix_ms": r.start_unix_ms,
                "duration_ms": r.duration_ms,
                "bytes": r.bytes,
                "files": r.files,
                "tcp_fallback_used": r.tcp_fallback_used,
                "ok": r.ok,
                "error_message": r.error_message,
                "files_failed": r.files_failed,
                "run_id": r.run_id,
            })
        })
        .collect();
    let counters = state.counters.as_ref().map(|c| {
        json!({
            "push_operations_total": c.push_operations_total,
            "pull_operations_total": c.pull_operations_total,
            "purge_operations_total": c.purge_operations_total,
            "active_transfers": c.active_transfers,
            "transfer_errors_total": c.transfer_errors_total,
        })
    });
    let modules: Vec<_> = state
        .modules
        .iter()
        .map(|m| {
            json!({
                "name": m.name,
                "path": m.path,
                "read_only": m.read_only,
            })
        })
        .collect();
    let body = json!({
        "version": state.version,
        "uptime_seconds": state.uptime_seconds,
        "delegation_enabled": state.delegation_enabled,
        "modules": modules,
        "active": active,
        "recent": recent,
        "counters": counters,
    });
    println!("{}", serde_json::to_string_pretty(&body)?);
    Ok(())
}

fn print_human(remote: &RemoteEndpoint, state: &DaemonState) {
    println!(
        "Daemon: blit {} on {} — uptime {}",
        state.version,
        remote.display(),
        format_uptime(state.uptime_seconds),
    );
    println!(
        "Delegation: {}",
        if state.delegation_enabled {
            "enabled"
        } else {
            "disabled"
        }
    );
    if state.modules.is_empty() {
        println!("Modules: (none)");
    } else {
        let names: Vec<&str> = state.modules.iter().map(|m| m.name.as_str()).collect();
        println!("Modules: {}", names.join(", "));
    }

    println!();
    if state.active.is_empty() {
        println!("Active: (none)");
    } else {
        println!("Active ({}):", state.active.len());
        for a in &state.active {
            // `<id> <kind> <module>/<path> peer=<peer> bytes=N/M files=N/M age=<ms>`
            let age_ms = age_ms_since(a.start_unix_ms);
            println!(
                "  {}  {}  {}  peer={}  bytes={}  files={}  age={}",
                a.transfer_id,
                jobs::kind_label(a.kind),
                module_path(&a.module, &a.path),
                a.peer,
                format_progress_pair(a.bytes_completed, a.bytes_total),
                format_progress_pair(a.files_completed, a.files_total),
                format_ms(age_ms),
            );
        }
    }

    println!();
    if state.recent.is_empty() {
        println!("Recent: (none)");
    } else {
        // Display newest-first for human eyes — the wire is
        // oldest-first, so iterate in reverse.
        println!("Recent ({}):", state.recent.len());
        for r in state.recent.iter().rev() {
            let status = finished_status(r);
            println!(
                "  {}  {}  {}  peer={}  duration={}  {}",
                r.transfer_id,
                jobs::kind_label(r.kind),
                module_path(&r.module, &r.path),
                r.peer,
                format_ms(r.duration_ms),
                status,
            );
        }
    }

    if let Some(c) = &state.counters {
        println!();
        println!(
            "Counters: push={} pull={} purge={} active={} errors={}",
            c.push_operations_total,
            c.pull_operations_total,
            c.purge_operations_total,
            c.active_transfers,
            c.transfer_errors_total,
        );
    }
}

fn module_path(module: &str, path: &str) -> String {
    match (module.is_empty(), path.is_empty()) {
        (true, true) => "/".to_string(),
        (true, false) => path.to_string(),
        (false, true) => module.to_string(),
        (false, false) => format!("{module}/{path}"),
    }
}

fn format_uptime(seconds: u64) -> String {
    let h = seconds / 3600;
    let m = (seconds % 3600) / 60;
    let s = seconds % 60;
    if h > 0 {
        format!("{h}h {m}m")
    } else if m > 0 {
        format!("{m}m {s}s")
    } else {
        format!("{s}s")
    }
}

fn format_ms(ms: u64) -> String {
    if ms >= 1000 {
        format!("{:.1}s", ms as f64 / 1000.0)
    } else {
        format!("{ms}ms")
    }
}

fn age_ms_since(start_unix_ms: u64) -> u64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    let now_ms = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0);
    now_ms.saturating_sub(start_unix_ms)
}

#[cfg(test)]
mod tests {

    /// Review cr-jlfix1-2: a raw-byte name and a UTF-8 name that reads like
    /// its escape are two files, counted and shown apart; the same file
    /// named twice counts once.
    #[test]
    fn failed_names_are_told_apart_by_identity() {
        let mut named = FailedNames::default();
        named.add("caf\u{fffd}".into(), Some("caf\\xe9".into()), "a".into());
        named.add("caf\\xe9".into(), None, "b".into());
        // A UTF-8 name that reads exactly like the raw one's label.
        named.add("raw:caf\\xe9".into(), None, "c".into());
        named.add(
            "caf\u{fffd}".into(),
            Some("caf\\xe9".into()),
            "a again".into(),
        );
        assert_eq!(named.distinct.len(), 3);
        assert_eq!(
            named.shown,
            [
                ("raw:caf\\xe9".to_string(), "a".to_string()),
                ("caf\\xe9".to_string(), "b".to_string()),
                ("utf8:raw:caf\\xe9".to_string(), "c".to_string()),
            ]
        );
    }

    use super::*;

    #[test]
    fn format_uptime_renders_hours_minutes_seconds() {
        assert_eq!(format_uptime(0), "0s");
        assert_eq!(format_uptime(45), "45s");
        assert_eq!(format_uptime(125), "2m 5s");
        assert_eq!(format_uptime(3661), "1h 1m");
    }

    #[test]
    fn format_ms_switches_to_seconds_above_1k() {
        assert_eq!(format_ms(0), "0ms");
        assert_eq!(format_ms(999), "999ms");
        assert_eq!(format_ms(1000), "1.0s");
        assert_eq!(format_ms(3500), "3.5s");
    }

    #[test]
    fn module_path_handles_each_empty_combination() {
        assert_eq!(module_path("", ""), "/");
        assert_eq!(module_path("", "p"), "p");
        assert_eq!(module_path("mod", ""), "mod");
        assert_eq!(module_path("mod", "sub/dir"), "mod/sub/dir");
    }

    #[test]
    fn progress_json_preserves_byte_and_file_denominators() {
        let value = watch_progress_json(&blit_core::generated::TransferProgress {
            transfer_id: "t1".into(),
            bytes_completed: 4096,
            bytes_total: 8192,
            files_completed: 2,
            files_total: 4,
            throughput_bps: 1024,
        });
        assert_eq!(value["bytes_completed"], 4096);
        assert_eq!(value["bytes_total"], 8192);
        assert_eq!(value["files_completed"], 2);
        assert_eq!(value["files_total"], 4);
        assert_eq!(
            human_progress_line(
                "t1",
                &blit_core::generated::TransferProgress {
                    transfer_id: "t1".into(),
                    bytes_completed: 4096,
                    bytes_total: 8192,
                    files_completed: 2,
                    files_total: 4,
                    throughput_bps: 1024,
                },
            ),
            "[progress] t1 bytes=4096/8192 files=2/4 throughput=1.00 KiB/s"
        );
    }

    /// `ExitCode` doesn't implement `PartialEq`, so we compare
    /// via the `Debug` repr — stable across releases of std
    /// and good enough to pin the contract.
    fn exit_code_repr(c: ExitCode) -> String {
        format!("{:?}", c)
    }

    fn sample_active_snapshot() -> ActiveSnapshot {
        ActiveSnapshot {
            kind: blit_core::generated::TransferKind::DelegatedPull as i32,
            peer: "10.0.0.5:443".to_string(),
            module: "mod-a".to_string(),
            path: "sub/dir".to_string(),
            start_unix_ms: 1_700_000_000_000,
            bytes_completed: 512,
            files_completed: 2,
        }
    }

    /// c-6 round 2 regression: terminal JSON shape on the
    /// stream-complete path must match the GetState-finished
    /// path. Pre-fix the stream path emitted only
    /// (state, transfer_id, bytes, files, duration_ms,
    /// tcp_fallback_used, ok), missing kind/peer/module/path/
    /// start_unix_ms that the snapshot path provides. Merging
    /// with the cached ActiveSnapshot restores parity.
    #[test]
    fn active_snapshot_to_finished_complete_carries_all_finished_fields() {
        let snap = sample_active_snapshot();
        let complete = blit_core::generated::TransferComplete {
            transfer_id: "t1-7".to_string(),
            bytes: 1024,
            files: 4,
            duration_ms: 1200,
            tcp_fallback_used: true,
            files_failed: 2,
        };
        let merged = snap.to_finished_complete(&complete);
        assert_eq!(merged.files_failed, 2);
        assert_eq!(merged.transfer_id, "t1-7");
        assert_eq!(merged.kind, snap.kind);
        assert_eq!(merged.peer, snap.peer);
        assert_eq!(merged.module, snap.module);
        assert_eq!(merged.path, snap.path);
        assert_eq!(merged.start_unix_ms, snap.start_unix_ms);
        assert_eq!(merged.duration_ms, 1200);
        assert_eq!(merged.bytes, 1024);
        assert_eq!(merged.files, 4);
        assert!(merged.tcp_fallback_used);
        assert!(merged.ok);
        assert!(merged.error_message.is_empty());
        let json = watch_json(&WatchSnapshot::Finished(merged));
        assert_eq!(json["bytes"], 1024);
        assert_eq!(json["files"], 4);
        assert_eq!(json["tcp_fallback_used"], true);
    }

    /// Same parity check on the stream-error path.
    #[test]
    fn active_snapshot_to_finished_error_carries_all_finished_fields() {
        let snap = sample_active_snapshot();
        let err = blit_core::generated::TransferError {
            transfer_id: "t1-7".to_string(),
            message: "module not found".to_string(),
        };
        let merged = snap.to_finished_error(&err);
        assert_eq!(merged.transfer_id, "t1-7");
        assert_eq!(merged.kind, snap.kind);
        assert_eq!(merged.peer, snap.peer);
        assert_eq!(merged.module, snap.module);
        assert_eq!(merged.path, snap.path);
        assert_eq!(merged.start_unix_ms, snap.start_unix_ms);
        // duration_ms is computed from now - start; just
        // sanity-check it's non-negative (saturating sub).
        let _ = merged.duration_ms;
        assert_eq!(merged.bytes, 512);
        assert_eq!(merged.files, 2);
        assert!(!merged.ok);
        assert_eq!(merged.error_message, "module not found");
    }

    #[test]
    fn cancel_exit_code_maps_each_outcome_to_the_contract_code() {
        let cancelled = CancelJobOutcome::Cancelled {
            transfer_id: "t1".to_string(),
        };
        let not_found = CancelJobOutcome::NotFound {
            transfer_id: "t2".to_string(),
        };
        let unsupported = CancelJobOutcome::Unsupported {
            transfer_id: "t3".to_string(),
            message: "kind not cancellable".to_string(),
        };

        assert_eq!(
            exit_code_repr(cancel_exit_code(&cancelled)),
            exit_code_repr(ExitCode::SUCCESS),
            "Cancelled must exit 0",
        );
        assert_eq!(
            exit_code_repr(cancel_exit_code(&not_found)),
            exit_code_repr(ExitCode::from(1)),
            "NotFound must exit 1",
        );
        assert_eq!(
            exit_code_repr(cancel_exit_code(&unsupported)),
            exit_code_repr(ExitCode::from(2)),
            "Unsupported must exit 2",
        );
    }
}
