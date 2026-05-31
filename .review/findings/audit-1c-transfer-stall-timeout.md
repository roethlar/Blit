# audit-1c-transfer-stall-timeout: no-bytes-for-30s idle timeout on the delegated pull

**Severity**: Robustness
**Status**: DESIGN — awaiting approach approval before implementation
**Branch**: `phase5/a1`
**Commit**: (none — design only)
**Parent finding**: `audit-1-daemon-timeouts` (item 3). Owner approved the
*concept* (no bytes for 30s, NOT a total deadline — memory
`audit-owner-decisions`); this captures the *how* before touching the
shared receive path.

## Why this is design-first

The audit suggested a timeout on `pull_sync_with_spec`. A naive
`tokio::time::timeout` around the whole call is wrong (it would abort
legitimate large transfers). The owner-approved fix is an **idle/stall
timeout**: abort only when no bytes arrive for 30s.

The catch: the byte receive isn't a single await. The delegated pull's
data plane is `receive_pull_data_plane` (pull.rs:1654) →
`execute_receive_pipeline` (pipeline.rs:201), which reads each wire
frame through **many** separate `socket` awaits — `read_exact(tag)`,
`read_file_header`, `read_u64/i64/u32`, `read_tar_shard`, `read_string`,
file-data streaming, etc. A stall can happen at *any* of them (e.g.
mid-file). So a correct detector can't wrap one call site.

This is a change to the **shared core receive pipeline** used by every
pull (CLI local pulls included), so it is the highest-blast-radius item
in the audit backlog and warrants a design sign-off before code.

## Recommended approach (opt-in, delegated-only, zero CLI regression)

1. A small `AsyncRead` adapter — `StallGuard<R>` — that wraps the socket
   and applies a per-read deadline: each `poll_read` arms a 30s timer;
   any read that doesn't make progress within the window resolves to an
   `io::Error(TimedOut)`. Because it sits at the `AsyncRead` layer, it
   catches a stall at *every* frame read without touching the parsing
   logic.
2. Thread an `Option<Duration> stall_timeout` from
   `pull_sync_with_spec` down to `receive_pull_data_plane`. When `Some`,
   wrap the `TcpStream` in `StallGuard` before
   `execute_receive_pipeline`; when `None`, pass the bare stream
   (**identical current behavior**).
3. The **delegated** handler (`delegated_pull.rs`) passes `Some(30s)`;
   the CLI / local pull paths pass `None`. This scopes the stall-timeout
   to the daemon-to-daemon case the audit flagged (a stalled delegated
   pull pins daemon resources), while a CLI user's pull is unchanged
   (they can Ctrl-C).

This mirrors the codebase's existing opt-in pattern (`byte_progress:
Option<&ByteProgressSink>` is threaded the same way, None for CLI).

## Open approach question (for owner/reviewer)

- **Scope:** delegated-only (recommended, above) vs. all pulls? The
  audit's concern was the *delegated* path; I recommend delegated-only
  to avoid changing CLI behavior.
- **`StallGuard` vs. simpler:** an `AsyncRead` adapter is the clean
  catch-all. A cheaper-but-partial alternative is to wrap only the
  per-record `read_exact(tag)` loop head (catches "no new file for 30s"
  but misses a mid-large-file stall). Recommend the adapter.

## Tests (planned)

- `StallGuard`: a reader that never yields → `TimedOut` after the
  window (deterministic via a pending read); a reader that streams →
  passes through.
- Delegated path passes `Some(30s)`; CLI path passes `None` (behavior
  unchanged) — assert at the call sites.

## Dependency

This is the **prerequisite** for the owner-approved `--retry`/`--wait`
follow-up feature: it converts an infinite stall into a clean, fast,
retryable failure, which the retry loop then catches (transfers resume,
so a retry continues rather than restarts).

## Reviewer comments

(empty — design pending approval)
