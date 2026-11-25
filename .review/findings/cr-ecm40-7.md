# cr-ecm40-7: the kernel-walk guard assumed `openat2`

**Severity**: MEDIUM — a guard fails where the seam or the host leaves the walk to serve; the code is right. Brought in by cr-ecm39-6.
**Status**: Reviewed, found incomplete — `combined-r42` (Codex openreview, 2026-11-25) reviewed `b6e6fab9..e2c2e08d` — the `combined-r41` fixes, log-relay-cut and GitHub CI's seven — with `guard_confirmed: true`: "I would keep the held-directory walk, the identity check when it reopens a parent, and the change that marks a collapsed failure list incomplete. I found six corrections the approach still needs." "On a kernel where a valid `openat2` succeeded, I introduced an invalid mode. The guard printed `no openat2 here` and passed. Probe capability independently and fail on unexpected errors." The remaining work is cr-ecm41-6
**Branch**: — (default-branch mode; fixes land on master)
**Commit**: `2b605ed7`
**Reviewer**: codex / gpt-6.2-astra (as dispatched) / max / frontier, codex-cli headless one-shot, openreview `combined-r41` over `d7f5f5e9..b6e6fab9` (verdict `acceptable_with_changes`, capability_ok and guard_confirmed true); `.review/results/combined-r41.codex.md`.

## Evidence
"With `openat2` unavailable through the repository's test seam, path resolution succeeded but the new fast-path test failed because the fallback held two descriptors. Condition the fast-path assertion on syscall availability and retain the bounded-descriptor test for the fallback."

## Intake
Admitted as written.

## What
`an_ordinary_path_is_walked_by_the_kernel` first asks the held root whether `openat2` beneath it works. Where it does not, the guard stands down and says why. The walk that then serves keeps its own guard, `a_deep_walk_holds_one_directory_at_a_time`.

## Guard proof
- Mutation, restored byte for byte: every path walked by the held-root walk → red, so the guard still bites where `openat2` serves.
- Checks:
  - native, Windows and macOS (`CARGO_FEATURE_PURE=1`) clippy are clean;
  - the workspace passes on Linux, with the round's seven fixes: 2,146 tests over 57 binaries.

## Files changed
- `crates/blit-core/src/copy/file_copy/mod.rs` — the guard.

## Known gaps
- On a host without `openat2` the guard proves nothing; it says so on its output.
