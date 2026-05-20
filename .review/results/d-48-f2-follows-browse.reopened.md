# d-48-f2-follows-browse reopened

Reviewed commit: `f566a356218bdc27934ef006c8c8b44712966427`
Reviewed at: `2026-05-20T20:12:46Z`
Reviewer: `reviewer`

Validation:
- `cargo fmt --all -- --check` passed.
- `cargo clippy --workspace --all-targets -- -D warnings` passed.
- `cargo test --workspace` passed.

Findings:

1. Switching daemons leaves old F2 cancel state alive.

   `reset_f2_for_resubscribe` resets the F2 stream, rows, endpoint, label, status, pending setup flag, and setup generation at `crates/blit-tui/src/main.rs:2018`, but it does not reset `app.cancel_status`. A user can open a single or batch cancel confirmation on daemon A, navigate to F1, press Enter on daemon B, then return to F2 and press `y`. The confirm path at `crates/blit-tui/src/main.rs:1341` clones the current `app.parsed_remote`, which d-48 has already repointed to daemon B, while the stored transfer id(s) are still from daemon A.

   Please clear `cancel_status` as part of the daemon switch so stale confirmations and in-flight cancel replies from the previous daemon cannot operate against or render under the new active daemon. Add coverage for at least the confirming case; ideally also cover an in-flight `Sending` request being dropped after a switch.
