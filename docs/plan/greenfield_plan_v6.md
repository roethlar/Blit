# Blit v2 – Implementation Plan (v6 “Feature Completeness & Transport”)

**Status**: Active (supersedes v5)  
**Purpose**: Capture the functionality v2 still needs—CLI verbs, remote semantics, daemon configuration, discovery, admin utilities—and integrate it with the streaming/hybrid transport roadmap. The focus is shipping the required features; backward compatibility with v1 is not a goal.

---

## 1. Guiding Principles

1. **Deliver the Needed Features** – Ensure the CLI, daemon, and utilities expose the commands and behaviours the project relies on (copy/mirror/move, remote discovery, module management, admin tooling). These are functional requirements, not a promise of backward compatibility.
2. **FAST / SIMPLE / RELIABLE / PRIVATE** – Same non-negotiables as v5: planner auto-tunes, no user speed knobs, correctness outweighs raw throughput, and metrics never leave the machine.
3. **Transport Evolution** – Hybrid TCP, automatic gRPC fallback, and future RDMA remain core differentiators, layered once the required feature set is present.
4. **Clarity Over Legacy** – Document what v2 provides; references to v1 exist only for historical context.

---

## 2. Feature Gaps To Close

### CLI & Remote Semantics
- Replace the current `push`/`pull` model with the required command set: `copy`, `mirror`, `move`, `scan`, `list`, plus diagnostics.
- Support local ↔ remote transfers in any direction for `copy`, `mirror`, and `move`.
- Adopt canonical remote syntax:
  - `server:/module/` → root of a named module (must end with `/`).
  - `server:/module/path` → path under the module root.
  - `server://path` → default export. If `--root` is supplied (or config defines a default root), that path is used; otherwise the daemon’s current working directory is exposed.
  - Bare `server` (optionally `:port`) → discovery (list modules).
  - `server:/module` without a trailing slash is invalid (ambiguous) and should error.
- Default remote port is 9031; allow overrides via `server:port/...` and CLI flags.
- `move` performs a mirror followed by source removal (local or remote).

### Module Configuration & Daemon Behaviour
- Load module definitions from a TOML config (`/etc/blit/config.toml` by default) with fields: `name`, `path`, `comment`, `read_only`, `use_chroot`, and daemon-level settings `bind`, `port`, `motd`, `no_mdns`, `mdns_name`.
- Expose flags such as `--config`, `--bind`, `--port`, `--root`, `--no-mdns`, `--mdns-name`.
- Behaviour when no modules are defined:
  - If `--root` is provided (or the config defines a default root), expose it via `server://`.
  - Otherwise `server://` resolves to the daemon’s working directory, matching historical behaviour. Log a warning so operators know they are running with an implicit root export.
- Enforce read-only modules and chroot semantics for every remote operation.

### Discovery & Admin Utilities (`blit-utils`)
- Implement subcommands: `scan`, `ls`, `list`, `rm`, `find`, `du`, `df`, `completions`, and a `profile` command for local performance capture.
- Utilities must share the URL parser and RPC clients with the CLI.
- Destructive operations (`rm` and any future destructive verbs) require confirmation unless `--yes` is supplied.
- `scan` consumes mDNS advertisements; `find`, `du`, `df` rely on new daemon RPCs.

### Remote Services
- Extend the daemon API to support utility verbs:
  - Directory listing (for `list`/`ls`), recursive enumeration (`find`), space usage (`du`, `df`), and remote remove (`rm`).
- Keep transfer verbs (`copy`, `mirror`, `move`) on the hybrid transport path (TCP + gRPC fallback).
- Administrative RPCs can remain gRPC-only but must honour module boundaries and read-only flags.
- Automatic gRPC fallback for data transfers is mandatory; CLI prints a warning but continues.

### Discovery & Advertisements
- Advertise `_blit._tcp.local.` via mDNS by default; provide opt-out (`--no-mdns`) and custom instance name (`--mdns-name`).
- Confirm behaviour on Linux, macOS, and Windows.

### Documentation & Tests
- Update CLI help/man pages to reflect the command set and remote syntax. No migration guide—documentation describes v2 only.
- Extend integration tests to cover:
  - All transfer permutations (local↔remote, remote↔local, remote↔remote).
  - Utility workflows (`scan`, `list`, `ls`, `find`, `du`, `df`, `rm`).
  - Daemon startup combinations (modules present, modules absent with `--root`, mDNS toggles).

---

## 3. Revised Phase Breakdown

### Phase 0 – Required Feature Surface
1. **CLI Command Set**
   - Replace command matrix with `copy`, `mirror`, `move`, `scan`, `list`, diagnostics.
   - Implement canonical remote URL parsing in `blit-core::remote::endpoint`.
   - Add tests for all transfer permutations and error cases (e.g., `server:/module` without `/`).
2. **blit-utils Tooling**
   - Implement admin verbs (`scan`, `ls`, `list`, `rm`, `find`, `du`, `df`, `completions`, `profile` command).
   - Share networking code with CLI.
3. **Daemon Config & Flags**
   - Load modules/root from TOML; respect overrides.
   - Enforce read-only/chroot semantics.
   - Define behaviour when no modules exist (require `--root` or emit clear errors).
4. **mDNS Advertising**
   - Advertise `_blit._tcp.local.` by default; verify discovery via CLI/util tests.

### Phase 1 – gRPC API & Service Skeleton (Carry-over from v5)
Unchanged scope, but proto definitions must include listing/usage RPCs needed by utilities.

### Phase 2 – Streaming Orchestrator & Local Operations
Same as v5 (streaming planner, predictor, quiet CLI). Benchmarks/tests now use the new command set.

### Phase 2.5 – Performance Gate
Benchmarks include remote scenarios using the canonical syntax (TCP + gRPC fallback).

### Phase 3 – Hybrid Remote Operations
Augment v5 tasks with:
- Module-aware authorisation (read_only, chroot).
- RPCs powering `list`, `find`, `du`, `df`, `rm`.
- Ensure transfer transport defaults to port 9031 (configurable).

### Phase 3.5 – RDMA Enablement
Future work (defer until after core TCP/gRPC paths and required features are complete).

### Phase 4 – Production Hardening & Packaging
- Package mDNS dependencies, config directories, and blit-utils alongside CLI/daemon.
- Document command surface and admin workflows.

---

## 4. Deliverables Checklist

- [ ] CLI (`copy`, `mirror`, `move`, `scan`, `list`, diagnostics) operational with canonical remote syntax.
- [ ] `blit-core::remote::endpoint` parses `server:/module/...`, `server://...`, discovery forms, and rejects ambiguous inputs.
- [ ] `blit-daemon` loads modules/root from config, supports flags, advertises via mDNS, enforces read_only/chroot semantics, and handles “no exports configured” cleanly.
- [ ] RPC surface supports `list`, `find`, `du`, `df`, `rm`.
- [ ] `blit-utils` implements `scan`, `ls`, `list`, `rm`, `find`, `du`, `df`, `completions`, `profile` command.
- [ ] Test suite covers transfer permutations, admin workflows, and daemon startup scenarios.
- [ ] Documentation (CLI, blit-daemon, blit-utils) reflects the updated feature set.
- [ ] Benchmarks include remote scenarios over TCP and gRPC fallback using the new commands.

---

## 5. Open Questions & Decisions Needed

| Topic | Decision Needed |
|-------|-----------------|
| Config search order | Confirm precedence (CLI flag → config). No environment variables. |
| Windows service specifics | Identify any mDNS/config nuances when running as a Windows service (e.g., service account working directory, firewall rules). |
| Future admin verbs | Additional utilities beyond `scan`, `ls`, `list`, `rm`, `find`, `du`, `df`, `completions`, `profile` will be added as requirements are confirmed. |

---

## 6. Next Steps

1. Archive v5 as historical reference (pointer only).
2. Break Phase 0 items into actionable tasks (CLI changes, utils implementation, config loader, mDNS).
3. Begin execution, ensuring DEVLOG/TODO/workflows record progress for seamless hand-offs.
