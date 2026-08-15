# Release 1.0

**Status**: Active (owner goal, 2026-08-01: "get this ready for a prod
release, targeting 1.0. commit & push & review as you deem appropriate.")
**Created**: 2026-08-01
**Model**: `RELEASE_COMPLETION.md` (v0.1.1) supplies the gate template:
exact reviewed candidate, green hosted CI on three platforms, truthful
docs, packaging/startup sanity — with the release act itself owner-gated.

> **DEFERRED, NOT CLOSED (D-2026-08-08-1, 2026-08-08).** A **v0.1.2 patch
> release shipped independently** of this plan by explicit owner ruling
> ("blit 0.1.2"), carrying all 177 commits since `v0.1.1` — including the
> D-2026-08-01-4 compare-contract change. **No gate below was closed,
> satisfied, or waived by it**: G2–G6 remain open exactly as written and
> still bind v1.0.0, which must close them on its own candidate. Two
> consequences for anyone resuming this plan: the workspace version is now
> `0.1.2`, so G5's "0.1.1 → 1.0.0" bump is now `0.1.2 → 1.0.0` and G5b's
> `1.0.0+ef9a13b21afc` build identity no longer exists on any binary
> (`ef9a13b2`'s candidate label is retired, its `dist/` archives are
> historical pipeline proof only); and `docs/RELEASE_NOTES_1_0.md` still
> reads as 1.0's notes while describing behavior that shipped in 0.1.2 —
> re-cutting it is part of closing G5. 0.1.2's own scope is stated in
> `CHANGELOG.md` `[0.1.2]`.

## Goal

Cut v1.0.0 from an exact reviewed, CI-green commit, with documentation
that tells the truth about current behavior (including the 2026-08-01
compare-contract change, D-2026-08-01-4) and release notes that name every
known limitation shipped open. "Ready" is agent work; the tag and publish
are the owner's act.

## Gates

- [x] **G1 — reviews clean.** Closed 2026-08-02: ls6-range program ran 3
  rounds / 5 findings (cr-ls5-1/-2/-3, nonutf8-release-claim, cr-a16-1),
  all VERIFIED-CLOSED; r3 over `6c7f67f0..016c8578` returned **clean,
  `guard_confirmed: true`, zero findings**
  (`.review/results/ls6-range.codex.r3.json`).
- [ ] **G2 — CI green on the exact candidate**, both workflows (CI: fmt,
  strict clippy, Linux/macOS/Windows tests; Docs Gate).
- [ ] **G3 — known broken behaviors dispositioned.** Fix now:
  audit-16's open half (sink-less heartbeat ignores `--verbose`).
  Owner design gates (fix pre-1.0 or ship documented): audit-18
  (non-UTF-8 filenames fail to transfer — per-file contained on LOCAL
  transfers only; a REMOTE push can still abort wholesale at source
  payload preparation, per ls6-range r2's nonutf8-release-claim), audit-19 (`--exclude` silently matches nothing
  for absolute paths and bare directory names). Whichever ships open goes
  in release notes as a known limitation, verbatim enough to be found.
- [ ] **G4 — docs truth pass.** README and `docs/TRANSFER_SESSION.md`
  claims verified against current behavior — expressly the ls-6 compare
  contract (default compare does not interrogate destination streams;
  `--checksum` does) and the ls-5 sweep. Plan-status sweep: `docs/plan/*`
  Active/Draft headers reconciled with reality (greenfield_plan_v6,
  OTP7/11/12, ONE_TRANSFER_PATH, UTE_REV4, ZERO_COPY, CLI_LIVE_PROGRESS,
  LOCAL_SMALL_FILE_PATH). `docs/STATE.md` queue de-drifted (pre-ls-6 text).
- [ ] **G5 — version + notes.** Workspace version 0.1.1 → 1.0.0; release
  notes cover 0.1.1 → 1.0.0 (pfc containment, ls-0..6 performance ~5.1×
  on the field mirror, colour output, compare-contract change, fixes,
  known limitations).
- [ ] **G5b — cross-OS smoke matrix (owner-raised, 2026-08-02).** CI only
  proves same-OS loopback daemon transfers; the six directed cross-OS
  pairs (win↔linux, win↔mac, mac↔linux) run with the CANDIDATE binaries
  on the real fleet (netwatch-01, magneto, q — identical build identity
  `1.0.0+ef9a13b21afc` satisfies the same-build handshake). Each pair:
  small mixed tree with nested dirs and a content change, initial copy,
  content verify, converged re-run must report 0 changed. Recorded here
  with results per pair.
- **G5c — DISSOLVED (D-2026-08-15-2): 1.0 is CLI + daemon only; no UI
  surface gates this or any release.** UIs are a separate app
  (`BlitAdmin_UIs` repo) with their own lifecycle. History: this gate was
  raised by D-2026-08-02-1, re-pointed by D-2026-08-02-2, scoped to both
  Console surfaces by D-2026-08-04-3; all superseded by D-2026-08-15-1
  (Blit is CLI + daemon only). Crate deletion runs under
  `docs/plan/UI_REMOVAL.md`, independent of this release plan.
- [ ] **G6 — OWNER GATE: tag v1.0.0 and publish.** Outward act; needs the
  owner's explicit go on the exact candidate SHA.

## Non-goals (post-1.0)

F15 logging epic, RDMA, wire-carrier comparison concurrency, sweep
prefetch, 25GbE work, competitor benchmarks. (TUI rework was listed here
until D-2026-08-02-1 made it gate G5c — my original scope call, owner
overruled.)

## Working rules for this effort

Coding fixes go to opus/sonnet subagents (owner instruction, 2026-08-01);
one finding per commit with guard proof and the full verification gate;
codex range review before the candidate is declared.
