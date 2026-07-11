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
///
/// The unified session reports its failures as `SessionFault` values
/// that REPLACE the original chain (the fault is what crosses the wire
/// and the driver boundary), so the fault carries the underlying
/// `io::ErrorKind` in `SessionFault::io_kind` and classifies here by
/// the same kind set (codex otp-10a F5) — a mid-transfer socket reset
/// stays retryable under `--retry` on the session paths.
pub fn is_retryable(err: &eyre::Report) -> bool {
    err.chain().any(|cause| {
        cause
            .downcast_ref::<io::Error>()
            .is_some_and(|io_err| is_retryable_io_kind(io_err.kind()))
            || cause
                .downcast_ref::<crate::transfer_session::SessionFault>()
                .is_some_and(|fault| fault.io_kind.is_some_and(is_retryable_io_kind))
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

    /// codex otp-10a F5: a `SessionFault` replaces the original error
    /// chain at the session-driver boundary, so its captured `io_kind`
    /// must classify exactly as the raw io::Error would — retryable
    /// kinds retry, fatal kinds and kind-less faults do not.
    #[test]
    fn session_fault_io_kind_classifies_like_the_raw_error() {
        use crate::generated::session_error::Code;
        use crate::transfer_session::SessionFault;

        let fault = |io_kind: Option<io::ErrorKind>| {
            eyre::Report::new(SessionFault {
                code: Code::DataPlaneFailed,
                message: "dialing session data plane: reset".into(),
                local_build_id: String::new(),
                peer_build_id: String::new(),
                peer_notified: false,
                relative_path: None,
                io_kind,
            })
            .wrap_err("pushing to host:/mod/")
        };

        assert!(is_retryable(&fault(Some(io::ErrorKind::ConnectionReset))));
        assert!(is_retryable(&fault(Some(io::ErrorKind::TimedOut))));
        assert!(!is_retryable(&fault(Some(io::ErrorKind::PermissionDenied))));
        assert!(!is_retryable(&fault(None)), "kind-less faults stay fatal");
    }
}
