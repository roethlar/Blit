# Code Inventory: blit-core copy/delete/buffer/tar/checksum/manifest

**Generated**: 2026-06-04 by audit workflow
**Cluster**: blit-core local I/O — block-level copy with resume, BufferPool lifecycle, tar streaming (local), checksum/Blake3, manifest format, local delete semantics + symlink handling.

## Coverage

| File | Lines | Notes |
|------|-------|-------|
| `crates/blit-core/src/copy/mod.rs` | 16 | Re-exports only |
| `crates/blit-core/src/copy/compare.rs` | 126 | 3 needs-copy predicates + per-mode dispatcher |
| `crates/blit-core/src/copy/parallel.rs` | 51 | Rayon parallel copy_file fan-out |
| `crates/blit-core/src/copy/stats.rs` | 17 | Simple `CopyStats` struct |
| `crates/blit-core/src/copy/windows.rs` | 432 | Windows CopyFileExW, FSCTL_DUPLICATE_EXTENTS_TO_FILE block clone, NO_BUFFERING heuristics |
| `crates/blit-core/src/copy/file_copy/mod.rs` | 295 | Cross-platform copy_file dispatcher; fast-path → fallback chain |
| `crates/blit-core/src/copy/file_copy/chunked.rs` | 68 | Single-buffer streaming copy |
| `crates/blit-core/src/copy/file_copy/clone.rs` | 283 | clonefile/fcopyfile/copy_file_range/sendfile/SEEK_HOLE primitives |
| `crates/blit-core/src/copy/file_copy/metadata.rs` | 17 | Thin wrapper around `fs_capability::preserve_metadata` |
| `crates/blit-core/src/copy/file_copy/mmap.rs` | 89 | "mmap" misnomer — actually Linux copy_file_range + sendfile, std::fs::copy elsewhere |
| `crates/blit-core/src/copy/file_copy/resume.rs` | 283 | Block-level resume with Blake3 |
| `crates/blit-core/src/delete.rs` | 93 | Mirror-mode delete plan computation |
| `crates/blit-core/src/buffer.rs` | 619 | BufferSizer + BufferPool with semaphore budget |
| `crates/blit-core/src/tar_stream.rs` | 414 | Tar pack+unpack via crossbeam channel (local-only) |
| `crates/blit-core/src/checksum.rs` | 260 | Rolling/strong/file/partial hashes |
| `crates/blit-core/src/manifest.rs` | 470 | FileHeader manifest diff for push/pull |

**Total lines read**: 3,533

## Behaviors (grouped by category)

### path-handling
- **tar-sanitize-rel-path** — `crates/blit-core/src/tar_stream.rs:43-59` — `sanitize_rel_path` rejects absolute paths and any `ParentDir`/`RootDir`/`Prefix` component when building tar entry names. _(notes: hardened against `../` injection; used by `tar_stream_transfer_list_cb`)_
- **tar-sanitize-rel-path-inline-duplicate** — `crates/blit-core/src/tar_stream.rs:204-224` — `tar_stream_transfer_cb` inlines the same sanitization logic instead of calling `sanitize_rel_path`. _(notes: duplicated pattern; if the helper is ever tightened the inline copy will drift)_
- **resume-create-parent-dir** — `crates/blit-core/src/copy/file_copy/resume.rs:60-64` — `resume_copy_file` calls `create_dir_all` on the destination's parent before opening. _(notes: silent — no log on creation)_
- **copy-file-parent-create** — `crates/blit-core/src/copy/file_copy/mod.rs:76-83` — `copy_file` calls `create_dir_all(parent)` TWICE: once via `if let Some(parent) = dst.parent()` then again via `ok_or_else`. _(notes: redundant — both branches do the same work; dead duplicate)_
- **delete-plan-strip-prefix** — `crates/blit-core/src/delete.rs:51-54` — `generate_delete_plan` skips source entries whose `strip_prefix(source_root)` produces an empty relative path (i.e. the root itself). _(notes: silently skips entries with malformed prefixes — no warning)_

### safety-check
- **resume-block-size-clamp** — `crates/blit-core/src/copy/file_copy/resume.rs:77-81` — `block_size == 0` falls back to `DEFAULT_BLOCK_SIZE` (1 MiB); any larger value is clamped to `MAX_BLOCK_SIZE` (64 MiB) to prevent huge buffer allocations. _(notes: silent clamp — caller cannot tell what was used)_
- **resume-truncate-longer-dest** — `crates/blit-core/src/copy/file_copy/resume.rs:146-148` — When destination is longer than source, calls `set_len(src_len)` to truncate after the comparison loop. _(notes: silently shortens dest; correct for resume semantics)_
- **resume-sync-all** — `crates/blit-core/src/copy/file_copy/resume.rs:150-151` — `sync_all()` (fsync) the destination before returning. _(notes: only the resume path syncs explicitly; `copy_file`/`chunked_copy_file` rely on writer flush only)_
- **windows-no-buffering-small-file-floor** — `crates/blit-core/src/copy/windows.rs:80-86` — Files ≤ `WINDOWS_NO_BUFFERING_SMALL_FILE_MAX` (512 MiB) always use the cached path; never apply NO_BUFFERING. _(notes: hard threshold)_
- **windows-no-buffering-memory-pressure** — `crates/blit-core/src/copy/windows.rs:97-130` — Triggers `COPY_FILE_NO_BUFFERING` (0x1000) when `file_size + 512MiB > avail_phys` OR file ≥ min(`WINDOWS_NO_BUFFERING_FLOOR` 2GiB, half of total RAM). _(notes: complex heuristic; tests at lines 386-431)_
- **windows-block-clone-zero-length-fast-path** — `crates/blit-core/src/copy/windows.rs:283-289` — `try_block_clone_same_volume` short-circuits zero-length files as "cloned" without calling `DeviceIoControl`. _(notes: edge case; sets `last_block_clone_success = false` first then returns 0)_
- **windows-block-clone-thread-local-flag** — `crates/blit-core/src/copy/windows.rs:28-42` — Tracks last block-clone success in a thread-local `Cell<bool>`, consumed by `take_last_block_clone_success`. _(notes: thread-local — if work crosses thread boundaries between `windows_copyfile` and the take, the flag is lost)_
- **clone-attempt-clonefile-macos-eexist** — `crates/blit-core/src/copy/file_copy/clone.rs:10-17`, `mod.rs:106-114` — macOS `clonefile(2)` requires destination NOT exist; `copy_file` defers `File::create(dst)` until streaming fallback so clone can succeed. _(notes: explicit fix for pre-fix EEXIST bug — R58-F11 — annotated in source)_

### state-machine
- **copy-file-fallback-chain-windows** — `crates/blit-core/src/copy/file_copy/mod.rs:119-160` — Windows fast-path order: block-clone → `sparse_copy_windows` (streaming with hole detection). _(notes: 4-arm match on BlockCloneOutcome)_
- **copy-file-fallback-chain-macos** — `crates/blit-core/src/copy/file_copy/mod.rs:161-181` — macOS order: `clonefile` → `fcopyfile` → BufReader/BufWriter streaming. _(notes: streaming path opens dst itself so clonefile EEXIST is avoided)_
- **copy-file-fallback-chain-linux** — `crates/blit-core/src/copy/file_copy/mod.rs:182-202` — Linux order: `copy_file_range` → `sendfile` → `attempt_sparse_copy_unix` → BufReader/BufWriter. _(notes: 4 fast paths)_
- **windows-copyfile-fallback-to-fs-copy** — `crates/blit-core/src/copy/windows.rs:362-368` — When `CopyFileExW` fails, falls back to `std::fs::copy(src, dst)`. _(notes: silently retries; the original `CopyFileExW` error is dropped — `last_os_error` not logged)_
- **copy-file-fallback-clone-success-false-on-streaming-tail** — `crates/blit-core/src/copy/file_copy/mod.rs:204-211` — When the fast-path chain falls through to streaming, `metadata::preserve_metadata` is called BUT clone_succeeded = false; this is the only path that needs manual metadata copying. _(notes: subtle invariant — fast paths preserve metadata internally, streaming does not)_

### timeout-or-retry
- **tar-channel-send-timeout-default** — `crates/blit-core/src/tar_stream.rs:33-41` — `TarConfig::default()` sets `send_timeout_ms = Some(30_000)` (30s) for the ChannelWriter drop-flush. _(notes: hardcoded 30s default; otherwise the drop-flush would block indefinitely on a stalled consumer)_
- **clone-copy-file-range-retry** — `crates/blit-core/src/copy/file_copy/clone.rs:62-72` — Linux `copy_file_range` retries on EINTR/EAGAIN; bails (returns Ok(false)) on EXDEV/EINVAL. _(notes: typical syscall loop; doesn't bound retries)_
- **clone-sendfile-retry** — `crates/blit-core/src/copy/file_copy/clone.rs:89-104` — Linux `sendfile` retries on EINTR/EAGAIN; bails on EINVAL/ENOSYS. _(notes: same pattern)_
- **mmap-copy-file-range-loop-no-iteration-cap** — `crates/blit-core/src/copy/file_copy/mmap.rs:26-55` — `copy_file_range` loop continues on EINTR/EAGAIN with no iteration cap or progress check beyond `to_copy == 0`. _(notes: could in theory spin under repeated EAGAIN — though copy_file_range generally won't return EAGAIN on regular files)_

### error-propagation
- **windows-copyfile-error-swallowed** — `crates/blit-core/src/copy/windows.rs:362-368` — `CopyFileExW` failure is converted to bool (`is_ok`) so the original Win32 error code is discarded; only the fs::copy fallback's error gets surfaced via `.context(...)`. _(notes: smell — lossy error path)_
- **chunked-windows-fallback-error-swallowed** — `crates/blit-core/src/copy/file_copy/chunked.rs:23-30` — `if let Ok(bytes) = windows::windows_copyfile(src, dst)` — error from CopyFileExW path is silently dropped; no log on fallback. _(notes: smell — `if let Ok` swallows root cause)_
- **windows-block-clone-failed-returns-ok-none** — `crates/blit-core/src/copy/windows.rs:323-328` — `BlockCloneOutcome::Failed(err)` logs at debug and returns `Ok(None)` so caller falls back; the underlying io::Error is logged but not propagated. _(notes: intentional — block-clone is an optimization; correct policy but reduces observability)_
- **windows-blockclone-handle-fallthrough** — `crates/blit-core/src/copy/file_copy/mod.rs:124-148` — When block clone outcome is Failed, only `log::debug!` runs — no metric or error counter. _(notes: subtle — silent degradation)_
- **parallel-copy-error-collect** — `crates/blit-core/src/copy/parallel.rs:38-43` — `parallel_copy_files` collects per-file errors into a Vec<String> on a Mutex but never aborts — every file is attempted regardless of how many fail. _(notes: no fail-fast option; no early cancellation)_
- **compare-mode-context-chain** — `crates/blit-core/src/copy/compare.rs:95-126` — `file_needs_copy_with_mode` wraps metadata calls in `.context(...)` for each branch, giving good diagnostics. _(notes: contrast with `file_needs_copy` 8-71 which has unwrap-or-defaults for mtime)_

### default-value
- **default-block-size-resume** — `crates/blit-core/src/copy/file_copy/resume.rs:16-19` — `DEFAULT_BLOCK_SIZE = 1 MiB`, `MAX_BLOCK_SIZE = 64 MiB` exported constants. _(notes: documented in module header)_
- **buffer-sizer-magic-numbers** — `crates/blit-core/src/buffer.rs:25-31` — `max_buffer_size = 16 MiB`, `min_buffer_size = 1 MiB` are hardcoded in `BufferSizer::new()`. _(notes: not configurable from the outside)_
- **buffer-sizer-network-base-2x** — `crates/blit-core/src/buffer.rs:57-58` — Network base buffer is 8 MiB vs 4 MiB local; hardcoded ratio. _(notes: magic numbers; no config)_
- **buffer-sizer-fallback-512mib** — `crates/blit-core/src/buffer.rs:42-47` — When sysinfo reports 0 available memory, falls back to 512 MiB (comment notes "safer than 4 GiB on low-memory systems"). _(notes: deliberate change documented inline)_
- **buffer-sizer-memory-cap-10-percent** — `crates/blit-core/src/buffer.rs:83-87` — Final buffer size capped at 10% of available memory, with absolute floor of 8 KiB. _(notes: 10% is hardcoded)_
- **buffer-sizer-scaling-curve-magic** — `crates/blit-core/src/buffer.rs:60-81` — Three-tier file-size scaling: <10 MiB → min, 10-100 MiB → base, 100 MiB-1 GiB → linear ramp to max over 900 MiB span. _(notes: all thresholds hardcoded; no test names them)_
- **tar-chunk-size-default** — `crates/blit-core/src/tar_stream.rs:33-41` — `TarConfig::default` sets `channel_capacity = 64`, `chunk_size = 1 MiB`, `send_timeout_ms = Some(30_000)`. _(notes: chunk_size is 1 MiB; not tied to BufferSizer)_
- **tar-dynamic-channel-buffer** — `crates/blit-core/src/tar_stream.rs:173-180` — Channel buffer auto-scaled by file count: 16/32/64/128 for <=100/<=1000/<=10000/more, capped by `config.channel_capacity * 2`. _(notes: scaling table duplicated for `tar_stream_transfer_list_cb` at lines 294-300)_
- **chunked-1gib-cliff** — `crates/blit-core/src/copy/file_copy/chunked.rs:35-39` — For files >1 GiB, ignores buffer_sizer and uses fixed 16 MiB chunk; otherwise defers to BufferSizer. _(notes: hardcoded cutoff; possibly inconsistent with copy_file path which trusts BufferSizer)_
- **manifest-comparemode-default-trait** — `crates/blit-core/src/manifest.rs:10-24` — `CompareMode` derives `Default` and the default is `Default` (size+mtime, skip if target newer). _(notes: name collision with derive — readable code but slightly confusing)_

### persistence
- **buffer-pool-restore-len-no-zero** — `crates/blit-core/src/buffer.rs:183-207` — Returned buffers are restored to `buffer_size` length via `truncate` (common path, no zeroing) or `resize` (rare grow path, zeroes only delta). Stale tail bytes are deliberately left in place — callers consume `[..bytes_read]`. _(notes: audit-13 fix documented inline; explicit "may be dirty" contract)_
- **buffer-pool-semaphore-permit-leak-fix** — `crates/blit-core/src/buffer.rs:239-284` and `288-325` — Owned permit is kept on the local stack until after `vec![0u8; self.buffer_size]` so that if allocation panics, unwind drops the permit. `mem::forget` only after successful allocation. _(notes: audit-12 fix documented inline; subtle invariant)_
- **buffer-pool-take-releases-permit** — `crates/blit-core/src/buffer.rs:428-435` — `PoolBuffer::take()` adds a permit back to the semaphore and decrements `in_use` since the buffer is removed from pool management. _(notes: contract with `return_vec` — must call return_vec if you want the buffer cached back)_
- **buffer-pool-return-vec-no-permit** — `crates/blit-core/src/buffer.rs:355-358` — `return_vec` caches the buffer but does NOT add a permit back; assumes the caller obtained the buffer via `take()` (which already added the permit). _(notes: subtle — undocumented invariant)_

### discovery
- **delete-plan-mark-parent-dirs** — `crates/blit-core/src/delete.rs:14-23` — For each expected destination path, climbs parents until reaching `dest_root` and marks them as expected so they survive the delete pass. _(notes: prevents deleting intermediate directories needed for retained files)_
- **delete-plan-deep-first-sort** — `crates/blit-core/src/delete.rs:81-83` — Sorts `dirs_to_delete` by component count then reverses — deepest first so empty parents are removed before their ancestors. _(notes: relies on caller actually deleting in returned order)_
- **delete-plan-no-symlink-handling** — `crates/blit-core/src/delete.rs:40-88` — Symlinks are not classified specially; whatever `enumerate_directory_filtered` returns determines is_directory vs file. _(notes: no follow vs. delete-target distinction visible here — must inspect fs_enum)_
- **tar-walkdir-no-filter** — `crates/blit-core/src/tar_stream.rs:160-170` — `tar_stream_transfer_cb` walks source with `WalkDir::new(source)`; no symlink follow control, no exclusion filter. _(notes: copies whatever WalkDir gives by default — follows symlinks unless caller pre-resolves)_

### format-output
- **manifest-source-target-mtime-compare** — `crates/blit-core/src/manifest.rs:182-193` — Default mode: source > target_mtime → Modified; else Unchanged. No tolerance — even 1-second skew triggers transfer. _(notes: contrast with `compare.rs:30-33` which uses 2s tolerance for FAT/exFAT)_
- **manifest-force-mode-always-modified** — `crates/blit-core/src/manifest.rs:164-172` — Force mode always returns `FileStatus::Modified` regardless of target state. R58-F9 fix documented inline. _(notes: behavior must match diff_planner — explicit cross-reference)_
- **manifest-ignore-existing-status** — `crates/blit-core/src/manifest.rs:111-114` — When ignore_existing is true and target has the file, status is `SkippedExisting` and the file is NOT added to `files_to_transfer`. _(notes: but `files_to_delete` for mirror mode is computed independently — could be inconsistent)_
- **manifest-checksum-mode-fallback-transfers** — `crates/blit-core/src/manifest.rs:194-209` — Checksum mode: if either side has empty checksum, transfers anyway (for verification). _(notes: documented — happens when server checksums disabled)_

### config-load
- **buffer-sizer-cached-memory-once** — `crates/blit-core/src/buffer.rs:21,53-55` — Available memory is captured once via `OnceCell` on first `calculate_buffer_size` call. _(notes: doesn't refresh during process lifetime; memory pressure increases won't shrink buffers)_

### naming
- **mmap-misnomer** — `crates/blit-core/src/copy/file_copy/mmap.rs:1-89` — Despite the name `mmap_copy_file`, this function does NOT use memory mapping. It uses Linux `copy_file_range`/`sendfile` syscalls, then falls back to `std::fs::copy`. _(notes: confusing name — leftover from previous implementation? Also pub from copy/mod.rs as `mmap_copy_file`)_
- **resume-vs-blockcopy-naming** — `crates/blit-core/src/copy/file_copy/resume.rs` — File is called `resume.rs` but the function `resume_copy_file` is actually block-level rsync-style delta copy via Blake3 hash comparison. _(notes: dual purpose — works for resume but also for in-place delta updates)_

### data-plane
- **resume-block-hash-compare** — `crates/blit-core/src/copy/file_copy/resume.rs:97-143` — Iterates source block-by-block, computes Blake3 of source block and (if available) destination block; transfers only when hashes differ or destination is shorter. _(notes: tracks `dst_cursor_pos` to avoid redundant seeks — audit-14 optimization)_
- **resume-partial-block-always-write** — `crates/blit-core/src/copy/file_copy/resume.rs:106-126` — A partial trailing block in destination is always rewritten (no partial-block hash optimization). _(notes: correct but a missed optimization opportunity)_
- **tar-two-thread-channel** — `crates/blit-core/src/tar_stream.rs:192-267` — Producer thread writes tar to `ChannelWriter` → bounded channel → consumer thread runs `Archive::unpack`. Both threads joined; panics surfaced as errors. _(notes: real "tar stream" is local-to-local pipe — name implies network but this module is purely on-host)_
- **tar-unpack-xattrs-off-permissions-on** — `crates/blit-core/src/tar_stream.rs:252-256, 348-350` — Both unpack paths set `set_unpack_xattrs(false)` and `set_preserve_permissions(true)`. _(notes: xattrs deliberately dropped; ACLs/quarantine attrs lost)_
- **sparse-copy-windows-zero-elision-threshold** — `crates/blit-core/src/copy/file_copy/clone.rs:230-282` — Runs of all-zero bytes ≥ 256 KiB are skipped via `seek(SeekFrom::Current(...))` to create holes; smaller runs are written explicitly with a 64 KiB zero buffer. _(notes: pre-set `mark_file_sparse(dst)` — best-effort)_
- **sparse-copy-unix-seek-data-hole** — `crates/blit-core/src/copy/file_copy/clone.rs:108-188` — Uses `lseek(SEEK_DATA)`/`SEEK_HOLE` to skip holes; returns `Ok(None)` when filesystem doesn't support these (ENXIO/EINVAL on first probe). _(notes: graceful — fast path with fallback)_

### flag-handling
- **windows-copyfile-no-buffering-flag** — `crates/blit-core/src/copy/windows.rs:24, 348-351` — `COPY_FILE_NO_BUFFERING_FLAG = 0x1000` is hardcoded (per docs); enables unbuffered IO for large files. _(notes: magic constant per Microsoft docs; cannot use windows-rs constant because it's not exported)_
- **compare-mode-orthogonal-ignore-existing** — `crates/blit-core/src/copy/compare.rs:93-94` — Documented that `ignore_existing` is orthogonal and MUST be handled by caller before invoking; the function itself doesn't accept that flag. _(notes: invariant on the caller — easy to forget)_

### rpc-handler
- (none — this cluster has no RPC handlers; all functions are direct calls)

### cancellation
- (none observed — copy primitives have no cancellation token; rely on process kill for stop)

### endpoint-parse
- (n/a — local-only cluster)

### render-or-display
- (n/a — no UI in this cluster)

### key-dispatch
- (n/a)

### spawn-task
- **parallel-copy-rayon-fanout** — `crates/blit-core/src/copy/parallel.rs:30-43` — Uses `pairs.par_iter()` to fan-out file copies via rayon's global thread pool. _(notes: no concurrency cap exposed — depends on rayon's default `RAYON_NUM_THREADS`)_
- **tar-thread-spawn-bare** — `crates/blit-core/src/tar_stream.rs:193, 249, 313, 345` — Uses `thread::spawn` directly (not a thread pool); two threads per tar transfer. _(notes: panics caught via `join().map_err(...)` then surfaced via `eyre!`)_

### confirmation-prompt
- (n/a — no interactive prompts in core)

## Smells / risks observed

1. **`mmap_copy_file` misnomer** — `crates/blit-core/src/copy/file_copy/mmap.rs:9-83` — function name suggests memory-mapped I/O but implementation is `copy_file_range`/`sendfile`/`fs::copy`. Confusing for readers; also re-exported as `mmap_copy_file` from the public API.

2. **`copy/file_copy/mod.rs:76-83`** — `create_dir_all(parent)` is called twice in succession (once via `if let Some(parent)`, once via `ok_or_else`). Second call is identical to the first; safe but dead.

3. **Error path swallows root cause** — `chunked.rs:25-30` uses `if let Ok(bytes) = windows::windows_copyfile(...)` and silently falls through on error; `windows.rs:362-368` converts CopyFileExW error to bool. Operators lose the original Win32 error code.

4. **Sanitize path logic duplicated** — `tar_stream.rs:43-59` defines `sanitize_rel_path` but `tar_stream_transfer_cb` (lines 204-224) inlines a near-identical loop instead of calling it. If one is tightened, the other will drift.

5. **Tar dynamic-channel-buffer table duplicated** — `tar_stream.rs:174-180` and `294-300` have the same 16/32/64/128 lookup table.

6. **`copy/compare.rs` has 3 nearly-identical needs-copy predicates** — `file_needs_copy`, `file_needs_copy_with_checksum_type`, `file_needs_copy_with_mode`. The first two use 2s mtime tolerance; the third (for `SizeMtime`) delegates to the second so it inherits the tolerance — but `manifest.rs::compare_file` Default mode has no tolerance at all. Within-cluster inconsistency.

7. **Tar walks symlinks via WalkDir defaults** — `tar_stream.rs:165` uses `WalkDir::new(source)` without configuring follow_links; default is no follow but caller cannot override. Combined with `is_file()` check at 166 this silently drops symlinks (and directory symlinks). No log or warning.

8. **`delete.rs` has no symlink awareness** — `compute_delete_plan` and `generate_delete_plan` treat entries as files or directories based on `is_directory` only. A symlink pointing to a directory or file is classified by whatever `enumerate_directory_filtered` decides, and the deletion order assumes ordinary directories. No special handling/test in this file.

9. **`tar_stream` is named for "streaming" but is purely local** — fits the audit cluster boundary (no remote transport), but the docstring and module purpose could mislead readers into thinking it's a network primitive.

10. **`BufferSizer::new()` thresholds (16 MiB max, 1 MiB min, 10% memory cap, 8 MiB network base, 900 MiB scaling span)** are all magic numbers with no `const` names. Documented in code but cannot be tuned without recompiling.

11. **`buffer.rs:355-358` `return_vec`** does not add a semaphore permit back — relies on caller having used `take()` (which already added a permit). Undocumented contract; easy to misuse.

12. **`BufferSizer` caches available memory in `OnceCell`** — captured on first call and never refreshed. Long-running processes won't react to memory pressure changes.

13. **`chunked_copy_file` 16 MiB cliff at 1 GiB** — diverges from `copy_file` which trusts `BufferSizer` exclusively. The two main entry points use different sizing for the same file size class.

14. **Windows block-clone thread-local success flag** — `windows.rs:28-42` uses `LAST_BLOCK_CLONE_SUCCESS: Cell<bool>` in a thread-local; `take_last_block_clone_success` consumes it. If the code ever crosses a thread boundary between the copy and the take (e.g. rayon worker boundary), the flag will appear reset. Currently safe because `copy_file` is synchronous on one thread, but fragile.

15. **`compare.rs::file_needs_copy_with_checksum_type` falls through to mtime for non-Blake3 / None** — lines 63-69. The docstring says "explicit checksum selection" but a `None` or non-Blake3 type silently degrades to mtime comparison without warning.

16. **`parallel_copy_files` has no fail-fast** — collects errors but keeps copying. No way to stop on first failure or to bound the error budget.

17. **`mmap_copy_file` retry loop has no iteration cap** — `mmap.rs:26-77`. Under repeated EINTR/EAGAIN with no forward progress, it could spin (unlikely for regular files but theoretically possible).

18. **No TODO/FIXME/XXX/HACK markers present** in any cluster file (manually grepped during reading). All annotated issues are documented as audit-NN or R58-FNN inline comments.

19. **`copy/file_copy/mod.rs:43`** — Windows path takes `&is_network` for non-Windows behavior parity but the macOS/Linux paths ignore it (BufferSizer takes it for buffer sizing only). Implicit asymmetry — Windows path is local-only optimization, non-Windows path runs for both local and network.

20. **No references to removed features** (blit-utils, BlitAuth, AI telemetry) found in any file.

## Contradictions (within cluster)

1. **mtime tolerance** — `copy/compare.rs` uses 2-second tolerance for FAT/exFAT compatibility (`file_needs_copy_with_checksum_type:64-69`); `manifest.rs::compare_file` Default mode has zero tolerance (`182-192`). The two layers thus disagree on edge-case mtime skew.

2. **Force mode handling** — `compare.rs::file_needs_copy_with_mode` returns `Ok(true)` immediately for `Force | IgnoreTimes` (line 97); `manifest.rs::compare_file` for `Force` discards target inputs and returns `Modified` (lines 164-172) — semantically equivalent but two separate implementations. Both annotated as R58-F9 review-followup fixes.

3. **`copy_file` vs `chunked_copy_file` buffer sizing for large files** — `copy_file` always uses BufferSizer; `chunked_copy_file` hardcodes 16 MiB for files >1 GiB and uses BufferSizer otherwise. The two public entry points produce different I/O patterns for the same workload.

4. **Block-clone success signaling** — `copy_file` on Windows uses `windows::try_block_clone_with_handles` which sets the thread-local flag, then reads it via `take_last_block_clone_success` (mod.rs:45). But `windows_copyfile` (windows.rs:331-369) also calls `try_block_clone_same_volume` internally and sets the flag — both paths poke the same global state. If both run sequentially the flag from the first is consumed correctly, but the layered design is fragile.

5. **Sanitization function vs inline duplicate** — `tar_stream.rs` has `sanitize_rel_path` (43-59) but `tar_stream_transfer_cb` (204-224) inlines an equivalent loop. `tar_stream_transfer_list_cb` correctly uses the helper. Asymmetric usage within one file.

6. **Default mtime granularity** — `compare.rs::file_needs_copy` lines 30-33 uses 2s tolerance; the comment in `file_needs_copy_with_mode` (lines 86-87) explicitly says "The 2s tolerance matches `file_needs_copy_with_checksum_type` and FAT/exFAT mtime granularity" — but `manifest.rs::compare_file` has no such tolerance. The manifest layer (used for remote diff) and local compare layer diverge.
