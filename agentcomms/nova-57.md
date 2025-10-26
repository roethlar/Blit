## nova → Win Agent (ReFS Clone Fast-Path)

### Objective
Implement and validate a ReFS block-clone fast path so 4 GiB same-volume mirrors match Robocopy performance (see `logs/windows/bench_local_windows_4gb_20251025T235715Z.log`, unoptimized blit ≈0.374 s vs robocopy 0.155 s).

### Context
- Repo: `blit_v2`, Windows dev box (CopyFileEx path already in `crates/blit-core/src/copy/windows.rs`).
- ReFS benchmark harness: `scripts/windows/bench-local-mirror.ps1 -SizeMB 4096`.
- Capability scaffolding: `crates/blit-core/src/fs_capability/windows.rs`.
- Planner/copy modules: `crates/blit-core/src/copy/windows.rs`, `crates/blit-core/src/copy/mod.rs`, integration via orchestrator/planner.

Bench evidence: Windows ReFS log proves current CopyFileEx copies the full 4 GiB; robocopy likely uses `FSCTL_DUPLICATE_EXTENTS_TO_FILE` (ReFS block cloning) when source/dest share volume. Target: Detect clone support and fall back to CopyFileEx if unavailable.

### Deliverables
1. **Capability Detection**  
   - Extend `fs_capability::windows` to probe same-volume ReFS destinations for `FSCTL_DUPLICATE_EXTENTS_TO_FILE`.  
   - Add capability flag (e.g., `BlockCloneSameVolume`) stored in journal/capability cache.

2. **Clone Implementation**  
   - Add helper in `copy::windows` that, when capability true and source/dest on same volume, invokes `DeviceIoControl` with `FSCTL_DUPLICATE_EXTENTS_TO_FILE`.  
   - Handle privileges (adjust token if necessary) and fall back to CopyFileEx on failure.  
   - Emit tracing to confirm clone path was taken.

3. **Planner Integration**  
   - Update planner/orchestrator to prefer clone path for eligible ReFS tasks.  
   - Ensure clone path bypasses tar shards/buffered copy (operate per-file).

4. **Validation**  
   - Unit tests for capability detection logic.  
   - Manual benchmark: rerun `scripts/windows/bench-local-mirror.ps1 -SizeMB 4096` and capture new log (expect blit ~robocopy).  
   - Document outcome in `logs/windows/bench_local_windows_4gb_clone_<timestamp>.log` and note in DEVLOG/TODO.

5. **Docs/TODO**  
   - Update `TODO.md` item once gap closes.  
   - Capture design summary in DEVLOG and note approach in workflow doc.

### Notes
- Keep changes Windows-only under `#[cfg(windows)]`.  
- Guard clone path carefully; unexpected failures must fall back to CopyFileEx to preserve correctness.  
- Use existing `win_fs` helpers for volume checks; clone requires same volume.
