# cr-g3fix6-2: Unicode glob matching still misses names the destination filesystem pairs

**Severity**: HIGH (reviewer: HIGH)
**Status**: Admitted
**Branch**: — (default-branch mode; fixes land on master)
**Commit**: —
**Reviewer**: codex / gpt-6-astra / xhigh / standard (the owner's `review`, 2026-10-18), codex-cli 0.162.1 headless one-shot, `-a never exec -m gpt-6-astra -c model_reasoning_effort="xhigh" -s workspace-write --json`, stdin closed, detached worktree `review-g3fix6-r1-19ace1ac`; codereview re-review g3fix6-r1 over `a157ae00..19ace1ac` (record `.review/results/g3fix6-r1.codex.json` + `.events.jsonl`); 2026-10-18 00:12–00:22Z; capability_ok=true; guard_confirmed=true (reviewer disabled `escaped_separator`'s star-run collapse → `an_escaped_separator_keeps_its_meaning_in_the_class_dialect` red, exit 101; restored by `git checkout`, blob hash verified, green; 1548/0/2 on its macOS gate, Linux cross-clippy clean, Windows cross-clippy stopped on a missing `ml64.exe` — its environment; it cites CI run 39080085302 green on all three OSes); SHAs pin-verified; closed: cr-g3fix5-3 true, cr-g3fix5-4 true, cr-g3fix5-1 false, cr-g3fix5-2 false, cr-g3fix4-2 false, cr-g3fix-1 false

## Evidence
crates/blit-core/src/transfers/filter.rs:1299 — fold retains characters with multi-character uppercase mappings and changes candidate byte lengths through NFC and uppercase conversion. The folded matcher assembled at line 1015 therefore does not preserve globset's byte-width wildcard semantics or all filesystem equivalences. On macOS APFS, mirror --exclude '*ß' excluded src/aß but deleted dst/aSS/saved.txt. Separately, --exclude '???' excluded an NFD src/é directory but deleted dst/É/saved.txt; --exclude '??' excluded src/ſ but deleted dst/s/saved.txt. Before each run, looking up the destination through the source spelling returned the same inode. Every run exited 0.

## Predicted observable failure
Mirror deletes excluded Unicode-equivalent destination subtrees despite the filesystem identifying the spellings as the same entry. Both any-depth glob rules and path glob components remain affected, leaving cr-g3fix5-1 incomplete.

## Reviewer's suggested approach
Preserve the original glob's matching semantics while applying destination equivalence, including normalization, byte-width wildcards, and multi-character mappings. Add deletion-survival coverage for these filesystem-verified pairs.

## Intake
Reproduced 2026-10-18 with the release build of the candidate's code (`blit 1.0.0+df61b1d6464c`, identical product code to `19ace1ac`; script `scratchpad/g3fix6/repro.sh`, output `repro.out`): `--exclude '*ß'` over `src/aß/` deleted `dst/aSS/saved.txt`; `--exclude '??'` over `src/ſ/` deleted `dst/s/saved.txt`; `--exclude '???'` over an NFD `src/é/` deleted the NFC `dst/É/saved.txt` — every run exit 0, `Deleted: 1 file(s), 1 dir(s)`; APFS pairs each source spelling with the destination's (the reviewer confirmed the same inode by lookup). Admitted as HIGH on the same rule. Root cause: no text fold reproduces the filesystem's equivalence while keeping a glob's semantics (`?` counts, byte widths, `ß`→`SS`); cr-g3fix5-1's fold was always an approximation, and the reviewer re-rated the residual as data loss.

## What
Under `mirror`, an entry the source walk excluded protects the destination's own spelling of the same entry — `aSS` for `aß`, `s` for `ſ`, NFC `É` for NFD `é` — by the filesystem's own pairing, with no text folding anywhere; the exclude's glob syntax never has to be re-read under an equivalence.

Fix shape, admitted (the exact mechanism; replaces the fold):
- **The source walk records what it prunes.** Where the enumeration drops an entry because of the exclude rules (`enumeration.rs` ~237 `allows_dir`, and the file-level `allows_entry` rejections; the remote source's `pruning_rules()` path in `remote/transfer/source.rs` ~937 likewise), the entry's relative path, exactly as the source spells it, is recorded with the session as a protected source path — directories and files alike (one path per pruned subtree root, one per pruned file; the set scales with the pruned entries, like the manifest scales with the sent ones).
- **They reach the destination's deletion pass** on every route the pass serves: in process on the local carrier and the pull client; on the wire for a destination daemon through a new repeated field on the message that already carries the manifest's end or the session's filter (`ManifestComplete`, or `SessionOpen` beside `FilterSpec filter = 4` — the agent picks the message the destination has in hand when `mirror_delete_pass` runs), and `CONTRACT_VERSION` (`transfer_session/mod.rs:108`) goes 7 → 8 (1.0 peers refuse other builds at session open anyway; the orchestrator updates the "Wire contract 7" text in the release notes and CHANGELOG).
- **The deletion scope resolves each protected source path at the destination** component by component through `stored_name` (cr-g3fix4-1's identity pairing: the entry the destination filesystem pairs with that spelling, the link itself never its target, every name of a tie), once per path, and protects the resolved entry, its subtree and its ancestors (cr-g3fix5-1's `keep_dirs_holding_left_out_entries`). A path with no destination counterpart protects nothing. FAST: one lookup chain per pruned path, no per-entry cost on the walk beyond a set membership test.
- The textual rules as written and the literal per-directory lookups stay as they are, so a destination-only entry spelled exactly like, or paired by the filesystem with, a LITERAL exclude is still protected. **Residual, recorded in Known gaps and to be stated in the release notes as a limitation:** a destination-only entry (one the source does not have) that a pure-glob exclude would match only under the filesystem's case or normalization equivalence stays in the deletion scope.
- Guards: survival of `dst/aSS` under `*ß`, `dst/s` under `??` (source `ſ`), NFC `dst/É` under `???` (source NFD `é`), each on the local carrier and the destination daemon; a wire test that the protected paths arrive on the daemon route; the three cr-g3fix5 Unicode/glob guards stay green. Mutations: drop the recording → red; drop the wire field → the daemon-route guard red; resolve without `stored_name` → red.

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
