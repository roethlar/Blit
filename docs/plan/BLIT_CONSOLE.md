# Blit Console — one core, two faces (GUI + TUI)

**Status**: Active — D1 egui (D-2026-08-04-1); D2 legacy TUI dies at T1 cutover (D-2026-08-04-2); D3 1.0 gates on C1–C4 + T1–T3 (D-2026-08-04-3)
**Created**: 2026-08-02
**Supersedes**: `TUI_REWORK.md` (scrapped whole by owner, 2026-08-02:
"nothing about current TUI works for me. scrap the entire UI"); re-points
`RELEASE_1_0.md` gate G5c per D-2026-08-02-2.
**Owner words defining the product** (2026-08-02): "the point to this is
to manage transfers between daemons and handle the CLI options I don't
want to remember." "It needs to work like a typical file manager. no
arcane keys." "For headless boxes, a GUI is useless" — so the TUI stays
a first-class surface alongside the GUI.

## 1. Why the last two attempts failed (named, so they are not repeated)

- **BlitAdmin (SwiftUI, Oct 2025)**: integrated by spawning the CLI and
  parsing stdout — condemned to chase every output change; macOS-only
  while the owner's desk is Windows; built to completion before the
  owner judged it. "Never actually met my requirements."
- **blit-tui (current)**: three half-browsers in one binary (F1/F3
  panes, an undocumented `dual_pane.rs` default shell, M1's picker),
  arcane keys, unfinished wiring. Owner: "not even provisionally
  useful."
- Common root causes: (a) UI integrated through text instead of APIs;
  (b) the owner — the only UX instrument agents have — saw the result
  months late; (c) no single state model, so every screen grew its own.

## 2. Product shape

ONE brain, two thin faces:

```
                 ┌────────────────────────────┐
                 │      blit-console-core     │
                 │  endpoints · browse trees  │
                 │  transfer composer (typed  │
                 │  options = the CLI flags)  │
                 │  task registry + progress  │
                 │  (direct blit-app/core API │
                 │   — NO stdout parsing)     │
                 └──────┬──────────────┬──────┘
                        │              │
              ┌─────────┴───┐    ┌─────┴────────┐
              │  blit-gui   │    │ blit-tui v2  │
              │ (D1: egui   │    │ (ratatui,    │
              │  or Tauri)  │    │ file-manager │
              │             │    │ layout)      │
              └─────────────┘    └──────────────┘
```

- `blit-console-core` (new workspace crate): endpoint model (local
  filesystem + daemons via mDNS discovery and manual add), browse state
  per endpoint, a transfer composer whose options struct maps one-to-one
  onto the CLI's transfer flags (single source of truth — "the flags you
  don't want to remember" become labeled fields), a task registry
  consuming the existing typed `ProgressEvent` stream, Elm-ish
  message/update architecture, zero UI dependencies. This crate carries
  the product logic and therefore the red/green guards and codex review,
  like any engine slice — the part agents are demonstrably good at.
- Frontends are dumb views: render model, dispatch messages. A click in
  the GUI and a key in the TUI dispatch the same message. A frontend
  that disappoints is rewritten without touching the brain.

## 3. Non-negotiable principles

1. **No stdout parsing, ever.** Core links `blit-app`/`blit-core`
   directly, same APIs `blit-cli` uses.
2. **The owner is the acceptance gate.** Every milestone ends with a
   build the owner runs for sixty seconds; the owner's verdict — not
   agent claims, not tests — closes the milestone. Verdicts are recorded
   in this doc.
3. **Nothing to memorize.** GUI: labeled controls only. TUI: plain
   file-manager conventions (arrows, Enter, Tab between panes, Space
   mark) plus a permanent on-screen action bar; if an action is not
   visible, it does not exist.
4. **Views stay thin enough that "scrap it" is always cheap.**
5. FAST, SIMPLE, RELIABLE applies to surfaces: no modes, no config
   sprawl, no feature a first-time user must be taught.

## 4. Milestones (each: subagent slices → codex review → OWNER RUN)

GUI first (the daily surface; most forgiving to make good), TUI second
on the proven core.

| M | Deliverable | Owner 60-second test |
|---|---|---|
| **C1** | `blit-console-core`: endpoints + discovery + browse (unit-tested, headless) + **GUI shell**: window, daemon/local sidebar, one browse pane | "I can see my fleet and walk directories" |
| **C2** | Transfer composer: source pane + dest pane + options panel (copy/mirror/move, dry-run, checksum, resume, delete scope, filters…) + Go; runs a real transfer through core | "I ran a real transfer without touching the CLI or remembering a flag" |
| **C3** | Task view: live progress, speed/ETA, cancel, concurrent transfers, end-of-run failure surfaced like the CLI's failure block | "I watched two transfers run and killed one" |
| **C4** | GUI polish pass driven solely by the owner's C1–C3 feedback backlog | owner declares the GUI daily-usable |
| **T1** | TUI v2 on the same core: two-pane file-manager layout, visible action bar, browse + copy/mirror/move to other pane | "over ssh, I moved files between daemons without reading docs" |
| **T2** | TUI: composer options + task progress parity | same bar as C2/C3, in a terminal |
| **T3** | TUI polish from owner feedback; **cutover: legacy blit-tui surfaces deleted** (D2) | owner declares the TUI usable |

Milestone slices are sized for opus/sonnet subagents per the owner's
standing instruction; every slice through the full verification gate and
the codex review loop.

## 5. Owner decisions (queued one at a time)

- **D1 — GUI framework. RULED 2026-08-04: egui (D-2026-08-04-1).** ~~(a) **egui**: pure Rust, single static binary,
  no extra toolchain, utilitarian look; recommendation for integration
  simplicity and repo fit. (b) **Tauri**: webview shell, native-feeling
  polish, adds a web toolchain (npm/TS) to the repo and CI. Owner has
  said either is acceptable in principle.~~
- **D2 — fate of legacy `blit-tui` code. RULED 2026-08-04: at T1 cutover (D-2026-08-04-2).** ~~(a) Delete the F1/F3 panes,
  `dual_pane.rs`, and M1's `f3picker.rs` at T1 cutover (recommendation);
  (b) delete immediately and leave the crate a stub until T1.~~
- **D3 — what 1.0 gates on. RULED 2026-08-04: both surfaces (D-2026-08-04-3).** ~~(a) GUI MVP (C1–C4) only, TUI v2 follows
  post-1.0; (b) both surfaces (C1–C4 + T1–T3).~~ Re-points RELEASE_1_0
  G5c to C1–C4 + T1–T3; owner's D-2026-08-02-1 intent ("no 1.0 with an
  unusable UI") is now fully concrete: nothing broken ships, and the
  legacy TUI is retired from the release surface at cutover regardless.

## 6. Out of scope

Daemon-side config editing (BlitAdmin's TOML editor), Windows service
management, packaging/signing beyond the existing release-script shape,
mobile/web remotes.

## 7. Risks

- "Agents aren't good at UI" (owner, from experience): mitigated
  structurally — logic lives in the reviewed core, views stay thin, the
  owner judges every milestone early enough to kill cheaply.
- egui look-and-feel may read as utilitarian; Tauri buys polish at the
  cost of a second toolchain. That trade is exactly D1.
- Frontend churn is expected and priced in; core churn is not — core API
  changes after C2 need a recorded reason.

## 8. Next step

C1 core (slices 1+2) and the egui GUI shell (`crates/blit-gui`) have
landed. Next: owner 60-second test — `cargo run -p blit-gui`, "I can
see my fleet and walk directories." A pass is recorded in the
acceptance log and unlocks C2 (transfer composer).

## Owner acceptance log

(recorded per milestone as verdicts land)
