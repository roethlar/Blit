# cr-jl4fix1-1: settling a detached run keeps failures a later pass fixed

**Severity**: MEDIUM (reviewer: MEDIUM)
**Status**: Open
**Branch**: — (default-branch mode; fixes land on master)
**Commit**: —
**Reviewer**: codex / gpt-5.6-sol / xhigh / frontier (fallback grade), codex-cli 0.159.3; openreview jl4fix1-r1 over `b2204f9a..041f993e` (record `.review/results/jl4fix1-r1.codex.json`); owner goal of 2026-10-07

## Evidence
crates/blit-core/src/job_record.rs:821 — `LogRead::read_line` adds every `FileFailed` but has no `FileCopied`/`FileSent` removal; `ended_record` then copies that historical union into the settled record.

## Predicted observable failure
A detached job whose retry pass later lands a file still records that file as failed; `jobs retry` can resend or overwrite it, and can retry a run whose terminal failed count is zero.

## Reviewer's suggested approach
Fold daemon log events in attempt order by `(path, raw)`, remove an identity when it later lands, derive left-in-place only from identities still failed, and refuse reconciliation when the terminal set disagrees with the summary count.

## Intake
Admitted. cr-jl4-3 made this machine's log read its terminal state, but `LogRead` — a daemon's log, settling a detached run — still adds every `file-failed` and never removes one a later event landed. It must fold the same way, by identity, and mark the list incomplete when its terminal count disagrees with the summary's.
