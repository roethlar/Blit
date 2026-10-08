# cr-jl2-1: a run that writes nothing is logged as copying

**Severity**: MEDIUM (reviewer: MEDIUM)
**Status**: Fixed — awaiting re-review
**Branch**: — (default-branch mode; fixes land on master)
**Commit**: `58ac16fb`
**Reviewer**: codex / gpt-5.6-sol / xhigh / frontier (fallback grade), codex-cli 0.159.3; openreview jl2-r1 over `744c3708..2bc595e0` (record `.review/results/jl2-r1.codex.json`); owner goal of 2026-10-07

## Evidence
crates/blit-cli/src/transfers/local.rs:166 — every local transfer attaches the audit lane; crates/blit-core/src/run_log.rs:280 maps every FileComplete to FileCopied; crates/blit-cli/src/run_log.rs:253 reports copied_files unchanged, while crates/blit-core/src/remote/transfer/sink.rs:3304 also counts null-sink payloads as written.

## Predicted observable failure
A successful `blit copy --dry-run` leaves the destination untouched but `blit jobs log` reports `copied planned.txt` and `summary 1 copied`; `--null` similarly claims copies while discarding writes and is not identified in the recorded options.

## Reviewer's suggested approach
Thread a real/dry-run/null-sink disposition into RunLog, emit compatible planned or discarded events—or suppress copied events and counts—for non-real runs, record the disposition in RunInfo, and test that untouched destinations never produce copied claims.

## Intake
Admitted. The reviewer's guard reproduced it at head (a dry-run copy's log named the file copied and counted it). Both modes are local-only (remote routes refuse them), so the fix is in the run log and the CLI that starts it.

## What
A run log has a disposition (`run_log::Disposition`: written, dry run, discarded), set by the CLI from `--dry-run`/`--null`. A run that writes nothing names no file copied (its completions are not logged), its summary counts nothing copied or deleted, a closing line says what it would have done (dry run: the files and bytes it would have copied and how many it would have deleted; `--null`: what it read and discarded), and its `run-end` says "dry run: nothing was written" (or the `--null` wording) when nothing else does. `null` is now among the options `run-start` records (`dry-run` already was).

## Guard proof
`a_run_that_writes_nothing_logs_no_copies` (blit-cli `job_log_cli`): a dry-run copy, a `--null` copy and a dry-run mirror over a destination-only file — no `copied`/`deleted` lines, zero counts, the would-have line, the run-end wording, the options. Four mutations each red alone, restored green: the suppression arm off, the summary's copied count unzeroed, the CLI not setting dry run, `null` dropped from the options.
