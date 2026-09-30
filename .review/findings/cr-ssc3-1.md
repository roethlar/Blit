# cr-ssc3-1: Resume success skips the post-read source-size check

**Severity**: HIGH — a resumed file that grew after the scan is finalised short and reported successful; with `move --resume` the omitted tail is deleted at the source (data loss under a "success" exit)
**Status**: Open
**Branch**: — (default-branch mode; fixes land on master)
**Commit**: (filled after the fix commit)
**Reviewer**: codex / gpt-5.6-sol / xhigh / standard (D-2026-07-31-3 standing codereview; range 905ddb37..b70ef03a, record .review/results/ssc-3-range.codex.json)

## Evidence
crates/blit-core/src/remote/transfer/resume_diff.rs:102 — `next_event` reads only to the manifest size and returns `Ok(None)` at line 147 without re-statting the opened handle; both callers interpret `None` as `ok=true` (`transfer_session/mod.rs:3348-3361`, `remote/transfer/sink.rs:2307-2323`), after which `sink.rs:1016` truncates and stamps the destination at that stale size.

## Predicted observable failure
If a source file grows after its manifest entry is scanned, a resumed transfer ignores the appended tail, reports the file successfully resumed, and leaves the destination shorter than the source. With `move --resume`, `files_failed` remains zero, so source deletion can permanently discard the omitted tail.

## Reviewer's suggested approach
Validate the opened source handle's length before starting the resume diff and again exactly once before returning `None`. Convert a mismatch into the same contained changed-size outcome used by whole-file records, and add both-carrier guards for growth before and during a resume.

## What
(coder fills in)

## Guard proof
(red/green proof; mutation described)

## Known gaps
(none yet)
