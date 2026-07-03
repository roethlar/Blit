# BLIT-UTILS(1) blit Manual
% Blit v2 Team
% 2026-03-06

## NAME
blit-utils - administrative tooling for Blit v2 daemons

## SYNOPSIS
`blit-utils scan [--wait <SECONDS>]`
`blit-utils list-modules [--json] <REMOTE>`
`blit-utils ls [--json] <TARGET>`
`blit-utils find [--pattern <PAT>] [--files] [--dirs] [--case-insensitive] [--limit <N>] [--json] <REMOTE>`
`blit-utils du [--max-depth <N>] [--json] <REMOTE>`
`blit-utils df [--json] <REMOTE>`
`blit-utils rm [--yes] <REMOTE>`
`blit-utils completions [--files] [--dirs] [--prefix <FRAGMENT>] <REMOTE>`
`blit-utils profile [--json] [--limit <N>]`

## DESCRIPTION
`blit-utils` provides operator tooling for discovery, inspection, and
maintenance of remote Blit daemons. All remote commands connect via gRPC to a
running `blit-daemon`. Output defaults to human-readable tables; most commands
accept `--json` for machine-parsable output.

### Commands

- `scan`
  Discovers blit daemons on the local network via mDNS (`_blit._tcp.local.`).
  Prints host, port, addresses, version, and exported modules for each daemon
  found within the wait period.

- `list-modules`
  Lists modules exported by a daemon. Output includes module name, read/write
  mode, and filesystem path.

- `ls` (alias: `list`)
  Lists directory entries within a remote module path or a local directory.
  Remote targets use the `host:/module/path` syntax. Displays entry type
  (FILE/DIR), size, and name.

- `find`
  Recursively searches for files matching a pattern on a remote daemon. Results
  stream as they are found. The pattern is a substring match (not glob).

- `du`
  Summarises disk usage for a remote subtree. Displays path, total bytes, file
  count, and directory count. Use `--max-depth` to limit traversal.

- `df`
  Reports filesystem statistics (total, used, free bytes) for a remote module.

- `rm`
  Removes files or directories on a remote daemon. Prompts for confirmation
  unless `--yes` is provided. Refuses to delete an entire module root; a
  sub-path must be specified. Read-only modules reject deletion.

- `completions`
  Fetches remote path completions for interactive shell integration. Outputs
  one completion per line. Use `--files` or `--dirs` to filter by entry type.

- `profile`
  Displays local performance history and predictor state. Shows whether
  history capture is enabled, the number of stored records, and the predictor
  state file location.

## OPTIONS

### scan
- `--wait <SECONDS>`
  Duration to wait for mDNS responses (default: 2).

### list-modules
- `--json`
  Output as JSON array of objects with `name`, `path`, `read_only` fields.

### ls
- `--json`
  Output entries as a JSON array with `name`, `is_dir`, `size`, `mtime_seconds`.

### find
- `--pattern <PAT>`
  Substring pattern to match against file/directory names.

- `--files`
  Include only files in results.

- `--dirs`
  Include only directories in results.

- `--case-insensitive`
  Enable case-insensitive pattern matching.

- `--limit <N>`
  Maximum number of results to return (0 = unlimited).

- `--json`
  Output as JSON array with `path`, `is_dir`, `size`, `mtime_seconds`.

### du
- `--max-depth <N>`
  Limit traversal depth (0 = unlimited).

- `--json`
  Output as JSON array with `path`, `bytes`, `files`, `dirs`.

### df
- `--json`
  Output as JSON object with `module`, `total_bytes`, `used_bytes`, `free_bytes`.

### rm
- `--yes`
  Skip the confirmation prompt.

### completions
- `--files`
  Include only file completions.

- `--dirs`
  Include only directory completions.

- `--prefix <FRAGMENT>`
  Additional prefix appended to the base path for filtering.

### profile
- `--json`
  Output as JSON with `enabled`, `records`, and `predictor_path`.

- `--limit <N>`
  Number of recent records to load (default: 50).

## REMOTE ENDPOINT SYNTAX

Remote targets follow the same syntax as `blit`:

- `server:/module/path` - explicit module and path
- `server:/module/` - module root
- `server` - bare host (valid for `list-modules` and `scan`)

Remote paths must use forward slashes (`/`), not backslashes.

## EXAMPLES

```sh
# Discover daemons on the LAN
blit-utils scan --wait 5

# List modules in JSON
blit-utils list-modules fileserver --json

# Browse a remote directory
blit-utils ls fileserver:/backup/2026/

# Find all CSV files in a module
blit-utils find fileserver:/data/ --pattern ".csv" --files

# Check free space
blit-utils df fileserver:/backup

# Remove old data non-interactively
blit-utils rm fileserver:/backup/2024-archive --yes

# Feed completions to a shell script
blit-utils completions fileserver:/data/ --dirs --prefix "proj"
```

## EXIT CODES

- **0** - Success.
- **1** - Error (connection failure, RPC error, invalid arguments).

## SEE ALSO
`blit(1)`, `blit-daemon(1)`
