# BLIT(1) blit Manual
% Blit v2 Team
% 2025-11-21

## NAME
blit – local and hybrid-transport file transfer CLI

## SYNOPSIS
`blit copy [OPTIONS] <SOURCE> <DESTINATION>`  
`blit mirror [OPTIONS] <SOURCE> <DESTINATION>`  
`blit move [OPTIONS] <SOURCE> <DESTINATION>`  
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

- `--verbose`  
  Emit planner heartbeat messages and fast-path decisions to stderr.

- `--progress`  
  Show an interactive ASCII spinner while the transfer runs.

- `--force-grpc`  
  Bypass the TCP data plane negotiation and stream payloads over gRPC.

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

## SEE ALSO
`blit-daemon(1)`
