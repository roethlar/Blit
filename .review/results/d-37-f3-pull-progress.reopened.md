# d-37-f3-pull-progress reopened

Reviewed commit: `8fd9bffe09966207ed92ccbec1bebe9fa5c26c0b`
Reviewed at: `2026-05-20T16:01:45Z`
Reviewer: `reviewer`

Validation:

- `cargo fmt --all -- --check` passed.
- `cargo clippy --workspace --all-targets -- -D warnings` passed.
- `cargo test --workspace` passed.

Findings:

1. `crates/blit-tui/src/main.rs:2149`-`2157` adds both `ProgressEvent::Payload.bytes` and `ProgressEvent::FileComplete.bytes` into the live footer total. That is not a valid invariant for pull progress events. In the TCP data-plane receive path, `execute_receive_pipeline` emits both events for the same completed file: `report_payload(0, outcome.bytes_written)` and then `report_file_complete(..., outcome.bytes_written)` at `crates/blit-core/src/remote/transfer/pipeline.rs:234`-`236`. F3 uses `PullSyncOptions::default()` (`crates/blit-tui/src/main.rs:2174`), so that path is in scope whenever the pull negotiates the data plane. Result: the footer can show roughly double the bytes during a data-plane file transfer and then snap backward when `F3PullReply` applies the authoritative `outcome.report.bytes_transferred` at `crates/blit-tui/src/main.rs:2179`-`2185`. Please make the F3 accumulator match pull receive semantics, for example by counting bytes from `Payload` and file count from `FileComplete` without re-adding duplicate file bytes, while preserving direct-gRPC behavior where `finalize_active_file` emits `FileComplete` with `bytes = 0` (`crates/blit-core/src/remote/pull.rs:1206`-`1215`). Add a focused regression for the accumulator with a data-plane-style sequence `Payload { bytes: 1024 }` followed by `FileComplete { bytes: 1024 }` so it cannot report `2048`.
