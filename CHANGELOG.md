# Changelog

All notable changes to Blit are documented in this file.

## [0.1.0] - Unreleased

### Transfer Engine
- Local copy, mirror, and move operations with async orchestrator
- Remote push/pull via hybrid TCP data plane + gRPC control plane
- Remote-to-remote transfers (`blit copy server1:/mod/ server2:/mod/`)
- Block-level resumable transfers with Blake3 hashing (`--resume`)
- Multiplexed TCP data plane with auto-tuned stream counts
- Small file batching via tar shards (parallel unpacking on daemon)
- Async read-ahead pipeline with buffer pool (double-buffered I/O)
- Streaming manifest exchange for arbitrarily large file sets
- Adaptive performance predictor using online gradient descent

### Platform Support
- **macOS**: `clonefile()` CoW, FSEvents change journal, `statfs` FS detection
- **Linux**: `copy_file_range()`, metadata snapshot change journal, `statfs` FS detection
- **Windows**: `CopyFileExW`, USN Change Journal, ReFS block clone (when privileged)
- Filesystem capability probing for 12+ filesystem types (APFS, btrfs, XFS, ext4, ZFS, NFS, CIFS, NTFS, ReFS, etc.)
- Device-keyed capability cache

### CLI (`blit`)
- Commands: `copy`, `mirror`, `move`, `scan`, `list`, `du`, `df`, `rm`, `find`
- `diagnostics perf` for performance history management
- Progress spinner (`--progress`), verbose output (`--verbose`)
- `--dry-run`, `--checksum`, `--force-grpc`, `--workers`
- Destructive operations prompt unless `--yes` is supplied

### Daemon (`blit-daemon`)
- TOML configuration with `[[module]]` exports
- mDNS service discovery (`_blit._tcp.local.`)
- Admin RPCs: ListModules, List, Find, DiskUsage, FilesystemStats, CompletePath, Purge
- Hybrid transport: TCP data plane negotiation with gRPC fallback
- `--root` default export, `--no-mdns`, `--force-grpc-data`

### Admin Utilities (built into `blit`)
- Commands: `scan`, `list-modules`, `ls`, `find`, `du`, `df`, `rm`, `completions`, `profile`
- `--json` output for all inspection commands
- Human-readable byte formatting in `df` output
- Local path support for `ls`
- Originally a separate `blit-utils` binary; merged into `blit` for a single install/distribution surface

### Performance History
- JSONL storage with schema versioning (v0/v1 migration)
- Capped at ~1 MiB with rotation
- Adaptive predictor with per-profile coefficients
- `blit diagnostics perf` for inspection and management

### Documentation
- Man pages: `blit(1)`, `blit-daemon(1)`
- Architecture guide (`docs/ARCHITECTURE.md`)
- Daemon configuration guide (`docs/DAEMON_CONFIG.md`)
- Performance roadmap (`docs/PERFORMANCE_ROADMAP.md`)
- AI telemetry analysis scoping doc

### Testing
- Integration tests: admin verbs (10), admin commands (21, in `crates/blit-cli/tests/blit_utils.rs`), remote transfers, transfer edges, parity, resume, move, remote-to-remote
- Unit tests: predictor regression (9), schema versioning (7), filesystem probing (7), mirror planner, checksum, enumeration
- GitHub Actions CI: fmt/clippy checks, tri-platform tests (Linux/macOS/Windows), release artifact builds

### Security
- Path traversal protection
- Block size limits
- Token verification (placeholder)
- Module-level read/write permissions
- No built-in TLS; operators use SSH tunnels, VPN, or reverse proxy

### Known Limitations
- TCP data plane throughput not yet benchmarked at 10+ Gbps (implementation complete, hardware testing pending)
- No built-in TLS encryption
- No authentication beyond module-level access control
- `find` uses substring matching, not glob patterns
- Windows ReFS block clone requires SeManageVolumePrivilege
