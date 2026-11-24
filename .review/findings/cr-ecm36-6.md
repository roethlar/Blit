# cr-ecm36-6: an unknown disposition was cleared by a later event that never touched the destination

**Severity**: HIGH — a recovered `--ignore-existing` retry can skip an earlier partial. cr-ecm35-5 kept "unknown" per file and cleared it when a later event said anything about that file. Replaying an unknown and then a source-open failure carrying `Some(false)` gave an empty left-in-place list marked whole. The later attempt never reached the destination, so it said nothing about what the earlier one had left there.
**Status**: Verified — `combined-r38` (Codex openreview, 2026-11-24) reviewed `2c55cb3a..6ee7ed5e` — the `combined-r37` fixes — with `guard_confirmed: true`: "Passing the root into the stream and tar writers, marking failed mmap copies through the open file handle, and preserving uncertainty after a source-only failure are sound changes."
**Branch**: — (default-branch mode; fixes land on master)
**Commit**: `2e5dd3ad`
**Reviewer**: codex / gpt-6.2-astra (as dispatched) / max / frontier, codex-cli headless one-shot, openreview `combined-r37` over `d2494965..2c55cb3a` (verdict `acceptable_with_changes`, capability_ok and guard_confirmed true); `.review/results/combined-r37.codex.md`. It completes cr-ecm35-5 (`51608679`).

## Evidence
"Replaying an unknown disposition followed by a source-open failure carrying `Some(false)` produced an empty leftover list marked complete. That later attempt never touched the destination. A recovered `--ignore-existing` retry can consequently skip an earlier partial. Clear uncertainty on evidence of a copy, removal or confirmed leftover."

## Intake
Admitted as the reviewer put it.

## What
In the job record's log reader (`LogRead`) and in `blit jobs retry`'s (`failed_identities`), an unknown file is settled only by what the destination shows:
- **a copy** of it (`FileCopied`), as before;
- **a removal** (`FileDeleted`), new — which also clears a leftover;
- **a leftover confirmed** (`left_in_place: Some(true)`).

A later failure saying it left nothing (`Some(false)`) no longer settles it.

## Guard proof
- `job_record::tests::reading_a_log_notes_what_it_lost`:
  - an unknown, then a failure of the same file saying `Some(false)`, stays unknown, and the rebuilt list is not whole;
  - a removal of it settles it.
- `jobs::retry_name_tests::an_unknown_is_not_settled_by_a_failure_saying_nothing_left`: a whole log with that sequence still refuses an older record's ambiguous leftover.
- Mutations, each restored byte for byte:
  - the record settles by a word → red;
  - the retry settles by a word → red;
  - a removal settles nothing → red.
- Checks: native clippy is clean.

## Files changed
- `crates/blit-core/src/job_record.rs` — `LogRead::read_line`; guard.
- `crates/blit-cli/src/jobs.rs` — the stream's reading; guard.

## Known gaps
- None known.
