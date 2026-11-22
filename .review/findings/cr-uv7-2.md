# cr-uv7-2: a module whose folder had gone was refused whole

**Severity**: MEDIUM — `ls`, `du` and `find` of a module whose folder was moved or removed end with exit 1 and nothing in `--json`, where the local commands say `cannot access … No such file or directory`
**Status**: Fixed, awaiting review
**Branch**: — (default-branch mode; fixes land on master)
**Commit**: this commit (`Fix cr-uv7-2`)
**Reviewer**: codex / gpt-6.2-astra (as dispatched) / max / frontier, codex-cli 0.171.0 headless one-shot, openreview `combined-r18` over `7912da16..ca377805` (verdict `acceptable_with_changes`, capability_ok and guard_confirmed true); `.review/results/combined-r18.codex.md`. It completes cr-uv6-2 (`269d3b88`).

## Evidence
"After I renamed the module directory, remote `ls --json` exited 1 with empty stdout. Local `ls --json` on the missing path exited 2 with structured output. Return the lookup error through the utility's operand-failure response while continuing to refuse a root that resolves outside the export."

cr-uv6-2 resolves the module's root when it is read and refuses it unless it resolves to itself — and a root that could not be resolved at all was refused the same way, though nothing there leads anywhere. A path BELOW a missing root was refused too: judged by its deepest ancestor that is there, which lies above the module.

## Intake
Admitted as the reviewer put it, and widened to the paths below the module, which the same check refused for the same reason (found by the guard written for this).

## What
`start_is_contained` first asks whether anything stands at the root's path at all. Nothing there: the read goes on for the module and for anything below it, finds nothing, and says so in its command's own words and exit status. Something there that cannot be resolved (a link to nothing, a ring) and a root that resolves elsewhere are refused, as before.

## Guard proof
- `typed_operand_tests::a_start_is_judged_by_the_part_of_it_that_can_be_looked_up` (extended): with a link to nothing where the module's root was, the module is refused; with nothing there, the module and a path below it are let through to be read.
- `crates/blit-cli/tests/ls.rs::a_module_whose_folder_has_gone_is_told_as_a_path_that_is_not_there`: a daemon whose module folder is moved away after it started — `ls --json` exit 2 with the failure in it (`cannot access`, `No such file or directory`); `ls` exit 2 in `ls`'s words; `du` and `find` exit 1 in theirs.
- Mutations, each restored byte for byte: a root that is not there refused whole, as it was (both guards red); any root that cannot be resolved let through (the unit guard red).

## Files changed
- `crates/blit-daemon/src/service/admin.rs` — `start_is_contained`; guard.
- `crates/blit-cli/tests/ls.rs` — guard.

## Known gaps
- Between the look that finds nothing at the root's path and the read, something can be put there; the read then reads it. It is the window every check of a path before a read by path has (cr-uv6-2's record).
- `rm` and `find -delete` read their operands their own way and are not changed here.
- Linux only.
