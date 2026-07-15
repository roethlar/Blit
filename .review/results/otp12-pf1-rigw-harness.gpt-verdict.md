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

Fix: `661cf75`. q deletion failures now propagate explicitly and are followed
by an absence check plus creation of a plain empty directory. The Windows
branch uses terminating removal, then proves absence, directory type, no
reparse point, and emptiness. The shell self-test mutation-proves both prior
fail-open shapes.

### F2 — uncharged post-client delay can bias durability

**Verdict: Accepted (High).** All three after-clock SSH samples run before the
destination flush. If those probes overrun the fixed 250 ms deadline, the
harness accepts up to 999 ms of post-client writeback while `total_ms` remains
only `transfer_ms + flush_ms`. A synthetic 128-arm session with every
`settled_ms=999` receives `ANALYSIS-PASS` without changing any durable total.
That delay is neither fixed nor charged and can reduce the subsequent flush
differently between adjacent role arms.

Fix: `1617546`. The registered first 250 ms remains common excluded time, but
every excess settle millisecond is now charged:
`total_ms = transfer_ms + (settled_ms - 250) + flush_ms`. The analyzer verifies
that exact Decimal identity and exports `settled_ms`; mutations of the harness
formula, analyzer formula, and export all fail. An equal client-to-durability
regression proves differing settle/flush partitions cannot create a role
delta.

### F3 — two resize prerequisites are not causal guards

**Verdict: Accepted (High).** Two direct synthetic mutations both receive
`ANALYSIS-PASS`: moving destination-initiator `socket_dial_begin` ahead of
`resize_received`, and moving source-initiator `socket_accept_begin` ahead of
`resize_arm_ready`. The Rust emitter establishes both prerequisite chains, so
the analyzer must require them before using the phase trace for attribution.

Fix: `2dd977e`. A complete audit found eight deterministic local resize edge
families, including the two reported by the reviewer; the analyzer now asserts
all eight under both applicable initiator layouts. Mutation cases reverse each
edge while preserving valid local producer sequences. The audit also found
that both SOURCE dial emitters attached the socket trace before
`socket_dial_end`; runtime emission now matches the causal contract, with a
Rust mutation proof at epoch zero and resize epoch one. Concurrent send/ACK
and all cross-endpoint orderings remain deliberately unasserted.

Fresh Codex review of the fixes is pending; these accepted fixes do not by
themselves close the review row.

## Validation context

Before review, formatting, strict Clippy, documentation checks, harness
self-tests, all 19 analyzer tests, and a complete workspace test run passed.
One first-run `admin_verbs::test_admin_rm` integration flake passed immediately
in isolation and the complete workspace rerun passed; the repository already
records this daemon-harness wrong-listener flake class.
