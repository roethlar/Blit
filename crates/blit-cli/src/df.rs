//! `blit df` — `df`, run on a daemon's own disk
//! (`docs/plan/UTILITY_VERBS_LOCAL_PARITY.md` slice 1).
//!
//! The filesystem that holds the path given, as `df -k` prints it: the
//! operand in the first column and no "Mounted on" column (the mount
//! point is the daemon's business).

use std::process::ExitCode;

use crate::cli::DfArgs;
use blit_core::admin::df::{self, FilesystemSpace};
use blit_core::admin::operand::Operand;
use blit_core::disk_usage::{blocks_1k, human_size, use_percent};
use eyre::{bail, Result};

pub async fn run_df(args: DfArgs) -> Result<ExitCode> {
    // The operand as typed (cr-uv1-1, cr-uv1-4).
    let Some(operand) = Operand::read(&args.target)? else {
        bail!(
            "`blit df` requires a remote path (received local path: {})",
            args.target.to_string_lossy()
        );
    };
    let typed = operand.shown();
    let asked = df::query(&operand.remote, operand.module.clone(), operand.wire_path());
    let space = match asked.await {
        Ok(space) => space,
        // `df: PATH: No such file or directory`, and exit 1.
        Err(error) => {
            eprintln!("blit df: {typed}: {error}");
            return Ok(ExitCode::from(1));
        }
    };

    if args.json {
        println!("{}", serde_json::to_string_pretty(&space)?);
    } else {
        print!("{}", table(&typed, &space, args.human_readable));
    }
    Ok(ExitCode::SUCCESS)
}

/// `df`'s two lines for one filesystem: the header and the numbers, the
/// first column wide enough for the operand and each number right-aligned
/// under its heading.
fn table(operand: &str, space: &FilesystemSpace, human: bool) -> String {
    let size = |bytes: u64| {
        if human {
            human_size(bytes)
        } else {
            blocks_1k(bytes).to_string()
        }
    };
    let percent = match use_percent(&(*space).into()) {
        Some(percent) => format!("{percent}%"),
        None => "-".to_string(),
    };
    let headings = if human {
        ["Size", "Used", "Avail", "Use%"]
    } else {
        ["1K-blocks", "Used", "Available", "Use%"]
    };
    let values = [
        size(space.total_bytes),
        size(space.used_bytes),
        size(space.available_bytes),
        percent,
    ];
    let first = operand.len().max(14);
    let mut header = format!("{:<first$}", "Filesystem");
    let mut row = format!("{operand:<first$}");
    for (at, (heading, value)) in headings.iter().zip(&values).enumerate() {
        // `df -h` keeps its three sizes at least five wide.
        let least = if human && at < 3 { 5 } else { 0 };
        let width = heading.len().max(value.len()).max(least);
        header.push_str(&format!(" {heading:>width$}"));
        row.push_str(&format!(" {value:>width$}"));
    }
    format!("{header}\n{row}\n")
}

#[cfg(test)]
mod tests {
    use super::*;

    /// AC-1: the shape `df -k` prints, with the operand first and no
    /// "Mounted on" — the numbers below are a real `df -k` line for a
    /// 16 GiB filesystem (GNU coreutils 9.12, 2026-11-20), and the second
    /// table is its `df -h` line.
    #[test]
    fn the_table_is_dfs_for_one_filesystem() {
        let space = FilesystemSpace {
            total_bytes: 16_118_172 * 1024,
            used_bytes: 259_788 * 1024,
            available_bytes: 15_858_384 * 1024,
        };
        assert_eq!(
            table("tmpfs", &space, false),
            "Filesystem     1K-blocks   Used Available Use%\n\
             tmpfs           16118172 259788  15858384   2%\n"
        );
        assert_eq!(
            table("tmpfs", &space, true),
            "Filesystem      Size  Used Avail Use%\n\
             tmpfs            16G  254M   16G   2%\n"
        );
        // A long operand widens the first column for both lines.
        let wide = table("server.example:/module/some/path", &space, false);
        let lines: Vec<&str> = wide.lines().collect();
        assert_eq!(lines[0].len(), lines[1].len());
        assert!(lines[1].starts_with("server.example:/module/some/path  16118172"));
        // A filesystem of no size has no percentage.
        let empty = FilesystemSpace {
            total_bytes: 0,
            used_bytes: 0,
            available_bytes: 0,
        };
        assert!(table("x", &empty, false).ends_with("   -\n"));
    }
}
