use crate::cli::ListArgs;
use crate::transfers::{format_bytes, parse_transfer_endpoint, Endpoint};
use blit_core::generated::blit_client::BlitClient;
use blit_core::generated::{ListModulesRequest, ListRequest};
use blit_core::remote::{RemoteEndpoint, RemotePath};
use eyre::{bail, eyre, Context, Result};
use std::path::{Path, PathBuf};

pub async fn run_list(args: &ListArgs) -> Result<()> {
    let endpoint = match parse_transfer_endpoint(&args.target) {
        Ok(endpoint) => endpoint,
        Err(err) => {
            // Propagate helpful error messages (like "use forward slashes")
            let err_msg = err.to_string();
            if err_msg.contains("forward slashes") {
                return Err(err);
            }
            // Treat as local path fallback
            let path = PathBuf::from(&args.target);
            if !path.exists() {
                bail!("path does not exist: {}", path.display());
            }
            list_local_path(&path)?;
            return Ok(());
        }
    };

    match endpoint {
        Endpoint::Local(path) => {
            if !path.exists() {
                bail!("path does not exist: {}", path.display());
            }
            list_local_path(&path)?;
            Ok(())
        }
        Endpoint::Remote(remote) => run_remote_list(remote).await,
    }
}

fn list_local_path(path: &Path) -> Result<()> {
    let metadata = std::fs::metadata(path)
        .with_context(|| format!("reading metadata for {}", path.display()))?;
    if metadata.is_file() {
        println!(
            "FILE {:>12} {}",
            format_bytes(metadata.len()),
            path.display()
        );
        return Ok(());
    }

    let mut entries: Vec<_> = std::fs::read_dir(path)
        .with_context(|| format!("reading directory {}", path.display()))?
        .collect::<Result<_, _>>()
        .with_context(|| format!("iterating directory {}", path.display()))?;
    entries.sort_by_key(|entry| entry.path());
    println!("Listing {}:", path.display());
    for entry in entries {
        let entry_path = entry.path();
        let meta = entry
            .metadata()
            .with_context(|| format!("metadata {}", entry_path.display()))?;
        let name = entry_path.file_name().unwrap_or_default().to_string_lossy();
        if meta.is_dir() {
            println!("DIR  {:>12} {}/", "-", name);
        } else {
            println!("FILE {:>12} {}", format_bytes(meta.len()), name);
        }
    }
    Ok(())
}

async fn run_remote_list(remote: RemoteEndpoint) -> Result<()> {
    let uri = remote.control_plane_uri();
    let mut client = BlitClient::connect(uri.clone())
        .await
        .with_context(|| format!("connecting to {}", uri))?;

    match &remote.path {
        RemotePath::Discovery => {
            let response = client
                .list_modules(ListModulesRequest {})
                .await
                .map_err(|status| eyre!(status.message().to_string()))?
                .into_inner();
            if response.modules.is_empty() {
                println!("No modules exported by {}", remote.display());
            } else {
                println!("Modules on {}:", remote.display());
                for module in response.modules {
                    println!(
                        "{}\t{}\t{}",
                        module.name,
                        module.path,
                        if module.read_only { "read-only" } else { "rw" }
                    );
                }
            }
            Ok(())
        }
        RemotePath::Module { module, rel_path } => {
            list_remote_path(&mut client, &remote, module.clone(), rel_path.clone()).await
        }
        RemotePath::Root { rel_path } => {
            list_remote_path(&mut client, &remote, String::new(), rel_path.clone()).await
        }
    }
}

async fn list_remote_path(
    client: &mut BlitClient<tonic::transport::Channel>,
    endpoint: &RemoteEndpoint,
    module: String,
    rel_path: PathBuf,
) -> Result<()> {
    let path_str = if rel_path.as_os_str().is_empty() {
        String::new()
    } else {
        rel_path
            .iter()
            .map(|component| component.to_string_lossy())
            .collect::<Vec<_>>()
            .join("/")
    };

    let response = client
        .list(ListRequest {
            module,
            path: path_str.clone(),
        })
        .await
        .map_err(|status| eyre!(status.message().to_string()))?
        .into_inner();

    if response.entries.is_empty() {
        println!("No entries under {}", endpoint.display());
    } else {
        println!("Listing {}:", endpoint.display());
        for entry in response.entries {
            let indicator = if entry.is_dir { "DIR " } else { "FILE" };
            println!(
                "{} {:>12} {}",
                indicator,
                if entry.is_dir {
                    "-".to_string()
                } else {
                    format_bytes(entry.size)
                },
                entry.name
            );
        }
    }

    Ok(())
}
