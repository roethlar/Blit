# cr-g3fix2-2: Canonicalized literal prefixes are reinterpreted as glob syntax

**Severity**: MEDIUM (reviewer: MEDIUM)
**Status**: Open
**Branch**: — (default-branch mode; fixes land on master)
**Commit**: —
**Reviewer**: codex / gpt-6-astra / xhigh / standard (the owner's `review`, 2026-10-15), codex-cli 0.162.1 headless one-shot, `-a never exec -m gpt-6-astra -c model_reasoning_effort="xhigh" -s workspace-write --json`, stdin closed, detached worktree `review-g3fix2-r1-eddfab6c`; codereview re-review g3fix2-r1 over `2324e3cb..eddfab6c` (record `.review/results/g3fix2-r1.codex.json` + `.events.jsonl`); 2026-10-15 19:05–19:16Z; capability_ok=true; guard_confirmed=true (reviewer disabled `normalize_separators`'s Windows branch → `backslash_forms_follow_the_platform_rule` red at `fs_enum.rs:726`, restored by `git checkout`, blob hash verified, green; 1510/0/2 on its macOS gate; its Windows cross-clippy stopped on a missing `ml64.exe` for blake3 — the reviewer's environment, not the change); SHAs pin-verified; closed: cr-g3fix-2 true, cr-g3fix-3 true, cr-g3fix-1 false

## Evidence
crates/blit-core/src/transfers/filter.rs:309 — resolved.join(rest) inserts canonical filesystem names into the pattern without escaping their glob metacharacters. Reproduced with alias pointing to src/data[1] and exclude alias/*.txt: both builders emitted data[1]/*.txt. The CLI copied data[1]/private.txt but omitted the unrelated data1/private.txt, exiting successfully without an unmatchable warning.

## Predicted observable failure
An alias resolving into a directory whose name contains glob syntax excludes the wrong files while transferring the intended excluded files. An unmatched '[' in the resolved name can instead make a previously valid pattern fail validation.

## Reviewer's suggested approach
Keep canonicalized literal components distinct from the user-written glob suffix, escape those literal components for the matcher, and append the suffix unchanged. Test aliases into directories containing brackets and Unix backslashes.

## Intake
Admitted; reproduced by the orchestrator with the release binary on macOS: `alias2 -> s2/data[1]`, `blit copy s2/ d2/ --exclude <alias2>/*.txt` printed no warning, copied `data[1]/private.txt` (meant to be excluded) and left out `data1/private.txt` (never named). Root cause: the components that come from the disk — the resolved prefix `through_resolved_root` writes (`filter.rs:309`) and the respelled names `stored_spelling` produces — are literal by construction but are joined into the pattern as glob text, so `[1]` becomes a character class and an unmatched `[` fails glob compilation. Fix shape: keep the literal components apart from the user-written suffix until the pattern is written, escape each literal component for the matcher (`globset::escape`, globset 0.4.20, brackets `?*[]{}`; on Unix a literal `\` must be escaped as well since globset reads `\` as an escape there — check how `has_glob_meta` and the Unix rule treat it), and append the typed suffix unchanged; a literal component must never stop the stored-spelling walk the way a typed glob component does. Keep the anchored `./name` and subtree rules in `ExcludeRules::compile` matching an escaped literal exactly as they match the plain name (check whether those rules compare literally or compile globs; both must still match `data[1]`). Guards: an alias into `data[1]` with `*.txt` excludes `data[1]/private.txt` and keeps `data1/private.txt`; an alias into a directory named with an unmatched `[` neither fails nor warns; a stored-spelling component holding `[` is escaped; on Unix a resolved name holding `\` still matches; a CLI end-to-end copy; all mutation-red.

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
