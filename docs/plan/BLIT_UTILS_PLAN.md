# blit-utils Command Plan

## Overview

`blit-utils` provides operator tooling for discovery, inspection, and maintenance of remote Blit daemons. The
utility must remain composable (stdout-friendly), support non-interactive scripting, and honour daemon security
constraints (read-only modules, chroot boundaries).

## Command Matrix

| Command | Description | Notes |
|---------|-------------|-------|
| `blit-utils scan [--wait <s>]` | mDNS discovery of daemons advertising `_blit._tcp.local.` | Shares implementation with `blit scan`; prints table of host, port, mdns name. |
| `blit-utils list-modules <remote>` | Lists modules exported by a daemon (like `blit list server`). | Wrapper around ListModules RPC. |
| `blit-utils ls <remote[:/module/path]>` | Directory listing within a module or root export. | Human + machine friendly (optional `--json`). |
| `blit-utils list <remote[:/module/path]>` | Alias of `ls` (compatible with plan v6). | |
| `blit-utils find <remote:/module/path> [--pattern <glob>]` | Recursive find reporting paths matching criteria. | Streams results, optional JSON. |
| `blit-utils du <remote:/module/path>` | Summarises disk usage for a subtree. | Displays total size/file count; optional depth parameter. |
| `blit-utils df <remote>` | Reports module storage metrics (space used/free). | Requires new RPC. |
| `blit-utils rm <remote:/module/path> [--yes]` | Removes files/dirs remotely (respecting read-only). | Prompts unless `--yes`; requires Purge RPC. Implemented 2025-10-23. |
| `blit-utils completions <shell>` | Emits shell completion scripts for CLI and utils. | Integrates with canonical URL parser. |
| `blit-utils profile` | Displays local performance history / predictor coefficients. | Reuses existing JSONL + predictor state. |

## UX Principles

1. **Safety First** – destructive commands (`rm`) require confirmation unless `--yes` is provided. Read-only modules
   must reject mutation attempts with clear error messages.
2. **Consistent Formatting** – default output is tabular text; `--json` flag emits machine-parsable JSON arrays. Time
   stamps in ISO 8601; sizes printed via `format_bytes` alongside raw bytes when relevant.
3. **Exit Codes** – success returns 0; partial failures return non-zero with aggregated error messages.
4. **Shared Endpoint Parsing** – reuse `RemoteEndpoint` for URL parsing to ensure identical behaviour with `blit` CLI.
5. **Authentication Hooks** – plan for future token support (e.g., `--auth-token`). CLI should accept but ignore token
   for now, forwarding to RPC once implemented.

## RPC Requirements

- **ListModules** *(implemented)* – returns modules (name, path, read_only).
- **List** *(implemented)* – returns directory entries.
- **Find** – daemon walks subtree with filters; responds as stream to avoid buffering.
- **Du** – daemon computes total size/count (optionally depth-limited).
- **Df** – daemon reports filesystem stats for module root.
- **Purge** – deletes provided paths (daemon enforces read-only/chroot and reports counts).

## Implementation Checklist

1. Flesh out `blit_utils` crate skeleton with Clap-based subcommands.
2. Share endpoint parsing + gRPC client helpers with `blit-cli` via `blit-utils` or a common module.
3. Implement streaming RPC consumption using `tonic` async clients.
4. Add local fallbacks where useful (e.g., `profile` reading local JSONL).
5. Write integration tests calling daemon RPCs (Phase 3.5 test suite).
6. Update documentation (CLI manpages, quick-start) once commands land.

## Testing Strategy

- Unit tests for option parsing / confirmation flows.
- Integration tests using localhost daemon fixture covering each command, including read-only and error conditions.
- Snapshot tests for JSON output (serde_json comparisons).
- Fault injection: simulate daemon returning errors, ensure CLI surfaces them with non-zero exit codes.
