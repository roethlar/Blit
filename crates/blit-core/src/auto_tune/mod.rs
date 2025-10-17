//! Auto-tuning and adaptive defaults for network transfers
//!
//! Provides warmup probes and heuristics for chunk sizing, stream counts,
//! and buffer allocation based on network characteristics.

/// Tuning parameters determined by warmup and workload analysis
#[derive(Debug, Clone)]
pub struct TuningParams {
    /// Chunk size in bytes for network I/O
    pub chunk_bytes: usize,
    /// Initial number of parallel streams
    pub initial_streams: usize,
    /// Maximum parallel streams
    pub max_streams: usize,
    /// Detected bandwidth (if warmup succeeded)
    pub warmup_gbps: Option<f64>,
}

/// Analyze warmup results and determine optimal chunk size
///
/// Helper for interpreting warmup probe bandwidth measurements.
pub fn analyze_warmup_result(gbps: f64) -> usize {
    if gbps >= 6.0 {
        32 * 1024 * 1024 // High bandwidth
    } else {
        16 * 1024 * 1024 // Standard
    }
}

/// Determine tuning parameters based on plan and optional warmup
pub fn determine_tuning(
    default_chunk_bytes: usize,
    warmup_result: Option<(f64, usize)>,
    ludicrous: bool,
) -> TuningParams {
    let (warmup_gbps, chunk_bytes) = match warmup_result {
        Some((gbps, chunk)) => (Some(gbps), chunk),
        None => (None, default_chunk_bytes),
    };

    // Initial streams based on detected bandwidth
    let initial_streams = if let Some(gbps) = warmup_gbps {
        if gbps > 8.0 {
            6 // 10GbE or better
        } else if gbps > 3.0 {
            4 // Multi-gigabit
        } else {
            2 // Gigabit
        }
    } else if ludicrous {
        4
    } else {
        2
    };

    TuningParams {
        chunk_bytes,
        initial_streams,
        max_streams: 8,
        warmup_gbps,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tuning_with_high_bandwidth() {
        let params = determine_tuning(16 * 1024 * 1024, Some((9.5, 32 * 1024 * 1024)), false);
        assert_eq!(params.chunk_bytes, 32 * 1024 * 1024);
        assert_eq!(params.initial_streams, 6);
        assert_eq!(params.max_streams, 8);
    }

    #[test]
    fn test_tuning_fallback() {
        let params = determine_tuning(16 * 1024 * 1024, None, false);
        assert_eq!(params.chunk_bytes, 16 * 1024 * 1024);
        assert_eq!(params.initial_streams, 2);
    }

    #[test]
    fn test_ludicrous_mode() {
        let params = determine_tuning(16 * 1024 * 1024, None, true);
        assert_eq!(params.initial_streams, 4);
    }
}
