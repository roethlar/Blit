# blit-utils Consolidation — Changes Made

Response to `CONSOLIDATION_REQUIREMENTS.md`. All requirements and nice-to-haves completed.

---

## Summary for Admin UI Developers

**One binary: `blit`.** The `blit-utils` crate has been removed. All commands are in `blit-cli`. Every query/admin command supports `--json`. Transfer commands (`copy`, `mirror`, `move`) support `--json` for both structured progress (NDJSON on stderr) and final summary (JSON on stdout).

---

## Complete CLI Surface

```
TRANSFERS
  blit copy   <SOURCE> <DEST> [--json] [-p] [-v] [-y] [--dry-run] [--checksum] [--resume] [--null] [--force-grpc]
  blit mirror <SOURCE> <DEST> [--json] [-p] [-v] [-y] [--dry-run] [--checksum] [--resume] [--null] [--force-grpc]
  blit move   <SOURCE> <DEST> [--json] [-p] [-v] [-y] [--dry-run] [--checksum] [--resume] [--force-grpc]

DISCOVERY & INSPECTION
  blit scan [--wait <N>] [--json]
  blit list-modules <REMOTE> [--json]
  blit ls <TARGET> [--json]                          (alias: list)
  blit du <REMOTE> [--max-depth <N>] [--json]
  blit df <REMOTE> [--json]
  blit find <REMOTE> [--pattern <PAT>] [--files] [--dirs] [--case-insensitive] [--limit <N>] [--json]

ADMIN
  blit rm <REMOTE> [--yes] [--json]
  blit completions <REMOTE> [--files] [--dirs] [--prefix <FRAG>]

DIAGNOSTICS
  blit profile [--json] [--limit <N>]
  blit diagnostics perf [--json] [--limit <N>] [--enable] [--disable] [--clear]
```

---

## JSON Response Shapes (verified from source)

### `blit scan --json`
```json
[{
  "instance_name": "blit@myserver",
  "host": "myserver.local",
  "port": 9031,
  "addresses": ["192.168.1.100"],
  "version": "0.1.0",
  "modules": ["backup", "media"]
}]
```

### `blit list-modules <REMOTE> --json`
```json
[{ "name": "backup", "path": "/data/backups", "read_only": false }]
```

### `blit ls <TARGET> --json`
```json
[{ "name": "file.txt", "is_dir": false, "size": 1024, "mtime_seconds": 1712534400 }]
```

### `blit du <REMOTE> --json`
```json
[{ "path": "subdir", "bytes": 10485760, "files": 42, "dirs": 3 }]
```

### `blit df <REMOTE> --json`
```json
{ "module": "backup", "total_bytes": 499963174912, "used_bytes": 312475648000, "free_bytes": 187487526912 }
```

### `blit find <REMOTE> --json`
```json
[{ "path": "data/report.csv", "is_dir": false, "size": 2048, "mtime_seconds": 1712534400 }]
```

### `blit rm <REMOTE> --yes --json`
```json
{ "path": "old-data/archive", "host": "192.168.1.100", "port": 9031, "entries_deleted": 47 }
```

### `blit profile --json`
```json
{ "enabled": true, "records": [...], "predictor_path": "/path/to/predictor.json" }
```

### `blit diagnostics perf --json`
```json
{ "enabled": true, "history_path": "...", "record_count": 5, "records": [...] }
```

---

## Transfer Progress (NDJSON on stderr)

When running transfers with `-p --json`, progress is emitted as newline-delimited JSON objects to **stderr** (one per second, plus one per file completion). The final transfer summary goes to **stdout** as a single JSON object.

### stderr (NDJSON progress stream)

```
{"event":"manifest","total_files":150}
{"event":"progress","files":25,"total_files":150,"bytes_copied":537919488,"avg_bytes_sec":1073741824,"current_bytes_sec":1258291200}
{"event":"file_complete","path":"data/backup.tar","bytes":1073741824}
{"event":"progress","files":100,"total_files":150,"bytes_copied":1610612736,"avg_bytes_sec":1006632960,"current_bytes_sec":985661440}
{"event":"final","files_transferred":150,"total_bytes":2147483648,"avg_bytes_sec":1053818880}
```

**Event types:**

| Event | Fields | When |
|-------|--------|------|
| `manifest` | `total_files` | After manifest enumeration |
| `progress` | `files`, `total_files`, `bytes_copied`, `avg_bytes_sec`, `current_bytes_sec` | Every ~1 second during transfer |
| `file_complete` | `path`, `bytes` | After each file finishes |
| `final` | `files_transferred`, `total_bytes`, `avg_bytes_sec` | Transfer complete |

All numeric values are raw integers (bytes, bytes/sec). No string formatting.

### stdout (final summary)

**Local transfer:**
```json
{
  "operation": "copy",
  "source": "/data/src",
  "destination": "/data/dst",
  "files_transferred": 150,
  "total_bytes": 2147483648,
  "deleted_files": 0,
  "deleted_dirs": 0,
  "duration_ms": 2038,
  "dry_run": false
}
```

**Remote push:**
```json
{
  "operation": "push",
  "destination": "server:/module/",
  "files_requested": 150,
  "files_transferred": 150,
  "bytes_transferred": 2147483648,
  "bytes_zero_copy": 0,
  "entries_deleted": 0,
  "tcp_fallback": false,
  "first_payload_ms": 245
}
```

**Remote pull:**
```json
{
  "operation": "pull",
  "destination": "/local/path",
  "files_transferred": 150,
  "bytes_transferred": 2147483648,
  "bytes_zero_copy": 0,
  "tcp_fallback": false
}
```

---

## Swift Integration Pattern

```swift
// Read NDJSON progress from stderr
for await line in process.standardError.lines {
    let data = Data(line.utf8)
    let event = try JSONDecoder().decode(ProgressEvent.self, from: data)
    switch event.event {
    case "progress":
        updateProgressBar(files: event.files, total: event.totalFiles, bytes: event.bytesCopied)
    case "file_complete":
        appendToLog(event.path)
    case "final":
        showCompletionBanner(event.filesTransferred, event.totalBytes)
    default:
        break
    }
}

// Read final summary from stdout
let summaryData = process.standardOutput.readToEnd()
let summary = try JSONDecoder().decode(TransferSummary.self, from: summaryData)
```

---

## Requirements Addressed

### Requirement 1: `--json` on all commands
All query commands plus transfers now support `--json`. Complete.

### Requirement 2: Missing subcommands merged
`list-modules`, `ls`, `completions`, `profile` added to `blit`. Complete.

### Requirement 3: `blit df` human-readable output
Uses `format_bytes()`: `Total: 465.63 GiB (499963174912 bytes)`. Complete.

### Requirement 4: Duplicate `format_bytes()`
Single implementation in `util.rs`. Complete.

### Requirement 5: Unused `list.rs`
Deleted, replaced by `ls.rs`. Complete.

### Requirement 6: Error messages
All `"blit-utils ..."` messages changed to `"blit ..."`. Complete.

### Nice-to-have: Unused `_ctx` parameters
Removed from `run_remote_push_transfer` and `run_remote_pull_transfer`. Complete.

### Nice-to-have: `--json` on `diagnostics perf`
Added. Outputs `enabled`, `history_path`, `record_count`, and full `records` array. Complete.

### Nice-to-have: Transfer `--json` with NDJSON progress
Added. `-p --json` emits structured progress to stderr, summary to stdout. Complete.

---

## Workspace Changes

- `crates/blit-utils` removed from workspace members
- Build scripts reference only `blit-cli` + `blit-daemon`
- CI workflow builds/tests only `blit-cli` + `blit-daemon`
- All 31 integration tests use `blit-cli` binary exclusively
