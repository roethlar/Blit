# cr-fix1-2: Shard metadata failures are reported as open failures

**Severity**: LOW — a contained shard member whose stat fails after a successful open is reported with the wrong cause ("cannot open"), misdirecting the user
**Status**: Open
**Branch**: — (default-branch mode; fixes land on master)
**Commit**: (filled after the fix commit)
**Reviewer**: codex / gpt-5.6-sol / xhigh / standard (D-2026-07-31-3 standing codereview; range ed4bc773..b342d636 (review-fix batch), record .review/results/ssc-fix1-range.codex.json)

## Evidence
crates/blit-core/src/remote/transfer/payload.rs:429 — file.metadata()? shares open_member_from_fs's undifferentiated io::Result, whose Err arm at line 476 always emits "source: cannot open".

## Predicted observable failure
When File::open succeeds but metadata retrieval fails, the member is contained but the failure report falsely says it could not be opened instead of identifying the metadata/stat failure, obscuring the actionable cause.

## Reviewer's suggested approach
Preserve the failing stage in the opener error type, or perform the handle metadata query separately, so open and stat errors retain distinct reasons.

## What
(coder fills in)

## Guard proof
(red/green proof; mutation described)

## Known gaps
(none yet)
