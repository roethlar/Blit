# cr-ecm40-6: the failed-set encoder called a collapsed set whole

**Severity**: MEDIUM — a list of failures reaching the wire says it is whole with one failure missing. Brought in by cr-ecm39-1.
**Status**: Fixed; awaiting review
**Branch**: — (default-branch mode; fixes land on master)
**Commit**: this commit (`Fix cr-ecm40-6`)
**Reviewer**: codex / gpt-6.2-astra (as dispatched) / max / frontier, codex-cli headless one-shot, openreview `combined-r41` over `d7f5f5e9..b6e6fab9` (verdict `acceptable_with_changes`, capability_ok and guard_confirmed true); `.review/results/combined-r41.codex.md`.

## Evidence
"Before manifest resolution, the public encoder returned one path and `truncated=false` for two failures sharing a display name. After resolution it returned both. Require resolved identities at the encoding boundary or mark the list incomplete when distinct failures collapse in the emitted set."

## Intake
Admitted; the second remedy. The encoder is asked at points that cannot yet name every write site's failure, so it says the list is incomplete when one collapses.

## What
`wire_failed_identities` checks for a write site's failure, not yet named by the manifest, whose text is also a known failure's identity. Such a failure collapses into the known one: two failures, one entry. The list then says it is incomplete. Once named, both are there and the list is whole.

## Guard proof
- `sink::…::an_unnamed_failure_that_collapses_leaves_the_set_incomplete`: a known failure and an unnamed write failure of one spelling. Before naming: one path, incomplete. Named: two, whole.
- Mutation, restored byte for byte: a collapsed set called whole → red.
- Checks:
  - native, Windows and macOS (`CARGO_FEATURE_PURE=1`) clippy are clean;
  - the workspace passes on Linux, with the round's seven fixes: 2,146 tests over 57 binaries.

## Files changed
- `crates/blit-core/src/remote/transfer/sink.rs` — `wire_failed_identities`; guard.

## Known gaps
- None known.
