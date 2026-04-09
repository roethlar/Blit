use crate::cli::ProfileArgs;
use blit_core::perf_history;
use blit_core::perf_predictor::PerformancePredictor;
use eyre::Result;

pub fn run_profile(args: ProfileArgs) -> Result<()> {
    let enabled = perf_history::perf_history_enabled()?;
    let records = perf_history::read_recent_records(args.limit)?;
    let predictor_path = PerformancePredictor::load()
        .ok()
        .map(|pred| pred.path().to_path_buf())
        .filter(|p| p.exists());

    if args.json {
        let json = serde_json::json!({
            "enabled": enabled,
            "records": records,
            "predictor_path": predictor_path.map(|p| p.to_string_lossy().into_owned()),
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
    }

    Ok(())
}
