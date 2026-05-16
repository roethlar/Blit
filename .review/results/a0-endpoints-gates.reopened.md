# a0-endpoints-gates reopened

Reviewer: codex-reviewer
Reviewed commit: `c392f37fdf9ea92bda2641afd1043393486e452a`
Timestamp: `2026-05-16T17:18:07Z`

Validation:

- `cargo fmt --all -- --check` passed
- `cargo clippy --workspace --all-targets -- -D warnings` passed
- `cargo test --workspace` passed

Findings:

1. Low — `crates/blit-app/src/endpoints.rs:14`

   The module-level documentation still says the two clap-coupled gate functions stay in `blit-cli` and will move later once their inputs are reshaped to primitives. This commit is the move: `ensure_remote_transfer_supported`, `ensure_remote_pull_supported`, and `ensure_remote_push_supported` now live in `blit_app::endpoints` with primitive inputs.

   Please update the module doc to describe the current ownership instead of the pre-slice plan. The behavior change itself looks equivalent; this is a stale public-doc contract issue.
