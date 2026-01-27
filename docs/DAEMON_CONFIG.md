# Daemon Configuration Guide

This document describes how to configure the Blit daemon (`blit-daemon`) for remote file transfer operations.

## Quick Start

```bash
# Start with command-line options
blit-daemon --bind 0.0.0.0 --port 50051 --root /data

# Start with configuration file
blit-daemon --config /etc/blit/config.toml
```

## Configuration File

The daemon reads configuration from a TOML file. Default location: `/etc/blit/config.toml`

### Full Configuration Example

```toml
# /etc/blit/config.toml

# Server binding configuration
bind_address = "0.0.0.0"
port = 50051

# Message of the day (displayed to connecting clients)
motd = "Welcome to Blit file server"

# mDNS service discovery
no_mdns = false
mdns_name = "my-file-server"

# Module definitions
[modules.backup]
path = "/data/backups"
read_only = false

[modules.media]
path = "/srv/media"
read_only = true

[modules.home]
path = "/home/shared"
read_only = false
```

### Configuration Options

#### Server Settings

| Option | Type | Default | Description |
|--------|------|---------|-------------|
| `bind_address` | string | `"127.0.0.1"` | IP address to bind the gRPC server |
| `port` | integer | `50051` | Port for gRPC connections |
| `motd` | string | none | Message displayed to clients on connect |

#### mDNS Settings

| Option | Type | Default | Description |
|--------|------|---------|-------------|
| `no_mdns` | boolean | `false` | Disable mDNS service advertisement |
| `mdns_name` | string | hostname | Custom mDNS instance name |

#### Module Configuration

Modules define named export points that clients can access.

```toml
[modules.<name>]
path = "/absolute/path"
read_only = false
```

| Option | Type | Default | Description |
|--------|------|---------|-------------|
| `path` | string | required | Absolute filesystem path to export |
| `read_only` | boolean | `false` | Prevent write operations |

## Command-Line Options

Command-line options override configuration file settings.

```
blit-daemon [OPTIONS]

Options:
    --config <PATH>        Path to TOML configuration file
    --bind <ADDR>          Override bind address
    --port <PORT>          Override port
    --root <PATH>          Export root path (no modules)
    --no-mdns              Disable mDNS advertisement
    --mdns-name <NAME>     Override mDNS instance name
    --force-grpc-data      Force gRPC data plane (disable TCP)
    -h, --help             Print help
```

## Access Patterns

### Module-Based Access

With modules configured, clients access paths via module names:

```bash
# Access the "backup" module
blit-cli copy /local/path server://host:50051/backup/subdir

# Access the "media" module
blit-cli copy server://host:50051/media/videos /local/videos
```

### Root-Based Access

Without modules (using `--root`), clients access the exported root directly:

```bash
# Start daemon with root export
blit-daemon --root /data

# Client access
blit-cli copy /local/path server://host:50051/subdir
```

## Security Recommendations

### Network Binding

```toml
# Production: bind to specific interface
bind_address = "10.0.0.5"

# Development only: bind to all interfaces
bind_address = "0.0.0.0"
```

### Read-Only Modules

For directories that should never be modified:

```toml
[modules.archives]
path = "/data/archives"
read_only = true
```

### Firewall Configuration

```bash
# Linux (firewalld)
firewall-cmd --add-port=50051/tcp --permanent
firewall-cmd --reload

# Linux (ufw)
ufw allow 50051/tcp

# Windows PowerShell
New-NetFirewallRule -DisplayName "Blit Daemon" -Direction Inbound -Port 50051 -Protocol TCP -Action Allow
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
ReadWritePaths=/data/backups

[Install]
WantedBy=multi-user.target
```

Enable and start:

```bash
sudo systemctl daemon-reload
sudo systemctl enable blit-daemon
sudo systemctl start blit-daemon
```

### Windows (NSSM)

Using NSSM (Non-Sucking Service Manager):

```powershell
# Install service
nssm install BlitDaemon "C:\Program Files\Blit\blit-daemon.exe"
nssm set BlitDaemon AppParameters "--config C:\ProgramData\Blit\config.toml"
nssm set BlitDaemon AppDirectory "C:\Program Files\Blit"

# Start service
nssm start BlitDaemon
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
</dict>
</plist>
```

Load the service:

```bash
launchctl load ~/Library/LaunchAgents/com.blit.daemon.plist
```

## Monitoring

### Logging

Enable debug logging:

```bash
RUST_LOG=debug blit-daemon --config /etc/blit/config.toml
```

Log levels:
- `error` - Critical errors only
- `warn` - Warnings and errors
- `info` - General operation info (default)
- `debug` - Detailed debugging
- `trace` - Very verbose tracing

### Health Checks

```bash
# Check if daemon is responding
blit-cli list server://localhost:50051

# List available modules
blit-utils list-modules server://localhost:50051
```

## Troubleshooting

### Common Issues

**Daemon won't start**
- Check port availability: `netstat -an | grep 50051`
- Verify config file syntax: `cat /etc/blit/config.toml`
- Check file permissions on module paths

**Clients can't connect**
- Verify firewall rules
- Check bind address configuration
- Test with localhost first

**mDNS not working**
- Ensure `no_mdns = false` in config
- Check multicast network support
- Verify mDNS/Bonjour service is running

**Permission denied on transfers**
- Check daemon user has access to module paths
- Verify `read_only` setting for write operations
- Check filesystem permissions

### Debug Mode

```bash
# Maximum verbosity
RUST_LOG=trace blit-daemon --config /etc/blit/config.toml 2>&1 | tee daemon.log
```

## Performance Tuning

### TCP Data Plane

For large transfers, the TCP data plane provides better performance:

```bash
# Force gRPC-only mode (slower, but simpler)
blit-daemon --force-grpc-data

# Default: hybrid mode with TCP data plane (faster)
blit-daemon
```

### Module Path Performance

Place modules on fast storage:
- SSD/NVMe for frequently accessed data
- Consider RAID configuration for throughput
- Use local filesystems (avoid NFS for high performance)

### Network Configuration

For high-throughput environments:
- Enable jumbo frames if supported
- Consider dedicated network interface for transfers
- Tune TCP buffer sizes at OS level
