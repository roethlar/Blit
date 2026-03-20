//! Error classification and retry logic for transfer operations.

use std::io;
use std::time::Duration;

/// Classification of errors for retry decisions.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ErrorClass {
    /// Transient error that may resolve on retry.
    Retryable,
    /// Permanent error that will not resolve on retry.
    Permanent,
    /// Fatal error requiring immediate abort (resource exhaustion).
    Fatal,
}

/// Configuration for retry behavior.
#[derive(Debug, Clone)]
pub struct RetryConfig {
    pub max_retries: u8,
    pub initial_delay: Duration,
    pub max_delay: Duration,
    pub backoff_factor: f64,
}

impl Default for RetryConfig {
    fn default() -> Self {
        Self {
            max_retries: 3,
            initial_delay: Duration::from_millis(500),
            max_delay: Duration::from_secs(30),
            backoff_factor: 2.0,
        }
    }
}

impl RetryConfig {
    /// Create a config from a simple retry count (uses sensible defaults).
    pub fn from_max_retries(max_retries: u8) -> Self {
        Self {
            max_retries,
            ..Default::default()
        }
    }

    /// Compute the delay for the given attempt number (0-indexed).
    pub fn delay_for_attempt(&self, attempt: u8) -> Duration {
        let delay_ms =
            self.initial_delay.as_millis() as f64 * self.backoff_factor.powi(attempt as i32);
        let capped = delay_ms.min(self.max_delay.as_millis() as f64);
        Duration::from_millis(capped as u64)
    }
}

/// Classify an IO error for retry decisions.
pub fn classify_io_error(err: &io::Error) -> ErrorClass {
    match err.kind() {
        io::ErrorKind::ConnectionReset
        | io::ErrorKind::ConnectionAborted
        | io::ErrorKind::BrokenPipe
        | io::ErrorKind::TimedOut
        | io::ErrorKind::Interrupted
        | io::ErrorKind::WouldBlock => ErrorClass::Retryable,

        io::ErrorKind::OutOfMemory => ErrorClass::Fatal,

        io::ErrorKind::NotFound
        | io::ErrorKind::PermissionDenied
        | io::ErrorKind::AlreadyExists
        | io::ErrorKind::InvalidInput
        | io::ErrorKind::InvalidData => ErrorClass::Permanent,

        _ => ErrorClass::Retryable,
    }
}

/// Classify an eyre error by inspecting the chain for IO errors and
/// known message patterns.
pub fn classify_error(err: &eyre::Report) -> ErrorClass {
    // Check for wrapped IO errors
    if let Some(io_err) = err.downcast_ref::<io::Error>() {
        return classify_io_error(io_err);
    }

    let msg = err.to_string().to_lowercase();

    // Fatal: resource exhaustion
    if msg.contains("no space left")
        || msg.contains("disk quota")
        || msg.contains("out of memory")
    {
        return ErrorClass::Fatal;
    }

    // Permanent: won't change on retry
    if msg.contains("permission denied")
        || msg.contains("no such file")
        || msg.contains("not a directory")
        || msg.contains("is a directory")
        || msg.contains("read-only file system")
    {
        return ErrorClass::Permanent;
    }

    // Retryable: transient conditions
    if msg.contains("resource temporarily unavailable")
        || msg.contains("connection reset")
        || msg.contains("broken pipe")
        || msg.contains("timed out")
        || msg.contains("interrupted")
        || msg.contains("would block")
    {
        return ErrorClass::Retryable;
    }

    // Default: retryable for unknown errors
    ErrorClass::Retryable
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn classify_io_errors() {
        assert_eq!(
            classify_io_error(&io::Error::new(io::ErrorKind::ConnectionReset, "reset")),
            ErrorClass::Retryable
        );
        assert_eq!(
            classify_io_error(&io::Error::new(io::ErrorKind::PermissionDenied, "denied")),
            ErrorClass::Permanent
        );
        assert_eq!(
            classify_io_error(&io::Error::new(io::ErrorKind::OutOfMemory, "oom")),
            ErrorClass::Fatal
        );
    }

    #[test]
    fn classify_eyre_with_io() {
        let io_err = io::Error::new(io::ErrorKind::TimedOut, "timed out");
        let err: eyre::Report = io_err.into();
        assert_eq!(classify_error(&err), ErrorClass::Retryable);
    }

    #[test]
    fn classify_eyre_by_message() {
        assert_eq!(
            classify_error(&eyre::eyre!("permission denied for /etc/shadow")),
            ErrorClass::Permanent
        );
        assert_eq!(
            classify_error(&eyre::eyre!("no space left on device")),
            ErrorClass::Fatal
        );
        assert_eq!(
            classify_error(&eyre::eyre!("connection reset by peer")),
            ErrorClass::Retryable
        );
    }

    #[test]
    fn backoff_calculation() {
        let config = RetryConfig {
            max_retries: 3,
            initial_delay: Duration::from_millis(100),
            max_delay: Duration::from_secs(5),
            backoff_factor: 2.0,
        };
        assert_eq!(config.delay_for_attempt(0), Duration::from_millis(100));
        assert_eq!(config.delay_for_attempt(1), Duration::from_millis(200));
        assert_eq!(config.delay_for_attempt(2), Duration::from_millis(400));
    }

    #[test]
    fn backoff_caps_at_max() {
        let config = RetryConfig {
            max_retries: 10,
            initial_delay: Duration::from_secs(1),
            max_delay: Duration::from_secs(5),
            backoff_factor: 10.0,
        };
        assert_eq!(config.delay_for_attempt(2), Duration::from_secs(5));
    }

    #[test]
    fn from_max_retries() {
        let config = RetryConfig::from_max_retries(5);
        assert_eq!(config.max_retries, 5);
        assert_eq!(config.initial_delay, Duration::from_millis(500));
    }
}
