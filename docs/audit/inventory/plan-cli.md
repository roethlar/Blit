# Plan Inventory: CLI surface (manpage, README, CHANGELOG)
**Generated**: 2026-06-04 by audit workflow
**Coverage**: 4 files, 505 lines total

| File | Lines |
|---|---|
| docs/cli/blit.1.md | 202 |
| README.md | 153 |
| CHANGELOG.md | 78 |
| docs/plan/BLIT_UTILS_PLAN.md | 72 |

---

## Claims (grouped by category)

### interface

#### verb-copy
**Source**: docs/cli/blit.1.md:SYNOPSIS, Transfer Commands
**Specificity**: high

`blit copy [OPTIONS] <SOURCE> <DESTINATION>` — copies a `<SOURCE>` (file or directory) to `<DESTINATION>` without deleting extraneous files.

#### verb-mirror
**Source**: docs/cli/blit.1.md:SYNOPSIS, Transfer Commands
**Specificity**: high

`blit mirror [OPTIONS] [--yes] <SOURCE> <DESTINATION>` — performs the same copy but removes files that are only present at the destination.

#### verb-move
**Source**: docs/cli/blit.1.md:SYNOPSIS, Transfer Commands
**Specificity**: high

`blit move [OPTIONS] [--yes] <SOURCE> <DESTINATION>` — mirrors the source into the destination and then removes the original tree.

#### verb-scan
**Source**: docs/cli/blit.1.md:SYNOPSIS, Admin Commands
**Specificity**: high

`blit scan [--wait <SECONDS>] [--json]` — discovers blit daemons on the local network via mDNS.

#### verb-list
**Source**: docs/cli/blit.1.md:SYNOPSIS, Admin Commands
**Specificity**: high

`blit list <REMOTE> [--json]` — smart-dispatches by target shape: a bare host (`server`, `server:9031`) routes to `list-modules`; a target with a module or path (`server:/module/`) routes to `ls`.

#### verb-list-modules
**Source**: docs/cli/blit.1.md:SYNOPSIS, Admin Commands
**Specificity**: high

`blit list-modules <REMOTE> [--json]` — lists modules exported by a daemon.

#### verb-ls
**Source**: docs/cli/blit.1.md:SYNOPSIS, Admin Commands
**Specificity**: high

`blit ls <TARGET> [--json]` — lists directory contents inside a module (or local path).

#### verb-du
**Source**: docs/cli/blit.1.md:SYNOPSIS, Admin Commands
**Specificity**: high

`blit du [--max-depth <N>] [--json] <REMOTE>` — shows disk usage for a remote path.

#### verb-df
**Source**: docs/cli/blit.1.md:SYNOPSIS, Admin Commands
**Specificity**: high

`blit df [--json] <REMOTE>` — shows filesystem statistics (total/used/free) for a remote module.

#### verb-rm
**Source**: docs/cli/blit.1.md:SYNOPSIS, Admin Commands
**Specificity**: high

`blit rm [--yes] <REMOTE>` — removes a file or directory on a remote daemon.

#### verb-find
**Source**: docs/cli/blit.1.md:SYNOPSIS, Admin Commands
**Specificity**: high

`blit find [--pattern <GLOB>] [--case-insensitive] [--limit <N>] [--json] <REMOTE>` — searches for files on a remote daemon (glob `--pattern`, e.g. `*.csv` or `**/*.log`; `*` does not cross `/`).

#### verb-completions-shell
**Source**: docs/cli/blit.1.md:SYNOPSIS, Admin Commands
**Specificity**: high

`blit completions shell <SHELL>` — writes a clap-generated shell-completion script to stdout (bash/zsh/fish/powershell/elvish). Source it from your shell's rc file or completion directory.

#### verb-completions-remote
**Source**: docs/cli/blit.1.md:SYNOPSIS, Admin Commands
**Specificity**: high

`blit completions remote <REMOTE> [--prefix <STR>] [--files] [--dirs]` — returns daemon-side path completions via the `CompletePath` RPC. Used internally by the generated shell scripts for `server:/module/<TAB>`-style completion; usable directly for scripting too.

#### verb-profile
**Source**: docs/cli/blit.1.md:SYNOPSIS, Admin Commands
**Specificity**: high

`blit profile [--limit <N>] [--json]` — prints local performance history records and the predictor's coefficients per transfer mode (`--json` for scripting). Reads `perf_local.jsonl` and the predictor state file; no network access.

#### verb-diagnostics-perf
**Source**: docs/cli/blit.1.md:SYNOPSIS, DIAGNOSTICS
**Specificity**: high

`blit diagnostics perf [--limit <N>] [--enable|--disable] [--clear]` — inspects and manages the local performance history. `--limit <N>` shows most recent N (0=all); `--enable`/`--disable` toggle capture; `--clear` removes the stored history file.

#### verb-diagnostics-dump
**Source**: docs/cli/blit.1.md:SYNOPSIS, DIAGNOSTICS
**Specificity**: high

`blit diagnostics dump [--json] <SOURCE> <DESTINATION>` — prints a pasteable snapshot of what blit sees for an invocation: parsed endpoints, rsync destination resolution, filesystem caps, free/total disk space, and (for local pairs) whether source and destination are on the same device. No transfer is performed. Intended for bug reports.

#### endpoint-syntax-module
**Source**: docs/cli/blit.1.md:Destination Semantics
**Specificity**: high

Remote endpoint form: `server:/module/path` (explicit module export).

#### endpoint-syntax-root
**Source**: docs/cli/blit.1.md:Destination Semantics
**Specificity**: high

Remote endpoint form: `server://path` (default root export, if configured).

#### endpoint-syntax-bare
**Source**: docs/cli/blit.1.md:Destination Semantics
**Specificity**: high

Remote endpoint form: `server` (implies default root).

#### endpoint-forward-slashes-only
**Source**: docs/cli/blit.1.md:Destination Semantics (Note)
**Specificity**: high

Remote paths must use forward slashes (`/`), not backslashes (`\`), regardless of platform. Incorrect: `server:\module\path` — Correct: `server:/module/path`.

#### opt-dry-run
**Source**: docs/cli/blit.1.md:OPTIONS Transfer Options
**Specificity**: high

`--dry-run` — Enumerate and plan the transfer without modifying the destination.

#### opt-checksum
**Source**: docs/cli/blit.1.md:OPTIONS Transfer Options
**Specificity**: high

`--checksum` — Force checksum validation for changed files (metadata comparison is the default).

#### opt-resume
**Source**: docs/cli/blit.1.md:OPTIONS Transfer Options
**Specificity**: high

`--resume` — Enable block-level resumption for interrupted transfers. Compares source and destination files block-by-block (hashing) and transfers only the changed parts. Useful for resuming large file transfers or updating files with small changes.

#### opt-verbose
**Source**: docs/cli/blit.1.md:OPTIONS Transfer Options
**Specificity**: high

`--verbose` — Emit planner heartbeat messages and fast-path decisions to stderr.

#### opt-progress
**Source**: docs/cli/blit.1.md:OPTIONS Transfer Options
**Specificity**: high

`--progress` — Show an interactive ASCII spinner while the transfer runs.

#### opt-force-grpc
**Source**: docs/cli/blit.1.md:OPTIONS Transfer Options
**Specificity**: high

`--force-grpc` — Bypass the TCP data plane negotiation and stream payloads over gRPC.

#### opt-relay-via-cli
**Source**: docs/cli/blit.1.md:OPTIONS Transfer Options
**Specificity**: high

`--relay-via-cli` — For remote-to-remote transfers, force the legacy relay path where the CLI pulls from the source and pushes to the destination. Use this only when the destination daemon cannot reach the source daemon, or for benchmarking.

#### opt-yes-transfer
**Source**: docs/cli/blit.1.md:OPTIONS Transfer Options
**Specificity**: high

`--yes`, `-y` (mirror, move) — Skip the confirmation prompt for destructive operations. By default, `mirror` prompts before deleting extraneous files at the destination, and `move` prompts before deleting the source after transfer.

#### opt-wait
**Source**: docs/cli/blit.1.md:OPTIONS Admin Options
**Specificity**: high

`--wait <SECONDS>` (scan) — Duration to wait for mDNS responses (default: 2).

#### opt-max-depth
**Source**: docs/cli/blit.1.md:OPTIONS Admin Options
**Specificity**: high

`--max-depth <N>` (du) — Limit traversal depth (0 = unlimited).

#### opt-json-admin
**Source**: docs/cli/blit.1.md:OPTIONS Admin Options
**Specificity**: high

`--json` (du, df) — Output results as JSON.

#### opt-yes-rm
**Source**: docs/cli/blit.1.md:OPTIONS Admin Options
**Specificity**: high

`--yes` (rm) — Skip confirmation prompt.

#### opt-pattern
**Source**: docs/cli/blit.1.md:OPTIONS Admin Options
**Specificity**: high

`--pattern <GLOB>` (find) — Glob pattern to match (e.g., "*.txt").

#### opt-case-insensitive
**Source**: docs/cli/blit.1.md:OPTIONS Admin Options
**Specificity**: high

`--case-insensitive` (find) — Enable case-insensitive pattern matching.

#### opt-limit-find
**Source**: docs/cli/blit.1.md:OPTIONS Admin Options
**Specificity**: high

`--limit <N>` (find) — Limit number of results.

#### opt-config-dir
**Source**: docs/cli/blit.1.md:CONFIGURATION DIRECTORY
**Specificity**: high

`--config-dir <PATH>` — overrides the default configuration directory.

#### opt-workers-changelog
**Source**: CHANGELOG.md:CLI (`blit`)
**Specificity**: medium

CHANGELOG lists `--workers` as a shipped CLI option alongside `--dry-run`, `--checksum`, `--force-grpc`.

#### diag-dump-json-flag
**Source**: docs/cli/blit.1.md:DIAGNOSTICS
**Specificity**: high

`blit diagnostics dump --json` emits machine-readable JSON instead of the human-readable default.

#### files-perf-local
**Source**: docs/cli/blit.1.md:FILES
**Specificity**: high

`${XDG_CONFIG_HOME:-$HOME/.config}/blit/perf_local.jsonl` – local performance history file path.

#### files-settings
**Source**: docs/cli/blit.1.md:FILES
**Specificity**: high

`${XDG_CONFIG_HOME:-$HOME/.config}/blit/settings.json` – persisted CLI settings file path.

#### remote-completion-cli-flags
**Source**: README.md:Admin Utilities
**Specificity**: high

`blit completions remote <REMOTE> [--prefix <STR>] [--files] [--dirs]` — `<REMOTE>` is the target host (e.g. `server:9031`) and `--prefix` narrows the returned path set.

### behavior

#### rsync-dest-nest
**Source**: docs/cli/blit.1.md:Destination Semantics table
**Specificity**: high

`blit copy SRC DEST/` or `blit copy SRC DEST` when `DEST` is an existing dir → `DEST/<basename(SRC)>/...` (nest).

#### rsync-dest-merge
**Source**: docs/cli/blit.1.md:Destination Semantics table
**Specificity**: high

`blit copy SRC/ DEST` or `blit copy SRC/. DEST` → `DEST/...` (merge contents).

#### rsync-dest-new
**Source**: docs/cli/blit.1.md:Destination Semantics table
**Specificity**: high

`blit copy SRC DEST` when `DEST` does not exist → `DEST/...` (DEST becomes copy of SRC).

#### rsync-file-into-dir
**Source**: docs/cli/blit.1.md:Destination Semantics table
**Specificity**: high

`blit copy file.txt DEST/` → `DEST/file.txt`.

#### rsync-file-rename
**Source**: docs/cli/blit.1.md:Destination Semantics table
**Specificity**: high

`blit copy file.txt renamed.txt` (renamed.txt does not exist) → `renamed.txt` (rename).

#### rsync-rule-summary
**Source**: docs/cli/blit.1.md:Destination Semantics
**Specificity**: high

The rule: a trailing slash (or `/.`) on the **source** means "copy the contents of this directory". Without a trailing slash, whether the source's basename is appended depends on the destination — if the destination has a trailing slash or is an existing directory, the source is nested under it; otherwise the destination path is used as the exact target.

#### rsync-windows-backslashes
**Source**: docs/cli/blit.1.md:Destination Semantics
**Specificity**: high

On Windows, trailing `\` and `\.` are also recognized.

#### rsync-remote-dest-no-probe
**Source**: docs/cli/blit.1.md:Destination Semantics
**Specificity**: high

For remote destinations, only the trailing slash is consulted (no directory probe).

#### remote-to-remote-direct-default
**Source**: docs/cli/blit.1.md:Destination Semantics
**Specificity**: high

Remote-to-remote transfers are supported (e.g., `blit copy server1:/mod/A server2:/mod/B`). By default, the CLI asks the destination daemon to pull directly from the source daemon, so payload bytes flow source→destination and do not cross the CLI host.

#### remote-to-remote-delegation-gate
**Source**: docs/cli/blit.1.md:Destination Semantics
**Specificity**: high

The destination daemon must opt in with `[delegation] allow_delegated_pull = true`; if its gate rejects the request, the CLI fails with the daemon's reason instead of silently relaying.

#### rsync-trailing-slash-applies-identically
**Source**: docs/cli/blit.1.md:Destination Semantics
**Specificity**: high

`copy`, `mirror`, and `move` resolve the destination using rsync's trailing-slash convention, applied identically regardless of which side is local or remote.

#### find-pattern-no-cross-slash
**Source**: docs/cli/blit.1.md:Admin Commands; CHANGELOG.md:Admin Utilities
**Specificity**: high

`find --pattern` glob: `*` does not cross `/`.

#### find-pattern-syntax
**Source**: CHANGELOG.md:Admin Utilities
**Specificity**: high

`find --pattern <GLOB>` uses POSIX shell-glob syntax (`*`, `?`, `[abc]`, `**/`).

#### find-pattern-matches-rel-and-basename
**Source**: CHANGELOG.md:Admin Utilities
**Specificity**: high

Pattern matches against both the relative path and the file-name basename so `*.csv` finds nested entries.

#### verbose-emits-to-stderr
**Source**: docs/cli/blit.1.md:OPTIONS
**Specificity**: high

`--verbose` emits planner heartbeat messages and fast-path decisions to stderr (not stdout).

#### default-comparison-metadata
**Source**: docs/cli/blit.1.md:OPTIONS
**Specificity**: high

Metadata comparison is the default for change detection (vs --checksum).

#### list-smart-dispatch-bare
**Source**: docs/cli/blit.1.md:Admin Commands; CHANGELOG.md:Admin Utilities
**Specificity**: high

`blit list <bare-host>` (`server`, `server:9031`) smart-dispatches to `list-modules`.

#### list-smart-dispatch-path
**Source**: docs/cli/blit.1.md:Admin Commands; CHANGELOG.md:Admin Utilities
**Specificity**: high

`blit list <target-with-module-or-path>` (e.g. `server:/module/`) falls through to `ls`.

#### mirror-default-prompt
**Source**: docs/cli/blit.1.md:OPTIONS
**Specificity**: high

By default, `mirror` prompts before deleting extraneous files at the destination.

#### move-default-prompt
**Source**: docs/cli/blit.1.md:OPTIONS
**Specificity**: high

By default, `move` prompts before deleting the source after transfer.

#### destructive-prompt-unless-yes
**Source**: CHANGELOG.md:CLI (`blit`)
**Specificity**: high

Destructive operations prompt unless `--yes` is supplied.

#### diag-dump-no-transfer
**Source**: docs/cli/blit.1.md:DIAGNOSTICS
**Specificity**: high

`blit diagnostics dump` performs no transfer; it only emits a snapshot.

#### diag-dump-same-device-local
**Source**: docs/cli/blit.1.md:DIAGNOSTICS
**Specificity**: high

For local source/destination pairs, `blit diagnostics dump` reports whether they are on the same device.

#### profile-local-only
**Source**: docs/cli/blit.1.md:Admin Commands
**Specificity**: high

`blit profile` does no network access; it reads `perf_local.jsonl` and the predictor state file.

#### completions-shell-stdout
**Source**: docs/cli/blit.1.md:Admin Commands; CHANGELOG.md:Admin Utilities
**Specificity**: high

`blit completions shell <SHELL>` writes the completion script to stdout; users pipe it to their completion directory.

#### completions-shells-supported
**Source**: docs/cli/blit.1.md:Admin Commands; README.md:Admin Utilities; CHANGELOG.md:Admin Utilities
**Specificity**: high

Completion shells supported: bash, zsh, fish, powershell, elvish.

#### resume-block-level-hashing
**Source**: README.md:Features; docs/cli/blit.1.md:OPTIONS; CHANGELOG.md:Transfer Engine
**Specificity**: high

`--resume` performs block-level comparison using hashing (Blake3 per README/CHANGELOG); only changed blocks are transferred.

#### find-streams-results
**Source**: docs/plan/BLIT_UTILS_PLAN.md:Command Matrix
**Specificity**: medium

`find` streams results (optional JSON output).

#### rm-respects-readonly
**Source**: docs/plan/BLIT_UTILS_PLAN.md:Command Matrix
**Specificity**: high

`rm` respects read-only modules.

#### du-depth-parameter
**Source**: docs/plan/BLIT_UTILS_PLAN.md:Command Matrix
**Specificity**: medium

`du` displays total size/file count; optional depth parameter.

### principle

#### principle-composable-stdout
**Source**: docs/plan/BLIT_UTILS_PLAN.md:Overview
**Specificity**: medium

Admin utilities must remain composable (stdout-friendly), support non-interactive scripting, and honour daemon security constraints.

#### principle-readonly-canonical-containment
**Source**: docs/plan/BLIT_UTILS_PLAN.md:Overview
**Specificity**: high

Daemon security constraints honoured: read-only modules, always-on canonical-path containment within each module.

#### principle-safety-first
**Source**: docs/plan/BLIT_UTILS_PLAN.md:UX Principles #1
**Specificity**: high

Safety First — destructive commands (`rm`) require confirmation unless `--yes` is provided. Read-only modules must reject mutation attempts with clear error messages.

#### principle-consistent-formatting
**Source**: docs/plan/BLIT_UTILS_PLAN.md:UX Principles #2
**Specificity**: high

Consistent Formatting — default output is tabular text; `--json` flag emits machine-parsable JSON arrays. Timestamps in ISO 8601; sizes printed via `format_bytes` alongside raw bytes when relevant.

#### principle-exit-codes
**Source**: docs/plan/BLIT_UTILS_PLAN.md:UX Principles #3
**Specificity**: high

Exit Codes — success returns 0; partial failures return non-zero with aggregated error messages.

#### principle-shared-endpoint-parsing
**Source**: docs/plan/BLIT_UTILS_PLAN.md:UX Principles #4
**Specificity**: high

Shared Endpoint Parsing — reuse `RemoteEndpoint` for URL parsing to ensure identical behaviour with `blit` CLI.

#### principle-auth-token-hook
**Source**: docs/plan/BLIT_UTILS_PLAN.md:UX Principles #5
**Specificity**: medium

Authentication Hooks — plan for future token support (e.g., `--auth-token`). CLI should accept but ignore token for now, forwarding to RPC once implemented.

#### principle-bytes-formatting
**Source**: README.md:Features; CHANGELOG.md:Admin Utilities
**Specificity**: medium

Human-readable byte formatting in `df` output.

#### principle-json-all-inspection
**Source**: CHANGELOG.md:Admin Utilities
**Specificity**: high

`--json` output is available for all inspection commands.

### invariant

#### inv-readonly-rejects
**Source**: docs/plan/BLIT_UTILS_PLAN.md:UX Principles
**Specificity**: high

Read-only modules must reject mutation attempts with clear error messages.

#### inv-canonical-path-containment
**Source**: docs/plan/BLIT_UTILS_PLAN.md:Overview, RPC Requirements (Purge)
**Specificity**: high

Daemon enforces always-on canonical-path containment within each module; Purge specifically enforces per-entry canonical-path containment.

#### inv-exit-zero-success
**Source**: docs/plan/BLIT_UTILS_PLAN.md:UX Principles
**Specificity**: high

Success exit code is 0.

#### inv-exit-nonzero-partial
**Source**: docs/plan/BLIT_UTILS_PLAN.md:UX Principles
**Specificity**: high

Partial failures return non-zero with aggregated error messages.

#### inv-endpoint-parsing-shared
**Source**: docs/plan/BLIT_UTILS_PLAN.md:UX Principles
**Specificity**: high

`RemoteEndpoint` parsing is shared between `blit` CLI and former blit-utils subcommands to ensure identical behaviour.

### scope

#### scope-admin-purpose
**Source**: docs/plan/BLIT_UTILS_PLAN.md:Overview
**Specificity**: medium

Admin utilities provide operator tooling for discovery, inspection, and maintenance of remote Blit daemons.

#### scope-cli-and-daemon
**Source**: README.md:Features
**Specificity**: medium

Blit ships both CLI and daemon binaries; CLI is minimal and ergonomic, daemon supports automation and concurrent requests.

#### scope-rust-2021
**Source**: README.md:Prerequisites
**Specificity**: high

Rust 1.56+ (edition 2021); protoc required for gRPC (auto-handled for most workflows); Windows, Linux, or macOS supported.

#### scope-supported-platforms
**Source**: README.md badges + Features; CHANGELOG.md:Platform Support
**Specificity**: high

Windows, Linux, macOS optimized — per-filesystem capability detection with platform-native fast-copy paths.

### shipped

#### shipped-scan
**Source**: docs/plan/BLIT_UTILS_PLAN.md:Command Matrix
**Specificity**: high

`scan` implemented 2025-10-23 (mDNS discovery of daemons advertising `_blit._tcp.local.`).

#### shipped-rm
**Source**: docs/plan/BLIT_UTILS_PLAN.md:Command Matrix
**Specificity**: high

`rm` implemented 2025-10-23.

#### shipped-find
**Source**: docs/plan/BLIT_UTILS_PLAN.md:Command Matrix
**Specificity**: high

`find` implemented 2025-10-24.

#### shipped-du
**Source**: docs/plan/BLIT_UTILS_PLAN.md:Command Matrix
**Specificity**: high

`du` implemented 2025-10-24.

#### shipped-df
**Source**: docs/plan/BLIT_UTILS_PLAN.md:Command Matrix
**Specificity**: high

`df` implemented 2025-10-24.

#### shipped-completions
**Source**: docs/plan/BLIT_UTILS_PLAN.md:Command Matrix
**Specificity**: high

`completions` implemented 2025-10-24 (CompletePath RPC; files/dirs filters supported).

#### shipped-clap-skeleton
**Source**: docs/plan/BLIT_UTILS_PLAN.md:Implementation Checklist #1
**Specificity**: medium

Clap-based subcommand skeleton fleshed out (Done).

#### shipped-shared-endpoint
**Source**: docs/plan/BLIT_UTILS_PLAN.md:Implementation Checklist #2
**Specificity**: medium

Endpoint parsing + gRPC client helpers shared via `util.rs` with `parse_endpoint_or_local`, `module_and_rel_path` (Done).

#### shipped-streaming-rpcs
**Source**: docs/plan/BLIT_UTILS_PLAN.md:Implementation Checklist #3
**Specificity**: medium

Streaming RPC consumption via tonic async clients — `find` and `du` use streaming RPCs (Done).

#### shipped-local-fallbacks
**Source**: docs/plan/BLIT_UTILS_PLAN.md:Implementation Checklist #4
**Specificity**: medium

Local fallbacks added: `profile` reads local JSONL, `ls` supports local paths (Done).

#### shipped-manpage-update
**Source**: docs/plan/BLIT_UTILS_PLAN.md:Implementation Checklist #6
**Specificity**: medium

Documentation (CLI manpages, quick-start) updated; `docs/cli/blit-utils.1.md` created 2026-03-06 (Done).

#### shipped-merge-into-blit
**Source**: CHANGELOG.md:Admin Utilities
**Specificity**: high

Originally a separate `blit-utils` binary; merged into `blit` for a single install/distribution surface.

#### shipped-merge-superseded-doc
**Source**: docs/plan/BLIT_UTILS_PLAN.md:Header
**Specificity**: high

BLIT_UTILS_PLAN is marked Superseded; admin utilities shipped as subcommands of `blit` (not separate `blit-utils` artifact). No `crates/blit-utils` crate, no `blit-utils(1)` manpage, no standalone binary.

#### shipped-cli-options
**Source**: CHANGELOG.md:CLI (`blit`)
**Specificity**: high

Shipped CLI options: `--dry-run`, `--checksum`, `--force-grpc`, `--workers`, plus `--progress`, `--verbose`.

#### shipped-cli-verbs
**Source**: CHANGELOG.md:CLI (`blit`)
**Specificity**: high

Shipped CLI transfer/admin verbs: `copy`, `mirror`, `move`, `scan`, `list`, `du`, `df`, `rm`, `find`.

#### shipped-diagnostics-perf
**Source**: CHANGELOG.md:CLI (`blit`)
**Specificity**: high

`diagnostics perf` shipped for performance history management.

#### shipped-admin-rpcs
**Source**: CHANGELOG.md:Daemon
**Specificity**: high

Admin RPCs shipped on daemon: ListModules, List, Find, DiskUsage, FilesystemStats, CompletePath, Purge.

#### shipped-daemon-flags
**Source**: CHANGELOG.md:Daemon
**Specificity**: high

Daemon flags shipped: `--root` default export, `--no-mdns`, `--force-grpc-data`.

#### shipped-hybrid-transport
**Source**: CHANGELOG.md:Transfer Engine; README.md:Features
**Specificity**: high

Hybrid transport: TCP data plane for high throughput, gRPC fallback (described as "fallback for diagnostics" in README; "control plane with gRPC fallback" in CHANGELOG).

#### shipped-resume
**Source**: CHANGELOG.md:Transfer Engine
**Specificity**: high

Block-level resumable transfers with Blake3 hashing (`--resume`).

#### shipped-r2r
**Source**: CHANGELOG.md:Transfer Engine
**Specificity**: high

Remote-to-remote transfers (`blit copy server1:/mod/ server2:/mod/`).

#### shipped-perf-jsonl
**Source**: CHANGELOG.md:Performance History
**Specificity**: high

JSONL storage with schema versioning (v0/v1 migration); capped at ~1 MiB with rotation; adaptive predictor with per-profile coefficients.

#### shipped-tests-admin
**Source**: CHANGELOG.md:Testing
**Specificity**: medium

Integration tests: admin verbs (10), admin commands (21, in `crates/blit-cli/tests/blit_utils.rs`), remote transfers, transfer edges, parity, resume, move, remote-to-remote.

### deferred

#### deferred-integration-tests
**Source**: docs/plan/BLIT_UTILS_PLAN.md:Implementation Checklist #5
**Specificity**: high

Integration tests calling daemon RPCs (Phase 3.5 test suite) — Pending.

#### deferred-auth-token
**Source**: docs/plan/BLIT_UTILS_PLAN.md:UX Principles #5
**Specificity**: medium

`--auth-token` flag future work; CLI should accept but ignore token for now, forwarding to RPC once implemented.

### non-goal

#### non-goal-tls
**Source**: docs/cli/blit.1.md:SECURITY; CHANGELOG.md:Security/Known Limitations
**Specificity**: high

Remote transfers do not include built-in TLS encryption. Data is transmitted in plaintext over the TCP data plane and gRPC control plane. Operators are expected to secure remote transfers themselves (trusted network, SSH tunnel, VPN, reverse proxy with TLS).

#### non-goal-auth-beyond-modules
**Source**: CHANGELOG.md:Known Limitations
**Specificity**: high

No authentication beyond module-level access control.

#### non-goal-utils-binary
**Source**: docs/plan/BLIT_UTILS_PLAN.md:Header
**Specificity**: high

No separate `blit-utils` binary; no `crates/blit-utils` crate; no `blit-utils(1)` manpage.

### decision

#### decision-default-pull-direct
**Source**: docs/cli/blit.1.md:Destination Semantics
**Specificity**: high

Default remote-to-remote mode is direct delegated pull (destination daemon pulls from source); legacy CLI-relay is opt-in via `--relay-via-cli`.

#### decision-bind-zero
**Source**: docs/cli/blit.1.md:SECURITY
**Specificity**: high

The daemon binds to `0.0.0.0` by default. In untrusted environments, use `--bind 127.0.0.1` and access via SSH tunnel or VPN.

#### decision-default-mdns-wait
**Source**: docs/cli/blit.1.md:OPTIONS
**Specificity**: high

Default mDNS wait duration is 2 seconds.

#### decision-default-max-depth-unlimited
**Source**: docs/cli/blit.1.md:OPTIONS
**Specificity**: high

`du --max-depth 0` means unlimited traversal depth.

#### decision-merge-blit-utils
**Source**: CHANGELOG.md:Admin Utilities; BLIT_UTILS_PLAN.md:Header
**Specificity**: high

blit-utils merged into `blit` for a single install/distribution surface.

#### decision-completions-via-clap
**Source**: docs/cli/blit.1.md:Admin Commands; CHANGELOG.md:Admin Utilities
**Specificity**: high

`blit completions shell` uses `clap_complete` to generate static completion scripts.

#### decision-completepath-rpc
**Source**: docs/cli/blit.1.md:Admin Commands
**Specificity**: high

Remote-path completions are served by daemon `CompletePath` RPC, called internally by the generated shell scripts.

### rejected

(none called out explicitly within this cluster beyond the "non-goal" items above)

---

## Contradictions

1. **gRPC role description**: README.md:Features describes gRPC as a "fallback for diagnostics" ("TCP data plane for high-throughput transfers (10+ Gbps), with gRPC fallback for diagnostics"), while CHANGELOG.md:Transfer Engine describes it as the "control plane" ("Remote push/pull via hybrid TCP data plane + gRPC control plane") and blit.1.md:DESCRIPTION says "hybrid TCP/gRPC transport". The role of gRPC (control plane vs diagnostic fallback) is described inconsistently across the three docs.

2. **`--workers` flag presence**: CHANGELOG.md:CLI lists `--workers` as a shipped CLI option, but the manpage (docs/cli/blit.1.md) does not document `--workers` anywhere in OPTIONS, SYNOPSIS, or DIAGNOSTICS. Either the manpage omits a shipped flag or the CHANGELOG references a flag not in the live surface.

3. **`scan --json` and `list --json` documentation gap**: The SYNOPSIS in blit.1.md lists `--json` on `scan`, `list`, `list-modules`, `ls`, `du`, `df`, `find`, but the OPTIONS section only explicitly documents `--json` for `du, df` (line 141-142) — `--json` for scan/list/list-modules/ls/find is in the SYNOPSIS but missing from the OPTIONS description.

4. **`--limit` for profile**: SYNOPSIS lists `blit profile [--limit <N>] [--json]` but OPTIONS section does not describe `--limit` for `profile` (only documents `--limit <N>` for `find`).

5. **BLIT_UTILS_PLAN `completions` shape mismatch**: BLIT_UTILS_PLAN's Command Matrix lists `blit-utils completions <remote[:/path]> [--prefix <fragment>]`, but the live surface in blit.1.md splits completions into two subcommands: `completions shell <SHELL>` (static script generation, new) and `completions remote <REMOTE>` (daemon-backed, original). The plan's single-form completions has been restructured.

6. **`list` command in plan vs shipped**: BLIT_UTILS_PLAN describes `list` as "Alias of `ls` (compatible with plan v6)", but the shipped behavior in blit.1.md is a *smart dispatcher* that routes by target shape (bare host → `list-modules`, with-module → `ls`), not a pure alias of `ls`.

7. **Manpage date stamp**: blit.1.md is dated 2025-11-21 in its header but the CHANGELOG version 0.1.0 is dated 2026-05-31; the manpage predates the changelog version. Document freshness drift.

---

## Coverage attestation

| File | Lines | Notes |
|---|---|---|
| docs/cli/blit.1.md | 202 | Read end-to-end (single pass, no pagination needed). NAME/SYNOPSIS/DESCRIPTION/Transfer Commands/Destination Semantics table + rule prose + Windows note + remote dest note/Admin Commands/OPTIONS (Transfer Options, Admin Options)/DIAGNOSTICS/CONFIGURATION DIRECTORY/FILES/SECURITY (incl. SSH/VPN/reverse proxy bullets and bind note)/SEE ALSO. |
| README.md | 153 | Read end-to-end. Badges, intro, Features list (10 bullets), Repo Structure, Quick Start (Prerequisites, Building & Testing, Usage examples, Daemon, Admin Utilities + remote completions paragraph). |
| CHANGELOG.md | 78 | Read end-to-end. v0.1.0 sections: Transfer Engine, Platform Support, CLI, Daemon, Admin Utilities, Performance History, Documentation, Testing, Security, Known Limitations. |
| docs/plan/BLIT_UTILS_PLAN.md | 72 | Read end-to-end. Superseded banner, Overview, Command Matrix (10 rows), UX Principles (5 numbered), RPC Requirements (6 bullets), Implementation Checklist (6 items), Testing Strategy (4 bullets). |

**Total lines**: 505
