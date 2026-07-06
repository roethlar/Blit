# otp-4b-3 — GPT (codex, gpt-5.5) review adjudication

**Reviewed commit**: `3ae0a5f` (otp-4b-3 source cancel responsiveness + e2e).
**Reviewer**: codex-cli 0.142.5, model gpt-5.5, `-s read-only`.
**Raw**: `.review/results/otp-4b3-data-plane.codex.md`.
**Codex verdict**: NEEDS FIXES (test accounting confirmed 1513 → 1515, none removed).

Three findings, all **Accepted**. Fixes in follow-up commit (sha appended below).

## F1 (High) — `mod.rs:888` `dp.queue()` not raced against a peer fault — ACCEPTED
Real. My original scoping ("finish() holds the byte-transfer wall time")
is wrong for a multi-file push: backpressure spreads the blocking across
`queue()` calls, so a mid-transfer cancel commonly lands in `queue()`,
not the final `finish()`. In codex's scenario (earlier batches actively
moving, this send half blocked on backpressure) a cancel closes the send
pipeline, so `queue()` returns a data-plane error — which propagated as
`DATA_PLANE_FAILED`, not the peer's `CANCELLED`.

**Fix**: on a `queue()` error, prefer the peer's framed reason via the
same `prefer_peer_fault` helper the finish() drain uses. NOT raced against
the events channel (unlike finish()): live `Need`s still arrive during the
payload loop and `recv_peer_fault` would consume them. The residual
reader-stuck-*inside*-an-early-`queue()` hang (a worker blocked reading a
slow local file while the channel is full) is the pre-existing slow-local-
read pathology, not cancel-specific, and is bounded by the peer's stall
guard; noted in the finding doc Known gaps.

## F2 (Medium) — `transfer_session_e2e.rs:253` "bytes flowed" gate fires before TCP — ACCEPTED
Real. The `started` notify fired after `write_all` into a 256 KiB local
`tokio::io::duplex` buffer, so it could fire before any body byte crossed
the data-plane socket — the test proved "transfer is mid-flight" but not
the stated "payload bytes flow over the TCP data plane".

**Fix**: shrink the duplex buffer to 4 KiB (< one 64 KiB chunk) so
`write_all` of the chunk only completes once the send pipeline has drained
it out to the socket. `started` now fires after payload bytes have flowed
over the data plane.

## F3 (Medium) — `mod.rs:1176` `recv_peer_fault` silently drops non-fault events — ACCEPTED
Real (low-likelihood but a strict regression in error precision). During
the drain (after `resolve_in_flight_resize`, before `SourceDone`) the
channel is drained and no non-fault event is legitimate, but the old
`Some(_) => continue` dropped a buggy peer's stray `Summary` / duplicate
`NeedComplete` / late `Need` — deferring or losing a fail-fast protocol
error, and risking a hang if the drain is also stuck.

**Fix**: `recv_peer_fault` now returns each non-fault event as a specific
protocol-violation `SessionFault` instead of dropping it (a `Fault` still
passes through; a closed channel still parks so the raced data-plane
future decides). On the happy path the drain completes and the helper is
dropped while parked on `recv()` having consumed nothing.

## Verification after fixes
`cargo fmt --check` ✓, `cargo clippy --workspace --all-targets -D warnings`
✓, `cargo test --workspace` **1515/0** ✓. Guard proofs from the reviewed
commit still hold (e2e select revert → hang→timeout FAIL; unit
prefer_peer_fault revert → wrong code FAIL).

**Fix sha**: `__FIXSHA__` (to be filled after commit).
