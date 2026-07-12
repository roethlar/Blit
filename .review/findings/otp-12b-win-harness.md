# otp-12b — Mac↔Windows harness (converge-up + initiator/verb invariance)

**Plan**: `docs/plan/OTP12_ACCEPTANCE_RUN.md` (Active), sub-slice 12b,
harness half. The recorded-run half follows on the rig (fresh bundle to
the Windows box; its checkout moves off the `0f922de` detachment — the
owner's `bench-cargo-lock` stash untouched).
**Status**: implemented, codex review pending.

## What

`scripts/bench_otp12_win.sh` — two blocks on the owner-designated
closest-spec pair. Block 1: the otp-2w matrix as matched-pair
interleaved old(`0f922de`)/new A/B, Mac-initiated, verdicts against
both references (same-session old + the committed
`otp2w-baseline-2026-07-10` medians). Block 2 — the plan's headline
criterion measured for the first time: per data direction × fixture ×
carrier, Mac-initiated vs Windows-initiated arms interleaved ABBA
(`mw_*`/`wm_*` cells), plus per-arm converge rows (design F3), the F4
cross-direction rows vs `min(old_push, old_pull)` committed, and
D-2026-07-12-1 discriminator gap rows (kind `cross-gap`, outcome
`RECORDED` — the harness never adjudicates the residue).

## Approach

Windows plumbing verbatim from the frozen `bench_otp2w_baseline.sh`
(WMI launch, stale-refusal + PID-scoped teardown, TOML literal paths,
Get-Counter drain with fail-loud errors, standby purge, self-timed
`Write-VolumeCache`, CRLF stripping). Every otp-12a lesson carried:
ABBA counterbalance, pair-void valid-run rule (2×RUNS cap, INCOMPLETE),
exit codes checked, `+sha` provenance greps (Windows side via
`Select-String -SimpleMatch`), fail-closed sha256 manifest (7 hashes),
per-run destination sweep after the measured flush (the zoey storm
lesson, kept uniform), PREFLIGHT_ONLY, CELLS allowlist + typo
validation, session-gated traps with identity-verified kills both
hosts. New mechanics: arm swap via a fixed active exe path
(`bins\active\blit-daemon.exe`, one program-scoped firewall rule
`blit-otp12-daemon`; sha-named source dirs keep provenance); a Mac
daemon serving `$MAC_WORK` itself as the module root (design F6 — both
initiators of a Mac→Win cell read the same physical inodes); the
Windows-initiated timed window measured ON Windows (Stopwatch brackets
the `blit.exe` run inside one ssh call, printing `<ms>,<exit>` — the
otp-2w F3 rule applied to a whole client run); flush keyed by
destination OS never verb; block-2 arms do identical work (symmetric
no-trailing-slash nesting both arms — block 1 keeps the otp-2w shapes
for baseline comparability); a win→mac smoke gates the macOS
application-firewall unknown before anything is timed.

## Files

- `scripts/bench_otp12_win.sh` (new, executable; self-contained per D5 —
  the frozen otp-2w script untouched).
- `docs/plan/OTP12_ACCEPTANCE_RUN.md` — D5 cell grammar extended for
  the invariance (`mw|wm`) and `gap_*`/`cross-gap` rows.

## Tests

- `bash -n` clean; shellcheck not installed on this machine (recorded).
- No crates/proto/Cargo changes; the suite stands at the recorded 1484.
- The harness itself is verified by the preflight/smoke discipline on
  the rig; the recorded-run half commits the evidence.

## Known gaps

- Not yet executed — first contact with: `Select-String` provenance
  greps on ~10 MB exes, `$LASTEXITCODE` capture through the mux'd ssh,
  the macOS application firewall (smoke-gated), and blit.exe writing
  into the served module directory as a local path.
- The old Mac client (`0f922de`) predates embedded client build ids
  (otp-12a-run F1) — `OLD_CLIENT_PROVENANCE_BY_BUILD=1` acknowledgment
  required, provenance = clean-worktree rebuild + manifest.
- Mac-destination runs have no drain equivalent (recorded design
  decision D3): `sync` + purge only, exactly as the recorded otp-2w
  pull cells.
- Block-2 converge rows compare against block-1 same-session old arms;
  if block 1 is CELLS-filtered away in an escalation session, those
  rows fall back to committed-reference-only (the python emits what
  exists — the README of the recorded run must note any such session).
