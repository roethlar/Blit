use crate::auto_tune::TuningParams;

/// Determine remote transfer tuning from the estimated byte count.
///
/// One honest static table (w2-1): chunk size, stream counts, and
/// socket sizing are keyed on `total_bytes` only — there is no
/// runtime bandwidth adaptation (a real warmup probe is H10b-class
/// future work). Note the stream ladder here is the *client's*
/// authority; the daemon's push negotiation currently runs its own
/// ladder and wins (single-owner consolidation is w2-2).
pub fn determine_remote_tuning(total_bytes: u64) -> TuningParams {
    let chunk_bytes = if total_bytes >= 8 * 1024 * 1024 * 1024 {
        64 * 1024 * 1024
    } else if total_bytes >= 512 * 1024 * 1024 {
        32 * 1024 * 1024
    } else {
        16 * 1024 * 1024
    };

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

    let (tcp_buffer_size, prefetch_count) = if total_bytes >= 8 * 1024 * 1024 * 1024 {
        (Some(8 * 1024 * 1024), Some(32))
    } else if total_bytes >= 512 * 1024 * 1024 {
        (Some(4 * 1024 * 1024), Some(16))
    } else {
        (None, None)
    };

    TuningParams {
        chunk_bytes,
        initial_streams,
        max_streams,
        tcp_buffer_size,
        prefetch_count,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const MIB: u64 = 1024 * 1024;
    const GIB: u64 = 1024 * MIB;

    #[test]
    fn small_transfers_get_the_floor_tier() {
        let t = determine_remote_tuning(10 * MIB);
        assert_eq!(t.chunk_bytes, 16 * 1024 * 1024);
        assert_eq!((t.initial_streams, t.max_streams), (4, 8));
        assert_eq!(t.tcp_buffer_size, None);
        assert_eq!(t.prefetch_count, None);
    }

    #[test]
    fn mid_tier_scales_chunk_streams_and_buffers_together() {
        let t = determine_remote_tuning(GIB);
        assert_eq!(t.chunk_bytes, 32 * 1024 * 1024);
        assert_eq!((t.initial_streams, t.max_streams), (8, 12));
        assert_eq!(t.tcp_buffer_size, Some(4 * 1024 * 1024));
        assert_eq!(t.prefetch_count, Some(16));
    }

    #[test]
    fn large_transfers_get_the_64mib_chunk_tier() {
        let t = determine_remote_tuning(10 * GIB);
        assert_eq!(t.chunk_bytes, 64 * 1024 * 1024);
        assert_eq!((t.initial_streams, t.max_streams), (16, 24));
        assert_eq!(t.tcp_buffer_size, Some(8 * 1024 * 1024));
        assert_eq!(t.prefetch_count, Some(32));
    }

    #[test]
    fn top_tier_engages_at_32_gib() {
        let below = determine_remote_tuning(32 * GIB - 1);
        assert_eq!((below.initial_streams, below.max_streams), (16, 24));
        let at = determine_remote_tuning(32 * GIB);
        assert_eq!((at.initial_streams, at.max_streams), (24, 32));
    }
}
