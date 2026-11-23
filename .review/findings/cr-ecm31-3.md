# cr-ecm31-3: a partial the cleanup could not remove was left unmarked and unreported

**Severity**: MEDIUM — a fresh partial whose first look at itself failed was left at no length, an empty file that could look finished. A partial whose readying failed and whose removal then failed (its directory barred, say) was left without a word: the error named only the preparation, and lookup and unlink errors were dropped.
**Status**: Fixed on Unix, awaiting review
**Branch**: — (default-branch mode; fixes land on master)
**Commit**: this commit (`Fix cr-ecm31-3`)
**Reviewer**: codex / gpt-6.2-astra (as dispatched) / max / frontier, codex-cli 0.171.0 headless one-shot, openreview `combined-r32` over `93ecf7b2..714ed0b7` (verdict `acceptable_with_changes`, capability_ok and guard_confirmed true); `.review/results/combined-r32.codex.md`. It completes cr-ecm30-8 (`f4b1249b`).

## Evidence
"The first failed metadata read after creation leaves a zero-byte file before its unfinished length is set. Later cleanup also discards lookup and unlink errors. In a focused check, loss of directory write permission left a 19-byte partial, and the returned error mentioned only preparation. Attempt the unfinished mark through the held handle and report any retained file and cleanup error."

## Intake
Admitted as the reviewer put it.

## What
In `Resolved::fresh_as`, a partial left behind is marked unfinished through the handle that made it: its length set one byte past its source's. The error then says where it is left and why. It also says whether the mark itself failed.
- The partial's first look at itself failing: left, marked, and said.
- Readying failing, then the removal failing, or the look at the name failing: left, marked, and said. The preparation's error stays underneath.
- Something else at the name by the time of the cleanup: nothing of this run's is there, and nothing is removed.

`FRESH_LOOK_FAULTS` is a new seam: the partial's first look refused (`EIO`).

## Guard proof
- `resume::tests::a_fresh_partial_its_cleanup_cannot_remove_is_left_marked_and_said` (Unix):
  - The readying fails after barring the destination's directory (0555). The error says "left at" the destination, and the partial is 9 bytes, marked unfinished. Where the process is not barred (root), this half is skipped and says so. Here it ran.
  - With the partial's first look refused, the error says it is marked unfinished, and it is 9 bytes.
- Mutations, each restored byte for byte:
  - a removal that failed dropped → red;
  - a partial left without its mark → red.
- The resume, sink, session and copy tests (355) pass. Windows and macOS (`CARGO_FEATURE_PURE=1`) cross-clippy are clean.

## Files changed
- `crates/blit-core/src/copy/file_copy/resume.rs` — `fresh_as`, `fresh_look_fault`; guard.

## Known gaps
- A partial left because the readying itself was refused its length (a file-size limit) cannot be marked that way either. The error says so ("could not be marked unfinished").
