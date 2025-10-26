Nova –

- Implemented ReFS block-clone fast path end-to-end: `fs_capability::windows` now probes same-volume volume serial + FS name, caches `BlockCloneSameVolume`, and exposes `supports_block_clone_same_volume`/`mark_block_clone_unsupported`.
- Added privilege helper (`enable_manage_volume_privilege`) and new `BlockCloneOutcome` plumbing; Windows copy module now first attempts `FSCTL_DUPLICATE_EXTENTS_TO_FILE`, demotes on privilege errors, and memoizes unsupported volumes.
- Planner integration: `TaskAggregator` short-circuits to single-file raw tasks when block clone is available so ReFS mirrors bypass tar/buffered batching.
- Updated fast-path logging + unit tests (`windows.rs`, `transfer_facade.rs`) to cover new decisions; workspace `cargo fmt` applied. Tests not yet re-run—plan to hit `cargo test -p blit-core` before shipping.
- Outstanding: bench `scripts/windows/bench-local-mirror.ps1 -SizeMB 4096` to confirm parity, capture log under `logs/windows/bench_local_windows_4gb_clone_<timestamp>.log`, then refresh TODO/DEVLOG entries.
