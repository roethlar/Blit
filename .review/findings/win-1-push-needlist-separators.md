# win-1: Windows daemon push need-list echoed native separators — nested pushes stalled

**Source**: found 2026-07-03 while validating `ue-r2-1h` on the owner's
Windows host (the first time the full suite ran on Windows; STATE's
Windows note anticipated exactly this triage).
**Severity**: High (any push of nested paths to a Windows daemon stalls
30s and fails; affects push AND the relay's push half).
**Status**: Fixed same day, own commit (reviewed with the ue-r2-1h batch).

## What

`crates/blit-daemon/src/service/push/control.rs` rebuilt each manifest
entry's wire path with `PathBuf::to_string_lossy()` after validation:

```rust
let rel = resolve_manifest_relative_path(&file.relative_path)?; // PathBuf
let sanitized = rel.to_string_lossy().to_string();              // "nested\mid.txt" on Windows
```

The need-list (`FilesToUpload`) echoed those strings back to the
client. The client's planner keys its `manifest_lookup` by its own
POSIX wire paths (`nested/mid.txt`), so on a Windows daemon every
need-list entry containing a separator missed the lookup,
`drain_pending_headers` parked it forever, the client planned zero
payloads for those files, never finished the data plane, and the
daemon's 30s receive stall guard killed the transfer:
`"data plane receive: reading data-plane record tag: transfer
stalled"`. Top-level files (no separator) were unaffected — which is
why single-file/flat-layout tests always passed and CI on
linux/macos (where `to_string_lossy` yields `/`) never saw it.

## Fix

Emit the canonical POSIX form via the existing chokepoint:

```rust
let sanitized = blit_core::path_posix::relative_path_to_posix(&rel);
```

Identical strings on unix (behavior unchanged there); on Windows the
echo now matches the client's keys. Sole wire-path production site in
the daemon that used `to_string_lossy` (workspace grep); the other
`to_string_lossy` uses are display/admin values, not wire paths.

## Evidence / guarding tests

Failing on Windows before the fix, green after (each reproduced in
isolation, not just under load):

- `remote_transfer_edges::test_push_nested_directories` (timeout-kill)
- `remote_tcp_fallback::forced_grpc_push_many_files_completes`
  (300s timeout → 0.77s)
- `remote_remote::remote_to_remote_relay_transfers_nested_tree` (the
  new ue-r2-1h relay e2e that surfaced the bug)

These pins are platform-conditional by nature: they only guard the
regression when the suite runs on a Windows host (now the owner's
primary dev machine; also windows-latest CI).
