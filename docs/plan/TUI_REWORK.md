# TUI Rework — From Typing to Navigation

**Status**: Design doc, awaiting owner sign-off before code changes
**Created**: 2026-05-31
**Supersedes**: portions of `TUI_DESIGN.md` §6 (trigger modal text inputs)
and the F3 free-text destination prompt.
**Driving principle**: a TUI's job is to make every operator input
**visible-and-pickable**. Any field that asks the operator to recall
and accurately type a path that isn't on screen is a failure of the
interface.

---

## 1. Problem statement

Walking through the four cardinal transfer scenarios in the shipped
`blit-tui` (verified 2026-05-31 against `phase5/a1` head):

| Scenario | UI helps you with | UI makes you type |
|---|---|---|
| Remote → local single file | Selecting the source file (F3 browse) | The local destination directory |
| Local → local folder | (nothing — rejected outright) | (would require both paths) |
| Local → N remotes | Selecting one daemon at a time | The local source + N remote targets (per-daemon repeat) |
| Remote A → remote B | Selecting source daemon | The source module/subpath **on A** AND the full destination path on B |

**Every scenario asks the operator to type at least one path that isn't
on screen.** The TUI half-delegates to the operator's memory. This is
the same failure mode as a file dialog that opens with a blank text
field instead of a directory tree.

The violated principle (TUI_DESIGN §4 already implies it but doesn't
state it as a hard rule): **path inputs are Pick, not Type.** A typed
path field is acceptable only as an escape hatch for the rare case the
target isn't reachable by navigation (e.g., creating a brand-new
directory by name).

---

## 2. Design principles for the rework

1. **Pick, don't type.** Every path field must default to a navigable
   picker. Typing is an explicit secondary mode, never the only mode.
2. **One browser, two modes.** F3's existing browse machinery is
   already a good picker — it lists daemons, modules, directories,
   filters, ascends, descends. The rework promotes it to a reusable
   picker invokable from any trigger flow, with both a Browse mode
   (current behavior) and a Pick mode (single-shot, returns a path
   to the caller).
3. **Local is a first-class location.** The "Local" row in F1 is
   currently a dead row (filtered out of descend at
   `crates/blit-tui/src/main.rs:2702`). It must become a navigable
   pseudo-daemon whose root is the local filesystem (default = `$HOME`,
   configurable), browsable via the same code path as a remote daemon.
4. **Multi-select where it matters.** Daemons in F1 gain Space-to-mark
   (already the F3 multi-select convention). A trigger with N daemons
   marked is a single batch fan-out, not N manual triggers.
5. **Source pre-fill from current selection.** If you're standing on a
   file or directory in F3 (or in a Local browse) and press the
   trigger key, the source field is *already* what you were looking at.
   Only the destination remains to be picked.
6. **CLI parity is the floor** (already TUI_DESIGN §4 principle 4 —
   restated here because Local↔Local violates it today). Every
   transfer the CLI can perform must have a TUI path; no scenario
   ends with "fall back to the shell."

---

## 3. Target workflows (post-rework)

Each line is one operator key. None require the operator to recall a
path that isn't on screen.

### W1 — Copy a single file from remote A to local
1. `↓` until cursor on remote A in F1
2. `Enter` (descends into A; auto-jumps to F3)
3. `Enter` repeatedly until cursor on the target file
4. `p` (or `t` if we unify) — destination picker opens, defaults to
   Local browser at `$HOME` (or last-used dir)
5. `↑↓ Enter` to navigate to the desired local directory
6. `Enter` to confirm

No path typed.

### W2 — Sync a folder from local to another folder on local
1. `1` to F1
2. `↓` to the Local row
3. `Enter` — descends into Local filesystem browser (the new behavior)
4. Navigate to source folder
5. `t` — source = current selection; destination picker opens
6. Navigate to the destination folder
7. `↑↓` to choose kind (copy / mirror / move) if it's not the default
8. `Enter` to confirm (destructive kinds prompt y/N)

No path typed.

### W3 — Sync a folder from local to multiple remotes
1. `1` to F1
2. `Space` on each remote daemon to mark (cursor still on a daemon
   row when done is fine)
3. `Enter` on the Local row OR `↓` to a different starting state, then:
4. Navigate Local browser to the source folder
5. `t` — source = current selection, destinations = marked daemons
   (multi-target trigger)
6. For each marked daemon: destination picker opens at that daemon's
   root (or auto-applies the same module/path to all — see open
   decision in §6)
7. `Enter` once to launch the batch

No path typed.

### W4 — Sync a folder from remote A to remote B
1. `↓` to remote A in F1
2. `Enter` to descend (auto-jumps to F3 browsing A)
3. Navigate to source folder on A
4. `t` — source pre-filled; destination picker opens
5. `1` (or some pane-switch within the picker) to jump the picker
   focus to daemon B
6. Navigate B's modules + folders to the destination
7. `Enter` to confirm

No path typed.

---

## 4. Architectural changes

### 4.1 New: `LocalDaemon` pseudo-target
Add a "Local" daemon analog that exposes the local filesystem under
the same `RemoteEndpoint`-shaped interface as remote daemons.
Concretely: a small adapter that the browse machinery can drive over
local `std::fs` reads, returning the same `DirEntry` shape the
existing F3 browse pipeline consumes.

**Existing relevant code:**
- `crates/blit-tui/src/browse.rs` — generic browse pane (~1.9K lines)
- `crates/blit-tui/src/screens/f3.rs` — F3 renderer

**New code estimate:** ~300–500 LoC in a new `crates/blit-tui/src/local_browse.rs`
that implements the same listing interface against the local FS, plus
~50 LoC plumbing in `browse.rs` to route by endpoint kind.

### 4.2 New: F3 picker mode
F3 gains a single-shot picker invocation. Caller (the trigger flow)
passes:
- A start location (default = `$HOME` for Local, daemon root for
  remote)
- A "what kind of picker" hint (file? directory? either?)
- A continuation (`Sender<PathPicked>`)

In picker mode F3 visually signals it's modal (border accent or title
suffix `· picker`), absorbs `Enter` as "confirm" instead of "descend"
when the cursor is on the right kind of entry, and `Esc` as cancel.
All other browse keys (filter, ascend, etc.) work normally.

**Existing relevant code:**
- `crates/blit-tui/src/f3pull.rs` — current pull state machine (will
  become a thin wrapper over the picker invocation for the legacy
  `p`/`m`/`v` keys, OR be replaced entirely — see §6).

### 4.3 Modified: F1 trigger modal
The two text fields become two picker buttons. Tab still toggles
focus; Enter on a focused button opens the F3 picker. Once both
endpoints are populated, the kind cycle (↑↓) and final Enter behave
as today.

**Code touched:**
- `crates/blit-tui/src/f1trigger.rs` — state machine (current
  state carries two strings; will become `Option<EndpointSelection>`
  pairs)
- `crates/blit-tui/src/main.rs::plan_f1_trigger` — accept already-typed
  `Endpoint` values rather than re-parsing strings; the Local→Local
  rejection at lines 3505–3508 is **removed** (the whole point of the
  rework)

### 4.4 New: multi-daemon mark + batch trigger
F1 grows Space-to-mark. When ≥1 daemons are marked and `t` is pressed,
the trigger modal opens in "fan-out" mode with `Destinations: [N
daemons]` instead of a single dest picker. A single `plan_f1_trigger`
call internally launches N transfers (serial today, since `F1Push`
serializes — see §6 on whether to parallelize).

### 4.5 Source pre-fill from current selection
If the active pane has a cursor on a path-bearing row and `t` is
pressed, the trigger opens with source = that path. The operator
only picks the destination.

---

## 5. Implementation milestones

Each milestone ships as its own reviewed slice and leaves the TUI
usable.

| M | What | Unlocks | Risk |
|---|---|---|---|
| **M1** | F3 picker mode (just the mode + the path-return plumbing; no new caller wiring yet) | Internal: foundation for M3 | Low — pure addition, no behavior change for existing users |
| **M2** | `LocalDaemon` pseudo-target + Local row descends in F1 | **Local↔Local** in any subsequent flow; the Local row stops being a dead end | Medium — new code path, but isolated to a new module |
| **M3** | F1 trigger modal: replace text fields with picker invocations | All four W1–W4 workflows lose their typing | High — changes the trigger UX every user sees; needs careful default-fallback |
| **M4** | Multi-daemon Space-mark + batch trigger in F1 | W3 (local → N remotes) becomes one operation | Medium — touches F1 state + plan_f1_trigger fan-out |
| **M5** | Source pre-fill on F3 / Local row selection + `t` | Removes the source-pick step entirely when the operator just navigated to it | Low — purely additive |
| **M6** | Polish: keep type-it-anyway escape hatch as a secondary entry in the picker (`/` filter + `:` to enter a literal path) | Power users who want to paste a path | Low |

**Suggested order:** M1 → M2 → M3 → M5 → M4 → M6. M3 is the gating
change for any real usability win; M5 is small, high-value, and can
ride alongside M3.

**Total estimate:** ~2500–3500 LoC net new + ~1000 LoC modified across
4–5 weeks of focused work. Most of M1/M2 are mechanical; M3 and M4
need careful state-machine design.

---

## 6. Open decisions (need owner sign-off)

1. **Picker invocation key.** Today F3 uses `p`/`m`/`v` for
   pull/mirror/move with implicit kind selection. Post-rework, do we
   keep `p`/`m`/`v` (each invokes a picker pre-set to that kind), or
   unify to `t` everywhere (picker plus a kind-cycle) and let `p`/`m`/`v`
   die? Default proposal: **unify to `t`**, keep `p`/`m`/`v` as
   aliases for one release for muscle-memory.

2. **Local browser start directory.** `$HOME`, `$PWD` at TUI launch,
   `/`, or persisted-last-visited? Default proposal: persisted in
   `tui.toml` under `[local] start_dir`, default `$HOME`.

3. **Type-it-anyway escape hatch.** Some operators want to paste a
   path. Inside the picker, a key (proposed: `:`) opens a one-line
   input that accepts a literal path and jumps the picker cursor to
   it (with completion). This stays as a power-user feature, not the
   primary path. Default proposal: **yes, include it in M6**.

4. **Fan-out execution model.** When N daemons are marked and the
   transfer is committed, do we launch N pushes serially (current
   `F1Push` serializes) or parallelize? Parallel saturates the
   uplink and risks the local source being slowest. Default
   proposal: **serial with a visible queue in the F1 footer**, with
   a future `--parallel` config flag deferred to a later milestone.

5. **Per-daemon destinations in fan-out.** When multi-targeting, do
   all destinations share one path (e.g., `:/backup/photos`) or does
   the operator pick per-daemon? Default proposal: **shared path
   across daemons**, with a Tab-key cycle if the operator wants
   per-daemon override. The shared case is the 95%.

6. **F3 picker visual treatment.** Border accent color? Title
   suffix? Status-bar mode indicator? Default proposal: **all
   three** — accent border + title `· picker` + status bar
   "PICK A LOCATION · Enter to confirm · Esc to cancel".

---

## 7. Risks & compatibility

- **Muscle memory.** The `p`/`m`/`v` keys are documented in `?` help
  and TUI_DESIGN. Removing or repurposing them is a breaking UX
  change. Mitigation: keep them as aliases for one release with a
  deprecation hint in the help footer.
- **Trigger modal screen reading.** Today the modal has visible
  source + destination text. After M3, source/dest are summary
  strings (e.g., `Source: nas-a:/photos/2024-trip · Dest: pick…`).
  This is a different shape; needs a focused render pass.
- **Performance of the local browser.** Browsing a directory with
  100K+ entries via blocking `std::fs::read_dir` will stall the TUI.
  Mitigation: `tokio::task::spawn_blocking` for any local-fs listing
  larger than some threshold, with a "loading…" spinner — same
  pattern F3 already uses for remote module listings.
- **Local-fs security/symlinks.** A picker that navigates the local
  filesystem freely is fine for `blit copy` source selection, but
  needs the same containment thinking as `path_safety.rs` applies
  on the wire side. Mitigation: the picker is read-only navigation;
  the transfer itself still goes through `resolve_destination` and
  daemon-side `path_safety` on push. No new attack surface beyond
  what `blit copy` already accepts at the CLI.
- **Crate boundary.** Currently `blit-app` doesn't know about
  filesystem-as-pseudo-daemon. The `LocalDaemon` adapter could live
  in `blit-tui` (TUI-only concern) or `blit-app` (in case the future
  GUI/web client wants the same). Default proposal: start in
  `blit-tui`, promote to `blit-app` if/when a second consumer
  appears.

---

## 8. Out of scope for this rework

- Removing the Prometheus bridge (separate scope-trim conversation).
- Changing the daemon's wire protocol (this is pure TUI work; gRPC
  surface stays exactly as it is).
- Rewriting F2 (Transfers) or F4 (Profile / Verify) — they don't have
  the typing problem; their inputs are already navigable.
- Touching CLI verbs — CLI parity is the floor, not the ceiling.

---

## 9. Next step

Owner reviews this doc. If the direction is right:
1. Open `phase6/tui-rework` branch from master.
2. Ship M1 (F3 picker mode) as the first reviewed slice.
3. Wire up subsequent milestones in the order above.

If the direction is wrong: pin which principles or workflows don't
match the actual intent and we iterate the doc before any code.
