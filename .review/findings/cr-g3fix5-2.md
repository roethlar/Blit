# cr-g3fix5-2: A glob component prevents protection of differently spelled destination descendants

**Severity**: HIGH (reviewer: HIGH)
**Status**: Admitted
**Branch**: — (default-branch mode; fixes land on master)
**Commit**: —
**Reviewer**: codex / gpt-6-astra / xhigh / standard (the owner's `review`, 2026-10-17), codex-cli 0.162.1 headless one-shot, `-a never exec -m gpt-6-astra -c model_reasoning_effort="xhigh" -s workspace-write --json`, stdin closed, detached worktree `review-g3fix5-r1-de9a333f`; codereview re-review g3fix5-r1 over `89575776..de9a333f` (record `.review/results/g3fix5-r1.codex.json` + `.events.jsonl`); 2026-10-17 16:18–16:29Z; capability_ok=true; guard_confirmed=true (reviewer made `mirror_delete_pass` plan with the unscoped filter → `mirror_delete_pass_keeps_an_excluded_entry_stored_in_another_spelling` red, exit 101, `Cache/saved.txt` deleted; restored by `git checkout`, blob hashes verified, green; 1533/0/2 on its macOS gate, Linux cross-clippy clean); SHAs pin-verified; closed: cr-g3fix4-1 true, cr-g3fix4-3 true, cr-g3fix4-4 true, cr-g3fix3-1 true, cr-g3fix2-1 true, cr-g3fix4-2 false, cr-g3fix-1 false

## Evidence
crates/blit-core/src/transfers/filter.rs:810 — respelled_below stops at the first glob component; crates/blit-core/src/fs_enum.rs:142 keeps path rules case-sensitive. Reproduced on macOS with src/x/cache and dst/x/Cache/saved.txt: both --exclude '**/cache' and the absolute --exclude '<src>/*/cache' omitted the source subtree but deleted saved.txt, exiting 0. The literal './x/cache' and bare 'cache' controls preserved it.

## Predicted observable failure
Valid relative and absolute glob exclusions still allow mirror to delete their excluded destination counterparts when stored spelling differs. This leaves cr-g3fix4-2 and the broader cr-g3fix-1 filesystem-case contract incomplete.

## Reviewer's suggested approach
Preserve destination filesystem equivalence throughout path-rule matching, including components after globs, without expanding or rewriting wildcard syntax. Add survival coverage for '**/cache' and absolute wildcard paths.

## Intake
Reproduced 2026-10-17 with `blit 1.0.0+cf3405f65366` (the release build of the pushed candidate; script `scratchpad/g3fix5/repro.sh`, output `repro.out`): `mirror -y --exclude '**/cache'` over `src/x/cache/` with a pre-existing `dst/x/Cache/saved.txt` printed `Deleted: 1 file(s), 2 dir(s)` and the file was gone; the absolute `--exclude <src>/*/cache` did the same. Admitted as HIGH on the same rule; cr-g3fix4-2's Known gaps had recorded the glob-component case as remaining.

## What
Under `mirror`, an exclude with a glob component (`**/cache`, `*/cache`, an absolute `<src>/*/cache`) protects the destination's own spelling of the literal components after the glob (`dst/x/Cache` for `**/cache`), on every route the deletion pass serves.

Fix shape, admitted: with cr-g3fix5-1 (see its shape) — literal components after a glob component are resolved at the destination per visited directory through `stored_name`, glob components are fold-matched when the destination ignores case, and `respelled_below` no longer stops at the first glob component. Survival guards for `**/cache`, `*/cache` and an absolute `<src>/*/cache` against `dst/x/Cache/saved.txt`, local carrier and destination daemon.

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
