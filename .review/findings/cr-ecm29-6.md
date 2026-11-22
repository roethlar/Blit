# cr-ecm29-6: the staging directory was removed by its name, whatever stood there

**Severity**: MEDIUM — the staging directory was let go by its name: moved, an unrelated empty directory put at its old name was removed; and one refused when it was opened was removed by its name too
**Status**: Fixed on Unix, awaiting review
**Branch**: — (default-branch mode; fixes land on master)
**Commit**: this commit (`Fix cr-ecm29-6`)
**Reviewer**: codex / gpt-6.2-astra (as dispatched) / max / frontier, codex-cli 0.171.0 headless one-shot, openreview `combined-r30` over `e3ea9517..fd603571` (verdict `acceptable_with_changes`, capability_ok and guard_confirmed true); `.review/results/combined-r30.codex.md`. It completes cr-ecm28-3 (`d3c390f8`).

## Evidence
"If the staging directory moves, cleanup can remove an unrelated empty directory at its old name. This also happens on the path that rejects a directory during opening. Leave the entry in place when cleanup cannot establish ownership."

## Intake
Admitted as the reviewer put it.

## What
- `open_stage` returns which directory it opened (device, inode).
- The staging directory is let go by its name only while what stands at that name is the directory held (`entry_at`, an `fstatat` that does not follow a link). It must also be empty, as before. Moved away, it is left wherever it went, and what stands at its old name is left too.
- A staging directory refused when it was opened is left as it is. The error names it.
- `entry_at` and `Entry` (mode, owner, device and inode of what stands at a name in a held directory) are new; later fixes use them.

## Guard proof
- `resume::tests::a_moved_stage_is_not_let_go_by_its_old_name` (Unix): while the partial is staged, the staging directory is moved away and an empty directory made at its old name. The resume succeeds, and the directory at the old name is left.
- `resume::tests::a_stage_not_known_to_be_the_one_made_is_left` (Unix): after the staging directory is made and before it is opened, it is replaced by a 0755 directory (a new seam, `AFTER_A_STAGE_IS_MADE_HOOKS`). The resume fails, the 0755 directory is left, and the destination is as it was.
- Mutations, each restored byte for byte:
  - the stage let go by its name whatever stands there → red;
  - a refused stage removed by its name → red.
- The resume, sink and session tests (341) pass.

## Files changed
- `crates/blit-core/src/copy/file_copy/resume.rs` — `fresh_as`, `open_stage`, `Entry`, `entry_at`, the seam; guards.

## Known gaps
- Between the look at the name and the removal, the name could still change hands. Only an empty directory can be removed that way.
- A staging directory left behind (moved, or refused) stays until someone removes it.
- Unix only.
