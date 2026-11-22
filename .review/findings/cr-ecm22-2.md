# cr-ecm22-2: a chain of links at the destination was followed one step

**Severity**: HIGH — a resume through `entry → middle → actual` replaced `middle` with a regular file and left the old bytes in `actual`, reporting success
**Status**: Fixed on Unix, awaiting review
**Branch**: — (default-branch mode; fixes land on master)
**Commit**: this commit (`Fix cr-ecm22-2`)
**Reviewer**: codex / gpt-6.2-astra (as dispatched) / max / frontier, codex-cli 0.171.0 headless one-shot, openreview `combined-r23` over `5d2f8def..24d4064f` (verdict `acceptable_with_changes`, capability_ok and guard_confirmed true); `.review/results/combined-r23.codex.md`. It completes cr-ecm21-3 (`abe9d4a1`).

## Evidence
"For `entry → middle → actual`, Linux `copy --resume` exited 0, replaced `middle` with a regular file, and left the old bytes in `actual`. Resolve the final referent within the destination boundary before placing the replacement."

## Intake
Admitted. "The final referent" is taken as the file that opening the name reaches — the file a resume in place patches, since that open follows every link of the chain. The boundary is the one that open already has: a fresh partial replaces exactly the file the in-place path would have written, no other.

## What
`fresh_partial` follows the chain of links at the destination to its end (at most 40, Linux's own limit; more is an error), each relative link resolved against the directory it stands in, and makes the partial beside the file there. Every link of the chain is kept. The local resume and the wire's hold both take it.

## Guard proof
- `resume::tests::a_fresh_partial_through_a_chain_of_links_replaces_its_end`: a 600 source resumed through `entry → sub/middle → ../actual`, `actual` 644 — both links kept, `actual` holds the source's bytes.
- Mutation, restored byte for byte: one link of the chain followed (red).
- The resume and sink tests (131) pass. Windows cross-clippy clean.

## Files changed
- `crates/blit-core/src/copy/file_copy/resume.rs` — `fresh_partial`; guard.

## Known gaps
- The chain is read by name, then the rename made by name: a link changed between the two is not seen. The in-place path opens by name too.
- Unix only.
