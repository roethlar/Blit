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
  **`docs/plan/SOURCE_SIDE_CONTAINMENT.md` (owner ruling D-2026-09-29-1,
  2026-09-29): a source-side per-file failure — locked, vanished, or
  size-drifted since the scan — must never end the run (D-2026-09-28-2);
  all six slices ssc-1..ssc-6 shipped with CI green on the candidate.
  Status 2026-09-30: all six slices LANDED on master (plan §Execution
  record). Status 2026-10-07: plan SHIPPED (D-2026-10-07-1), CI fully
  green on the three OSes at `189ae11b` (run 37570469240).**
  audit-16's open half (sink-less heartbeat ignores `--verbose`) — CLOSED
  2026-08-02 (DEVLOG 2026-08-02 03:25Z), listed here in error until the
  2026-09-29 refresh.
  Owner design gates (fix pre-1.0 or ship documented): **audit-18 RETIRED
  2026-09-30** by SOURCE_SIDE_CONTAINMENT ssc-5 (`6c266dd4`,
  D-2026-09-29-3: non-UTF-8 names transfer by raw bytes on every route;
  `CHANGELOG.md` `[Unreleased]`); **audit-19 OPEN** (`--exclude` silently
  matches nothing for absolute paths and bare directory names;
  `TODO.md`). Whichever ships open goes in release notes as a known
  limitation, verbatim enough to be found (drafted † in
  `docs/RELEASE_NOTES_1_0.md`). Also under G3 by D-2026-10-07-3 ("ship
  nothing until the open work is done"): the 2026-10-07 defects (a)(b)(c)
  are FIXED (`bbc75dec`, `b097aec0`, JOB_LOGS jl-1c); JOB_LOGS jl-1a..jl-5
  are landed and reviewed to consensus; one item found since is open — a
  failed `--detach` job's `jobs list` entry says only `delegated_pull
  handler failed` (`crates/blit-daemon/src/service/core.rs:793`), owner
  call fix-now or ship documented (drafted †).
- [ ] **G4 — docs truth pass.** README and `docs/TRANSFER_SESSION.md`
  claims verified against current behavior — expressly the ls-6 compare
  contract (default compare does not interrogate destination streams;
  `--checksum` does) and the ls-5 sweep. Plan-status sweep: `docs/plan/*`
  Active/Draft headers reconciled with reality (greenfield_plan_v6,
  OTP7/11/12, ONE_TRANSFER_PATH, UTE_REV4, ZERO_COPY, CLI_LIVE_PROGRESS,
  LOCAL_SMALL_FILE_PATH). `docs/STATE.md` queue de-drifted (pre-ls-6 text).
- [ ] **G5 — version + notes.** Workspace version 0.1.3 → 1.0.0 (root
  `Cargo.toml`, the two internal `blit-core` dependency pins, `Cargo.lock`),
  as the candidate's last commit; `CHANGELOG.md` `[Unreleased]` becomes
  `[1.0.0] - <date>`. The 0.1.1 → 0.1.3 content this gate once named
  shipped in 0.1.2 and 0.1.3 (`CHANGELOG.md`). Status 2026-10-14:
  `[Unreleased]` completed (source-side containment, retry passes,
  non-UTF-8 names, job logs/records/saved jobs, the fixes);
  `docs/RELEASE_NOTES_1_0.md` re-cut as 0.1.3 → 1.0.0 with the known
  limitations marked † where a G3 ruling is open. Bump not yet made.
- [ ] **G5b — cross-OS smoke matrix (owner-raised, 2026-08-02).** CI only
  proves same-OS loopback daemon transfers; the six directed cross-OS
  pairs (win↔linux, win↔mac, mac↔linux) run with the CANDIDATE binaries
  on the real fleet (netwatch-01, magneto, q — identical build identity
  `1.0.0+ef9a13b21afc` satisfies the same-build handshake). Each pair:
  small mixed tree with nested dirs and a content change, initial copy,
  content verify, converged re-run must report 0 changed. Recorded here
  with results per pair. **Facts 2026-10-14:** every pair can be driven
  from the Mac (push to, or pull from, the other host's daemon), so
  daemons are needed only on Linux and Windows; the candidate archives are
  the `Build Release` artifacts of the candidate's own CI run.
  `michael@magneto` answers (Arch, cargo 1.99). No Windows host answered:
  netwatch-01 (`10.1.10.173`/`.177`) and the ARM64 dev VM (`10.1.10.222`,
  which needs `en11` carrier and is emulated x86_64 — supporting evidence
  only) all timed out. The four Windows legs wait on a Windows host; the
  two Mac↔Linux legs can run as soon as the candidate's CI has built. `q`
  is no longer a fleet host for this.
  **Pre-candidate dry run 2026-10-14 with CI's `8844dfbd` archives**
  (identity `0.1.3+8844dfbd8f37` on both ends; sidecars verified; a
  modules-only daemon on magneto `10.1.10.10:9031`, mDNS off, state inside
  its scratch dir; evidence `docs/bench/g5b-dryrun-2026-10-14/`):

  | pair | initial copy | content verify | edit → re-run | converged re-run | result |
  | --- | --- | --- | --- | --- | --- |
  | mac → linux (push) | 12 files, 5,382,443 B | 12/12 sha256 match, no extras | 1 file, 39 B | 0 files | PASS |
  | linux → mac (pull) | 12 files, 5,382,472 B | 12/12 sha256 match, entry lists identical | — | 0 files | PASS |
  | win ↔ linux, win ↔ mac (4 legs) | — | — | — | — | BLOCKED: no Windows host answered |

  Every command exited 0; no warning or stall notice during a transfer;
  `blit jobs list` on the Mac and `blit jobs list magneto` show the same
  five runs under the same job IDs. To repeat on the candidate's own
  archives once CI has built them; the Windows legs when a host answers.
- **G5c — DISSOLVED (D-2026-08-15-2): 1.0 is CLI + daemon only; no UI
  surface gates this or any release.** UIs are a separate app
  (`BlitAdmin_UIs` repo) with their own lifecycle. History: this gate was
  raised by D-2026-08-02-1, re-pointed by D-2026-08-02-2, scoped to both
  Console surfaces by D-2026-08-04-3; all superseded by D-2026-08-15-1
  (Blit is CLI + daemon only). Crate deletion runs under
  `docs/plan/UI_REMOVAL.md`, independent of this release plan.
- [ ] **G6 — OWNER GATE: tag v1.0.0 and publish.** Outward act; needs the
  owner's explicit go on the exact candidate SHA. The tag's CI run
  publishes the GitHub Release and updates the bot channels (Homebrew tap,
  Scoop, AUR, winget); homebrew-core is NOT configured (no
  `HOMEBREW_CORE_TOKEN` secret, verified 2026-10-14), so D-2026-10-04-2's
  pm-5 hold cannot be breached by the tag. crates.io publishes of
  `blit-core`, `blit-transfer`, `blit-daemon` 1.0.0 are the owner's
  (the harness blocks an agent's `cargo publish`; `--dry-run` is not).

## Readiness as of 2026-10-14 (owner goal: "do everything necessary to get this ready for 1.0")

Where each gate stands and who moves it. CI is fully green on `8844dfbd`
(run 38474595246), the last code commit; every slice since `v0.1.3` is
reviewed to consensus.

| Gate | Status | Moves on |
| --- | --- | --- |
| G2 CI on the candidate | pending the candidate (the version-bump commit) | push (owner go), then the run |
| G3 dispositions | SSC shipped; audit-18 retired; **audit-19 open**; **detached recents reason open** | two owner rulings, fix-now or ship documented |
| G4 docs truth | README + TRANSFER_SESSION audit running 2026-10-14; plan-status sweep needs rulings on 9 Active plans (below); STATE de-drift agent's | audit result → edits; one owner ruling on the sweep |
| G5 version + notes | CHANGELOG and notes done 2026-10-14; bump pending | agent, after the G3 rulings land |
| G5b cross-OS matrix | Mac↔Linux ready once the candidate's CI has built; Windows legs blocked (no host reachable) | a Windows host |
| G6 tag + publish | — | owner |

Plan-status sweep (G4): nine plans read `Active` while their work is
landed or deferred post-release — ONE_TRANSFER_PATH (otp-1..12c `[x]`,
otp-12d/13 post-release by D-2026-07-22-1), OTP7_RESUME, OTP11_LOCAL_SESSION,
OTP12_ACCEPTANCE_RUN, UNIFIED_TRANSFER_ENGINE_REV4 (code-complete;
measurement gates post-release), CLI_LIVE_PROGRESS (clp-1..3 landed; F2/F5
owner calls), LOCAL_SMALL_FILE_PATH (ls-0..6 shipped), SMALL_FILE_CEILING
(sf-3d landed; rig A/B owed), PACKAGE_MANAGER_DISTRIBUTION (pm-5 held).
Proposed: flip the first seven to Shipped (or Historical where the doc is
a design record) with a one-line note each; keep SMALL_FILE_CEILING and
PACKAGE_MANAGER_DISTRIBUTION Active with their open item named. Status
flips are owner declarations (precedent D-2026-10-07-1).

Ordered path to the tag:

1. Owner rulings, one at a time: audit-19; the detached recents reason;
   the plan-status sweep; the JOB_LOGS acceptance ticks (evidence per
   criterion in `docs/plan/JOB_LOGS.md` slices).
2. Any fix ruled lands as a slice under this plan (G3 fix-now): guard,
   gate, Codex codereview with the owner's dispatch approval.
3. G4 edits: README/TRANSFER_SESSION corrections from the audit; STATE
   queue de-drift; plan-status flips as ruled.
4. G5: version bump + `[1.0.0]` + final notes as one commit → push (owner
   go) → G2 CI green on that SHA.
5. G5b with the candidate's archives: Mac↔Linux now; Windows legs when a
   host answers (recorded per pair here).
6. Optional final Codex range review of the release-prep commits (owner
   dispatch approval).
7. G6: tag (owner) → release job → crates.io publishes (owner).

## Non-goals (post-1.0)

F15 logging epic, RDMA, wire-carrier comparison concurrency, sweep
prefetch, 25GbE work, competitor benchmarks. (TUI rework was listed here
until D-2026-08-02-1 made it gate G5c — my original scope call, owner
overruled.)

## Working rules for this effort

Coding fixes go to opus/sonnet subagents (owner instruction, 2026-08-01);
one finding per commit with guard proof and the full verification gate;
codex range review before the candidate is declared.
