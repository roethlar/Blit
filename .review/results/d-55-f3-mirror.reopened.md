# d-55-f3-mirror reopened

Reviewed commit: `3d1a1ad1ae2bbf002a15414354ad50a6d7fbc8fa`
Reviewed at: `2026-05-20T22:10:18Z`
Reviewer: `claude-reviewer`

Validation:
- `cargo fmt --all -- --check` passed.
- `cargo clippy --workspace --all-targets -- -D warnings` passed.
- `cargo test --workspace` passed.

Findings:

1. F3 mirror never asks the daemon for mirror mode, so no purge list is produced.

   `spawn_f3_pull` builds the TUI mirror execution with `options: PullSyncOptions::default()` at `crates/blit-tui/src/main.rs:2881` through `crates/blit-tui/src/main.rs:2886`. The separate `PullSyncExecution.mirror_mode` field is passed to `RemotePullClient::pull_sync` as `track_paths` at `crates/blit-app/src/transfers/remote.rs:343` through `crates/blit-app/src/transfers/remote.rs:349`, but the wire `TransferOperationSpec` is built only from `execution.options` at `crates/blit-core/src/remote/pull.rs:598` through `crates/blit-core/src/remote/pull.rs:606`. Since `PullSyncOptions::default().mirror_mode` is false, `build_spec_from_options` emits `MirrorMode::Off` at `crates/blit-core/src/remote/pull.rs:558` through `crates/blit-core/src/remote/pull.rs:565`.

   Result: pressing `m`, entering a destination, and confirming with `y` performs the receive/copy part, but the daemon was not asked to compute mirror deletions. `apply_pull_mirror_purge(&outcome, true)` then has no `paths_to_delete` to apply, so local files absent from the remote source remain in place. The user-facing operation says "mirror" and shows a destructive confirmation, but behaves like a plain pull.

   The CLI path handles this correctly by setting both fields: `PullSyncOptions { mirror_mode, ... }` and `PullSyncExecution { mirror_mode, ... }` at `crates/blit-cli/src/transfers/remote.rs:369` through `crates/blit-cli/src/transfers/remote.rs:386`. Please make the TUI mirror path set `PullSyncOptions.mirror_mode = mirror` as well, and add regression coverage that the F3 mirror execution builds a mirror-enabled pull spec rather than only carrying the post-pull purge flag.
