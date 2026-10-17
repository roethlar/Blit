# cr-g3fix5-3: Rewriting an escaped separator changes recursive-glob semantics

**Severity**: MEDIUM (reviewer: MEDIUM)
**Status**: Admitted
**Branch**: — (default-branch mode; fixes land on master)
**Commit**: —
**Reviewer**: codex / gpt-6-astra / xhigh / standard (the owner's `review`, 2026-10-17), codex-cli 0.162.1 headless one-shot, `-a never exec -m gpt-6-astra -c model_reasoning_effort="xhigh" -s workspace-write --json`, stdin closed, detached worktree `review-g3fix5-r1-de9a333f`; codereview re-review g3fix5-r1 over `89575776..de9a333f` (record `.review/results/g3fix5-r1.codex.json` + `.events.jsonl`); 2026-10-17 16:18–16:29Z; capability_ok=true; guard_confirmed=true (reviewer made `mirror_delete_pass` plan with the unscoped filter → `mirror_delete_pass_keeps_an_excluded_entry_stored_in_another_spelling` red, exit 101, `Cache/saved.txt` deleted; restored by `git checkout`, blob hashes verified, green; 1533/0/2 on its macOS gate, Linux cross-clippy clean); SHAs pin-verified; closed: cr-g3fix4-1 true, cr-g3fix4-3 true, cr-g3fix4-4 true, cr-g3fix3-1 true, cr-g3fix2-1 true, cr-g3fix4-2 false, cr-g3fix-1 false

## Evidence
crates/blit-core/src/fs_enum.rs:300 and :326 — escaped '/' is emitted as a bare '/', turning '**\/cache' into '**/cache'. Globset parses the latter as a recursive prefix that also matches top-level cache; the original requires a preceding separator. A direct FileFilter comparison confirmed that cache/needed.txt changes from allowed to excluded. The built CLI's copy --exclude '**\/cache' omitted that file and exited 0.

## Predicted observable failure
Previously valid Unix exclusions acquire broader matching behavior, silently omitting additional files from copies or backups and incorrectly protecting additional paths from mirror deletion.

## Reviewer's suggested approach
Translate escapes while preserving parsed glob semantics and rule anchoring, including the context-sensitive interpretation of '**'. Add equivalence tests for escaped separators adjacent to recursive wildcards.

## Intake
Reproduced 2026-10-17 with `blit 1.0.0+cf3405f65366` (the release build of the pushed candidate; script `scratchpad/g3fix5/repro.sh`, output `repro.out`): `copy --exclude '**\/cache'` over `src/cache/needed.txt` and `src/a/cache/x` left out `cache/needed.txt`, which the typed pattern (a preceding separator required) allows; `a/cache/x` left out as expected. Admitted as MEDIUM: a valid Unix exclusion silently broadened by cr-g3fix4-3's rewrite.

## What
A typed Unix escape of `/` keeps its meaning after cr-g3fix4-3's rewrite: `**\/cache` still requires a preceding separator (it leaves out `a/cache`, not a top-level `cache`), locally and on the wire.

Fix shape, admitted: `typed_exclude_dialect` writes an escaped `/` in a form with the same meaning as globset reads `\/`: a literal separator that does not take part in `**`'s recursive-prefix reading. Candidates to prove by globset equivalence tests (typed vs written, over names with and without a leading component): `**\/x` → `*/**/x` (at least one component before `x`; `*` matches across `/` in this filter, so prove it), or leaving `\/` unrewritten where no equal spelling exists under the Unix rule and recording the Windows-peer reading as a gap. The tests cover `**\/x`, `a\/b`, `\/x`, and an escaped `/` inside and outside alternations.

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
