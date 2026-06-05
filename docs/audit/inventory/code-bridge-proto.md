# Code Inventory: blit-prometheus-bridge + proto + build.rs
**Generated**: 2026-06-04 by audit workflow
**Coverage**: 6 files / 1901 lines (Cargo.toml 15, main.rs 123, metrics.rs 244, server.rs 506, blit.proto 994, blit-core/build.rs 19)

Note: only `blit-core/build.rs` exists. `blit-app/build.rs` and `blit-daemon/build.rs` do NOT exist on disk — those crates do not run codegen, they consume `blit_core::generated::*` re-exports. Inventory scoped accordingly.

## Behaviors (grouped by category)

### format-output
- **bridge-fmt-gauges-only** — `crates/blit-prometheus-bridge/src/metrics.rs:38-96` — `format_metrics` emits SIX gauges (`blit_daemon_up{version}`, `blit_daemon_uptime_seconds`, `blit_daemon_modules`, `blit_daemon_delegation_enabled`, `blit_active_transfers`, `blit_recent_transfers`) and deliberately omits the operation counters (push/pull/purge/active/errors). _(notes: gauges only by design; tied to the GetState.Counters present-but-zero bug recorded in feedback_getstate_counters_zero.md.)_
- **bridge-fmt-up-help-shared** — `crates/blit-prometheus-bridge/src/metrics.rs:34, 44, 105` — single `UP_HELP` constant is reused by both `format_metrics` and `down_metrics` so Prometheus never sees a HELP-text mismatch across alternating up/down scrapes. _(notes: defensive — Prometheus warns on HELP drift between scrapes.)_
- **bridge-fmt-down-metrics** — `crates/blit-prometheus-bridge/src/metrics.rs:103-107` — `down_metrics()` returns just `blit_daemon_up 0` (no version label, no other series). Used by HTTP server path on scrape failure / timeout so the target registers as up-but-down instead of a scrape error. _(notes: returns 200, not 5xx, with the down body — intentional.)_
- **bridge-fmt-label-escape** — `crates/blit-prometheus-bridge/src/metrics.rs:124-136` — `escape_label` escapes `\\`, `"`, `\n`, AND `\r` per exposition spec. audit-5 closed the `\r` gap (previously unescaped). _(notes: latent — current label values don't carry CR — but spec-compliant.)_
- **bridge-fmt-metric-helper** — `crates/blit-prometheus-bridge/src/metrics.rs:112-116` — `metric()` always emits HELP/TYPE/sample triple, never bare lines, ensuring well-formed exposition output. _(notes: writeln returns swallowed via `let _ =` (writes to `String` are infallible).)_
- **bridge-fmt-trailing-newline** — `crates/blit-prometheus-bridge/src/metrics.rs:38-96` — uses `writeln!` for every line so output always ends in `\n`; safe for concatenation/streaming. _(notes: explicit contract per doc-comment line 37.)_

### rpc-handler
- **bridge-http-serve-loop** — `crates/blit-prometheus-bridge/src/server.rs:91-142` — `serve()` accept loop: acquires semaphore permit BEFORE accept (back-pressure into OS backlog), races accept against `ctrl_c` (biased select for prompt shutdown), spawns per-connection task holding the permit until completion. _(notes: permit lifetime tied to handler future, not just accept — slot freed on handler drop.)_
- **bridge-http-handle-conn** — `crates/blit-prometheus-bridge/src/server.rs:145-188` — `handle_conn` reads request head under `REQUEST_HEAD_TIMEOUT` (5s), routes target `/metrics` → scrape under `SCRAPE_TIMEOUT` (8s) → 200 with body, otherwise 404 with hint. On head-timeout writes 408 (best-effort, ignored errors). _(notes: every write goes through `write_all_within` with `WRITE_TIMEOUT` (10s).)_
- **bridge-http-method-allowlist** — `crates/blit-prometheus-bridge/src/server.rs:269-278` — `request_target` returns `Some(target)` ONLY for `GET` requests; HEAD, POST, PUT, etc. all fall through to None → 404. _(notes: not 405 Method Not Allowed — quirk; tested at line 482-483.)_
- **bridge-http-response-builder** — `crates/blit-prometheus-bridge/src/server.rs:282-287` — `http_response()` always emits accurate `Content-Length` (byte length, not char count) + `Connection: close`. Multibyte UTF-8 test at line 500-505 confirms byte-count semantics. _(notes: HTTP/1.1, no keep-alive — one response per connection.)_
- **bridge-http-read-head** — `crates/blit-prometheus-bridge/src/server.rs:239-264` — reads in 1024-byte chunks until `\r\n\r\n` OR `MAX_REQUEST_HEAD` (16 KiB) hit. Outer `tokio::time::timeout` wraps the read; `read_head_bytes` has no internal timeout. _(notes: `MAX_REQUEST_HEAD` only bounds BYTES; only the timeout wrapper actually bounds time.)_
- **bridge-http-scrape-body** — `crates/blit-prometheus-bridge/src/server.rs:217-232` — `scrape_body`: timeout-Err OR scrape-Err both yield `down_metrics()` (`blit_daemon_up 0`). Generic over the future so it tests with `pending()`. _(notes: both error paths print to stderr but do NOT propagate — handler always 200s.)_
- **bridge-http-write-all-within** — `crates/blit-prometheus-bridge/src/server.rs:194-209` — `write_all_within`: `write_all` + `flush` under `tokio::time::timeout`; elapsed deadline is an error (not silent truncation). Generic over writer (tested with `tokio::io::duplex`). _(notes: closes audit-5b1 round 2 — even the 408 write is bounded.)_

### timeout-or-retry
- **bridge-cnst-oneshot-timeout** — `crates/blit-prometheus-bridge/src/main.rs:36` — `ONESHOT_TIMEOUT = 8s` — matches server-side `SCRAPE_TIMEOUT`. Pre-audit-5 inherited OS connect timeout (60-127s) and hung cron jobs against dead hosts. _(notes: one-shot path fails LOUDLY on timeout (non-zero exit) while HTTP path returns 200 + down metrics — asymmetric by design.)_
- **bridge-cnst-request-head-timeout** — `crates/blit-prometheus-bridge/src/server.rs:39` — `REQUEST_HEAD_TIMEOUT = 5s` — the actual slowloris guard. Prometheus sends head in one segment immediately. _(notes: matters because `--listen` accepts any SocketAddr including non-loopback binds.)_
- **bridge-cnst-scrape-timeout** — `crates/blit-prometheus-bridge/src/server.rs:47` — `SCRAPE_TIMEOUT = 8s` — bounds `jobs::query` since it has no internal timeout. Below Prometheus default `scrape_timeout` of 10s so the bridge can answer with down_metrics BEFORE Prometheus gives up. _(notes: feedback_server_await_timeouts.md — bridge-2 reopened ×2 for missing await timeouts.)_
- **bridge-cnst-write-timeout** — `crates/blit-prometheus-bridge/src/server.rs:53` — `WRITE_TIMEOUT = 10s` — bounds response writes so a client that stops reading (full socket buffer) can't park the handler. _(notes: audit-5 round 2 closure — write side was previously unbounded.)_

### spawn-task
- **bridge-spawn-per-conn** — `crates/blit-prometheus-bridge/src/server.rs:132-139` — every accepted connection spawns a tokio task holding the semaphore permit; permit dropped on task completion. Errors logged to stderr, never escalated. _(notes: no JoinHandle tracked — fire-and-forget; permit lifetime is the only completion gate.)_

### safety-check
- **bridge-cnst-max-concurrent-scrapes** — `crates/blit-prometheus-bridge/src/server.rs:63, 104` — `MAX_CONCURRENT_SCRAPES = 64` semaphore caps in-flight handlers. Permit acquired BEFORE accept, so excess connections queue in OS backlog instead of as tasks. _(notes: audit-5 — prevents flood DoS from spawning unbounded tasks each firing GetState.)_
- **bridge-cnst-max-request-head** — `crates/blit-prometheus-bridge/src/server.rs:31` — `MAX_REQUEST_HEAD = 16 KiB` byte cap per connection. _(notes: byte cap only — the time cap is REQUEST_HEAD_TIMEOUT.)_
- **bridge-listener-reuseaddr** — `crates/blit-prometheus-bridge/src/server.rs:69-87` — `build_listener` sets `SO_REUSEADDR` (TcpListener::bind doesn't on all platforms), so quick restart can rebind while old socket lingers in TIME_WAIT. Backlog 1024. _(notes: explicit IPv4/IPv6 split via `addr.is_ipv4()`.)_
- **bridge-graceful-shutdown** — `crates/blit-prometheus-bridge/src/server.rs:114-121` — accept loop races against `tokio::signal::ctrl_c()` (biased select); on signal, stops accepting but in-flight handlers run to completion. _(notes: no explicit join — process exits when main task returns; in-flight tasks may be cancelled if the runtime drops.)_

### endpoint-parse
- **bridge-endpoint-parse** — `crates/blit-prometheus-bridge/src/main.rs:61-62` — args.remote parsed through `RemoteEndpoint::parse()` with eyre context; failure is fatal-exit. _(notes: delegates strict parsing to blit-core — consistent with feedback_endpoint_parse_err.md.)_

### cancellation
- **bridge-no-cancel-propagation** — `crates/blit-prometheus-bridge/src/server.rs:132-139` — spawned handlers do NOT receive a cancellation token; ctrl_c only stops accept loop. In-flight tasks continue until their own timeouts elapse. _(notes: by design — each handler is already bounded; no need for cross-cutting cancellation.)_

### default-value
- **bridge-cli-recent-limit-default** — `crates/blit-prometheus-bridge/src/main.rs:49` — `--recent-limit` defaults to 50 (matches daemon's default per blit.proto:732). _(notes: hardcoded duplicate of `GetStateRequest`'s server-side default.)_
- **bridge-spec-version-history** — `proto/blit.proto:429-434` — `TransferOperationSpec.spec_version` history: 1 = original; 2 = added `require_complete_scan` (R49-F2). Receivers reject versions they don't understand. _(notes: explicit fail-closed contract — v1 daemons must not silently drop the safety-critical field.)_

### data-plane
- **proto-data-plane-negotiation** — `proto/blit.proto:121-127` — `DataTransferNegotiation` carries `tcp_port`, `one_time_token`, `tcp_fallback`, `stream_count`. Reserves fields 5..10 for future RDMA (Phase 3.5). _(notes: forward-compat field reservation.)_

### discovery
- **proto-find-rpc** — `proto/blit.proto:30-31, 368-383` — `Find(FindRequest) returns (stream FindEntry)`. `FindRequest.max_results` caps results; `FindEntry` mirrors FileInfo shape. _(notes: server-streaming.)_
- **proto-disk-usage** — `proto/blit.proto:33-34, 385-396` — `DiskUsage` is server-streaming; `DiskUsageRequest.max_depth` bounds recursion. _(notes: stream-of-entries, not a single aggregate.)_
- **proto-filesystem-stats** — `proto/blit.proto:35-36, 398-407` — module-scoped df-style stats (total/used/free bytes).  _(notes: per-module, not whole-daemon.)_
- **proto-complete-path** — `proto/blit.proto:24, 352-358` — `CompletePath` for shell completion; `include_files`/`include_directories` filter what's returned.

### state-machine
- **proto-push-stream** — `proto/blit.proto:6-7, 130-150` — bidirectional `Push`. ClientPushRequest oneof = 8 variants (PushHeader, FileHeader, ManifestComplete, FileData, UploadComplete, TarShardHeader/Chunk/Complete). ServerPushResponse oneof = 4 (Ack, FileList, PushSummary, DataTransferNegotiation). _(notes: "check-then-send" — manifest first, server replies with NeedList, client streams data.)_
- **proto-pull-deprecated** — `proto/blit.proto:10-11, 238-243` — `Pull(PullRequest) returns (stream PullChunk)` marked DEPRECATED in doc-comment, but not annotated with proto `deprecated = true`. _(notes: dead-code candidate; replaced by PullSync. PullRequest still defined in full.)_
- **proto-pullsync-stream** — `proto/blit.proto:14-15, 273-331` — bidirectional `PullSync`. ClientPullMessage oneof = spec+local_file+manifest_done+block_hashes (4). ServerPullMessage oneof = 15 variants including the new `delete_list`, `tar_shard_*`, `block_*` resume payloads. _(notes: leading message now `TransferOperationSpec` — PullSyncHeader removed entirely.)_

### naming
- **proto-deprecated-server-pullsync-ack** — `proto/blit.proto:302` — `ServerPullMessage.ack` field 1 carries inline `(deprecated, use pull_sync_ack)` comment but isn't marked `deprecated = true` proto-side. _(notes: latent inconsistency — field still wired.)_
- **proto-bridge-version-string** — `proto/blit.proto:736` — `DaemonState.version` is "CARGO_PKG_VERSION" — bridge labels every `up` gauge with this string. _(notes: if daemons differ in patch version, Prometheus sees multiple label series.)_

### key-dispatch
- (no key-dispatch behaviors in this cluster — bridge has no keybindings.)

### config-load
- **bridge-cli-args** — `crates/blit-prometheus-bridge/src/main.rs:42-56` — clap derive with 3 args: `--remote` (required), `--recent-limit` (u32, default 50), `--listen` (optional SocketAddr). Presence of `--listen` switches modes. _(notes: minimal CLI surface; no config file.)_

### persistence
- (no persistence — bridge is stateless; this is highlighted in metrics.rs module docs.)

### render-or-display
- **bridge-display-startup-msg** — `crates/blit-prometheus-bridge/src/server.rs:97-100` — `serve()` prints "blit-prometheus-bridge: serving http://{addr}/metrics (scraping ...)" to stderr on bind. _(notes: stderr — keeps stdout reserved for one-shot metrics output mode.)_
- **bridge-display-errors-stderr** — `crates/blit-prometheus-bridge/src/server.rs:117, 127, 137, 224, 228` — accept errors, connection errors, scrape failure/timeout all logged to stderr; never crash, never surface to client. _(notes: consistent stderr-as-log convention.)_

### flag-handling
- **bridge-mode-switch** — `crates/blit-prometheus-bridge/src/main.rs:64-74` — presence of `--listen` switches between server mode and one-shot mode. Different timeout semantics in each (server soft-fail to `up 0`, one-shot hard-fail with exit code). _(notes: documented in module head.)_

### error-propagation
- **bridge-eyre-with-context** — `crates/blit-prometheus-bridge/src/main.rs:61-71`, `crates/blit-prometheus-bridge/src/server.rs:74-86, 204-207` — every fallible call carries `.with_context(|| format!(...))` for an audit-trail-friendly error chain. _(notes: consistent eyre usage; uses `{err:#}` alt-format for chains.)_
- **bridge-scrape-err-swallowed-to-down** — `crates/blit-prometheus-bridge/src/server.rs:223-230` — scrape errors in server mode are intentionally swallowed → log to stderr → return down_metrics. _(notes: contradicts one-shot path which surfaces the error verbatim — see contradictions section.)_

### path-handling
- (proto only defines path STRINGS — actual containment lives in blit-core, not in this cluster.)

## Smells / risks observed

1. **Deprecated `Pull` RPC still defined in full** — `proto/blit.proto:10-11` marks `Pull` deprecated in a comment but it remains in the service with no proto `option deprecated = true`, and its PullRequest/PullChunk messages are not reserved. Risk: tooling won't warn callers; deprecated path may quietly grow new clients.

2. **`ServerPullMessage.ack` field 1 is comment-deprecated only** — `proto/blit.proto:302` — same pattern. No `[deprecated = true]` marker, so generated client code shows it as normal.

3. **Counters present-but-zero on the wire** — `proto/blit.proto:752-756` documents that `counters` is always `Some` even with `--metrics` off; the bridge explicitly works around this (`metrics.rs:90-93`). This is the root cause of feedback_getstate_counters_zero.md. The clean fix (omit Counters when metrics off, or add a `metrics_enabled` bool) is deferred. Bridge can never publish counter series until then.

4. **Asymmetric error semantics across the two bridge modes** — one-shot path treats timeout as hard error / non-zero exit (`main.rs:69-71`); HTTP server path treats timeout as soft-fail to `blit_daemon_up 0` (`server.rs:217-231`). Documented intentionally in main.rs:27-35 — flag for review consistency.

5. **`build_listener` always sets SO_REUSEADDR but never SO_REUSEPORT** — `server.rs:69-87` — fine for single-process, but two bridges aiming at the same port both bind cleanly on Linux/macOS only if REUSEPORT is set on both. Quirk only if someone tries hot-failover scenarios.

6. **`#[tokio::main]` uses default multi-thread runtime** — `main.rs:58` — entire bridge has at most 64 concurrent handlers + 1 accept loop; multi-thread overhead is overkill. Likely fine but unexamined.

7. **Method allowlist returns 404, not 405** — `server.rs:269-278` and tested at line 482-483 — non-GET method receives 404 Not Found instead of 405 Method Not Allowed. Non-standard but functional.

8. **No HEAD support** — bridge.rs returns 404 for HEAD requests. Prometheus doesn't probe with HEAD, but some monitoring rigs (e.g. blackbox_exporter) might.

9. **`build.rs` hardcodes proto path traversal** — `blit-core/build.rs:9` — `manifest_dir.join("..").join("..").join("proto")` — fragile to workspace restructuring. No cargo metadata lookup.

10. **`build.rs` mutates env via `std::env::set_var`** — `blit-core/build.rs:6` — within build script scope only. The `set_var` is marked unsafe in newer Rust editions; current code uses Rust 2021 so still safe-callable.

11. **`PROTOC` env var leak risk** — `blit-core/build.rs:6` — `set_var("PROTOC", ...)` affects the build script's process env. If `tonic_prost_build::configure()` spawns a subprocess for `protoc`, this is the intended path, but it overrides any user-set `PROTOC` without warning.

12. **`reserved 5 to 10` in DataTransferNegotiation** — `proto/blit.proto:126` — comment claims "RDMA fields for Phase 3.5". If Phase 3.5 ever lands, the field numbers are locked; if not, the gap is permanent.

13. **`reserved 10` and `reserved "delegated_credential"` in RemoteSourceLocator** — `proto/blit.proto:656-657` — references the removed `BlitAuth` service. Correctly reserved per RULES — flagging the removed-feature reference.

14. **`reserved 5` and `reserved "COMPARISON_MODE_IGNORE_EXISTING"` in ComparisonMode** — `proto/blit.proto:527-528` — removed enum variant; correctly reserved.

15. **`reserved 5, 6` in DaemonEvent.payload** — `proto/blit.proto:901-904` — for future `ModuleListChanged`/`DaemonHeartbeat` per `c-2-subscribe-skeleton`. Fine; flagged so the slice owner knows the gap.

16. **`spec_version = 1` allowed to silently accept** — `proto/blit.proto:429-434` — comment says "Receivers should reject specs with a version they don't understand" — actual rejection lives in `blit-core` (not in this cluster). Risk: if a future spec_version=3 adds another safety field, the bump must happen at the same time as the receiver check.

17. **`blit_daemon_up` gauge always = 1 when bridge produces a value** — `metrics.rs:38-48` — the gauge cannot represent "scrape partial success"; you either get full metrics or `down_metrics`. Probably fine but worth noting if a Counters family is ever added back.

18. **`spawn_one_shot` doesn't exist** — main.rs one-shot path doesn't spawn; it inline-runs `jobs::query`. Consistent with simplicity but a flag-rerun under load could exhaust connection pool to a single daemon — currently nobody does this in cron.

## Contradictions / intra-cluster smells

1. **Soft-fail (HTTP) vs hard-fail (one-shot) on identical error class** — `main.rs:69-71` propagates `query timed out` as eyre error → non-zero exit, while `server.rs:217-231` returns 200 + `up 0`. Documented as intentional, but a future operator running one-shot in node_exporter's textfile collector instead of cron will get different observability behavior than expected.

2. **`recent_limit` default duplicated** — `main.rs:49` defaults to 50, `proto/blit.proto:730-733` says daemon default is 50. Two sources of truth; if the daemon's default changes, bridge silently keeps requesting 50.

3. **HELP-text identical contract enforced only for `blit_daemon_up`** — `UP_HELP` constant (`metrics.rs:34`) is shared, but the other 5 gauges' HELP text only appears at one call site each. If a metric is ever conditionally emitted with different HELP, Prometheus will warn. Today's code never does, but no constant guards against it.

4. **`build.rs` uses `protoc_bin_vendored` for the protoc binary** — `blit-core/build.rs:1, 5` — vendored binary path. `tonic_prost_build` is used (not `tonic_build`); subtle naming — current `prost`-based stack.

5. **`build_server(true).build_client(true)`** — `blit-core/build.rs:14-17` — both server and client stubs generated in blit-core. blit-app + blit-daemon both consume `blit_core::generated::*` — single source of truth.

6. **No `serde` derive request in build.rs** — generated types don't get Serialize/Deserialize; if anyone tries to JSON-encode a `DaemonState` they'll have to map fields manually. Bridge does NOT need this; flagging in case ever needed.

7. **Slow consumers documented but not enforced in proto** — `proto/blit.proto:104-106` — "Slow consumers receive a gRPC Status::Aborted" — that contract lives in `blit-daemon`'s `Subscribe` impl, not this cluster. Couldn't audit-check the actual behavior from this cluster's files alone.

## Coverage attestation

| File | Lines read | Notes |
|---|---|---|
| crates/blit-prometheus-bridge/Cargo.toml | 15 | full |
| crates/blit-prometheus-bridge/src/main.rs | 123 | full |
| crates/blit-prometheus-bridge/src/metrics.rs | 244 | full (including tests) |
| crates/blit-prometheus-bridge/src/server.rs | 506 | full (including tests) |
| proto/blit.proto | 994 | full |
| crates/blit-core/build.rs | 19 | full |

**Total lines read**: 1901

**Files NOT read** (with reason):
- `crates/blit-app/build.rs` — does not exist (blit-app consumes blit-core's generated proto re-exports; verified via `ls`).
- `crates/blit-daemon/build.rs` — does not exist (same reason; verified via `ls`).
