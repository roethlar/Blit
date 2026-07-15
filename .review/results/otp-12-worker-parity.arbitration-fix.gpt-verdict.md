# otp-12-worker-parity arbitration-fix review — adjudication

**Slice**: `641916e` — serialize tuner resize decisions with settlement.
**Reviewer**: `gpt-5.6-sol` @ `model_reasoning_effort = xhigh`.
**Raw review**: `.review/results/otp-12-worker-parity.arbitration-fix.codex.md`
**Review verdict**: **FAIL** — 1 LOW.
**Adjudication**: 1 finding, 1 ACCEPTED, 0 rejected, 0 deferred.

## LOW — accepted-settlement interleaving was inferred, not forced → ACCEPTED

The first deterministic guard paused the tuner and proved the epoch mutex was
held at that point, but then released and joined the tuner before applying the
accepted settlement. Dropping the guard after the hook but before signal/claim
could restore the stale-decision window while that schedule still passed.

Fix: start the matching accepted settler while the tuner is paused with the
epoch lock. Releasing the tuner must let it claim epoch 1 before the queued
settlement applies; the test then requires live count 2, no pending epoch, and
a reset cooldown. Mutation proof: dropping/reacquiring the epoch guard after
the hook makes the settlement miss epoch 1 and fails `accepted settlement
applied` (live count remains 4); continuous arbitration passes in debug and
optimized builds.

Full workspace fmt, strict clippy, and tests are green: 1,490 passed, 2
ignored, no failures. Documentation checks are green.

Fix commit: pending.

reviewer: gpt-5.6-sol
