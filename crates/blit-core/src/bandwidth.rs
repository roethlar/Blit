//! Token-bucket bandwidth limiter for throttling transfer throughput.

use std::sync::Mutex;
use std::time::{Duration, Instant};

/// Token-bucket bandwidth limiter.
///
/// Limits throughput to approximately `bytes_per_sec`. Tokens accumulate
/// up to one second's worth of burst capacity.
pub struct BandwidthLimiter {
    bytes_per_sec: u64,
    state: Mutex<LimiterState>,
}

struct LimiterState {
    tokens: f64,
    last_refill: Instant,
}

impl BandwidthLimiter {
    /// Create a limiter that allows `bytes_per_sec` throughput.
    pub fn new(bytes_per_sec: u64) -> Self {
        Self {
            bytes_per_sec,
            state: Mutex::new(LimiterState {
                tokens: bytes_per_sec as f64,
                last_refill: Instant::now(),
            }),
        }
    }

    /// Acquire permission to transfer `bytes` bytes asynchronously.
    pub async fn acquire(&self, bytes: usize) {
        loop {
            let sleep_duration = {
                let mut state = self.state.lock().unwrap();
                self.refill(&mut state);

                if state.tokens >= bytes as f64 {
                    state.tokens -= bytes as f64;
                    return;
                }

                let deficit = bytes as f64 - state.tokens;
                Duration::from_secs_f64(deficit / self.bytes_per_sec as f64)
            };

            tokio::time::sleep(sleep_duration).await;
        }
    }

    /// Blocking version for synchronous code paths.
    pub fn acquire_blocking(&self, bytes: usize) {
        loop {
            let sleep_duration = {
                let mut state = self.state.lock().unwrap();
                self.refill(&mut state);

                if state.tokens >= bytes as f64 {
                    state.tokens -= bytes as f64;
                    return;
                }

                let deficit = bytes as f64 - state.tokens;
                Duration::from_secs_f64(deficit / self.bytes_per_sec as f64)
            };

            std::thread::sleep(sleep_duration);
        }
    }

    fn refill(&self, state: &mut LimiterState) {
        let now = Instant::now();
        let elapsed = now.duration_since(state.last_refill).as_secs_f64();
        state.tokens = (state.tokens + elapsed * self.bytes_per_sec as f64)
            .min(self.bytes_per_sec as f64); // cap at 1 second burst
        state.last_refill = now;
    }
}

/// Parse a human-readable rate like "10M", "1G", "500K" into bytes per second.
pub fn parse_rate(s: &str) -> eyre::Result<u64> {
    // Delegate to filter::parse_size since the format is identical
    crate::filter::parse_size(s)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_rate_units() {
        assert_eq!(parse_rate("100").unwrap(), 100);
        assert_eq!(parse_rate("10K").unwrap(), 10_000);
        assert_eq!(parse_rate("10M").unwrap(), 10_000_000);
        assert_eq!(parse_rate("1G").unwrap(), 1_000_000_000);
        assert_eq!(parse_rate("1Mi").unwrap(), 1 << 20);
    }

    #[tokio::test]
    async fn limiter_allows_burst() {
        let limiter = BandwidthLimiter::new(1_000_000);
        let start = Instant::now();
        limiter.acquire(500_000).await;
        assert!(start.elapsed() < Duration::from_millis(100));
    }

    #[tokio::test]
    async fn limiter_throttles_after_burst() {
        let limiter = BandwidthLimiter::new(100_000); // 100 KB/s
        limiter.acquire(100_000).await; // exhaust burst
        let start = Instant::now();
        limiter.acquire(50_000).await;
        let elapsed = start.elapsed();
        assert!(
            elapsed >= Duration::from_millis(300),
            "elapsed: {:?}",
            elapsed
        );
        assert!(
            elapsed < Duration::from_millis(800),
            "elapsed: {:?}",
            elapsed
        );
    }

    #[test]
    fn blocking_limiter_works() {
        let limiter = BandwidthLimiter::new(1_000_000);
        let start = Instant::now();
        limiter.acquire_blocking(500_000);
        assert!(start.elapsed() < Duration::from_millis(100));
    }
}
