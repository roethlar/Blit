# cr-uv8-2: a missing module root was let through to a second look-up

**Severity**: HIGH — `ls`, `du` or `find` of a module whose folder is gone looks its path up again, and can read outside the module if a link has been put there by then
**Status**: Fixed, awaiting review
**Branch**: — (default-branch mode; fixes land on master)
**Commit**: this commit (`Fix cr-uv8-2`)
**Reviewer**: codex / gpt-6.2-astra (as dispatched) / max / frontier, codex-cli 0.171.0 headless one-shot, openreview `combined-r19` over `ca377805..17f4eb97` (verdict `acceptable_with_changes`, capability_ok and guard_confirmed true); `.review/results/combined-r19.codex.md`. It completes cr-uv7-2 (`cde61b9e`).

## Evidence
"`start_is_contained` treats a missing root as a successful check. `ls`, `du`, and `find` then look up the path again. If the root resolves elsewhere by then, the command can read outside the configured module. Pass the observed missing-path error directly to the verb's existing failure formatter."

## Intake
Admitted as the reviewer put it, and made structural: a verb can now read only a path the check hands it.

## What
`start_is_contained` returns `Readable`: `At(path)` — the start to read, the only way a read gets one — or `Gone(error)`, the not-found error seen at the root's path. `ls`, `du` and `find` match on it before anything is read: on `Gone` each sends its own failure for the start (`cannot access …`, and `find`'s form), with the error seen, and returns. Nothing is looked up again.

## Guard proof
- `typed_operand_tests::a_start_is_judged_by_the_part_of_it_that_can_be_looked_up` (changed): with nothing at the root's path, the module and a path below it are `Gone` with the not-found error — not a path to read.
- `ls.rs::a_module_whose_folder_has_gone_is_told_as_a_path_that_is_not_there` (cr-uv7-2's, end to end) passes unchanged: the words and exit statuses are the same.
- Mutation, restored byte for byte: a missing root let through to be read, as it was (the unit guard red).
- The verbs cannot read on `Gone`: the path they read comes only from `At`, by the type.

## Files changed
- `crates/blit-daemon/src/service/admin.rs` — `start_is_contained`, `Readable`, the three verbs; guard.

## Known gaps
- No guard reproduces the race itself (a link put at the root's path between the check and a read): there is no read after `Gone` for it to reach.
- Linux only.
