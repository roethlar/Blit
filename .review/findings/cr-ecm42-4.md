# cr-ecm42-4: an absolute alias to a missing file beneath the root was refused

**Severity**: MEDIUM — a copy and a resume fail for a file inside the destination. cr-ecm41-4's remaining half.
**Status**: Fixed; awaiting review
**Branch**: — (default-branch mode; fixes land on master)
**Commit**: this commit (`Fix cr-ecm42-4`)
**Reviewer**: codex / gpt-6.2-astra (as dispatched) / max / frontier, codex-cli headless one-shot, openreview `combined-r43` over `e2c2e08d..4e12d761` (verdict `acceptable_with_changes`, capability_ok and guard_confirmed true); `.review/results/combined-r43.codex.md`.

## Evidence
"On Linux, I pointed `dst/entry` through an absolute alias to `dst/sub/leaf`. Ordinary copy and resume both succeeded with an existing leaf. Both exited 2 and reported that the path led outside when the leaf was absent. Resolve the dangling alias to its existing parent, verify that parent against the held root, and create the leaf there."

## Intake
Admitted as written.

## What
`alias_place` takes the target's real path as the system would follow it (`real_path`, new):
- what exists, resolved;
- a link to nothing, followed to where it points, at most `MAX_FOLLOWED_LINKS`;
- a name not there, kept after its parent's real path.

The root is then found among that real path's ancestors by identity, as before. The alias to a file not yet made had no real path; its outside parent was asked, and the file was refused.

## Guard proof
- `file_copy::…::an_absolute_link_to_an_alias_of_a_missing_file_beneath_the_root_has_it_made` (Linux), with `openat2` and without:
  - the file made through the alias (`create_regular_within`, as a copy makes it) and written lands beneath the root;
  - through an alias of a missing file outside, refused, and nothing made there.
- Mutation, restored byte for byte: a link to nothing taken for a name not there → red.
- Checks:
  - native, Windows and macOS (`CARGO_FEATURE_PURE=1`) clippy are clean;
  - the workspace passes on Linux, with the round's fixes: 2,156 tests over 57 binaries.

## Files changed
- `crates/blit-core/src/copy/file_copy/resume.rs` — `alias_place`, `real_path`.
- `crates/blit-core/src/copy/file_copy/mod.rs` — guard.

## Known gaps
- Off Linux the held-root walk is not taken (cr-ecm33-4, cr-ecm34-3's halves).
