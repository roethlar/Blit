# cr-jl3afix2-1: a failure seen live hides another of the same text when the log closes

**Severity**: MEDIUM (reviewer: MEDIUM)
**Status**: Fixed — awaiting re-review
**Branch**: — (default-branch mode; fixes land on master)
**Commit**: —
**Reviewer**: codex / gpt-5.6-sol / xhigh / frontier (fallback grade), codex-cli 0.159.3; openreview jl3afix2-r1 over `739c722d..4b65db0c` (record `.review/results/jl3afix2-r1.codex.json`); owner goal of 2026-10-07

## Evidence
crates/blit-core/src/run_log.rs:495 — closing skips the entire summary bucket when active.failed contains the lossy path, before examining the multiple raw-aware FileFailure entries at lines 501–521.

## Predicted observable failure
If the first of two raw Unix filenames that render identically fails during transfer and the second is rejected as a duplicate, the job log contains only the first failure; the duplicate failure and its distinct raw bytes are omitted. A temporary regression test failed with one event instead of two, then passed when reconciliation used composite failure identity; the worktree was restored cleanly to the reviewed head.

## Reviewer's suggested approach
Store observed live failures using their path, resolved raw identity, and reason, then compare each summary FileFailure against that set individually instead of skipping all failures sharing a display path.

## Intake
Admitted. The closing pass skips every summary failure whose text was already logged live, so a second entry of that text — a different file — never reaches the log. What was logged must be tracked by text and bytes, and each summary failure compared on its own.

## What
The run log tracks the failures it has named by text and escaped bytes, not by text alone. Live failures are recorded with the bytes the log knows for their text; at close each summary failure is compared on its own — named unless that exact (text, bytes) was named already — so a second entry of a text, with its own bytes, is logged beside the first, and no failure is logged twice.

## Guard proof
`a_live_failure_hides_no_other_failure_of_its_text` (blit-core run_log): the first of two entries of one text fails live, the summary names both; the log holds exactly the live failure and the duplicate, each with its own bytes. Two mutations each red alone — skipping by text alone (the defect), dropping the once-only check — restored green.
