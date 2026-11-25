# cr-ecm40-5: an absolute alias to a directory beneath the root was refused

**Severity**: MEDIUM — a copy and a resume fail for a file inside the destination. Brought in by cr-ecm39-5.
**Status**: Reviewed, found incomplete — `combined-r42` (Codex openreview, 2026-11-25) reviewed `b6e6fab9..e2c2e08d` — the `combined-r41` fixes, log-relay-cut and GitHub CI's seven — with `guard_confirmed: true`: "I would keep the held-directory walk, the identity check when it reopens a parent, and the change that marks a collapsed failure list incomplete. I found six corrections the approach still needs." "On Linux, `dst/entry` pointed through an absolute alias to an existing `dst/sub/file`. Both `copy` and `copy --resume` exited 2 and left the old bytes in place. The resolver skips the canonical leaf because it is a regular file, then examines the alias's outside parent. Resolve the leaf when following it and verify its resolved parent against the held root." The remaining work is cr-ecm41-4
**Branch**: — (default-branch mode; fixes land on master)
**Commit**: `89b107d6`
**Reviewer**: codex / gpt-6.2-astra (as dispatched) / max / frontier, codex-cli headless one-shot, openreview `combined-r41` over `d7f5f5e9..b6e6fab9` (verdict `acceptable_with_changes`, capability_ok and guard_confirmed true); `.review/results/combined-r41.codex.md`.

## Evidence
"An absolute alias to the destination root worked. An alias directly to a subdirectory of that root caused both ordinary copy and resume to exit 2 for a file inside the destination. Verify the resolved directory's ancestry against the held root for aliases to descendants."

## Intake
Admitted as written.

## What
`alias_place` takes the deepest directory of the target there is, and its real path, links resolved (`canonicalize`). Among that real path's ancestors it finds the one that is the root, by identity: device and inode, as cr-ecm39-5 told the root. The target's place beneath the root is the rest of the real path, then the rest of the target. An alias to a directory beneath the root has no ancestor that is the root by its own names; its real path has one.

## Guard proof
- `file_copy::…::an_absolute_link_through_an_alias_beneath_the_root_is_followed` (Linux), with `openat2` and without:
  - a write through an alias of a subdirectory lands there;
  - a directory is made through it;
  - a link through an alias of a directory outside is refused, with nothing made there.
- `an_absolute_link_through_an_alias_of_the_root_is_followed` stays green.
- Mutation, restored byte for byte: an alias placed by its own names → red.
- Checks:
  - native, Windows and macOS (`CARGO_FEATURE_PURE=1`) clippy are clean;
  - the workspace passes on Linux, with the round's seven fixes: 2,146 tests over 57 binaries.

## Files changed
- `crates/blit-core/src/copy/file_copy/resume.rs` — `alias_place`.
- `crates/blit-core/src/copy/file_copy/mod.rs` — guard.

## Known gaps
- Off Linux the held-root walk is not taken (cr-ecm33-4, cr-ecm34-3's halves).
