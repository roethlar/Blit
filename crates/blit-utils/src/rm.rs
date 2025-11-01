use crate::cli::RmArgs;
use crate::util::{Endpoint, parse_endpoint_or_local};
use blit_core::generated::PurgeRequest;
use blit_core::generated::blit_client::BlitClient;
use blit_core::remote::endpoint::{RemoteEndpoint, RemotePath};
use eyre::{Context, Result, bail};
use std::io::{self, Write};
use std::path::Path;

pub async fn run_rm(args: RmArgs) -> Result<()> {
    let remote = match parse_endpoint_or_local(&args.target) {
        Endpoint::Local(path) => {
            bail!(
                "`blit-utils rm` only supports remote paths (received local path: {})",
                path.display()
            );
        }
        Endpoint::Remote(remote) => remote,
    };

    let (module, rel_path) = extract_module_and_path(&remote)?;

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

fn extract_module_and_path(remote: &RemoteEndpoint) -> Result<(String, std::path::PathBuf)> {
    match &remote.path {
        RemotePath::Module { module, rel_path } => Ok((module.clone(), rel_path.clone())),
        RemotePath::Root { .. } => {
            bail!("removing paths from server:// exports is not supported yet; configure a module")
        }
        RemotePath::Discovery => {
            bail!("remote removal requires module syntax (e.g., server:/module/path)")
        }
    }
}
