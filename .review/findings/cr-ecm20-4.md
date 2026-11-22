# cr-ecm20-4: the local and Windows delivery checks

**Severity**: HIGH — a local resume whose destination is replaced once its blocks are written gives the other file the copy's time and reports the copy done, and a move takes its source as delivered
**Status**: Reviewed, found incomplete — `combined-r22` (Codex openreview, 2026-11-22) reviewed `b92c9a17..5d2f8def` — the eight `combined-r21` fixes — with `guard_confirmed: true`. "The wire hold returns only device and inode. Failed-block cleanup checks the pathname and then reopens it to mark the partial, so a rename between those operations can extend somebody else's file. The local success path also turns a failed identity lookup into `None`, which permits acknowledgment. Retain the active descriptor for patching and cleanup, and require a successful destination identity check before a move releases its source. Windows also needs a handle-based file identity and directory walk. Its current paths can accept a replacement or descend through a changed pathname." The remaining work is cr-ecm21-4
**Branch**: — (default-branch mode; fixes land on master)
**Commit**: `ae10d707`
**Reviewer**: codex / gpt-6.2-astra (as dispatched) / max / frontier, codex-cli 0.171.0 headless one-shot, openreview `combined-r21` over `c15ebf5a..b92c9a17` (verdict `acceptable_with_changes`, capability_ok and guard_confirmed true); `.review/results/combined-r21.codex.md`. It completes cr-ecm19-1 (`c8130e4a`) and carries cr-ecm19-2's (`3f7d60b4`) check to the local resume.

## Evidence
"A successful local resume retains a handle for cleanup but leaves its delivery identity unset. A concurrent rename can therefore cause the replacement to receive the timestamp and the copy to report success. A failed `try_clone()` silently restores pathname-based cleanup. On Windows, streams and shards also accept an unavailable identity as success. Use the writer's handle through final metadata and verification, and retain a move source whenever delivery is uncertain."

## Intake
Admitted as the reviewer put it. The Windows half — a file identity there (the volume and file index of a handle), or a move that keeps its source when delivery cannot be told — cannot be written and proven on this Linux host; it stays open with cr-uv6-3 (a move's account on Windows), which needs the same.

## What
- The local route keeps the handle a resume wrote through (`resumed`) for the tail: the time is given through it, and the resume is delivered only while that file is what the name holds, or a link there leads to it (`still_its_own`, cr-ecm19-2's check) — otherwise `REPLACED_WHILE_WRITTEN`, the other file left as it is, and a move's account loses the path.
- `PartialTarget::arm_held`: a handle that cannot be held for the cleanup is an error before a byte is written — it was let go quietly, and a failure settled by name after all.

## Guard proof
- `sink::…::a_local_resume_lands_only_where_its_own_file_stands`: a local resume into a 600 partial; once its blocks are written (the claim seam, now passed again there) another file is renamed over the name — the copy fails with `REPLACED_WHILE_WRITTEN`, and the other file keeps its bytes and its own time (1,500,000,000), not the copy's.
- Mutations, each restored byte for byte: the time given by the name (red); the resume delivered whoever holds the name (red).
- cr-ecm19-1's guard passes (its seam now acts once). The sink and resume tests (123) pass. Windows cross-clippy clean.

## Files changed
- `crates/blit-core/src/remote/transfer/sink.rs` — `PartialTarget::arm_held`, the local route's resume arm and tail; guard.

## Known gaps
- Windows (BLOCKED): no file identity there; streams, shard members and the local resume deliver on the name alone, as before.
- The handle that cannot be duplicated has no guard: nothing a test can do makes `dup` fail for one file.
- A plain local copy (not `--resume`) opens its destination inside the copy engine; which file it wrote is still not asked (cr-ecm19-2's gap).
