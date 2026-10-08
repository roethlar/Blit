mod check;
mod cli;
mod completions;
mod context;
mod df;
mod diagnostics;
mod du;
mod find;
mod jobs;
mod list_modules;
mod ls;
mod profile;
mod rm;
mod run_log;
mod scan;
mod style;
mod transfers;

use crate::check::run_check;
use crate::cli::{Cli, Commands, DiagnosticsCommand, JobsCommand, TransferArgs};
use crate::context::AppContext;
use crate::diagnostics::{run_diagnostics_dump, run_diagnostics_perf};
use crate::jobs::run_jobs;
use crate::transfers::{run_move, run_transfer};
use blit_core::config;
use blit_core::remote::transfer::{
    outcome_for_report, TransferLifecycleOutcome, TransferLifecycleTrace,
};
use blit_core::transfers::dispatch::TransferKind;
use blit_core::transfers::retry::run_with_retries;
use clap::Parser;
use eyre::Result;
use std::future::Future;
use std::pin::Pin;
use std::process::ExitCode;
use std::time::Duration;

#[tokio::main]
async fn main() -> Result<ExitCode> {
    let lifecycle_trace = TransferLifecycleTrace::from_env();
    lifecycle_trace.record("async_main_enter", None);
    // win-1: the command future lives on the heap, not in the main
    // thread's frames. `#[tokio::main]` polls this body on the process's
    // MAIN thread, whose stack is 1 MiB on Windows, and a debug build's
    // poll frames hold every nested future inline — the destination
    // routes overflowed it (see `docs/plan/SOURCE_SIDE_CONTAINMENT.md`,
    // "Windows fixes"). `tests/main_thread_stack_budget.rs` holds the
    // budget on every platform.
    let result = Box::pin(run_cli(&lifecycle_trace)).await;
    finish_command_lifecycle(&lifecycle_trace, result).await
}

async fn run_cli(lifecycle_trace: &TransferLifecycleTrace) -> Result<ExitCode> {
    // w5-1: without a backend every log::warn!/error! in blit-core is
    // silently discarded. Stderr, warn level, `blit: <level>: <msg>`.
    blit_core::stderr_log::init("blit");
    color_eyre::install()?;
    let Cli {
        config_dir,
        diagnostics_counter_file,
        diagnostics_scan_failure_name_cap,
        command,
    } = Cli::parse();
    lifecycle_trace.record(
        "argument_parse_end",
        Some(TransferLifecycleOutcome::Success),
    );

    if let Some(dir) = config_dir.as_ref() {
        config::set_config_dir(dir);
    }

    // audit-l39: pre-0.1.1 this was BLIT_TEST_COUNTER_FILE. Env vars
    // are out for app + diagnostic config; install via the CLI flag.
    if let Some(path) = diagnostics_counter_file {
        blit_core::remote::instrumentation::set_counter_path(path);
    }
    if let Some(cap) = diagnostics_scan_failure_name_cap {
        blit_core::remote::instrumentation::set_scan_failure_name_cap(cap);
    }

    lifecycle_trace.record("context_load_begin", None);
    let mut ctx = AppContext::load();
    lifecycle_trace.record("context_load_end", Some(TransferLifecycleOutcome::Success));

    let exit_code = match command {
        // pfc-5: the transfer verbs' status carries the destination's
        // per-file failure verdict — 0 when every file landed, 2 when the
        // operation completed with files that did not. Propagated directly,
        // like `check` and `jobs` below.
        // win-1: `run_with_retries` boxes each attempt, and `run_transfer`
        // / `run_move` box their route dispatch, so this frame holds
        // pointers rather than sessions.
        Commands::Copy(args) => {
            Box::pin(run_command(&ctx, Verb::Copy, args, lifecycle_trace, None)).await?
        }
        Commands::Mirror(args) => {
            Box::pin(run_command(&ctx, Verb::Mirror, args, lifecycle_trace, None)).await?
        }
        Commands::Move(args) => {
            Box::pin(run_command(&ctx, Verb::Move, args, lifecycle_trace, None)).await?
        }
        Commands::Scan(args) => {
            scan::run_scan(args).await?;
            ExitCode::SUCCESS
        }
        Commands::ListModules(args) => {
            list_modules::run_list_modules(args).await?;
            ExitCode::SUCCESS
        }
        Commands::Ls(args) => {
            ls::run_ls(args).await?;
            ExitCode::SUCCESS
        }
        Commands::Du(args) => {
            du::run_du(args).await?;
            ExitCode::SUCCESS
        }
        Commands::Df(args) => {
            df::run_df(args).await?;
            ExitCode::SUCCESS
        }
        Commands::Rm(args) => {
            rm::run_rm(args).await?;
            ExitCode::SUCCESS
        }
        Commands::Find(args) => {
            find::run_find(args).await?;
            ExitCode::SUCCESS
        }
        Commands::Completions(args) => {
            completions::run_completions(args).await?;
            ExitCode::SUCCESS
        }
        Commands::Profile(args) => {
            profile::run_profile(args)?;
            ExitCode::SUCCESS
        }
        // `check`'s exit code carries semantic info (0 identical /
        // 1 differences / 2 errors), so we propagate it directly — as do
        // the transfer verbs above and `jobs` below. The remaining
        // commands return Ok(()) and use the default 0.
        Commands::Check(args) => run_check(&args).await?,
        Commands::Diagnostics { command } => match command {
            DiagnosticsCommand::Perf(args) => {
                run_diagnostics_perf(&mut ctx, &args)?;
                ExitCode::SUCCESS
            }
            DiagnosticsCommand::Dump(args) => {
                run_diagnostics_dump(&args)?;
                ExitCode::SUCCESS
            }
        },
        // `jobs cancel` exits 0/1/2 (Cancelled / NotFound /
        // Unsupported) per the §6.5 contract; `jobs list`
        // always exits 0. The runner returns the right
        // `ExitCode` for both; propagate it directly like
        // `check`.
        // JOB_LOGS jl-3b: a saved job runs as the command it was.
        Commands::Jobs {
            command: JobsCommand::Run(args),
        } => {
            let job = jobs::job_to_run(&args.job).await?;
            let verb = Verb::from_word(&job.verb)
                .ok_or_else(|| eyre::eyre!("a job of an unknown kind ({})", job.verb))?;
            let jobs::JobToRun {
                args, saved, _list, ..
            } = job;
            Box::pin(run_command(&ctx, verb, args, lifecycle_trace, saved)).await?
        }
        Commands::Jobs { command } => run_jobs(command).await?,
    };

    Ok(exit_code)
}

/// The transfer verbs.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Verb {
    Copy,
    Mirror,
    Move,
}

impl Verb {
    fn word(self) -> &'static str {
        match self {
            Verb::Copy => "copy",
            Verb::Mirror => "mirror",
            Verb::Move => "move",
        }
    }

    fn from_word(word: &str) -> Option<Self> {
        [Verb::Copy, Verb::Mirror, Verb::Move]
            .into_iter()
            .find(|verb| verb.word() == word)
    }
}

/// One copy, mirror or move command — or a saved job run again — as one run
/// (JOB_LOGS jl-2, jl-3): one ID, its own log and job record, around every
/// pass and `--retry` rerun; `--save` keeps the job first, `--export`
/// writes it, with how the run went, after.
///
/// pfc-5: the status carries the destination's per-file failure verdict —
/// 0 when every file landed, 2 when the operation completed with files
/// that did not. win-1: `run_with_retries` boxes each attempt, and
/// `run_transfer` / `run_move` box their route dispatch, so this frame
/// holds pointers rather than sessions.
async fn run_command(
    ctx: &AppContext,
    verb: Verb,
    mut args: TransferArgs,
    lifecycle_trace: &TransferLifecycleTrace,
    saved_job: Option<String>,
) -> Result<ExitCode> {
    args.run = crate::run_log::CommandRun::start(verb.word(), &args, saved_job).await;
    let run = args.run.clone();
    if let Some(name) = &args.save {
        let run = run.as_ref().ok_or_else(|| {
            eyre::eyre!("this command cannot be saved as a job: it has no run ID")
        })?;
        if let Err(error) = run.save_as(name, args.json).await {
            run.finish(&Err(eyre::eyre!("{error:#}"))).await;
            return Err(error);
        }
    }
    let wait = Duration::from_secs(args.wait);
    let result = run_with_retries(
        args.retry,
        wait,
        |_n| -> Pin<Box<dyn Future<Output = Result<ExitCode>> + '_>> {
            match verb {
                Verb::Copy => Box::pin(run_transfer(
                    ctx,
                    &args,
                    TransferKind::Copy,
                    lifecycle_trace,
                )),
                Verb::Mirror => Box::pin(run_transfer(
                    ctx,
                    &args,
                    TransferKind::Mirror,
                    lifecycle_trace,
                )),
                Verb::Move => Box::pin(run_move(ctx, &args, lifecycle_trace)),
            }
        },
    )
    .await;
    if let Some(run) = &run {
        run.finish(&result).await;
        if let Some(file) = &args.export {
            if let Err(error) = run.export_to(file, args.json).await {
                // The transfer's own error comes first; this one is said.
                match &result {
                    Ok(_) => return Err(error),
                    Err(_) => eprintln!("blit: {error:#}"),
                }
            }
        }
    } else if args.export.is_some() {
        eyre::bail!("this command's job cannot be exported: it has no run ID");
    }
    result
}

fn lifecycle_result_outcome<T>(result: &Result<T>) -> TransferLifecycleOutcome {
    match result {
        Ok(_) => TransferLifecycleOutcome::Success,
        Err(err) => outcome_for_report(err),
    }
}

async fn finish_command_lifecycle<T>(
    lifecycle_trace: &TransferLifecycleTrace,
    result: Result<T>,
) -> Result<T> {
    lifecycle_trace.record("command_terminal", Some(lifecycle_result_outcome(&result)));
    lifecycle_trace.flush_async().await;
    result
}

#[cfg(test)]
mod lifecycle_tests {
    use super::*;
    use blit_core::generated::session_error::Code;
    use blit_core::transfer_session::SessionFault;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::{Arc, Mutex};

    async fn terminal_for(
        result: Result<()>,
    ) -> (
        Result<()>,
        Vec<blit_core::remote::transfer::TransferLifecycleEvent>,
        usize,
    ) {
        let events: Arc<Mutex<Vec<_>>> = Arc::default();
        let captured = Arc::clone(&events);
        let flushes = Arc::new(AtomicUsize::new(0));
        let captured_flushes = Arc::clone(&flushes);
        let trace = TransferLifecycleTrace::capture_with_flush(
            "cli-terminal",
            move |event| captured.lock().unwrap().push(event),
            move || {
                captured_flushes.fetch_add(1, Ordering::Relaxed);
            },
        );

        let returned = finish_command_lifecycle(&trace, result).await;
        let captured = events.lock().unwrap().clone();
        (returned, captured, flushes.load(Ordering::Relaxed))
    }

    #[tokio::test]
    async fn command_terminal_is_once_outcome_aware_and_flushed() {
        let (success, events, flushes) = terminal_for(Ok(())).await;
        assert!(success.is_ok());
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].event, "command_terminal");
        assert_eq!(events[0].outcome, Some(TransferLifecycleOutcome::Success));
        assert_eq!(flushes, 1);

        let refusal = eyre::Report::new(SessionFault::refusal(
            Code::ReadOnly,
            "presentation does not classify this",
        ));
        let (returned, events, flushes) = terminal_for(Err(refusal)).await;
        assert!(returned.is_err());
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].outcome, Some(TransferLifecycleOutcome::Refused));
        assert_eq!(flushes, 1);

        let (returned, events, flushes) = terminal_for(Err(eyre::eyre!("ordinary error"))).await;
        assert!(returned.is_err());
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].outcome, Some(TransferLifecycleOutcome::Error));
        assert_eq!(flushes, 1);
    }
}
