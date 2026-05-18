# a1-2-f2-transfers reopened

Reviewed sha: `71840a0347f6ed57532ad4ddcd8a47dc0677e949`
Reviewed at: 2026-05-18T03:39:17Z
Reviewer: reviewer
Verdict: reopened

## Findings

### 1. Subscribe events detach in-flight keystroke polls

Severity: Medium

Location: `crates/blit-tui/src/main.rs:265`

Each render-loop iteration creates a fresh `tokio::task::spawn_blocking` keystroke poll, then races that `JoinHandle` against `rx.recv()` in `tokio::select!`. When a Subscribe event wins, the losing `JoinHandle` is dropped. Dropping a Tokio `JoinHandle` does not stop the spawned blocking task, so the old crossterm poll continues detached for up to the 50 ms timeout and can still call `event::read()`.

Under active progress traffic this can leave multiple detached blocking tasks polling/reading terminal input concurrently. A detached task can consume `q` or `r` and discard it, so quit/refresh becomes lossy exactly while transfers are active. It can also accumulate unnecessary blocking tasks when many progress events arrive before the poll timeout.

Please make terminal input a single owner. A dedicated input task that loops over `event::poll/read` and sends key presses through an mpsc would fit this design; the main loop can then `select!` between `key_rx.recv()` and Subscribe events without dropping unfinished crossterm reads. Keeping one pending join handle across loop iterations would also need to account for the fact that `spawn_blocking` work is not cancellable once started.

### 2. Idle successful Subscribe streams stay stuck on Connecting

Severity: Low

Location: `crates/blit-tui/src/main.rs:239`

After a remote parses successfully, the loop spawns the Subscribe forwarder but only changes `ConnectionStatus::Connecting` to `Live` after receiving a daemon event. An idle daemon can establish the Subscribe stream successfully and then emit no events indefinitely, leaving the footer on `connecting...` even though the live stream is already open.

Please surface successful stream establishment separately from transfer events, for example by having the forwarder send a `Connected`/`Live` control message immediately after `jobs::subscribe(...)` succeeds. That keeps "no events yet" distinct from "still connecting".

## Validation

- `cargo fmt --all -- --check` passed.
- `cargo clippy --workspace --all-targets -- -D warnings` passed.
- `cargo test -p blit-tui` passed.
- `cargo test --workspace` passed.
