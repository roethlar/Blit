//! Retry-with-wait for transfers (owner-approved robocopy-style
//! `--retry`/`--wait`). Part 1: the retryable-error classifier and the
//! generic retry loop. Part 2 wires the CLI flags and the transfer
//! dispatch through `run_with_retries`.
//!
//! This is viable because blit transfers are **resumable** — a retry
//! re-runs the same transfer, the manifest compare only re-sends
//! missing/changed files, so a retry continues rather than restarts. The
//! audit-1c stall-timeout is what turns an infinite stall into the clean,
//! fast, retryable failure this loop catches.

use std::future::Future;
use std::io;
use std::time::Duration;

use eyre::Result;

/// Decide whether a failed transfer is worth retrying. Conservative: only
/// transient transport-level failures are retryable. A fatal error
/// (path-safety rejection, gate denial, auth, invalid argument — all
/// surfaced as plain `eyre` messages with no transient I/O source) is
/// NOT retried, so we never loop forever on a deterministic failure.
///
/// Retryable = the error chain contains a `std::io::Error` whose kind is
/// a transient transport condition, which is exactly what a mid-transfer
/// network drop or the audit-1c `StallGuard` timeout surfaces.
pub fn is_retryable(err: &eyre::Report) -> bool {
    err.chain().any(|cause| {
        cause
            .downcast_ref::<io::Error>()
            .is_some_and(|io_err| is_retryable_io_kind(io_err.kind()))
    })
}

fn is_retryable_io_kind(kind: io::ErrorKind) -> bool {
    matches!(
        kind,
        io::ErrorKind::TimedOut
            | io::ErrorKind::ConnectionReset
            | io::ErrorKind::ConnectionAborted
            | io::ErrorKind::ConnectionRefused
            | io::ErrorKind::BrokenPipe
            | io::ErrorKind::UnexpectedEof
            | io::ErrorKind::NotConnected
    )
}

/// Run `attempt` (a fresh transfer attempt; `attempt_no` is 0 on the
/// first try) with up to `retries` retries spaced by `wait`. Retries fire
/// only when [`is_retryable`] accepts the error; a fatal error returns
/// immediately. `retries == 0` reproduces the no-retry default.
///
/// The transfer's resumability means each retry continues the prior
/// attempt rather than restarting from scratch.
pub async fn run_with_retries<F, Fut>(retries: u32, wait: Duration, mut attempt: F) -> Result<()>
where
    F: FnMut(u32) -> Fut,
    Fut: Future<Output = Result<()>>,
{
    let mut attempt_no = 0u32;
    loop {
        match attempt(attempt_no).await {
            Ok(()) => return Ok(()),
            Err(err) => {
                if attempt_no >= retries || !is_retryable(&err) {
                    return Err(err);
                }
                attempt_no += 1;
                eprintln!(
                    "blit: transfer failed, retrying ({attempt_no}/{retries}) in {}s: {err:#}",
                    wait.as_secs()
                );
                tokio::time::sleep(wait).await;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::Cell;

    fn io_err(kind: io::ErrorKind) -> eyre::Report {
        // Wrap in a context layer so the io::Error is a *source* in the
        // chain, mirroring how the transfer code reports it.
        eyre::Report::new(io::Error::new(kind, "boom")).wrap_err("receiving data")
    }

    #[test]
    fn classifies_transient_io_as_retryable() {
        assert!(is_retryable(&io_err(io::ErrorKind::TimedOut)));
        assert!(is_retryable(&io_err(io::ErrorKind::ConnectionReset)));
        assert!(is_retryable(&io_err(io::ErrorKind::BrokenPipe)));
        assert!(is_retryable(&io_err(io::ErrorKind::UnexpectedEof)));
    }

    #[test]
    fn classifies_fatal_errors_as_not_retryable() {
        // A plain eyre message (path-safety / gate / invalid-arg shape).
        assert!(!is_retryable(&eyre::eyre!("path escapes module root")));
        // An io error of a non-transient kind.
        assert!(!is_retryable(&io_err(io::ErrorKind::PermissionDenied)));
        assert!(!is_retryable(&io_err(io::ErrorKind::NotFound)));
    }

    #[tokio::test]
    async fn retries_a_retryable_failure_then_succeeds() {
        let calls = Cell::new(0u32);
        let result = run_with_retries(3, Duration::from_millis(0), |_n| {
            calls.set(calls.get() + 1);
            let this_call = calls.get();
            async move {
                if this_call < 3 {
                    Err(io_err(io::ErrorKind::TimedOut)) // transient: retried
                } else {
                    Ok(())
                }
            }
        })
        .await;
        assert!(result.is_ok(), "should succeed on the 3rd attempt");
        assert_eq!(calls.get(), 3, "two failures + one success");
    }

    #[tokio::test]
    async fn does_not_retry_a_fatal_failure() {
        let calls = Cell::new(0u32);
        let result = run_with_retries(5, Duration::from_millis(0), |_n| {
            calls.set(calls.get() + 1);
            async { Err::<(), _>(eyre::eyre!("invalid argument")) }
        })
        .await;
        assert!(result.is_err());
        assert_eq!(calls.get(), 1, "a fatal error must not be retried");
    }

    #[tokio::test]
    async fn retries_zero_means_a_single_attempt() {
        let calls = Cell::new(0u32);
        let result = run_with_retries(0, Duration::from_millis(0), |_n| {
            calls.set(calls.get() + 1);
            async { Err::<(), _>(io_err(io::ErrorKind::TimedOut)) }
        })
        .await;
        assert!(result.is_err());
        assert_eq!(calls.get(), 1, "retries=0 ⇒ no retries even if retryable");
    }

    #[tokio::test]
    async fn exhausts_retry_budget_then_returns_last_error() {
        let calls = Cell::new(0u32);
        let result = run_with_retries(2, Duration::from_millis(0), |_n| {
            calls.set(calls.get() + 1);
            async { Err::<(), _>(io_err(io::ErrorKind::ConnectionReset)) }
        })
        .await;
        assert!(result.is_err());
        assert_eq!(calls.get(), 3, "1 initial + 2 retries, all failing");
    }
}
