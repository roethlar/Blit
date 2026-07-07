# otp-6a — GPT review adjudication

**Slice**: otp-6a (filters on the session), commit `c026692`.
**Reviewer**: gpt-5.5 (codex-cli 0.142.5, read-only sandbox).
**Raw review**: `.review/results/otp-6a.codex.md`.
**Codex verdict**: FAIL — 1 Medium finding.

## F1 (Medium) — filter honored via `scan()` arg, not the universal chokepoint — ACCEPTED

**Codex**: `SessionOpen.filter` is only passed through the `TransferSource::scan`
argument, but that is not a universal chokepoint: `RemoteTransferSource::scan`
ignores it and `FilteredSource::scan` ignores caller-provided filters. Any session
using those source implementations can silently manifest/transfer unfiltered files.
The new test covers only `FsTransferSource`.

**Adjudication: ACCEPTED (real).** Verified against source:

- `crates/blit-core/src/remote/transfer/source.rs:233-261` — `RemoteTransferSource::scan`
  takes `_filter: Option<FileFilter>` and drops it; the doc comment there states
  filtering is the `FilteredSource` decorator's job, not per-source.
- `source.rs:329-379` — `FilteredSource` is documented as "the SINGLE filter
  chokepoint for every src/dst combination"; its `scan` **ignores** the passed
  filter and applies its own stored one.
- Only `FsTransferSource::scan` (`source.rs:61-75`) honors the `scan(filter)` arg.

Why this matters for the session specifically: for **pull**, the SOURCE is the
remote daemon responder, which builds its own `FsTransferSource` from the module
root (`run_responder` → `SourceResponderTarget::Resolve`) — so today the daemon-as-
source path happens to work. But the mechanism is impl-dependent: a session whose
source is a `RemoteTransferSource` (the otp-9 delegated / remote→remote relay) with
`open.filter` set would silently transfer everything. That breaks the "SOURCE scan
honors open.filter" claim and the initiator/verb invariance the plan requires. The
fix is cheap and makes filtering correct-by-construction.

**Fix**: apply the filter through the `FilteredSource` decorator (the universal
chokepoint) rather than the per-impl `scan(filter)` arg. `source_send_half` wraps
the source in `FilteredSource` when `open.filter` is present, then scans through the
wrapper — which applies the filter uniformly regardless of the inner source type.
Fix commit: `0bb27f5`. New guard test `session_filters_via_chokepoint_not_scan_arg`
drives a push session whose source ignores the scan arg (models
`RemoteTransferSource`) and asserts the filter still applies; guard-proved (revert
to scan-arg → all files transfer → FAIL). Gate green: fmt/clippy clean, `cargo test
--workspace` → **1524 passed**, 2 ignored.

## Items codex confirmed clean

- Invalid pull-side peer globs are refused at OPEN (peer-notified) — the validator
  change is correct.
- No bytes move on a refused handshake.
- Test attr count increased by 1 (1522 → 1523).

**reviewer: gpt-5.5**
