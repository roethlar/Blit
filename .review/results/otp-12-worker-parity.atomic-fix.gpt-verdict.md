# otp-12-worker-parity atomic-fix review — adjudication

**Slice**: `8e993aa` — atomic resize epoch/refusal arbitration.
**Reviewer**: `gpt-5.6-sol` @ `model_reasoning_effort = xhigh`.
**Raw review**: `.review/results/otp-12-worker-parity.atomic-fix.codex.md`
**Review verdict**: **FAIL** — 1 MEDIUM, 1 LOW.
**Adjudication**: 2 findings, 2 ACCEPTED, 0 rejected, 0 deferred.

## MEDIUM — tuner decision survives accepted settlement → ACCEPTED

`resize_tick` checked terminal/pending state, then computed cooldown, sustain,
direction, and target outside the epoch lock before reacquiring it to claim a
proposal. A shape proposal could settle successfully in that window. The
stale tuner decision could then claim the following epoch immediately,
bypassing the settlement's cooldown reset and overshooting convergence.

Fix: keep tuner eligibility, signal calculation, live/target derivation, and
epoch claim in the same short arbitration critical section as settlement. The
lock spans no await, socket operation, or callback.

## LOW — concurrency guard relies on scheduler luck → ACCEPTED

The stress guard announced producer readiness before any producer crossed the
split observation/claim window. Buggy code could pass unless the scheduler
happened to pause a producer at the vulnerable instruction, and accepted
settlement was never exercised.

Fix: replace it with a deterministic test hook that pauses a tuner after it
owns arbitration and proves the mutex remains held through its claim. The same
test holds arbitration while a shape waiter starts, then applies the production
settlement helper before releasing it. Releasing the tuner lock early fails at
the lock assertion; omitting the refusal record deterministically permits an
epoch-2 proposal and fails at the terminal-refusal assertion.

During implementation, an optimized-build mutation also proved that settlement
must execute outside `debug_assert!`: placing the call inside the assertion
fails the exact test under `cargo test --release`; the unconditional call passes.

Full workspace fmt, strict clippy, and tests are green: 1,490 passed, 2
ignored, no failures. Documentation checks are green.

Fix commit: `641916e`.

reviewer: gpt-5.6-sol
