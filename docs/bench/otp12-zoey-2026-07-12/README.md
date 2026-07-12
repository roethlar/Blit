# otp-12a — unified-path vs OLD-path interleaved A/B on the Mac↔zoey rig (2026-07-12)

**Status**: Recorded. **Scope (load-bearing)**: rig Z anchors
**per-direction converge-up only** (hardware-asymmetric endpoints,
D-2026-07-05-1; `docs/bench/otp2-baseline-2026-07-10/README.md` §Status).
Cross-direction and initiator/verb-invariance claims belong to rig W
(otp-12b). **This README declares nothing** — pass/fail is the owner's
at the otp-13 walk (design doc `docs/plan/OTP12_ACCEPTANCE_RUN.md`,
Governs); it records the computed D2 comparisons.

**Harness**: `scripts/bench_otp12_zoey.sh` (methodology inherited from
the frozen `bench_otp2_baseline.sh`; new mechanics — ABBA counterbalance,
pair-void valid-run rule, both-reference verdicts — per the design doc
D1/D2/D5). RUNS=4 main session, RUNS=8 escalation (the pre-registered D2
rule). Zero voided pairs in any recorded session.

## Builds (matched pairs, clean trees, sha-embedded — manifests committed)

- **old arm**: clean `e757dcc` rebuilds BOTH ends (Mac client via
  detached worktree; zoey daemon `cargo zigbuild --release --target
  aarch64-unknown-linux-musl`, staged as `blit-temp/blit-daemon-e757dcc`).
  **Provenance correction found en route**: the 2026-07-10 staging at
  `blit-temp/blit-daemon` embeds `731023bfc8a1.dirty.…`, NOT `e757dcc`
  as the otp-2 README claimed (correction note committed there,
  `b2b6901`); that artifact was left untouched and NOT used here.
- **new arm**: the run commit both ends — `042c06f` (main session),
  `6bc9cb6` (escalation; the inter-session diff is harness-script-only).
  Zero `crates/**`/`proto/**` changes exist anywhere in otp-12: the
  transfer code both sessions is exactly the plan's post-otp-11 HEAD
  (`ce36da3` lineage), suite 1484.
- sha256 of every binary + the committed reference CSV:
  `staging-manifest.txt` / `escalation-staging-manifest.txt`.

## Sessions

1. **Aborted storm session** (`aborted-storm-runs.csv`, 12 runs kept):
   zoey degraded progressively — load average 1.4 → 444, run times ~10×
   the committed baseline, BOTH arms equally, drains still "passing."
   Root cause consistent with accumulated per-run push destinations
   (~15 GiB) congesting the pool write path: after stopping, load fell
   within minutes; three back-to-back probes WITH per-run deletion held
   at baseline (2466/2525/3714 ms vs committed 2702). Harness now sweeps
   each destination right after its flush is measured (outside the timed
   window). No data from this session feeds any verdict.
2. **Main session** (RUNS=4; `runs.csv`/`summary.csv`/`verdicts.csv`):
   full 12-comparison matrix, 48 pairs, all valid. 9/12 PASS both
   references; 3 escalated per the pre-registered D2 rules.
3. **Escalation session** (RUNS=8, `CELLS` allowlist;
   `escalation-*.csv`): the three flagged comparisons re-run fresh.

## Final per-comparison state (escalation supersedes where run — D2)

| comparison | new ms | old same-session | ratio | committed | ratio | combined |
|------------|-------:|-----------------:|------:|----------:|------:|----------|
| push_tcp_large  | 2464 | 2570 | 0.959 | 2702 | 0.912 | **PASS** (RUNS=8; the RUNS=4 FAIL-BOTH was noise — new arm spread was 100%, its best run beat the old median) |
| push_grpc_large | 4567 | 4369 | 1.045 | 4510 | 1.013 | **PASS** |
| pull_tcp_large  | 2167 | 2177 | 0.995 | 1744 | 1.243 | **FAIL-REFERENCE-DRIFT** (persisted at RUNS=8; see Drift) |
| pull_grpc_large | 2702 | 2706 | 0.999 | 2585 | 1.045 | **PASS** |
| push_tcp_small  | 3984 | 3605 | 1.105 | 4263 | 0.935 | **FAIL-SAME-SESSION** (persisted; see the marginal-gap note) |
| push_grpc_small | 4731 | 4727 | 1.001 | 5217 | 0.907 | **PASS** |
| pull_tcp_small  | 2277 | 2266 | 1.005 | 2784 | 0.818 | **PASS** |
| pull_grpc_small | 3148 | 3463 | 0.909 | 4188 | 0.752 | **PASS** |
| push_tcp_mixed  | 2142 | 2053 | 1.043 | 2070 | 1.035 | **PASS** |
| push_grpc_mixed | 3468 | 3666 | 0.946 | 3889 | 0.892 | **PASS** |
| pull_tcp_mixed  | 1521 | 1575 | 0.966 | 1401 | 1.086 | **PASS** |
| pull_grpc_mixed | 2107 | 2252 | 0.936 | 2222 | 0.948 | **PASS** |

Rollup: **10 PASS, 1 FAIL-REFERENCE-DRIFT, 1 FAIL-SAME-SESSION** — both
non-PASS cells carried to the otp-13 walk with the analysis below.

## Drift analysis (pull_tcp_large)

The drift is provably rig-side, not the unified path's: the OLD arm —
the same old-path code the committed baseline measured — ran 2177 ms
median this session vs its own committed 1744 ms (**1.248×**), while
new-vs-old same-session is **0.995** (the unified path is not slower
than the old path on this rig, this day). The rig's large-pull speed
changed between 2026-07-10 and 2026-07-12 (uptime 22 days; an owner-side
maintenance touched the box on 07-11). Per D2 a persisting drift stands
recorded, never silently excused.

## The marginal same-session gap (push_tcp_small)

Reproducible across both sessions (1.109 at RUNS=4, **1.105** at RUNS=8
with tight spreads: new 16.7%, old 18.7%) — a real ≈10.5% same-session
gap, 0.5% over the ±10% noise bar, on this cell only. Context the walk
needs: the unified path BEATS the committed old-path baseline by 6.5%
(3984 vs 4263) — the rig ran small pushes ~15% faster today than on
07-10 in both arms, and against that faster old arm the session sits a
hair over the bar. Every other small/mixed cell has the unified path at
or ahead of old (pull_grpc_small 0.909, push_grpc_small 1.001,
pull_tcp_small 1.005). If a per-cell look is wanted, it is a
post-otp-12 item; nothing here blocks otp-12b/c mechanically.

## Reproduction

```
export ZOEY_SSH=root@zoey
export ZOEY_TEMP=/volume/<pool-uuid>/.srv/.unifi-drive/michael/.data/blit-temp
export ZOEY_HOST=10.1.10.206
RUNS=4 ./scripts/bench_otp12_zoey.sh                     # full matrix
CELLS=<comma-list> RUNS=8 ./scripts/bench_otp12_zoey.sh  # D2 escalation
PREFLIGHT_ONLY=1 ./scripts/bench_otp12_zoey.sh           # checks only
```

Requires: clean tree at the run commit; old client staged at
`~/blit-bench-work/bins/blit-e757dcc`; both sha-named daemons staged in
`blit-temp/`; python3 + NOPASSWD purge on the Mac. The staged 2026-07-10
`blit-temp/blit-daemon` (dirty-`731023b`) is an otp-2 artifact — never
run it for otp-12 arms.
