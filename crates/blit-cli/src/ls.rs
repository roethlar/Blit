use crate::cli::ListArgs;
use blit_app::admin::ls::{self, DirEntry, LocalListing};
use blit_app::display::format_bytes;
use blit_app::endpoints::{parse_endpoint_or_local, rel_path_to_string, Endpoint};
use blit_core::remote::endpoint::{RemoteEndpoint, RemotePath};
use eyre::{bail, Result};
use std::path::Path;

pub async fn run_ls(args: ListArgs) -> Result<()> {
    match parse_endpoint_or_local(&args.target) {
        Endpoint::Local(path) => list_local_path(&path, args.json),
        Endpoint::Remote(remote) => list_remote_path(remote, args.json).await,
    }
}

fn list_local_path(path: &Path, json: bool) -> Result<()> {
    let listing = ls::list_local(path)?;

    if json {
        // Pre-A.0 the JSON path emitted a single-entry vec for the
        // non-directory case and the full sorted vec for the
        // directory case. `LocalListing::into_entries` preserves
        // that shape.
        println!("{}", serde_json::to_string_pretty(&listing.into_entries())?);
        return Ok(());
    }

    match listing {
        LocalListing::Target { entry } => {
            // Single non-directory target (file, device, FIFO,
            // socket). Pre-A.0 text printed the full path here,
            // not the basename — preserved.
            println!("FILE {:>12} {}", format_bytes(entry.size), path.display());
        }
        LocalListing::Directory { entries } => {
            println!("Listing {}:", path.display());
            for entry in entries {
                if entry.is_dir {
                    println!("DIR  {:>12} {}/", "-", entry.name);
                } else {
                    println!("FILE {:>12} {}", format_bytes(entry.size), entry.name);
                }
            }
        }
    }
    Ok(())
}

async fn list_remote_path(remote: RemoteEndpoint, json: bool) -> Result<()> {
    // Smart-dispatch: bare-host targets (no module, no path) list
    // modules, matching the v6 plan's `blit list server` semantics.
    // Module/path targets fall through to the directory-listing
    // path. The explicit `blit list-modules <host>` and
    // `blit ls <host:/module/path>` commands stay available;
    // `blit list <target>` (which aliases `ls`) just routes
    // intelligently between the two. (R41-prev / Plan §2.3.)
    if matches!(remote.path, RemotePath::Discovery) {
        return crate::list_modules::list_modules_remote(remote, json).await;
    }

    let (module, rel_path) = match &remote.path {
        RemotePath::Module { module, rel_path } => (module.clone(), rel_path.clone()),
        RemotePath::Root { rel_path } => (String::new(), rel_path.clone()),
        RemotePath::Discovery => {
            // Unreachable — handled above. Kept defensive in case
            // a future RemotePath variant lands.
            bail!("listing a bare host requires `list-modules` or module/path syntax");
        }
    };

    let path_str = rel_path_to_string(&rel_path);
    let entries: Vec<DirEntry> = ls::list_remote(&remote, module.clone(), path_str.clone()).await?;

    if json {
        println!("{}", serde_json::to_string_pretty(&entries)?);
    } else if entries.is_empty() {
        println!(
            "No entries under {}:/{}",
            module,
            if path_str.is_empty() { "" } else { &path_str }
        );
    } else {
        println!(
            "Listing {}:/{}:",
            module,
            if path_str.is_empty() { "" } else { &path_str }
        );
        for entry in entries {
            let indicator = if entry.is_dir { "DIR " } else { "FILE" };
            let size_str = if entry.is_dir {
                "-".to_string()
            } else {
                format_bytes(entry.size)
            };
            println!("{} {:>12} {}", indicator, size_str, entry.name);
        }
    }

    Ok(())
}
