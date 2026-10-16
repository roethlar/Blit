# cr-g3fix4-2: Destination spelling differences still expose excluded subtrees to mirror deletion

**Severity**: HIGH (reviewer: HIGH)
**Status**: Admitted
**Branch**: — (default-branch mode; fixes land on master)
**Commit**: —
**Reviewer**: codex / gpt-6-astra / xhigh / standard (the owner's `review`, 2026-10-16), codex-cli 0.162.1 headless one-shot, `-a never exec -m gpt-6-astra -c model_reasoning_effort="xhigh" -s workspace-write --json`, stdin closed, detached worktree `review-g3fix4-r1-548625f5`; codereview re-review g3fix4-r1 over `2e42f849..548625f5` (record `.review/results/g3fix4-r1.codex.json` + `.events.jsonl`); 2026-10-16 15:55–16:06Z; capability_ok=true; guard_confirmed=true (reviewer made `CompiledPatterns::compile` put accepted patterns into `refused` too → `an_escaped_literal_matches_its_name_and_not_its_encoded_spelling` red, restored by `git checkout`, blob hash verified, green; 1521/0/2 on its macOS gate; its Windows cross-clippy stopped on a missing `ml64.exe` — the reviewer's environment); SHAs pin-verified; closed: cr-g3fix3-2 true, cr-g3fix3-3 true, cr-g3fix3-1 false, cr-g3fix2-1 false, cr-g3fix-1 false

## Evidence
crates/blit-core/src/fs_enum.rs:127 — subtree matching compares the destination's spelling against the source-derived case-sensitive pattern. transfer_session/mod.rs:4177 reuses that filter for destination enumeration. Reproduced on case-insensitive macOS with src/cache pointing outside, absolute exclude src/CACHE, and existing dst/Cache/saved.txt: mirror deleted the saved file and directory. The identical run with destination spelling dst/cache preserved them.

## Predicted observable failure
Even after source-name resolution succeeds, mirror deletes an excluded destination counterpart whose stored spelling differs from the source's. This leaves the earlier filesystem-case contract materially open.

## Reviewer's suggested approach
Resolve literal exclusions against the destination filesystem when constructing deletion scope, preserving glob semantics and protecting equivalent destination spellings. Add a survival test where source, typed exclude, and destination use different spellings.

## Intake
Admitted; reproduced by the orchestrator with the release binary `blit 1.0.0+548625f5c7cf` on macOS APFS (`scratchpad/g3fix4/repro.out`), and wider than the reviewer's case: a plain directory `src/cache` excluded as `<src>/cache`, typed exactly as stored, with a pre-existing `dst/Cache/saved.txt` — `blit mirror -y` reported "Deleted: 1 file(s), 1 dir(s)". The symlink form reproduced the same. Root cause: the mirror deletion scope (`transfer_session/mod.rs` ~4171-4190, `mirror_filter` — the user's `FileFilter` on the local carrier, `filter_from_spec` on a daemon) applies the source-spelled patterns to the destination walk as typed, case-sensitively; the destination's own stored spelling is never consulted. Recorded as an observation since cr-g3fix2-1; it is a data-loss path under `mirror`, so it cannot stay one. Fix shape (the reviewer's): where the deletion-scope filter is built — that one place, which runs wherever the destination is local, the local carrier and a destination daemon alike, since the destination walk is the session's own — respell each exclude pattern's literal path components against the destination root with the same `stored_name` machinery the source side uses. A pattern with a `/` (anchored `./a/b` or relative `a/b`) is walked component by component from the destination root: a component is plain when it holds no glob syntax other than the single-character classes `globset::escape`/`literal_glob` emit (`[*]` `[?]` `[[]` `[]]` `[{]` `[}]` `[\]`); it is unescaped to its name, probed with `stored_name(dst_root/so-far/name)`, and re-escaped; the walk stops at the first component with real glob syntax or that does not exist on the destination. The respelled pattern is ADDED beside the original: the original still protects a destination spelled like the source, the respelled one protects the destination's own spelling, and adding patterns can only shrink the deletion scope. A tie (cr-g3fix4-1) adds every name. The any-depth name rule (a pattern with no `/`, `--exclude cache`) cannot be respelled per directory; for it the deletion-scope filter's name globs are compiled `case_insensitive(true)` when the destination filesystem is case-insensitive, detected once at the root without writing anything: `symlink_metadata` of the root path with the case of its last alphabetic component swapped resolves to the root's own identity (no letter to swap → assume case-sensitive, and say so in the doc). Normalization-insensitivity for the any-depth rule (an NFC name against an NFD destination) is out of reach without a per-entry probe and is recorded as the remaining gap (APFS preserves the spelling blit wrote, so it needs a destination created by another tool in the other form). FAST: both run once when the deletion-scope filter is built, never per entry; a case-insensitive globset matches at the same cost. Guards: CLI `mirror` survival with `src/cache` (a directory) excluded as `<src>/cache` against a pre-existing `dst/Cache/saved.txt`, and the symlink form, and a nested `a/b` form, each skipped where the destination filesystem is case-sensitive; the any-depth form `--exclude cache` with `dst/Cache` (case-insensitive fs); unit tests of the respelling helper (pure, a stand-in `stored` closure; the escape-aware component parser; a glob component stops the walk; a missing destination component stops it); the daemon route through the existing loopback-daemon mirror test in `exclude_contract` if the harness allows a destination-spelling case; mutation-red.

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
