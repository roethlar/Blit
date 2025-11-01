use crate::cli::DfArgs;
use crate::util::{Endpoint, module_and_rel_path, parse_endpoint_or_local};
use blit_core::generated::FilesystemStatsRequest;
use blit_core::generated::blit_client::BlitClient;
use eyre::{Context, Result, bail};
use serde::Serialize;

#[derive(Serialize)]
struct FilesystemStatsJson {
    module: String,
    total_bytes: u64,
    used_bytes: u64,
    free_bytes: u64,
}

pub async fn run_df(args: DfArgs) -> Result<()> {
    let remote = match parse_endpoint_or_local(&args.remote) {
        Endpoint::Local(path) => {
            bail!(
                "`blit-utils df` requires a remote module (received local path: {})",
                path.display()
            );
        }
        Endpoint::Remote(remote) => remote,
    };
    let (module, _) = module_and_rel_path(&remote)?;
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
