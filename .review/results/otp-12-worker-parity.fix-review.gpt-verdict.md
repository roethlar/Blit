# otp-12-worker-parity fix review — adjudication

**Slice**: `cfd9dd7` — first-byte-safe convergence and terminal resize refusal.
**Reviewer**: `gpt-5.6-sol` @ `model_reasoning_effort = xhigh`.
**Raw review**: `.review/results/otp-12-worker-parity.fix-review.codex.md`
**Review verdict**: **FAIL** — 1 MEDIUM.
**Adjudication**: 1 finding, 1 ACCEPTED, 0 rejected, 0 deferred.

## MEDIUM — terminal refusal remains racy across producers → ACCEPTED

Both proposal APIs read the terminal-refusal atomic before attempting a
separate pending-epoch CAS. A producer could read `false`, pause while the
matching refusal consumed and cleared the pending epoch, then win the CAS and
reopen the supposedly terminal resize policy. The sequential refusal test did
not exercise that interleaving.

Fix: settled epoch, pending epoch, and terminal refusal are one
mutex-protected arbitration state. Both the shape path and tuner recheck and
claim that state while deriving live count, target, and epoch; settlement
updates it atomically. This also closes the adjacent accepted-settlement ABA
case, where a stale producer could otherwise reuse an epoch and stale live
count after a complete proposal/settle cycle.

The initial guard raced eight shape/tuner producers against refusal and passed
51 consecutive runs; removing terminal state made one proposal escape. The
next independent review correctly found that guard scheduler-dependent and
not protective of accepted settlement. It was replaced by the deterministic
lock/settlement guard recorded in
`.review/results/otp-12-worker-parity.atomic-fix.gpt-verdict.md`.

Full workspace fmt, strict clippy, and tests are green: 1,490 passed, 2
ignored, no failures. Documentation checks are green.

Fix commit: `8e993aa`.

reviewer: gpt-5.6-sol
