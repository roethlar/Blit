# cr-jl4fix1-1: settling a detached run keeps failures a later pass fixed

**Severity**: MEDIUM (reviewer: MEDIUM)
**Status**: Fixed — awaiting re-review
**Branch**: — (default-branch mode; fixes land on master)
**Commit**: `6ecc86c2`
**Reviewer**: codex / gpt-5.6-sol / xhigh / frontier (fallback grade), codex-cli 0.159.3; openreview jl4fix1-r1 over `b2204f9a..041f993e` (record `.review/results/jl4fix1-r1.codex.json`); owner goal of 2026-10-07

## Evidence
crates/blit-core/src/job_record.rs:821 — `LogRead::read_line` adds every `FileFailed` but has no `FileCopied`/`FileSent` removal; `ended_record` then copies that historical union into the settled record.

## Predicted observable failure
A detached job whose retry pass later lands a file still records that file as failed; `jobs retry` can resend or overwrite it, and can retry a run whose terminal failed count is zero.

## Reviewer's suggested approach
Fold daemon log events in attempt order by `(path, raw)`, remove an identity when it later lands, derive left-in-place only from identities still failed, and refuse reconciliation when the terminal set disagrees with the summary count.

## Intake
Admitted. cr-jl4-3 made this machine's log read its terminal state, but `LogRead` — a daemon's log, settling a detached run — still adds every `file-failed` and never removes one a later event landed. It must fold the same way, by identity, and mark the list incomplete when its terminal count disagrees with the summary's.

## What
`LogRead::read_line` folds a daemon's log to the run's terminal state, as cr-jl4-3 made this machine's log: a `file-copied` or `file-sent` removes the failure of the same (text, bytes) identity, and a text no failure is left for is no longer left in place. A complete log whose terminal list names fewer failures than its summary counts settles with the list marked incomplete.

## Guard proof
`reading_a_log_notes_what_it_lost` (a later copy of the same identity lands a failure and clears its left-in-place mark; a copy of another identity of that text does not) and `a_detached_run_trusts_only_a_complete_log` (a complete log naming fewer failures than its summary: marked incomplete), blit-core. Two mutations each red alone — no removal on a later copy, no incomplete mark — restored green.
