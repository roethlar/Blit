# a1-6-screen-router reopened

Reviewed sha: `55d89e8985e231f408ea3d9906b09a8b667cd297`

Verdict: reopened

## Finding

### 1. Medium — Router navigation reintroduces overlapping terminal input readers

The new router switches panes by returning `LoopOutcome::Navigate(target)` from the active pane and immediately entering the destination pane's event loop. Each pane still creates its own `mpsc` channel and calls `spawn_input_task(key_tx)` on entry:

- `run_router`: `crates/blit-tui/src/main.rs:159`
- F2 input task: `crates/blit-tui/src/main.rs:333`
- F1 input task: `crates/blit-tui/src/main.rs:510`
- F3 input task: `crates/blit-tui/src/main.rs:731`
- F4 input task: `crates/blit-tui/src/main.rs:953`
- blocking reader: `crates/blit-tui/src/main.rs:1101`

On navigation, the old loop returns and drops its receiver, but the old `spawn_blocking` reader can remain inside `event::poll(Duration::from_millis(EVENT_POLL_INTERVAL_MS))` until the timeout or the next read/send attempt. The router starts the next pane immediately, which spawns a second blocking reader on the same crossterm event stream. For at least that transition window, two threads can call `event::poll` / `event::read`.

That can drop or misroute operator input. A realistic sequence is: user presses F2, the old pane returns `Navigate(F2)`, then the user quickly presses F3. The old reader can consume the F3 event before it notices its receiver is closed; `blocking_send` then fails, or the event lands in the old channel that is being dropped. The new pane never sees the F3 key. This is the same class of "multiple terminal readers consume keystrokes" problem that the a1-2 single-owner input task comment says was fixed.

The fix should make terminal input owned once for the whole TUI lifetime, probably at the router level, and pass a single receiver/stream through pane handlers. Pane transitions should not spawn a fresh crossterm reader.

## Gates

- `cargo fmt --all -- --check` passed
- `cargo clippy --workspace --all-targets -- -D warnings` passed
- `cargo test -p blit-tui` passed
- `cargo test --workspace` passed
