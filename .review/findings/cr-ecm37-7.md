# cr-ecm37-7: the summary join still searched the leftover lists for each failed file

**Severity**: MEDIUM — a run with many failures stalls at its summary. cr-ecm36-4 made the join of failures to failed paths a hash lookup. It left the lookup into the leftover lists, `left_in_place` and `left_in_place_raw`, as a search of each list for each failed file. With every failed file leaving a partial, the join took 91 ms for 6,000 files, 356 ms for 12,000 and 1.41 s for 24,000 in a debug build.
**Status**: Fixed; awaiting review
**Branch**: — (default-branch mode; fixes land on master)
**Commit**: this commit (`Fix cr-ecm37-7`)
**Reviewer**: codex / gpt-6.2-astra (as dispatched) / max / frontier, codex-cli headless one-shot, openreview `combined-r38` over `2c55cb3a..6ee7ed5e` (verdict `acceptable_with_changes`, capability_ok and guard_confirmed true); `.review/results/combined-r38.codex.md`. It completes cr-ecm36-4 (`c8fe7147`) in part.

## Evidence
"With every failed file leaving a partial, the summary join took 91 ms for 6,000 files, 356 ms for 12,000 and 1.41 s for 24,000 in the debug build. It still scans the leftover vector for each identity. Build text and raw-byte lookup sets once."

## Intake
Admitted as the reviewer put it.

## What
`RunTotals::failed_files` (`run_log.rs`) builds a set over each leftover list once:
- one over the texts;
- one over the byte names.

It looks each failed file up in the matching set.

## Guard proof
- `run_log::tests::the_join_is_linear_in_the_leftovers` covers 100,000 failed files that all left a copy:
  - half are named by text and half by bytes;
  - each is joined as left in place;
  - the join must take under 3 seconds. It takes about 0.1 s in a debug build, the test's setup included.
- Mutations, each restored byte for byte:
  - the byte-name leftovers searched → red;
  - the text leftovers searched → red.

  Each search takes several seconds for its half.
- Checks: native clippy is clean.

## Files changed
- `crates/blit-core/src/run_log.rs` — the leftover sets; guard.

## Known gaps
- The guard is a bound on time, set far from both sides: the hash join takes about a thirtieth of it and either search about twice it. A machine thirty times slower than this one would fail it.
