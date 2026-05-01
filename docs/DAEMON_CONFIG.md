# Daemon Configuration Guide

This document describes how to configure the Blit daemon (`blit-daemon`) for
remote file transfer operations.

## Quick Start

```bash
# Start with command-line options
blit-daemon --bind 0.0.0.0 --port 9031 --root /data

# Start with configuration file
blit-daemon --config /etc/blit/config.toml
```

## Configuration File

The daemon reads configuration from a TOML file. Default locations:

| Platform | Path |
|----------|------|
| Linux/macOS | `/etc/blit/config.toml` |
| Windows | `C:\ProgramData\Blit\config.toml` |

The file is loaded automatically if it exists at the default location. Use
`--config <PATH>` to specify a custom path.

### Full Configuration Example

```toml
# /etc/blit/config.toml

[daemon]
bind = "0.0.0.0"
port = 9031
motd = "Welcome to Blit file server"

# mDNS service discovery
no_mdns = false
mdns_name = "my-file-server"

# Optional: export a default root for server:// requests
# root = "/srv/blit"
# root_read_only = true
# root_use_chroot = false

# Optional: disable server-side checksum computation
# no_server_checksums = false

[[module]]
name = "backup"
path = "/data/backups"
read_only = false
comment = "Primary backup storage"

[[module]]
name = "media"
path = "/srv/media"
read_only = true
comment = "Media library (read-only)"

[[module]]
name = "home"
path = "/home/shared"
read_only = false
use_chroot = false
```

### Configuration Reference

#### `[daemon]` Section

| Option | Type | Default | Description |
|--------|------|---------|-------------|
| `bind` | string | `"0.0.0.0"` | IP address to bind the gRPC server |
| `port` | integer | `9031` | Port for gRPC connections |
| `motd` | string | none | Message displayed to clients on connect |
| `no_mdns` | boolean | `false` | Disable mDNS service advertisement |
| `mdns_name` | string | `blit@<hostname>` | Custom mDNS instance name |
| `root` | string | none | Default export path for `server://` requests |
| `root_read_only` | boolean | `false` | Make the default root export read-only |
| `root_use_chroot` | boolean | `false` | Enable chroot for the default root |
| `no_server_checksums` | boolean | `false` | Disable server-side checksum computation |

#### `[[module]]` Array

Modules define named export points that clients can access. Each module is
declared with `[[module]]` (TOML array-of-tables syntax).

| Option | Type | Default | Description |
|--------|------|---------|-------------|
| `name` | string | required | Module name (used in remote paths) |
| `path` | string | required | Absolute filesystem path to export |
| `read_only` | boolean | `false` | Prevent write operations |
| `comment` | string | none | Description shown in module listings |
| `use_chroot` | boolean | `false` | Enable chroot for this module |

Module names must be non-empty and unique within the configuration.

## Command-Line Options

Command-line options override configuration file settings.

```
blit-daemon [OPTIONS]

Options:
    --config <PATH>          Path to TOML configuration file
    --bind <ADDR>            Override bind address
    --port <PORT>            Override port (default: 9031)
    --root <PATH>            Export root path (for server:// requests)
    --no-mdns                Disable mDNS advertisement
    --mdns-name <NAME>       Override mDNS instance name
    --force-grpc-data        Force gRPC data plane (disable TCP)
    --no-server-checksums    Disable server-side checksums
    -h, --help               Print help
```

### Priority

When both a config file and CLI arguments are provided, CLI flags take
precedence:

1. CLI flag value (highest priority)
2. Config file value
3. Built-in default

## Access Patterns

### Module-Based Access

With modules configured, clients access paths via module names:

```bash
# Push to the "backup" module
blit copy /local/path server:/backup/subdir --yes

# Pull from the "media" module
blit copy server:/media/videos /local/videos

# Mirror to a specific host and port
blit mirror /local/path myserver:9031:/backup/ --yes
```

### Root-Based Access

Without modules (using `--root`), the daemon exports the root path as a
module named `default`:

```bash
# Start daemon with root export
blit-daemon --root /data

# Client access via default module
blit copy /local/path server:/default/subdir --yes
```

If no modules and no `--root` are specified, the daemon exports its current
working directory as `default` and emits a warning.

### Remote URL Syntax

```
host:/module/path       # Explicit module + path
host:port:/module/path  # With custom port
host                    # Bare host (for list-modules, scan)
```

Remote paths must use forward slashes (`/`), not backslashes.

## Client Configuration

The `blit` CLI stores settings and performance data
in a platform-specific configuration directory:

| Platform | Default Path |
|----------|-------------|
| macOS | `~/Library/Application Support/Blit/Blit/` |
| Linux | `$XDG_CONFIG_HOME/Blit/` or `~/.config/Blit/` |
| Windows | `C:\Users\<user>\AppData\Local\Blit\Blit\` |
| Fallback | `~/.config/blit/` |

Override with `--config-dir <PATH>` on the CLI.

**Files stored:**

| File | Purpose |
|------|---------|
| `settings.json` | Performance history toggle |
| `perf_local.jsonl` | Transfer performance records (~1 MiB cap) |
| `journal_cache.json` | Change journal checkpoints (Windows USN, macOS FSEvents, Linux metadata) |

## Security Recommendations

### Network Binding

```toml
[daemon]
# Production: bind to specific interface
bind = "10.0.0.5"

# Development only: bind to all interfaces
bind = "0.0.0.0"

# Localhost only: access via SSH tunnel
bind = "127.0.0.1"
```

### Transport Security

The daemon does not implement built-in TLS. Secure remote transfers via:

- **SSH tunnel**: Forward the daemon port through SSH:
  ```bash
  ssh -L 9031:localhost:9031 remote-host
  blit copy /local/path localhost:/module/path
  ```

- **VPN**: Connect clients and servers via an encrypted VPN tunnel.

- **Reverse proxy with TLS**: Place the daemon behind a TLS-terminating
  reverse proxy (e.g., nginx, Caddy).

- **Trusted network**: Run daemons only on isolated, trusted networks.

### Read-Only Modules

For directories that should never be modified:

```toml
[[module]]
name = "archives"
path = "/data/archives"
read_only = true
```

### Firewall Configuration

```bash
# Linux (firewalld)
firewall-cmd --add-port=9031/tcp --permanent
firewall-cmd --reload

# Linux (ufw)
ufw allow 9031/tcp

# macOS
sudo /usr/libexec/ApplicationFirewall/socketfilterfw --add /usr/local/bin/blit-daemon

# Windows PowerShell
New-NetFirewallRule -DisplayName "Blit Daemon" -Direction Inbound -Port 9031 -Protocol TCP -Action Allow
```

## Service Installation

### Linux (systemd)

Create `/etc/systemd/system/blit-daemon.service`:

```ini
[Unit]
Description=Blit File Transfer Daemon
After=network.target

[Service]
Type=simple
ExecStart=/usr/local/bin/blit-daemon --config /etc/blit/config.toml
Restart=on-failure
RestartSec=5

# Security hardening
User=blit
Group=blit
NoNewPrivileges=yes
ProtectSystem=strict
ProtectHome=read-only
ReadWritePaths=/data/backups /home/shared

[Install]
WantedBy=multi-user.target
```

Enable and start:

```bash
sudo systemctl daemon-reload
sudo systemctl enable blit-daemon
sudo systemctl start blit-daemon
sudo systemctl status blit-daemon
```

### macOS (launchd)

Create `~/Library/LaunchAgents/com.blit.daemon.plist`:

```xml
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>Label</key>
    <string>com.blit.daemon</string>
    <key>ProgramArguments</key>
    <array>
        <string>/usr/local/bin/blit-daemon</string>
        <string>--config</string>
        <string>/etc/blit/config.toml</string>
    </array>
    <key>RunAtLoad</key>
    <true/>
    <key>KeepAlive</key>
    <true/>
    <key>StandardOutPath</key>
    <string>/tmp/blit-daemon.stdout.log</string>
    <key>StandardErrorPath</key>
    <string>/tmp/blit-daemon.stderr.log</string>
</dict>
</plist>
```

Load the service:

```bash
launchctl load ~/Library/LaunchAgents/com.blit.daemon.plist
```

For system-wide service (runs as root):

```bash
sudo cp com.blit.daemon.plist /Library/LaunchDaemons/
sudo launchctl load /Library/LaunchDaemons/com.blit.daemon.plist
```

### Windows (NSSM)

Using [NSSM](https://nssm.cc/) (Non-Sucking Service Manager):

```powershell
# Install service
nssm install BlitDaemon "C:\Program Files\Blit\blit-daemon.exe"
nssm set BlitDaemon AppParameters "--config C:\ProgramData\Blit\config.toml"
nssm set BlitDaemon AppDirectory "C:\Program Files\Blit"

# Start service
nssm start BlitDaemon
```

## mDNS Discovery

By default, the daemon advertises itself on the local network via mDNS
(`_blit._tcp.local.`) so clients can discover it with `blit scan` or
`blit scan`.

The mDNS TXT record includes:
- `version` — daemon version
- `modules` — comma-separated list of exported module names (truncated to 180 chars)

The instance name defaults to `blit@<hostname>`. Override with `--mdns-name`
or `mdns_name` in the config file.

Disable with `--no-mdns` or `no_mdns = true`.

## Monitoring

### Logging

```bash
# Default logging
blit-daemon --config /etc/blit/config.toml

# Debug logging
RUST_LOG=debug blit-daemon --config /etc/blit/config.toml

# Trace logging (very verbose)
RUST_LOG=trace blit-daemon --config /etc/blit/config.toml 2>&1 | tee daemon.log
```

Log levels: `error`, `warn`, `info` (default), `debug`, `trace`.

### Health Checks

```bash
# List modules (verifies daemon is responding)
blit list-modules localhost

# Or via blit CLI
blit list localhost
```

## Troubleshooting

**Daemon won't start**
- Check port availability: `lsof -i :9031` (Unix) or `netstat -an | findstr 9031` (Windows)
- Verify config file syntax with a TOML linter
- Check file permissions on module paths
- Run with `RUST_LOG=debug` for detailed output

**Clients can't connect**
- Verify firewall rules allow port 9031
- Check bind address — `127.0.0.1` only accepts local connections
- Test with `blit list-modules localhost` first

**mDNS not working**
- Ensure `no_mdns` is not set to `true`
- Check that multicast is enabled on the network
- macOS: Bonjour is built-in; Linux: ensure `avahi-daemon` is running

**Permission denied on transfers**
- Check daemon user has read/write access to module paths
- Verify `read_only` setting for write operations
- Check filesystem permissions and ownership

## Performance Tuning

### TCP Data Plane

The daemon uses a hybrid transport: gRPC for control and TCP for bulk data.
This achieves higher throughput than gRPC alone.

```bash
# Force gRPC-only mode (slower, for debugging or restrictive networks)
blit-daemon --force-grpc-data

# Default: hybrid mode with TCP data plane (recommended)
blit-daemon
```

### Storage

- Place modules on fast storage (SSD/NVMe) for best performance
- Use local filesystems; NFS/CIFS add latency
- Consider RAID for sustained throughput

### Network

For high-throughput environments (10+ GbE):
- Enable jumbo frames if supported
- Consider dedicated network interface for transfers
- Tune TCP buffer sizes at OS level:
  ```bash
  # Linux
  sysctl -w net.core.rmem_max=16777216
  sysctl -w net.core.wmem_max=16777216
  ```
