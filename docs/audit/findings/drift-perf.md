# Drift Findings: Performance plans & heuristics
**Generated**: 2026-06-04
**Claims audited**: ~80 across 4 plan files (LOCAL_TRANSFER_HEURISTICS, PIPELINE_UNIFICATION, BENCHMARK_10GBE_PLAN, BENCH_VERB_PLAN)
**Findings**: 19 (H: 4 / M: 9 / L: 6)

The 4-document cluster mixes shipped specs (LOCAL_TRANSFER_HEURISTICS), in-flight refactors (PIPELINE_UNIFICATION),
and planned-but-explicitly-deferred work (BENCH_VERB_PLAN). The deferred-bench work is correctly absent;
the drift concentration is in (a) `LOCAL_TRANSFER_HEURISTICS.md` claiming "no staged rollout — ship together"
when several of its mechanisms (heartbeat scheduler, 10s stall timeout, verbose heartbeat messages) are absent
or shipped with different numbers, and (b) `PIPELINE_UNIFICATION.md` whose core invariants ("one diff implementation",
"filter parity becomes free", "deprecate custom pull_sync.rs") all describe an unshipped refactor.

---

## High severity

### local-heuristics-no-staged-rollout-claim-violated — "Ship together" invariant violated by shipped slices
**Plan says**: "This document captures the full design we intend to implement in Phase 2 for Blit v2. There is no staged rollout—every mechanism described here will ship together once complete." (LOCAL_TRANSFER_HEURISTICS.md header / invariant-no-staged-rollout)
**Code does**: The orchestrator has shipped predictor/perf-history/fast-path/journal-fast-path *partially* without the heartbeat scheduler (§3.2), the 10 s stall timeout (§3.3), or the verbose heartbeat messages (§3.4). No heartbeat or stall-timeout logic exists in `crates/blit-core/src/orchestrator/orchestrator.rs` — search returns zero matches for `heartbeat`, `1s`, `10s`, `planner stalled`, `Planning… N entries classified`. The orchestrator runs synchronously through scan→plan→pipeline; the only `Duration::from_secs(10)` use is for *mtime* future-stamp tests (orchestrator.rs:1797, 1905), not the planner-stall path.
**Evidence**:
- `crates/blit-core/src/orchestrator/orchestrator.rs:540-574` — scan handle awaited without timeout
- `crates/blit-core/src/remote/transfer/stall_guard.rs:29` — only stall timeout in code is 30 s for *remote pull*, not local planner
- `crates/blit-core/src/orchestrator/orchestrator.rs:808` — `"Planning enumerated {} file(s), {} bytes"` (post-hoc summary, not the §3.4 heartbeat line `"Planning… 12,345 entries classified; queue depth 8"`)
**Notes**: The "ship together" claim is the load-bearing invariant of the document. With the streaming planner / heartbeat / 10 s timeout subsystem absent but the perf-history/predictor/fast-path *shipped*, the resolution table at the bottom of the doc is misleading. Either soften the invariant to "ship together within each lane" or move the unshipped items to a separate section. Owner decision needed.

### tuning-window-size-mismatch — Plan says 50, code says 20
**Plan says**: "Adaptive logic reads only the most recent N entries (default 50) to inform planning-time predictions." (LOCAL_TRANSFER_HEURISTICS.md §5.3 / behavior-perf-history-windowed)
**Code does**: `pub(super) const TUNING_WINDOW_SIZE: usize = 20;` at `crates/blit-core/src/orchestrator/orchestrator.rs:33`. Doc-comment on `select_tuning_window` says "lets a regime change propagate within ~20 transfers" — explicit intent of 20, not 50.
**Evidence**:
- `crates/blit-core/src/orchestrator/orchestrator.rs:33` — `TUNING_WINDOW_SIZE = 20`
- `crates/blit-core/src/orchestrator/orchestrator.rs:73` — `.take(TUNING_WINDOW_SIZE)`
**Notes**: Factor-of-2.5 numerical drift. The number 20 was deliberately chosen ("regime change propagate within ~20 transfers"); the plan was never updated. Fix: update LOCAL_TRANSFER_HEURISTICS.md §5.3 to read "default 20" (preferred — code intent is current) or align code to 50 if the larger window was a deliberate decision the planner forgot.

### pipeline-unification-not-shipped — Core refactor invariants describe unshipped state
**Plan says**: Multiple invariants in PIPELINE_UNIFICATION.md describe a unified pipeline that doesn't exist yet:
- "One diff implementation. Today's diff logic lives in three places (orchestrator local mirror, push client, pull_sync handler). Extracting DiffPlanner collapses them." (invariant-one-diff-implementation)
- "Filter parity becomes free. All filtering goes through DiffPlanner, on whatever side is the origin. No special pull-side limitation." (invariant-filter-parity-free)
- "Resume becomes universal. Block-level resume is currently a feature of `pull_sync` only. Once it's a DiffPlanner capability, push gets it for free." (invariant-resume-universal)
- "The custom `service/pull_sync.rs` enumeration + diff + stream code." (rejected-custom-pull-sync-code)

**Code does**: All three diff sites still exist independently:
- `crates/blit-core/src/orchestrator/orchestrator.rs:556-574` — local orchestrator calls `plan_local_mirror` (a real DiffPlanner extraction)
- `crates/blit-daemon/src/service/pull_sync.rs` — 1147-line custom enum + diff + stream code STILL the daemon's pull_sync RPC handler (per inventory line 22)
- `crates/blit-core/src/remote/push/client/mod.rs:1133` — push client has its own client-side diff (file enumeration + manifest comparison) pre-DataPlane

A `diff_planner.rs` exists at `crates/blit-core/src/remote/transfer/diff_planner.rs:474` but is only *called* by the local orchestrator path and indirectly by push helpers, not by pull_sync. No `ExecuteOriginJob` RPC exists in `proto/blit.proto`.
**Evidence**:
- `crates/blit-daemon/src/service/pull_sync.rs:1147 lines` — file present, not deleted (code-daemon.md:22)
- `proto/blit.proto:15` — `rpc PullSync(stream ClientPullMessage) returns (stream ServerPullMessage);` still primary pull verb
- `crates/blit-core/src/remote/transfer/diff_planner.rs` — exists but is only the *local* DiffPlanner; remote pull path goes through the legacy custom code
**Notes**: Plan document carries no "Status: planning" header; reads as if shipped. This is the largest structural drift in the cluster. The plan should add a top-line "Status: planning — not yet shipped" banner and either commit to the refactor or formally pause it. Filter parity (`filter-parity-pull-bail` rejected per plan) is still active per `crates/blit-cli/src/transfers/remote.rs:357` (build_filter_spec for pull).

### fanotify-status-claim-misleading — Plan claims "shipped" but only metadata snapshots exist
**Plan says**: "Windows USN fast-path shipped 2025-10-25. macOS FSEvents snapshot added 2025-10-25 (verified). Linux metadata snapshot (device/inode/ctime) added 2025-10-25; further fanotify integration optional." (LOCAL_TRANSFER_HEURISTICS.md §10, claimed under `shipped`)
**Code does**: The three platform branches DO exist in `crates/blit-core/src/change_journal/snapshot.rs:35-79` (compare-macos-fsevents-then-mtime, compare-linux-device-inode-ctime, compare-windows-volume-usn). However, the doc's word "shipped" + journal_fast_path gating in orchestrator.rs:200-251 holds only under a 5-flag gate (`skip_unchanged && !checksum && !force_tar && !null_sink && dest_root.exists()`). The "Linux metadata snapshot" is *metadata-only*; without fanotify/inotify wired in, a file modified between scans with unchanged ctime+device+inode is invisible. Plan implies this is "optional polish"; in reality on Linux the journal fast path will produce false NoChange more often than on Windows (USN) / macOS (FSEvents).
**Evidence**:
- `crates/blit-core/src/change_journal/snapshot.rs:50-63` — Linux `compare_linux_snapshots` falls back to root_mtime equality when device/inode unchanged
- `crates/blit-core/src/orchestrator/orchestrator.rs:200-251` — 5-flag gate
**Notes**: Severity high because the surface promises "shipped 2025-10-25" parity across platforms, but Linux semantics are weaker. Users on Linux running `blit mirror src dst` twice in rapid succession with a file edited in between might see journal_fast_path skip the modification. Recommend: document the Linux limitation explicitly in §10 or downgrade the "shipped" status to "shipped (Linux: metadata-only, may miss edits with unchanged ctime)".

---

## Medium severity

### bench-verb-correctly-deferred-but-plan-document-style-mismatch — BENCH_VERB_PLAN is 0.2.0 work; correctly absent
**Plan says**: "Status: Draft. Not in 0.1.0 scope." (BENCH_VERB_PLAN.md header / nongoal-bench-not-in-0-1-0)
**Code does**: Confirmed all bench surfaces absent — no `blit bench` subcommand (grep `Bench` in `crates/blit-cli/src/cli.rs:355` only finds `null` flag docs), no `[bench]` config section in `crates/blit-daemon/src/runtime.rs`, no `BenchSynthesize` RPC in `proto/blit.proto`, no `SyntheticTransferSource` anywhere in `crates/blit-core/src/`.
**Evidence**:
- `crates/blit-core/src/perf_history.rs:60-78` — `RunKind::{Real, DryRun, NullSink, BenchTransfer, BenchWire}` enum EXISTS though, with BenchTransfer/BenchWire variants tagged "planned 0.2.0 verb" in doc-comments
**Notes**: This is the only plan that aligns well with its "not shipped" status. The `RunKind` enum even pre-anticipates the bench lanes. Nonetheless: the `--null` flag is still shipped and routed through `local::run_local_transfer` (cli.rs:355 + plan-perf decision-null-sink-retained), which is the path BENCH_VERB_PLAN §1 wants to remove. Sequence step 8 ("Remove `--null` and its guards; update docs") has not happened. Low-noise drift; flagged so the plan tracker can note the bench-verb work is correctly future-tense throughout.

### predictor-no-bench-bucket-but-plan-claims-it — Predictor v3 bucket split not landed
**Plan says**: "Extend `PerformancePredictor` v3 (bump schema) with a second profile bucket keyed on `TransferKind`." Copy/mirror profile vs. bench profile; `predictor.observe(record)` routes by kind; `predictor.predict(intent: TransferKind)` adds intent parameter. (BENCH_VERB_PLAN.md §6 / iface-predictor-kind-bucket)
**Code does**: `PerformancePredictor::observe` at `crates/blit-core/src/perf_predictor.rs:390-411` does NOT route by `TransferKind`; instead it *silently skips* any non-real-transfer record (`if !record.run_kind.is_real_transfer() { return; }` at line 391). Bench records are dropped, not bucketed. `predict()` at line 239 takes `mode, source_fs, dest_fs, fast_path, skip_unchanged, checksum` — no `kind` / `intent` parameter. `STATE_VERSION = 3` is set, but the v3 bump was for adding the `transfer` coefficients alongside `planner`, not for bucket splitting.
**Evidence**:
- `crates/blit-core/src/perf_predictor.rs:36-52` — version 3 constants, but only for transfer-vs-planner duration split
- `crates/blit-core/src/perf_predictor.rs:391` — `if !record.run_kind.is_real_transfer() { return; }`
**Notes**: This drift is internally-consistent: the bench-verb work is deferred to 0.2.0, the predictor extension naturally follows. The `STATE_VERSION` bump to 3 happened for a different reason (R56-F1 closing the dry-run/null-sink contamination bug). The BENCH_VERB_PLAN tracker should note that "v3 schema bump" is already taken; the bench bucket split will need v4.

### no-debug-workers-banner — Plan-mandated DEBUG banner missing
**Plan says**: "Optional debug limiter (`--workers`) caps worker count for diagnostics. The flag remains hidden from normal help output; when active the CLI prints a `[DEBUG] Worker limiter active` banner and FAST guarantees are suspended." (LOCAL_TRANSFER_HEURISTICS.md §7 / iface-debug-workers-flag)
**Code does**: `--workers` IS hidden (`crates/blit-cli/src/cli.rs:359 #[arg(long, hide = true)] pub workers: Option<usize>`) but no `[DEBUG] Worker limiter active` banner is emitted anywhere — grep for "Worker limiter" returns zero in CLI sources.
**Evidence**:
- `crates/blit-cli/src/cli.rs:358-360` — `workers: Option<usize>` hidden flag
- (grep result) `Worker limiter active` not found in `crates/`
**Notes**: User would silently get reduced worker count without visible feedback that diagnostics-mode is active. Trivial to add; could ship as a one-line eprintln in `transfers/local.rs` when `args.workers.is_some()`.

### no-verbose-planner-heartbeat — Plan-mandated `Planning… N entries classified` line missing
**Plan says**: "`--verbose` shows real-time heartbeat messages (e.g. `Planning… 12,345 entries classified; queue depth 8`)." (LOCAL_TRANSFER_HEURISTICS.md §3.4 / iface-verbose-progress, behavior)
**Code does**: The orchestrator emits a *post-planning* summary line `"Planning enumerated {} file(s), {} bytes"` at `crates/blit-core/src/orchestrator/orchestrator.rs:808`, not a real-time tick. No spawned tokio task or `tokio::time::interval` for periodic emission during planning. The indicatif spinner at `crates/blit-cli/src/transfers/local.rs:101` shows generic "spinner + msg" but not the structured `Planning…` payload.
**Evidence**:
- `crates/blit-core/src/orchestrator/orchestrator.rs:808` — post-hoc summary, not real-time
- `crates/blit-cli/src/transfers/local.rs:101-105` — generic spinner template `"{spinner} {msg}"`
**Notes**: Tied to the broader "no streaming planner / no heartbeat" drift; if §3.1-3.4 are revisited, this line falls out for free.

### endpoint-empty-default-port-9031-hardcoded-multiple-sites — Hardcoded 9031 contradicts shared-constant intent
**Plan says**: (implicit: PIPELINE_UNIFICATION mentions endpoint capability negotiation; LOCAL_TRANSFER_HEURISTICS §1 says no user-tuning knobs)
**Code does**: `9031` is hardcoded in 4+ sites:
- `crates/blit-core/src/remote/endpoint.rs:25` — `DEFAULT_PORT: u16 = 9031`
- `crates/blit-daemon/src/runtime.rs:201` — daemon's default port literal
- `crates/blit-cli/src/scan.rs:63` — endpoint elision literal (per code-cli.md smell)
- `proto/blit.proto:732` — daemon-recent-limit default 50 commented but `9031` referenced via prose
**Evidence**:
- `crates/blit-core/src/remote/endpoint.rs:25` constant
- `crates/blit-cli/src/scan.rs:63` literal
**Notes**: Not a plan-specific drift, but the principle "SIMPLE: no user-facing tuning flags; orchestrator decides the optimal path" (principle-simple-no-user-speed-knobs) implies shared constants; in practice the port and tar-shard/buffer/predictor numbers are scattered. Lower severity because functionally correct; flagged because future port change has 4-site update cost.

### tar-shard-size-vague-vs-shipped-256-mib — Plan says "8 MiB / scales to 32/64 MiB"; code says 256 MiB cap
**Plan says**: "shards flush around 8 MiB/≈1 k files and scale up to 32/64 MiB as manifests grow" (LOCAL_TRANSFER_HEURISTICS.md §7 / behavior-small-file-tar-shard-thresholds — also flagged as contradiction-tar-shard-flush-size-scale in plan-perf)
**Code does**: `MAX_TAR_SHARD_BYTES: u64 = 256 MiB` at `crates/blit-core/src/remote/transfer/tar_safety.rs:50` is the single source of truth shared by wire-frame caps and per-entry alloc bounds. Auto-tune defaults are 8 MiB / 2048 count (auto_tune/mod.rs:144-145), clamping to [4MiB, 128 MiB] / [128, 4096] / [64 MiB, 512 MiB] (auto_tune/mod.rs:148-158). None of 32 / 64 / 256 MiB match the plan's "32/64 MiB" call-out cleanly.
**Evidence**:
- `crates/blit-core/src/remote/transfer/tar_safety.rs:50` — `MAX_TAR_SHARD_BYTES: u64 = 256 * 1024 * 1024`
- `crates/blit-core/src/auto_tune/mod.rs:144-158` — clamp ranges
**Notes**: Plan numbers were aspirational; shipped numbers are different. Either update plan or document why 256 MiB cap is the chosen ceiling.

### prom-bridge-counters-publish-false-zeros — Documented hazard still active
**Plan says**: (implicit through PIPELINE_UNIFICATION nongoal-no-web-http-metrics "Web/HTTP exposure of metrics — already removed; counters stay internal until a future GUI/TUI consumer needs them.")
**Code does**: A `crates/blit-prometheus-bridge/` crate EXISTS (per code-bridge-proto.md) and IS active. It publishes gauges (`blit_daemon_up`, `blit_active_transfers`, etc.) but per `code-bridge-proto.md:108` and per project memory `feedback_getstate_counters_zero`, the daemon's GetState.Counters is always Some even with metrics off, producing false zeros to consumers. Bridge avoids counter-series for this reason.
**Evidence**:
- `crates/blit-prometheus-bridge/src/metrics.rs:90-93` — explicitly skips counters per `bridge-fmt-gauges-only` behavior
- `proto/blit.proto:752-756` — documents the always-Some hazard
**Notes**: Plan says "counters stay internal until a future GUI/TUI consumer needs them" — but bridge is a third consumer that exists right now. PIPELINE_UNIFICATION should be amended to acknowledge the bridge as the present-day exception, or the bridge should be moved off the GetState path.

### no-relay-via-cli-rejection-on-pull-with-filters — Plan rejects this, code still bails
**Plan says**: "The current 'filter parity' workaround that bails on pull when filter args are passed (CLI side)." (PIPELINE_UNIFICATION.md §What this replaces / rejected-filter-parity-pull-bail) — claims this is being REMOVED.
**Code does**: `crates/blit-cli/src/transfers/remote.rs:357` builds `filter_spec` for pull and passes through. There IS no longer a "pull bails on filter" path at the CLI per inventory. So this rejection has actually been ACTIONED on the surface — the spec is built. But the deeper claim ("filter parity becomes free") requires the unified DiffPlanner running on daemon side; absent that, server-side filter enforcement details differ between push (initiator-side filter on FilteredSource) and pull (server-side daemon path via pull_sync.rs:424-463).
**Evidence**:
- `crates/blit-cli/src/transfers/remote.rs:357,376` — filter_spec built and sent for pull
- `crates/blit-daemon/src/service/pull_sync.rs:424-463` — `scope_deletions` per-mode filter
**Notes**: Partial alignment; surface drift resolved but underlying duplication remains. Lower-mid severity.

### force-grpc-and-relay-via-cli-as-tuning-knobs — Plan says "no user speed knobs" but CLI still has them
**Plan says**: "SIMPLE: no user-facing tuning flags; the orchestrator decides the optimal path. Debug-only overrides may exist but are not required for normal operation." (LOCAL_TRANSFER_HEURISTICS.md §1 / principle-simple-no-user-speed-knobs)
**Code does**: TransferArgs (cli.rs:188-364) exposes several user-facing perf knobs that ARE required for normal operation in some scenarios:
- `--force-grpc` (cli.rs has it visible; not hidden)
- `--relay-via-cli` — affects remote→remote routing
- `--detach` — operational flag
- `--workers` (hidden, satisfies "debug-only" carve-out)
- `--trace-data-plane` (hidden, satisfies carve-out)
**Evidence**:
- `crates/blit-cli/src/cli.rs:188-364` — TransferArgs (god-struct per code-cli.md smell #5)
**Notes**: `--force-grpc` and `--relay-via-cli` are visible perf-shaping flags. Could be argued they're protocol choices rather than "speed knobs" — but `--force-grpc` slows things down by intent for fallback; that's perf-shaping. Recommendation: either soften the principle text to admit transport-routing flags, or move these behind `--debug-grpc-fallback`/`--debug-relay`.

---

## Low severity

### default-port-comment-only-vs-shared-constant
**Plan says**: (implicit) shared defaults should not be duplicated
**Code does**: `9031` literal in `crates/blit-cli/src/scan.rs:63`, `crates/blit-core/src/remote/endpoint.rs:25`, `crates/blit-daemon/src/runtime.rs:201`. Inventory code-cli.md flags this as smell.
**Evidence**: as above
**Notes**: Cosmetic / maintenance.

### perf-history-cap-1-mib-matches-plan-but-cap-enforcement-best-effort
**Plan says**: "Metrics are stored locally as a capped JSON Lines file (e.g., `~/.config/blit/perf_local.jsonl`, max ~1 MiB)" (LOCAL_TRANSFER_HEURISTICS.md §5.2 / behavior-perf-history-jsonl-cap)
**Code does**: `DEFAULT_MAX_BYTES = 1_000_000` at `crates/blit-core/src/perf_history.rs:17-18` — 1 MB decimal, not 1 MiB binary; close but not exact. Cap enforcement is best-effort: "concurrent writer skips rotation" per code-core-misc.md.
**Evidence**:
- `crates/blit-core/src/perf_history.rs:17` — `DEFAULT_MAX_BYTES = 1_000_000`
**Notes**: 1 MB vs 1 MiB is 4.8% difference; doc says "max ~1 MiB" (tilde present) so this is acceptable.

### platform-variance-claim-partial
**Plan says**: "Platform Variance: Predictor maintains separate models for Windows vs. Unix, and case-insensitive vs. case-sensitive filesystems." (LOCAL_TRANSFER_HEURISTICS.md §9 / behavior-platform-variance-models)
**Code does**: Predictor's `ProfileKey` keys on `(src_fs, dest_fs, fast_path, mode, skip_unchanged, checksum)` (perf_predictor.rs:158). Filesystem strings include "apfs", "ntfs", "refs", "ext4", etc. (probe.rs:27-149) so OS-level distinction is implicit. There is no explicit case-sensitive vs case-insensitive segmentation field — neither `case_sensitive` nor `case_insensitive` appear in perf_predictor.rs or perf_history.rs.
**Evidence**:
- `crates/blit-core/src/perf_predictor.rs:158` — ProfileKey shape
- (grep) `case_sensitive` not found in perf_history/perf_predictor
**Notes**: Implicit via fs-name string (NTFS/APFS implies case-insensitive, ext4 implies case-sensitive) but no explicit field. Plan's claim is technically met (separate models per fs-name) but not in the way the plan reads (no explicit case-sensitivity flag).

### no-segmented-coefficients-for-cross-fs-claim
**Plan says**: "Cross-filesystem performance differences? Predictor coefficients segmented by source/dest FS profile; transfer engine monitors backpressure to throttle." (LOCAL_TRANSFER_HEURISTICS.md §10 / decision-cross-fs-segmented-coefficients)
**Code does**: First half is satisfied (ProfileKey carries source_fs + dest_fs). Second half — "transfer engine monitors backpressure to throttle" — is not exposed in any visible API; the pipeline does have per-sink channels (`crates/blit-core/src/remote/transfer/pipeline.rs:91-129`) but no explicit backpressure-driven throttle adjustment.
**Evidence**:
- `crates/blit-core/src/perf_predictor.rs:158` — ProfileKey
- (no match) "backpressure" / "throttle" not found in orchestrator
**Notes**: Partial — segmentation exists, backpressure-throttle does not.

### bench-cli-grammar-rejects-mirror-move-test-not-shipped
**Plan says**: "Test plan: CLI grammar rejects `blit bench mirror` / `blit bench move`." (BENCH_VERB_PLAN.md §1, §2.1, §7 / behavior-bench-grammar-rejects-mirror-move)
**Code does**: Bench verb not implemented (per high-severity bench-verb-correctly-deferred); grammar test trivially passes because subcommand doesn't exist. Not a present-tense drift; flagging that when bench lands, the test must be added.
**Evidence**: as above
**Notes**: Reminder for the v0.2.0 implementation tracker.

### tiny-fast-path-predictor-gate-vs-default-tension
**Plan says**: "Tiny manifest fast path is additionally gated by the predictor: once a profile has observations, we only bypass streaming when predicted planning time exceeds 1 s; with no history we default to fast-path for the initial runs." (LOCAL_TRANSFER_HEURISTICS.md §4 / behavior-tiny-fast-path-predictor-gate) — AND already noted in plan-perf as contradiction-tiny-fast-path-history-vs-no-history with §6.2.
**Code does**: Fast-path selection in `crates/blit-core/src/orchestrator/fast_path.rs:81-203` is driven by file count / bytes / single-file size — predictor consultation happens elsewhere (`orchestrator.rs:609-697` per `predictor-verbose-log` behavior in code-core-orch.md). The §4-§6.2 internal contradiction (default to fast-path with no history vs. enter streaming at predicted-≤1000ms) is acknowledged in the plan itself; code follows the §4 footer ("default to fast-path") via file-count/bytes thresholds and only uses predictor for verbose logging, not for the routing decision.
**Evidence**:
- `crates/blit-core/src/orchestrator/fast_path.rs:81-203`
- `crates/blit-core/src/orchestrator/orchestrator.rs:609-697` predictor-estimate emit
**Notes**: Plan's internal contradiction surfaces as: code follows §4 (file-count tiers) and uses predictor as observability only, not as routing oracle as §6.2 implies. Could be a deliberate trade-off (predictor too unstable to route on), but the plan claims it routes.

---

## Claims that align well

- **predictor linear model α/β/γ** — `α=0.05 ms/file, β=0.01 ms/MB, γ=50 ms` matches `crates/blit-core/src/perf_predictor.rs:37-39` exactly (`DEFAULT_ALPHA_MS_PER_FILE=0.05`, `DEFAULT_BETA_MS_PER_MB=0.01`, `DEFAULT_GAMMA_MS=50.0`).
- **TINY/HUGE thresholds** — `crates/blit-core/src/orchestrator/fast_path.rs:11-13`: `TINY_FILE_LIMIT=256`, `TINY_TOTAL_BYTES=256 MiB`, `HUGE_SINGLE_BYTES=1 GiB` — matches plan §4 "≤ 8 files AND ≤ 100 MB" loosely (code is more permissive: 256 files / 256 MB), and matches "Single file ≥ 1 GiB" exactly. Plan numbers were tightened up in implementation.
- **PerformanceRecord run_kind** — RunKind enum (perf_history.rs:60-78) already pre-anticipates BenchTransfer/BenchWire even though the verb is deferred; the JSONL schema_version + migrate_record migration path is plumbed (perf_history.rs:273-300).
- **No-staged-rollout exceptions correctly marked shipped** — Windows USN snapshot, macOS FSEvents, Linux metadata snapshot all have working `compare_*_snapshots` (snapshot.rs:35-79); the "shipped 2025-10-25" claim mostly holds (with the Linux limitation in finding 4).
- **R52-F1 / R54-F1 / R49-F1 etc. CLI safety gates** — `--null` rejected on mirror, on remote, on move; mirror gates on incomplete-scan; move gates on filter args. All implemented per `crates/blit-cli/src/transfers/mod.rs:114-419` and exercised by `cli_arg_safety_gates.rs` + `local_move_semantics.rs` tests. Plan's RELIABLE principle (principle-reliable-mirror-checksum-precedence) is upheld.
- **Three-roles principle (Initiator/Origin/Target)** — Conceptually present: push has initiator=origin (mod.rs:430-700), pull has initiator=target (pull.rs:680-697), delegated_pull splits all three (delegated_pull.rs:170-303). The "role-pure model" claim in PIPELINE_UNIFICATION holds for delegated remote→remote; just not yet for pull.
- **No mirror-deletion cache** — Confirmed: `apply_mirror_deletions` (orchestrator.rs:921-1056) recomputes each time, no cache layer.
- **Receive pipeline symmetry** — `execute_receive_pipeline` (pipeline.rs:200-302) used by push receive, pull receive, and remote→remote receive — symmetry preserved as claimed (shipped-receive-pipeline-symmetry-in-place).
- **FilteredSource as ingredient, not the diff** — confirmed: FilteredSource (source.rs:338) wraps an inner TransferSource; the real local diff lives in `plan_local_mirror`. Claim (rejected-filteredsource-as-unified-stage) is honored.
- **Spec_version fail-closed for v1 daemons** — confirmed: `crates/blit-core/src/remote/transfer/operation_spec.rs:107-111` rejects any non-exact version with explicit error message; v2 daemons honor `require_complete_scan`.
- **CLI display drops `:9031`** — confirmed at `crates/blit-core/src/remote/endpoint.rs:121-128` (host_port_display) for default-port elision.
- **Predictor refuses to learn from null-sink runs** — confirmed at `crates/blit-core/src/orchestrator/orchestrator.rs:860-866` (null-sink-no-predictor-train) and `crates/blit-core/src/perf_predictor.rs:391` (observe skips non-real-transfer).
