mod check;
mod cli;
mod completions;
mod context;
mod df;
mod diagnostics;
mod du;
mod find;
mod list_modules;
mod ls;
mod profile;
mod rm;
mod scan;
mod transfers;
mod util;

use crate::check::run_check;
use crate::cli::{Cli, Commands, DiagnosticsCommand};
use crate::context::AppContext;
use crate::diagnostics::{run_diagnostics_dump, run_diagnostics_perf};
use crate::transfers::{run_move, run_transfer, TransferKind};
use blit_core::config;
use clap::Parser;
use eyre::Result;
use std::process::ExitCode;

#[tokio::main]
async fn main() -> Result<ExitCode> {
    color_eyre::install()?;
    let Cli {
        config_dir,
        command,
    } = Cli::parse();

    if let Some(dir) = config_dir.as_ref() {
        config::set_config_dir(dir);
    }

    let mut ctx = AppContext::load();

    match command {
        Commands::Copy(args) => run_transfer(&ctx, &args, TransferKind::Copy).await?,
        Commands::Mirror(args) => run_transfer(&ctx, &args, TransferKind::Mirror).await?,
        Commands::Move(args) => run_move(&ctx, &args).await?,
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
        Commands::Check(args) => return Ok(run_check(&args).await?),
        Commands::Diagnostics { command } => match command {
            DiagnosticsCommand::Perf(args) => run_diagnostics_perf(&mut ctx, &args)?,
            DiagnosticsCommand::Dump(args) => run_diagnostics_dump(&args)?,
        },
    }

    Ok(ExitCode::SUCCESS)
}
