# audit-h3b — Write-side StallGuardWriter on daemon pull data plane

**Source**: 2026-06-04 audit chain, R2/R3 finding **H3** (DoS-class hardening gap).
**Parent finding**: R3 H3. Closes one of the three remaining h3 gaps. h3a (daemon
push-receive) shipped at master `dd51a1c`; h3c (CLI gRPC fallback) still pending.
**Owner-ratified 2026-06-05**: extend `TRANSFER_STALL_TIMEOUT` (30 s) symmetrically to
write progress on sender paths.

## What

R2/R3 H3 originally listed "daemon pull-data-plane accepts" as a missing
receive-side `StallGuard` target. Wrong category:

- The daemon is the **sender** on a pull (writes bytes to the puller via
  `DataPlaneSession::send_*`).
- The stall surface is **TCP write backpressure**: when the remote reader stalls
  (slow / wedged process, congested link, etc.), the kernel send buffer fills and
  the daemon's `write_all` blocks indefinitely — until OS-level TCP retransmit
  exhaustion (15+ minutes), at which point the connection finally dies on its own.
- The accept and token-read phases on those paths are already bounded by
  `PULL_ACCEPT_TIMEOUT` / `PULL_TOKEN_TIMEOUT`. What was missing is **write
  progress after token acceptance**.

`StallGuard` (audit-1c) wraps `AsyncRead` and would never fire on a write path.
h3b adds the symmetric write-side adapter.

## Approach

1. **New `StallGuardWriter<W>` adapter** in
   `crates/blit-core/src/remote/transfer/stall_guard.rs`. AsyncWrite mirroring
   `StallGuard` on the read side: `poll_write` resets the idle deadline on every
   successful return; a `Pending` past the window trips
   `io::ErrorKind::TimedOut`. `poll_flush` / `poll_shutdown` pass through cleanly.
   Same idle-vs-total semantics: a steadily-progressing transfer (any non-trivial
   network at all) is never aborted; only "no observable progress for 30 s" trips.

2. **`DataPlaneSession.stream` field type change** in
   `crates/blit-core/src/remote/transfer/data_plane.rs`: `TcpStream` →
   `StallGuardWriter<TcpStream>`. `from_stream` wraps the input internally; the
   ~30 existing `self.stream.write_all/.flush` call sites in this file (file
   headers, chunks, tar shards, block/resume records, terminator, flush) compose
   against the wrapper's AsyncWrite impl with no per-site edits.

3. **All three production call sites pick up the guard unchanged**:
   - `crates/blit-daemon/src/service/pull.rs:743` (regular multi-stream pull).
   - `crates/blit-daemon/src/service/pull_sync.rs:641` (regular single-stream
     pull-sync).
   - `crates/blit-daemon/src/service/pull_sync.rs:765` (resume mode).
   The client-side `DataPlaneSession::connect` (used by `RemotePushClient`)
   internally calls `from_stream`, so the client push path also inherits the
   guard — secondary benefit, not the focus of this slice.

4. **Audit-doc wording fix** in
   `docs/audit/AUDIT_REPORT_2026-06-04_R2.md` §H3 per GPT's review: the original
   "daemon pull-data-plane accepts" phrasing was imprecise. Updated to "daemon
   pull-data-plane write progress after token acceptance," with h3a/h3b
   remediation status recorded and h3c flagged as pending.

5. **Module doc in `stall_guard.rs`** expanded to name all four data-plane paths
   and clarify which adapter covers each.

## Files changed

- `crates/blit-core/src/remote/transfer/stall_guard.rs`:
  - Add `use tokio::io::AsyncWrite`.
  - Update module doc to name all four scope paths (1c, h3a, h3b, h3c).
  - Update `TRANSFER_STALL_TIMEOUT` doc to mention `StallGuardWriter` and the
    h3b clarification.
  - Add `pub struct StallGuardWriter<W>` + `impl<W> StallGuardWriter<W> { new,
    into_inner }` + `impl<W: AsyncWrite + Unpin> AsyncWrite for
    StallGuardWriter<W>`.
  - Add 3 unit tests under the existing `mod tests`:
    `write_times_out_when_reader_stalls`,
    `write_passes_data_through_unchanged`,
    `write_does_not_trip_on_steady_trickle_past_total_window`.
- `crates/blit-core/src/remote/transfer/data_plane.rs`:
  - Add `use super::stall_guard::{StallGuardWriter, TRANSFER_STALL_TIMEOUT};`.
  - Change `DataPlaneSession.stream` field type to
    `StallGuardWriter<TcpStream>`. Add a comment explaining the audit-h3b
    rationale.
  - In `from_stream`, wrap: `stream: StallGuardWriter::new(stream,
    TRANSFER_STALL_TIMEOUT)`. Doc-comment updated to reference the three
    production call sites that inherit the guard.
- `docs/audit/AUDIT_REPORT_2026-06-04_R2.md`:
  - §H3 heading and prose updated for the write-side framing; remediation
    status now records h3a verified + h3b shipped + h3c pending.

## Tests added

- `stall_guard::tests::write_times_out_when_reader_stalls` — duplex(64) with the
  reader half held but never read; the first 64-byte write fills the kernel
  buffer; the next write goes Pending; the StallGuardWriter trips with
  `io::ErrorKind::TimedOut` after the 20 ms test window.
- `stall_guard::tests::write_passes_data_through_unchanged` — actively-draining
  peer completes writes normally; bytes received intact end-to-end.
- `stall_guard::tests::write_does_not_trip_on_steady_trickle_past_total_window`
  — 3 writes spaced 20 ms apart against a 50 ms idle window; total span exceeds
  the window but each gap is under it. The trickle must NOT trip. Mirror of the
  existing read-side load-bearing property test.

Workspace validation suite green: `cargo fmt --all -- --check`,
`cargo clippy --workspace --all-targets -- -D warnings`,
`cargo test --workspace`. blit-core test count went 312 → 315 = the 3 new tests.

## Known gaps

- **DataPlaneSession-level integration test**: no test directly constructs a
  `DataPlaneSession` over a real (or duplex-simulated) stall and asserts the
  TimedOut surfaces through `send_file_*` / `finish`. The wiring is one-line
  (the wrap inside `from_stream`) and the AsyncWrite composition is exercised
  by the three new unit tests; a future slice that touches the session
  abstraction may want to add a virtual-time DataPlaneSession test.
- **Client-side push** inherits the guard incidentally via
  `DataPlaneSession::connect` → `from_stream`. That's correct (a stalled remote
  receiver on a push also pins the CLI worker), but it wasn't the audit-h3b
  scope. If owner wants a separate client-side guarantee, the existing wiring
  covers it; the secondary coverage is documented in `from_stream`'s rustdoc.
- **gRPC fallback (h3c)** is unchanged. It sits below `tonic::Streaming<T>`
  rather than `AsyncRead`/`AsyncWrite` and needs a different mechanism
  (per-message `tokio::time::timeout` or a `Stream` adapter mirroring
  StallGuard). Separate slice, separate design decision.

## Cross-references

- R3 finding H3, `docs/audit/AUDIT_REPORT_2026-06-04_R3.md`.
- R2 finding H3, `docs/audit/AUDIT_REPORT_2026-06-04_R2.md` (updated by this
  slice with the corrected wording).
- audit-1c (read-side parent guard), `.review/findings/audit-1c1-stall-guard.md`
  and `audit-1c2-stall-wiring.md`.
- audit-h3a (daemon push-receive symmetric guard, verified at master
  `dd51a1c`), `.review/findings/audit-h3a-push-receive-stall.md`.
- Memory `audit-owner-decisions`: 30 s no-bytes scope, now extended to write
  progress per 2026-06-05 owner directive.
- Memory `feedback-server-await-timeouts`: this is the write-side application
  of that rule for long-running daemon handlers.
