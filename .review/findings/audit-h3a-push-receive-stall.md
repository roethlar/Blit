# audit-h3a — Stall guard on daemon push-receive socket

**Source**: 2026-06-04 audit chain, R2/R3 finding **H3** (DoS-class hardening gap).
**Parent finding**: R2/R3 H3, "Stall guard covers ONE of FOUR receive paths." This slice
closes one of the three missing paths.
**Branch**: shares `fix/audit-h1-mirror-relay-incomplete-scan` with audit-h1 as a linear
sequence — independent slices but stacked for review convenience after slice 1's sentinel
went out. Reviewer can review h3a in isolation by diffing
`c54b208..dd51a1c` (h1 handoff → h3a impl).

## What

Before this slice the daemon push-receive socket had no idle deadline. After accepting
the data plane and the per-stream token (bounded by `DATA_PLANE_TOKEN_TIMEOUT`),
`handle_data_plane_stream` handed the raw TcpStream straight into
`execute_receive_pipeline`. A hostile or wedged push client that completed the token
handshake and then went silent (or sent partial wire-record bytes and stopped) would
park the receive worker indefinitely. DoS-class surface: a single rogue client can pin
a worker; N clients exhaust the daemon's per-module concurrency budget.

The symmetric guard `StallGuard` was shipped on the CLI pull-receive TCP path in
audit-1c1 / audit-1c2. audit-h3a applies the same guard to the daemon push-receive
side, using a single hoisted constant `TRANSFER_STALL_TIMEOUT` (renamed from the
prior pull-scoped `PULL_STALL_TIMEOUT`).

## Approach

Three mechanical changes + one new test:

1. **Hoist the constant** in `blit-core::remote::transfer::stall_guard`:
   `PULL_STALL_TIMEOUT` → `TRANSFER_STALL_TIMEOUT`. Same 30 s value, broader semantic.
   Module doc updated to name the four receive paths (1c CLI pull, h3a daemon push,
   h3b daemon pull-data-plane, h3c gRPC fallbacks).
2. **Update the existing caller** at `blit-core::remote::pull` to the new name.
   Comment refreshed to remove the stale "gRPC-fallback is covered by HTTP/2
   keepalive" claim (R3 H3 / GPT-12 showed that's false — h3c will close it).
3. **Extract a helper** `receive_push_data_plane<R: AsyncRead + Unpin + Send>(socket,
   sink)` in `service::push::data_plane`. The helper composes
   `StallGuard::new(socket, TRANSFER_STALL_TIMEOUT)` + `execute_receive_pipeline`.
   `handle_data_plane_stream` now delegates to it. Helper is generic over AsyncRead
   so it's directly unit-testable without a TcpListener + token handshake.
4. **Regression test** `receive_push_data_plane_aborts_on_stall` uses
   `#[tokio::test(start_paused = true)]` and `tokio::time::advance` to fast-forward
   past the production 30 s `TRANSFER_STALL_TIMEOUT` without wall-clock waits. The
   test exercises the real helper composing the production constant — a future
   refactor that removes the StallGuard wrap fails this test.

The audit-1c2 contract test at `pipeline::tests::receive_pipeline_aborts_on_stall`
covers the pipeline+StallGuard composition at the wire layer; h3a's new test pins
that the daemon push-receive helper itself wires the guard up with the production
constant.

## Files changed

- `crates/blit-core/src/remote/transfer/stall_guard.rs`:
  - Module doc broadened (audit-1c → audit-1c / audit-h3 scope).
  - `PULL_STALL_TIMEOUT` renamed to `TRANSFER_STALL_TIMEOUT`. Doc updated to name
    the four receive paths.
- `crates/blit-core/src/remote/pull.rs`:
  - Import + comment + call-site updated to `TRANSFER_STALL_TIMEOUT`.
  - Comment refreshed to acknowledge h3c (gRPC fallback) is a separate slice rather
    than the prior "covered by HTTP/2 keepalive" claim.
- `crates/blit-daemon/src/service/push/data_plane.rs`:
  - File-level imports for `execute_receive_pipeline`, `SinkOutcome`, `TransferSink`,
    `StallGuard`, `TRANSFER_STALL_TIMEOUT`, `AsyncRead`, `Result`.
  - New private helper `receive_push_data_plane`.
  - `handle_data_plane_stream` delegates to the helper; data-plane comment updated.
  - Test module gains the `receive_push_data_plane_aborts_on_stall` virtual-time
    regression test.
- `crates/blit-daemon/Cargo.toml`:
  - Add `tokio = { features = ["full", "test-util"] }` to `[dev-dependencies]` for
    `tokio::time::pause` / `advance` in the new test.

## Tests added

- `service::push::data_plane::tests::receive_push_data_plane_aborts_on_stall` —
  Virtual-time test (`start_paused = true`), advances past
  `TRANSFER_STALL_TIMEOUT + 1s`, asserts the receive task aborts with a "stalled"
  error in the chain. Exercises the real helper + production constant.

Existing audit-1c1 / audit-1c2 tests continue to pass:
- `stall_guard::tests::times_out_when_reader_stalls`
- `stall_guard::tests::passes_data_through_unchanged`
- `stall_guard::tests::does_not_trip_on_steady_trickle_past_total_window`
- `pipeline::tests::receive_pipeline_aborts_on_stall`

Workspace validation suite green: `cargo fmt --all -- --check`,
`cargo clippy --workspace --all-targets -- -D warnings`,
`cargo test --workspace`.

## Known gaps

- **h3b** — daemon pull-data-plane accepts at `service/pull.rs:702-757` and
  `service/pull_sync.rs:600-755` are not yet covered. Same shape as h3a (TCP
  AsyncRead socket → StallGuard wrap → execute_receive_pipeline). Next slice.
- **h3c** — CLI gRPC-fallback pull message awaits at `pull.rs:752` sit below
  `tonic::Streaming<T>` (not AsyncRead), so the StallGuard shape doesn't apply
  directly. h3c requires a different mechanism (per-message `tokio::time::timeout`
  or a `Stream` adapter). Separate slice with separate design decision.
- **Token-read path**: `socket.read_exact(&mut token_buf)` at line 168 is bounded
  by `DATA_PLANE_TOKEN_TIMEOUT` (existing), so the new StallGuard wrapping the
  post-token receive is fine. The token read does not need the StallGuard.

## Cross-references

- R3 finding H3, see `docs/audit/AUDIT_REPORT_2026-06-04_R3.md` H3 (split into
  R2-H3 + GPT-12).
- audit-1c (parent guard implementation), see
  `.review/findings/audit-1c1-stall-guard.md` and
  `.review/findings/audit-1c2-stall-wiring.md`.
- Memory `audit-owner-decisions`: 30 s no-bytes scope (owner decision).
- Memory `feedback-port-cli-safety-guards`: the rule that motivates symmetric
  coverage when porting CLI guards.
- Memory `feedback-server-await-timeouts`: every long-running `.await` on a socket
  read in a long-running handler needs an outer bound; this is the daemon-side
  variant of that rule.
