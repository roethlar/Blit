# cr-jl4-1: a retry of an `--ignore-existing` job skips its own incomplete files

**Severity**: HIGH (reviewer: HIGH)
**Status**: Fixed — awaiting re-review
**Branch**: — (default-branch mode; fixes land on master)
**Commit**: `7e935b64`
**Reviewer**: codex / gpt-5.6-sol / xhigh / frontier (fallback grade), codex-cli 0.159.3; openreview jl4-r1 over `3913ab70..2dab5d21` (record `.review/results/jl4-r1.codex.json`); owner goal of 2026-10-07

## Evidence
crates/blit-cli/src/jobs.rs:280 — job_to_retry restores the original options and installs only retry_only, while crates/blit-cli/src/transfers/retry.rs:451 documents and implements the required left_in_place split for --ignore-existing.

## Predicted observable failure
If a failed write leaves Blit's own incomplete destination and the original job used --ignore-existing, jobs retry treats that partial file as pre-existing, skips it, exits successfully, and records an apparently clean child run while the damaged file remains. The required red/green guard reproduced this behavior.

## Reviewer's suggested approach
Persist the exact left-in-place set and truncation flag in RunRecord; retry those paths with ignore_existing disabled, retry the remaining failed paths with the original flag, and refuse unclassifiable paths.

## Intake
Admitted. The in-command retry passes retry a path whose failed write left this run's own incomplete copy with `--ignore-existing` off; `jobs retry` restores the original options and sends only the retry set, so that copy is skipped as existing and the run reports success. The record must keep which failures left a copy in place (and whether that list is whole), and the retry must split as the passes do — refusing what it cannot classify.

## What
A run's record keeps which failed files left its own incomplete copy, and whether that list is whole (`RunRecord.left_in_place`, `left_in_place_truncated`, from every summary's own set; a detached run settled from a complete daemon log reads it from the log's whole reasons, otherwise marks it not whole). `jobs retry` of an `--ignore-existing` job splits as the in-command passes do: those paths are retried with the flag off — the copy is the run's own, not one the person asked to keep — and the rest with it on, as two parts of one run (`main::retry_parts`; `CommandRun::add_up_parts` adds their accounts); when the list is not whole it refuses. A list completed from the log takes the log's whole reasons too.

## Guard proof
`an_ignore_existing_retry_replaces_only_its_own_incomplete_copies` (blit-cli `job_retry`): one failure left its own partial copy, another's path now holds the person's file; the retry replaces the first and keeps the second, its two parts adding up to one file copied; with the list marked not whole, refused. Unit: the record keeps the totals' list (`a_record_keeps_a_failed_names_exact_bytes`), a log's whole reason marks a failure as left in place (`reading_a_log_notes_what_it_lost`). Seven mutations each red alone — no split, the own part keeping the flag, no refusal, parts not added up, the record's list ignored, the list not persisted, the reason not read — restored green.
