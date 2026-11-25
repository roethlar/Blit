# ci-linux-ls-reference: the `ls` guards compared with whatever `ls` the system has

**Severity**: MEDIUM — two guards fail where the system's `ls` is not the reference (Ubuntu 26.04's uutils); the code says what GNU `ls` says.
**Status**: Fixed (the guards); awaiting the next CI run on Linux
**Branch**: — (default-branch mode; fixes land on master)
**Commit**: this commit (`Fix ci-linux-ls-reference`)
**Reviewer**: none — found by GitHub CI run 45520218555 at `e2c2e08d`, 2026-11-25 — the run after the first's seven fixes, which reached the test binaries the first never ran; for the next openreview.

## Evidence
`ls::ls_equals_the_system_ls` and `ls::ls_of_a_local_path_equals_the_system_ls` (Linux, runner image `ubuntu-26.04`), for a link loop:
- blit printed `ls: cannot access 't/ring/x': Too many levels of symbolic links`, which is GNU's;
- the system `ls` printed `ls: unknown io error: 't/ring/x', 'Os { code: 40, kind: FilesystemLoop, … }'`.

Ubuntu from 25.10 ships uutils coreutils as `ls`. The plan's reference is GNU coreutils and findutils under `C.UTF-8` (UTILITY_VERBS_LOCAL_PARITY), and the tests' own header says so: "where the system has the reference".

## Intake
Admitted. The fault is the guard's assumption of a platform, not the code under it.

## What
- `common::gnu_coreutils(tool)` finds GNU coreutils' own tool, as `tool` or as `gnu<tool>`. It knows GNU's by the first line of `--version`, the only way to tell an outside program apart; nothing of blit's own is read from words.
- The two `ls` guards compare with it.
- Where there is none, each guard says so on its output and compares nothing.

## Guard proof
- With a shim on `PATH` whose `ls --version` says uutils, both guards stand down and say why.
- With GNU `ls` 9.12 (this host), all 10 `ls` guards pass and compare as before.
- Checks: native, Windows and macOS (`CARGO_FEATURE_PURE=1`) clippy are clean.
- The fault itself, uutils's words, cannot be shown here: this host has no uutils.

## Files changed
- `crates/blit-cli/tests/common/mod.rs` — `gnu_coreutils`.
- `crates/blit-cli/tests/ls.rs` — the two guards.

## Known gaps
- On CI's `ubuntu-26.04`, unless GNU coreutils is installed there, the `ls` comparison stands down: CI no longer compares `ls` with the reference. Installing GNU coreutils on that runner is the owner's call (the workflow).
- `rm`, `du` and `df` compare with the system's tools the same way. `du` and `df` passed against uutils on this run; `rm` never ran, its binary coming after `ls`.
