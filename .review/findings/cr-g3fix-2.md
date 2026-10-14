# cr-g3fix-2: Canonicalizing the whole pattern fails for glob paths through symlinks

**Severity**: MEDIUM (reviewer: MEDIUM)
**Status**: Open
**Branch**: — (default-branch mode; fixes land on master)
**Commit**: —
**Reviewer**: codex / gpt-6-astra / xhigh / standard (the owner's `review`, 2026-10-14), codex-cli 0.162.1 headless one-shot, `-a never exec -m gpt-6-astra -c model_reasoning_effort="xhigh" -s workspace-write --json`, stdin closed, detached worktree `review-g3fix-r1-b7da0d81`; codereview defect hunt g3fix-r1 over `b186e8b8..b7da0d81` (record `.review/results/g3fix-r1.codex.json` + `.events.jsonl`); 2026-10-14 16:30–17:08Z; capability_ok=true; guard_confirmed=true (reviewer flipped `anchored` in `ExcludeRules::compile` → `a_dot_slash_name_is_anchored_to_the_root` red, restored, green; 1502/0/2 on its macOS gate); SHAs pin-verified

## Evidence
crates/blit-core/src/transfers/filter.rs:232 — canonicalize(pattern) requires the wildcard-containing path itself to exist. Reproduced with alias pointing to src: copying src/ with an absolute exclude alias/*.txt warned that it could not match and copied keep.txt. The corresponding literal exclude alias/keep.txt worked.

## Predicted observable failure
An absolute glob through a symlink alias, including an alternate /tmp spelling on macOS, fails to exclude matching files even though its directory resolves under the source.

## Reviewer's suggested approach
Canonicalize the existing literal prefix, preserve the glob suffix, and then relativize against the source-root forms.

## Intake
Admitted. `relative_to_root` canonicalizes the whole pattern, so an absolute glob through a symlink (`/alias/*.txt`, `/tmp/...` vs `/private/tmp/...` on macOS) fails to canonicalize — the glob path does not exist — and is reported as unmatchable, while the literal `/alias/keep.txt` works. Reproduced by the reviewer. Fix shape: split the pattern at the first component holding a glob metacharacter, canonicalize the existing literal prefix, re-append the glob suffix unchanged, then relativize against the source-root forms (as typed, then resolved). This also supplies cr-g3fix-1's on-disk spelling for the literal prefix. Guard: an absolute glob through a symlinked root excludes its matches.

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
