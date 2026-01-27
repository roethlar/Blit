# Blit

A fast, high-performance file transfer and synchronization tool built in Rust.

## Overview

Blit is a modern file transfer tool designed for efficient local and remote file operations. It features intelligent change detection, parallel transfers, and platform-specific optimizations for maximum performance.

### Key Features

- **High-Performance Transfers**: Parallel file copying with platform-native optimizations
- **Smart Synchronization**: Mirror mode with deletion support for true directory mirroring
- **Remote Operations**: gRPC-based daemon for secure remote file transfers
- **Change Detection**: Platform-specific change journals (Windows USN, macOS FSEvents, Linux)
- **Zero-Copy Support**: Kernel-level optimizations where available
- **Cross-Platform**: Windows, macOS, and Linux support

## Installation

### From Source

```bash
# Clone the repository
git clone https://github.com/your-org/blit.git
cd blit

# Build release binaries
cargo build --release

# Binaries are located in target/release/
# - blit-cli    (command-line interface)
# - blit-daemon (remote transfer daemon)
# - blit-utils  (administrative utilities)
```

### Prerequisites

- Rust 1.70+ (with cargo)
- Windows: Visual Studio Build Tools with C++ workload
- macOS: Xcode Command Line Tools
- Linux: GCC and standard development libraries

## Quick Start

### Local File Operations

```bash
# Copy a directory
blit-cli copy /path/to/source /path/to/destination

# Mirror a directory (sync with deletions)
blit-cli mirror /path/to/source /path/to/destination

# Move a directory
blit-cli move /path/to/source /path/to/destination

# Dry run to preview changes
blit-cli mirror --dry-run /path/to/source /path/to/destination
```

### Remote File Operations

```bash
# Start the daemon on the remote machine
blit-daemon --bind 0.0.0.0 --port 50051 --root /data

# Copy files to a remote server
blit-cli copy /local/path server://192.168.1.100:50051/remote/path

# Pull files from a remote server
blit-cli copy server://192.168.1.100:50051/remote/path /local/path

# Mirror to remote (with deletions)
blit-cli mirror /local/path server://192.168.1.100:50051/backup
```

### Service Discovery

```bash
# Discover daemons on the local network via mDNS
blit-cli scan

# List available modules on a daemon
blit-cli list server://192.168.1.100:50051
```

## Command Reference

### blit-cli

The main command-line interface for file transfer operations.

| Command | Description |
|---------|-------------|
| `copy` | Copy files between local and/or remote locations |
| `mirror` | Mirror a directory (including deletions at destination) |
| `move` | Move a directory or file (mirror + remove source) |
| `scan` | Discover daemons advertising via mDNS |
| `list` | List modules or paths on a remote daemon |
| `du` | Show disk usage for a remote path |
| `df` | Show filesystem statistics for a remote module |
| `rm` | Remove a file or directory on a remote daemon |
| `find` | Search for files on a remote daemon |
| `diagnostics` | Diagnostics and performance tooling |

#### Common Options

| Option | Description |
|--------|-------------|
| `--dry-run` | Preview changes without executing |
| `--checksum` | Force checksum comparison of files |
| `-v, --verbose` | Enable verbose logging |
| `-p, --progress` | Show interactive progress indicator |
| `-y, --yes` | Skip confirmation for destructive operations |
| `--force-grpc` | Force gRPC data path instead of TCP |
| `--config-dir <PATH>` | Override configuration directory |

### blit-daemon

The remote transfer daemon that handles incoming connections.

```bash
blit-daemon [OPTIONS]
```

| Option | Description |
|--------|-------------|
| `--config <PATH>` | Path to TOML configuration file |
| `--bind <ADDR>` | Host/IP address to bind |
| `--port <PORT>` | Port to bind |
| `--root <PATH>` | Exported root path for server:// URLs |
| `--no-mdns` | Disable mDNS advertisement |
| `--mdns-name <NAME>` | Override advertised mDNS instance name |
| `--force-grpc-data` | Force gRPC data plane instead of TCP |

### blit-utils

Administrative utilities for daemon management and diagnostics.

| Command | Description |
|---------|-------------|
| `scan` | Discover daemons via mDNS |
| `list-modules` | List modules exported by a daemon |
| `ls` | List directory entries (remote or local) |
| `find` | Recursive find for remote paths |
| `du` | Disk usage summary for a remote subtree |
| `df` | Filesystem stats for a remote module |
| `rm` | Remove files/directories remotely |
| `completions` | Fetch remote path completions for shells |
| `profile` | Show local performance history summary |

## Configuration

### Daemon Configuration

Create a TOML configuration file for the daemon:

```toml
# /etc/blit/config.toml

bind_address = "0.0.0.0"
port = 50051
motd = "Welcome to Blit file server"
no_mdns = false
mdns_name = "my-blit-server"

[modules.backup]
path = "/data/backups"
read_only = false

[modules.media]
path = "/srv/media"
read_only = true
```

### Client Configuration

Client configuration is stored in the user's config directory:
- Windows: `%APPDATA%\blit\`
- macOS: `~/Library/Application Support/blit/`
- Linux: `~/.config/blit/`

## Architecture

Blit uses a modular architecture with clear separation of concerns:

```
blit/
├── crates/
│   ├── blit-core/      # Core library (transfer engine, protocols)
│   ├── blit-cli/       # Command-line interface
│   ├── blit-daemon/    # Remote transfer daemon
│   └── blit-utils/     # Administrative utilities
└── proto/
    └── blit.proto      # gRPC service definitions
```

### Core Modules

| Module | Purpose |
|--------|---------|
| `transfer_engine` | Orchestrates file transfer operations |
| `orchestrator` | Coordinates parallel transfers with fast-path routing |
| `mirror_planner` | Computes file differences for synchronization |
| `change_journal` | Platform-specific change detection |
| `remote` | gRPC client/server for remote operations |
| `copy` | Platform-optimized file copying |
| `checksum` | File integrity verification (Blake3, XXHash, MD5) |

### Remote Protocol

Blit uses gRPC for control-plane communication with an optional high-performance TCP data plane for bulk transfers.

**Services:**
- `Push` - Bidirectional streaming for client-to-server transfers
- `Pull` - Server-side streaming for server-to-client transfers
- `List` - Directory listing
- `Purge` - Remote file deletion for mirror operations
- `Find` - Recursive file search
- `DiskUsage` - Storage analysis
- `FilesystemStats` - Capacity information

## Performance Features

### Platform Optimizations

| Platform | Optimization |
|----------|--------------|
| Windows | USN Change Journal, Block Cloning (ReFS), native APIs |
| macOS | FSEvents, `clonefile()`, `fcopyfile()` |
| Linux | Standard POSIX with parallel I/O |

### Transfer Optimizations

- **Parallel Workers**: Configurable worker threads (defaults to CPU count)
- **Zero-Copy**: Kernel-level data transfer where available
- **Smart Batching**: Small files grouped into tar shards for efficiency
- **Performance Prediction**: Historical data used to optimize transfer strategies

## Examples

### Backup Workflow

```bash
# Initial backup
blit-cli copy ~/Documents server://backup-server:50051/backups/docs

# Incremental sync (only changed files)
blit-cli copy ~/Documents server://backup-server:50051/backups/docs

# Full mirror (removes deleted files from backup)
blit-cli mirror ~/Documents server://backup-server:50051/backups/docs -y
```

### Remote Administration

```bash
# Check disk usage on remote
blit-cli du server://192.168.1.100:50051/data

# Check filesystem capacity
blit-cli df server://192.168.1.100:50051/backup

# Find large files
blit-cli find server://192.168.1.100:50051/data --pattern "*.log"

# Remove old files (with confirmation)
blit-cli rm server://192.168.1.100:50051/data/old-backups
```

### Dry Run and Verification

```bash
# Preview what would be transferred
blit-cli mirror --dry-run /source /destination

# Force checksum verification (slower but ensures integrity)
blit-cli copy --checksum /source /destination

# Verbose output for debugging
blit-cli copy -v /source /destination
```

## Development

### Building

```bash
# Debug build
cargo build

# Release build (optimized)
cargo build --release

# Run tests
cargo test

# Run with verbose test output
cargo test -- --nocapture
```

### Project Structure

```
crates/blit-core/src/
├── lib.rs                 # Core library exports
├── transfer_engine.rs     # Main transfer orchestration
├── mirror_planner.rs      # Synchronization planning
├── orchestrator/          # Parallel transfer coordination
├── remote/                # gRPC client/server implementation
├── copy/                  # Platform-specific copy operations
├── change_journal/        # OS change detection
├── checksum.rs            # File integrity verification
└── config.rs              # Configuration handling
```

## Troubleshooting

### Common Issues

**Connection refused to remote daemon**
- Verify the daemon is running: `blit-cli scan`
- Check firewall rules for the configured port (default: 50051)
- Ensure the bind address is accessible

**Slow transfers**
- Use `--progress` to monitor transfer speed
- Check `blit-cli diagnostics perf` for historical performance data
- Consider using `--force-grpc` if TCP data plane has issues

**Permission denied errors**
- Ensure the daemon has read/write access to configured paths
- On Windows, run with appropriate privileges for change journal access

### Debug Mode

```bash
# Enable verbose logging
RUST_LOG=debug blit-cli copy /source /destination

# Daemon with debug logging
RUST_LOG=debug blit-daemon --config /etc/blit/config.toml
```

## License

[Specify your license here]

## Contributing

[Add contribution guidelines]
