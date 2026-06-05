# Code Inventory: blit-core remaining modules

**Generated**: 2026-06-04 by audit workflow
**Cluster**: Auto-tuning, FS capability probing, change-journal abstractions, mDNS, predictor + history, path containment & POSIX-form helpers, config dir resolution, error types

## Coverage (file list with line counts)

| File | Lines |
|---|---:|
| `crates/blit-core/src/lib.rs` | 54 |
| `crates/blit-core/src/config.rs` | 43 |
| `crates/blit-core/src/errors.rs` | 154 |
| `crates/blit-core/src/logger.rs` | 80 |
| `crates/blit-core/src/auto_tune/mod.rs` | 359 |
| `crates/blit-core/src/fs_capability/mod.rs` | 117 |
| `crates/blit-core/src/fs_capability/probe.rs` | 334 |
| `crates/blit-core/src/fs_capability/macos.rs` | 173 |
| `crates/blit-core/src/fs_capability/unix.rs` | 208 |
| `crates/blit-core/src/fs_capability/windows.rs` | 313 |
| `crates/blit-core/src/change_journal/mod.rs` | 10 |
| `crates/blit-core/src/change_journal/snapshot.rs` | 277 |
| `crates/blit-core/src/change_journal/tracker.rs` | 115 |
| `crates/blit-core/src/change_journal/types.rs` | 63 |
| `crates/blit-core/src/change_journal/util.rs` | 47 |
| `crates/blit-core/src/mdns.rs` | 360 |
| `crates/blit-core/src/perf_history.rs` | 781 |
| `crates/blit-core/src/perf_predictor.rs` | 1368 |
| `crates/blit-core/src/path_safety.rs` | 830 |
| `crates/blit-core/src/path_posix.rs` | 198 |

**Total lines read**: 5884

## Behaviors (grouped by category)

### config-load

- **config-dir-override-priority** — `crates/blit-core/src/config.rs:27-43` — `config_dir()` resolves in priority: explicit `set_config_dir` override → `ProjectDirs("com","Blit","Blit")` (platform standard XDG/AppData) → `~/.config/blit` via `BaseDirs` → error. RwLock + static `OnceLock`-equivalent (`Lazy`) gates override. _(notes: silent fallback to `~/.config/blit` even on Windows if `ProjectDirs` fails; hardcoded triple `"com"/"Blit"/"Blit"`)_
- **perf-history-config-path** — `crates/blit-core/src/perf_history.rs:357-385` — `history_path()` → `config_dir()/perf_local.jsonl`; `settings_path()` → `config_dir()/settings.json` (hardcoded constant `SETTINGS_FILE`). Re-exports `config_dir` from this module — a duplicate accessor on top of `crate::config`. _(notes: filename strings `perf_local.jsonl` and `settings.json` are buried mid-file; small contract overlap with `change_journal::util::journal_store_path`)_
- **change-journal-config-path** — `crates/blit-core/src/change_journal/util.rs:8-10` — `journal_store_path()` → `config_dir()/journal_cache.json`. _(notes: similar hardcoded filename pattern as perf history; consider centralizing config-file names)_
- **predictor-state-path** — `crates/blit-core/src/perf_predictor.rs:220-223,34` — `PerformancePredictor::load()` reads `config_dir()/perf_predictor.json` via `STATE_FILENAME` constant. _(notes: hardcoded filename; behaves like change-journal/perf-history with separate file alongside)_

### persistence

- **journal-cache-load-fresh-on-parse-error** — `crates/blit-core/src/change_journal/tracker.rs:10-36` — `ChangeTracker::load()` opens journal cache JSON; on parse error logs to stderr and starts fresh (empty map) instead of returning Err. _(notes: tolerant — fails open with eprintln; no telemetry; silent loss of stored snapshots)_
- **journal-cache-persist-truncate-write** — `crates/blit-core/src/change_journal/tracker.rs:84-110` — `persist()` creates parent dir then opens with `create+truncate+write` and `serde_json::to_writer_pretty` then flushes. _(notes: non-atomic write; concurrent process could see partial JSON; no temp+rename)_
- **perf-history-append-with-cap** — `crates/blit-core/src/perf_history.rs:239-266,446-492` — `append_local_record()` checks `perf_history_enabled()` first; appends one JSONL line; then `enforce_size_cap` with `DEFAULT_MAX_BYTES = 1_000_000` (~1 MiB). Cap enforcement is best-effort: re-reads metadata before truncating-and-rewriting; bails if file changed under it to avoid concurrent-writer data loss. _(notes: comment says "concurrent writer skips rotation" — the file CAN exceed the cap if writers race; rotation is "best effort", not strict)_
- **perf-history-settings-persist** — `crates/blit-core/src/perf_history.rs:404-419` — `store_settings()` writes `settings.json` with pretty JSON then appends trailing `\n`; non-atomic `File::create`. _(notes: no temp+rename; default if missing is `perf_history_enabled: true`)_
- **predictor-save-non-atomic** — `crates/blit-core/src/perf_predictor.rs:476-484` — `PerformancePredictor::save()` does `File::create(path)` + `write_all(json)`; non-atomic. _(notes: pretty-print JSON; no fsync)_
- **predictor-state-version-reset** — `crates/blit-core/src/perf_predictor.rs:564-580,33` — `load_state_from_path()` reads JSON; if `state.version != STATE_VERSION` (currently 3), resets to fresh `PredictorState::new()` — discards prior coefficients silently. _(notes: R56-F1 documented invariant; prevents contaminated pre-R56 state file from training)_

### safety-check

- **wire-path-nul-byte-rejection** — `crates/blit-core/src/path_safety.rs:71-92` — `validate_wire_path()` rejects strings containing `\0`. _(notes: load-bearing for protobuf-decoded strings that may carry embedded NULs)_
- **wire-path-windows-shape-reject-unix** — `crates/blit-core/src/path_safety.rs:344-363` — `looks_like_windows_absolute()` flags `\\?\…`, `\\.\…`, `\\server\share`, `//server/share`, single `\foo`, and drive-letter forms (`C:\`, `c:foo`, `Z:\`) regardless of host platform. _(notes: ensures Unix daemons reject Windows-shaped wire paths uniformly; cross-platform symmetry)_
- **wire-path-component-walk-rejection** — `crates/blit-core/src/path_safety.rs:95-117` — Iterates `Path::components()` rejecting `Prefix`, `RootDir`, `ParentDir`; silently strips `CurDir`. _(notes: defense in depth on top of `looks_like_windows_absolute`; `..` always rejected as a component)_
- **wire-path-empty-vs-dot-distinction** — `crates/blit-core/src/path_safety.rs:119-130` — Empty input legitimate single-file case → empty `PathBuf`. Non-empty input that normalizes to empty (`"."`, `"./"`, `"./."`) is rejected (R1-F3 per followup_review_2026-05-02). _(notes: subtle distinction — empty `""` allowed, but pure-dot inputs rejected; conflating these caused real bug)_
- **safe-join-root-passthrough** — `crates/blit-core/src/path_safety.rs:142-149` — `safe_join(root, "")` returns `root` unchanged; load-bearing for single-file destination — `PathBuf::join("")` would otherwise append separator causing `ENOTDIR`. _(notes: documented invariant)_
- **contained-join-canonicalize-deepest-ancestor** — `crates/blit-core/src/path_safety.rs:180-220` — `contained_join` performs `safe_join` then walks up via `probe.pop()` until `canonicalize` succeeds; checks `canonical_ancestor.starts_with(canonical_module_root)`; returns LEXICAL target (not canonical). _(notes: F2 chokepoint; TOCTOU window documented; race-prone alternative is openat/O_NOFOLLOW)_
- **verify-contained-helper** — `crates/blit-core/src/path_safety.rs:228-260` — `verify_contained` does canonicalize-and-check on an already-built target without re-validating wire path. _(notes: nearly identical loop body to `contained_join` — duplicated logic)_
- **canonical-dest-root-walk** — `crates/blit-core/src/path_safety.rs:283-306` — `canonical_dest_root` walks deepest-existing-ancestor when dest_root may not exist yet. _(notes: third place the "walk up + canonicalize" loop is repeated; loop body is duplicated 3 times)_
- **safe-join-contained-one-stop** — `crates/blit-core/src/path_safety.rs:321-329` — `safe_join_contained` = `safe_join` + `verify_contained`. _(notes: R46-F3 single chokepoint; preferred call site for local-receive paths)_
- **path-posix-no-blanket-replace** — `crates/blit-core/src/path_posix.rs:26-44` — `relative_path_to_posix` joins `Path::components` with `/` instead of doing `replace('\\','/')`. On POSIX, `\` is a legal filename byte and survives intact. _(notes: documented regression fix — Logic Pro plug-ins like `1\4 Single.pst` broke under blanket replace)_
- **path-posix-trailing-sep-preserved** — `crates/blit-core/src/path_posix.rs:64-78` — `relative_str_to_posix` detects trailing separator via `std::path::is_separator` and re-attaches `/` after canonicalization. Trailing `\` is preserved as literal on POSIX (not a separator there). _(notes: round-1 reopen — `sub/` was canonicalizing to `sub`)_
- **windows-block-clone-cache** — `crates/blit-core/src/fs_capability/windows.rs:31-32,124-135,137-162` — Per-volume-serial cache (`BLOCK_CLONE_CACHE: HashMap<u32, BlockCloneStatus>`); `supports_block_clone_same_volume_internal` checks same-volume + ReFS; `mark_block_clone_unsupported` inserts negative entry. _(notes: cache lives at module scope via `Lazy`/`RwLock`; never invalidated for the lifetime of the process)_
- **fs-capability-probe-cache** — `crates/blit-core/src/fs_capability/mod.rs:87-117` — Global `OnceLock<Mutex<HashMap<u64, Capabilities>>>` keyed by `metadata.dev()` (Unix only); never invalidated. Non-Unix code path bypasses cache entirely. _(notes: cgf-divergent; on Windows every probe goes through `volume_info_for_path` lookup with no shared cache here — only the block-clone cache exists in `windows.rs`)_

### path-handling

- **canonicalize-windows-vs-unix-divergence** — `crates/blit-core/src/change_journal/util.rs:12-26` — Windows path: uses `normpath::BasePath::new(current_dir)` + join → returns lexical absolute path WITHOUT resolving symlinks. Unix path: `std::fs::canonicalize` which DOES resolve symlinks. _(notes: divergent semantics across platforms; comparing two `canonicalize` calls on different OSes may behave differently for same logical path)_
- **canonical-to-key-lossy** — `crates/blit-core/src/change_journal/util.rs:28-30` — Map key is `path.to_string_lossy().to_string()` — non-UTF-8 paths get replacement characters and may collide. _(notes: small data-loss risk on non-UTF-8 paths)_

### state-machine

- **change-state-four-variant** — `crates/blit-core/src/change_journal/types.rs:5-11` — `ChangeState`: Unsupported / Unknown / NoChanges / Changes. _(notes: lattice over "has anything changed since last probe?")_
- **probe-state-derivation** — `crates/blit-core/src/change_journal/tracker.rs:38-54` — `probe()`: if snapshot is None → Unsupported; if no stored record → Unknown; else compare via platform-specific `compare_snapshots`. _(notes: state determined at probe time, not persistence time)_
- **compare-macos-fsevents-then-mtime** — `crates/blit-core/src/change_journal/snapshot.rs:35-48` — fsid mismatch → Changes; equal event_id → NoChanges; else fall back to root_mtime equality. _(notes: relies on FSEventsGetCurrentEventId; gracefully degrades to mtime)_
- **compare-linux-device-inode-ctime** — `crates/blit-core/src/change_journal/snapshot.rs:50-63` — Device or inode change → Changes; ctime sec+nsec equal → NoChanges; else fall back to mtime. _(notes: ctime captures metadata changes too — broader than mtime)_
- **compare-windows-volume-usn** — `crates/blit-core/src/change_journal/snapshot.rs:65-79` — Volume serial change → Changes; journal_id change → Changes; equal `next_usn` → NoChanges; else mtime fallback. _(notes: rejects across-volume comparisons even if same logical content)_
- **schema-version-1-to-2-migration** — `crates/blit-core/src/perf_history.rs:273-300` — `migrate_record` derives `run_kind` from `options.dry_run` + `fast_path == "null_sink"`; preserves `mode`. _(notes: load-bearing — pre-v2 records lack run_kind on wire and would default to `Real` via serde — that's wrong for dry-run records, so explicit migration overrides)_
- **predictor-fallback-chain-depth-0-3** — `crates/blit-core/src/perf_predictor.rs:239-326` — `predict()` walks 4 ProfileKey candidates: depth 0 = exact (src_fs, dest_fs, fast_path, skip_unchanged, checksum); depth 1 drops fast_path; depth 2 also drops dest_fs; depth 3 mode-only. Each candidate gated by `MIN_OBSERVATIONS_FOR_CONFIDENCE` (5). _(notes: callers requiring high confidence should refuse `fallback_depth >= 3`; documented contract)_

### default-value

- **auto-tune-chunk-bytes-thresholds** — `crates/blit-core/src/auto_tune/mod.rs:27-31` — `analyze_warmup_result`: ≥6 Gbps → 32 MiB; else 16 MiB. _(notes: hardcoded thresholds; magic numbers in code)_
- **auto-tune-stream-count-tiers** — `crates/blit-core/src/auto_tune/mod.rs:46-67` — Initial streams: >8 Gbps→6; >3 Gbps→4; else 2. TCP buf and prefetch follow same 3-tier scale (8 MiB/32, 4 MiB/16, 1 MiB/8). _(notes: `max_streams = 8` hardcoded regardless of bandwidth)_
- **local-plan-tuning-bucket-targets** — `crates/blit-core/src/auto_tune/mod.rs:116-166` — Defaults: small target 8 MiB / 2048 count; medium target 128 MiB. History-derived overrides clamped to [4MiB,128MiB]/[128,4096]/[64MiB,512MiB] ranges. _(notes: hardcoded clamp ranges; only counts `run_kind.is_real_transfer()` records — R56-F1 invariant)_
- **predictor-default-coefficients** — `crates/blit-core/src/perf_predictor.rs:36-52` — `DEFAULT_ALPHA_MS_PER_FILE=0.05`, `DEFAULT_BETA_MS_PER_MB=0.01`, `DEFAULT_GAMMA_MS=50.0`, `LEARNING_RATE=0.0005`, `MIN_COEFFICIENT=0.000001`, `MIN_OBSERVATIONS_FOR_CONFIDENCE=5`. _(notes: all hardcoded; no runtime tuning)_
- **perf-history-default-max-bytes** — `crates/blit-core/src/perf_history.rs:17-18` — `DEFAULT_MAX_BYTES = 1_000_000` (~1 MiB). _(notes: per design docs; not configurable)_

### error-propagation

- **error-category-three-buckets** — `crates/blit-core/src/errors.rs:11-19,89-118` — `ErrorCategory`: Retryable / Fatal / NoRetry. `categorize_io_error` maps `io::ErrorKind` to categories: TimedOut/Interrupted/ConnReset/ConnAborted/BrokenPipe/WouldBlock → Retryable; PermissionDenied/NotFound/InvalidData/InvalidInput/AlreadyExists → Fatal; WriteZero/UnexpectedEof/AddrInUse/etc → Fatal (conservative); unknown → Fatal. _(notes: "default to fatal to avoid infinite loops" — explicit policy)_
- **transfer-error-retry-counter** — `crates/blit-core/src/errors.rs:34-75` — `TransferError` carries `attempts: u8`, `should_retry(max)` returns true only if Retryable AND attempts < max; `with_attempt()` saturating_add. _(notes: saturating add — no overflow; per-error counter not shared across error instances)_
- **logger-poisoned-mutex-recovery** — `crates/blit-core/src/logger.rs:34-48` — `TextLogger::line` handles `Mutex::lock().Err(poison)` by `poison.into_inner()` and still writes. _(notes: tolerates panic-poisoned mutex; could mask correctness issues)_
- **mdns-advertise-drop-shutdown-errors** — `crates/blit-core/src/mdns.rs:87-96` — `MdnsAdvertiser::Drop` unregisters with 1-sec recv_timeout; logs shutdown errors via `warn!` but doesn't propagate. _(notes: standard cleanup pattern)_

### timeout-or-retry

- **mdns-discover-poll-200ms** — `crates/blit-core/src/mdns.rs:194-219` — `discover()` polls `recv_timeout(min(remaining, 200ms))` in a loop until `timeout` reached; rejects zero timeout. _(notes: 200ms hardcoded; cooperative cancellation via timeout check at top of loop)_
- **mdns-advertiser-drop-1s-recv-timeout** — `crates/blit-core/src/mdns.rs:88-90` — Unregister waits up to 1 sec for confirmation in Drop. _(notes: hardcoded 1-sec; long-running daemon shutdowns wait at most 1s per advertiser)_

### discovery

- **mdns-service-type-constant** — `crates/blit-core/src/mdns.rs:14-15` — `BLIT_SERVICE_TYPE = "_blit._tcp.local."` _(notes: exported constant)_
- **mdns-hostname-suffix-handling** — `crates/blit-core/src/mdns.rs:113-126` — mdns-sd 0.19 enforces `.local.` suffix; code appends if missing. Bare hostname → `host.local.`; `host.local` → `host.local.`. _(notes: tolerates legacy 0.8 lenient format)_
- **mdns-txt-truncation-180-bytes** — `crates/blit-core/src/mdns.rs:263-281` — `truncate_modules` enforces ~180-byte TXT record cap with "...(+N more)" suffix when overflow. _(notes: mDNS TXT record practical size cap; magic number 180)_
- **mdns-module-count-vs-modules** — `crates/blit-core/src/mdns.rs:42-65,140-156` — Distinct accessors: `modules()` returns parsed list (truncated past 180 bytes); `module_count()` returns authoritative u32 from `module_count` TXT — present only on §3.2 daemons (Optional). _(notes: two-source-of-truth pattern; explicit comment on backward compat)_
- **mdns-default-instance-name** — `crates/blit-core/src/mdns.rs:283-290` — Default is `blit@{hostname}`; falls back to `"blit"` if hostname unavailable. _(notes: hardcoded prefix)_

### format-output

- **logger-iso8601-timestamp** — `crates/blit-core/src/logger.rs:36,42` — Each line: `[{Utc::now().to_rfc3339()}] {payload}`. _(notes: RFC3339 timestamps; never localized; UTC)_
- **logger-event-payloads** — `crates/blit-core/src/logger.rs:51-79` — Fixed text-format events: `START src=… dst=…`, `COPY src=… dst=… bytes=…`, `ERROR ctx=… path=… msg=…`, `DONE files=… bytes=… seconds=…`. _(notes: not structured JSON; trailing fields use `{n:.3}` for seconds)_

### data-plane

- **fs-capability-trait-3-methods** — `crates/blit-core/src/fs_capability/mod.rs:36-45` — `FilesystemCapability` trait: `preserve_metadata`, `capabilities`, `fast_copy`. _(notes: minimal surface; platform impls return `FastCopyResult::Fallback` if OS primitive unavailable)_
- **macos-fast-copy-clonefile-then-fcopyfile** — `crates/blit-core/src/fs_capability/macos.rs:83-103` — First tries `clonefile()` (APFS CoW), then `fcopyfile()` with ACL+STAT+XATTR+DATA flags; otherwise Fallback. _(notes: sequential attempts; both return `Result<bool>` but logic ignores Err and just falls through)_
- **unix-fast-copy-copy-file-range-then-sendfile** — `crates/blit-core/src/fs_capability/unix.rs:88-107` — Linux only: tries `copy_file_range` syscall in `i32::MAX`-chunked loop; then `sendfile` chunked the same way. _(notes: bypasses fs-type capability flags — even if probe says no copy_file_range, code still tries it)_
- **windows-fast-copy-copyfileex** — `crates/blit-core/src/fs_capability/windows.rs:108-121,229-256` — Single attempt: `CopyFileExW`; reads dst metadata len. _(notes: stub `try_copyfileex` on non-windows panics with `bail!` but path is gated by `#[cfg(windows)]`)_
- **macos-metadata-mtime-then-perms** — `crates/blit-core/src/fs_capability/macos.rs:36-77` — Sets file mtime via filetime; preserves Unix permissions via PermissionsExt; xattrs hardcoded to `false`. _(notes: xattrs comment "would need xattr crate dependency" — TODO-ish but no explicit TODO marker)_
- **unix-metadata-mtime-perms-owner-group** — `crates/blit-core/src/fs_capability/unix.rs:36-82` — mtime + Unix perms + best-effort `fchown(uid,gid)` (requires privileges). _(notes: fchown silently fails for unprivileged callers — preserved boolean reflects that)_
- **windows-metadata-mtime-readonly-only** — `crates/blit-core/src/fs_capability/windows.rs:64-101` — Sets mtime; copies read-only attribute; ACLs hardcoded `false` ("would require Windows API") — never preserved. _(notes: no ACL preservation; xattrs/streams not handled)_

### naming

- **fs-capability-fs-type-strings** — `crates/blit-core/src/fs_capability/probe.rs:27-149` — Match arms hardcode lowercase fs-type strings: `"apfs"`, `"hfs"`, `"hfs+"`, `"btrfs"`, `"xfs"`, `"ext4/3/2"`, `"zfs"`, `"tmpfs"`, `"nfs"`, `"nfs4"`, `"cifs"`, `"smbfs"`, `"ntfs"`, `"refs"`. Each maps to a `Capabilities` struct. _(notes: long list; no enum; adding fs requires touching this match)_
- **linux-fs-magic-numbers** — `crates/blit-core/src/fs_capability/probe.rs:236-251` — Maps `statfs.f_type` magic numbers to fs-name strings (0x9123683E=btrfs, 0x58465342=xfs, 0xEF53=ext4 — covers ext2/3/4, 0x2FC12FC1=zfs, 0x01021994=tmpfs, 0x6969=nfs, 0xFF534D42=cifs, 0x5346544E=ntfs, etc). Unknown → `format!("unknown(0x{:X})", f_type)` string. _(notes: magic numbers without named constants; unknown-string approach prevents detection failures from being silent)_
- **windows-fs-name-refs-case-insensitive** — `crates/blit-core/src/fs_capability/windows.rs:225-227` — `is_refs_filesystem` does `eq_ignore_ascii_case("ReFS")` — handles `REFS`, `refs`, `ReFS`. _(notes: case-insensitive match; sibling probe.rs match arm uses lowercase only)_

### spawn-task

- **mdns-daemon-arc-shared** — `crates/blit-core/src/mdns.rs:81-96,110` — `MdnsAdvertiser` holds `Arc<ServiceDaemon>`; daemon shared with mdns-sd internal threads. Drop unregisters + shuts down. _(notes: implicit thread lifecycle managed by mdns-sd crate; Arc lets daemon outlive scope if cloned)_

### flag-handling

- **perf-history-runtime-toggle** — `crates/blit-core/src/perf_history.rs:422-431` — `perf_history_enabled()` reads persisted settings; `set_perf_history_enabled(bool)` writes. Default `true`. _(notes: toggle via CLI `blit diagnostics perf --enable/--disable`)_

## Smells / risks observed

- **Duplicated canonicalize-and-walk loops** — `path_safety.rs:188-208`, `230-250`, `283-306` repeat the same "loop { canonicalize(probe); pop on NotFound; bail on other err }" pattern three times. Inconsistency risk if one is updated without the others (e.g. logging, error message, max-depth limit).
- **Cache-with-no-invalidation** — `BLOCK_CLONE_CACHE` (`fs_capability/windows.rs:31`) and the global probe cache (`fs_capability/mod.rs:88`) hold per-volume/per-device entries for the life of the process. If a volume is unmounted/remounted with different capabilities (e.g. a USB drive reattached), stale entries persist. No invalidation API.
- **Non-atomic persistence writes** — `change_journal/tracker.rs:84-110`, `perf_history.rs:404-419`, `perf_predictor.rs:476-484`, `perf_history.rs:487-491` all use `File::create(path)` + write without temp-rename + fsync. Crash mid-write corrupts the file. Acceptable for telemetry/cache, but predictor state loss = 30-day training reset.
- **Silent fallthrough on parse errors** — `ChangeTracker::load` (tracker.rs:23-33) prints to stderr and starts fresh; `PerformancePredictor::load_state_from_path` (perf_predictor.rs:564-580) propagates parse errors as `Err`. Inconsistent posture between sibling subsystems.
- **Windows canonicalize is lexical-only** — `change_journal/util.rs:12-19` uses `normpath::BasePath` on Windows (no symlink resolution) while Unix uses `std::fs::canonicalize` (resolves symlinks). Same call name, divergent semantics — `path_safety::contained_join` correctness on Windows hinges on this not crossing the wire path layer.
- **Hardcoded magic constants without named symbols** — auto_tune chunk sizes (8/3 Gbps thresholds, 32/16/8 MiB), TCP buffer sizes (8/4/1 MiB), prefetch counts (32/16/8), `max_streams = 8`, predictor `LEARNING_RATE = 0.0005`, mDNS `MAX_LEN = 180`, `DEFAULT_MAX_BYTES = 1_000_000`. Some are documented constants, others inline.
- **`config::config_dir` re-exported via `perf_history`** — `perf_history.rs:357-359` defines its own `pub fn config_dir()` that just forwards to `crate::config::config_dir`. Predictor depends on `perf_history::config_dir`, not the root one. Removing the indirection would simplify the dependency graph.
- **`run_kind` field not serialized as authoritative tag** — `perf_history.rs:140-147` carries `#[serde(default)]`, so a v1 record explicitly marked as null-sink lacks the lane on the wire and migration must re-derive from `fast_path`. A future record where the user manually edits the file and sets a wrong `run_kind` overrides the inference.
- **`looks_like_windows_absolute` doesn't handle UNC `//`-twice in same prefix variants** — `path_safety.rs:344-363` checks `s.starts_with("\\\\")` and `s.starts_with("//")` for UNC, but a path like `\/server\share` (mixed) is not caught. Marginal attack surface; tests don't cover this mixed form.
- **`Component::CurDir` silently stripped** — `path_safety.rs:110-112` strips `.` components without bookkeeping. The post-strip empty-check (line 125) is the only safety net distinguishing legitimate empty from `"./."`.
- **macOS `xattrs` preservation perpetually `false`** — `fs_capability/macos.rs:72-74` says "would need xattr crate dependency" — effectively a TODO without the marker. The `Capabilities::xattrs: true` flag advertises support that `preserve_metadata` doesn't implement.
- **Windows ACL preservation perpetually `false`** — `fs_capability/windows.rs:97-99` — same pattern as xattrs on macOS. Advertised in `Capabilities::acls: true`, but `preserve_metadata().acls = false` always.
- **Per-volume block-clone cache keyed only on serial number** — `fs_capability/windows.rs:31` — collision risk if two volumes happen to share serial (rare but documented Windows quirk for cloned VHDs).
- **`fs_capability` cache uses `dev()` on Unix but not on Windows** — `fs_capability/mod.rs:97-117` divergent code paths; on non-Unix `cached_probe` short-circuits straight to `probe_capabilities` (no cache benefit). Block-clone has its own cache in `windows.rs`. Two caches, two designs.
- **`detect_filesystem_type_impl` Windows special-cases nonexistent paths** — `fs_capability/probe.rs:255-266` explicitly returns `None` for missing paths to match Linux semantics, while the underlying `volume_info_for_path` would otherwise walk to ancestor. Behavioral matching done at the wrapper level rather than the impl level — subtle divergence.
- **fsid computation on macOS uses unsafe transmute** — `change_journal/snapshot.rs:104-114` `std::mem::transmute(statfs_info.f_fsid)` from `fsid_t` to `[i32; 2]`. Documented as macOS-only and verified, but `transmute` is the heaviest unsafe primitive available.
- **`Settings` struct only carries one field** — `perf_history.rs:365-377` — `Settings { perf_history_enabled: bool }`. New settings would need to share the file or duplicate the load/store machinery.
- **No `#[derive(Default)]` for `Capabilities`** — `fs_capability/mod.rs:58-71` — three platform-impl copies of an effective "default Capabilities" exist (probe.rs:152-195 has 3 cfg branches). A single derived default + explicit overrides would be DRYer.
- **`relative_str_to_posix` calls `Path::new` on user input** — `path_posix.rs:64-78` — for the trailing-sep detection it inspects the raw `&str`, but `Path::new(s)` underneath uses OS-native semantics. On Windows, `Folder\` is split; on POSIX it's a single 7-byte filename. Documented and tested but subtle.
- **`MdnsDiscoveredService` re-exposes `properties` as a public `HashMap<String,String>`** — `mdns.rs:18-26` — TXT key collisions would silently overwrite. Multi-line TXT values not handled.

## Coverage attestation

| File | Lines read | Notes |
|---|---:|---|
| `crates/blit-core/src/lib.rs` | 54 | full read |
| `crates/blit-core/src/config.rs` | 43 | full read |
| `crates/blit-core/src/errors.rs` | 154 | full read |
| `crates/blit-core/src/logger.rs` | 80 | full read |
| `crates/blit-core/src/auto_tune/mod.rs` | 359 | full read (entry is dir, single mod.rs) |
| `crates/blit-core/src/fs_capability/mod.rs` | 117 | full read (entry is dir, recursive enumerated) |
| `crates/blit-core/src/fs_capability/probe.rs` | 334 | full read |
| `crates/blit-core/src/fs_capability/macos.rs` | 173 | full read |
| `crates/blit-core/src/fs_capability/unix.rs` | 208 | full read |
| `crates/blit-core/src/fs_capability/windows.rs` | 313 | full read |
| `crates/blit-core/src/change_journal/mod.rs` | 10 | full read |
| `crates/blit-core/src/change_journal/snapshot.rs` | 277 | full read |
| `crates/blit-core/src/change_journal/tracker.rs` | 115 | full read |
| `crates/blit-core/src/change_journal/types.rs` | 63 | full read |
| `crates/blit-core/src/change_journal/util.rs` | 47 | full read |
| `crates/blit-core/src/mdns.rs` | 360 | full read |
| `crates/blit-core/src/perf_history.rs` | 781 | full read |
| `crates/blit-core/src/perf_predictor.rs` | 1368 | full read |
| `crates/blit-core/src/path_safety.rs` | 830 | full read |
| `crates/blit-core/src/path_posix.rs` | 198 | full read |

**Total lines read**: 5884
**Files NOT read**: none
