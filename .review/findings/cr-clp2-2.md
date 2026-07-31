# cr-clp2-2: Verbose rendering performs blocking terminal I/O in an async task

**Severity**: MEDIUM — a backpressured PTY can block a Tokio worker from
the drain task; on a current-thread runtime that stalls transfer futures
and timers. Also a direct violation of the repo's no-blocking-in-async
style law.
**Status**: Open
**Branch**: — (default-branch mode)
**Commit**: (filled at landing)

Reviewer provenance (generation pass): codex / gpt-5.6-sol / xhigh /
standard; codex-cli 0.146.0; range `246acb54..9532ee20`
(`.review/results/clp-2-range.codex.json`).

## Evidence
`blit-cli/src/transfers/local.rs:386-393` — `RowOutput for ProgressBar`
calls synchronous `ProgressBar::println`/`set_message` (terminal I/O
under indicatif's internal lock); `:402-424` invokes it directly from
the async `drain_progress_lane` for every FileComplete under `-v`.

## Predicted observable failure
Many files + slow/backpressured PTY: terminal writes block the async
worker; on a single-worker runtime the transfer stalls until the
terminal drains.

## What
Terminal I/O rides the async drain task instead of a dedicated writer.

## Approach
(planned) A dedicated writer thread owned by `LiveProgressRow`: the
drain task's `RowOutput` handle becomes a non-blocking unbounded-channel
sender; the thread applies messages/lines to the real `ProgressBar`.
The log-redirect sink shares the same sender, so backend lines keep
their ordering relative to `-v` lines. Lifetime: senders drop at
`finish()`, the thread ends when the channel drains, bounded join.

## Files changed
(filled with the fix commit)

## Guard proof
(planned) Plumbing test: messages and lines sent through the
thread-backed handle arrive at a collector in order. The no-blocking
property itself is structural (the async task only does channel sends);
stated as the manual-check rationale per the playbook's untestable-
property clause.

## Coder dispute (if any)
None.

## Known gaps
The hang scenario itself is not reproducible deterministically in a
test; the guard pins the plumbing, the structure removes the blocking
call from async context.

## Reviewer comments
(pending per-finding verification)
