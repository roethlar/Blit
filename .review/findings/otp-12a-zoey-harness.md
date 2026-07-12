# otp-12a — zoey converge-up harness (interleaved old-vs-new)

**Plan**: `docs/plan/OTP12_ACCEPTANCE_RUN.md` (Active, owner 2026-07-12),
sub-slice 12a, harness half. The recorded-run half follows on the rig
(needs the owner's fresh go for daemon runs on zoey + zoey out of
maintenance).
**Status**: implemented, codex review pending.

## What

`scripts/bench_otp12_zoey.sh` — the otp-2 verdict matrix ({large, small,
mixed} × {push, pull} × {tcp, grpc} = 12 comparisons) rerun as
matched-pair interleaved A/B: arm old = pinned `e757dcc` pair (Mac client
staged at `$MAC_WORK/bins/blit-e757dcc`, zoey's kept 2026-07-10 daemon),
arm new = the run commit's pair (local release build + freshly zigbuilt
musl daemon staged beside the old one). Per-direction converge-up only
(D-2026-07-05-1); verdicts computed against BOTH references (same-session
old arm AND the committed `docs/bench/otp2-baseline-2026-07-10/summary.csv`
medians), per design D2.

## Approach

Methodology functions carried verbatim from `bench_otp2_baseline.sh`
(wall-clock windows, self-timed destination flushes, drain-then-purge
ordering, fixture recipes, ControlMaster mux). New mechanics per the
design doc: ABBA counterbalanced pair order (F5); pair-void-and-re-run
valid-run rule with a 2×RUNS attempt cap and INCOMPLETE surfacing (F7);
blit exit codes captured with per-run logs under `$OUT_DIR/blit-logs/`
(the old harness swallowed them); daemon lifecycle parameterized by arm
with swap-only-on-arm-change (untimed) plus a stale-daemon refusal
(otp-2w F2 posture, new on this rig); binary provenance recorded to
`staging-manifest.txt` (sha256 all four binaries — the OLD pair predates
the handshake, so provenance is the staging record; the NEW pair's smoke
transfer doubles as its build-identity check via D-2026-07-05-2);
`PREFLIGHT_ONLY=1` mode (no daemon start, nothing timed); summary +
verdict computation in one python3 pass (macOS ships bash 3.2 — no
associative arrays anywhere).

## Files

- `scripts/bench_otp12_zoey.sh` (new; self-contained by design D5 — the
  frozen `bench_otp2_baseline.sh` is untouched).
- `docs/plan/OTP12_ACCEPTANCE_RUN.md` — D5 `runs.csv` schema gains the
  `valid` column (pair-fate under the D2 rule; one-line amendment).

## Tests

- `bash -n` clean. shellcheck not installed on this machine (recorded
  here rather than claimed).
- No crates/proto/Cargo changes anywhere in otp-12
  (`git diff --stat ce36da3..HEAD -- crates proto Cargo.toml Cargo.lock`
  is empty); the suite stands at the recorded 1484 green from otp-11b.
  A fresh gate run at this tree confirmed fmt + clippy pass and showed
  no test failures.
- The harness itself is verified by the probe/recorded-run discipline on
  the rig (otp-2 precedent): the recorded-run half commits the evidence.

## Known gaps

- Not yet executed against the rig — PREFLIGHT_ONLY and the full matrix
  both need zoey (maintenance 2026-07-11) and the owner's fresh daemon
  go. First live session may surface busybox/ssh quirks the otp-2 script
  did not (pgrep availability, sha256sum path).
- Old-arm provenance rests on the staging record + sha256 manifest, not
  a handshake (pre-handshake binaries) — accepted residual risk per the
  design doc.
- The escalation rule (straddle + spread > 25% → RUNS=8 fresh session)
  is manual by design, not automated in the script.
- `meta.csv` (pairs-attempted/completeness) is a working file consumed
  by the verdict pass; the committed evidence carries its content via
  `summary.csv`'s `pairs_attempted` column and the INCOMPLETE verdict
  rows.
