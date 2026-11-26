# cr-ecm43-5: the mirror guard expected a plain-name exclude not to keep the entry the filesystem pairs with it

**Severity**: MEDIUM — the guard fails on macOS and Windows, where the product keeps that entry by its documented rule. cr-ecm42-7's remaining half; GitHub CI run 45731868559.
**Status**: Fixed; awaiting review
**Branch**: — (default-branch mode; fixes land on master)
**Commit**: this commit (`Fix cr-ecm43-5`)
**Reviewer**: codex / gpt-6.2-astra (as dispatched) / max / frontier, codex-cli headless one-shot, openreview `combined-r44` over `4e12d761..34c77a48` (verdict `acceptable_with_changes`, capability_ok and guard_confirmed true); `.review/results/combined-r44.codex.md`.

## Evidence
"Correct the mirror fixture's expectation for literal excludes. The pinned macOS and Windows jobs both kept `keep` under `--exclude Keep`. The test expected deletion even though the documented rule protects a destination spelling that resolves to the excluded literal. Keep the per-entry assertions and account for that pairing."

GitHub CI run 45731868559 (`34c77a48`): on macOS and on Windows, `["Keep", "[!a].dat"], keep alone, None` — `left: true`, `right: false`. Reproduced here on a filesystem that folds case (`fold_run.sh`).

## Intake
Admitted as written. The rule is cr-g3fix5-1's: a plain-name exclude keeps the destination entry its filesystem pairs with it.

## What
Each entry run alone asks the filesystem, with that entry alone there, whether a plain-name exclude of the case — `Keep`, `a`, `./A` at the root — names it (`dir.join(name).exists()`); where it does, the entry is expected kept, as typed and with `--ignore-case`. Where nothing folds, a plain name pairs only with itself, which the table already keeps: Linux's expectations are unchanged.

## Guard proof
- `case_note::the_line_claims_only_what_the_mirror_then_does`: green on Linux, and on a filesystem that folds case, where it failed as CI's did.
- Mutations, restored byte for byte, each → red on the folding filesystem:
  - the pairing not asked of the filesystem (CI's failure);
  - a plain-name exclude not paired through the destination (`DestinationNames::lookup` gives nothing) — the product's rule, which the guard now checks where the filesystem folds.
- On Linux, cr-ecm42-7's mutation (an entry alone expected the other way) → red.
- Checks:
  - native, Windows and macOS (`CARGO_FEATURE_PURE=1`) clippy are clean;
  - the workspace passes on Linux, with the round's fixes: 2158 tests over 57 binaries;
  - and with its temporary files on a filesystem that folds case (`fold_run.sh`: a casefold tmpfs in a user
    namespace, the tests run as this user): 2157 passed, the one failure a setgid test the namespace's unmapped
    groups refuse (`a_fresh_partial_takes_its_setgid_directorys_group`, `EINVAL`), green outside it.

## Files changed
- `crates/blit-cli/tests/case_note.rs` — the guard.

## Known gaps
- The next CI run is the evidence on APFS and NTFS.
