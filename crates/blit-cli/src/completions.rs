use crate::cli::{Cli, CompletionArgs, CompletionKind, RemoteCompletionArgs, ShellCompletionArgs};
use crate::util::{
    append_completion_prefix, module_and_rel_path, parse_endpoint_or_local, Endpoint,
};
use blit_core::generated::blit_client::BlitClient;
use blit_core::generated::CompletionRequest;
use clap::CommandFactory;
use eyre::{bail, Context, Result};

pub async fn run_completions(args: CompletionArgs) -> Result<()> {
    match args.kind {
        CompletionKind::Shell(shell_args) => run_shell_completions(shell_args),
        CompletionKind::Remote(remote_args) => run_remote_completions(remote_args).await,
    }
}

/// Generate a static shell-completion script for the requested shell
/// and write it to stdout. clap_complete handles every shell listed
/// in `clap_complete::Shell`; users pipe the output to their shell's
/// completion-loading directory. Closes the README:33 promise of
/// "shell completions" with real script generation, not just the
/// remote-path-completion RPC (which is now under `completions
/// remote`).
fn run_shell_completions(args: ShellCompletionArgs) -> Result<()> {
    let mut cmd = Cli::command();
    let bin_name = "blit";
    clap_complete::generate(args.shell, &mut cmd, bin_name, &mut std::io::stdout());
    Ok(())
}

async fn run_remote_completions(args: RemoteCompletionArgs) -> Result<()> {
    let remote = match parse_endpoint_or_local(&args.target) {
        Endpoint::Local(path) => {
            bail!(
                "`blit completions remote` requires a remote path (received local path: {})",
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
