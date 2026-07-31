# cr-pfc4-1: Delegated pulls discard the destination failure report

**Severity**: HIGH — a delegated remote→remote mirror with contained
destination failures returns clean success; the requested file is
silently absent.
**Status**: In progress — fixed, awaiting per-finding reviewer verdict
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

As built: `DelegatedPullSummary` gains `files_failed = 7` + `failures = 8` (reusing `FileFailure`, same cap semantics); the ONE re-encode extracted to `delegated_summary_from_session` (copies verbatim, no second capping path) with the v6 doc line naming it as the second site any future summary field must reach; `DelegatedPullOutcome::files_failed()/contained_failures()` consumer accessors (rendering stays pfc-5); detach-path literal explicitly zeroed with reason. Guards red three ways under the copy-through drop (unit round-trip 0≠70; consumer surface 0≠70; real two-daemon delegated e2e 0≠1 with the report's path+reason pinned and siblings landing byte-identical). blit-core 466/0, daemon 176/0, app 59/0, workspace 1663/0.
