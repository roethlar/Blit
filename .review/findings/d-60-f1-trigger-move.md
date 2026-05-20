# d-60-f1-trigger-move: copy/mirror/move cycle in the F1 trigger

**Severity**: Feature (designed — TUI_DESIGN §1 / §5)
**Status**: In progress / pending review
**Branch**: `phase5/a1`
**Commit**: `e3a8836`

## What

d-59 gave the F1 trigger a copy ⇄ mirror toggle. TUI_DESIGN §1
frames the launcher as "copy / mirror / move … between any two
endpoints". d-60 completes the kind matrix: a copy → mirror →
move cycle.

## Approach

- d-59's `mirror: bool` becomes `kind: f3pull::PullKind`
  (Copy/Mirror/Move), reusing the verified enum. `cycle_kind(fwd)`
  replaces `toggle_mirror`: Up cycles Copy → Mirror → Move → Copy,
  Down reverses. `take()` returns the `PullKind`.
- The Enter handler matches the kind:
  - **Copy** → `start_pull` (direct launch + spawn), as d-58.
  - **Mirror / Move** → route through the *verified* F3
    destructive confirm gate (`begin_mirror`/`begin_move` → push
    the dest chars → `begin_run` lands in `Confirm`, no spawn) →
    jump to F3, where the operator confirms y/N. No new
    execution path — d-55/57's F3 machinery owns it.
- **Move data-loss guard:** a move deletes the remote source,
  so — like F3 `v` (d-57 R2) — a module-root source is refused
  up front via `is_deletable_remote_path` (the daemon rejects
  empty/root purge paths; without the gate a typed `nas:/mod`
  move would copy the whole module then fail the source delete).
  Mirror writes only locally, so it has no such gate.
- Render: the prompt tag shows `[copy]` (green) / `[mirror]` /
  `[move]` (red — destructive); the hint reads `↑↓
  copy/mirror/move`. `TriggerPrompt` carries a `mode` str + a
  `destructive` bool, so `screens/f1.rs` stays decoupled from
  `PullKind`.

## Files changed

- `crates/blit-tui/src/f1trigger.rs`: `mirror: bool` →
  `kind: PullKind`; `cycle_kind`; `take` returns the kind; doc.
- `crates/blit-tui/src/main.rs`: Up/Down → `cycle_kind`; Enter
  matches Copy vs Mirror/Move (move gated by
  `is_deletable_remote_path`); bridge maps kind → `mode` /
  `destructive`.
- `crates/blit-tui/src/screens/f1.rs`: `TriggerPrompt.mode` /
  `.destructive`; tag + hint.

## Tests

528 total (was 526):

f1trigger.rs: begin starts Copy;
`cycle_kind_advances_copy_mirror_move_and_take_reports_it`
(Up cycles, Down reverses, take reports).

main.rs: `..._mirror_routes_to_f3_confirm` (Up→mirror→confirm
gate, no direct launch); `..._move_routes_to_f3_confirm`
(Up×2→move→confirm); `..._move_rejects_module_root_source` (a
module-root move source is gated — no confirm, stays on F1);
the copy-path Enter test still covers the direct launch.

## Known gaps

1. **Push / remote→remote still pending** (d-58 gap #1) — the
   trigger remains remote→local; the source must parse as a
   remote endpoint. Local→remote push and remote→remote
   (delegated) are the remaining capability gaps and need new
   execution paths.
2. **No inline parse-error feedback** (d-58 gap #2).

## Out of scope

- Push / remote→remote triggers; F1 `d` diagnostics.

## Reviewer comments

(empty — pending grade)
