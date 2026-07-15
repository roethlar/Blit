# otp-12-worker-parity accepted-race-fix review — adjudication

**Slice**: `f7f12ec` — start accepted settlement before tuner release.
**Reviewer**: `gpt-5.6-sol` @ `model_reasoning_effort = xhigh`.
**Raw review**: `.review/results/otp-12-worker-parity.accepted-race-fix.codex.md`
**Review verdict**: **FAIL** — 1 LOW.
**Adjudication**: 1 finding, 1 ACCEPTED, 0 rejected, 0 deferred.

## LOW — pre-call signal leaves the schedule unproved → ACCEPTED

The settler thread sent `started` immediately before calling
`resize_settled`. Receiving that signal did not prove it had attempted the
epoch mutex. A guard-drop mutation could let the tuner reacquire and claim
before the settler reached arbitration, so the test could still pass according
to scheduler order.

Fix: the test hook now lives inside `resize_settled` and reports the result of
an actual `try_lock`; the accepted call therefore proves it observed contention.
In addition, `resize_tick`'s mutex guard owns a test-only monotonically unique
acquisition token recorded at initial eligibility and epoch claim. A temporary
drop/reacquire changes token 11 to 12 and fails deterministically even when the
settler loses the scheduling race. Restoring one continuous guard passes in
debug and optimized builds.

Full workspace fmt, strict clippy, and tests are green: 1,490 passed, 2
ignored, no failures. Documentation checks are green.

Fix commit: pending.

reviewer: gpt-5.6-sol
