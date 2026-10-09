# cr-jl4fix5-1: a source log can erase a destination log's leftover evidence

**Severity**: HIGH (reviewer: HIGH)
**Status**: Fixed — awaiting re-review
**Branch**: — (default-branch mode; fixes land on master)
**Commit**: —
**Reviewer**: codex / gpt-5.6-sol / xhigh / frontier (fallback grade), codex-cli 0.159.3; openreview jl4fix5-r1 over `5f084f87..411b2f4b` (record `.review/results/jl4fix5-r1.codex.json`); owner goal of 2026-10-07

## Evidence
crates/blit-cli/src/jobs.rs:428 — logs from all roles are folded into one `logged.left` set, and lines 453-457 remove an identity for either `FileCopied` or `FileSent`, although crates/blit-core/src/job_log.rs:253 explicitly says `FileSent` does not establish that the file landed.

## Predicted observable failure
For a legacy ambiguous raw-named failure, a complete destination log can record that its failed write left an incomplete copy while a later-folded source log records `FileSent` and a less-specific failure. The shared set becomes empty, the complete-log branch classifies the file as not left in place, and an `--ignore-existing` retry can skip the incomplete destination copy instead of replacing it.

## Reviewer's suggested approach
Maintain terminal leftover state per participant/role stream, folding that stream's attempts in order, then union positive leftover identities across streams. Events from one role must never remove another role's evidence; add the two-role case as a regression test.

## Intake
Admitted. `failed_identities` folds every log of the run in the per-user folder into one leftover set and lets a `FileSent` remove an identity, though a sent file is not a landed one. When a daemon shares that folder (no `STATE_DIRECTORY`), its destination log and the command's own log of one run sit together, and the source-side `FileSent` erases the destination's marker. Each log must be folded on its own, positive leftover evidence united across logs, and only a `FileCopied` may clear a leftover.

## What
`failed_identities` folds each log of the run on its own — a leftover set and a still-failed set per log, attempts of a stream in the order the folder lists them — and unites what every log still holds at its end; one log's events clear nothing of another's. Within a log, only a `FileCopied` clears a leftover: a `FileSent` means the file was sent, not that it landed, so it clears only the still-failed mark. The daemon-side fold (`LogRead`, destination logs only) follows the same rule for the principle.

## Guard proof
`an_old_records_ambiguity_is_settled_from_a_whole_log` (CLI) grew three cases on real logs: the reviewer's — a destination log's marker beside the command's own log that sent the file — keeps the leftover; a send after the marker within one log keeps it; two logs that each landed the other's leftover keep both, whichever is folded first. `reading_a_log_notes_what_it_lost` (core): a send of the identity clears no leftover. Three mutations each red alone — a send clearing the leftover, one shared set across logs, the daemon-side send clearing — restored green.
