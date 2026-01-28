# Blit Workspace

![Rust](https://img.shields.io/badge/Rust-2021-brightgreen?logo=rust)
![Build](https://img.shields.io/badge/build-passing-green)
![Tests](https://img.shields.io/badge/tests-passing-blue)
![Windows Supported](https://img.shields.io/badge/Windows-Supported-blue?logo=windows)
![Linux Supported](https://img.shields.io/badge/Linux-Supported-brightgreen?logo=linux)
![Open Source](https://img.shields.io/badge/license-choose_yours-important)

---

Blit delivers a high-performance, extensible file enumeration, planning, transfer, and orchestration platform for robust local and remote backups, file migration, and cross-platform syncing. With both CLI and daemon interfaces, async-aware planning, and strong Windows support, Blit is designed for speed and reliability—whether run interactively or as part of automated workflows.

---

## Features

- **Modular Core Engine**  
  Async file enumeration, planner, transfer, and orchestrator modules in `blit-core` for maximum performance and extensibility.
- **CLI and Daemon Binaries**  
  Minimal, ergonomic command-line interface; full daemon/server for automation and concurrent requests.
- **Platform Optimization**  
  Windows- and Linux-friendly; dedicated scripts and test logs for platform parity.
- **gRPC API**  
  Robust proto definitions in `proto/blit.proto` enable remote orchestration and integrations.
- **Developer Experience**  
  Utilities for scripting, automated checks, robust tests, and clear repo organization.
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
- Windows or Linux (see notes below)

### Building & Testing

```sh
git clone https://github.com/your_org/blit.git
cd blit
cargo build                # Compile the full workspace
cargo test                 # Run all tests

# Windows (with scripting/log capture)
scripts/windows/run-blit-tests.ps1
```

### Running the CLI

```sh
cargo run -p blit-cli -- --help                # Show available options

# Example: Enumerate and transfer a folder (dummy args)
cargo run -p blit-cli -- enumerate --src ./input_dir --dest ./output_dir
```

### Running the Daemon

```sh
cargo run -p blit-daemon                        # Launch the server for background transfers
# See API docs in proto/blit.proto or use gRPC client for orchestration