# Plan Inventory: Phase history + post-review fixes + delegation
**Generated**: 2026-06-04 by audit workflow
**Coverage**: 6 files, 1929 lines total

- `docs/plan/POST_REVIEW_FIXES.md` (308 lines)
- `docs/plan/REMOTE_REMOTE_DELEGATION_PLAN.md` (1084 lines)
- `docs/plan/WORKFLOW_PHASE_2.md` (100 lines)
- `docs/plan/WORKFLOW_PHASE_3.md` (127 lines)
- `docs/plan/WORKFLOW_PHASE_4.md` (130 lines)
- `docs/plan/PROJECT_STATE_ASSESSMENT.md` (180 lines)

---

## Claims (grouped by category)

### principle

#### principle-no-user-tunables
**Source**: WORKFLOW_PHASE_2.md §"Guiding Principles" #1
**Specificity**: high

> "No user tunables – Planner owns performance decisions. The sole debug limiter (`--workers`) must be clearly labelled, pause "FAST" guarantees when active, and remain hidden from normal help output (documented in `docs/cli/blit.1.md`)."

#### principle-telemetry-stays-local
**Source**: WORKFLOW_PHASE_2.md §"Guiding Principles" #2
**Specificity**: high

> "Telemetry stays local – JSONL log under config dir, capped to ~1 MiB. Opt-out should be driven by CLI/config settings (no environment variables once work completes)."

#### principle-documentation-first
**Source**: WORKFLOW_PHASE_2.md §"Guiding Principles" #3
**Specificity**: medium

> "Documentation-first – Update plan/docs/DEVLOG as tasks complete to survive context resets."

#### principle-fast-simple-reliable-private
**Source**: WORKFLOW_PHASE_2.md §"Goal"
**Specificity**: low

> "Deliver the local transfer pipeline defined in plan v6 (streaming planner, adaptive predictor, local performance history, and progress UX) while keeping FAST/SIMPLE/RELIABLE/PRIVATE principles intact."

#### principle-no-backcompat-tech-debt
**Source**: REMOTE_REMOTE_DELEGATION_PLAN.md §4.4
**Specificity**: high

> "The release directive ('carry no tech debt for the sake of backwards compatibility') rules out automatic fallback. Auto-falling-back on `Unimplemented` is exactly stale-daemon support."

#### principle-operator-trust-anchor
**Source**: REMOTE_REMOTE_DELEGATION_PLAN.md §4.3.6
**Specificity**: medium

> "The operator remains the trust anchor; daemons never trust each other implicitly."

#### principle-no-silent-fallback
**Source**: REMOTE_REMOTE_DELEGATION_PLAN.md §4.4, §4.5
**Specificity**: high

> "Auto-falling-back on `CONNECT_SOURCE` is exactly the kind of silent reroute that masks real misconfiguration. The operator gets one explicit escape hatch (`--relay-via-cli`); other failures surface verbatim." … "Every fallback heuristic that we removed has a clear failure mode that's better surfaced than papered over."

#### principle-cli-orchestrator-not-byte-path
**Source**: REMOTE_REMOTE_DELEGATION_PLAN.md §1
**Specificity**: high

> "make `server-A:/x server-B:/y` transfers route the byte path directly between A and B. The CLI orchestrates and reports; it does not relay bytes."

#### principle-pipeline-already-symmetric
**Source**: REMOTE_REMOTE_DELEGATION_PLAN.md §2 ("Critical observation")
**Specificity**: high

> "the data plane is *already* symmetric. `TcpListener::bind` on the daemon side and `TcpStream::connect` on the connector side are independent of which host is "client." A daemon can play the connector role with no protocol change."

#### principle-policy-before-network
**Source**: REMOTE_REMOTE_DELEGATION_PLAN.md §4.2, §4.3.3
**Specificity**: high

> "no path resolution and no outbound connect before policy approves" — load-bearing security invariant; gate ordering: locator parse → daemon-wide gate → module metadata lookup → per-module override → path resolution + F2 containment → outbound connect.

#### principle-gate-is-policy-not-auth
**Source**: REMOTE_REMOTE_DELEGATION_PLAN.md §4.3.4
**Specificity**: high

> "The gate is policy, not authentication. With auth disabled (today's default), anyone reaching the daemon's control plane can request a delegation against any allowlisted source. … The gate does not pretend to be auth."

#### principle-client-capabilities-describes-byte-recipient
**Source**: REMOTE_REMOTE_DELEGATION_PLAN.md §4.1, §4.2 step 8, §4.2 "Spec authorship"
**Specificity**: high

> "`TransferOperationSpec.client_capabilities` describes 'the initiator's side of the wire' — what the host that will *receive* payload bytes from the origin can handle. … `client_capabilities` is the one field where CLI-supplied values are non-authoritative. The destination handler mandatorily REPLACES `client_capabilities` with its own PeerCapabilities before forwarding the spec to src."

#### principle-endpoint-transport-only
**Source**: REMOTE_REMOTE_DELEGATION_PLAN.md §4.2 ("Required core-side refactor"), §8
**Specificity**: high

> "`pull_sync_with_spec` MUST NOT read `self.endpoint.path` to derive any spec field. The endpoint is purely a transport handle (host:port for the gRPC connection); the spec is authoritative for module + source_path + every other field."

#### principle-loud-failure-on-invalid-config
**Source**: REMOTE_REMOTE_DELEGATION_PLAN.md §4.3.3 #2
**Specificity**: high

> "Invalid entries fail config load (loud failure beats silent permissive default)."

#### principle-no-silent-error-swallow
**Source**: POST_REVIEW_FIXES.md §1.1
**Specificity**: high

> "Several `let _ = ...` patterns hide failures that the caller assumes succeeded. Most consequential: the `file.flush().await` immediately before mtime application — if flush fails (disk full, EIO), the user believes the file is durable when it isn't." — flush failure must propagate as Err; other best-effort calls must log via tracing::warn.

---

### invariant

#### invariant-flush-must-propagate
**Source**: POST_REVIEW_FIXES.md §1.1
**Specificity**: high

> "Line ~256 `write_file_stream`: change `let _ = file.flush().await;` to `file.flush().await.with_context(|| format!("flushing {}", dst.display()))?;` — flush failure is a data-loss signal, propagate it."

#### invariant-pipeline-real-error-surfaced
**Source**: POST_REVIEW_FIXES.md §1.1b
**Specificity**: high

> "When the streaming pipeline dies (sink worker errored, remote daemon closed, disk full on dest), the receiver inside `execute_sink_pipeline_streaming` is dropped. The next `tx.send().await` in `queue()` then fails with the generic: `data plane pipeline closed unexpectedly` — and the *actual* error (the `Err` sitting inside `pipeline_handle`) is never surfaced." — fix: await `pipeline_handle` to extract the real error and propagate that.

#### invariant-block-complete-includes-mtime-perms
**Source**: POST_REVIEW_FIXES.md §1.3, "Already done"
**Specificity**: high

> "BLOCK_COMPLETE := 0x03 path_len:u32 path:bytes total_size:u64 mtime:i64 perms:u32" — mtime/perms now travel with the terminator so the auto-promote (zero-block-transfer) case correctly updates the destination metadata.

#### invariant-auto-promote-modified-with-resume
**Source**: POST_REVIEW_FIXES.md "Already done"
**Specificity**: high

> "Auto-promote `Modified` (size-match, mtime-mismatch) → block-hash compare without `--resume`" — landed in a7d659f.

#### invariant-f2-canonical-containment-always-on
**Source**: WORKFLOW_PHASE_3.md §3.2.1, §3.3.2, §3.4.1, "Deliverables Checklist"
**Specificity**: high

> "`use_chroot` field removed in F13 / 2026-05-02; containment is now always-on via F2 canonical-path enforcement." — daemon enforces F2 canonical-path containment per-call; admin RPCs/path-resolution all gated.

#### invariant-no-tokio-mutex-across-await
**Source**: POST_REVIEW_FIXES.md §2.2
**Specificity**: high

> "Holds `tokio::sync::Mutex` guard across `await` for the entire data-plane transfer. … canonical anti-pattern." — restructure so the receiver is owned by exactly one task (mpsc::Receiver passed directly, or use flume).

#### invariant-dns-rebinding-mitigation
**Source**: REMOTE_REMOTE_DELEGATION_PLAN.md §4.3.3 #5
**Specificity**: high

> "DNS-rebinding mitigation (load-bearing) — the IP set produced by the resolution in step 4 is bound to the connection. The outbound `RemotePullClient` connects to a specific resolved IP, not to the hostname. … pass an already-resolved `SocketAddr` to the gRPC connector; do not pass a hostname URI that would re-resolve."

#### invariant-all-resolved-addresses-must-pass
**Source**: REMOTE_REMOTE_DELEGATION_PLAN.md §4.3.3 #4
**Specificity**: high

> "Every resolved address must match either a CIDR entry or a bare IP entry (the literal hostname can also match per rule 3 — but only if listed). If any resolved address is unmatched, the gate denies."

#### invariant-loopback-requires-ip-form-authorization
**Source**: REMOTE_REMOTE_DELEGATION_PLAN.md §4.3.3 #7, R25-F3
**Specificity**: high

> "Loopback and link-local addresses require IP/CIDR authorization, not hostname authorization." — A resolved address in `127.0.0.0/8`, `169.254.0.0/16`, `::1`, `fe80::/10`, `fc00::/7`, `0.0.0.0/8`, `::` is rejected unless an IP- or CIDR-form allowlist entry covers it; hostname-form match alone is insufficient. Closes SSRF-via-DNS pivot.

#### invariant-wire-equivalence-pull-sync
**Source**: REMOTE_REMOTE_DELEGATION_PLAN.md §4.2 (Required tests), Phase 1 #7
**Specificity**: high

> "Wire-equivalence: `build_spec_from_options(endpoint, opts)` followed by `pull_sync_with_spec(... spec ...)` produces an identical on-the-wire spec to today's `pull_sync(... opts ...)` for a representative options matrix."

#### invariant-endpoint-isolation
**Source**: REMOTE_REMOTE_DELEGATION_PLAN.md §4.2, R25-F1
**Specificity**: high

> "Endpoint-isolation: a hand-built spec with `module = "alpha"`, `source_path = "x/y"` produces those exact values on the wire when handed to `pull_sync_with_spec`, even when the `RemotePullClient`'s endpoint was constructed with a different `RemotePath::Module { module: "beta", rel_path: "z" }`. The spec wins; the endpoint is transport-only."

#### invariant-mandatory-client-capabilities-override
**Source**: REMOTE_REMOTE_DELEGATION_PLAN.md §4.2 step 8, R25-F2
**Specificity**: high

> "Mandatory `client_capabilities` override: replace `spec.client_capabilities` with this destination daemon's actual `PeerCapabilities`. … This override is unconditional — the field is rewritten regardless of what the CLI sent."

#### invariant-stall-detector-10s
**Source**: WORKFLOW_PHASE_2.md §"Success Criteria"
**Specificity**: high

> "Planner flushes batches incrementally; stall detector aborts with clear messaging after 10 s of inactivity."

#### invariant-first-byte-under-1s
**Source**: WORKFLOW_PHASE_2.md §"Success Criteria"
**Specificity**: high

> "`blit copy` / `blit mirror` start emitting data within ≤ 1 s of command invocation for qualifying workloads." … "Remote transfers expose first-payload latency via `--progress`/`--verbose` so benchmarks can confirm < 1 s start."

---

### interface

#### interface-blit-binary-merged
**Source**: PROJECT_STATE_ASSESSMENT.md §2 (CLI Surface), WORKFLOW_PHASE_3.md/PHASE_4.md historical notes
**Specificity**: high

> "`blit`: copy, mirror, move, scan, list, list-modules, ls, find, du, df, rm, completions, profile, diagnostics (admin utilities merged into `blit` since this snapshot — no separate `blit-utils` binary)."

#### interface-blit-daemon
**Source**: PROJECT_STATE_ASSESSMENT.md §2
**Specificity**: high

> "`blit-daemon`: TOML config, modules, mDNS, hybrid transport."

#### interface-remote-url-syntax
**Source**: WORKFLOW_PHASE_3.md §1 (Success Criteria), §3.1.2
**Specificity**: high

> "`blit copy`, `blit mirror`, `blit move` accept local ↔ remote endpoints using `server:/module/...` and `server://...` syntax; hybrid transport negotiates TCP data plane with secure tokens and falls back to gRPC automatically." Canonical URL parser shared across CLI/daemon/utils (no `blit://` scheme).

#### interface-toml-config
**Source**: WORKFLOW_PHASE_3.md §3.2.1
**Specificity**: high

> "Add TOML config loader (`/etc/blit/config.toml` or path via `--config`) with module definitions (`name`, `path`, `comment`, `read_only`) and daemon settings (`bind`, `port`, `motd`, `no_mdns`, `mdns_name`, optional default root)."

#### interface-mdns-default-on
**Source**: WORKFLOW_PHASE_3.md §3.2.3, PROJECT_STATE_ASSESSMENT.md §2
**Specificity**: high

> "Integrate mDNS advertisement (`_blit._tcp.local.`) enabled by default, disabled via `--no-mdns`."

#### interface-delegated-pull-rpc
**Source**: REMOTE_REMOTE_DELEGATION_PLAN.md §4.1
**Specificity**: high

> "rpc DelegatedPull(DelegatedPullRequest) returns (stream DelegatedPullProgress);" — dst-side delegated initiator. CLI calls this on the destination daemon when both endpoints are remote.

#### interface-delegation-toml-block
**Source**: REMOTE_REMOTE_DELEGATION_PLAN.md §4.3.2
**Specificity**: high

> "[delegation] allow_delegated_pull = false … allowed_source_hosts = [...] (entries: hostname, CIDR IPv4/v6, bare IP). Per-module narrowing override under [[modules]] entries; override can only narrow, never widen."

#### interface-relay-via-cli-flag
**Source**: REMOTE_REMOTE_DELEGATION_PLAN.md §4.2, §4.5, §8
**Specificity**: high

> "New flag: `--relay-via-cli` (operator-selected; forces the old CLI-relay path). Lives on `TransferArgs`." — the only fallback; no silent automatic fallback on `Unimplemented` / `Unavailable` / `CONNECT_SOURCE`.

#### interface-remote-source-locator
**Source**: REMOTE_REMOTE_DELEGATION_PLAN.md §4.1
**Specificity**: high

> "message RemoteSourceLocator { string host = 1; uint32 port = 2; bytes delegated_credential = 10; }" — strict Blit remote endpoint, parsed through the same RemoteEndpoint code the CLI uses; rejects schemes other than the Blit gRPC control-plane scheme.

#### interface-delegated-pull-progress
**Source**: REMOTE_REMOTE_DELEGATION_PLAN.md §4.1
**Specificity**: high

> "DelegatedPullProgress oneof: DelegatedPullStarted started, ManifestBatch manifest_batch, BytesProgress bytes_progress, DelegatedPullSummary summary, DelegatedPullError error."

#### interface-pull-sync-with-spec
**Source**: REMOTE_REMOTE_DELEGATION_PLAN.md §4.2 "Required core-side refactor"
**Specificity**: high

> "pub async fn pull_sync_with_spec(&mut self, dest_root: &Path, local_manifest: Vec<FileHeader>, spec: TransferOperationSpec, track_paths: bool, progress: Option<&RemotePullProgress>) -> Result<RemotePullReport>" — pull using a pre-built, normalized spec; spec travels on wire unchanged.

#### interface-build-spec-from-options
**Source**: REMOTE_REMOTE_DELEGATION_PLAN.md §4.2 "Required core-side refactor"
**Specificity**: high

> "pub(crate) fn build_spec_from_options(endpoint: &RemoteEndpoint, options: &PullSyncOptions) -> Result<TransferOperationSpec>" — pure function lifting both pull.rs:397-409 endpoint/path derivation AND lines 433-484 spec construction.

#### interface-block-complete-wire
**Source**: POST_REVIEW_FIXES.md §1.3
**Specificity**: high

> "BLOCK_COMPLETE := 0x03 path_len:u32 path:bytes total_size:u64 mtime:i64 perms:u32"

---

### behavior

#### behavior-quiet-by-default
**Source**: WORKFLOW_PHASE_2.md §"Success Criteria", §2.3.2
**Specificity**: high

> "CLI remains quiet by default; progress mode exposes manifest + throughput events for GUIs/verbose listeners." … "Keep CLI quiet while exposing progress hooks (events/verbose logging) for GUI surfaces."

#### behavior-debug-limiters-banner
**Source**: WORKFLOW_PHASE_2.md §2.3.3
**Specificity**: high

> "When debug limiters are used, make it obvious (CLI banner/log) that FAST mode is capped."

#### behavior-perf-history-jsonl
**Source**: WORKFLOW_PHASE_2.md §2.2.1, §2.2.2
**Specificity**: high

> "Implement local performance history writer (capped JSONL)." … "EMA-based predictor segmented by filesystem profile."

#### behavior-diagnostics-perf-toggle
**Source**: WORKFLOW_PHASE_2.md §2.2.5
**Specificity**: high

> "Add CLI/config toggle for telemetry (`profile` command remains visible). Replace environment variable usage." … "Implementation must avoid environment-variable configuration."

#### behavior-admin-verbs-mdns-scan
**Source**: WORKFLOW_PHASE_3.md §3.2.4
**Specificity**: high

> "Ensure `blit scan` and `blit-utils scan` consume mDNS results cross-platform." — historical text; admin verbs now under `blit`.

#### behavior-grpc-fallback-warning
**Source**: WORKFLOW_PHASE_3.md §3.3.5
**Specificity**: high

> "Handle gRPC fallback automatically when TCP negotiation fails; emit warning and continue." Integration test (`remote_tcp_fallback`) forces client `--force-grpc` and asserts CLI reports `[gRPC fallback]` with files mirrored.

#### behavior-usn-fast-path-28ms
**Source**: WORKFLOW_PHASE_3.md §3.3.6, PROJECT_STATE_ASSESSMENT.md §5
**Specificity**: high

> "cache reprobe+comparison relaxed; zero-change NTFS mirror completes in 28 ms (wingpt-53)." (Windows zero-change USN fast-path).

#### behavior-streaming-manifest-no-vec
**Source**: WORKFLOW_PHASE_3.md §3.3.9, §3.4.6
**Specificity**: high

> "CLI streams manifests via bounded channel, daemon batches need lists with back-pressure." — remote push manifest streaming + chunked gRPC fallback landed; avoid materialising large payloads.

#### behavior-tar-shard-parallel-unpack
**Source**: WORKFLOW_PHASE_3.md §3.4 Status 2025-10-28
**Specificity**: high

> "the daemon side now parallelises tar-shard unpacking (`TarShardExecutor`, four blocking workers) so the data plane keeps flowing while shards decode." (Note: §1.2 of POST_REVIEW_FIXES later proposes deleting TarShardExecutor since it became redundant — see contradictions.)

#### behavior-purge-with-confirmation
**Source**: WORKFLOW_PHASE_3.md §3.4.3, §3.4 Status notes
**Specificity**: high

> "Provide safety prompts for destructive operations (default confirm, `--yes` bypass)." … "Purge RPC + `blit-utils rm` landed (with confirmation prompts). Remote mirror pushes reuse the purge helper to delete extraneous files (summary reports `entries_deleted`)."

#### behavior-multi-stream-tcp-up-to-16
**Source**: WORKFLOW_PHASE_3.md §3.3.4 (2025-11-15 note)
**Specificity**: high

> "`RemotePushClient` gained size-aware batching so multi-stream sends actually utilize every TCP worker, and daemon/client heuristics now negotiate up to 16 TCP streams on multi-GiB manifests."

#### behavior-remote-to-local-mirror-purge
**Source**: WORKFLOW_PHASE_3.md §3.4 Status 2025-11-06
**Specificity**: high

> "Remote-to-local mirrors now reuse the pull path plus a local purge pass, enabling `blit mirror skippy://module dest/` to delete extraneous files after copying."

#### behavior-trace-data-plane-hidden-flag
**Source**: WORKFLOW_PHASE_3.md §3.4 Status 2025-11-06
**Specificity**: medium

> "Added a hidden `--trace-data-plane` flag for diagnostics, restored streamed-file mtimes/permissions on the daemon path."

#### behavior-cli-streams-progress-via-RemoteTransferProgress
**Source**: WORKFLOW_PHASE_3.md §3.3.4 (2025-11-10 note)
**Specificity**: medium

> "the CLI reuses the shared `RemoteTransferProgress` monitor for both push/pull. Auto-tune chunk sizing + payload prefetch are now plumbed through both push and pull, and manifest need-lists flush immediately so first payloads launch within milliseconds even on huge manifests."

#### behavior-delegation-cli-session-bound
**Source**: REMOTE_REMOTE_DELEGATION_PLAN.md §4.2 step 12
**Specificity**: high

> "Cancellation: if the gRPC return stream closes (CLI Ctrl-C), drop the `pull_sync_with_spec` future, which propagates cancellation through the existing pull-side cleanup. Document that delegated pulls are CLI-session-bound; `--detach` is out of scope (§9)."

#### behavior-delegation-error-verbatim
**Source**: REMOTE_REMOTE_DELEGATION_PLAN.md §4.3.3, §4.4
**Specificity**: high

> "The `delegated_pull` handler emits `DelegatedPullError{phase=DELEGATION_REJECTED}` with a clear reason string for any rejected case … The CLI surfaces the reason verbatim. No silent denials."

#### behavior-stale-daemon-explicit-upgrade
**Source**: REMOTE_REMOTE_DELEGATION_PLAN.md §4.4
**Specificity**: high

> "Dst daemon doesn't implement `DelegatedPull` (stale daemon) | `tonic::Code::Unimplemented` from RPC | Fail with: 'destination daemon does not implement DelegatedPull; upgrade or pass `--relay-via-cli`'"

#### behavior-src-acl-refusal-no-fallback
**Source**: REMOTE_REMOTE_DELEGATION_PLAN.md §4.4
**Specificity**: high

> "Src refuses dst's connection (auth/ACL) | `DelegatedPullError{phase=NEGOTIATE}` with upstream message | Surface verbatim. Do **not** fall back — silently rerouting through CLI would defeat intentional ACLs."

#### behavior-tcp-tuning-warn-don't-fail
**Source**: POST_REVIEW_FIXES.md §1.1 (data_plane.rs)
**Specificity**: high

> "TCP tuning failures (`set_keepalive`, `set_send_buffer_size`, `set_recv_buffer_size`) are silently swallowed. Wrap each in a logged warn so missed buffer sizes show up in tracing. Don't fail — these are best-effort knobs — but the silence has been masking config issues."

#### behavior-mtime-perms-warn
**Source**: POST_REVIEW_FIXES.md §1.1
**Specificity**: high

> "Lines ~269 (mtime), ~282 (perms), ~426 (tar shard mtime), ~508–525 (block-complete mtime + perms): keep best-effort but log via `tracing::warn!(\"set mtime on {}: {}\", dst.display(), e);` so failures are visible."

---

### scope

#### scope-feature-complete-0.1.0
**Source**: PROJECT_STATE_ASSESSMENT.md §1 (Executive Summary)
**Specificity**: medium

> "Blit v2 is feature-complete for a 0.1.0 release. All phases through Phase 4 (Production Hardening) are substantially done. The remaining open items are benchmarking tasks that require dedicated hardware (10+ GbE network) and post-release investigations (RDMA, ReFS privilege)."

#### scope-phases-table
**Source**: PROJECT_STATE_ASSESSMENT.md §1 (table)
**Specificity**: high

> Phase 0 – Foundation: Done | Phase 1 – gRPC Scaffolding: Done | Phase 2 – Local Ops: Done | Phase 2.5 – Validation: Done | Phase 3 – Remote Ops: Done | Phase 4 – Production: Done | Phase 3.5 – RDMA: Deferred.

#### scope-trusted-lan-deployment
**Source**: REMOTE_REMOTE_DELEGATION_PLAN.md §4.3.4
**Specificity**: medium

> "That is acceptable for the 'trusted LAN' deployment model `DAEMON_CONFIG.md` already documents. For internet-exposed deployments, both `BlitAuth` (separate work) and the gate are required."

#### scope-0.1.0-trust-model
**Source**: REMOTE_REMOTE_DELEGATION_PLAN.md §4.3.1
**Specificity**: medium

> "The 0.1.0 trust model can still accept direct remote→remote, but the capability has to be opt-in and operator-controlled at the daemon, not implicit."

#### scope-operator-trust-network-controls
**Source**: REMOTE_REMOTE_DELEGATION_PLAN.md (top auth note 2026-05-13)
**Specificity**: high

> "`BlitAuth` and the `RemoteSourceLocator.delegated_credential` passthrough field were removed from project scope. … the trust model is operator network controls (firewall / VPN / SSH tunnel)."

#### scope-phase4-4.8.1-only-0.1.0
**Source**: WORKFLOW_PHASE_4.md §4.8 scope note
**Specificity**: high

> "0.1.0 ships **4.8.1 only** (client-side `fs_capability` cache + probes). 4.8.2 (daemon startup/idle persistence) and 4.8.3 (`blit diagnostics profile` capability probes) are deferred to 0.2.0 per `RELEASE_PLAN_v2_2026-05-04.md` §3.3 (D6 owner sign-off)."

#### scope-no-detach-mode
**Source**: REMOTE_REMOTE_DELEGATION_PLAN.md §9, §4.2 step 12, §7
**Specificity**: high

> "`--detach` mode where CLI exits and dst continues. Track as separate future feature." Out-of-scope explicitly.

#### scope-no-peer-mesh
**Source**: REMOTE_REMOTE_DELEGATION_PLAN.md §9
**Specificity**: high

> "Direct daemon-to-daemon connections that bypass the CLI control plane entirely (CLI as orchestrator is preserved — this is not a peer mesh)."

#### scope-rdma-deferred
**Source**: PROJECT_STATE_ASSESSMENT.md §1, §3, REMOTE_REMOTE_DELEGATION_PLAN.md §9
**Specificity**: high

> "RDMA/RoCE investigation (control-plane negotiation, transport abstraction)" — Post-Release. "RDMA/RoCE data plane (Phase 3.5)" — out of scope for delegation.

---

### non-goal

#### non-goal-keep-status-quo-relay
**Source**: REMOTE_REMOTE_DELEGATION_PLAN.md §1
**Specificity**: high

> "Non-goal: preserve the existing relay path as a fallback (we will keep it as a fallback for hostile-NAT cases and `--relay-via-cli` benchmarking, but it is not the default after this work lands)."

#### non-goal-pull-delegation-rejected
**Source**: REMOTE_REMOTE_DELEGATION_PLAN.md §3 Option B
**Specificity**: high

> "Pull delegation (rejected): Symmetric: CLI tells **src (A)** to push to B. Rejected because: 1) A would need write-side scheduling and diffing logic it doesn't carry today. 2) Existing pull-from-A semantics would need re-derivation in push-to-B form. 3) Auth credential delegation is harder."

#### non-goal-co-orchestration-rejected
**Source**: REMOTE_REMOTE_DELEGATION_PLAN.md §3 Option C
**Specificity**: high

> "Co-orchestration with TCP rendezvous (rejected): CLI does control-plane handshakes with both daemons, mints data-plane tokens on each, and tells one to connect to the other. Rejected: Two control-plane streams to manage, two error surfaces, more state in CLI; half the diff/manifest planning ends up on the wrong side or duplicated; 'dst as initiator' in Option A is strictly simpler and reuses 100% of existing pull/push internals."

#### non-goal-status-quo-rejected
**Source**: REMOTE_REMOTE_DELEGATION_PLAN.md §3 Option D
**Specificity**: high

> "Keep status quo (rejected per user direction): Current relay shape works, but the user's standing directive is that pipeline optimization is the next priority once the case for it is sound."

#### non-goal-f64-precision-not-pursued
**Source**: POST_REVIEW_FIXES.md §"Not pursuing"
**Specificity**: high

> "f64 precision loss for transfers > 9 PiB — academic; threshold is unrealistic. Cost of fixing is non-zero (`bytes_to_mb` is in the hot path of every prediction); trade isn't worth it."

#### non-goal-non-linear-predictor-not-pursued
**Source**: POST_REVIEW_FIXES.md §"Not pursuing"
**Specificity**: high

> "Linear perf-predictor model — a per-profile linear regression is a reasonable starting point. Going to non-linear / Bayesian / confidence intervals is real work for ambiguous gain."

#### non-goal-ai-telemetry-removed
**Source**: WORKFLOW_PHASE_4.md §4.9 scope note
**Specificity**: high

> "**Removed from project scope.** Per `RELEASE_PLAN_v2_2026-05-04.md` §5.4 (owner decision 2026-05-13), AI telemetry analysis is not on the roadmap. Performance history will continue to be collected for the predictor (§2.8) but no 'analyze my history' feature is planned."

#### non-goal-blit-auth-removed
**Source**: REMOTE_REMOTE_DELEGATION_PLAN.md auth note 2026-05-13
**Specificity**: high

> "`BlitAuth` and the `RemoteSourceLocator.delegated_credential` passthrough field were removed from project scope. Mentions of either in the body below describe the original forward-compat design intent and are now obsolete."

#### non-goal-no-blit-utils-binary
**Source**: WORKFLOW_PHASE_3.md/PHASE_4.md historical notes; PROJECT_STATE_ASSESSMENT.md §2
**Specificity**: high

> "the admin verbs originally scoped here as `blit-utils <verb>` ship as subcommands of the single `blit` binary." / "admin utilities merged into the `blit` binary."

#### non-goal-no-wildcards-allowlist
**Source**: REMOTE_REMOTE_DELEGATION_PLAN.md §4.3.3 #3
**Specificity**: high

> "Hostname-entry matches — exact post-normalization equality only. No wildcards in 0.1.0 (suffix matchers would be a footgun against `evil-server-a.lan`-style typos)."

#### non-goal-no-env-var-telemetry-config
**Source**: WORKFLOW_PHASE_2.md §"Guiding Principles" #2, §2.2.5
**Specificity**: high

> "Opt-out should be driven by CLI/config settings (no environment variables once work completes)." / "Implementation must avoid environment-variable configuration."

#### non-goal-no-protobuf-unknown-fields-strategy
**Source**: REMOTE_REMOTE_DELEGATION_PLAN.md §4.1, §7
**Specificity**: high

> "We do **not** rely on detecting unknown protobuf fields (proto3 silently preserves them; that's not a compatibility strategy)." Version drift handled through `TransferOperationSpec.spec_version` and `PeerCapabilities`.

---

### shipped

#### shipped-auto-promote-modified
**Source**: POST_REVIEW_FIXES.md "Already done"
**Specificity**: high

> "Auto-promote `Modified` (size-match, mtime-mismatch) → block-hash compare without `--resume` | a7d659f"

#### shipped-block-complete-mtime-perms
**Source**: POST_REVIEW_FIXES.md "Already done"
**Specificity**: high

> "`BLOCK_COMPLETE` wire format extended with `mtime + perms` so zero-block transfers still update dest mtime | a7d659f"

#### shipped-pull-sync-no-deadlock-test
**Source**: POST_REVIEW_FIXES.md "Already done"
**Specificity**: high

> "Regression test: `pull_sync_does_not_deadlock_with_populated_destination` | a7d659f"

#### shipped-pull-preserves-mtime-test
**Source**: POST_REVIEW_FIXES.md "Already done"
**Specificity**: high

> "Regression test: `pull_preserves_mtime_end_to_end` | a7d659f"

#### shipped-mtime-only-no-retransfer-test
**Source**: POST_REVIEW_FIXES.md "Already done"
**Specificity**: high

> "Regression test: `mtime_only_change_does_not_re_transfer_full_file` | a7d659f"

#### shipped-fuzz-wire-format-parser
**Source**: POST_REVIEW_FIXES.md "Already done"
**Specificity**: high

> "Wire-format fuzz harness (`fuzz_wire_format_parser_does_not_panic`) | a7d659f"

#### shipped-dos-bounds-parser
**Source**: POST_REVIEW_FIXES.md "Already done"
**Specificity**: high

> "DoS bounds on parser allocations (path / tar shard / block) | a7d659f"

#### shipped-round1-closed-2026-05-05
**Source**: POST_REVIEW_FIXES.md §"Round 1" status
**Specificity**: high

> "Round 1 — Status: Closed `2026-05-05`. §1.1, §1.1b, and §1.3 fully closed; §1.2 explicitly deferred with a docstring on `TarShardExecutor` and a tracked post-0.1.0 plan."

#### shipped-streaming-planner
**Source**: WORKFLOW_PHASE_2.md §2.1 (table)
**Specificity**: high

> "✅ `TransferFacade::stream_local_plan` emitting `PlannerEvent` ; ✅ Heartbeat loop in `drive_planner_events` ; ✅ Stall guard in `drive_planner_events`; Windows+Linux verified ; ✅ Fast-path routing implemented in orchestrator."

#### shipped-diagnostics-perf-cli
**Source**: WORKFLOW_PHASE_2.md §2.2.4
**Specificity**: high

> "✅ Command prints recent runs + stats." (`blit diagnostics perf`).

#### shipped-tcp-fallback-test
**Source**: WORKFLOW_PHASE_3.md §3.3.5
**Specificity**: high

> "2025-10-25 – integration test (`remote_tcp_fallback`) forces client `--force-grpc` and asserts CLI reports `[gRPC fallback]` with files mirrored."

#### shipped-usn-fast-path
**Source**: WORKFLOW_PHASE_3.md §3.3.6
**Specificity**: high

> "2025-10-25 – cache reprobe+comparison relaxed; zero-change NTFS mirror completes in 28 ms (wingpt-53)."

#### shipped-linux-metadata-snapshot
**Source**: WORKFLOW_PHASE_3.md §3.3.8
**Specificity**: high

> "2025-10-25 – metadata snapshot (device/inode/ctime) powers no-op fast-path; further fanotify work optional."

#### shipped-streaming-manifest-pushpull
**Source**: WORKFLOW_PHASE_3.md §3.3.9
**Specificity**: high

> "2025-10-26 – CLI streams manifests via bounded channel, daemon batches need lists with back-pressure."

#### shipped-admin-rpcs
**Source**: WORKFLOW_PHASE_3.md §3.4.1, §3.4.2
**Specificity**: high

> "✅ RPC handlers live (2025-10-24); F2 canonical containment + per-call read-only checks added 2026-05-02." / "✅ `crates/blit-utils/src/main.rs` updated; docs + TODO synced (2025-10-24)."

#### shipped-delegation-phase1-15991ed
**Source**: REMOTE_REMOTE_DELEGATION_PLAN.md (top header)
**Specificity**: high

> "Phase 3 cleanup/harness in workspace, 2026-05-03 (Phase 1 committed as `15991ed`; Phase 2 CLI dispatch + no-fallback tests implemented; live benchmark results still TBD)"

#### shipped-delegation-phase2-checked
**Source**: REMOTE_REMOTE_DELEGATION_PLAN.md §5 Phase 2 (all [x] except live)
**Specificity**: high

> "[x] Add `run_remote_to_remote_direct`; [x] Update dispatch; [x] Add `--relay-via-cli` flag; [x] Integration tests; [x] no_silent_fallback tests; [x] explicit_relay test."

#### shipped-delegation-phase3-cleanup
**Source**: REMOTE_REMOTE_DELEGATION_PLAN.md §5 Phase 3
**Specificity**: high

> "[x] Audit `RemoteTransferSource` usages. … [x] Update `docs/DAEMON_CONFIG.md`; [x] Update CLI usage docs; [x] Update TODO.md; [x] Add benchmark script."

#### shipped-pipeline-unification-phase4.7
**Source**: PROJECT_STATE_ASSESSMENT.md (top note)
**Specificity**: medium

> "Phase 4.7 (pipeline-unification) and Phase 4.8.1 (destination-side delegated-pull rework) supersede the snapshot's view."

#### shipped-remote-to-local-mirror-purge
**Source**: WORKFLOW_PHASE_3.md §3.4 Status 2025-11-06
**Specificity**: high

> "Remote-to-local mirrors now reuse the pull path plus a local purge pass."

#### shipped-cli-utils-merged
**Source**: PROJECT_STATE_ASSESSMENT.md §2, WORKFLOW_PHASE_3.md & 4.md headers
**Specificity**: high

> "admin utilities merged into the `blit` binary" — single `blit` CLI binary; no separate blit-utils.

---

### deferred

#### deferred-1.2-tar-shard-executor
**Source**: POST_REVIEW_FIXES.md §"Round 1" status, §1.2
**Specificity**: high

> "§1.2 explicitly deferred with a docstring on `TarShardExecutor` and a tracked post-0.1.0 plan." Per §1.2: "After Phase 5 of the receive-pipeline unification, the daemon's TCP push receive routes through `FsTransferSink::write_tar_shard_payload` (rayon-parallel). `TarShardExecutor` is now used **only** by the gRPC fallback path."

#### deferred-2.1-change-journal-tests
**Source**: POST_REVIEW_FIXES.md §2.1
**Specificity**: medium

> "`change_journal/` test coverage … grep -r 'fn test_' crates/blit-core/src/change_journal/ | wc -l → 0. … Three platform backends (Linux ctime snapshot, macOS FSEvents, Windows USN) all untested at the unit level. … Estimated 200 LOC of test."

#### deferred-2.2-drain-mutex-anti-pattern
**Source**: POST_REVIEW_FIXES.md §2.2
**Specificity**: high

> "Round 2 — Drain task `tokio::Mutex` anti-pattern — `data_plane.rs:147` holds `tokio::sync::Mutex` guard across `await` for the entire data-plane transfer. Low contention in practice (single drain task) but a canonical anti-pattern."

#### deferred-2.3-pre-allocation-guard
**Source**: POST_REVIEW_FIXES.md §2.3
**Specificity**: high

> "(Optional) Pre-allocation guard in `read_tar_shard` … Consider growing the vec lazily (`Vec::new()` + `push`)… Trade-off: marginal CPU vs marginal memory. Probably defer."

#### deferred-3.1-adaptive-tuning-expansion
**Source**: POST_REVIEW_FIXES.md §3.1
**Specificity**: medium

> "Adaptive tuning expansion — `auto_tune` covers `chunk_bytes`, `initial_streams`, `prefetch_count`, `tcp_buffer_size`. Doesn't cover: manifest batch size, channel capacities, planner thresholds (size buckets, tar shard targets), `RECEIVE_CHUNK_SIZE`."

#### deferred-3.2-journal-remote-rpc
**Source**: POST_REVIEW_FIXES.md §3.2
**Specificity**: medium

> "`change_journal` consulted for remote transfers … Big change: requires a journal-snapshot RPC, client-side caching of the snapshot ID per peer, and an 'if your last cached ID matches mine, skip the manifest' fast path on top of `pull_sync`."

#### deferred-3.3-mid-transfer-adaptation
**Source**: POST_REVIEW_FIXES.md §3.3
**Specificity**: medium

> "Mid-transfer parameter adaptation … research-y item — the right design borrows from BBR's bandwidth/RTT estimator. Defer until Round 3.1's adaptive batching provides a stable adaptation framework to build on top of."

#### deferred-rdma-investigation
**Source**: PROJECT_STATE_ASSESSMENT.md §3
**Specificity**: medium

> "Post-Release: RDMA/RoCE investigation (control-plane negotiation, transport abstraction)."

#### deferred-refs-privilege
**Source**: PROJECT_STATE_ASSESSMENT.md §3, §5
**Specificity**: medium

> "ReFS block clone SeManageVolumePrivilege investigation on Windows" — currently falls back to CopyFileExW (~0.6s vs ~0.17s robocopy for 4 GiB).

#### deferred-structured-logging
**Source**: PROJECT_STATE_ASSESSMENT.md §3 ("Nice-to-Have")
**Specificity**: medium

> "Full structured logging migration (eprintln → log macros across ~50 sites)"

#### deferred-fsevents-verification
**Source**: WORKFLOW_PHASE_3.md §3.3.7
**Specificity**: high

> "⚠️ 2025-10-25 – snapshot capture landed (`MacSnapshot` stores FSID/event ID/mtime); macOS verification run pending (`scripts/macos/run-journal-fastpath.sh`)."

#### deferred-benchmark-10gbe
**Source**: PROJECT_STATE_ASSESSMENT.md §3, WORKFLOW_PHASE_4.md §4.3.3
**Specificity**: high

> "Benchmark TCP data plane throughput targeting 10+ Gbps per stream; Benchmark remote fallback + data-plane streaming (sub-second first-byte); Capture remote benchmark runs (TCP vs gRPC fallback) and log results."

#### deferred-fs-capability-daemon-side-0.2.0
**Source**: WORKFLOW_PHASE_4.md §4.8.2, §4.8.3
**Specificity**: high

> "*(0.2.0)* Have `blit-daemon` probe during startup/idle windows and persist results per export … *(0.2.0)* Extend `blit diagnostics profile` to run local probes."

#### deferred-detach-mode
**Source**: REMOTE_REMOTE_DELEGATION_PLAN.md §9, §5 Phase 4
**Specificity**: high

> "`--detach` mode where CLI exits and dst continues. Track as separate future feature." Out of scope; sync delegation only.

#### deferred-blit-auth-passthrough-wiring
**Source**: REMOTE_REMOTE_DELEGATION_PLAN.md §5 Phase 4, §4.3.6
**Specificity**: high

> "`RemoteSourceLocator.delegated_credential` honored end-to-end once `BlitAuth` becomes real. Track as separate task; this plan does not block on it." (Later: BlitAuth removed from scope entirely.)

#### deferred-live-bench-remote-remote
**Source**: REMOTE_REMOTE_DELEGATION_PLAN.md §5 Phase 3 #6
**Specificity**: high

> "[ ] Run the benchmark on the target network and fill `docs/perf/remote_remote_benchmarks.md` with real results."

---

### rejected

#### rejected-tar-shard-status-quo
**Source**: POST_REVIEW_FIXES.md §1.2
**Specificity**: high

> "the current state of 'this exists in two places' is wrong." — TarShardExecutor duplicates BufferPool; refactor or document.

#### rejected-blit-auth-scope
**Source**: REMOTE_REMOTE_DELEGATION_PLAN.md auth note 2026-05-13; WORKFLOW_PHASE_4.md §4.9 by analogy
**Specificity**: high

> "`BlitAuth` and the `RemoteSourceLocator.delegated_credential` passthrough field were removed from project scope."

#### rejected-ai-telemetry-feature
**Source**: WORKFLOW_PHASE_4.md §4.9
**Specificity**: high

> "Telemetry Intelligence Exploration — Removed from project scope. ~~Scope optional AI-powered telemetry analysis~~, ~~Prototype `blit diagnose --ai`~~, ~~Document policy~~ — all removed."

#### rejected-keep-status-quo-relay-default
**Source**: REMOTE_REMOTE_DELEGATION_PLAN.md §3 Option D
**Specificity**: high

> "Keep status quo (rejected per user direction)."

#### rejected-pull-delegation
**Source**: REMOTE_REMOTE_DELEGATION_PLAN.md §3 Option B
**Specificity**: high

> "Pull delegation (rejected)." A would need write-side scheduling and diffing logic it doesn't carry.

#### rejected-co-orchestration-rendezvous
**Source**: REMOTE_REMOTE_DELEGATION_PLAN.md §3 Option C
**Specificity**: high

> "Co-orchestration with TCP rendezvous (rejected). Two control-plane streams to manage, two error surfaces; 'dst as initiator' is strictly simpler."

#### rejected-silent-auto-fallback
**Source**: REMOTE_REMOTE_DELEGATION_PLAN.md §4.4–4.5 + R21-F5
**Specificity**: high

> "Removed silent auto-fallback on `Unimplemented` / `Unavailable` / `CONNECT_SOURCE`. `--relay-via-cli` is now the only fallback (R21-F5)."

#### rejected-filtered-sink
**Source**: REMOTE_REMOTE_DELEGATION_PLAN.md §4.2 + R21-F3
**Specificity**: high

> "Daemon handler reuses `RemotePullClient::pull_sync`; removed the nonexistent `FilteredSink` (R21-F3). There is no `FilteredSink` (filters are source-side via `FilteredSource`)."

#### rejected-wildcard-allowlist
**Source**: REMOTE_REMOTE_DELEGATION_PLAN.md §4.3.3 #3
**Specificity**: high

> "No wildcards in 0.1.0 (suffix matchers would be a footgun against `evil-server-a.lan`-style typos)."

#### rejected-validate-then-reconstruct
**Source**: REMOTE_REMOTE_DELEGATION_PLAN.md §4.2 + R23-F1
**Specificity**: high

> "Daemon handler calls a new `RemotePullClient::pull_sync_with_spec` that accepts a pre-built `TransferOperationSpec` and forwards it unchanged. … This removes the 'validate-then-reconstruct' drift surface (R23-F1)."

---

### decision

#### decision-r21-f1-embed-spec
**Source**: REMOTE_REMOTE_DELEGATION_PLAN.md "v2 changes vs v1"
**Specificity**: high

> "R21-F1: `DelegatedPullRequest` now embeds existing `TransferOperationSpec` instead of duplicating fields."

#### decision-r21-f2-delegation-gate
**Source**: REMOTE_REMOTE_DELEGATION_PLAN.md "v2 changes vs v1" §4.3
**Specificity**: high

> "R21-F2: New 'Delegation gate' subsection: default-disabled config flag with host allowlist."

#### decision-r21-f3-reuse-pull-sync
**Source**: REMOTE_REMOTE_DELEGATION_PLAN.md "v2 changes vs v1"
**Specificity**: high

> "R21-F3: Daemon handler reuses `RemotePullClient::pull_sync`; removed the nonexistent `FilteredSink`."

#### decision-r21-f4-spec-version-compat
**Source**: REMOTE_REMOTE_DELEGATION_PLAN.md "v2 changes vs v1" §7
**Specificity**: high

> "R21-F4 + R21-F6: Removed FilterSpec proto-to-be-added line; clarified spec_version/capabilities is the compatibility model."

#### decision-r21-f5-no-silent-fallback
**Source**: REMOTE_REMOTE_DELEGATION_PLAN.md "v2 changes vs v1" §4.4–4.5
**Specificity**: high

> "R21-F5: Removed silent auto-fallback on `Unimplemented` / `Unavailable` / `CONNECT_SOURCE`. `--relay-via-cli` is now the only fallback."

#### decision-r21-f7-cli-byte-counter
**Source**: REMOTE_REMOTE_DELEGATION_PLAN.md "v2 changes vs v1" §6
**Specificity**: high

> "R21-F7: Byte-path-isolation test redesigned with two independent observables; `negotiated_endpoint` is informational only."

#### decision-r23-f1-pull-sync-with-spec
**Source**: REMOTE_REMOTE_DELEGATION_PLAN.md "v3 changes vs v2"
**Specificity**: high

> "R23-F1: Daemon handler calls a new `RemotePullClient::pull_sync_with_spec` that accepts a pre-built `TransferOperationSpec` and forwards it unchanged."

#### decision-r23-f2-gate-ordering
**Source**: REMOTE_REMOTE_DELEGATION_PLAN.md "v3 changes vs v2"
**Specificity**: high

> "R23-F2: Delegation gate ordering specified explicitly: source locator parse → daemon-wide gate → module metadata lookup → per-module override → path resolution + F2 containment → outbound connect."

#### decision-r23-f3-allowlist-semantics
**Source**: REMOTE_REMOTE_DELEGATION_PLAN.md "v3 changes vs v2"
**Specificity**: high

> "R23-F3: `allowed_source_hosts` matching semantics specified: hostname case/dot/punycode normalization, CIDR support, all-resolved-addresses-must-pass, IPv6 normalization, DNS-rebinding mitigation via resolve-once-connect-by-IP."

#### decision-r23-f4-source-peer-diagnostic
**Source**: REMOTE_REMOTE_DELEGATION_PLAN.md "v3 changes vs v2"
**Specificity**: high

> "R23-F4: `source_peer_observed` demoted to diagnostic. CLI-side byte counter is the load-bearing byte-path-isolation assertion."

#### decision-r25-f1-endpoint-isolation
**Source**: REMOTE_REMOTE_DELEGATION_PLAN.md "v4 changes vs v3"
**Specificity**: high

> "R25-F1: `pull_sync_with_spec` extraction widened. Spec construction in `pull.rs` includes the endpoint→spec field mapping at lines 397–409 (module + source_path), not just lines 433–484. The new `pull_sync_with_spec` MUST NOT read `self.endpoint.path`; the endpoint is transport-only."

#### decision-r25-f2-client-capabilities-override
**Source**: REMOTE_REMOTE_DELEGATION_PLAN.md "v4 changes vs v3"
**Specificity**: high

> "R25-F2: `client_capabilities` is the one field where CLI-supplied values are non-authoritative. The destination handler mandatorily replaces the field with its own `PeerCapabilities` before forwarding the spec to src, because the byte recipient in delegation is the dst, not the CLI."

#### decision-r25-f3-loopback-ip-form
**Source**: REMOTE_REMOTE_DELEGATION_PLAN.md "v4 changes vs v3" §4.3.3
**Specificity**: high

> "R25-F3: Loopback / link-local / unique-local resolved addresses require an IP- or CIDR-form allowlist entry; a hostname-form match alone does not authorize them. Closes the SSRF-via-DNS pivot."

#### decision-design-option-a-push-delegation
**Source**: REMOTE_REMOTE_DELEGATION_PLAN.md §3 Option A, §8
**Specificity**: high

> "Push delegation (recommended) — CLI sends a delegation request to dst (B). B becomes the initiator: it opens a `RemotePullClient` to A and runs the universal pipeline against itself as the filesystem sink. … Why dst, not src: every existing diff/manifest path on Blit lives on the destination side."

#### decision-f13-remove-use-chroot
**Source**: WORKFLOW_PHASE_3.md §3.2.1
**Specificity**: high

> "(`use_chroot` field removed in F13 / 2026-05-02; containment is now always-on via F2 canonical-path enforcement.)"

#### decision-perf-history-toggle-tbd-by-benchmarks
**Source**: WORKFLOW_PHASE_2.md §2.2.5 note
**Specificity**: medium

> "Final release toggle (enabled by default vs. opt-in) will be decided from benchmark evidence; once committed, the setting remains stable across releases. Implementation must avoid environment-variable configuration."

#### decision-1.2-delete-or-document
**Source**: POST_REVIEW_FIXES.md §1.2
**Specificity**: high

> "Delete `TarShardExecutor` (or document why it stays). … If for some reason the gRPC fallback can't use the unified path (unlikely — it's just bytes-then-extract), document why in a comment and keep the executor. Either way, the current state of 'this exists in two places' is wrong." (Outcome per Round 1 closure: deferred with docstring.)

#### decision-keep-remote-transfer-source-as-relay-primitive
**Source**: REMOTE_REMOTE_DELEGATION_PLAN.md §4.2 "Core side"
**Specificity**: high

> "`RemoteTransferSource` … is the relay primitive used by the legacy CLI-relay path. Keep it: the `--relay-via-cli` escape hatch still uses it, and removing it would remove the explicit fallback. Document in the module docstring that this is the relay primitive, not the default remote→remote path."

---

## Contradictions

### contradiction-tar-shard-executor-status
The two docs disagree on whether TarShardExecutor is still wanted:
- WORKFLOW_PHASE_3.md §3.4 Status 2025-10-28 advertises "the daemon side now parallelises tar-shard unpacking (`TarShardExecutor`, four blocking workers)" as a positive shipped behavior.
- POST_REVIEW_FIXES.md §1.2 reports that after Phase 5 of receive-pipeline unification, "TarShardExecutor is now used only by the gRPC fallback path" and calls "the current state of 'this exists in two places' wrong." (Resolution: §1.2 was explicitly deferred 2026-05-05 with a docstring rather than deleted, so the disagreement is now a known deferral, but the phase-3 doc still reads as celebratory while POST_REVIEW marks it for removal.)

### contradiction-blit-utils-binary
- WORKFLOW_PHASE_3.md and WORKFLOW_PHASE_4.md scope `blit-utils <verb>` as a separate binary at the time the phases were written.
- Headers added 2026-05-05 on both files plus PROJECT_STATE_ASSESSMENT.md §2 explicitly say "the admin verbs ship as subcommands of the single `blit` binary; no separate `blit-utils` artifact." (Resolution: phase docs explicitly marked as historical, but the body text still refers to `blit-utils` and `crates/blit-utils/src/main.rs` paths.)

### contradiction-use-chroot-config
- WORKFLOW_PHASE_3.md §3.2.1 originally described daemon settings including a `use_chroot` field as part of the modules/daemon TOML.
- Same line, updated: "(`use_chroot` field removed in F13 / 2026-05-02; containment is now always-on via F2 canonical-path enforcement.)" — the field still appears in the original phase text but the inline note says it was removed.

### contradiction-blit-auth-design-presence
- REMOTE_REMOTE_DELEGATION_PLAN.md body (§4.3.6, §4.1, §8) describes a `BlitAuth` flow and `delegated_credential` passthrough.
- The doc's own top "Auth note (2026-05-13)" says BlitAuth and `delegated_credential` are "removed from project scope. Mentions of either in the body below … are now obsolete." (Resolution: explicitly flagged in the header — kept verbatim "as a historical design record" — but readers must remember the body is no longer authoritative.)

### contradiction-perf-history-toggle-decision
- WORKFLOW_PHASE_2.md §"Guiding Principles" #2 says "Telemetry stays local — JSONL log under config dir, capped to ~1 MiB. Opt-out should be driven by CLI/config settings."
- §2.2.5 note says "Final release toggle (enabled by default vs. opt-in) will be decided from benchmark evidence" — leaving the default undecided in this doc, even though §1 already implies an opt-out (default-on) shape.

### contradiction-fs-capability-shipped-vs-scoped-out
- PROJECT_STATE_ASSESSMENT.md §2 lists "Filesystem capability probing for 12+ FS types, cached per device ID" as Done.
- WORKFLOW_PHASE_4.md §4.8 scope note says only 4.8.1 (client-side cache + probes) ships in 0.1.0; 4.8.2 (daemon probing) and 4.8.3 (`blit diagnostics profile` integration) deferred to 0.2.0. (Resolution: §2's Done statement appears to refer to the client-side cache shipped per 4.8.1; not strictly contradictory, but easy to misread.)

### contradiction-project-assessment-superseded
- PROJECT_STATE_ASSESSMENT.md is explicitly stamped "Superseded" at the top — predates Phase 4.7 and 4.8.1, lists `blit-utils` as a separate artifact. Body still asserts "feature-complete" and "All phases through Phase 4 done." For audit purposes treat this whole doc as advisory historical, not current.

---

## Coverage attestation

| File | Lines | Notes |
|---|---:|---|
| docs/plan/POST_REVIEW_FIXES.md | 308 | Read 1–308 in full |
| docs/plan/REMOTE_REMOTE_DELEGATION_PLAN.md | 1084 | Read in three passes (1–400, 400–800, 800–1085); covered all sections including v1→v4 change notes, §3 Options A–D, §4.1–4.5, §5 Phases 1–4, §6 test strategy, §7 risks, §8 decision summary, §9 out-of-scope |
| docs/plan/WORKFLOW_PHASE_2.md | 100 | Read 1–100 in full |
| docs/plan/WORKFLOW_PHASE_3.md | 127 | Read 1–127 in full including all status notes |
| docs/plan/WORKFLOW_PHASE_4.md | 130 | Read 1–130 in full incl. §4.8 scope note + §4.9 removed-from-scope |
| docs/plan/PROJECT_STATE_ASSESSMENT.md | 180 | Read 1–180 in full incl. Superseded header |

**Total lines**: 1929
