# d-70-f1-delegated-mirror: remote→remote delegated mirror

**Severity**: Feature (TUI_DESIGN §1 "mirror … between any two endpoints")
**Status**: In progress / pending review
**Branch**: `phase5/a1`
**Commit**: `0b98666`

## What

Extends the remote→remote F1 trigger (copy in d-68, live progress
in d-69) with **mirror** — the destination daemon deletes entries
absent from the source. Move stays a follow-up (it needs a
remote-source delete RPC).

## Approach

- `plan_f1_delegated` now takes `confirmed`. Mirror is destructive
  (`kind.is_destructive()`), so unconfirmed → `NeedsConfirm`
  (opens the trigger's y/N gate, same path as the local→remote
  push mirror in d-65); `y` re-runs with `confirmed = true` →
  delegated launch. Copy still launches straight away. Move →
  `Rejected("move isn't supported yet")`.
- `build_delegated_execution(src, dst, kind)` extracted from
  `spawn_f1_delegated_pull` so the mirror option is unit-pinned
  (cf. the d-65 push builder the reviewer asked for). Options come
  from `f3_pull_options(kind)`.
- `begin_delegated` + `spawn_f1_delegated_pull` take a `kind`; the
  footer verb reads "mirroring/mirrored" for a delegated mirror
  ("delegating/delegated" stays for copy).

## Mirror safety — why `require_complete_scan` is OFF

This is the d-65 question, and the answer differs for delegated.
The CLI's delegated path passes `require_complete_scan = false`
for copy **and** mirror (`crates/blit-cli/src/transfers/mod.rs`
`RemoteToRemoteDelegated` → `run_remote_to_remote_direct(.., mirror,
false)`); it only forces a complete scan for **move**. Rationale:
d-65's guard protects against a partial **client-side** scan
driving a purge, but in a delegated transfer the **daemons**
enumerate — the client (TUI/CLI) isn't in the scan path. So d-70
matches the CLI exactly: delegated mirror → `mirror_mode: true`,
`require_complete_scan: false`. `build_delegated_execution` is
tested to pin this.

## Files changed

- `crates/blit-tui/src/main.rs`: `build_delegated_execution`;
  `plan_f1_delegated` (`confirmed` + mirror confirm, move reject);
  `spawn_f1_delegated_pull` + `begin_delegated` take `kind`; verb
  helpers map delegated mirror → "mirroring"; 3 tests (1 replaced).
- `crates/blit-tui/src/f1push.rs`: `begin_delegated(label, kind)`.
- `crates/blit-tui/src/f1trigger.rs`: module-doc refresh
  (delegated copy + mirror wired; move a follow-up).

## Tests

569 total (net +2 vs d-69):

- `build_delegated_execution_mirror_options` — mirror →
  `mirror_mode` on, `require_complete_scan` off; copy → neither.
- `plan_f1_trigger_remote_to_remote_mirror_confirms_then_launches`
  — unconfirmed → `NeedsConfirm` (no launch); confirmed →
  delegated `Running { kind: Mirror }`.
- `plan_f1_trigger_remote_to_remote_move_rejected` — move →
  `Rejected("move isn't supported")` (replaces the old combined
  mirror+move reject test, since mirror is now wired).

## Known gaps / follow-ups

1. remote→remote **move** delegation (needs a remote-source delete
   RPC after the copy).
2. **detach + F2 visibility** (needs multi-daemon F2).

## Reviewer comments

(empty — pending grade)
