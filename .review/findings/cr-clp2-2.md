# cr-clp2-2: Verbose rendering performs blocking terminal I/O in an async task

**Severity**: MEDIUM — a backpressured PTY can block a Tokio worker from
the drain task; on a current-thread runtime that stalls transfer futures
and timers. Also a direct violation of the repo's no-blocking-in-async
style law.
**Status**: In progress — fixed, awaiting per-finding reviewer verdict
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

As built: `ThreadedRowOutput` (unbounded std mpsc sender) is the only handle the async side holds; a dedicated writer thread runs `row_writer_loop` applying messages/lines to the real ProgressBar; the log-redirect sink and the drain task share the one channel so ordering is production order; the initial row message also rides the channel; `finish` drops the redirect (closing the last sender), joins the writer off-thread bounded by the grace, then clears the bar. Guard `the_writer_loop_applies_queued_writes_in_order` FAILED red with the Line arm dropped, green restored. The no-blocking property is structural: the async side contains only channel sends (stated per the playbook's untestable-property clause).

Round 1: Reviewer codex / gpt-5.6-sol / xhigh / standard; codex-cli 0.146.0. Reviewed `6f26b1819ebb5ee4546a28dfd8a5892d057b56ce` base `5fbe9abbf7883b5fa570eb08f99b033d2965d8cd`. guard_confirmed true; verdict **reopened** (2026-07-31T05:10Z): (1) local.rs:613 — the timeout only drops the handle of an unabortable spawn_blocking task blocked in writer.join(); the runtime waits for it at shutdown, so a wedged writer still hangs CLI exit. (2) local.rs:618 — finish_and_clear() runs on the async side after the timeout and takes the same ProgressBar lock a wedged writer can hold. Record: `.review/results/cr-clp2-2.codex.r1.json`. Repair: the writer thread itself performs finish_and_clear as its final act and signals a oneshot; finish() awaits the signal bounded and never joins or touches the bar — no blocking-pool entry, no shared-lock touch.

Repair (r2 candidate): the writer thread now runs `run_row_writer` — drain, `finish_and_clear` on ITS side, then a oneshot signal. `finish()` awaits the SIGNAL bounded; no `spawn_blocking` join (nothing enters the pool the runtime waits for at shutdown) and no bar access from the async side after finish begins (the shared-lock hazard is gone). The writer is a plain detached std thread process exit reaps. Guard `the_writer_clears_and_only_then_signals_done` (blocking clear + premature-signal revert) FAILED red with the send hoisted above the drain/clear, green restored.
