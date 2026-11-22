# cr-ecm29-1: a failed swap-back could unlink the entry it said it kept

**Severity**: HIGH — after an exchange found an unexpected occupant, the cleanup removed the stage's entry even when the exchange back failed, unlinking the displaced file the error said was kept; and what came to the name during the putting back landed in the cleanup's path and was removed too
**Status**: Fixed on Linux, awaiting review
**Branch**: — (default-branch mode; fixes land on master)
**Commit**: this commit (`Fix cr-ecm29-1`)
**Reviewer**: codex / gpt-6.2-astra (as dispatched) / max / frontier, codex-cli 0.171.0 headless one-shot, openreview `combined-r30` over `e3ea9517..fd603571` (verdict `acceptable_with_changes`, capability_ok and guard_confirmed true); `.review/results/combined-r30.codex.md`. It completes cr-ecm28-2 (`3c25cd73`).

## Evidence
"After an exchange finds an unexpected occupant, `unmade()` runs even if the reverse exchange fails. It can unlink the displaced file while the error says it was retained. A concurrent replacement during rollback can also put a different file in cleanup's path. Preserve the entry on an ambiguous restore and report its exact recovery path."

## Intake
Admitted as the reviewer put it.

## What
- `fresh_as` learns which file the partial is (device and inode, through its own handle) as soon as it is made. If that look fails, the file fails and the error says where the partial is left.
- The cleanup (`unmade`) removes the stage's entry only while it is the partial (`entry_at` on the stage's descriptor). The stage itself goes only once empty, and only while it is the one held (cr-ecm29-6). Removed whatever it was, what a failed swap-back left there went, and so did what came to the name meanwhile.
- `Resolved::publish_replacing`, on an unexpected occupant, swaps it back and runs the cleanup. If anything is still in the stage after that, the error names its exact path (`<directory>/.blit-partial-<pid>-<n>/partial`):
  - swap-back failed: what stood at the name is left there, and the unfinished partial at the name;
  - swap-back done: what came to the name while it was put back is left there.
- `Resolved` carries its held directory's canonical path (`place`) to say so.
- New seams: `BACK_FAULTS` (the swap-back refused, `EIO`) and `AFTER_AN_EXCHANGE_HOOKS`.

## Guard proof
- `resume::tests::what_is_not_put_back_is_left_where_the_error_says` (Linux): a link takes the name while the partial is staged, and the swap-back is refused. The resume fails, the link is left in the stage at the path the error names, and the file it points to is untouched.
- `resume::tests::what_comes_while_it_is_put_back_is_left_where_the_error_says` (Linux): as above, and after the exchange a regular file takes the name. The swap-back brings that file into the stage and puts the link back at the name. The resume fails, the file is left in the stage at the path the error names, and the link stands at the name.
- Mutations, each restored byte for byte:
  - the stage's entry removed whatever it is → red (both);
  - the failed swap-back's error not naming where its entry is → red.
- The resume, sink and session tests (347) pass. Windows cross-clippy clean.

## Files changed
- `crates/blit-core/src/copy/file_copy/resume.rs` — `Resolved::place`, `fresh_as`, `publish_replacing`, the seams; guards.

## Known gaps
- What is left in a stage stays until someone moves it back or removes it. The error says where.
- Between the cleanup's look at the stage and its removal, a same-user process could still change the stage. No other user can write in it.
