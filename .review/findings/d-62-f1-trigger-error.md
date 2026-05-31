# d-62-f1-trigger-error: inline validation feedback in the trigger

**Severity**: Feature (closes d-58 known gap #2)
**Status**: In progress / pending review (round 2)
**Branch**: `phase5/a1`
**Commit**: `0b47a72` (round 1: `f48a65e`)

## What

The F1 trigger modal (d-58…d-61) silently closed on a bad commit
— a typo'd source, an unsupported destination, or a not-yet-wired
combination just dropped with no feedback, and the operator had
to re-open with `t` and guess what was wrong. d-62 adds inline
validation feedback: on a failed commit the modal stays open and
shows the reason.

## Approach

### `peek` / `close` / `set_error` replace `take`

`take()` (read + close) is split:

- `peek() -> Option<(src, dest, kind)>` reads the trimmed fields
  **without** closing (`None` when a field is blank → stay open
  silently, as before).
- `close()` closes after a successful launch.
- `set_error(msg)` records a message and keeps the modal open;
  `Editing` gains an `error: Option<String>` field, cleared on
  the next edit (push/pop/tab/cycle) so stale feedback never
  lingers.

### `plan_f1_trigger` — one place that validates + launches

The dispatcher's classify-and-launch logic (which had grown across
the Enter arm over d-58…d-61) moved into a `plan_f1_trigger(app,
src, dest, kind) -> Result<(), String>` helper. It returns
`Ok(())` when a transfer started (caller `close()`s) or
`Err(message)` with a human reason (caller `set_error`s). The
Enter handler is now just:

```rust
if let Some((src, dest, kind)) = app.f1_trigger.peek() {
    match plan_f1_trigger(app, &src, &dest, kind) {
        Ok(()) => app.f1_trigger.close(),
        Err(msg) => app.f1_trigger.set_error(msg),
    }
}
```

The validation (unchanged semantics from d-61 R3, just centralized
+ given messages):

- source via `parse_transfer_endpoint`: `Err` → "invalid source: …"
- remote source: Copy → `start_pull` (or "a transfer is already in
  flight"); Mirror/Move → confirm gate (Move module-root →
  "cannot move a module root")
- local source: non-Copy → "push supports copy only (mirror/move
  not yet)"; dest must parse remote ("push destination must be
  remote …") and pass `ensure_remote_destination_supported`
  ("destination needs a module (host:/module/)"); "a push is
  already running" if busy.

### Render

The prompt's trailing hint is replaced by `⚠ <message>` in red
when an error is set; otherwise the usual key hint shows.
`TriggerPrompt` carries `error: Option<String>` (bridged), so
`screens/f1.rs` stays decoupled.

## Files changed

- `crates/blit-tui/src/f1trigger.rs`: `error` field; `peek` /
  `close` / `set_error` (replacing `take`); edits clear the error;
  tests retargeted.
- `crates/blit-tui/src/main.rs`: `plan_f1_trigger`; the Enter
  handler peeks + closes/sets-error; bridge passes `error`.
- `crates/blit-tui/src/screens/f1.rs`: `TriggerPrompt.error`;
  red `⚠` tail in `render_trigger`.

## Tests

541 total:

f1trigger.rs: `peek_yields_trimmed_pair_without_closing` (peek
doesn't close; `close` does);
`cycle_kind_advances_..._and_peek_reports_it`;
`peek_is_none_when_a_field_is_blank`;
`set_error_shows_and_edits_clear_it`.

main.rs: the malformed-source test now also asserts the modal
stays open with an `error` set (inline feedback), alongside the
existing no-push/no-pull guards.

## Known gaps

1. **Push mirror/move, live progress, remote→remote** still
   pending (d-61 gaps) — d-62 only adds the feedback layer.
2. Error messages are static strings, not localized.

## Out of scope

- Push mirror/move; push live progress; remote→remote; F1 `d`
  diagnostics.

## Reviewer comments

### Round 1 (reopened)

> The module flow comment (`f1trigger.rs:25`) still documents
> `Enter` as calling `F1TriggerState::take`, which this slice
> removed in favor of `peek` / `close` / `set_error`. Update the
> flow comment to match.

**Response (0b47a72):** Updated the module-doc flow step 3 to
describe `peek` (reads without closing), `close` on a successful
launch, and `set_error` (keep the modal open with an inline
message) on validation failure. Doc-only; 541 tests still green.
