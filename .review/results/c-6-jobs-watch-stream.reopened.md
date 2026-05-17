# c-6-jobs-watch-stream reopened

Verdict: Reopened
Reviewed sha: `ea7a8d7853bd9301c04a46051df982c147aa8480`
Reviewer: `reviewer`
Timestamp: `2026-05-17T15:48:07Z`

Validation:

- `cargo fmt --all -- --check`: pass
- `cargo clippy --workspace --all-targets -- -D warnings`: pass
- `cargo test --workspace`: pass

## Findings

1. Medium - `jobs watch` can miss the terminal event between `GetState` and `Subscribe`, then hang forever.

   `run_jobs_watch` snapshots the daemon first (`crates/blit-cli/src/jobs.rs:125-160`) and only opens the filtered Subscribe stream afterwards (`jobs.rs:162-164`). If the transfer is active in that snapshot but completes before the Subscribe RPC registers its receiver, the daemon has already emitted `TransferComplete`/`TransferError` and no replay exists in this slice. The new stream then waits for future events for that transfer ID that will never arrive. With the default `--timeout-secs 0` (`crates/blit-cli/src/cli.rs:127-130`), this is an unbounded hang.

   This is the race the streaming design needs to remove from the old polling path, not reintroduce in a narrower window. Register the filtered Subscribe stream before the state snapshot, or otherwise do a second `GetState` reconciliation after Subscribe registration and before waiting on stream events. The safe ordering is: subscribe/register receiver, query `GetState`, return immediately if recent/not-found, otherwise consume the already-registered stream. Add a regression/integration test or a factored unit seam that proves an active snapshot followed by a terminal event before stream consumption returns a terminal exit instead of waiting for timeout.

2. Medium - streaming terminal JSON no longer preserves the existing `finished` object shape.

   The finding doc says existing `active` / `finished` / `not_found` / `timeout` JSON states are preserved verbatim, with only a new `progress` state added. The terminal streaming emitters do not preserve that contract. The existing `WatchSnapshot::Finished` JSON from `print_watch_json` includes `kind`, `peer`, `module`, `path`, `start_unix_ms`, `duration_ms`, `ok`, and `error_message` (`crates/blit-cli/src/jobs.rs:430-440`). The new `TransferComplete` path emits `state`, `transfer_id`, `bytes`, `files`, `duration_ms`, `tcp_fallback_used`, and `ok` (`jobs.rs:387-397`), while the new `TransferError` path emits only `state`, `transfer_id`, `ok`, and `error_message` (`jobs.rs:403-410`).

   JSON-Lines consumers watching for the old `finished` schema now get a different terminal object whenever the transfer finishes after Subscribe is opened, but the old schema when it was already in `recent[]` during the initial snapshot. Make terminal streaming output stable with the pre-existing `finished` shape, either by caching the active snapshot fields and merging event-only fields carefully, or by reconciling through `GetState` on terminal events before printing JSON. Add focused tests for the JSON emitters so the field set cannot drift silently.
