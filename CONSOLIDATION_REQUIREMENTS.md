# blit-utils Consolidation into blit-cli

## Goal

Deprecate `blit-utils` by rolling all of its functionality into `blit-cli` (`blit`). After this work, `blit` is the single binary for all operations. Standardize `--json` output across every subcommand.

---

## Requirements

### 1. Add `--json` to commands that lack it

| Command | Has `--json` today | Notes |
|---------|-------------------|-------|
| `blit du` | Yes | No action needed |
| `blit df` | Yes | No action needed |
| `blit find` | Yes | No action needed |
| `blit scan` | **No** | Add `--json`. Output array of objects: `{ instance_name, host, port, addresses: [str], version, modules: [str] }` |
| `blit list` | **No** | Add `--json`. When listing modules: `[{ name, path, read_only }]`. When listing directory: `[{ name, is_dir, size, mtime_seconds }]` |
| `blit rm` | **No** | Add `--json`. Output: `{ path, host, port, entries_deleted }` |

### 2. Add missing subcommands from blit-utils

These exist in `blit-utils` but not in `blit-cli`:

| Command | What it does | Suggested integration |
|---------|-------------|----------------------|
| `list-modules` | Lists modules exported by a daemon (`--json` supported) | Merge into `blit list`. When target is a bare host (discovery endpoint), list modules. Already implemented in the unused `src/list.rs` — wire it up. |
| `ls` | Directory listing with `--json`, includes `mtime_seconds` | Merge into `blit list`. When target includes a module/path, list directory contents. The unused `src/list.rs` already handles both cases. |
| `completions` | Shell path completion for remote paths | Add as `blit completions` subcommand. Copy from blit-utils as-is. |
| `profile` | Shows perf history status (read-only) | Merge into `blit diagnostics perf`. When called with no management flags (no `--enable`, `--disable`, `--clear`), show the profile status output. Remove `profile` as a separate concept. |

### 3. Fix `blit df` text output — missing byte formatting

`blit df` (in `admin.rs`) outputs raw byte counts:
```
Total: 10737418240 bytes
```

`blit-utils df` formats them:
```
Total: 10.00 GiB (10737418240 bytes)
```

Fix `blit df` text output to use `format_bytes()` like blit-utils does.

### 4. Consolidate duplicate `format_bytes()` implementations

Two versions exist in blit-cli:
- `src/util.rs` — formats `1024` as `"1024.00 B"` (always uses decimals)
- `src/transfers/mod.rs` — formats `1024` as `"1024 B"` (no decimals for byte unit)

Pick one (the `transfers/` version is better — raw bytes don't need decimal places) and delete the other. Use it everywhere including the `df` fix above.

### 5. Remove unused `src/list.rs`

`src/list.rs` (156 lines) implements a unified list command that handles both module listing and directory listing, supporting local and remote paths. It is never called from `main.rs` — `ls.rs` is dispatched instead.

Either wire up `list.rs` as the implementation for the consolidated `blit list` command (recommended — it already does what we need), or delete it if starting fresh.

### 6. Update error messages

All error messages that reference `blit-utils` need to say `blit`:
```
// Before
"`blit-utils du` requires a remote path"
// After
"`blit du` requires a remote path"
```

This applies to: `du`, `df`, `find`, `rm`, `ls`, `list-modules`, `completions`.

---

## Nice to Have

### Standardize function signatures

Some command functions take args by value (`args: ScanArgs`), others by reference (`args: &ScanArgs`). Pick one convention (references are more idiomatic for read-only access) and apply consistently.

### Clean up unused `_ctx` parameters

Several admin functions accept `_ctx: &AppContext` but never use it. Either remove the parameter or use it for something (e.g., respecting config-dir for format preferences).

### Add `--json` to `blit diagnostics perf`

Currently only has text output. JSON would be useful for tooling that wants to read perf history programmatically (including the admin app).

### Standardize scan text output

`blit scan` and `blit-utils scan` produce different text formats. blit-cli's version is better (produces copy-pasteable endpoint strings, suppresses default port 9031). Keep that format. With `--json` added this matters less, but the text output should still be clean.

### Consider `blit list` subcommand unification

After merging `list-modules` and `ls` into `blit list`, the behavior would be:
- `blit list server` → list modules (server is a bare host)
- `blit list server:/module/` → list directory contents
- `blit list /local/path` → list local directory

This is what the unused `list.rs` already implements. Add `--json` on top and it covers all listing use cases with one command.
