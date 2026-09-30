# cr-ssc2-3: The post-stat growth regression test never exercises the new probe

**Severity**: LOW — a regression of the one-byte growth probe is not detected by the existing guard
**Status**: Verified
**Branch**: — (default-branch mode; fixes land on master)
**Commit**: (filled after the fix commit)
**Reviewer**: codex / gpt-5.6-sol / xhigh / standard (D-2026-07-31-3 standing codereview; range 6bc3d08f..905ddb37, record .review/results/ssc-2-range.codex.json)

## Evidence
crates/blit-core/tests/source_side_containment.rs:865 — the fixture creates a 5096-byte file before calling the packer while declaring 4096 bytes, so the metadata-length check returns at payload.rs:386 and the one-byte probe at payload.rs:401 is never reached.

## Predicted observable failure
Removing or breaking the one-byte probe still leaves this test green, allowing growth between metadata inspection and the bounded read to regress without automated detection.

## Reviewer's suggested approach
Add an injectable reader or test synchronization hook that appends data after the metadata check but before the probe, then assert that the member is skipped.

## What
`build_tar_shard` is now a thin wrapper over `build_tar_shard_with(source_root, headers, open)` where `open: &dyn Fn(&Path) -> io::Result<OpenedMember { len, reader }>`; production passes `open_member_from_fs` (`File::open` + `metadata().len()` on that handle, the handle as the reader — unchanged behaviour). A test opener can make the reported length and the yielded bytes disagree, which is the only way to reach the one-byte probe deterministically.

## Guard proof
`packer_probe_catches_growth_after_the_stat` (`tests/source_side_containment.rs`): the opener reports `SMALL` bytes but yields `SMALL + 1`; the stat passes, the bounded read fills exactly, the probe reads one byte → the member is skipped with the changed-size reason. Mutation: the probe's result ignored (`map(|_| 0)`) → the guard FAILED (member packed; `scratchpad/cr-ssc-mutations.txt`); restored → green; all packer and shard tests green.

## Known gaps
None.
