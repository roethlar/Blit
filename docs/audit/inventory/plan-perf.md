# Plan Inventory: Performance plans & heuristics
**Generated**: 2026-06-04 by audit workflow
**Coverage**: 4 files, 944 lines total

- docs/plan/LOCAL_TRANSFER_HEURISTICS.md (189 lines)
- docs/plan/PIPELINE_UNIFICATION.md (260 lines)
- docs/plan/BENCHMARK_10GBE_PLAN.md (133 lines)
- docs/plan/BENCH_VERB_PLAN.md (362 lines)

---

## Claims (grouped by category)

### principle

#### principle-fast-bytes-move-immediately
**Source**: LOCAL_TRANSFER_HEURISTICS.md §1 Design Tenets
**Specificity**: medium

"FAST: bytes start moving immediately, planner overhead stays invisible unless something genuinely stalls."

#### principle-simple-no-user-speed-knobs
**Source**: LOCAL_TRANSFER_HEURISTICS.md §1 Design Tenets
**Specificity**: high

"SIMPLE: no user-facing tuning flags; the orchestrator decides the optimal path. Debug-only overrides may exist but are not required for normal operation."

#### principle-reliable-mirror-checksum-precedence
**Source**: LOCAL_TRANSFER_HEURISTICS.md §1 Design Tenets
**Specificity**: high

"RELIABLE: mirror deletions, checksum parity, filesystem safety checks always take precedence over raw speed."

#### principle-privacy-local-only
**Source**: LOCAL_TRANSFER_HEURISTICS.md §1 Design Tenets
**Specificity**: high

"Privacy: performance history is strictly local; nothing leaves the user's machine."

#### principle-three-roles
**Source**: PIPELINE_UNIFICATION.md §Premise
**Specificity**: high

"A transfer involves three roles. They aren't always on the same machine, and the protocol shouldn't pretend they are." Roles: Initiator (parses CLI, builds spec, coordinates start/end, does not handle bulk data); Origin (holds source, enumerates, plans, sends bytes); Target (holds destination, receives bytes, writes files, optionally enforces mirror deletion).

#### principle-direction-agnostic
**Source**: PIPELINE_UNIFICATION.md §Premise
**Specificity**: high

"Push, pull, local-local, and remote-remote stop being separate code paths and become direction-agnostic arrangements of the same three roles."

#### principle-bench-honest-naming
**Source**: BENCH_VERB_PLAN.md §1 Why a verb, not `copy --null`
**Specificity**: medium

"Honest naming. `copy` implies a write happens; `bench` implies it doesn't. The current `--null` confuses operators who reach for 'why isn't the destination file there?'"

#### principle-bench-different-output-shape
**Source**: BENCH_VERB_PLAN.md §1
**Specificity**: medium

"A copy emits a transfer summary; a benchmark emits structured timing/throughput metrics. Overloading copy means consumers parsing `--json` have to branch on a flag."

#### principle-bench-different-predictor-channel
**Source**: BENCH_VERB_PLAN.md §1
**Specificity**: high

"Different predictor channel. Bench records feed a separate profile so they don't bias the predictor used by real transfers (see §6)."

---

### invariant

#### invariant-no-staged-rollout
**Source**: LOCAL_TRANSFER_HEURISTICS.md (header)
**Specificity**: high

"This document captures the full design we intend to implement in Phase 2 for Blit v2. There is no staged rollout—every mechanism described here will ship together once complete."

#### invariant-mirror-deletion-requires-full-planner
**Source**: LOCAL_TRANSFER_HEURISTICS.md §9 Safety & Reliability Guarantees
**Specificity**: high

"Mirror Deletions: Always require completed planner output; direct copy path is disabled when deletions are requested."

#### invariant-checksum-requires-full-planner
**Source**: LOCAL_TRANSFER_HEURISTICS.md §9
**Specificity**: high

"Checksums: Planner mandatory; skip logic depends on hash comparisons."

#### invariant-error-aborts-with-accumulation
**Source**: LOCAL_TRANSFER_HEURISTICS.md §9
**Specificity**: high

"Error Handling: Any planner or worker error aborts the run with accumulation of all errors (existing behaviour)."

#### invariant-one-diff-implementation
**Source**: PIPELINE_UNIFICATION.md §Why this matters
**Specificity**: high

"One diff implementation. Today's diff logic lives in three places (orchestrator local mirror, push client, pull_sync handler). Extracting DiffPlanner collapses them."

#### invariant-filter-parity-free
**Source**: PIPELINE_UNIFICATION.md §Why this matters
**Specificity**: high

"Filter parity becomes free. All filtering goes through DiffPlanner, on whatever side is the origin. No special pull-side limitation."

#### invariant-resume-universal
**Source**: PIPELINE_UNIFICATION.md §Why this matters
**Specificity**: high

"Resume becomes universal. Block-level resume is currently a feature of `pull_sync` only. Once it's a DiffPlanner capability, push gets it for free."

#### invariant-fewer-receive-path-safety-entry-points
**Source**: PIPELINE_UNIFICATION.md §Why this matters
**Specificity**: medium

"Path safety has fewer entry points. F1's safe-join helper protects the receive sink path; with one receive pipeline (instead of pull_sync's parallel one), there are fewer sites to instrument."

#### invariant-spec-version-fail-closed
**Source**: BENCH_VERB_PLAN.md §3.1
**Specificity**: high

"Bump SUPPORTED_SPEC_VERSION from 2 → 3 so old daemons fail closed instead of silently writing real data when a benchmark client expects them to discard."

#### invariant-bench-wire-requires-opt-in
**Source**: BENCH_VERB_PLAN.md §3.3
**Specificity**: high

"Authorization: gated behind a `[bench]` config section on the daemon (off by default; operator must opt in since it lets remote clients consume CPU/network with no filesystem read)."

#### invariant-receive-pipeline-symmetry-preserved
**Source**: PIPELINE_UNIFICATION.md §What this preserves
**Specificity**: medium

"The receive pipeline's symmetry (push receive, pull receive, remote→remote receive all share code) is already in place — keep it."

---

### interface

#### iface-transfer-operation-spec
**Source**: PIPELINE_UNIFICATION.md §The wire contract
**Specificity**: high

`TransferOperationSpec` proto carries: source_path (string, field 1), filter (FilterSpec, 2), compare_mode (ComparisonMode, 3), mirror_mode (MirrorMode, 4), resume (ResumeSettings, 5), capabilities (PeerCapabilities, 6), spec_version (uint32, 7). "The initiator → origin handshake carries a normalized intent message, not a flag bag."

#### iface-filter-spec
**Source**: PIPELINE_UNIFICATION.md §The wire contract
**Specificity**: high

`FilterSpec`: repeated include (1), repeated exclude (2), optional min_size (3), max_size (4), min_age_secs (5), max_age_secs (6). "files_from list streamed separately if non-empty (large lists shouldn't bloat the spec message)."

#### iface-destination-manifest-streamed-separately
**Source**: PIPELINE_UNIFICATION.md §The wire contract
**Specificity**: high

"The destination manifest is streamed separately as part of the operation, not packed into this message — manifests can be enormous."

#### iface-diff-planner-inputs-outputs
**Source**: PIPELINE_UNIFICATION.md §The pipeline
**Specificity**: high

`DiffPlanner` takes: source header stream; user filter (FileFilter); target's destination manifest stream; comparison mode (size+mtime / hash / size-only / ignore-times); capabilities (resume support, server checksums, etc.). Emits stream of `TransferPayload` variants: Drop (no payload), Full file → `TransferPayload::File`, Block-level delta → `FileBlock` + `FileBlockComplete`, Small file batch → `TarShard`.

#### iface-execute-sink-pipeline-streaming
**Source**: PIPELINE_UNIFICATION.md §The pipeline
**Specificity**: medium

"execute_sink_pipeline_streaming dispatches payloads to one or more sinks. Already exists."

#### iface-transfer-source-abstraction
**Source**: PIPELINE_UNIFICATION.md §The pipeline
**Specificity**: medium

"TransferSource enumerates the origin's filesystem. Already abstracted (FsTransferSource, RemoteTransferSource)."

#### iface-transfer-sink-abstraction
**Source**: PIPELINE_UNIFICATION.md §The pipeline
**Specificity**: medium

"TransferSink writes to the target. Already abstracted (FsTransferSink, DataPlaneSink, GrpcFallbackSink, NullSink)."

#### iface-blit-diagnostics-perf
**Source**: LOCAL_TRANSFER_HEURISTICS.md §5.2, §10
**Specificity**: high

"Use `blit diagnostics perf --disable` (and `--enable`) to toggle recording in the local config directory." Metrics surface via `blit diagnostics perf` (local only).

#### iface-debug-workers-flag
**Source**: LOCAL_TRANSFER_HEURISTICS.md §7
**Specificity**: high

"Optional debug limiter (`--workers`) caps worker count for diagnostics. The flag remains hidden from normal help output; when active the CLI prints a `[DEBUG] Worker limiter active` banner and FAST guarantees are suspended."

#### iface-verbose-progress
**Source**: LOCAL_TRANSFER_HEURISTICS.md §3.2, §3.4
**Specificity**: medium

"CLI progress spinner (indicatif) tracks elapsed time and surfaces final throughput when `--progress` is supplied; otherwise the CLI stays quiet, while `--verbose` keeps raw planner logs." "`--verbose` shows real-time heartbeat messages (e.g. `Planning… 12,345 entries classified; queue depth 8`)."

#### iface-bench-transfer-subcommand
**Source**: BENCH_VERB_PLAN.md §2.1
**Specificity**: high

`blit bench transfer <SRC> <DST>`: "Real source reads, real network, destination uses `NullSink`. Measures: source-side read pipeline + planning + network (+ on remote paths, daemon-side pipeline) — but not the destination disk write cost."

#### iface-bench-wire-subcommand
**Source**: BENCH_VERB_PLAN.md §2.2
**Specificity**: high

`blit bench wire [--size=N] [--files=N] <SRC-HOST> <DST-HOST>`: synthetic bytes from source daemon, streamed to destination null sink. "Measures: pure daemon-to-daemon throughput, no filesystem on either end." `--size` controls total bytes; `--files` controls payload count.

#### iface-discard-writes-spec-field
**Source**: BENCH_VERB_PLAN.md §3.1
**Specificity**: high

New field `TransferOperationSpec.discard_writes: bool`. Honored by push handler (selects NullSink), DelegatedPull handler (selects NullSink for local writes). pull_sync handler: daemon as source — field doesn't change source behavior, but carries through to DelegatedPull.

#### iface-push-control-discard-writes
**Source**: BENCH_VERB_PLAN.md §3.2
**Specificity**: high

"Push uses its own control protocol (ClientPushRequest). Add a `discard_writes: bool` to the push start message (or another existing init message — wherever target capabilities get negotiated). Same semantic as the spec field."

#### iface-bench-synthesize-rpc
**Source**: BENCH_VERB_PLAN.md §3.3
**Specificity**: medium

New RPC `BenchSynthesize`: `{ size_bytes, file_count, payload_pattern }`. Stream of synthetic `FileHeader` + payload bytes through the normal data-plane machinery. "Alternative shape: piggyback on `pull_sync` with a new 'synthetic source' flag on the spec. Decide during impl — piggyback is simpler if the existing flow accommodates fake manifests cleanly."

#### iface-bench-config-section
**Source**: BENCH_VERB_PLAN.md §4 blit-daemon
**Specificity**: high

"New config section `[bench]` with `allow_synthetic: bool` (default false) so operators must opt in to letting remote clients consume CPU + network for synthetic benchmarks."

#### iface-bench-output-json
**Source**: BENCH_VERB_PLAN.md §5
**Specificity**: high

JSON output fields: operation, src, dst, files, bytes_read, bytes_sent_wire, bytes_received_dst, bytes_discarded, streams, tcp_fallback_used, durations_ms{scan, plan, transfer, total}, throughput_mibps{read, wire, received}, kind.

#### iface-bench-output-human
**Source**: BENCH_VERB_PLAN.md §5
**Specificity**: high

Human form is 6-line block including: benchmark name, source, destination (null sink), workload, duration (scan/plan/transfer), throughput with streams and data plane type.

#### iface-performance-record-kind
**Source**: BENCH_VERB_PLAN.md §4 blit-core
**Specificity**: high

"Tag `PerformanceRecord` with `kind: TransferKind` where `TransferKind = { Copy, Mirror, BenchTransfer, BenchWire }`. Bump the JSONL schema version, migrate older records via `migrate_record`."

#### iface-predictor-kind-bucket
**Source**: BENCH_VERB_PLAN.md §6
**Specificity**: high

"Extend `PerformancePredictor` v3 (bump schema) with a second profile bucket keyed on `TransferKind`." Copy/mirror profile = records where kind ∈ {Copy, Mirror}; bench profile = records where kind ∈ {BenchTransfer, BenchWire}. `predictor.observe(record)` routes by kind; `predictor.predict(intent: TransferKind)` adds intent parameter; `blit profile --json` surfaces both buckets.

#### iface-synthetic-transfer-source
**Source**: BENCH_VERB_PLAN.md §4 blit-core
**Specificity**: high

"`SyntheticTransferSource: TransferSource` that emits N file headers totaling M bytes of zero-fill (or a cheap PRNG fill if zeros compress trivially over wire) without touching the filesystem. Lives in `blit-core::remote::transfer::source`."

---

### behavior

#### behavior-streaming-planner
**Source**: LOCAL_TRANSFER_HEURISTICS.md §3.1
**Specificity**: high

"Convert `TransferFacade::build_local_plan` into a streaming producer that classifies entries as they appear. Batches (large-file, raw bundle, tar shard) are emitted incrementally to the TransferEngine. Workers start as soon as the first batch is emitted; there is no requirement to wait for whole-manifest planning."

#### behavior-heartbeat-1s-default
**Source**: LOCAL_TRANSFER_HEURISTICS.md §3.2
**Specificity**: high

"The orchestrator maintains a 1 s heartbeat timer. At each tick it flushes whatever batches are ready to the worker queue."

#### behavior-heartbeat-adaptive
**Source**: LOCAL_TRANSFER_HEURISTICS.md §3.2
**Specificity**: high

"Flush cadence dynamically adjusts based on queue saturation: 1000 ms while the queue is empty, tightening to 500 ms if workers are draining results quickly, relaxing back when the queue fills."

#### behavior-no-timeout-while-progress
**Source**: LOCAL_TRANSFER_HEURISTICS.md §3.3
**Specificity**: high

"As long as either the planner emits new batches or workers report progress, no timeout triggers. Planning is free to exceed 1 s when necessary (e.g., large manifests)."

#### behavior-10s-stall-timeout
**Source**: LOCAL_TRANSFER_HEURISTICS.md §3.3
**Specificity**: high

"If both planner output and worker progress are idle for 10 s, the orchestrator aborts with a precise message: e.g., `Planner stalled while reading /path/to/dir; aborting after 10s without progress.` The prior 30 s timeout concept is dropped; 10 s is the hard limit for no-progress scenarios."

#### behavior-tiny-manifest-fast-path
**Source**: LOCAL_TRANSFER_HEURISTICS.md §4
**Specificity**: high

"≤ 8 files AND ≤ 100 MB total AND no mirror/checksum/tar → Direct sequential copy. Planner overhead dominates; reuse `copy::copy_file` path." Additionally gated by predictor.

#### behavior-large-single-file-fast-path
**Source**: LOCAL_TRANSFER_HEURISTICS.md §4
**Specificity**: high

"Single file ≥ 1 GiB → Dispatch immediately to large-file (zero-copy) worker. Planner continues streaming remaining entries."

#### behavior-tiny-fast-path-predictor-gate
**Source**: LOCAL_TRANSFER_HEURISTICS.md §4
**Specificity**: high

"Tiny manifest fast path is additionally gated by the predictor: once a profile has observations, we only bypass streaming when predicted planning time exceeds 1 s; with no history we default to fast-path for the initial runs."

#### behavior-perf-history-jsonl-cap
**Source**: LOCAL_TRANSFER_HEURISTICS.md §5.2
**Specificity**: high

"Metrics are stored locally as a capped JSON Lines file (e.g., `~/.config/blit/perf_local.jsonl`, max ~1 MiB). Each entry includes: timestamp, workload signature (file count, total bytes, flags), planning_ms, copy_ms, stall_count, filesystem profile."

#### behavior-perf-history-windowed
**Source**: LOCAL_TRANSFER_HEURISTICS.md §5.3
**Specificity**: high

"Adaptive logic reads only the most recent N entries (default 50) to inform planning-time predictions."

#### behavior-predictor-linear-model
**Source**: LOCAL_TRANSFER_HEURISTICS.md §6.1
**Specificity**: high

"Linear combination of factors: `planning_ms ≈ α * files + β * total_bytes + γ`. Coefficients updated via exponential moving average after each run. Separate coefficients maintained per filesystem profile (e.g., SSD, HDD, network share). Initialized with conservative defaults (e.g., α=0.05 ms/file, β=0.01 ms/MB) so predictions err on the side of using the planner."

#### behavior-predictor-routing-1s-threshold
**Source**: LOCAL_TRANSFER_HEURISTICS.md §6.2
**Specificity**: high

"If predicted planning_ms ≤ 1000 ms: enter streaming planner immediately. If predicted planning_ms > 1000 ms and fast-path conditions are met (Section 4), use fast-path. If predicted planning_ms > 1000 ms and no fast-path applies, still enter streaming planner but emit a verbose warning."

#### behavior-predictor-self-correction
**Source**: LOCAL_TRANSFER_HEURISTICS.md §6.3
**Specificity**: high

"After each run, record actual planning_ms. If prediction error exceeds 25%, adjust coefficients more aggressively."

#### behavior-predictor-opt-out-fallback
**Source**: LOCAL_TRANSFER_HEURISTICS.md §6.4
**Specificity**: high

"When performance history capture is disabled, prediction falls back to conservative defaults and no updates occur."

#### behavior-buffer-sizes-auto
**Source**: LOCAL_TRANSFER_HEURISTICS.md §7
**Specificity**: medium

"The planner automatically selects aggressive buffer sizes, tar shard targets, and worker counts based on workload and available CPU (no manual speed flags)."

#### behavior-small-file-tar-shard-thresholds
**Source**: LOCAL_TRANSFER_HEURISTICS.md §7
**Specificity**: high

"Small-file workloads (≥32 sub-1 MiB files or avg size ≤64 KiB) immediately enter the tar-shard path; shards flush around 8 MiB/≈1 k files and scale up to 32/64 MiB as manifests grow, keeping per-file overhead invisible. Recent performance history nudges these thresholds to mirror what actually worked best on the current machine."

#### behavior-worker-count-default
**Source**: LOCAL_TRANSFER_HEURISTICS.md §7
**Specificity**: high

"Default worker count = `num_cpus::get()` (with safeguards for hyper-threaded vs. physical cores). Upper bound clamps to 16 by default but adapts if the machine proves capable."

#### behavior-quiet-cli-during-transfer
**Source**: LOCAL_TRANSFER_HEURISTICS.md §7
**Specificity**: medium

"CLI stays quiet during transfers; progress events are emitted for verbose/log subscribers and GUI surfaces."

#### behavior-buffer-memory-pressure-detection
**Source**: LOCAL_TRANSFER_HEURISTICS.md §7
**Specificity**: medium

"Buffer sizing logic evaluates run-time conditions (e.g., detect memory pressure via `sysinfo`) to avoid over-allocating on small systems."

#### behavior-platform-variance-models
**Source**: LOCAL_TRANSFER_HEURISTICS.md §9
**Specificity**: medium

"Platform Variance: Predictor maintains separate models for Windows vs. Unix, and case-insensitive vs. case-sensitive filesystems."

#### behavior-pull-cli-not-doing-enumeration
**Source**: PIPELINE_UNIFICATION.md §Where each role lives
**Specificity**: high

"The CLI's job in pull is not to do enumeration or comparison work — it's to ship a normalized operation spec to daemon A and then stand by as a receive-side sink. Daemon A runs its source pipeline and streams to daemon B (or back to the CLI when daemon B is the CLI)."

#### behavior-local-copy-pipeline
**Source**: PIPELINE_UNIFICATION.md §Local copy/mirror
**Specificity**: medium

"Initiator builds the spec, hands it to the origin (which is itself). Origin runs FsTransferSource → DiffPlanner → pipeline → FsTransferSink. No protocol involved."

#### behavior-push-pipeline
**Source**: PIPELINE_UNIFICATION.md §Push
**Specificity**: high

"Initiator builds spec. Initiator is also origin: it enumerates locally, runs DiffPlanner against the destination manifest the daemon ships up, streams payloads via DataPlaneSink to the daemon. Daemon is target: runs the receive pipeline writing into FsTransferSink."

#### behavior-pull-pipeline
**Source**: PIPELINE_UNIFICATION.md §Pull
**Specificity**: high

"Initiator builds spec, sends it to daemon over the new `ExecuteOriginJob` RPC (or extension of an existing one). Daemon is origin: enumerates locally, receives the destination manifest the initiator streams up, runs DiffPlanner, streams payloads back via DataPlaneSink to the initiator. Initiator is target: runs the receive pipeline writing into FsTransferSink."

#### behavior-remote-remote-direct
**Source**: PIPELINE_UNIFICATION.md §Remote→remote
**Specificity**: high

"Initiator builds spec, hands it to daemon A: 'you're the origin, your target is daemon B.' Daemon A enumerates, requests daemon B's destination manifest directly (no relay through the initiator), runs DiffPlanner, streams payloads to daemon B via DataPlaneSink. Daemon B is target: runs receive pipeline. Initiator just observes progress."

#### behavior-bench-direction-matrix
**Source**: BENCH_VERB_PLAN.md §2.1
**Specificity**: high

`bench transfer` direction matrix: local→local = FsTransferSource → NullSink locally; local→remote = push manifest as usual, daemon swaps in NullSink on receive via wire flag; remote→local = pull as usual, CLI swaps in NullSink locally; remote→remote = direct delegated pull (no `--relay-via-cli`), destination daemon receives via NullSink.

#### behavior-bench-wire-local-rejected
**Source**: BENCH_VERB_PLAN.md §2.2
**Specificity**: high

"Local-only? No. `bench wire` only makes sense between two daemons (or arguably localhost-to-localhost-on-two-daemons for loopback testing). Reject the local→local case at the CLI with a pointer at `bench transfer` for that direction."

#### behavior-bench-grammar-rejects-mirror-move
**Source**: BENCH_VERB_PLAN.md §1, §2.1, §7
**Specificity**: high

"`blit bench` can refuse mirror/move semantics at the CLI grammar level (they're not bench subcommands) instead of as runtime args-rejection guards." Test plan: "CLI grammar rejects `blit bench mirror` / `blit bench move`."

---

### scope

#### scope-implementation-checklist
**Source**: LOCAL_TRANSFER_HEURISTICS.md §8
**Specificity**: high

Eight implementation lanes: Streaming Planner Refactor; Heartbeat Scheduler; Fast-Path Integration; Telemetry Store; Timeout & Messaging; CLI Cleanup (remove `--ludicrous-speed`); Testing & Benchmarks (Phase 2.5 suite covers 1-file, 8-file, 100k-file, cross-FS).

#### scope-priority-sequence-pipeline
**Source**: PIPELINE_UNIFICATION.md §Priority sequence
**Specificity**: high

Five-step sequence: (1) F1 receive-side path safety prereq; (2) TransferOperationSpec proto + Rust mirrors; (3) DiffPlanner extracted to `blit-core::remote::transfer::diff_planner`; (4) Refactor pull_sync.rs to unified pipeline (old PullSync stays as deprecated path behind capability flag); (5) Revisit remote→remote.

#### scope-bench-impl-tasks-per-crate
**Source**: BENCH_VERB_PLAN.md §4
**Specificity**: high

blit-core: discard_writes proto + bump SUPPORTED_SPEC_VERSION → 3; plumb through NormalizedTransferOperation::from_spec; PerformanceRecord.kind + JSONL bump + migrate_record; SyntheticTransferSource. blit-daemon: push receive honors discard_writes; DelegatedPull handler honors discard_writes; `[bench]` config section; BenchSynthesize RPC or pull_sync flag. blit-cli: `Bench { command: BenchCommand }` clap subcommand; remove `--null` and R54-F1 guards; update cli_arg_safety_gates.rs; bench output formatting (human + JSON); update README.md, docs/cli/blit.1.md, BENCHMARK_10GBE_PLAN.md.

#### scope-bench-sequencing
**Source**: BENCH_VERB_PLAN.md §10
**Specificity**: high

Eight-step sequence: (1) Land 0.1.0 with current `--null` narrowing intact; (2) Capture §2.6 hardware-bound numbers via existing playbook; (3) Spec field + proto bump → 3; (4) Daemon-side null sink wiring (push receive + DelegatedPull); (5) CLI scaffolding for `bench transfer` only; (6) SyntheticTransferSource + `bench wire`; (7) Predictor bucket split (can land before or after bench verbs); (8) Remove `--null` and its guards; update docs. "Cost estimate: ~3-4 days of focused work, plus the §2.6 capture wall-clock."

#### scope-bench-test-plan
**Source**: BENCH_VERB_PLAN.md §7
**Specificity**: high

Unit tests: SyntheticTransferSource emits correct file/byte counts; discard_writes round-trips proto serialize + from_spec normalize; predictor splits Copy/Bench profiles correctly. Integration: bench transfer LOCAL/LOCAL; LOCAL/REMOTE; REMOTE/LOCAL; REMOTE/REMOTE (delegated path); bench wire with allow_synthetic=true; bench wire refused with default config; CLI grammar rejects bench mirror/move.

#### scope-benchmark-phases-10gbe
**Source**: BENCHMARK_10GBE_PLAN.md (entire doc)
**Specificity**: high

Five-phase plan: Phase 1 = local-only (no network) to validate unified pipeline; Phase 2 = local→NFS/SMB mount; Phase 3 = remote push/pull with daemon on TrueNAS; Phase 4 = reverse direction (daemon on this machine); Phase 5 = stress test (4 GiB single file; 100k small files).

#### scope-benchmark-binary-build
**Source**: BENCHMARK_10GBE_PLAN.md §Prerequisites
**Specificity**: high

"cd ~/dev/Blit; cargo build --release". TrueNAS setup: create dataset `blit-bench` on pool; create NFS share exporting `/mnt/<pool>/blit-bench`; optionally create SMB share; copy daemon binary via scp.

#### scope-benchmark-recording
**Source**: BENCHMARK_10GBE_PLAN.md §Recording Results
**Specificity**: medium

"After all phases, results are in `logs/bench_10gbe_*/`. To document: (1) Copy the best results.csv into CHANGELOG.md benchmark section; (2) Update TODO.md — check off the three benchmark items; (3) Note any issues found (throughput bottlenecks, errors, etc.)."

#### scope-todo-items-covered
**Source**: BENCHMARK_10GBE_PLAN.md §TODO Items Covered
**Specificity**: medium

Plan covers three TODO items: Benchmark remote fallback + data-plane streaming (line 78); Benchmark TCP data plane throughput targeting 10+ Gbps (line 98); Capture remote benchmark runs TCP vs gRPC fallback (line 116).

---

### non-goal

#### nongoal-no-web-http-metrics
**Source**: PIPELINE_UNIFICATION.md §What we're not doing yet
**Specificity**: high

"Web/HTTP exposure of metrics — already removed; counters stay internal until a future GUI/TUI consumer needs them."

#### nongoal-no-daemon-auth-tls
**Source**: PIPELINE_UNIFICATION.md §What we're not doing yet
**Specificity**: high

"Daemon-side authentication/TLS — operator's responsibility per existing docs; out of scope for pipeline unification."

#### nongoal-no-backcompat-old-daemons
**Source**: PIPELINE_UNIFICATION.md §What we're not doing yet
**Specificity**: medium

"Backwards-compat layer for old daemons. Versioning via spec_version and capability flags should let new clients fall back, but spec-out the upgrade path before deletion."

#### nongoal-no-mirror-deletion-cache
**Source**: LOCAL_TRANSFER_HEURISTICS.md §10
**Specificity**: high

"Cache mirror deletion plans? No; correctness risk outweighs gain."

#### nongoal-no-hardcoded-hardware-rules
**Source**: LOCAL_TRANSFER_HEURISTICS.md §10
**Specificity**: high

"Different thresholds for low-powered hardware? Managed automatically by adaptive predictor; no hard-coded per-hardware rules."

#### nongoal-bench-no-tail-latency
**Source**: BENCH_VERB_PLAN.md §9 Out of scope for 0.2.0
**Specificity**: high

"Latency/p99 measurements. Throughput-only for now. Tail-latency benchmarking needs different instrumentation (per-payload timestamps, histogram aggregation) and is its own piece of work."

#### nongoal-bench-no-concurrent-runs
**Source**: BENCH_VERB_PLAN.md §9
**Specificity**: high

"Concurrent benchmark runs. Single bench operation at a time. Concurrent stress-testing is a separate verb."

#### nongoal-bench-no-disk-only-read-mode
**Source**: BENCH_VERB_PLAN.md §9
**Specificity**: medium

"Disk-only `bench read` mode. GPT's earlier framing mentioned a source-read-only mode (no destination at all). Useful but easy to add later as `bench transfer SRC /dev/null` once we have a 'local null destination' alias — defer until someone asks for it."

#### nongoal-bench-no-rsync-compare-builtin
**Source**: BENCH_VERB_PLAN.md §9
**Specificity**: medium

"Comparison against rsync/scp built into the verb. Operators can wrap `bench transfer` with their own comparison harness; `scripts/bench_10gbe.sh` already does this for the §2.6 workflow."

#### nongoal-bench-not-in-0-1-0
**Source**: BENCH_VERB_PLAN.md (header)
**Specificity**: high

"Status: Draft. Not in 0.1.0 scope. Captured here so it doesn't get lost; ties to RELEASE_PLAN_v2_2026-05-04.md §2.6 (live remote benchmark capture)."

#### nongoal-fanotify-optional
**Source**: LOCAL_TRANSFER_HEURISTICS.md §10
**Specificity**: medium

"OS-specific event logs (USN journal)? Windows USN fast-path shipped 2025-10-25. macOS FSEvents snapshot added 2025-10-25 (verified). Linux metadata snapshot (device/inode/ctime) added 2025-10-25; further fanotify integration optional."

---

### shipped

#### shipped-windows-usn-fast-path
**Source**: LOCAL_TRANSFER_HEURISTICS.md §10
**Specificity**: high

"Windows USN fast-path shipped 2025-10-25."

#### shipped-macos-fsevents-snapshot
**Source**: LOCAL_TRANSFER_HEURISTICS.md §10
**Specificity**: high

"macOS FSEvents snapshot added 2025-10-25 (verified)."

#### shipped-linux-metadata-snapshot
**Source**: LOCAL_TRANSFER_HEURISTICS.md §10
**Specificity**: high

"Linux metadata snapshot (device/inode/ctime) added 2025-10-25; further fanotify integration optional."

#### shipped-receive-pipeline-symmetry-in-place
**Source**: PIPELINE_UNIFICATION.md §What this preserves
**Specificity**: medium

"The receive pipeline's symmetry (push receive, pull receive, remote→remote receive all share code) is already in place — keep it."

#### shipped-filteredsource-decorator-recent
**Source**: PIPELINE_UNIFICATION.md §What this preserves
**Specificity**: medium

"FilteredSource decorator from the recent commit becomes one ingredient fed into DiffPlanner, not the diff itself."

---

### deferred

#### deferred-fanotify-inotify-enhancements
**Source**: LOCAL_TRANSFER_HEURISTICS.md §11
**Specificity**: medium

"Finalise any remaining fanotify/inotify enhancements; metadata snapshot in place for Linux (Windows/macOS fast-paths shipped 2025-10-25)."

#### deferred-incremental-journal-fast-path
**Source**: LOCAL_TRANSFER_HEURISTICS.md §11
**Specificity**: medium

"Add incremental fast-path that consumes filesystem change journals where available; re-run the 0-change incremental benchmark afterwards to document the delta."

#### deferred-predictor-regression-suite
**Source**: LOCAL_TRANSFER_HEURISTICS.md §11
**Specificity**: medium

"Define an automated regression suite for the adaptive predictor (validation of parsing, coefficient updates, accuracy, and runtime overhead)."

#### deferred-gpu-checksum
**Source**: LOCAL_TRANSFER_HEURISTICS.md §11
**Specificity**: low

"Explore GPU/accelerated hashing for checksum mode."

#### deferred-remote-perf-history-opt-in
**Source**: LOCAL_TRANSFER_HEURISTICS.md §11
**Specificity**: medium

"Consider remote performance history opt-in to improve heuristics globally (opt-in only)."

#### deferred-max-threads-revisit
**Source**: LOCAL_TRANSFER_HEURISTICS.md §11
**Specificity**: medium

"Revisit `--max-threads` flag usage; deprecate if unused."

#### deferred-remote-remote-direct-decision
**Source**: PIPELINE_UNIFICATION.md §Priority sequence step 5
**Specificity**: medium

"Revisit remote→remote (architectural decision). Decide whether daemon A → daemon B should bypass the initiator. If yes: define daemon-to-daemon ExecuteOriginJob invocation, keep initiator as the spec-shipper and progress observer only. Probably yes long-term, but evaluate after pull is unified."

#### deferred-bench-verb-0-2-0
**Source**: BENCH_VERB_PLAN.md (header), §10
**Specificity**: high

`blit bench` verb deferred to 0.2.0. Sequence step 1: "Land 0.1.0 with the current `--null` narrowing intact."

---

### rejected

#### rejected-ludicrous-speed-flag
**Source**: LOCAL_TRANSFER_HEURISTICS.md §8 step 6
**Specificity**: high

"Remove `--ludicrous-speed`; no compatibility shim required prior to CLI v2."

#### rejected-30s-timeout
**Source**: LOCAL_TRANSFER_HEURISTICS.md §3.3
**Specificity**: high

"The prior 30 s timeout concept is dropped; 10 s is the hard limit for no-progress scenarios."

#### rejected-null-flag-on-copy
**Source**: BENCH_VERB_PLAN.md §1, §8
**Specificity**: high

"`--null` is removed in this work. The CLI rejections added in R54-F1 disappear with the flag." Replaced by `blit bench` verb. The R52-F1 move guard for `--null` and R54-F1 docstring also removed.

#### rejected-filteredsource-as-unified-stage
**Source**: PIPELINE_UNIFICATION.md §The pipeline
**Specificity**: high

"The crucial correction: the unified stage is DiffPlanner, not FilteredSource. FilteredSource is a special case that handles only the user-filter input. The full diff requires destination-manifest + comparison-mode + capabilities to produce the right payload types, particularly for resume."

#### rejected-cli-mediated-remote-remote
**Source**: PIPELINE_UNIFICATION.md §What this replaces
**Specificity**: high

"CLI-mediated remote→remote relay (eventually)." Currently CLI runs `RemoteTransferSource` to pull from A and re-pushes to B; "the role-pure model takes the CLI out of the data path; bytes go A → B directly."

#### rejected-filter-parity-pull-bail
**Source**: PIPELINE_UNIFICATION.md §What this replaces
**Specificity**: high

"The current 'filter parity' workaround that bails on pull when filter args are passed (CLI side)."

#### rejected-custom-pull-sync-code
**Source**: PIPELINE_UNIFICATION.md §What this replaces
**Specificity**: high

"The custom `service/pull_sync.rs` enumeration + diff + stream code."

---

### decision

#### decision-perf-summary-exposed
**Source**: LOCAL_TRANSFER_HEURISTICS.md §10
**Specificity**: high

"Do we expose performance summaries? Yes, via `blit diagnostics perf` (local only)."

#### decision-cross-fs-segmented-coefficients
**Source**: LOCAL_TRANSFER_HEURISTICS.md §10
**Specificity**: high

"Cross-filesystem performance differences? Predictor coefficients segmented by source/dest FS profile; transfer engine monitors backpressure to throttle."

#### decision-old-pull-sync-deprecated-path
**Source**: PIPELINE_UNIFICATION.md §Priority sequence step 4
**Specificity**: medium

"Old PullSync protocol stays as a deprecated path for rollout compatibility (marked behind a capability flag)."

#### decision-null-sink-retained-routed-via-bench
**Source**: BENCH_VERB_PLAN.md §8
**Specificity**: high

"The existing NullSink in `blit-core::remote::transfer::sink` stays — it's now reachable through `bench transfer` instead of through `--null`-routed `local::run_local_transfer`."

#### decision-bench-not-applicable-to-mirror-move
**Source**: BENCH_VERB_PLAN.md §2.1
**Specificity**: high

"Mirror/move semantics: not applicable. `bench transfer` is a copy-shaped operation; the subcommand grammar doesn't expose those modes."

#### decision-streaming-only-for-non-fast-path
**Source**: LOCAL_TRANSFER_HEURISTICS.md §6.2
**Specificity**: high

"If predicted planning_ms > 1000 ms and no fast-path applies, still enter streaming planner but emit a verbose warning (`expected planning time >1s; continuing due to mirror/checksum requirements`). When verbose mode is enabled, report the predictor estimate before entering the planner; defaults remain quiet unless a stall occurs."

---

### Benchmark targets (numeric)

#### target-local-no-op-mirror-100ms
**Source**: BENCHMARK_10GBE_PLAN.md Phase 1
**Specificity**: high

"No-op mirror runs complete in <100ms (journal fast-path)"

#### target-local-matches-or-beats-rsync
**Source**: BENCHMARK_10GBE_PLAN.md Phase 1
**Specificity**: high

"blit matches or beats rsync on all workloads"

#### target-large-file-nfs-10gbps
**Source**: BENCHMARK_10GBE_PLAN.md Phase 2
**Specificity**: high

"Large file throughput approaches 10 Gbps (~1.1 GB/s)"

#### target-tcp-push-5gbps
**Source**: BENCHMARK_10GBE_PLAN.md Phase 3
**Specificity**: high

"TCP push large file: target >5 Gbps (>625 MB/s)" and "TCP pull large file: similar throughput."

#### target-tcp-vs-grpc-2-to-5x
**Source**: BENCHMARK_10GBE_PLAN.md Phase 3
**Specificity**: high

"TCP vs gRPC fallback: TCP should be 2-5× faster"

#### target-first-payload-under-1s
**Source**: BENCHMARK_10GBE_PLAN.md Phase 3
**Specificity**: high

"First payload timing visible in `-v` output (target <1s for all workloads)"

#### target-small-file-tar-shard-batching
**Source**: BENCHMARK_10GBE_PLAN.md Phase 3
**Specificity**: medium

"Small file push/pull: tar shard batching should keep throughput reasonable"

#### target-stress-4gib-single-file
**Source**: BENCHMARK_10GBE_PLAN.md Phase 5
**Specificity**: high

Stress: `REMOTE_HOST=<truenas-ip> SIZE_MB=4096 SMALL_COUNT=0 RUNS=3 ./scripts/bench_10gbe.sh` (4 GiB single file).

#### target-stress-100k-small-files
**Source**: BENCHMARK_10GBE_PLAN.md Phase 5
**Specificity**: high

Stress: `REMOTE_HOST=<truenas-ip> SIZE_MB=0 SMALL_COUNT=100000 RUNS=1 ./scripts/bench_10gbe.sh` (100k small files).

#### target-phase-2-5-bench-suite-coverage
**Source**: LOCAL_TRANSFER_HEURISTICS.md §8 step 7
**Specificity**: high

"Phase 2.5 benchmark suite expanded to include 1-file, 8-file, 100 k-file cases and cross-FS copies."

---

## Contradictions (within cluster)

### contradiction-bench-plan-vs-10gbe-doc-on-null
**Source**: BENCH_VERB_PLAN.md §4 vs BENCHMARK_10GBE_PLAN.md
**Description**: BENCH_VERB_PLAN.md §4 says to "Update `BENCHMARK_10GBE_PLAN.md` so the §2.6 playbook uses `bench transfer` and `bench wire` instead of any prior `--null` invocations (the doc currently doesn't reference `--null` so likely no change, but verify)." This frames a possible inconsistency — but the actual BENCHMARK_10GBE_PLAN.md doc has no references to `bench transfer`/`bench wire` either; it relies on raw `blit copy` + a `scripts/bench_10gbe.sh` harness. The bench-verb plan implicitly expects integration that the current 10GbE plan does not reflect (one says bench will live in a verb; the other still drives via raw copy).

### contradiction-data-plane-streams-vs-grpc-fallback
**Source**: BENCH_VERB_PLAN.md §5 output vs BENCHMARK_10GBE_PLAN.md Phase 3
**Description**: The bench output includes `streams` (data-plane stream count) and `tcp_fallback_used: bool`, modeling TCP↔gRPC fallback as a binary toggle. The 10GbE plan instead frames "TCP vs gRPC fallback: TCP should be 2-5× faster" as a measurement question. These are consistent in spirit but slightly mismatched: the verb plan presumes a single negotiated path per run, while the 10GbE plan implies separate runs to compare paths. Minor.

### contradiction-supported-spec-version
**Source**: BENCH_VERB_PLAN.md §3.1, §10
**Description**: BENCH_VERB_PLAN.md says to "Bump SUPPORTED_SPEC_VERSION from 2 → 3", but PIPELINE_UNIFICATION.md introduces `spec_version` as a new field on `TransferOperationSpec` with rollout strategy itself listed under "Open questions for later steps" (§Open questions for later steps). The two plans agree the field exists but disagree on the assumed starting version: PIPELINE_UNIFICATION treats spec_version as freshly introduced (no concrete starting number stated), while BENCH_VERB_PLAN assumes version 2 already exists prior to the bench feature work. The "from 2" implies the pipeline-unification work has already shipped before the bench bump — but PIPELINE_UNIFICATION is itself only in planning per its header.

### contradiction-bench-records-and-predictor-coupling
**Source**: BENCH_VERB_PLAN.md §6 vs LOCAL_TRANSFER_HEURISTICS.md §6
**Description**: LOCAL_TRANSFER_HEURISTICS.md §6 defines a single predictor model with per-filesystem coefficients (α, β, γ). BENCH_VERB_PLAN.md §6 extends it with a `TransferKind`-keyed bucket so bench data doesn't bias the copy predictor. The heuristics doc never mentions bench-data isolation; the bench plan assumes the predictor extension. Not strictly contradictory, but the heuristics doc would need amending to reflect the v3 predictor schema and `kind`-routed observe/predict surface.

### contradiction-tiny-fast-path-history-vs-no-history
**Source**: LOCAL_TRANSFER_HEURISTICS.md §4 footer vs §6.2
**Description**: Section 4 footer says: "with no history we default to fast-path for the initial runs." Section 6.2 says: "If predicted planning_ms ≤ 1000 ms: enter streaming planner immediately." With no history, §6.1 defaults are α=0.05 ms/file, β=0.01 ms/MB, γ=0 — which for any small workload predicts well under 1000 ms, so the streaming planner should be entered immediately. That contradicts §4's "default to fast-path for the initial runs." Either the fast-path footer needs qualification ("only when other Section-4 conditions are also met") or §6.2 needs an exception carve-out for first-run/no-history scenarios.

### contradiction-tar-shard-flush-size-scale
**Source**: LOCAL_TRANSFER_HEURISTICS.md §7
**Description**: "shards flush around 8 MiB/≈1 k files and scale up to 32/64 MiB as manifests grow" — the "32/64" range is ambiguous (does it cap at 32 MiB or 64 MiB? what is the trigger for moving from 32 to 64?). The §3.2 heartbeat cadence is precise (1000 ms / 500 ms) but tar-shard flush thresholds are vague.

---

## Coverage attestation

| File | Lines | Notes |
|------|-------|-------|
| docs/plan/LOCAL_TRANSFER_HEURISTICS.md | 189 | Read in full (single Read). All 11 sections covered including resolution table and future-work bullets. |
| docs/plan/PIPELINE_UNIFICATION.md | 260 | Read in full (single Read). All sections including Premise, role tables, proto sketches, priority sequence, preserve/replace/non-goal lists, open questions. |
| docs/plan/BENCHMARK_10GBE_PLAN.md | 133 | Read in full. All five phases + prerequisites + recording + TODO mapping. |
| docs/plan/BENCH_VERB_PLAN.md | 362 | Read in full. All 10 sections including motivation, subcommands, wire changes, per-crate impl, output spec, predictor integration, test plan, sequencing. |

**Total lines**: 944
