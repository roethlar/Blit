//! The retryable-error classifier — single owner of retry policy.
//!
//! w5-2: moved here from `blit_app::transfers::retry` (which still
//! re-exports it) so the policy lives next to the transfer code that
//! produces the errors it classifies, and so in-crate tests can pin
//! chain-preservation behavior (the queued W1.1 work). The dead,
//! contradictory `blit_core::errors` module this replaces classified
//! ConnectionRefused/UnexpectedEof/NotConnected as fatal and
//! Interrupted/WouldBlock as retryable — the exact inversions the
//! contract test below pins against.

use std::io;

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

/// The transient-transport kind set. Public so callers (and the W1.1
/// chain-preservation tests) can classify a bare kind without
/// constructing a Report.
pub fn is_retryable_io_kind(kind: io::ErrorKind) -> bool {
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

#[cfg(test)]
mod tests {
    use super::*;

    fn io_err(kind: io::ErrorKind) -> eyre::Report {
        // Wrap in a context layer so the io::Error is a *source* in the
        // chain, mirroring how the transfer code reports it.
        eyre::Report::new(io::Error::new(kind, "boom")).wrap_err("receiving data")
    }

    /// The full retry contract, pinned kind by kind. The deleted
    /// `blit_core::errors` module disagreed on five of these; any
    /// future edit that flips one must consciously break this test.
    #[test]
    fn retryable_kind_contract() {
        let retryable = [
            io::ErrorKind::TimedOut,
            io::ErrorKind::ConnectionReset,
            io::ErrorKind::ConnectionAborted,
            io::ErrorKind::ConnectionRefused,
            io::ErrorKind::BrokenPipe,
            io::ErrorKind::UnexpectedEof,
            io::ErrorKind::NotConnected,
        ];
        for kind in retryable {
            assert!(is_retryable_io_kind(kind), "{kind:?} must be retryable");
            assert!(is_retryable(&io_err(kind)), "{kind:?} via chain");
        }
        // The dead classifier marked Interrupted/WouldBlock retryable
        // and the three connection kinds above fatal — both wrong.
        let fatal = [
            io::ErrorKind::Interrupted,
            io::ErrorKind::WouldBlock,
            io::ErrorKind::PermissionDenied,
            io::ErrorKind::NotFound,
            io::ErrorKind::WriteZero,
            io::ErrorKind::AddrInUse,
            io::ErrorKind::InvalidData,
        ];
        for kind in fatal {
            assert!(!is_retryable_io_kind(kind), "{kind:?} must be fatal");
            assert!(!is_retryable(&io_err(kind)), "{kind:?} via chain");
        }
    }

    #[test]
    fn plain_eyre_messages_are_fatal() {
        assert!(!is_retryable(&eyre::eyre!("path escapes module root")));
    }

    /// Chain-preservation: the io::Error may sit arbitrarily deep in
    /// the context chain and must still classify (the W1.1 class —
    /// a chain-amputating wrapper would break this).
    #[test]
    fn classifies_through_deep_context_chains() {
        let deep = eyre::Report::new(io::Error::new(io::ErrorKind::TimedOut, "boom"))
            .wrap_err("layer 1")
            .wrap_err("layer 2")
            .wrap_err("layer 3");
        assert!(is_retryable(&deep));
    }
}
