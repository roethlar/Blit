//! audit-1b: a tiny deadline helper for the daemon's outbound network
//! operations.
//!
//! Per the memory `feedback-server-await-timeouts`, every `.await` on a
//! socket read / DNS / RPC connect in a long-running handler needs a
//! bounded deadline so a slow or black-holed peer can't pin a handler
//! (and its `ActiveJobs` row + resources) indefinitely. [`within`] wraps
//! such a future in a wall-clock timeout; it is deliberately generic and
//! error-type-free so each call site maps an elapsed deadline to its own
//! domain error (an `io::Error` for DNS, a `DelegatedPullProgress` error
//! frame for connect). Being a thin wrapper over `tokio::time::timeout`,
//! it's unit-testable with `std::future::pending()`.

use std::future::Future;
use std::time::Duration;

/// Run `fut` to completion, but no longer than `deadline`. Returns
/// `Some(output)` if it finished in time, or `None` if the deadline
/// elapsed first. The caller decides what a `None` means.
pub(crate) async fn within<F: Future>(deadline: Duration, fut: F) -> Option<F::Output> {
    tokio::time::timeout(deadline, fut).await.ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn within_returns_none_when_the_deadline_elapses() {
        // A never-resolving future can only ever hit the timeout, so a
        // short real deadline is deterministic.
        let out = within(Duration::from_millis(10), std::future::pending::<u8>()).await;
        assert!(out.is_none());
    }

    #[tokio::test]
    async fn within_passes_through_a_prompt_value() {
        let out = within(Duration::from_secs(30), async { 42u8 }).await;
        assert_eq!(out, Some(42));
    }
}
