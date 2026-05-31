# d-53-f3-batch-pull reopened

Reviewed commit: `7188da62427d661d9cb5271980318155a2fc9050`
Reviewed at: `2026-05-20T21:31:51Z`
Reviewer: `reviewer`

Validation:
- `cargo fmt --all -- --check` passed.
- `cargo clippy --workspace --all-targets -- -D warnings` passed.
- `cargo test --workspace` passed.

Findings:

1. Batch pull reuses single-source destination resolution, so multiple sources can target the same non-directory path.

   `P` captures the destination once at `crates/blit-tui/src/main.rs:3611` through `crates/blit-tui/src/main.rs:3618`, then `advance_batch_pull` reuses that raw string for each queued source at `crates/blit-tui/src/main.rs:2853` through `crates/blit-tui/src/main.rs:2860`. Each queued source goes through `F3PullState::launch`, which applies the single-source `resolve_destination` semantics at `crates/blit-tui/src/f3pull.rs:223` through `crates/blit-tui/src/f3pull.rs:247`: if the destination does not already exist as a directory and has no trailing slash, it is treated as an exact target/rename.

   That is correct for single `p`, but unsafe for batch `P`. Example: mark `a.txt` and `b.txt`, press `P`, and enter `/tmp/out` where `/tmp/out` does not exist. The first pull resolves to `/tmp/out`; after it completes, the next queued pull reuses `/tmp/out` and can resolve to the same file target, overwriting/failing late instead of putting both files under a container. For a multi-source operation, the destination needs to be a directory/container contract.

   Please make batch pull enforce container semantics before launching the first transfer. Reasonable fixes: require the entered destination to be an existing directory or explicit trailing slash, or resolve a non-existing batch destination as a directory and append every source basename under it. Add regression coverage for at least two marked file sources with a non-existing destination lacking a slash, proving they cannot both resolve to the same target path.
