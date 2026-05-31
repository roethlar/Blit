//! audit-7d6: pure sleep-budget math extracted from `main.rs`
//! (behavior-preserving — verbatim move, no logic change). The event loop
//! uses these to decide how long its optional `live_tick` future may sleep
//! so a short auto-hide deadline isn't masked by a longer tick interval.
//! Pure `Duration`/`Option` math, no AppState.

/// d-24 round 2: pick the actual sleep budget for the loop's
/// optional `live_tick` future.
///
/// - When the live tick is needed AND a cancel fragment is
///   pending, sleep the shorter of the two (cancel deadline
///   wins for short TTLs).
/// - When only the live tick is needed, use its interval.
/// - When only a cancel fragment is pending (no other
///   freshness-driven ticks), wake just for the deadline.
/// - When neither applies, return `None` — the loop sleeps
///   indefinitely waiting on real events.
pub(crate) fn compute_tick_budget(
    needs_live_tick: bool,
    live_tick_interval: std::time::Duration,
    cancel_remaining: Option<std::time::Duration>,
) -> Option<std::time::Duration> {
    match (needs_live_tick, cancel_remaining) {
        (true, Some(rem)) => Some(live_tick_interval.min(rem)),
        (true, None) => Some(live_tick_interval),
        (false, Some(rem)) => Some(rem),
        (false, None) => None,
    }
}

/// d-40 round 2: the shorter of two optional deadlines.
///
/// The loop has more than one auto-hide fragment that can
/// pull the sleep budget below `live_tick.interval_ms`: the
/// F2 cancel status (d-24) and the F3 pull outcome (d-40).
/// They live on different screens so at most one is `Some`
/// in practice, but merging generically keeps the budget
/// correct if that ever changes — the loop must wake for
/// whichever deadline is nearer.
pub(crate) fn min_opt(
    a: Option<std::time::Duration>,
    b: Option<std::time::Duration>,
) -> Option<std::time::Duration> {
    match (a, b) {
        (Some(x), Some(y)) => Some(x.min(y)),
        (Some(x), None) => Some(x),
        (None, b) => b,
    }
}
