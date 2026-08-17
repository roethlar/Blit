# Interface Platform — blit-core is the platform; standalone front-ends

**Status**: Draft — R1 (direction) APPROVED = D-2026-08-17-1; R2
(interface-crate name) DISSOLVED = D-2026-08-17-2 (no new crate:
everything folds into `blit-core`, which publishes to crates.io).
Awaiting R3 (`blit-tui` disposition), R4 (`blit-gui` disposition),
R5 (`blit-prometheus-bridge` deletion). No code until **Status**:
Active and a per-slice go.
**Created**: 2026-08-17 (reworked same day under D-2026-08-17-2; the
original draft's merged-SDK-crate shape, slices if-1..if-3, is
superseded)
**Supersedes**: `docs/plan/UI_REMOVAL.md` (Draft, never activated;
D-2026-08-17-1); amends D-2026-08-15-1 (UIs stay out-of-repo, but the
headless interface capability is a first-class product of this repo,
not UI residue to delete). D-2026-08-15-2 (1.0 gates on CLI + daemon
only) is unchanged.

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
The layer they build on lives here — and it is `blit-core` itself.

Additionally (D-2026-08-17-2): Blit is embeddable. Third-party Rust
apps integrate by adding the `blit-core` crate; everything else
integrates through the daemon's gRPC surface. `blit-core` publishes to
crates.io.

## Architecture invariant

**There is exactly one programmatic interface to the engine:
`blit-core`.** No intermediate crate. Every front-end — the in-repo
CLI, the out-of-repo TUI/GUI, any third-party embedder — depends on
`blit-core` alone. The CLI is the reference consumer: if the CLI can
only do something by reaching around the library, the library API is
incomplete and gets extended. That rule is what makes third-party
embedding work without a special case.

```
blit-core          THE platform crate (publishes to crates.io):
                   engine (enumerate, plan, transfer_session),
                   verbs (check/scan/df/diagnostics/…), daemon client,
                   mDNS discovery, remote browse, event/observer surface
blit-daemon        blit-core wrapped in the gRPC service (proto/blit.proto)
  consumers of blit-core:
  ├── blit (CLI)              this repo — ships first, 1.0 gate
  ├── blit-tui                BlitAdmin_UIs — later
  ├── blit-gui                BlitAdmin_UIs — later
  └── third-party Rust apps   crates.io
```

Target workspace after this plan: **`blit-core`, `blit-cli`,
`blit-daemon`** (three crates; `blit-prometheus-bridge` pending R5).

Out-of-repo consumers pin `blit-core` by crates.io version (or git rev
on the LAN gitea before the first publish) and upgrade deliberately.

## Rulings (chat, one at a time; record in `docs/DECISIONS.md`)

- **R1 — direction**: RULED, approved 2026-08-17 (D-2026-08-17-1).
- **R2 — interface-crate name**: DISSOLVED (D-2026-08-17-2); no new
  crate exists to name.
- **R3 — `blit-tui`** (29,172 lines, pre-dates the one-session
  architecture decisions and D-2026-08-15-1): (a) delete here, future
  TUI starts fresh in BlitAdmin_UIs against `blit-core` (git history
  keeps the code recoverable), or (b) move as-is to BlitAdmin_UIs.
  Recommendation: **(a)**.
- **R4 — `blit-gui`** (552 lines, the landed C1 eframe shell): (a)
  move as-is to BlitAdmin_UIs as the GUI starting point, then delete
  here, or (b) delete here, GUI starts fresh. Recommendation: **(a)**.
- **R5 — `blit-prometheus-bridge`** (877 lines, standalone binary, 20
  inline test fns, no integration tests, no consumers, not in any
  release archive; owner 2026-08-17: "never been executed, tested,
  requested, or desired"): delete, or keep. Recommendation: **delete**
  (if metrics are ever wanted, the daemon exposes them itself; git
  history keeps this code).

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
- **No module-name collisions**: none of the 13 incoming module names
  exists at `blit-core`'s top level (verified against
  `crates/blit-core/src/lib.rs`). Note `endpoint` (browse-pane
  endpoint model, from console-core) vs `endpoints` (CLI endpoint
  parsing, from blit-app) — different things; keep both names,
  document each in its module header.
- Dependency delta of the fold: `chrono` (new to core) and the
  `"disk"` feature on core's existing `sysinfo`. Everything else
  blit-app/console-core use, core already has.
- `blit-cli` references `blit_app::` at 96 sites and `blit_core::` at
  112 sites (grep counts; re-verify at implementation time). After the
  fold its only path dep is `blit-core`.
- gRPC client type: `blit_core::generated::blit_client::BlitClient`;
  bounded connect lives in `blit_app::client::connect_with_timeout`.
- crates.io names (checked 2026-08-17 via API): `blit-core` and
  `blit-daemon` AVAILABLE; `blit` and `blit-cli` TAKEN by unrelated
  projects (a sprite library; a terminal client). Irrelevant to
  binaries — they ship via package managers
  (`PACKAGE_MANAGER_DISTRIBUTION.md`), not crates.io. Only
  `blit-core` publishes.
- CI `.github/workflows/ci.yml` installs GUI headers in three jobs
  (lines 28–31, 52–55, 116–119) solely because `blit-gui` (egui)
  compiles in the workspace.
- `README.md:42` draws `blit-tui/` in the tree; comment-only `blit-tui`
  references remain in surviving crates (`crates/blit-app/Cargo.toml:8`,
  `crates/blit-app/src/lib.rs:2`,
  `crates/blit-cli/src/transfers/failures.rs:12`, …).
- `blit-core`'s build script vendors protoc, so crate consumers need
  no system protoc; `cargo package` must be verified to include
  `proto/` (see ip-6).

## Non-goals

- Building TUI or GUI features (BlitAdmin_UIs work, not this repo's).
- Relaxing D-2026-07-05-2 (same-build peers only) — see Deferred.
- New package IDs / channels — `PACKAGE_MANAGER_DISTRIBUTION.md` owns
  packaging; `blit-tui`/`blit-gui` IDs are added there when those
  front-ends ship.
- Curating/regrouping `blit-core`'s module tree — the fold moves
  modules in flat at top level; naming/grouping polish is future work
  (see Deferred).
- History rewrite; plain commits on `master`, one slice each.

## Acceptance criteria

- [ ] `cargo metadata` lists no `blit-app`, `blit-console-core`
      (and no `blit-gui`/`blit-tui`/`blit-prometheus-bridge` per
      R3/R4/R5); workspace is core + cli + daemon.
- [ ] `blit-cli`'s `Cargo.toml` depends on `blit-core` only; grep
      shows zero `blit_app::` / `blit_console_core::` paths anywhere
      in the tree.
- [ ] The three CI GUI-header blocks removed; CI green.
- [ ] `cargo package -p blit-core` succeeds (dry-run publishability);
      the actual `cargo publish` is a separately owner-gated act.
- [ ] `README.md` tree and surviving-crate comments match the new
      shape.
- [ ] Full gate green per slice (fmt; clippy native + strict
      `x86_64-unknown-linux-gnu` cross-target at `-D warnings`;
      `cargo test --workspace`; `scripts/agent/check-docs.sh`).
      Test-count deltas recorded per slice; drops only from deleted
      crates' own tests, named in the slice record.

## Slices (each: own go, own commit, full gate, DEVLOG entry)

1. **ip-1 — fold `blit-app` into `blit-core`.** Move the nine modules
   into `crates/blit-core/src/` as top-level `pub mod`s (no
   collisions, verified above); move their unit tests with them; add
   `chrono` and the sysinfo `"disk"` feature to core's Cargo.toml;
   rewrite `blit_app::` → `blit_core::` in blit-cli,
   blit-console-core, blit-tui and blit-prometheus-bridge (while
   present); drop `blit-app` from blit-cli's (and the others')
   Cargo.toml and from workspace members; delete `crates/blit-app`;
   regenerate `Cargo.lock`. Internal references from the moved
   `client` module to `blit_core::generated` become crate-local
   (`crate::generated`). Zero behavior change; test delta 0.
2. **ip-2 — fold `blit-console-core` into `blit-core`.** Move
   `browse.rs`, `discover.rs`, `endpoint.rs`, `model.rs` in as
   top-level `pub mod`s; document `endpoint` vs `endpoints` in both
   module headers; rewire `blit-gui` to `blit-core`; drop the member;
   delete the crate dir; regenerate `Cargo.lock`. Test delta 0.
3. **ip-3 — `blit-gui` per R4.** If (a): copy the crate into
   BlitAdmin_UIs with its dependency rewritten to `blit-core` pinned
   by git rev (every push to BlitAdmin_UIs is owner-gated,
   named-remote, approved in-session). Then, either ruling: delete
   `crates/blit-gui`, drop the member, remove the three CI GUI-header
   blocks, regenerate `Cargo.lock`. Record deleted-test delta.
4. **ip-4 — `blit-tui` per R3.** Delete (or move, per ruling) the
   crate; drop the member; regenerate `Cargo.lock`; fix `README.md`
   tree and comment-only references in surviving crates. Record
   deleted-test delta.
5. **ip-5 — `blit-prometheus-bridge` per R5.** If delete: remove the
   crate and member, regenerate `Cargo.lock`, record the 20-test
   delta.
6. **ip-6 — crates.io publishability.** Add publish metadata to
   `blit-core`'s Cargo.toml (`description`, `repository`, `readme`,
   `keywords`, `categories`; license already inherited MIT); verify
   `cargo package -p blit-core` succeeds and the packaged crate
   builds (protoc vendoring + `proto/` inclusion — fix
   `include`/`exclude` if the package misses files). **The actual
   `cargo publish` is outward-facing: owner's crates.io account/token,
   owner-approved in-session, never run by an agent on its own.**
   First published version = the workspace version current at publish
   time.
7. **ip-7 — record the shape.** Update `README.md` architecture prose
   and crate list to the final workspace; `docs/STATE.md` queue and
   authoritative-docs entries; any remaining supersession pointers.

Order: ip-1 → ip-2 strictly first (everything else touches the folded
crate). ip-3, ip-4, ip-5 are independent of each other; default order
as numbered. ip-6 after ip-1/ip-2; ip-7 last.

## Deferred (recorded so it is not rediscovered)

- **Peer version skew.** Independently installed front-ends and
  daemons WILL meet across versions — and crates.io embedders make
  this certain. D-2026-07-05-2 (same-build peers only, refusal at
  session open) stands until an explicit owner decision replaces it
  with contract-version compatibility (`CONTRACT_VERSION` in
  `proto/blit.proto` is the hook). Must be ruled before any
  front-end ships as a separately-versioned install.
- **Module-tree curation.** The fold leaves ~13 new top-level modules
  in `blit-core`. Whether/how to group them (and what the curated
  public API of a published crate should hide) is its own future
  plan; semver discipline starts mattering at first publish.
- **Curated event surface.** Front-ends initially use engine
  progress/observer APIs directly. If the raw surface proves too
  sharp for the TUI/GUI, a curated event-stream API is future work.
- **Publish cadence** (which versions/tags go to crates.io, and
  whether `blit-daemon` ever publishes) — decide at first publish.
