# cr-ecm8-4: Eight streams at once did not bound what a client holds unread

**Severity**: MEDIUM — the bound stated for the payload one connection can hold unread (and the number the small-frame budget is proved against) was not established on a client: responses whose streams have ended are not counted by the stream limit
**Status**: Fixed — awaiting review
**Branch**: — (default-branch mode; fixes land on master)
**Commit**: this commit (`Fix cr-ecm8-4`)
**Reviewer**: codex / gpt-6.2-astra (as dispatched) / max / frontier, codex-cli 0.171.0 headless one-shot, openreview `exclude-case-r8` over `3bc3ee47..f4203c31` (verdict `acceptable_with_changes`, capability_ok and guard_confirmed true); `.review/results/exclude-case-r8.codex.md`.

## Evidence
The reviewer retained nine unread one-row listing responses on one channel. A finished stream releases its slot while its body can remain buffered, so multiplying eight active streams by their windows does not establish the stated 16 MiB client ceiling. Asked for: an aggregate receive limit that lasts until bodies are consumed or dropped, and a test of completed unread responses.

Cause: `h2-small-frames` set the HTTP/2 connection window to its maximum, which removed the one limit that had bounded unread payload across a connection. cr-ecm6-5 restated the bound as "eight streams at once, each holding at most its stream window". The stream limit counts streams that are OPEN. A response's stream ends when the daemon has sent it; its body stays in the connection until the program reads it. So the count of unread bodies was bounded by nothing, and with it both the payload held and the small frames held — the quantity the budget proof (`the_small_frame_budget_covers_every_stream_a_connection_carries`) multiplies by eight.

## Predicted observable failure
A program built on `blit-core` that asks for many listings on one channel and reads them later (a front end filling several panes) holds a stream window of each, without limit; with enough of them made of small frames, the library closes the connection as it did before `h2-small-frames`. blit's own commands open one or two streams on a connection and read them as they come.

## Intake
Admitted, as asked: a limit that lasts until bodies are consumed or dropped.

## What
- `client::BoundedResponses` wraps every channel a Blit client is built on (`client::BlitChannel`; `connect_with_timeout`, and the transfer session's own connect). It holds a place for each response from before the request is sent until the response's BODY has been read to its end or dropped; there are `HTTP2_MAX_CONCURRENT_STREAMS` places, eight, and a request beyond them waits for one, as a request beyond the stream limit waits for a stream. Clones of a client share one connection and one set of places.
- The place rides in the body, not beside it: a caller that takes the stream out of a response (`into_inner`) and drops the rest still holds it.
- The policy's account of itself (`grpc_server.rs`) now says what the bound is a bound on — bodies not yet read — and where it is enforced on a client. On a server a request's body belongs to the call answering it and goes when that call ends, with its stream.

No new package: `http-body` (the crate tonic's bodies are made of, already in the lock file) becomes a direct dependency of `blit-core`, for the `Body` trait the wrapper implements.

## Guard proof
- `client::tests::a_ninth_response_waits_while_eight_are_unread`, on the wrapper alone with a service that answers every request at once and whole: eight responses unread, and a ninth request (through a clone) waits; one dropped, it goes through; eight again, one read part of the way, the tenth still waits; that one read to its end and KEPT, the tenth goes through.
- `tests/small_frames.rs::a_ninth_listing_waits_while_eight_complete_ones_are_unread`, the reviewer's case through `connect_with_timeout` and a real daemon: eight one-row `find` listings asked for and none read, each complete on the wire; the ninth waits; one listing read to its end and kept, the ninth is answered; then a tenth waits until an unread one is dropped.
- The five earlier small-frame guards and the whole suite run through the wrapper unchanged.
- Mutations, each restored byte for byte: a place given back as soon as the response is answered → both guards red; a response read to its end keeping its place until dropped → both; each request drawing on places of its own → both.

## Files changed
- `crates/blit-core/src/client.rs` — `BoundedResponses`, `BlitChannel`, `bounded`; `connect_with_timeout`; the guard.
- `crates/blit-core/src/remote/transfer/session_client.rs` — the session's client is built on the bounded channel.
- `crates/blit-core/src/remote/grpc_server.rs` — the policy's account of the bound.
- `crates/blit-core/Cargo.toml`, `Cargo.lock` — `http-body` named directly.
- `crates/blit-cli/tests/small_frames.rs` — the real-daemon guard.
- `CHANGELOG.md` — the bound as it now is; the API change.

## Known gaps
- **A program that holds eight responses unread and asks for a ninth on the same channel waits until it reads or drops one** — for ever, if it never does. That is the limit doing its work, and it is a way to hang a front end that collected responses to read them all later; the changelog says so.
- The server's side is not enforced by a wrapper: it rests on the stream limit and on a call's request body going when the call ends. A client that resets streams it has filled can make a daemon hold more than eight windows for as long as its calls take to notice; the HTTP/2 library has its own limits on reset streams, which this change does not examine.
- A raw `tonic` client built on a bare `Channel` (tests do this) has neither the window policy nor this bound.
- Run on Linux only.
