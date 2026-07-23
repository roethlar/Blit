# Changelog

All notable changes to Blit are documented in this file.

## [0.1.1] - 2026-07-23

### Packages and compatibility

- Release archives contain `blit` and `blit-daemon` for
  `x86_64-unknown-linux-gnu`, `aarch64-apple-darwin`, and
  `x86_64-pc-windows-msvc`; each archive has a SHA-256 sidecar and embeds the
  exact source commit in `BUILD.txt`.
- Both executables report the same exact `0.1.1+<commit>` build identity.
  Remote sessions require identical build identities and refuse mixed builds
  before transferring data.
- Packaged-release validation checks safe extraction, checksum and build
  identity, CLI/daemon startup, one tiny local copy, and one tiny loopback
  remote copy with exact byte comparison and bounded teardown.

### Transfer correctness and operation

- Local, push, pull, and remote-to-remote operations now use one role-based
  transfer session instead of separate direction-specific transfer engines.
- Windows file attributes and named `$DATA` streams are preserved across
  supported local, TCP, in-stream, tar-batched, and resumed transfers. A
  non-Windows destination refuses Windows metadata unless the operator
  explicitly selects the warned metadata-drop option.
- Transfer progress now carries declared and completed file/byte totals, live
  served and delegated byte counts, and the final carrier through daemon state,
  events, persisted recent rows, CLI output, and TUI output.
- Retry re-runs destination comparison and skips files that already completed;
  `--resume` additionally enables block-level continuation for eligible partial
  files in every transfer layout.
- Failure handling now preserves the first actionable file/worker fault across
  shutdown races, bounds network and child-process waits, and reports daemon
  startup diagnostics without blocking on stderr.

### Security and known limitations

- Blit has no built-in TLS and no user authentication. Module access controls
  and per-session data-plane tokens are not an authentication system. Run the
  daemon only on a trusted network or through an operator-managed VPN or SSH
  tunnel.
- These packages are correctness- and smoke-tested on the three listed target
  triples. Hardware throughput ceilings are not release claims. Mac-to-Mac
  Thunderbolt testing, further small-file tuning, and zero-copy optimization
  remain post-release work.

## [0.1.0] - 2026-05-31

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
- `blit list <bare-host>` smart-dispatches to module listing; `blit list <module/path>` falls through to `ls`
- `find --pattern <GLOB>` uses POSIX shell-glob syntax (`*`, `?`, `[abc]`, `**/`); `*` does not cross `/`. Pattern matches against both the relative path and the file-name basename so `*.csv` finds nested entries.
- `blit completions shell <SHELL>` generates static bash/zsh/fish/powershell/elvish completion scripts via `clap_complete`; pipe to your shell's completion directory.
- `blit completions remote <PREFIX>` is the existing daemon-backed remote-path completion; called internally by the generated shell scripts.
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
- Windows ReFS block clone requires SeManageVolumePrivilege
