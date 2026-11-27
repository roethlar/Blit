# ci-macos-clone-no-room: two `job_retry` guards make a write fail by a limit on a file's size, which a clone never meets

**Severity**: MEDIUM — the guards fail on macOS, where the code is right.
**Status**: Fixed; awaiting review
**Branch**: — (default-branch mode; fixes land on master)
**Commit**: this commit (`Fix ci-macos-clone-no-room`)
**Reviewer**: none — GitHub CI run 45823369010 (`ebc9d81c`, 2026-11-26), macOS; for the next openreview.

## Evidence
GitHub CI run 45823369010, macOS — the first run there to reach `job_retry`:
- `an_ignore_existing_run_leaves_no_partial_for_a_retry_to_write_over`: `left: Some(0)`, `right: Some(2)`, the run's words `Copied: 1 file(s), 128.00 KiB`;
- `a_retry_writes_where_the_run_it_retries_wrote` (`there at first false`): the same, `Copied: 2 file(s)`.

Both make a write fail for want of room with `ulimit -f` (`blit_with_no_room_past`). On macOS a copy within one APFS volume is a clone (`fclonefileat`, `crates/blit-core/src/copy/file_copy/mod.rs`): no byte is written, so no limit on a file's size fails it. The file lands whole, as it should.

## Intake
Admitted; the guards', not the code's.

## What
`met_no_limit` (new, in the test): a run made under the limit that exited 0 with the whole file landed met no limit at all. On macOS the guard then has nothing to check and says so; anywhere else it fails, saying the limit did not hold — so the stand-down cannot hide a limit that stopped working on Linux. A run that exits 0 WITHOUT the whole file is not that: it is a copy reported as landed that did not, and fails the guard as before.

## Guard proof
- Both guards: green on Linux.
- macOS's condition made here — no limit set — off macOS: both red, `the limit on a file's size did not hold here` (the detection the stand-down rests on).
- The same with the platform taken for macOS: both green, each saying `the copy here is a clone, which no size limit fails: nothing to check`.
- The stand-down itself on APFS: the next CI run.
- Checks:
  - native, Windows and macOS (`CARGO_FEATURE_PURE=1`) clippy are clean;
  - the workspace passes on Linux, with the round's fixes: 2162 tests over 57 binaries;
  - and with its temporary files on a filesystem that folds case: 2161 passed, the one failure the setgid test
    the namespace's unmapped groups refuse, green outside it.

## Files changed
- `crates/blit-cli/tests/job_retry.rs` — `met_no_limit`; the two guards.

## Known gaps
- On macOS the two guards check nothing of a failed write: a write that fails for want of room cannot be made there by a size limit. The Linux run is their evidence.
