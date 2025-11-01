use clap::{Args, Parser, Subcommand};
use std::path::PathBuf;

#[derive(Parser)]
#[command(name = "blit")]
#[command(about = "A fast, AI-built file transfer tool (v2)")]
pub struct Cli {
    /// Override the configuration directory for this invocation
    #[arg(long, global = true, value_name = "PATH")]
    pub config_dir: Option<PathBuf>,
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Copy files between local and/or remote locations
    Copy(TransferArgs),
    /// Mirror a directory (including deletions at destination)
    Mirror(TransferArgs),
    /// Move a directory or file (mirror + remove source)
    Move(TransferArgs),
    /// Discover daemons advertising via mDNS
    Scan(ScanArgs),
    /// List modules or paths on a remote daemon
    List(ListArgs),
    /// Diagnostics and tooling commands
    Diagnostics {
        #[command(subcommand)]
        command: DiagnosticsCommand,
    },
}

#[derive(Subcommand)]
pub enum DiagnosticsCommand {
    /// Show recent performance history captured locally
    Perf(PerfArgs),
}

#[derive(Args, Clone, Debug)]
pub struct PerfArgs {
    /// Number of recent records to display (0 = all)
    #[arg(long, default_value_t = 50)]
    pub limit: usize,
    /// Enable performance history capture
    #[arg(long, conflicts_with = "disable")]
    pub enable: bool,
    /// Disable performance history capture
    #[arg(long, conflicts_with = "enable")]
    pub disable: bool,
    /// Remove the stored performance history file
    #[arg(long)]
    pub clear: bool,
}

#[derive(Args, Clone, Debug)]
pub struct TransferArgs {
    /// Source path for the transfer
    pub source: String,
    /// Destination path for the transfer
    pub destination: String,
    /// Perform a dry run without making changes
    #[arg(long)]
    pub dry_run: bool,
    /// Force checksum comparison of files
    #[arg(long)]
    pub checksum: bool,
    /// Keep verbose logs from the orchestrator
    #[arg(long, short = 'v')]
    pub verbose: bool,
    /// Show an interactive progress indicator
    #[arg(long, short = 'p')]
    pub progress: bool,
    /// Limit worker threads (advanced debugging only)
    #[arg(long, hide = true)]
    pub workers: Option<usize>,
    /// Force gRPC control-plane data path instead of hybrid TCP
    #[arg(long)]
    pub force_grpc: bool,
}

#[derive(Args, Clone, Debug)]
pub struct ScanArgs {
    /// Seconds to wait for mDNS responses
    #[arg(long, default_value_t = 2)]
    pub wait: u64,
}

#[derive(Args, Clone, Debug)]
pub struct ListArgs {
    /// Remote location to list (e.g., server:/module/, server:/module/path, server)
    pub target: String,
}
