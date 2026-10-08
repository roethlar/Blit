# cr-jl2-1: a run that writes nothing is logged as copying

**Severity**: MEDIUM (reviewer: MEDIUM)
**Status**: Open
**Branch**: — (default-branch mode; fixes land on master)
**Commit**: —
**Reviewer**: codex / gpt-5.6-sol / xhigh / frontier (fallback grade), codex-cli 0.159.3; openreview jl2-r1 over `744c3708..2bc595e0` (record `.review/results/jl2-r1.codex.json`); owner goal of 2026-10-07

## Evidence
crates/blit-cli/src/transfers/local.rs:166 — every local transfer attaches the audit lane; crates/blit-core/src/run_log.rs:280 maps every FileComplete to FileCopied; crates/blit-cli/src/run_log.rs:253 reports copied_files unchanged, while crates/blit-core/src/remote/transfer/sink.rs:3304 also counts null-sink payloads as written.

## Predicted observable failure
A successful `blit copy --dry-run` leaves the destination untouched but `blit jobs log` reports `copied planned.txt` and `summary 1 copied`; `--null` similarly claims copies while discarding writes and is not identified in the recorded options.

## Reviewer's suggested approach
Thread a real/dry-run/null-sink disposition into RunLog, emit compatible planned or discarded events—or suppress copied events and counts—for non-real runs, record the disposition in RunInfo, and test that untouched destinations never produce copied claims.

## Intake
Admitted. The reviewer's guard reproduced it at head (a dry-run copy's log named the file copied and counted it). Both modes are local-only (remote routes refuse them), so the fix is in the run log and the CLI that starts it.
