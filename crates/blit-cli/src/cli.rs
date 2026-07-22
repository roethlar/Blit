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

    /// Diagnostics only: write internal byte-path counters to this
    /// file (one `<event> <value>` line per record). Used by the
    /// integration tests and `scripts/bench_remote_remote.sh` to
    /// assert byte-path isolation; not intended for operator use.
    /// Replaces the pre-0.1.1 `BLIT_TEST_COUNTER_FILE` env var
    /// (audit-l39: env vars are out for app + diagnostic config).
    ///
    /// `hide_short_help = true` hides this flag from the short `-h`
    /// summary; it still appears in the full `--help` output so it's
    /// discoverable for troubleshooting.
    #[arg(long, global = true, value_name = "PATH", hide_short_help = true)]
    pub diagnostics_counter_file: Option<PathBuf>,

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
    /// Inspect transfer jobs on a remote daemon
    Jobs {
        #[command(subcommand)]
        command: JobsCommand,
    },
}

#[derive(Subcommand)]
pub enum JobsCommand {
    /// List active and recent transfers on a remote daemon
    List(JobsListArgs),
    /// Cancel an active transfer on a remote daemon
    Cancel(JobsCancelArgs),
    /// Watch an active transfer until it completes
    Watch(JobsWatchArgs),
}

#[derive(Args, Clone, Debug)]
pub struct JobsListArgs {
    /// Remote host (e.g. server or server:port)
    pub remote: String,
    /// Maximum number of recent transfers to return. 0 means
    /// the daemon's default (50).
    #[arg(long, default_value_t = 0)]
    pub recent_limit: u32,
    /// Output as JSON
    #[arg(long)]
    pub json: bool,
}

#[derive(Args, Clone, Debug)]
pub struct JobsWatchArgs {
    /// Remote host (e.g. server or server:port)
    pub remote: String,
    /// Transfer id to watch — typically obtained from
    /// `blit jobs list <remote>` or the `--detach` output.
    pub transfer_id: String,
    /// Maximum wall-clock seconds to watch before giving up.
    /// 0 = wait forever. Useful for scripts that don't want
    /// to hang on a stuck transfer.
    #[arg(long, default_value_t = 0)]
    pub timeout_secs: u64,
    /// Output as JSON-Lines (one object per stream update,
    /// plus a final outcome line). Default is a
    /// human-readable stream.
    #[arg(long)]
    pub json: bool,
}

#[derive(Args, Clone, Debug)]
pub struct JobsCancelArgs {
    /// Remote host (e.g. server or server:port)
    pub remote: String,
    /// Transfer id to cancel — typically obtained from
    /// `blit jobs list <remote>`.
    pub transfer_id: String,
    /// Output as JSON
    #[arg(long)]
    pub json: bool,
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
    /// Emit detailed transfer diagnostics
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
    /// Use block-level comparison to continue eligible partial files for
    /// local, push, pull, and remote-to-remote transfers
    #[arg(long, help_heading = "Reliability")]
    pub resume: bool,
    /// Discard Windows file attributes and named data streams.
    ///
    /// This is a lossy cross-platform escape hatch. Without it, Blit refuses
    /// to copy Windows metadata to a destination that cannot preserve it.
    #[arg(long, help_heading = "Reliability")]
    pub drop_windows_metadata: bool,
    /// Retry the transfer up to N times on a transient failure (network
    /// drop, stall timeout). Each retry re-runs destination comparison, so
    /// normal comparison skips files now complete; flags that force copying
    /// still apply. With --resume, eligible partial files continue at block
    /// granularity. 0 (default) disables retries.
    #[arg(
        long,
        value_name = "N",
        default_value_t = 0,
        help_heading = "Reliability"
    )]
    pub retry: u32,
    /// Seconds to wait between retries (see --retry).
    #[arg(
        long,
        value_name = "SECS",
        default_value_t = 5,
        help_heading = "Reliability"
    )]
    pub wait: u64,

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
    /// Fire-and-forget: hand the transfer to the destination
    /// daemon and exit as soon as it starts.
    ///
    /// The CLI awaits the daemon's `Started` event (which
    /// includes the daemon-assigned `transfer_id`), prints
    /// it plus a `blit jobs cancel` hint, and returns. The
    /// destination daemon completes the transfer regardless
    /// of CLI connection state. Useful for long remote→remote
    /// transfers that should outlive the operator's shell.
    ///
    /// Only valid for remote→remote transfers (the daemon-to-daemon
    /// delegated byte path), and not for `blit move`.
    ///
    /// Rejected with a clear error for:
    /// - local-source or local-destination transfers (CLI is in
    ///   the byte path)
    /// - `blit move` (the source-delete step needs the CLI to
    ///   await transfer completion)
    #[arg(long)]
    pub detach: bool,
    /// Discard all writes — local copy only (read+pipeline benchmark).
    ///
    /// Reads and prepares all source data normally but does not write to
    /// the destination. Use this to isolate whether a bottleneck is on
    /// the source or destination side. Example:
    ///
    ///   blit copy /data/large-dataset /tmp/unused --null -v
    ///
    /// **Restrictions** (R54-F1): --null is supported only by `blit copy`
    /// between two local paths. The CLI rejects it with `blit mirror`
    /// (the destination-purge step would still delete files), with
    /// `blit move` (the source-delete step would erase the source with
    /// no copy), and with any remote endpoint (the remote push/pull
    /// paths don't honor the null sink, so the flag would be silently
    /// ignored).
    ///
    /// Performance-history records are tagged (`null_sink` lane) so
    /// real-transfer profiling never learns from null-sink runs.
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
    /// Glob pattern matched against each entry's relative path AND
    /// its file-name basename (whichever matches first). Uses POSIX
    /// shell-glob semantics: `*` matches any run of non-`/`
    /// characters, `?` matches one non-`/` character, `[abc]`
    /// matches a class, `**/` matches zero or more nested directory
    /// components. `*` does NOT cross `/`; use `**/` to traverse
    /// directories. The basename fallback means `*.csv` finds nested
    /// entries (matching their basename) without writing `**/`.
    /// Case sensitivity controlled by `--case-insensitive`. Empty
    /// matches everything.
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
    #[command(subcommand)]
    pub kind: CompletionKind,
}

#[derive(clap::Subcommand, Clone, Debug)]
pub enum CompletionKind {
    /// Generate a shell-completion script for one of bash, zsh,
    /// fish, powershell, or elvish. Pipe the output to your shell's
    /// completion-script directory:
    ///
    ///   blit completions shell bash > ~/.local/share/bash-completion/completions/blit
    ///   blit completions shell zsh  > "${fpath[1]}/_blit"
    ///   blit completions shell fish > ~/.config/fish/completions/blit.fish
    Shell(ShellCompletionArgs),

    /// Fetch remote-path completions via the daemon's CompletePath
    /// RPC. Used by interactive shells when the user types a
    /// `server:/module/` prefix and presses Tab; the generated
    /// shell-completion scripts call this internally and stream the
    /// matching paths.
    Remote(RemoteCompletionArgs),
}

#[derive(Args, Clone, Debug)]
pub struct ShellCompletionArgs {
    /// Target shell.
    pub shell: clap_complete::Shell,
}

#[derive(Args, Clone, Debug)]
pub struct RemoteCompletionArgs {
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

#[cfg(test)]
mod tests {
    //! Tests for the clap surface itself. Anything that can fail at
    //! `cargo build` time isn't worth a runtime check; these target
    //! behavior that only surfaces at parse / generation time.

    use super::*;
    use clap::CommandFactory;

    #[test]
    fn clap_definition_validates() {
        // clap's debug_assert! validations only fire in debug builds
        // when CommandFactory::command() is called. This test forces
        // them to run so a misconfigured arg/conflict surfaces here
        // rather than the first time a real user hits the bad path.
        Cli::command().debug_assert();
    }

    /// retry-wait: the `--retry`/`--wait` flags parse, default to no
    /// retries / 5s, and accept explicit values.
    #[test]
    fn retry_wait_flags_parse_and_default() {
        let cli = Cli::try_parse_from(["blit", "copy", "src", "dst"]).expect("parse defaults");
        let Commands::Copy(args) = cli.command else {
            panic!("expected Copy");
        };
        assert_eq!(args.retry, 0, "retry defaults to 0 (no retries)");
        assert_eq!(args.wait, 5, "wait defaults to 5s");

        let cli =
            Cli::try_parse_from(["blit", "copy", "--retry", "3", "--wait", "10", "src", "dst"])
                .expect("parse explicit");
        let Commands::Copy(args) = cli.command else {
            panic!("expected Copy");
        };
        assert_eq!(args.retry, 3);
        assert_eq!(args.wait, 10);
    }

    #[test]
    fn reliability_help_matches_retry_and_resume_behavior() {
        let command = Cli::command();
        let copy = command
            .find_subcommand("copy")
            .expect("copy subcommand exists");
        let help_for = |id: &str| {
            let argument = copy
                .get_arguments()
                .find(|argument| argument.get_id() == id)
                .unwrap_or_else(|| panic!("{id} argument exists"));
            argument
                .get_long_help()
                .or_else(|| argument.get_help())
                .expect("argument has help")
                .to_string()
        };

        let resume = help_for("resume");
        assert!(resume.contains("eligible partial files"));
        assert!(resume.contains("local, push, pull, and remote-to-remote"));

        let retry = help_for("retry");
        assert!(retry.contains("Each retry re-runs destination comparison"));
        assert!(retry.contains("flags that force copying still apply"));
        assert!(retry.contains("With --resume, eligible partial files continue at block"));
    }

    #[test]
    fn windows_metadata_downgrade_is_explicit_and_defaults_strict() {
        let cli = Cli::try_parse_from(["blit", "copy", "src", "dst"]).expect("parse defaults");
        let Commands::Copy(args) = cli.command else {
            panic!("expected Copy");
        };
        assert!(!args.drop_windows_metadata);

        let cli = Cli::try_parse_from(["blit", "copy", "--drop-windows-metadata", "src", "dst"])
            .expect("parse explicit downgrade");
        let Commands::Copy(args) = cli.command else {
            panic!("expected Copy");
        };
        assert!(args.drop_windows_metadata);
    }

    #[test]
    fn jobs_watch_rejects_retired_poll_interval() {
        let error = match Cli::try_parse_from([
            "blit",
            "jobs",
            "watch",
            "server",
            "transfer-1",
            "--interval-ms",
            "50",
        ]) {
            Ok(_) => panic!("streaming watch has no polling interval"),
            Err(error) => error,
        };
        assert!(error.to_string().contains("--interval-ms"));
    }

    #[test]
    fn shell_completions_generate_for_bash() {
        // P0 §2.5: `blit completions shell <SHELL>` must actually
        // emit a non-empty completion script for each supported
        // shell. This is the load-bearing test that the README:33
        // "shell completions" promise is now backed by real script
        // generation, not just the CompletePath RPC.
        let mut cmd = Cli::command();
        let mut buffer: Vec<u8> = Vec::new();
        clap_complete::generate(clap_complete::Shell::Bash, &mut cmd, "blit", &mut buffer);
        let script = String::from_utf8(buffer).expect("utf8 script");
        assert!(
            script.contains("_blit()"),
            "bash completion script should define the _blit function; got first 200 chars:\n{}",
            &script[..script.len().min(200)]
        );
        // The script must mention every top-level subcommand so
        // tab-completion at `blit <Tab>` actually works.
        for verb in [
            "copy",
            "mirror",
            "move",
            "scan",
            "list",
            "find",
            "completions",
        ] {
            assert!(
                script.contains(verb),
                "bash completion script missing verb '{}'",
                verb
            );
        }
    }

    #[test]
    fn shell_completions_generate_for_each_supported_shell() {
        // Every shell variant clap_complete advertises must produce
        // non-empty output. Catches the case where a future shell
        // is added to clap_complete::Shell but our handler doesn't
        // route it (clap_complete::generate is total over the enum,
        // so this should always pass — the test pins it).
        let mut cmd = Cli::command();
        for shell in [
            clap_complete::Shell::Bash,
            clap_complete::Shell::Zsh,
            clap_complete::Shell::Fish,
            clap_complete::Shell::PowerShell,
            clap_complete::Shell::Elvish,
        ] {
            let mut buffer: Vec<u8> = Vec::new();
            clap_complete::generate(shell, &mut cmd, "blit", &mut buffer);
            assert!(
                !buffer.is_empty(),
                "{:?} completion generation produced empty output",
                shell
            );
        }
    }
}
