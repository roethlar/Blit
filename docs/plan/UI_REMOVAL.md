# UI Crate Removal

**Status**: Superseded — by `docs/plan/INTERFACE_PLATFORM.md`
(D-2026-08-17-1) before ever activating. This plan deleted
`blit-console-core` as UI residue; the owner instead ruled that the
headless interface layer is a first-class product (the SDK crate) and
future UIs are standalone SDK consumers in BlitAdmin_UIs. The deletion
slices below are absorbed/reshaped there (ui-rm-1/2 → if-4/if-5; the
`blit-app` question → `blit-app` becomes the SDK). Historical record
only; do not execute.
**Created**: 2026-08-15
**Supersedes**: nothing (executes the removal that D-2026-08-15-1 gated on
"its own approved plan and go")
**Decision ref**: D-2026-08-15-1 (Blit is CLI + daemon only),
D-2026-08-15-2 (1.0 gates on CLI + daemon only)

## Goal

Make the workspace build exactly what Blit ships: the CLI and the daemon.
Delete `crates/blit-gui`, `crates/blit-console-core`, and `crates/blit-tui`,
resolve `crates/blit-app` (Open questions), and strip UI-only CI residue.

## Non-goals

- Migrating any UI code to `BlitAdmin_UIs` — owner-gated and undecided
  (D-2026-08-15-1); deletion here does not decide it (git history keeps
  the code recoverable).
- Touching historical records: `DEVLOG.md`, `docs/`, `.review/` keep their
  UI references as history.
- Any release/packaging work; the GitHub Release archives already ship
  only `blit` + `blit-daemon`.

## Constraints

- No history rewrite; plain removal commits on `master`, one slice each.
- Test count drops only by the deleted crates' own tests; the exact delta
  is recorded in each slice's record (repo rule: removals named in Known
  gaps).
- Full gate after each slice: fmt, clippy native + strict
  `x86_64-unknown-linux-gnu` cross-target, workspace tests, check-docs.

## Facts (verified 2026-08-15)

- Root `Cargo.toml` members include `blit-app`, `blit-tui`,
  `blit-console-core`, `blit-gui`.
- Dependency chain: `blit-gui` → `blit-console-core` → `blit-app`;
  `blit-tui` → `blit-app`; **`blit-cli` → `blit-app`** (heavy, real use:
  `check`, `endpoints`, `client`, `admin::df`, `diagnostics`, `display`,
  `transfers::filter`, …). `blit-app` has no UI-toolkit deps (no
  egui/eframe/winit/ratatui) — it is a headless app layer over
  `blit-core`, not a UI crate.
- CI `.github/workflows/ci.yml`: three jobs install GUI headers (lines
  28–31, 52–55, 116–119 — libx11/xrandr/xi/xcursor/xinerama/
  libgl1-mesa/wayland/xkbcommon), needed only while `blit-gui` (egui)
  compiles in the workspace.
- `README.md:42` still draws `blit-tui/` in the tree.
- Comment-only references to `blit-tui` remain in surviving crates
  (`crates/blit-app/Cargo.toml:8`, `crates/blit-app/src/lib.rs:2`,
  `crates/blit-cli/src/transfers/failures.rs:12`, ...) — prose cleanup,
  not code motion.

## Acceptance criteria

- [ ] `cargo metadata` lists no `blit-gui`, `blit-console-core`,
      `blit-tui`; `blit-app` state matches the owner's answer below.
- [ ] Full gate green locally on macOS + cross-target clippy; CI green
      with the GUI-header install steps removed.
- [ ] `README.md` tree and surviving-crate comments no longer imply the
      UI crates exist.
- [ ] Removed-test delta recorded per slice.

## Slices

1. `ui-rm-1` — delete `crates/blit-gui` + `crates/blit-console-core`
   (leaf pair), drop their workspace members, regenerate `Cargo.lock`,
   remove all three CI GUI-header install blocks (headers exist only for
   egui). Full gate.
2. `ui-rm-2` — delete `crates/blit-tui`, drop its member, regenerate
   `Cargo.lock`, fix `README.md` tree and surviving comment references.
   Full gate.
3. `ui-rm-3` — `blit-app` per the owner's answer: either (a) fold what
   `blit-cli` uses into `blit-cli`/`blit-core` and delete the crate, or
   (b) keep `blit-app` as the CLI's internal headless layer and rename
   nothing. Full gate.

## Open questions

- **`blit-app` disposition (owner call, blocks only ui-rm-3):**
  D-2026-08-15-1's text lists `blit-app` with the UI crates, but it is
  headless and load-bearing for `blit-cli`. (a) fold + delete — fewer
  crates, larger diff, no shipped-behavior change; (b) keep — smallest
  change, workspace retains one internal library crate that is not a UI.
  Recommendation: (b).
