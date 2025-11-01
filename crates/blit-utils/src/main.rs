mod cli;
mod completions;
mod df;
mod du;
mod find;
mod list_modules;
mod ls;
mod profile;
mod rm;
mod scan;
mod util;

use crate::cli::{Cli, Commands};
use clap::Parser;
use eyre::Result;

#[tokio::main]
async fn main() -> Result<()> {
    color_eyre::install()?;
    let Cli { command } = Cli::parse();

    match command {
        Commands::Scan(args) => scan::run_scan(args).await?,
        Commands::ListModules(args) => list_modules::run_list_modules(args).await?,
        Commands::Ls(args) => ls::run_ls(args).await?,
        Commands::Find(args) => find::run_find(args).await?,
        Commands::Du(args) => du::run_du(args).await?,
        Commands::Df(args) => df::run_df(args).await?,
        Commands::Rm(args) => rm::run_rm(args).await?,
        Commands::Completions(args) => completions::run_completions(args).await?,
        Commands::Profile(args) => profile::run_profile(args)?,
    }

    Ok(())
}
