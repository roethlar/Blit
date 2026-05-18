# a1-1-tui-scaffold: `blit-tui` crate + minimal ratatui event loop

**Severity**: Feature (first slice of milestone A.1 — the TUI itself)
**Status**: In progress / pending review
**Branch**: `phase5/a1`
**Commit**: filled by the sentinel commit

## What

Lands a new `blit-tui` binary crate with a minimal
ratatui-backed event loop. Terminal lifecycle (alternate
screen, raw mode, cursor hide/show, panic-safe teardown),
keystroke polling, and a placeholder splash screen — that's
it. The four-screen layout (F1 Daemons / F2 Transfers / F3
Browse / F4 Profile/Verify) lands in subsequent A.1
sub-slices.

## Why scaffold-first

The TUI is ~3000 LOC of new code per the design doc
(§11). Splitting it into "scaffold, then per-screen
slices" lets the reviewer audit:

1. The terminal lifecycle (this slice) — does the binary
   restore the terminal on every exit path, including
   panics?
2. The async runtime story (this slice) — tokio
   current_thread vs multi_thread, blocking-vs-async event
   poll, future `tokio::select!` between keystrokes and
   Subscribe streams.
3. The crate boundary (this slice) — does `blit-tui` depend
   on `blit-core` + `blit-app` only, no daemon-internal
   types?

Each subsequent screen slice can then focus on its specific
rendering + data-flow concern without re-litigating
scaffold choices.

## Approach

### Crate scaffold

```toml
[package]
name = "blit-tui"
version = "0.1.0"
edition = "2021"

[[bin]]
name = "blit-tui"
path = "src/main.rs"

[dependencies]
blit-core = { path = "../blit-core" }
blit-app  = { path = "../blit-app" }
tokio = { version = "1", features = ["full"] }
ratatui = "0.30"
crossterm = "0.29"
eyre = "0.6"
clap = { version = "4.6", features = ["derive"] }
```

`blit-tui` depends on `blit-core` (proto-generated types
for the future Subscribe consumer) and `blit-app` (the
admin/jobs helpers for the future F2 Transfers pane). No
`blit-daemon` dependency — the TUI is purely a client.

### Terminal lifecycle

```rust
let mut terminal = enter_tui()?;      // raw + alt-screen + hide cursor
let result = run_event_loop(&mut terminal).await;
leave_tui(&mut terminal)?;            // always restore, even on err
result
```

`enter_tui` runs the standard ratatui setup. `leave_tui`
intentionally ignores per-call errors so a partial restore
failure can't mask the actual loop error.

For panic safety: ratatui's recommended pattern uses
`std::panic::take_hook` to install a teardown hook. Today's
slice doesn't (just normal `?`-propagation). A follow-up can
add the hook once the binary has nontrivial state to lose.
Documented as a known gap.

### Event loop

```rust
loop {
    terminal.draw(render_splash)?;
    if event::poll(50ms)? {
        if let Event::Key(key) = event::read()? {
            if key.kind == KeyEventKind::Press
               && should_quit(key.code, key.modifiers) {
                return Ok(());
            }
        }
    }
}
```

50ms poll interval keeps keystroke latency below human
perception (~20Hz refresh) without burning CPU on idle. The
poll is non-blocking — future slices will replace the
blocking `event::poll` with a tokio `tokio::select!` between
a polled keystroke future and an async stream future.

### Quit predicate

`q` / `Esc` are the muscle-memory shortcuts. `Ctrl-C` is the
safety net for a stuck UI. Factored as a pure function
(`should_quit(code, modifiers) -> bool`) so it's unit-
testable without spinning up a terminal.

### Splash content

Centered placeholder text via `center_within` helper. Notes
the future four screens and the exit shortcuts. Replaced
entirely by the F1/F2/F3/F4 router in subsequent slices.

## Files changed

- `Cargo.toml`:
  - `+"crates/blit-tui"` in `[workspace] members`.
- `crates/blit-tui/Cargo.toml` (new):
  - Binary crate; deps on `blit-core` + `blit-app` +
    ratatui 0.30 + crossterm 0.29 + tokio + clap + eyre.
- `crates/blit-tui/src/main.rs` (new):
  - `Args` (clap; `--remote` field captured but unused).
  - `enter_tui` / `leave_tui` lifecycle wrappers.
  - `run_event_loop` (async, polls keystrokes).
  - `should_quit` predicate.
  - `render_splash` placeholder content.
  - `center_within` layout helper.
  - `#[cfg(test)] mod tests` with 3 unit tests.

## Tests added

3 in the new `blit-tui` test module:

- `should_quit_recognises_q_esc_ctrl_c` — assertion sweep
  over the three exit shortcuts.
- `should_quit_ignores_other_keys` — confirms plain `c`
  without Ctrl isn't a quit shortcut (negative case).
- `center_within_returns_middle_band` — verifies the
  layout helper centers `height` lines within an `area`.

Workspace: 578 passing serially (was 575; +3).

## Known gaps

1. **No panic hook.** If a panic fires during the event
   loop, the terminal is left in raw mode / alternate
   screen until the OS resets it. Subsequent slice should
   install a `std::panic::take_hook` wrapper that runs
   `leave_tui` before re-raising. Deferred — today's
   placeholder content has no panic-able state.

2. **`--remote` is parsed but unused.** Captured in the
   `Args` struct so the next slice doesn't have to
   re-litigate flag shape. Documented in the rustdoc.

3. **No screens yet.** Splash only. F1/F2/F3/F4 land in
   subsequent A.1 sub-slices.

4. **No SIGWINCH handling.** ratatui auto-resizes on the
   next draw, but a future slice could explicitly handle
   `Event::Resize` for layout-aware repaints.

## Out of scope (next A.1 slices)

- **a1-2-f2-transfers**: F2 Transfers pane reading from
  Subscribe stream against a default daemon. Most
  load-bearing screen since it exercises the c-2..c-5b
  machinery end-to-end with a TUI consumer.
- **a1-3-f1-daemons**: F1 Daemons via mDNS + per-daemon
  GetState detail.
- **a1-4-f3-browse**: F3 Browse via List/Find/DiskUsage
  /FilesystemStats.
- **a1-5-f4-profile**: F4 reads `~/.config/blit/perf_local.jsonl`.
- **a1-6-screen-router**: F1↔F2↔F3↔F4 navigation + the
  global keymap.

## Round 2 (sha `a880559`)

Reviewer caught the load-bearing gap: round 1's restore
only fired after a fully successful setup AND a normal-
return event loop. Setup failures (EnterAlternateScreen,
Terminal::new, terminal.clear, hide_cursor) and panics
during the loop both left the terminal in raw / alternate-
screen / cursor-hidden until the user manually `reset`'d.

The scaffold's only deliverable is terminal lifecycle
correctness, so this had to land before verification.

Fix:

1. **`TuiGuard` RAII wrapper.** Owns the terminal handle.
   `new()` is transactional — every stage that committed
   side-effects is rolled back if any subsequent stage
   fails. `Drop` runs `restore_terminal()` unconditionally
   on every exit path (normal return, `?`-propagated error
   from setup or loop, panic unwinding through main).
2. **Panic hook.** `install_panic_hook` chains the original
   hook with a `restore_terminal()` call. Catches panics
   that abort the process too quickly for Drop to compete.
   On normal unwind Drop fires first; the hook's restore
   is a cheap no-op via the `TUI_ACTIVE` flag swap.
3. **`restore_terminal()` is idempotent.** Uses an
   `AtomicBool TUI_ACTIVE` and `swap`s it to false on the
   first call — subsequent calls early-return. Both Drop
   and panic hook can fire safely.

The four paths the reviewer flagged are now covered:

- **Partial setup failure**: `TuiGuard::new()` rewinds any
  state it committed and returns Err. `main` sees the Err
  and returns; no guard exists, but `restore_terminal`'s
  rollback path already ran inline.
- **Normal error from loop**: `?` propagates → `run_event_loop`
  returns Err → guard drops as `main` returns → restore.
- **Normal quit**: loop returns Ok → guard drops → restore.
- **Panic**: panic hook fires first → restore. On normal
  unwind the guard's Drop also fires (no-op via
  TUI_ACTIVE swap).

+2 unit tests:

- `restore_terminal_is_noop_when_not_active` — flag-false
  path is a clean return.
- `restore_terminal_idempotent_across_repeated_calls` —
  repeated calls swap → no-op semantics, validates the
  Drop + panic-hook safety contract.

Workspace: 580 passing serially (was 578; +2 new in
blit-tui).

## Round 3 (sha `38df2bb`)

Reviewer caught that the round-2 idempotency tests called
`restore_terminal()` directly. That fires real `Show` +
`LeaveAlternateScreen` crossterm escape bytes to stderr —
visible in `cargo test` output and polluting CI logs.

Fix: extract a pure state-transition helper:

```rust
fn take_active_for_restore() -> bool {
    TUI_ACTIVE.swap(false, Ordering::SeqCst)
}
```

`restore_terminal()` calls it; if true, fires the
crossterm sequences. Tests call `take_active_for_restore`
directly — no terminal I/O during `cargo test`.

Renamed the two tests to reflect the function they
exercise:

- `take_active_for_restore_inactive_returns_false`
- `take_active_for_restore_active_then_inactive`

Same idempotency contract, no escape sequences in test
output.

## Round 4 (sha `2237521`)

Round 3's tests both mutated the process-global `TUI_ACTIVE`.
Under Rust's parallel test harness they could interleave —
one test storing `false` between the other's setup and
assertion. Reviewer flagged the resulting order-dependence.

Fix: `take_active_for_restore(flag: &AtomicBool)` now takes
its flag by reference. Production calls
`take_active_for_restore(&TUI_ACTIVE)`; tests pass local
`AtomicBool::new(...)` instances. Parallel test execution
no longer races. Same 5 tests, no flake.

## Reviewer comments

(empty — pending grade)
