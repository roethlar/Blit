use clap::{Args, Parser, Subcommand};
use std::io::IsTerminal;
use std::path::PathBuf;

/// Appended to `--help` (not `-h`) for copy/mirror/move so the three
/// semantic rules that bit real users are visible without a manpage trip.
const PATH_SEMANTICS_HELP: &str = "\
PATHS (rsync-style trailing-slash semantics):
  blit copy SRC/  DEST/   merge SRC's contents into DEST (no basename nesting)
  blit copy SRC   DEST/   nest SRC under DEST  -> DEST/<basename(SRC)>
  blit copy SRC   DEST    if DEST exists as a dir: nest; else DEST becomes the copy
  blit copy f.txt DEST/   DEST/f.txt (into the directory)
  blit copy f.txt new.txt rename (when new.txt does not exist)

A trailing slash on SRC means \"copy the contents\". Without one, the basename is
appended when DEST is (or ends in) a directory; otherwise DEST is the exact
target path. See blit(1) for the full table.";

/// Appended to `blit check --help` so the equivalence model is
/// discoverable. F12 of docs/reviews/codebase_review_2026-05-01.md.
const CHECK_SEMANTICS_HELP: &str = "\
EQUIVALENCE MODEL:
  blit check verifies that a destination tree matches what `blit copy` or
  `blit mirror` would have produced — not full filesystem equivalence.

  Compared:    Regular files (by size+mtime, or hash with --checksum).
  Skipped:     Symlinks, FIFOs, devices, and other non-regular entries.
               Empty directories. Two trees differing only in those will
               be reported identical.
  Mismatches:  File-vs-directory at the same path produces a diff entry
               on the file side.

If you need full filesystem-tree equivalence (symlinks-as-targets,
empty-dir presence, etc.), use `diff -r` or a similar tool.";

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
    /// Copy files between local and/or remote locations (rsync-style slash semantics)
    Copy(TransferArgs),
    /// Mirror a directory, deleting extraneous files at destination (rsync-style slash semantics)
    Mirror(TransferArgs),
    /// Move files (copy + remove source, rsync-style slash semantics)
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
    /// Compare two trees by size+mtime or hash (no transfer — read-only verification)
    Check(CheckArgs),
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
    /// Emit a diagnostic snapshot for a SRC -> DEST invocation (no transfer performed)
    Dump(DiagnosticsDumpArgs),
}

#[derive(Args, Clone, Debug)]
pub struct DiagnosticsDumpArgs {
    /// Source path or remote endpoint (same syntax as `blit copy`)
    pub source: String,
    /// Destination path or remote endpoint (same syntax as `blit copy`)
    pub destination: String,
    /// Emit JSON instead of the default human-readable report
    #[arg(long)]
    pub json: bool,
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
#[command(after_long_help = PATH_SEMANTICS_HELP)]
pub struct TransferArgs {
    /// Source path or remote endpoint (host:/module/path).
    ///
    /// Trailing slash means "copy contents" (merge). Without a trailing slash,
    /// the source directory is nested under the destination (if destination is
    /// a container) or used as the exact target (otherwise).
    pub source: String,
    /// Destination path or remote endpoint.
    ///
    /// Trailing slash means "into this directory" (container). See `blit(1)`
    /// for the full rsync-style resolution rules.
    pub destination: String,

    // -- Common options (no heading — rendered in the default "Options"
    // section so first-time users see them at the top).
    /// Perform a dry run without making changes
    #[arg(long)]
    pub dry_run: bool,
    /// Keep verbose logs from the orchestrator
    #[arg(long, short = 'v')]
    pub verbose: bool,
    /// Show an interactive progress indicator.
    ///
    /// Auto-enabled when stdout is a TTY (and --json is not set) so
    /// interactive users get feedback by default; piping/redirecting
    /// stdout disables it so scripts aren't affected. Use this flag to
    /// force-enable when stdout is not a TTY (e.g. under `tee`).
    #[arg(long, short = 'p')]
    pub progress: bool,
    /// Skip confirmation prompt for destructive operations (mirror deletions, move)
    #[arg(long, short = 'y')]
    pub yes: bool,
    /// Output as JSON. With -p, emits NDJSON progress to stderr. Final
    /// transfer summary is written to stdout as a JSON object.
    #[arg(long)]
    pub json: bool,

    // -- Comparison options: how blit decides which files to transfer.
    /// Force checksum comparison of files (slower but more accurate)
    #[arg(long, short = 'c', help_heading = "Comparison")]
    pub checksum: bool,
    /// Compare only by size, ignoring modification time
    #[arg(long, conflicts_with = "checksum", help_heading = "Comparison")]
    pub size_only: bool,
    /// Transfer all files unconditionally, ignoring size and modification time
    #[arg(long, conflicts_with_all = ["checksum", "size_only"], help_heading = "Comparison")]
    pub ignore_times: bool,
    /// Skip files that already exist on the destination (regardless of differences)
    #[arg(long, conflicts_with = "force", help_heading = "Comparison")]
    pub ignore_existing: bool,
    /// Force exact mirror even if destination files are newer (dangerous)
    #[arg(long, help_heading = "Comparison")]
    pub force: bool,
    /// Mirror deletion scope: `subset` (default) deletes only files in the
    /// source filter scope; `all` deletes any destination file absent from
    /// the (filtered) source set, including files that wouldn't have been
    /// transferred in the first place. `all` is destructive — use with
    /// caution.
    #[arg(long, value_name = "SCOPE", default_value = "subset", value_parser = ["subset", "all"], help_heading = "Comparison")]
    pub delete_scope: String,

    // -- Reliability options: recovery + retries.
    /// Resume interrupted transfers using block-level comparison
    #[arg(long, help_heading = "Reliability")]
    pub resume: bool,
    /// Number of retries for failed transfers (0-255, default: 1)
    #[arg(long, default_value_t = 1, value_parser = clap::value_parser!(u8), help_heading = "Reliability")]
    pub retries: u8,

    // -- Filtering: restrict which files are eligible for transfer.
    // Filters apply identically to all source/destination combinations
    // (local-local, push, pull, remote-remote) — they live on the
    // pipeline's TransferSource so every path enforces them.
    /// Exclude files matching this glob pattern (repeatable)
    #[arg(long, action = clap::ArgAction::Append, value_name = "PATTERN", help_heading = "Filtering")]
    pub exclude: Vec<String>,
    /// Include only files matching this glob pattern (repeatable). When set,
    /// any include match is required; excludes still apply on top.
    #[arg(long, action = clap::ArgAction::Append, value_name = "PATTERN", help_heading = "Filtering")]
    pub include: Vec<String>,
    /// Only transfer files listed in FILE (one relative path per line, # comments allowed)
    #[arg(long, value_name = "FILE", help_heading = "Filtering")]
    pub files_from: Option<PathBuf>,
    /// Minimum file size to transfer (e.g. 100K, 10M, 1G)
    #[arg(long, value_name = "SIZE", help_heading = "Filtering")]
    pub min_size: Option<String>,
    /// Maximum file size to transfer (e.g. 1G, 500M)
    #[arg(long, value_name = "SIZE", help_heading = "Filtering")]
    pub max_size: Option<String>,
    /// Only transfer files older than this duration (e.g. 1h, 7d, 30m)
    #[arg(long, value_name = "DURATION", help_heading = "Filtering")]
    pub min_age: Option<String>,
    /// Only transfer files newer than this duration (e.g. 1h, 7d, 30m)
    #[arg(long, value_name = "DURATION", help_heading = "Filtering")]
    pub max_age: Option<String>,

    // -- Performance / debug knobs — niche, kept at the bottom so new
    // users aren't distracted by them.
    /// Force gRPC control-plane data path instead of hybrid TCP
    #[arg(long, help_heading = "Performance / debug")]
    pub force_grpc: bool,
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
    #[arg(long, help_heading = "Performance / debug")]
    pub null: bool,

    // -- Hidden flags (don't appear in --help).
    /// Limit worker threads (advanced debugging only)
    #[arg(long, hide = true)]
    pub workers: Option<usize>,
    /// Emit verbose TCP data-plane diagnostics (advanced debugging only)
    #[arg(long, hide = true)]
    pub trace_data_plane: bool,
}

impl TransferArgs {
    /// Effective progress setting: explicit `--progress` wins; otherwise
    /// enable automatically when stdout is a TTY and `--json` is off. This
    /// matches rsync/rclone/restic defaults so a first-time interactive
    /// user isn't staring at a silent terminal for 60+ seconds on a big
    /// transfer, while piped/redirected stdout keeps scripts unaffected.
    pub fn effective_progress(&self) -> bool {
        if self.progress {
            return true;
        }
        if self.json {
            return false;
        }
        std::io::stdout().is_terminal()
    }

    /// True when `--delete-scope all` was passed. Maps onto
    /// `MirrorMode::All` on the wire — every destination file absent
    /// from the (filtered) source set is purged, including files
    /// outside the filter scope. Defaults to false (`subset`).
    pub fn delete_scope_all(&self) -> bool {
        self.delete_scope.eq_ignore_ascii_case("all")
    }
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

/// Arguments for `blit check` — read-only tree comparison.
///
/// Reuses the same filter machinery (`FileFilter` + `build_filter`) that
/// transfers do, so `--exclude '*.tmp'` here behaves identically to
/// `--exclude '*.tmp'` on `blit copy`.
///
/// Equivalence model — `check` verifies *transfer equivalence*, not
/// full filesystem-tree equivalence. That means:
///
///   - **Regular files** are compared by size+mtime (default) or
///     Blake3 hash (with `--checksum`).
///   - **Symlinks** and other non-regular entries (FIFOs, devices,
///     etc.) are skipped silently. The transfer pipeline doesn't
///     replicate symlink contents byte-for-byte either, so two
///     trees that differ only in their symlinks will be reported
///     identical.
///   - **Empty directories** are not part of the diff. Transfers
///     don't preserve them today, so verifying their presence
///     would report false negatives on legitimate trees.
///   - **File-vs-directory mismatches** at the same path produce a
///     diff entry on the entry that's a file (the directory side
///     contributes nothing on its own).
///
/// Use `blit check` to verify that a `blit copy` or `blit mirror`
/// produced an equivalent destination, not as a general-purpose
/// `diff -r`. F12 of `docs/reviews/codebase_review_2026-05-01.md`.
#[derive(Args, Clone, Debug)]
#[command(after_long_help = CHECK_SEMANTICS_HELP)]
pub struct CheckArgs {
    /// Source tree to compare from
    pub source: String,
    /// Destination tree to compare against
    pub destination: String,
    /// Compare by Blake3 hash instead of size+mtime (slower, more accurate)
    #[arg(long, short = 'c')]
    pub checksum: bool,
    /// Only flag files missing on destination (ignore extras on dest)
    #[arg(long)]
    pub one_way: bool,
    /// Output as JSON
    #[arg(long)]
    pub json: bool,

    // Filter flags — same set/semantics as the transfer commands so
    // verification matches the transfer that produced the destination.
    #[arg(long, action = clap::ArgAction::Append, value_name = "PATTERN", help_heading = "Filtering")]
    pub exclude: Vec<String>,
    #[arg(long, action = clap::ArgAction::Append, value_name = "PATTERN", help_heading = "Filtering")]
    pub include: Vec<String>,
    #[arg(long, value_name = "FILE", help_heading = "Filtering")]
    pub files_from: Option<PathBuf>,
    #[arg(long, value_name = "SIZE", help_heading = "Filtering")]
    pub min_size: Option<String>,
    #[arg(long, value_name = "SIZE", help_heading = "Filtering")]
    pub max_size: Option<String>,
    #[arg(long, value_name = "DURATION", help_heading = "Filtering")]
    pub min_age: Option<String>,
    #[arg(long, value_name = "DURATION", help_heading = "Filtering")]
    pub max_age: Option<String>,
}
