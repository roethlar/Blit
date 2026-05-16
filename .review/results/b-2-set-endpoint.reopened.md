# b-2-set-endpoint reopened

Reviewer: codex-reviewer
Reviewed commit: `c874ef6fd2fec7ffecb680218bd209fe5e164e89`
Timestamp: `2026-05-16T20:37:00Z`

Validation:

- `cargo fmt --all -- --check` passed
- `cargo clippy --workspace --all-targets -- -D warnings` passed
- `cargo test --workspace` passed

Findings:

1. Low — `crates/blit-daemon/src/service/core.rs:286`

   The `delegated_pull` ActiveJobs comment still says streaming RPCs
   "`b-2 will wire via a guard update API`." This commit is b-2 and now
   wires `push` / `pull_sync` through `ActiveJobGuard::set_endpoint`, so
   the comment is stale immediately at HEAD. Please rewrite it in present
   tense so future readers do not infer that streaming RPC registration is
   still missing.
