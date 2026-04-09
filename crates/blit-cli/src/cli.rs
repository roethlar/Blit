use clap::{Args, Parser, Subcommand};
use std::path::PathBuf;

#[derive(Parser)]
#[command(name = "blit")]
#[command(about = "A fast, AI-built file transfer tool (v2)")]
#[command(after_help = "Run '<command> --help' for detailed options on each command.")]
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
    /// List modules exported by a remote daemon
    ListModules(ListModulesArgs),
    /// List directory entries (remote or local)
    #[command(alias = "list")]
    Ls(ListArgs),
    /// Show disk usage for a remote path
    Du(DuArgs),
    /// Show filesystem statistics for a remote module
    Df(DfArgs),
    /// Remove a file or directory on a remote daemon
    Rm(RmArgs),
    /// Search for files on a remote daemon
    Find(FindArgs),
    /// Fetch remote path completions for interactive shells
    Completions(CompletionArgs),
    /// Show local performance history summary
    Profile(ProfileArgs),
    /// Diagnostics and tooling commands
    Diagnostics {
        #[command(subcommand)]
        command: DiagnosticsCommand,
    },
}

#[derive(Subcommand)]
pub enum DiagnosticsCommand {
    /// Manage performance history capture (enable/disable/clear)
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
    /// Output as JSON
    #[arg(long)]
    pub json: bool,
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
    /// Force checksum comparison of files (slower but more accurate)
    #[arg(long, short = 'c')]
    pub checksum: bool,
    /// Compare only by size, ignoring modification time
    #[arg(long, conflicts_with = "checksum")]
    pub size_only: bool,
    /// Transfer all files unconditionally, ignoring size and modification time
    #[arg(long, conflicts_with_all = ["checksum", "size_only"])]
    pub ignore_times: bool,
    /// Skip files that already exist on the destination (regardless of differences)
    #[arg(long)]
    pub ignore_existing: bool,
    /// Force exact mirror even if destination files are newer (dangerous)
    #[arg(long)]
    pub force: bool,
    /// Number of retries for failed transfers (0-255, default: 1)
    #[arg(long, default_value_t = 1, value_parser = clap::value_parser!(u8))]
    pub retries: u8,
    /// Keep verbose logs from the orchestrator
    #[arg(long, short = 'v')]
    pub verbose: bool,
    /// Show an interactive progress indicator
    #[arg(long, short = 'p')]
    pub progress: bool,
    /// Skip confirmation prompt for destructive operations (mirror deletions, move)
    #[arg(long, short = 'y')]
    pub yes: bool,
    /// Limit worker threads (advanced debugging only)
    #[arg(long, hide = true)]
    pub workers: Option<usize>,
    /// Emit verbose TCP data-plane diagnostics (advanced debugging only)
    #[arg(long, hide = true)]
    pub trace_data_plane: bool,
    /// Force gRPC control-plane data path instead of hybrid TCP
    #[arg(long)]
    pub force_grpc: bool,
    /// Resume interrupted transfers using block-level comparison
    #[arg(long)]
    pub resume: bool,
    /// Discard all writes (measure source read + pipeline throughput only).
    ///
    /// Reads and prepares all source data normally but does not write to the
    /// destination. Use this to isolate whether a bottleneck is on the source
    /// or destination side. Example:
    ///
    ///   blit copy /data/large-dataset /tmp/unused --null -v
    ///
    /// The destination path is still required for planning but nothing is
    /// written there. Performance records are tagged so the adaptive predictor
    /// does not learn from null-sink runs.
    #[arg(long)]
    pub null: bool,
}

#[derive(Args, Clone, Debug)]
pub struct ScanArgs {
    /// Seconds to wait for mDNS responses
    #[arg(long, default_value_t = 2)]
    pub wait: u64,
    /// Output as JSON
    #[arg(long)]
    pub json: bool,
}

#[derive(Args, Clone, Debug)]
pub struct ListModulesArgs {
    /// Remote host (e.g. server or server:port)
    pub remote: String,
    #[arg(long)]
    pub json: bool,
}

#[derive(Args, Clone, Debug)]
pub struct ListArgs {
    /// Local path or remote endpoint (host:/module/path)
    pub target: String,
    #[arg(long)]
    pub json: bool,
}

#[derive(Args, Clone, Debug)]
pub struct DuArgs {
    /// Remote path to check (e.g., server:/module/path)
    pub target: String,
    /// Max depth to traverse (0 = unlimited)
    #[arg(long)]
    pub max_depth: Option<u32>,
    /// Output as JSON
    #[arg(long)]
    pub json: bool,
}

#[derive(Args, Clone, Debug)]
pub struct DfArgs {
    /// Remote module to check (e.g., server:/module/)
    pub remote: String,
    /// Output as JSON
    #[arg(long)]
    pub json: bool,
}

#[derive(Args, Clone, Debug)]
pub struct RmArgs {
    /// Remote path to delete (e.g., server:/module/path)
    pub target: String,
    /// Skip confirmation prompt
    #[arg(long, short = 'y')]
    pub yes: bool,
    /// Output as JSON
    #[arg(long)]
    pub json: bool,
}

#[derive(Args, Clone, Debug)]
pub struct FindArgs {
    /// Remote path to search (e.g., server:/module/path)
    pub target: String,
    /// Pattern to match (substring match)
    #[arg(long)]
    pub pattern: Option<String>,
    /// Include only files in results
    #[arg(long)]
    pub files: bool,
    /// Include only directories in results
    #[arg(long)]
    pub dirs: bool,
    /// Case-insensitive pattern matching
    #[arg(long)]
    pub case_insensitive: bool,
    /// Limit number of results
    #[arg(long)]
    pub limit: Option<u32>,
    /// Output as JSON
    #[arg(long)]
    pub json: bool,
}

#[derive(Args, Clone, Debug)]
pub struct CompletionArgs {
    /// Remote path (e.g., server:/module/)
    pub target: String,
    /// Include only file completions
    #[arg(long)]
    pub files: bool,
    /// Include only directory completions
    #[arg(long)]
    pub dirs: bool,
    /// Additional prefix for filtering
    #[arg(long)]
    pub prefix: Option<String>,
}

#[derive(Args, Clone, Debug)]
pub struct ProfileArgs {
    #[arg(long)]
    pub json: bool,
    #[arg(long, default_value_t = 50)]
    pub limit: usize,
}
