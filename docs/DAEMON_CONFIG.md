# Daemon Configuration Guide

This document describes how to configure the Blit daemon (`blit-daemon`) for
remote file transfer operations.

## Trust Model and Network Exposure

`blit-daemon` is a network file-transfer service. By default it binds to
`0.0.0.0:9031` so that remote `blit` clients can reach it — that is the
service's whole purpose. Loopback-by-default would only make sense for a
"local sidecar" or "must-SSH-tunnel" model, neither of which is Blit's
deployment model.

Security comes from **operator network controls + per-transfer auth tokens**,
not from the bind address:

- The TCP data plane uses one-time tokens minted per transfer; a peer that
  doesn't present the right token is dropped before any bytes flow.
- Module containment is always-on (see [Path containment](#path-containment));
  a symlink inside an exported module that points outside it cannot be
  followed by daemon operations.
- TLS termination, mutual auth, and access-control fronting are explicitly
  out of scope for the daemon — operators are expected to firewall the
  daemon port, restrict it to a trusted network, or front it with
  WireGuard / Tailscale / SSH-tunnel / whatever fits the deployment.

If you want the daemon reachable only from the local host, set `bind =
"127.0.0.1"` in the config or pass `--bind 127.0.0.1`. There is no
"default safe" mode — you choose the exposure that fits your network.

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

# Optional: enable destination-side delegated pull for direct
# remote→remote transfers. See "Outbound delegation" under Path
# containment for the full security model.
# [delegation]
# allow_delegated_pull = false
# allowed_source_hosts = ["server-a.lan", "10.0.0.0/8"]
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
| `delegation_allowed` | boolean | `true` | Per-module narrowing override for the `[delegation]` master switch. Set to `false` to opt this module out of being a `DelegatedPull` destination even when daemon-wide delegation is enabled. Cannot widen — has no effect when `allow_delegated_pull = false` daemon-wide. |

Module names must be non-empty and unique within the configuration.

#### `[delegation]` Section

Controls destination-side delegated pull (direct remote→remote
transfers). Default: feature off. See [Outbound delegation
(`DelegatedPull`)](#outbound-delegation-delegatedpull) for the full
matching semantics, the SSRF/network-pivot rationale, and the
loopback IP-form rule.

| Option | Type | Default | Description |
|--------|------|---------|-------------|
| `allow_delegated_pull` | boolean | `false` | Master switch. When false, the daemon refuses `DelegatedPull` requests. |
| `allowed_source_hosts` | array of strings | `[]` | Source allowlist. Accepts hostnames (IDNA-normalized), CIDR blocks (IPv4 or IPv6, parsed via `ipnet`), and bare IP literals (with optional brackets for IPv6). Invalid entries fail config load. Empty + master switch true means "any host." |

#### Path containment

Module containment is **always on** as of F2. Every daemon read or
write resolves the target's deepest existing ancestor through
`std::fs::canonicalize` and refuses the operation if the canonical
form escapes the module root. A symlink inside the module that
points outside it cannot be traversed by daemon operations.

There is no opt-out. The previous `use_chroot` and `root_use_chroot`
config options were removed (they never enforced anything beyond
the lexical `safe_join` check, which doesn't follow symlinks).
Configs containing those keys will silently ignore them — TOML
unknown-field tolerance — and the runtime behavior is the always-on
containment described above.

**TOCTOU caveat:** the check is "canonicalize then operate," so a
symlink swapped between the check and the actual filesystem call
could in principle be followed. The trust model is authenticated
peers and operator-controlled module contents — not adversarial
local processes racing the daemon. A fully race-proof variant
would use `openat` + `O_NOFOLLOW` per-component descent; that is
deferred until there's a concrete threat that warrants it.

#### Outbound delegation (`DelegatedPull`)

When the CLI runs `blit copy server-A:/x server-B:/y`, the destination
daemon (server-B) can be told to pull directly from the source daemon
(server-A) — bytes flow `A → B` over a single data plane and the CLI
host is not in the byte path. This optimization is opt-in per
destination daemon because it changes the daemon's network surface:

- **Default off.** A daemon will refuse `DelegatedPull` requests
  unless the operator sets `[delegation] allow_delegated_pull = true`.
  CLI clients that hit a daemon with delegation off receive a clear
  upgrade-or-relay error and can fall back to `--relay-via-cli` to
  route through the CLI host instead.
- **Why opt-in.** Allowing `DelegatedPull` lets any caller that can
  reach this daemon's control plane make the daemon initiate a TCP
  connection to a source endpoint of the caller's choosing. That is
  a new outbound-network capability — an SSRF/network-pivot
  primitive — that didn't exist before. The gate exists so the
  operator decides when this capability is on and against which
  hosts.

##### Configuration

```toml
[delegation]
# Master switch. Default: false. Setting to true allows the daemon
# to act as a delegated pull initiator on behalf of CLI clients
# that can reach its control plane.
allow_delegated_pull = false

# Source allowlist. When non-empty, every resolved IP of the source
# host must match at least one entry. Empty + master switch true
# means "any host" — only honor that posture on a fully trusted
# LAN.
#
# Accepted entry forms:
#   - hostname        e.g. "server-a.lan"
#   - CIDR (IPv4/v6)  e.g. "10.0.0.0/8", "fd00::/8"
#   - bare IP         e.g. "10.1.2.3", "::1", "[::1]"
allowed_source_hosts = ["server-a.lan", "10.0.0.0/8"]
```

Per-module narrowing override (under `[[module]]`):

```toml
[[module]]
name = "secrets"
path = "/srv/secrets"
delegation_allowed = false   # opt this module out even when
                              # daemon-wide delegation is on.
                              # Defaults to true.
```

Per-module overrides can only **narrow** the daemon-wide policy.
Setting `delegation_allowed = true` on a module when
`allow_delegated_pull = false` daemon-wide does not enable
delegation for that module.

##### Allowlist matching semantics

Strict and explicit (a permissive matcher would defeat the gate's
purpose):

1. **Hostname normalization.** Comparison is case-insensitive after
   trimming a trailing dot and applying IDNA punycode. `Server-A.LAN.`
   and `server-a.lan` both match `server-a.lan`.
2. **Hostname matches** are exact post-normalization equality. No
   wildcards in 0.1.0.
3. **CIDR / bare-IP entries** are parsed once at config load via
   the `ipnet` crate. Invalid entries fail config load loudly —
   the daemon refuses to start rather than silently treating an
   unparseable line as "deny everything," which would mask a
   typo'd permit.
4. **Resolution.** A hostname locator resolves once to its A/AAAA
   set; **every** resolved address must match either a CIDR entry
   or a bare-IP entry (the literal hostname can also match per (2)
   — but only for non-special-range addresses; see (6)). Mixed-
   result resolution where some addresses are inside the
   allowlist and some are outside is denied.
5. **DNS-rebinding mitigation.** The validated IP is bound to the
   outbound connection. The daemon connects to a specific
   `host = <ip>` URI, never to a re-resolvable hostname. A
   malicious DNS authority cannot swap addresses between the gate
   check and the connect.
6. **Loopback / link-local / unique-local addresses require
   IP- or CIDR-form authorization.** If `evil.example.com` is in
   the allowlist and resolves to `127.0.0.1`, accepting that on
   the strength of the hostname alone would let any actor with
   control of `evil.example.com`'s A record point the daemon at
   its own loopback services (an SSRF-via-DNS pivot). The gate
   denies in this case unless an explicit IP/CIDR entry covers
   the resolved address. The affected ranges are: `127.0.0.0/8`,
   `169.254.0.0/16`, `0.0.0.0/8`, `::1`, `fe80::/10`, `fc00::/7`,
   `::`. To delegate against a same-host source for tests, the
   operator writes `allowed_source_hosts = ["127.0.0.1"]` or
   `["127.0.0.0/8"]`, not just the hostname.

##### Auth posture (current and future)

In 0.1.0 there is no daemon authentication — the trust model is
"reachable on a trusted network." The delegation gate is policy,
not authentication; it does not turn an internet-exposed daemon
into a safely-internet-exposed one. For internet-exposed
deployments, both the gate and external auth (TLS termination,
WireGuard, SSH tunnel, etc.) are required.

`RemoteSourceLocator.delegated_credential` is a forward-compatible
field defined in the wire format and currently ignored. When
operator-issued bearer tokens land (post-0.1.0), the CLI will mint
a token scoped to "operator + dst-host" and pass it through; the
destination daemon will present it on its outbound connection to
src.

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
