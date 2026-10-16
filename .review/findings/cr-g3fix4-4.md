# cr-g3fix4-4: Windows absolute patterns still rewrite backslashes inside bracket classes

**Severity**: MEDIUM (reviewer: MEDIUM)
**Status**: Admitted
**Branch**: — (default-branch mode; fixes land on master)
**Commit**: —
**Reviewer**: codex / gpt-6-astra / xhigh / standard (the owner's `review`, 2026-10-16), codex-cli 0.162.1 headless one-shot, `-a never exec -m gpt-6-astra -c model_reasoning_effort="xhigh" -s workspace-write --json`, stdin closed, detached worktree `review-g3fix4-r1-548625f5`; codereview re-review g3fix4-r1 over `2e42f849..548625f5` (record `.review/results/g3fix4-r1.codex.json` + `.events.jsonl`); 2026-10-16 15:55–16:06Z; capability_ok=true; guard_confirmed=true (reviewer made `CompiledPatterns::compile` put accepted patterns into `refused` too → `an_escaped_literal_matches_its_name_and_not_its_encoded_spelling` red, restored by `git checkout`, blob hash verified, green; 1521/0/2 on its macOS gate; its Windows cross-clippy stopped on a missing `ml64.exe` — the reviewer's environment); SHAs pin-verified; closed: cr-g3fix3-2 true, cr-g3fix3-3 true, cr-g3fix3-1 false, cr-g3fix2-1 false, cr-g3fix-1 false

## Evidence
crates/blit-core/src/transfers/filter.rs:352 — absolute_components still replaces every backslash, bypassing the new bracket-aware normalizer. A probe of resolve_excludes_on with the Windows rule converted <root>/back[s\]lash into back[s/]lash without warning. The original glob matches backslash but not back/lash; the resulting filter excluded both backslash/saved.txt and back/lash/saved.txt. On Windows, C:\src\back[s\]lash follows this same absolute-pattern branch.

## Predicted observable failure
A valid Windows absolute exclude silently omits an unrelated subtree from copy or backup and incorrectly removes it from mirror deletion scope. build_spec also sends the corrupted pattern to peers.

## Reviewer's suggested approach
Make absolute-pattern tokenization preserve bracket-class contents as well as relative-pattern normalization. Test absolute Windows patterns containing classes with both a backslash and an ordinary member.

## Intake
Admitted on analysis (the Windows rule as a parameter; `absolute_components` at `filter.rs:350-375` reads `path.replace('\\', "/")` unconditionally under the Windows rule), the same footing as cr-g3fix-3 and cr-g3fix3-3: cr-g3fix3-3 made `normalize_separators` bracket-aware for relative patterns and `ExcludeRules::compile`, and missed the third place a typed Windows pattern is split. Fix shape (the reviewer's): `absolute_components` normalizes with the bracket-aware `normalize_separators` instead of `replace`, and the verbatim/UNC prefix handling reads the normalized text as before (a bracket class cannot be part of a prefix). Guards: `resolve_excludes_on` under the Windows rule turns `<root>/back[s\]lash` and `C:\src\back[s\]lash` into `back[s\]lash`, the compiled filter excludes `backslash/saved.txt` and leaves `back/lash/saved.txt` in scope, `build_spec` carries the same; the earlier UNC and verbatim-prefix cases stay green; mutation-red.

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
