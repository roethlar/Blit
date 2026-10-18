# cr-g3fix6-3: A recursive prefix followed by a crossing glob adds quadratic work per entry

**Severity**: MEDIUM (reviewer: MEDIUM)
**Status**: Fixed
**Branch**: — (default-branch mode; fixes land on master)
**Commit**: `0767b17e`
**Reviewer**: codex / gpt-6-astra / xhigh / standard (the owner's `review`, 2026-10-18), codex-cli 0.162.1 headless one-shot, `-a never exec -m gpt-6-astra -c model_reasoning_effort="xhigh" -s workspace-write --json`, stdin closed, detached worktree `review-g3fix6-r1-19ace1ac`; codereview re-review g3fix6-r1 over `a157ae00..19ace1ac` (record `.review/results/g3fix6-r1.codex.json` + `.events.jsonl`); 2026-10-18 00:12–00:22Z; capability_ok=true; guard_confirmed=true (reviewer disabled `escaped_separator`'s star-run collapse → `an_escaped_separator_keeps_its_meaning_in_the_class_dialect` red, exit 101; restored by `git checkout`, blob hash verified, green; 1548/0/2 on its macOS gate, Linux cross-clippy clean, Windows cross-clippy stopped on a missing `ml64.exe` — its environment; it cites CI run 39080085302 green on all three OSes); SHAs pin-verified; closed: cr-g3fix5-3 true, cr-g3fix5-4 true, cr-g3fix5-1 false, cr-g3fix5-2 false, cr-g3fix4-2 false, cr-g3fix-1 false

## Evidence
crates/blit-core/src/transfers/filter.rs:1274 — advance joins and folds the complete suffix for every retained crossing-glob start; line 1280 retains each start separately. With '**/*/missing', the preceding '**' creates another start at every depth, producing quadratic work per entry. On the head's debug binary, a dry-run mirror with 250 destination files took approximately 47, 133, 463, and 2010 ms at depths 20, 40, 80, and 160. The '**/missing' control took approximately 14, 26, 52, and 137 ms and produced identical deletion counts.

## Predicted observable failure
Deep mirror trees become disproportionately slow under recursive exclusions containing a spanning glob. The repeated suffix processing is paid for every unexcluded destination file even when the exclusion matches nothing.

## Reviewer's suggested approach
Carry incremental matcher state or merge equivalent crossing states instead of rebuilding and refolding every suffix for each entry. Verify bounded depth scaling with a representative recursive-exclusion benchmark.

## Intake
Admitted on the reviewer's measurement (dry-run `mirror` with 250 destination files under `--exclude '**/*/missing'` took ≈47/133/463/2010 ms at depths 20/40/80/160 on the debug binary, against ≈14/26/52/137 ms for `**/missing`): `advance` (`filter.rs:1274-1280`) joins and folds the complete suffix for every retained crossing-glob start, and a preceding `**` opens a start at every depth. Admitted as MEDIUM (FAST): the cost is paid per unexcluded destination entry even when the rule matches nothing.

## What
A `**` followed by a crossing glob costs the deletion walk time linear in depth per entry, not quadratic.

Fix shape, admitted: with the fold gone, a path rule's components are matched against as-written text; the `Step` reading of `**` followed by a crossing glob keeps one state per rule per depth (merge equivalent starts, or match the rule's compiled remainder once against the joined remainder of the path) so the work per entry is linear in depth. Guard: a test that counts the matcher's work (or measures it) for `**/*/missing` at depths 20 and 160 and asserts it grows no faster than linearly; mutation: the quadratic retention again → red.

## Approach
- A `**` immediately before a glob component that can span `/` compiles one combined `**/glob` matcher. It carries the first possible start through the destination walk and advances past both steps when matched. `**/` can absorb everything before a later start, so a later start cannot add a match. State insertion merges equivalent combined states by retaining the earliest start.
- Other path-rule steps retain their existing reading. A focused guard checks matching at shallow and deep paths, a literal prefix before the recursive component, and a glob that spans separators. It also inspects the retained states at depths 1, 20 and 160: at most two for the rule, including the state after the crossing glob.

## Files changed
- `crates/blit-core/src/transfers/filter.rs` — combined matcher and state coalescing; bounded-state and matching guard.

## Guard proof
- Mutation: disabled the combined matcher. At depth 20 the guard failed with 23 retained states (limit two). The mutation was removed.
- macOS: the post-restore guard passed (1/1), the filter suite passed (37/37), and the CLI exclude-contract suite passed (25/25). `cargo fmt --all -- --check` and native strict workspace clippy passed. Linux cross-target strict workspace clippy passed. The completed round's full workspace suite passed 1560/0/2 over 47 binaries (cargo exit 0). Windows MSVC cross-clippy stopped in `blake3`'s build script because this Mac has no `ml64.exe`; the GNU target is not installed.

## Known gaps
- This removes the per-entry state growth for `**` immediately followed by a crossing glob. Other glob layouts were not benchmarked in this finding.

## Reviewer comments
(filled by the re-review)
