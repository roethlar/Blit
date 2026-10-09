# cr-jl4fix3-1: a record from before the raw list can treat a leftover raw-named file as not left in place

**Severity**: MEDIUM (reviewer: MEDIUM)
**Status**: Open
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
