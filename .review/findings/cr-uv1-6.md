# cr-uv1-6: `df` on Windows answered for the folder above a path that is not there

**Severity**: MEDIUM — a path that does not exist reports another path's numbers and success
**Status**: Reviewed — `utility-slices-r2` (Codex openreview, 2026-11-21) reviewed `4841b2d4..3cd49e40`, the seven `utility-slices-r1` fixes, with `guard_confirmed: true`, which required nothing further on this finding and, like this session, ran nothing on Windows. Still unproven there
**Branch**: — (default-branch mode; fixes land on master)
**Commit**: `278b38fc`
**Reviewer**: codex / gpt-6.2-astra (as dispatched) / max / frontier, codex-cli 0.171.0 headless one-shot, openreview `utility-slices-r1` over `b55efff7..49350572` (verdict `acceptable_with_changes`, capability_ok and guard_confirmed true); `.review/results/utility-slices-r1.codex.md`. Found by source inspection; the reviewer had no Windows run either.

## Evidence
The Windows branch uses `path.is_dir()` to choose between the operand and its parent. A missing leaf under an existing directory therefore returns the parent's statistics and success. Use a fallible metadata lookup first and preserve the OS error kind. The existing cross-platform test already expects `NotFound` for this case.

Cause. `GetDiskFreeSpaceExW` wants a directory, so a file is answered for by its folder; `is_dir()` is `false` for a file and for a path that is not there alike.

## Predicted observable failure
On a Windows daemon, `blit df HOST:/m/no-such-path` prints the module's filesystem and exits 0, where `df` says `No such file or directory`, exit 1.

## Intake
Admitted as the reviewer put it.

## What
`filesystem_space` (Windows) asks `std::fs::metadata(path)?` what the path is: a path that is not there returns the system's own error, kind and all, before any folder is chosen.

## Guard proof
- The guard already exists and is not gated: `disk_usage::tests::filesystem_space_asks_about_the_path_given` asserts `NotFound` for a missing entry under an existing folder. On Linux and macOS `statvfs` gives that itself, so the test passes there with or without this fix; **on Windows it must have been red since slice 1 landed** (`9b6a180f`) — no Windows run has happened since 2026-10-24, so nothing showed it.
- No mutation can go red on this machine: the changed lines compile only for Windows. They compile for `x86_64-pc-windows-gnu` under clippy `-D warnings`. That is a compile check, not evidence the fix works.

## Files changed
- `crates/blit-core/src/disk_usage.rs` — `filesystem_space`, Windows branch.
- `CHANGELOG.md`.

## Known gaps
- **Unproven until a Windows run**: CI's Windows leg (not run — GitHub has not been pushed) or `scripts/windows/run-blit-tests.ps1`.
- A link on Windows is followed by the lookup, as `df` follows one.
