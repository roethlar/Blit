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

Primary (4-pair) — 7 rows: **5 PASS / 2 FAIL**. FAIL rows verbatim
from `verdicts.csv`:

```
sw_tcp_mixed,delegated,delegated,direct,2154,1925,1.119,1.10,FAIL
ws_tcp_large,delegated,delegated,direct,4647,4115,1.129,1.10,FAIL
```

Confirmation (8-pair, same cells) — 2 rows: **2 PASS**, verbatim:

```
sw_tcp_mixed,delegated,delegated,direct,2054,1985,1.035,1.10,PASS
ws_tcp_large,delegated,delegated,direct,4093,4370,1.068,1.10,PASS
```

Reading notes (numbers only, no adjudication):

- The primary FAILs sit on high delegated-arm spread:
  `sw_tcp_mixed` 86.0%, `sw_tcp_small` 93.6%, `ws_tcp_large` 55.4%
  vs 8.5%/9.1%/17.3% direct — single slow early slots pull the avg
  well above the median (`sw_tcp_mixed` delegated median 2154 vs avg
  2533; `sw_tcp_small` 1860 vs 2277).
- At 8 pairs the same two cells recorded 1.035 and 1.068, and the
  spread appeared on the **direct** arm too (31.5% / 64.0%);
  `ws_tcp_large` direct median moved 4115→4370, landing above the
  delegated median. The noise is not arm-specific at higher n.
- `ws_tcp_large` primary delegated best (3000ms) is **faster** than
  the direct best (3870ms) in the same session.
- The gRPC secondary cell `sw_grpc_large` recorded 1.012.
- Both the 4-pair FAIL rows and the 8-pair PASS rows are on record;
  neither supersedes the other here.

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
