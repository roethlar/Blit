# cr-ecm22-5: a source-reported failure left the partial's handle open

**Severity**: MEDIUM — every resume record the source closes failed kept a descriptor open to the session's end; enough of them exhaust descriptors in a transfer that could have survived them
**Status**: Reviewed, found incomplete — `combined-r24` (Codex openreview, 2026-11-22) reviewed `24d4064f..c89b1eb1` — the six `combined-r23` fixes — with `guard_confirmed: true`: "The short staging name, pre-rename sizing, and release of held files after a data-bearing failure all serve that goal." "When the source failed before sending its first block, the control lane reported the failure directly. In both initiator roles I measured one saved baseline still present at sink destruction. Route this branch through `fail_resume_record` too." The remaining work is cr-ecm23-6
**Branch**: — (default-branch mode; fixes land on master)
**Commit**: `bdc472dd`
**Reviewer**: codex / gpt-6.2-astra (as dispatched) / max / frontier, codex-cli 0.171.0 headless one-shot, openreview `combined-r23` over `5d2f8def..24d4064f` (verdict `acceptable_with_changes`, capability_ok and guard_confirmed true); `.review/results/combined-r23.codex.md`. It completes cr-ecm21-4 (`50b416ed`).

## Evidence
"After one block and a failed close, my Linux probe measured zero active resume grants and one descriptor still open for the partial. Repeated source failures can exhaust descriptors during an otherwise recoverable transfer. Route failed closes on both transports through the sink's cleanup."

## Intake
Admitted as the reviewer put it.

## What
- `FsTransferSink::fail_resume_record` (new override): a record the source closed failed lets its partial go — the held handle, its noted baseline, any failure held for it — as a completion does. The partial stays unstamped, one byte longer than its source.
- The data plane's `NeedListSink::fail_resume_record` settles its ledger and then passes the call to the sink it wraps; it answered itself.
- The control lane's failed close goes to the sink as `fail_resume_record`; it reported the file itself.
- The trait's word says so.

## Guard proof
- `sink::tests::a_failed_resume_record_lets_its_partial_go` (Unix): a partial held, then its record failed — no partial held, the file 11 bytes for a 10-byte source.
- `data_plane::tests::a_failed_resume_record_reaches_the_sink_it_wraps`: the wrapped sink is told once.
- `own_incomplete_copy_tests::a_failed_close_on_the_control_lane_reaches_the_sink`: a block and then a failed close over an in-process transport — the sink is told once, and the file is reported failed.
- Mutations, each restored byte for byte: the sink keeps the handle (red); the data plane does not pass it on (red); the control lane reports it itself (red).
- The resume, sink and session tests (293) pass. Windows cross-clippy clean.

## Files changed
- `crates/blit-core/src/remote/transfer/sink.rs` — the trait's word, the override, a test accessor; guard.
- `crates/blit-core/src/transfer_session/data_plane.rs` — `NeedListSink::fail_resume_record`; guard.
- `crates/blit-core/src/transfer_session/mod.rs` — `receive_block_record`; guard.

## Known gaps
- The sink's guard runs on Unix only, where the hold keeps a handle. The data-plane and control-lane guards run everywhere.
