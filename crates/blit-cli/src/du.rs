use crate::cli::DuArgs;
use crate::util::{module_and_rel_path, parse_endpoint_or_local, rel_path_to_string, Endpoint};
use blit_app::admin::du::{self, DiskUsageEntry};
use eyre::{bail, Result};

pub async fn run_du(args: DuArgs) -> Result<()> {
    let remote = match parse_endpoint_or_local(&args.target) {
        Endpoint::Local(path) => {
            bail!(
                "`blit du` requires a remote path (received local path: {})",
                path.display()
            );
        }
        Endpoint::Remote(remote) => remote,
    };
    let (module, rel_path) = module_and_rel_path(&remote)?;
    let start_path = rel_path_to_string(&rel_path);
    let max_depth = args.max_depth.unwrap_or(0);

    if args.json {
        let mut rows: Vec<DiskUsageEntry> = Vec::new();
        du::stream(&remote, module, start_path, max_depth, |entry| {
            rows.push(entry);
            Ok(())
        })
        .await?;
        println!("{}", serde_json::to_string_pretty(&rows)?);
    } else {
        println!(
            "{:<40} {:>12} {:>8} {:>8}",
            "PATH", "BYTES", "FILES", "DIRS"
        );
        du::stream(&remote, module, start_path, max_depth, |entry| {
            println!(
                "{:<40} {:>12} {:>8} {:>8}",
                entry.path, entry.bytes, entry.files, entry.dirs
            );
            Ok(())
        })
        .await?;
    }

    Ok(())
}
