# cr-ecm18-3: the check before a failed write's second removal was two-way

**Severity**: MEDIUM — a link put at the name between a failed write's two removals, to the partial itself, leaves the partial reachable there, reported removed
**Status**: Fixed, awaiting review
**Branch**: — (default-branch mode; fixes land on master)
**Commit**: this commit (`Fix cr-ecm18-3`)
**Reviewer**: codex / gpt-6.2-astra (as dispatched) / max / frontier, codex-cli 0.171.0 headless one-shot, openreview `combined-r19` over `ca377805..17f4eb97` (verdict `acceptable_with_changes`, capability_ok and guard_confirmed true); `.review/results/combined-r19.codex.md`. It completes cr-ecm17-2 (`6c1cc68d`).

## Evidence
"If the first unlink fails and the name becomes a symlink to the held partial, the second check reports `REMOVED` even though the partial is still reachable there. Preserve the incomplete-file outcome on that path."

cr-ecm17-2 told three cases apart before the first removal — the file held, a link to it, somebody else's — and, before the second, only two.

## Intake
Admitted as the reviewer put it.

## What
Before the second removal the same three cases: a link to the file held is reported left in place (it was marked through the handle before the handle was let go), not removed.

## Guard proof
- `sink::…::a_second_removal_is_asked_whose_file_stands_there` (extended to two cases): between the two removals, somebody else's file is moved over the name — whole, nothing of the write's said to be there; or the partial is moved aside and a link to it put at the name — reported left in place, not removed, the link standing.
- Mutation, restored byte for byte: the second check two-way, as it was (red).

## Files changed
- `crates/blit-core/src/remote/transfer/sink.rs` — `settle_failed_target_known`; guard.

## Known gaps
- None beyond cr-ecm17-2's.
