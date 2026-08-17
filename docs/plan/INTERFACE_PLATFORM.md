# Interface Platform — one SDK layer, standalone front-ends

**Status**: Draft — awaiting owner rulings R1 (direction/supersession),
R2 (SDK crate name), R3 (`blit-tui` disposition), R4 (`blit-gui`
disposition). No code until **Status**: Active and a per-slice go.
**Created**: 2026-08-17
**Supersedes on activation**: `docs/plan/UI_REMOVAL.md` (Draft, never
activated); amends D-2026-08-15-1 (UIs stay out-of-repo, but the
headless interface layer is a first-class product of this repo, not UI
residue to delete). D-2026-08-15-2 (1.0 gates on CLI + daemon only) is
unchanged.

## Direction (owner intent, 2026-08-17)

Blit ships one engine and, over time, three independent front-ends:
CLI first, then TUI and GUI. Each front-end is a standalone installable
binary (`winget install blit-gui` gives a GUI that works alone) that:

- does local work with no daemon and no other Blit install present
  (the engine is linked in, not shelled out to);
- can discover and interface with `blit-daemon` instances on other
  machines;
- is a **first-class interface to the blit engines** — never a wrapper
  that drives the CLI in the background;
- works independently of every other front-end.

UI-specific work (TUI, GUI) lives in `http://q:3000/michael/BlitAdmin_UIs.git`.
The layer they build on lives here.

## Architecture invariant

**There is exactly one programmatic interface to the engine: the SDK
crate.** Every front-end consumes it. The CLI is the reference
consumer: if the CLI can only do something by reaching around the SDK,
the SDK API is incomplete and gets extended — that is the enforcement
mechanism for "first-class."

```
blit-core          engine: enumerate, plan, transfer_session
blit-daemon        engine wrapped in the gRPC service (proto/blit.proto)
<sdk crate>        THE headless interface layer (library, no I/O to a screen)
  ├── blit (CLI)               this repo — ships first, 1.0 gate
  ├── blit-tui                 BlitAdmin_UIs — later
  └── blit-gui                 BlitAdmin_UIs — later
```

SDK responsibilities (all exist today, split across `blit-app` and
`blit-console-core`; this plan consolidates, it does not invent):

- in-process engine operations (local transfers, scan, check, profile);
- daemon discovery (`blit_core::mdns` via `discover`);
- remote daemon connection (`BlitClient` gRPC with bounded connect,
  contract-version handling) and remote browse (`browse`, `endpoint`,
  `model` with stale-completion generation tags);
- admin verbs (df, diagnostics, endpoints, transfer filtering, display
  helpers);
- full engine access for front-ends through one dependency
  (`pub use blit_core as core;` re-export — see if-3).

Dependency rule: front-ends (`blit-cli`, and out-of-repo TUI/GUI)
depend on the SDK crate only. Services (`blit-daemon`,
`blit-prometheus-bridge`) may keep direct `blit-core` deps.
Out-of-repo consumers pin the SDK by git rev/tag on the LAN gitea and
upgrade deliberately; no crates.io publication (non-goal).

## Rulings required (chat, one at a time; record in `docs/DECISIONS.md`)

- **R1 — direction**: adopt this architecture; supersede
  `UI_REMOVAL.md`; amend D-2026-08-15-1 as described above.
- **R2 — SDK crate name**: `blit-app` + `blit-console-core` merge into
  one crate. Recommendation: **`blit-sdk`** — says exactly what it is
  (the supported surface you build a Blit front-end on), no false
  UI connotation ("console"), no vague one ("app"). Alternate
  considered: `blit-control`. The rest of this plan writes `blit-sdk`;
  a different ruling substitutes mechanically.
- **R3 — `blit-tui`** (29,172 lines, pre-dates the one-session
  architecture decisions and D-2026-08-15-1): (a) delete here, future
  TUI starts fresh in BlitAdmin_UIs against the SDK (git history keeps
  the code recoverable), or (b) move as-is to BlitAdmin_UIs.
  Recommendation: **(a)**.
- **R4 — `blit-gui`** (552 lines, the landed C1 eframe shell, already
  console-core-only): (a) move as-is to BlitAdmin_UIs as the GUI
  starting point, then delete here, or (b) delete here, GUI starts
  fresh. Recommendation: **(a)**; the shell is current and thin.

## Facts (verified 2026-08-17 against the tree)

- Workspace members: blit-core, blit-app, blit-cli, blit-daemon,
  blit-tui, blit-prometheus-bridge, blit-console-core, blit-gui.
- Dependency edges: `blit-cli` → core + app; `blit-app` → core;
  `blit-console-core` → app + core; `blit-tui` → core + app;
  `blit-gui` → console-core only; `blit-daemon` → core;
  `blit-prometheus-bridge` → core + app.
- `blit-app` modules (4,290 lines): admin, check, client, diagnostics,
  display, endpoints, profile, scan, transfers. No UI-toolkit deps.
- `blit-console-core` modules (1,416 lines): browse, discover,
  endpoint, model. `discover` wraps `blit_core::mdns`; `browse`/`model`
  carry the generation-tag stale-completion design.
- `blit-cli` references `blit_core::` at 112 sites and `blit_app::` at
  96 sites (grep counts; re-verify at implementation time).
- gRPC client type: `blit_core::generated::blit_client::BlitClient`;
  bounded connect lives in `blit_app::client::connect_with_timeout`.
- CI `.github/workflows/ci.yml` installs GUI headers in three jobs
  (lines 28–31, 52–55, 116–119) solely because `blit-gui` (egui)
  compiles in the workspace.
- `README.md:42` draws `blit-tui/` in the tree; comment-only `blit-tui`
  references remain in surviving crates (`crates/blit-app/Cargo.toml:8`,
  `crates/blit-app/src/lib.rs:2`,
  `crates/blit-cli/src/transfers/failures.rs:12`, …).

## Non-goals

- Building TUI or GUI features (BlitAdmin_UIs work, not this repo's).
- Relaxing D-2026-07-05-2 (same-build peers only) — see Deferred.
- New package IDs / channels — `PACKAGE_MANAGER_DISTRIBUTION.md` owns
  packaging; `blit-tui`/`blit-gui` IDs are added there when those
  front-ends ship.
- crates.io publication of any crate.
- History rewrite; plain commits on `master`, one slice each.

## Acceptance criteria

- [ ] One SDK crate exists containing today's `blit-app` +
      `blit-console-core` capability; both old crates gone from
      `cargo metadata`.
- [ ] `blit-cli`'s `Cargo.toml` depends on the SDK crate only (no
      direct `blit-core`); grep shows zero `blit_core::` /
      `blit_app::` paths in `crates/blit-cli/src`.
- [ ] `blit-gui`/`blit-tui` resolved per R3/R4; workspace builds
      core / sdk / cli / daemon / prometheus-bridge only; the three CI
      GUI-header blocks removed; CI green.
- [ ] `README.md` tree and surviving-crate comments match the new
      shape.
- [ ] Full gate green per slice (fmt; clippy native + strict
      `x86_64-unknown-linux-gnu` cross-target at `-D warnings`;
      `cargo test --workspace`; `scripts/agent/check-docs.sh`).
      Test-count deltas recorded per slice; drops only from deleted
      crates' own tests, named in the slice record.

## Slices (each: own go, own commit, full gate, DEVLOG entry)

1. **if-1 — rename `blit-app` → `blit-sdk`.** `git mv
   crates/blit-app crates/blit-sdk`; update package name, workspace
   member, and the dependents' Cargo.tomls (blit-cli,
   blit-console-core, blit-tui if present, blit-prometheus-bridge);
   rewrite `blit_app::` imports to `blit_sdk::`; regenerate
   `Cargo.lock`. Mechanical; zero behavior change; test delta 0.
2. **if-2 — fold `blit-console-core` into `blit-sdk`.** Move
   `browse.rs`, `discover.rs`, `endpoint.rs`, `model.rs` in as public
   modules (note: existing `endpoints.rs` vs incoming `endpoint.rs` —
   keep both names, they are different things: CLI endpoint parsing vs
   browse-pane endpoint model; document each in its module header).
   Rewire `blit-gui` to `blit-sdk`; drop the member; regenerate
   `Cargo.lock`. Move console-core's tests with their modules; test
   delta 0.
3. **if-3 — single-dependency surface.** Add `pub use blit_core as
   core;` to `blit-sdk`'s lib.rs. Migrate `blit-cli` off its direct
   `blit-core` dep: rewrite `use blit_core::…` → `use
   blit_sdk::core::…` (112 sites, mechanical), remove `blit-core` from
   `crates/blit-cli/Cargo.toml`. Gate addition: a grep check proving
   `crates/blit-cli/src` contains no `blit_core::`/`blit_app::` paths.
4. **if-4 — `blit-gui` per R4.** If (a): copy the crate into
   BlitAdmin_UIs with its dependency rewritten to a git-pinned
   `blit-sdk` (every push to BlitAdmin_UIs is owner-gated,
   named-remote, approved in-session). Then, either ruling: delete
   `crates/blit-gui`, drop the member, remove the three CI GUI-header
   blocks, regenerate `Cargo.lock`. Record deleted-test delta.
5. **if-5 — `blit-tui` per R3.** Delete (or move, per ruling) the
   crate; drop the member; regenerate `Cargo.lock`; fix `README.md`
   tree and comment-only references in surviving crates. Record
   deleted-test delta.
6. **if-6 — record the shape.** Update `README.md` architecture prose
   and crate list to the final workspace; `docs/STATE.md` queue and
   authoritative-docs entries; supersession edits to `UI_REMOVAL.md`
   (Status → Superseded, pointer here) if not done at activation.

Order: if-1 → if-2 strictly first (everything else touches the merged
crate). if-3, if-4, if-5 are independent of each other; default order
as numbered. if-6 last.

## Deferred (recorded so it is not rediscovered)

- **Peer version skew.** Independently installed front-ends and
  daemons WILL meet across versions. D-2026-07-05-2 (same-build peers
  only, refusal at session open) stands until an explicit owner
  decision replaces it with contract-version compatibility
  (`CONTRACT_VERSION` in `proto/blit.proto` is the hook). Must be
  ruled before any front-end ships as a separately-versioned install.
- **Curated event surface.** Front-ends initially reach engine
  progress/observer APIs through the `core` re-export. If the raw
  surface proves too sharp for the TUI/GUI, a curated event-stream
  API in `blit-sdk` is its own future plan.
- **SDK tagging discipline** for BlitAdmin_UIs pinning (which tags,
  when) — decide when the first out-of-repo consumer lands.
