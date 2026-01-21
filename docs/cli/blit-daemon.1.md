# BLIT-DAEMON(1) blit Manual
% Blit v2 Team
% 2025-11-21

## NAME
blit-daemon - remote transfer daemon for blit v2

## SYNOPSIS
`blit-daemon [OPTIONS]`

## DESCRIPTION
`blit-daemon` exposes the gRPC control plane and hybrid transport data services
consumed by `blit` clients. The daemon listens on the specified address (default
`0.0.0.0:9031`) and negotiates a TCP data plane for high-throughput file
transfers.

Unless disabled, the daemon advertises itself via mDNS (`_blit._tcp.local.`) so
local clients can discover exports with `blit scan`.

## CONFIGURATION
The daemon can be configured via a TOML file (default: `/etc/blit/config.toml`
on Unix, `%ProgramData%\Blit\config.toml` on Windows).

### Example Config
```toml
[daemon]
bind = "0.0.0.0"
port = 9031
motd = "Welcome to Blit Server"
no_mdns = false
mdns_name = "my-server"
# Optional: export a default root for server:// requests
# root = "/srv/blit"
# root_read_only = true

[[module]]
name = "backup"
path = "/mnt/backups"
read_only = true
comment = "Backup storage"

[[module]]
name = "public"
path = "/srv/public"
read_only = false
```

If no modules are defined and no default root is configured, the daemon will
export its current working directory as the `default` module (and warn about it).

## OPTIONS
- `--config <PATH>`  
  Read TOML configuration from `<PATH>` instead of the default location.

- `--bind <ADDR>`  
  Bind address for the gRPC control plane (overrides config).

- `--port <PORT>`  
  Port number for the gRPC control plane (overrides config).

- `--root <PATH>`  
  Export `<PATH>` as the default module when no named modules are defined.

- `--no-mdns`  
  Disable mDNS advertisement.

- `--mdns-name <NAME>`  
  Override the advertised instance name (defaults to `blit@<hostname>`).

- `--force-grpc-data`  
  Skip the TCP data listener and stream file payloads over the gRPC control
  plane. Intended for diagnostics and locked-down environments.

## ENVIRONMENT
None (configuration is sourced from CLI flags and optional TOML files).

## FILES
- `/etc/blit/config.toml` – default daemon configuration file.

## SECURITY
The daemon does not implement built-in TLS encryption. All data is transmitted in
plaintext over the TCP data plane and gRPC control plane. Operators must secure
remote transfers through external means:

- **Trusted private network**: Deploy daemons only on isolated, trusted networks.

- **SSH tunnel**: Forward the daemon port through SSH from the client:
  ```
  ssh -L 9031:localhost:9031 server-host
  ```
  Then connect to `localhost:/module/path` from the client.

- **VPN**: Connect clients and servers via an encrypted VPN tunnel.

- **Reverse proxy with TLS**: Place the daemon behind a TLS-terminating reverse
  proxy (e.g., nginx, Caddy).

By default, the daemon binds to `0.0.0.0:9031`, accepting connections from any
interface. In untrusted environments:
- Use `--bind 127.0.0.1` to restrict to localhost only
- Use firewall rules to limit access to trusted IP ranges
- Access the daemon via SSH tunnel or VPN

The daemon does not implement authentication. Any client that can reach the daemon
can access all configured modules. Use network-level access controls to restrict
who can connect.

## SEE ALSO
`blit(1)`
