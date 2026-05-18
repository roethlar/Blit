# d-4-f4-local-transfers: reopened

Reviewed sha: `faec1e3568ac8a6a3a695c6a8ffd4e1b7ccdad84`

Verdict: reopened.

## Findings

1. High - `M` starts a destructive mirror without confirmation.

   The TUI dispatches `UserAction::TransferMirror` directly into
   `spawn_local_transfer` and sets `LocalMirrorOptions { mirror: true, .. }`
   (`crates/blit-tui/src/main.rs:787`, `crates/blit-tui/src/main.rs:1305`).
   The existing CLI explicitly prompts before mirror unless `--yes` or
   `--dry-run`, because mirror can delete extraneous destination files
   (`crates/blit-cli/src/transfers/mod.rs:183`). Making this a single
   uppercase key from F4 is not acceptable as a known gap; the first TUI
   surface that exposes mirror needs the same destructive-operation guard
   or it needs to omit the mirror trigger until the confirmation path exists.

2. High - TUI transfers skip destination resolution, so they do not match
   CLI transfer semantics.

   `spawn_local_transfer` converts the form strings directly with
   `PathBuf::from(&source)` and `PathBuf::from(&destination)`, then calls
   `blit_app::transfers::local::run` (`crates/blit-tui/src/main.rs:1309`).
   The CLI resolves the raw destination before dispatching a local transfer
   (`crates/blit-cli/src/transfers/mod.rs:105`), and the core transfer engine
   documents that the caller must already have produced the exact target path
   for single-file copies, including trailing-slash and existing-directory
   semantics (`crates/blit-core/src/orchestrator/orchestrator.rs:172`,
   `crates/blit-core/src/orchestrator/orchestrator.rs:1134`).

   Concrete regressions: `source=file.txt, destination=existing_dir` should
   copy to `existing_dir/file.txt`, while this path passes `existing_dir` as
   the final file target. Directory sources also lose the CLI's basename
   append behavior for container destinations. The finding doc says this uses
   the same path as `blit copy` / `blit mirror` verbatim, but the resolver is
   part of that CLI path and is missing here.

3. Medium - F4 local transfers ignore the perf-history enabled setting.

   The TUI uses `LocalMirrorOptions::default()` for every local transfer
   (`crates/blit-tui/src/main.rs:1305`), whose default has
   `perf_history: true` (`crates/blit-core/src/orchestrator/options.rs:90`).
   The CLI instead wires `perf_history: ctx.perf_history_enabled`
   (`crates/blit-cli/src/transfers/local.rs:184`). Since F4 already exposes
   profile/perf lifecycle controls, a disabled perf-history setting should not
   be bypassed by transfers launched from the same screen.

## Validation

Run in detached worktree at the reviewed SHA:

- `cargo fmt --all -- --check` passed.
- `cargo clippy --workspace --all-targets -- -D warnings` passed.
- `cargo test -p blit-tui` passed: 129 tests.
- `cargo test --workspace` passed.
