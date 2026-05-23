//! audit-7d4: F2 cancel state→display helpers extracted from `main.rs`
//! (behavior-preserving — verbatim move, no logic change). Pure functions
//! bridging the F2 cancel state machine to the renderer-facing
//! `screens::f2::F2CancelDisplay` plus the auto-hide deadline the event
//! loop reads to bound its tick budget. `F2CancelStatus` itself stays in
//! `main.rs` (the event loop mutates it throughout); these mappers only
//! read it, via the crate-root path.

use crate::{screens, F2CancelStatus};
use std::time::Instant;

/// d-22: convert the internal `F2CancelStatus` to the
/// renderer-facing `F2CancelDisplay` (which lives in
/// `screens/f2.rs` to avoid the screens layer reaching
/// into main.rs's types).
pub(crate) fn cancel_status_to_display(
    status: &F2CancelStatus,
    now: Instant,
    ttl: std::time::Duration,
) -> screens::f2::F2CancelDisplay {
    use blit_app::admin::jobs::CancelJobOutcome;
    use screens::f2::F2CancelDisplay;
    match status {
        F2CancelStatus::Idle => F2CancelDisplay::Hidden,
        F2CancelStatus::Confirming { transfer_id, .. } => F2CancelDisplay::ConfirmingCancel {
            transfer_id: transfer_id.clone(),
        },
        F2CancelStatus::ConfirmingBatch { targets } => F2CancelDisplay::ConfirmingBatch {
            count: targets.len(),
        },
        F2CancelStatus::ConfirmingClearRecent => F2CancelDisplay::ConfirmingClearRecent,
        F2CancelStatus::BatchInitiated { count, finished_at } => {
            if now.saturating_duration_since(*finished_at) >= ttl {
                return F2CancelDisplay::Hidden;
            }
            F2CancelDisplay::BatchInitiated { count: *count }
        }
        F2CancelStatus::Sending { transfer_id, .. } => F2CancelDisplay::Sending {
            transfer_id: transfer_id.clone(),
        },
        F2CancelStatus::Done {
            outcome,
            finished_at,
        } => {
            // d-23: hide the terminal fragment after the
            // TTL. The state itself stays — we don't mutate
            // it from the renderer — but the operator sees
            // the footer self-clean.
            if now.saturating_duration_since(*finished_at) >= ttl {
                return F2CancelDisplay::Hidden;
            }
            match outcome {
                CancelJobOutcome::Cancelled { transfer_id: id } => F2CancelDisplay::Cancelled {
                    transfer_id: id.clone(),
                },
                CancelJobOutcome::NotFound { transfer_id: id } => F2CancelDisplay::NotFound {
                    transfer_id: id.clone(),
                },
                CancelJobOutcome::Unsupported {
                    transfer_id: id,
                    message,
                } => F2CancelDisplay::Unsupported {
                    transfer_id: id.clone(),
                    message: message.clone(),
                },
            }
        }
        F2CancelStatus::Error {
            transfer_id,
            message,
            finished_at,
        } => {
            if now.saturating_duration_since(*finished_at) >= ttl {
                return F2CancelDisplay::Hidden;
            }
            F2CancelDisplay::Failed {
                transfer_id: transfer_id.clone(),
                message: message.clone(),
            }
        }
    }
}

/// d-24 round 2: how much wall-clock time remains before the
/// d-23 auto-hide kicks in on a Done/Error cancel fragment.
///
/// Returns `Some(remaining)` only while the fragment is still
/// visible. `None` for:
/// - `Idle` / `Sending` — no deadline (Sending waits for the
///   RPC reply, not a timer).
/// - Already-expired Done/Error — the renderer already returns
///   `Hidden`, so no further wakeup is needed.
///
/// The event loop reads this to ensure a short
/// `cancel_status_ttl_ms` isn't silently bounded by a longer
/// `live_tick.interval_ms` (round-1 R2 reopen). The fix is
/// `min(live_tick_interval, remaining)` while F2 is visible.
pub(crate) fn cancel_status_remaining_ttl(
    status: &F2CancelStatus,
    now: Instant,
    ttl: std::time::Duration,
) -> Option<std::time::Duration> {
    let finished_at = match status {
        F2CancelStatus::Done { finished_at, .. } => *finished_at,
        F2CancelStatus::Error { finished_at, .. } => *finished_at,
        // d-30: BatchInitiated has a finished_at like
        // Done/Error — the loop must wake to hide it on
        // the same TTL boundary as the single-cancel
        // variants.
        F2CancelStatus::BatchInitiated { finished_at, .. } => *finished_at,
        _ => return None,
    };
    let elapsed = now.saturating_duration_since(finished_at);
    if elapsed >= ttl {
        None
    } else {
        Some(ttl - elapsed)
    }
}
