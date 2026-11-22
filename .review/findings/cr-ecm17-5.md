# cr-ecm17-5: the small-file probe's close timer timed a hand-over, not a close

**Severity**: LOW — a diagnostic's figure: the reported cost of closing a shard member's file read as nothing
**Status**: Fixed, awaiting review
**Branch**: — (default-branch mode; fixes land on master)
**Commit**: this commit (`Fix cr-ecm17-5`)
**Reviewer**: codex / gpt-6.2-astra (as dispatched) / max / frontier, codex-cli 0.171.0 headless one-shot, openreview `combined-r18` over `7912da16..ca377805` (verdict `acceptable_with_changes`, capability_ok and guard_confirmed true); `.review/results/combined-r18.codex.md`. It follows from cr-ecm16-2 (`064534f4`).

## Evidence
"One lower-severity correction concerns the Unix small-file diagnostic. Its `close` timer now measures moving the descriptor into `Some`. Measure the actual descriptor drop after metadata stamping so the reported close cost remains meaningful."

cr-ecm16-2 kept a shard member's handle through its metadata tail on Unix; the probe's `close` timer stayed where the close used to be, around the hand-over.

## Intake
Admitted as the reviewer put it.

## What
The probed shard writer's `close` is the time of the hand-over (where the handle is closed there, off Unix) plus the time of the drop after the tail (where it is kept, Unix). A member whose write fails is settled with its handle as before.

## Guard proof
None that can go red: the change is to what a duration measures, and no duration of a close can be told from that of a move by a deterministic test. `sink::…::probed_tar_shard_writer_contains_one_members_failure`, which drives the probed writer, passes; the sink tests (98) pass. Stated here, not claimed as proven.

## Files changed
- `crates/blit-core/src/remote/transfer/sink.rs` — the probed shard writer.

## Known gaps
- No guard (above).
