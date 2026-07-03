mod admin;
mod cli;
mod context;
mod diagnostics;
mod list;
mod scan;
mod transfers;

use crate::admin::{run_df, run_du, run_find, run_rm};
use crate::cli::{Cli, Commands, DiagnosticsCommand};
use crate::context::AppContext;
use crate::diagnostics::run_diagnostics_perf;
use crate::list::run_list;
use crate::scan::run_scan;
use crate::transfers::{run_move, run_transfer, TransferKind};
use blit_core::config;
use clap::Parser;
use eyre::Result;

#[tokio::main]
async fn main() -> Result<()> {
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
        Commands::Scan(args) => run_scan(&args).await?,
        Commands::List(args) => run_list(&args).await?,
        Commands::Du(args) => run_du(&ctx, &args).await?,
        Commands::Df(args) => run_df(&ctx, &args).await?,
        Commands::Rm(args) => run_rm(&ctx, &args).await?,
        Commands::Find(args) => run_find(&ctx, &args).await?,
        Commands::Diagnostics { command } => match command {
            DiagnosticsCommand::Perf(args) => run_diagnostics_perf(&mut ctx, &args)?,
        },
    }

    Ok(())
}
