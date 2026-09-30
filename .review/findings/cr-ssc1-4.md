# cr-ssc1-4: Contain resume-source open failures on the TCP carrier

**Severity**: MEDIUM — one unopenable resume-granted file still ends the whole run on the data plane, against D-2026-09-28-2
**Status**: Open
**Branch**: — (default-branch mode; fixes land on master)
**Commit**: (filled after the fix commit)
**Reviewer**: codex / gpt-5.6-sol / xhigh / standard (D-2026-07-31-3 standing codereview; range f74b0b1a..6bc3d08f, record .review/results/ssc-1-range.codex.json)

## Evidence
crates/blit-core/src/remote/transfer/sink.rs:2222 — the ResumeFile arm propagates ResumeBlockDiff::open with `?`, so it never emits the new SKIP record when the source cannot be opened.

Verified against master after ssc-3 (2026-09-30) by the orchestrator before admission.

## Predicted observable failure
During a resume-enabled data-plane transfer, a file deleted, locked, or denied after the manifest aborts the entire send pipeline and session; unrelated files do not finish, unlike the in-stream carrier which reports the file as skipped.

## Reviewer's suggested approach
Catch ResumeBlockDiff::open failures before any BLOCK record, send `send_skip` with a bounded `source:` reason, and return a failed SinkOutcome so the pipeline continues.

## What
(coder fills in)

## Guard proof
(red/green proof of the new guard; mutation described)

## Known gaps
(none yet)
