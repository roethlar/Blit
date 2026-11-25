# cr-ecm41-5: the disposition evidence held every copied or removed name

**Severity**: MEDIUM — a retry's reading of a log grows with the run, not its failures. Brought in by cr-ecm40-2.
**Status**: Fixed; awaiting review
**Branch**: — (default-branch mode; fixes land on master)
**Commit**: this commit (`Fix cr-ecm41-5`)
**Reviewer**: codex / gpt-6.2-astra (as dispatched) / max / frontier, codex-cli headless one-shot, openreview `combined-r42` over `b6e6fab9..e2c2e08d` (verdict `acceptable_with_changes`, capability_ok and guard_confirmed true); `.review/results/combined-r42.codex.md`.

## Evidence
"The new `settled` set retains every copied or deleted name, even when the retry needs no disposition evidence. With one failure and 200,000 successful copy events, I measured 56 MiB peak RSS for distinct successful names versus 27 MiB for one repeated name. Retain disposition evidence only for candidate failures."

## Intake
Admitted as written.

## What
A copy or a removal is kept as evidence of a file's disposition only where a failure is pending on that file: it stands in the stream's leftovers, unknowns or failures. A failure that says what it left keeps its evidence as before. The set is bounded by the failures.

## Guard proof
- `jobs::retry_name_tests::disposition_evidence_is_kept_for_the_failed_files_only`: 500 copies, a removal and two failures. The evidence holds the two failed files and nothing else.
- Mutation, restored byte for byte: every copied name kept → red.
- Checks:
  - native, Windows and macOS (`CARGO_FEATURE_PURE=1`) clippy are clean;
  - the workspace passes on Linux, with the round's six fixes: 2,152 tests over 57 binaries.

## Files changed
- `crates/blit-cli/src/jobs.rs` — the log's reading; guard.

## Known gaps
- Memory itself is not measured here; the guard bounds the set by what it holds.
