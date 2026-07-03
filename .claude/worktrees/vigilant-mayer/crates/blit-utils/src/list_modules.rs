use crate::cli::ListModulesArgs;
use eyre::{Context, Result};
use serde::Serialize;

use blit_core::generated::ListModulesRequest;
use blit_core::generated::blit_client::BlitClient;
use blit_core::remote::endpoint::RemoteEndpoint;

#[derive(Serialize)]
struct ModuleJson<'a> {
    name: &'a str,
    path: &'a str,
    read_only: bool,
}

pub async fn run_list_modules(args: ListModulesArgs) -> Result<()> {
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
