# cr-ssc3-1: Resume success skips the post-read source-size check

**Severity**: HIGH — a resumed file that grew after the scan is finalised short and reported successful; with `move --resume` the omitted tail is deleted at the source (data loss under a "success" exit)
**Status**: Verified
**Branch**: — (default-branch mode; fixes land on master)
**Commit**: `e4405156`
**Reviewer**: codex / gpt-5.6-sol / xhigh / standard (D-2026-07-31-3 standing codereview; range 905ddb37..b70ef03a, record .review/results/ssc-3-range.codex.json)

## Evidence
crates/blit-core/src/remote/transfer/resume_diff.rs:102 — `next_event` reads only to the manifest size and returns `Ok(None)` at line 147 without re-statting the opened handle; both callers interpret `None` as `ok=true` (`transfer_session/mod.rs:3348-3361`, `remote/transfer/sink.rs:2307-2323`), after which `sink.rs:1016` truncates and stamps the destination at that stale size.

## Predicted observable failure
If a source file grows after its manifest entry is scanned, a resumed transfer ignores the appended tail, reports the file successfully resumed, and leaves the destination shorter than the source. With `move --resume`, `files_failed` remains zero, so source deletion can permanently discard the omitted tail.

## Reviewer's suggested approach
Validate the opened source handle's length before starting the resume diff and again exactly once before returning `None`. Convert a mismatch into the same contained changed-size outcome used by whole-file records, and add both-carrier guards for growth before and during a resume.

## What
`ResumeBlockDiff::open` (`remote/transfer/resume_diff.rs`) now stats the opened handle and fails with the `source:` changed-size (or cannot-read-metadata) reason when it does not match `header.size`; both carriers turn that into the pre-block skip via the new `ResumeBlockDiff::skip_reason` (in-stream `send_resume_block_records`; TCP `DataPlaneSink` ResumeFile arm). `next_event` re-stats the same handle exactly once (`end_checked`) before returning `None`; a mismatch is the changed-size error, which both carriers already close as a FAILED `BlockComplete` — the partial stays unstamped and the file is reported, so `move --resume`'s source-delete gate refuses.

## Guard proof
`{in_stream,data_plane}_resume_growth_before_the_diff_is_skipped` (`assert_resume_pre_diff_skip` with `Fault::DeclaresLen`: skipped, partial byte-identical and unstamped, move gate refuses) and `{in_stream,data_plane}_resume_growth_during_the_diff_is_reported_and_unstamped` (`Fault::DriftsAfterBody`: every block lands, record FAILED with the drift reason, not stamped with the source mtime, move gate refuses, other file lands). Mutations: pre-diff check disabled → the before-guard FAILED; post-diff check disabled → the during-guard FAILED (`scratchpad/cr-ssc-mutations.txt`); restored → green, `resume_diff` unit tests and `transfer_session_roles` resume tests green.

## Known gaps
A file that shrinks during the diff was already the short-read failure (ssc-3); growth during the diff is now caught by the post-diff re-stat rather than by the read (the read stops at the manifest size by design).
