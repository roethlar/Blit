# BLIT-DAEMON(1) blit Manual
% Blit v2 Team
% 2025-10-19

## NAME
blit-daemon - remote transfer daemon for blit v2

## SYNOPSIS
`blit-daemon [OPTIONS]`

## DESCRIPTION
`blit-daemon` exposes the gRPC control plane and hybrid transport data services
used by `blit push` and the forthcoming remote operations. The daemon listens on
the specified address (default `127.0.0.1:50051`) and, when possible, negotiates
a TCP data plane for high-throughput file transfers. A debug flag allows
operators to force the daemon to stay on the gRPC control plane for testing or
firewalled environments.

## OPTIONS
- `--bind <ADDR>`  
  Bind address for the gRPC control plane (default `127.0.0.1:50051`).

- `--force-grpc-data`  
  Skip the TCP data listener and stream file payloads over the gRPC control
  plane. Intended for diagnostics and locked-down environments; when active the
  daemon reports transfers with `tcp_fallback_used = true` in the summary.

## ENVIRONMENT
None.

## FILES
None.

## SEE ALSO
`blit(1)`
