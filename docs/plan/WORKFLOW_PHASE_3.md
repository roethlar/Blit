# Phase 3: Remote Operations & Admin Tooling

**Goal**: Deliver the remote feature set defined in plan v6 – hybrid transport for `blit copy/mirror/move`, canonical remote syntax, mDNS discovery, and the `blit-utils` admin verbs backed by daemon RPCs.  
**Prerequisites**: Phase 2 gate passed (streaming orchestrator stable) and Phase 2.5 benchmarks meeting targets.  
**Status**: In progress.  
**Critical Path**: Hybrid transport completion, CLI/daemon/utility alignment, admin RPC implementation.

---

## 1. Success Criteria

- `blit copy`, `blit mirror`, `blit move` accept local ↔ remote endpoints using `server:/module/...` and `server://...` syntax; hybrid transport negotiates TCP data plane with secure tokens and falls back to gRPC automatically.
- `blit scan` discovers daemons via mDNS (with opt-out flag).
- `blit list` and `blit ls` surface module lists and directory contents.
- `blit-utils` verbs (`scan`, `ls`, `list`, `rm`, `find`, `du`, `df`, `completions`, `profile`) operate against the daemon, respecting read-only/chroot rules and prompting before destructive actions.
- Canonical URL parser shared across CLI/daemon/utils (no `blit://` scheme).
- Daemon loads modules/root from TOML config (and `--root`), advertises via mDNS unless disabled, and warns when using implicit working-directory exports.
- Integration tests cover remote transfer + admin scenarios on Linux, Windows, and macOS.

---

## 2. Work Breakdown

### 3.1 Command & Parser Realignment
| Task | Description | Deliverable |
|------|-------------|-------------|
| 3.1.1 | Replace `push`/`pull` CLI commands with `copy`, `mirror`, `move`, `scan`, `list`, diagnostics. | Updated `blit-cli` parser, help/man pages. |
| 3.1.2 | Implement canonical remote URL parsing (`server:/module/...`, `server://...`, discovery on bare host) in `blit-core::remote::endpoint`. | Shared parser + unit tests covering IPv4/IPv6, module roots, errors. |
| 3.1.3 | Wire CLI transfer verbs to hybrid transport flow (reuse local orchestrator, remote endpoints). | CLI invocation triggers remote path resolution and orchestrator/transport pipeline. |
| 3.1.4 | Update CLI docs (`docs/cli/blit.1.md`, help output) to reflect new verb surface and remote syntax. | Documentation and CLI `--help` output. |

### 3.2 Daemon Configuration & Discovery
| Task | Description | Deliverable |
|------|-------------|-------------|
| 3.2.1 | Add TOML config loader (`/etc/blit/config.toml` or path via `--config`) with module definitions (`name`, `path`, `comment`, `read_only`, `use_chroot`) and daemon settings (`bind`, `port`, `motd`, `no_mdns`, `mdns_name`, optional default root). | ✅ `blit-daemon` now parses config/CLI overrides, populates modules/default root, and warns when falling back to the working directory. |
| 3.2.2 | Implement behaviour when no modules defined: use `--root`/config root export; otherwise default to daemon working directory with warning. | Daemon startup logic and log messaging. |
| 3.2.3 | Integrate mDNS advertisement (`_blit._tcp.local.`) enabled by default, disabled via `--no-mdns`. | Advertising helper with lifecycle management + tests. |
| 3.2.4 | Ensure `blit scan` and `blit-utils scan` consume mDNS results cross-platform. | CLI/util output demonstrating discovery, integration tests. |

### 3.3 Hybrid Transport Completion
| Task | Description | Deliverable |
|------|-------------|-------------|
| 3.3.1 | Finalise `proto/blit.proto` to include hybrid transport negotiation (`DataTransferNegotiation`, summary fields) and admin RPCs (`ListModules`, directory listing, recursive enumeration, disk usage, remote remove). *(2025-11-10: shared `remote::transfer::{payload,progress,data_plane}` modules extracted and push updated to use them; PullChunk/daemon pull wiring still pending.)* | Updated proto + regenerated Rust code. |
| 3.3.2 | Implement daemon-side control plane: accept negotiates, spawn TCP listener, issue secure tokens, enforce read-only/chroot, stream Pull responses. | ✅ Logic now lives in `service.rs` (split out of `main.rs` on 2025-10-27); tests cover negotiation + fallback. |
| 3.3.3 | Implement TCP data plane server (zero-copy when available, buffered fallback). | Data-plane module + unit tests. |
| 3.3.4 | Implement CLI data plane client: token validation, gRPC fallback, progress events. | `blit-cli` transport layer + integration tests. |
| 3.3.5 | Handle gRPC fallback automatically when TCP negotiation fails; emit warning and continue. | ✅ 2025-10-25 – integration test (`remote_tcp_fallback`) forces client `--force-grpc` and asserts CLI reports `[gRPC fallback]` with files mirrored. |
| 3.3.6 | Integrate Windows USN journal checkpoints into incremental planner fast-path so no-op mirror runs avoid full enumeration. | ✅ 2025-10-25 – cache reprobe+comparison relaxed; zero-change NTFS mirror completes in 28 ms (wingpt-53). |
| 3.3.7 | Integrate macOS FSEvents checkpoints into incremental planner fast-path. | ⚠️ 2025-10-25 – snapshot capture landed (`MacSnapshot` stores FSID/event ID/mtime); macOS verification run pending (`scripts/macos/run-journal-fastpath.sh`). |
| 3.3.8 | Integrate Linux fanotify/inotify (or documented alternative) into incremental planner fast-path. | ✅ 2025-10-25 – metadata snapshot (device/inode/ctime) powers no-op fast-path; further fanotify work optional. |
| 3.3.9 | Stream manifest and need-list negotiation so remote pushes do not allocate manifest/need-list Vecs (see Blocker 3.4.4). | ✅ 2025-10-26 – CLI streams manifests via bounded channel, daemon batches need lists with back-pressure. |

### 3.4 Admin RPCs, Utilities, and Streaming Need List
| 3.4.0 | Draft detailed blit-utils plan covering command matrix (see `docs/plan/BLIT_UTILS_PLAN.md`) (`scan`, `list`, `ls`, `rm`, `find`, `du`, `df`, `completions`, `profile`), CLI UX, confirmation flows, and safety prompts. | Design doc + TODO entries. |
| Task | Description | Deliverable |
|------|-------------|-------------|
| 3.4.1 | Implement daemon RPCs for: module listing, directory listing, recursive enumeration (`find`), disk usage (`du`, `df`), remote removal (`rm`). | ✅ RPC handlers live (2025-10-24) with read-only/chroot enforcement + tests. |
| 3.4.2 | Implement `blit-utils` verbs (`scan`, `ls`, `list`, `rm`, `find`, `du`, `df`, `completions`, `profile`) using shared client helpers. | ✅ `crates/blit-utils/src/main.rs` updated; docs + TODO synced (2025-10-24). |
| 3.4.3 | Provide safety prompts for destructive operations (default confirm, `--yes` bypass). | CLI UX + tests. |
| 3.4.4 | Expose shell completions using canonical URL syntax (bash/zsh/fish/powershell). | Completion scripts updated. |
| 3.4.6 | Stream manifest/need-list handling to avoid materialising large payloads (blocker). | ✅ 2025-10-26 – Remote push manifest streaming + chunked gRPC fallback landed; stress-test follow-up tracked in TODO. |
| 3.4.5 | Integrate `profile` command with local performance history (read-only insights). | CLI output + documentation. |

*Status 2025-11-06*: Remote push TCP data plane is now resilient and metadata-preserving. Added a hidden `--trace-data-plane` flag for diagnostics, restored streamed-file mtimes/permissions on the daemon path, and verified a 256 k-file (~9.8 GiB) home-directory mirror runs to completion without resets (`logs/blit-cli.log`, `logs/blitd.log`). Remote-to-local mirrors now reuse the pull path plus a local purge pass, enabling `blit mirror skippy://module dest/` to delete extraneous files after copying. Throughput tuning continues, but repeated re-uploads caused by mismatched metadata are resolved.  

*Status 2025-10-28*: Remote push manifest/need-list streaming now avoids Vec allocation and feeds the daemon incrementally; both the TCP data plane and gRPC fallback batch small files into tar shards, removing per-file overhead. The daemon side now parallelises tar-shard unpacking (`TarShardExecutor`, four blocking workers) so the data plane keeps flowing while shards decode—line-rate verification on `skippy`/`mycroft` is the next gating task. CLI now pipelines tar-shard preparation so TCP transfers start as soon as the first need-list batch arrives. Added verbose `[data-plane] …` logging around the daemon to diagnose the remaining “connection reset by peer” failures (current suspect: upload channel closes when the server hits an I/O error mid-stream). Purge RPC + `blit-utils rm` landed (with confirmation prompts). Remote mirror pushes reuse the purge helper to delete extraneous files (summary reports `entries_deleted`). Daemon advertises `_blit._tcp.local.` by default and `blit scan`/`blit-utils scan` consume the results. Admin RPCs (`Find`, `DiskUsage`, `FilesystemStats`, `CompletePath`) and the corresponding `blit-utils` verbs are in place. Windows USN journal checkpoints drive the incremental fast-path (3.3.6); macOS FSEvents verification remains pending. CLI and utilities entrypoints refactored into dedicated modules so each `main.rs` is now a thin dispatcher, and the daemon service has been split into `service/{core,push,pull,admin,util}` for better maintainability.

### 3.5 Testing & Validation
| Task | Description | Deliverable |
|------|-------------|-------------|
| 3.5.1 | Add integration tests covering remote copy/mirror/move (TCP + forced gRPC fallback). | Cross-platform integration suite. |
| 3.5.2 | Add tests for admin verbs (list/find/du/df/rm) including error cases (missing module, read-only, traversal attempts). | Tests in `crates/blit-cli`, `crates/blit-utils`, or `/tests`. |
| 3.5.3 | Verify mDNS discovery on Linux, macOS, Windows (CI where possible, manual logs otherwise). | Documented test logs in `DEVLOG` and `logs/`. |
| 3.5.4 | Validate canonical URL parser with exhaustive inputs (unit + property tests). | Parser test suite. |
| 3.5.5 | Update benchmark harnesses to include remote scenarios (TCP + fallback) once Phase 2.5 complete. | Scripts + recorded results. |

### 3.6 Documentation & Logging
| Task | Description | Deliverable |
|------|-------------|-------------|
| 3.6.1 | Update CLI manpages (`docs/cli/blit*.md`) and README snippets. | Documentation matching new verbs + syntax. |
| 3.6.2 | Update daemon documentation (`docs/cli/blit-daemon.1.md`) to describe TOML config, `--root`, mDNS. | Documentation text + examples. |
| 3.6.3 | Update workflows (`WORKFLOW_PHASE_3.md`, `MASTER_WORKFLOW.md`, `PROJECT_STATE_ASSESSMENT.md`) as milestones land. | Docs remain authoritative. |
| 3.6.4 | Record progress and benchmark evidence in `DEVLOG.md` + `TODO.md`. | Entries per milestone. |

---

## 3. Execution Order (Suggested)
1. **Parser & CLI Alignment (3.1)** – unblock remote features and documentation.  
2. **Daemon Config & mDNS (3.2)** – ensure exports and discovery behave correctly.  
3. **Hybrid Transport Completion (3.3)** – solidify control/data plane flows.  
4. **Admin RPCs & Utilities (3.4)** – deliver management tooling.  
5. **Testing & Validation (3.5)** – integration suite + remote benchmarks.  
6. **Documentation & Logging (3.6)** – update docs/DEVLOG as work lands (do not defer).

---

## 4. Risks & Mitigations

| Risk | Impact | Mitigation |
|------|--------|------------|
| CLI/API drift during refactor | Users face inconsistent verbs/syntax | Finish parser work before expanding remote functionality; update docs simultaneously |
| Security gaps in data-plane tokens | Potential replay attacks | Use cryptographically strong tokens with nonce + expiry; bind socket before streaming |
| mDNS instability across platforms | Discovery fails | Provide `--no-mdns` flag, document manual connection, add tests/logs per platform |
| Admin RPC complexity | Timeline slip | Implement iteratively: modules → dir list → recursive → metrics → destructive actions |

---

## 5. Deliverables Checklist (Phase 3 Gate)

- [ ] `blit copy/mirror/move` accept canonical remote syntax; hybrid transport + fallback validated.
- [ ] `blit scan`, `blit list`, `blit ls` operational.
- [x] Remote push/pull streaming manifest + need list (no in-memory exhaustion). *(Push implemented with streaming manifest + chunked fallback; add pull stress test if future workloads demand it.)*
- [x] `blit-utils` commands (`scan`, `ls`, `list`, `rm`, `find`, `du`, `df`, `completions`, `profile`) implemented and tested.
- [ ] Daemon loads modules/root from TOML, advertises via mDNS (with opt-out), enforces read-only/chroot.
- [x] Admin RPCs (list modules, dir listing, recursive search, disk usage, remote remove) implemented.
- [ ] Integration tests cover remote transfer + admin verbs (TCP + fallback) across platforms.
- [ ] Documentation (CLI, daemon, utils) updated; DEVLOG/TODO entries recorded.
- [ ] Remote benchmarks (TCP vs fallback) captured once features stabilize.

Completion of these items satisfies the Phase 3 gate defined in Plan v6 and allows progression to Phase 4.
