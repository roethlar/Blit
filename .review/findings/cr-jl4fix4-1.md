# cr-jl4fix4-1: an ambiguous legacy leftover is never resolved from a complete log

**Severity**: MEDIUM (reviewer: MEDIUM)
**Status**: Fixed — awaiting re-review
**Branch**: — (default-branch mode; fixes land on master)
**Commit**: —
**Reviewer**: codex / gpt-5.6-sol / xhigh / frontier (fallback grade), codex-cli 0.159.3; openreview jl4fix4-r1 over `beab73c9..e522afe8` (record `.review/results/jl4fix4-r1.codex.json`); owner goal of 2026-10-07

## Evidence
crates/blit-cli/src/jobs.rs:393 — `short` only identifies an incomplete failure list; crates/blit-cli/src/jobs.rs:402 — logs are scanned only when that predicate is true, while lines 357-369 refuse every unresolved legacy raw/text match.

## Predicted observable failure
A legacy record with a complete failure list is refused by `jobs retry` even when its complete local log identifies the exact raw leftover; a log proving that a UTF-8 sibling owned the text entry cannot authorize the safe retry either.

## Reviewer's suggested approach
Detect ambiguous legacy identities independently of failure-list completeness, fold a complete terminal log into an exact terminal leftover set, and refuse only if that log is absent, incomplete, or unreadable.

## Intake
Admitted. The log is read only when the record's failure list is cut short, so a legacy record with a whole list is refused even when this machine's complete log names the exact leftover — or proves the text entry was a UTF-8 sibling's. The log must be read whenever a legacy identity is ambiguous, and a complete log settles the question both ways; only an absent or incomplete log leaves it unknowable.

## What
`failed_identities` reads this machine's log of the run whenever a record from before identities names a raw-named failure whose text is in its text list, whether or not the failure list is whole, and reports what it found (`LoggedLeftovers`): read at all, complete — every log of the run finished, ending with its `run-end`, no `log-incomplete`, no unreadable line — and the leftover identities. `own_leftovers` then settles an ambiguous failure from the log: named there with the marker, it is the run's own; not named in a complete log, it is not (no refusal); an absent or incomplete log leaves it unknowable, and the retry is refused naming the file.

## Guard proof
`an_old_records_ambiguity_is_settled_from_a_whole_log` (CLI): a real finished log naming the identity with the marker, read for an old record with a whole failure list — read, complete, the file the run's own; then a run whose log recorded a `log-incomplete` event — read, not whole, refused. `an_old_records_raw_named_leftover_is_ambiguous` (CLI): a complete log without the identity settles it as not the run's own; an incomplete one refuses. Three mutations each red alone — the log read gated on a short list again, completeness ignoring lost events, the settlement ignoring completeness — restored green.
