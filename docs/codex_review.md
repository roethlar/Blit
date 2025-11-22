# Code Review: Admin/Transfer Additions (remote tooling)

**Reviewer**: Codex  
**Date**: 2025-11-09  
**Scope**: New CLI admin verbs (du/df/find/rm), remote move extensions, remote transfer sources, and daemon config parsing.

---

## Summary
- Remote-to-remote transfers now disregard the requested source subpath and can delete the wrong content. The new admin verbs bypass shared connection/config logic. A new daemon config parser is added but not integrated. Tar building for remote sources fully buffers files.

## Findings (ordered by severity)

### Critical — Remote path scoping & deletion risk
- **File**: `crates/blit-core/src/remote/transfer/source.rs` (RemoteTransferSource::scan)  
  - Always scans `"."`, ignoring the caller’s requested subpath. Remote→remote push/move will copy the entire module instead of the target path.
- **File**: `crates/blit-cli/src/transfers/mod.rs` (run_move)  
  - After a remote→remote push, the source path is unconditionally deleted. With the mis-scoped scan above, we can delete the user-specified path even though the transfer copied the wrong scope.
- **Impact**: Incorrect data copied; potential destructive delete of user path.
- **Action**: Respect the source relative path in scan/payload building and only delete after a correctly scoped transfer completes (or gate on a verified manifest match).

### Critical — Tar construction fully buffers remote files
- **File**: `crates/blit-core/src/remote/transfer/source.rs` (prepare_payload for RemoteTransferSource)  
  - Reads each remote file into memory and builds tar in-memory. Large files/shards will exhaust memory and violate streaming design.
- **Impact**: OOM risk, violates streaming data-plane goals.
- **Action**: Stream remote file reads into tar or reuse the existing streaming data-plane; avoid `read_to_end` on entire files.

### High — Admin verbs bypass shared config/context
- **Files**: `crates/blit-cli/src/admin.rs`, `crates/blit-cli/src/cli.rs`, `crates/blit-cli/src/main.rs`  
  - Admin commands open raw gRPC connections without `AppContext`/config-dir/TLS/mdns handling used elsewhere.
- **Impact**: Fails against secured daemons; diverges from established connection behavior.
- **Action**: Thread `AppContext`/config resolution into admin verbs or clearly document the limitation.

### Medium — Duplicate daemon config types
- **File**: `crates/blit-daemon/src/config.rs`  
  - Introduces new `DaemonConfig`/`ModuleConfig` parser parallel to existing config structures in `crates/blit-daemon/src/types.rs` and is not integrated.
- **Impact**: Confusion and maintenance overhead.
- **Action**: Consolidate with the canonical config parser or remove unused code.

### Low — Misc polish
- **File**: `crates/blit-cli/src/cli.rs`  
  - Duplicate doc comment on `List` variant.

---

## Recommendations
1. Fix remote source scoping in `RemoteTransferSource` and guard deletes in `run_move` behind verified, scoped transfers.
2. Replace in-memory tar building for remote sources with streamed reads or data-plane reuse.
3. Route admin verbs through shared connection/config logic (or document explicit limitations).
4. Remove or merge the new daemon config parser to a single source of truth.
5. Clean minor doc/comment nits once higher-priority fixes land.
