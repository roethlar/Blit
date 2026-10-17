# cr-g3fix5-1: Unicode-equivalent destination names remain inside mirror deletion scope

**Severity**: HIGH (reviewer: HIGH)
**Status**: Admitted
**Branch**: — (default-branch mode; fixes land on master)
**Commit**: —
**Reviewer**: codex / gpt-6-astra / xhigh / standard (the owner's `review`, 2026-10-17), codex-cli 0.162.1 headless one-shot, `-a never exec -m gpt-6-astra -c model_reasoning_effort="xhigh" -s workspace-write --json`, stdin closed, detached worktree `review-g3fix5-r1-de9a333f`; codereview re-review g3fix5-r1 over `89575776..de9a333f` (record `.review/results/g3fix5-r1.codex.json` + `.events.jsonl`); 2026-10-17 16:18–16:29Z; capability_ok=true; guard_confirmed=true (reviewer made `mirror_delete_pass` plan with the unscoped filter → `mirror_delete_pass_keeps_an_excluded_entry_stored_in_another_spelling` red, exit 101, `Cache/saved.txt` deleted; restored by `git checkout`, blob hashes verified, green; 1533/0/2 on its macOS gate, Linux cross-clippy clean); SHAs pin-verified; closed: cr-g3fix4-1 true, cr-g3fix4-3 true, cr-g3fix4-4 true, cr-g3fix3-1 true, cr-g3fix2-1 true, cr-g3fix4-2 false, cr-g3fix-1 false

## Evidence
crates/blit-core/src/fs_enum.rs:205 — GlobBuilder::case_insensitive(true) uses globset's byte-regex mode, which folds ASCII, not Unicode. On case-insensitive macOS, mirror --exclude 'σ' with src/x/σ and dst/x/ς/saved.txt deleted saved.txt and exited 0. The filesystem identified both destination spellings as the same inode. The same failure reproduced for é versus É and NFC café versus NFD café. The ASCII cache/Cache control preserved saved.txt.

## Predicted observable failure
A mirror deletes pre-existing data inside an excluded destination subtree when its stored name differs by Unicode case or normalization. The shared destination deletion pass exposes local and daemon destinations to this failure.

## Reviewer's suggested approach
Match destination name rules using the destination's supported case and normalization equivalence. Do not rely on globset's case-insensitive flag for Unicode folding. Add Unicode destination-survival tests.

## Intake
Reproduced 2026-10-17 with `blit 1.0.0+cf3405f65366` (the release build of the pushed candidate; script `scratchpad/g3fix5/repro.sh`, output `repro.out`): `mirror -y --exclude σ` over `src/x/σ/` with a pre-existing `dst/x/ς/saved.txt` printed `Deleted: 1 file(s), 2 dir(s)` and `dst/x/ς/saved.txt` was gone; the same with an NFC `café` exclude and an NFD `café` destination directory. Both runs also removed `dst/x` itself (2 dirs), which the fix must account for. Admitted as HIGH on the standing rule that a data-loss path under `mirror` cannot stay a known gap; cr-g3fix4-2's Known gaps had recorded the NFC/NFD case as remaining.

## What
Under `mirror`, an excluded entry the destination stores in a Unicode-equivalent spelling (final sigma for σ, NFD for NFC, another Unicode case) is out of the deletion scope on a destination whose filesystem pairs those spellings, exactly as an ASCII case variant already is (cr-g3fix4-2).

Fix shape, admitted (one mechanism with cr-g3fix5-2; `transfers::filter::destination_scope` and the deletion walk in `transfer_session::mirror_delete_pass`, which every destination — local carrier, pull client, destination daemon — plans through):
- A literal component — of a path rule, before or after a glob component, or a literal any-depth name rule — is resolved at the destination by the destination filesystem itself, per visited directory: `stored_name(dir/literal)` (cr-g3fix4-1's identity pairing: the entry the filesystem pairs with that spelling, the link itself never its target, every name of a tie) gives the destination's own spelling(s), which are protected. One lookup per literal per visited directory, never per entry (FAST); a lookup that finds nothing costs one failed `lstat`.
- A glob component is matched against a directory's entries under a fold that approximates the destination's equivalence when the destination ignores case (`case_insensitive_at`): NFC normalization plus per-character uppercase (ς and σ both upcase to Σ; NFD café normalizes to NFC) applied to both the pattern text and the entry name. Over-protection (an entry kept that NTFS would distinguish) is the safe direction. `unicode-normalization` may be added as a dependency for NFC; if the agent finds a reason not to, the residual (a glob holding Unicode letters in another case) is recorded as a known gap and the literal mechanism above still closes the reviewer's cases.
- A protected entry protects its ancestors: a directory holding a protected entry is never removed, whether or not the source still has it. The reproductions removed `dst/x` whole (`2 dir(s)`); establish why (the source's `x` held only the excluded entry) and make the walk stop at protected subtrees.
- The source-side copy filter is untouched; no new user-visible option (SIMPLE); the wire form is unchanged.

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
