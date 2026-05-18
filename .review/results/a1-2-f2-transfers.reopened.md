# a1-2-f2-transfers reopened

Reviewed sha: `024e4068a206389fb1887a2c86fea60fe6b5e5b9`
Reviewed at: 2026-05-18T04:16:18Z
Reviewer: reviewer
Verdict: reopened

## Findings

### 1. Connected masks an initial GetState failure

Severity: Low

Location: `crates/blit-tui/src/main.rs:254`

Round 4 closes the ordering and recent-row de-dup issues, but it introduces a status regression when Subscribe succeeds and the initial `GetState` fails. In that path `run_event_loop` first sets `status = Degraded("initial GetState failed: ...")`, then immediately drains the startup buffer. The buffer always contains the pre-sent `EventOrError::Connected`, and `drain_startup_events` handles that by unconditionally setting `status = Live`.

That means the footer can say `live` even though the initial active/recent snapshot is missing. The stream may be healthy for future events, but the F2 pane is not fully reconciled and can be missing transfers that were active or recently finished before the TUI opened.

Please only let `Connected` transition `Connecting -> Live`; it should not overwrite an existing `Degraded` status from a failed snapshot. A targeted unit test around the startup-drain status transition would pin this.

## Validation

- `cargo fmt --all -- --check` passed.
- `cargo clippy --workspace --all-targets -- -D warnings` passed.
- `cargo test -p blit-tui` passed.
- `cargo test --workspace` passed.
