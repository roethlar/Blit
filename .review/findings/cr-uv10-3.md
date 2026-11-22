# cr-uv10-3: a module root that cannot be looked up was refused, not told

**Severity**: MEDIUM — `ls`, `du` or `find` of a module a folder above which the daemon may not search fails the request as a containment refusal, not as the verb's own `cannot access`
**Status**: Verified — `combined-r22` (Codex openreview, 2026-11-22) reviewed `b92c9a17..5d2f8def` — the eight `combined-r21` fixes — with `guard_confirmed: true`: "preserving the original lookup error … [is] sound"
**Branch**: — (default-branch mode; fixes land on master)
**Commit**: `97815d1e`
**Reviewer**: codex / gpt-6.2-astra (as dispatched) / max / frontier, codex-cli 0.171.0 headless one-shot, openreview `combined-r21` over `c15ebf5a..b92c9a17` (verdict `acceptable_with_changes`, capability_ok and guard_confirmed true); `.review/results/combined-r21.codex.md`. It completes cr-uv9-1 (`5a663780`).

## Evidence
"With search permission removed from the module's parent, my Linux probe observed `EACCES`. The daemon converted it into a request-level containment refusal, bypassing the utility's operand-error reporting. Return the observed lookup error to the verb."

## Intake
Admitted as the reviewer put it.

## What
`start_is_contained`: any error looking at the module root itself — not there (cr-uv7-2), a folder above it made a file (cr-uv9-1), a folder above it this daemon may not search — is the verb's to tell, with that error, for the start it was asked about (`Readable::Gone`). A root that is there but cannot be resolved (a link to nothing, a ring) is still refused, as before.

## Guard proof
- `typed_operand_tests::a_start_is_judged_by_the_part_of_it_that_can_be_looked_up` (a case added): a module whose parent is mode 000 is `Gone` with `PermissionDenied`. Skipped, and says so, for a process a directory's mode does not bar.
- Mutation, restored byte for byte: only a root not there, or under a file, told (red).

## Files changed
- `crates/blit-daemon/src/service/admin.rs` — `start_is_contained`; guard.

## Known gaps
- The guard proves nothing run as root; it says so and passes.
