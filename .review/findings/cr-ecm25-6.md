# cr-ecm25-6: a file the source skipped left its baseline at the destination

**Severity**: MEDIUM — a resumed file the source skipped before its first block was settled in the ledger, and the sink kept its baseline to the session's end
**Status**: Fixed, awaiting review
**Branch**: — (default-branch mode; fixes land on master)
**Commit**: this commit (`Fix cr-ecm25-6`)
**Reviewer**: codex / gpt-6.2-astra (as dispatched) / max / frontier, codex-cli 0.171.0 headless one-shot, openreview `combined-r26` over `19025839..05ff50d7` (verdict `acceptable_with_changes`, capability_ok and guard_confirmed true); `.review/results/combined-r26.codex.md`. It completes cr-ecm24-8 (`b38564c2`) for skips.

## Evidence
"When the source skipped a granted file before its first block, I measured zero open grants and one retained baseline. Route skips on both carriers through sink cleanup."

## Intake
Admitted as the reviewer put it.

## What
- `TransferSink::forget_resume` (new; does nothing by default): a record closed without the sink being asked. `FsTransferSink` drops everything noted for it — its held handle, its baseline, a failure held for it, its raw name, its hold's mark — and its `fail_resume_record` does the same through it.
- The data plane's `NeedListSink::skip_record` passes the skip on after settling it, and forwards `forget_resume`.
- The control lane closes a skip through `close_skipped_record`: the ledger settles it, then the sink forgets.

## Guard proof
- `sink::tests::a_skipped_record_leaves_nothing_noted_for_it`: a baseline and raw name noted, then forgotten — none left.
- `data_plane::tests::a_skip_reaches_the_sink_it_wraps`: the wrapped sink forgets once.
- `own_incomplete_copy_tests::a_skip_on_the_control_lane_reaches_the_sink`: the sink forgets once, and a second skip is still a violation.
- Mutations, each restored byte for byte: the sink keeping the baseline (red); the data plane not passing a skip on (red); the control lane settling a skip in the ledger alone (red).
- The resume, sink and session tests (315) pass. Windows cross-clippy clean.

## Files changed
- `crates/blit-core/src/remote/transfer/sink.rs` — `forget_resume` in the trait and in `FsTransferSink`, a test accessor; guard.
- `crates/blit-core/src/transfer_session/data_plane.rs` — `NeedListSink::skip_record`, `forget_resume`; guard.
- `crates/blit-core/src/transfer_session/mod.rs` — `close_skipped_record`, its call; guard.

## Known gaps
- None known.
