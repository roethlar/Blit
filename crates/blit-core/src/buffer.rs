//! Smart buffer sizing for 10GbE saturation (minimal, used APIs only)

use once_cell::sync::OnceCell;

const KB: usize = 1024;
const MB: usize = 1024 * KB;

pub struct BufferSizer {
    max_buffer_size: usize,
    min_buffer_size: usize,
    cached_available_memory: OnceCell<u64>,
}

impl BufferSizer {
    pub fn new() -> Self {
        BufferSizer {
            max_buffer_size: 16 * MB, // 16MB max
            min_buffer_size: MB,      // 1MB min
            cached_available_memory: OnceCell::new(),
        }
    }

    /// Get available memory using sysinfo
    fn get_available_memory() -> u64 {
        use sysinfo::System;
        let mut sys = System::new_all();
        sys.refresh_memory();
        // sysinfo reports memory in kilobytes. Convert to bytes.
        let avail_kib = sys.available_memory();
        let avail_bytes = avail_kib.saturating_mul(1024);
        // Apply a conservative fallback only if the reported value is zero.
        if avail_bytes == 0 {
            // Safer fallback than 4 GiB on low-memory systems.
            512_u64 * 1024 * 1024
        } else {
            avail_bytes
        }
    }

    /// Calculate optimal buffer size based on file size and available memory
    pub fn calculate_buffer_size(&self, file_size: u64, is_network: bool) -> usize {
        // Get or cache available memory
        let available_memory = *self
            .cached_available_memory
            .get_or_init(Self::get_available_memory);

        // Base size: bigger for network
        let base_size = if is_network { 8 * MB } else { 4 * MB };

        // Scale based on file size
        let small_limit = 10 * MB as u64;
        let medium_limit = 100 * MB as u64;
        let optimal_size = if file_size < small_limit {
            self.min_buffer_size
        } else if file_size <= medium_limit {
            base_size
        } else {
            // For files larger than medium_limit, scale linearly from base_size up to max_buffer_size
            // over a span of 900 MiB beyond the medium limit.
            let max_size = self.max_buffer_size;
            let range = max_size.saturating_sub(base_size);
            let span: u64 = 900 * MB as u64;
            let over = file_size.saturating_sub(medium_limit).min(span);
            let incr = if span == 0 {
                0
            } else {
                // Avoid floating point; compute proportion in integer arithmetic.
                (range as u64).saturating_mul(over).saturating_div(span) as usize
            };
            base_size.saturating_add(incr)
        };

        // Cap to 10% of available memory, enforce minimum
        let memory_limit = (available_memory / 10) as usize;
        // Do not enforce a high minimum that could over-allocate on constrained systems.
        // Ensure a tiny lower bound to avoid zero-sized buffers.
        optimal_size.min(memory_limit).max(8 * KB)
    }
}

impl Default for BufferSizer {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    impl BufferSizer {
        #[cfg(test)]
        fn new_with_memory(mock_bytes: u64) -> Self {
            let s = BufferSizer::new();
            let _ = s.cached_available_memory.set(mock_bytes);
            s
        }
    }

    #[test]
    fn test_buffer_sizing_small_file() {
        let sizer = BufferSizer::new_with_memory(8_u64 * 1024 * 1024 * 1024); // 8 GiB
        let size = sizer.calculate_buffer_size(1 * MB as u64, true);
        assert!(size >= 8 * KB);
        assert!(size <= sizer.max_buffer_size);
    }

    #[test]
    fn test_buffer_sizing_medium_and_large() {
        let sizer = BufferSizer::new_with_memory(16_u64 * 1024 * 1024 * 1024); // 16 GiB
                                                                               // Medium file ~100MB should be base size for local
        let local_buf = sizer.calculate_buffer_size(100 * MB as u64, false);
        assert_eq!(local_buf, 4 * MB);

        // Large file should scale up but not exceed max or memory limit
        let large_buf = sizer.calculate_buffer_size(2 * 1024 * MB as u64, true);
        assert!(large_buf >= 8 * MB);
        assert!(large_buf <= sizer.max_buffer_size);
    }
}
