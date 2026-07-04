# w1-2-data-socket-policy-helper — adjudication of codex review

**Reviewed commit**: `16237e2` (code) — records commit `f60942e` was
docs-only (the finding doc) and carried no code.
**Raw review**: `.review/results/w1-2-data-socket-policy-helper.codex.md`
**Reviewer**: gpt-5.5 (codex), adjudicated by the coding agent
**Codex verdict**: PASS (0 findings)
**Fix commit**: none required

Codex returned an empty findings list. Its stated basis, checked against
the review transcript and source:

- Acceptance criteria met (W1.2): one shared
  `configure_data_socket(stream, tcp_buffer_size)` in blit-core, called
  from the pull client connect and every daemon accept path; the
  audit-era "three daemon accepts" resolve post-REV4 to the push
  accept family (3 call sites through the deleted private twin) plus
  the pull_sync epoch-0/resize/resume accepts.
- Socket policy stays outside framing/session construction: codex
  independently confirmed the remaining production `from_stream*`
  sites are all in `pull_sync.rs` with the socket configured
  immediately before wrapping — StallGuard, record framing, and byte
  accounting untouched (verified against the diff: no
  `write_all`/`read_exact`/accounting lines changed outside test
  scaffolding and the `let stream` → `let mut stream` rename).
- Resize accept postures preserved: push epoch-N arm still
  logs-and-lapses (`Err` arm at the `configure_data_socket` match);
  pull_sync `accept_one_resize_socket` errors still land in the
  caller's non-fatal settle-refused path; the two fatal accept paths
  stay fatal.
- Error-posture change (daemon nodelay silently swallowed →
  `Status::internal` on fatal paths / logged on the resize arm) judged
  defensible by codex; adjudicator concurs — it unifies on the core
  side's deliberate posture (nodelay hard per POST_REVIEW_FIXES §1.1
  lineage) and the daemon's fatal paths already hard-errored on the
  `into_std`/`from_std` conversions this change deleted.
- design-3 coordination honored: no connect timeouts added.
- Test count delta +3 confirmed (blit-core 414 → 417 measured at HEAD
  by stash-baseline; blit-daemon 168 unchanged; workspace 1445 green —
  codex could not run cargo read-only; the coding agent's gate run is
  the executable evidence).

No findings to accept, reject, or defer. Validation at the reviewed
commit: fmt clean, clippy clean (workspace, all targets,
`-D warnings`), `cargo test --workspace` 1445 passed / 2 ignored, 37
suites; helper + call-site wiring mutation-verified (M1/M2/M3, see the
finding doc).
