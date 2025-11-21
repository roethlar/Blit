use crate::auto_tune::{determine_tuning, TuningParams};

/// Determine remote transfer tuning heuristics based on estimated byte counts.
pub fn determine_remote_tuning(total_bytes: u64) -> TuningParams {
    let default_chunk_bytes = if total_bytes >= 8 * 1024 * 1024 * 1024 {
        64 * 1024 * 1024
    } else if total_bytes >= 512 * 1024 * 1024 {
        32 * 1024 * 1024
    } else {
        16 * 1024 * 1024
    };

    let mut tuning = determine_tuning(default_chunk_bytes, None);
    let (initial_streams, max_streams) = if total_bytes >= 32 * 1024 * 1024 * 1024 {
        (24, 32)
    } else if total_bytes >= 8 * 1024 * 1024 * 1024 {
        (16, 24)
    } else if total_bytes >= 2 * 1024 * 1024 * 1024 {
        (12, 16)
    } else if total_bytes >= 512 * 1024 * 1024 {
        (8, 12)
    } else if total_bytes >= 128 * 1024 * 1024 {
        (6, 10)
    } else {
        (4, 8)
    };
    tuning.initial_streams = initial_streams;
    tuning.max_streams = max_streams;

    if total_bytes >= 8 * 1024 * 1024 * 1024 {
        tuning.tcp_buffer_size = Some(8 * 1024 * 1024);
        tuning.prefetch_count = Some(32);
    } else if total_bytes >= 512 * 1024 * 1024 {
        tuning.tcp_buffer_size = Some(4 * 1024 * 1024);
        tuning.prefetch_count = Some(16);
    }

    tuning
}
