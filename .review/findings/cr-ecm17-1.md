# cr-ecm17-1: a record cancelled while it noted its file left an empty file

**Severity**: HIGH — a cancelled transfer under `--ignore-existing` leaves an empty file at the destination, which every later run keeping what exists skips as somebody's
**Status**: Fixed on Unix, awaiting review; the Windows half (cr-ecm16-2's) stays open
**Branch**: — (default-branch mode; fixes land on master)
**Commit**: this commit (`Fix cr-ecm17-1`)
**Reviewer**: codex / gpt-6.2-astra (as dispatched) / max / frontier, codex-cli 0.171.0 headless one-shot, openreview `combined-r18` over `7912da16..ca377805` (verdict `acceptable_with_changes`, capability_ok and guard_confirmed true); `.review/results/combined-r18.codex.md`. It completes cr-ecm16-2 (`064534f4`).

## Evidence
"I cancelled `begin_record` during the new metadata lookup. The head left a zero-byte file that a later `--ignore-existing` run skips. The base sink removed it under the same cancellation. Arm cleanup as part of creation and keep it armed through commit. On Windows, an unavailable identity currently lets a replacement pass the delivery check. Require handle-based identity before reporting delivery or touching a replacement."

cr-ecm16-2 noted which file a record made by awaiting the file's metadata BETWEEN making the file and making the writer whose guard removes it. A record cancelled at that await had no guard. And `commit` let its guard go before awaiting the flush and the hand-back of its handle, so a record cancelled in those had none either.

## Intake
Admitted. The Windows sentence is cr-ecm16-2's open half and stays open (no Windows machine).

## What
- `begin_record` makes the writer as soon as the file is made, and only then awaits which file it is; a cancellation there drops the writer, and its guard removes the file.
- `commit` keeps the guard armed through `flush` and `into_std`, and lets it go only once nothing more is awaited.

## Guard proof
- `sink::…::a_record_cancelled_as_it_notes_its_file_leaves_nothing`: a record of a run keeping what exists is polled by hand until its file is made (a seam says so) and it waits to learn which file that is — then dropped. Nothing is left at the name. Three runs, green each time.
- Mutation, restored byte for byte: the writer not armed while it notes its file (red).
- The sink tests pass (101).

## Files changed
- `crates/blit-core/src/remote/transfer/sink.rs` — `begin_record`, `commit`; guard.

## Known gaps
- Cancellation inside `commit`'s two awaits has no deterministic guard: whether the flush waits at all depends on the blocking pool's timing. The guard is armed there by reading, not by a test.
- Windows: the delivery check has no identity to compare (cr-ecm16-2's open half).
