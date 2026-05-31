# d-65-f1-push-mirror-move reopened

Reviewed sha: `2e8e8a23366db0b1e746ed716a79d126ec031afc`
Reviewer: `claude-reviewer`
Timestamp: `2026-05-22T23:39:19Z`

Gates run from detached worktree:

- `cargo fmt --all -- --check` passed.
- `cargo clippy --workspace --all-targets -- -D warnings` passed.
- `cargo test --workspace` passed: 552 tests.

## Findings

1. High: TUI mirror push can purge the remote destination after a partial local source scan.

   In `crates/blit-tui/src/main.rs:3361`, `spawn_f1_push` always builds `PushExecution { require_complete_scan: false, ... }`. For `kind == PullKind::Mirror`, the same execution also sets `mirror_mode: true` and `mirror_kind: MirrorMode::All`, so the daemon is allowed to delete destination entries based on whatever source manifest the client produced.

   The CLI mirror-push path intentionally does the opposite: `crates/blit-cli/src/transfers/remote.rs:248` sets `require_complete_scan: mirror_mode`. That guard exists because a local enumeration error can omit files from the source manifest; allowing a mirror purge after that can delete valid remote files that only appear absent because the source scan was incomplete. The existing `remote_push_mirror_safety` coverage exercises the CLI path, not this new TUI construction path, so this slipped through.

   Please make the F1 mirror-push execution require a complete source scan, and add focused coverage that pins the TUI's mirror-push execution options or the extracted builder used by `spawn_f1_push`. Copy/move push should keep their current semantics unless there is a separate reason to require complete scans for those paths.
