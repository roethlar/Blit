# cr-ssc2-3: The post-stat growth regression test never exercises the new probe

**Severity**: LOW — a regression of the one-byte growth probe is not detected by the existing guard
**Status**: Open
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
(coder fills in)

## Guard proof
(red/green proof; mutation described)

## Known gaps
(none yet)
