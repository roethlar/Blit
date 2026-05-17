# c-5a-transfer-id-filter reopened

Verdict: Reopened
Reviewed sha: `7587b46531e95c26dda20a1b04023511457297ca`
Reviewer: `reviewer`
Timestamp: `2026-05-17T15:08:10Z`

Validation:

- `cargo fmt --all -- --check`: pass
- `cargo clippy --workspace --all-targets -- -D warnings`: pass
- `cargo test --workspace`: pass

## Findings

1. Medium - filtered `Subscribe` forwarder tasks can leak after client disconnect.

   Round 2 fixes the prior lag semantics by inserting a per-subscriber forwarder before the client-paced stream (`crates/blit-daemon/src/service/core.rs:389-419`). That part addresses the original finding. The new spawned task, however, has no cancellation path tied to the returned `ReceiverStream`.

   The task owns `tx` and waits on `broadcast_rx.recv().await` forever. It only observes that the client dropped the stream when it tries `tx.send(Ok(event)).await` for a matching event (`core.rs:395-401`) or when it sends a lag error (`core.rs:403-411`). If a filtered watcher disconnects during a quiet daemon period, or after the watched transfer has emitted its terminal event, there may never be another matching event. Unrelated events are filtered with `continue` before any `tx` operation, so they also do not notice that the receiver is gone.

   Practically, each disconnected `jobs watch <id>` can leave behind a task plus a live broadcast receiver indefinitely, keeping the global subscriber count inflated and forcing future broadcasts to retain stale receiver state until a same-id event happens, which is usually never for completed transfer IDs.

   Wire the forwarder to client cancellation. A simple shape is `tokio::select!` between `broadcast_rx.recv()` and `tx.closed()`, or an `if tx.is_closed() { break; }` check before/after filtering unrelated events. Add a regression test that subscribes with a non-empty `transfer_id_filter`, drops the returned stream without sending another matching event, emits unrelated events, and asserts the forwarder exits or at least that the broadcast sender no longer has the stale subscriber.
