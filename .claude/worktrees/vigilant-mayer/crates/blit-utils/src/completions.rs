use crate::cli::CompletionArgs;
use crate::util::{
    Endpoint, append_completion_prefix, module_and_rel_path, parse_endpoint_or_local,
};
use blit_core::generated::CompletionRequest;
use blit_core::generated::blit_client::BlitClient;
use eyre::{Context, Result, bail};

pub async fn run_completions(args: CompletionArgs) -> Result<()> {
    let remote = match parse_endpoint_or_local(&args.target) {
        Endpoint::Local(path) => {
            bail!(
                "`blit-utils completions` requires a remote path (received local path: {})",
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
    let prefix = append_completion_prefix(&rel_path, args.prefix.as_deref());

    let uri = remote.control_plane_uri();
    let mut client = BlitClient::connect(uri.clone())
        .await
        .with_context(|| format!("connecting to {}", uri))?;

    let response = client
        .complete_path(CompletionRequest {
            module,
            path_prefix: prefix,
            include_files,
            include_directories: include_dirs,
        })
        .await
        .map_err(|status| eyre::eyre!(status.message().to_string()))?
        .into_inner();

    if response.completions.is_empty() {
        println!("(no matches)");
    } else {
        for item in response.completions {
            println!("{}", item);
        }
    }

    Ok(())
}
