# cr-ecm22-6: the fresh partial's name beside the destination could be too long

**Severity**: MEDIUM — a resume into a destination whose name is the longest a name may be fails with `File name too long`
**Status**: Fixed, awaiting review
**Branch**: — (default-branch mode; fixes land on master)
**Commit**: this commit (`Fix cr-ecm22-6`)
**Reviewer**: codex / gpt-6.2-astra (as dispatched) / max / frontier, codex-cli 0.171.0 headless one-shot, openreview `combined-r23` over `5d2f8def..24d4064f` (verdict `acceptable_with_changes`, capability_ok and guard_confirmed true); `.review/results/combined-r23.codex.md`. It completes cr-ecm21-3 (`abe9d4a1`).

## Evidence
"A valid 255-byte basename made Linux `copy --resume` exit 2 with `File name too long` because the temporary name includes the entire basename plus a suffix. Generate a short sibling name independent of the destination basename and retry collisions."

## Intake
Admitted as the reviewer put it.

## What
`fresh_partial` makes its partial beside the destination as `.blit-partial-<pid>-<n>` — short, and nothing of the destination's name in it — and tries the next one when that name is taken (up to sixteen).

## Guard proof
- `resume::tests::a_destination_of_the_longest_name_takes_a_fresh_partial`: a 600 source into a 644 destination named with 255 bytes — resumed, the source's bytes.
- Mutation, restored byte for byte: the name made beside it holding the destination's (red).
- The resume and sink tests (129) pass. Windows cross-clippy clean.

## Files changed
- `crates/blit-core/src/copy/file_copy/resume.rs` — `fresh_partial`; guard.

## Known gaps
- The retry on a taken name has no guard: the counter is the process's, and no test can tell which name comes next.
