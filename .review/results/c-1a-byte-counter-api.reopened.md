# c-1a-byte-counter-api reopened

Verdict: Reopened
Reviewed sha: `ff36a8e78b7394625d5e8dacb191d3a758d4d930`
Reviewer: `reviewer`
Timestamp: `2026-05-16T23:39:06Z`

Validation:

- `cargo fmt --all -- --check`: pass
- `cargo clippy --workspace --all-targets -- -D warnings`: pass
- `cargo test --workspace`: pass

## Findings

1. Medium - `ByteProgressSink` is on the wrong side of the crate boundary for the advertised c-1b API.

   The slice documents `ByteProgressSink` as the public type that the core data-plane receive loop will accept next: `crates/blit-daemon/src/active_jobs.rs:45` says the sink is public so `blit-core` can take it as a parameter, and `crates/blit-daemon/src/active_jobs.rs:62` says c-1b will add an optional `&ByteProgressSink` to `receive_stream_double_buffered`.

   That cannot compile with the current dependency direction. `receive_stream_double_buffered` lives in `crates/blit-core/src/remote/transfer/data_plane.rs:532`, while `crates/blit-daemon/Cargo.toml:10` has `blit-daemon` depending on `blit-core`. `blit-core` cannot name a type defined in `blit-daemon` without introducing a dependency cycle.

   Fix the API boundary before c-1b depends on it. Reasonable options are to define a small progress reporter trait/callback in `blit-core` and have the daemon pass an implementation, or move the shared sink/trait shape into `blit-core`. The finding doc and module docs should also stop promising that `blit-core` will directly accept `blit_daemon::active_jobs::ByteProgressSink`.
