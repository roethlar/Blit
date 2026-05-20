# d-41-f3-du reopened

Reviewed commit: `d804f227a80cdca4400a8247c725e5b9233dab2a`
Reviewed at: `2026-05-20T17:13:32Z`
Reviewer: `reviewer`

Validation:
- `cargo fmt --all -- --check` passed.
- `cargo clippy --workspace --all-targets -- -D warnings` passed.
- `cargo test --workspace` passed.

Findings:

1. F3 `u` uses unbounded `DiskUsage` for a one-line aggregate.

   `crates/blit-tui/src/main.rs:2240` calls `du::stream(..., 0, ...)`, and the d-41 notes describe `max_depth = 0` as a single aggregate root row. The daemon does not implement that meaning: `crates/blit-daemon/src/service/core.rs:1144` maps request `max_depth == 0` to `None`, which is unbounded depth. With `None`, the accumulation helpers at `crates/blit-daemon/src/service/admin.rs:365` and `crates/blit-daemon/src/service/admin.rs:379` keep every prefix, and `crates/blit-daemon/src/service/admin.rs:442` streams every accumulated path back to the client.

   The TUI then discards almost all of that stream by keeping the largest byte entry in `crates/blit-tui/src/main.rs:2253`. That preserves the displayed total in normal cases, but it makes a cursor-stats hotkey perform the full unbounded `blit du` response shape over gRPC. On a large subtree this can allocate and stream one row per descendant path just to render `Subtree: X across N files`, contradicting the feature's "single Stats line" behavior and the finding's "re-querying is cheap" assumption.

   Please route F3 du through an aggregate-only request path, or otherwise make the request bounded so it cannot emit the full descendant stream. Add coverage that would fail if F3 aggregate mode asks for unbounded depth; a daemon-side test around `stream_disk_usage` with many descendants would pin the contract better than only testing `du_total_from_entries`.
