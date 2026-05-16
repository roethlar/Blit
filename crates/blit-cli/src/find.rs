use crate::cli::FindArgs;
use blit_app::admin::find::{self, FindEntry, FindParams};
use blit_app::endpoints::{
    module_and_rel_path, parse_endpoint_or_local, rel_path_to_string, Endpoint,
};
use eyre::{bail, Result};

pub async fn run_find(args: FindArgs) -> Result<()> {
    let remote = match parse_endpoint_or_local(&args.target) {
        Endpoint::Local(path) => {
            bail!(
                "`blit find` requires a remote path (received local path: {})",
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

    let params = FindParams {
        module,
        start_path: rel_path_to_string(&rel_path),
        pattern: args.pattern.unwrap_or_default(),
        case_sensitive: !args.case_insensitive,
        include_files,
        include_directories: include_dirs,
        max_results: args.limit.unwrap_or(0),
    };

    if args.json {
        let mut rows: Vec<FindEntry> = Vec::new();
        find::stream(&remote, params, |entry| {
            rows.push(entry);
            Ok(())
        })
        .await?;
        println!("{}", serde_json::to_string_pretty(&rows)?);
    } else {
        println!("{:<48} {:>12} {:<5}", "PATH", "BYTES", "TYPE");
        find::stream(&remote, params, |entry| {
            let ty = if entry.is_dir { "dir" } else { "file" };
            let size = if entry.is_dir {
                "-".to_string()
            } else {
                entry.size.to_string()
            };
            println!("{:<48} {:>12} {:<5}", entry.path, size, ty);
            Ok(())
        })
        .await?;
    }

    Ok(())
}
