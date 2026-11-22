# cr-ecm16-3: on macOS a file copied on its own under `--ignore-existing` no longer carries ACLs and extended attributes

**Severity**: HIGH — on macOS, a local copy with `--ignore-existing` loses a file's ACLs, extended attributes and resource fork, which the same copy without the option keeps
**Status**: Admitted — NOT FIXED. Blocked: the remedy is macOS-only code (`fcopyfile` on the held destination) that must be verified on a Mac, and no Mac is reachable from this machine (2026-11-22).
**Branch**: — (default-branch mode; fixes land on master)
**Commit**: —
**Reviewer**: codex / gpt-6.2-astra (as dispatched) / max / frontier, codex-cli 0.171.0 headless one-shot, openreview `combined-r17` over `13faafd2..7912da16` (verdict `acceptable_with_changes`, capability_ok and guard_confirmed true); `.review/results/combined-r17.codex.md`. It follows from cr-ecm15-1 (`b2a9e435`).

## Evidence
"By inspection, a standalone local file copied with `--ignore-existing` now takes the buffered branch, which carries bytes, mode, and time. The previous macOS `fcopyfile` path also requested ACLs and extended attributes, including resource forks. Use its descriptor-based operation with the acquired destination handle and verify it on macOS." (`crates/blit-core/src/copy/file_copy/mod.rs`, `copy_opened_into`; `crates/blit-core/src/copy/file_copy/clone.rs`, the `fcopyfile` flags.)

cr-ecm15-1 made a copy under the option write through the handle that made the destination's name (`copy_opened_into`). On Linux that is the same cascade as before, on descriptors; elsewhere it is a buffered copy. On macOS the copy before it went by `fclonefileat` (which needs the destination absent) and then `fcopyfile`, which copies data, ACLs and extended attributes — the buffered copy carries none of those.

## What it needs (not done)
- `copy_opened_into` on macOS: `fcopyfile(src_fd, made_fd, NULL, COPYFILE_ALL)` (or the flags `attempt_fcopyfile_macos_fd` uses) on the held destination, the buffered copy only when it fails; `fclonefileat` cannot be used, the name being already made.
- A guard on macOS: a file with an extended attribute (`xattr -w`) and an ACL copied with and without `--ignore-existing` arrives with both.

## What unblocks it
A Mac this session can reach. The owner's workstation is one, and is not reachable from here; the repo's macOS firewall rules (`.agents/repo-guidance.md`) apply to any daemon run there.
