# otp12-pf1-rigw-harness round 3 — Grok second-eye adjudication

- Reviewer: `grok-4.5` via `grok 0.2.101 (5bc4b5dfadcf)`, reasoning `high`
- Reviewed range: `4c7c7544db69289cf2e5fc0cf21093b40f00bc0d..53bb5e56a864abe0ee2d2b00c411846a1e7d24d5`
- Reviewed: `2026-07-15T10:58:21Z`
- Raw envelope: `.review/results/otp12-pf1-rigw-harness-r3.grok.json`
- Additive verdict: `REOPENED`
- Guard confirmed: `false`

The orchestrator verified an `EndTurn` envelope,
`structuredOutputError: null`, the exact base and reviewed SHAs, the registered
verdict enum, nonempty comments, and literal `guard_confirmed=false`. The
detached review worktree was restored clean at the reviewed SHA before removal.

## Adjudication

### G3 — path-parity mutation guard is vacuous under Bash 3.2

**Verdict: Accepted (High instrument-correctness gap).** The production
`destination_relative_path` correctly omits the initiator role, so this is not
evidence of a production path divergence. However, the self-test's central
role-parity assertions in `scripts/bench_otp12pf_rigw.sh` are bare `[[ ... ]]`
commands. On the intended macOS Bash 3.2 runtime, a false bare assertion in
that context can survive `set -e`. Grok added the role to the physical
destination path, observed different SOURCE-initiated and
DESTINATION-initiated paths, and the self-test still exited zero. That
falsifies the recorded F5 mutation proof and would allow the diagnostic to run
without proving its only-initiator-varies contract.

The timing-anchor mutation and the launcher PID-journal-before-gate mutation
both turned the self-test red and returned green after restoration. Grok also
confirmed one `Transfer` per arm, SOURCE sends / DESTINATION receives under
either caller, role-independent worker behavior, the fixed schedule, the
exclusive launcher-smoke branch, verify-only firewall handling, and
failure-preserving cleanup.

No rig run is authorized. G3 must make every path-construction/parity assertion
fail explicitly, prove the role-in-path mutation red-to-green on Bash 3.2, and
receive fresh complete Codex and additive Grok review.

Fix: `27c94b0`. Every path construction, role equality, and CLI destination
assertion now fails explicitly. Adding the initiator role to the physical path
turns the macOS Bash 3.2 self-test red; restoring the one canonical path returns
it to green. The follow-up systematic audit admitted the separate G4 lifecycle
guard finding, fixed at `7e9d2d5`; fresh review must cover both commits.

reviewer: grok-4.5
