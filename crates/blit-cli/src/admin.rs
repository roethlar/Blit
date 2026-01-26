use crate::cli::{DfArgs, DuArgs, FindArgs, RmArgs};
use crate::context::AppContext;
use crate::transfers::{parse_transfer_endpoint, Endpoint};
use blit_core::generated::blit_client::BlitClient;
use blit_core::generated::{DiskUsageRequest, FilesystemStatsRequest, FindRequest, PurgeRequest};
use blit_core::remote::{RemoteEndpoint, RemotePath};
use eyre::{bail, Context, Result};
use serde::Serialize;
use std::io::{self, Write};
use std::path::PathBuf;

#[derive(Serialize)]
struct DiskUsageJsonRow {
    path: String,
    bytes: u64,
    files: u64,
    dirs: u64,
}

pub async fn run_du(_ctx: &AppContext, args: &DuArgs) -> Result<()> {
    let remote = match parse_transfer_endpoint(&args.target)? {
        Endpoint::Local(path) => {
            bail!(
                "`blit du` requires a remote path (received local path: {})",
                path.display()
            );
        }
        Endpoint::Remote(remote) => remote,
    };

    let (module, rel_path) = extract_module_and_path(&remote)?;
    let uri = remote.control_plane_uri();
    let mut client = BlitClient::connect(uri.clone())
        .await
        .with_context(|| format!("connecting to {}", uri))?;

    let request = DiskUsageRequest {
        module: module.clone(),
        start_path: rel_path_to_string(&rel_path),
        max_depth: args.max_depth.unwrap_or(0),
    };

    let mut stream = client
        .disk_usage(request)
        .await
        .map_err(|status| eyre::eyre!(status.message().to_string()))?
        .into_inner();

    if args.json {
        let mut rows = Vec::new();
        while let Some(entry) = stream
            .message()
            .await
            .map_err(|status| eyre::eyre!(status.message().to_string()))?
        {
            rows.push(DiskUsageJsonRow {
                path: entry.relative_path,
                bytes: entry.byte_total,
                files: entry.file_count,
                dirs: entry.dir_count,
            });
        }
        println!("{}", serde_json::to_string_pretty(&rows)?);
    } else {
        println!(
            "{:<40} {:>12} {:>8} {:>8}",
            "PATH", "BYTES", "FILES", "DIRS"
        );
        while let Some(entry) = stream
            .message()
            .await
            .map_err(|status| eyre::eyre!(status.message().to_string()))?
        {
            println!(
                "{:<40} {:>12} {:>8} {:>8}",
                entry.relative_path, entry.byte_total, entry.file_count, entry.dir_count
            );
        }
    }

    Ok(())
}

#[derive(Serialize)]
struct FilesystemStatsJson {
    module: String,
    total_bytes: u64,
    used_bytes: u64,
    free_bytes: u64,
}

pub async fn run_df(_ctx: &AppContext, args: &DfArgs) -> Result<()> {
    let remote = match parse_transfer_endpoint(&args.remote)? {
        Endpoint::Local(path) => {
            bail!(
                "`blit df` requires a remote module (received local path: {})",
                path.display()
            );
        }
        Endpoint::Remote(remote) => remote,
    };
    let (module, _) = extract_module_and_path(&remote)?;
    let uri = remote.control_plane_uri();
    let mut client = BlitClient::connect(uri.clone())
        .await
        .with_context(|| format!("connecting to {}", uri))?;

    let response = client
        .filesystem_stats(FilesystemStatsRequest {
            module: module.clone(),
        })
        .await
        .map_err(|status| eyre::eyre!(status.message().to_string()))?
        .into_inner();

    if args.json {
        let json = FilesystemStatsJson {
            module: response.module,
            total_bytes: response.total_bytes,
            used_bytes: response.used_bytes,
            free_bytes: response.free_bytes,
        };
        println!("{}", serde_json::to_string_pretty(&json)?);
    } else {
        println!("Module: {}", response.module);
        println!("Total: {} bytes", response.total_bytes);
        println!("Used : {} bytes", response.used_bytes);
        println!("Free : {} bytes", response.free_bytes);
    }

    Ok(())
}

pub async fn run_rm(_ctx: &AppContext, args: &RmArgs) -> Result<()> {
    let remote = match parse_transfer_endpoint(&args.target)? {
        Endpoint::Local(path) => {
            bail!(
                "`blit rm` only supports remote paths (received local path: {})",
                path.display()
            );
        }
        Endpoint::Remote(remote) => remote,
    };

    let (module, rel_path) = extract_module_and_path(&remote)?;

    if rel_path.as_os_str().is_empty() || rel_path == std::path::Path::new(".") {
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

    let deleted = delete_remote_path(&remote, &rel_string).await?;

    match deleted {
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

#[derive(Serialize)]
struct FindJsonRow {
    path: String,
    is_dir: bool,
    size: u64,
    mtime_seconds: i64,
}

pub async fn run_find(_ctx: &AppContext, args: &FindArgs) -> Result<()> {
    let remote = match parse_transfer_endpoint(&args.target)? {
        Endpoint::Local(path) => {
            bail!(
                "`blit find` requires a remote path (received local path: {})",
                path.display()
            );
        }
        Endpoint::Remote(remote) => remote,
    };

    let (module, rel_path) = extract_module_and_path(&remote)?;
    let include_files = if args.files || args.dirs {
        args.files
    } else {
        true
    };
    let include_dirs = if args.files || args.dirs {
        args.dirs
    } else {
        true
    };
    let start_path = rel_path_to_string(&rel_path);
    let request = FindRequest {
        module: module.clone(),
        start_path,
        pattern: args.pattern.clone().unwrap_or_default(),
        case_sensitive: !args.case_insensitive,
        include_files,
        include_directories: include_dirs,
        max_results: args.limit.unwrap_or(0),
    };

    let uri = remote.control_plane_uri();
    let mut client = BlitClient::connect(uri.clone())
        .await
        .with_context(|| format!("connecting to {}", uri))?;

    let mut stream = client
        .find(request)
        .await
        .map_err(|status| eyre::eyre!(status.message().to_string()))?
        .into_inner();

    if args.json {
        let mut rows = Vec::new();
        while let Some(entry) = stream
            .message()
            .await
            .map_err(|status| eyre::eyre!(status.message().to_string()))?
        {
            rows.push(FindJsonRow {
                path: entry.relative_path,
                is_dir: entry.is_dir,
                size: entry.size,
                mtime_seconds: entry.mtime_seconds,
            });
        }
        println!("{}", serde_json::to_string_pretty(&rows)?);
    } else {
        println!("{:<48} {:>12} {:<5}", "PATH", "BYTES", "TYPE");
        while let Some(entry) = stream
            .message()
            .await
            .map_err(|status| eyre::eyre!(status.message().to_string()))?
        {
            let ty = if entry.is_dir { "dir" } else { "file" };
            let size = if entry.is_dir {
                "-".to_string()
            } else {
                entry.size.to_string()
            };
            println!("{:<48} {:>12} {:<5}", entry.relative_path, size, ty);
        }
    }

    Ok(())
}

pub async fn delete_remote_path(remote: &RemoteEndpoint, rel_path: &str) -> Result<u64> {
    let (module, _) = extract_module_and_path(remote)?;
    let uri = remote.control_plane_uri();
    let mut client = BlitClient::connect(uri.clone())
        .await
        .with_context(|| format!("connecting to {}", uri))?;

    let response = client
        .purge(PurgeRequest {
            module,
            paths_to_delete: vec![rel_path.to_string()],
        })
        .await
        .map_err(|status| eyre::eyre!(status.message().to_string()))?
        .into_inner();

    Ok(response.files_deleted)
}

fn extract_module_and_path(remote: &RemoteEndpoint) -> Result<(String, PathBuf)> {
    match &remote.path {
        RemotePath::Module { module, rel_path } => Ok((module.clone(), rel_path.clone())),
        RemotePath::Root { .. } => {
            bail!("removing/querying paths from server:// exports is not supported yet; configure a module")
        }
        RemotePath::Discovery => {
            bail!("remote operation requires module syntax (e.g., server:/module/path)")
        }
    }
}

fn rel_path_to_string(path: &PathBuf) -> String {
    path.iter()
        .map(|c| c.to_string_lossy())
        .collect::<Vec<_>>()
        .join("/")
}
