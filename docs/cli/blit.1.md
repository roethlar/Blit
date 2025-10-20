# BLIT(1) blit Manual
% Blit v2 Team
% 2025-10-19

## NAME
blit - high-performance local file transfer CLI with streaming planner

## SYNOPSIS
`blit copy [OPTIONS] <SOURCE> <DESTINATION>`  
`blit mirror [OPTIONS] <SOURCE> <DESTINATION>`  
`blit push <SOURCE> blit://host:port/module`  
`blit pull blit://host:port/module[/path] <DESTINATION>`  
`blit diagnostics perf [--limit <N>]`

## DESCRIPTION
`blit` drives the v2 streaming transfer engine. Local commands keep
runtime output minimal to avoid impacting throughput while still emitting
structured events that future GUI layers can display.

`copy` performs a one-way local transfer from `<SOURCE>` to
`<DESTINATION>` without deleting extraneous files.  
`mirror` performs a local transfer that also removes files present only
at the destination. The planner automatically selects fast paths for tiny
and large workloads and tunes worker counts without user input.

## OPTIONS
- `--dry-run`  
  Enumerate and plan the transfer, but do not modify the destination.

- `--checksum`  
  Force checksum validation for changed files (defaults to metadata
  comparison).

- `--verbose`  
  Emit progress and planner diagnostics to stderr.

- `--no-progress`  
  Disable the interactive progress spinner (already quiet by default).

These options apply to both `copy` and `mirror`.

## DEBUG OPTIONS
- `--workers <N>` *(hidden)*  
  Caps the planner at `N` worker threads for diagnostic runs. When this
  limiter is active the CLI prints `[DEBUG] Worker limiter active – FAST
  planner auto-tuning capped to N thread(s).` The transfer still succeeds,
  but throughput guarantees are suspended. This flag exists solely for
  engineering analysis; remove it for production runs.

Planner tuning is otherwise automatic. There are no other CLI tunables or
environment variables that affect worker selection.

## DIAGNOSTICS
`blit diagnostics perf [--limit <N>]` prints the most recent local
performance history captured by the orchestrator (50 records by default).
This command is read-only and never uploads data.

## REMOTE COMMANDS
`push` uploads local trees to a remote daemon module, negotiating either
the TCP data plane or the gRPC fallback automatically. `pull` downloads
remote files or directories into the specified destination directory.
`ls` remains reserved for future Phase 3 work.

## ENVIRONMENT
None.

## FILES
- `${XDG_CONFIG_HOME:-$HOME/.config}/blit/perf_local.jsonl` – local
  performance history captured after each transfer.

## SEE ALSO
`docs/plan/LOCAL_TRANSFER_HEURISTICS.md`, `docs/plan/MASTER_WORKFLOW.md`
