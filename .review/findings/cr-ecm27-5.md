# cr-ecm27-5: a sink that only forgets was no longer told of a skip

**Severity**: MEDIUM — a sink that implements `forget_resume` and inherits `skip_record` kept its state on the data plane: the base revision's skip test, run at head, saw no cleanup call
**Status**: Fixed, awaiting review
**Branch**: — (default-branch mode; fixes land on master)
**Commit**: this commit (`Fix cr-ecm27-5`)
**Reviewer**: codex / gpt-6.2-astra (as dispatched) / max / frontier, codex-cli 0.171.0 headless one-shot, openreview `combined-r28` over `e025a3e2..10145127` (verdict `acceptable_with_changes`, capability_ok and guard_confirmed true); `.review/results/combined-r28.codex.md`. It completes cr-ecm26-6 (`1cf8782c`).

## Evidence
"Running the base revision's skip test against head produced zero cleanup calls where it expected one. A sink that implements `forget_resume` and inherits `skip_record` now retains its state on the data plane. Have the trait's default `skip_record` call `forget_resume`, and keep the wrapper's ledger validation and delegation."

## Intake
Admitted as the reviewer put it.

## What
`TransferSink::skip_record`'s default forgets what the sink noted for the file (`forget_resume`) and then reports it failed. `FsTransferSink` inherits it, and its own override is gone. The data plane's wrapper still settles its ledger and hands the skip on.

## Guard proof
- `data_plane::tests::a_skip_reaches_the_sink_it_wraps` (back to a sink that only forgets, as at the base): told once.
- `sink::tests::the_sinks_own_skip_leaves_nothing_noted`: still nothing noted after the sink's own skip.
- Mutation, restored byte for byte: the default skip not forgetting (red, both).
- The resume, sink and session tests (327) pass. Windows cross-clippy clean.

## Files changed
- `crates/blit-core/src/remote/transfer/sink.rs` — the trait's default; `FsTransferSink`'s override removed.
- `crates/blit-core/src/transfer_session/data_plane.rs` — its guard's sink.

## Known gaps
- None known.
