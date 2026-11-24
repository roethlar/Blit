# cr-ecm35-4: a resume abort forgot that the receiver made the partial it held

**Severity**: MEDIUM — a run reports nothing left where it left a partial. A wire resume whose receiver made a fresh partial (the comparison's whole-file baseline) and wrote a block met a source that closed the record failed. The partial stayed, unstamped and one byte longer than its eight-byte source, but the outcome said `left_in_place = false`. cr-ecm34-4 kept a held failure's account across the source's abort, but the hold itself did not say whether it had made the file.
**Status**: Fixed on Unix; awaiting review
**Branch**: — (default-branch mode; fixes land on master)
**Commit**: this commit (`Fix cr-ecm35-4`)
**Reviewer**: codex / gpt-6.2-astra (as dispatched) / max / frontier, codex-cli headless one-shot, openreview `combined-r36` over `dc14afc4..d2494965` (verdict `acceptable_with_changes`, capability_ok and guard_confirmed true); `.review/results/combined-r36.codex.md`. It completes cr-ecm34-4 (`13675159`).

## Evidence
"After the receiver created a fresh partial and wrote a block, a source abort left nine bytes for an eight-byte source and reported `left_in_place = false`. Retain whether the receiver created the held file and use that fact when closing the record."

## Intake
Admitted as the reviewer put it.

## What
- **The hold records `made`.** `HeldPartial` says whether this run made its partial:
  - A whole-file hold (`hold_whole`) always makes a fresh partial in place.
  - A hashed hold reuses the file it hashed.
  - A hold with no word of the hashes (`hold_partial_unfinished_made`) now makes its partial by a creation that fails if the name is taken, in the held directory, and only then opens what has it. So it knows, on Unix.
- **`FsTransferSink::fail_resume_record`** reports a partial this run made as left in place: "…; an incomplete copy was left at the destination, one byte longer than the source, for the next run to resume", with the flag set. A failure held for the record still wins, as cr-ecm34-4 made it.

Elsewhere than Unix the hold holds nothing (cr-ecm23-2), and nothing changes.

## Guard proof
- `sink::tests::a_failed_resume_record_says_the_partial_it_made_is_left` (Unix):
  - a whole-file hold's fresh partial, its record failed by the source, is flagged left in place, in the left-in-place set, at nine bytes;
  - a hashed hold's record so failed is not flagged.
- `a_failed_resume_record_lets_its_partial_go` and cr-ecm34-4's guard still hold.
- Mutations, each restored byte for byte:
  - the made partial forgotten → red;
  - a whole hold not recorded as made → red.
- Checks:
  - native, Windows and macOS (`CARGO_FEATURE_PURE=1`) clippy are clean;
  - the workspace passes on Linux: 2,091 tests over 57 binaries.

## Files changed
- `crates/blit-core/src/remote/transfer/sink.rs`:
  - `HeldPartial.made`;
  - `hold_whole`, `hold_hashed`, `hold_partial_unfinished_made` (the old name kept for its Unix tests);
  - `fail_resume_record`;
  - guard.

## Known gaps
- None known.
