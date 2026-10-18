# cr-g3fix6-1: Folding character-class text changes its membership and exposes excluded data

**Severity**: HIGH (reviewer: HIGH)
**Status**: Fixed
**Branch**: — (default-branch mode; fixes land on master)
**Commit**: `9bec0bf3`
**Reviewer**: codex / gpt-6-astra / xhigh / standard (the owner's `review`, 2026-10-18), codex-cli 0.162.1 headless one-shot, `-a never exec -m gpt-6-astra -c model_reasoning_effort="xhigh" -s workspace-write --json`, stdin closed, detached worktree `review-g3fix6-r1-19ace1ac`; codereview re-review g3fix6-r1 over `a157ae00..19ace1ac` (record `.review/results/g3fix6-r1.codex.json` + `.events.jsonl`); 2026-10-18 00:12–00:22Z; capability_ok=true; guard_confirmed=true (reviewer disabled `escaped_separator`'s star-run collapse → `an_escaped_separator_keeps_its_meaning_in_the_class_dialect` red, exit 101; restored by `git checkout`, blob hash verified, green; 1548/0/2 on its macOS gate, Linux cross-clippy clean, Windows cross-clippy stopped on a missing `ml64.exe` — its environment; it cites CI run 39080085302 green on all three OSes); SHAs pin-verified; closed: cr-g3fix5-3 true, cr-g3fix5-4 true, cr-g3fix5-1 false, cr-g3fix5-2 false, cr-g3fix4-2 false, cr-g3fix-1 false

## Evidence
crates/blit-core/src/transfers/filter.rs:1217 — path_steps folds the entire glob component before compiling it; fold at line 1301 normalizes and uppercases class syntax too. '[A-z]*' becomes '[A-Z]*', losing punctuation members including '_'. Reproduced on case-insensitive macOS with src/_x/cache/source.txt and dst/_x/Cache/saved.txt: mirror --exclude '[A-z]*/cache' omitted source.txt but deleted saved.txt and both destination directories, exiting 0. The absolute '<src>/[A-z]*/cache' form behaved identically. '[é]*/cache' also failed because NFC composes two independent class members.

## Predicted observable failure
A valid glob exclusion silently permits mirror to delete pre-existing data in its destination counterpart. The shared deletion matcher exposes local and daemon destinations to this failure.

## Reviewer's suggested approach
Parse glob syntax before applying equivalence. Preserve class membership by transforming members and ranges semantically, and never normalize across independent class members. Add survival tests for these relative and absolute exclusions.

## Intake
Reproduced 2026-10-18 with the release build of the candidate's code (`blit 1.0.0+df61b1d6464c`, identical product code to `19ace1ac`; script `scratchpad/g3fix6/repro.sh`, output `repro.out`): `mirror -y --exclude '[A-z]*/cache'` over `src/_x/cache/` with a pre-existing `dst/_x/Cache/saved.txt` printed `Deleted: 1 file(s), 2 dir(s)` and the file was gone (the reviewer's `[é]*/cache` case with precomposed é on both sides kept its file here; the reviewer's failing form had NFC composing two class members). The literal control `--exclude cache` kept `dst/x/Cache/saved.txt`. Admitted as HIGH on the standing rule that a data-loss path under `mirror` cannot stay a known gap. Root cause: cr-g3fix5-1/-2 fold glob TEXT (NFC + uppercase) and match the folded pattern only, so a class's membership changes (`[A-z]` → `[A-Z]` loses `_` and the other punctuation between `Z` and `a`) and the as-written reading is lost.

## What
Under `mirror`, a glob exclude's component is read as written — a class keeps every member the user typed — so `[A-z]*/cache` protects `dst/_x/Cache` on a case-insensitive destination as it protects `dst/_x/cache`.

Fix shape, admitted (with cr-g3fix6-2; `transfers::filter::DestinationNames` and its `Step` reading, `fs_enum::ExcludeRules`):
- Text folding of glob patterns is REMOVED from the deletion scope: a glob component, and a glob rule with no literal component, is matched exactly as written (globset, as `ExcludeRules` reads it everywhere else), on every destination. `fold` and the folded matchers go; `unicode-normalization` goes with them unless cr-g3fix6-2 needs it (it should not).
- The as-written reading is the only reading of glob text; equivalence for what a glob excluded comes from cr-g3fix6-2's exact mechanism, and literal components and literal any-depth names keep cr-g3fix5-1's per-directory `stored_name` lookup (exact, by identity).
- Guards: survival of `dst/_x/Cache/saved.txt` under `--exclude '[A-z]*/cache'` and the absolute `<src>/[A-z]*/cache` (local carrier and destination daemon), and a pure test that a class component is compiled from its typed text; mutation: fold the text again → red.

## Approach
- Removed the destination's NFC/uppercase glob-text fold and its dependency. `DestinationNames` now compiles a glob component from its typed text; pure glob rules remain under the filter's as-written reading. Literal components still resolve through the destination filesystem, and the source's pruned paths added by cr-g3fix6-2 protect paired destination entries.
- Added local and push-to-daemon mirror guards for the relative and absolute `[A-z]*/cache` forms. They check both a pruned source entry and a destination-only class member, plus deletion of an unrelated entry. A unit guard checks the compiled component text and membership.

## Files changed
- `crates/blit-core/src/transfers/filter.rs` — remove folding and compile glob components as written; replace folded-rule tests with class-membership guards.
- `crates/blit-core/src/transfer_session/mod.rs` — update the deletion-scope contract comment.
- `crates/blit-core/Cargo.toml`, `Cargo.lock` — remove the unused `unicode-normalization` dependency.
- `crates/blit-cli/tests/exclude_contract.rs` — local and daemon mirror guards.

## Guard proof
- macOS APFS case-insensitive: `cargo test -p blit-core a_glob_is_matched_as_written_at_the_destination --lib` passed (1/1); `cargo test -p blit-transfer --test exclude_contract class_glob_component_as_typed -- --nocapture` passed (2/2, no skip line).
- Mutation: compiling the component after `to_ascii_uppercase()` changed `[A-z]*` to `[A-Z]*`; the unit guard failed at the typed-text assertion. The mutation was removed. Full gate and post-restore focused results are recorded with the round's completion.

## Known gaps
- A destination-only entry that a pure glob matches only through the destination filesystem's case or Unicode equivalence can still be deleted; see cr-g3fix6-2 and the 1.0 release limitation. The source has no pruned counterpart to send for it.

## Reviewer comments
(filled by the re-review)
