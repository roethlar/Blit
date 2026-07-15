# otp12-pf1-rigw-harness — GPT verdict

- Reviewer: `gpt-5.6-sol` (`xhigh`) via `codex-cli 0.144.4`
- Reviewed range: `4c7c7544db69289cf2e5fc0cf21093b40f00bc0d..0fb8237c2e6f63feb9cfc613d8af1602730061b0`
- Review timestamp: `2026-07-15T08:40:41Z`
- Raw review: `.review/results/otp12-pf1-rigw-harness.codex.md`
- Review verdict: `NEEDS FIXES`

## Adjudication

### F1 — destination reset can fail open

**Verdict: Accepted (High).** `run_arm` invokes `prepare_destination` on the
left side of `||`. Bash consequently disables inherited `errexit` inside the
function: a failed q `rm -rf` is masked when the following `mkdir -p`
succeeds. A Bash 3.2 probe replaced `rm` with an rc-73 failure and observed
`prepare_destination` return success while a stale file survived. The Windows
branch separately uses `Remove-Item -ErrorAction SilentlyContinue` before a
successful `New-Item`, which has the same fail-open shape. A stale canonical
tree could therefore be accepted while the current transfer skips work.

Fix: pending. Make both exact deletion and post-deletion absence explicit,
then prove the newly created destination is an empty plain directory.

### F2 — uncharged post-client delay can bias durability

**Verdict: Accepted (High).** All three after-clock SSH samples run before the
destination flush. If those probes overrun the fixed 250 ms deadline, the
harness accepts up to 999 ms of post-client writeback while `total_ms` remains
only `transfer_ms + flush_ms`. A synthetic 128-arm session with every
`settled_ms=999` receives `ANALYSIS-PASS` without changing any durable total.
That delay is neither fixed nor charged and can reduce the subsequent flush
differently between adjacent role arms.

Fix: pending. Remove observer work from the pre-flush interval and fail closed
on excess uncharged settle latency, with mutations for both ordering and the
upper bound.

### F3 — two resize prerequisites are not causal guards

**Verdict: Accepted (High).** Two direct synthetic mutations both receive
`ANALYSIS-PASS`: moving destination-initiator `socket_dial_begin` ahead of
`resize_received`, and moving source-initiator `socket_accept_begin` ahead of
`resize_arm_ready`. The Rust emitter establishes both prerequisite chains, so
the analyzer must require them before using the phase trace for attribution.

Fix: pending. Add the two endpoint-local causal assertions and mutation tests
that reverse each edge.

## Validation context

Before review, formatting, strict Clippy, documentation checks, harness
self-tests, all 19 analyzer tests, and a complete workspace test run passed.
One first-run `admin_verbs::test_admin_rm` integration flake passed immediately
in isolation and the complete workspace rerun passed; the repository already
records this daemon-harness wrong-listener flake class.
