# cr-ecm26-6: the sink's own skip kept the file's baseline

**Severity**: MEDIUM — `FsTransferSink::skip_record`, called directly as the public receive pipeline calls it, left the file's recorded baseline; only the session's wrappers cleaned up
**Status**: Reviewed, found incomplete — `combined-r28` (Codex openreview, 2026-11-22) reviewed `e025a3e2..10145127` — the six `combined-r27` fixes — with `guard_confirmed: true`. "Running the base revision's skip test against head produced zero cleanup calls where it expected one. A sink that implements `forget_resume` and inherits `skip_record` now retains its state on the data plane. Have the trait's default `skip_record` call `forget_resume`, and keep the wrapper's ledger validation and delegation." The remaining work is cr-ecm27-5
**Branch**: — (default-branch mode; fixes land on master)
**Commit**: `1cf8782c`
**Reviewer**: codex / gpt-6.2-astra (as dispatched) / max / frontier, codex-cli 0.171.0 headless one-shot, openreview `combined-r27` over `05ff50d7..e025a3e2` (verdict `acceptable_with_changes`, capability_ok and guard_confirmed true); `.review/results/combined-r27.codex.md`. It completes cr-ecm25-6 (`c913ad4f`).

## Evidence
"Calling `skip_record` directly on `FsTransferSink` left one recorded baseline where the test expected zero. Both session wrappers perform cleanup, but the public receive pipeline can call the sink directly. Have `skip_record` perform cleanup and let wrappers delegate after validating the ledger transition."

## Intake
Admitted as the reviewer put it.

## What
`FsTransferSink::skip_record` forgets what it noted for the file (`forget_resume`) before it reports the file. The data plane's `NeedListSink::skip_record` settles its ledger and then hands the skip on to the sink it wraps; it no longer cleans up for it. The control lane still settles the ledger and has the sink forget (`close_skipped_record`), since it reports the failure itself.

## Guard proof
- `sink::tests::the_sinks_own_skip_leaves_nothing_noted`: a baseline noted, then the sink's own skip — the file failed, no baseline left.
- `data_plane::tests::a_skip_reaches_the_sink_it_wraps` (now counting the wrapped sink's skip): told once.
- Mutations, each restored byte for byte: the sink's own skip keeping what it noted (red); the data plane not handing the skip on (red).
- The resume, sink and session tests (322) pass. Windows cross-clippy clean.

## Files changed
- `crates/blit-core/src/remote/transfer/sink.rs` — `FsTransferSink::skip_record`; guard.
- `crates/blit-core/src/transfer_session/data_plane.rs` — `NeedListSink::skip_record`; its guard updated.

## Known gaps
- None known.
