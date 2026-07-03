use crate::cli::FindArgs;
use crate::util::{Endpoint, module_and_rel_path, parse_endpoint_or_local, rel_path_to_string};
use blit_core::generated::FindRequest;
use blit_core::generated::blit_client::BlitClient;
use eyre::{Context, Result, bail};
use serde::Serialize;

#[derive(Serialize)]
struct FindJsonRow {
    path: String,
    is_dir: bool,
    size: u64,
    mtime_seconds: i64,
}

pub async fn run_find(args: FindArgs) -> Result<()> {
    let remote = match parse_endpoint_or_local(&args.target) {
        Endpoint::Local(path) => {
            bail!(
                "`blit-utils find` requires a remote path (received local path: {})",
                path.display()
            );
        }
        Endpoint::Remote(remote) => remote,
    };

    let (module, rel_path) = module_and_rel_path(&remote)?;
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
        pattern: args.pattern.unwrap_or_default(),
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
