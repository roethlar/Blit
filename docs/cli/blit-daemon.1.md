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

## SEE ALSO
`blit(1)`
