use crate::cli::{DiagnosticsDumpArgs, PerfArgs};
use crate::context::AppContext;
use crate::transfers::{
    dest_is_container, format_remote_endpoint, parse_transfer_endpoint, resolve_destination,
    source_is_contents, Endpoint,
};
use blit_core::fs_capability::cached_probe;
use blit_core::perf_history;
use blit_core::remote::RemotePath;
use chrono::{DateTime, Utc};
use eyre::Result;
use serde_json::{json, Value};
use std::path::Path;
use std::time::{Duration, UNIX_EPOCH};

pub fn run_diagnostics_perf(ctx: &mut AppContext, args: &PerfArgs) -> Result<()> {
    if args.enable {
        perf_history::set_perf_history_enabled(true)?;
        ctx.perf_history_enabled = true;
        println!("Performance history enabled (persisted).");
    }

    if args.disable {
        perf_history::set_perf_history_enabled(false)?;
        ctx.perf_history_enabled = false;
        println!("Performance history disabled (persisted).");
    }

    if args.clear {
        match perf_history::clear_history()? {
            true => println!("Cleared performance history log."),
            false => println!("No performance history log to clear."),
        }
    }

    // Refresh status from disk in case multiple toggles happened earlier.
    if let Ok(enabled) = perf_history::perf_history_enabled() {
        ctx.perf_history_enabled = enabled;
    }

    let history_path = perf_history::config_dir()?.join("perf_local.jsonl");
    let records = perf_history::read_recent_records(args.limit)?;

    if args.json {
        let output = json!({
            "enabled": ctx.perf_history_enabled,
            "history_path": history_path.to_string_lossy(),
            "record_count": records.len(),
            "records": records,
        });
        println!("{}", serde_json::to_string_pretty(&output)?);
        return Ok(());
    }

    println!(
        "Performance history (showing up to {} entries): {}",
        args.limit,
        records.len()
    );
    println!("History file: {}", history_path.display());
    println!(
        "Status: {}",
        if ctx.perf_history_enabled {
            if records.is_empty() {
                "enabled (no entries yet)"
            } else {
                "enabled"
            }
        } else {
            "disabled via CLI settings"
        }
    );

    if records.is_empty() {
        return Ok(());
    }

    let total_runs = records.len();
    let total_runs_f64 = total_runs as f64;
    let avg_planner = records
        .iter()
        .map(|r| r.planner_duration_ms as f64)
        .sum::<f64>()
        / total_runs_f64;
    let avg_transfer = records
        .iter()
        .map(|r| r.transfer_duration_ms as f64)
        .sum::<f64>()
        / total_runs_f64;
    let fast_path_runs = records.iter().filter(|r| r.fast_path.is_some()).count();
    let fast_pct = if total_runs == 0 {
        0.0
    } else {
        100.0 * fast_path_runs as f64 / total_runs_f64
    };

    println!(
        "Fast-path runs: {} ({:.1}%), streaming runs: {}",
        fast_path_runs,
        fast_pct,
        total_runs - fast_path_runs
    );
    println!(
        "Average planner: {:.1} ms | Average transfer: {:.1} ms",
        avg_planner, avg_transfer
    );

    if let Some(last) = records.last() {
        let millis = last.timestamp_epoch_ms.min(u64::MAX as u128) as u64;
        let timestamp = DateTime::<Utc>::from(UNIX_EPOCH + Duration::from_millis(millis));
        let mode = match last.mode {
            perf_history::TransferMode::Copy => "copy",
            perf_history::TransferMode::Mirror => "mirror",
        };
        let fast_path_label = last.fast_path.as_deref().unwrap_or("streaming");

        println!("Most recent run:");
        println!(
            "  Timestamp : {}",
            timestamp.format("%Y-%m-%d %H:%M:%S UTC")
        );
        println!("  Mode      : {}", mode);
        println!("  Fast path : {}", fast_path_label);
        println!(
            "  Planned   : {} file(s), {} bytes",
            last.file_count, last.total_bytes
        );
        println!(
            "  Planner   : {} ms | Transfer: {} ms",
            last.planner_duration_ms, last.transfer_duration_ms
        );
        println!(
            "  Options   : checksum={} skip_unchanged={} workers={}",
            last.options.checksum, last.options.skip_unchanged, last.options.workers
        );
        if let Some(fs) = &last.source_fs {
            println!("  Source FS : {}", fs);
        }
        if let Some(fs) = &last.dest_fs {
            println!("  Dest FS   : {}", fs);
        }
    }

    Ok(())
}

/// Emit a diagnostic snapshot for a SRC/DEST pair without performing a
/// transfer. Motivation: bug reporters and bisectors need a consistent
/// way to answer "what did blit see when you ran this?" — parse results,
/// rsync destination resolution, filesystem caps, disk space — without
/// reading source. One invocation → a single pasteable blob.
pub fn run_diagnostics_dump(args: &DiagnosticsDumpArgs) -> Result<()> {
    let src_endpoint = parse_transfer_endpoint(&args.source)?;
    let raw_dst = parse_transfer_endpoint(&args.destination)?;
    let pre_resolve_dst = raw_dst.clone();
    let resolved_dst = resolve_destination(&args.source, &args.destination, &src_endpoint, raw_dst);

    let source_contents_mode = source_is_contents(&args.source);
    let dest_is_container_flag = dest_is_container(&args.destination, &pre_resolve_dst);

    let src_json = endpoint_snapshot(&args.source, &src_endpoint);
    let dst_json = endpoint_snapshot(&args.destination, &resolved_dst);
    let pre_resolve_json = endpoint_display(&pre_resolve_dst);
    let resolved_display = endpoint_display(&resolved_dst);

    let same_device = same_device(&src_endpoint, &resolved_dst);

    let output = json!({
        "blit_version": env!("CARGO_PKG_VERSION"),
        "invocation": std::env::args().collect::<Vec<_>>(),
        "source": src_json,
        "destination": dst_json,
        "rsync_resolution": {
            "source_is_contents": source_contents_mode,
            "destination_is_container": dest_is_container_flag,
            "pre_resolve_destination": pre_resolve_json,
            "resolved_destination": resolved_display,
            "resolution_changed": pre_resolve_json != resolved_display,
        },
        "same_device": same_device,
    });

    if args.json {
        println!("{}", serde_json::to_string_pretty(&output)?);
    } else {
        print_dump_human(&output);
    }
    Ok(())
}

fn endpoint_snapshot(raw: &str, endpoint: &Endpoint) -> Value {
    match endpoint {
        Endpoint::Local(path) => local_path_snapshot(raw, path),
        Endpoint::Remote(remote) => {
            let (kind, module, rel_path) = match &remote.path {
                RemotePath::Module { module, rel_path } => (
                    "module",
                    Some(module.as_str().to_string()),
                    Some(rel_path.display().to_string()),
                ),
                RemotePath::Root { rel_path } => {
                    ("root", None, Some(rel_path.display().to_string()))
                }
                RemotePath::Discovery => ("discovery", None, None),
            };
            json!({
                "raw": raw,
                "kind": "remote",
                "host": remote.host.to_string(),
                "port": remote.port,
                "path_kind": kind,
                "module": module,
                "rel_path": rel_path,
                "display": format_remote_endpoint(remote),
            })
        }
    }
}

fn local_path_snapshot(raw: &str, path: &Path) -> Value {
    let abs_path = std::fs::canonicalize(path)
        .ok()
        .map(|p| p.display().to_string());
    let metadata = std::fs::metadata(path).ok();
    let exists = metadata.is_some();
    let is_file = metadata.as_ref().map(|m| m.is_file()).unwrap_or(false);
    let is_dir = metadata.as_ref().map(|m| m.is_dir()).unwrap_or(false);
    let size = metadata.as_ref().filter(|m| m.is_file()).map(|m| m.len());

    let caps = cached_probe(path);
    let (fs_type, reflink, block_clone) = caps
        .as_ref()
        .map(|c| {
            (
                c.filesystem_type.clone(),
                Some(c.reflink),
                Some(c.block_clone_same_volume),
            )
        })
        .unwrap_or((None, None, None));

    let (free_bytes, total_bytes) = disk_free_total(path);

    json!({
        "raw": raw,
        "kind": "local",
        "input_path": path.display().to_string(),
        "absolute_path": abs_path,
        "exists": exists,
        "is_file": is_file,
        "is_dir": is_dir,
        "size_bytes": size,
        "filesystem_type": fs_type,
        "reflink": reflink,
        "block_clone_same_volume": block_clone,
        "free_bytes": free_bytes,
        "total_bytes": total_bytes,
    })
}

fn endpoint_display(endpoint: &Endpoint) -> String {
    match endpoint {
        Endpoint::Local(p) => p.display().to_string(),
        Endpoint::Remote(r) => format_remote_endpoint(r),
    }
}

/// Returns (free_bytes, total_bytes) for the disk containing `path`, if
/// we can match the path against one of sysinfo's mount points. Returns
/// (None, None) if no match — better than a guess.
fn disk_free_total(path: &Path) -> (Option<u64>, Option<u64>) {
    // Walk up `path` until we find a prefix that matches a mount_point.
    // sysinfo's list of disks is not sorted, so find the longest match.
    let disks = sysinfo::Disks::new_with_refreshed_list();
    let canonical = std::fs::canonicalize(path).unwrap_or_else(|_| path.to_path_buf());
    let mut best: Option<(&sysinfo::Disk, usize)> = None;
    for disk in disks.iter() {
        let mp = disk.mount_point();
        if canonical.starts_with(mp) {
            let len = mp.as_os_str().len();
            if best.map_or(true, |(_, prev_len)| len > prev_len) {
                best = Some((disk, len));
            }
        }
    }
    match best {
        Some((disk, _)) => (Some(disk.available_space()), Some(disk.total_space())),
        None => (None, None),
    }
}

/// Same-device check: the biggest single predictor of reflink eligibility
/// and general zero-copy viability on Linux. Remote endpoints short-circuit
/// to `false` (no shared-device semantics across the wire).
fn same_device(src: &Endpoint, dst: &Endpoint) -> Option<bool> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::MetadataExt;
        let (Endpoint::Local(s), Endpoint::Local(d)) = (src, dst) else {
            return Some(false);
        };
        // Fall back to parent directory for dest because the resolved
        // path may not exist yet on a fresh target.
        let src_meta = std::fs::metadata(s).ok()?;
        let dst_meta = std::fs::metadata(d)
            .or_else(|_| d.parent().map_or_else(
                || Err(std::io::Error::from(std::io::ErrorKind::NotFound)),
                std::fs::metadata,
            ))
            .ok()?;
        Some(src_meta.dev() == dst_meta.dev())
    }
    #[cfg(not(unix))]
    {
        let _ = (src, dst);
        None
    }
}

fn print_dump_human(v: &Value) {
    println!("blit diagnostics dump");
    println!("  version     : {}", v["blit_version"].as_str().unwrap_or("?"));
    if let Some(invocation) = v["invocation"].as_array() {
        let joined: Vec<&str> = invocation.iter().filter_map(|s| s.as_str()).collect();
        println!("  invocation  : {}", joined.join(" "));
    }
    println!();
    println!("Source");
    print_endpoint_human(&v["source"]);
    println!();
    println!("Destination");
    print_endpoint_human(&v["destination"]);
    println!();
    let res = &v["rsync_resolution"];
    println!("Rsync resolution");
    println!("  source_is_contents     : {}", res["source_is_contents"].as_bool().unwrap_or(false));
    println!("  destination_is_container: {}", res["destination_is_container"].as_bool().unwrap_or(false));
    println!("  pre_resolve_destination: {}", res["pre_resolve_destination"].as_str().unwrap_or("?"));
    println!("  resolved_destination   : {}", res["resolved_destination"].as_str().unwrap_or("?"));
    println!("  resolution_changed     : {}", res["resolution_changed"].as_bool().unwrap_or(false));
    if let Some(same) = v["same_device"].as_bool() {
        println!();
        println!("Transport hints");
        println!("  same_device (local-only): {}", same);
    }
}

fn print_endpoint_human(v: &Value) {
    if v["kind"] == "local" {
        println!("  raw            : {}", v["raw"].as_str().unwrap_or("?"));
        println!("  absolute_path  : {}", v["absolute_path"].as_str().unwrap_or("(not canonicalized)"));
        println!(
            "  exists         : {}  is_file={} is_dir={}",
            v["exists"].as_bool().unwrap_or(false),
            v["is_file"].as_bool().unwrap_or(false),
            v["is_dir"].as_bool().unwrap_or(false)
        );
        if let Some(size) = v["size_bytes"].as_u64() {
            println!("  size_bytes     : {}", size);
        }
        if let Some(fs) = v["filesystem_type"].as_str() {
            println!("  filesystem     : {}", fs);
        }
        if let Some(reflink) = v["reflink"].as_bool() {
            println!("  reflink        : {}", reflink);
        }
        if let Some(free) = v["free_bytes"].as_u64() {
            let total = v["total_bytes"].as_u64().unwrap_or(0);
            println!(
                "  disk (free/total): {} / {} bytes",
                free, total
            );
        }
    } else {
        println!("  raw       : {}", v["raw"].as_str().unwrap_or("?"));
        println!("  display   : {}", v["display"].as_str().unwrap_or("?"));
        println!("  host      : {}", v["host"].as_str().unwrap_or("?"));
        if let Some(port) = v["port"].as_u64() {
            println!("  port      : {}", port);
        }
        println!("  path_kind : {}", v["path_kind"].as_str().unwrap_or("?"));
        if let Some(module) = v["module"].as_str() {
            println!("  module    : {}", module);
        }
        if let Some(rel) = v["rel_path"].as_str() {
            println!("  rel_path  : {}", rel);
        }
    }
}
