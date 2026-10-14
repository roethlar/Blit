# cr-g3fix-1: Absolute excludes lose filesystem case semantics after rewriting

**Severity**: MEDIUM (reviewer: MEDIUM)
**Status**: Open
**Branch**: — (default-branch mode; fixes land on master)
**Commit**: —
**Reviewer**: codex / gpt-6-astra / xhigh / standard (the owner's `review`, 2026-10-14), codex-cli 0.162.1 headless one-shot, `-a never exec -m gpt-6-astra -c model_reasoning_effort="xhigh" -s workspace-write --json`, stdin closed, detached worktree `review-g3fix-r1-b7da0d81`; codereview defect hunt g3fix-r1 over `b186e8b8..b7da0d81` (record `.review/results/g3fix-r1.codex.json` + `.events.jsonl`); 2026-10-14 16:30–17:08Z; capability_ok=true; guard_confirmed=true (reviewer flipped `anchored` in `ExcludeRules::compile` → `a_dot_slash_name_is_anchored_to_the_root` red, restored, green; 1502/0/2 on its macOS gate); SHAs pin-verified

## Evidence
crates/blit-core/src/transfers/filter.rs:235 — the first lexical match wins before the canonical spelling is considered. relative_to_root preserves the suffix's typed case at line 315, while fs_enum.rs:213 builds case-sensitive globs. Reproduced on case-insensitive macOS: src/CACHE and src/cache identify the same directory, but excluding the absolute src/CACHE still copied cache/private.txt without warning. Windows has the same suffix-matching problem despite its case-insensitive root comparison.

## Predicted observable failure
A differently cased absolute exclude silently transfers the intended excluded subtree and leaves it within a mirror's deletion scope.

## Reviewer's suggested approach
Preserve filesystem case semantics in the rewritten matcher, including the wire representation, or resolve literal suffixes to their filesystem spelling. Test case differences below the source root.

## Intake
Admitted. On a case-insensitive filesystem (macOS by default, Windows) `--exclude /src/CACHE` for an on-disk `src/cache` is rewritten to the anchored literal `./CACHE`, and the anchored rule compares case-sensitively, so nothing matches and no warning fires: the subtree the user named copies, and under mirror its counterpart is deletable. The reviewer reproduced it on macOS. Fix shape: when the absolute pattern's literal part names an existing path, take the filesystem's own spelling of each component below the root (canonicalize the literal path, then relativize), so the rewritten rule — and the wire form a push sends — name the entry as the disk does; a path that does not exist keeps its typed case. A guard with a differently cased absolute exclude on a case-insensitive filesystem (skipped where the filesystem is case-sensitive).

## What
(filled when fixed)

## Approach
(filled when fixed)

## Files changed
(filled when fixed)

## Guard proof
(filled when fixed)

## Known gaps
(filled when fixed)

## Reviewer comments
(filled by the re-review)
