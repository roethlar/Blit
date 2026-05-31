use crate::cli::RmArgs;
use blit_app::admin::rm;
use blit_app::endpoints::{parse_endpoint_or_local, Endpoint};
use eyre::{bail, Result};
use serde::Serialize;
use std::io::{self, Write};
use std::path::Path;

// Re-export the helper used by `crate::transfers::mod::run_move`
// so existing call sites stay working without an import update.
pub use blit_app::admin::rm::delete_remote_path;

pub async fn run_rm(args: RmArgs) -> Result<()> {
    let remote = match parse_endpoint_or_local(&args.target) {
        Endpoint::Local(path) => {
            bail!(
                "`blit rm` only supports remote paths (received local path: {})",
                path.display()
            );
        }
        Endpoint::Remote(remote) => remote,
    };

    let (module, rel_path) = rm::extract_module_and_path(&remote)?;

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

    let entries_deleted = rm::purge(&remote, module.clone(), vec![rel_string.clone()]).await?;

    if args.json {
        #[derive(Serialize)]
        struct RmResult {
            path: String,
            host: String,
            port: u16,
            entries_deleted: u64,
        }
        let result = RmResult {
            path: rel_string,
            host: remote.host.clone(),
            port: remote.port,
            entries_deleted,
        };
        println!("{}", serde_json::to_string_pretty(&result)?);
    } else {
        match entries_deleted {
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
    }

    Ok(())
}
