# tui-key-dispatch-press-only-filter: TUI input task dropped Repeat events

**Severity**: Bug fix / TUI input regression
**Status**: In progress / pending owner test
**Branch**: `phase5/a1`
**Commit**: pending build

## Surfaced by

Owner-reported: "panes render but keys do nothing, none of the arrow or
F keys do anything. I can't select anything." Apple_Terminal /
`TERM=xterm-256color`, post-build of `blit-tui` from `phase5/a1`.

## Root cause (suspected)

`spawn_input_task` in `crates/blit-tui/src/main.rs` matched only
`Event::Key(key) if key.kind == KeyEventKind::Press`. Anything reported
with `KeyEventKind::Repeat` (autorepeat) or any other kind was silently
dropped through the catch-all `Ok(_)` arm. Depending on terminal +
crossterm version + autorepeat behavior, some keystrokes can be
delivered as `Repeat` rather than `Press`, which would manifest as
"key does nothing" with no error visible to the operator.

Apple_Terminal is normally Press-only, but crossterm 0.29's behavior on
some terminal/OS combinations is documented to vary. Without a
reliable in-terminal trace, the Press-only filter is too narrow to be
defensive.

## Fix

`spawn_input_task` now accepts every `Event::Key` whose `kind` is **not
`KeyEventKind::Release`**. Press + Repeat both produce a forwarded
`KeyEvent` on the channel; Release is dropped (would double-fire any
matched action).

A diagnostic env-var was also added: setting `BLIT_TUI_INPUT_TRACE=1`
makes the input task append a JSON line per raw crossterm event to
`/tmp/blit-tui-input.log`, including the `kind`/`code`/`modifiers`,
plus task-lifecycle markers (`# input task started`,
`# input task: receiver dropped, exiting`, poll/read error reports).
If the kind-filter relaxation isn't enough, the trace will show
exactly what events the terminal is sending — separating "input task
never runs," "events never arrive," and "events arrive with unexpected
`kind`" so the next iteration can target the actual gap.

## Verification

`cargo fmt --all --check`, `cargo clippy --workspace --all-targets -D
warnings`, `cargo test --workspace` all green. The change does not
touch the inline test suite (which uses the local `KeyEvent` struct
that doesn't carry `kind`).

**Pending owner interactive test**: rebuild `target/release/blit-tui`
and confirm arrow + F-keys dispatch. If not, run with
`BLIT_TUI_INPUT_TRACE=1` and the log will tell us what's actually
arriving from the terminal.

## Cross-ref

Surfaced during the post-audit-7d shakedown along with
`bug-mirror-literal-backslash`. The audit-7d refactor was
behavior-preserving and did not touch the input task; the Press-only
filter pre-dates this branch (introduced with the original a1-2
single-owner input task).
