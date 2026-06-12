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
mod scan;
mod transfers;

use crate::check::run_check;
use crate::cli::{Cli, Commands, DiagnosticsCommand};
use crate::context::AppContext;
use crate::diagnostics::{run_diagnostics_dump, run_diagnostics_perf};
use crate::jobs::run_jobs;
use crate::transfers::{run_move, run_transfer};
use blit_app::transfers::dispatch::TransferKind;
use blit_app::transfers::retry::run_with_retries;
use blit_core::config;
use clap::Parser;
use eyre::Result;
use std::process::ExitCode;
use std::time::Duration;

#[tokio::main]
async fn main() -> Result<ExitCode> {
    // w5-1: without a backend every log::warn!/error! in blit-core is
    // silently discarded. Stderr, warn level, `blit: <level>: <msg>`.
    blit_core::stderr_log::init("blit");
    color_eyre::install()?;
    let Cli {
        config_dir,
        diagnostics_counter_file,
        command,
    } = Cli::parse();

    if let Some(dir) = config_dir.as_ref() {
        config::set_config_dir(dir);
    }

    // audit-l39: pre-0.1.1 this was BLIT_TEST_COUNTER_FILE. Env vars
    // are out for app + diagnostic config; install via the CLI flag.
    if let Some(path) = diagnostics_counter_file {
        blit_core::remote::instrumentation::set_counter_path(path);
    }

    let mut ctx = AppContext::load();

    match command {
        Commands::Copy(args) => {
            let wait = Duration::from_secs(args.wait);
            run_with_retries(args.retry, wait, |_n| {
                run_transfer(&ctx, &args, TransferKind::Copy)
            })
            .await?
        }
        Commands::Mirror(args) => {
            let wait = Duration::from_secs(args.wait);
            run_with_retries(args.retry, wait, |_n| {
                run_transfer(&ctx, &args, TransferKind::Mirror)
            })
            .await?
        }
        Commands::Move(args) => {
            let wait = Duration::from_secs(args.wait);
            run_with_retries(args.retry, wait, |_n| run_move(&ctx, &args)).await?
        }
        Commands::Scan(args) => scan::run_scan(args).await?,
        Commands::ListModules(args) => list_modules::run_list_modules(args).await?,
        Commands::Ls(args) => ls::run_ls(args).await?,
        Commands::Du(args) => du::run_du(args).await?,
        Commands::Df(args) => df::run_df(args).await?,
        Commands::Rm(args) => rm::run_rm(args).await?,
        Commands::Find(args) => find::run_find(args).await?,
        Commands::Completions(args) => completions::run_completions(args).await?,
        Commands::Profile(args) => profile::run_profile(args)?,
        // `check` is the only command whose exit code carries semantic
        // info (0 identical / 1 differences / 2 errors), so we propagate
        // it directly. Other commands return Ok(()) and use the default 0.
        Commands::Check(args) => return run_check(&args).await,
        Commands::Diagnostics { command } => match command {
            DiagnosticsCommand::Perf(args) => run_diagnostics_perf(&mut ctx, &args)?,
            DiagnosticsCommand::Dump(args) => run_diagnostics_dump(&args)?,
        },
        // `jobs cancel` exits 0/1/2 (Cancelled / NotFound /
        // Unsupported) per the §6.5 contract; `jobs list`
        // always exits 0. The runner returns the right
        // `ExitCode` for both; propagate it directly like
        // `check`.
        Commands::Jobs { command } => return run_jobs(command).await,
    }

    Ok(ExitCode::SUCCESS)
}
