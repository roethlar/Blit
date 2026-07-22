# Blit 

Blit delivers a high-performance, extensible file enumeration, planning, transfer, and orchestration platform for robust local and remote backups, file migration, and cross-platform syncing. With both CLI and daemon interfaces, async-aware planning, and strong Windows support, Blit is designed for speed and reliability—whether run interactively or as part of automated workflows.

---

## Features

- **Modular Core Engine**
  Async file enumeration, planner, and one unified transfer session in `blit-core` for maximum performance and extensibility.
- **CLI and Daemon Binaries**
  Minimal, ergonomic command-line interface; full daemon/server for automation and concurrent requests.
- **Resumable Transfers**
  With `--resume`, eligible partial files continue through block-level Blake3 comparison across local, push, pull, and remote-to-remote transfers. Retries re-run the selected destination comparison (so normal comparison skips files now complete); partial-file continuation requires `--resume`.
- **Hybrid Transport**
  TCP data plane by default, with an in-stream gRPC carrier for diagnostics or when direct TCP is unavailable.
- **Platform Optimization**
  Windows, Linux, and macOS optimized; per-filesystem capability detection (reflink, sparse, xattr) with platform-native fast-copy paths (`clonefile`, `copy_file_range`, `CopyFileEx`).
- **gRPC API**
  Robust proto definitions in `proto/blit.proto` enable remote orchestration and integrations.
- **Security Boundaries**
  Path containment, bounded wire records, and per-session data-plane tokens. The daemon has no built-in TLS or user authentication; operate it only on a trusted network or through an SSH tunnel or VPN.
- **Admin Utilities**
  Built into `blit`: daemon inspection and maintenance via mDNS discovery, module listing, remote `ls`/`find`/`du`/`df`/`rm`, shell completions, and performance profiling.
- **Developer Experience**
  Robust test suite, clear repo organization, and scripting-friendly JSON output across all tools.
- **Extensive Documentation**
  Agent collaboration (`AGENTS.md`), process docs, session logs (`DEVLOG.md`), and roadmap (`TODO.md`).

---

## Repository Structure

```
.
├── crates/        # Rust workspace
│   ├── blit-core/
│   ├── blit-app/
│   ├── blit-cli/  # Produces `blit` (admin verbs included)
│   ├── blit-daemon/
│   ├── blit-prometheus-bridge/
│   └── blit-tui/
├── proto/         # gRPC (protobuf) definitions
├── scripts/       # Helper scripts (Windows, etc.)
├── test/          # Test data/resources
├── docs/          # Workflow/process docs
├── AGENTS.md      # Agent and collaboration framework
├── DEVLOG.md      # Development log/context
├── TODO.md        # Feature roadmap/tasks
└── report.xsl     # Output/report formatting
```

---

## Quick Start

### Prerequisites

- The current stable [Rust toolchain](https://www.rust-lang.org/tools/install)
  (the project does not declare an older minimum supported Rust version)
- No separate `protoc` install is required; the build uses a vendored compiler
- Windows, Linux, or macOS

### Building & Testing

```sh
git clone https://github.com/roethlar/Blit.git
cd Blit
cargo build                # Compile the full workspace
cargo test                 # Run all tests

# Windows (with scripting/log capture)
scripts/windows/run-blit-tests.ps1
```

### Usage

```sh
# Local mirror (copy + delete extraneous)
blit mirror ./source ./dest --yes

# Remote push to daemon
blit mirror ./local server:/module/ --yes

# Remote pull from daemon
blit mirror server:/module/ ./local --yes

# Remote-to-remote copy (daemon-to-daemon; destination daemon must allow delegation)
blit copy server-a:/module/data/ server-b:/module/data/

# Resume interrupted transfer (block-level comparison)
blit mirror ./source ./dest --resume

# Show progress
blit copy ./large_file.iso ./dest/ --progress
```

### Running the Daemon

```sh
blit-daemon --config /etc/blit/config.toml
```

See `docs/cli/blit-daemon.1.md` and `docs/DAEMON_CONFIG.md` for configuration details.

### Admin Utilities

```sh
# Discover daemons on the local network
blit scan

# List modules exported by a daemon
blit list-modules server

# Browse remote directories
blit ls server:/module/path

# Search for files on a remote daemon (glob pattern)
blit find server:/module/ --pattern "*.csv"

# Disk usage summary
blit du server:/module/path --json

# Filesystem statistics
blit df server:/module

# Remove remote files (with confirmation)
blit rm server:/module/path/old-data

# Show local performance history
blit profile

# Install shell-completion scripts (bash / zsh / fish / powershell / elvish)
blit completions shell bash > ~/.local/share/bash-completion/completions/blit
blit completions shell zsh  > "${fpath[1]}/_blit"
blit completions shell fish > ~/.config/fish/completions/blit.fish
```

`blit completions remote <REMOTE> [--prefix <STR>] [--files] [--dirs]`
is the daemon-backed remote-path completion the generated shell
scripts call internally — `<REMOTE>` is the target host (e.g.
`server:9031`) and `--prefix` narrows the returned path set. You
don't typically run it directly.
