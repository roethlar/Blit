use crate::cli::ListModulesArgs;
use blit_app::admin::list_modules;
use blit_core::remote::endpoint::RemoteEndpoint;
use eyre::{Context, Result};

pub async fn run_list_modules(args: ListModulesArgs) -> Result<()> {
    let remote = RemoteEndpoint::parse(&args.remote)
        .with_context(|| format!("parsing remote endpoint '{}'", args.remote))?;
    list_modules_remote(remote, args.json).await
}

/// Core "list modules on this remote" logic. Shared between
/// `blit list-modules <remote>` (the explicit form) and
/// `blit list <bare-host>` smart-dispatch in `ls::run_ls`. Keeping
/// the two entry points routed through a single function ensures the
/// two surfaces print exactly the same output and `--json` shape.
pub(crate) async fn list_modules_remote(remote: RemoteEndpoint, json: bool) -> Result<()> {
    let modules = list_modules::query(&remote).await?;

    if json {
        println!("{}", serde_json::to_string_pretty(&modules)?);
    } else if modules.is_empty() {
        println!("No modules exported by {}", remote.display());
    } else {
        println!("Modules on {}:", remote.display());
        for module in modules {
            let mode = if module.read_only { "ro" } else { "rw" };
            println!("{} ({})\t{}", module.name, mode, module.path);
        }
    }

    Ok(())
}
