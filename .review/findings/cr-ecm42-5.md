# cr-ecm42-5: the `ls` guards ran GNU's `ls` under the name it is installed as

**Severity**: MEDIUM — two guards fail where GNU's `ls` is installed as `gnuls` (CI's `ubuntu-26.04`); the code is right. ci-linux-ls-reference's remaining half.
**Status**: Fixed; awaiting review and the next CI run on Linux
**Branch**: — (default-branch mode; fixes land on master)
**Commit**: this commit (`Fix cr-ecm42-5`)
**Reviewer**: codex / gpt-6.2-astra (as dispatched) / max / frontier, codex-cli headless one-shot, openreview `combined-r43` over `e2c2e08d..4e12d761` (verdict `acceptable_with_changes`, capability_ok and guard_confirmed true); `.review/results/combined-r43.codex.md`. GitHub CI run 45715973925 at `4e12d761` failed the guards first.

## Evidence
"The pinned Linux CI runner found GNU `ls` as `gnuls`. Both parity tests failed because its diagnostic began `gnuls:` and the expected prefix was `ls:`. Invoke the selected reference with `argv[0]` set to `ls`, or normalize just that program-name prefix."

`ls::ls_equals_the_system_ls` and `ls::ls_of_a_local_path_equals_the_system_ls` (Linux):
- blit printed `ls: cannot access 't/a.txt/': Not a directory`;
- the reference printed `gnuls: cannot access 't/a.txt/': Not a directory`.

ci-linux-ls-reference found GNU's `ls` on the runner as `gnuls`, so the comparison ran. GNU names itself by what it was called.

## Intake
Admitted; the reviewer's first remedy, made from CI's evidence before the review returned. The fault is the guards', not the code under them.

## What
The guards run the reference called `ls` (`CommandExt::arg0`), whatever name it is installed under.

## Guard proof
- CI's case reproduced on this host: on `PATH`, an `ls` whose `--version` says uutils, and GNU's `ls` as `gnuls`.
  - Before the fix, both guards failed, as on CI: `gnuls: cannot access …`.
  - After it, both pass.
- With this host's own GNU `ls` 9.12, all ten `ls` guards pass.
- Checks: native, Windows and macOS (`CARGO_FEATURE_PURE=1`) clippy are clean.

## Files changed
- `crates/blit-cli/tests/ls.rs` — `system_ls`.

## Known gaps
- None known. CI's `ls` comparison now runs against GNU's.
