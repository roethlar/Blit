# d-35-f3-pull-execute reopened

Reviewed commit: `eab7b367e0b242c16e35aceff59f2410d8eca70b`
Reviewed at: `2026-05-19T22:34:37Z`
Reviewer: `reviewer`

Validation:

- `cargo fmt --all -- --check` passed.
- `cargo clippy --workspace --all-targets -- -D warnings` passed.
- `cargo test --workspace` passed.

Findings:

1. `crates/blit-tui/src/f3pull.rs:175` turns the typed destination into `PathBuf::from(dest.trim())` and `crates/blit-tui/src/main.rs:1995`-`2003` passes that path directly as `PullSyncExecution.dest_root`. That skips the same `resolve_destination(...)` step the CLI applies before `run_remote_pull_transfer` (`crates/blit-cli/src/transfers/mod.rs:101`-`105`, then `:222`-`:228`). `run_pull_sync` treats `dest_root` as already resolved: `crates/blit-app/src/transfers/remote.rs:339`-`346` forwards it unchanged, and `crates/blit-core/src/remote/pull.rs:246`-`250` documents that a single-file pull expects `dest_root` to be the final file path. The low-level resolver also writes an empty wire relative path directly to `dest_root` (`crates/blit-core/src/remote/pull.rs:1718`-`1725`). Result: selecting a single file in F3 and entering an existing local directory can try to create the directory itself as the output file, while selecting a directory and entering an existing local directory merges its contents instead of nesting under the selected directory basename. The expected remote-pull semantics are already pinned by `crates/blit-cli/tests/remote_pull_subpath.rs:50`-`68` and `:86`-`:105`. Please resolve the F3 destination through the same `blit_app::transfers::resolution::resolve_destination` contract, or explicitly redefine and test different TUI semantics.
