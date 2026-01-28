//! Error categorization for intelligent retry handling.
//!
//! Errors are categorized to enable smart retry decisions:
//! - Retryable: Transient failures that may succeed on retry (network, temp disk full)
//! - Fatal: Permanent failures that will never succeed (permissions, corrupt data)
//! - NoRetry: Not an error condition, but operation should not be retried

use std::io;

/// Category of transfer error for retry decision-making.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ErrorCategory {
    /// Transient error - retry may succeed (network timeout, temp disk full, etc.)
    Retryable,
    /// Permanent error - retry will never succeed (permission denied, corrupt file, etc.)
    Fatal,
    /// Not an error, but should not retry (e.g., file modified during transfer)
    NoRetry,
}

/// A transfer error with its category.
#[derive(Debug)]
pub struct TransferError {
    /// The underlying error message.
    pub message: String,
    /// The file path that failed (if applicable).
    pub path: Option<String>,
    /// The error category for retry decisions.
    pub category: ErrorCategory,
    /// Number of retry attempts made.
    pub attempts: u8,
}

impl TransferError {
    /// Create a new retryable transfer error.
    pub fn retryable(message: impl Into<String>, path: Option<String>) -> Self {
        Self {
            message: message.into(),
            path,
            category: ErrorCategory::Retryable,
            attempts: 0,
        }
    }

    /// Create a new fatal transfer error.
    pub fn fatal(message: impl Into<String>, path: Option<String>) -> Self {
        Self {
            message: message.into(),
            path,
            category: ErrorCategory::Fatal,
            attempts: 0,
        }
    }

    /// Create a new no-retry transfer error.
    pub fn no_retry(message: impl Into<String>, path: Option<String>) -> Self {
        Self {
            message: message.into(),
            path,
            category: ErrorCategory::NoRetry,
            attempts: 0,
        }
    }

    /// Check if this error should be retried.
    pub fn should_retry(&self, max_retries: u8) -> bool {
        self.category == ErrorCategory::Retryable && self.attempts < max_retries
    }

    /// Increment the attempt counter and return self for chaining.
    pub fn with_attempt(mut self) -> Self {
        self.attempts = self.attempts.saturating_add(1);
        self
    }
}

impl std::fmt::Display for TransferError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if let Some(ref path) = self.path {
            write!(f, "{}: {}", path, self.message)
        } else {
            write!(f, "{}", self.message)
        }
    }
}

impl std::error::Error for TransferError {}

/// Categorize an IO error for retry decisions.
pub fn categorize_io_error(err: &io::Error) -> ErrorCategory {
    match err.kind() {
        // Retryable: transient conditions
        io::ErrorKind::TimedOut
        | io::ErrorKind::Interrupted
        | io::ErrorKind::ConnectionReset
        | io::ErrorKind::ConnectionAborted
        | io::ErrorKind::BrokenPipe
        | io::ErrorKind::WouldBlock => ErrorCategory::Retryable,

        // Fatal: permanent conditions
        io::ErrorKind::PermissionDenied
        | io::ErrorKind::NotFound
        | io::ErrorKind::InvalidData
        | io::ErrorKind::InvalidInput
        | io::ErrorKind::AlreadyExists => ErrorCategory::Fatal,

        // These could go either way - default to fatal to avoid infinite loops
        io::ErrorKind::WriteZero
        | io::ErrorKind::UnexpectedEof
        | io::ErrorKind::AddrInUse
        | io::ErrorKind::AddrNotAvailable
        | io::ErrorKind::NotConnected
        | io::ErrorKind::ConnectionRefused => ErrorCategory::Fatal,

        // Unknown errors - default to fatal to be safe
        _ => ErrorCategory::Fatal,
    }
}

/// Result type for transfer operations.
pub type TransferResult<T> = std::result::Result<T, TransferError>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_retryable_error_should_retry() {
        let err = TransferError::retryable("network timeout", Some("file.txt".to_string()));
        assert!(err.should_retry(3));
    }

    #[test]
    fn test_fatal_error_should_not_retry() {
        let err = TransferError::fatal("permission denied", Some("file.txt".to_string()));
        assert!(!err.should_retry(3));
    }

    #[test]
    fn test_retry_limit_exhausted() {
        let mut err = TransferError::retryable("network timeout", Some("file.txt".to_string()));
        err.attempts = 3;
        assert!(!err.should_retry(3));
    }

    #[test]
    fn test_io_error_categorization() {
        let timeout = io::Error::new(io::ErrorKind::TimedOut, "timeout");
        assert_eq!(categorize_io_error(&timeout), ErrorCategory::Retryable);

        let perm = io::Error::new(io::ErrorKind::PermissionDenied, "denied");
        assert_eq!(categorize_io_error(&perm), ErrorCategory::Fatal);
    }
}
