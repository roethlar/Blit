# Release 1.0

**Status**: Active (owner goal, 2026-08-01: "get this ready for a prod
release, targeting 1.0. commit & push & review as you deem appropriate.")
**Created**: 2026-08-01
**Model**: `RELEASE_COMPLETION.md` (v0.1.1) supplies the gate template:
exact reviewed candidate, green hosted CI on three platforms, truthful
docs, packaging/startup sanity — with the release act itself owner-gated.

## Goal

Cut v1.0.0 from an exact reviewed, CI-green commit, with documentation
that tells the truth about current behavior (including the 2026-08-01
compare-contract change, D-2026-08-01-4) and release notes that name every
known limitation shipped open. "Ready" is agent work; the tag and publish
are the owner's act.

## Gates

- [ ] **G1 — reviews clean.** `7fa18f5b..ba3fc73a` r1 returned
  `guard_confirmed: true`, VERIFIED-CLOSED cr-ls5-1, and found one HIGH
  successor: cr-ls5-2 (session-stale absent snapshots re-open the alias
  overwrite; taint-on-verdict fix prescribed). G1 = cr-ls5-2 fixed and
  the range re-reviewed to clean.
- [ ] **G2 — CI green on the exact candidate**, both workflows (CI: fmt,
  strict clippy, Linux/macOS/Windows tests; Docs Gate).
- [ ] **G3 — known broken behaviors dispositioned.** Fix now:
  audit-16's open half (sink-less heartbeat ignores `--verbose`).
  Owner design gates (fix pre-1.0 or ship documented): audit-18
  (non-UTF-8 filenames fail to transfer — per-file contained since pfc,
  but unfixable-by-user), audit-19 (`--exclude` silently matches nothing
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
- [ ] **G6 — OWNER GATE: tag v1.0.0 and publish.** Outward act; needs the
  owner's explicit go on the exact candidate SHA.

## Non-goals (post-1.0)

TUI rework M1–M6 (`TUI_REWORK.md` continues), F15 logging epic, RDMA,
wire-carrier comparison concurrency, sweep prefetch, 25GbE work,
competitor benchmarks.

## Working rules for this effort

Coding fixes go to opus/sonnet subagents (owner instruction, 2026-08-01);
one finding per commit with guard proof and the full verification gate;
codex range review before the candidate is declared.
