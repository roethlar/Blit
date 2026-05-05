use crate::cli::ProfileArgs;
use blit_core::perf_history;
use blit_core::perf_history::TransferMode;
use blit_core::perf_predictor::PerformancePredictor;
use eyre::Result;

pub fn run_profile(args: ProfileArgs) -> Result<()> {
    let enabled = perf_history::perf_history_enabled()?;
    let records = perf_history::read_recent_records(args.limit)?;
    let predictor = PerformancePredictor::load().ok();
    let predictor_path = predictor
        .as_ref()
        .map(|pred| pred.path().to_path_buf())
        .filter(|p| p.exists());

    // §2.8 phase 2: surface what the predictor actually believes
    // about the two canonical mode profiles so operators can audit
    // it without writing custom tooling. We probe with mode-only
    // (other key components default), letting the fallback chain
    // walk down to a profile that's actually been trained.
    let predictor_summary = predictor.as_ref().map(|pred| {
        let copy = profile_summary(pred, TransferMode::Copy);
        let mirror = profile_summary(pred, TransferMode::Mirror);
        (copy, mirror)
    });

    if args.json {
        let predictor_json = predictor_summary.as_ref().map(|(copy, mirror)| {
            serde_json::json!({
                "copy": coefficient_json(copy),
                "mirror": coefficient_json(mirror),
            })
        });
        let json = serde_json::json!({
            "enabled": enabled,
            "records": records,
            "predictor_path": predictor_path.as_ref().map(|p| p.to_string_lossy().into_owned()),
            "predictor": predictor_json,
        });
        println!("{}", serde_json::to_string_pretty(&json)?);
    } else {
        println!(
            "Performance history {} ({} record(s) loaded)",
            if enabled { "ENABLED" } else { "DISABLED" },
            records.len()
        );
        if let Some(path) = predictor_path {
            println!("Predictor state: {}", path.display());
        } else {
            println!("Predictor state: not initialised");
        }
        if let Some((copy, mirror)) = predictor_summary.as_ref() {
            print_coefficient_block("copy", copy);
            print_coefficient_block("mirror", mirror);
        }
    }

    Ok(())
}

/// What `coefficients_for(mode-only)` returns, with mode preserved
/// for display.
struct ProfileSummary {
    coefficients: Option<blit_core::perf_predictor::DurationCoefficients>,
    observations: u64,
    fallback_depth: usize,
}

fn profile_summary(predictor: &PerformancePredictor, mode: TransferMode) -> ProfileSummary {
    // Probe with all secondary key components defaulted: this
    // exercises the same fallback chain `predict()` walks but we're
    // looking for the broadest profile that has training data.
    match predictor.coefficients_for(mode, None, None, None, true, false) {
        Some((coeffs, obs, depth)) => ProfileSummary {
            coefficients: Some(coeffs),
            observations: obs,
            fallback_depth: depth,
        },
        None => ProfileSummary {
            coefficients: None,
            observations: 0,
            fallback_depth: 0,
        },
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
        println!("Predictor [{label}]: no profile yet (needs ≥5 observations)");
    }
}
