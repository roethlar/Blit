# cr-g3fix2-1: Differently cased symlink excludes still fail to protect mirror destinations

**Severity**: HIGH (reviewer: HIGH)
**Status**: Open
**Branch**: — (default-branch mode; fixes land on master)
**Commit**: —
**Reviewer**: codex / gpt-6-astra / xhigh / standard (the owner's `review`, 2026-10-15), codex-cli 0.162.1 headless one-shot, `-a never exec -m gpt-6-astra -c model_reasoning_effort="xhigh" -s workspace-write --json`, stdin closed, detached worktree `review-g3fix2-r1-eddfab6c`; codereview re-review g3fix2-r1 over `2324e3cb..eddfab6c` (record `.review/results/g3fix2-r1.codex.json` + `.events.jsonl`); 2026-10-15 19:05–19:16Z; capability_ok=true; guard_confirmed=true (reviewer disabled `normalize_separators`'s Windows branch → `backslash_forms_follow_the_platform_rule` red at `fs_enum.rs:726`, restored by `git checkout`, blob hash verified, green; 1510/0/2 on its macOS gate; its Windows cross-clippy stopped on a missing `ml64.exe` for blake3 — the reviewer's environment, not the change); SHAs pin-verified; closed: cr-g3fix-2 true, cr-g3fix-3 true, cr-g3fix-1 false

## Evidence
crates/blit-core/src/transfers/filter.rs:444 — stored_name returns None for every symlink at line 450, so stored_spelling preserves its typed case. Reproduced on case-insensitive macOS with src/cache pointing to an outside directory: excluding absolute src/CACHE produced ./CACHE in both the local filter and wire spec, without a warning. Mirroring into a destination containing cache/saved.txt deleted that file and its directory. Repeating with absolute src/cache preserved them.

## Predicted observable failure
A mirror can delete pre-existing destination data inside a directory explicitly excluded through a differently cased source symlink. This leaves cr-g3fix-1 materially open; the observed consequence is data loss.

## Reviewer's suggested approach
Recover the symlink entry's own stored spelling without following its target, and preserve that spelling in both filter representations. Add a case-insensitive regression that verifies an existing destination subtree survives.

## Intake
Admitted; reproduced by the orchestrator with the release binary `blit 1.0.0+eddfab6c2c22` on macOS APFS (case-insensitive): `src/cache -> <outside>`, `blit mirror -y src/ dst/ --exclude <src>/CACHE` printed no warning, copied 1 file and reported "Deleted: 1 file(s), 1 dir(s)" — `dst/cache/saved.txt` and `dst/cache/` gone; the exact-case control `--exclude <src>/cache` kept them. cr-g3fix-1's Known gaps named this case as "one link entry, never the data behind it" and missed that under `mirror` the entry's counterpart at the destination is deletable: the gap is a data-loss path, not a cosmetic one. Root cause: `stored_name` (`filter.rs:444-457`) reads the stored spelling through `std::fs::canonicalize`, which cannot name a symlink without following it, so it returns `None` for every symlink and `stored_spelling` keeps the typed case. Fix shape: name the entry without following it — read the parent directory (`std::fs::read_dir`) and take the entry whose name equals the typed component exactly, else the single entry equal to it under case folding (the typed path exists per `symlink_metadata`, so on a case-insensitive filesystem exactly one entry answers); use that lookup for every component, symlink or not, with `canonicalize`'s file name only as the fallback for a non-symlink the directory listing cannot name (Windows short names). The respelled name must reach both `build` (the local rule) and `build_spec` (the wire form). Guards: the stored-name unit test gains a symlink that must report its own stored name (not its target's, not `None`); the case guard gains a symlink entry; a CLI `mirror` test on a case-insensitive filesystem (skipped elsewhere) proves an existing destination subtree under a differently cased symlink exclude survives, with the `--exclude` typed in the other case; all mutation-red.

## What
(filled by the fix)

## Approach
(filled by the fix)

## Files changed
(filled by the fix)

## Guard proof
(filled by the fix)

## Known gaps
(filled by the fix)

## Reviewer comments
(filled by the re-review)
