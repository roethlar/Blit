# Drift Findings: Phase history + post-review fixes + delegation
**Generated**: 2026-06-04
**Claims audited**: ~95 (principle / invariant / interface / behavior / scope / non-goal / shipped / deferred / rejected / decision)
**Findings**: 14 (H: 3 / M: 7 / L: 4)

## High severity

### detach-flag-shipped-despite-out-of-scope
**Plan says**: REMOTE_REMOTE_DELEGATION_PLAN.md §9, §4.2 step 12, §7 (cited as `scope-no-detach-mode`): *"`--detach` mode where CLI exits and dst continues. Track as separate future feature." Out-of-scope explicitly.* And §4.2 step 12 (`behavior-delegation-cli-session-bound`): *"Document that delegated pulls are CLI-session-bound; `--detach` is out of scope (§9)."*
**Code does**: `--detach` is a fully wired CLI flag on `TransferArgs` (`crates/blit-cli/src/cli.rs:325-335` — `pub detach: bool`). It is honored throughout the delegated pull pipeline: `crates/blit-cli/src/transfers/mod.rs:161-178` (gates `--detach` to remote→remote only), `crates/blit-cli/src/transfers/remote_remote_direct.rs:126-189` (full detach execution path including `print_detach_json` / `print_detach_human` / `run_delegated_pull_until_started`), daemon-side `crates/blit-daemon/src/service/core.rs:1314-1320` (the `if !detach` guard on `tx.closed()` for the delegated_pull RPC select), and the proto field `DelegatedPullRequest` (per code inventory `code-bridge-proto.md`). Tests `local_move_semantics.rs:381-402` and the CLI gate-rejection cases all assume `--detach` exists.
**Evidence**:
- `crates/blit-cli/src/cli.rs:325-335`
- `crates/blit-cli/src/transfers/mod.rs:161-178`
- `crates/blit-cli/src/transfers/remote_remote_direct.rs:126-189`
- `crates/blit-daemon/src/service/core.rs:1314-1320` (`detach-disables-tx-closed`)
- `crates/blit-daemon/src/active_jobs.rs:59` (doc-comment `// detach field on DelegatedPullRequest`)
**Notes**: This is the largest scope drift in the cluster. The plan emphatically says "out of scope, track as separate future feature" in three places; the feature is fully shipped. Either the plan needs to be updated to reflect that detach is now in scope and was implemented, or the feature needs to be hidden / removed. Given the gating tests and daemon RPC support, removal is no longer trivial. Suggested remediation: update `REMOTE_REMOTE_DELEGATION_PLAN.md` §9 and §4.2 step 12 to record that `--detach` shipped (and reference the actual code sites), so future readers don't believe "session-bound only" is the invariant.

### stall-detector-30s-not-10s
**Plan says**: WORKFLOW_PHASE_2.md §"Success Criteria" (cited as `invariant-stall-detector-10s`): *"Planner flushes batches incrementally; stall detector aborts with clear messaging after 10 s of inactivity."*
**Code does**: The only stall detector in the data plane is `PULL_STALL_TIMEOUT: Duration = Duration::from_secs(30)` (`crates/blit-core/src/remote/transfer/stall_guard.rs:29`). No 10-second planner stall detector is shipped: there is no `TransferFacade::stream_local_plan`, no `PlannerEvent`, and no `drive_planner_events` function anywhere in the workspace (verified by repository-wide grep). The orchestrator is synchronous: `crates/blit-core/src/orchestrator/orchestrator.rs:540-550` consumes scan headers via `header_rx.recv().await` with **no timeout** (see also code-core-orch smell #19 "scan-task-await-no-timeout").
**Evidence**:
- `crates/blit-core/src/remote/transfer/stall_guard.rs:29` (`PULL_STALL_TIMEOUT = 30s`)
- `crates/blit-core/src/remote/pull.rs:1712-1720` (wraps pull receiver in 30s StallGuard)
- `crates/blit-core/src/orchestrator/orchestrator.rs:540-550` (no timeout on scan recv/handle await)
- Absence of `PlannerEvent` / `stream_local_plan` / `drive_planner_events` in the workspace (grep returned zero matches in `crates/`)
**Notes**: Two distinct drifts here. (a) The documented 10s threshold is wrong by 3×. (b) The promised streaming-planner architecture (`shipped-streaming-planner`: ✅ `TransferFacade::stream_local_plan` emitting `PlannerEvent`; ✅ Heartbeat loop in `drive_planner_events`; ✅ Stall guard in `drive_planner_events`") has never landed under those names — what shipped is a different architecture (synchronous orchestrator + `StallGuard` on the pull data plane). The user-visible behavior may still be reasonable, but the plan/code mismatch is severe: a fresh contributor reading WORKFLOW_PHASE_2 would search for symbols that don't exist. Suggested remediation: update `WORKFLOW_PHASE_2.md` §"Success Criteria" and §2.1 to (1) reflect the actual 30s pull stall threshold, (2) describe the synchronous orchestrator + StallGuard architecture as what shipped, and (3) note the local scan path has no idle timeout (potential follow-up: bound `header_rx.recv().await` for slow filesystems).

### tar-shard-executor-not-removed-still-default-on-grpc-fallback
**Plan says**: POST_REVIEW_FIXES.md §1.2 + Round 1 closure (cited as `deferred-1.2-tar-shard-executor`): *"After Phase 5 of the receive-pipeline unification, the daemon's TCP push receive routes through `FsTransferSink::write_tar_shard_payload` (rayon-parallel). `TarShardExecutor` is now used **only** by the gRPC fallback path."* Round-1 status: *"§1.2 explicitly deferred with a docstring on `TarShardExecutor` and a tracked post-0.1.0 plan."*
**Code does**: `TarShardExecutor` is still defined and is still used as the **primary path** for `apply_tar_shard` in the daemon. `crates/blit-daemon/src/service/push/data_plane.rs:327` constructs `let mut tar_executor = TarShardExecutor::new(MAX_PARALLEL_TAR_TASKS)` at the top of receive (line 327 is not the gRPC fallback branch — it's the main push path). The docstring claims "Currently only used by the gRPC fallback path" (data_plane.rs:645) and "Plan after 0.1.0: collapse this into the unified sink path" (data_plane.rs:624). These two assertions contradict each other in the same file.
**Evidence**:
- `crates/blit-daemon/src/service/push/data_plane.rs:327` (constructor at top of main receive path)
- `crates/blit-daemon/src/service/push/data_plane.rs:620-647` (docstring claims gRPC-fallback only + `#[allow(dead_code)] async fn acquire_buffer`)
- POST_REVIEW_FIXES.md §1.2 claim that `FsTransferSink::write_tar_shard_payload` is the new unified path (which exists at `crates/blit-core/src/remote/transfer/sink.rs:280` — but is reached from the receive pipeline, not directly from `apply_tar_shard` on the TCP push path)
**Notes**: The plan's §1.2 reasoning ("TarShardExecutor is now used **only** by the gRPC fallback path") assumes a refactor that did not fully happen — the executor is still on the TCP push path's hot loop in the daemon. This is a contradiction the plan flags but mis-resolves. The risk is that future work-planning treats the executor as cold gRPC-only code and refactors it without exercising it under TCP push load. Suggested remediation: either complete the Phase-5 unification (let `FsTransferSink::write_tar_shard_payload` be the only tar-shard receiver and delete `TarShardExecutor`), or fix the docstring + plan to accurately describe the executor's current role as the **primary** tar-shard receiver.

## Medium severity

### no-blit-utils-binary-but-docs-still-reference-it
**Plan says**: PROJECT_STATE_ASSESSMENT.md §2 / WORKFLOW_PHASE_3.md & 4.md headers (cited as `non-goal-no-blit-utils-binary`): *"the admin verbs originally scoped here as `blit-utils <verb>` ship as subcommands of the single `blit` binary."* / *"admin utilities merged into the `blit` binary."*
**Code does**: The workspace contains no `blit-utils` crate (`crates/` contains only `blit-app`, `blit-cli`, `blit-core`, `blit-daemon`, `blit-prometheus-bridge`, `blit-tui`). But code comments still reference `BLIT_UTILS_PLAN.md` as policy authority: `crates/blit-daemon/src/service/admin.rs:500` ("Pattern matching is glob-based, matching `BLIT_UTILS_PLAN.md`") and `crates/blit-cli/tests/admin_verbs.rs:323` ("`--pattern` is a glob (per BLIT_UTILS_PLAN)").
**Evidence**:
- `ls crates/` (no `blit-utils`)
- `crates/blit-daemon/src/service/admin.rs:500`
- `crates/blit-cli/tests/admin_verbs.rs:323`
- WORKFLOW_PHASE_3.md §3.2.4 still says `blit-utils scan` literally (`behavior-admin-verbs-mdns-scan`)
**Notes**: Doc/code partial drift — the merge happened, but the trailing breadcrumbs (`BLIT_UTILS_PLAN.md` references, body text in WORKFLOW phases referring to `blit-utils <verb>`, `crates/blit-utils/src/main.rs` paths in shipped-admin-rpcs claim) need a sweep. Functionally fine; cosmetic + onboarding hazard.

### counters-published-when-metrics-off
**Plan says**: WORKFLOW_PHASE_2.md §"Guiding Principles" #2 (cited as `principle-telemetry-stays-local`) and behavior-quiet-by-default — telemetry is opt-out via CLI/config, expected behavior is that disabled means **disabled**.
**Code does**: `GetState` always publishes `counters: Some(Counters{...})` even when metrics are off, producing FALSE zeros. Daemon comment at `crates/blit-daemon/src/runtime.rs:104` notes "Reserved for a future GUI/TUI gRPC GetState-style RPC" but the RPC publishes them now (`crates/blit-daemon/src/service/core.rs:1072-1078`). The prometheus bridge knows this and works around it by emitting gauges only — `crates/blit-prometheus-bridge/src/metrics.rs:38-96` (`bridge-fmt-gauges-only`). This is documented in the user's project memory `feedback_getstate_counters_zero.md` and `proto/blit.proto:752-756`.
**Evidence**:
- `crates/blit-daemon/src/service/core.rs:1072-1078` (always-Some Counters)
- `crates/blit-daemon/src/runtime.rs:104` (doc-comment)
- `crates/blit-prometheus-bridge/src/metrics.rs:38-96` (defensive workaround)
- `proto/blit.proto:752-756` (proto-level documentation of the bug)
**Notes**: This is acknowledged technical debt with a defensive workaround at the bridge. Counts as drift because the principle "telemetry stays local + opt-out via CLI" is contradicted by always-published (but lying) counters. Suggested remediation: either add `metrics_enabled: bool` next to `Counters` in `DaemonState`, or make `counters: Option<Counters>` actually `None` when metrics disabled. Documented as `feedback_getstate_counters_zero.md`.

### env-var-still-used-for-test-counter-config
**Plan says**: WORKFLOW_PHASE_2.md §"Guiding Principles" #2 + §2.2.5 (cited as `non-goal-no-env-var-telemetry-config`): *"Opt-out should be driven by CLI/config settings (no environment variables once work completes)." / "Implementation must avoid environment-variable configuration."*
**Code does**: `crates/blit-core/src/remote/instrumentation.rs:10-13` reads `BLIT_TEST_COUNTER_FILE` env var to control byte-path-isolation counter emission. The constant `COUNTER_FILE_ENV = "BLIT_TEST_COUNTER_FILE"` is read on every transfer. The module comment justifies this: *"These hooks are inert unless `BLIT_TEST_COUNTER_FILE` is set. They are intentionally env-gated instead of `cfg(test)` because CLI integration tests execute the compiled `blit` binary as a child process."*
**Evidence**:
- `crates/blit-core/src/remote/instrumentation.rs:10-22`
- Used by tests (`crates/blit-cli/tests/remote_remote.rs:293, 358`) and benchmark script (`scripts/bench_remote_remote.sh:81`)
**Notes**: This is partial drift — the principle is about **telemetry/perf-history** env-var avoidance (which is honored: `perf_history.rs` is settings-file based with no env-var reads), but the test instrumentation hook is still env-driven and is shipped in the release binary. The shipping concern is small (production users won't set this var), but the principle is stated absolutely ("no environment variables once work completes"). The fact that a bench script depends on the env var, not just tests, is the real risk — operators copy bench scripts. Suggested remediation: document the env-var as a permitted test-instrumentation hook with operator-facing warning, or move to a hidden subcommand.

### scan-task-no-idle-timeout
**Plan says**: WORKFLOW_PHASE_2.md §"Success Criteria" (cited as `invariant-stall-detector-10s`): *"stall detector aborts with clear messaging after 10 s of inactivity"*; PHASE_2.md §"Goal" frames FAST + RELIABLE as core principles.
**Code does**: `crates/blit-core/src/orchestrator/orchestrator.rs:540-550` has unbounded `header_rx.recv().await` and `scan_handle.await`. Code-core-orch smell #19 calls this out explicitly: *"`header_rx.recv().await` and `scan_handle.await` have no timeout; if the source scan hangs (e.g., stalled network FS), the orchestrator is wedged."* No second-tier timeout/heartbeat on the local enumeration side.
**Evidence**:
- `crates/blit-core/src/orchestrator/orchestrator.rs:540-550`
- code-core-orch.md smell #19
**Notes**: Local FS scans are typically fast; the risk is non-zero for SMB/NFS-backed paths. Combined with #stall-detector-30s-not-10s (above), suggests the planner-side stall-detection feature promised in Phase 2 was never implemented — only the pull-side data-plane stall guard. Suggested remediation: bound `scan_handle.await` with a heartbeat-aware timeout, OR explicitly add a NOTE to plan that scan timeouts are not part of the 0.1.0 stall guarantee.

### perf-history-cap-1mb-not-mib
**Plan says**: WORKFLOW_PHASE_2.md §"Guiding Principles" #2 (cited as `principle-telemetry-stays-local`): *"Telemetry stays local – JSONL log under config dir, capped to ~1 MiB."*
**Code does**: `crates/blit-core/src/perf_history.rs:17` sets `DEFAULT_MAX_BYTES: u64 = 1_000_000` (decimal megabyte, ≈ 0.95 MiB), not 1 MiB (1,048,576). Off by 4.8%.
**Evidence**:
- `crates/blit-core/src/perf_history.rs:17` (`DEFAULT_MAX_BYTES = 1_000_000`)
**Notes**: Pedantic / low-impact, but the plan explicitly says MiB and the code is decimal MB. Either is fine in practice; the inconsistency should be reconciled. Plan-truth or code-truth.

### journal-cache-non-atomic-write
**Plan says**: WORKFLOW_PHASE_3.md §3.3.8 + PROJECT_STATE_ASSESSMENT.md §5 (cited as `shipped-linux-metadata-snapshot` and `behavior-usn-fast-path-28ms`): the change-journal subsystem is positioned as a load-bearing fast-path that callers can rely on.
**Code does**: `crates/blit-core/src/change_journal/tracker.rs:84-110` uses `File::create(path)` + `serde_json::to_writer_pretty` + flush — non-atomic. Crash mid-write corrupts the file. code-core-misc.md flags this as "Non-atomic persistence writes" smell. Tolerant `load()` (`tracker.rs:10-36`) falls back to empty map on parse error, so corruption is silently recovered with full performance loss.
**Evidence**:
- `crates/blit-core/src/change_journal/tracker.rs:84-110`
- `crates/blit-core/src/change_journal/tracker.rs:10-36` (silent fresh-start on parse error)
- code-core-misc.md "Smells" #3
**Notes**: Same pattern for `perf_history.rs:404-419` (`store_settings`), `perf_predictor.rs:476-484` (`save`), and `perf_history.rs:487-491` (rotation). 4 non-atomic writers, no temp+rename + fsync, all silently fail-fresh on corruption. Plan principle is RELIABLE; the silent re-fresh of journal cache means a crash during write can degrade a NTFS user from the 28ms zero-change fast path to a full scan without any user-visible signal. Suggested remediation: small helper `atomic_write_json(path, value)` doing temp+fsync+rename, used by all 4 sites.

### req-pre-allocation-guard-deferred-but-also-not-tested
**Plan says**: POST_REVIEW_FIXES.md §2.3 (cited as `deferred-2.3-pre-allocation-guard`): *"Pre-allocation guard in `read_tar_shard` … Consider growing the vec lazily … Trade-off: marginal CPU vs marginal memory. Probably defer."*
**Code does**: code-core-transfer.md confirms `pull-archive-size-cap` is in place: `crates/blit-core/src/remote/pull.rs:838-857` caps `archive_size > MAX_TAR_SHARD_BYTES` and uses `min(archive_size, 1 MiB, MAX_TAR_SHARD_BYTES)` for initial capacity. So the guard exists. But the deferred plan suggests "growing the vec lazily" — which is **not** done; the initial allocation is `min(declared, 1 MiB, MAX_TAR_SHARD_BYTES)`. Whether the deferral is honored or fixed is ambiguous in the doc.
**Evidence**:
- `crates/blit-core/src/remote/pull.rs:838-857` (`pull-archive-size-cap`)
- `crates/blit-core/src/remote/pull.rs:863-878` (`pull-shard-chunk-overflow-guard`)
**Notes**: The plan-vs-code state is internally consistent (the guard exists and is conservative; lazy-grow is not done), but reading the plan, an auditor would have to dig into code to know whether §2.3 is closed or open. Suggested remediation: update POST_REVIEW_FIXES.md §2.3 to record "guard landed via 1 MiB initial-capacity clamp; lazy-grow not pursued".

## Low severity

### blit-utils-binary-still-referenced-in-comments
**Plan says**: WORKFLOW_PHASE_3.md historical (`non-goal-no-blit-utils-binary`): the binary was merged into `blit`.
**Code does**: Test comments and the daemon admin pattern comment still reference `BLIT_UTILS_PLAN.md` as the source of glob policy. The actual policy is in the daemon, but the source-of-truth doc lives under a name that implies a removed binary.
**Evidence**:
- `crates/blit-cli/tests/admin_verbs.rs:323` (`per BLIT_UTILS_PLAN`)
- `crates/blit-daemon/src/service/admin.rs:500` (`matching BLIT_UTILS_PLAN.md`)
**Notes**: Pure naming hygiene. Already partially captured under medium-severity drift above; recorded separately as the test/comment leak is a different surface.

### json-summary-schemas-not-unified
**Plan says**: behavior-quiet-by-default + PHASE_2 §2.3.2 — JSON should be the predictable surface for GUIs.
**Code does**: code-cli.md observes that `operation`-string drifts across paths: `local.rs` uses one shape; `remote.rs` push uses `destination` as plain string not endpoint shape; `remote_remote_direct.rs:235-253` delegated path uses `operation=delegated_pull` with `source_peer_observed`. *"distinct 'operation' string from push/pull — schemas not unified."*
**Evidence**:
- `crates/blit-cli/src/transfers/local.rs:46-71, 207-316`
- `crates/blit-cli/src/transfers/remote.rs:447-488` (push & pull JSON)
- `crates/blit-cli/src/transfers/remote_remote_direct.rs:235-253`
**Notes**: The plan doesn't mandate schema unification, but it implies stable GUI-facing JSON. Drift is minor / cosmetic until a TUI/GUI consumer trips over it. Suggested remediation: document the per-path JSON schemas in `docs/cli/blit.1.md` (or equivalent) so consumers have a contract.

### bin-name-blit-hardcoded-twice
**Plan says**: interface-blit-binary-merged — `blit` is the merged binary.
**Code does**: `bin_name = "blit"` is hardcoded in `crates/blit-cli/src/completions.rs:44` AND `crates/blit-cli/Cargo.toml:13`. Renaming requires editing both; out of sync would break completions.
**Evidence**: code-cli.md, smell list.
**Notes**: Cosmetic. Use `Cli::command().get_name()` to centralize.

### perf-history-toggle-decision-deferred
**Plan says**: WORKFLOW_PHASE_2.md §2.2.5 note (cited as `decision-perf-history-toggle-tbd-by-benchmarks`): *"Final release toggle (enabled by default vs. opt-in) will be decided from benchmark evidence."*
**Code does**: `crates/blit-cli/src/context.rs:9-22` (`appcontext-perf-default-enabled`): on settings read failure, default is `perf_history_enabled = true`. So the de facto release decision is "enabled by default" — but the plan still treats this as undecided.
**Evidence**:
- `crates/blit-cli/src/context.rs:9-22`
- `crates/blit-core/src/perf_history.rs:422-431` (toggle reads persisted settings, default true)
**Notes**: Plan should be updated to reflect the shipped default (opt-out / on by default) so the "TBD by benchmark" wording doesn't read as live indecision.

## Claims that align well

The following plan claims/principles are honored by code:

- **principle-no-silent-fallback / behavior-stale-daemon-explicit-upgrade**: `crates/blit-app/src/transfers/remote.rs:709-713` returns "destination daemon does not implement DelegatedPull; upgrade …" on `Code::Unimplemented` — matches the plan's §4.4 language verbatim. Test `crates/blit-cli/tests/remote_remote.rs:277-309` asserts this.
- **interface-relay-via-cli-flag**: `--relay-via-cli` exists on `TransferArgs`; CLI dispatch obeys it. Move + remote-source combo correctly rejects per `move-rejects-relay-with-remote-src`. Tests cover the explicit-relay vs. direct-delegation byte-path-isolation invariant (`remote_remote.rs:184-273`).
- **interface-pull-sync-with-spec / interface-build-spec-from-options**: Both exist at `crates/blit-core/src/remote/pull.rs:534` (`build_spec_from_options`) and `:639` (`pull_sync_with_spec`). Wire-equivalence + endpoint-isolation tests in `crates/blit-core/tests/pull_sync_with_spec_wire.rs` pin the contract (R23-F1 / R25-F1).
- **invariant-endpoint-isolation / decision-r25-f1-endpoint-isolation**: `pull_sync_with_spec` does not read `self.endpoint.path`; the spec wins. Verified by `pull_sync_with_spec_wire.rs:251-310` SpyServer test.
- **invariant-mandatory-client-capabilities-override / decision-r25-f2**: Honored by `crates/blit-daemon/src/service/delegated_pull.rs:170-303` (`delegation-handler-ordering` step 9 "caps override").
- **invariant-dns-rebinding-mitigation / decision-r23-f3 / invariant-loopback-requires-ip-form-authorization**: `crates/blit-daemon/src/delegation_gate.rs:288-392` `validate_source` does resolve-once-connect-by-IP, special-range detection (loopback / link-local / unique-local / unspecified) requires IP-form match. Code-daemon §safety-check `delegation-gate-validate-source`.
- **principle-policy-before-network / decision-r23-f2-gate-ordering**: Honored — gate runs before module resolution and outbound connect. `crates/blit-daemon/src/service/delegated_pull.rs:170-303` walks gate → module-lookup → containment → connect.
- **invariant-flush-must-propagate**: `crates/blit-core/src/remote/transfer/sink.rs:396-398` — flush failure propagates (no `let _ =`). `tokio-file-flush-propagates`.
- **behavior-mtime-perms-warn / behavior-tcp-tuning-warn-don't-fail**: `crates/blit-core/src/remote/transfer/sink.rs:415-417, 425-429, 514-523` and `data_plane.rs:88-103` use `log::warn!` rather than silent drops. Matches POST_REVIEW §1.1.
- **invariant-block-complete-includes-mtime-perms / interface-block-complete-wire**: `BLOCK_COMPLETE := 0x03 path_len:u32 path:bytes total_size:u64 mtime:i64 perms:u32` shipped at `crates/blit-core/src/remote/transfer/data_plane.rs:466-516`.
- **invariant-pipeline-real-error-surfaced**: `crates/blit-core/src/remote/push/client/mod.rs:175-197` (`pipeline-streaming-surfaces-underlying-error`) drains `pipeline_handle` to extract the real error rather than emit the generic "pipeline closed unexpectedly".
- **interface-blit-binary-merged / interface-blit-daemon**: Workspace ships `blit` (single CLI binary) + `blit-daemon`. No `blit-utils` crate.
- **interface-toml-config**: `crates/blit-daemon/src/runtime.rs:140-239` reads `[daemon]`, `[delegation]`, `[[modules]]` per spec. `delegation_allowed` defaults true (`runtime.rs:157-163`), can narrow.
- **interface-mdns-default-on**: `crates/blit-daemon/src/main.rs:51-83` advertises mDNS unless `no_mdns` set; `crates/blit-core/src/mdns.rs:14-15` exports `BLIT_SERVICE_TYPE = "_blit._tcp.local."`.
- **interface-delegation-toml-block + invariant-loud-failure-on-invalid-config**: `crates/blit-daemon/src/runtime.rs:226-235` parses every `allowed_source_hosts` entry at config load; failure on bad entry surfaces as config-load error (delegation-config-parse).
- **shipped-tcp-fallback-test (`remote_tcp_fallback`)**: Test exists at `crates/blit-cli/tests/remote_tcp_fallback.rs:243` with `[gRPC fallback]` stderr assertion.
- **shipped-pull-sync-no-deadlock-test / shipped-pull-preserves-mtime-test / shipped-mtime-only-no-retransfer-test**: All three exist at `crates/blit-cli/tests/remote_regression.rs` (`pull_sync_does_not_deadlock_with_populated_destination` and friends).
- **shipped-fuzz-wire-format-parser / shipped-dos-bounds-parser**: Caps live at `crates/blit-core/src/remote/transfer/pipeline.rs:325-408` (`MAX_WIRE_PATH_LEN=64KiB`, `MAX_WIRE_TAR_SHARD_FILES=1_048_576`, etc.).
- **invariant-f2-canonical-containment-always-on / decision-f13-remove-use-chroot**: No `use_chroot` field anywhere in `crates/`; F2 chokepoints exist at `crates/blit-core/src/path_safety.rs:142-329` (`safe_join_contained`).
- **non-goal-blit-auth-removed / non-goal-ai-telemetry-removed**: No `BlitAuth` / `delegated_credential` use in `crates/`. Proto reserves field 10 + `"delegated_credential"` per `code-bridge-proto.md`. No AI-telemetry code.
- **behavior-purge-with-confirmation**: `crates/blit-cli/src/rm.rs:48-58` + `crates/blit-cli/src/transfers/mod.rs:87-99` prompt unless `--yes`.
- **behavior-tar-shard-parallel-unpack** (cited but contradicted by §1.2 plan note): rayon parallelism does land at `crates/blit-core/src/remote/transfer/sink.rs:600-625` (`rayon-parallel-tar-extract`). The "two places" criticism is the one called out under high-severity drift `tar-shard-executor-not-removed-still-default-on-grpc-fallback`.
