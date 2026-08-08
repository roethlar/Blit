# Changelog

All notable changes to Blit are documented in this file.

## [0.1.2] - 2026-08-08

A patch version number, but not a small release: this covers everything
landed since 0.1.1 (tagged 2026-07-23). The headline items are per-file
failure containment, a large reduction in comparison cost on real mirror
workloads, live terminal progress, code-signed macOS and Windows binaries,
and one user-visible change to what the default comparison checks. Read
"Behavior change" below even if you skip the rest.

### Behavior change: default compare no longer inspects destination streams

Every run — including a no-op run over an already-mirrored tree — used to
ask the destination whether each file that already matched by size and
modification time also had matching Windows named `$DATA` streams. That
check is gone from the default comparison.

- The default comparison is now size + modification time only. A stray or
  altered named stream on an otherwise size/mtime-matched destination file
  can go undetected until a run made with `--checksum`.
- `--checksum` is unchanged and still performs the exhaustive per-file
  comparison, streams included.
- Streams are still carried whenever a file's content actually transfers.
  This affects detection of an out-of-band stream change on a file blit
  otherwise considers unchanged — not stream fidelity on copy.
- File attributes are unaffected: they are still compared, and an
  attribute-only divergence is now repaired in place (see Reliability).
- Rationale: the remedy for a detected stream divergence was always
  whole-file replacement, so the check never protected the destination
  data — it only found it sooner in order to overwrite it sooner, and it
  accounted for most of the comparison cost on a converged mirror.

### Reliability: one bad file no longer aborts the whole transfer

Previously a single file blit could not write — locked, permission denied,
an attribute it could not apply — ended the entire transfer, discarding
work already done and everything still queued.

- A file that fails is recorded and skipped; the rest of the transfer
  proceeds. Containment covers single-file paths, streaming receive,
  tar-batched shards, and resume block patching, on both local and remote
  routes.
- Runs that completed with failures print an end-of-run block naming the
  failed files and reasons (capped list, with an elided-count note and a
  re-run hint) and exit with status **2** — distinct from clean success
  (`0`) and from a hard abort, which keeps its own non-zero status.
  `--json` output carries both signals: the status and a `files_failed`
  count with a `failures` list.
- **`move` never deletes a source file whose copy did not land.** This
  gate now covers every move route, CLI and TUI. Re-running converges.
- Attribute-only divergence is repaired in place with no file content
  re-sent. This also fixes mirroring to a default-configuration Samba
  share, which synthesizes a HIDDEN attribute on dot-named files that
  previously could never converge.
- Volume-level problems deliberately stay fatal rather than being
  contained per file: a read-only destination mount or an exhausted volume
  ends the session instead of reporting thousands of individually failed
  files.
- Fixed (Linux/macOS): when a path component was an existing file rather
  than a directory, that could abort a whole batch instead of failing only
  the affected entry.

### Performance

Measured on one real workload — roughly 46,000 files / ~8 GB mirrored to
an SMB share — on the maintainer's hardware. These are that workload's
numbers, not a general guarantee.

- **Converged (no-op) SMB mirror check: 283.92 s → 55.57 s, about 5.1x.**
  This is comparison cost, from four changes together: reusing file
  metadata already read during enumeration instead of re-reading it per
  file, one directory scan per destination directory instead of one stat
  per file, a self-tuning concurrency level for the comparison stage, and
  dropping the destination stream check described above.
- **Local-to-local mirror of the same tree: 17.68 s → 8.70 s, about 2x**,
  from running the apply stage's writes in parallel instead of on one
  worker.
- All of it is automatic. No configuration was added and none is
  available: blit works these values out at runtime.

### CLI and terminal output

- Running with `-p` shows one stable in-place progress row — phase, files
  and bytes so far, current file — instead of a spinner that other output
  could scroll away. `-v` per-file lines scroll cleanly above it, and
  warnings no longer erase it.
- The phase names tell the truth about what is happening (enumerating,
  comparing, copying, deleting), and a `--dry-run` never claims to delete.
- The live row and the failure block are colour-coded. Colour is disabled
  automatically for `NO_COLOR`, `TERM=dumb`, non-interactive output, and
  `--json`; redirected output is byte-identical to before. There is no
  `--color` flag and no theming.
- The end-of-run summary separates files copied from files repaired, and
  labels its transfer rate as a whole-run average.
- The enumeration heartbeat on long scans is now quiet by default and
  audible under `--verbose` or `-p`, on both local and remote push routes.

### Packages, signing, and compatibility

- **macOS and Windows binaries in the release archives are code-signed for
  the first time.** macOS: Developer ID Application signature with
  hardened runtime and a secure timestamp, and the binaries are notarized
  by Apple. Windows: Authenticode via Azure Trusted Signing with an
  RFC 3161 timestamp. Linux binaries are not signed.
- Two honest caveats about that signing:
  - **Windows SmartScreen may still warn.** The signature is genuine and
    verifiable, but SmartScreen's "reputation" is earned from download
    volume over time, so a fresh certificate can still produce a warning
    on first download. Check the publisher in the file's Digital
    Signatures tab rather than relying on the absence of a prompt.
  - **The macOS notarization ticket is not stapled**, because a flat
    executable has no bundle to hold one. Gatekeeper resolves the ticket
    online instead, so a first run on a machine with no network path to
    Apple can still be blocked even though the binary is notarized.
- Releases are now built, signed, smoke-tested, and attached to the tag by
  CI rather than uploaded by hand. Publication is atomic: all three
  platform legs must succeed or no release is created or modified at all.
- Archive contents are unchanged in shape: `blit` and `blit-daemon` for
  `x86_64-unknown-linux-gnu`, `aarch64-apple-darwin`, and
  `x86_64-pc-windows-msvc`, each archive with a SHA-256 sidecar and the
  exact source commit in `BUILD.txt`.
- Both executables report the same `0.1.2+<commit>` build identity, and
  remote sessions still refuse peers whose build identity differs. The
  wire contract version moved 5 → 6 (the summary now carries the failure
  report), so a 0.1.2 peer will not transfer with a 0.1.1 peer.

### Not in this release

Named here because the code is present in the tree and could otherwise be
mistaken for a shipped feature:

- **There is no GUI, and no new TUI.** Work on the Blit Console has begun
  as a library crate (`blit-console-core`: endpoint model, browse state,
  message/update loop, LAN daemon discovery, remote daemon browse), but no
  binary depends on it and none of it is reachable from any command. It
  ships in the source tree, not in any user-facing surface.
- Specifically, the daemon discovery and remote-browse functions inside
  that crate are **not** a new user feature. The shipped ways to find and
  browse daemons are the ones 0.1.1 already had and they are unchanged:
  `blit scan` (mDNS), `blit list <host>`, and `blit ls <host:module/path>`.
- The legacy TUI's F3 pane gained an internal picker mode with no caller
  and no keybinding — nothing opens it. TUI behavior is unchanged.
- Some default-off timing instrumentation was added for latency
  diagnostics. It is off unless explicitly enabled and changes nothing
  about a normal run.

### Known limitations

- **Non-UTF-8 source filenames fail to transfer.** A filename that is
  valid on Linux/ext4 but not valid UTF-8 is corrupted in blit's internal
  path handling. On LOCAL transfers the failure is contained to that one
  file (the run continues and exits 2, per Reliability above). On REMOTE
  transfers it is **not** contained: the corrupted path can fail payload
  preparation at the source and abort the whole session. The only current
  workaround is renaming the affected files.
- **`--exclude` only matches source-relative paths, and a directory
  pattern does not match that directory's contents.** An absolute path, or
  a bare directory name meant to exclude everything beneath it, silently
  matches nothing — the pattern is accepted and no warning is issued. Use
  a source-relative glob with an explicit subtree wildcard, e.g.
  `--exclude '.java/**'` rather than `--exclude .java`.
- Blit still has no built-in TLS and no user authentication. Module access
  controls and per-session data-plane tokens are not an authentication
  system. Run the daemon only on a trusted network or through an
  operator-managed VPN or SSH tunnel.
- Throughput is not a release claim. Hardware ceiling work, small-file
  tuning, and zero-copy receive remain open.
- Cross-OS daemon transfers were not smoke-tested as a matrix for this
  release. Automated testing covers all three platforms and same-OS
  loopback daemon transfers; the six directed cross-OS pairs were not run.

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
