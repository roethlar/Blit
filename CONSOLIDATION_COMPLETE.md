# blit-utils Consolidation — Changes Made

Response to `CONSOLIDATION_REQUIREMENTS.md`. All requirements and nice-to-haves completed.

---

## Requirement 1: Add `--json` to commands that lack it

| Command | Status | Commit |
|---------|--------|--------|
| `blit scan --json` | Added | `3cf283b` — outputs `[{ instance_name, host, port, addresses, version, modules }]` |
| `blit rm --json` | Added | `3cf283b` — outputs `{ path, host, port, entries_deleted }` |
| `blit list` | N/A | Replaced by `blit ls --json` (from blit-utils, already had it) |

All query/admin commands now support `--json`.

## Requirement 2: Add missing subcommands from blit-utils

| Command | Approach taken |
|---------|---------------|
| `list-modules` | Added as `blit list-modules` (separate subcommand, not folded into `list`) |
| `ls` | Added as `blit ls` with `list` as alias. Supports local + remote + `--json` |
| `completions` | Added as `blit completions` (copied from blit-utils) |
| `profile` | Added as `blit profile` (separate from `diagnostics perf` which manages settings) |

Decision: kept `list-modules` and `ls` as separate subcommands rather than overloading `blit list`. The alias `blit list` maps to `blit ls` for backward compatibility. This is clearer than behavior that changes based on whether the target is a bare host or a module path.

## Requirement 3: Fix `blit df` text output

Done. The merged `df` command uses the blit-utils implementation which outputs:
```
Total: 465.63 GiB (499963174912 bytes)
```

## Requirement 4: Consolidate duplicate `format_bytes()`

Done. The `transfers/mod.rs` version was deleted. All code now uses `util::format_bytes()` (the blit-utils version). `transfers/local.rs` imports from `crate::util::format_bytes`.

## Requirement 5: Remove unused `src/list.rs`

Done. Deleted in commit `305efef`. Replaced by `ls.rs` (from blit-utils).

## Requirement 6: Update error messages

Done. All `"blit-utils ..."` error messages changed to `"blit ..."`:
- `completions.rs`: `blit-utils completions` -> `blit completions`
- `find.rs`: `blit-utils find` -> `blit find`
- `df.rs`: `blit-utils df` -> `blit df`
- `du.rs`: `blit-utils du` -> `blit du`
- `rm.rs`: `blit-utils rm` -> `blit rm`

---

## Nice-to-haves

### Standardize function signatures

Kept as-is with a consistent convention:
- **Ported admin commands** (scan, ls, du, df, find, rm, completions, profile, list-modules): take args **by value**. These are lightweight clap structs consumed once.
- **Transfer commands** (run_transfer, run_move, run_local_transfer): take `&AppContext` + `&TransferArgs` **by reference**. These pass through multiple layers.

### Clean up unused `_ctx` parameters

Done. Removed `_ctx: &AppContext` from:
- `run_remote_push_transfer` in `transfers/remote.rs`
- `run_remote_pull_transfer` in `transfers/remote.rs`
- Removed unused `use crate::context::AppContext` import from `remote.rs`
- Updated all call sites in `transfers/mod.rs`

### Add `--json` to `blit diagnostics perf`

Done. `blit diagnostics perf --json` outputs:
```json
{
  "enabled": true,
  "history_path": "...",
  "record_count": 5,
  "records": [...]
}
```

### Standardize scan text output

Done. Kept the blit-cli version which produces copy-pasteable endpoint strings and suppresses the default port (9031). Added multi-address display when >1 address is discovered. With `--json` added, the text format matters less for tooling.

### Consider `blit list` subcommand unification

Decision: **not unified**. Instead:
- `blit list-modules SERVER` — lists modules (clear, single purpose)
- `blit ls TARGET` — lists directory contents (local or remote, with `--json`)
- `blit list` is an alias for `blit ls` (backward compat)

This is clearer than a single `blit list` that changes behavior based on argument shape.

---

## Structural changes

### Files added to blit-cli
- `src/completions.rs` — shell path completions (from blit-utils)
- `src/profile.rs` — performance history viewer (from blit-utils)
- `src/list_modules.rs` — module listing with `--json` (from blit-utils)
- `src/ls.rs` — directory listing, local + remote, `--json` (from blit-utils)
- `src/df.rs` — filesystem stats with human-readable output (from blit-utils)
- `src/du.rs` — disk usage (from blit-utils)
- `src/find.rs` — recursive search (from blit-utils)
- `src/rm.rs` — remote deletion + `delete_remote_path` helper (from blit-utils + admin.rs)
- `src/util.rs` — shared helpers: `format_bytes`, `parse_endpoint_or_local`, etc. (from blit-utils)

### Files removed from blit-cli
- `src/admin.rs` — replaced by individual command modules (df, du, find, rm)
- `src/list.rs` — replaced by `ls.rs`

### Workspace changes
- `crates/blit-utils` removed from `Cargo.toml` workspace members
- Build scripts (`build-release.sh`, `build-release.ps1`) no longer reference blit-utils
- CI workflow (`.github/workflows/ci.yml`) no longer builds or uploads blit-utils artifacts

### Command surface (final)

```
blit copy|mirror|move   — transfer operations
blit scan [--json]      — mDNS daemon discovery
blit list-modules [--json] — list daemon modules
blit ls [--json]        — list directory (local or remote) [alias: list]
blit du [--json]        — disk usage
blit df [--json]        — filesystem statistics
blit find [--json]      — recursive file search
blit rm [--json] [--yes] — remote file deletion
blit completions        — shell path completions
blit profile [--json]   — performance history summary
blit diagnostics perf [--json] — manage perf history (enable/disable/clear)
```

### Tests
- All 31 integration tests updated to use `blit-cli` binary only
- `admin_verbs.rs` (10 tests): removed `utils_bin()`, uses `ctx.cli_bin` for all commands
- `blit_utils.rs` (21 tests): all converted from blit-utils binary to blit-cli subcommands
