use crate::cli::ProfileArgs;
use blit_core::profile::{self, PredictorReport, ProfileSummary};
use eyre::Result;

pub fn run_profile(args: ProfileArgs) -> Result<()> {
    let report = profile::query(args.limit)?;

    if args.json {
        let predictor_json = report.predictor.as_ref().map(|pred| {
            serde_json::json!({
                "copy": coefficient_json(&pred.copy),
                "mirror": coefficient_json(&pred.mirror),
            })
        });
        let json = serde_json::json!({
            "enabled": report.enabled,
            "records": report.records,
            "aggregates": report.aggregates,
            "daemon_history_path": report.daemon_history_path.as_ref().map(|p| p.to_string_lossy().into_owned()),
            "daemon_note": report.daemon_note,
            "predictor_path": report.predictor_path.as_ref().map(|p| p.to_string_lossy().into_owned()),
            "predictor": predictor_json,
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
        if let Some(path) = report.predictor_path {
            println!("Predictor state: {}", path.display());
        } else {
            println!("Predictor state: not initialised");
        }
        if let Some(pred) = report.predictor.as_ref() {
            print_predictor_block(pred);
        }
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

fn coefficient_json(summary: &ProfileSummary) -> serde_json::Value {
    if let Some(coeffs) = summary.coefficients.as_ref() {
        serde_json::json!({
            "observations": summary.observations,
            "fallback_depth": summary.fallback_depth,
            "planner": {
                "alpha_ms_per_file": coeffs.planner.alpha_ms_per_file,
                "beta_ms_per_mb": coeffs.planner.beta_ms_per_mb,
                "gamma_ms": coeffs.planner.gamma_ms,
            },
            "transfer": {
                "alpha_ms_per_file": coeffs.transfer.alpha_ms_per_file,
                "beta_ms_per_mb": coeffs.transfer.beta_ms_per_mb,
                "gamma_ms": coeffs.transfer.gamma_ms,
            },
        })
    } else {
        serde_json::json!(null)
    }
}

fn print_predictor_block(pred: &PredictorReport) {
    print_coefficient_block("copy", &pred.copy);
    print_coefficient_block("mirror", &pred.mirror);
}

fn print_coefficient_block(label: &str, summary: &ProfileSummary) {
    if let Some(coeffs) = summary.coefficients.as_ref() {
        println!(
            "Predictor [{label}]: n={}, fallback_depth={}",
            summary.observations, summary.fallback_depth
        );
        println!(
            "  planner  : {:>9.4} ms/file + {:>9.4} ms/MiB + {:>7.2} ms",
            coeffs.planner.alpha_ms_per_file,
            coeffs.planner.beta_ms_per_mb,
            coeffs.planner.gamma_ms
        );
        println!(
            "  transfer : {:>9.4} ms/file + {:>9.4} ms/MiB + {:>7.2} ms",
            coeffs.transfer.alpha_ms_per_file,
            coeffs.transfer.beta_ms_per_mb,
            coeffs.transfer.gamma_ms
        );
    } else {
        println!(
            "Predictor [{label}]: no profile (historical — training \
             retired with the engine at otp-11b; persisted profiles \
             still display)"
        );
    }
}
