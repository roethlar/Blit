# BLIT(1) blit Manual
% Blit v2 Team
% 2025-11-21

## NAME
blit – local and hybrid-transport file transfer CLI

## SYNOPSIS
`blit copy [OPTIONS] <SOURCE> <DESTINATION>`
`blit mirror [OPTIONS] [--yes] <SOURCE> <DESTINATION>`
`blit move [OPTIONS] [--yes] <SOURCE> <DESTINATION>`
`blit scan [--wait <SECONDS>]`
`blit list <REMOTE>`
`blit du [--max-depth <N>] [--json] <REMOTE>`
`blit df [--json] <REMOTE>`
`blit rm [--yes] <REMOTE>`
`blit find [--pattern <GLOB>] [--case-insensitive] [--limit <N>] [--json] <REMOTE>`
`blit diagnostics perf [--limit <N>] [--enable|--disable] [--clear]`

## DESCRIPTION
`blit` drives the v2 streaming transfer engine. It supports local transfers and
remote operations via a hybrid TCP/gRPC transport.

### Transfer Commands
- `copy` copies a `<SOURCE>` into `<DESTINATION>` without deleting extraneous files.
- `mirror` performs the same copy but removes files that are only present at the destination.
- `move` mirrors the source into the destination and then removes the original tree.

Any `<SOURCE>` or `<DESTINATION>` may be a local path or a remote endpoint:
- `server:/module/path` (explicit module export)
- `server://path` (default root export, if configured)
- `server` (implies default root)

Remote-to-remote transfers are supported (e.g., `blit copy server1:/mod/A server2:/mod/B`).

### Admin Commands
- `scan` discovers blit daemons on the local network via mDNS.
- `list` lists modules (on a bare host) or directory contents (on a module path).
- `du` shows disk usage for a remote path.
- `df` shows filesystem statistics (total/used/free) for a remote module.
- `rm` removes a file or directory on a remote daemon.
- `find` searches for files on a remote daemon.

## OPTIONS
### Transfer Options
- `--dry-run`  
  Enumerate and plan the transfer without modifying the destination.

- `--checksum`  
  Force checksum validation for changed files (metadata comparison is the default).

- `--resume`  
  Enable block-level resumption for interrupted transfers. Compares source and
  destination files block-by-block (hashing) and transfers only the changed parts.
  Useful for resuming large file transfers or updating files with small changes.

- `--verbose`  
  Emit planner heartbeat messages and fast-path decisions to stderr.

- `--progress`  
  Show an interactive ASCII spinner while the transfer runs.

- `--force-grpc`
  Bypass the TCP data plane negotiation and stream payloads over gRPC.

- `--yes`, `-y` (mirror, move)
  Skip the confirmation prompt for destructive operations. By default, `mirror`
  prompts before deleting extraneous files at the destination, and `move` prompts
  before deleting the source after transfer.

### Admin Options
- `--wait <SECONDS>` (scan)  
  Duration to wait for mDNS responses (default: 2).

- `--max-depth <N>` (du)  
  Limit traversal depth (0 = unlimited).

- `--json` (du, df)  
  Output results as JSON.

- `--yes` (rm)  
  Skip confirmation prompt.

- `--pattern <GLOB>` (find)  
  Glob pattern to match (e.g., "*.txt").

- `--case-insensitive` (find)  
  Enable case-insensitive pattern matching.

- `--limit <N>` (find)  
  Limit number of results.

## DIAGNOSTICS
`blit diagnostics perf` inspects and manages the local performance history.

- `--limit <N>` shows the most recent `N` entries (0 = all).
- `--enable` / `--disable` toggle capture.
- `--clear` removes the stored history file.

## CONFIGURATION DIRECTORY
- `--config-dir <PATH>` overrides the default configuration directory.

## FILES
- `${XDG_CONFIG_HOME:-$HOME/.config}/blit/perf_local.jsonl` – local performance history.
- `${XDG_CONFIG_HOME:-$HOME/.config}/blit/settings.json` – persisted CLI settings.

## SECURITY
Remote transfers do not include built-in TLS encryption. Data is transmitted in
plaintext over the TCP data plane and gRPC control plane. Operators are expected
to secure remote transfers through one of the following methods:

- **Trusted private network**: Run blit daemons only on isolated, trusted networks
  where traffic cannot be intercepted.

- **SSH tunnel**: Forward the daemon port through SSH:
  ```
  ssh -L 9031:localhost:9031 remote-host
  blit mirror /local/path localhost:/module/path
  ```

- **VPN**: Connect clients and servers via an encrypted VPN tunnel.

- **Reverse proxy with TLS**: Place the daemon behind a TLS-terminating reverse
  proxy (e.g., nginx, Caddy) for encrypted connections.

The daemon binds to `0.0.0.0` by default. In untrusted environments, use
`--bind 127.0.0.1` and access via SSH tunnel or VPN.

## SEE ALSO
`blit-daemon(1)`
