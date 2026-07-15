# otp-12-worker-parity — adjudication of Codex review

**Slice**: `a76b785` — initiator-independent stream target and shared
receiver-ceiling semantics.
**Reviewer**: `gpt-5.6-sol` @ `model_reasoning_effort = xhigh`.
**Raw review**: `.review/results/otp-12-worker-parity.codex.md`
**Review verdict**: **FAIL** — 1 HIGH, 1 MEDIUM.
**Adjudication**: 2 findings, 2 ACCEPTED, 0 rejected, 0 deferred.

## HIGH — pre-dispatch convergence delays first byte and can trip StallGuard → ACCEPTED

The implementation awaited every one-stream resize epoch before queueing a
payload. With an initial worker count of 1 and a target up to 32, that placed as
many as 31 control RTTs and socket connects ahead of useful work while receive
workers already had 30-second idle guards. Loopback exact-count pins could not
expose that latency/stall regression.

Fix: payloads now enter the existing bounded work-stealing pipeline immediately,
with queue sends selected against resize/control events. Once every payload is
queued, the input closes before residual convergence, so idle and late-added
workers emit END instead of waiting through the tail ramp. A deterministic test
holds resize ACK #2 and proves all 2,000 files move before release under both
initiator layouts, followed by exact target 4 and identical trees.

## MEDIUM — a refused resize can be reproposed → ACCEPTED

`resize_settled(..., false)` cleared the pending slot without consuming the
epoch or recording terminal refusal. A later plain or resume batch could
therefore propose the same target and epoch with a new credential, contradicting
the monotonic epoch contract and potentially repeating forever.

Fix: a matched refusal preserves live count but consumes its epoch, records
terminal refusal before clearing pending, and disables both shape and tuner
proposals for the rest of the transfer. The unit pin is mutation-proven: omit
the terminal record and the next shape proposal appears immediately.

## Validation

- Nonblocking gated-ACK pin: green under both initiator layouts; mutation red.
- Exact-target 10,000-file pins: source initiator = 8, destination initiator = 8.
- Terminal-refusal pin: green; mutation red.
- Full workspace fmt/clippy/test gate: green (1,489 passed, 2 ignored; 1,491
  test functions, no failures).

reviewer: gpt-5.6-sol
