use crate::cli::ProfileArgs;
use blit_core::profile;
use eyre::Result;

pub fn run_profile(args: ProfileArgs) -> Result<()> {
    let report = profile::query(args.limit)?;

    if args.json {
        // ph-3 (R1, D-2026-08-20-2): the `predictor` and
        // `predictor_path` keys retired with the predictor itself —
        // an owner-ruled output change. `blit_utils.rs` pins their
        // absence.
        let json = serde_json::json!({
            "enabled": report.enabled,
            "records": report.records,
            "aggregates": report.aggregates,
            "daemon_history_path": report.daemon_history_path.as_ref().map(|p| p.to_string_lossy().into_owned()),
            "daemon_note": report.daemon_note,
        });
        println!("{}", serde_json::to_string_pretty(&json)?);
    } else {
        println!(
            "Performance history {} ({} record(s) loaded)",
            if report.enabled {
                "ENABLED"
            } else {
                "DISABLED"
            },
            report.records.len()
        );
        if let Some(path) = report.daemon_history_path.as_ref() {
            println!("Daemon history merged from: {}", path.display());
        }
        if let Some(note) = report.daemon_note.as_ref() {
            println!("Warning: {note}");
        }
        print_aggregates(&report.aggregates);
    }

    Ok(())
}

/// Per-key aggregate lines (ph-2): one row per
/// `(origin, topology, role, initiator, peer_key)` group, so a
/// daemon-served run and an operator-driven run to the same peer never
/// blend into one number.
pub(crate) fn print_aggregates(aggregates: &[blit_core::profile::RouteAggregate]) {
    if aggregates.is_empty() {
        return;
    }
    println!("Per-route aggregates ({} key(s)):", aggregates.len());
    for agg in aggregates {
        println!(
            "  [{}] {}/{}/{} key={} — {} run(s) ({} real), {} file(s), {:.1} MiB, avg transfer {:.1} ms",
            agg.origin.label(),
            agg.topology.label(),
            agg.local_role.label(),
            agg.initiator.label(),
            agg.peer_key.as_deref().unwrap_or("-"),
            agg.runs,
            agg.real_runs,
            agg.total_files,
            agg.total_bytes as f64 / (1024.0 * 1024.0),
            agg.avg_transfer_ms,
        );
    }
}
