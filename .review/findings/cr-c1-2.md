# cr-c1-2: SelectEndpoint leaves the previous endpoint's browse state in place

**Severity**: LOW — real incorrect-display behavior in the model API, but slice 1 registers only Local by default and daemon browse is a typed stub, so practical reachability is minimal.
**Status**: Verified — codex accepted r2 (owner-ordered dispatch, D-2026-08-04-4's explicit-order carve-out)
**Branch**: —
**Commit**: `22261bf5`

## Evidence
`crates/blit-console-core/src/model.rs:118-126` — the `SelectEndpoint` arm sets `selected` and clears `last_error` but does not touch `listing`, `current_path`, or `loading`, while the model holds a single shared pane state (`model.rs:40-46`) and the Msg doc (`model.rs:16-17`) describes selection as bringing that endpoint's pane into focus. Trigger: `add_endpoint(Daemon…)`, `NavigateTo(local path)`, `ListingLoaded`, then `SelectEndpoint(daemon)` → the daemon is selected while `listing()`/`current_path()` still describe the local filesystem. If selection changes while `loading == true` and the host abandons the old effect, `is_loading()` stays true forever.

## Predicted observable failure
A face rendering straight from the model shows endpoint A's directory contents attributed to endpoint B after a switch, or a permanent loading spinner. No existing test exercises selection change after a completed or in-flight browse.

## What
Endpoint selection does not reset (or re-load) the single shared browse pane, so stale cross-endpoint state is displayed as current.

## Approach
The reviewer's first option, chosen because it also closes the residual shape the cr-c1-1 verifier noted (a bare `SelectEndpoint` not followed by `NavigateTo` left `loading` and the generation untouched): a successful `SelectEndpoint` now owns the pane — it clears the listing, resets `current_path` to `/`, and immediately issues a fresh `Effect::Browse` for the new endpoint's root. Issuing the browse bumps `browse_generation`, so any completion still in flight from the old endpoint is dropped as stale by cr-c1-1's guards rather than landing in the new pane; `loading` always tracks the switch's own browse, so it cannot stick. Unknown-endpoint selection is unchanged (error, no effect).

## Files changed
- `crates/blit-console-core/src/model.rs:137-161` — `SelectEndpoint` arm resets pane state and emits `Effect::Browse` for the new endpoint's root
- `crates/blit-console-core/src/model.rs` tests — `select_registered_daemon_clears_error` updated for the emitted effect; new `select_resets_pane_and_replaces_in_flight_browse` pins reset, stale-drop, and load resolution across a switch

## Guard proof
- `model::tests::select_resets_pane_and_replaces_in_flight_browse` and the updated `select_registered_daemon_clears_error` — mutation guard: reverting the arm to the pre-fix body (select + clear error, no reset, no effect) turns both tests red; restored, green (16/16 crate suite).

## Coder dispute
None.

## Known gaps
Overlaps cr-c1-1's correlation concern (both live in the same update arms); fixed as separate commits per the one-finding-one-commit rule.

## Reviewer comments
Reviewer: claude / claude-fable-5 / high / standard — claude-cli 2.1.221; range `3a602a94..243f4d4d`, SHAs pin-verified; capability_ok=true; 2026-08-04T~04:30Z; record `.review/results/c1-1-range.claude.json` (raw envelope `logs/c1-scaffold-codereview-claude-fable-5.json`). Dispatch note: 8 early Bash denials from a local `rtk` command-rewriting wrapper outside the launch grant; reviewer recovered on plain `git` — environment quirk recorded per owner ruling 2026-07-23, not an invalidation.
Generation verdict: findings (2) — this finding admitted at intake; see also cr-c1-1.

**r1 (verification, 2026-08-04T~06:10Z): INVALID — transport failure, contested.** Reviewer: claude / claude-fable-5 / high / standard — claude-cli 2.1.221; base `824b8db3` head `22261bf5` pin-verified; guard_confirmed=false; capability_ok=false; record `.review/results/cr-c1-2.claude.json`. The reviewer session had a blanket execution denial (direct `git`/`cargo` refused; child-subagent fallback failed), so no guard proof could run; its read-only inspection of the fix and tests was favorable but is not a verdict. Not auto-retried (D-2026-07-23-7). Contested record and owner options: `.review/cr-c1-2.contested.md`.

**r2 (verification, 2026-08-04T~06:40Z): ACCEPTED.** Reviewer: codex / gpt-5.6-sol / xhigh / standard — codex-cli 0.146.0; owner-ordered dispatch under D-2026-08-04-4's explicit-order carve-out; base `824b8db3` head `22261bf5` pin-verified; guard_confirmed=true; capability_ok=true; record `.review/results/cr-c1-2.codex.json` (raw events `logs/cr-c1-2-verify-codex.jsonl`). Reviewer independently re-ran the mutation guard in its own worktree at the head SHA: pre-fix arm made both required guards fail; restoration passed all 16 crate tests, no adjacent regression. Supersedes the r1 local-closure status — the finding is now externally verified.
