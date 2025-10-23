# BLIT(1) blit Manual
% Blit v2 Team
% 2025-10-20

## NAME
blit – local and hybrid-transport file transfer CLI

## SYNOPSIS
`blit copy [OPTIONS] <SOURCE> <DESTINATION>`  
`blit mirror [OPTIONS] <SOURCE> <DESTINATION>`  
`blit move [OPTIONS] <SOURCE> <DESTINATION>`  
`blit scan [--wait <SECONDS>]`  
`blit list <REMOTE>`  
`blit diagnostics perf [--limit <N>] [--enable|--disable] [--clear]`

## DESCRIPTION
`blit` drives the v2 streaming transfer engine. Local commands stay quiet
by default so that terminal output never throttles the transfer, yet they
still emit structured progress events that future GUI surfaces can
subscribe to.

- `copy` copies a local `<SOURCE>` into `<DESTINATION>` without deleting
  extraneous files.
- `mirror` performs the same copy but removes files that are only present
  at the destination.
- `move` mirrors the source into the destination and then removes the
  original tree.
- `scan` and `list` are reserved for Phase 3 remote discovery commands and
  currently return *not implemented* errors.

Remote transfers already reuse `copy`, `mirror`, and `move`. Any
`<SOURCE>` or `<DESTINATION>` may be a local path **or** a remote endpoint
in the form `host:/module/path` (explicit module export) or
`host://path` (default root export when configured). Remote-to-remote
transfers are not yet supported; one side must be local. Default port
9031 is implied when not specified (`host:port:/module/...` works), and
paths are always canonicalised using forward slashes.

## OPTIONS
- `--dry-run`  
  Enumerate and plan the transfer without modifying the destination.

- `--checksum`  
  Force checksum validation for changed files (metadata comparison is the
  default).

- `--verbose`  
  Emit planner heartbeat messages and fast-path decisions to stderr.

- `--progress`  
  Show an interactive ASCII spinner while the transfer runs. When omitted
  the CLI prints only the final summary so that scripting and logging
  stay clean.

These options apply to `copy`, `mirror`, and `move`.

## DEBUG OPTIONS
- `--workers <N>` *(hidden)*  
  Caps the planner at `N` worker threads for diagnostic runs. When this
  limiter is active the CLI prints  
  `[DEBUG] Worker limiter active – FAST planner auto-tuning capped to N thread(s).`  
  The transfer still succeeds, but throughput guarantees are suspended.
  This flag exists solely for engineering analysis; remove it for
  production runs.

Planner tuning is otherwise automatic. There are no other CLI tunables or
environment variables that affect worker selection.

## DIAGNOSTICS
`blit diagnostics perf` inspects and manages the local performance
history captured by the orchestrator (50 records shown by default).

- `--limit <N>` shows the most recent `N` entries (0 = all).
- `--enable` / `--disable` toggle capture in the on-disk settings file.
- `--clear` removes the stored JSONL history file.

These flags operate on the config directory described below; toggling is
persistent until changed again.

## ENVIRONMENT
The CLI does not use environment variables for behaviour, but the
following testing hook is honoured:

- `BLIT_CONFIG_DIR` – overrides the config directory path. Useful for
  integration tests and benchmark harnesses.

## FILES
- `${XDG_CONFIG_HOME:-$HOME/.config}/blit/perf_local.jsonl` – local
  performance history captured after each run.
- `${XDG_CONFIG_HOME:-$HOME/.config}/blit/settings.json` – persisted CLI
  settings (currently just the performance-history toggle).

## SEE ALSO
`docs/plan/LOCAL_TRANSFER_HEURISTICS.md`, `docs/plan/MASTER_WORKFLOW.md`
