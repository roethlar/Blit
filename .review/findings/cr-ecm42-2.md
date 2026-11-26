# cr-ecm42-2: the detached reader asked only the last session's summary for the delivery evidence

**Severity**: HIGH — a detached run's record keeps a session's leftovers and calls itself whole. cr-ecm41-1's detached half.
**Status**: Verified — `combined-r44` (Codex openreview, 2026-11-26) reviewed `4e12d761..34c77a48` — the `combined-r43` fixes, GitHub CI's three and gate-put-race — with `guard_confirmed: true`: "I would keep the shared `LogShape` validator, per-session delivery checks, scripted manifest order and synchronized `--ignore-existing` fixture. The following cases still need changes."
**Branch**: — (default-branch mode; fixes land on master)
**Commit**: `44ad5ac6`
**Reviewer**: codex / gpt-6.2-astra (as dispatched) / max / frontier, codex-cli headless one-shot, openreview `combined-r43` over `e2c2e08d..4e12d761` (verdict `acceptable_with_changes`, capability_ok and guard_confirmed true); `.review/results/combined-r43.codex.md`.

## Evidence
"A first session whose summary lacked `all_events_logged` correctly read as incomplete. Adding a second, complete session made the combined result complete, and `settle` produced an untruncated record that retained the first session's leftover. A missing or false delivery marker in any contributing session must keep the aggregate incomplete."

## Intake
Admitted as written.

## What
A session's summary counts as given only where it holds the evidence. Each session's own is asked at its `end_log`; the last one's no longer speaks for all.

## Guard proof
- `job_record::tests::a_detached_log_is_whole_only_in_sequence_with_its_summary_then_its_end`: a session whose summary holds no evidence, then a whole one → not complete.
- Mutation, restored byte for byte: any session's summary taken, evidence or not → red.
- Checks:
  - native, Windows and macOS (`CARGO_FEATURE_PURE=1`) clippy are clean;
  - the workspace passes on Linux, with the round's fixes: 2,156 tests over 57 binaries.

## Files changed
- `crates/blit-core/src/job_record.rs` — `LogRead`; guard.

## Known gaps
- cr-ecm42-1 folds this per-session check into the shape both readers share.
