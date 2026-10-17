# cr-g3fix5-4: Bracket-aware pattern parsing is incorrectly applied to literal Windows roots

**Severity**: MEDIUM (reviewer: MEDIUM)
**Status**: Admitted
**Branch**: — (default-branch mode; fixes land on master)
**Commit**: —
**Reviewer**: codex / gpt-6-astra / xhigh / standard (the owner's `review`, 2026-10-17), codex-cli 0.162.1 headless one-shot, `-a never exec -m gpt-6-astra -c model_reasoning_effort="xhigh" -s workspace-write --json`, stdin closed, detached worktree `review-g3fix5-r1-de9a333f`; codereview re-review g3fix5-r1 over `89575776..de9a333f` (record `.review/results/g3fix5-r1.codex.json` + `.events.jsonl`); 2026-10-17 16:18–16:29Z; capability_ok=true; guard_confirmed=true (reviewer made `mirror_delete_pass` plan with the unscoped filter → `mirror_delete_pass_keeps_an_excluded_entry_stored_in_another_spelling` red, exit 101, `Cache/saved.txt` deleted; restored by `git checkout`, blob hashes verified, green; 1533/0/2 on its macOS gate, Linux cross-clippy clean); SHAs pin-verified; closed: cr-g3fix4-1 true, cr-g3fix4-3 true, cr-g3fix4-4 true, cr-g3fix3-1 true, cr-g3fix2-1 true, cr-g3fix4-2 false, cr-g3fix-1 false

## Evidence
crates/blit-core/src/transfers/filter.rs:390 and :415 — absolute_components applies the bracket-aware normalizer to the source root as well as the pattern. A parameterized Windows-rule probe on macOS used root 'C:\data[1\test]\src' and pattern 'C:/data[1/test]/src/cache'; relative_to_root returned None instead of ['cache']. The backslash between the root's two literal directory names was treated as a bracket-class member. through_resolved_root also stops at the bracket-bearing ancestor at line 335.

## Predicted observable failure
On Windows, an absolute exclude using forward slashes fails to relativize against an equivalent backslash-spelled source root whose directory components contain brackets spanning a separator. The command reports the exclude as unmatchable and proceeds without excluding the requested subtree.

## Reviewer's suggested approach
Tokenize filesystem roots and resolved prefixes as literal paths, normalizing every native separator. Reserve bracket-aware tokenization for user pattern text, and test mixed separators with bracket-bearing root components.

## Intake
Admitted on analysis (the reviewer's parameterized Windows-rule probe on macOS: root `C:\data[1\test]\src`, pattern `C:/data[1/test]/src/cache`, `relative_to_root` → `None`): cr-g3fix4-4 applied the bracket-aware `normalize_separators` to every input of `absolute_components`, including the source root and the resolved root, which are literal paths and hold no classes. cr-g3fix4-4's Known gaps had recorded exactly this root shape.

## What
Under the Windows rule, `absolute_components` reads a literal path (the source root, the resolved root, a verbatim or UNC prefix) with every `\` a separator, and only the typed pattern bracket-aware, so `C:/data[1/test]/src/cache` relativizes against the root `C:\data[1\test]\src` to `./cache`.

Fix shape, admitted: `absolute_components` splits literal paths — the source root, the resolved root, `\\?\` and UNC prefixes — with the plain separator replace, and only the typed pattern bracket-aware (`normalize_separators`); `through_resolved_root` the same. Guard: root `C:\data[1\test]\src`, pattern `C:/data[1/test]/src/cache` → `./cache`, and the cr-g3fix4-4 class case still `./back[s\]lash`.

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
