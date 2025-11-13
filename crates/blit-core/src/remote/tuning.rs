use crate::auto_tune::{determine_tuning, TuningParams};

/// Determine remote transfer tuning heuristics based on estimated byte counts.
pub fn determine_remote_tuning(total_bytes: u64) -> TuningParams {
    let default_chunk_bytes = if total_bytes >= 8 * 1024 * 1024 * 1024 {
        32 * 1024 * 1024
    } else if total_bytes >= 512 * 1024 * 1024 {
        16 * 1024 * 1024
    } else {
        8 * 1024 * 1024
    };
    determine_tuning(default_chunk_bytes, None)
}
