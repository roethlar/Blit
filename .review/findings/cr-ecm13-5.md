# cr-ecm13-5: With `--size-only`, a file of the right length where a copy was left was reported copied

**Severity**: HIGH — a retry reports one file copied, no bytes written, exit 0, and the destination holds something else
**Status**: Fixed, awaiting review
**Branch**: — (default-branch mode; fixes land on master)
**Commit**: this commit (`Fix cr-ecm13-5`)
**Reviewer**: codex / gpt-6.2-astra (as dispatched) / max / frontier, codex-cli 0.171.0 headless one-shot, openreview `exclude-case-r13` over `c99ad483..bd84649f` (verdict `acceptable_with_changes`, capability_ok and guard_confirmed true); `.review/results/exclude-case-r13.codex.md`. It completes cr-ecm12-2. Fixed by cr-ecm13-1; this commit is its guard.

## Evidence
"In a saved-job fixture with a same-length stamped partial, `--size-only` reported one file copied, zero bytes, and exit 0. The source contained `good` and the destination still contained `junk`. The sink repeated the size comparison after the session had already ordered repair."

Cause. cr-ecm12-2 had the session order a marked copy replaced whatever a comparison would say of it; the local sink compares again before it writes, by the run's own rule, and by size the two were the same file.

## Intake
Admitted. cr-ecm13-1 removes the cause rather than carrying the decision through the sink: no file that is there is ordered replaced under `--ignore-existing`, so there is no decision to carry.

## What
Nothing in this commit but the guard.

## Guard proof
- `job_retry.rs::a_size_only_retry_does_not_call_a_file_of_the_right_length_copied`: a `copy --size-only --ignore-existing` job whose record says it left a copy of `data.bin`; `junk` standing there, four bytes like the source's `good`. The retry exits 2, says the file was kept, leaves `junk`, and its record holds no file copied and one failed; with the file removed, a retry copies `good`.
- Mutation, restored byte for byte: the session ordering a file where a copy was left written over, the sink comparing again → the guard red — exit 0, as the reviewer saw.

## Files changed
- `crates/blit-cli/tests/job_retry.rs`.

## Known gaps
- The sink still compares again after the session, for every file; that the two agree rests on their asking the same question of the same file.
- Run on Linux only.
