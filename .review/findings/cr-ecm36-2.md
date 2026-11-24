# cr-ecm36-2: the Linux mmap copy's cleanup judged a link at the name by the link

**Severity**: HIGH — a failed copy can look finished. Brought in by cr-ecm35-2. When the destination's name was a link to the file the copy wrote through, the cleanup compared the link's identity with the written file's, found them different, and gave up with the error alone. The file stayed at the source's advertised length, unmarked, with no `LeftInPlace` marker.
**Status**: Fixed on Linux; awaiting review
**Branch**: — (default-branch mode; fixes land on master)
**Commit**: this commit (`Fix cr-ecm36-2`)
**Reviewer**: codex / gpt-6.2-astra (as dispatched) / max / frontier, codex-cli headless one-shot, openreview `combined-r37` over `d2494965..2c55cb3a` (verdict `acceptable_with_changes`, capability_ok and guard_confirmed true); `.review/results/combined-r37.codex.md`. It completes cr-ecm35-2 (`795a9662`).

## Evidence
"When the destination is a symlink to the file being written, cleanup compares the link's inode with the target's inode and returns the original error. A read failure can leave a partially written target at the advertised source length with no `LeftInPlace` marker. The sink already handles this case through its retained descriptor."

## Intake
Admitted as the reviewer put it.

## What
`settle_failed_copy` (`copy/file_copy/mmap.rs`) takes the sink's rule (cr-ecm17-2):
- **The name is the file written.** It is removed, and marked if it cannot be removed, as before.
- **The name is a link that leads to the file written.** The link is not the copy's to remove. The file is marked unfinished through the handle the copy wrote through, one byte longer than its source, and the error carries `LeftInPlace`: "its name is a link to it, which is not this copy's to remove".
- **Anything else at the name** is somebody's, and is left as it is.

## Guard proof
- `file_copy::mmap::tests::a_failed_copy_through_a_link_marks_what_it_wrote` (Linux): a copy through a link whose source shrinks fails. The target the link names is nine bytes long, the link stays, and the error is marked left in place.
- Mutation, restored byte for byte: judged by the link → red.
- Checks: native clippy is clean.

## Files changed
- `crates/blit-core/src/copy/file_copy/mmap.rs` — `settle_failed_copy`; guard.

## Known gaps
- macOS unchanged (cr-ecm33-4).
