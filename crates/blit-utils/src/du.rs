use crate::cli::DuArgs;
use crate::util::{Endpoint, module_and_rel_path, parse_endpoint_or_local, rel_path_to_string};
use blit_core::generated::DiskUsageRequest;
use blit_core::generated::blit_client::BlitClient;
use eyre::{Context, Result, bail};
use serde::Serialize;

#[derive(Serialize)]
struct DiskUsageJsonRow {
    path: String,
    bytes: u64,
    files: u64,
    dirs: u64,
}

pub async fn run_du(args: DuArgs) -> Result<()> {
    let remote = match parse_endpoint_or_local(&args.target) {
        Endpoint::Local(path) => {
            bail!(
                "`blit-utils du` requires a remote path (received local path: {})",
                path.display()
            );
        }
        Endpoint::Remote(remote) => remote,
    };
    let (module, rel_path) = module_and_rel_path(&remote)?;
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
