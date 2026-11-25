# cr-ecm41-4: an absolute alias to a regular file beneath the root was refused

**Severity**: MEDIUM — a copy and a resume fail for a file inside the destination. cr-ecm40-5's remaining half.
**Status**: Fixed; awaiting review
**Branch**: — (default-branch mode; fixes land on master)
**Commit**: this commit (`Fix cr-ecm41-4`)
**Reviewer**: codex / gpt-6.2-astra (as dispatched) / max / frontier, codex-cli headless one-shot, openreview `combined-r42` over `b6e6fab9..e2c2e08d` (verdict `acceptable_with_changes`, capability_ok and guard_confirmed true); `.review/results/combined-r42.codex.md`.

## Evidence
"On Linux, `dst/entry` pointed through an absolute alias to an existing `dst/sub/file`. Both `copy` and `copy --resume` exited 2 and left the old bytes in place. The resolver skips the canonical leaf because it is a regular file, then examines the alias's outside parent. Resolve the leaf when following it and verify its resolved parent against the held root."

## Intake
Admitted as written.

## What
`alias_place` takes the deepest part of the target that resolves, whatever it is: a regular file has a real path too. Among that real path's ancestors it finds the root by identity, as cr-ecm40-5 does. Passed over as no directory, the leaf's outside parent was asked instead.

## Guard proof
- `file_copy::…::an_absolute_link_to_an_alias_of_a_file_beneath_the_root_is_followed` (Linux), with `openat2` and without:
  - a write through an alias of a file beneath the root lands in it;
  - through an alias of a file outside, refused, and that file untouched.
- Mutation, restored byte for byte: the leaf passed over when it is no directory → red.
- Checks:
  - native, Windows and macOS (`CARGO_FEATURE_PURE=1`) clippy are clean;
  - the workspace passes on Linux, with the round's six fixes: 2,152 tests over 57 binaries.

## Files changed
- `crates/blit-core/src/copy/file_copy/resume.rs` — `alias_place`.
- `crates/blit-core/src/copy/file_copy/mod.rs` — guard.

## Known gaps
- Off Linux the held-root walk is not taken (cr-ecm33-4, cr-ecm34-3's halves).
