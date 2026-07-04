# w1-4-accept-token-constants — adjudication of codex review

**Reviewed commit**: `6a19e1d` (code) — records commit `484e70b` was
docs-only (the finding doc) and carried no code.
**Raw review**: `.review/results/w1-4-accept-token-constants.codex.md`
**Reviewer**: gpt-5.5 (codex), adjudicated by the coding agent
**Codex verdict**: NEEDS FIXES (1 Low)
**Fix commit**: `d17b089`

## Per-finding adjudication

1. **stall_guard.rs:31/:65 — comments name the deleted
   `PULL_ACCEPT_TIMEOUT` / `PULL_TOKEN_TIMEOUT`** (Low, doc drift) —
   **Accepted.** Verified at source: both the module doc (audit-h3b
   paragraph) and the `TRANSFER_STALL_TIMEOUT` rustdoc referenced the
   names `6a19e1d` deleted. Fixed in `d17b089`: both now point at the
   shared `DATA_PLANE_*` pair in `remote::transfer::socket`. While
   editing those exact lines, also corrected the adjacent
   `daemon::service::{pull, pull_sync}` reference — the `pull` service
   died at ue-r2-1h, so the list is now `pull_sync` with a
   history note (same-line comment accuracy, disclosed here rather
   than silently bundled).

Codex's clean checks, spot-verified: every renamed use site kept its
original bound (accepts → 30 s, token reads → 15 s, no swaps —
confirmed by reading the post-commit grep of use sites);
`RESIZE_ARM_TTL` still equals the accept timeout; the surviving
`from_secs(30)` literals are different policy families (control-plane
connect bounds, test harness timeouts, `TRANSFER_STALL_TIMEOUT`); the
no-new-tests reasoning in the finding doc judged sound for a
byte-identical constant consolidation.

Validation after the fix: fmt clean, clippy clean (workspace, all
targets, `-D warnings`), `cargo test --workspace` 1446 passed / 0
failed / 2 ignored across 37 suites — unchanged; grep confirms zero
`PULL_ACCEPT_TIMEOUT` / `PULL_TOKEN_TIMEOUT` references remain.
