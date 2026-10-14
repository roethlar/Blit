# cr-g3fix-3: Windows relative anchors and trailing backslashes are not normalized

**Severity**: MEDIUM (reviewer: MEDIUM)
**Status**: Open
**Branch**: — (default-branch mode; fixes land on master)
**Commit**: —
**Reviewer**: codex / gpt-6-astra / xhigh / standard (the owner's `review`, 2026-10-14), codex-cli 0.162.1 headless one-shot, `-a never exec -m gpt-6-astra -c model_reasoning_effort="xhigh" -s workspace-write --json`, stdin closed, detached worktree `review-g3fix-r1-b7da0d81`; codereview defect hunt g3fix-r1 over `b186e8b8..b7da0d81` (record `.review/results/g3fix-r1.codex.json` + `.events.jsonl`); 2026-10-14 16:30–17:08Z; capability_ok=true; guard_confirmed=true (reviewer flipped `anchored` in `ExcludeRules::compile` → `a_dot_slash_name_is_anchored_to_the_root` red, restored, green; 1502/0/2 on its macOS gate); SHAs pin-verified

## Evidence
crates/blit-core/src/fs_enum.rs:82 — compilation trims only '/', recognizes only './', and classifies separators using only '/'. resolve_excludes also passes relative patterns through unchanged. Consequently, Windows patterns such as '.\cache' and 'cache\' never acquire the corresponding anchored or subtree rules; the legacy whole-path checks do not match ordinary cache directory entries either. This finding is based on Windows-path code analysis, not a Windows runtime test.

## Predicted observable failure
On Windows, these ordinary directory-path exclusions silently copy cache and its contents instead of pruning them.

## Reviewer's suggested approach
Normalize Windows separators before anchor detection, trailing-separator removal, classification, and wire-spec construction. Preserve literal backslashes on Unix and add Windows coverage for both forms.

## Intake
Admitted. Rule compilation trims only `/`, recognizes only `./` as the anchor and classifies "has a separator" by `/` alone, and `resolve_excludes` passes relative patterns through unchanged, so on Windows `.\cache`, `cache\` and `a\.java` get neither the anchored nor the subtree rule, nor the trailing-separator trim, and the wire spec carries the backslash form; the legacy whole-path checks match nothing useful either. Analysis-only (no Windows runtime in the review), consistent with the agent's own "not done" note. Fix shape: on Windows, normalize `\` to `/` in relative patterns before anchor detection, trailing-separator removal, classification and `build_spec`; a backslash on Unix stays a literal character. Put the platform rule behind a parameter so the normalizer is unit-tested on every OS, and gate the end-to-end Windows forms on `cfg(windows)` for CI's Windows leg.

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
