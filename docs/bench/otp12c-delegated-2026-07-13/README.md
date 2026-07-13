# otp-12c — skippy↔Windows delegated session (rig D, 2026-07-13)

**Status**: Recorded. **Scope**: the rig-D delegated-vs-direct parity
matrix (netwatch-01↔skippy), run at harness commit `a2dea3f` with the
cutover binaries (`EXPECT_SHA=f35702a`). Companion to the direct-path
Mac↔Windows record at `docs/bench/otp12c-win-2026-07-13/`. **This
README declares nothing** — pass/fail belongs to the owner at otp-13;
it records the computed rows.

**Harness**: `scripts/bench_otp12_delegated.sh` at run commit
`a2dea3f` (repo HEAD, working tree clean). Preflight recorded
`HEAD=a2dea3f EXPECT_SHA=f35702a`. Orchestrated from the Mac
(`10.1.10.91`); endpoints skippy `10.1.10.143:9031` ↔ netwatch-01
`10.1.10.177:9031`.

Two sessions, both committed here:

- **Primary (full matrix)**: session `021026`, RUNS=4, ABBA,
  pair-void rule; 7 cells × 4 pairs × 2 arms = **56 timed runs, 7/7
  cells complete, 0 voided pairs**. 02:10:26–02:23:43 local. Files at
  the top level of this directory.
- **Confirmation re-run**: session `031155`, RUNS=8 on the two cells
  the primary session failed (`CELLS=sw_tcp_mixed,ws_tcp_large`);
  **32 timed runs, 0 voided pairs**. 03:11:55–03:20:16 local. Files
  under `rerun-8pair/`.

## Builds (sha-verified; 5 hashes in `staging-manifest.txt`)

All five binaries (mac client; skippy client+daemon; windows
client+daemon) staged from the same build, manifest sha `a2dea3f`,
embed-verified at preflight against `EXPECT_SHA=f35702a` (the windows
binaries live under `D:\blit-test\bins\f35702a\`; skippy under
`/mnt/generic-pool/video/blit-bin/`). Identical manifest hashes in
both sessions — no restaging between them.

## Verdicts

**Primary (RUNS=4)** — 7 rows: 5 PASS, 2 FAIL. The two FAIL rows,
verbatim from `verdicts.csv`:

```
sw_tcp_mixed,delegated,delegated,direct,2154,1925,1.119,1.10,FAIL
ws_tcp_large,delegated,delegated,direct,4647,4115,1.129,1.10,FAIL
```

**Both cells met D2's pre-registered escalation trigger** — each
straddles its bar (1.119 / 1.129 against 1.10) *and* has an arm whose
spread exceeds 25% (delegated 86.0% / 55.4%). Per that rule they were
re-run at RUNS=8, interleaved, in a fresh session; per D2's supersession
amendment (2026-07-12, codex otp-12a-run F2) **the RUNS=8 medians govern
the escalated comparison's outcome**, and the RUNS=4 rows stay committed
and visible. The escalated rows, verbatim from
`rerun-8pair/verdicts.csv`:

```
sw_tcp_mixed,delegated,delegated,direct,2054,1985,1.035,1.10,PASS
ws_tcp_large,delegated,delegated,direct,4093,4370,1.068,1.10,PASS
```

**Governing outcome for rig D: 7/7 PASS** (5 at RUNS=4 + 2 escalated
to RUNS=8). Acceptance is still the owner's at otp-13; this README
applies the pre-registered arithmetic and declares nothing beyond it.

Reading notes (numbers, no adjudication):

- The primary FAILs ride high **delegated-arm** spread — `sw_tcp_mixed`
  86.0% and `ws_tcp_large` 55.4%, against 8.5% / 17.3% on their direct
  arms — where single slow early slots pull the average far above the
  median (`sw_tcp_mixed` delegated median 2154 vs avg 2533). The
  widest spread in the session, `sw_tcp_small` delegated at 93.6%,
  belongs to a cell that **passed** (1.034): spread alone does not
  decide a cell.
- At 8 pairs comparable spread appears on the **direct** arm too
  (31.5% / 64.0%), and `ws_tcp_large`'s direct median moves 4115→4370,
  landing *above* the delegated median. The noise is not arm-specific
  once n is larger — which is what the escalation rule exists to
  resolve.
- `ws_tcp_large`'s primary delegated best (3000 ms) is **faster** than
  its direct best (3870 ms) in the same session.
- The secondary gRPC carrier cell `sw_grpc_large` recorded 1.012.

## Files

Primary session at top level, confirmation under `rerun-8pair/`; each
set: `runs.csv`, `summary.csv`, `verdicts.csv`, `meta.csv`,
`staging-manifest.txt`, `drain-outcomes.txt` (the harness also emits
`bench.log`/`drain.log`, not committed — the repo `.gitignore`
excludes `*.log`). Per-run daemon/client logs stayed with the
sessions at `logs/otp12_delegated_20260713T021026/blit-logs/` (29
files) and `logs/otp12_delegated_20260713T031155/blit-logs/` (20
files), not committed; session TOMLs (`skippy-bench.toml`,
`win-bench.toml`) likewise stayed with the session dirs.
