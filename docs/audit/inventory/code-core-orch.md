# Code Inventory: blit-core orchestrator + mirror planner + enumeration
**Generated**: 2026-06-04 by audit workflow
**Coverage**: see attestation table at end

## Behaviors (grouped by category)

### state-machine

- **orch-execute-sync-wrapper** — `crates/blit-core/src/orchestrator/orchestrator.rs:130-143` — `execute_local_mirror` is a blocking convenience: it builds a new multi-thread Tokio runtime sized to `options.workers.max(1)` and `block_on`s `execute_local_mirror_async`. Comment explicitly warns that calling from inside a Tokio runtime will panic at `Runtime::new` (closed F9 of 2026-05-01 review).
- **orch-execute-async-core** — `crates/blit-core/src/orchestrator/orchestrator.rs:151-869` — async core. Phases: (1) exists check, (2) create destination parent (skipped under `dry_run`), (3) single-file short-circuit, (4) journal-fast-path probe, (5) fast-path selection (NoWork/Tiny/Huge), (6) streaming pipeline via `FilteredSource` → `plan_local_mirror` → `FsTransferSink`/`NullSink`, (7) optional mirror-deletion gated by unreadable-snapshot, (8) journal checkpoint persist, (9) perf-history record + predictor observe.
- **fast-path-decision-types** — `crates/blit-core/src/orchestrator/fast_path.rs:15-31` — `FastPathDecision::{NoWork{examined},Tiny{files},Huge{file,size}}`. `examined` field distinguishes "source empty / no files" (examined=0) from "all skipped under skip_unchanged" (examined>0); used to set `TransferOutcome::SourceEmpty` vs `UpToDate`.
- **fast-path-outcome-streaming** — `crates/blit-core/src/orchestrator/fast_path.rs:33-63` — `FastPathOutcome` carries `decision: Option<FastPathDecision>` plus `unreadable_paths: Vec<String>`; `streaming()` builder means "no fast-path decision, fall through to streaming pipeline".
- **fast-path-abort-mechanism** — `crates/blit-core/src/orchestrator/fast_path.rs:65-74,144-152` — uses a sentinel error `FastPathAbort` thrown from the visit callback to escape the walk when the tiny budget is exceeded (`>TINY_FILE_LIMIT files` or `>TINY_TOTAL_BYTES && files>1`). Downcast at line 166 separates the abort from genuine walk errors. _(notes: clever, but mixing control-flow with `eyre::Result` is subtle.)_
- **single-file-short-circuit** — `crates/blit-core/src/orchestrator/orchestrator.rs:177-179,1138-1298` — when `src_root.is_file()`, bypass enumerator/planner/pipeline entirely and route through `execute_single_file_copy`. Comment notes a previous data-loss class: enumerator skipped depth-0 root, fast-path reported NoWork, file silently never copied.

### path-handling

- **single-file-relative-empty** — `crates/blit-core/src/enumeration.rs:214-238` — when WalkDir depth==0 and it's a file (not a dir), emits the entry with `relative_path: PathBuf::new()` (empty) so `src_root.join(rel) === src_root` and `dest_root.join(rel) === dest_root`. Directory roots are skipped at depth 0 to avoid root-as-self emission.
- **relative-path-helper** — `crates/blit-core/src/enumeration.rs:303-309` — `relative_path(root, path)` returns `PathBuf::from(".")` for empty strip-prefix result, else the stripped result, falling back to the absolute path if strip-prefix fails. _(notes: `"."` sentinel is a hardcoded string that downstream code must avoid joining naively.)_
- **casefold-key-windows** — `crates/blit-core/src/mirror_planner.rs:33-43` — Windows `CasefoldKey` constructs by `relative_path_to_posix(path).to_ascii_lowercase()`. Used for source/dest comparison in mirror-delete planning. _(notes: ASCII-lowercase only — non-ASCII case-folding gaps on Windows.)_
- **casefold-key-unix** — `crates/blit-core/src/mirror_planner.rs:45-54` — Unix `CasefoldKey` is just `PathBuf` (case-sensitive). _(notes: diverges from Windows, intentional.)_
- **dest-dir-sort-reverse** — `crates/blit-core/src/mirror_planner.rs:290-291` — `plan_from_sets` sorts `dirs` by `components().count()` then reverses, so deepest dirs delete first.
- **apply-mirror-deletion-deepest-first** — `crates/blit-core/src/orchestrator/orchestrator.rs:957-958` — `dirs_to_delete.sort_by_key(|b| std::cmp::Reverse(b.components().count()))`; same deepest-first ordering as `mirror_planner::plan_from_sets`. _(notes: pattern duplicated rather than shared.)_
- **posix-key-for-dest-set** — `crates/blit-core/src/orchestrator/orchestrator.rs:943` — `apply_mirror_deletions` rebuilds the per-entry key with `crate::path_posix::relative_path_to_posix(&entry.relative_path)` for HashSet lookup. _(notes: separate code path from mirror_planner's `CasefoldKey`; on Windows the two paths' lowercasing semantics could diverge.)_
- **source-dirs-derived-from-files** — `crates/blit-core/src/orchestrator/orchestrator.rs:921-937` — `source_paths` set contains file rels only (because `source.scan()` emits file headers); the code rebuilds `source_dirs` by walking each file's `parent()` chain. R48-F1 fix: without this, every dest dir was "absent at source" and got queued for `remove_dir`. _(notes: O(files × depth); also: empty `parent().as_os_str()` is the break sentinel, not Option::None — fragile.)_
- **dir-skip-current-on-exclude** — `crates/blit-core/src/enumeration.rs:240-244` — When a dir doesn't pass `filter.allows_dir`, calls `walker.skip_current_dir()`. _(notes: this prunes the subtree, so excluded dirs never produce suppressed errors either.)_
- **glob-match-both-rel-and-filename** — `crates/blit-core/src/fs_enum.rs:215-240` — Includes/excludes match against BOTH `filename` (bare basename) and `path_str` (relative path). Comment: `--include '*.log'` works without users having to write `**/*.log`. Dual matching via globset AND ad-hoc `glob_match`. _(notes: doubly redundant — `gs.is_match()` then `self.X.iter().any(|p| glob_match(p,...))`; smell.)_
- **exclude-dirs-component-walk** — `crates/blit-core/src/fs_enum.rs:281-294` — `should_include_dir` walks every path component and runs `glob_match(pattern, component_str)` against each. _(notes: behaves like rsync's per-component exclude; differs from file matching.)_

### endpoint-parse / data-plane

- **journal-fast-path-gate** — `crates/blit-core/src/orchestrator/orchestrator.rs:200-251` — journal fast-path probe ONLY runs when `skip_unchanged && !checksum && !force_tar && !null_sink && dest_root.exists()`. Both source AND dest must report `ChangeState::NoChanges` for `journal_skip = true`. Missing dest probe (unsupported FS) cannot assert no-change → falls through. _(notes: 5-flag gate is the load-bearing safety check.)_
- **fast-path-mirror-checksum-force-tar-disable** — `crates/blit-core/src/orchestrator/fast_path.rs:81-94` — fast-path is disabled when `mirror || checksum || force_tar`, AND when `compare_mode != SizeMtime`. R58-F7 comment: fast-path's `should_copy_entry` only understands SizeMtime/Checksum, so `--size-only`/`--force`/`--ignore-times` previously silently became SizeMtime here.
- **fast-path-null-sink-bypass** — `crates/blit-core/src/orchestrator/orchestrator.rs:291-295` — `null_sink` → unconditionally `FastPathOutcome::streaming()` (bypasses fast-path entirely because fast-path doesn't go through the sink abstraction).
- **fast-path-tiny-budget** — `crates/blit-core/src/orchestrator/fast_path.rs:11-13,144-152` — `TINY_FILE_LIMIT=256`, `TINY_TOTAL_BYTES=256MB`, `HUGE_SINGLE_BYTES=1GB`. Abort when either limit exceeded.
- **huge-fast-path-condition** — `crates/blit-core/src/orchestrator/fast_path.rs:196-203` — `Huge` decision requires `huge_candidate` to be Some (set only when first file seen and no second appeared) AND `size >= HUGE_SINGLE_BYTES`. The `huge_candidate` is unset (set to None at line 138) the moment a second file appears. _(notes: subtle — only fires for exactly-one-file scans where that file ≥1GB.)_
- **dry-run-no-parent-mkdir** — `crates/blit-core/src/orchestrator/orchestrator.rs:101-112` (also orch:161-167 short-circuit) and `local_worker.rs:101-112` — R58-F4: dry-run must not create the destination parent directory. The destination-parent `create_dir_all` is gated behind `!options.dry_run`; in `local_worker::copy_path_maybe` dry-run returns after stat-only `file_needs_copy_with_checksum_type`. _(notes: copy_large_blocking at local_worker.rs:46-51 still calls `create_dir_all(parent)` unconditionally even for dry-run — see smell below.)_

### safety-check

- **mirror-refuse-on-incomplete-scan** — `crates/blit-core/src/orchestrator/orchestrator.rs:752-784` — R46-F2: when `mirror == true`, if `unreadable_snapshot` is non-empty, `bail!` with a detailed message (count, plural form, first-5 paths). Refuses to delete from destination when source scan dropped any subtree.
- **mirror-delete-failures-bail** — `crates/blit-core/src/orchestrator/orchestrator.rs:1041-1056` — R46-F5: `apply_mirror_deletions` collects per-entry failures into `failures: Vec<String>` and bails at the end (after attempting all deletions). Pre-fix, errors were printed as warnings and Ok was returned. Bail message includes count, first-5 preview, total deleted dirs+files.
- **filtered-subset-enotempty-tolerance** — `crates/blit-core/src/orchestrator/orchestrator.rs:1007-1034` — In `FilteredSubset` mode, ENOTEMPTY on a dest dir is expected (the dir contains out-of-scope filter-excluded files). Detected via `err.kind() == DirectoryNotEmpty || raw_os_error() == Some(66)` (ENOTEMPTY macOS/BSD). Skipped silently; surface error only under `LocalMirrorDeleteScope::All`. _(notes: `Some(66)` is a hardcoded numeric errno; Linux ENOTEMPTY is 39, NOT covered here — see smell.)_
- **enumeration-non-root-error-capture** — `crates/blit-core/src/enumeration.rs:184-209` — Non-root walk errors are pushed to `outcome.suppressed_errors` (path display, io_kind, message); root errors propagate as `Err`. Capture is the load-bearing R46-F2 fix; `enumerate_local_streaming` drops the outcome for callers that opted into best-effort semantics.
- **single-file-filter-applies** — `crates/blit-core/src/orchestrator/orchestrator.rs:1184-1205` — R58-F5: single-file short-circuit re-applies the filter via `options.filter.allows_entry(Some(&name), src_root, size, mtime)`; on excluded, returns `scanned_files=1, copied_files=0, outcome=UpToDate` summary.
- **single-file-ignore-existing** — `crates/blit-core/src/orchestrator/orchestrator.rs:1210-1221` — Single-file path honors `--ignore-existing` (return zero-copy summary when dest exists).
- **null-sink-no-predictor-train** — `crates/blit-core/src/orchestrator/orchestrator.rs:860-866` — `update_predictor` is gated behind `!options.null_sink` so synthetic null-sink runs don't teach the predictor that transfers are faster than reality.
- **path-no-absolute-or-parent** — `crates/blit-core/src/local_worker.rs:86-96` — `copy_path_maybe` `bail!`s on absolute relative paths or relative paths containing `ParentDir` components. Defense against header injection.
- **enum-root-must-exist** — `crates/blit-core/src/enumeration.rs:172-174` — `enumerate_local_streaming_capturing` `bail!`s if root doesn't exist (this is a hard error vs the per-walk-error suppression).

### error-propagation

- **outcome-into-summary-mapping** — `crates/blit-core/src/orchestrator/orchestrator.rs:303-344` — NoWork: if `examined == 0` → `SourceEmpty`, else `UpToDate`. Verbose log line varies accordingly.
- **scan-handle-await-propagation** — `crates/blit-core/src/orchestrator/orchestrator.rs:547-550` — `scan_handle.await.context("scan task panicked")?.context("scan failed")?` — distinguishes JoinError (panic) from inner Err.
- **diff-plan-spawn-blocking** — `crates/blit-core/src/orchestrator/orchestrator.rs:556-574` — `plan_local_mirror` runs under `spawn_blocking` and double-`??`s (`context("diff_planner task panicked")??`).
- **mtime-warn-not-silence** — `crates/blit-core/src/local_worker.rs:62-65, 130-134` and `orchestrator.rs:1271-1278` — R42-F1: failures setting mtime now `log::warn!`, not silently dropped.
- **predictor-save-err-verbose-only** — `crates/blit-core/src/orchestrator/history.rs:128-133` — predictor `.save()` error printed only when `verbose`; otherwise silent.
- **perf-history-append-verbose-only** — `crates/blit-core/src/orchestrator/history.rs:46-50` — `append_local_record` failure printed only under verbose.

### default-value

- **local-mirror-defaults** — `crates/blit-core/src/orchestrator/options.rs:82-106` — defaults: mirror=false, dry_run=false, perf_history=true, preserve_symlinks=true, include_symlinks=true, skip_unchanged=true, checksum=false, workers=`num_cpus::get().max(1)`, preserve_times=true, debug_mode=false, resume=false, null_sink=false.
- **compare-mode-default-sizemtime** — `crates/blit-core/src/orchestrator/options.rs:25-40` — `LocalCompareMode` default is `SizeMtime`.
- **delete-scope-default-subset** — `crates/blit-core/src/orchestrator/options.rs:9-20` — `LocalMirrorDeleteScope` default is `FilteredSubset`.
- **tuning-window-size** — `crates/blit-core/src/orchestrator/orchestrator.rs:29-33` — `TUNING_WINDOW_SIZE = 20`. Comment: lets a regime change propagate within ~20 transfers.
- **categorize-files-limits** — `crates/blit-core/src/fs_enum.rs:498-516` — small <1MB (tar streaming), medium <100MB (parallel copy), large ≥100MB (chunked). Hard-coded constants `SMALL_LIMIT=1_048_576`, `MEDIUM_LIMIT=104_857_600`.

### persistence

- **journal-tracker-load** — `crates/blit-core/src/orchestrator/orchestrator.rs:181-183` — `ChangeTracker::load().ok()` — failures swallowed; Tracker is purely opportunistic.
- **predictor-load-ok** — `crates/blit-core/src/orchestrator/orchestrator.rs:185` — `PerformancePredictor::load().ok()` — same opportunistic pattern.
- **persist-journal-checkpoints** — `crates/blit-core/src/orchestrator/orchestrator.rs:1061-1090` — reprobes each token's canonical_path; clears snapshot on error; persists via `tracker.refresh_and_persist`. Persist failures only logged under verbose.

### timeout-or-retry

- **mtime-diff-tolerance-2s** — `crates/blit-core/src/mirror_planner.rs:215, 262` — Both `should_copy_remote_entry` and `should_fetch_remote_file` allow `-2..=2` second diff window for mtime comparison. _(notes: 2-second tolerance is a hardcoded magic number; arguably should be a constant.)_

### naming

- **transfer-outcome-variants** — `crates/blit-core/src/orchestrator/summary.rs:10-26` — `TransferOutcome::{Transferred,JournalSkip,UpToDate,SourceEmpty}`. SourceEmpty distinguishes empty-source from all-skipped-up-to-date.
- **fast-path-label-strings** — `crates/blit-core/src/orchestrator/orchestrator.rs:280,337,374,410,849` — string labels passed to `record_performance_history`: `"journal_no_work"`, `"no_work"`, `"tiny_manifest"`, `"single_huge_file"`, `"null_sink"`. _(notes: these strings are load-bearing — `select_tuning_window` references `"tiny_manifest"` literally as an exclude gate at orch:63.)_

### format-output

- **predictor-verbose-log** — `crates/blit-core/src/orchestrator/orchestrator.rs:697-711` — `eprintln!` of predictor estimate (planner ms, transfer ms, total ms, n, fallback_depth) when `verbose`; "unavailable" message when None.
- **predictor-delta-pct-summary** — `crates/blit-core/src/orchestrator/orchestrator.rs:824-845` — `pct()` closure formats `+nn%` or `-nn%` (special-cases actual_ms==0 → "n/a"). Side-by-side predicted-vs-actual line in verbose mode.
- **completed-local-mirror-line** — `crates/blit-core/src/orchestrator/orchestrator.rs:428-436, 806-819` — final summary line in verbose mode with file count, total_bytes, duration, plan ms, xfer ms.
- **journal-probe-log** — `crates/blit-core/src/orchestrator/orchestrator.rs:1092-1126` — `log_probe()` dumps StoredSnapshot variant-by-variant (Windows: volume/journal_id/next_usn; macOS: fsid/event_id; Linux: device/inode/ctime/mtime).

### spawn-task

- **scan-emits-via-mpsc** — `crates/blit-core/src/orchestrator/orchestrator.rs:540-546` — `source.scan(None, unreadable)` returns `(header_rx, scan_handle)`; orchestrator consumes all headers via `recv().await` then awaits the handle.
- **runtime-build-workers** — `crates/blit-core/src/orchestrator/orchestrator.rs:136-141` — runtime gets `worker_threads(options.workers.max(1))`. _(notes: not enable_io+enable_time, but `enable_all`.)_

### discovery

- **enumerate-streaming-callback** — `crates/blit-core/src/enumeration.rs:164-300` — `enumerate_local_streaming_capturing<F: FnMut(EnumeratedEntry) -> Result<()>>` — visit callback returns Result so callers can abort mid-walk by returning Err (used by fast-path's FastPathAbort).
- **enumerate-symlink-handling** — `crates/blit-core/src/enumeration.rs:274-296` — when `is_symlink() && include_symlinks`, skip if `follow_symlinks` (else handled by WalkDir's `follow_links`). Uses `fs::symlink_metadata` and `fs::read_link` to get target. Filter check uses `size=0` for symlinks.
- **follow-include-mutually-exclusive-doc** — `crates/blit-core/src/enumeration.rs:89-93` — comment: "with `follow_symlinks`, only one of these should typically be enabled" — not enforced in code, just documented.
- **fast-path-mirror-uses-clone-without-cache** — `crates/blit-core/src/orchestrator/fast_path.rs:96` — uses `options.filter.clone_without_cache()`. Pattern repeated in `enumeration.rs:176`, `mirror_planner.rs:73,118,164`. Clone resets compiled glob caches.

### flag-handling / config-load

- **compare-mode-bool-translation-orch** — `crates/blit-core/src/orchestrator/orchestrator.rs:520-532, 1159-1171` — Match arm translates `LocalCompareMode` → `generated::ComparisonMode` in TWO places (streaming path orch:520 AND single-file path orch:1159). Both honor backward-compat `if options.checksum && SizeMtime → Checksum`. _(notes: duplicated 12-line translation — if a new variant is added it must be patched in both places + `snapshot_compare_mode` in history.rs:11-25 + tuning-window query at orch:467-487 = 4 sites.)_
- **compare-mode-translation-history** — `crates/blit-core/src/orchestrator/history.rs:11-25` — third translation site: `snapshot_compare_mode` maps to `CompareModeSnapshot` for perf-history record.
- **compare-mode-tuning-query** — `crates/blit-core/src/orchestrator/orchestrator.rs:467-487` — fourth translation site, this time for querying the tuning window.
- **plan-options-tuning-load** — `crates/blit-core/src/orchestrator/orchestrator.rs:447-499` — `plan_options.small_target / small_count_target / medium_target` populated from `derive_local_plan_tuning` when history has eligible records.
- **validate-globs-construction-time** — `crates/blit-core/src/fs_enum.rs:126-158` — R58-F12: `validate_globs` is the up-front pattern checker for CLI. Reports the bad pattern verbatim. Differs from `build_globset` (runtime fallback) which silently drops invalid patterns.

### render-or-display (verbose telemetry)

- **fast-path-route-log** — `crates/blit-core/src/orchestrator/fast_path.rs:215-256 tests; orch:310-319,346-352,382-389` — eprintln on fast-path branch chosen, with byte/file counts.
- **scan-incomplete-error-format** — `crates/blit-core/src/orchestrator/orchestrator.rs:763-783` — bail message includes plural-form "y"/"ies", first-5 unreadable preview joined by "; ".

### rpc-handler / data-plane / cross-crate-bridge

- **diff-planner-input-shape** — `crates/blit-core/src/orchestrator/orchestrator.rs:559-572` — `LocalDiffInputs { src_root, dst_root, compare_mode, ignore_existing, plan_options, skip_unchanged }`. The shared cross-direction abstraction.
- **sink-fs-config** — `crates/blit-core/src/orchestrator/orchestrator.rs:580-598` — `FsSinkConfig { preserve_times, dry_run, checksum, resume, compare_mode }`. Mirror-followup: compare_mode threaded through; pre-fix sink hard-coded `file_needs_copy_with_checksum_type` (SizeMtime/Checksum only).

### copy-strategy (local_worker)

- **copy-paths-blocking-fast-path** — `crates/blit-core/src/local_worker.rs:19-36` — `copy_paths_blocking` loops per-rel via `copy_path_maybe`. Uses `BufferSizer::default()` and `NoopLogger`. Empty rels → no-op Ok.
- **mmap-copy-large-linux-only** — `crates/blit-core/src/local_worker.rs:54-69` — `copy_large_blocking` uses `mmap_copy_file` only on `cfg(all(unix, not(target_os = "macos")))` (i.e. Linux/BSD). macOS and Windows fall through to `copy_paths_blocking` (line 71-75).
- **copy-large-dry-run-side-effect** — `crates/blit-core/src/local_worker.rs:44-52` — `copy_large_blocking` calls `std::fs::create_dir_all(parent)` BEFORE the dry-run check at line 50. _(notes: directly contradicts R58-F4 in `copy_path_maybe` which dropped the side effect; smell — see below.)_
- **resume-copy-path** — `crates/blit-core/src/local_worker.rs:117-120` — `resume_copy_file(&src, &dst, 0)` if `config.resume`. `did_copy = bytes_transferred > 0` — but no preserve_times branch for resume (line 127 gates on `did_copy && !clone_succeeded`; resume sets `clone_succeeded=false`, so mtime IS preserved). _(notes: subtle interaction.)_

### filter semantics

- **files-from-bypasses-other-rules** — `crates/blit-core/src/fs_enum.rs:201-205` — when `files_from.is_some()`, all other filter rules are bypassed; only the explicit path-set lookup decides. Returns false if `rel_path` is None.
- **age-needs-mtime-and-ref** — `crates/blit-core/src/fs_enum.rs:255-268` — age constraints applied only when BOTH `mtime` and `reference_time` are present; otherwise the entry passes age check.
- **allows-dir-not-files-from-aware** — `crates/blit-core/src/fs_enum.rs:275-279` — when `files_from.is_some()`, directories are unconditionally allowed (need to descend).
- **glob-match-trivia** — `crates/blit-core/src/fs_enum.rs:399-418` — hand-rolled glob matcher: `*` alone matches all; `*X*` substring; `*X` suffix; `X*` prefix; else exact. _(notes: duplicates globset; smell — see below.)_

### misc summary fields

- **summary-scanned-vs-copied-doc** — `crates/blit-core/src/orchestrator/summary.rs:47-73` — explicit contract: `scanned_*` is source-side workload (used by predictor + tuner); `planned_files` is planner's decision; `copied_files` is actual writes; `total_bytes` is pipeline-written bytes (not scanned).
- **predictor-estimate-fields** — `crates/blit-core/src/orchestrator/summary.rs:36-46` — `PredictorEstimate { planner_ms, transfer_ms, total_ms, observations, fallback_depth }`. Only populated on streaming-pipeline runs (fast-path branches leave it None).
- **unreadable-paths-on-summary** — `crates/blit-core/src/orchestrator/summary.rs:111-120` — `unreadable_paths` field is the load-bearing surface so the CLI's source-delete step (move) can refuse to remove a partially-scanned source.

## Smells / risks observed

- **enotempty-errno-66-only-macos-bsd** — `crates/blit-core/src/orchestrator/orchestrator.rs:1020` — `raw_os_error() == Some(66)` matches ENOTEMPTY on macOS/BSD but Linux ENOTEMPTY is errno 39. The `err.kind() == DirectoryNotEmpty` check above it should cover modern Rust on Linux (stable since 1.79), but if the kind isn't populated, Linux's ENOTEMPTY would slip through and report as failure under FilteredSubset. Worth verifying.
- **copy-large-dry-run-creates-parent** — `crates/blit-core/src/local_worker.rs:46-52` — `copy_large_blocking` calls `create_dir_all(parent)` BEFORE checking `config.dry_run`. Directly contradicts the R58-F4 invariant (no side effects under dry-run). Fast-path Huge route from orchestrator.rs:390 hits this — `blit copy huge.bin dst.bin --dry-run` would create `dst.bin`'s parent.
- **two-glob-impls-duplicate-work** — `crates/blit-core/src/fs_enum.rs:218-240` — `allows_entry` runs both `gs.is_match()` (globset) AND iterates `self.include_files` calling the hand-rolled `glob_match` on each. The hand-rolled version has different semantics (no `?`, no `{a,b}`, no `**`) — patterns that compile cleanly under globset behave differently from the fallback. If globset accepts them, the fallback redundant; if it doesn't (silently dropped by `build_globset`), the fallback is the only check. Inconsistent.
- **glob-build-silently-drops-bad-patterns** — `crates/blit-core/src/fs_enum.rs:112-124` — `build_globset` silently drops patterns that don't compile, then unwraps an empty builder. Only `validate_globs` (R58-F12) catches bad patterns; if any caller skips validation, errors are swallowed and the filter is permissive in ways the user didn't intend.
- **compare-mode-translation-x4** — orch:520-532, orch:1159-1171, history.rs:11-25, orch:467-487 — four sites translate `LocalCompareMode` to other enums. Adding a new variant requires patching all four. The history-side `snapshot_compare_mode` (history.rs:11) duplicates orch:467-487's logic but uses different output type. High refactor risk.
- **casefold-key-vs-posix-key-divergence** — `mirror_planner.rs:39-40` lowercases for the planner-API mirror-deletion path, but `apply_mirror_deletions` at orch:943 uses `relative_path_to_posix` without lowercasing. On Windows, two parallel deletion paths may treat "Foo/Bar" and "foo/bar" differently. The orchestrator path is the one actually used for local mirror runs; the planner API helpers are called separately by remote/pull code. Audit dependents.
- **glob-match-hand-rolled** — `crates/blit-core/src/fs_enum.rs:399-418` — independent reimplementation of trivial glob matching alongside `globset`. Either remove (let globset handle it) or document why both must exist.
- **windows-enum-functions-duplicate** — `crates/blit-core/src/fs_enum.rs:421-495` — `enumerate_directory_filtered` and `enumerate_symlinks` have identical bodies in `cfg(not(windows))` (425-459) and `cfg(windows)` (461-495) — duplicated boilerplate; the comment "All Windows-specific code removed" (420) suggests the cfg split is now vestigial.
- **fast-path-huge-only-when-exactly-one-file** — `fast_path.rs:135-139` — `huge_candidate` is set ONLY when `files.is_empty()` (i.e. on the first file seen); explicitly cleared when a second file appears. So Huge fast-path can only fire when the scan saw exactly one file. Documented behavior, but easy to misread.
- **fast-path-abort-suppressed-empty-comment** — `crates/blit-core/src/orchestrator/fast_path.rs:169-178` — comment justifies why `suppressed` is `Vec::new()` on abort path (streaming planner re-scans with capturing). But if streaming-planner ever changes to skip the capturing scan, abort-path unreadable signal is dropped silently. Coupling risk.
- **dest-probe-none-falls-through-silently** — `crates/blit-core/src/orchestrator/orchestrator.rs:233-241` — When dest journal probe returns None (unsupported FS), `dest_no_change = false` and journal fast-path doesn't apply. Verbose logs "dest unsupported", but non-verbose silently uses full planner. Probably correct, but worth confirming users on unsupported FSs aren't surprised by the slower path.
- **mtime-tolerance-2s-magic** — `crates/blit-core/src/mirror_planner.rs:215,262` — `!(-2..=2).contains(&diff)` repeated twice. Should be a named constant.
- **relative-path-dot-sentinel** — `crates/blit-core/src/enumeration.rs:303-309` — returns `PathBuf::from(".")` for empty strip-prefix. Only reachable when `path == root` (depth-0 directory case handled separately). The `"."` sentinel could surprise downstream code joining onto a dest root.
- **categorize-files-unused-by-orchestrator** — `crates/blit-core/src/fs_enum.rs:498-516` — `categorize_files` returns (small, medium, large) by hardcoded thresholds, but the orchestrator path uses the streaming planner / `derive_local_plan_tuning` instead. May be dead code or only used by older paths; worth confirming.
- **enum-streaming-drops-outcome** — `crates/blit-core/src/enumeration.rs:143-152` — `enumerate_local_streaming` discards the `EnumerationOutcome`. Comment says callers opt-in, but the API is easy to misuse for destructive workflows.
- **double-mirror-deletion-planner-and-orch** — `mirror_planner.rs:67-86, 112-132` and `orchestrator.rs:889-1059` — two parallel deletion-planning implementations: `MirrorPlanner::plan_local_deletions{,_from_entries}` (used by remote/pull paths) and `apply_mirror_deletions` (used by local orchestrator). They have the same job but different code (one uses `CasefoldKey`, the other uses `relative_path_to_posix`). Divergence risk.
- **scan-task-await-no-timeout** — `crates/blit-core/src/orchestrator/orchestrator.rs:544-550` — `header_rx.recv().await` and `scan_handle.await` have no timeout; if the source scan hangs (e.g., stalled network FS), the orchestrator is wedged. Probably fine for local, risky if reused for slow filesystems.

## Coverage attestation

| File | Lines read | Notes |
|---|---|---|
| crates/blit-core/src/orchestrator/mod.rs | 9 | full |
| crates/blit-core/src/orchestrator/options.rs | 106 | full |
| crates/blit-core/src/orchestrator/summary.rs | 121 | full |
| crates/blit-core/src/orchestrator/history.rs | 220 | full |
| crates/blit-core/src/orchestrator/fast_path.rs | 280 | full |
| crates/blit-core/src/orchestrator/orchestrator.rs | 2466 | full (read 1-500, 500-999, 999-1499, 1499-1999, 1999-2466) |
| crates/blit-core/src/mirror_planner.rs | 294 | full |
| crates/blit-core/src/local_worker.rs | 139 | full |
| crates/blit-core/src/enumeration.rs | 400 | full |
| crates/blit-core/src/fs_enum.rs | 531 | full |

**Total lines read**: 4566
**Files NOT read** (with reason): (none)
