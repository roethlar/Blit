# cr-ecm19-1: a local resume's cleanup was armed before its partial was had

**Severity**: HIGH — a resume that cannot take its destination cuts the user's file to one byte longer than the source
**Status**: Reviewed, found incomplete — `combined-r21` (Codex openreview, 2026-11-22) reviewed `c15ebf5a..b92c9a17` — the seven `combined-r20` fixes — with `guard_confirmed: true`. "A successful local resume retains a handle for cleanup but leaves its delivery identity unset. A concurrent rename can therefore cause the replacement to receive the timestamp and the copy to report success. A failed `try_clone()` silently restores pathname-based cleanup. On Windows, streams and shards also accept an unavailable identity as success. Use the writer's handle through final metadata and verification, and retain a move source whenever delivery is uncertain." The remaining work is cr-ecm20-4
**Branch**: — (default-branch mode; fixes land on master)
**Commit**: `c8130e4a`
**Reviewer**: codex / gpt-6.2-astra (as dispatched) / max / frontier, codex-cli 0.171.0 headless one-shot, openreview `combined-r20` over `17f4eb97..c15ebf5a` (verdict `acceptable_with_changes`, capability_ok and guard_confirmed true); `.review/results/combined-r20.codex.md`. It completes cr-ecm18-2 (`7413b6c6`).

## Evidence
"I ran `copy --resume` with a three-byte source and a writable `0644` destination in a `0555` directory. Replacement failed with `Permission denied` and exit 2, yet cleanup truncated the existing 29-byte file to `prev`. Mark an acquired file through its handle. An acquisition failure must preserve the existing contents."

The local route armed `KeepUnfinished` and then called `resume_copy_from`, which acquired the partial inside; a failure to acquire it was settled by opening the name and marking whatever stood there.

## Intake
Admitted as the reviewer put it.

## What
- `resume.rs`: the acquisition (`open_resume_partial`) and the patch of a partial already had (`resume_copy_into`) are apart; `resume_copy_from` is the two together, as before.
- The local route has the partial first and arms its cleanup only then, with a handle to it (`PartialTarget::arm_held`); a failure from there marks THAT file through the handle. A failure to have it leaves what stands at the name exactly as it is.

## Guard proof
- `sink::…::a_local_resume_settles_only_the_partial_it_has`: a 28-byte destination this run may write but not read (mode 200) in a folder closed to removals (555) — the resume fails and the file is byte for byte as it was; then a partial that is had, another file moved over its name (the claim seam), and a failing metadata tail — the other file is untouched. Skipped, and says so, for a process a directory's mode does not bar.
- Mutations, each restored byte for byte: the cleanup armed before the partial is had (red); a failure marking whatever stands at the name (red).
- The sink and resume tests (115) pass.

## Files changed
- `crates/blit-core/src/copy/file_copy/resume.rs`, `crates/blit-core/src/copy/file_copy/mod.rs`, `crates/blit-core/src/copy/mod.rs` — the acquisition apart from the patch.
- `crates/blit-core/src/remote/transfer/sink.rs` — the local route's resume arm, `PartialTarget`; guard.

## Known gaps
- The guard proves nothing run as root; it says so and passes.
- The working mode of a partial (the rest of `combined-r20`'s resume finding) is cr-ecm19-4's.
