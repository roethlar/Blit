# a1-6b-state-preservation reopened

Reviewed sha: `5cf2fe6499f669ff4f4112e4bc0fe97bdc985b39`

Verdict: reopened

## Finding

### 1. Medium — F2 refresh can spawn overlapping setup tasks with stale replies

Round 2 correctly moves F2 setup off the first draw and into `spawn_f2_setup_task`, but there is no "setup in flight" guard or generation check:

- initial setup spawn: `crates/blit-tui/src/main.rs:290`
- F2 refresh spawn while `transfers_event_rx.is_none()`: `crates/blit-tui/src/main.rs:521`
- unqualified setup reply application: `crates/blit-tui/src/main.rs:449`

At startup with a valid but slow remote, `app.transfers_status` is `Connecting` and `transfers_event_rx` is still `None`. If the operator switches to F2 and presses `r`, `handle_pane_action` starts a second `spawn_f2_setup_task` even though the first setup is still pending. Both tasks send the same unversioned `F2SetupReply` into `f2_setup_rx`, and the unified loop applies whichever replies arrive in arrival order.

This can corrupt visible status and stream ownership. Example: setup A succeeds and installs a live `event_rx`; setup B, started by the refresh key, fails later and sets `app.transfers_status = Degraded(err)` even though the stream from A is live. Since normal Subscribe events only promote `Connecting -> Live`, that stale failure can leave F2 showing degraded until another manual refresh. If both succeed, the later reply replaces the active Subscribe receiver with a second stream and drops the first.

The setup path needs either a boolean/generation in `AppState` or a request id in `F2SetupReply`, so `r` does not start duplicate setup while one is pending and stale setup results cannot overwrite newer state.

## Closed From Round 1

- Hidden F2 setup no longer blocks the first draw.
- Background feeds are drained by the unified loop regardless of active pane.
- Remote parse errors preserve the parser's detailed message.

## Gates

- `cargo fmt --all -- --check` passed
- `cargo clippy --workspace --all-targets -- -D warnings` passed
- `cargo test -p blit-tui` passed
- `cargo test --workspace` passed
