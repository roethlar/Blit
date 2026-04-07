# Blit v2 Project State Assessment

**Last Updated**: 2026-04-07
**Plan Reference**: [greenfield_plan_v6.md](./greenfield_plan_v6.md)

---

## 1. Executive Summary

Blit v2 is feature-complete for a 0.1.0 release. All phases through Phase 4
(Production Hardening) are substantially done. The remaining open items are
benchmarking tasks that require dedicated hardware (10+ GbE network) and
post-release investigations (RDMA, ReFS privilege).

**High-Level Status**

| Phase | Status | Notes |
|-------|--------|-------|
| Phase 0 – Foundation | Done | Workspace + core modules ported |
| Phase 1 – gRPC Scaffolding | Done | Proto + tonic scaffolding live |
| Phase 2 – Local Ops | Done | Streaming planner, predictor, CopyFileEx, change journals |
| Phase 2.5 – Validation | Done | All benchmarks GO on Linux/macOS/Windows |
| Phase 3 – Remote Ops | Done | Hybrid transport, all admin RPCs, remote-to-remote |
| Phase 4 – Production | Done | Packaging, CI, docs, logging review, release notes |
| Phase 3.5 – RDMA | Deferred | Post-release investigation |

---

## 2. What's Done

### Transfer Engine
- Local copy/mirror/move with async orchestrator and parallel workers
- Remote push/pull via hybrid TCP data plane + gRPC control plane
- Remote-to-remote transfers (`server1:/mod/ → server2:/mod/`)
- Block-level resumable transfers with Blake3 hashing
- Multiplexed TCP data plane with auto-tuned stream counts
- Small file batching via tar shards with parallel daemon unpacking
- Async read-ahead pipeline with buffer pool (double-buffered I/O)
- Streaming manifest exchange for arbitrarily large file sets
- Adaptive performance predictor with online gradient descent
- Performance history with schema versioning (v0/v1 migration)

### Platform Support
- macOS: clonefile, FSEvents journal, statfs FS detection
- Linux: copy_file_range, metadata snapshot journal, statfs FS detection
- Windows: CopyFileExW, USN Change Journal, ReFS block clone
- Filesystem capability probing for 12+ FS types, cached per device ID

### CLI Surface
- `blit`: copy, mirror, move, scan, list, du, df, rm, find, diagnostics
- `blit-daemon`: TOML config, modules, mDNS, hybrid transport
- `blit-utils`: scan, list-modules, ls, find, du, df, rm, completions, profile

### Testing
- Integration tests: admin verbs (10), blit-utils (21), remote transfers,
  transfer edges, parity, resume, move, remote-to-remote
- Unit tests: predictor regression (9), schema versioning (7), FS probing (7),
  mirror planner, checksum, enumeration, buffer pool
- GitHub Actions CI: fmt/clippy, tri-platform tests, release artifacts

### Documentation
- Man pages: blit(1), blit-daemon(1), blit-utils(1)
- ARCHITECTURE.md, DAEMON_CONFIG.md, PERFORMANCE_ROADMAP.md
- CHANGELOG.md with full 0.1.0 feature inventory
- AI telemetry analysis scoping doc

### Packaging & CI
- `scripts/build-release.sh` (Unix) with tarball creation
- `scripts/windows/build-release.ps1`
- `.github/workflows/ci.yml` — check/test/build on all platforms

---

## 3. What's Left

### Benchmarking (needs hardware)
- [ ] Benchmark TCP data plane throughput targeting 10+ Gbps per stream
- [ ] Benchmark remote fallback + data-plane streaming (sub-second first-byte)
- [ ] Capture remote benchmark runs (TCP vs gRPC fallback) and log results

These require a 10+ GbE test environment (e.g., `skippy`/`mycroft`). All
implementation work is done — only the measurement runs remain.

### Post-Release
- [ ] RDMA/RoCE investigation (control-plane negotiation, transport abstraction)
- [ ] ReFS block clone SeManageVolumePrivilege investigation on Windows

### Nice-to-Have (Implementation Deferred)
- AI telemetry analysis (scoped in `docs/plan/AI_TELEMETRY_ANALYSIS.md`)
- Full structured logging migration (eprintln → log macros across ~50 sites)

---

## 4. Architecture Overview

```
User Layer
├── blit-cli      (CLI: copy/mirror/move/scan/list/du/df/rm/find/diagnostics)
├── blit-daemon   (gRPC server: modules, hybrid transport, admin RPCs)
└── blit-utils    (admin: scan/list-modules/ls/find/du/df/rm/completions/profile)

blit-core
├── orchestrator      — parallel transfer coordination
├── transfer_engine   — end-to-end transfer lifecycle
├── transfer_facade   — unified local/remote interface
├── mirror_planner    — sync diff computation
├── enumeration       — directory traversal
├── copy              — platform-optimized copying (clone/sendfile/CopyFileEx)
├── checksum          — Blake3/XXHash/MD5 integrity
├── change_journal    — USN (Win), FSEvents (mac), metadata (Linux)
├── remote            — gRPC client, pull, push, transfer modules
├── tar_stream        — small-file batching
├── perf_predictor    — adaptive heuristics
├── perf_history      — versioned JSONL storage
├── fs_capability     — FS detection + capability probing + cache
├── buffer            — pooled buffers with semaphore control
├── config            — platform config directory resolution
└── mdns              — mDNS discovery
```

---

## 5. Key Files for Windows Development

When continuing development on Windows, these are the most relevant files:

### Windows-Specific Code
- `crates/blit-core/src/copy/windows.rs` — ReFS block clone, CopyFileExW
- `crates/blit-core/src/copy/file_copy/mod.rs` — Platform dispatch (clone path)
- `crates/blit-core/src/change_journal/` — USN Change Journal (Windows path)
- `crates/blit-core/src/fs_capability/windows.rs` — Windows FS capabilities
- `crates/blit-core/src/zero_copy.rs` — Platform zero-copy abstractions

### Build & Test
- `scripts/windows/run-blit-tests.ps1` — Windows test runner
- `scripts/windows/build-release.ps1` — Windows release build
- `scripts/windows/bench-local-mirror.ps1` — Windows benchmark
- `scripts/windows/probe-usn-volume.ps1` — USN journal probe

### Configuration
- Default daemon config: `C:\ProgramData\Blit\config.toml`
- Client config: `C:\Users\<user>\AppData\Local\Blit\Blit\`

### Known Windows Issues
- ReFS block clone requires `SeManageVolumePrivilege` — currently falls back
  to CopyFileExW (~0.6s vs ~0.17s robocopy for 4 GiB)
- Windows integration tests deferred to CI (run via GitHub Actions)
- `blit df` required path prefix fix (`\\?\` stripping) — already fixed

### Benchmarking on Windows
- Small files: 100k × 4 KiB → blit 60.6s vs robocopy 218.5s
- Mixed workload: 512 MiB + 50k × 2 KiB → blit 31.3s vs robocopy 110.5s
- Incremental: touch 2k/delete 1k/add 1k → blit 6.5s vs robocopy 6.9s
- Zero-change (USN fast-path): 28ms

---

## 6. Development Workflow

1. Read `TODO.md` for remaining items (mostly benchmarking)
2. Read `DEVLOG.md` for recent changes (newest entries at top)
3. Run `cargo test --workspace` to verify everything passes
4. On Windows: `.\scripts\windows\run-blit-tests.ps1`
5. Build release: `.\scripts\windows\build-release.ps1`
6. Update `TODO.md` and `DEVLOG.md` after completing work

---

**Project**: Blit v2
**Status**: Feature-complete, pending benchmark validation
