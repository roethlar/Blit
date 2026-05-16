# b-4-getstate: GetState RPC + DaemonState handler

**Severity**: Feature (new wire surface, no behavior change to
existing RPCs)
**Status**: In progress / pending review
**Branch**: `phase5/getstate`
**Commit**: filled by the sentinel commit

## What

Lands `GetState` from §6.3 of `docs/plan/TUI_DESIGN.md`. After
b-1/b-2/b-3 the daemon has the always-on `ActiveJobs` table +
recent-runs ring; this slice makes both observable over the
wire via a new RPC. `blit jobs list <remote>` (b-5) and the
future A.1 TUI screens will consume it.

## Approach

Proto: `rpc GetState(GetStateRequest) returns (DaemonState)`
plus the supporting messages — `GetStateRequest` (with
`recent_limit`), `DaemonState`, `ActiveTransfer`,
`TransferRecord`, `Counters` — and a top-level `TransferKind`
enum (Push / Pull / PullSync / DelegatedPull). The enum is
top-level rather than nested under a Subscribe-era
`TransferStarted` so C can share it without a cycle.

`BlitService` grows a `started_at: Instant` captured once at
construction. `ActiveJobKind` gets a `to_wire()` method that
maps the internal enum to the wire one.

The handler reads from the snapshots `active_jobs` already
exposes (no new accessors), maps each row to its wire shape,
loads the `TransferMetrics` atomics with `Relaxed` for the
`Counters` block, and computes uptime as
`Instant::now().duration_since(started_at).as_secs()`.

The handler is always available regardless of `--metrics`. The
counters block reads zeros when the flag is off (the atomics
never incremented), but `active[]` / `recent[]` always
populate from the always-on table.

`recent_limit` is currently unused — the handler returns the
full ring. The proto field is reserved so per-request
truncation in the upcoming `blit jobs list` slice is
non-breaking.

## Files changed

- `proto/blit.proto`:
  - Added `rpc GetState(...) returns (DaemonState)` to the
    Blit service.
  - Added `TransferKind` enum, `GetStateRequest`,
    `DaemonState`, `ActiveTransfer`, `TransferRecord`,
    `Counters` messages at the end of the file.
- `crates/blit-daemon/src/active_jobs.rs`:
  - `ActiveJobKind::to_wire(self) -> generated::TransferKind`
    mapping helper.
- `crates/blit-daemon/src/service/core.rs`:
  - `BlitService` gained a `started_at: Instant` field;
    `from_runtime` initializes it.
  - New `get_state` handler (~80 LOC including doc).
  - New imports: `ActiveTransfer`, `Counters`, `DaemonState`,
    `GetStateRequest`, `TransferRecord`.
  - `+#[cfg(test)] mod tests` block with 3 unit tests
    exercising the handler.
- `crates/blit-cli/tests/remote_remote.rs` — two test
  Blit-impl doubles grew `get_state` stubs returning
  `Status::unimplemented`. Same treatment as the prior
  `delegated_pull` addition.
- `crates/blit-core/tests/pull_sync_with_spec_wire.rs` —
  same shape on `SpyServer`.

## Tests added

- `get_state_empty_daemon_returns_zero_active_and_recent` —
  version equals `CARGO_PKG_VERSION`, counters present but
  zero, all lists empty.
- `get_state_surfaces_live_active_row_and_recent_row` —
  registers a Pull row, asserts it appears in `active[]`
  with the expected kind/peer/module/path; drops the guard
  after `record_outcome(true, None)`; asserts the row moves
  to `recent[]` with `ok=true` and empty `error_message`.
  Byte/file fields asserted to be zero (milestone C will fill).
- `get_state_failure_record_carries_error_message` —
  `record_outcome(false, Some("module not found"))` ends up
  in `recent[]` with `ok=false` + the message preserved.

Workspace: 517 passed (was 514; +3 unit tests).

## Known gaps

1. **`recent_limit` is accepted but unused.** The handler
   returns the full ring today. b-5 (the `blit jobs list`
   CLI verb) will wire truncation. Proto field is reserved so
   adding the truncation is non-breaking.

2. **`active[].bytes_completed` / `bytes_total` and
   `recent[].bytes` / `files` are always zero.** These
   fields come from milestone C's write-loop
   instrumentation. The proto fields are populated for the
   wire shape; the handler explicitly emits zeros.

3. **No integration test against a real tonic server.** The
   unit tests call the handler directly. An end-to-end test
   would spin up a tonic transport and a BlitClient against
   it; that's b-5's territory once the CLI verb has
   something to call.

4. **Counters read independently from the atomics.** No
   single lock surrounds the counter snapshot, so the five
   fields could read from slightly different instants. The
   atomics are `Relaxed`-loaded as elsewhere in
   `TransferMetrics`; tearing on individual fields is
   impossible (each is a single u64). Cross-field
   consistency is approximate — acceptable for an
   observability surface.

## Reviewer comments

(empty — pending grade)
