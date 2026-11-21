# h2-small-frames: A stream of small frames closed its own connection

**Severity**: HIGH — a push of many small files to a daemon failed outright; a listing read through a pager failed part way. On the default branch since 2026-10-17; in no release
**Status**: Fixed `f72c7dd1`; found by the working agent. `exclude-case-r6` (Codex openreview, 2026-11-21) had it in its range `3bc3ee47..a13238b6` and required one change to it, from reading the code: the buffering it allows across concurrent streams is not bounded (cr-ecm6-5)
**Branch**: — (default-branch mode; fixes land on master)
**Commit**: `f72c7dd1`
**Reviewer**: none. Found 2026-11-20 while measuring `du` for `docs/plan/UTILITY_VERBS_LOCAL_PARITY.md` (AC-7): the old `du` failed on its second run over 200,000 files.

## Evidence
`docs/bench/h2-small-frames-2026-11-20/` (scripts, output, the library's rule). On `585d3599`, Linux, loopback, release builds:
- `blit copy tree/ HOST:/module/t/` with 200,000 empty files: exit 1 after 35 ms, 768 files written, "session INTERNAL: transfer transport peer closed". The mirror of the same: exit 1.
- `blit find HOST:/module/` with nothing read from it for two seconds: exit 1, three runs of three, "h2 protocol error: error reading a body from connection", about 17,500 of 202,001 lines printed. `blit du` failed the same way without any pause.

Cause: the `h2` library (0.4.16 and later; this workspace took 0.4.20 in `3f15f835`, 2026-10-17, for RUSTSEC-2026-0258) closes a connection with `ENHANCE_YOUR_CALM` once small DATA frames the application has not yet taken have used up a budget, and the budget is half the connection's window. With hyper's default windows that is about 10,000 unread small frames on a client and 2,000 on a server. Every stream blit has is made of small frames — a file header, a manifest entry, a `find` row — so a receiver slower than its sender met the limit. hyper and tonic give no setting for the budget, only for the window.

## Predicted observable failure
- A push or pull of some thousands of small or empty files through a daemon fails within milliseconds, whenever the receiving side falls behind the sender. Files with content are less exposed (a frame of 256 bytes or more gives budget back), which is why ordinary trees mostly passed.
- `blit find … | less`, `blit du -a` to a slow terminal: the listing ends part way with an error.

## Intake
Self-found; nothing to admit or decline. Outside both Active plans as written, and fixed under the owner's standing 1.0 goal (D-2026-10-19-2: every known imperfection blocks the release) — a transfer that fails is the first thing RELIABLE rules out. The owner has not ruled on the fix; it is one constant and is recorded in STATE for their word.

## What
One HTTP/2 window policy for every Blit gRPC connection, in the module that already owned the keepalive policy (`crates/blit-core/src/remote/grpc_server.rs`): the CONNECTION window is the largest HTTP/2 allows (2 GiB − 1), on servers (`production_server_builder`) and on clients (`blit_core::client::endpoint`, through which both client connection sites now go — the admin client and the transfer client). The budget that follows, 1 GiB, is more than a whole stream window of one-byte frames can use, so flow control stops a sender before the budget is spent. The per-stream windows are stated at hyper's defaults (2 MiB client, 1 MiB server) and not changed: a stream still buffers at most its own window, and the speed of a large file is the same (2 GiB: push 286 ms against 285, pull 244 against 244).

Not done: pinning `h2` below 0.4.16 (it was raised for a security advisory), or batching small frames in the protocol (a wire redesign).

## Guard proof
- `grpc_server::tests::a_stream_window_of_the_smallest_frames_fits_the_budget`: the arithmetic — two full client streams, or four server streams, of one-byte frames fit the budget — and the three constants.
- `crates/blit-cli/tests/small_frames.rs`, three guards on every OS, each against a stand-in daemon that sends or receives 40,000 small frames, one message to a frame, taken only after all have arrived: `a_listing_read_later_than_it_arrives_is_whole` (the real `blit find`, through `connect_with_timeout`); `a_transfer_clients_stream_read_later_than_it_arrives_is_whole` (`connect_transfer_client`); `small_frames_sent_to_a_daemon_that_reads_them_later_all_arrive` (`production_server_builder`).
- Mutations, each restored byte for byte, each red in three rounds of three: a server keeping the library's connection window → the third guard; the admin client built from a bare endpoint → the first; the transfer client built from a bare endpoint → the second; the client endpoint setting no connection window → the first and second.
- A first pair of guards over a real tree of 30,000 empty files did NOT go red under those mutations in debug builds (the frames come out large when the sender is polled slowly) and was thrown away; the stand-in makes the frames small on purpose.

## Files changed
- `crates/blit-core/src/remote/grpc_server.rs` — the window constants, the server builder, the arithmetic guard.
- `crates/blit-core/src/client.rs` — `endpoint`; `connect_with_timeout` goes through it.
- `crates/blit-core/src/remote/transfer/session_client.rs` — the transfer client goes through it.
- `crates/blit-cli/tests/small_frames.rs` — the three guards.
- `docs/bench/h2-small-frames-2026-11-20/` — evidence.

## Known gaps
- The limit can still be met by more streams than the arithmetic covers on one connection: three or more client streams, or five or more server streams, each holding a full window of one-byte frames unread. Blit opens one or two streams per connection, and its smallest frame is five bytes.
- A stalled receiver may now hold up to a stream window of small frames in memory with the library's per-frame overhead on each — the cost the library's limit exists to bound.
- Linux only, loopback only. The guards run on every OS but were run on Linux; no CI has run since 2026-10-24.
- A front end outside this repository that builds its own tonic `Endpoint` instead of going through `blit_core::client` keeps the old limit.
