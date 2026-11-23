# cr-ecm32-6: the release note said a destination FIFO is left, but a resume replaces it

**Severity**: MEDIUM — brought in by cr-ecm31-4: the `CHANGELOG.md` entry said a FIFO or device at a destination path fails that file and is left as it is, but with `--resume` such a destination is replaced by a fresh copy of the file (cr-ecm26-5), as a guard of its own shows
**Status**: Fixed, awaiting review
**Branch**: — (default-branch mode; fixes land on master)
**Commit**: this commit (`Fix cr-ecm32-6`)
**Reviewer**: codex / gpt-6.2-astra (as dispatched) / max / frontier, codex-cli 0.171.0 headless one-shot, openreview `combined-r33` over `714ed0b7..70efef8a` (verdict `acceptable_with_changes`, capability_ok and guard_confirmed true); `.review/results/combined-r33.codex.md`. It completes cr-ecm31-4 (`374ed558`).

## Evidence
"It promises that a destination FIFO survives. The existing Unix local `--resume` test deliberately replaces the FIFO with a regular file containing the source bytes, and that test passed. Document the distinction between an ordinary copy and a local resume." (`CHANGELOG.md`; `crates/blit-core/src/copy/file_copy/resume.rs`, `a_fifo_at_the_name_is_replaced_not_opened`.)

## Intake
Admitted as the reviewer put it. The behaviour is right; the words were wrong.

## What
The entry now tells the two apart. At a destination path, an ordinary copy fails that file and leaves the FIFO or device as it is. With `--resume`, a destination that is no regular file is replaced by a fresh copy of the file, as before (cr-ecm26-5).

## Guard proof
Docs only. The behaviour each half describes has its guard:
- `sink::tests::a_fifo_at_a_destination_does_not_hold_the_copy` (an ordinary copy: the FIFO left, the file failed);
- `resume::tests::a_fifo_at_the_name_is_replaced_not_opened` (a resume: the FIFO replaced).
`git diff --check` and `scripts/agent/check-docs.sh` pass.

## Files changed
- `CHANGELOG.md`.

## Known gaps
- None.
