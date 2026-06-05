# Code Inventory: blit-daemon — every RPC + service module
**Generated**: 2026-06-04 by audit workflow
**Coverage**: 17 files under `crates/blit-daemon/src/` + `Cargo.toml` (12,489 lines total)

## File list

| File | Lines |
|---|---:|
| `crates/blit-daemon/Cargo.toml` | 36 |
| `crates/blit-daemon/src/main.rs` | 147 |
| `crates/blit-daemon/src/net_timeout.rs` | 41 |
| `crates/blit-daemon/src/recents_store.rs` | 177 |
| `crates/blit-daemon/src/metrics.rs` | 302 |
| `crates/blit-daemon/src/runtime.rs` | 499 |
| `crates/blit-daemon/src/active_jobs.rs` | 2009 |
| `crates/blit-daemon/src/delegation_gate.rs` | 877 |
| `crates/blit-daemon/src/service/mod.rs` | 22 |
| `crates/blit-daemon/src/service/util.rs` | 168 |
| `crates/blit-daemon/src/service/core.rs` | 2537 |
| `crates/blit-daemon/src/service/admin.rs` | 850 |
| `crates/blit-daemon/src/service/pull.rs` | 1038 |
| `crates/blit-daemon/src/service/pull_sync.rs` | 1147 |
| `crates/blit-daemon/src/service/push/mod.rs` | 5 |
| `crates/blit-daemon/src/service/push/control.rs` | 520 |
| `crates/blit-daemon/src/service/push/data_plane.rs` | 1133 |
| `crates/blit-daemon/src/service/delegated_pull.rs` | 981 |

## Behaviors (grouped by category)

### rpc-handler
- **rpc-push-dispatch** — `crates/blit-daemon/src/service/core.rs:457-546` — `push` handler: extracts peer, increments `metrics.inc_push()`, takes `enter_transfer()` RAII guard, registers `ActiveJob` with EMPTY module/path (filled by handler via `set_endpoint`), emits `TransferStarted` event, then spawns `handle_push_stream`. After completion: records outcome, builds terminal event, drops guard BEFORE broadcast so subscriber-observable state has `active[]` already drained. _(notes: ordering invariant — drop guard before metrics.log_completion so `active=` reflects post-RPC state)_
- **rpc-pull-dispatch** — `crates/blit-daemon/src/service/core.rs:548-599` — `pull` handler: synchronously resolves module (returns NotFound before spawn), increments `metrics.inc_pull()`. Registers `ActiveJob` with populated module/path (unlike push/pull_sync). Honors both `req.force_grpc` AND service `force_grpc_data`.
- **rpc-pull-sync-dispatch** — `crates/blit-daemon/src/service/core.rs:601-661` — `pull_sync` handler: increments `inc_pull()` (NOT a separate pull_sync counter), registers with EMPTY module/path. _(notes: counter conflation — pull_sync attempts roll into `pull_operations` total. delegated_pull also calls inc_pull in `run_delegated_pull`)_
- **rpc-delegated-pull-dispatch** — `crates/blit-daemon/src/service/core.rs:663-843` — Three-way `tokio::select!` via `resolve_delegated_pull_outcome`: handler-first biased select races handler completion vs `tx.closed()` (client hangup, disabled by `detach=true`) vs `cancel_token.cancelled()` (CancelJob). Maps outcome `None` to either "cancelled via CancelJob" (if token fired) or "client cancelled". _(notes: audit-10 fix — handler ordered first so a transfer that completed at the same instant CancelJob fired isn't mis-recorded as cancelled. R30-F2 client-hangup race. m-jobs-3 detach gating)_
- **rpc-list** — `crates/blit-daemon/src/service/core.rs:845-914` — `list` handler: resolves module, treats empty path as `.`, runs `fs::metadata`+`read_dir` in `spawn_blocking`. Returns sorted `FileInfo` for dirs; single-element list for files. Rejects symlinks/other types with InvalidArgument.
- **rpc-purge** — `crates/blit-daemon/src/service/core.rs:916-936` — `purge` handler: ALWAYS increments `inc_purge()` at dispatch boundary (F5 fix — was previously post-success), then runs `purge_inner` which resolves module, rejects read-only, sanitizes paths, deletes. Emits `--metrics` completion line.
- **rpc-complete-path** — `crates/blit-daemon/src/service/core.rs:938-971` — `complete_path` handler: requires `include_files || include_directories` (InvalidArgument otherwise), splits prefix via `split_completion_prefix`, runs `list_completions` in `spawn_blocking`.
- **rpc-list-modules** — `crates/blit-daemon/src/service/core.rs:973-988` — `list_modules` handler: returns sorted `ModuleInfo` from the modules map (no default-root synthesis).
- **rpc-get-state** — `crates/blit-daemon/src/service/core.rs:990-1091` — `get_state` handler: returns `version` from `CARGO_PKG_VERSION`, `uptime_seconds` from `started_at`, sorted modules, snapshot of `active_jobs`, recent ring (truncated to `recent_limit` from the FRONT when nonzero — oldest-first preserved), and Counters reading atomics with Relaxed ordering. `delegation_enabled` mirrors the master switch. _(notes: per feedback-getstate-counters-zero memory, Counters is ALWAYS Some even when metrics disabled — all-zero values; the doc-comment at the field site explicitly says "Reserved for future GUI/TUI" but the RPC publishes them now)_
- **rpc-cancel-job** — `crates/blit-daemon/src/service/core.rs:1093-1129` — `cancel_job` handler: captures `remote_addr()` BEFORE consuming request (audit-9), rejects empty id with InvalidArgument, dispatches `cancel_authorized` returning {Cancelled→Ok, Unsupported→FailedPrecondition, NotFound→NotFound, Unauthorized→PermissionDenied}.
- **rpc-clear-recent** — `crates/blit-daemon/src/service/core.rs:1138-1146` — `clear_recent` handler: calls `active_jobs.clear_recent()`, returns count removed as u32. Empty request type. _(notes: per memory project_recent_persistence — must wipe ONLY recents ring, never planner's perf_local.jsonl)_
- **rpc-disk-usage** — `crates/blit-daemon/src/service/core.rs:1148-1196` — `disk_usage` handler: spawns a task that wraps `stream_disk_usage` in `spawn_blocking`. `max_depth==0` → `None` (unbounded). Empty `start_path` → `.`.
- **rpc-find** — `crates/blit-daemon/src/service/core.rs:1198-1261` — `find` handler: requires `include_files || include_directories`, `max_results==0` → `None` (unbounded), empty start_path → `.`. Runs `stream_find_entries` in `spawn_blocking`.
- **rpc-filesystem-stats** — `crates/blit-daemon/src/service/core.rs:1263-1272` — `filesystem_stats`: resolves module, runs `filesystem_stats_for_path` (sync; reads `sysinfo::Disks`).
- **rpc-subscribe** — `crates/blit-daemon/src/service/core.rs:353-455` — `subscribe` handler: atomically obtains broadcast Receiver + ring snapshot via `subscribe_with_ring`. Spawns per-subscriber forwarder that races `tx.closed()` against `broadcast_rx.recv()` (c-5a round 3 — prevents leaked Receiver during quiet periods). Replays per-row ring events BEFORE live broadcast forwarding. Lagged → `Status::aborted` with re-subscribe hint.

### state-machine
- **active-jobs-register** — `crates/blit-daemon/src/active_jobs.rs:400-441` — `register()` inserts row + cancellation token + bytes counter + per-row event ring (capacity 64) under `std::sync::Mutex`. Returns `ActiveJobGuard` whose `Drop` synchronously removes row and pushes `TransferRecord` to recent ring. Sync (not async) deliberately — keeps `Drop` deterministic.
- **active-jobs-drop** — `crates/blit-daemon/src/active_jobs.rs:1012-1058` — `Drop` for `ActiveJobGuard`: takes outcome cell (defaults to `ok=false, "cancelled before outcome recorded"` if never recorded), removes row from table, builds `TransferRecord` with final byte count, pushes to recent ring (if `recent_limit > 0`), pings persistence channel via `try_send`. Lock order: table → recent (sequential, no nesting).
- **active-jobs-set-endpoint** — `crates/blit-daemon/src/active_jobs.rs:956-962` — `set_endpoint` updates module/path of LIVE row only; no-op if drained. Used by streaming RPCs (push, pull_sync) once header parsed.
- **emit-event-under-lock** — `crates/blit-daemon/src/active_jobs.rs:552-569` — `emit_event` pushes to per-row ring AND broadcasts under the SAME table lock. Ordering invariant ensures no event delivered to a `subscribe_with_ring` consumer twice or zero times.
- **tick-progress-emit** — `crates/blit-daemon/src/active_jobs.rs:675-711` — `tick_progress_emit` snapshots bytes counter, computes throughput `(delta_bytes * 1000) / delta_ms` (0 when delta_ms==0), pushes event into row's ring AND broadcasts — under the table lock so progress events for a transfer can never follow its terminal event.
- **cancel-authorized** — `crates/blit-daemon/src/active_jobs.rs:484-501` — `cancel_authorized` resolves: NotFound → Unsupported (kind doesn't support) → Unauthorized (peer IP mismatch) → Cancelled. The token is NEVER fired for the first three outcomes.
- **cancel-peer-authorized-rules** — `crates/blit-daemon/src/active_jobs.rs:1121-1132` — `cancel_peer_authorized`: no observable caller → allow (UDS); loopback caller → allow; otherwise caller IP must equal owner IP (port ignored); unparseable owner ("unknown") → deny non-loopback. _(notes: IP-only comparison is deliberate — ephemeral source port differs between cancel RPC and original transfer)_
- **supports-cancellation-policy** — `crates/blit-daemon/src/active_jobs.rs:162-164` — `ActiveJobKind::supports_cancellation` returns true ONLY for `DelegatedPull`. Push/Pull/PullSync have the CLI in the byte path, so client-side drop already cancels via `tx.closed()`.

### persistence
- **recents-arm-persistence** — `crates/blit-daemon/src/active_jobs.rs:777-804` — `arm_persistence` hydrates the in-memory ring from disk (oldest-first), installs the `OnceLock<UnboundedSender<()>>`, returns `RecentsWriter`. Unbounded channel so startup-window pings never lose persistence signals.
- **recents-writer-loop** — `crates/blit-daemon/src/active_jobs.rs:825-835` — `RecentsWriter::run`: receives signal, COALESCES via `try_recv` drain, then atomically rewrites the file with current ring contents. Logs to stderr on failure but doesn't abort.
- **recents-store-load** — `crates/blit-daemon/src/recents_store.rs:45-66` — `load`: missing file → empty Vec (never fails startup). Skips unparseable lines via `filter_map`. Trims to `limit` from the front (preserves newest).
- **recents-store-write-atomic** — `crates/blit-daemon/src/recents_store.rs:72-93` — `write_atomic`: writes to `.jsonl.tmp` sibling, calls `sync_all`, then `rename`. Creates parent dir if needed.
- **clear-recent-scope** — `crates/blit-daemon/src/active_jobs.rs:751-765` — `clear_recent`: empties ring, returns count, pings persistence channel. Doc-comment EXPLICITLY states never touches `perf_local.jsonl`. _(notes: tested in `clear_recent_empties_store_but_not_perf_local`)_

### config-load
- **runtime-config-precedence** — `crates/blit-daemon/src/runtime.rs:173-353` — `load_runtime`: CLI arg overrides config-file values for `bind`, `port`, `mdns_name`. Default port 9031, default bind `0.0.0.0`. Default config path `/etc/blit/config.toml` (Unix) or `C:\ProgramData\Blit\config.toml` (Windows).
- **module-config-canonicalization** — `crates/blit-daemon/src/runtime.rs:248-267` — Every module's path is canonicalized at load time (fails config load on bad path). `canonical_root` stored separately so push handler's mutation of `path` doesn't escape original boundary.
- **delegation-config-parse** — `crates/blit-daemon/src/runtime.rs:226-239` — Every `allowed_source_hosts` entry parsed at config load via `parse_allow_entry`. Invalid CIDR or empty entry fails LOUDLY (Phase 1 R23-F3 contract).
- **default-root-fallback** — `crates/blit-daemon/src/runtime.rs:269-340` — No modules + no root → exports cwd as "default" (warning emitted). Modules + --root → root is `default_root` but not added to modules map. Modules + no root → warning that `server://` requests will be rejected.
- **per-module-delegation-allowed** — `crates/blit-daemon/src/runtime.rs:157-158` — `delegation_allowed` defaults to `true` (via `default_true()` serde helper). Doc explicitly: can only NARROW daemon-wide policy, never widen.
- **server-checksums-flag-cascade** — `crates/blit-daemon/src/runtime.rs:216-220` — Server checksums: enabled by default. `--no-server-checksums` OR `daemon.no_server_checksums = true` in config disables.

### timeout-or-retry
- **http2-keepalive** — `crates/blit-daemon/src/main.rs:137-142` — HTTP/2 keepalive: 30s interval, 20s timeout. audit-1 fix to reap dead subscribers without disturbing healthy idle ones. _(notes: per feedback_keymap_collisions / project_audit_decisions — owner decision was keepalive, not idle-close)_
- **net-timeout-helper** — `crates/blit-daemon/src/net_timeout.rs:20-22` — `within(deadline, fut)`: thin wrapper over `tokio::time::timeout`, returns `Option<F::Output>` (None on timeout). Generic + error-type-free so callers map elapsed to domain error.
- **dns-resolve-timeout** — `crates/blit-daemon/src/delegation_gate.rs:266-282` — `StdResolver::resolve` wraps `tokio::net::lookup_host` in 10s deadline; elapsed returns `io::Error(TimedOut)`. _(notes: audit-1 — bounds the OS resolver timeout (5-30s+))_
- **source-connect-timeout** — `crates/blit-daemon/src/service/delegated_pull.rs:35-36` — `SOURCE_CONNECT_TIMEOUT = 30s` bounds dst→src TCP connect against firewalled/black-holed source.
- **push-data-plane-accept-timeout** — `crates/blit-daemon/src/service/push/data_plane.rs:67-74` — `DATA_PLANE_ACCEPT_TIMEOUT = 30s`, `DATA_PLANE_TOKEN_TIMEOUT = 15s`. R46-F7: pre-fix unbounded; a peer that opened control + never opened data would pin the stream task.
- **pull-accept-token-timeout** — `crates/blit-daemon/src/service/pull.rs:697-699,724-737` — `PULL_ACCEPT_TIMEOUT = 30s`, `PULL_TOKEN_TIMEOUT = 15s`. R47-F5.
- **pull-sync-accept-token-timeout** — `crates/blit-daemon/src/service/pull_sync.rs:596-598,721-735` — `PULL_SYNC_ACCEPT_TIMEOUT = 30s`, `PULL_SYNC_TOKEN_TIMEOUT = 15s`. R46-F7. Two constants of the same name re-declared in the same file (line 596 in `stream_via_data_plane` and line 719 in `stream_via_data_plane_resume`) — DUPLICATED constants.

### safety-check
- **delegation-gate-validate-source** — `crates/blit-daemon/src/delegation_gate.rs:288-392` — `validate_source` ordering: master switch → empty host → port 0 → IP-vs-hostname normalization → resolve once → IP-form authorization required for special ranges (loopback/link-local/unique-local/unspecified) → bound to FIRST validated IP (DNS-rebinding mitigation).
- **special-range-detection** — `crates/blit-daemon/src/delegation_gate.rs:204-220` — `is_special_range`: 127.0.0.0/8, 169.254.0.0/16, 0.0.0.0/8 unspecified, IPv6 ::1, fe80::/10, fc00::/7, ::.
- **ipv4-mapped-ipv6-normalize** — `crates/blit-daemon/src/delegation_gate.rs:191-199` — `::ffff:V4` flattened to IPv4 for both `is_special_range` and matching.
- **path-containment-push** — `crates/blit-daemon/src/service/push/control.rs:106-127` — Push header's `destination_path` is verified against `module.canonical_root` (not `module.path` — which is then MUTATED to include the subpath). F2/R13-F1.
- **path-containment-tar-shard** — `crates/blit-daemon/src/service/push/data_plane.rs:795-799` — Per-extracted-file containment check inside `apply_tar_shard_sync`, defending against pre-existing on-disk symlinks even after tar_safety rejected tar-encoded symlinks.
- **tar-symlink-rejection** — `crates/blit-daemon/src/service/push/data_plane.rs:783-815` — Push tar-shard receive routes through `safe_extract_tar_shard` which rejects every non-regular entry type (closes latent symlink-injection bug).
- **mirror-purge-containment** — `crates/blit-daemon/src/service/admin.rs:69-87` — `purge_extraneous_entries` verifies `module_path` itself against `canonical_root` BEFORE enumerating — defense in depth against post-handshake escape paths.
- **delete-rel-paths-per-entry-check** — `crates/blit-daemon/src/service/admin.rs:155-247` — `delete_rel_paths_sync` verifies each rel path against canonical_root BEFORE stat. `cfg(windows)` clears readonly attr recursively before `remove_file`.
- **sanitize-request-paths** — `crates/blit-daemon/src/service/admin.rs:27-44` — `sanitize_request_paths`: empty entry → InvalidArgument; refuses to delete module root (empty rel or `.`) — explicit guard, NOT just validation.
- **scope-deletions-mirror-mode** — `crates/blit-daemon/src/service/pull_sync.rs:424-463` — `scope_deletions`: Off|Unspecified → empty. All → candidates verbatim. FilteredSubset → keep only candidates whose client manifest entry passes `filter.allows_relative` (size/mtime gates). Closes F4.
- **disk-usage-start-containment** — `crates/blit-daemon/src/service/admin.rs:352-355` — `stream_disk_usage` verifies start path against module_root (start point can be symlink even when enumerator has `follow_symlinks=false`).
- **find-start-containment** — `crates/blit-daemon/src/service/admin.rs:489-498` — `stream_find_entries` mirrors disk-usage containment check.
- **delegation-handler-ordering** — `crates/blit-daemon/src/service/delegated_pull.rs:170-303` — `run_delegated_pull` order: (1) parse locator, (2) validate spec, (3) gate (DNS+allowlist), (4) resolve module, (5) per-module narrow, (6) read-only check, (7) F2 containment on dst, (8) metrics RAII, (9) caps override, (10) outbound connect (bounded), (11) `pull_sync_with_spec`. Gate runs BEFORE any module resolution or outbound connect.
- **apply-delete-list-canonical** — `crates/blit-daemon/src/service/delegated_pull.rs:445-502` — Mirror-purge delete uses `safe_join_contained` (canonical containment, not lexical) per R58-F3. Refuses delete-list referencing dest_root itself. Failures to canonicalize dest_root abort the whole operation.
- **delete-list-authorized** — `crates/blit-daemon/src/service/delegated_pull.rs:92-95` — Mirror-mode authorization gate: only `FilteredSubset` and `All` may apply a delete list. `Off`/`Unspecified` silently ignore source-attested deletes (R32-F1 — defends against hostile source).
- **require-complete-scan-pull-sync** — `crates/blit-daemon/src/service/pull_sync.rs:130-159` — Refuses pull_sync with FailedPrecondition when scan was incomplete AND (mirror_mode OR require_complete_scan). The latter is the `blit move` initiator's flag. _(notes: per memory project_audit_decisions and feedback_port_cli_safety_guards)_
- **require-complete-scan-push** — `crates/blit-daemon/src/service/push/control.rs:327-333` — Same guard on push: refuses to purge when client demanded complete-scan AND scan was incomplete. R59 #1 F1.

### cancellation
- **delegated-pull-cancel-race** — `crates/blit-daemon/src/service/core.rs:741-783` — Three-way race via `resolve_delegated_pull_outcome` with `biased` handler-first ordering. `tx.closed()` arm gated by `!detach`. _(notes: audit-10 was the round that made handler-completion win over simultaneous cancel)_
- **cancel-job-rejects-non-delegated** — `crates/blit-daemon/src/active_jobs.rs:459-470` — `cancel(transfer_id)` returns `Unsupported` for kinds whose `supports_cancellation()==false`. Token NOT fired.

### endpoint-parse
- **delegated-pull-locator-parse** — `crates/blit-daemon/src/service/delegated_pull.rs:170-188` — Source locator: missing → DelegationRejected. Empty host → DelegationRejected. Port `try_into u16` failure → DelegationRejected. No early classification before gate.
- **allow-entry-parser** — `crates/blit-daemon/src/delegation_gate.rs:141-168` — Order: CIDR (presence of `/`) → bare IP (with optional `[...]` brackets) → hostname (IDNA normalized). Empty entry rejected.

### default-value
- **default-config-path** — `crates/blit-daemon/src/runtime.rs:165-171` — Hardcoded `/etc/blit/config.toml` (Unix) vs `C:\ProgramData\Blit\config.toml` (Windows). _(notes: `cfg!(windows)` runtime check, not `#[cfg]`)_
- **default-port** — `crates/blit-daemon/src/runtime.rs:201` — `9031` hardcoded as port default.
- **default-bind-host** — `crates/blit-daemon/src/runtime.rs:200` — `"0.0.0.0"` hardcoded as bind default.
- **default-recent-limit** — `crates/blit-daemon/src/active_jobs.rs:102` — `DEFAULT_RECENT_LIMIT = 50` constant.
- **job-event-ring-cap** — `crates/blit-daemon/src/active_jobs.rs:114` — `JOB_EVENT_RING_CAP = 64` per-row event ring depth.
- **subscribe-broadcast-capacity** — `crates/blit-daemon/src/service/core.rs:41` — `SUBSCRIBE_BROADCAST_CAPACITY = 256` broadcast ring.
- **subscribe-mpsc-capacity** — `crates/blit-daemon/src/service/core.rs:53` — `SUBSCRIBE_MPSC_CAPACITY = 64` per-subscriber buffer.
- **progress-tick-ms** — `crates/blit-daemon/src/service/core.rs:61` — `DEFAULT_PROGRESS_TICK_MS = 100` (10 Hz progress ticker cadence).
- **push-batcher-limits** — `crates/blit-daemon/src/service/push/control.rs:25-31` — `FILE_LIST_BATCH_MAX_ENTRIES=16384`, `_MAX_BYTES=512KB`, `_MAX_DELAY=25ms`; early-flush thresholds `128 entries / 64KB / 5ms`.
- **upload-channel-cap** — `crates/blit-daemon/src/service/push/control.rs:31` — `FILE_UPLOAD_CHANNEL_CAPACITY = FILE_LIST_BATCH_MAX_ENTRIES * 16` (262144).
- **token-len** — `crates/blit-daemon/src/service/push/data_plane.rs:23` — `TOKEN_LEN = 32` for data-plane handshake.
- **max-parallel-tar-tasks** — `crates/blit-daemon/src/service/push/data_plane.rs:24` — `MAX_PARALLEL_TAR_TASKS = 4` semaphore depth.
- **tar-buffer-pool-defaults** — `crates/blit-daemon/src/service/push/data_plane.rs:27-29` — `TAR_BUFFER_SIZE = 4 MiB`, `TAR_BUFFER_POOL_SIZE = 8` (fallback path only).
- **enum-batch-size** — `crates/blit-daemon/src/service/pull.rs:31` — `ENUM_BATCH_SIZE = 500` files per streaming-enumeration batch.
- **min-bytes-for-tuning** — `crates/blit-daemon/src/service/pull.rs:34` — `MIN_BYTES_FOR_TUNING = 16 MiB` before kicking off data plane.
- **pull-stream-count-tiers** — `crates/blit-daemon/src/service/pull.rs:915-933` — Stream count: 32GiB→16, 8GiB→12, 2GiB→10, 512MiB→8, 128MiB→4, 32MiB→2, else 1. Clamped to `tuning_max`.
- **desired-streams-push-tiers** — `crates/blit-daemon/src/service/push/control.rs:499-520` — Push stream count tiers by bytes OR file count; same shape as pull but separate ladder for file_count thresholds. _(notes: thresholds duplicated across push/pull — not a shared constant)_
- **recents-filename** — `crates/blit-daemon/src/recents_store.rs:29` — `RECENTS_FILE = "recents.jsonl"` constant (in `config_dir`, alongside `perf_local.jsonl`).

### spawn-task
- **progress-ticker-task** — `crates/blit-daemon/src/service/core.rs:256-271` — `spawn_progress_ticker` runs interval with `MissedTickBehavior::Skip` (no catch-up burst after a pause).
- **recents-writer-task** — `crates/blit-daemon/src/active_jobs.rs:842-844` — `spawn_recents_writer` runs `RecentsWriter::run` for daemon lifetime; aborted at process exit.
- **subscribe-forwarder-task** — `crates/blit-daemon/src/service/core.rs:395-452` — Per-subscriber `tokio::spawn` forwarder. Replays ring events first, then races `tx.closed()` against `broadcast_rx.recv()`. Exits cleanly on disconnect.
- **handler-dispatch-spawns** — `crates/blit-daemon/src/service/core.rs:499,579,631,741` — Every transfer RPC spawns its handler so the dispatch boundary returns the response stream immediately. ActiveJob guard moves into the spawned task.

### data-plane
- **bind-data-plane-listener** — `crates/blit-daemon/src/service/push/data_plane.rs:38-42` — Hardcoded `0.0.0.0:0` (ephemeral port). Same call used by push, pull, pull_sync, pull_sync_resume.
- **data-plane-socket-tuning** — `crates/blit-daemon/src/service/push/data_plane.rs:110-120` — Per-stream socket: `set_tcp_nodelay(true)` + `set_keepalive(true)` via socket2 round-trip. Failures silently ignored (`let _ =`). _(notes: smell — TCP keepalive errors aren't surfaced; only push path applies this tuning, pull/pull_sync paths don't)_
- **generate-token** — `crates/blit-daemon/src/service/push/data_plane.rs:52-58` — Uses `rand::rngs::SysRng::try_fill_bytes`; maps RNG failure to `Status::Internal`. audit-3b — pre-fix this panicked.
- **token-validation-constant-time** — `crates/blit-daemon/src/service/push/data_plane.rs:182-186` — Token compared with `==` on Vec<u8>. NOT constant-time. _(notes: smell — timing-side-channel resistant comparison would use `subtle::ConstantTimeEq`. Tokens are 32 random bytes from OS RNG so practical risk is low.)_

### render-or-display
- **stderr-info-messages** — `crates/blit-daemon/src/main.rs:34-72` — Daemon emits `[warn]`, `[info]` messages directly to stderr (e.g. mDNS advertise success, delegation enabled). No structured logging framework.
- **stderr-aggregate-throughput** — `crates/blit-daemon/src/service/pull.rs:658-665,901-906` — Pull data plane prints aggregate `Gbps` to stderr. Two identical `eprintln!` blocks.
- **stderr-data-plane-events** — `crates/blit-daemon/src/service/push/data_plane.rs:121,183-191,238-242` — Multiple `[data-plane] ...` stderr lines for accept/token-accept/stream complete. Buffer-pool stats logged at end of transfer when `total_allocated > 0`.
- **metrics-completion-line** — `crates/blit-daemon/src/metrics.rs:107-131` — `log_completion` emits compact one-line `[metrics] op status duration (counters)` only when metrics enabled.

### format-output
- **transfer-id-format** — `crates/blit-daemon/src/active_jobs.rs:1089-1097` — `t<unix_ms>-<counter>` format. Counter via `fetch_add(Relaxed)` so race-safe but resets at daemon restart. _(notes: per code comment, durability across restart is "deferred per §10 open questions")_
- **peer-addr-string** — `crates/blit-daemon/src/service/core.rs:1278-1283` — Peer formatted as `<ip>:<port>` or `"unknown"` when `remote_addr()` is None (UDS / in-process tests).
- **pathbuf-to-display** — `crates/blit-daemon/src/service/util.rs:160-168` — Joins components with `/` regardless of platform (POSIX-style display).
- **normalize-relative-path** — `crates/blit-daemon/src/service/util.rs:153-158` — Delegates to `blit_core::path_posix::relative_path_to_posix` — explicit comment about historical `replace('\\', "/")` being destructive on POSIX.

### path-handling
- **resolve-relative-path** — `crates/blit-daemon/src/service/util.rs:51-67` — Request-path context: trims trailing `/`, folds empty / `.` to `PathBuf::from(".")`. Used by list/find/du request paths.
- **resolve-manifest-relative-path** — `crates/blit-daemon/src/service/util.rs:78-81` — Strict file-path validation: preserves empty as empty (NOT folded to `.`). For per-file manifest entries — single-file source emits `relative_path = ""` legitimately.
- **resolve-dest-path** — `crates/blit-daemon/src/service/util.rs:88-94` — `base.join(rel)` but preserves `base` verbatim when `rel` is empty (avoids `PathBuf::join("")` appending trailing separator).
- **resolve-contained-path** — `crates/blit-daemon/src/service/util.rs:107-112` — Joins rel under `module.path`, then verifies against `module.canonical_root` (NOT `module.path` which may have been munged by push handler).
- **resolve-module-empty-name** — `crates/blit-daemon/src/service/util.rs:9-38` — Empty name → synthesizes "default" ModuleConfig from `default_root` (delegation_allowed=true). Synthesizes — not stored in map. NotFound if default_root absent.
- **split-completion-prefix** — `crates/blit-daemon/src/service/admin.rs:249-273` — Routes through `path_posix::relative_str_to_posix` so `\` is correctly disambiguated between POSIX filename literal vs Windows separator. Comment explicitly calls out the bug `replace('\\', "/")` would cause.

### error-propagation
- **outcome-from-status** — `crates/blit-daemon/src/service/core.rs:1327-1332` — Helper translating `Result<_, Status>` into `(ok, Option<message>)` for push/pull/pull_sync. delegated_pull has its own shape and inlines equivalent mapping.
- **gate-denial-reasons** — `crates/blit-daemon/src/delegation_gate.rs:101-132` — Each `GateDenial` variant has a `reason()` string surfaced verbatim as `upstream_message` of `DelegatedPullError{phase=DELEGATION_REJECTED}`.
- **push-fallback-error-precision** — `crates/blit-daemon/src/service/push/data_plane.rs:553-574` — `receive_fallback_data` rejects: EOF mid-transfer (in-flight), missing `UploadComplete`, missing files after explicit complete. R8-F2/R9-F1.
- **pull-sync-spec-error-classification** — `crates/blit-daemon/src/service/delegated_pull.rs:362-375` — Distinguishes `PullSyncError::is_negotiation` → Phase::Negotiate, else → Phase::Transfer. Preserves typed negotiation boundary for R37-F1.

### discovery
- **mdns-advertise** — `crates/blit-daemon/src/main.rs:51-83` — On startup: builds mDNS advertiser via `blit_core::mdns::advertise` with port, instance name, module names, delegation flag. `delegation_enabled` field on the TXT record makes `blit scan` show which daemons are delegation destinations (§3.2).
- **mdns-disabled-warning** — `crates/blit-daemon/src/main.rs:52-58` — If mdns disabled AND name set in config, emits "instance name '<name>' ignored" warning.

### naming
- **active-job-kind-strings** — `crates/blit-daemon/src/active_jobs.rs:133-141` — `as_str` returns "push", "pull", "pull_sync", "delegated_pull".
- **service-module-rpc-counter-name-mismatch** — `crates/blit-daemon/src/service/core.rs:613` — `pull_sync` calls `metrics.inc_pull()` — its attempts are conflated with `pull_operations_total`. No separate `pull_sync_operations` counter exists. _(notes: smell — metrics granularity)_

### flag-handling
- **detach-disables-tx-closed** — `crates/blit-daemon/src/service/core.rs:1314-1320` — `if !detach` guard on `tx.closed()` select arm makes a CLI hangup non-terminal when `detach=true`. m-jobs-3.
- **force-grpc-OR-semantics** — `crates/blit-daemon/src/service/core.rs:556` — `req.force_grpc OR self.force_grpc_data` — either side can force gRPC fallback.

### confirmation-prompt
(none — daemon has no interactive prompts; the user-facing confirmation lives in CLI per `feedback_resolve_destination`)

## Smells / risks observed

1. **counter-conflation** — `service/core.rs:613` increments `inc_pull()` for `pull_sync`, and `service/delegated_pull.rs:249` likewise. `pull_operations_total` therefore counts `pull + pull_sync + delegated_pull` attempts as one bucket. No granularity for which pull variant.
2. **getstate-counters-always-some** — `service/core.rs:1072-1078` publishes `Counters` unconditionally; when `--metrics` is off all values are zero. Per memory `feedback_getstate_counters_zero`, these are FALSE zeros, not real counts. Doc comment at runtime.rs:104 says "Reserved for a future GUI/TUI gRPC GetState-style RPC" but the RPC publishes them now.
3. **timeout-constants-duplicated** — `service/pull_sync.rs:597-598` and `:719-720` redeclare `PULL_SYNC_ACCEPT_TIMEOUT` / `PULL_SYNC_TOKEN_TIMEOUT` in two functions with the same values. `service/pull.rs:698-699` redeclares again as `PULL_ACCEPT_TIMEOUT` / `PULL_TOKEN_TIMEOUT`. Drift risk if one is updated.
4. **stream-count-tier-duplication** — `service/pull.rs:915-933` and `service/push/control.rs:499-520` have parallel byte-tier ladders (32 GiB → 16 streams, etc.) but the push ladder has file-count thresholds while pull doesn't. Easy to drift; should share a constant.
5. **socket-tuning-only-on-push** — `service/push/data_plane.rs:110-120` applies `tcp_nodelay` + `keepalive` after accepting the data plane connection; the pull/pull_sync data plane accept paths (`pull.rs:700`, `pull_sync.rs:600`, `pull_sync.rs:721`) do NOT. Asymmetric socket configuration.
6. **socket-tuning-errors-silenced** — `service/push/data_plane.rs:115-116` — `let _ = s2.set_tcp_nodelay(true); let _ = s2.set_keepalive(true);` — these can fail silently.
7. **token-compare-not-constant-time** — `service/push/data_plane.rs:183`, `pull.rs:738`, `pull_sync.rs:631,753` — token bytes compared with `==` (variable-time). Token is 32 random bytes; practical risk is low but the pattern is repeated across 4 sites.
8. **bind-host-hardcoded-default-0000** — `runtime.rs:200` defaults to `0.0.0.0`. No `--bind` and no config → daemon binds all interfaces. No warning.
9. **default-root-fallback-exports-cwd** — `runtime.rs:283-298` — When no modules AND no `--root`, daemon exports `cwd` as "default" with read-write. Operator launching from arbitrary directory could over-expose. Only a warning emitted.
10. **kind-mismatch-in-handler-helper** — `service/core.rs:1303-1320` `resolve_delegated_pull_outcome` is generic over future types but the `_ = tx_closed, if !detach => None` branch will not fire when `detach=true`. Comment is clear but the API shape is subtle.
11. **stderr-noise** — `eprintln!` scattered through `service/push/data_plane.rs:121,162,183,187-191,238-242`, `service/pull.rs:660-664,902-905`, `delegation_gate.rs` (none here actually), `main.rs` (lots). No log levels, no filtering.
12. **bytes-counter-arc-leakable** — `active_jobs.rs:984-987` — `bytes_counter()` clones the Arc<AtomicU64>; a clone outliving the guard "just bumps an orphaned atomic" per the doc, but `report_after_drop_does_not_resurrect_row` test confirms intent.
13. **windows-cfg-divergence** — `service/admin.rs:197-200` clears readonly attr on Windows before `remove_file`. `service/admin.rs:620-628` strips `\\?\` prefix on Windows for sysinfo path comparison.  `runtime.rs:166-170` cfg!(windows) for default config path. Three separate Windows quirks.
14. **udp-vs-tcp-keepalive-conflation** — `main.rs:130-142` doc-comment talks about HTTP/2 keepalive reaping silently-dead peers, but the socket2 tuning at `data_plane.rs:115-116` sets TCP keepalive too. Two unrelated keepalive concepts in one codebase.
15. **delegated-pull-summary-uses-metrics-inc-pull-not-inc-delegated** — `delegated_pull.rs:249` increments `inc_pull`. Per code comment "from this daemon's perspective the body of work is a pull from src" — but operator-facing metrics can't distinguish.
16. **scope-deletions-includes-unspecified-as-filtered-subset** — `service/push/control.rs:343-345` — When mirror_mode=true with `MirrorMode::Unspecified` OR `Off` value, the purge filter uses `purge_filter.clone_without_cache()` (i.e. the user-supplied filter). The proto-default Off being treated like FilteredSubset is the explicit "back-compat for older clients" behavior but is a non-obvious deviation from the enum's nominal semantics.
17. **scope-deletions-pull-sync-and-push-disagree** — `pull_sync.rs:430-462` `scope_deletions` returns `Vec::new()` for `Off | Unspecified` — opposite to push's behavior in (16). Two mirror-purge sites with subtly different fallback for `Off|Unspecified`.
18. **dead-code-allowlist** — Many `#[allow(dead_code)]` markers on `ActiveJob` fields (active_jobs.rs:199), `transfer_id()` getter (:906), `as_str` (:133), `recent()` (:731), `cancel()` (:459), `with_recent_limit` (:374), `bytes_counter` (:984), `acquire_buffer` in TarShardExecutor (data_plane.rs:646). Indicates ahead-of-consumer plumbing.
19. **find-glob-double-match** — `service/admin.rs:545-559` — Pattern matches against full relative_path OR basename. Comment frames it as "intuitive fallback for patterns that don't use `**`" but a glob like `nested*` would match a basename `nested` even when the full path is `dir/nested` — both surfaces tested.
20. **complete-path-no-test-of-windows-separators** — `service/admin.rs:249-273` `split_completion_prefix` has the right defensive comment but no unit test in this file pinning the Windows/POSIX `\` ambiguity behavior.

### TODO/FIXME/HACK scan
None of the read files contain literal `TODO`, `FIXME`, `XXX`, or `HACK` markers. Instead, the codebase uses inline "in scope for slice X / lands in slice Y / deferred per §N" prose comments (e.g. `active_jobs.rs:55-69`, `service/core.rs:1099-1101`, `service/push/data_plane.rs:602-627`) to defer work. The deferred work is heavily documented but harder to grep for than `TODO`.

### Deprecated/removed feature references
- No references to `blit-utils`, `BlitAuth`, or AI telemetry detected in this cluster.
- `service/admin.rs:500` comment mentions `BLIT_UTILS_PLAN.md` as the source of glob-pattern policy — a planning doc reference that still survives.

## Coverage attestation

| File | Lines read | Notes |
|---|---:|---|
| `crates/blit-daemon/Cargo.toml` | 36 | Full |
| `crates/blit-daemon/src/main.rs` | 147 | Full |
| `crates/blit-daemon/src/net_timeout.rs` | 41 | Full |
| `crates/blit-daemon/src/recents_store.rs` | 177 | Full |
| `crates/blit-daemon/src/metrics.rs` | 302 | Full |
| `crates/blit-daemon/src/runtime.rs` | 499 | Full |
| `crates/blit-daemon/src/active_jobs.rs` | 2009 | Full — two pages (1-1333, 1334-2010) |
| `crates/blit-daemon/src/delegation_gate.rs` | 877 | Full |
| `crates/blit-daemon/src/service/mod.rs` | 22 | Full |
| `crates/blit-daemon/src/service/util.rs` | 168 | Full |
| `crates/blit-daemon/src/service/core.rs` | 2537 | Full — two pages (1-1429, 1430-2538) |
| `crates/blit-daemon/src/service/admin.rs` | 850 | Full |
| `crates/blit-daemon/src/service/pull.rs` | 1038 | Full |
| `crates/blit-daemon/src/service/pull_sync.rs` | 1147 | Full |
| `crates/blit-daemon/src/service/push/mod.rs` | 5 | Full |
| `crates/blit-daemon/src/service/push/control.rs` | 520 | Full |
| `crates/blit-daemon/src/service/push/data_plane.rs` | 1133 | Full |
| `crates/blit-daemon/src/service/delegated_pull.rs` | 981 | Full |

**Total lines read**: 12,489
**Files NOT read**: none
