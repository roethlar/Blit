# cr-fix3-1: Collision shielding preserves an unrelated lossy-text subtree

**Severity**: MEDIUM — while a raw-named source entry keeps failing, a distinct valid-UTF-8 destination path equal to its lossy rendering (and its subtree) is never deleted by mirror, so the destination cannot converge
**Status**: Verified
**Branch**: — (default-branch mode; fixes land on master)
**Commit**: `3c3e56fb`
**Reviewer**: codex / gpt-5.6-sol / xhigh / standard (D-2026-07-31-3 standing codereview; range be0b9fda..5251584c (review-fix batch 3), record .review/results/ssc-fix3-range.codex.json)

## Evidence
crates/blit-core/src/mirror_planner.rs:305 — The new unconditional insertion shields the lossy text path even when raw_names_storable is true; lines 275-282 establish that without a representable source entry this text path is a distinct extraneous destination identity.

## Predicted observable failure
On a byte-keyed Linux/BSD destination, if a raw non-UTF-8 source entry fails while the destination also contains a valid UTF-8 path equal to its lossy rendering, mirror skips that unrelated path and its descendants. The partial run leaves stale destination data, and repeated runs cannot converge while the source failure persists.

## Reviewer's suggested approach
On storable destinations, shield every matching decoded raw path but add the text path only when source_files contains that representable identity; on unstorable destinations, continue shielding the text identity. Carrying the failed entry's structured raw identity would make this exact.

## What
`plan_session_deletions` (`mirror_planner.rs`) still shields every raw-named entry collapsing to a failed text identity (decoded where storable, cr-fix2-3), but now inserts the text path itself only where it is a real identity of the failure: always when raw names are unstorable (the text is then the raw entry's one identity, cr-ssc5-5), and on a storable host only when no raw entry claims the text or a representable source entry (`source_files`) carries that very name. On a byte-capable destination a valid-UTF-8 path equal to a failed raw entry's lossy rendering, with no representable source entry of that name, is therefore extraneous and deleted like any other, so the destination converges while the raw failure persists.
Judgment call beyond the reviewer's wording: the "no raw entry claims the text" clause keeps the shield for failures tied to no raw entry at all, e.g. a retry scan's `ManifestComplete.scan_failures` path (a requested file missing at retry is in neither `source_files` nor `raw_entries`). A literal "only when `source_files` contains it" rule would have dropped that cr-ssc1-1 shield.

## Guard proof
Guard: `mirror_planner::shield_tests::a_failed_raw_entry_shields_its_lossy_text_only_as_a_real_identity`. The destination holds a populated valid-UTF-8 dir `caf\u{fffd}dir/child.bin`, the failed raw entry's decoded path `realdir/child.bin`, and `extraneous.bin`; one raw entry (text `caf\u{fffd}dir`, bytes `realdir`) is the failure. Four arms: (1) storable, no representable entry → `extraneous.bin` + `caf\u{fffd}dir/child.bin` files and the `caf\u{fffd}dir` dir planned, `realdir` untouched; (2) storable, `source_files` holds `caf\u{fffd}dir` → only `extraneous.bin`; (3) unstorable → the text subtree is kept, `realdir` is extraneous; (4) storable, no raw entries → the failed text path keeps its shield. ASCII raw bytes and the explicit `raw_names_storable` argument keep it portable (ungated, runs on every platform).
Red/green and mutations (`scratchpad/cr-ssc-mutations-4.txt`): against the unfixed cr-fix2-3 code (unconditional text insertion) the guard FAILED at arm 1 (only `extraneous.bin` planned); fix applied → green, all six shield tests pass. Mutations, each restored byte-identical from backup: unconditional insertion restored → FAILED (arm 1); representable clause dropped → FAILED (arm 2); unstorable clause dropped → FAILED (arm 3); unclaimed clause dropped → FAILED (arm 4). Restored → 6/6 green.
Gate (macOS, at `3c3e56fb`): fmt clean; clippy `-D warnings` clean native and `x86_64-unknown-linux-gnu`; `cargo test --workspace` 1330 → 1331 passed / 0 failed / 2 ignored; check-docs OK; diff-check clean.

## Known gaps
Still conservative where a representable source entry and a raw entry share the failed text: the failure is recorded by text only, so the shield cannot tell whose it was and shields both identities. Carrying a structured raw identity in the failure record (the reviewer's "exact" option) would remove that residue; not needed for safety. Production storable hosts are Linux/BSD; the guard exercises the storable logic on macOS through the explicit argument, and no Linux run or CI was executed for this fix (CI unverified until a push).
