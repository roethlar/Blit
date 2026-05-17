# c-1b-byte-counter-wiring reopened

Verdict: Reopened
Reviewed sha: `8ff9ebabe86e9730b2c2085795694a747019df7f`
Reviewer: `reviewer`
Timestamp: `2026-05-17T00:33:56Z`

Validation:

- `cargo fmt --all -- --check`: pass
- `cargo clippy --workspace --all-targets -- -D warnings`: pass
- `cargo test --workspace`: pass

## Findings

1. Medium - Byte progress is only wired for `DATA_PLANE_RECORD_FILE`; delegated-pull tar-shard and resume-block bytes still never reach `ActiveJobs`.

   `delegated_pull` advertises both tar shards and resume support to the source (`crates/blit-daemon/src/service/delegated_pull.rs:35`), so the source is allowed to send data-plane records other than plain files. The new progress hook is attached only through `FsTransferSink::write_file_stream` (`crates/blit-core/src/remote/transfer/sink.rs:362`), which is called from the `DATA_PLANE_RECORD_FILE` branch (`crates/blit-core/src/remote/transfer/pipeline.rs:217`).

   Tar shards and resume blocks bypass that path. `execute_receive_pipeline` handles `DATA_PLANE_RECORD_TAR_SHARD` by reading the shard, calling `sink.write_payload(...)`, and merging the outcome (`crates/blit-core/src/remote/transfer/pipeline.rs:240`), and handles `DATA_PLANE_RECORD_BLOCK` the same way (`crates/blit-core/src/remote/transfer/pipeline.rs:253`). Inside `FsTransferSink::write_payload`, tar shards and blocks return positive `bytes_written` (`crates/blit-core/src/remote/transfer/sink.rs:258`, `crates/blit-core/src/remote/transfer/sink.rs:619`, `crates/blit-core/src/remote/transfer/sink.rs:661`) but never call `ByteProgressSink::report`.

   Result: a delegated pull dominated by tar shards, which is the normal many-small-files path, can complete with `GetState.active[].bytes_completed` and `GetState.recent[].bytes` undercounted or still zero despite bytes being written. Resume/block transfers have the same issue. Either thread byte progress through the receive pipeline for every record type, or have `FsTransferSink` report `outcome.bytes_written` for `write_payload` paths when `byte_progress` is set and the write is real. Add regression coverage for at least tar-shard receive, and preferably block receive too.
