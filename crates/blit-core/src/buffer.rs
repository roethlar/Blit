//! Smart buffer sizing and pooling for 25GbE saturation.
//!
//! This module provides:
//! - `BufferSizer`: Calculates optimal buffer sizes based on file size and available memory
//! - `BufferPool`: Reusable buffer pool with semaphore-based memory control (rclone-inspired)
//! - `PoolBuffer`: RAII guard that returns buffers to pool on drop

use once_cell::sync::OnceCell;
use parking_lot::Mutex;
use std::ops::{Deref, DerefMut};
use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
use std::sync::Arc;
use tokio::sync::Semaphore;

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

// ============================================================================
// Buffer Pool Implementation (rclone-inspired)
// ============================================================================

/// A reusable buffer pool that reduces allocation overhead for high-throughput transfers.
///
/// Based on rclone's `lib/pool/pool.go` design:
/// - Maintains a cache of reusable buffers
/// - Uses semaphore-based memory budget control
/// - Tracks statistics for monitoring
///
/// **Integration with Orchestrator**: This pool does NOT define default sizes.
/// The orchestrator/auto_tune system determines optimal `buffer_size` and `pool_size`
/// based on runtime conditions (available memory, network bandwidth, file sizes).
/// Create the pool with those tuned parameters.
///
/// # Example
/// ```ignore
/// // Orchestrator determines these values via auto_tune
/// let tuned_buffer_size = tuning_params.chunk_bytes;
/// let tuned_pool_size = tuning_params.stream_count * 2;
/// let memory_budget = available_memory / 4;
///
/// let pool = BufferPool::new(tuned_buffer_size, tuned_pool_size, Some(memory_budget));
/// let buffer = pool.acquire().await;
/// // Use buffer...
/// // Buffer automatically returned to pool on drop
/// ```
pub struct BufferPool {
    /// Cached buffers available for reuse
    cache: Mutex<Vec<Vec<u8>>>,
    /// Size of each buffer in the pool
    buffer_size: usize,
    /// Maximum number of buffers to keep in cache
    pool_size: usize,
    /// Semaphore controlling total memory usage across all active buffers
    memory_semaphore: Option<Arc<Semaphore>>,
    /// Statistics: total buffers ever allocated
    total_allocated: AtomicUsize,
    /// Statistics: buffers currently in use (not in cache)
    in_use: AtomicUsize,
    /// Statistics: total bytes transferred through pooled buffers
    bytes_through: AtomicU64,
}

impl BufferPool {
    /// Create a new buffer pool.
    ///
    /// # Arguments
    /// * `buffer_size` - Size of each buffer in bytes (default: 1MB)
    /// * `pool_size` - Maximum buffers to cache (default: 64)
    /// * `memory_budget` - Optional total memory budget in bytes. If set, acquire()
    ///   will wait when the budget is exceeded.
    pub fn new(buffer_size: usize, pool_size: usize, memory_budget: Option<usize>) -> Self {
        let memory_semaphore = memory_budget.map(|budget| {
            // Number of permits = budget / buffer_size
            let permits = budget / buffer_size.max(1);
            Arc::new(Semaphore::new(permits.max(1)))
        });

        Self {
            cache: Mutex::new(Vec::with_capacity(pool_size)),
            buffer_size,
            pool_size,
            memory_semaphore,
            total_allocated: AtomicUsize::new(0),
            in_use: AtomicUsize::new(0),
            bytes_through: AtomicU64::new(0),
        }
    }

    /// Acquire a buffer from the pool.
    ///
    /// Returns a buffer from the cache if available, otherwise allocates a new one.
    /// If a memory budget is set and exceeded, this will wait until memory is released.
    pub async fn acquire(self: &Arc<Self>) -> PoolBuffer {
        // If we have a memory budget, acquire a permit first
        if let Some(ref sem) = self.memory_semaphore {
            // This will wait if we've exceeded the memory budget
            let permit = sem
                .clone()
                .acquire_owned()
                .await
                .expect("semaphore closed unexpectedly");
            // We need to forget the permit since we'll release it manually in PoolBuffer::drop
            std::mem::forget(permit);
        }

        // Try to get a buffer from the cache
        let buffer = {
            let mut cache = self.cache.lock();
            cache.pop()
        };

        let data = match buffer {
            Some(buf) => buf,
            None => {
                // Allocate a new buffer
                self.total_allocated.fetch_add(1, Ordering::Relaxed);
                vec![0u8; self.buffer_size]
            }
        };

        self.in_use.fetch_add(1, Ordering::Relaxed);

        PoolBuffer {
            data: Some(data),
            pool: Arc::clone(self),
        }
    }

    /// Try to acquire a buffer without waiting.
    /// Returns None if memory budget is exceeded or no buffers available.
    pub fn try_acquire(self: &Arc<Self>) -> Option<PoolBuffer> {
        // If we have a memory budget, try to acquire a permit
        if let Some(ref sem) = self.memory_semaphore {
            match sem.clone().try_acquire_owned() {
                Ok(permit) => std::mem::forget(permit),
                Err(_) => return None,
            }
        }

        let buffer = {
            let mut cache = self.cache.lock();
            cache.pop()
        };

        let data = match buffer {
            Some(buf) => buf,
            None => {
                self.total_allocated.fetch_add(1, Ordering::Relaxed);
                vec![0u8; self.buffer_size]
            }
        };

        self.in_use.fetch_add(1, Ordering::Relaxed);

        Some(PoolBuffer {
            data: Some(data),
            pool: Arc::clone(self),
        })
    }

    /// Return a buffer to the pool for reuse.
    fn release(&self, mut buffer: Vec<u8>) {
        self.in_use.fetch_sub(1, Ordering::Relaxed);

        // Only cache if we haven't exceeded pool size
        let should_cache = {
            let cache = self.cache.lock();
            cache.len() < self.pool_size
        };

        if should_cache && buffer.capacity() >= self.buffer_size {
            // Reset length but keep capacity
            buffer.clear();
            buffer.resize(self.buffer_size, 0);

            let mut cache = self.cache.lock();
            if cache.len() < self.pool_size {
                cache.push(buffer);
            }
        }
        // Otherwise buffer is dropped and memory freed

        // Release semaphore permit if we have a memory budget
        if let Some(ref sem) = self.memory_semaphore {
            sem.add_permits(1);
        }
    }

    /// Record bytes transferred through a pooled buffer (for statistics)
    pub fn record_bytes(&self, bytes: u64) {
        self.bytes_through.fetch_add(bytes, Ordering::Relaxed);
    }

    /// Get the buffer size for this pool
    pub fn buffer_size(&self) -> usize {
        self.buffer_size
    }

    /// Get current statistics
    pub fn stats(&self) -> BufferPoolStats {
        let cache_len = self.cache.lock().len();
        BufferPoolStats {
            buffer_size: self.buffer_size,
            pool_size: self.pool_size,
            cached: cache_len,
            in_use: self.in_use.load(Ordering::Relaxed),
            total_allocated: self.total_allocated.load(Ordering::Relaxed),
            bytes_through: self.bytes_through.load(Ordering::Relaxed),
        }
    }

    /// Flush all cached buffers to free memory
    pub fn flush(&self) {
        let mut cache = self.cache.lock();
        cache.clear();
    }
}

/// Statistics about buffer pool usage
#[derive(Debug, Clone)]
pub struct BufferPoolStats {
    pub buffer_size: usize,
    pub pool_size: usize,
    pub cached: usize,
    pub in_use: usize,
    pub total_allocated: usize,
    pub bytes_through: u64,
}

/// RAII guard that holds a buffer and returns it to the pool on drop.
pub struct PoolBuffer {
    data: Option<Vec<u8>>,
    pool: Arc<BufferPool>,
}

impl PoolBuffer {
    /// Get the underlying buffer as a mutable slice
    pub fn as_mut_slice(&mut self) -> &mut [u8] {
        self.data.as_mut().expect("buffer already taken").as_mut_slice()
    }

    /// Get the underlying buffer as a slice
    pub fn as_slice(&self) -> &[u8] {
        self.data.as_ref().expect("buffer already taken").as_slice()
    }

    /// Get the buffer length
    pub fn len(&self) -> usize {
        self.data.as_ref().map(|d| d.len()).unwrap_or(0)
    }

    /// Check if buffer is empty
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Take ownership of the underlying buffer, removing it from pool management.
    /// The buffer will NOT be returned to the pool.
    pub fn take(mut self) -> Vec<u8> {
        // Release semaphore permit since we're taking the buffer out of pool management
        if let Some(ref sem) = self.pool.memory_semaphore {
            sem.add_permits(1);
        }
        self.pool.in_use.fetch_sub(1, Ordering::Relaxed);
        self.data.take().expect("buffer already taken")
    }
}

impl Deref for PoolBuffer {
    type Target = [u8];

    fn deref(&self) -> &Self::Target {
        self.as_slice()
    }
}

impl DerefMut for PoolBuffer {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.as_mut_slice()
    }
}

impl Drop for PoolBuffer {
    fn drop(&mut self) {
        if let Some(data) = self.data.take() {
            self.pool.release(data);
        }
    }
}

#[cfg(test)]
mod pool_tests {
    use super::*;

    #[tokio::test]
    async fn test_buffer_pool_acquire_release() {
        let pool = Arc::new(BufferPool::new(1024, 4, None));

        // Acquire a buffer
        let buf = pool.acquire().await;
        assert_eq!(buf.len(), 1024);
        assert_eq!(pool.stats().in_use, 1);
        assert_eq!(pool.stats().cached, 0);

        // Drop returns buffer to pool
        drop(buf);
        assert_eq!(pool.stats().in_use, 0);
        assert_eq!(pool.stats().cached, 1);

        // Acquire again reuses cached buffer
        let buf2 = pool.acquire().await;
        assert_eq!(pool.stats().cached, 0);
        assert_eq!(pool.stats().total_allocated, 1); // Same buffer reused
        drop(buf2);
    }

    #[tokio::test]
    async fn test_buffer_pool_memory_budget() {
        // Pool with budget for only 2 buffers
        let pool = Arc::new(BufferPool::new(1024, 4, Some(2048)));

        let buf1 = pool.acquire().await;
        let buf2 = pool.acquire().await;

        // Third acquire would block, so use try_acquire
        let buf3 = pool.try_acquire();
        assert!(buf3.is_none());

        // Release one buffer
        drop(buf1);

        // Now we can acquire
        let buf3 = pool.try_acquire();
        assert!(buf3.is_some());

        drop(buf2);
        drop(buf3);
    }

    #[tokio::test]
    async fn test_buffer_pool_flush() {
        let pool = Arc::new(BufferPool::new(1024, 4, None));

        // Acquire multiple buffers at once, then release them to fill cache
        let buf1 = pool.acquire().await;
        let buf2 = pool.acquire().await;
        let buf3 = pool.acquire().await;
        let buf4 = pool.acquire().await;

        assert_eq!(pool.stats().in_use, 4);
        assert_eq!(pool.stats().cached, 0);

        // Release all - they should go into cache
        drop(buf1);
        drop(buf2);
        drop(buf3);
        drop(buf4);
        assert_eq!(pool.stats().cached, 4);

        // Flush clears the cache
        pool.flush();
        assert_eq!(pool.stats().cached, 0);
    }

    #[tokio::test]
    async fn test_buffer_pool_stats() {
        let pool = Arc::new(BufferPool::new(1024, 4, None));

        let buf = pool.acquire().await;
        pool.record_bytes(500);
        pool.record_bytes(500);

        let stats = pool.stats();
        assert_eq!(stats.buffer_size, 1024);
        assert_eq!(stats.pool_size, 4);
        assert_eq!(stats.in_use, 1);
        assert_eq!(stats.bytes_through, 1000);

        drop(buf);
    }
}
