# w1-3-tcp-keepalive-honesty — adjudication of codex review

**Reviewed commit**: `865fc1e` (code) — records commit `894f5e8` was
docs-only (the finding doc) and carried no code.
**Raw review**: `.review/results/w1-3-tcp-keepalive-honesty.codex.md`
**Reviewer**: gpt-5.5 (codex), adjudicated by the coding agent
**Codex verdict**: PASS (0 findings)
**Fix commit**: none required

Codex returned an empty findings list. Its stated basis, checked
against the review transcript and source:

- Acceptance criteria met (W1.3, real-timing exit): the bare
  `set_keepalive(true)` is gone; the single shared helper (both
  audit-era sites collapsed into it by w1-2) configures
  `TcpKeepalive` 60 s / 10 s / 5 and logs failure; the comments now
  describe detection ("~2 minutes instead of ~2 hours") rather than
  asserted liveness.
- RELIABLE risk questions run to ground: TCP keepalive kills only
  unresponsive peers (a healthy-idle connection answers probes with
  ACKs — no false-positive kill class); the ~110 s detection window is
  strictly more conservative than the gRPC control plane's HTTP/2
  keepalive (audit-1b), so the data plane never declares death first;
  StallGuard's 30 s continues to own the active-transfer no-progress
  case. Adjudicator concurs on all three from source.
- Windows semantics verified: socket2 sets retries via `TCP_KEEPCNT`
  on Windows 10+; `features = ["all"]` adds API surface only — no new
  dependencies, no Cargo.lock delta (codex checked CI's three-OS
  matrix compiles the same feature set).
- Test count preserved: blit-core 417 → 418, workspace 1445 → 1446
  green (codex read the recorded evidence; the coding agent's gate run
  is the executable proof — codex cannot run cargo read-only).

No findings to accept, reject, or defer. Validation at the reviewed
commit: fmt clean, clippy clean (workspace, all targets,
`-D warnings`), `cargo test --workspace` 1446 passed / 2 ignored, 37
suites; the timing pin mutation-verified (bare-enable revert fails the
kernel read-back test; see the finding doc).
