# Drift Findings: Core principles & release scope
**Generated**: 2026-06-04
**Claims audited**: 78 (principles 12, invariants 14, interfaces 33, behaviors 10, scope 11, non-goals 7, shipped 13, deferred 14, rejected 7, decisions 17 — overlap allowed; see plan-principles.md)
**Findings**: 12 (H 4 / M 5 / L 3)

## High severity

### drift-env-vars-violate-no-env-config — Two BLIT_* env vars exist in shipped code despite "no env vars for config" invariant
**Plan says**:
- MASTER_WORKFLOW.md §3 Decision Log line 75: "Environment variables | ✅ Not used for configuration; precedence is CLI flag → config file"
- greenfield_plan_v6.md §5 v6 Open Questions line 432: "Config search order | Confirm precedence (CLI flag → config). No environment variables."

**Code does**:
- `crates/blit-tui/src/main.rs:4911-4921` — `BLIT_TUI_INPUT_TRACE=1` enables an append-mode log to a hardcoded `/tmp/blit-tui-input.log`. Reachable by setting the env var; not a CLI flag, not a config-file key.
- `crates/blit-core/src/remote/instrumentation.rs:3, 10` — `BLIT_TEST_COUNTER_FILE` toggles outbound-bytes counter file. Doc-comment claims "inert unless set," but the env var is the only way to enable; not a CLI flag, not a config-file key.
- `scripts/bench_remote_remote.sh:81` consumes the same `BLIT_TEST_COUNTER_FILE` hook as production benchmarking machinery (per `code-tests-scripts.md` smell #6).

**Evidence**:
- `crates/blit-tui/src/main.rs:4911,4917`
- `crates/blit-core/src/remote/instrumentation.rs:3,10`
- `crates/blit-cli/tests/remote_remote.rs:293,358` (counter env consumed by both tests and benches)

**Notes**: The two env vars are diagnostic/test-instrumentation, not user-facing performance/behavior toggles, so the violation is narrow. But the invariant text is absolute ("No environment variables"), so they are nonetheless drift. Remediation options: (1) add an explicit carve-out in the plan ("env vars permitted for test instrumentation only"); (2) replace `BLIT_TUI_INPUT_TRACE` with a TUI config flag and `BLIT_TEST_COUNTER_FILE` with a `#[cfg(test)]`-only path. The `PROTOC` env var set in `crates/blit-core/build.rs:6` is a build-script-only concern and is not a runtime violation.

### drift-1s-perceived-latency-target-no-mechanism — "Adaptive predictor … perceived latency ≤ 1 s" claimed, no streaming heartbeat / adaptive cadence mechanism found
**Plan says**:
- greenfield_plan_v6.md §1.1 v5 line 154: "Adaptive predictor fed by local telemetry to keep perceived latency ≤ 1 s."
- greenfield_plan_v6.md §1.1 v5 line 152: "Incremental planner that emits work every heartbeat (1 s default, 500 ms when workers are starved)."
- MASTER_WORKFLOW.md §1 line 12: "FAST – Transfers begin immediately; planner keeps perceived latency ≤ 1 s."

**Code does**:
- `crates/blit-core/src/orchestrator/orchestrator.rs` — no `HEARTBEAT_INTERVAL`, no `PLANNER_HEARTBEAT`, no "worker-starved" cadence reduction logic. `grep` for `starved`/`worker_starved`/`HEARTBEAT` across `crates/blit-core/` returns no matches.
- The closest mechanism is `derive_local_plan_tuning` (history-driven bucket-target tuning) and `select_tuning_window`, both batch-style, not a streaming-heartbeat cadence.
- The "predictor" exists (`crates/blit-core/src/perf_predictor.rs` 1368 lines) and is wired (D9 decision-d9-predictor-wire), but it predicts total durations to populate an observability line — it does NOT enforce or even measure a "≤ 1 s" perceived-latency invariant.

**Evidence**:
- `crates/blit-core/src/orchestrator/orchestrator.rs:1-2466` — full read, no heartbeat scheduler
- `crates/blit-core/src/perf_predictor.rs:239-326` — fallback-chain predictor (depth 0-3)
- `crates/blit-core/src/orchestrator/orchestrator.rs:824-845` — predictor verbose surface (`pct` closure for `+nn%` deltas)

**Notes**: The plan describes the predictor as the FAST principle's mechanism. As implemented the predictor is *observability* (verbose log of predicted vs actual), not *enforcement*. No code refuses, retries, or re-tunes when perceived latency exceeds 1 s. Either the plan should be revised to describe the predictor as observability (matching ship state per §2.8 closure note), or a real heartbeat/adaptive-tick mechanism must be added. This is the single most load-bearing FAST-principle claim and it has no enforcement code path.

### drift-10s-stall-detector-claim-vs-30s-pull-stall-actual — Plan says "10 s stall detector (planner *and* workers idle)"; code has 30 s data-plane idle timeout only
**Plan says**:
- greenfield_plan_v6.md §1.1 v5 line 153: "10 s stall detector (planner *and* workers idle) with precise error reporting."

**Code does**:
- `crates/blit-core/src/remote/transfer/stall_guard.rs:29`: `PULL_STALL_TIMEOUT: Duration = Duration::from_secs(30)` — read-side idle, not "planner AND workers idle." Only applied on pull-receiver socket; push doesn't have a parallel guard.
- No planner-side stall detection exists. `grep` for `stall|STALL` in `crates/blit-core/src/orchestrator/` returns no matches.
- The plan-doc value also conflicts with the inline owner decision quoted in `feedback_server_await_timeouts` memory (which approves 30 s, not 10 s).

**Evidence**:
- `crates/blit-core/src/remote/transfer/stall_guard.rs:29` (30 s constant)
- `crates/blit-core/src/remote/transfer/stall_guard.rs:51-79` (idle-deadline-reset semantics; not "both idle")
- `crates/blit-core/src/orchestrator/orchestrator.rs:544-550` (scan-handle await with NO timeout)

**Notes**: The plan number (10 s) and the planner-and-workers-idle composition are both wrong relative to shipped code. The owner decision recorded in memory (`audit-owner-decisions` → "transfer stall-timeout 30 s no-bytes") is what's actually live. Plan should be updated, or a planner-side stall detector should be added to match the claim. Conservative remediation: revise plan to match code (30 s data-plane idle).

### drift-grpc-fallback-env-override-missing — Plan promises `BLIT_FORCE_GRPC_DATA=1` and `BLIT_DISABLE_LOCAL_TELEMETRY=1` env-var overrides that don't exist in code
**Plan says**:
- greenfield_plan_v6.md §1.2 v5 line 161: "advanced `--force-grpc-data`/`BLIT_FORCE_GRPC_DATA=1` override for locked-down environments."
- greenfield_plan_v6.md §1.3 v5 line 168: "`BLIT_DISABLE_LOCAL_TELEMETRY=1` opt-out for debugging."

**Code does**:
- `grep -rn "BLIT_FORCE_GRPC_DATA\|BLIT_DISABLE_LOCAL_TELEMETRY"` across `crates/` and `proto/` returns ZERO matches.
- The daemon-side `--force-grpc-data` CLI flag exists (`crates/blit-daemon/src/runtime.rs:100, 375, 461, 492`; `crates/blit-daemon/src/service/core.rs:66`; `crates/blit-daemon/src/main.rs:106`). The client-side `--force-grpc` flag exists (`crates/blit-cli/src/cli.rs:305`).
- Perf-history opt-out is via `blit diagnostics perf --disable` (writes `settings.json`), not via an env var.

**Evidence**:
- `crates/blit-daemon/src/runtime.rs:100` — daemon force-grpc-data flag
- `crates/blit-cli/src/cli.rs:305` — client `--force-grpc` flag
- `crates/blit-core/src/perf_history.rs:422-431` — perf-history toggle via settings, not env
- (absence) `grep BLIT_FORCE_GRPC_DATA` across repo: no matches outside the plan doc itself

**Notes**: This is a documented-but-unimplemented user-facing override. Operators following the plan's documentation will set `BLIT_FORCE_GRPC_DATA=1` expecting the locked-down-environment fallback and observe no effect. Because the plan calls these out as the explicit env-vars-permitted exceptions, this collides with both `drift-env-vars-violate-no-env-config` and the inviolable principle. Remediation: either implement the env vars (and update the plan's "no env vars" rule accordingly) OR remove them from the plan and document the CLI/config-only path.

## Medium severity

### drift-v6-deliverables-checklist-still-says-blit-utils — v6 §4 deliverables checklist references `blit-utils` even though merged
**Plan says**:
- greenfield_plan_v6.md §4 line 421: "[ ] `blit-utils` implements `scan`, `ls`, `list`, `rm`, `find`, `du`, `df`, `completions`, `profile` command."
- The implementation note at top of greenfield_plan_v6.md (lines 7-16) explicitly notes "the as-shipped surface is `blit <subcommand>` built from `crates/blit-cli`."
- MASTER_WORKFLOW.md §1 line 20: "Admin subcommands on `blit`: ... (admin verbs were merged into the main binary; there is no separate `blit-utils`)."
- RELEASE_PLAN_v2_2026-05-04.md §2.2 lines 226-232: closed `aac13bf` — merged.

**Code does**:
- `ls /Users/michael/Dev/Blit/crates/` shows: `blit-app`, `blit-cli`, `blit-core`, `blit-daemon`, `blit-prometheus-bridge`, `blit-tui`. NO `blit-utils` crate.
- `crates/blit-cli/src/cli.rs:48-87` defines `Commands` enum including Copy/Mirror/Move/Scan/ListModules/Ls/Du/Df/Rm/Find/Completions/Profile/Check/Diagnostics/Jobs — all in the single `blit` binary.
- Only two surviving references in code are comments: `crates/blit-cli/tests/admin_verbs.rs:323` and `crates/blit-daemon/src/service/admin.rs:500` cite `BLIT_UTILS_PLAN.md` as the *origin* of the glob policy. No live code path uses the name.

**Evidence**:
- `crates/blit-cli/src/cli.rs:48-87` (single-binary subcommand enum)
- `crates/blit-cli/Cargo.toml:13` (`[[bin]] name = "blit"`)
- (absence) `crates/blit-utils/` does not exist
- `docs/plan/greenfield_plan_v6.md:421` still has the checklist entry

**Notes**: This is the long-running plan/code drift the inventory flagged as `contradiction-blit-utils-presence`. The v6 plan can be self-consistent two ways: (1) strike line 421 from the deliverables checklist and replace with a `blit` single-binary equivalent; (2) move the entire v6 body to a "historical" annex. Currently the doc tries to be both authoritative and historical at once. Low blast radius (developer confusion only) but high "is this doc trustworthy?" cost.

### drift-spinner-vs-quiet-default-decision-conflict — Plan v5 promises "unified progress indicator (spinner + throughput + ETA)" for copy/mirror; MASTER_WORKFLOW says "CLI quiet by default"
**Plan says**:
- greenfield_plan_v6.md §2 v5 line 222: "Introduce unified progress indicator (spinner + throughput + ETA) for copy/mirror."
- MASTER_WORKFLOW.md §3 line 73: "Progress UX | ✅ CLI quiet by default; structured events exposed for GUIs/debug"

**Code does**:
- `crates/blit-cli/src/cli.rs:367-380` — `effective_progress()`: explicit `--progress` wins; `--json` disables; otherwise auto-enable when stdout `is_terminal()`. Rsync-style default.
- `crates/blit-cli/src/transfers/local.rs:6,101-103` — uses `indicatif::ProgressBar::new_spinner()` + `ProgressStyle::with_template("{spinner} {msg}")` when progress is on.
- Net effect: CLI is quiet when piped (`!is_terminal()`) and spinner-y on TTY. Neither pure "quiet by default" nor pure "unified spinner always."

**Evidence**:
- `crates/blit-cli/src/cli.rs:372-380` — auto-progress predicate
- `crates/blit-cli/src/transfers/local.rs:101-103` — spinner setup
- `crates/blit-cli/Cargo.toml:26` — `indicatif = "0.18"` dep present

**Notes**: This is a doc inconsistency that the code resolves pragmatically. Either MASTER_WORKFLOW's "quiet by default" needs the "(stdout-piped) qualifier" added or v5's "unified progress indicator" needs the "(when isatty) qualifier." Code is correct; both docs are imprecise.

### drift-counters-always-some-violates-private-principle — `Counters` field is published on the wire whether or not metrics are enabled, contradicting the "metrics stay local" private principle as it relates to the proto contract
**Plan says**:
- MASTER_WORKFLOW.md §3 line 74: "Telemetry | ✅ Local JSONL history (optional opt-out); `blit profile` surfaces data"
- greenfield_plan_v6.md §1.3 v5 line 164: "All metrics stay on-device"
- greenfield_plan_v6.md §1.2 v6 line 320: "metrics never leave the machine"

**Code does**:
- `proto/blit.proto:756`: `Counters counters = 6;` — non-optional message field on `DaemonState` per proto2/proto3 rules (proto3 message fields are implicit-optional but the proto3 generated code emits `Some(Default::default())` when populated).
- Daemon publishes the field via `crates/blit-daemon/src/service/core.rs:1072-1078` (per code-daemon inventory line 184: "publishes `Counters` unconditionally; when `--metrics` is off all values are zero").
- Bridge consumes-but-hides: `crates/blit-prometheus-bridge/src/metrics.rs:38-96` deliberately omits the operation counters series, citing this exact issue (bridge inventory smell #3).
- Consumed by `crates/blit-cli/src/jobs.rs:643-651` JSON output (CLI inventory smell #11), which propagates the false-zero values to JSON consumers.

**Evidence**:
- `proto/blit.proto:745-756` — DaemonState.counters always present
- `crates/blit-daemon/src/service/core.rs:1072-1078` (per inventory)
- `crates/blit-prometheus-bridge/src/metrics.rs:38-96` — explicit workaround
- Memory: `feedback_getstate_counters_zero` — flagged the false-zero hazard

**Notes**: This isn't a true "metrics leave the machine" violation (the gRPC RPC is operator-network only by trust model, and the values are zero when metrics off). But it IS a violation of the PRIVATE principle's spirit AND the documented contract: an operator who disabled `--metrics` would reasonably expect `GetState.counters` to be absent or null, not all-zero. The bridge has to hardcode awareness of this contract violation. Remediation per bridge-inventory smell #3: make `counters` optional in the proto (or add `metrics_enabled: bool` sibling so consumers can distinguish "off" from "zero").

### drift-no-tls-while-plan-mentions-it — Phase-4 TLS for control plane (and STARTTLS for data plane) is named in the plan but no TLS / rustls dependency exists in any crate
**Plan says**:
- greenfield_plan_v6.md §3 v5 Phase 4 line 262: "TLS for control plane (and optionally data plane via STARTTLS-style negotiation)."
- greenfield_plan_v6.md §6 v5 line 304 marks "TLS for data plane | Deferred."

**Code does**:
- `grep -rn "TLS\|tls\|rustls"` in `crates/blit-core/Cargo.toml` and `crates/blit-daemon/Cargo.toml` returns no matches.
- tonic is used WITHOUT `tls` feature flag (default `transport = []` minus `tls`); no `Identity`, no `ServerTlsConfig`, no `ClientTlsConfig` references in `service/core.rs` or push/pull client code.
- Push/pull client built via `tonic::transport::Endpoint::from_uri` over plain TCP (`crates/blit-core/src/remote/push/client/mod.rs:299-317`, `crates/blit-core/src/remote/pull.rs:230-248`).

**Evidence**:
- (absence) no TLS deps in workspace Cargo files
- `crates/blit-core/src/remote/push/client/mod.rs:299-317` (plain Endpoint)
- `crates/blit-core/src/remote/pull.rs:230-248`

**Notes**: Per the RELEASE_PLAN explicitly deferring auth (RELEASE_PLAN §5.2 line 646: "Auth is out of project scope") and v5's Phase 4 label for TLS, this absence is in-scope-deferred, NOT shipped drift — but it's surfaced because (a) the principle-bound trust model is "operator network controls (firewall/VPN/SSH tunnel)" (RELEASE_PLAN §5.2), and (b) the proto has reserved fields for the removed BlitAuth service (`proto/blit.proto:653, 656-657`) which a reader might mistake for in-progress auth work. Remediation: leave the deferred status as-is; consider a single line in the release notes confirming "no transport-layer encryption in 0.1.0; use a VPN/SSH tunnel."

### drift-pull-rpc-deprecated-only-in-comment — `Pull` RPC is marked DEPRECATED in proto comment but no `[deprecated = true]` annotation; full message types still defined
**Plan says**:
- RELEASE_PLAN_v2_2026-05-04.md §2.8 line 436: "Silent dead code is incompatible with the 'no tech debt for the sake of backwards compatibility' release directive."

**Code does**:
- `proto/blit.proto:10-11`: `rpc Pull(PullRequest) returns (stream PullChunk);` with prose comment marking it deprecated.
- `proto/blit.proto:302`: `Ack ack = 1; // ... (deprecated, use pull_sync_ack)` — same comment-only deprecation.
- Neither carries `option deprecated = true;` or field `[deprecated = true]`. Generated client/server code will compile without any deprecation warning. PullRequest/PullChunk are not `reserved`.

**Evidence**:
- `proto/blit.proto:10-11`
- `proto/blit.proto:302`
- `bridge-proto` inventory smells #1, #2

**Notes**: Direct violation of `invariant-no-tech-debt-for-back-compat`. Either: (a) remove the `Pull` RPC and its message types entirely, OR (b) mark them `[deprecated = true]` so downstream tonic-prost generates `#[deprecated]` attributes that surface in user warnings. Currently it's the worst of both: live in proto, dead in policy.

## Low severity

### drift-mdns-platform-coverage-confirmation-incomplete — Plan says "Confirm behaviour on Linux, macOS, and Windows" for mDNS; no cross-platform test exists
**Plan says**:
- greenfield_plan_v6.md §2 v6 line 363: "Confirm behaviour on Linux, macOS, and Windows."

**Code does**:
- `crates/blit-core/src/mdns.rs` is platform-agnostic; uses `mdns-sd 0.19` crate.
- CI matrix runs on all three OSes (`.github/workflows/ci.yml:28-56`), but `crates/blit-cli/tests/blit_utils.rs:11-33` daemon launches with `no_mdns: true`; the only mDNS-touching test in CI deliberately disables advertising.
- No integration test verifies a discoverable daemon is found on any OS.

**Evidence**:
- `crates/blit-cli/tests/blit_utils.rs:11-33` (scan-no-mdns test launches WITHOUT mDNS)
- (absence) no `mdns_advertise_*` integration test in `crates/blit-cli/tests/`

**Notes**: Plan asks for a confirmation step that the test suite doesn't perform. Low severity because the mdns-sd crate is widely used and platform-independent. Remediation: add a single multi-OS smoke test that advertises + browses, or mark this as explicitly deferred-to-release-QA.

### drift-doc-survives-context-resets-not-enforced — "Every change must update relevant docs + DEVLOG" — not enforced by CI
**Plan says**:
- greenfield_plan_v6.md §5 v5 line 293: "Every change must update relevant docs + DEVLOG to survive context resets."

**Code does**:
- `.github/workflows/ci.yml:1-86` runs `cargo fmt`, `cargo clippy`, `cargo test`; no DEVLOG.md or docs-update check.
- DEVLOG.md is updated by convention only.

**Evidence**:
- `.github/workflows/ci.yml` — no doc-update gate

**Notes**: This is a process invariant, not a code invariant. Documenting it. Remediation if it matters: a CI step that fails when a PR touches `crates/` without touching `DEVLOG.md`.

### drift-deferred-tls-data-plane-cross-ref-broken — RELEASE_PLAN §5.5 cross-reference to "§3.3" points to FS-capability section, not TransferMetrics; description is also stale
**Plan says**:
- RELEASE_PLAN_v2_2026-05-04.md §5.5 lines 666-669: "TUI is deferred and 'The daemon's `TransferMetrics` are kept as scaffolding (see §3.3)'"
- Inventory `contradiction-tui-deferred-vs-metrics-scaffolding-modified` flags the §3.3 cross-ref as wrong (should be §3.1).

**Code does**:
- `crates/blit-daemon/src/metrics.rs:107-131` — `log_completion` emits per-RPC summary lines when `--metrics` is on. NOT dormant scaffolding (D5 owner decision modified to "keep + emit per-RPC summary").
- The TUI is shipping (Phase 6 work, currently in flight per `git status`); it's not deferred. See `crates/blit-tui/` — full TUI binary with F1/F2/F3/F4 panes + dual-pane shell.

**Evidence**:
- `crates/blit-daemon/src/metrics.rs:107-131`
- `crates/blit-tui/src/main.rs` (10,838 lines)
- Inventory `contradiction-tui-deferred-vs-metrics-scaffolding-modified`

**Notes**: Low severity because the actual code state is correct; the doc text is internally inconsistent. The RELEASE_PLAN_v2 was 2026-05-04; the D5-modified outcome dated 2026-05-13 closed §3.1 but §5.5 wasn't reconciled. Remediation: rewrite §5.5 to point at §3.1 and to drop "deferred" if TUI is in scope (or mark the TUI work as a separate work-stream).

## Claims that align well

- **principle-deliver-features-v6** — CLI command set (`copy`, `mirror`, `move`, `scan`, `list`, `list-modules`, `ls`, `du`, `df`, `rm`, `find`, `completions`, `profile`, `check`, `jobs`, `diagnostics`) all present in `crates/blit-cli/src/cli.rs:48-87`.
- **iface-default-port-9031** — Confirmed `DEFAULT_PORT: u16 = 9031` in `crates/blit-core/src/remote/endpoint.rs:25`; also at `crates/blit-daemon/src/runtime.rs:201`.
- **iface-cli-verbs-master / iface-cli-verbs-v6** — All claimed verbs present; aliases (`list` → `ls`) wired correctly per `crates/blit-cli/src/cli.rs:61`.
- **iface-remote-syntax-canonical** — `crates/blit-core/src/remote/endpoint.rs:27-96` implements module/Root/Discovery forms, rejects bare `server:/module` (missing trailing slash) cleanly. Per-memory `feedback_endpoint_parse_err` rule honored — `Err` rejects.
- **iface-no-blit-utils-binary / scope-d2-utils-merged** — Verified: no `crates/blit-utils/` directory; single `blit` binary.
- **iface-find-pattern-glob / decision-d7-find-glob** — Verified via `crates/blit-daemon/src/service/admin.rs:545-559` (glob via globset with literal_separator); R41 follow-up tests at `crates/blit-cli/tests/admin_verbs.rs:314-461`.
- **iface-completions-split-shipped / decision-d8** — Two-form completions verified: `crates/blit-cli/src/cli.rs:495-525` (shell + remote subcommands).
- **iface-blit-list-smart-dispatch / decision-d3** — Verified via `crates/blit-cli/src/ls.rs:49-69` (Discovery → list_modules; Module → ls).
- **iface-binary-name-blit / decision-d1** — `crates/blit-cli/Cargo.toml:13` confirms `[[bin]] name = "blit"`.
- **iface-mdns-txt-fields-shipped / decision-d4** — `crates/blit-core/src/mdns.rs:42-65` exposes `module_count()` and `delegation_enabled()` per §3.2.
- **iface-fs-capability-client-side-only-010 / decision-d6** — Client-side `blit diagnostics dump` exists (`crates/blit-cli/src/diagnostics.rs`); no daemon-side FS-capability probe — properly deferred.
- **invariant-canonical-containment-always-on** — `crates/blit-core/src/path_safety.rs:180-260` `contained_join` + `verify_contained`; daemon enforces at every chokepoint per `code-daemon.md` "safety-check" cluster.
- **rejected-blit-auth-stub** — `proto/blit.proto:110, 409, 653` confirm BlitAuth removal (only reservation comments remain, correctly per protobuf rules).
- **rejected-ai-telemetry-analysis** — No `docs/plan/AI_TELEMETRY_ANALYSIS.md`; no code path mentions it.
- **rejected-ludicrous-speed-flag / rejected-mir-flag** — `grep -rn "ludicrous\|--mir "` returns no matches in `crates/`.
- **shipped-binary-rename-blit** — Verified above.
- **shipped-predictor-observability** — `crates/blit-core/src/perf_predictor.rs` is 1368 lines, wired, observable via `blit profile --json` (`crates/blit-cli/src/profile.rs:8-22`).
- **shipped-metrics-per-rpc-summary** — `crates/blit-daemon/src/metrics.rs:107-131` confirms.
- **iface-token-cryptographic** — `crates/blit-daemon/src/service/push/data_plane.rs:23,52-58` — 32-byte tokens via `rand::SysRng`; pull/pull_sync follow same pattern. (Comparison non-constant-time per code-daemon smell #7; tokens are 32 random bytes from OS RNG so practical risk low.)
- **deferred-rdma-roce / deferred-rdma-phase-3.5** — Reserved fields `5 to 10` in `proto/blit.proto:126` (`DataTransferNegotiation`); no RDMA code paths.
- **deferred-daemon-fs-capability-4.8** — Daemon-startup/idle-probe absent per `crates/blit-daemon/src/main.rs:1-147` and `crates/blit-daemon/src/runtime.rs:165-353` — properly deferred.
