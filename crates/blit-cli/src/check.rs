//! `blit check` — read-only tree comparison.
//!
//! The comparison algorithm + result types live in
//! `blit_app::check`. This module owns clap-arg handling,
//! filter construction (via `crate::transfers`), the
//! `spawn_blocking` lift, the JSON / text presenters, and the
//! exit-code policy:
//!   - 0 = identical
//!   - 1 = differences found
//!   - 2 = errors

use std::path::PathBuf;
use std::process::ExitCode;

use blit_app::check::{compare_trees, CheckResult};
use eyre::{bail, Context, Result};

use crate::cli::CheckArgs;
use crate::transfers::{build_filter_from_inputs, FilterInputs};

pub async fn run_check(args: &CheckArgs) -> Result<ExitCode> {
    let src = PathBuf::from(&args.source);
    let dst = PathBuf::from(&args.destination);
    if !src.exists() {
        bail!("source path does not exist: {}", src.display());
    }
    if !dst.exists() {
        bail!("destination path does not exist: {}", dst.display());
    }

    // Build filter via the same chokepoint that copy/mirror/move use, so
    // `blit check --exclude '*.tmp'` matches `blit copy --exclude '*.tmp'`.
    let filter = build_filter_from_inputs(&FilterInputs {
        include: &args.include,
        exclude: &args.exclude,
        files_from: args.files_from.as_ref(),
        min_size: args.min_size.as_deref(),
        max_size: args.max_size.as_deref(),
        min_age: args.min_age.as_deref(),
        max_age: args.max_age.as_deref(),
    })?;

    let use_checksum = args.checksum;
    let one_way = args.one_way;
    let json = args.json;

    let result = tokio::task::spawn_blocking(move || {
        compare_trees(&src, &dst, use_checksum, one_way, filter)
    })
    .await
    .context("check task panicked")??;

    if json {
        println!("{}", serde_json::to_string_pretty(&result)?);
    } else {
        print_result(&result, one_way);
    }

    if !result.errors.is_empty() {
        Ok(ExitCode::from(2))
    } else if !result.differing.is_empty()
        || !result.missing_on_dest.is_empty()
        || (!one_way && !result.missing_on_src.is_empty())
    {
        Ok(ExitCode::from(1))
    } else {
        Ok(ExitCode::SUCCESS)
    }
}

fn print_result(result: &CheckResult, one_way: bool) {
    let total_diffs = result.differing.len()
        + result.missing_on_dest.len()
        + if one_way {
            0
        } else {
            result.missing_on_src.len()
        };

    if total_diffs == 0 && result.errors.is_empty() {
        println!(
            "Check complete: {} files match, no differences found.",
            result.matching
        );
        return;
    }

    println!(
        "Check complete: {} matching, {} difference(s) found.",
        result.matching, total_diffs
    );

    if !result.differing.is_empty() {
        println!("\nDiffering ({}):", result.differing.len());
        for entry in &result.differing {
            println!("  * {} ({})", entry.path, entry.reason);
        }
    }
    if !result.missing_on_dest.is_empty() {
        println!(
            "\nMissing on destination ({}):",
            result.missing_on_dest.len()
        );
        for path in &result.missing_on_dest {
            println!("  + {path}");
        }
    }
    if !one_way && !result.missing_on_src.is_empty() {
        println!("\nMissing on source ({}):", result.missing_on_src.len());
        for path in &result.missing_on_src {
            println!("  - {path}");
        }
    }
    if !result.errors.is_empty() {
        println!("\nErrors ({}):", result.errors.len());
        for (path, err) in &result.errors {
            println!("  ! {path}: {err}");
        }
    }
}
