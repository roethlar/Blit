# cr-pfc4-1: Delegated pulls discard the destination failure report

**Severity**: HIGH — a delegated remote→remote mirror with contained
destination failures returns clean success; the requested file is
silently absent.
**Status**: Open — fix queued behind cr-pfc3-1/-2 (ordering only)
**Branch**: — (default-branch mode)
**Commit**: (filled at landing)

Reviewer provenance (generation pass): codex / gpt-5.6-sol / xhigh /
standard; codex-cli 0.146.0; range `72079331..bdc3e4a4`
(`.review/results/pfc-4-range.codex.json`).

## Evidence
`blit-daemon/src/service/delegated_pull.rs:495-510` converts the
authoritative `TransferSummary` into `DelegatedPullSummary` WITHOUT
`files_failed`/`failures`; `proto/blit.proto:645-658` has no fields for
them; `blit-app/src/transfers/remote.rs:613-642` treats any received
summary as success.

## Predicted observable failure
Delegated remote→remote mirror with a contained write failure at the
destination daemon: the initiating CLI/TUI reports completion with no
failed path or reason.

## What
pfc-4's single-construction-site propagation holds for direct
topologies, but the delegated topology re-encodes into a second summary
message that was not extended.

## Approach
(planned) Extend `DelegatedPullSummary` with `files_failed` (exact) +
`failures` (same 64-entry sender cap; same encoded-byte budget as
cr-pfc4-2), copied from the `TransferSummary`; the consumer surfaces
them on the same `LocalMirrorSummary`-shaped path pfc-5 renders.
Within contract v6 (unreleased — v5 is the last shipped contract; no
second bump needed, recorded here).

## Files changed
(filled with the fix commit)

## Guard proof
(planned) Delegated round-trip test: a contained failure at the
destination daemon arrives in the initiator's summary with count and
path; red when the copy-through is dropped.

## Coder dispute (if any)
None.

## Known gaps
Partial-failure *rendering* for delegated results is pfc-5's, with the
rest of the CLI surfacing.

## Reviewer comments
(pending per-finding verification after the fix)
