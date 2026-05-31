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

**Metadata translation (review 2 refinement):** the local adapter
must translate `std::fs::Metadata` (size, mtime, mode, file_type,
symlink target) into the same `DirEntry` shape the daemon-side listing
RPC returns, so `crates/blit-tui/src/display_f3.rs` and the existing
F3 row renderer get formatting consistency for free. No new display
branches; "Local" rows look exactly like remote rows except for the
host column.

### 4.2 New: F3 picker mode
F3 gains a single-shot picker invocation. Caller (the trigger flow)
passes:
- A start location (default = `$HOME` for Local, daemon root for
  remote)
- A "what kind of picker" hint (file? directory? either?)
- A continuation (`Sender<PathPicked>`)

Picker mode keymap — explicit, no ambiguity (review round-1 fix):

| Key | Behavior |
|---|---|
| `Enter` | **Always descend / open the highlighted entry.** Never "confirm." On a directory, descend into it. On a file in **file-picker** mode, return that file as the selection (this is the only case where Enter is terminal). On a file in **directory-picker** mode, Enter is a no-op (with a footer hint). |
| `.` (period) | **Pick the directory currently being viewed** (i.e., the picker's cwd). This is the *only* way to choose a directory destination — there is no Enter-on-empty-cursor magic. The mnemonic matches `cd .` (this here). Only fires in directory-picker mode. |
| `Esc` | Cancel the picker; return `None` to the caller; restore the previous pane state. The continuation channel is drained + dropped (see §7 risks). |
| `←` / `h` / `Backspace` | Ascend one level. |
| `/` | Open the existing filter input (already in F3 Browse). |
| pane-switch keys (`1`/`2`/`3`/`4`, `F1`..`F4`) | **Disabled in picker mode** — the picker is modal; switching panes mid-pick would orphan the continuation. Status-bar shows "PICK MODE · pane switch disabled · Esc to cancel". |

Visual signals: accent border, title suffix `· picker (file)` or
`· picker (directory)` to match what the caller asked for, and a
status-bar line stating exactly which keys do what
("Enter descends · `.` picks this directory · Esc cancels").

The Enter-always-descends rule means **a directory-picker user always
navigates *into* the directory they want, then presses `.` to choose
it** — never "select-while-pointing-at-the-row." This removes the
ambiguity Review 1 flagged where Enter could mean either descend or
confirm depending on cursor position.

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
| **M3a** | F1 trigger modal: replace both text fields with picker invocations | Workflows W2, W3, W4 lose source/destination typing | High — changes the most-visible trigger UX |
| **M3b** | F3 pull destination prompt: replace the existing free-text local-dest input (`p`/`m`/`v` flow) with a picker invocation | Workflow W1 loses its typed local-destination step | Medium — touches `f3pull.rs::F3PullStatus::EnteringDest` and `handle_f3_pull_keystroke` |
| **M4** | Multi-daemon Space-mark + batch trigger in F1 | W3 (local → N remotes) becomes one operation | Medium — touches F1 state + plan_f1_trigger fan-out |
| **M5** | Source pre-fill on F3 / Local row selection + `t` | Removes the source-pick step entirely when the operator just navigated to it | Low — purely additive |
| **M6** | Polish: keep type-it-anyway escape hatch as a secondary entry in the picker (`/` filter + `:` to enter a literal path) | Power users who want to paste a path | Low |

**Acceptance criterion gating M3a + M3b together (review round-1 fix):**
After both M3a and M3b ship, **no normal-flow trigger or pull path
reachable from the default keymap accepts a free-text path field.**
Verified by code grep: zero remaining `*::push_char` on a path-bearing
state in `f1trigger.rs` or `f3pull.rs`, and the existing
`handle_f1_trigger_keystroke` / `handle_f3_pull_keystroke` text-input
handlers either are removed or only run inside the M6 escape-hatch
input. M3a alone is not sufficient to call the rework "done for the
common case" — W1 (the single-most-common pull workflow) needs M3b too.

**Suggested order:** M1 → M2 → M3a → M3b → M5 → M4 → M6. M3a and M3b
together are the gating change for the typing-elimination promise; M5
is small, high-value, and can ride alongside either M3 slice.

**Total estimate:** ~2500–3500 LoC net new + ~1000 LoC modified across
4–5 weeks of focused work. Most of M1/M2 are mechanical; M3 and M4
need careful state-machine design.

---

## 6. Decisions (locked, both reviewers concurred 2026-05-31)

The original v1 of this doc listed six items as "needs owner sign-off."
Both reviewers (chat-review + `TUI_REWORK_REVIEW.md`) endorsed all six
default proposals. They are now locked in below; the implementation
follows these without re-asking.

1. **Picker invocation key.** ✅ **Unify to `t` for all transfer
   triggers across F1 and F3.** Keep `p`/`m`/`v` as aliases for one
   release (deprecation hint in `?` overlay), then remove. Each
   alias maps to "open trigger with kind pre-set to copy / mirror /
   move respectively."

2. **Local browser start directory.** ✅ **`tui.toml [local] start_dir`,
   default `$HOME`.** Persisted last-visited dir is a follow-up
   enhancement after M2 ships; not in the critical path.

3. **Type-it-anyway escape hatch.** ✅ **Include in M6.** Inside any
   picker, `:` opens a single-line literal-path input with shell-style
   completion. Power-user feature, not the primary flow.

4. **Fan-out execution model.** ✅ **Serial, with a visible queue in
   the F1 footer** (`Pushing 2/5 · nas-c:/backup/`). Avoids local
   uplink thrashing and simplifies error handling. `[fanout] parallel
   = N` config flag deferred.

5. **Per-daemon destinations in fan-out.** ✅ **Shared path by default.**
   The 95% workflow ("same module + path on every box"). Tab in the
   batch-trigger modal cycles to per-daemon override for the
   remaining 5%.

6. **F3 picker visual treatment.** ✅ **All three signals together** —
   accent border + title suffix `· picker (file)` / `· picker (directory)` +
   status bar line "PICK MODE · Enter descends · `.` picks this directory · Esc cancels".

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

- **Picker continuation channel hygiene (review 2 refinement).** The
  picker invocation passes a `tokio::sync::oneshot::Sender<PathPicked>`
  to a transient picker pane. Any termination path — `Esc`, pane
  switch (which is disabled in picker mode but a defense-in-depth
  panic-handler restore could still fire), TUI quit (`q`), or a panic
  inside the picker render — must drop that sender so the awaiting
  caller receives `Err(RecvError)` and falls back to its cancel path
  cleanly. The picker pane itself must reset all transient state
  (filter buffer, cursor history) before returning control to the
  prior pane, so a re-opened picker starts from its configured
  `start_dir` (decision 2) rather than wherever the last picker ran
  ended.

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

---

## Review log

**Round 1 (2026-05-31):** chat reviewer requested changes, second
reviewer (`TUI_REWORK_REVIEW.md`) signed off on the direction with
refinements. Both rounds folded into this revision:

| Source | Severity | Issue | Resolution |
|---|---|---|---|
| Chat review | Medium | Picker `Enter` ambiguous: descend vs confirm on a directory | §4.2 rewritten with explicit keymap table — Enter **always** descends; `.` picks the currently-viewed directory; status-bar line makes it explicit |
| Chat review | Medium | M3 overclaimed "all W1–W4 lose typing" while only touching F1 modal — F3 pull dest left typed | M3 split into M3a (F1 trigger pickers) + M3b (F3 pull dest picker); explicit acceptance criterion gating both ("zero remaining `push_char` on a path-bearing state") |
| Chat review | Low | TODO.md still pointed agents at superseded `TUI_DESIGN.md` for Phase 5 work | `TODO.md` Phase 5 section flipped to ✅ SHIPPED status with a new Phase 6 section pointing at this doc as the active plan |
| `TUI_REWORK_REVIEW.md` | Refinement | `LocalDaemon` should translate `std::fs::Metadata` into `DirEntry`-shape for display consistency | §4.1 augmented with explicit metadata-translation requirement |
| `TUI_REWORK_REVIEW.md` | Refinement | Picker continuation channel needs cleanup-on-cancel hardening | §7 risks augmented with explicit continuation-channel hygiene note |
| `TUI_REWORK_REVIEW.md` | Endorsement | All six open decisions endorsed | §6 locked all six with both reviewers concurring |
