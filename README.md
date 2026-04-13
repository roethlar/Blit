# Blit Workspace

![Rust](https://img.shields.io/badge/Rust-2021-brightgreen?logo=rust)
![Build](https://img.shields.io/badge/build-passing-green)
![Tests](https://img.shields.io/badge/tests-passing-blue)
![Windows Supported](https://img.shields.io/badge/Windows-Supported-blue?logo=windows)
![Linux Supported](https://img.shields.io/badge/Linux-Supported-brightgreen?logo=linux)
![License: MIT](https://img.shields.io/badge/license-MIT-blue)

---

Blit delivers a high-performance, extensible file enumeration, planning, transfer, and orchestration platform for robust local and remote backups, file migration, and cross-platform syncing. With both CLI and daemon interfaces, async-aware planning, and strong Windows support, Blit is designed for speed and reliability—whether run interactively or as part of automated workflows.

---

## Features

- **Modular Core Engine**
  Async file enumeration, planner, transfer, and orchestrator modules in `blit-core` for maximum performance and extensibility.
- **CLI and Daemon Binaries**
  Minimal, ergonomic command-line interface; full daemon/server for automation and concurrent requests.
- **Resumable Transfers**
  Block-level resume with Blake3 hashing (`--resume`). Interrupted transfers continue from where they stopped; only changed blocks are transferred.
- **Hybrid Transport**
  TCP data plane for high-throughput transfers (10+ Gbps), with gRPC fallback for diagnostics.
- **Platform Optimization**
  Windows, Linux, and macOS optimized; per-filesystem capability detection (reflink, sparse, xattr) with platform-native fast-copy paths (`clonefile`, `copy_file_range`, `CopyFileEx`).
- **gRPC API**
  Robust proto definitions in `proto/blit.proto` enable remote orchestration and integrations.
- **Security Hardened**
  Path traversal protection, block size limits, token verification. TLS via operator-provided SSH tunnels or VPN.
- **Admin Utilities**
  `blit-utils` provides daemon inspection and maintenance: mDNS discovery, module listing, remote `ls`/`find`/`du`/`df`/`rm`, shell completions, and performance profiling.
- **Developer Experience**
  Robust test suite, clear repo organization, and scripting-friendly JSON output across all tools.
- **Extensive Documentation**
  Agent collaboration (`AGENTS.md`), process docs, session logs (`DEVLOG.md`), and roadmap (`TODO.md`).

---

## Repository Structure

```
.
├── crates/        # Rust workspace: core lib, CLI, daemon, utils
│   ├── blit-core/
│   ├── blit-cli/
│   ├── blit-daemon/
│   └── blit-utils/
├── proto/         # gRPC (protobuf) definitions
├── scripts/       # Helper scripts (Windows, etc.)
├── tests/         # Integration test suite
├── test/          # Test data/resources
├── docs/          # Workflow/process docs
├── AGENTS.md      # Agent and collaboration framework
├── DEVLOG.md      # Development log/context
├── TODO.md        # Feature roadmap/tasks
├── Windows_Build_Failures.txt # Special issues log
└── report.xsl     # Output/report formatting
```

---

## Quick Start

### Prerequisites

- [Rust](https://www.rust-lang.org/tools/install) 1.56+ (edition 2021)
- For gRPC: [protoc](https://grpc.io/docs/protoc-installation/) (auto-handled for most workflows)
- Windows, Linux, or macOS

### Building & Testing

```sh
git clone https://github.com/your_org/blit.git
cd blit
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
blit-utils scan

# List modules exported by a daemon
blit-utils list-modules server

# Browse remote directories
blit-utils ls server:/module/path

# Search for files on a remote daemon
blit-utils find server:/module/ --pattern ".csv"

# Disk usage summary
blit-utils du server:/module/path --json

# Filesystem statistics
blit-utils df server:/module

# Remove remote files (with confirmation)
blit-utils rm server:/module/path/old-data

# Show local performance history
blit-utils profile