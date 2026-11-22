# cr-ecm23-6: a record failed before its first block was reported without the sink

**Severity**: MEDIUM — a resume record the source closed failed before sending a block left what the sink had noted for the file (its hash baseline) to the session's end
**Status**: Verified — `combined-r25` (Codex openreview, 2026-11-22) reviewed `c89b1eb1..19025839` — the six `combined-r24` fixes — with `guard_confirmed: true`: "The effective-user ownership check, decoding the Unix mode from the wire header, learning the staged file's identity before rename, and sending zero-block failures to the sink are all sound." What a closed record leaves beside its baseline is cr-ecm24-8's
**Branch**: — (default-branch mode; fixes land on master)
**Commit**: `a3b73d30`
**Reviewer**: codex / gpt-6.2-astra (as dispatched) / max / frontier, codex-cli 0.171.0 headless one-shot, openreview `combined-r24` over `24d4064f..c89b1eb1` (verdict `acceptable_with_changes`, capability_ok and guard_confirmed true); `.review/results/combined-r24.codex.md`. It completes cr-ecm22-5 (`bdc472dd`).

## Evidence
"When the source failed before sending its first block, the control lane reported the failure directly. In both initiator roles I measured one saved baseline still present at sink destruction. Route this branch through `fail_resume_record` too."

## Intake
Admitted as the reviewer put it. On the data plane such a close already reaches the sink: the pipeline calls `fail_resume_record`, which `NeedListSink` passes on (cr-ecm22-5).

## What
The control lane closes a resume record that no block came before through `close_zero_block_record`: finalized when the source says it is complete; otherwise the sink's `fail_resume_record` reports the file and lets go of what it noted for it.

## Guard proof
- `own_incomplete_copy_tests::a_failed_close_with_no_block_reaches_the_sink`: a failed close with no block — the sink is told once and the file reported failed.
- Mutation, restored byte for byte: the failure reported without the sink (red).
- The resume, sink and session tests (298) pass. Windows cross-clippy clean.

## Files changed
- `crates/blit-core/src/transfer_session/mod.rs` — `close_zero_block_record`, its call; guard.

## Known gaps
- None known.
