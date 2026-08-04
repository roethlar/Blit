# cr-c1-2: SelectEndpoint leaves the previous endpoint's browse state in place

**Severity**: LOW — real incorrect-display behavior in the model API, but slice 1 registers only Local by default and daemon browse is a typed stub, so practical reachability is minimal.
**Status**: Open
**Branch**: —
**Commit**: (pending)

## Evidence
`crates/blit-console-core/src/model.rs:118-126` — the `SelectEndpoint` arm sets `selected` and clears `last_error` but does not touch `listing`, `current_path`, or `loading`, while the model holds a single shared pane state (`model.rs:40-46`) and the Msg doc (`model.rs:16-17`) describes selection as bringing that endpoint's pane into focus. Trigger: `add_endpoint(Daemon…)`, `NavigateTo(local path)`, `ListingLoaded`, then `SelectEndpoint(daemon)` → the daemon is selected while `listing()`/`current_path()` still describe the local filesystem. If selection changes while `loading == true` and the host abandons the old effect, `is_loading()` stays true forever.

## Predicted observable failure
A face rendering straight from the model shows endpoint A's directory contents attributed to endpoint B after a switch, or a permanent loading spinner. No existing test exercises selection change after a completed or in-flight browse.

## What
Endpoint selection does not reset (or re-load) the single shared browse pane, so stale cross-endpoint state is displayed as current.

## Approach
(pending — coder completes when work starts)

## Files changed
(pending)

## Guard proof
(pending — a selection-after-load test that FAILS pre-fix, PASSES post-fix)

## Coder dispute
None.

## Known gaps
Overlaps cr-c1-1's correlation concern (both live in the same update arms); fixed as separate commits per the one-finding-one-commit rule.

## Reviewer comments
Reviewer: claude / claude-fable-5 / high / standard — claude-cli 2.1.221; range `3a602a94..243f4d4d`, SHAs pin-verified; capability_ok=true; 2026-08-04T~04:30Z; record `.review/results/c1-1-range.claude.json` (raw envelope `logs/c1-scaffold-codereview-claude-fable-5.json`). Dispatch note: 8 early Bash denials from a local `rtk` command-rewriting wrapper outside the launch grant; reviewer recovered on plain `git` — environment quirk recorded per owner ruling 2026-07-23, not an invalidation.
Generation verdict: findings (2) — this finding admitted at intake; see also cr-c1-1.
