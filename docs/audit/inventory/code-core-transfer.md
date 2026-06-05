# Code Inventory: blit-core remote transfer pipeline
**Generated**: 2026-06-04 by audit workflow
**Coverage**: full read of every file under `crates/blit-core/src/remote/{transfer/, push/}` plus `pull.rs`, `endpoint.rs`, `mod.rs` (17 source files).

| File | Lines |
|------|-------|
| `crates/blit-core/src/remote/mod.rs` | 10 |
| `crates/blit-core/src/remote/endpoint.rs` | 417 |
| `crates/blit-core/src/remote/pull.rs` | 2167 |
| `crates/blit-core/src/remote/push/mod.rs` | 5 |
| `crates/blit-core/src/remote/push/data_plane.rs` | 1 (re-export shim) |
| `crates/blit-core/src/remote/push/payload.rs` | 1 (re-export shim) |
| `crates/blit-core/src/remote/push/client/mod.rs` | 1133 |
| `crates/blit-core/src/remote/push/client/helpers.rs` | 293 |
| `crates/blit-core/src/remote/push/client/types.rs` | 21 |
| `crates/blit-core/src/remote/transfer/mod.rs` | 27 |
| `crates/blit-core/src/remote/transfer/data_plane.rs` | 801 |
| `crates/blit-core/src/remote/transfer/diff_planner.rs` | 474 |
| `crates/blit-core/src/remote/transfer/operation_spec.rs` | 364 |
| `crates/blit-core/src/remote/transfer/payload.rs` | 386 |
| `crates/blit-core/src/remote/transfer/pipeline.rs` | 956 |
| `crates/blit-core/src/remote/transfer/progress.rs` | 126 |
| `crates/blit-core/src/remote/transfer/sink.rs` | 2071 |
| `crates/blit-core/src/remote/transfer/source.rs` | 805 |
| `crates/blit-core/src/remote/transfer/stall_guard.rs` | 141 |
| `crates/blit-core/src/remote/transfer/tar_safety.rs` | 450 |

## Behaviors (grouped by category)

### endpoint-parse

- **endpoint-parse-empty-bail** — `crates/blit-core/src/remote/endpoint.rs:27-46` — `RemoteEndpoint::parse` trims input, bails on empty, and routes through `check_local_path` to distinguish local-path / backslash-form / not-local before parsing. _(notes: local-path detection is a hand-rolled heuristic with several special cases — drive letters, UNC, leading `.`/`/`/`\`/`~`)_
- **endpoint-parse-root-form** — `crates/blit-core/src/remote/endpoint.rs:48-58` — `://` triggers root-export shape (`RemotePath::Root`); host[:port] before `://`, remainder is normalized relative path.
- **endpoint-parse-module-form** — `crates/blit-core/src/remote/endpoint.rs:61-87` — `:/` triggers module shape; module must have trailing `/` separator and a non-empty name, rest is rel_path.
- **endpoint-parse-discovery-fallback** — `crates/blit-core/src/remote/endpoint.rs:90-96` — Bare `host` or `host:port` → `RemotePath::Discovery`. _(notes: per memory `feedback_endpoint_parse_err`, Err must reject — current implementation does)_
- **endpoint-parse-ipv6-bracketed** — `crates/blit-core/src/remote/endpoint.rs:167-182` — IPv6 literal parsed via `[host]:port`, host stored bracket-less but re-bracketed in `control_plane_uri()` and `display_host()`.
- **endpoint-default-port** — `crates/blit-core/src/remote/endpoint.rs:25` — Hard-coded `DEFAULT_PORT: u16 = 9031`.
- **endpoint-display-drops-default-port** — `crates/blit-core/src/remote/endpoint.rs:121-128, 131-159` — `host_port_display()` and `display()` omit `:9031` when port is default; preserve other ports verbatim.
- **endpoint-backslash-helpful-error** — `crates/blit-core/src/remote/endpoint.rs:37-44, 268-285` — `server:\module\path` → bailout with helpful "use forward slashes" message; distinguished from genuinely local Windows paths by hostname heuristic (`before_colon.len() > 1` AND no slashes).
- **endpoint-path-posix-delegation** — `crates/blit-core/src/remote/endpoint.rs:215-218` — `rel_path_to_string` delegates to `crate::path_posix::relative_path_to_posix` for slash normalization on every platform.

### path-handling

- **rel-path-posix-canonical** — `crates/blit-core/src/remote/transfer/payload.rs:214-219`, `crates/blit-core/src/remote/push/client/helpers.rs:52-57` — `normalize_relative_path` routes through `path_posix::relative_path_to_posix` so a literal `\` in a POSIX filename is preserved (the historical `replace('\\', "/")` was destructive). _(notes: duplicated identical helper in push/client/helpers.rs and transfer/payload.rs)_
- **tar-rel-string-posix-canonical** — `crates/blit-core/src/remote/transfer/tar_safety.rs:132-139` — Same POSIX delegation applied to tar entry paths before manifest lookup.
- **safe-join-contained-chokepoint** — `crates/blit-core/src/remote/transfer/sink.rs:190-205, 463-481, 651-657, 696-702` — `FsTransferSink::resolve_destination` runs `safe_join_contained` (canonical containment) when canonical root captured, else falls back to lexical `safe_join` with a `log::warn!`. _(notes: 4 nearly-identical fallback ladders across sink.rs sites — `write_file_payload`, `write_tar_shard_payload`, `write_file_block_payload`, `write_file_block_complete` — duplicated pattern)_
- **canonical-dest-root-capture** — `crates/blit-core/src/remote/transfer/sink.rs:144-162`, `crates/blit-core/src/remote/pull.rs:279-280, 663` — `canonical_dest_root` captured once at construction/entry via `path_safety::canonical_dest_root`, best-effort (`.ok()`).
- **resolve-pull-dest-empty-single-file** — `crates/blit-core/src/remote/pull.rs:1741-1747` — Empty `relative_path` means "write to dest_root directly" (single-file pull) — explicit guard avoids `dest_root.join("")` producing trailing-slash form that `File::create` rejects as ENOTDIR.
- **fs-source-open-empty-rel** — `crates/blit-core/src/remote/transfer/source.rs:91-106` — `FsTransferSource::open_file` mirrors the same empty-rel-path special case: uses `self.root.clone()` rather than `join("")`.
- **build-tar-shard-empty-rel** — `crates/blit-core/src/remote/transfer/payload.rs:347-385` — `build_tar_shard` does the same single-file-root special-case before `File::open`.
- **filter-readable-headers-empty-rel** — `crates/blit-core/src/remote/push/client/helpers.rs:200-211` — Same single-file empty-rel special case in push availability check. _(notes: pattern duplicated four times — source.rs, payload.rs, helpers.rs, pull.rs all guard against `root.join("")` ENOTDIR)_
- **wire-path-validate-chokepoint** — `crates/blit-core/src/remote/transfer/pipeline.rs:358-373`, `:386-401`, `crates/blit-core/src/remote/pull.rs:1790-1793` — Every wire-supplied path routed through `path_safety::validate_wire_path` (rejects `..`, absolute, drive prefix, UNC, NUL). Defense-in-depth — sinks re-validate via `safe_join`.
- **filtered-source-single-file-basename-fallback** — `crates/blit-core/src/remote/transfer/source.rs:421-440` — Empty `relative_path` for single-file push uses the source root's basename for filter matching, so `--include '*.txt'` works against the root's actual filename. _(notes: R59 finding #4 regression area)_

### timeout-or-retry

- **pull-stall-timeout-const** — `crates/blit-core/src/remote/transfer/stall_guard.rs:29` — Hard-coded `PULL_STALL_TIMEOUT: Duration = 30s`. Owner-decided per project memory `audit-owner-decisions`.
- **stall-guard-idle-not-deadline** — `crates/blit-core/src/remote/transfer/stall_guard.rs:51-79` — `StallGuard<R>` re-arms deadline on every read that returns (data OR clean EOF) — idle timeout, not total-duration cap. Trips with `io::ErrorKind::TimedOut` after `timeout` of no progress.
- **stall-guard-wraps-pull-receiver** — `crates/blit-core/src/remote/pull.rs:1712-1720` — Every data-plane pull TCP socket wrapped in `StallGuard(stream, PULL_STALL_TIMEOUT)` before `execute_receive_pipeline`.
- **control-plane-connect-timeout** — `crates/blit-core/src/remote/push/client/mod.rs:299-317`, `crates/blit-core/src/remote/pull.rs:230-248` — Both push and pull control-plane `connect()` are bounded by both `Endpoint::connect_timeout(30s)` AND an outer `tokio::time::timeout(30s)` (audit-2). _(notes: comment explains outer timeout is needed because tonic/hyper-util resolve DNS before `connect_timeout` engages)_
- **no-retry-policy** — `crates/blit-core/src/remote/transfer/data_plane.rs:60-115` — `DataPlaneSession::connect` makes a single connection attempt; no retry/backoff loop in core. _(notes: callers above expect retry to happen elsewhere)_

### data-plane

- **tcp-nodelay-on-data-plane** — `crates/blit-core/src/remote/transfer/data_plane.rs:78-82` — `socket.set_tcp_nodelay(true)` — hard requirement, propagates errors.
- **tcp-keepalive-best-effort** — `crates/blit-core/src/remote/transfer/data_plane.rs:88-90` — `set_keepalive(true)` failures only `log::warn!` rather than propagate.
- **tcp-buffer-size-best-effort** — `crates/blit-core/src/remote/transfer/data_plane.rs:92-103` — `set_send_buffer_size`/`set_recv_buffer_size` failures only logged.
- **data-plane-token-handshake** — `crates/blit-core/src/remote/transfer/data_plane.rs:109-112`, `crates/blit-core/src/remote/pull.rs:1672-1675` — Both push and pull data planes write the negotiation token immediately after connect, before any record framing.
- **dp-record-tags** — `crates/blit-core/src/remote/transfer/data_plane.rs:15-19` — Record type tags: `FILE=0`, `TAR_SHARD=1`, `BLOCK=2`, `BLOCK_COMPLETE=3`, `END=0xFF`. Used symmetrically on send (data_plane.rs) and receive (pipeline.rs).
- **dp-receive-end-tag** — `crates/blit-core/src/remote/transfer/pipeline.rs:214-215` — Unknown tag = `bail!` with `0x{:02X}` format.
- **send-file-wire-shape** — `crates/blit-core/src/remote/transfer/data_plane.rs:200-249` — `send_file_from_reader`: `[tag:1][path_len:4 be][path][size:8 be][mtime:8 be i64][perms:4 be]` + raw file bytes.
- **double-buffered-send** — `crates/blit-core/src/remote/transfer/data_plane.rs:256-335` — Two-buffer overlap of disk reads with network writes via `tokio::join!`.
- **double-buffered-send-clamp** — `crates/blit-core/src/remote/transfer/data_plane.rs:286-291, 313-317` — audit-11: clamps `bytes_a` / `bytes_b` to `remaining` so an over-returning reader (file grew, or lying source) can't underflow `remaining` (debug-panic / release-wrap) or push undeclared bytes onto the wire.
- **double-buffered-receive** — `crates/blit-core/src/remote/transfer/data_plane.rs:551-615` — `receive_stream_double_buffered` symmetric mirror of send-side. Reads `expected` bytes only; `read_up_to` (619-632) caps per-read at `min(buf.len(), cap)`.
- **receive-chunk-default** — `crates/blit-core/src/remote/transfer/data_plane.rs:528` — `RECEIVE_CHUNK_SIZE: usize = 1 MiB`; comment notes 8 KiB caps throughput at ~1 Gbps even with 10 GbE/14 Gbps disk.
- **control-plane-chunk-default** — `crates/blit-core/src/remote/transfer/data_plane.rs:14` — `CONTROL_PLANE_CHUNK_SIZE: usize = 1 MiB`.
- **send-prepared-tar-shard-wire** — `crates/blit-core/src/remote/transfer/data_plane.rs:337-412` — `[tag][count:u32 be]{[path_len:u32 be][path][size:u64 be][mtime:i64 be][perms:u32 be]}*[tar_size:u64 be][tar_bytes]`.
- **send-block-wire** — `crates/blit-core/src/remote/transfer/data_plane.rs:416-463` — `[tag][path_len:u32][path][offset:u64][block_len:u32][content]`.
- **send-block-complete-wire** — `crates/blit-core/src/remote/transfer/data_plane.rs:466-516` — `[tag][path_len][path][total_size:u64][mtime:i64][perms:u32]`. Carries mtime+perms inline so receiver can stamp even when zero blocks transferred.
- **receive-pipeline-dispatch** — `crates/blit-core/src/remote/transfer/pipeline.rs:200-302` — `execute_receive_pipeline` reads tag, dispatches to `write_file_stream` / `write_payload` (tar shard / FileBlock / FileBlockComplete); finally `sink.finish()`.
- **receive-pipeline-file-take** — `crates/blit-core/src/remote/transfer/pipeline.rs:226-228` — Uses `AsyncReadExt::take(file_size)` to give the sink exactly the declared bytes of the wire (canonical tokio limit pattern).
- **bytes-sent-counter** — `crates/blit-core/src/remote/transfer/data_plane.rs:27, 139, 149, 461` — `DataPlaneSession.bytes_sent` updated with `saturating_add` after every file/tar-shard/block.

### state-machine

- **push-transfer-mode-undecided** — `crates/blit-core/src/remote/push/client/types.rs:14-19` — `TransferMode { Undecided, DataPlane, Fallback }` — initial state, then negotiation chooses.
- **push-event-loop-biased-select** — `crates/blit-core/src/remote/push/client/mod.rs:430-700` — `tokio::select! { biased; response_rx, manifest_rx }`; response always wins to drain need-lists before pumping more manifest.
- **push-need-lists-done-terminator** — `crates/blit-core/src/remote/push/client/mod.rs:444-454, 877-888` — Empty `FilesToUpload` is a terminator signal — sets `need_lists_done`, does NOT `continue` (fall through so early-finish check fires on this iteration).
- **push-data-plane-early-finish** — `crates/blit-core/src/remote/push/client/mod.rs:877-888` — Data plane closes when `need_lists_done && pending_queue empty && manifest_done && data_plane_outstanding == 0 && data_plane_files_sent >= files_requested.len()`.
- **push-grpc-fallback-finish** — `crates/blit-core/src/remote/push/client/mod.rs:858-875` — Fallback `UploadComplete` sent through a transient `GrpcFallbackSink::new(...).finish()`. _(notes: hard-coded label `PathBuf::from("grpc-fallback")`)_
- **manifest-send-task-spawn** — `crates/blit-core/src/remote/pull.rs:704-740` — Pull side: local manifest pushed in a separate spawned task so response-stream draining isn't blocked. Wrapped in `AbortOnDrop`.
- **active-shard-protocol-guard** — `crates/blit-core/src/remote/pull.rs:836-907, 1244-1252` — `InProgressShard` state must close cleanly via `TarShardComplete`; ending the stream with `Some(_)` is a `bail!` (R6-F2).
- **active-file-protocol-guard** — `crates/blit-core/src/remote/pull.rs:348-358, 822-833` — `FileData` without a preceding `FileHeader` → `bail!`.

### cancellation

- **abort-on-drop-wrapper** — `crates/blit-core/src/remote/pull.rs:31-68` — RAII `AbortOnDrop<T>` for `tokio::spawn` so a cancelled outer future aborts (not detaches) the inner task. `Drop::drop` calls `handle.abort()`; `.join()` mutably borrows the handle out of `self` but keeps `self` alive across the await (R34-F2).
- **abort-on-drop-applied-to-data-plane** — `crates/blit-core/src/remote/pull.rs:371-377, 912-918, 1604-1631` — All spawned data-plane receivers wrapped in `AbortOnDrop`.
- **abort-on-drop-applied-to-manifest-send** — `crates/blit-core/src/remote/pull.rs:712-740` — Manifest-send task also wrapped.
- **pipeline-dispatcher-drops-sink-senders** — `crates/blit-core/src/remote/transfer/pipeline.rs:134-146` — Dispatcher drops `sink_senders` on payload_rx close so sink workers see EoS and call `finish()`.
- **stall-guard-deadline-reset** — `crates/blit-core/src/remote/transfer/stall_guard.rs:58-65` — On every read that returns (including clean EOF) the idle deadline is reset; only mid-pending state can trip the timeout.

### confirmation-prompt

- **none-in-cluster** — — No interactive confirmation prompts in this cluster. CLI confirm-Y/n flows live above this layer (per project memory `project_audit_decisions`, clear-recent Y/n confirm is a CLI concern not core).

### rpc-handler

- **remote-push-client-connect** — `crates/blit-core/src/remote/push/client/mod.rs:298-317` — Builds `tonic::transport::Endpoint` from `control_plane_uri()`, doubles up `connect_timeout` + outer `timeout`.
- **remote-pull-client-connect** — `crates/blit-core/src/remote/pull.rs:229-248` — Same connection shape as push (deliberate duplication for symmetry).
- **push-bidi-stream-spawn-response** — `crates/blit-core/src/remote/push/client/mod.rs:347-353`, `crates/blit-core/src/remote/push/client/helpers.rs:236-260` — Response stream parsed in a separate task draining into `mpsc::channel(32)`.
- **pull-sync-bidi-stream** — `crates/blit-core/src/remote/pull.rs:680-697` — Opens `pull_sync` bidi stream FIRST, then sends `Spec`, then drains responses. Channel capacity 32. _(notes: comment documents historical deadlock — for >30 entries the prior code (send manifest first, open stream second) deadlocked because the gRPC server wasn't consuming)_
- **pull-checksum-mismatch-rejects** — `crates/blit-core/src/remote/pull.rs:764-779` — F11: if user passed `--checksum` and daemon `server_checksums_enabled=false`, return `PullSyncError::Negotiation` BEFORE any data flows (avoids silent degrade to size+mtime).
- **pull-sync-error-phases** — `crates/blit-core/src/remote/pull.rs:80-113, 752-758` — `PullSyncError::Negotiation` vs `Transfer` toggled on first successful response; preserves phase across `eyre::Report` for delegation callers.
- **grpc-fallback-sink-outbound** — `crates/blit-core/src/remote/transfer/sink.rs:926-1073` — `GrpcFallbackSink` streams `FileManifest`/`FileData`/`TarShardHeader`/`TarShardChunk`/`TarShardComplete`/`UploadComplete` as `ClientPushRequest`s; rejects FileBlock/FileBlockComplete (outbound only).
- **grpc-server-streaming-sink** — `crates/blit-core/src/remote/transfer/sink.rs:1090-1231` — Daemon-side mirror; emits `ServerPullMessage`. _(notes: nearly identical body to GrpcFallbackSink — File/TarShard arms 90% identical, just different payload enum)_
- **remote-file-stream-adapter** — `crates/blit-core/src/remote/pull.rs:1162-1215` — Wraps `Streaming<PullChunk>` as `AsyncRead`. Skips non-data payloads, recurses through them.
- **scan-remote-files-force-grpc** — `crates/blit-core/src/remote/pull.rs:461-494` — Sets `force_grpc=true` + `metadata_only=true` so headers come back on the gRPC control stream. _(notes: comment claims "Force gRPC to get headers in the control stream")_

### spawn-task

- **manifest-task-blocking** — `crates/blit-core/src/remote/push/client/helpers.rs:91-181` — `spawn_blocking` walks source via `FileEnumerator::enumerate_local_streaming_capturing`. Uses `manifest_tx.blocking_send`. _(notes: R46-F2 captures suppressed walk errors into `unreadable_paths` so mirror-deletion can refuse on incomplete scan)_
- **payload-prepare-blocking** — `crates/blit-core/src/remote/transfer/payload.rs:43-65` — `prepare_payload` runs `build_tar_shard` in `spawn_blocking`; `task::JoinError` mapped to `eyre!`.
- **sink-worker-spawn** — `crates/blit-core/src/remote/transfer/pipeline.rs:91-129` — Each sink runs in its own spawned task with per-sink channel capacity `prefetch.max(1)`. Errors aggregated with "first wins" semantics.
- **sink-worker-spawn-blocking-for-local** — `crates/blit-core/src/remote/transfer/sink.rs:258-297` — `FsTransferSink::write_payload` for File/TarShard wraps the actual write in `spawn_blocking` (the zero-copy cascade + tar extraction use `std::fs`).
- **rayon-parallel-tar-extract** — `crates/blit-core/src/remote/transfer/sink.rs:600-625` — Tar-shard write uses `rayon::into_par_iter` to parallelize many-small-files write; per-task best-effort mtime/perms.

### safety-check

- **scan-completeness-gate-spec-v2** — `crates/blit-core/src/remote/transfer/operation_spec.rs:42-47, 86-92, 131-142` — `SUPPORTED_SPEC_VERSION = 2` adds `require_complete_scan` field. v1 daemons fail closed (the bump forces this) — historical move-with-incomplete-scan deletion bug.
- **ignore-existing-vs-force-contradictory** — `crates/blit-core/src/remote/transfer/operation_spec.rs:119-121` — `from_spec` rejects `ignore_existing=true + compare_mode=Force` as contradictory.
- **unknown-enum-rejected** — `crates/blit-core/src/remote/transfer/operation_spec.rs:160-179` — Unknown `compare_mode`/`mirror_mode` integer values rejected via `try_from` rather than silently picking a default.
- **glob-pre-validate** — `crates/blit-core/src/remote/transfer/operation_spec.rs:186-208` — Every include/exclude glob validated via `globset::Glob::new` so a malformed pattern is a hard error here rather than silently dropped (R5-F4).
- **tar-shard-max-bytes** — `crates/blit-core/src/remote/transfer/tar_safety.rs:50` — `MAX_TAR_SHARD_BYTES: u64 = 256 MiB` (single source of truth shared by wire-frame caps and per-entry alloc bounds).
- **tar-extract-rejects-non-regular** — `crates/blit-core/src/remote/transfer/tar_safety.rs:122-130` — `EntryType::Directory` skipped; anything other than `Regular`/`Continuous` rejected (no symlink/hardlink/device).
- **tar-extract-size-mismatch-pre-alloc** — `crates/blit-core/src/remote/transfer/tar_safety.rs:147-162` — Tar-header `entry.size()` must equal manifest `FileHeader.size` AND must not exceed cap, before any allocation.
- **tar-extract-bounded-alloc** — `crates/blit-core/src/remote/transfer/tar_safety.rs:173-181` — `Vec::try_reserve_exact` with `with_context` for the error path; no panic on pathological size.
- **tar-extract-exact-headers** — `crates/blit-core/src/remote/transfer/tar_safety.rs:214-217` — `require_exact_headers=true` (default) bails if any header in the manifest wasn't seen in the tar (R6-F2 family).
- **remote-tar-validate-shard-sizes** — `crates/blit-core/src/remote/transfer/source.rs:123-148` — Per-entry + cumulative size bounds for the remote→remote relay's tar-shard build; uses `checked_add` to defend against u64 overflow.
- **remote-tar-read-bounded** — `crates/blit-core/src/remote/transfer/source.rs:162-208` — `read_remote_entry_bounded` wraps reader with `take(expected_size + 1)` so over-read is detected at +1 byte (R11-F1 — previously `read_to_end` could grow the Vec past the bound).
- **wire-path-len-cap** — `crates/blit-core/src/remote/transfer/pipeline.rs:325, 341-356` — `MAX_WIRE_PATH_LEN = 64 KiB` cap on wire path strings to bound allocations from a hostile peer.
- **wire-tar-file-count-cap** — `crates/blit-core/src/remote/transfer/pipeline.rs:329, 378-385` — `MAX_WIRE_TAR_SHARD_FILES = 1_048_576` cap on file count per tar shard.
- **wire-tar-bytes-cap-unified** — `crates/blit-core/src/remote/transfer/pipeline.rs:335-336, 403-408` — `MAX_WIRE_TAR_SHARD_BYTES` aliases `tar_safety::MAX_TAR_SHARD_BYTES` so the wire reader rejects shards the helper would. _(notes: comment cites F8 of 2026-05-01 review fixing prior 1 GiB vs 256 MiB inconsistency)_
- **wire-block-bytes-cap** — `crates/blit-core/src/remote/transfer/pipeline.rs:339, 256-262` — `MAX_WIRE_BLOCK_BYTES = 64 MiB` cap on single resume block.
- **pull-archive-size-cap** — `crates/blit-core/src/remote/pull.rs:838-857` — `TarShardHeader.archive_size > MAX_TAR_SHARD_BYTES` rejected before any allocation. Initial capacity is `min(archive_size, 1 MiB, MAX_TAR_SHARD_BYTES)` so allocation never trusts the wire size.
- **pull-shard-chunk-overflow-guard** — `crates/blit-core/src/remote/pull.rs:863-878` — Each `TarShardChunk` arrival checks `new_total <= declared_size` AND `<= MAX_TAR_SHARD_BYTES`.
- **pull-shard-final-size-check** — `crates/blit-core/src/remote/pull.rs:886-894` — On `TarShardComplete`, buffer length must equal declared size, else `bail!`.
- **block-hash-block-size-cap** — `crates/blit-core/src/remote/pull.rs:1117-1129` — `compute_block_hashes` rejects `block_size > MAX_BLOCK_SIZE`.
- **canonical-escape-rejection-symlink** — `crates/blit-core/src/remote/transfer/sink.rs:572-585` — Tar-shard writes verify_contained against canonical root for every extracted entry (R47-F1). Pre-existing dst→/outside symlinks are caught.
- **path-tracker-thread-safe-mutex** — `crates/blit-core/src/remote/transfer/sink.rs:207-213` — `path_tracker.lock()` is best-effort; poisoned mutex silently drops the push (rare but quiet failure mode).

### error-propagation

- **sink-error-aggregation-first-wins** — `crates/blit-core/src/remote/transfer/pipeline.rs:149-165` — Sink-worker errors aggregated with "first wins"; later worker errors dropped silently.
- **pipeline-streaming-surfaces-underlying-error** — `crates/blit-core/src/remote/push/client/mod.rs:175-197` — When `tx.send` fails the producer drops `payload_tx` and drains `pipeline_handle` via `drain_pipeline_error` so the real cause (sink error / disk full / channel close) surfaces — replaces previous generic "data plane pipeline closed unexpectedly". POST_REVIEW_FIXES §1.1b.
- **drain-pipeline-outcome-wrapping** — `crates/blit-core/src/remote/push/client/mod.rs:62-91` — Shared helper wraps `Err` with "data plane pipeline failed:" prefix; `JoinError` becomes "data plane pipeline panicked:".
- **drain-clean-close-race-diagnostic** — `crates/blit-core/src/remote/push/client/mod.rs:82-91` — Special case: pipeline `Ok` but producer saw channel closed → diagnostic "closed cleanly but the producer channel was already closed — likely a race".
- **eof-mid-file-bail** — `crates/blit-core/src/remote/transfer/data_plane.rs:277-283, 306-312`, `crates/blit-core/src/remote/transfer/payload.rs:271-277`, `crates/blit-core/src/remote/transfer/sink.rs:988-994` — Every send loop bails on `read = 0` with "unexpected EOF while reading ... ({remaining} bytes remaining)".
- **best-effort-metadata-warn-not-fail** — `crates/blit-core/src/remote/transfer/sink.rs:415-417, 425-429, 514-523, 609-621`, `crates/blit-core/src/remote/transfer/tar_safety.rs:240-253` — `set_file_mtime` and `set_permissions` failures uniformly downgraded to `log::warn!` (post review §1.1: must be visible, not swallowed silently).
- **tokio-file-flush-propagates** — `crates/blit-core/src/remote/transfer/sink.rs:396-398` — Flush failure is propagated, not swallowed (data-loss signal — user believes file durable when it isn't).
- **no-sync-all-on-write-file-stream** — `crates/blit-core/src/remote/transfer/sink.rs:402-407` — Intentionally NO `sync_all` because ZFS-style fsync is multi-second on spinning rust; relies on END marker + OS flush.
- **sync-all-on-block-complete** — `crates/blit-core/src/remote/transfer/sink.rs:713-716` — Resume-block finalization DOES `sync_all` — divergent from `write_file_stream`. _(notes: inconsistent — write_file_stream skips sync_all "for ZFS throughput" but block-complete forces it; no comment explains why)_
- **flush-active-pull-file** — `crates/blit-core/src/remote/pull.rs:1217-1229` — `finalize_active_file` calls `file.sync_all()` for the legacy gRPC-receive file path.
- **unreadable-tracking** — `crates/blit-core/src/remote/push/client/helpers.rs:183-191`, `crates/blit-core/src/remote/push/client/mod.rs:905-920` — Permission/notfound errors recorded into `unreadable_paths`; at end of push, presence of any entries fails the whole transfer with a summary message.

### naming

- **rename-payloadsink-suffix-double** — `crates/blit-core/src/remote/push/data_plane.rs`, `payload.rs` — Single-line files re-export from `remote::transfer::data_plane::*` / `payload::*`. _(notes: stub shim — likely vestigial post-refactor, could be removed but keeps `push::data_plane` import path stable)_

### default-value

- **default-payload-prefetch** — `crates/blit-core/src/remote/transfer/payload.rs:108` — `DEFAULT_PAYLOAD_PREFETCH: usize = 8`.
- **fs-sink-config-defaults** — `crates/blit-core/src/remote/transfer/sink.rs:100-110` — `preserve_times: true`, `dry_run: false`, `checksum: None`, `resume: false`, `compare_mode: SizeMtime`.
- **tar-shard-extract-defaults** — `crates/blit-core/src/remote/transfer/tar_safety.rs:67-74` — Default: `max_entry_bytes=MAX_TAR_SHARD_BYTES`, `require_exact_headers=true`.
- **null-sink-label-dev-null** — `crates/blit-core/src/remote/transfer/sink.rs:847-852` — Hard-coded `/dev/null` label PathBuf.
- **compare-mode-unspecified-folds-to-size-mtime** — `crates/blit-core/src/remote/transfer/operation_spec.rs:160-167`, `crates/blit-core/src/remote/transfer/diff_planner.rs:119-120` — `Unspecified` → `SizeMtime` (historical default).
- **mirror-mode-unspecified-folds-to-off** — `crates/blit-core/src/remote/transfer/operation_spec.rs:170-179` — `Unspecified` → `Off` (the safe default).
- **default-block-size** — `crates/blit-core/src/remote/pull.rs:1117-1121` — `block_size == 0` folds to `DEFAULT_BLOCK_SIZE`.
- **chunk-bytes-min-clamp** — `crates/blit-core/src/remote/transfer/data_plane.rs:47-48` — `chunk_bytes.max(64 KiB)` lower bound on data-plane chunk size.
- **buffer-size-floor** — `crates/blit-core/src/remote/transfer/data_plane.rs:566` — `receive_stream_double_buffered` floors buffer to 64 KiB.

### flag-handling

- **force-grpc-vs-data-plane-mode** — `crates/blit-core/src/remote/push/client/mod.rs:411-418` — `force_grpc=true` → `transfer_mode = TransferMode::Fallback` straight away.
- **force-grpc-scan-metadata-only** — `crates/blit-core/src/remote/pull.rs:470-473, 509-512` — `scan_remote_files` / `open_remote_file` always set `force_grpc=true`.
- **dry-run-must-be-side-effect-free** — `crates/blit-core/src/remote/transfer/sink.rs:343-364, 484-491, 538-544` — Dry-run drains wire for protocol alignment but skips mkdir and write. `bytes_written = 0`. (R58-F4 — pre-fix the mkdir ran before dry-run check.) _(notes: comment in write_file_stream warns "do NOT report against byte_progress" so daemon counter doesn't advance for an aborted preview)_
- **delete-all-scope-vs-filtered-subset** — `crates/blit-core/src/remote/pull.rs:569-580` — `mirror_mode=true` + `delete_all_scope=false` → `FilteredSubset`; with scope → `All`.
- **track-paths-opt-in** — `crates/blit-core/src/remote/pull.rs:1696-1703`, `crates/blit-core/src/remote/transfer/sink.rs:168-171` — `track_paths` enables the path tracker (used by mirror purge).
- **preserve-times-strip-mtime** — `crates/blit-core/src/remote/transfer/sink.rs:589-594` — If `preserve_times=false`, `mtime` stripped from extracted files (helper would otherwise apply).
- **require-complete-scan-on-move** — `crates/blit-core/src/remote/transfer/operation_spec.rs:86-92`, `crates/blit-core/src/remote/pull.rs:153-161` — Independent of mirror; set by `blit move` to prevent source-side EACCES → silent source-delete data loss.

### persistence

- **byte-progress-arc-atomic** — `crates/blit-core/src/remote/transfer/progress.rs:28-67` — `ByteProgressSink` wraps `Arc<AtomicU64>`; `from_counter` lets daemon row-counter and sink share the atomic. `Relaxed` ordering.
- **byte-progress-reported-after-write** — `crates/blit-core/src/remote/transfer/data_plane.rs:587-591` — Reported AFTER `write_all` succeeds so `bytes_completed` observed by GetState never exceeds bytes actually written (memory `feedback_getstate_counters_zero` is related).
- **byte-progress-reported-on-write-payload** — `crates/blit-core/src/remote/transfer/sink.rs:310-312` — c-1b round 2: tar shards / resume blocks land via `write_payload` not `write_file_stream`, so byte counter must be bumped here too (chunk-granular hook would otherwise miss them).
- **null-sink-no-byte-progress** — `crates/blit-core/src/remote/transfer/sink.rs:894-903` — `--null` benchmark bytes never land on user disk; explicitly does NOT advance byte_progress.
- **dry-run-no-byte-progress** — `crates/blit-core/src/remote/transfer/sink.rs:345-358` — Same logic: dry-run drain doesn't bump byte_progress.
- **instrumentation-callout-construction** — `crates/blit-core/src/remote/transfer/source.rs:226-228` — `RemoteTransferSource::new` records a constructed-instance counter via `crate::remote::instrumentation::record_remote_transfer_source_constructed` (visibility for the relay path).
- **instrumentation-outbound-bytes** — `crates/blit-core/src/remote/transfer/data_plane.rs:302, 330, 402, 459` — Every send-side wire write feeds `crate::remote::instrumentation::record_cli_data_plane_outbound_bytes`.

### render-or-display

- **enumerated-progress-stderr** — `crates/blit-core/src/remote/push/client/helpers.rs:148-153, 172-176` — Manifest enumeration progress and completion lines go to **stderr** (R46-F4 — stdout reserved for `--json` structured output).
- **need-list-trace-stderr** — `crates/blit-core/src/remote/push/client/mod.rs:464-466, 543-561, 681-685, 798-822` — `eprintln!("[push] need-list includes {}", ...)` / `[push] enqueue` / `[push] daemon did not request N file(s); skipping` all to stderr.
- **data-plane-trace-stderr** — `crates/blit-core/src/remote/transfer/data_plane.rs:30-36` — `trace_client!` macro emits `[data-plane-client]` lines to stderr only if `trace=true`. _(notes: hard-coded prefix string)_
- **push-skipping-red-eprintln** — `crates/blit-core/src/remote/push/client/helpers.rs:183-191` — Uses `owo_colors::OwoColorize::red()` for the "[push] skipping ..." line. _(notes: color-only feedback bypasses `--no-color` policy)_
- **aggregate-throughput-gbps-stderr** — `crates/blit-core/src/remote/push/client/mod.rs:214-222`, `crates/blit-core/src/remote/pull.rs:1647-1653` — Final throughput logs `Gbps` and `MiB` to stderr at finish.
- **pull-data-plane-completed-without-summary-warning** — `crates/blit-core/src/remote/pull.rs:414-416` — Diagnostic if data plane completes without summary payload.

### discovery

- **discovery-path-bails-in-spec-build** — `crates/blit-core/src/remote/pull.rs:541-544`, `crates/blit-core/src/remote/push/client/helpers.rs:284-293` — `RemotePath::Discovery` bails with "remote source must specify a module" in both push (`module_and_path`) and pull (`build_spec_from_options`).

### format-output

- **destination-path-join-iter** — `crates/blit-core/src/remote/push/client/helpers.rs:262-271`, `crates/blit-core/src/remote/pull.rs:1795-1804` — `destination_path` / `normalize_for_request` iterate `path.iter()` and join components with `/` for wire formatting.

### config-load

- **tuning-determine-remote** — `crates/blit-core/src/remote/push/client/mod.rs:226-248` — `ensure_remote_tuning` lazily calls `determine_remote_tuning(size_hint)` and applies `chunk_bytes_override` to `plan_options`. _(notes: implicit assumption that first sizing decides for the whole transfer)_

## Smells / risks observed

- **Duplicated empty-rel single-file guard across four files** — `source.rs:91-106`, `payload.rs:355-359`, `helpers.rs:200-211`, `pull.rs:1741-1747` all special-case `relative_path == ""` to avoid `root.join("")` ENOTDIR. Same comment text repeated. Could be folded into a shared helper.
- **Duplicated POSIX-normalization helpers** — `transfer/payload.rs:214-219`, `push/client/helpers.rs:52-57` both wrap `path_posix::relative_path_to_posix` under identical names. Both still document the historical destructive `replace('\\', "/")` bug.
- **Duplicated canonical-fallback ladder** — `sink.rs:190-205`, `:463-481`, `:651-657`, `:696-702` all do `match canonical_dst_root { Some => safe_join_contained, None => log::warn!() + safe_join }`. Could be a free helper.
- **`sync_all` divergence on file finalization** — `write_file_stream` (sink.rs:402-407) explicitly skips `sync_all` for ZFS throughput, but `write_file_block_complete` (sink.rs:713-716) calls it. `finalize_active_file` (pull.rs:1221-1228) also calls `sync_all`. No comment explains the inconsistency.
- **`GrpcServerStreamingSink` ↔ `GrpcFallbackSink` near-identical bodies** — Both ~150 LOC of `match payload { File / TarShard / FileBlock-reject }`. Differ only in payload enum (`ClientPushRequest` vs `ServerPullMessage`). Ripe for a generic helper.
- **Hard-coded label strings** — `"grpc-fallback"` (push/client/mod.rs:871, 970), `"/dev/null"` (sink.rs:850), `"[data-plane-client]"`, `"[push]"`, `"[pull-data-plane]"`, `"[pull]"` prefixes. Not const-ified.
- **Manifest poisoned-mutex silent drop** — `helpers.rs:188-190` and `pull.rs:905-907` use `if let Ok(...) = guard.lock()` so a panic-poisoned mutex silently drops the data. Quiet failure mode.
- **Sink error aggregation drops later worker errors** — `pipeline.rs:149-165` only surfaces first error; with N=8 streams a cascade of disk-full errors collapses to one.
- **`SinkOutcome::merge` uses `+=` not `saturating_add`** — `sink.rs:33-36` — Could overflow on extreme byte counts (though u64).
- **Wire receive of zero-length token not checked** — `data_plane.rs:109-112`, `pull.rs:1672-1675` write `token` unconditionally without checking it's non-empty.
- **`#[allow(dead_code)]` on `ByteProgressSink::new`** — `progress.rs:37` — Indicates the constructor is only exercised by tests; daemon uses `from_counter`. Could be `#[cfg(test)]`-gated.
- **Stub re-export files** — `push/data_plane.rs` and `push/payload.rs` are 1-line `pub use ...::*` shims. Vestigial.
- **`color::red()` bypass-able output** — `helpers.rs:183-191` uses `OwoColorize::red()` unconditionally, no `--no-color` respect at this layer (caller's CLI config could be honored but isn't threaded through).
- **`force_grpc` semantics overloaded** — On pull this also implies `metadata_only=true` for scan, but the push call site (`mod.rs:411-418`) uses it as a strict mode lock. Two different meanings.
- **`#[allow(clippy::too_many_arguments)]`** — `push/client/mod.rs:109-110` — `MultiStreamSender::connect` takes 11 positional args. Reasonable candidate for a config struct.
- **No graceful handler for `tokio::io::ErrorKind::TimedOut` from StallGuard at the pipeline layer** — `execute_receive_pipeline` (pipeline.rs:200-302) treats the error like any other; test `receive_pipeline_aborts_on_stall` (pipeline.rs:925-955) confirms it surfaces but no retry/diagnostic upgrade.
- **R59 finding #4 single-file-basename fallback** — `source.rs:421-440` is the only site that handles the edge case. A new source impl that emits `relative_path == ""` would silently break filtering again.
- **Spec `from_spec` accepts version exact-equal, no migration** — `operation_spec.rs:107-113` rejects any non-exact version. No backwards compat planned, but a v1 daemon hitting v2 client fails with a generic "unsupported spec_version" — not a fail-closed action message.
- **No TODO/FIXME/HACK/XXX comments in cluster** — Searched, none found. (Comment-level hygiene is unusually clean.)
- **No `cfg(target_os)` divergence** — Only `cfg(unix)` / `cfg(not(unix))` for `PermissionsExt`-based perms (sink.rs:422-432, tar_safety.rs:245-253). No Windows-specific path code.
- **No references to removed features (blit-utils, BlitAuth, AI telemetry)** — Cluster is clean of these.
- **Pipeline streaming pipeline panic-safety** — `pipeline.rs:155-159` converts `JoinError` to `eyre!("sink worker panicked: {}", join)`. Other join sites in the cluster (push `mod.rs:891-893`, `pull.rs:401-404`) use the same conversion.

## Coverage attestation

| File | Lines read | Notes |
|------|------------|-------|
| `crates/blit-core/src/remote/mod.rs` | 1-10 (full) | tiny re-export module |
| `crates/blit-core/src/remote/endpoint.rs` | 1-417 (full) | with tests |
| `crates/blit-core/src/remote/pull.rs` | 1-2167 (full) | read in 3 segments: 1-700, 700-1400, 1400-end |
| `crates/blit-core/src/remote/push/mod.rs` | 1-5 (full) | re-exports |
| `crates/blit-core/src/remote/push/data_plane.rs` | 1 (full) | shim |
| `crates/blit-core/src/remote/push/payload.rs` | 1 (full) | shim |
| `crates/blit-core/src/remote/push/client/mod.rs` | 1-1133 (full) | read in 2 segments |
| `crates/blit-core/src/remote/push/client/helpers.rs` | 1-293 (full) | — |
| `crates/blit-core/src/remote/push/client/types.rs` | 1-21 (full) | — |
| `crates/blit-core/src/remote/transfer/mod.rs` | 1-27 (full) | re-exports |
| `crates/blit-core/src/remote/transfer/data_plane.rs` | 1-801 (full) | — |
| `crates/blit-core/src/remote/transfer/diff_planner.rs` | 1-474 (full) | — |
| `crates/blit-core/src/remote/transfer/operation_spec.rs` | 1-364 (full) | — |
| `crates/blit-core/src/remote/transfer/payload.rs` | 1-386 (full) | — |
| `crates/blit-core/src/remote/transfer/pipeline.rs` | 1-956 (full) | — |
| `crates/blit-core/src/remote/transfer/progress.rs` | 1-126 (full) | — |
| `crates/blit-core/src/remote/transfer/sink.rs` | 1-2071 (full) | read in 3 segments |
| `crates/blit-core/src/remote/transfer/source.rs` | 1-805 (full) | — |
| `crates/blit-core/src/remote/transfer/stall_guard.rs` | 1-141 (full) | — |
| `crates/blit-core/src/remote/transfer/tar_safety.rs` | 1-450 (full) | — |

**Total lines read**: 10619
**Files NOT read** (with reason): none.
