# d-58-f1-trigger: `t` trigger-transfer modal on F1

**Severity**: Feature (designed — TUI_DESIGN §5.1)
**Status**: In progress / pending review
**Branch**: `phase5/a1`
**Commit**: `debb2c1`

## What

TUI_DESIGN §5.1's F1 detail block advertises three actions:
`[enter] browse  [t] trigger transfer  [d] diagnostics`. Only
`enter` (browse, d-47) existed. d-58 adds `t` — a
trigger-transfer modal launched from a daemon row.

This **first slice** covers the remote→local **pull**: `t`
opens a source/dest modal, and on commit it hands off to the
*verified* F3 pull machine and jumps to F3 to watch. Push,
mirror, and remote→remote (delegated) triggers are explicit
follow-ups (see Known gaps).

## Approach

### A modal that delegates to the verified pull machine

The unique cost of a transfer launcher is collecting the two
endpoints; the *execution* is already solved by the F3 pull
machine (d-35…d-57: progress, throughput, auto-hide TTL,
generation guard). So d-58 adds **only** the field-collection
modal and delegates execution — no new spawn path, reply
channel, progress UI, or display.

`F1TriggerState` (new module, matching the `f3pull`/`f3del`
pattern) holds `Editing { source, dest, focus }`:

- `t` on a daemon row → `begin(prefill)` where `prefill =
  "<host:port>:/"` (the operator just appends a module path);
  focus starts on the **dest** field (the always-empty one).
- `Tab` toggles focus; chars / Backspace edit the focused
  field; `Esc` cancels.
- `Enter` → `take()` yields the trimmed `(source, dest)` (only
  when both are non-blank — a blank field keeps the modal
  open). The keystroke handler parses `source` to a
  `RemoteEndpoint`; on success it calls
  `F3PullState::start_pull(source, dest)` (a `Copy`-kind pull,
  bypassing the F3 prompt) and sets `current_screen = F3`.

The input router gets one new guard: while
`f1_trigger.is_editing()` on F1, keystrokes route to
`handle_f1_trigger_keystroke` before the dispatcher (so `t`
etc. are text, not actions) — same input-mode pattern as the
F3 filter / pull-dest prompts.

### Decoupled render

`render_into` gains an `Option<TriggerPrompt>`; when present it
replaces the discovery footer line with `trigger src: … dst: …
(Tab switch · Enter pull · Esc cancel)`, the focused field bold
+ cursor. `TriggerPrompt` is a screens-side struct (plain
strings + a `source_focused` bool) built by a `main.rs` bridge,
so `screens/f1.rs` never sees `F1TriggerState`.

## Files changed

- `crates/blit-tui/src/f1trigger.rs` (new): the modal state +
  unit tests.
- `crates/blit-tui/src/main.rs`: `mod f1trigger`; `f1_trigger`
  AppState field + inits; `UserAction::F1TriggerBegin` + `t`
  mapping + F1 dispatch; input-routing guard +
  `handle_f1_trigger_keystroke`; `f1_trigger_prompt` bridge;
  render call.
- `crates/blit-tui/src/screens/f1.rs`: `TriggerPrompt` +
  `render_trigger`; `render_into` param.
- `crates/blit-tui/src/help.rs`: `t` keymap row; modal 46→47;
  test backends bumped.

## Tests

524 total (was 510):

f1trigger.rs (7): idle; begin prefills source + focuses dest;
typing edits focused field + Tab toggles; pop_char; cancel;
take yields trimmed pair + closes; take keeps open on a blank
field; begin no-op while open.

main.rs (6): `t` → F1TriggerBegin; Esc cancels; Tab toggles
focus; chars edit dest; **Enter launches the pull + jumps to
F3** (tokio test — spawn needs a reactor); `?`/Ctrl-c/F-keys
bubble.

The pull execution itself is the verified F3 path (already
covered); d-58's tests cover the modal + the hand-off.

## Known gaps

1. **Pull only.** Push (local→remote), mirror, move, and
   remote→remote (delegated) triggers aren't wired yet — the
   modal always starts a `Copy` pull. The design's general
   launcher (copy/mirror across any endpoint pair) is a
   follow-up; this slice establishes the modal + the
   remote→local path.
2. **No inline parse-error feedback.** A source string that
   doesn't parse as a `RemoteEndpoint` is silently dropped on
   Enter (modal closes, no launch). With the `host:port:/`
   prefill a parse failure is unlikely, but an inline error
   line is a follow-up.
3. **Delegates to F3 pull state.** If a pull is already running
   on F3, `start_pull` no-ops (the trigger is ignored). Rare;
   acceptable for now.

## Out of scope

- F1 `d` diagnostics (the third §5.1 action).
- Push / mirror / remote→remote triggers.

## Reviewer comments

(empty — pending grade)
