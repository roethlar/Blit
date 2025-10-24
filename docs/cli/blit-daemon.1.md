# BLIT-DAEMON(1) blit Manual
% Blit v2 Team
% 2025-10-19

## NAME
blit-daemon - remote transfer daemon for blit v2

## SYNOPSIS
`blit-daemon [OPTIONS]`

## DESCRIPTION
`blit-daemon` exposes the gRPC control plane and hybrid transport data services
consumed by `blit copy`, `blit mirror`, and the forthcoming admin tooling. The
daemon listens on the specified address (default `127.0.0.1:9031`) and, when
possible, negotiates a TCP data plane for high-throughput file transfers. A
debug flag allows operators to force the daemon to stay on the gRPC control
plane for testing or firewalled environments. Unless disabled, the daemon
advertises itself via mDNS (`_blit._tcp.local.`) so local clients can discover
exports with `blit scan` or `blit-utils scan`.

## OPTIONS
- `--config <PATH>`  
  Read TOML configuration from `<PATH>` instead of the default
  `/etc/blit/config.toml` (or the file pointed to by
  `$BLIT_DAEMON_CONFIG` when set).

- `--bind <ADDR>`  
  Bind address for the gRPC control plane (default `127.0.0.1`).

- `--port <PORT>`  
  Port number for the gRPC control plane (default `9031`).

- `--root <PATH>`  
  Export `<PATH>` as the default module when no named modules are defined.

- `--no-mdns`  
  Disable mDNS advertisement.

- `--mdns-name <NAME>`  
  Override the advertised instance name (defaults to `blit@<hostname>`).

- `--force-grpc-data`  
  Skip the TCP data listener and stream file payloads over the gRPC control
  plane. Intended for diagnostics and locked-down environments; when active the
  daemon reports transfers with `tcp_fallback_used = true` in the summary.

## ENVIRONMENT
None (configuration is sourced from CLI flags and optional TOML files).

## FILES
- `/etc/blit/config.toml` – default daemon configuration file (module exports,
  bind options, mDNS settings).

## SEE ALSO
`blit(1)`
