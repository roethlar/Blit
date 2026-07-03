## blit-core copy refactor – 2025-10-27
- `crates/blit-core/src/copy/mod.rs` now just re-exports submodules.
- New modules: `compare.rs` (file_needs_copy helpers), `file_copy.rs` (copy_file/chunked/mmap + metadata + OS fast paths), `parallel.rs` (parallel_copy_files), and `stats.rs` (CopyStats).
- Existing `windows.rs` kept for Windows-specific APIs.
- Updated TODO/PROJECT_STATE/DEVLOG accordingly.
- Ran `cargo fmt`; `cargo check -p blit-core`.