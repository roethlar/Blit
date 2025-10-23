use std::fs;
use std::io::{self, Write};
use std::path::{Path, PathBuf};

use blit_core::generated::blit_client::BlitClient;
use blit_core::generated::{FileInfo, ListModulesRequest, ListRequest, PurgeRequest};
use blit_core::perf_history;
use blit_core::perf_predictor::PerformancePredictor;
use blit_core::remote::endpoint::{RemoteEndpoint, RemotePath};
use clap::{Args, Parser, Subcommand};
use eyre::{Context, Result, bail};
use serde::Serialize;

#[derive(Parser)]
#[command(name = "blit-utils")]
#[command(about = "Administrative tooling for Blit v2 daemons")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Discover daemons via mDNS (pending Phase 3)
    Scan(ScanArgs),
    /// List modules exported by a daemon
    ListModules(ListModulesArgs),
    /// List directory entries (remote or local)
    #[command(alias = "list")]
    Ls(ListArgs),
    /// Recursive find (not yet implemented)
    Find(FindArgs),
    /// Disk usage summary (not yet implemented)
    Du(DuArgs),
    /// Filesystem stats (not yet implemented)
    Df(DfArgs),
    /// Remove files/directories remotely (confirmation required unless --yes)
    Rm(RmArgs),
    /// Generate shell completions (not yet implemented)
    Completions(CompletionArgs),
    /// Show local performance history summary
    Profile(ProfileArgs),
}

#[derive(Args)]
struct ScanArgs {
    #[arg(long, default_value_t = 2)]
    wait: u64,
}

#[derive(Args)]
struct ListModulesArgs {
    /// Remote host (e.g. server or server:port)
    remote: String,
    #[arg(long)]
    json: bool,
}

#[derive(Args)]
struct ListArgs {
    /// Local path or remote endpoint (host:/module/path)
    target: String,
    #[arg(long)]
    json: bool,
}

#[derive(Args)]
struct FindArgs {
    #[arg()]
    target: String,
}

#[derive(Args)]
struct DuArgs {
    #[arg()]
    target: String,
}

#[derive(Args)]
struct DfArgs {
    #[arg()]
    remote: String,
}

#[derive(Args)]
struct RmArgs {
    #[arg()]
    target: String,
    #[arg(long)]
    yes: bool,
}

#[derive(Args)]
struct CompletionArgs {
    #[arg()]
    shell: String,
}

#[derive(Args)]
struct ProfileArgs {
    #[arg(long)]
    json: bool,
    #[arg(long, default_value_t = 50)]
    limit: usize,
}

#[tokio::main]
async fn main() -> Result<()> {
    color_eyre::install()?;
    let cli = Cli::parse();

    match cli.command {
        Commands::Scan(args) => run_scan(args).await,
        Commands::ListModules(args) => run_list_modules(args).await,
        Commands::Ls(args) => run_ls(args).await,
        Commands::Find(_args) => {
            bail!("`blit-utils find` is not implemented yet (Phase 3 task)");
        }
        Commands::Du(_args) => {
            bail!("`blit-utils du` is not implemented yet (Phase 3 task)");
        }
        Commands::Df(_args) => {
            bail!("`blit-utils df` is not implemented yet (Phase 3 task)");
        }
        Commands::Rm(args) => run_rm(args).await,
        Commands::Completions(_args) => {
            bail!("`blit-utils completions` is not implemented yet (Phase 3 task)");
        }
        Commands::Profile(args) => run_profile(args),
    }
}

async fn run_scan(_args: ScanArgs) -> Result<()> {
    bail!(
        "`blit-utils scan` is not implemented yet; use `blit scan` for mDNS discovery until Phase 3 landing."
    );
}

async fn run_list_modules(args: ListModulesArgs) -> Result<()> {
    let remote = RemoteEndpoint::parse(&args.remote)
        .with_context(|| format!("parsing remote endpoint '{}'", args.remote))?;
    let uri = remote.control_plane_uri();
    let mut client = BlitClient::connect(uri.clone())
        .await
        .with_context(|| format!("connecting to {}", uri))?;

    let response = client
        .list_modules(ListModulesRequest {})
        .await
        .map_err(|status| eyre::eyre!(status.message().to_string()))?
        .into_inner();

    if args.json {
        let json_modules: Vec<_> = response
            .modules
            .iter()
            .map(|m| ModuleJson {
                name: &m.name,
                path: &m.path,
                read_only: m.read_only,
            })
            .collect();
        println!("{}", serde_json::to_string_pretty(&json_modules)?);
    } else if response.modules.is_empty() {
        println!("No modules exported by {}", remote.display());
    } else {
        println!("Modules on {}:", remote.display());
        for module in response.modules {
            let mode = if module.read_only { "ro" } else { "rw" };
            println!("{} ({})\t{}", module.name, mode, module.path);
        }
    }

    Ok(())
}

async fn run_ls(args: ListArgs) -> Result<()> {
    match parse_endpoint_or_local(&args.target) {
        Endpoint::Local(path) => list_local_path(&path, args.json),
        Endpoint::Remote(remote) => list_remote_path(remote, args.json).await,
    }
}

async fn run_rm(args: RmArgs) -> Result<()> {
    let remote = match parse_endpoint_or_local(&args.target) {
        Endpoint::Local(path) => {
            bail!(
                "`blit-utils rm` only supports remote paths (received local path: {})",
                path.display()
            );
        }
        Endpoint::Remote(remote) => remote,
    };

    let (module, rel_path) = match &remote.path {
        RemotePath::Module { module, rel_path } => (module.clone(), rel_path.clone()),
        RemotePath::Root { .. } => {
            bail!("removing paths from server:// exports is not supported yet; configure a module");
        }
        RemotePath::Discovery => {
            bail!("remote removal requires module syntax (e.g., server:/module/path)");
        }
    };

    if rel_path.as_os_str().is_empty() || rel_path == Path::new(".") {
        bail!(
            "refusing to delete entire module '{}'; specify a sub-path",
            module
        );
    }

    let rel_components: Vec<String> = rel_path
        .iter()
        .map(|component| component.to_string_lossy().into_owned())
        .collect();
    let rel_string = rel_components.join("/");
    if rel_string.is_empty() {
        bail!(
            "refusing to delete entire module '{}'; specify a sub-path",
            module
        );
    }

    let module_display = format!("{}:/{}", module, rel_string);
    let endpoint_display = format!("{}:{}", remote.host, remote.port);

    if !args.yes {
        print!("Delete {} on {}? [y/N]: ", module_display, endpoint_display);
        io::stdout().flush()?;
        let mut input = String::new();
        io::stdin().read_line(&mut input)?;
        let decision = input.trim().to_ascii_lowercase();
        if !(decision == "y" || decision == "yes") {
            println!("Aborted.");
            return Ok(());
        }
    }

    let uri = remote.control_plane_uri();
    let mut client = BlitClient::connect(uri.clone())
        .await
        .with_context(|| format!("connecting to {}", uri))?;

    let response = client
        .purge(PurgeRequest {
            module: module.clone(),
            paths_to_delete: vec![rel_string.clone()],
        })
        .await
        .map_err(|status| eyre::eyre!(status.message().to_string()))?
        .into_inner();

    match response.files_deleted {
        0 => println!(
            "No entries removed for {} on {}; path may already be absent.",
            module_display, endpoint_display
        ),
        1 => println!("Deleted {} on {}.", module_display, endpoint_display),
        count => println!(
            "Deleted {} entries under {} on {}.",
            count, module_display, endpoint_display
        ),
    };

    Ok(())
}

fn list_local_path(path: &Path, json: bool) -> Result<()> {
    let metadata =
        fs::metadata(path).with_context(|| format!("reading metadata for {}", path.display()))?;

    if json {
        let entries_json = if metadata.is_dir() {
            let mut entries = Vec::new();
            for entry in fs::read_dir(path)
                .with_context(|| format!("reading directory {}", path.display()))?
            {
                let entry =
                    entry.with_context(|| format!("reading entry in {}", path.display()))?;
                let meta = entry
                    .metadata()
                    .with_context(|| format!("reading metadata for {}", entry.path().display()))?;
                entries.push(DirEntryJson::from_fs(entry.file_name(), &meta));
            }
            entries
        } else {
            vec![DirEntryJson::from_path(path.file_name(), &metadata)]
        };
        println!("{}", serde_json::to_string_pretty(&entries_json)?);
        return Ok(());
    }

    if metadata.is_dir() {
        println!("Listing {}:", path.display());
        let mut entries: Vec<_> = fs::read_dir(path)
            .with_context(|| format!("reading directory {}", path.display()))?
            .collect::<Result<Vec<_>, _>>()
            .with_context(|| format!("collecting entries for {}", path.display()))?;
        entries.sort_by_key(|entry| entry.path());
        for entry in entries {
            let meta = entry
                .metadata()
                .with_context(|| format!("reading metadata for {}", entry.path().display()))?;
            let name = entry.file_name().to_string_lossy().into_owned();
            if meta.is_dir() {
                println!("DIR  {:>12} {}/", "-", name);
            } else {
                println!("FILE {:>12} {}", format_bytes(meta.len()), name);
            }
        }
    } else {
        println!(
            "FILE {:>12} {}",
            format_bytes(metadata.len()),
            path.display()
        );
    }
    Ok(())
}

async fn list_remote_path(remote: RemoteEndpoint, json: bool) -> Result<()> {
    let (module, rel_path) = match &remote.path {
        RemotePath::Module { module, rel_path } => (module.clone(), rel_path.clone()),
        RemotePath::Root { .. } => {
            bail!("listing root exports (server://...) is not supported yet");
        }
        RemotePath::Discovery => {
            bail!("listing a bare host requires `list-modules` or module/path syntax");
        }
    };

    let path_str = if rel_path.as_os_str().is_empty() {
        String::new()
    } else {
        rel_path
            .iter()
            .map(|component| component.to_string_lossy())
            .collect::<Vec<_>>()
            .join("/")
    };

    let uri = remote.control_plane_uri();
    let mut client = BlitClient::connect(uri.clone())
        .await
        .with_context(|| format!("connecting to {}", uri))?;
    let response = client
        .list(ListRequest {
            module: module.clone(),
            path: path_str.clone(),
        })
        .await
        .map_err(|status| eyre::eyre!(status.message().to_string()))?
        .into_inner();

    if json {
        let entries: Vec<_> = response
            .entries
            .iter()
            .map(DirEntryJson::from_proto)
            .collect();
        println!("{}", serde_json::to_string_pretty(&entries)?);
    } else if response.entries.is_empty() {
        println!(
            "No entries under {}:/{}",
            module,
            if path_str.is_empty() { "" } else { &path_str }
        );
    } else {
        println!(
            "Listing {}:/{}:",
            module,
            if path_str.is_empty() { "" } else { &path_str }
        );
        for entry in response.entries {
            let indicator = if entry.is_dir { "DIR " } else { "FILE" };
            let size_str = if entry.is_dir {
                "-".to_string()
            } else {
                format_bytes(entry.size)
            };
            println!("{} {:>12} {}", indicator, size_str, entry.name);
        }
    }

    Ok(())
}

fn run_profile(args: ProfileArgs) -> Result<()> {
    let enabled = perf_history::perf_history_enabled()?;
    let records = perf_history::read_recent_records(args.limit)?;
    let predictor_path = PerformancePredictor::load()
        .ok()
        .map(|pred| pred.path().to_path_buf())
        .filter(|p| p.exists());

    if args.json {
        let json = serde_json::json!({
            "enabled": enabled,
            "records": records,
            "predictor_path": predictor_path.map(|p| p.to_string_lossy().into_owned()),
        });
        println!("{}", serde_json::to_string_pretty(&json)?);
    } else {
        println!(
            "Performance history {} ({} record(s) loaded)",
            if enabled { "ENABLED" } else { "DISABLED" },
            records.len()
        );
        if let Some(path) = predictor_path {
            println!("Predictor state: {}", path.display());
        } else {
            println!("Predictor state: not initialised");
        }
    }

    Ok(())
}

#[derive(Serialize)]
struct ModuleJson<'a> {
    name: &'a str,
    path: &'a str,
    read_only: bool,
}

#[derive(Serialize)]
struct DirEntryJson {
    name: String,
    is_dir: bool,
    size: u64,
    mtime_seconds: i64,
}

impl DirEntryJson {
    fn from_proto(info: &FileInfo) -> Self {
        Self {
            name: info.name.clone(),
            is_dir: info.is_dir,
            size: info.size,
            mtime_seconds: info.mtime_seconds,
        }
    }

    fn from_fs(name: std::ffi::OsString, meta: &fs::Metadata) -> Self {
        let is_dir = meta.is_dir();
        let size = if is_dir { 0 } else { meta.len() };
        let mtime_seconds = metadata_mtime_seconds(meta).unwrap_or(0);
        Self {
            name: name.to_string_lossy().into_owned(),
            is_dir,
            size,
            mtime_seconds,
        }
    }

    fn from_path(name: Option<&std::ffi::OsStr>, meta: &fs::Metadata) -> Self {
        let default = std::ffi::OsStr::new(".");
        Self::from_fs(name.unwrap_or(default).to_os_string(), meta)
    }
}

enum Endpoint {
    Local(PathBuf),
    Remote(RemoteEndpoint),
}

fn parse_endpoint_or_local(input: &str) -> Endpoint {
    match RemoteEndpoint::parse(input) {
        Ok(endpoint) => Endpoint::Remote(endpoint),
        Err(_) => Endpoint::Local(PathBuf::from(input)),
    }
}

fn format_bytes(bytes: u64) -> String {
    const UNITS: [&str; 5] = ["B", "KiB", "MiB", "GiB", "TiB"];
    if bytes == 0 {
        return "0 B".to_string();
    }
    let mut size = bytes as f64;
    let mut unit = 0;
    while size >= 1024.0 && unit < UNITS.len() - 1 {
        size /= 1024.0;
        unit += 1;
    }
    format!("{size:.2} {}", UNITS[unit])
}

fn metadata_mtime_seconds(meta: &fs::Metadata) -> Option<i64> {
    use std::time::UNIX_EPOCH;

    let modified = meta.modified().ok()?;
    match modified.duration_since(UNIX_EPOCH) {
        Ok(duration) => Some(duration.as_secs() as i64),
        Err(err) => {
            let dur = err.duration();
            Some(-(dur.as_secs() as i64))
        }
    }
}
