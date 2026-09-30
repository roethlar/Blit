# cr-ssc1-3: Reserve tar-shard needs atomically before writing them

**Severity**: MEDIUM — duplicate concurrent delivery can succeed with nondeterministic contents instead of a protocol violation
**Status**: Open
**Branch**: — (default-branch mode; fixes land on master)
**Commit**: (filled after the fix commit)
**Reviewer**: codex / gpt-5.6-sol / xhigh / standard (D-2026-07-31-3 standing codereview; range f74b0b1a..6bc3d08f, record .review/results/ssc-1-range.codex.json)

## Evidence
crates/blit-core/src/transfer_session/data_plane.rs:2029 — check_shard validates members while they remain Granted and releases the ledger before the inner write at line 2222; crates/blit-core/src/transfer_session/need_ledger.rs:266 then silently settles only entries still Granted.

Verified against master after ssc-3 (2026-09-30) by the orchestrator before admission.

## Predicted observable failure
Two data-plane sockets can concurrently submit the same granted path, or a shard can race a FILE/SKIP record. Both pass validation and write the same destination concurrently; settlement can silently ignore one delivery, letting the session succeed with duplicate accounting and nondeterministic file contents instead of raising a protocol violation.

## Reviewer's suggested approach
After validating the complete shard under one lock, atomically move every unique member into a shard-reserved Active state tied to its lane. Settle only that exact state after the sink returns, and reject duplicate paths within a shard.

## What
(coder fills in)

## Guard proof
(red/green proof of the new guard; mutation described)

## Known gaps
(none yet)
