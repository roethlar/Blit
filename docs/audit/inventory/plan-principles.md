# Plan Inventory: Core principles & release scope

**Generated**: 2026-06-04 by audit workflow
**Coverage**:
- `docs/plan/greenfield_plan_v6.md` — 442 lines (full read)
- `docs/plan/MASTER_WORKFLOW.md` — 120 lines (full read)
- `docs/plan/RELEASE_PLAN_v2_2026-05-04.md` — 805 lines (full read)
- **Total lines read**: 1367

The greenfield plan file contains v4 (lines 1–141), v5 (lines 143–308), and v6 (lines 310–442) concatenated; each is marked Superseded by the next. Claims are tagged with the version that asserts them so the supersession order is preserved.

---

## Claims (grouped by category)

### principle

#### principle-fast-v5
**Source**: greenfield_plan_v6.md §1.4 (v5 "Inviolable Principles") / line 171
**Specificity**: medium

> **FAST**: Start copying immediately, minimise perceived latency.

#### principle-simple-v5
**Source**: greenfield_plan_v6.md §1.4 (v5 "Inviolable Principles") / line 172
**Specificity**: high

> **SIMPLE**: No user tunables for speed; planner chooses the best path automatically.

#### principle-reliable-v5
**Source**: greenfield_plan_v6.md §1.4 (v5 "Inviolable Principles") / line 173
**Specificity**: medium

> **RELIABLE**: Mirror deletions, checksums, and correctness outweigh speed.

#### principle-private-v5
**Source**: greenfield_plan_v6.md §1.4 (v5 "Inviolable Principles") / line 174
**Specificity**: high

> **PRIVATE**: No external telemetry; user data never leaves the machine.

#### principle-fast-master
**Source**: MASTER_WORKFLOW.md §1 Core Delivery Principles / line 12
**Specificity**: high

> **FAST** – Transfers begin immediately; planner keeps perceived latency ≤ 1 s.

#### principle-simple-master
**Source**: MASTER_WORKFLOW.md §1 Core Delivery Principles / line 13
**Specificity**: high

> **SIMPLE** – No user-facing speed knobs. Planner, orchestrator, and heuristics own performance.

#### principle-reliable-master
**Source**: MASTER_WORKFLOW.md §1 Core Delivery Principles / line 14
**Specificity**: medium

> **RELIABLE** – Correctness beats speed. Mirror deletions, checksums, and failure handling remain uncompromised.

#### principle-private-master
**Source**: MASTER_WORKFLOW.md §1 Core Delivery Principles / line 15
**Specificity**: high

> **PRIVATE** – Metrics stay local; no external telemetry.

#### principle-fast-simple-reliable-private-v6
**Source**: greenfield_plan_v6.md §1.2 (v6 Guiding Principles) / line 320
**Specificity**: high

> **FAST / SIMPLE / RELIABLE / PRIVATE** – Same non-negotiables as v5: planner auto-tunes, no user speed knobs, correctness outweighs raw throughput, and metrics never leave the machine.

#### principle-deliver-features-v6
**Source**: greenfield_plan_v6.md §1.1 (v6 Guiding Principles) / line 319
**Specificity**: medium

> **Deliver the Needed Features** – Ensure the CLI, daemon, and utilities expose the commands and behaviours the project relies on (copy/mirror/move, remote discovery, module management, admin tooling). These are functional requirements, not a promise of backward compatibility.

#### principle-transport-evolution-v6
**Source**: greenfield_plan_v6.md §1.3 (v6 Guiding Principles) / line 321
**Specificity**: medium

> **Transport Evolution** – Hybrid TCP, automatic gRPC fallback, and future RDMA remain core differentiators, layered once the required feature set is present.

#### principle-clarity-over-legacy-v6
**Source**: greenfield_plan_v6.md §1.4 (v6 Guiding Principles) / line 322
**Specificity**: medium

> **Clarity Over Legacy** – Document what v2 provides; references to v1 exist only for historical context.

#### principle-evidence-over-doc-claims
**Source**: RELEASE_PLAN_v2_2026-05-04.md §9 Methodology / line 791
**Specificity**: medium

> Where the two audits disagreed ..., the more rigorous reading wins: ship-state must be verifiable against running code, not just file presence.

---

### invariant

#### invariant-no-speed-flags-v5
**Source**: greenfield_plan_v6.md §1.1 v5 / line 156
**Specificity**: high

> No user speed flags (`--ludicrous-speed` is deprecated); buffers/workers auto-tuned.

#### invariant-no-reintroduce-deprecated-flags
**Source**: greenfield_plan_v6.md §5 v5 Non-Negotiables item 2 / line 290
**Specificity**: high

> Never reintroduce deprecated flags (`--mir`, `--ludicrous-speed`) as behaviour toggles.

#### invariant-no-new-perf-tunables
**Source**: greenfield_plan_v6.md §5 v5 Non-Negotiables item 3 / line 291
**Specificity**: high

> Do not add user-facing performance tunables unless explicitly approved.

#### invariant-no-external-telemetry-without-signoff
**Source**: greenfield_plan_v6.md §5 v5 Non-Negotiables item 4 / line 292
**Specificity**: high

> Telemetry stays local; no remote logging without signed-off design change.

#### invariant-doc-devlog-update
**Source**: greenfield_plan_v6.md §5 v5 Non-Negotiables item 5 / line 293
**Specificity**: medium

> Every change must update relevant docs + DEVLOG to survive context resets.

#### invariant-respect-principles-always
**Source**: greenfield_plan_v6.md §5 v5 Non-Negotiables item 1 / line 289
**Specificity**: medium

> Respect the FAST/SIMPLE/RELIABLE/PRIVATE principles at all times.

#### invariant-canonical-containment-always-on
**Source**: greenfield_plan_v6.md §2 v6 Module Configuration / line 346
**Specificity**: high

> Enforce read-only modules and always-on canonical-path containment (F2) for every remote operation. Containment is not a per-module opt-in; symlinks inside a module that resolve outside the module root are refused by the daemon.

#### invariant-no-env-vars-for-config
**Source**: MASTER_WORKFLOW.md §3 Decision Log / line 75
**Specificity**: high

> Environment variables | ✅ Not used for configuration; precedence is CLI flag → config file

#### invariant-config-precedence-cli-then-file
**Source**: greenfield_plan_v6.md §5 v6 Open Questions / line 432
**Specificity**: high

> Config search order | Confirm precedence (CLI flag → config). No environment variables.

#### invariant-telemetry-on-device-only
**Source**: greenfield_plan_v6.md §1.3 v5 Telemetry / line 164
**Specificity**: high

> **Telemetry & Diagnostics** — All metrics stay on-device

#### invariant-no-network-by-default-scripts
**Source**: greenfield_plan_v6.md §4 v5 Tooling & Logging / line 284
**Specificity**: medium

> All scripts/configs default to no network access; explicit callouts required otherwise.

#### invariant-fast-target-1s-perceived-latency
**Source**: greenfield_plan_v6.md §1.1 v5 / line 154
**Specificity**: high

> Adaptive predictor fed by local telemetry to keep perceived latency ≤ 1 s.

#### invariant-no-backward-compat-with-v1
**Source**: greenfield_plan_v6.md v6 header / line 313
**Specificity**: high

> The focus is shipping the required features; backward compatibility with v1 is not a goal.

#### invariant-no-migration-guide
**Source**: greenfield_plan_v6.md §2 v6 Documentation & Tests / line 366
**Specificity**: medium

> Update CLI help/man pages to reflect the command set and remote syntax. No migration guide—documentation describes v2 only.

#### invariant-no-tech-debt-for-back-compat
**Source**: RELEASE_PLAN_v2_2026-05-04.md §2.8 / line 436
**Specificity**: high

> Silent dead code is incompatible with the "no tech debt for the sake of backwards compatibility" release directive.

---

### interface

#### iface-hybrid-transport-grpc-control-tcp-data-v4
**Source**: greenfield_plan_v6.md §1 v4 Architecture / lines 20–25
**Specificity**: high

> **Control Plane (gRPC):** All negotiation, metadata exchange, and commands (manifests, file lists, purge requests, progress) will be handled over a standard gRPC connection. ... **Data Plane (Raw TCP):** For the actual bulk transfer of large files, the control plane will negotiate a separate, short-lived, raw TCP connection.

#### iface-hybrid-transport-v5
**Source**: greenfield_plan_v6.md §1.2 v5 Hybrid Remote Transport / lines 158–162
**Specificity**: high

> gRPC control plane for manifests, negotiations, progress, and purge/list operations. Raw TCP data plane negotiated via one-time, cryptographically strong token for bulk transfers (zero-copy on Linux via `sendfile`, `copy_file_range`, `splice`).

#### iface-grpc-fallback-mandatory
**Source**: greenfield_plan_v6.md §2 v6 Remote Services / line 359
**Specificity**: high

> Automatic gRPC fallback for data transfers is mandatory; CLI prints a warning but continues.

#### iface-force-grpc-data-override
**Source**: greenfield_plan_v6.md §1.2 v5 / line 161
**Specificity**: high

> Automatic fallback to gRPC-streamed data when the negotiated TCP port cannot be reached (firewall/NAT); surface as a warning and continue, with an advanced `--force-grpc-data`/`BLIT_FORCE_GRPC_DATA=1` override for locked-down environments.

#### iface-cli-verbs-v6
**Source**: greenfield_plan_v6.md §2 v6 CLI & Remote Semantics / line 329
**Specificity**: high

> Replace the current `push`/`pull` model with the required command set: `copy`, `mirror`, `move`, `scan`, `list`, plus diagnostics.

#### iface-cli-verbs-master
**Source**: MASTER_WORKFLOW.md §1 Feature-Completeness Goals / line 18
**Specificity**: high

> CLI verbs: `copy`, `mirror`, `move`, `scan`, `list`, diagnostics.

#### iface-admin-subcommands-master
**Source**: MASTER_WORKFLOW.md §1 Feature-Completeness Goals / line 20
**Specificity**: high

> Admin subcommands on `blit`: `scan`, `ls`, `list`, `list-modules`, `rm`, `find`, `du`, `df`, `completions`, `profile` (admin verbs were merged into the main binary; there is no separate `blit-utils`).

#### iface-remote-syntax-canonical
**Source**: greenfield_plan_v6.md §2 v6 / lines 331–337
**Specificity**: high

> Adopt canonical remote syntax: `server:/module/` → root of a named module (must end with `/`). `server:/module/path` → path under the module root. `server://path` → default export. ... Bare `server` (optionally `:port`) → discovery (list modules). `server:/module` without a trailing slash is invalid (ambiguous) and should error.

#### iface-remote-syntax-master
**Source**: MASTER_WORKFLOW.md §1 Feature-Completeness Goals / line 19
**Specificity**: high

> Remote syntax: `server:/module/...`, `server://...`, discovery on bare host.

#### iface-default-port-9031
**Source**: greenfield_plan_v6.md §2 v6 / line 338
**Specificity**: high

> Default remote port is 9031; allow overrides via `server:port/...` and CLI flags.

#### iface-move-semantics
**Source**: greenfield_plan_v6.md §2 v6 / line 339
**Specificity**: high

> `move` performs a mirror followed by source removal (local or remote).

#### iface-default-config-path
**Source**: greenfield_plan_v6.md §2 v6 / line 341
**Specificity**: high

> Load module definitions from a TOML config (`/etc/blit/config.toml` by default) with fields: `name`, `path`, `comment`, `read_only`, and daemon-level settings `bind`, `port`, `motd`, `no_mdns`, `mdns_name`.

#### iface-daemon-flags-expose
**Source**: greenfield_plan_v6.md §2 v6 / line 342
**Specificity**: high

> Expose flags such as `--config`, `--bind`, `--port`, `--root`, `--no-mdns`, `--mdns-name`.

#### iface-no-modules-defaults
**Source**: greenfield_plan_v6.md §2 v6 / lines 343–345
**Specificity**: high

> Behaviour when no modules are defined: If `--root` is provided (or the config defines a default root), expose it via `server://`. Otherwise `server://` resolves to the daemon's working directory, matching historical behaviour. Log a warning so operators know they are running with an implicit root export.

#### iface-utility-subcommands-v6
**Source**: greenfield_plan_v6.md §2 v6 Discovery & Admin Utilities / line 350
**Specificity**: high

> Implement subcommands: `scan`, `ls`, `list`, `rm`, `find`, `du`, `df`, `completions`, and a `profile` command for local performance capture.

#### iface-destructive-confirm
**Source**: greenfield_plan_v6.md §2 v6 / line 352
**Specificity**: high

> Destructive operations (`rm` and any future destructive verbs) require confirmation unless `--yes` is supplied.

#### iface-mdns-advertise-by-default
**Source**: greenfield_plan_v6.md §2 v6 / line 361
**Specificity**: high

> Advertise `_blit._tcp.local.` via mDNS by default; provide opt-out (`--no-mdns`) and custom instance name (`--mdns-name`).

#### iface-mdns-platform-coverage
**Source**: greenfield_plan_v6.md §2 v6 / line 363
**Specificity**: medium

> Confirm behaviour on Linux, macOS, and Windows.

#### iface-rpc-list-find-du-df-rm
**Source**: greenfield_plan_v6.md §2 v6 Remote Services / lines 355–356
**Specificity**: high

> Directory listing (for `list`/`ls`), recursive enumeration (`find`), space usage (`du`, `df`), and remote remove (`rm`).

#### iface-admin-rpcs-respect-modules
**Source**: greenfield_plan_v6.md §2 v6 / line 358
**Specificity**: high

> Administrative RPCs can remain gRPC-only but must honour module boundaries and read-only flags.

#### iface-pushsummary-transport-stats
**Source**: greenfield_plan_v6.md §2 v5 Protocol Updates / line 184
**Specificity**: medium

> Ensure `PushSummary` carries transport stats (bytes/sec, zero-copy usage) for diagnostics.

#### iface-perf-history-path
**Source**: greenfield_plan_v6.md §1.3 v5 / line 166
**Specificity**: high

> Capped JSONL log (`~/.config/blit/perf_local.jsonl`) storing workload signature, planner/copy durations, stall events.

#### iface-blit-diagnostics-perf
**Source**: greenfield_plan_v6.md §1.3 v5 / line 167
**Specificity**: high

> `blit diagnostics perf` surfaces recent runs for troubleshooting.

#### iface-disable-local-telemetry-env
**Source**: greenfield_plan_v6.md §1.3 v5 / line 168
**Specificity**: high

> `BLIT_DISABLE_LOCAL_TELEMETRY=1` opt-out for debugging.

#### iface-token-cryptographic
**Source**: greenfield_plan_v6.md §3 v5 Phase 3 item 1 / line 243
**Specificity**: high

> Token must be cryptographically strong (e.g., signed JWT with nonce/expiry) and bound to the accepted socket to prevent replay.

#### iface-tls-control-plane
**Source**: greenfield_plan_v6.md §3 v5 Phase 4 / line 262
**Specificity**: medium

> TLS for control plane (and optionally data plane via STARTTLS-style negotiation).

#### iface-rpc-pingpong-skeleton
**Source**: greenfield_plan_v6.md §3 v5 Phase 1 / line 204
**Specificity**: medium

> Add integration test to prove Ping/Pong RPC and negotiation message wiring.

#### iface-no-blit-utils-binary
**Source**: greenfield_plan_v6.md implementation note / lines 7–16
**Specificity**: high

> the as-shipped surface is `blit <subcommand>` built from `crates/blit-cli`

#### iface-cli-shipped-surface
**Source**: RELEASE_PLAN_v2_2026-05-04.md §1 CLI surface / lines 150–155
**Specificity**: high

> `blit copy / mirror / move / scan / list-modules / ls / list / find / du / df / rm / completions / profile / check / diagnostics`. `blit-daemon` config file (TOML), CLI overrides, `[delegation]` block, mDNS, motd.

#### iface-completions-split-shipped
**Source**: RELEASE_PLAN_v2_2026-05-04.md §2.5 / lines 306–316
**Specificity**: high

> The completions subcommand split into two forms: `blit completions shell <SHELL>` writes a clap_complete-generated bash/zsh/fish/powershell/elvish script to stdout; `blit completions remote <REMOTE> [--prefix <STR>] [--files] [--dirs]` keeps the daemon-backed `CompletePath` RPC form.

#### iface-find-pattern-glob
**Source**: RELEASE_PLAN_v2_2026-05-04.md §2.4 / lines 284–303
**Specificity**: high

> Implement glob via `globset` (already a workspace dep). Substring users can still match with `*foo*`. Keep the existing `--pattern` flag name; semantics change.

#### iface-blit-list-smart-dispatch
**Source**: RELEASE_PLAN_v2_2026-05-04.md §2.3 / lines 269–276
**Specificity**: high

> Make `blit list` smart-dispatch. If the target parses as a bare host (no module, no path), treat it as `list-modules`. If the target has a path, treat it as `ls`.

#### iface-binary-name-blit
**Source**: RELEASE_PLAN_v2_2026-05-04.md §2.1 / lines 186–193
**Specificity**: high

> Crate name stayed `blit-cli`; binary produced is named `blit` via `[[bin]] name = "blit"`.

#### iface-metrics-stderr-summary
**Source**: RELEASE_PLAN_v2_2026-05-04.md §3.1 / lines 496–519
**Specificity**: high

> the daemon now emits `[metrics] {op} {ok|err} in {dt} (push_ops=N pull_ops=N purge_ops=N active=N errors=N)` lines on push / pull / pull_sync / delegated_pull / purge completion when `--metrics` is on.

#### iface-mdns-txt-fields-shipped
**Source**: RELEASE_PLAN_v2_2026-05-04.md §3.2 / lines 538–547
**Specificity**: high

> `_blit._tcp.local.` advertisements now carry `module_count` (authoritative count even when the `modules` list is truncated past the ~180-byte TXT cap) and `delegation_enabled` (`1`/`0`, whether the daemon accepts `DelegatedPull` requests).

#### iface-fs-capability-client-side-only-010
**Source**: RELEASE_PLAN_v2_2026-05-04.md §3.3 / lines 568–574
**Specificity**: high

> 0.1.0 ships client-side capability probing only (`blit diagnostics dump`); daemon-startup and idle-probe mechanisms (`WORKFLOW_PHASE_4.md` §4.8.2 / §4.8.3) are explicitly out of 0.1.0 scope.

---

### behavior

#### behavior-streaming-orchestrator-heartbeat
**Source**: greenfield_plan_v6.md §1.1 v5 / line 152
**Specificity**: high

> Incremental planner that emits work every heartbeat (1 s default, 500 ms when workers are starved).

#### behavior-stall-detector-10s
**Source**: greenfield_plan_v6.md §1.1 v5 / line 153
**Specificity**: high

> 10 s stall detector (planner *and* workers idle) with precise error reporting.

#### behavior-fast-path-trivial-huge
**Source**: greenfield_plan_v6.md §1.1 v5 / line 154
**Specificity**: medium

> Automatic fast-paths for trivial workloads and huge single files.

#### behavior-trust-model-operator-controls
**Source**: RELEASE_PLAN_v2_2026-05-04.md §5.2 / lines 646–650
**Specificity**: high

> Auth is out of project scope — the trust model is "operator network controls" (firewall / VPN / SSH tunnel). If authentication is ever needed, design from scratch rather than retaining a misleading stub.

#### behavior-cli-quiet-default
**Source**: MASTER_WORKFLOW.md §3 Decision Log / line 73
**Specificity**: high

> Progress UX | ✅ CLI quiet by default; structured events exposed for GUIs/debug

#### behavior-progress-spinner-throughput-eta
**Source**: greenfield_plan_v6.md §2 v5 Phase 2 / line 222
**Specificity**: medium

> Introduce unified progress indicator (spinner + throughput + ETA) for copy/mirror.

#### behavior-no-silent-fallback-delegation
**Source**: RELEASE_PLAN_v2_2026-05-04.md §1 Remote / line 142
**Specificity**: high

> No-silent-fallback CLI dispatch.

#### behavior-detach-not-supported
**Source**: RELEASE_PLAN_v2_2026-05-04.md §5.6 / lines 672–675
**Specificity**: high

> CLI Ctrl-C abort is the current contract. `--detach` would require a job-tracking RPC and durable state on the daemon. Out of scope.

#### behavior-network-tuning-nagle-buffers
**Source**: greenfield_plan_v6.md §3 v5 Phase 3 item 4 / line 247
**Specificity**: medium

> Network tuning: disable Nagle, set large send/recv buffers, optional BBR hints.

#### behavior-progress-piped-back
**Source**: greenfield_plan_v6.md §3 v5 Phase 3 item 5 / line 248
**Specificity**: medium

> Progress signals piped back to CLI from remote operations.

---

### scope

#### scope-feature-completeness-goals-master
**Source**: MASTER_WORKFLOW.md §1 / lines 17–22
**Specificity**: high

> CLI verbs: `copy`, `mirror`, `move`, `scan`, `list`, diagnostics. Remote syntax: `server:/module/...`, `server://...`, discovery on bare host. Admin subcommands on `blit`: `scan`, `ls`, `list`, `list-modules`, `rm`, `find`, `du`, `df`, `completions`, `profile`. Daemon configuration via TOML modules + optional `--root`; mDNS advertised unless disabled. Hybrid transport (gRPC control plane + TCP data plane with secure tokens and gRPC fallback).

#### scope-deliverables-checklist-v6
**Source**: greenfield_plan_v6.md §4 v6 Deliverables Checklist / lines 415–424
**Specificity**: high

> [ ] CLI (`copy`, `mirror`, `move`, `scan`, `list`, diagnostics) operational with canonical remote syntax. [ ] `blit-core::remote::endpoint` parses ... [ ] `blit-daemon` loads modules/root from config, supports flags, advertises via mDNS, enforces per-module `read_only` + always-on canonical-path containment (F2), and handles "no exports configured" cleanly. [ ] RPC surface supports `list`, `find`, `du`, `df`, `rm`. [ ] `blit-utils` implements `scan`, `ls`, `list`, `rm`, `find`, `du`, `df`, `completions`, `profile` command. [ ] Test suite covers transfer permutations, admin workflows, and daemon startup scenarios. [ ] Documentation (CLI, blit-daemon, blit-utils) reflects the updated feature set. [ ] Benchmarks include remote scenarios over TCP and gRPC fallback using the new commands.

#### scope-phase-map
**Source**: MASTER_WORKFLOW.md §2 Phase Map / lines 28–36
**Specificity**: high

Phase 0 Complete, Phase 1 Complete, Phase 2 (Streaming Orchestrator & Local Ops) gate = local copy/mirror/move fast & reliable; tests and benchmarks pass; Phase 2.5 gate = local workloads ≥ 95% v1 throughput; resource usage within budgets; Phase 3 gate = remote copy/mirror/move, scan, list/ls, find, du, df, rm, profile all succeed with fallback + error handling; Phase 4 gate = packages & installers, integration suite, docs complete; Phase 3.5 (RDMA) deferred until after core release.

#### scope-phase2-gate-detail
**Source**: MASTER_WORKFLOW.md §2 Detailed Gates / lines 39–43
**Specificity**: high

> blit copy / blit mirror / blit move (local) deliver within FAST target. Streaming planner heartbeat + stall detector operational. Predictor/history toggles documented; CLI remains quiet by default with verbose hooks. Unit/integration tests green.

#### scope-phase25-gate-95pct
**Source**: MASTER_WORKFLOW.md §2 Detailed Gates / lines 45–48
**Specificity**: high

> Benchmarks: large-file, many-small-files, mixed workloads, incremental mirrors. Throughput ≥ 95 % of v1; memory/cpu within budgets. Bench results logged (DEVLOG + Phase 2.5 doc).

#### scope-phase3-gate-detail
**Source**: MASTER_WORKFLOW.md §2 Detailed Gates / lines 50–56
**Specificity**: high

> Remote transfer verbs operate across the network with hybrid transport and gRPC fallback. `blit scan` discovers daemons via mDNS. `blit list` / `blit ls` enumerate modules and paths; forbid traversal outside exports. Admin verbs on `blit` (`scan`, `ls`, `list`, `list-modules`, `rm`, `find`, `du`, `df`, `completions`, `profile`) succeed against daemon with read-only modules and always-on canonical-path containment (F2). Structured progress events exist for future GUIs; CLI remains quiet unless verbose. Integration tests cover remote transfer + admin flows.

#### scope-phase4-gate-detail
**Source**: MASTER_WORKFLOW.md §2 Detailed Gates / lines 58–62
**Specificity**: high

> Packages built for supported platforms (Linux, macOS, Windows). Installation/configuration docs ready (daemon config, mDNS, service guidance). End-to-end test suite covers local and remote operations. Release checklist complete.

#### scope-v6-supersedes-v5
**Source**: greenfield_plan_v6.md v6 header / line 312
**Specificity**: high

> **Status**: Active (supersedes v5)

#### scope-v5-supersedes-v4
**Source**: greenfield_plan_v6.md v5 header / line 145
**Specificity**: high

> **Version**: 5.0 (Supersedes v4)

#### scope-archive-v5
**Source**: greenfield_plan_v6.md §6 v6 Next Steps / line 440
**Specificity**: medium

> Archive v5 as historical reference (pointer only).

#### scope-p0-only-blocks-release
**Source**: RELEASE_PLAN_v2_2026-05-04.md §2 / lines 181–185
**Specificity**: high

> Each item must close before tagging 0.1.0. Each has a recommended default; product owner can override.

#### scope-release-target-0.1.0
**Source**: RELEASE_PLAN_v2_2026-05-04.md header / line 3
**Specificity**: high

> **Status:** Authoritative for the 0.1.0 release. Supersedes `PROJECT_STATE_ASSESSMENT.md` (dated 2026-04-07, materially stale).

#### scope-tag-0.1.0-blocked-on-2.6
**Source**: RELEASE_PLAN_v2_2026-05-04.md §6 / line 721
**Specificity**: high

> Tag 0.1.0 | ⏳ Blocked on step 2 only — all P1 cleared

#### scope-decisions-required-d1-d9
**Source**: RELEASE_PLAN_v2_2026-05-04.md §7 / lines 729–747
**Specificity**: high

D1=binary name (default `blit`), D2=blit-utils standalone vs merged (default merged), D3=`blit list` semantics (default smart-dispatch), D4=mDNS TXT enrichment (default yes), D5=TransferMetrics scaffolding-or-remove (default keep+document; modified to wire), D6=Phase 4.8.2/4.8.3 daemon FS capability in 0.1.0 (default defer), D7=find --pattern glob vs substring (default glob), D8=shell completions clap_complete vs README edit (default clap_complete), D9=predictor wire vs delete (default wire).

#### scope-p0-items-eight
**Source**: RELEASE_PLAN_v2_2026-05-04.md v2.1 changes vs v2 / line 66
**Specificity**: high

> §6 commit sequence: revised for 8 P0 items (was 6). Total cost band shifts from 3-5 days to 4-7 days depending on predictor path.

---

### non-goal

#### nongoal-no-v1-backward-compat
**Source**: greenfield_plan_v6.md v6 header / line 313
**Specificity**: high

> backward compatibility with v1 is not a goal.

#### nongoal-no-user-speed-knobs
**Source**: greenfield_plan_v6.md §1.2 v6 / line 320
**Specificity**: high

> no user speed knobs

#### nongoal-no-external-telemetry
**Source**: greenfield_plan_v6.md §1.2 v6 / line 320
**Specificity**: high

> metrics never leave the machine

#### nongoal-no-env-var-configuration
**Source**: greenfield_plan_v6.md §5 v6 Open Questions / line 432
**Specificity**: high

> No environment variables.

#### nongoal-auth-out-of-scope
**Source**: RELEASE_PLAN_v2_2026-05-04.md §5.2 / lines 643–650
**Specificity**: high

> **Status:** Removed from scope, not deferred. The `BlitAuth` service stub and `RemoteSourceLocator.delegated_credential` forward-compat field were stripped from `proto/blit.proto`. Auth is out of project scope.

#### nongoal-no-ai-telemetry-analysis
**Source**: RELEASE_PLAN_v2_2026-05-04.md §5.4 / lines 656–664
**Specificity**: high

> **Status:** Removed from scope, not deferred. The scoping doc `docs/plan/AI_TELEMETRY_ANALYSIS.md` was deleted (owner decision). Performance history will continue to be collected for the predictor ... but no "analyze my history" feature is planned.

#### nongoal-no-final-qa-checklist-for-010
**Source**: RELEASE_PLAN_v2_2026-05-04.md §8 cross-reference / line 771
**Specificity**: medium

> Final QA checklist absent | GPT | §5.7 (acceptable for 0.1.0)

---

### shipped

#### shipped-core-engine-inventory
**Source**: RELEASE_PLAN_v2_2026-05-04.md §1 Core engine / lines 106–129
**Specificity**: high

> Universal `TransferOperationSpec` + `NormalizedTransferOperation::from_spec` validation chokepoint. Used by Push, PullSync, DelegatedPull. `DiffPlanner` + streaming planner with 10s stall detector and heartbeat scheduler. Fast-path routing (Tiny / Huge / NoWork). Always-on canonical-path containment (F2). `safe_join` and `verify_contained` chokepoints; per-module `canonical_root`. Tar-shard receive safety (R5-F2 / R6-F1 / R6-F3). Hybrid TCP data plane with one-time tokens; automatic gRPC fallback. Block-resume. F1 through F13 of 15 baseline review findings closed. F14 (FSEvents → `objc2-core-services`) closed. Change journals: Windows USN, macOS FSEvents, Linux metadata snapshot. Local performance history (`perf_history.rs`). Adaptive bucket-target tuning via `derive_local_plan_tuning`.

#### shipped-remote-features
**Source**: RELEASE_PLAN_v2_2026-05-04.md §1 Remote / lines 131–149
**Specificity**: high

> Push: control plane + bounded-channel manifest, NeedList, TCP data plane with parallel streams, gRPC fallback, force-grpc flag. Pull (PullSync): unified spec, filter parity, tar shards, delete list, checksum negotiation (F11/R15), gRPC fallback. Remote→remote delegation (`DelegatedPull`): Default direct path. `--relay-via-cli` operator escape hatch. Delegation gate (`[delegation]` config block, IDNA/CIDR/IP matching, R25-F3 special-range rule, DNS-rebinding mitigation, per-module override). No-silent-fallback CLI dispatch. Admin RPCs: ListModules, List, Find, DiskUsage, FilesystemStats, CompletePath, Purge.

#### shipped-cli-surface-final
**Source**: RELEASE_PLAN_v2_2026-05-04.md §1 CLI surface / lines 151–155
**Specificity**: high

> `blit copy / mirror / move / scan / list-modules / ls / list / find / du / df / rm / completions / profile / check / diagnostics`. `blit-daemon` config file (TOML), CLI overrides, `[delegation]` block, mDNS, motd.

#### shipped-documentation
**Source**: RELEASE_PLAN_v2_2026-05-04.md §1 Documentation / lines 157–163
**Specificity**: medium

> `docs/cli/blit.1.md` and `docs/cli/blit-daemon.1.md`. `docs/DAEMON_CONFIG.md` (extensive: trust model, containment, delegation, mDNS). `README.md`, `CHANGELOG.md`. Reviews: `codebase_review_2026-05-01.md` and 35+ followup rounds in `followup_review_2026-05-02.md`.

#### shipped-ci-build
**Source**: RELEASE_PLAN_v2_2026-05-04.md §1 CI / build / lines 165–169
**Specificity**: high

> Tri-platform CI: `cargo fmt --all -- --check`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test --workspace` on Linux, macOS, Windows. Release builds + artifact upload.

#### shipped-test-totals
**Source**: RELEASE_PLAN_v2_2026-05-04.md §1 / lines 171–178
**Specificity**: high

> Test totals: 383 workspace tests, 0 failed (as of `30b95a2`, the 2026-05-04 baseline). As of 2026-05-07: 407 / 0 after R41 through R45 review-fix commits added regression coverage.

#### shipped-binary-rename-blit
**Source**: RELEASE_PLAN_v2_2026-05-04.md §2.1 status / lines 187–191
**Specificity**: high

> ✅ Closed `0ca489b`. Crate name stayed `blit-cli`; binary produced is named `blit` via `[[bin]] name = "blit"`. R41 followup `e8f6aec` swept the test files that hardcoded `"blit-cli"` / `"blit-cli.exe"` as filesystem paths.

#### shipped-blit-utils-merged
**Source**: RELEASE_PLAN_v2_2026-05-04.md §2.2 status / lines 226–232
**Specificity**: high

> ✅ Closed `aac13bf` ... All plan/architecture/manpage references now point at the single `blit` binary; `BLIT_UTILS_PLAN.md` got a Superseded banner; phase-3/phase-4 workflow docs got post-phase notes.

#### shipped-blit-list-smart-dispatch
**Source**: RELEASE_PLAN_v2_2026-05-04.md §2.3 status / lines 252–257
**Specificity**: high

> ✅ Closed `4d07177`. Smart-dispatch implemented: bare-host targets route to `list-modules`; module/path targets route to `ls`. The explicit `blit list-modules <remote>` form continues to work.

#### shipped-find-glob
**Source**: RELEASE_PLAN_v2_2026-05-04.md §2.4 status / lines 284–289
**Specificity**: high

> ✅ Closed `090f5cd` (initial glob via `globset`) + R41 followup `e8f6aec` (set `literal_separator(true)` so `*` does not cross `/`, plus added basename-fallback regression test).

#### shipped-shell-completions
**Source**: RELEASE_PLAN_v2_2026-05-04.md §2.5 status / lines 307–316
**Specificity**: high

> ✅ Closed `0139a71` — picked Option A (clap_complete script generation). The completions subcommand split into two forms.

#### shipped-post-review-fixes-r1
**Source**: RELEASE_PLAN_v2_2026-05-04.md §2.7 status / lines 388–393
**Specificity**: high

> ✅ Closed `96cbb10`. R42 followup `3d953d9` finished the §1.1 metadata-error sweep ... R43 `8fd928e` factored `drain_pipeline_outcome`.

#### shipped-predictor-observability
**Source**: RELEASE_PLAN_v2_2026-05-04.md §2.8 status / lines 454–477
**Specificity**: high

> ✅ Closed. Phase 1 `ebcbb45` (data-model v2: dual targets + fallback chain), phase 2 `da6ced2` (orchestrator query + verbose/JSON surface). ... Steps 1, 2, and 4 are done — the predictor learns planner + transfer separately, walks the fallback chain, and is visible in `--verbose` and `blit profile --json`. Step 3 (Tiny extension) was deferred to post-0.1.0 with explicit reasoning in DEVLOG. So §2.8 closed as **predictor observability and training** — the predictor is no longer dead code (it is queried, surfaced, and audit-able), but adaptive planning behavior (Tiny picking up predictor signals) is still future work.

#### shipped-metrics-per-rpc-summary
**Source**: RELEASE_PLAN_v2_2026-05-04.md §3.1 status / lines 496–519
**Specificity**: high

> ✅ Closed (D5 outcome modified by owner). Rather than ship `--metrics` as dormant scaffolding, the daemon now emits a one-line stderr summary at the end of each push / pull / pull_sync RPC when `--metrics` is on.

#### shipped-mdns-txt-enrichment
**Source**: RELEASE_PLAN_v2_2026-05-04.md §3.2 status / lines 538–547
**Specificity**: high

> ✅ Closed. D4 default ("Yes (small, useful)") taken. `_blit._tcp.local.` advertisements now carry `module_count` ... and `delegation_enabled`.

#### shipped-doc-cleanup-table
**Source**: RELEASE_PLAN_v2_2026-05-04.md §4 / lines 598–625
**Specificity**: high

> ✅ Closed `aac13bf` (sweep) + `8d43e4d` (followup catching stale binary paths in BENCHMARK_10GBE_PLAN.md and WHITEPAPER.md, plus README completion-syntax).

---

### deferred

#### deferred-remote-benchmark-2.6
**Source**: RELEASE_PLAN_v2_2026-05-04.md §2.6 status / lines 357–365
**Specificity**: high

> ✅ Deferred to 0.1.1 (owner sign-off, 2026-05-31). Hardware-bound: needs a two-daemon network on real fabric. ... 0.1.0 ships with documented "performance claims to be verified" and a clear path to 0.1.1 once the benchmark numbers land. No code in 0.1.0 depends on the benchmark numbers.

#### deferred-daemon-fs-capability-4.8
**Source**: RELEASE_PLAN_v2_2026-05-04.md §3.3 status / lines 568–574
**Specificity**: high

> ✅ Deferred to 0.2.0 (owner sign-off, D6 default taken). 0.1.0 ships client-side capability probing only (`blit diagnostics dump`); daemon-startup and idle-probe mechanisms ... are explicitly out of 0.1.0 scope.

#### deferred-structured-logging-f15
**Source**: RELEASE_PLAN_v2_2026-05-04.md §5.1 / lines 634–639
**Specificity**: high

> Many `eprintln!` paths remain in daemon and core. `tracing` / structured `log` migration is a 1-2 week effort. Explicitly deferred per `PROJECT_STATE_ASSESSMENT.md`. Re-open in 0.2.0 once operational pain demonstrates need.

#### deferred-rdma-roce
**Source**: RELEASE_PLAN_v2_2026-05-04.md §5.3 / lines 652–654
**Specificity**: high

> Proto-only reservation. Hardware-bound, post-release investigation.

#### deferred-tui
**Source**: RELEASE_PLAN_v2_2026-05-04.md §5.5 / lines 666–669
**Specificity**: high

> `TUI_DESIGN.md` exists. No `Subscribe` / `GetState` RPCs in proto. No TUI binary or scaffolding. Deferred. The daemon's `TransferMetrics` are kept as scaffolding (see §3.3).

#### deferred-detach-mode
**Source**: RELEASE_PLAN_v2_2026-05-04.md §5.6 / lines 672–675
**Specificity**: high

> CLI Ctrl-C abort is the current contract. `--detach` would require a job-tracking RPC and durable state on the daemon. Out of scope.

#### deferred-packaging-matrix
**Source**: RELEASE_PLAN_v2_2026-05-04.md §5.7 / lines 677–681
**Specificity**: high

> 0.1.0 ships raw binaries + tarball + tri-platform CI artifacts. Debian/RPM, Homebrew formula, Windows installer, systemd/launchd service unit installers all deferred to 0.2.0.

#### deferred-hardware-benchmarks
**Source**: RELEASE_PLAN_v2_2026-05-04.md §5.8 / lines 684–688
**Specificity**: high

> `BENCHMARK_10GBE_PLAN.md`'s NFS/SMB-mount, daemon-pair, and reverse-direction phases all need 10GbE hardware. Do them post-release when hardware is available; 0.1.0's release notes note "10+ Gbps benchmarking pending hardware access."

#### deferred-investigations
**Source**: RELEASE_PLAN_v2_2026-05-04.md §5.9 / lines 690–697
**Specificity**: medium

> macOS FSEvents fast-path real-network field-test (`UNVERIFIED` in audit). Windows ReFS `SeManageVolumePrivilege` requirement for block clone (`TODO.md:259`).

#### deferred-tls-data-plane
**Source**: greenfield_plan_v6.md §6 v5 Open Questions / line 304
**Specificity**: medium

> TLS for data plane | Deferred | Evaluate cost once TCP path proven.

#### deferred-progress-ui-granularity
**Source**: greenfield_plan_v6.md §6 v5 Open Questions / line 302
**Specificity**: low

> Progress UI granularity | Planned | Must include throughput + ETA; evaluate `indicatif`.

#### deferred-rdma-hardware
**Source**: greenfield_plan_v6.md §6 v5 Open Questions / line 303
**Specificity**: medium

> RDMA hardware procurement | Pending | Coordinate when Phase 3.5 starts.

#### deferred-windows-rdma-tbd
**Source**: greenfield_plan_v6.md §6 v5 Open Questions / line 301
**Specificity**: low

> Windows RDMA viability | TBD | Evaluate once TCP hybrid stabilises.

#### deferred-phase-5-future-opts
**Source**: greenfield_plan_v6.md §3 v5 Phase 5 / lines 269–274
**Specificity**: medium

> Change journal integrations (USN, FSEvents) for faster incremental planning. GPU-accelerated hashing for checksum mode. Optional remote telemetry opt-in (if ever justified, with explicit user consent). Advanced storage tuning (stripe-aware writes, preallocation heuristics).

#### deferred-windows-service-specifics
**Source**: greenfield_plan_v6.md §5 v6 Open Questions / line 433
**Specificity**: low

> Windows service specifics | Identify any mDNS/config nuances when running as a Windows service.

#### deferred-rdma-phase-3.5
**Source**: greenfield_plan_v6.md §3 v6 Phase 3.5 / lines 406–407
**Specificity**: high

> Future work (defer until after core TCP/gRPC paths and required features are complete).

---

### rejected

#### rejected-blit-utils-standalone
**Source**: greenfield_plan_v6.md implementation note / lines 7–16
**Specificity**: high

> this plan originally scoped a separate `blit-utils` crate / binary for discovery and admin verbs. During Phase 3/4 those verbs were merged into the single `blit` binary ... the as-shipped surface is `blit <subcommand>` built from `crates/blit-cli`.

#### rejected-blit-auth-stub
**Source**: RELEASE_PLAN_v2_2026-05-04.md §5.2 / lines 643–650
**Specificity**: high

> The `BlitAuth` service stub and `RemoteSourceLocator.delegated_credential` forward-compat field were stripped from `proto/blit.proto`. Auth is out of project scope.

#### rejected-ai-telemetry-analysis
**Source**: RELEASE_PLAN_v2_2026-05-04.md §5.4 / lines 656–664
**Specificity**: high

> The scoping doc `docs/plan/AI_TELEMETRY_ANALYSIS.md` was deleted (owner decision).

#### rejected-ship-both-binaries-workaround
**Source**: RELEASE_PLAN_v2_2026-05-04.md §2.1 / lines 219–220
**Specificity**: high

> The "ship both binaries" workaround adds artifact confusion and is rejected.

#### rejected-ludicrous-speed-flag
**Source**: greenfield_plan_v6.md §1.1 v5 / line 156
**Specificity**: high

> No user speed flags (`--ludicrous-speed` is deprecated).

#### rejected-mir-flag
**Source**: greenfield_plan_v6.md §5 v5 Non-Negotiables / line 290
**Specificity**: high

> Never reintroduce deprecated flags (`--mir`, `--ludicrous-speed`) as behaviour toggles.

#### rejected-blit-utils-shipping-claim
**Source**: RELEASE_PLAN_v2_2026-05-04.md §2.2 / lines 240–243
**Specificity**: high

> Roadmap audit incorrectly marked `blit-utils` as shipping (an AppleDouble `._blit-utils.1.md` sidecar likely confused the grep).

---

### decision

#### decision-transport-hybrid
**Source**: MASTER_WORKFLOW.md §3 Decision Log / line 70
**Specificity**: high

> Transport model | ✅ Hybrid (gRPC control + TCP data plane with secure tokens, auto fallback)

#### decision-error-handling-eyre
**Source**: MASTER_WORKFLOW.md §3 Decision Log / line 71
**Specificity**: high

> Error handling | ✅ `eyre`/`color-eyre` for CLI + daemon, consistent context-rich errors

#### decision-async-tokio
**Source**: MASTER_WORKFLOW.md §3 Decision Log / line 72
**Specificity**: high

> Async runtime | ✅ Tokio across crates

#### decision-progress-quiet
**Source**: MASTER_WORKFLOW.md §3 Decision Log / line 73
**Specificity**: high

> Progress UX | ✅ CLI quiet by default; structured events exposed for GUIs/debug

#### decision-telemetry-local
**Source**: MASTER_WORKFLOW.md §3 Decision Log / line 74
**Specificity**: high

> Telemetry | ✅ Local JSONL history (optional opt-out); `blit profile` surfaces data

#### decision-env-vars-not-used
**Source**: MASTER_WORKFLOW.md §3 Decision Log / line 75
**Specificity**: high

> Environment variables | ✅ Not used for configuration; precedence is CLI flag → config file

#### decision-d1-binary-name
**Source**: RELEASE_PLAN_v2_2026-05-04.md §7 / line 739
**Specificity**: high

> D1 | Binary name: `blit` or `blit-cli`? | `blit` (rename via `[[bin]]`) | ✅ Taken — `0ca489b`

#### decision-d2-utils-merged
**Source**: RELEASE_PLAN_v2_2026-05-04.md §7 / line 740
**Specificity**: high

> D2 | `blit-utils` artifact: standalone or merged? | Merged | ✅ Taken — `aac13bf`

#### decision-d3-list-smart-dispatch
**Source**: RELEASE_PLAN_v2_2026-05-04.md §7 / line 741
**Specificity**: high

> D3 | `blit list` semantics: smart-dispatch or `list-modules`-only? | Smart-dispatch | ✅ Taken — `4d07177`

#### decision-d4-mdns-txt
**Source**: RELEASE_PLAN_v2_2026-05-04.md §7 / line 742
**Specificity**: high

> D4 | mDNS TXT enrichment in 0.1.0? | Yes (small, useful) | ✅ Taken — `0d76c4f`

#### decision-d5-metrics-modified
**Source**: RELEASE_PLAN_v2_2026-05-04.md §7 / line 743
**Specificity**: high

> D5 | `TransferMetrics` keep-as-scaffolding or remove? | Keep + document as dormant | ✅ Modified — owner chose "keep + emit per-RPC summary line" instead of dormant (2026-05-13)

#### decision-d6-fs-capability-defer
**Source**: RELEASE_PLAN_v2_2026-05-04.md §7 / line 744
**Specificity**: high

> D6 | Phase 4.8.2/4.8.3 daemon FS capability in 0.1.0? | Defer to 0.2.0; doc only | ✅ Taken — owner sign-off 2026-05-13

#### decision-d7-find-glob
**Source**: RELEASE_PLAN_v2_2026-05-04.md §7 / line 745
**Specificity**: high

> D7 | `find --pattern` glob or substring? | Glob | ✅ Taken — `090f5cd`

#### decision-d8-completions-clap
**Source**: RELEASE_PLAN_v2_2026-05-04.md §7 / line 746
**Specificity**: high

> D8 | Shell completions: clap_complete generation OR README edit? | Option A (clap_complete) | ✅ Taken — `0139a71`

#### decision-d9-predictor-wire
**Source**: RELEASE_PLAN_v2_2026-05-04.md §7 / line 747
**Specificity**: high

> D9 | Predictor: wire OR delete? | Wire | ✅ Taken — `ebcbb45` + `da6ced2`

#### decision-perf-gate-95pct
**Source**: greenfield_plan_v6.md §3 v5 Phase 2.5 / line 235
**Specificity**: high

> Gate: all scenarios ≥ 95 % of v1 throughput, planner overhead perceptions ≤ 1 s.

#### decision-v4-perf-gate-5pct
**Source**: greenfield_plan_v6.md §3 v4 Phase 2.5 / line 123
**Specificity**: high

> Decision Gate: Performance must be within a 5% margin of v1. Do not proceed until this is met.

#### decision-future-arch-must-be-logged
**Source**: MASTER_WORKFLOW.md §3 / line 77
**Specificity**: medium

> Future architectural decisions must be recorded here and in DEVLOG before implementation.

#### decision-net-blocker-count-2.6
**Source**: RELEASE_PLAN_v2_2026-05-04.md tracker / lines 37–38
**Specificity**: high

> **Net release-blocker count:** §2.6 (hardware-bound benchmark capture) is the last remaining P0. All P1 items closed.

---

## Contradictions

### contradiction-perf-gate-5pct-vs-95pct
The v4 Phase 2.5 gate (greenfield_plan_v6.md line 123) says **"Performance must be within a 5% margin of v1"** (i.e., v2 ≥ 95 % of v1). The v5 Phase 2.5 gate (line 235) says **"all scenarios ≥ 95 % of v1 throughput"** — same numerical target but phrased differently. MASTER_WORKFLOW.md §2 (line 47) also says "≥ 95 % of v1 throughput". v4 is superseded so not a live contradiction, but the v4 wording "within a 5% margin" is ambiguous (could be ±5%); both later docs settle on a one-sided ≥ 95 % floor.

### contradiction-blit-utils-presence
greenfield_plan_v6.md §2 v6 (lines 348–352, 421) still uses the term "blit-utils" and lists "`blit-utils` implements `scan`, `ls`, ...". MASTER_WORKFLOW.md §1 (line 20) and the implementation note at the top of greenfield_plan_v6.md (lines 7–16) state there is no separate `blit-utils` and the verbs are merged into the `blit` binary. RELEASE_PLAN_v2_2026-05-04.md §2.2 confirms merge as the final outcome. The v6 §4 Deliverables Checklist still has the line "`blit-utils` implements ..." which has not been retroactively updated to match the §2.2 closure note.

### contradiction-progress-spinner-vs-quiet-default
greenfield_plan_v6.md §2 v5 (line 222) says "Introduce unified progress indicator (spinner + throughput + ETA) for copy/mirror." MASTER_WORKFLOW.md §3 (line 73) says "CLI quiet by default; structured events exposed for GUIs/debug." These can both hold (spinner under verbose flag), but they read as conflicting defaults; the master workflow decision log is the more current statement.

### contradiction-config-default-path
greenfield_plan_v6.md §2 v6 (line 341) names `/etc/blit/config.toml` as the default config path. None of the three docs explicitly contradict this, but MASTER_WORKFLOW.md and RELEASE_PLAN do not reaffirm the default; only the precedence rule (CLI flag → config) is reaffirmed. Not a hard contradiction — a coverage gap that an auditor could flag if implementation chose a different path (e.g., `~/.config/blit/`).

### contradiction-utils-superseded-but-checklist-active
greenfield_plan_v6.md §1 implementation note (top, lines 7–16) flags the body's references to `blit-utils` as historical, "Kept verbatim as a historical architectural record". Yet §4 Deliverables Checklist (line 421) still lists `blit-utils implements …` as an active deliverable. Either the entire v6 body is historical or the checklist is the authoritative live list — the doc tries to be both.

### contradiction-tui-deferred-vs-metrics-scaffolding-modified
RELEASE_PLAN_v2_2026-05-04.md §5.5 (lines 666–669) says TUI is deferred and "The daemon's `TransferMetrics` are kept as scaffolding (see §3.3)" — but §3.3 is the Phase 4.8 daemon FS capability section, not TransferMetrics. The cross-reference is broken (should point at §3.1). And §3.1's actual outcome contradicts the §5.5 description: TransferMetrics is no longer dormant scaffolding; it emits a per-RPC stderr summary line (D5 modified by owner). §5.5 still describes the pre-resolution state.

### contradiction-utils-checklist-also-renamed
RELEASE_PLAN_v2_2026-05-04.md §4 doc cleanup table (line 608) lists MASTER_WORKFLOW.md as having had its "`blit-utils` → `blit` admin subcommands" reword applied. MASTER_WORKFLOW.md still mentions "there is no separate `blit-utils`" parenthetically (line 20) but the parenthetical is corrective, not a remaining reference. Not a contradiction in itself; flagged because the absence-statement is the corrected form.

### contradiction-utils-supersedes-detail
greenfield_plan_v6.md v6 §3 Phase 3 (line 403) says "RPCs powering `list`, `find`, `du`, `df`, `rm`." MASTER_WORKFLOW.md Phase 3 gate (line 54) requires admin verbs including `list-modules`, `ls`, `completions`, `profile`. The v6 RPC list is a subset of the master workflow gate's verb list. The verb-vs-RPC distinction softens this, but a literal reader would note the v6 §3 doesn't mention `list-modules` as an RPC at all (it's a CLI-side dispatch — see §2.3 closure).

---

## Coverage attestation

| File | Lines | Notes |
|---|---|---|
| docs/plan/greenfield_plan_v6.md | 442 | Read in full (lines 1–442). Contains three concatenated versions: v4 (1–141), v5 (143–308), v6 (310–442). v4 marked superseded by v5 (line 308); v5 marked superseded by v6 (line 312). |
| docs/plan/MASTER_WORKFLOW.md | 120 | Read in full. |
| docs/plan/RELEASE_PLAN_v2_2026-05-04.md | 805 | Read in full. |

**Total lines**: 1367
