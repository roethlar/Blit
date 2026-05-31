# audit-1c1-stall-guard: idle-timeout AsyncRead adapter (StallGuard)

**Severity**: Robustness
**Status**: In progress / pending review
**Branch**: `phase5/a1`
**Commit**: `0cfa534`
**Parent finding**: `audit-1c-transfer-stall-timeout` (part 1 of 2).

## Context

Owner approved the audit-1c concept (no-bytes-for-30s idle timeout, NOT a
total deadline) and **scoped it to ALL pulls** (memory
`audit-owner-decisions`). The design's opt-in `Option<Duration>`
threading was specifically to make it delegated-only; with the all-pulls
decision the timeout can instead be applied unconditionally at the
receive-pipeline boundary, so part 2's wiring is simpler (no per-caller
threading).

This part 1 lands the self-contained adapter; part 2 (`audit-1c2`) wires
it into the receive pipeline.

## What

`StallGuard<R>` (`crates/blit-core/src/remote/transfer/stall_guard.rs`):
an `AsyncRead` wrapper that re-arms a per-read deadline on every read
that makes progress (data or clean EOF), and while a read is `Pending`
trips `io::ErrorKind::TimedOut` once the idle window elapses. At the
`AsyncRead` layer it catches a stall at *any* wire-frame read without
touching the parsing logic, and being re-armed per read it is an **idle**
timeout — a steadily-progressing large transfer is never aborted.
`PULL_STALL_TIMEOUT = 30s`.

## Tests (blit-core, +3)

- `times_out_when_reader_stalls` — a duplex whose writer half is held
  open but never written → the read is perpetually `Pending` → `TimedOut`
  after the (20ms test) window.
- `passes_data_through_unchanged` — bytes flow through intact.
- `does_not_trip_on_steady_trickle_past_total_window` (load-bearing) — 3
  writes 20ms apart (~60ms total) under a 50ms window: no single gap
  exceeds the window, so it must NOT trip — proving idle-not-total
  semantics.

## Files changed

- `crates/blit-core/src/remote/transfer/stall_guard.rs` (new) + `mod.rs`
  module declaration.

## Part 2 (audit-1c2, next)

`execute_receive_pipeline` and its six read helpers (`read_u32`/`u64`/
`i64`/`string`/`file_header`/`tar_shard`) all take a concrete
`&mut TcpStream`; the receive path is read-only on the socket (writes go
to the sink). Part 2 generic-izes/dyn-ifies that chain over
`AsyncRead + Unpin` and wraps the socket in `StallGuard` at the receive
boundary (`pull.rs` receive site) so every pull gets the idle timeout.

## Reviewer comments

(empty — pending review)
