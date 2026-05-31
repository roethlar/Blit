# audit-1c2-stall-wiring: apply StallGuard to the receive pipeline (all pulls)

**Severity**: Robustness
**Status**: In progress / pending review
**Branch**: `phase5/a1`
**Commit**: `906cedf`
**Parent finding**: `audit-1c-transfer-stall-timeout` (part 2 of 2 —
completes audit-1c). Builds on `audit-1c1-stall-guard` (the adapter).

## What

Wires the `StallGuard` idle-timeout adapter into the data-plane receive
path so every pull aborts a no-bytes-for-30s stall with a clean
`TimedOut` instead of pinning resources forever (owner scope: all pulls).

## Approach

- **`pipeline.rs`** — generic-ize `execute_receive_pipeline` and its six
  read helpers (`read_u32`/`u64`/`i64`/`string`/`file_header`/
  `tar_shard`) from concrete `&mut TcpStream` to `R: AsyncRead + Unpin
  (+ Send)`. The receive path is **read-only** on the socket — file bytes
  are streamed to the sink via `write_file_stream`, which already takes
  `&mut (dyn AsyncRead + Unpin + Send)` — so this is a pure
  generalization. `TcpStream` still satisfies the bound (the loopback
  fuzz test caller is unchanged); the now-unused `TcpStream` import was
  dropped.
- **`pull.rs`** (`receive_data_plane_stream_inner`) — wrap the connected
  socket in `StallGuard::new(stream, PULL_STALL_TIMEOUT)` immediately
  before `execute_receive_pipeline`. Applied **unconditionally**, so it
  covers delegated daemon→daemon and direct remote→local pulls alike
  (the all-pulls decision lets us wrap at the boundary instead of
  threading an opt-in through every caller). The gRPC-fallback receive is
  separately bounded by HTTP/2 keepalive (audit-1b).

## Files changed

- `crates/blit-core/src/remote/transfer/pipeline.rs`: generic signatures
  + dropped unused import + 1 test.
- `crates/blit-core/src/remote/pull.rs`: StallGuard wrap + import.

## Tests

`receive_pipeline_aborts_on_stall` — a `StallGuard` over a never-written
duplex makes the first record-tag read stall; the pipeline surfaces the
"stalled" `TimedOut` instead of hanging. The adapter's own behavior
(idle-not-total, passthrough) is covered by audit-1c1's 3 tests. Existing
pipeline + pull tests unchanged and green; full workspace gate green.

## Follow-up

This is the prerequisite for the owner-approved `--retry`/`--wait`
feature: a stall is now a clean fast failure the retry loop can catch
(transfers resume, so a retry continues rather than restarts).

## Reviewer comments

(empty — pending review)
