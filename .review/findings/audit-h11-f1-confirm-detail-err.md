# audit-h11 — F1 confirm-detail must not lie about move deletion target

**Source**: 2026-06-04 audit chain, R3 finding **H11** (data-loss-adjacent UI lie).
**Memory**: `feedback_endpoint_parse_err` — 4-bucket classification rule that Err must reject.
Reopened d-61, d-68 ×3 historically for this exact pattern.

## What

`crates/blit-tui/src/display_f1.rs:46-54` previously classified the move-source endpoint
for the F1 confirm-detail phrase with:

```rust
match parse_transfer_endpoint(source) {
    Ok(Endpoint::Remote(_)) => "deletes the remote source",
    _ => "deletes the local source",
}
```

The `_` arm folded `Err(_)` AND `Ok(Endpoint::Local(_))` together. The state-machine
invariant from `plan_f1_trigger` (`main.rs:3588` rejects `Err` sources via
`TriggerOutcome::Rejected`) means the renderer should never see `Err` in practice — but
the catch-all silently classifies a future-loosened gate's `Err` arm as "local source."
That's a data-loss-adjacent UI lie: the operator confirms "yes, delete the local side"
while the actual delete might run against an unparseable string the system chose to
interpret differently.

## Approach

1. Extract the classifier into a testable helper
   `move_delete_target_phrase(source: &str) -> &'static str`.
2. Match all three arms explicitly: `Ok(Remote)`, `Ok(Local)`, `Err(_)`.
3. `Err` arm uses `debug_assert!(false, …)` paired with the gate-invariant pointer —
   debug builds panic loudly with the source string in the message; release builds
   degrade to "(parse error — refusing to classify)" rather than silently claiming
   "local source."
4. Add 4 unit tests pinning the property.

The `debug_assert!` is the load-bearing guard. The "(parse error)" release-mode string
is a degrading safety net so a real operator never sees a confident-but-wrong
deletion-target phrase even if the assert is compiled out.

## Files changed

- `crates/blit-tui/src/display_f1.rs`:
  - Replace the inline `confirm_detail` match-on-`PullKind::Move` with a call to
    `move_delete_target_phrase(source)`.
  - Add the helper with full Local / Remote / Err arms.
  - Add `mod tests` with 4 regression tests.

## Tests added

- `move_delete_target_phrase_classifies_remote_source` — `host-a:/m/` →
  `"deletes the remote source"`.
- `move_delete_target_phrase_classifies_root_remote_source` — `host-a://root/sub` →
  `"deletes the remote source"` (covers the `RemotePath::Root` variant per
  `feedback_endpoint_parse_err`).
- `move_delete_target_phrase_classifies_local_source` — `/tmp/src` →
  `"deletes the local source"`.
- `move_delete_target_phrase_local_and_remote_differ` — pins the load-bearing
  property: local and remote produce distinct phrases (a fold would have made them
  identical and silently lied about a remote-source move). Any future pattern collapse
  would fail this test.

Workspace test count went 642 → 646 in blit-tui (= the 4 new tests). All other
suites unchanged. Validation suite green: fmt --check, clippy -D warnings,
full test --workspace.

## Known gaps

- The release-mode `debug_assert!` is compiled out; the helper falls through to the
  non-lying string. That's the intended degradation behavior — better to show a
  vague "(parse error)" than a confident lie — but it does mean an Err in release
  builds is silent unless the operator notices the phrase change. The load-bearing
  invariant (`plan_f1_trigger` rejects Err before reaching this state) is the real
  safety net; this slice's job is to make a future regression of that gate fail
  loudly in debug.
- No test exercises the `Err` arm directly. `parse_transfer_endpoint` is permissive
  enough that constructing a definitively-Err input proved brittle (empty string,
  for example, parses as `Local("")`). The four-way match's exhaustiveness on the
  Ok side + the debug_assert in the Err arm is the structural guard.

## Cross-references

- R3 finding H11, see `docs/audit/AUDIT_REPORT_2026-06-04_R3.md`.
- Memory `feedback_endpoint_parse_err` (rule + d-61/d-68 reopen history).
- `plan_f1_trigger` at `crates/blit-tui/src/main.rs:3588` — the upstream gate that
  enforces the invariant the helper's debug_assert documents.
