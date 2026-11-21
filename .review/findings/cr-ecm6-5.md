# cr-ecm6-5: Nothing bounded what one connection could buffer across streams

**Severity**: MEDIUM — memory, not data: many slow readers on one connection could hold far more unread than before the window fix; found by reading the code
**Status**: Fixed, awaiting re-review
**Branch**: — (default-branch mode; fixes land on master)
**Commit**: the commit carrying this record ("Fix cr-ecm6-5: …")
**Reviewer**: codex / gpt-6.2-astra (as dispatched) / max / frontier, codex-cli 0.171.0 headless one-shot, openreview `exclude-case-r6` over `3bc3ee47..a13238b6` (verdict `acceptable_with_changes`, capability_ok and guard_confirmed true); `.review/results/exclude-case-r6.codex.md`. On the fix `h2-small-frames` (`f72c7dd1`), which was in that range.

## Evidence
The small-frame fix raises the connection window to nearly 2 GiB. Its size proof covers two client streams or four server streams, while the builder permits higher concurrency. Simultaneous slow readers can accumulate substantially more buffered data than the former connection limit allowed. The reviewer asked for the windows and concurrency to be sized to an explicit aggregate memory budget and verified under multiplexed traffic.

Checked in the libraries (hyper 1.11.0, tonic 0.14.6): hyper's own server default is 200 concurrent streams, but tonic hands its own setting through, and tonic's default is none — so a Blit server took ANY number of streams on a connection. Before the window fix the 1 MiB (server) or 5 MiB (client) connection window capped what all of them together could hold unread; at its maximum that window caps nothing.

## Predicted observable failure
One connection opening many streams and reading none: each holds up to its stream window unread (1 MiB on a daemon, 2 MiB on a client), without limit on their number.

## Intake
Admitted. `h2-small-frames` recorded the uncovered stream counts as a known gap and left them unbounded.

## What
`HTTP2_MAX_CONCURRENT_STREAMS = 8`, set on every Blit server (`production_server_builder`): a connection carries at most eight streams at once, and a ninth request waits its turn on the client. That states the bound the connection window used to give: per connection, at most 8 MiB of unread payload on a server and 16 MiB on a client. The small-frame budget is now proved against that number of streams, with the smallest frame a gRPC message makes (five bytes, its length prefix), not against the two or four streams of one-byte frames the first proof used; one-byte frames by the windowful are what the library's limit exists to cut off.

## Guard proof
- `grpc_server::tests::the_small_frame_budget_covers_every_stream_a_connection_carries`: at the limit, each stream's whole window full of five-byte frames fits the budget, on a client and on a server; twice the streams would not; the unread-payload bounds are 8 MiB and 16 MiB; the four constants.
- `small_frames::every_stream_a_connection_may_carry_read_later_is_whole`: eight streams multiplexed on ONE connection, 40,000 small frames each, all unread until every one has been sent — each stream then reads whole.
- `small_frames::a_connection_carries_no_more_streams_at_once_than_the_policy_allows`: twelve streams asked for on one connection, each read to its end — all whole, and never more than eight being sent at once.
- Mutations, each restored byte for byte: no limit on a server → the third red (twelve at once); the limit at 16 → the first red; and the two window mutations of `h2-small-frames` still red, the multiplexed guard among them.

## Files changed
- `crates/blit-core/src/remote/grpc_server.rs` — `HTTP2_MAX_CONCURRENT_STREAMS`, the server builder, the arithmetic guard.
- `crates/blit-cli/tests/small_frames.rs` — the stand-in counts its streams; two guards.
- `CHANGELOG.md` — the limit, for a program built on the library.

## Known gaps
- The multiplexed guard holds a quarter of each stream's window unread, not all of it; filling eight windows with small frames takes millions of messages. The full-window case rests on the arithmetic.
- The limit is the server's. A Blit client against a server that advertises none could open more; both ends are the same build by rule.
- The number of CONNECTIONS a daemon accepts is not bounded, and was not before.
- A front end that multiplexes more than eight long-lived streams on one channel waits on the ninth. Blit's own commands open one or two.
- Run on Linux only, loopback only.
