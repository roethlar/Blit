use crate::cli::DfArgs;
use blit_app::admin::df;
use blit_app::display::format_bytes;
use blit_app::endpoints::{module_and_rel_path, parse_endpoint_or_local, Endpoint};
use eyre::{bail, Result};

pub async fn run_df(args: DfArgs) -> Result<()> {
    let remote = match parse_endpoint_or_local(&args.remote) {
        Endpoint::Local(path) => {
            bail!(
                "`blit df` requires a remote module (received local path: {})",
                path.display()
            );
        }
        Endpoint::Remote(remote) => remote,
    };
    let (module, _) = module_and_rel_path(&remote)?;
    let stats = df::query(&remote, module).await?;

    if args.json {
        println!("{}", serde_json::to_string_pretty(&stats)?);
    } else {
        println!("Module: {}", stats.module);
        println!(
            "Total: {} ({} bytes)",
            format_bytes(stats.total_bytes),
            stats.total_bytes
        );
        println!(
            "Used : {} ({} bytes)",
            format_bytes(stats.used_bytes),
            stats.used_bytes
        );
        println!(
            "Free : {} ({} bytes)",
            format_bytes(stats.free_bytes),
            stats.free_bytes
        );
    }

    Ok(())
}
