# a1-6b-state-preservation reopened

Reviewed sha: `2fbcd1a13747c17ea420490edde5d30f1661588a`

Verdict: reopened

## Findings

### 1. Medium — Hidden F2 setup can block the TUI before the first F1 render

`run_router` now does the F2 Subscribe setup and initial `GetState` inline before entering the screen loop:

- `crates/blit-tui/src/main.rs:287`
- `crates/blit-tui/src/main.rs:288`
- `crates/blit-tui/src/main.rs:290`

Because `--screen` now defaults to F1, a normal invocation like `blit-tui --remote host:9031` should be able to draw the Daemons pane immediately while F2/F3 consume the remote later. Instead, if the remote is valid syntax but slow or unreachable, `open_subscribe_stream(endpoint).await` / `jobs::query(endpoint, 0).await` can hold the entire TUI before any frame is rendered. That also changes the earlier contract documented around F1: F1 is mDNS-only and ignores `--remote`.

The F2 setup needs to be lazy or actually backgrounded. It should not block the router's initial draw for screens that do not need F2 state.

### 2. Medium — Background feeds are still drained only by the active pane

The finding doc says the refactor replaces the four pane loops with a single loop that selects over all background channels plus keystrokes. The implementation still enters one pane loop at a time:

- router dispatch: `crates/blit-tui/src/main.rs:320`
- F1 drains discovery/detail only inside `run_f1_pane_loop`: `crates/blit-tui/src/main.rs:650`
- F2 drains Subscribe only inside `run_f2_pane_loop`: `crates/blit-tui/src/main.rs:507`
- F3 drains browse fetch replies only inside `run_f3_pane_loop`: `crates/blit-tui/src/main.rs:839`
- F4 drains profile replies only inside `run_f4_pane_loop`: `crates/blit-tui/src/main.rs:1004`

That means hidden-pane producers can back up. F1 discovery has a bounded channel of 4; if the operator spends more than a few scan intervals on another pane, the discovery task can block on send. When the operator returns, `replace_from_discovery(&services, Instant::now())` stamps an old scan as freshly seen. F2's Subscribe mpsc is bounded at 256; a long-running transfer while the operator is on F1/F3/F4 can fill it, causing the TUI forwarder to stop reading the gRPC stream until F2 is revisited.

This is not just an implementation-shape nit: the state being preserved can be stale, and the "background tasks stay alive across navigation" guarantee is incomplete unless their outputs are continuously drained. The fix should be a real app-level event loop, or equivalent background fan-in, that processes all active feeds regardless of the visible pane.

### 3. Low — Remote parse errors are reduced to `invalid endpoint`

The router now parses with `.ok()`:

- `crates/blit-tui/src/main.rs:220`

and later reports generic messages:

- F3: `crates/blit-tui/src/main.rs:278`
- F2: `crates/blit-tui/src/main.rs:308`

Before this refactor, F2/F3 surfaced the actual `RemoteEndpoint::parse` error, including helpful messages like backslash guidance or missing module-path syntax. Store the parse `Result` or parse error string so both panes keep the specific diagnostic.

## Gates

- `cargo fmt --all -- --check` passed
- `cargo clippy --workspace --all-targets -- -D warnings` passed
- `cargo test -p blit-tui` passed
- `cargo test --workspace` passed
