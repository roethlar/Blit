# Verdict — w4-5-supports-cancellation-flip

Slice commit: `05a8b39`. Codex verdict: **NEEDS FIXES** (1 finding,
Low). reviewer: gpt-5.5

## Finding 1 — active_jobs.rs module rustdoc stale on Pull wiring (Low)

- Claim: the module-level "Scope so far" rustdoc still says `Pull` is
  wired at dispatch (b-1 bullet: "wiring at the `pull` and
  `delegated_pull` dispatch sites") and that "All four `ActiveJobKind`
  variants are now actually constructed on the wire path" (b-2
  bullet), contradicting the new `supports_cancellation` policy
  rustdoc which says `Pull` is history-only.
- Adjudication: **Accepted.** Verified against
  `crates/blit-daemon/src/active_jobs.rs:9-21` — the "Scope so far"
  list is a milestone changelog, and both statements were true when
  their slices landed, but the b-2 sentence is present-tense ("are now
  actually constructed") and sits 140 lines above a policy rustdoc
  this slice rewrote to say the opposite. A maintainer reading top-down
  gets contradicted before reaching the authoritative text. Same drift
  class codex caught in w1-4 (stall_guard comments naming deleted
  constants).
- Fix: `1708075` — both bullets annotated in place, changelog framing
  preserved: b-1 notes the `pull` dispatch site died with the Pull RPC
  at ue-r2-1h; b-2's claim is past-tensed and points at
  [`ActiveJobKind::supports_cancellation`] for the current state.

## Gate

Slice commit `05a8b39`: fmt clean; clippy `-D warnings` clean;
`cargo test --workspace` 37 suites, 1448 passed / 0 failed / 2 ignored
(baseline 1446 → +2; blit-daemon 168 → 170). Fix commit `1708075`
(comment-only): full gate re-run, identical results.

Mutation verification (slice): reverting `supports_cancellation` to
`matches!(self, ActiveJobKind::DelegatedPull)` fails exactly the four
tests that pin the flip —
`supports_cancellation_matches_dispatch_policy` (Push assert),
`cancel_fires_token_for_push_and_pull_sync` (Unsupported ≠ Cancelled),
`cancel_authorized_bypass_and_precedence` (Unsupported ≠ Unauthorized),
`cancel_job_ok_for_push_and_pull_sync` (FailedPrecondition status) —
15 neighbors still pass; flip restored, all 19 cancel-filtered tests
green (`logs/w4-5-mutation.log`, `logs/w4-5-restore.log`).
