use clap::{Args, Parser, Subcommand};

#[derive(Parser)]
#[command(name = "blit-utils")]
#[command(about = "Administrative tooling for Blit v2 daemons")]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Discover daemons via mDNS
    Scan(ScanArgs),
    /// List modules exported by a daemon
    ListModules(ListModulesArgs),
    /// List directory entries (remote or local)
    #[command(alias = "list")]
    Ls(ListArgs),
    /// Recursive find for remote paths
    Find(FindArgs),
    /// Disk usage summary for a remote subtree
    Du(DuArgs),
    /// Filesystem stats for a remote module
    Df(DfArgs),
    /// Remove files/directories remotely (confirmation required unless --yes)
    Rm(RmArgs),
    /// Fetch remote path completions for interactive shells
    Completions(CompletionArgs),
    /// Show local performance history summary
    Profile(ProfileArgs),
}

#[derive(Args, Clone, Debug)]
pub struct ScanArgs {
    #[arg(long, default_value_t = 2)]
    pub wait: u64,
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
pub struct FindArgs {
    pub target: String,
    #[arg(long)]
    pub pattern: Option<String>,
    #[arg(long)]
    pub files: bool,
    #[arg(long)]
    pub dirs: bool,
    #[arg(long)]
    pub case_insensitive: bool,
    #[arg(long)]
    pub limit: Option<u32>,
    #[arg(long)]
    pub json: bool,
}

#[derive(Args, Clone, Debug)]
pub struct DuArgs {
    pub target: String,
    #[arg(long)]
    pub max_depth: Option<u32>,
    #[arg(long)]
    pub json: bool,
}

#[derive(Args, Clone, Debug)]
pub struct DfArgs {
    pub remote: String,
    #[arg(long)]
    pub json: bool,
}

#[derive(Args, Clone, Debug)]
pub struct RmArgs {
    pub target: String,
    #[arg(long)]
    pub yes: bool,
}

#[derive(Args, Clone, Debug)]
pub struct CompletionArgs {
    pub target: String,
    #[arg(long)]
    pub files: bool,
    #[arg(long)]
    pub dirs: bool,
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
