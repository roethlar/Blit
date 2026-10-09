# cr-jl4fix3-1: a record from before the raw list can treat a leftover raw-named file as not left in place

**Severity**: MEDIUM (reviewer: MEDIUM)
**Status**: Fixed — awaiting re-review
**Branch**: — (default-branch mode; fixes land on master)
**Commit**: —
**Reviewer**: codex / gpt-5.6-sol / xhigh / frontier (fallback grade), codex-cli 0.159.3; openreview jl4fix3-r1 over `1a3c4704..6b50d41f` (record `.review/results/jl4fix3-r1.codex.json`); owner goal of 2026-10-07

## Evidence
crates/blit-core/src/job_record.rs:179 — older records deliberately deserialize with an empty left_in_place_raw list; crates/blit-cli/src/jobs.rs:333 — a raw failure consults only that new list, while logs are consulted only when the failure list itself is short.

## Predicted observable failure
After upgrading from the base snapshot, an --ignore-existing run record containing a non-UTF-8 failed file can be retried with ignore-existing still enabled; Blit's own incomplete destination copy is skipped as an existing file and the retry can report success without repairing it.

## Reviewer's suggested approach
For raw failures from records lacking exact left-in-place identities, always consult the run log's composite identities; if the log is unavailable and the legacy text list matches ambiguously, refuse the retry with the existing incomplete-classification error.

## Intake
Admitted. A record written before the raw list filed a raw-named leftover under its text; read now, the raw list is empty and the text list is not consulted for a raw failure, so the file is retried with `--ignore-existing` on and its incomplete copy is skipped. A record must say whether its lists are identity-exact; from one that is not, a raw-named failure whose text is in the old list is ambiguous — the retry is made only when this machine's log names that exact identity, and refused otherwise.

## What
A record says whether its left-in-place lists keep each file by its own identity (`RunRecord.left_in_place_exact`; set by every writer now — the command's record at its end, a detached run settled from its daemon; `false` when absent, as in a record from before). `jobs retry` classifies leftovers through `own_leftovers`: from an exact record, by identity as before; from one that is not, a raw-named failure whose text is in the old text list is ambiguous — its own leftover filed under its text, or a UTF-8 sibling's — and counts as the run's own only when this machine's log names that exact identity (the log is read when the failure list is short); otherwise the retry is refused, naming the file, as when the list is not whole. A raw-named failure whose text is not in the list was not left in place under either scheme and proceeds.

## Guard proof
`an_old_records_raw_named_leftover_is_ambiguous` (CLI): an old record with the text in its list — refused with no log, the run's own when the log names the identity, no ambiguity for a text not in the list; the same record marked exact classifies the raw-named failure as not its own. `a_record_keeps_a_failed_names_exact_bytes` (CLI) and `a_detached_run_trusts_only_a_complete_log` (core) require the writers to mark their records exact. Three mutations each red alone — the ambiguity check dropped, the command's record not marked exact, a settled record not marked exact — restored green.
