# cr-ecm40-3: the held-root walk's `..` found its parent again by name

**Severity**: HIGH — a write can land in another directory. Brought in by cr-ecm39-6.
**Status**: Fixed; awaiting review
**Branch**: — (default-branch mode; fixes land on master)
**Commit**: this commit (`Fix cr-ecm40-3`)
**Reviewer**: codex / gpt-6.2-astra (as dispatched) / max / frontier, codex-cli headless one-shot, openreview `combined-r41` over `d7f5f5e9..b6e6fab9` (verdict `acceptable_with_changes`, capability_ok and guard_confirmed true); `.review/results/combined-r41.codex.md`.

## Evidence
"After an in-root parent was renamed and its old name reused, `Walk::up` returned the replacement directory's inode. A subsequent write can land in the wrong directory. Verify the identity of the reacquired parent before continuing through `..`."

## Intake
Admitted as written.

## What
The walk keeps, beside the name of each directory it enters, which directory that is: device and inode, read from the descriptor it opened (`identity_of`). Walking `..` back, it finds the parent again from the root, as cr-ecm39-6 made it, by the kernel's walk or a name at a time. It goes on only where the directory found is the one it came down through. Where it is not, it fails the file: a directory on the way was replaced during the copy. An absolute link's splice starts the record afresh with the names.

## Guard proof
- `resume::walk_identity_tests::a_parent_replaced_at_its_name_is_not_walked_back_into` (Linux), with `openat2` and without:
  - a walk down `a/b` walks back into `a`;
  - after `a` is renamed and another `a/b` made, it refuses.
- Mutation, restored byte for byte: the parent found again taken unasked → red.
- Checks:
  - native, Windows and macOS (`CARGO_FEATURE_PURE=1`) clippy are clean;
  - the workspace passes on Linux, with the round's seven fixes: 2,146 tests over 57 binaries.

## Files changed
- `crates/blit-core/src/copy/file_copy/resume.rs` — `Walk::ids`, `up`, `enter`, `splice`, `identity_of`; guard.

## Known gaps
- A directory moved out of the root while the walk stands in it is not looked for; the walk's names are walked from the held root.
