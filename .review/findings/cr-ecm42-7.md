# cr-ecm42-7: on a filesystem that folds case the mirror guard checked no entry

**Severity**: MEDIUM — a wrong removal of an unambiguous name passes on macOS. ci-macos-case-mirror's remaining half.
**Status**: Fixed; awaiting review
**Branch**: — (default-branch mode; fixes land on master)
**Commit**: this commit (`Fix cr-ecm42-7`)
**Reviewer**: codex / gpt-6.2-astra (as dispatched) / max / frontier, codex-cli headless one-shot, openreview `combined-r43` over `e2c2e08d..4e12d761` (verdict `acceptable_with_changes`, capability_ok and guard_confirmed true); `.review/results/combined-r43.codex.md`.

## Evidence
"The new probe detects folding independently, but both post-mirror loops then skip every entry. An incorrect deletion of an unambiguous name such as `xA` can pass. Assert the outcomes for distinct entries and exercise colliding spellings in separate fixtures."

## Intake
Admitted; separate fixtures for every entry, which covers both the distinct and the colliding ones.

## What
Each entry of each case is also run on its own, in a destination of its own: written alone, mirrored as typed and with `--ignore-case`, and its outcome asserted both ways. On a filesystem that folds, two spellings are one entry in the run with all of them, which checks none; alone, each is itself. The runs alone go on every filesystem, so this host's runs check them too.

## Guard proof
- `case_note::the_line_claims_only_what_the_mirror_then_does`: all 16 `case_note` guards pass on Linux.
- With folding forced on, so that only the runs alone check, it passes.
- Mutation, restored byte for byte: an entry alone expected the other way → red.
- On a filesystem that folds, the proof is the next CI run on macOS: this host's filesystems do not fold.
- Checks:
  - native, Windows and macOS (`CARGO_FEATURE_PURE=1`) clippy are clean;
  - the workspace passes on Linux, with the round's fixes: 2,156 tests over 57 binaries.

## Files changed
- `crates/blit-cli/tests/case_note.rs` — the guard.

## Known gaps
- None known.
