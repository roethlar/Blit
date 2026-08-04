# cr-c1-1: Browse completions carry no request correlation — stale results overwrite newer state

**Severity**: MEDIUM — wrong-state bug in the console core's central contract (the brain both faces must trust); not yet user-reachable (no face ships in slice 1, local browse is synchronous).
**Status**: In progress — fix committed, external verification pending
**Branch**: —
**Commit**: (this fix commit; SHA filled in at the verification record)

## Evidence
`crates/blit-console-core/src/model.rs:138-150` — `Msg::ListingLoaded` unconditionally sets `current_path`/`listing` and clears `last_error`; `Msg::ListingFailed` unconditionally clears the listing and sets the error (discarding the failed path via `..`). Neither message carries the `EndpointId` or any request token that `Effect::Browse` (`model.rs:131`) issued, so the host cannot correlate a completion to its request. Trigger, per the module's own stated contract (`model.rs:1-7`, host executes effects asynchronously): `NavigateTo(A)`, `NavigateTo(B)`, A's result arrives after B's → model shows A's listing at `current_path=B`; a slow failure for A wipes B's fresh listing. Cross-endpoint shape: `SelectEndpoint(daemon)` while a local browse is in flight → the local listing lands in the daemon pane.

## Predicted observable failure
In the first async face (the egui GUI this crate exists to serve), rapid navigation or endpoint switching displays the wrong directory's contents or the wrong error — no crash, no test failure. Existing tests (`model.rs:206-245`) only cover the single matched-request case; nothing pins out-of-order delivery.

## What
The Elm-ish update loop has no generation/request token, so any out-of-order completion — guaranteed to occur once effects run concurrently — silently wins over newer state.

## Approach
Generation tagging, per the reviewer's better_approach: `Model` gains a `browse_generation` counter incremented on every issued `Effect::Browse`; the effect carries the generation and both completion messages (`Msg::ListingLoaded`/`ListingFailed`) echo it back. Both completion arms no-op unless the message is the in-flight latest (`!loading || generation != browse_generation` → drop). This closes the root cause — an uncorrelated completion channel — rather than any single stale-write symptom, and covers the cross-endpoint shape too (a new `NavigateTo` after `SelectEndpoint` issues a higher generation, so the old endpoint's late completion is dropped).

## Files changed
- `crates/blit-console-core/src/model.rs:21-57` — `generation` field on `Msg::ListingLoaded`/`ListingFailed` and `Effect::Browse`
- `crates/blit-console-core/src/model.rs:60-115` — `Model.browse_generation` field + init
- `crates/blit-console-core/src/model.rs:150-196` — `NavigateTo` issues the generation; completion arms drop stale/unsolicited messages
- `crates/blit-console-core/src/model.rs` tests — existing completion tests carry `generation: 1`; 3 new guards below

## Guard proof
- `model::tests::stale_loaded_is_dropped_in_favour_of_newer_request` and `stale_failure_is_dropped_in_favour_of_newer_request` — mutation guard: disabling both completion guards (`if !model.loading || generation != …` → `if false`) turns both tests red; restored, green. `unsolicited_completion_without_browse_is_dropped` pins the no-browse-in-flight case.

## Coder dispute
None.

## Known gaps
None.

## Reviewer comments
Reviewer: claude / claude-fable-5 / high / standard — claude-cli 2.1.221; range `3a602a94..243f4d4d`, SHAs pin-verified; capability_ok=true; 2026-08-04T~04:30Z; record `.review/results/c1-1-range.claude.json` (raw envelope `logs/c1-scaffold-codereview-claude-fable-5.json`). Dispatch note: 8 early Bash denials from a local `rtk` command-rewriting wrapper outside the launch grant; reviewer recovered on plain `git` — environment quirk recorded per owner ruling 2026-07-23, not an invalidation.
Generation verdict: findings (2) — this finding admitted at intake; see also cr-c1-2.
