# Blit v2 Roadmap Audit — 2026-05-04

**Auditor:** Roadmap audit pass (read-only — no code changes).
**Scope:** Every concrete planned feature appearing in `docs/plan/*` and
`docs/PERFORMANCE_ROADMAP.md`, classified against current code, commits,
and integration tests.
**Method:** See *Methodology* appendix at end.

## Summary

Approximately 95 distinct feature claims were audited across 17 plan
documents plus `TODO.md`/`DEVLOG.md`/two reviews. State distribution:

| State        | Count | What it means |
|--------------|------:|---------------|
| SHIPPING     | ~64   | Implementation present, called from production code paths, has tests. |
| PARTIAL      | ~12   | Code present but feature is dead, gated, missing a consumer, or has documented gaps. |
| DEFERRED     | ~7    | Plan explicitly defers (RDMA, AI telemetry, structured logging, TLS, BlitAuth, TUI features, `--detach`). |
| SUPERSEDED   | ~5    | Older plan reshaped by a later one (most v4/v5 architecture in `greenfield_plan_v6.md`, push/pull split → Pipeline Unification). |
| NOT-STARTED  | ~5    | No implementation found and not deferred (live remote benchmarks, mDNS TXT enrichment, `Subscribe`/`GetState` RPCs, `Run benchmark` items). |
| UNVERIFIED   | ~2    | macOS FSEvents fast-path verification run, Windows ReFS privilege investigation. |

**Headlines:**

1. The `PROJECT_STATE_ASSESSMENT.md` "feature-complete as of 2026-04-07"
   claim materially understates the work that has landed since (the entire
   pipeline-unification sequence + remote→remote delegation Phases 1+2),
   and overstates one item: the **adaptive predictor is dead code in
   production paths** (see "Cross-cutting findings → Predictor predictions
   never consumed").
2. **Daemon counters have no exposure mechanism** despite multiple plan
   docs implying a future TUI/GUI consumer. `TUI_DESIGN.md`'s
   `Subscribe`/`GetState` RPCs are not implemented (and not in the proto).
3. The remote→remote benchmark capture is the only remaining checked-in
   delegation Phase 3 item. Several benchmark items in `TODO.md` Phase 3
   ("Capture remote benchmark runs", "Benchmark TCP data plane targeting
   10+ Gbps") are still open and require hardware.
4. F15 (structured logging) is the single explicitly-deferred baseline
   review finding. F14 closed `30b95a2` on 2026-05-04. The other 13/15
   findings closed prior.

---

## docs/plan/README.md

The plan-dir entry point. No new features, just a doc index. **State:
SHIPPING** (the directory it indexes is current; "Last Updated 2026-04-07"
is now stale and predates pipeline unification + delegation work).

---

## docs/plan/PROJECT_STATE_ASSESSMENT.md

This document *claims* feature-complete; the audit verifies (or
contradicts) each line item.

| Feature claim | State | Evidence | Notes |
|---|---|---|---|
| Phase 0 Foundation done | SHIPPING | `crates/blit-core/src/{checksum,enumeration,mirror_planner,buffer,zero_copy}.rs` all present | |
| Phase 1 gRPC scaffolding done | SHIPPING | `proto/blit.proto:1-310`; `crates/blit-core/src/generated/` build via `build.rs`. | |
| Phase 2 Local Ops done | SHIPPING | `orchestrator/orchestrator.rs:60+` (`execute_local_mirror_async`); CopyFileEx in `copy/windows.rs`; change journals in `change_journal/`. | F9 (sync wrapper split) closed 2026-05-02. |
| Phase 2.5 Validation GO | SHIPPING (claim) | `WORKFLOW_PHASE_2.5.md:62-91` records benchmarks for 512 MiB, 2 GiB, 100k smallfiles, mixed, incremental on Linux/macOS/Windows. | The 4 GiB ReFS run is documented as 35% slower than robocopy — see ReFS notes. |
| Phase 3 Remote Ops done | PARTIAL | Hybrid TCP done (`remote/transfer/data_plane.rs`), gRPC fallback (`remote/pull.rs`, `service/pull.rs`), all admin RPCs (`Find`, `DiskUsage`, `FilesystemStats`, `CompletePath`, `Purge`, `ListModules` — `proto/blit.proto:24-36`). Remote→remote delegation Phases 1+2 shipped (`service/delegated_pull.rs`, `transfers/remote_remote_direct.rs`); CLI relay primitive retained as `--relay-via-cli`. | Phase 3 *still has open TODO items* for "Capture remote benchmark runs (TCP vs forced gRPC fallback)" (`TODO.md:228`) and "Benchmark remote fallback + data-plane streaming…" (`TODO.md:190`). |
| Phase 4 Production done | PARTIAL | CI workflow at `.github/workflows/ci.yml`. Release scripts: `scripts/build-release.sh`, `scripts/windows/build-release.ps1`. Man pages: `docs/cli/blit.1.md`, `blit-daemon.1.md`, `blit-utils.1.md`. CHANGELOG.md present. | F15 (structured logging) explicitly deferred. AI telemetry implementation deferred. |
| Phase 3.5 RDMA Deferred | DEFERRED | Only proto reservation: `proto/blit.proto:64` `reserved 5 to 10; // RDMA fields`. No transport abstraction. No code under `crates/blit-core/src/remote/transfer/` mentions RDMA. | Confirmed deferred. |
| Local copy/mirror/move with async orchestrator | SHIPPING | `orchestrator/orchestrator.rs`; CLI verbs in `cli.rs:91+`. | |
| Remote push/pull via hybrid TCP + gRPC control | SHIPPING | `remote/push/client/`, `remote/pull.rs`, `service/push/`, `service/pull.rs`. `remote_parity.rs` integration test. | |
| Remote-to-remote transfers (`server1:/mod/ → server2:/mod/`) | SHIPPING (delegation default) + PARTIAL (legacy relay) | `transfers/mod.rs:398-414, 503-516` dispatches to `run_remote_to_remote_direct` by default; `--relay-via-cli` keeps `RemoteTransferSource` path. `remote_remote.rs` integration test covers both. | Live benchmarks not yet captured (`docs/perf/remote_remote_benchmarks.md:34-41` is template only). |
| Block-level resumable transfers with Blake3 | SHIPPING | `copy/file_copy/resume.rs` (local), `remote/pull.rs:530+` (remote block-hash exchange), `remote/transfer/data_plane.rs` (block records over TCP). `remote_resume.rs` test. | |
| Multiplexed TCP data plane with auto-tuned stream counts | SHIPPING | `auto_tune/mod.rs:108` `derive_local_plan_tuning`; `MultiStreamSender` in `remote/push/client/mod.rs`; up to 16 streams negotiated for multi-GiB manifests. | But `auto_tune::TuningParams.max_streams` is hard-coded to 8 at `auto_tune/mod.rs:72` — bandwidth bracketing and RTT not used (cf. `POST_REVIEW_FIXES.md:243-258` Round 3.1). |
| Small file batching via tar shards | SHIPPING | `tar_stream.rs`, `remote/transfer/tar_safety.rs`, `transfer_plan.rs:138+` shard target sizing. | |
| Async read-ahead with double-buffered I/O | SHIPPING | `remote/transfer/data_plane.rs:242` `send_file_double_buffered`. `BufferPool` at `crates/blit-core/src/buffer.rs`. | Used in `service/pull.rs:656`, `service/pull_sync.rs:563`. |
| Streaming manifest exchange | SHIPPING | `remote/push/client/helpers.rs:19` `drain_pending_headers`; `service/push/control.rs:46` `FileListBatcher`. | |
| Adaptive performance predictor with online gradient descent | PARTIAL — **dead code on the consumer side** | `crates/blit-core/src/perf_predictor.rs:50,166,178` defines `predict_ms` and `predict_planner_ms`. Only call sites: the predictor's own tests. Production write path: `orchestrator/history.rs:68` calls `predictor.observe(record)` and `save()`. **No production code reads predictor outputs.** | The orchestrator's only adaptive consumer is `derive_local_plan_tuning` at `orchestrator.rs:349` — that bypasses the predictor entirely and reads raw `PerformanceRecord`s. Headline cross-cutting issue. |
| Performance history schema versioning v0/v1 | SHIPPING | `perf_history.rs:380` test asserts the v1 schema; `migrate_record()` and `migrate_history_file()` per DEVLOG 2026-03-06 entry. | |
| macOS clonefile, FSEvents journal, statfs FS detection | SHIPPING | `copy/file_copy/clone.rs`; `change_journal/snapshot.rs::macos` (now `objc2_core_services::FSEventsGetCurrentEventId` post-F14, `30b95a2`); `fs_capability/probe.rs` macOS branch. | F14 closed 2026-05-04. |
| Linux copy_file_range, metadata snapshot journal, statfs | SHIPPING | `copy/file_copy/`; `change_journal/snapshot.rs::linux`; `fs_capability/probe.rs:58-92`. | |
| Windows CopyFileExW, USN Change Journal, ReFS block clone | SHIPPING (block clone gated on privilege) | `copy/windows.rs`; `change_journal/snapshot.rs::windows`; `fs_capability/windows.rs:54+` `probe_block_clone_support`. `win_fs.rs:134+` documents `SeManageVolumePrivilege` requirement. | ReFS benchmark in `WORKFLOW_PHASE_2.5.md:72` shows blit 0.59s vs robocopy 0.165s — block clone path operates but ~3.5× slower than robocopy. Privilege investigation is `TODO.md:259` open item. |
| Filesystem capability probing for 12+ FS types | SHIPPING | `fs_capability/probe.rs:35-100+` covers APFS, HFS+, btrfs, XFS, ext4, ZFS, tmpfs, NFS, CIFS, NTFS, ReFS. Cached per device ID. | |
| `blit` verbs: copy, mirror, move, scan, list, du, df, rm, find, diagnostics | SHIPPING | `crates/blit-cli/src/cli.rs`; per-verb files (`check.rs`, `find.rs`, `du.rs`, `df.rs`, `rm.rs`, `ls.rs`, etc.). | |
| `blit-daemon`: TOML config, modules, mDNS, hybrid transport | SHIPPING | `crates/blit-daemon/src/runtime.rs:8,127` parses `[delegation]`, `[[module]]`, `[daemon]`. | |
| `blit-utils`: scan, list-modules, ls, find, du, df, rm, completions, profile | SHIPPING | `crates/blit-cli/src/{scan,list_modules,ls,find,du,df,rm,profile}.rs`; `blit_utils.rs` test covers all 9 (DEVLOG 2026-04-07). | |
| Integration test surface | SHIPPING | `crates/blit-cli/tests/`: 16 files including admin_verbs, blit_utils, remote_parity, remote_resume, remote_remote, remote_pull_mirror, remote_tcp_fallback, remote_checksum_negotiation, remote_transfer_edges, remote_push_single_file, remote_pull_subpath, f2_chroot_containment, single_file_copy, remote_move, diagnostics_dump, remote_regression. `crates/blit-core/tests/pull_sync_with_spec_wire.rs`. | |
| GitHub Actions CI tri-platform | SHIPPING | `.github/workflows/ci.yml`. | |
| Man pages (3) | SHIPPING | `docs/cli/blit.1.md`, `blit-daemon.1.md`, `blit-utils.1.md`. | |
| Release scripts (Unix + Windows) | SHIPPING | `scripts/build-release.sh`, `scripts/windows/build-release.ps1`. | |
| ReFS block clone privilege investigation | UNVERIFIED / open | `TODO.md:259` open: "Investigate SeManageVolumePrivilege requirement". `win_fs.rs:134+` has the privilege-enable scaffold but `WORKFLOW_PHASE_2.5.md:72` records the fallback path is ~3.5× slower than robocopy. | Confirmed open. Listed in PROJECT_STATE_ASSESSMENT.md "Post-Release". |
| AI telemetry analysis | DEFERRED | `docs/plan/AI_TELEMETRY_ANALYSIS.md` is the scoping doc. No `perf_analysis` module in `blit-core`. No `blit diagnostics analyze` subcommand. `blit diagnostics dump` is a different feature (snapshot for bug reports). | |
| Full structured logging migration | DEFERRED (F15) | `TODO.md:99-101` explicitly deferred. Confirmed `eprintln!` and `println!` calls remain across orchestrator/daemon. | |
| Benchmark TCP data plane throughput targeting 10+ Gbps | NOT-STARTED | `TODO.md:210` unchecked. `BENCHMARK_10GBE_PLAN.md` is the plan. `scripts/bench_10gbe.sh` exists but no captured results. | Hardware-bound. |
| Benchmark remote fallback + data-plane streaming | NOT-STARTED | `TODO.md:190` unchecked. | Hardware-bound. |
| Capture remote benchmark runs (TCP vs gRPC fallback) | NOT-STARTED | `TODO.md:228` unchecked. | Hardware-bound. |

---

## docs/plan/greenfield_plan_v6.md

The active architectural plan. v4 and v5 sections at the top are
**SUPERSEDED** by v6 (`Status: Active (supersedes v5)` at line 301).

**v6 deliverables checklist (lines 405-413):**

| Feature | State | Evidence |
|---|---|---|
| CLI: copy/mirror/move/scan/list/diagnostics with canonical remote syntax | SHIPPING | `cli.rs`; URL parsing in `crates/blit-core/src/remote/endpoint.rs`. Subcommand verbs match. |
| `blit-core::remote::endpoint` parses `server:/module/...`, `server://...`, discovery, rejects ambiguous | SHIPPING | `remote/endpoint.rs` exists; widely used by tests. (No exhaustive review of edge cases performed in this audit.) |
| Daemon loads modules/root from config, mDNS opt-out, read-only enforcement, F2 always-on containment, "no exports" handling | SHIPPING | `runtime.rs:127-237` parses `[[module]]` + `[daemon]` + `[delegation]`; `service/util.rs::resolve_module` applies F2 canonical containment. `--root` and implicit-cwd handling at `runtime.rs:204+`. |
| RPC surface: list/find/du/df/rm | SHIPPING | `proto/blit.proto:24-36` declares them all. Daemon implementations under `service/admin.rs` + `service/core.rs`. |
| `blit-utils` 9-verb surface | SHIPPING | All 9 implemented; integration tests at `crates/blit-cli/tests/blit_utils.rs`. |
| Test suite covers transfer permutations, admin workflows, daemon startup | SHIPPING (transfers + admin) / PARTIAL (daemon startup) | Transfer + admin coverage thorough. "Daemon startup combinations (modules present, modules absent with --root, mDNS toggles)" — no dedicated startup test file found; behaviors exercised indirectly via test fixtures. |
| Documentation reflects v2 only | SHIPPING | Man pages updated 2026-04-07. |
| Benchmarks include remote scenarios over TCP and gRPC fallback | NOT-STARTED | Remote benchmark capture remains a TODO item. |

**v6 phase notes:**

- v6 §3 phases 0-4: SHIPPING per Phase tables above.
- v6 §3 Phase 3.5 RDMA: DEFERRED (post-release, no proto/code work).
- v6 §5 Open Questions: ✅ "Config search order" resolved (CLI flag → config; no env vars per `MASTER_WORKFLOW.md:75`); Windows service nuances and "future admin verbs" questions still open in plan but not blocking 0.1.0.

---

## docs/plan/PIPELINE_UNIFICATION.md

5-step priority sequence. Drafted 2026-05-01.

| Step | Feature | State | Evidence |
|---|---|---|---|
| 1 | F1: shared `safe_join` helper, applied at every receive-sink site, migrate existing sanitizers, adversarial tests | SHIPPING | `crates/blit-core/src/path_safety.rs` (commit `cc77074`). 16 unit tests + 8 sink-level integration tests. Migrated `pull.rs::sanitize_relative_path` and `service/util.rs` validators. DEVLOG 2026-05-02 01:15Z. |
| 2 | `TransferOperationSpec`, `FilterSpec`, `ComparisonMode`, `MirrorMode`, `ResumeSettings`, `PeerCapabilities` proto messages | SHIPPING | `proto/blit.proto` includes all six (commit `21ad75a`). Re-exported via `blit-core::remote::transfer::operation_spec`. DEVLOG 2026-05-02 01:45Z. |
| 3 | `DiffPlanner` extraction; orchestrator + push + pull all route through it | SHIPPING | `crates/blit-core/src/remote/transfer/diff_planner.rs`. Step 3a (`8a15e5a`), 3b (`b229e44`), 4 (`e503938`). |
| 4 | `pull_sync.rs` refactored to unified pipeline; `TransferOperationSpec` on the wire; old `PullSyncHeader` deleted | SHIPPING | `service/pull_sync.rs` consumes spec via `NormalizedTransferOperation::from_spec`. `ClientPullMessage::Spec` replaces `Header` (no `PullSyncHeader` left in proto). |
| 5 | Re-evaluate remote→remote (CLI relay → daemon-A→daemon-B direct) | SHIPPING (Phases 1+2 of REMOTE_REMOTE_DELEGATION_PLAN), benchmark NOT-STARTED | Phase 1: `15991ed`. Phase 2: `0c00b4b`. Live bench: open. |

All five steps shipped. The plan is effectively closed pending live
remote→remote benchmarks.

---

## docs/plan/UNIFIED_RECEIVE_PIPELINE.md

9-phase plan to make the receive side mirror the unified send-side
pipeline.

| Phase | Feature | State | Evidence |
|---|---|---|---|
| 1 | Streaming whole-file receive | CORRECTED 2026-07-22 | No `PreparedPayload::FileStream` variant exists. `execute_receive_pipeline` hands a size-limited borrowed reader directly to `TransferSink::write_file_stream`. |
| 2 | Sink support for streamed files | SHIPPING | `FsTransferSink::write_file_stream` consumes the borrowed reader; non-stream payloads still use `write_payload`. |
| 3 | TCP file-record decode | SHIPPING | `execute_receive_pipeline` decodes `DATA_PLANE_RECORD_FILE` and calls `write_file_stream` directly; it does not emit a payload variant. |
| 4 | `execute_receive_pipeline` executor | SHIPPING | `remote/transfer/pipeline.rs:201` `pub async fn execute_receive_pipeline`. |
| 5 | Daemon push-receive call site swap | SHIPPING | `crates/blit-daemon/src/service/push/data_plane.rs:152-170` calls `execute_receive_pipeline`. |
| 6 | Daemon pull-receive call site swap (client-side application loop in `pull.rs`) | SHIPPING | `crates/blit-core/src/remote/pull.rs:1552-1573` `execute_receive_pipeline`. |
| 7 | Resume block records — payload variants | SHIPPING | `payload.rs` has `FileBlock`/`FileBlockComplete` arms (consistent with the recommended Option A). |
| 8 | Tests + cleanup (delete dead handle_*record functions) | SHIPPING (per DEVLOG 2026-04-14) | Devlog: "Deleted ~125 lines of legacy handlers in `service/pull.rs`". |
| 9 | Docs (`ARCHITECTURE.md` diagram, DEVLOG, CHANGELOG) | SHIPPING | `docs/ARCHITECTURE.md` updated; DEVLOG 2026-04-14 entry present. |

Plan complete. Performance gate ("push large TCP ≥ 90% of pull large
TCP") relies on remote-bench capture, still NOT-STARTED.

---

## docs/plan/REMOTE_TRANSFER_PARITY.md

Push/pull parity refactor.

| Feature | State | Evidence |
|---|---|---|
| Shared `remote::transfer::{payload, progress, data_plane}` modules | SHIPPING | All three exist as files. |
| `PullChunk` extended with `DataTransferNegotiation` + `PullSummary` | SHIPPING | `proto/blit.proto` PullChunk variants regenerated 2025-11-10. |
| Daemon pull pipeline rebuilt to use TCP data plane with `--force-grpc` fallback | SHIPPING | `crates/blit-daemon/src/service/pull.rs:656` instantiates `BufferPool` and TCP listener; gRPC fallback retained. |
| `RemotePullClient` connects to negotiated TCP, applies file/tar payloads, records summaries | SHIPPING | `crates/blit-core/src/remote/pull.rs:400-480` (negotiation + stream count); `pull.rs:1573` runs `execute_receive_pipeline`. |
| Auto-tune feeds chunk sizes/prefetch for both push and pull | SHIPPING (chunk + prefetch) / PARTIAL (no scheduler hook) | `auto_tune::TuningParams` includes `chunk_bytes`/`tcp_buffer_size`/`prefetch_count`/`stream_count`. Plan doc: "scheduler-specific hooks are still future work" (line 90). |
| Multiplexed TCP data-plane streams driven by auto-tuned worker counts | SHIPPING | Plan doc: up to 16 streams. Confirmed in `MultiStreamSender`. |
| Balanced TCP scheduling (32–512 MiB batches per stream) | SHIPPING | Plan doc line 19; `MultiStreamSender` in `remote/push/client/mod.rs`. |
| Integration/perf tests proving push/pull parity | SHIPPING (functional) / NOT-STARTED (perf) | `crates/blit-cli/tests/remote_parity.rs` covers TCP + forced gRPC for both directions. Live perf parity validation = the open benchmark items. |
| Remove "diagnostic-only" pull note | SHIPPING | DEVLOG entries refer to pull as a first-class path. |

---

## docs/plan/REMOTE_REMOTE_DELEGATION_PLAN.md

Most recent plan. v4 spec, ~1076 lines.

### Phase 1 — Wire protocol, gate, daemon handler

| Item | State | Evidence |
|---|---|---|
| Proto: `DelegatedPull`, `DelegatedPullRequest`, `RemoteSourceLocator`, `DelegatedPullProgress`, `DelegatedPullStarted`, `DelegatedPullSummary`, `DelegatedPullError` | SHIPPING | `proto/blit.proto:38-49` rpc; further messages declared. |
| Delegation gate config: `[delegation]` `allow_delegated_pull` (default `false`), `allowed_source_hosts` parsed at config load (Hostname/CIDR/IP) | SHIPPING | `crates/blit-daemon/src/runtime.rs:127-238`; `crates/blit-daemon/src/delegation_gate.rs` (`DelegationConfig`, `parse_allow_entry`). |
| Per-module narrowing override | SHIPPING | `runtime.rs:35` rustdoc explicitly notes "can only narrow…never widen it". |
| `validate_source` with DNS-rebinding mitigation (resolve once, bind to IP) | SHIPPING (per plan + handler) | `service/delegated_pull.rs` calls into delegation_gate validation. (Handler implementation correctness reviewed in followup rounds R30–R36 — closed.) |
| `pull_sync_with_spec` refactor in core (extracted from `pull_sync`) | SHIPPING | `crates/blit-core/src/remote/pull.rs` exposes both `pull_sync` (wrapper) and `pull_sync_with_spec`. Wire-equivalence test at `crates/blit-core/tests/pull_sync_with_spec_wire.rs`. |
| Endpoint-isolation (`pull_sync_with_spec` MUST NOT read `endpoint.path`) | SHIPPING | Test in `pull_sync_with_spec_wire.rs:281+` verifies hand-built spec wins over endpoint. |
| Mandatory `client_capabilities` override on dst | SHIPPING (per plan + R32-F1/R34-F1 closure) | `service/delegated_pull.rs` rewrites the field; tests under `crates/blit-daemon`. |
| F2 canonical containment on `dst_destination_path` | SHIPPING | F2 always-on after 2026-05-02. |
| Metrics RAII via `enter_transfer()` | SHIPPING | `service/delegated_pull.rs:227` `metrics.inc_pull()`; `service/core.rs` uses `ActiveGuard`. |
| Cancellation on dropped gRPC return stream | SHIPPING | R32-F2 + R34-F2 closures (`AbortOnDrop`). |
| Unit/integration test matrix (gate, allowlist, IP-form rule, DNS rebinding sim, capability override, endpoint isolation, wire equivalence) | SHIPPING | Tests across `crates/blit-daemon/`, `crates/blit-core/tests/`, `crates/blit-cli/tests/remote_remote.rs`. |

### Phase 2 — CLI dispatch and integration tests

| Item | State | Evidence |
|---|---|---|
| `run_remote_to_remote_direct` in `crates/blit-cli/src/transfers/remote_remote_direct.rs` | SHIPPING | File exists; routes through `DelegatedPull`. |
| Dispatch in `transfers/mod.rs` updated; no silent fallback | SHIPPING | `transfers/mod.rs:398-414, 503-516` matches plan §4.5 explicit-only fallback predicate. |
| `--relay-via-cli` flag | SHIPPING | `cli.rs:232`; `transfers/mod.rs:403,511`. |
| Byte-path-isolation integration test | SHIPPING | `crates/blit-cli/tests/remote_remote.rs:204` "direct path must not construct RemoteTransferSource". `BLIT_TEST_COUNTER_FILE` env-gated counter via `crates/blit-core/src/remote/instrumentation.rs`. |
| `remote_remote_no_silent_fallback` coverage (stale daemon `Unimplemented`, gate-rejected, src refusal) | SHIPPING (consolidated in `remote_remote.rs`) | `remote_remote.rs:302` checks `does not implement DelegatedPull`. R37-F1 closure introduced typed Negotiation phase. |
| `remote_remote_explicit_relay` counterpart | SHIPPING | Same test file; relay path counter assertion. |

### Phase 3 — Cleanup and benchmarking

| Item | State | Evidence |
|---|---|---|
| `RemoteTransferSource` documented as legacy `--relay-via-cli` primitive | SHIPPING | `crates/blit-core/src/remote/transfer/source.rs:216` "remains for the explicit `--relay-via-cli` escape". |
| Update `docs/DAEMON_CONFIG.md` (Trust Model + Path containment) | SHIPPING | DEVLOG 2026-05-02 records the additions. |
| Update `docs/cli/blit.1.md` and man pages | SHIPPING | DEVLOG 2026-05-03. |
| Update TODO.md (move out of Deferred) | SHIPPING | `TODO.md:109-115` records resolution. |
| Benchmark script `scripts/bench_remote_remote.sh` | SHIPPING | File exists; `bash -n` clean per DEVLOG. |
| `docs/perf/remote_remote_benchmarks.md` results template | SHIPPING (template) | File exists. |
| **Run benchmark and fill in `docs/perf/remote_remote_benchmarks.md`** | NOT-STARTED | Plan §5 line 921 unchecked. `docs/perf/remote_remote_benchmarks.md:34-41` shows "TBD". |

### Phase 4 — Future-proofing

| Item | State | Evidence |
|---|---|---|
| `RemoteSourceLocator.delegated_credential` honored end-to-end | DEFERRED | Plan: "Tracked as separate task; this plan does not block on it." Field exists in proto, ignored in 0.1.0. |
| `--detach` mode | DEFERRED (out of scope) | Plan §9. |

---

## docs/plan/MASTER_WORKFLOW.md

Phase coordination doc.

- Phase 0/1/2/2.5: SHIPPING (see PROJECT_STATE_ASSESSMENT entries).
- Phase 3 gate items: SHIPPING for the verb surface; the "remote benchmarks" gate criterion is NOT-STARTED.
- Phase 4 gate items: SHIPPING for packages, docs, integration test suite, release notes; the "benchmark snapshots in release notes" line item awaits hardware.
- Phase 3.5 (RDMA): DEFERRED.
- Decision log entries (transport model, `eyre`, Tokio, progress UX, telemetry, no env vars): SHIPPING — verified in code.

No new feature claims here beyond the phase tables.

---

## docs/plan/WORKFLOW_PHASE_2.md, WORKFLOW_PHASE_2.5.md, WORKFLOW_PHASE_3.md, WORKFLOW_PHASE_4.md, WORKFLOW_V2.md

Historical phase workflows. Items called out:

| Item | State | Evidence |
|---|---|---|
| Streaming planner / heartbeat / 10s stall detector (Phase 2) | CORRECTED 2026-07-22 — NOT SHIPPED AS SPECIFIED | The pipeline streamed work, but the promised `PlannerEvent` heartbeat/state machine and local-planner 10s stall detector never existed. Current transfer stall guards are not equivalent proof. |
| Local performance history (capped JSONL) | SHIPPING | `perf_history.rs`. |
| EMA-based predictor + integration | PARTIAL | Trained, persisted, but **predictions never read** (see cross-cutting). |
| `blit diagnostics perf` CLI | SHIPPING | `crates/blit-cli/src/diagnostics.rs:8-118`. |
| CLI/config toggle for telemetry (no env-var) | SHIPPING | `diagnostics.rs:18-25` `--enable/--disable`. Decision log confirms env-vars not used for config. |
| Phase 2 unit/integration tests passing | SHIPPING | Test suite green per DEVLOG. |
| Phase 2.5 benchmark execution complete | SHIPPING (claim in workflow doc) | Benchmark logs cited per workload — not re-verified in this audit. |
| Phase 3.3.1 hybrid transport completion | SHIPPING | See REMOTE_TRANSFER_PARITY entries. |
| Phase 3.3.6 Windows USN incremental fast-path | SHIPPING | DEVLOG 2025-10-25: 28 ms zero-change mirror. |
| Phase 3.3.7 macOS FSEvents incremental fast-path | PARTIAL — verification pending | `change_journal/snapshot.rs::macos` post-F14 captures correctly; "macOS verification run pending" per WORKFLOW_PHASE_3.md:49. |
| Phase 3.3.8 Linux metadata snapshot fast-path | SHIPPING | DEVLOG 2025-10-25: 3 ms zero-change mirror. |
| Phase 3.3.9 Streaming manifest+need-list | SHIPPING | `FileListBatcher` daemon-side; `drain_pending_headers` client-side. |
| Phase 3.4.1-3.4.5 admin RPCs + blit-utils + safety prompts + completions + profile | SHIPPING | All implemented. F2 containment + per-call read-only checks in admin handlers (DEVLOG 2026-05-02). |
| Phase 4.1 packaging targets (tarball/Debian/RPM/Homebrew/Windows installer) | PARTIAL | Tarball scripts shipped (Unix + Windows); Debian/RPM/Homebrew/installer NOT-STARTED. (DEFERRED to post-release per PROJECT_STATE_ASSESSMENT.) |
| Phase 4.5.3 Final QA on all platforms | NOT-STARTED | No evidence of a documented QA pass against installer artifacts (because installer isn't built). |
| Phase 4.6 Security/reliability hardening | PARTIAL | Trust-model docs added; F2 containment shipped; metrics RAII; receive-side safety F1+R5/R6/R7 closures. No formal "security review" report. |
| Phase 4.8 Filesystem capability probes | SHIPPING | `fs_capability/probe.rs` + integration into diagnostics dump. |
| Phase 4.9 AI telemetry exploration | DEFERRED | Scoping doc only. |

---

## docs/plan/POST_REVIEW_FIXES.md

Three rounds of follow-up items.

### Round 1 (cheap correctness wins)

| Item | State | Evidence |
|---|---|---|
| §1.1 Surface flush failures + tracing on metadata-apply best-effort | NOT-STARTED | `let _ = file.flush().await;` and silent metadata-set patterns are still in `sink.rs` per the plan doc; no DEVLOG entry indicates the fix landed. |
| §1.1b Real pipeline error from `MultiStreamSender::queue` | NOT-STARTED | No DEVLOG entry for `pipeline_handle.take()`/await for real-error retrieval. (Last logged crash described in plan: 2026-05-01 `mirror ~/dev …` produced "data plane pipeline closed unexpectedly" with no underlying cause.) |
| §1.2 Delete `TarShardExecutor` in daemon push fallback (rayon-parallel exists in `FsTransferSink::write_tar_shard_payload`) | UNVERIFIED | No DEVLOG closure entry; would require confirming `service/push/data_plane.rs` no longer instantiates `TarShardExecutor`. (Did not confirm in this audit.) |
| §1.3 Update `WHITEPAPER.md` for `BLOCK_COMPLETE` wire change | UNVERIFIED | Did not load `WHITEPAPER.md` to verify the §3 update. |

### Round 2

| Item | State | Evidence |
|---|---|---|
| §2.1 `change_journal/` test coverage | NOT-STARTED | Plan doc states `grep -r "fn test_" crates/blit-core/src/change_journal/ \| wc -l` → 0. No DEVLOG entry adds journal tests. |
| §2.2 Drain task `tokio::Mutex` anti-pattern (held across awaits) | NOT-STARTED | Plan doc cites `crates/blit-daemon/src/service/push/data_plane.rs:147`. No closure noted in DEVLOG. |
| §2.3 Pre-allocation guard in `read_tar_shard` | NOT-STARTED (deferred per plan doc) | Plan: "Probably defer." |

### Round 3 (architectural)

| Item | State | Evidence |
|---|---|---|
| §3.1 Adaptive tuning expansion (move `transfer_plan.rs` static thresholds into `TuningParams`) | NOT-STARTED | `auto_tune::TuningParams` still has hard-coded `max_streams = 8`, no RTT/FS bucketing. |
| §3.2 `change_journal` consulted for remote transfers | NOT-STARTED (research) | No journal-snapshot RPC exists in proto. |
| §3.3 Mid-transfer parameter adaptation (BBR-style estimator) | NOT-STARTED (research) | Same — research item. |

POST_REVIEW_FIXES.md as a whole is **substantially open**. The "Round 1
cheap correctness wins" set is the highest-priority open item this audit
finds — `§1.1b` directly affects user diagnostics quality.

---

## docs/plan/AI_TELEMETRY_ANALYSIS.md

Scoping doc, explicitly marked "Implementation deferred to post-release"
in `PROJECT_STATE_ASSESSMENT.md:89`.

| Feature | State | Evidence |
|---|---|---|
| `perf_analysis` module in `blit-core` (`classify_records`, `detect_anomalies`, `generate_recommendations`) | DEFERRED | No `perf_analysis` module under `crates/blit-core/src/`. |
| `blit diagnostics analyze [--days N] [--json]` | DEFERRED | Subcommand not in `cli.rs`. (`diagnostics dump` is unrelated.) |
| `blit-utils analyze` | DEFERRED | Not implemented. |

---

## docs/plan/LOCAL_TRANSFER_HEURISTICS.md

Phase 2 implementation design.

| Feature | State | Evidence |
|---|---|---|
| §3 Streaming planner + heartbeat + 10 s stall detector | CORRECTED 2026-07-22 — NOT SHIPPED AS SPECIFIED | See corrected Phase 2 row above. |
| §4 Immediate fast-paths (≤8 files & ≤100 MB direct copy; single ≥1 GiB file dispatched immediately) | HISTORICAL / RETIRED | These existed in the old orchestrator, which was deleted at otp-11b when local transfer moved to the unified session. |
| §5 Performance history (capped JSONL, signature, profile) | SHIPPING | `perf_history.rs`. |
| §6 Adaptive predictor (linear, EMA, FS-segmented, init defaults, 1 s threshold routing) | PARTIAL — **predictions never consumed** | See cross-cutting. The ≤1 s routing decision in §6.2 is not actually implemented in the orchestrator; orchestrator only uses `derive_local_plan_tuning` (which doesn't use the predictor). |
| §7 Worker/buffer tuning (auto, debug `--workers` limiter, hidden in help) | SHIPPING | CLI accepts `--workers`; `transfer_plan.rs` derives chunk sizes from total bytes. |
| §11 USN/FSEvents/Linux journals fast-path | SHIPPING | `change_journal/`. macOS verification still pending. |
| §11 Future Work: regression suite for adaptive predictor | SHIPPING (regression tests added 2026-03-06) — but tests cover the trained predictor, not the (missing) consumption path. |
| §11 Revisit `--max-threads` flag usage; deprecate if unused | NOT-STARTED | `--workers` retained; `--max-threads` not searched in this audit. |

---

## docs/plan/BENCHMARK_10GBE_PLAN.md

The benchmark execution plan; it is operator-facing.

| Phase | State |
|---|---|
| Phase 1 Local-Only sanity | SHIPPING (script: `scripts/bench_10gbe.sh`) |
| Phase 2 Local → NFS/SMB mount | NOT-STARTED (no captured logs) |
| Phase 3 Remote push/pull, daemon on TrueNAS | NOT-STARTED |
| Phase 4 Reverse direction | NOT-STARTED |
| Phase 5 Stress test | NOT-STARTED |
| Recording results into CHANGELOG.md | NOT-STARTED — CHANGELOG benchmark section is a placeholder. |

This entire plan is hardware-bound and deferred to a benchmark run.

---

## docs/plan/BLIT_UTILS_PLAN.md

Admin utilities matrix.

| Command | State | Evidence |
|---|---|---|
| `scan` | SHIPPING | `crates/blit-cli/src/scan.rs`. |
| `list-modules` | SHIPPING | `list_modules.rs`. |
| `ls` | SHIPPING | `ls.rs` (supports `--json`). |
| `list` (alias of ls) | SHIPPING | Same dispatch path. |
| `find` | SHIPPING | `find.rs`. Streams via `Find` RPC. |
| `du` | SHIPPING | `du.rs`. |
| `df` | SHIPPING | `df.rs` (human-readable bytes added 2026-04-07). |
| `rm` | SHIPPING | `rm.rs`; confirmation prompts; `--yes` bypass. |
| `completions` | SHIPPING | `completions.rs`. |
| `profile` | SHIPPING | `profile.rs`. |
| Auth-token forwarding hook | DEFERRED | Plan §UX line "CLI should accept but ignore token for now". Not yet wired. |
| Integration tests | SHIPPING | `crates/blit-cli/tests/blit_utils.rs` 21 tests. |

---

## docs/plan/TUI_DESIGN.md

Drafted 2026-05-01. No code yet (per plan).

| Feature | State | Evidence |
|---|---|---|
| TUI scaffolding (`blit-tui` crate with `ratatui`) | NOT-STARTED | No `blit-tui` crate in `crates/`. |
| mDNS TXT enrichment (v, mods, nmods, caps fields) | NOT-STARTED | `crates/blit-core/src/mdns.rs:15-83` advertises bare `_blit._tcp.local.` only. No TXT record fields added. **This is the cheapest TUI prerequisite — flagged in plan as "Do early" but not started.** |
| `Subscribe(SubscribeRequest) → stream DaemonEvent` RPC | NOT-STARTED | Not declared in `proto/blit.proto`. Only the Blit service RPCs (Push/PullSync/List/Purge/CompletePath/ListModules/Find/DiskUsage/FilesystemStats/DelegatedPull) and BlitAuth.Authenticate stub. |
| `GetState(GetStateRequest) → DaemonState` RPC | NOT-STARTED | Same — not in proto. |
| Counters readable via `GetState` | NOT-STARTED | `TransferMetrics` increments correctly but has no exposure. `crates/blit-daemon/src/main.rs:78`: "No exposure mechanism today (no HTTP, no RPC) — these are…". `crates/blit-daemon/src/metrics.rs:14` confirms: "No exposure mechanism (no HTTP, no RPC) yet". |
| F1 Daemons / F2 Transfers / F3 Browse panes | NOT-STARTED | Pre-req of crate. |

The whole TUI design is greenfield, with mDNS TXT enrichment being the
only standalone-useful prerequisite (improves `blit scan` independent of
the TUI). Nothing is started.

---

## docs/PERFORMANCE_ROADMAP.md

The 25 GbE roadmap.

### Part 1 — Bottleneck list

These are diagnoses, not features. Verifying current state:

| Bottleneck | Status today | Evidence |
|---|---|---|
| 1.1 Single-stream data plane (no parallel dispatch) | RESOLVED | `MultiStreamSender` spawns N concurrent workers; up to 16 streams on multi-GiB manifests (TODO.md line 200-203). |
| 1.2 Small (64KB) buffer size | RESOLVED | `auto_tune::TuningParams.tcp_buffer_size`; `chunk_bytes` ranges in `data_plane.rs`. |
| 1.3 No memory pool | RESOLVED | `crates/blit-core/src/buffer.rs:160` `BufferPool` with semaphore memory budget. |
| 1.4 Synchronous read-then-write | RESOLVED | `send_file_double_buffered` at `data_plane.rs:242`. |
| 1.5 No parallel file processing | RESOLVED | Same `MultiStreamSender` parallel workers. |

### Part 2 — Recommended improvements

| Priority | Item | State | Evidence |
|---|---|---|---|
| P1 Async I/O Pipeline | SHIPPING | `BufferPool` + double-buffered I/O. |
| P2 Buffer Pool with mmap | SHIPPING (no mmap) — PARTIAL on mmap path | `BufferPool` exists; mmap variant not implemented (none found in `buffer.rs`). |
| P3 Multi-stream parallel uploads (work-stealing) | SHIPPING (round-robin batch distribution) — work-stealing PARTIAL | TODO.md line 209: "Work-stealing queue is potential future optimization." |
| P4 Chunked large file transfers | NOT-STARTED | No `chunked.rs` module under `remote/transfer/`; large files transfer as single payloads (size-aware streaming, but not chunked-with-reassembly). |
| P5 TCP tuning (16 MB buffers, TCP_CORK, BUSY_POLL) | PARTIAL | `tcp_buffer_size` from auto_tune; `set_tcp_nodelay(true)` (Nagle off). No `TCP_CORK` in `data_plane.rs`. No `SO_BUSY_POLL`. |

### Part 3 — Enterprise features

| Feature | State | Evidence |
|---|---|---|
| 3.1 Resumable transfers (`--partial-suffix`-style state file) | PARTIAL | Block-level resume implemented but per-file (no `.blit-resume` state file with `completed_files` set). |
| 3.2 Bandwidth limiting (`--bwlimit`) | NOT-STARTED | No `BandwidthLimiter` / `governor` integration. |
| 3.3 Transfer retries with backoff | NOT-STARTED | No retry harness in `crates/blit-core/src/remote/`. |
| 3.4 Server-side copy (daemon-to-daemon) — `ServerSideCopy` RPC | SUPERSEDED by `DelegatedPull` | `proto/blit.proto:48` `DelegatedPull` is the realized form. The roadmap's specific RPC name was never used. |
| 3.5 Streaming checksum verification | PARTIAL | Blake3 used; "Stream checksums during transfer (no second pass)" — not implemented; checksum is computed on completed files. |
| 3.6 Progress webhooks (`--rc`) | NOT-STARTED | `RemoteTransferProgress` exists for in-process consumers (CLI); no webhook plumbing. |

### Part 4 — Roadmap phases (Weeks 1-11+)

The roadmap's phase tables (Phase 1 P0/P1 items) overlap with what
shipped; the roadmap was authored as a wish-list, not as the canonical
plan. Items not absorbed:

- io_uring (Linux): NOT-STARTED.
- RDMA: DEFERRED.
- Compression (zstd streaming): NOT-STARTED.

---

## TODO.md (synthesized cross-doc)

The "live punch list" duplicates many items above. Items that don't
appear elsewhere or warrant explicit re-classification:

| TODO line | Item | State | Evidence |
|---|---|---|---|
| 99-101 | F15 structured logging | DEFERRED | Confirmed. |
| 190 | Benchmark remote fallback + data-plane streaming | NOT-STARTED | Hardware. |
| 210 | Benchmark TCP data plane targeting 10+ Gbps | NOT-STARTED | Hardware. |
| 228 | Capture remote benchmark runs (TCP vs gRPC fallback) | NOT-STARTED | Hardware. |
| 257 | Phase 3.5 RDMA tracking | DEFERRED | Confirmed. |
| 259 | ReFS `SeManageVolumePrivilege` investigation | UNVERIFIED / open | Confirmed. |

All other TODO items found a closure in DEVLOG entries since 2026-04-07
(see commit log).

---

## Cross-cutting findings

### 1. Predictor predictions are never consumed (HEADLINE)

The user's pre-audit observation is **confirmed**. `PerformancePredictor`
(`crates/blit-core/src/perf_predictor.rs`) is loaded at
`orchestrator.rs:98`, observed via `update_predictor()` at five sites
(`orchestrator.rs:194,239,269,299,505`), and persisted to
`perf_predictor.json`. **The `predict_ms` and `predict_planner_ms`
methods are called only in the predictor's own unit tests**, never in
the orchestrator's routing decisions:

```
$ grep -rn "predict_ms\|predict_planner_ms" --include="*.rs" \
   | grep -v "fn predict_\|fn test\|tests\|predictor\.rs"
(no output)
```

Routing in `orchestrator.rs:340-353` reads raw `PerformanceRecord`s
through `derive_local_plan_tuning` (`auto_tune/mod.rs:108`), which
computes shard targets directly from `tar_shard_*`/`raw_bundle_*` fields
of the recent records — entirely bypassing the trained predictor.

**Plan docs that overpromise this:**

- `LOCAL_TRANSFER_HEURISTICS.md:96-115` (§6 Adaptive Planning Prediction):
  "If predicted planning_ms ≤ 1000 ms: enter streaming planner immediately."
  No code consumes a predicted_ms value.
- `PROJECT_STATE_ASSESSMENT.md:40` "Adaptive performance predictor with
  online gradient descent" (training-only is technically true; consumption
  is implied but absent).
- `WORKFLOW_PHASE_2.md` 2.2.3 "Integrate predictor into orchestrator
  routing decisions" was checked off but the integration is the
  `derive_local_plan_tuning` shortcut, not the predictor's outputs.

**Recommendation for the report's reader:** Either delete the predictor
machinery (it's purely overhead today: ~500 LOC + a JSON state file +
five `observe()` call sites) or wire its `predict_planner_ms` into
`maybe_select_fast_path` / planner-vs-streaming routing decisions. The
training tests are well-designed and the gradient descent works; it's
just unused. The cheaper fix is consumption.

### 2. Daemon counters are write-only

`TransferMetrics` (`crates/blit-daemon/src/metrics.rs`) increments push,
pull, purge, errors, and tracks `active_transfers` via `ActiveGuard`.
Confirmed:

- `metrics.rs:14`: `//! No exposure mechanism (no HTTP, no RPC) yet.`
- `main.rs:78-84`: `// No exposure mechanism today (no HTTP, no RPC)`

`TUI_DESIGN.md` (lines 30, 32, 110-135) defines `GetState` and
`Subscribe` RPCs. **Neither is in `proto/blit.proto`.** No plan doc
*requires* exposure for 0.1.0 — but anyone using the counters as
"production-readiness signal" should know there's no consumer. F6
(historical metrics HTTP server) was removed (commit `1593c86`); current
counters are opt-in via `--metrics`. PROJECT_STATE_ASSESSMENT.md does
not mention this — it should.

### 3. mDNS TXT enrichment never started despite "Do early"

`TUI_DESIGN.md:53-69` flags TXT enrichment (`v`, `mods`, `nmods`, `caps`)
as small (~30 LOC), useful today, no consumer dependency. None of the
four fields appear in `crates/blit-core/src/mdns.rs`. The advertised
record is bare. `blit scan` users see only host/port/instance name.

### 4. POST_REVIEW_FIXES.md Round 1 is open

`POST_REVIEW_FIXES.md` was authored after the external code reviews and
flags real bugs in production paths. None of Round 1 (§1.1, §1.1b, §1.2,
§1.3) has a DEVLOG closure entry between 2026-05-01 and 2026-05-04.
§1.1b is operator-impacting: it explains why a real-world push failure
on 2026-05-01 surfaced "data plane pipeline closed unexpectedly" with no
underlying cause. That defect is unfixed.

### 5. Plan claims that the audit contradicts

- **`PROJECT_STATE_ASSESSMENT.md:10-13` "feature-complete as of 2026-04-07"** —
  partially correct, but understates the substantial pipeline-unification
  and remote→remote delegation work shipped between 2026-04-14 and
  2026-05-04. The doc's "Last Updated" date should advance.
- **`WORKFLOW_PHASE_2.md:40` task 2.2.3 "Integrate predictor into orchestrator
  routing decisions"** marked done — but the integration uses
  `derive_local_plan_tuning`, not predictor outputs. Material gap.
- **`README.md`** in `docs/plan/` says "Last Updated: 2026-04-07" — the
  plan dir has gained `PIPELINE_UNIFICATION.md`, `UNIFIED_RECEIVE_PIPELINE.md`,
  `REMOTE_REMOTE_DELEGATION_PLAN.md`, `TUI_DESIGN.md`, and
  `POST_REVIEW_FIXES.md` since.
- **`MASTER_WORKFLOW.md:33` Phase 3 status "✅ Complete"** in the table is
  premature — Phase 3 gate criterion "Remote benchmarks (TCP vs fallback)
  captured" is unmet (`WORKFLOW_PHASE_3.md:118`).

### 6. Mismatched status across docs (≥ 2 docs)

| Feature | Doc A says | Doc B says | Reality |
|---|---|---|---|
| Adaptive predictor | `PROJECT_STATE_ASSESSMENT.md:40` "with online gradient descent" (implies live use) | `LOCAL_TRANSFER_HEURISTICS.md:104-107` "If predicted planning_ms ≤ 1000 ms: enter streaming planner immediately." | Trained, never consumed. PARTIAL. |
| Phase 4 production hardening | `MASTER_WORKFLOW.md:33` "✅ Complete" | `WORKFLOW_PHASE_4.md:5` "In progress. Repo-level production hardening review saved at `docs/reviews/codebase_review_2026-05-01.md`; packaging, benchmark capture, and hardening follow-ups remain." | PARTIAL. The newer review doc and DEVLOG support "in progress" framing. |
| Remote→remote re-evaluation | `TODO.md:109-115` Deferred design call | `REMOTE_REMOTE_DELEGATION_PLAN.md` Phase 1+2 shipped | TODO is stale wording but contents are right (it points to the new plan). |
| Filtered-pull filter parity | `WORKFLOW_PHASE_3.md:46` "✅ in place" | `codebase_review_2026-05-01.md:F10` "currently rejects filter flags" | Code shipped (Step 4B, commit `71d3c30`); F10 closed 2026-05-02. Docs are now consistent. |

### 7. Dead code / write-only feature surface

Beyond the predictor (already covered):

- **`BlitAuth` service in `proto/blit.proto:52-54`** with `Authenticate`
  RPC. Zero implementations, zero call sites. Existence is documented
  forward-compat for `RemoteSourceLocator.delegated_credential`. Honest
  but worth flagging — a plan reader could reasonably assume auth is
  partly wired.
- **`ResumableTransfer`-style state file** referenced in
  `PERFORMANCE_ROADMAP.md:236-244`. Block-level resume exists per-file,
  but the multi-file `.blit-resume` state file isn't implemented; the
  plan implies the infra is shared.
- **TarShardExecutor (per `POST_REVIEW_FIXES.md:122-148`)** is allegedly
  used only by the gRPC fallback path now that TCP receive uses
  `FsTransferSink::write_tar_shard_payload` (rayon-parallel). Plan
  recommends deleting it. State: UNVERIFIED — needs a `grep -rn
  TarShardExecutor crates/` to confirm; not closed.

### 8. Risk-ranked next-actions before declaring 0.1.0 release-ready

1. **Decide what to do about the predictor.** Either consume it or
   delete it. Status quo carries dead state and misleads readers.
2. **Close `POST_REVIEW_FIXES.md` Round 1 §1.1b.** Real-world push
   failures currently produce uninformative error messages. Cheap fix
   (~1 hour per the plan), high operator impact.
3. **Run the live remote→remote benchmark.** Plan §3 step 6 is the only
   remaining unchecked item in the delegation plan. Without it, "remote
   bytes flow A → B directly" is a code-review claim, not a measurement.
4. **Run hardware benchmarks (TCP 10 GbE + gRPC fallback)** and fill in
   `CHANGELOG.md` benchmark section. Three TODO items, one shared
   environment requirement. Whitepaper claims need real numbers.
5. **Decide whether the daemon counters need a consumer in 0.1.0.** If
   yes, ship `GetState` (smallest of the TUI-design RPCs, no streaming
   plumbing). If no, document explicitly that `--metrics` is a debug
   knob with no scrape endpoint and remove the counter increments from
   non-debug paths to avoid the false-readiness signal.
6. **`PROJECT_STATE_ASSESSMENT.md` and `docs/plan/README.md` last-updated
   refresh.** Both predate substantial work and will mislead a new
   reader.
7. **macOS FSEvents fast-path verification run.** `WORKFLOW_PHASE_3.md:49`
   notes the snapshot capture works but the `scripts/macos/run-journal-fastpath.sh`
   verification has not been run. Fast-path correctness is one of the
   load-bearing wins.
8. **Audit `TarShardExecutor` for deletability** (POST_REVIEW_FIXES §1.2).
   Either delete it or document why it stays — both are fine outcomes;
   "neither" is the wrong outcome.
9. **mDNS TXT enrichment.** Improves `blit scan` immediately. ~30 LOC.
   Would be the smallest visible UX win in this whole report. (Lower
   risk priority but very cheap.)
10. **F15 structured logging.** Explicitly deferred but worth confirming
    the deferral survives release: noisy `eprintln!` calls in hot paths
    on production daemons are a real operational issue.

---

## Methodology

### Documents read

- `docs/plan/README.md`
- `docs/plan/PROJECT_STATE_ASSESSMENT.md`
- `docs/plan/greenfield_plan_v6.md`
- `docs/plan/PIPELINE_UNIFICATION.md`
- `docs/plan/UNIFIED_RECEIVE_PIPELINE.md`
- `docs/plan/REMOTE_TRANSFER_PARITY.md`
- `docs/plan/REMOTE_REMOTE_DELEGATION_PLAN.md` (full, including Phase 4)
- `docs/plan/MASTER_WORKFLOW.md`
- `docs/plan/WORKFLOW_PHASE_2.md`
- `docs/plan/WORKFLOW_PHASE_2.5.md`
- `docs/plan/WORKFLOW_PHASE_3.md`
- `docs/plan/WORKFLOW_PHASE_4.md`
- `docs/plan/WORKFLOW_V2.md`
- `docs/plan/POST_REVIEW_FIXES.md`
- `docs/plan/AI_TELEMETRY_ANALYSIS.md`
- `docs/plan/LOCAL_TRANSFER_HEURISTICS.md`
- `docs/plan/BENCHMARK_10GBE_PLAN.md`
- `docs/plan/BLIT_UTILS_PLAN.md`
- `docs/plan/TUI_DESIGN.md`
- `docs/PERFORMANCE_ROADMAP.md`
- `TODO.md`
- `DEVLOG.md` (first 100 lines, then targeted greps for closures)
- `docs/reviews/codebase_review_2026-05-01.md`
- `docs/reviews/followup_review_2026-05-02.md` (heading scan, plus
  Round 30/37/38/39/40 contents)

### Code searched

- `crates/blit-core/src/`, `crates/blit-cli/src/`, `crates/blit-daemon/src/`,
  `proto/blit.proto`.
- `git log --oneline -50` to locate closure commits referenced in DEVLOG.

### Greps performed (representative, not exhaustive)

- `predict_ms\|predict_planner_ms` — confirmed predictor outputs are
  unused outside the module.
- `TransferMetrics\|active_transfers\|push_operations\|metrics\.inc` —
  confirmed counter increment sites and absence of HTTP/RPC reader.
- `GetState\|Subscribe\|DaemonEvent` — confirmed neither RPC exists.
- `derive_local_plan_tuning` — confirmed it's the orchestrator's only
  adaptive consumer.
- `mdns\|_blit\._tcp\|TXT` — confirmed bare advertisement, no TXT
  enrichment.
- `DelegatedPull\|delegated_pull\|relay_via_cli\|RemoteTransferSource` —
  confirmed delegation Phase 1+2 plumbing and `--relay-via-cli` retention.
- `BlitAuth\|Authenticate` — confirmed proto stub, no implementations.
- `RDMA\|rdma\|RoCE` — confirmed proto reservation only, no transport
  abstraction.
- `tcp_buffer\|TCP_CORK\|set_tcp_cork\|SO_BUSY_POLL` — confirmed Nagle
  off, send/recv buffers tuned, but no CORK/BUSY_POLL on Linux.
- `BufferPool\|buffer_pool\|SessionBuffer` — confirmed BufferPool
  shipped without mmap.
- `execute_receive_pipeline\|DataPlaneSource` — confirmed
  UNIFIED_RECEIVE_PIPELINE shipped end to end.
- `TransferOperationSpec\|spec_version` — confirmed spec proto and
  normalizer wired.
- `pull_sync_with_spec\b` — confirmed the extraction and the
  endpoint-isolation test.

### Items I did not verify with code-level checks

- `WHITEPAPER.md` `BLOCK_COMPLETE` text update (POST_REVIEW_FIXES §1.3)
  — recorded as UNVERIFIED.
- `TarShardExecutor` deletion status (POST_REVIEW_FIXES §1.2) — recorded
  as UNVERIFIED.
- The exhaustive `change_journal/` test count (POST_REVIEW_FIXES §2.1) —
  taken from the plan doc's grep result; not re-run here.
- Remote endpoint parser exhaustive edge cases (greenfield_plan_v6.md
  deliverable) — confirmed file exists and is widely used, no
  case-by-case URL parser audit.
- The 21 `blit_utils.rs` integration tests — file existence confirmed,
  per-test correctness not re-run.
- The 16 admin-verb integration tests cited in TODO line 226 — file
  existence confirmed (`admin_verbs.rs`, etc.); per-test correctness
  not re-run.
- Phase 2.5 benchmark numbers — accepted as DEVLOG-recorded, not re-run.
- macOS FSEvents fast-path *behavior* (only the build/binding closure
  via F14 was verified; the runtime correctness on a real macOS host is
  what `scripts/macos/run-journal-fastpath.sh` would prove).

### Definitions

- **SHIPPING**: implementation in `crates/`, called by production code,
  has at least one test (or an integration test exercises the path).
- **PARTIAL**: implementation present but feature is gated, dead, or
  documented as missing-a-piece.
- **DEFERRED**: plan or `PROJECT_STATE_ASSESSMENT.md` explicitly defers,
  and the deferral has not been overruled by a later commit.
- **SUPERSEDED**: an older plan reshaped by a later one (no implementation
  follows the older plan; the later plan's implementation took its place).
- **NOT-STARTED**: no implementation found and no deferral marker.
- **UNVERIFIED**: I did not run the verification check that would
  classify it (typically because doing so requires running tests, opening
  a non-listed file, or a hardware run).
