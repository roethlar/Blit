Reviewed sha: `c003bb30861919614d1c2a48723abdabebcf27a7`

Reopened.

The forwarder leak itself is fixed: `forward_step` races `tx.closed()` against `stream.message()`, so a silent stream observes the merged receiver being dropped and exits.

The setup hang is still open. `open_subscribe_stream` now wraps `jobs::subscribe(...)` in `tokio::time::timeout`, but `spawn_f2_setup_task` immediately awaits `jobs::query(endpoint, 0)` after a successful subscribe:

- `crates/blit-tui/src/main.rs:2345-2352`: `open_subscribe_stream(...).await.is_ok()` is followed by unbounded `jobs::query(endpoint, 0).await`.
- `crates/blit-app/src/admin/jobs.rs:22-29`: `query` uses `connect_with_timeout`, but the `get_state(...)` RPC await itself is unbounded.

So a daemon can accept/open Subscribe, then stall `GetState`. The setup task remains stuck, `transfers_setup_pending` stays true, later refans are blocked/deferred, and the not-yet-delivered `merged_rx` keeps the newly spawned forwarder alive. That leaves the same class of setup-task/connection leak the finding calls out, just moved from Subscribe-open to the initial snapshot fetch.

Expected fix: bound the initial `GetState` fetch in the setup path too, preferably with the same outer-timeout pattern used for subscribe-open. A focused test should cover "subscribe opens, initial snapshot future never resolves, setup returns/continues with a degraded snapshot instead of hanging."
