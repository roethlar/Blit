//! This command's run (JOB_LOGS jl-2): the run ID made where the command is
//! typed and sent with every session it opens, so every machine involved
//! logs the run under one ID; the numbering of those sessions; and the
//! command's own log — the initiator's — in the per-user folder.

use crate::cli::TransferArgs;
use blit_core::config;
use blit_core::job_log::{self, EventBody, Outcome, Role, RunInfo, RunTag};
use blit_core::remote::transfer::RemoteTransferProgress;
use blit_core::run_log::{End, LogPlace, RunLog, RunTotals};
use std::process::ExitCode;
use std::sync::atomic::{AtomicU32, Ordering};
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
    ending: Mutex<Option<(Outcome, String)>>,
}

impl std::fmt::Debug for CommandRun {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "CommandRun({})", self.inner.run_id)
    }
}

impl CommandRun {
    /// Start this command's run and its log in the per-user folder, after
    /// finishing any log an earlier, killed command left there. `None` only
    /// when the system cannot make a random ID; a log that cannot be kept
    /// is skipped with a warning — a log never stops a command.
    pub async fn start(verb: &str, args: &TransferArgs) -> Option<Self> {
        let run_id = job_log::new_run_id().ok()?;
        let place = tokio::task::spawn_blocking(cli_log_place)
            .await
            .ok()
            .flatten();
        let log = RunLog::new(place, &run_id);
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
        if args.verbose && !args.json {
            eprintln!("blit: job {run_id} (`blit jobs log {run_id}` shows its log)");
        }
        Some(Self {
            inner: Arc::new(Inner {
                run_id,
                sessions: AtomicU32::new(0),
                log,
                ending: Mutex::new(None),
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
            }),
        }
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
        self.inner.log.note_totals(totals);
    }

    /// The run went on, on a daemon, after this command returns
    /// (`--detach`).
    pub fn note_detached(&self, detail: String) {
        *self.ending() = Some((Outcome::Detached, detail));
    }

    /// The run stopped on Ctrl-C.
    pub fn note_interrupted(&self, detail: String) {
        *self.ending() = Some((Outcome::Interrupted, detail));
    }

    fn ending(&self) -> std::sync::MutexGuard<'_, Option<(Outcome, String)>> {
        self.inner
            .ending
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
    }

    /// Close the run's log with how the command ended.
    pub async fn finish(&self, result: &eyre::Result<ExitCode>) {
        let (outcome, detail) = match (self.ending().take(), result) {
            (Some((outcome, detail)), _) => (outcome, Some(detail)),
            (None, Err(error)) => (Outcome::Failed, Some(format!("{error:#}"))),
            (None, Ok(_)) => match self.inner.log.noted_files_failed() {
                0 => (Outcome::Ok, None),
                failed => (Outcome::Failed, Some(format!("{failed} file(s) failed"))),
            },
        };
        if let Some(report) = self.inner.log.close(outcome, detail, None).await {
            if !report.complete {
                eprintln!(
                    "blit: this run's log is incomplete: {}",
                    report.problem.unwrap_or_default()
                );
            }
        }
    }
}

/// Where this machine's per-user logs live: `<per-user folder>/jobs/logs`,
/// kept to `[jobs] keep` from the per-user `config.toml`. Finishes any log
/// an earlier, killed command left there. `None`, with a warning, when the
/// folder or the machine ID cannot be had. Blocking.
fn cli_log_place() -> Option<LogPlace> {
    let warn = |what: String| eprintln!("blit: warning: this run is not logged: {what}");
    let dir = match config::config_dir() {
        Ok(dir) => dir,
        Err(error) => {
            warn(format!("{error:#}"));
            return None;
        }
    };
    let settings = config::load_settings(&dir).unwrap_or_else(|error| {
        eprintln!("blit: warning: {error:#}; using the default settings");
        config::Settings::default()
    });
    let participant = match job_log::machine_id(&dir) {
        Ok(id) => id,
        Err(error) => {
            warn(format!("{error}"));
            return None;
        }
    };
    Some(LogPlace {
        dir: logs_dir(&dir),
        participant,
        keep: settings.jobs_keep,
        finished: None,
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
        failures: summary
            .failures
            .iter()
            .map(|failure| (failure.relative_path.clone(), failure.reason.clone()))
            .collect(),
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
            .map(|failure| (failure.relative_path.clone(), failure.reason.clone()))
            .collect(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
