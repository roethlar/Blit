mod endpoints;
mod local;
mod remote;

pub use endpoints::{format_remote_endpoint, parse_transfer_endpoint, Endpoint};

use crate::cli::TransferArgs;
use crate::context::AppContext;
use eyre::{bail, Context, Result};
use std::fs;

use endpoints::{
    ensure_remote_destination_supported, ensure_remote_source_supported,
    ensure_remote_transfer_supported,
};
use local::run_local_transfer;
use remote::{run_remote_pull_transfer, run_remote_push_transfer};

#[derive(Copy, Clone)]
pub enum TransferKind {
    Copy,
    Mirror,
}

pub async fn run_transfer(ctx: &AppContext, args: &TransferArgs, mode: TransferKind) -> Result<()> {
    let src_endpoint = parse_transfer_endpoint(&args.source)?;
    let dst_endpoint = parse_transfer_endpoint(&args.destination)?;

    let operation = match mode {
        TransferKind::Copy => "copy",
        TransferKind::Mirror => "mirror",
    };
    let transfer_scope = match (&src_endpoint, &dst_endpoint) {
        (Endpoint::Local(src_path), Endpoint::Local(dst_path)) => {
            format!("{} -> {}", src_path.display(), dst_path.display())
        }
        (Endpoint::Local(src_path), Endpoint::Remote(remote)) => {
            format!(
                "{} -> {}",
                src_path.display(),
                format_remote_endpoint(remote)
            )
        }
        (Endpoint::Remote(remote), Endpoint::Local(dst_path)) => {
            format!(
                "{} -> {}",
                format_remote_endpoint(remote),
                dst_path.display()
            )
        }
        (Endpoint::Remote(a), Endpoint::Remote(b)) => {
            format!(
                "{} -> {}",
                format_remote_endpoint(a),
                format_remote_endpoint(b)
            )
        }
    };
    println!(
        "blit v{}: starting {} {}",
        env!("CARGO_PKG_VERSION"),
        operation,
        transfer_scope
    );

    match (src_endpoint, dst_endpoint) {
        (Endpoint::Local(src_path), Endpoint::Local(dst_path)) => {
            if !src_path.exists() {
                bail!("source path does not exist: {}", src_path.display());
            }
            run_local_transfer(
                ctx,
                args,
                &src_path,
                &dst_path,
                matches!(mode, TransferKind::Mirror),
            )
            .await
        }
        (Endpoint::Local(src_path), Endpoint::Remote(remote)) => {
            if !src_path.exists() {
                bail!("source path does not exist: {}", src_path.display());
            }
            ensure_remote_transfer_supported(args)?;
            ensure_remote_destination_supported(&remote)?;
            run_remote_push_transfer(
                ctx,
                args,
                Endpoint::Local(src_path),
                remote,
                matches!(mode, TransferKind::Mirror),
            )
            .await
        }
        (Endpoint::Remote(remote), Endpoint::Local(dst_path)) => {
            ensure_remote_transfer_supported(args)?;
            ensure_remote_source_supported(&remote)?;
            run_remote_pull_transfer(
                ctx,
                args,
                remote,
                &dst_path,
                matches!(mode, TransferKind::Mirror),
            )
            .await
        }
        (Endpoint::Remote(src), Endpoint::Remote(dst)) => {
            ensure_remote_transfer_supported(args)?;
            ensure_remote_source_supported(&src)?;
            ensure_remote_destination_supported(&dst)?;
            run_remote_push_transfer(
                ctx,
                args,
                Endpoint::Remote(src),
                dst,
                matches!(mode, TransferKind::Mirror),
            )
            .await
        }
    }
}

pub async fn run_move(ctx: &AppContext, args: &TransferArgs) -> Result<()> {
    let src_endpoint = parse_transfer_endpoint(&args.source)?;
    let dst_endpoint = parse_transfer_endpoint(&args.destination)?;

    match (src_endpoint, dst_endpoint) {
        (Endpoint::Local(src_path), Endpoint::Local(dst_path)) => {
            if args.dry_run {
                bail!("move does not support --dry-run");
            }
            if !src_path.exists() {
                bail!("source path does not exist: {}", src_path.display());
            }
            run_local_transfer(ctx, args, &src_path, &dst_path, true).await?;

            if src_path.is_dir() {
                fs::remove_dir_all(&src_path)
                    .with_context(|| format!("removing {}", src_path.display()))?;
            } else if src_path.is_file() {
                fs::remove_file(&src_path)
                    .with_context(|| format!("removing {}", src_path.display()))?;
            }
            Ok(())
        }
        (Endpoint::Remote(_), Endpoint::Remote(_))
        | (Endpoint::Remote(_), Endpoint::Local(_))
        | (Endpoint::Local(_), Endpoint::Remote(_)) => {
            bail!("remote moves are not supported yet")
        }
    }
}

pub(crate) fn format_bytes(bytes: u64) -> String {
    const UNITS: [&str; 5] = ["B", "KiB", "MiB", "GiB", "TiB"];
    if bytes == 0 {
        return "0 B".to_owned();
    }
    let mut value = bytes as f64;
    let mut unit = 0usize;
    while value >= 1024.0 && unit < UNITS.len() - 1 {
        value /= 1024.0;
        unit += 1;
    }
    if unit == 0 {
        format!("{} {}", bytes, UNITS[unit])
    } else {
        format!("{:.2} {}", value, UNITS[unit])
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    fn runtime() -> tokio::runtime::Runtime {
        tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .expect("runtime")
    }

    #[test]
    fn copy_local_transfers_file() -> Result<()> {
        let tmp = tempdir()?;
        let src = tmp.path().join("src");
        let dest = tmp.path().join("dest");
        std::fs::create_dir_all(&src)?;
        std::fs::write(src.join("hello.txt"), b"hello")?;
        let ctx = AppContext {
            perf_history_enabled: false,
        };

        let args = TransferArgs {
            source: src.to_string_lossy().into_owned(),
            destination: dest.to_string_lossy().into_owned(),
            dry_run: false,
            checksum: false,
            verbose: false,
            progress: false,
            workers: None,
            trace_data_plane: false,
            force_grpc: false,
        };

        runtime().block_on(run_local_transfer(&ctx, &args, &src, &dest, false))?;
        let copied = std::fs::read(dest.join("hello.txt"))?;
        assert_eq!(copied, b"hello");
        Ok(())
    }

    #[test]
    fn copy_local_dry_run_creates_no_files() -> Result<()> {
        let tmp = tempdir()?;
        let src = tmp.path().join("src");
        let dest = tmp.path().join("dest");
        std::fs::create_dir_all(&src)?;
        std::fs::write(src.join("hello.txt"), b"hello")?;
        let ctx = AppContext {
            perf_history_enabled: false,
        };

        let args = TransferArgs {
            source: src.to_string_lossy().into_owned(),
            destination: dest.to_string_lossy().into_owned(),
            dry_run: true,
            checksum: false,
            verbose: false,
            progress: false,
            workers: None,
            trace_data_plane: false,
            force_grpc: false,
        };

        runtime().block_on(run_local_transfer(&ctx, &args, &src, &dest, false))?;
        assert!(!dest.join("hello.txt").exists());
        Ok(())
    }
}
