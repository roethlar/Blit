# Drift Findings: Architecture, whitepaper, daemon config, proto
**Generated**: 2026-06-04
**Claims audited**: ~180 across `docs/ARCHITECTURE.md`, `docs/WHITEPAPER.md`, `docs/DAEMON_CONFIG.md`, `proto/blit.proto`
**Findings**: 18 (H: 2 / M: 9 / L: 7)

Cross-reference: code inventories `code-bridge-proto.md`, `code-daemon.md`, `code-cli.md`, `code-core-transfer.md`, `code-core-io.md`, `code-core-misc.md`, `code-core-orch.md`, `code-tui-*.md`, `code-tests-scripts.md`.

The plan inventory already self-flags four contradictions inside the cluster (TLS+ACL non-goal vs planned, auth tokens vs no-auth, gRPC TLS not enforced, PullSync leading message). Those are surfaced here as drift against code rather than internal contradictions because the docs in question are still live and reachable.

---

## High severity

### h-1-architecture-security-section-stale-claims-removed-features
**Plan says**: `docs/ARCHITECTURE.md:446-453` "Security Considerations / Current State: Transport: gRPC with optional TLS (not enforced); Authentication: Token-based (placeholder in proto); Authorization: Module-level read/write permissions" and "Planned Enhancements: TLS certificate validation; Per-module access control lists; Audit logging".
**Code does**: `BlitAuth` service stub and all auth message types were removed 2026-05-13. `proto/blit.proto:110-116` carries an explicit tombstone comment; `proto/blit.proto:409` reserves `AuthRequest`/`AuthResponse`; `proto/blit.proto:652-657` reserves the removed `delegated_credential` field. `DAEMON_CONFIG.md:267-274` declares "There is no daemon authentication … not on the roadmap." There is no TLS termination, no token auth, no ACL framework in `crates/blit-daemon/src/`.
**Evidence**:
- `docs/ARCHITECTURE.md:444-454`
- `docs/DAEMON_CONFIG.md:259-274` (Auth posture)
- `proto/blit.proto:110-116, 409, 652-657`
- `crates/blit-daemon/src/service/core.rs` — no auth pre-handler; only `delegation_gate.rs` policy
**Notes**: This is high severity because the architecture document is the front door for new readers and it actively misleads on the security posture — readers will believe there is a "Planned" path to TLS/ACL/audit when DAEMON_CONFIG.md explicitly disowns those as out-of-scope. Plan inventory already flags this as `contradiction-tls-and-acl-non-goal-vs-planned`, `contradiction-auth-tokens-vs-no-auth`, `contradiction-grpc-tls-not-enforced`; collapsed here into one high-severity finding because they are all the same stale-doc symptom and trace to the same Security Considerations subsection. Fix: rewrite §"Security Considerations" + §"Planned Enhancements" to match the current model (operator network controls + per-transfer data-plane tokens; no plan for built-in TLS/ACL/audit).

### h-2-whitepaper-pull-sync-flow-narrative-references-removed-message
**Plan says**: `docs/WHITEPAPER.md:336-338` "Client sends `PullSyncHeader` then a `LocalFile` per local entry, then `ManifestDone`."
**Code does**: `PullSyncHeader` was removed entirely. `proto/blit.proto:269-275` says "The leading message is now `TransferOperationSpec` … The previous `PullSyncHeader` bool-soup shape was removed entirely because we don't carry backward compatibility into the next release." `crates/blit-core/src/remote/pull.rs:680-697` opens the bidi stream FIRST, then sends `TransferOperationSpec` (not PullSyncHeader). Wire test `crates/blit-core/tests/pull_sync_with_spec_wire.rs:251-310` pins the byte-for-byte spec leading message.
**Evidence**:
- `docs/WHITEPAPER.md:332-345`
- `proto/blit.proto:269-275`
- `crates/blit-core/src/remote/pull.rs:680-697`
- `crates/blit-core/tests/pull_sync_with_spec_wire.rs:251-310`
**Notes**: The plan inventory itself notes the contradiction (`contradiction-pullsync-leading-message`). High severity because §6 is the authoritative narrative for how pull_sync works — anyone reading WHITEPAPER to understand the resume/mirror protocol gets misled about the wire shape. The `LocalFile`/`ManifestDone` pseudonames are also stale (the wire uses `local_file` / `ManifestComplete`). Fix: rewrite §6 step 2 to "Client sends `TransferOperationSpec` (replacing the legacy `PullSyncHeader`), then a `FileHeader` per local entry on `local_file`, then `ManifestComplete`."

---

## Medium severity

### m-1-daemon-config-missing-metrics-flag
**Plan says**: `docs/DAEMON_CONFIG.md:280-293` `--config`, `--bind`, `--port`, `--root`, `--no-mdns`, `--mdns-name`, `--force-grpc-data`, `--no-server-checksums`, `-h/--help`. Plan claim `iface-daemon-cli-flags` enumerates the same list.
**Code does**: `crates/blit-daemon/src/runtime.rs:104-109` defines a `--metrics` flag controlling the internal counter atomics. Used by `crates/blit-daemon/src/service/core.rs:457-543` (`metrics.inc_push()`, etc.) and surfaced in `DaemonState.counters` (`proto/blit.proto:752-756`).
**Evidence**:
- `crates/blit-daemon/src/runtime.rs:104-109`
- `proto/blit.proto:752-756` (Counters field comment references `--metrics`)
- `docs/DAEMON_CONFIG.md:280-293` (omits the flag)
**Notes**: This is the same flag whose absence causes the "GetState Counters are present-but-zero" footgun documented in `feedback_getstate_counters_zero.md`. Operators reading DAEMON_CONFIG.md cannot discover that they need `--metrics` to get non-zero counters; the bridge (`metrics.rs:38-96`) silently emits zero gauges in the meantime. Fix: add `--metrics` to the §Command-Line Options block and a one-paragraph note explaining the on/off semantics + the `GetState.Counters` interaction.

### m-2-daemon-config-motd-claim-misleads
**Plan says**: `docs/DAEMON_CONFIG.md:107` "`motd` | string | none | Message displayed to clients on connect".
**Code does**: `motd` is loaded from config (`runtime.rs:67, 126, 203, 348`) and printed via `println!("motd: {motd}")` to the daemon's own stdout at startup (`crates/blit-daemon/src/main.rs:39-41`). It is never serialized into any proto message, never sent over gRPC, never advertised on mDNS — clients cannot observe it.
**Evidence**:
- `docs/DAEMON_CONFIG.md:60-65, 107`
- `crates/blit-daemon/src/main.rs:28, 39-41`
- `crates/blit-daemon/src/runtime.rs:67, 126, 203, 348` (load only)
- (No reference to `motd` anywhere in `proto/blit.proto` or `crates/blit-core/src/mdns.rs`)
**Notes**: "Displayed to clients on connect" implies a wire-visible message-of-the-day; the implementation is a one-shot operator-side log line. Suggest either (a) clarify the doc as "displayed in the daemon's own startup log; not visible to clients" or (b) actually surface it on `GetState`/mDNS if that was the intent. Adjacent smell: there's no test for `motd` anywhere in the test inventory.

### m-3-mdns-txt-record-undocumented-fields
**Plan says**: `docs/DAEMON_CONFIG.md:530-532` "The mDNS TXT record includes: `version` — daemon version; `modules` — comma-separated list of exported module names (truncated to 180 chars)". Plan claim `iface-mdns-service` repeats the same two fields.
**Code does**: `crates/blit-core/src/mdns.rs:140-156` advertises FOUR TXT fields: `version`, `modules`, `module_count` (authoritative integer because `modules` is truncated past ~180 bytes — §3.2), and `delegation_enabled` (mirrors `[delegation].allow_delegated_pull`). The latter two are NOT documented in DAEMON_CONFIG.md.
**Evidence**:
- `crates/blit-core/src/mdns.rs:140-156, 263-281` (truncate_modules)
- `crates/blit-core/src/mdns.rs:43-65` (`module_count` accessor + comment about backward compat)
- `crates/blit-core/src/mdns.rs:73-77` (`delegation_enabled` field comment "as the `delegation_enabled` TXT record")
- `docs/DAEMON_CONFIG.md:530-532` (lists only 2 of 4)
- `crates/blit-tui/src/screens/f1.rs:394-495` (TUI consumes `delegation_enabled` for F1 rendering — observable to operators)
**Notes**: Plan claim `decision-mdns-txt-modules-truncated` is correct about the 180-byte cap but doesn't surface that `module_count` is the authoritative source for the full count. Fix: add `module_count` and `delegation_enabled` to the §"mDNS Discovery" TXT record list in DAEMON_CONFIG.md and to the plan's `iface-mdns-service` claim. Adjacent: the proto field `DaemonState.delegation_enabled = 7` is documented to "mirror the mDNS TXT advertisement" (`proto/blit.proto:757-759`) — without the TXT being documented, that comment is dangling.

### m-4-architecture-linux-change-journal-fallback-mtime-only-incomplete
**Plan says**: `docs/ARCHITECTURE.md:293-297` "Platform | Implementation: … Linux | Fallback to mtime comparison".
**Code does**: `crates/blit-core/src/change_journal/snapshot.rs:50-63, 133-153` captures Linux snapshots as `(device, inode, ctime_sec, ctime_nsec)` and compares on device OR inode change first, then ctime equality, with mtime only as a final fallback within the compare. So Linux uses metadata snapshot — not just mtime.
**Evidence**:
- `crates/blit-core/src/change_journal/snapshot.rs:50-63, 133-153`
- `docs/ARCHITECTURE.md:293-297`
- Plan claim `behavior-change-journal-platforms` repeats the same "Linux = fallback to mtime"
**Notes**: ARCHITECTURE.md `code-misc` inventory line 58 already gets this right ("metadata snapshot on Linux") but the platform-implementation table on line 297 doesn't. Code is actually more robust than the doc claims — fix the table to "Metadata snapshot (device/inode/ctime, mtime fallback)" so reviewers don't underestimate Linux coverage.

### m-5-architecture-perf-history-shape-stale-vs-actual
**Plan says**: `docs/ARCHITECTURE.md:432-439` `PerformanceRecord { timestamp, bytes_transferred, duration_ms, file_count, strategy_used: TransferStrategy }`. Plan claim `behavior-perf-history-record` repeats this.
**Code does**: `crates/blit-core/src/perf_history.rs` carries a richer schema with explicit migration from v1→v2 (`migrate_record`), `run_kind`, `mode`, options-bag, etc. Plan-quoted shape is the v1 pre-migration shape.
**Evidence**:
- `crates/blit-core/src/perf_history.rs:140-147` (record fields with `#[serde(default)]` migration)
- `crates/blit-core/src/perf_history.rs:273-300` (migrate_record)
- `docs/ARCHITECTURE.md:432-439`
**Notes**: ARCHITECTURE doc shows a struct snippet that doesn't compile against current code. The plan inventory captured the doc faithfully but the doc is stale. Fix: regenerate the snippet from current `PerformanceRecord` source — or strike the snippet and link to the type, as for `compare_manifests` at WHITEPAPER §6.

### m-6-whitepaper-section-2-1-round-robin-comment-vs-bias-warning
**Plan says**: `docs/WHITEPAPER.md:§2.1` (per plan claim `behavior-round-robin-dispatch`): "`execute_sink_pipeline_streaming` dispatcher pulls from `payload_rx` and round-robins to per-sink channels. 'Round-robin is deliberately simple. Adaptive load-balancing is left to the lower layers.'"
**Code does**: `crates/blit-core/src/remote/transfer/pipeline.rs:91-129` actually fans payloads to per-sink mpsc channels then awaits a primary worker per sink. There's no explicit round-robin index counter in the dispatcher — it `try_send`s and falls back, which is closer to "first-available" than literal round-robin. The plan-quoted text is faithful to the doc, but the doc itself describes a behavior that doesn't quite match the code.
**Evidence**:
- `crates/blit-core/src/remote/transfer/pipeline.rs:91-165`
- `docs/WHITEPAPER.md:§2.1`
**Notes**: Minor narrative drift — call out the actual dispatch shape (load-balanced fan-out, not strict round-robin) to avoid leading reviewers to look for a non-existent counter.

### m-7-whitepaper-build-plan-thresholds-vs-code
**Plan says**: `docs/WHITEPAPER.md:§4` (per plan claim `behavior-build-plan-thresholds`): "size <64 KiB → small (tar shard candidate); <1 MiB → small; <256 MiB → medium (raw bundle); ≥256 MiB → large_files (single TransferTask::Large)". Plan also notes "use_tar: force_tar → ≥1; otherwise <2 small=false; ≥32 small or avg_small_size ≤128 KiB".
**Code does**: `crates/blit-core/src/transfer_plan.rs:55-225` confirms the use_tar branches verbatim. The "<64 KiB → small … <1 MiB → small" pair is a single double-arm in the code that the whitepaper renders as two separate buckets — fine, but the actual size bins in code are <1 MiB → small, 1–256 MiB → medium, ≥256 MiB → large.
**Evidence**:
- `crates/blit-core/src/transfer_plan.rs:55-225`
- `docs/WHITEPAPER.md:§4`
**Notes**: Low-risk narrative simplification. The plan claim is correct as a reading of the doc; the doc misrepresents code by saying "<64 KiB" tar-shard candidate when in practice all files <1 MiB are tar candidates and the 64 KiB threshold is the size estimate fallback in `size_map.get(p).unwrap_or(&(64 * 1024))` (line 160). Worth a one-line doc fix.

### m-8-whitepaper-3-1-1mib-buffer-vs-pool
**Plan says**: `docs/WHITEPAPER.md:§3.1` (per plan claim `behavior-double-buffered-1mib`): "Send and receive byte-copy loops are intentionally similar: double-buffered, 1 MiB chunks; max two outstanding per session."
**Code does**: `crates/blit-core/src/remote/transfer/data_plane.rs:528` `RECEIVE_CHUNK_SIZE: usize = 1 MiB` — confirms the 1 MiB. But the "two outstanding per session" is implemented via `pool: Arc<BufferPool>` (whitepaper code snippet at lines 237-238 references `self.pool.acquire().await` but the current code at `data_plane.rs:256-335` uses the inline `buf_a`/`buf_b` form, not a pool. The pool is documented in §3.1 but not used at that send site.
**Evidence**:
- `crates/blit-core/src/remote/transfer/data_plane.rs:256-335` (no `pool` member; inline `Vec<u8>` buffers)
- `docs/WHITEPAPER.md:236-274` (claims `pool.acquire()` and references a `BufferPool`)
- `crates/blit-core/src/buffer.rs:1-619` (BufferPool exists but is used for local copies, not data plane sessions per the docstring)
**Notes**: The whitepaper code snippet is illustrative but uses a `self.pool.acquire().await` pattern that doesn't match `DataPlaneSession` in current code. The 1 MiB chunk and double-buffer claim are correct; the pool-acquisition pattern is misleading. Fix the snippet to reflect the actual send loop.

### m-9-architecture-fs-capability-supported-types-vs-code
**Plan says**: `docs/ARCHITECTURE.md:§Filesystem Capability Probing` (per plan claim `behavior-fs-capability-probing`): "Supported FS types: APFS, HFS+, btrfs, XFS, ext2/3/4, ZFS, tmpfs, NFS/CIFS/SMB, NTFS, ReFS."
**Code does**: `crates/blit-core/src/fs_capability/probe.rs:27-149` covers the listed types. `:236-251` maps Linux statfs magic numbers including `0x6969=nfs`, `0xFF534D42=cifs`, `0x9123683E=btrfs`, `0xEF53=ext4`, `0x5346544E=ntfs`. Unknown FS types are routed to `format!("unknown(0x{:X})", f_type)` rather than dropped — meaning probe always returns *some* answer.
**Evidence**:
- `crates/blit-core/src/fs_capability/probe.rs:27-149, 236-251`
- `docs/ARCHITECTURE.md:§Filesystem Capability Probing`
**Notes**: Code does match the listed types, but the doc undersells the unknown-handling. Mention the `unknown(0x…)` fallback so operators understand probes never silently fail.

---

## Low severity

### l-1-blit-rpc-deprecated-only-in-comment
**Plan says**: `proto/blit.proto:10-11` marks Pull RPC as DEPRECATED in the doc-comment. Plan claim `rejected-pull-rpc-deprecated` carries this verbatim.
**Code does**: `proto/blit.proto:11` has the prose deprecation but no `option deprecated = true;` annotation. Generated tonic stubs therefore do NOT mark the RPC as deprecated; any client tooling that auto-detects deprecation will see Pull as live.
**Evidence**:
- `proto/blit.proto:10-11`
- `code-bridge-proto.md` smell §1 corroborates
**Notes**: Tooling-level inaccuracy; not a runtime bug. Fix: add `option deprecated = true;` to the RPC definition.

### l-2-server-pullsync-ack-deprecated-only-in-comment
**Plan says**: `proto/blit.proto:302` `ServerPullMessage.ack` is "(deprecated, use pull_sync_ack)". Plan claim `rejected-ack-deprecated` repeats this.
**Code does**: Field carries the comment but lacks `[deprecated = true]` proto-side. Same generated-stub issue as l-1.
**Evidence**:
- `proto/blit.proto:302`
- `code-bridge-proto.md` smell §2
**Notes**: Same fix shape as l-1.

### l-3-daemon-config-blit-scan-duplicated-command
**Plan says**: `docs/DAEMON_CONFIG.md:526-528` "clients can discover it with `blit scan` or `blit scan`."
**Code does**: There is only one `blit scan` command (`crates/blit-cli/src/scan.rs:26-105`); the duplicated phrasing is an editing slip caught by the plan inventory itself as `contradiction-blit-scan-command-form`.
**Evidence**:
- `docs/DAEMON_CONFIG.md:524-528`
- `crates/blit-cli/src/scan.rs:26-105`
**Notes**: Pure copy-edit fix.

### l-4-blit-prometheus-bridge-counters-not-emitted
**Plan says**: `proto/blit.proto:752-756` documents `Counters` as a published-but-zero snapshot when `--metrics` is off. Plan claim `iface-counters` enumerates the 5 fields.
**Code does**: `crates/blit-prometheus-bridge/src/metrics.rs:38-96` deliberately OMITS all 5 counter series ("gauges only by design; tied to the GetState.Counters present-but-zero bug recorded in feedback_getstate_counters_zero.md"). So a Prometheus consumer scraping the bridge sees NO `blit_push_operations_total` / `blit_pull_operations_total` / etc. at all.
**Evidence**:
- `crates/blit-prometheus-bridge/src/metrics.rs:38-96`
- `proto/blit.proto:752-756`
- `code-bridge-proto.md` smell §3
**Notes**: This isn't strictly drift — the bridge intentionally suppresses these counters because of the present-but-zero footgun. But neither the proto comment nor ARCHITECTURE.md §"blit-prometheus-bridge" mentions that the counters are omitted from the exposition. Add a sentence to either doc so readers don't expect them.

### l-5-daemon-data-plane-tcp-keepalive-silenced
**Plan says**: `docs/WHITEPAPER.md:§3.1` describes the symmetric send/receive loops but says nothing about TCP socket tuning.
**Code does**: `crates/blit-daemon/src/service/push/data_plane.rs:115-116` applies `set_tcp_nodelay(true)` + `set_keepalive(true)` per-stream but only on the push path; pull/pull_sync data plane accept paths do NOT (`code-daemon.md` smell §5). The errors are silently dropped (`let _ = …`).
**Evidence**:
- `crates/blit-daemon/src/service/push/data_plane.rs:115-116`
- `crates/blit-daemon/src/service/pull.rs:697-737` and `pull_sync.rs:596-735` (no equivalent tuning)
**Notes**: Doc gap, not a behavior bug. Worth a note in WHITEPAPER §3.1 about asymmetric socket tuning so reviewers know the push/pull paths diverge.

### l-6-token-comparison-not-constant-time
**Plan says**: `proto/blit.proto:DataTransferNegotiation.one_time_token` (per plan claim `invariant-data-plane-one-time-tokens`) — tokens minted per transfer, peers without the right token are dropped before any bytes flow.
**Code does**: `crates/blit-daemon/src/service/push/data_plane.rs:183`, `pull.rs:738`, `pull_sync.rs:631, 753` compare token bytes with `==` (variable-time). Tokens are 32 random bytes from OS RNG so practical risk is low, but the doc invariant suggests strict access control; an explicit "non-constant-time compare" caveat would be honest.
**Evidence**:
- `crates/blit-daemon/src/service/push/data_plane.rs:182-186`
- `code-daemon.md` smell §7
**Notes**: Defer pending a real threat model. Cite this as a known property in WHITEPAPER §3 if reviewers ever ask.

### l-7-architecture-future-directions-vs-non-goals-overlap
**Plan says**: `docs/ARCHITECTURE.md:496-503` Future Directions list includes "RDMA Support: Reserved fields in protocol for RDMA data plane" (no phase number) but `proto/blit.proto:126` references "Phase 3.5". Plan flags this as `contradiction-rdma-phase-3.5-vs-future-directions`.
**Code does**: Proto reservation matches; ARCHITECTURE simply doesn't carry the phase tag.
**Evidence**:
- `proto/blit.proto:126`
- `docs/ARCHITECTURE.md:498`
**Notes**: Minor narrative inconsistency. Either remove "Phase 3.5" from the proto comment or add it to ARCHITECTURE.

---

## Claims that align well

The following claim groups verified cleanly against code:

- **Wire format records and tags** (FILE=0x00, TAR_SHARD=0x01, BLOCK=0x02, BLOCK_COMPLETE=0x03, END=0xFF; 32-byte token prepends): code `crates/blit-core/src/remote/transfer/data_plane.rs:15-19, 200-516` matches WHITEPAPER §3 lines 196-204 exactly. `BLOCK_COMPLETE` carries mtime+perms inline as documented.
- **GetState/Subscribe/CancelJob/ClearRecent RPCs**: proto definitions (`proto/blit.proto:50-107`) match daemon implementations (`crates/blit-daemon/src/service/core.rs:353-1146`). Field numbers, default `recent_limit`, `JOB_EVENT_RING_CAP`, replay semantics — all match.
- **Module containment + canonicalize-then-operate**: `crates/blit-core/src/path_safety.rs:180-220, 228-260, 283-329` implements the documented contained-join with the same TOCTOU caveat surfaced in DAEMON_CONFIG.
- **Delegation gate ordering and DNS-rebinding mitigation**: `crates/blit-daemon/src/delegation_gate.rs:288-392` matches DAEMON_CONFIG's enumerated allowlist rules (master switch → empty host → port 0 → IP-form for special ranges → resolve-once → all-match → bind validated IP).
- **`require_complete_scan` purge gate and v2 spec bump**: `crates/blit-core/src/remote/transfer/operation_spec.rs:42-47, 86-92` matches `proto/blit.proto:178-186, 213-220, 426-434`.
- **MirrorMode default = FILTERED_SUBSET, ALL bypasses filter**: proto enum (`proto/blit.proto:548-562`) + daemon purge filter (`crates/blit-daemon/src/service/admin.rs:69-87`) + pull-sync delete-list scope (`crates/blit-daemon/src/service/pull_sync.rs:424-463`) all converge.
- **Workspace crate list + binary names**: `blit-core`, `blit-cli` (produces `blit`), `blit-daemon`, `blit-app`, `blit-tui`, `blit-prometheus-bridge` — all present on disk and consistent with ARCHITECTURE §Crate Structure.
- **Hardcoded constants WHITEPAPER §8.4 calls out** (`RECEIVE_CHUNK_SIZE = 1 MiB`, `MAX_PARALLEL_TAR_TASKS = 4`, tar shard count thresholds 32/1024/2048, mpsc capacities) — all present at the cited code sites and not yet routed through `TuningParams`. The doc's "open gap" is faithful to current code.
- **Auto-tune output struct `TuningParams { initial_streams, max_streams, chunk_bytes, prefetch_count, tcp_buffer_size }`**: matches `crates/blit-core/src/auto_tune/mod.rs:1-30`.
- **`compare_manifests` shape and CompareMode set** (Default/SizeOnly/IgnoreTimes/Force/Checksum): matches `crates/blit-core/src/manifest.rs:83-209`.
- **mDNS instance name default `blit@<hostname>`**: matches `crates/blit-core/src/mdns.rs:283-290`.
- **Default port 9031, default bind 0.0.0.0**: matches `crates/blit-daemon/src/runtime.rs:200-201`.
- **CLI verb list** (copy/mirror/move/scan/list/list-modules/ls/find/du/df/rm/completions/profile/diagnostics + check + jobs): matches `crates/blit-cli/src/cli.rs`.

The high-level architecture story (gRPC control + TCP data plane, identical pipeline for every direction, single `execute_sink_pipeline` entry point, hybrid wire format, adaptive planning) all holds up against code. The documented drift is concentrated in: (1) the Security Considerations subsection of ARCHITECTURE.md, which carries stale claims about removed auth/TLS work; (2) the WHITEPAPER §6 narrative, which still references the removed `PullSyncHeader`; (3) DAEMON_CONFIG.md gaps around `--metrics` and the full TXT record contents; and (4) a few proto-level `deprecated` annotations that exist only as comments.
