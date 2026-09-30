# cr-ssc4-3: Hydration skips retract unrelated successful files from progress totals

**Severity**: LOW — progress and the final landed count under-report by the number of pre-announcement skips on mixed remote runs
**Status**: Verified
**Branch**: — (default-branch mode; fixes land on master)
**Commit**: `d9076b92`
**Reviewer**: codex / gpt-5.6-sol / xhigh / standard (D-2026-07-31-3 standing codereview; range b70ef03a..261912bb, record .review/results/ssc-4-range.codex.json)

## Evidence
crates/blit-core/src/transfer_session/mod.rs:3104 — a hydration failure sends `FileSkipped` without reporting a file completion, but lines 2273-2277 later retract every destination-reported failure from the source's completion count.

## Predicted observable failure
For a remote transfer with one successful file and one metadata-hydration skip, progress first counts the successful file, then subtracts the skipped file and finishes at zero landed files even though one file landed. Larger mixed runs under-report by the number of pre-announcement skips.

## Reviewer's suggested approach
Reconcile the file total to the destination's authoritative `files_transferred` value rather than subtracting `files_failed`, or separately track which failures had previously emitted optimistic completions.

## What
`SummaryReconciled` carries `files_landed` (the summary's `files_transferred`); the shared fold and the daemon's live row adopt it rather than subtracting `files_failed` from optimistic completions. The daemon's unused `atomic_saturating_sub` is removed.

## Guard proof
Progress unit pin: one completion plus a reconciliation of {failed 1, landed 1} ends at 1 landed. Mutation: subtraction restored → red.

## Known gaps
Unit-level guard; the remote-carrier integration pin was not added (the fold is the single shared accumulator).
