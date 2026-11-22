# cr-uv6-2: the module itself passed a read's containment by how its path is spelled

**Severity**: HIGH — a listing of a module whose root has been replaced by a link shows the directory the link points to
**Status**: Reviewed, found incomplete — `combined-r18` (Codex openreview, 2026-11-22) reviewed `7912da16..ca377805` — the six `combined-r17` fixes — with `guard_confirmed: true`. "After I renamed the module directory, remote `ls --json` exited 1 with empty stdout. Local `ls --json` on the missing path exited 2 with structured output. Return the lookup error through the utility's operand-failure response while continuing to refuse a root that resolves outside the export." The remaining work is cr-uv7-2
**Branch**: — (default-branch mode; fixes land on master)
**Commit**: `269d3b88`
**Reviewer**: codex / gpt-6.2-astra (as dispatched) / max / frontier, codex-cli 0.171.0 headless one-shot, openreview `combined-r17` over `13faafd2..7912da16` (verdict `acceptable_with_changes`, capability_ok and guard_confirmed true); `.review/results/combined-r17.codex.md`. It completes cr-uv5-5 (`ca4e6855`).

## Evidence
"A request for the root passes containment on pathname equality alone. If that pathname resolves elsewhere after daemon startup, a plain listing can expose the other directory. Validate the current root against the configured export while preserving the correct error for a file operand typed with a trailing slash."

cr-uv5-5 let the module itself through `start_is_contained` whenever the start's path was the root's path, so that a module that is one file is not judged by its ancestors (which put it above itself). The root is the path the daemon resolved when it started, with no link in it; a link put in its place since is not seen by comparing spellings.

## Intake
Admitted as the reviewer put it.

## What
For the module itself, `start_is_contained` resolves the root as it stands now and lets the read go on only if it resolves to itself. A root that resolves elsewhere, or cannot be resolved, is refused (`path containment: the module's root … now resolves to …`). A module that is one file, typed as a directory, still passes here and is told `Not a directory` by the read (cr-uv5-5's guard, unchanged).

## Guard proof
- `typed_operand_tests::a_start_is_judged_by_the_part_of_it_that_can_be_looked_up` (extended): the module's directory is moved away and a link to a directory outside put at its path; the module typed `""` and `.`, and paths below it, are all refused with `PermissionDenied` and "path containment". The module put back, the rest of the guard runs as before.
- Mutation, restored byte for byte: the module taken for the module by its spelling, as it was (the guard red).
- `ls` end to end: `a_module_that_is_one_file_is_read_as_the_file_it_is` passes unchanged.

## Files changed
- `crates/blit-daemon/src/service/admin.rs` — `start_is_contained`; guard.

## Known gaps
- Between this check and the read the root can still be swapped: the same window as every check of a path before it is read by path. `find -delete` and `rm` do not go that way (they open what they remove, from the root down).
- A path BELOW the module was already judged by resolving it, and is unchanged.
- Linux only.
