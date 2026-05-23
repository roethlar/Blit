# audit-3-panic-resilience: Panic sites that should return errors instead

**Severity**: Robustness
**Status**: Open
**Branch**: (none yet)

## What

Ground-up audit identified panic sites in production code paths where a
fallible operation uses `expect()` or panics, silently killing tasks:

1. **`crates/blit-daemon/src/service/push/data_plane.rs:44-48`** —
   `generate_token()` calls `SysRng.try_fill_bytes(&mut buf).expect("system RNG failed")`.
   If the OS cryptographic RNG is unavailable (restricted container, exhausted
   file descriptors), the spawned data-plane task panics. The `JoinHandle` is
   fire-and-forget (spawned at `control.rs:218` so the control-plane stream
   hangs waiting for a data-plane handshake that will never arrive. Should
   return `Result` or `Status::Internal` instead of panicking.

2. **`crates/blit-daemon/src/active_jobs.rs:427,457,482,523,556,605,649,701`** —
   Seven sites use `expect("active_jobs table poisoned")` or
   `expect("active_jobs recent poisoned")`. If any task panics while holding
   the ActiveJobs mutex, ALL subsequent operations (new registrations, progress
   ticking, event emission, GetState queries) crash their tasks. The
   `ActiveJobGuard::Drop` path (line 996) already uses
   `unwrap_or_else(|e| e.into_inner())` — the non-Drop paths should use the
   same pattern instead of panicking.

## Approach

1. Change `generate_token()` to return `Result<Vec<u8>, std::io::Error>` and
   propagate the error to the caller, which can return a gRPC Status::Internal.

2. Replace all seven `expect` calls with
   `unwrap_or_else(|e| e.into_inner())` matching the existing Drop-path
   pattern. The risk of operating on potentially inconsistent state is
   acceptable trade-off (mutex poisoning means a prior panic already happened
   — better to serve degraded than not at all).

## Files changed

TBD by coder. Primarily `active_jobs.rs` and `data_plane.rs`.

## Tests

- Unit test: inject RNG failure → verify error propagated (not panic)
- Unit test: poisoned mutex → operation completes (not panic)
- Existing daemon tests must still pass
