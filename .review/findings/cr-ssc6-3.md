# cr-ssc6-3: Detached transfers silently ignore the retries option

**Severity**: MEDIUM — `--detach` accepts `--retries N` and silently applies none; recoverable files stay missing from the daemon-owned job
**Status**: Verified
**Branch**: — (default-branch mode; fixes land on master)
**Commit**: `93b93d26 + a8628e16`
**Reviewer**: codex / gpt-5.6-sol / xhigh / standard (D-2026-07-31-3 standing codereview; range b342d636..e14d1b72 (ssc-6), record .review/results/ssc-6-range.codex.json)

## Evidence
crates/blit-cli/src/transfers/mod.rs:474 — all retry-pass orchestration is skipped when args.detach is true, although --retries is accepted and defaults to one.

## Predicted observable failure
`blit copy --detach --retries N` behaves identically for every N: per-file failures in the daemon-owned remote-to-remote job receive no end-of-run retry despite the advertised reliability setting, leaving recoverable files missing.

## Reviewer's suggested approach
Carry the retry count and wait policy in the delegated request and execute retry passes in the daemon-owned job, or explicitly reject the option for detached transfers instead of accepting and ignoring it.

## What
A detached run with `--retries` above zero prints one stderr notice naming the ignored value and the `--retries 0` opt-out (`retry::detach_retry_notice`), wired into both delegated routes before the run starts. Carrying the passes into the daemon-owned job needs the pass loop and its switches on the wire — recorded as a known gap and a TODO item.

## Guard proof
Unit pin: notice text for detach+retries, none for `--retries 0` or attached runs. Mutation: notice suppressed → red.

## Known gaps
Daemon-owned (detached) jobs get no retry passes; tracked in TODO.md.
