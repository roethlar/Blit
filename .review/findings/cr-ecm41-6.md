# cr-ecm41-6: the kernel-walk guard took any `openat2` failure for its absence

**Severity**: MEDIUM — a guard passes over a broken call. Brought in by cr-ecm40-7.
**Status**: Fixed; awaiting review
**Branch**: — (default-branch mode; fixes land on master)
**Commit**: this commit (`Fix cr-ecm41-6`)
**Reviewer**: codex / gpt-6.2-astra (as dispatched) / max / frontier, codex-cli headless one-shot, openreview `combined-r42` over `b6e6fab9..e2c2e08d` (verdict `acceptable_with_changes`, capability_ok and guard_confirmed true); `.review/results/combined-r42.codex.md`.

## Evidence
"On a kernel where a valid `openat2` succeeded, I introduced an invalid mode. The guard printed `no openat2 here` and passed. Probe capability independently and fail on unexpected errors."

## Intake
Admitted as written.

## What
`an_ordinary_path_is_walked_by_the_kernel` asks the kernel with a call of its own: a plain `openat2` of the held root (`O_PATH | O_DIRECTORY`, `RESOLVE_BENEATH`).
- Only `ENOSYS`, or `EPERM` (a seccomp filter), stands the guard down; it says why.
- Any other error fails it.
- Where the probe succeeds, the code's own call must serve, as before.

## Guard proof
- Mutation, restored byte for byte: the code's own call broken (a mode it refuses) → red.
- Checks:
  - native, Windows and macOS (`CARGO_FEATURE_PURE=1`) clippy are clean;
  - the workspace passes on Linux, with the round's six fixes: 2,152 tests over 57 binaries.

## Files changed
- `crates/blit-core/src/copy/file_copy/mod.rs` — the guard.

## Known gaps
- None known.
