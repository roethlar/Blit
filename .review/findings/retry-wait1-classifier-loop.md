# retry-wait1-classifier-loop: retryable-error classifier + retry-loop helper

**Severity**: Feature (owner-approved follow-up)
**Status**: In progress / pending review
**Branch**: `phase5/a1`
**Commit**: `e5e59fb`
**Parent**: owner-approved robocopy-style `--retry`/`--wait` (memory
`audit-owner-decisions`). Part 1 of 2; unblocked by audit-1c (which lands
the clean, fast, retryable stall failure this loop catches).

## What

`blit_app::transfers::retry` — the self-contained logic for retry-with-
wait, with no CLI surface yet (the flags + dispatch wiring are part 2,
kept separate so we don't re-add an operator-facing flag before it's
wired — the R58-F8 "lying knob" lesson).

- `is_retryable(&eyre::Report)` — conservative: retryable iff the error
  chain contains a `std::io::Error` of a transient transport kind
  (`TimedOut` — incl. the audit-1c `StallGuard` — `ConnectionReset`/
  `Aborted`/`Refused`, `BrokenPipe`, `UnexpectedEof`, `NotConnected`).
  Fatal errors (path-safety, gate, auth, invalid-arg — plain `eyre`
  messages — or non-transient io kinds like `PermissionDenied`/
  `NotFound`) are NOT retried, so the loop never spins on a deterministic
  failure.
- `run_with_retries(retries, wait, attempt)` — re-runs `attempt` up to
  `retries` times spaced by `wait`, only while `is_retryable` holds;
  `retries == 0` is today's single-attempt behavior. Resumability means
  each retry continues rather than restarts.

## Tests (blit-app, +6)

transient kinds retryable; fatal + non-transient not retryable;
fail-twice-then-succeed drives the loop to success (3 calls); fatal →
single attempt; `retries=0` → single attempt; budget exhaustion (2
retries) → 3 calls then the last error.

## Files changed

- `crates/blit-app/src/transfers/retry.rs` (new) + `mod.rs` declaration.

## Part 2 (next)

Add `--retry=N` / `--wait=X` to `TransferArgs` (the `Reliability` heading
already reserves the spot, R58-F8) and wrap the `run_transfer` / `run_move`
invocation in `run_with_retries` so a transient transfer failure is
retried against the resumable transfer.

## Reviewer comments

(empty — pending review)
